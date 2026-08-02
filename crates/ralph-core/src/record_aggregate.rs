//! record-session JSONL 的 strict 解析与聚合摘要。
//!
//! 设计目标:
//! - strict parse: 用于"已完成的证据文件"(autopilot, record summary)。
//! - 聚合口径统一: topic_counts/stdout_tail/termination 等统计,避免各处各写一套而漂移。
//! - 错误可读: 一旦 JSONL 非法,尽量定位到第一个坏行(line number)便于排障。
//!
//! 说明:
//! - 这是"回放读取"域(SessionPlayer → 聚合),与运行时"写入"域的 evidence_index 区分。
//! - 渲染(Evidence Inspect 文本)留在 ralph-cli,这里只产出结构化聚合。

use anyhow::{Context, Result};
use ralph_proto::{Event, TerminalWrite};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::io::BufRead;
use std::io::BufReader;
use std::path::Path;

use crate::session_player::SessionPlayer;
use crate::session_recorder::Record;
use crate::{
    CapabilityParentFailedRecord, CapabilityParentResultRecord, CapabilityRequestRecord,
    TOPIC_CAPABILITY_FAILED, TOPIC_CAPABILITY_REQUEST, TOPIC_CAPABILITY_RESULT,
    TOPIC_TOPOLOGY_SPAWN_FAILED, TOPIC_TOPOLOGY_SPAWN_GROUP, TOPIC_TOPOLOGY_SPAWN_RESULT,
    TopologySpawnGroupFailed, TopologySpawnGroupRequest, TopologySpawnGroupResult,
};

/// `_meta.session_start` 的数据形状(最小集).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaSessionStart {
    pub cwd: Option<String>,
    pub workspace_root: Option<String>,
    pub argv: Vec<String>,
    pub argv_joined: Option<String>,
    pub pid: u32,
    pub current_exe: Option<String>,
    pub version: Option<String>,
}

/// `_meta.loop_start` 的数据形状(最小集).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaLoopStart {
    pub prompt_file: String,
    pub max_iterations: u32,
    pub ux_mode: String,
}

/// `_meta.termination` 的数据形状.
///
/// 说明:
/// - 老 cassette 可能缺某些字段,这里用 Option 做兼容(避免 strict parse 被历史数据卡死).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetaTermination {
    pub reason: Option<String>,
    pub iterations: Option<u32>,
    pub elapsed_secs: Option<f64>,
    pub ux_writes: Option<u32>,
}

/// record-session 的通用聚合摘要(不包含 autopilot 的业务断言).
#[derive(Debug, Clone, Default)]
pub struct RecordSessionAggregate {
    pub session_start: Option<MetaSessionStart>,
    pub loop_start: Option<MetaLoopStart>,
    pub termination: Option<MetaTermination>,
    pub termination_record_index: Option<usize>,
    pub topic_counts: BTreeMap<String, usize>,
    pub topic_timeline: Vec<String>,
    pub stdout_tail: String,
    pub evidence: EvidenceInspectAggregate,
}

/// 面向 `ralph record summary` 的统一证据聚合。
///
/// 说明:
/// - record-session 是主证据,所以 topology / capability / result topic 都从 bus event 中提取。
/// - `.ralph/agents.json` 是 sidecar,不放在这里,由渲染阶段显式传入。
#[derive(Debug, Clone, Default)]
pub struct EvidenceInspectAggregate {
    pub topology_spawn_groups: Vec<TopologySpawnGroupEvidence>,
    pub topology_spawn_results: Vec<TopologySpawnResultEvidence>,
    pub topology_spawn_failures: Vec<TopologySpawnFailedEvidence>,
    pub capability_requests: Vec<CapabilityRequestEvidence>,
    pub capability_results: Vec<CapabilityResultEvidence>,
    pub capability_failures: Vec<CapabilityFailedEvidence>,
    pub result_topics: BTreeMap<String, ResultTopicEvidence>,
    pub parse_errors: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TopologySpawnGroupEvidence {
    pub record_index: usize,
    pub event_id: Option<String>,
    pub request: TopologySpawnGroupRequest,
}

#[derive(Debug, Clone)]
pub struct TopologySpawnResultEvidence {
    pub record_index: usize,
    pub event_id: Option<String>,
    pub result: TopologySpawnGroupResult,
}

#[derive(Debug, Clone)]
pub struct TopologySpawnFailedEvidence {
    pub record_index: usize,
    pub event_id: Option<String>,
    pub failed: TopologySpawnGroupFailed,
}

#[derive(Debug, Clone)]
pub struct CapabilityRequestEvidence {
    pub record_index: usize,
    pub event_id: Option<String>,
    pub request: CapabilityRequestRecord,
}

#[derive(Debug, Clone)]
pub struct CapabilityResultEvidence {
    pub record_index: usize,
    pub event_id: Option<String>,
    pub result: CapabilityParentResultRecord,
}

#[derive(Debug, Clone)]
pub struct CapabilityFailedEvidence {
    pub record_index: usize,
    pub event_id: Option<String>,
    pub failed: CapabilityParentFailedRecord,
}

#[derive(Debug, Clone, Default)]
pub struct ResultTopicEvidence {
    pub count: usize,
    pub source_instances: BTreeSet<String>,
}

impl ResultTopicEvidence {
    fn record(&mut self, event: &Event) {
        self.count += 1;
        if let Some(source_instance) = &event.source_instance {
            self.source_instances.insert(source_instance.to_string());
        }
    }
}

/// 窄入口: 给定 record-session 路径, 返回结构化聚合。
///
/// 内部串联 strict 加载与聚合; 调用者不需要知道 SessionPlayer 的存在。
pub fn aggregate_session(path: &Path) -> Result<RecordSessionAggregate> {
    let player = load_session_player_strict(path)?;
    aggregate_record_session(&player)
}

pub fn load_session_player_strict(path: &Path) -> Result<SessionPlayer> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("Failed to open record-session file: {}", path.display()))?;
    let reader = BufReader::new(file);

    match SessionPlayer::from_reader(reader) {
        Ok(player) => Ok(player),
        Err(e) => Err(render_strict_parse_error(path, &e)),
    }
}

pub fn aggregate_record_session(player: &SessionPlayer) -> Result<RecordSessionAggregate> {
    // stdout tail: 只保留最后 N 段,再拼接.
    let mut tail_chunks: VecDeque<String> = VecDeque::new();
    const MAX_TAIL_CHUNKS: usize = 200;

    let mut out = RecordSessionAggregate::default();

    for (idx, rec) in player.records().iter().enumerate() {
        let event_type = rec.record.event.as_str();

        // 1) meta
        if event_type == "_meta.session_start" && out.session_start.is_none() {
            let meta: MetaSessionStart = serde_json::from_value(rec.record.data.clone())
                .with_context(|| format!("Failed to parse _meta.session_start at record[{idx}]"))?;
            out.session_start = Some(meta);
            continue;
        }

        if event_type == "_meta.loop_start" && out.loop_start.is_none() {
            let meta: MetaLoopStart = serde_json::from_value(rec.record.data.clone())
                .with_context(|| format!("Failed to parse _meta.loop_start at record[{idx}]"))?;
            out.loop_start = Some(meta);
            continue;
        }

        if event_type == "_meta.termination" {
            let meta: MetaTermination = serde_json::from_value(rec.record.data.clone())
                .with_context(|| {
                    format!("Failed to parse _meta.termination as MetaTermination at record[{idx}]")
                })?;
            out.termination = Some(meta);
            out.termination_record_index = Some(idx);
            continue;
        }

        // 2) bus.publish -> topic counts / timeline
        if event_type == "bus.publish" {
            let evt: Event =
                serde_json::from_value(rec.record.data.clone()).with_context(|| {
                    format!("Failed to parse bus.publish data as Event at record[{idx}]")
                })?;
            let topic = evt.topic.as_str().to_string();
            *out.topic_counts.entry(topic.clone()).or_insert(0) += 1;
            if out.topic_timeline.len() < 2000 {
                out.topic_timeline.push(topic);
            }
            collect_evidence_from_bus_event(&mut out.evidence, idx, &evt);
            continue;
        }

        // 3) ux.terminal.write -> stdout tail
        if event_type == "ux.terminal.write" {
            let write: TerminalWrite = serde_json::from_value(rec.record.data.clone())
                .with_context(|| {
                    format!("Failed to parse ux.terminal.write as TerminalWrite at record[{idx}]")
                })?;

            // 只收集 stdout(参与事件解析的输出),避免 stderr 噪音污染 tail.
            if !write.stdout {
                continue;
            }

            let text = if let Some(text) = write.text.clone() {
                text
            } else {
                // 旧 cassette 可能缺 `text`,这里回退用 bytes decode 的 lossy 视图.
                match write.decode_bytes() {
                    Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
                    Err(_) => String::new(),
                }
            };

            if !text.is_empty() {
                tail_chunks.push_back(text);
                while tail_chunks.len() > MAX_TAIL_CHUNKS {
                    tail_chunks.pop_front();
                }
            }
        }
    }

    out.stdout_tail = tail_chunks.into_iter().collect::<String>();
    Ok(out)
}

fn collect_evidence_from_bus_event(
    evidence: &mut EvidenceInspectAggregate,
    record_index: usize,
    event: &Event,
) {
    let topic = event.topic.as_str();

    if is_result_like_topic(topic) {
        evidence
            .result_topics
            .entry(topic.to_string())
            .or_default()
            .record(event);
    }

    match topic {
        TOPIC_TOPOLOGY_SPAWN_GROUP => {
            match TopologySpawnGroupRequest::parse_payload(&event.payload) {
                Ok(request) => evidence
                    .topology_spawn_groups
                    .push(TopologySpawnGroupEvidence {
                        record_index,
                        event_id: event.id.clone(),
                        request,
                    }),
                Err(error) => evidence.parse_errors.push(format!(
                    "record[{record_index}] {TOPIC_TOPOLOGY_SPAWN_GROUP}: {error}"
                )),
            }
        }
        TOPIC_TOPOLOGY_SPAWN_RESULT => {
            match serde_json::from_str::<TopologySpawnGroupResult>(&event.payload) {
                Ok(result) => evidence
                    .topology_spawn_results
                    .push(TopologySpawnResultEvidence {
                        record_index,
                        event_id: event.id.clone(),
                        result,
                    }),
                Err(error) => evidence.parse_errors.push(format!(
                    "record[{record_index}] {TOPIC_TOPOLOGY_SPAWN_RESULT}: {error}"
                )),
            }
        }
        TOPIC_TOPOLOGY_SPAWN_FAILED => {
            match serde_json::from_str::<TopologySpawnGroupFailed>(&event.payload) {
                Ok(failed) => evidence
                    .topology_spawn_failures
                    .push(TopologySpawnFailedEvidence {
                        record_index,
                        event_id: event.id.clone(),
                        failed,
                    }),
                Err(error) => evidence.parse_errors.push(format!(
                    "record[{record_index}] {TOPIC_TOPOLOGY_SPAWN_FAILED}: {error}"
                )),
            }
        }
        TOPIC_CAPABILITY_REQUEST => match CapabilityRequestRecord::parse_payload(&event.payload) {
            Ok(request) => evidence
                .capability_requests
                .push(CapabilityRequestEvidence {
                    record_index,
                    event_id: event.id.clone(),
                    request,
                }),
            Err(error) => evidence.parse_errors.push(format!(
                "record[{record_index}] {TOPIC_CAPABILITY_REQUEST}: {}",
                error.error
            )),
        },
        TOPIC_CAPABILITY_RESULT => {
            match serde_json::from_str::<CapabilityParentResultRecord>(&event.payload) {
                Ok(result) => evidence.capability_results.push(CapabilityResultEvidence {
                    record_index,
                    event_id: event.id.clone(),
                    result,
                }),
                Err(error) => evidence.parse_errors.push(format!(
                    "record[{record_index}] {TOPIC_CAPABILITY_RESULT}: {error}"
                )),
            }
        }
        TOPIC_CAPABILITY_FAILED => {
            match serde_json::from_str::<CapabilityParentFailedRecord>(&event.payload) {
                Ok(failed) => evidence.capability_failures.push(CapabilityFailedEvidence {
                    record_index,
                    event_id: event.id.clone(),
                    failed,
                }),
                Err(error) => evidence.parse_errors.push(format!(
                    "record[{record_index}] {TOPIC_CAPABILITY_FAILED}: {error}"
                )),
            }
        }
        _ => {}
    }
}

fn is_result_like_topic(topic: &str) -> bool {
    topic
        .rsplit_once('.')
        .is_some_and(|(_prefix, suffix)| suffix == "done")
        || topic == "reply.human.message"
        || matches!(
            topic,
            TOPIC_CAPABILITY_RESULT
                | TOPIC_CAPABILITY_FAILED
                | TOPIC_TOPOLOGY_SPAWN_RESULT
                | TOPIC_TOPOLOGY_SPAWN_FAILED
        )
}


fn render_strict_parse_error(path: &Path, err: &std::io::Error) -> anyhow::Error {
    // SessionPlayer 的默认错误信息缺少行号.
    // 这里做一次 best-effort 的定位,把“第一个坏行”找出来,便于用户快速排障.
    let detail = first_invalid_record_line(path)
        .map(|d| format!(" ({d})"))
        .unwrap_or_default();

    anyhow::anyhow!(
        "Invalid record-session JSONL: {}{detail}: {err}",
        path.display()
    )
}

fn first_invalid_record_line(path: &Path) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    let reader = std::io::BufReader::new(file);

    for (i, line) in reader.lines().enumerate() {
        let line = line.ok()?;
        if line.trim().is_empty() {
            continue;
        }

        if let Err(e) = serde_json::from_str::<Record>(&line) {
            // 只给 preview,避免把整行塞进错误消息(可能很长).
            let preview = truncate_preview(&line, 200);
            return Some(format!("line={} err={e} preview={preview}", i + 1));
        }
    }

    None
}

fn truncate_preview(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i >= max_chars {
            break;
        }
        out.push(ch);
    }
    out.push_str("...");
    out
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::session_recorder::Record;
    use crate::SessionRecorder;
    use ralph_proto::{TerminalWrite, UxEvent};

    #[test]
    fn strict_parse_error_includes_line_number_best_effort() -> Result<()> {
        let dir = tempfile::TempDir::new()?;
        let path = dir.path().join("bad.jsonl");
        std::fs::write(&path, "not-json\n")?;

        let err = load_session_player_strict(&path).unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("line="),
            "should include line hint, got: {msg}"
        );
        Ok(())
    }

    #[test]
    fn aggregate_collects_meta_topics_and_stdout_tail() -> Result<()> {
        let dir = tempfile::TempDir::new()?;
        let path = dir.path().join("session.jsonl");

        // 写一份最小可解析 cassette.
        let file = std::fs::File::create(&path)?;
        let recorder = SessionRecorder::new(std::io::BufWriter::new(file));

        let argv = vec!["ralph".to_string(), "run".to_string()];
        recorder.record_meta(Record::meta_session_start(
            Some("/tmp/project"),
            Some("/tmp/project"),
            &argv,
            42,
            Some("/tmp/bin/ralph"),
            Some("0.0.0-test"),
        ));
        recorder.record_meta(Record::meta_loop_start("PROMPT.md", 3, Some("cli")));
        recorder.record_bus_event(&Event::new("human.message", "hi"));
        recorder.record_ux_event(&UxEvent::TerminalWrite(TerminalWrite::new(
            b"hello", true, 0,
        )));
        recorder.record_meta(Record::meta_termination("Interrupted", 1, 0.1, 2));
        recorder.flush().ok();

        let player = load_session_player_strict(&path)?;
        let agg = aggregate_record_session(&player)?;

        assert!(
            agg.session_start.is_some(),
            "_meta.session_start should be present"
        );
        assert!(
            agg.loop_start.is_some(),
            "_meta.loop_start should be present"
        );
        assert!(
            agg.termination
                .as_ref()
                .and_then(|t| t.reason.as_deref())
                .is_some_and(|r| r == "Interrupted"),
            "_meta.termination.reason should be parsed"
        );
        assert_eq!(
            agg.topic_counts.get("human.message").copied().unwrap_or(0),
            1
        );
        assert!(
            agg.stdout_tail.contains("hello"),
            "stdout tail should include terminal output"
        );
        Ok(())
    }
}
