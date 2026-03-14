//! record-session JSONL 的 strict 解析与聚合摘要(供 autopilot/record summary 复用)。
//!
//! 设计目标:
//! - strict parse: 用于“已完成的证据文件”(autopilot, record summary).
//! - 聚合口径统一: topic_counts/stdout_tail/termination 等统计,避免各处各写一套而漂移.
//! - 错误可读: 一旦 JSONL 非法,尽量定位到第一个坏行(line number)便于排障.

use anyhow::{Context, Result};
use ralph_core::{Record, SessionPlayer};
use ralph_proto::{Event, TerminalWrite};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use std::io::BufRead;
use std::io::BufReader;
use std::path::Path;
use std::path::PathBuf;

/// `_meta.session_start` 的数据形状(最小集).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct MetaSessionStart {
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
pub(crate) struct MetaLoopStart {
    pub prompt_file: String,
    pub max_iterations: u32,
    pub ux_mode: String,
}

/// `_meta.termination` 的数据形状.
///
/// 说明:
/// - 老 cassette 可能缺某些字段,这里用 Option 做兼容(避免 strict parse 被历史数据卡死).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct MetaTermination {
    pub reason: Option<String>,
    pub iterations: Option<u32>,
    pub elapsed_secs: Option<f64>,
    pub ux_writes: Option<u32>,
}

/// record-session 的通用聚合摘要(不包含 autopilot 的业务断言).
#[derive(Debug, Clone, Default)]
pub(crate) struct RecordSessionAggregate {
    pub session_start: Option<MetaSessionStart>,
    pub loop_start: Option<MetaLoopStart>,
    pub termination: Option<MetaTermination>,
    pub termination_record_index: Option<usize>,
    pub topic_counts: BTreeMap<String, usize>,
    pub topic_timeline: Vec<String>,
    pub stdout_tail: String,
}

pub(crate) fn load_session_player_strict(path: &Path) -> Result<SessionPlayer> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("Failed to open record-session file: {}", path.display()))?;
    let reader = BufReader::new(file);

    match SessionPlayer::from_reader(reader) {
        Ok(player) => Ok(player),
        Err(e) => Err(render_strict_parse_error(path, &e)),
    }
}

pub(crate) fn aggregate_record_session(player: &SessionPlayer) -> Result<RecordSessionAggregate> {
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
            continue;
        }
    }

    out.stdout_tail = tail_chunks.into_iter().collect::<Vec<_>>().join("");
    Ok(out)
}

pub(crate) fn write_record_session_latest_pointer(
    workspace_root: &Path,
    record_path: &Path,
) -> Result<PathBuf> {
    // 说明:
    // - record-session 文件不一定放在 `.ralph/` 里.
    // - 但 `.ralph/record-session.latest` 作为“证据指针”应位于 workspace_root 下,
    //   便于在子目录执行 `ralph record watch` 时向上自动定位.
    let pointer_path = workspace_root.join(".ralph/record-session.latest");
    if let Some(parent) = pointer_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create pointer directory: {}", parent.display()))?;
    }

    // 优先写绝对路径,避免 watch 时的歧义.
    let absolute = if record_path.is_absolute() {
        record_path.to_path_buf()
    } else {
        workspace_root.join(record_path)
    };

    // canonicalize 是 best-effort:
    // - 失败时仍写一个“可解析路径”(absolute).
    let target = std::fs::canonicalize(&absolute).unwrap_or(absolute);

    std::fs::write(&pointer_path, format!("{}\n", target.display())).with_context(|| {
        format!(
            "Failed to write record-session latest pointer: {}",
            pointer_path.display()
        )
    })?;

    Ok(pointer_path)
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
    use ralph_core::SessionRecorder;
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
