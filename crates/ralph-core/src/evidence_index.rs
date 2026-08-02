//! 最小 runtime evidence index kernel。
//!
//! 设计边界:
//! - 这里只记录 artifact link 与 correlation 关系。
//! - 不实现 `ralph evidence summary` / `inspect` / `doctor` 这类展示或诊断 UX。
//! - 不把 runtime graph / Rerun layout 当作 durable truth source。

use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Evidence index v1 schema version。
pub const EVIDENCE_INDEX_SCHEMA_VERSION: u32 = 1;

/// Phase 1A 固定的最小 artifact kind 集合。
///
/// 说明:
/// - enum 序列化为 snake_case,与 OpenSpec 中的 kind 文本保持一致。
/// - 不包含 CLI summary / doctor diagnosis / graph layout 这些 Phase 1B 字段。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceArtifactKind {
    RecordSessionJsonl,
    EventLogJsonl,
    AgentsSnapshotJson,
    RuntimeDeliveryRecord,
    RuntimeLifecycleRecord,
    ReplyEvent,
    CapabilityInvokeJson,
    CapabilityResultJson,
    CapabilityFailedJson,
    ResolvedConfig,
    MissingArtifact,
}

/// Evidence artifact 的最小状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStatus {
    Success,
    Failure,
    Missing,
    Unknown,
}

/// 单条 evidence index 记录。
///
/// 说明:
/// - `correlation_id` 是查找主键,可以是 event id / reply id / invocation id 等。
/// - `artifact_path` 只保存路径文本,不读取或复制 artifact 内容。
/// - parent/child 字段只表达最小 lineage,不建 runtime graph。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceIndexEntry {
    pub schema_version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    pub correlation_id: String,
    pub artifact_kind: EvidenceArtifactKind,
    pub artifact_path: String,
    pub producer: String,
    pub status: EvidenceStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_correlation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub child_correlation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_topic: Option<String>,
    pub created_at: String,
}

impl EvidenceIndexEntry {
    /// 创建一条普通 artifact link。
    pub fn new(
        correlation_id: impl Into<String>,
        artifact_kind: EvidenceArtifactKind,
        artifact_path: impl Into<String>,
        producer: impl Into<String>,
        status: EvidenceStatus,
    ) -> Self {
        Self {
            schema_version: EVIDENCE_INDEX_SCHEMA_VERSION,
            session_id: None,
            run_id: None,
            correlation_id: correlation_id.into(),
            artifact_kind,
            artifact_path: artifact_path.into(),
            producer: producer.into(),
            status,
            parent_correlation_id: None,
            child_correlation_id: None,
            result_topic: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// 创建一条 missing artifact marker。
    pub fn missing(
        correlation_id: impl Into<String>,
        expected_kind: EvidenceArtifactKind,
        expected_path: impl Into<String>,
        producer: impl Into<String>,
    ) -> Self {
        Self::new(
            correlation_id,
            expected_kind,
            expected_path,
            producer,
            EvidenceStatus::Missing,
        )
    }

    /// 创建 dynamic role contract hash -> source artifact 的 correlation link。
    ///
    /// 说明:
    /// - index 只保存 hash / request / instance 关联和 artifact path。
    /// - 完整 role contract 与 record-session / agents snapshot 仍是源 artifact。
    pub fn dynamic_role_contract(
        role_contract_hash: impl Into<String>,
        artifact_kind: EvidenceArtifactKind,
        artifact_path: impl Into<String>,
        producer: impl Into<String>,
        spawn_request_id: impl Into<String>,
        instance_id: impl Into<String>,
    ) -> Self {
        Self::new(
            role_contract_hash,
            artifact_kind,
            artifact_path,
            producer,
            EvidenceStatus::Success,
        )
        .with_parent_correlation_id(spawn_request_id)
        .with_child_correlation_id(instance_id)
    }

    /// 创建 dynamic role result topic -> source artifact 的 correlation link。
    ///
    /// 说明:
    /// - 这是 `dynamic_role_contract` 的带 topic 版本。
    /// - `result_topic` 是轻量 correlation metadata,不是完整 result payload。
    pub fn dynamic_role_result_topic(
        role_contract_hash: impl Into<String>,
        artifact_kind: EvidenceArtifactKind,
        artifact_path: impl Into<String>,
        producer: impl Into<String>,
        spawn_request_id: impl Into<String>,
        instance_id: impl Into<String>,
        result_topic: impl Into<String>,
    ) -> Self {
        Self::dynamic_role_contract(
            role_contract_hash,
            artifact_kind,
            artifact_path,
            producer,
            spawn_request_id,
            instance_id,
        )
        .with_result_topic(result_topic)
    }

    /// 创建 spawn request id -> spawned child instance 的 correlation link。
    pub fn dynamic_spawn_request(
        spawn_request_id: impl Into<String>,
        artifact_kind: EvidenceArtifactKind,
        artifact_path: impl Into<String>,
        producer: impl Into<String>,
        instance_id: impl Into<String>,
    ) -> Self {
        Self::new(
            spawn_request_id,
            artifact_kind,
            artifact_path,
            producer,
            EvidenceStatus::Success,
        )
        .with_child_correlation_id(instance_id)
    }

    /// 创建 dynamic result 缺失 marker。
    ///
    /// 说明:
    /// - correlation_id 建议由调用方构造成稳定 marker id,例如
    ///   `missing-result:{request_id}:{instance_id}:{topic}`。
    /// - parent 绑定 spawn request id,child 绑定 role contract hash。
    pub fn missing_dynamic_result(
        marker_id: impl Into<String>,
        artifact_path: impl Into<String>,
        producer: impl Into<String>,
        spawn_request_id: impl Into<String>,
        role_contract_hash: impl Into<String>,
        expected_result_topic: impl Into<String>,
    ) -> Self {
        Self::missing(
            marker_id,
            EvidenceArtifactKind::MissingArtifact,
            artifact_path,
            producer,
        )
        .with_parent_correlation_id(spawn_request_id)
        .with_child_correlation_id(role_contract_hash)
        .with_result_topic(expected_result_topic)
    }

    /// 绑定 session id。
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// 绑定 run id。
    pub fn with_run_id(mut self, run_id: impl Into<String>) -> Self {
        self.run_id = Some(run_id.into());
        self
    }

    /// 绑定 parent correlation id。
    pub fn with_parent_correlation_id(mut self, parent: impl Into<String>) -> Self {
        self.parent_correlation_id = Some(parent.into());
        self
    }

    /// 绑定 child correlation id。
    pub fn with_child_correlation_id(mut self, child: impl Into<String>) -> Self {
        self.child_correlation_id = Some(child.into());
        self
    }

    /// 绑定 produced / expected result topic。
    ///
    /// 说明:
    /// - 这里只保存 topic 名称,不保存 result payload。
    /// - 原始事件仍在 record-session 或 event log 中。
    pub fn with_result_topic(mut self, topic: impl Into<String>) -> Self {
        self.result_topic = Some(topic.into());
        self
    }
}

/// Evidence lookup 的结果分类。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceLookup {
    /// 没有任何 index entry。
    NoEntry,
    /// 找到至少一个 success / failure / unknown artifact。
    Entries(Vec<EvidenceIndexEntry>),
    /// 找到 missing marker。
    Missing(Vec<EvidenceIndexEntry>),
}

impl EvidenceLookup {
    /// 返回所有匹配 entry,用于测试和后续调用方继续过滤。
    pub fn entries(&self) -> &[EvidenceIndexEntry] {
        match self {
            Self::NoEntry => &[],
            Self::Entries(entries) | Self::Missing(entries) => entries,
        }
    }
}

/// Evidence index 读写错误。
#[derive(Debug, Error)]
pub enum EvidenceIndexError {
    #[error("evidence index io error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("evidence index json error at {path} line {line}: {source}")]
    Json {
        path: PathBuf,
        line: usize,
        #[source]
        source: serde_json::Error,
    },
}

/// Append-only JSONL writer。
pub struct EvidenceIndexWriter {
    path: PathBuf,
    file: Option<File>,
}

impl EvidenceIndexWriter {
    /// Default path for the runtime evidence index JSONL file.
    pub const DEFAULT_PATH: &'static str = ".ralph/evidence-index.jsonl";

    /// 创建 writer。父目录会在首次写入时创建。
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            file: None,
        }
    }

    /// 返回 index 文件路径。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 记录一条 entry,并立即 flush,让测试和 watch 类工具能读到完整 JSONL 行。
    pub fn record(&mut self, entry: &EvidenceIndexEntry) -> Result<(), EvidenceIndexError> {
        let path = self.path.clone();
        let file = self.ensure_open()?;
        let mut line = serde_json::to_string(entry).map_err(|source| EvidenceIndexError::Json {
            path: path.clone(),
            line: 0,
            source,
        })?;
        line.push('\n');
        file.write_all(line.as_bytes())
            .map_err(|source| EvidenceIndexError::Io {
                path: path.clone(),
                source,
            })?;
        file.flush()
            .map_err(|source| EvidenceIndexError::Io { path, source })
    }

    fn ensure_open(&mut self) -> Result<&mut File, EvidenceIndexError> {
        if self.file.is_none() {
            if let Some(parent) = self.path.parent() {
                fs::create_dir_all(parent).map_err(|source| EvidenceIndexError::Io {
                    path: parent.to_path_buf(),
                    source,
                })?;
            }
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)
                .map_err(|source| EvidenceIndexError::Io {
                    path: self.path.clone(),
                    source,
                })?;
            self.file = Some(file);
        }
        Ok(self.file.as_mut().expect("evidence index file is open"))
    }
}

/// JSONL reader,只提供 Phase 1A 所需的 correlation lookup。
pub struct EvidenceIndexReader {
    path: PathBuf,
}

impl EvidenceIndexReader {
    /// 创建 reader。
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// 读取全部 entries。不存在的 index 视为 empty。
    pub fn read_all(&self) -> Result<Vec<EvidenceIndexEntry>, EvidenceIndexError> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&self.path).map_err(|source| EvidenceIndexError::Io {
            path: self.path.clone(),
            source,
        })?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();

        for (index, line) in reader.lines().enumerate() {
            let line = line.map_err(|source| EvidenceIndexError::Io {
                path: self.path.clone(),
                source,
            })?;
            if line.trim().is_empty() {
                continue;
            }
            let entry = serde_json::from_str(&line).map_err(|source| EvidenceIndexError::Json {
                path: self.path.clone(),
                line: index + 1,
                source,
            })?;
            entries.push(entry);
        }

        Ok(entries)
    }

    /// 按 correlation id 查找 entries。
    ///
    /// 说明:
    /// - 会匹配主 correlation id,也会匹配 parent / child correlation id。
    /// - 这样 request id 和 role contract hash 能查到挂在 lineage 上的 missing marker。
    pub fn find_by_correlation(
        &self,
        correlation_id: &str,
    ) -> Result<EvidenceLookup, EvidenceIndexError> {
        let matched: Vec<EvidenceIndexEntry> = self
            .read_all()?
            .into_iter()
            .filter(|entry| {
                entry.correlation_id == correlation_id
                    || entry.parent_correlation_id.as_deref() == Some(correlation_id)
                    || entry.child_correlation_id.as_deref() == Some(correlation_id)
                    || entry.result_topic.as_deref() == Some(correlation_id)
            })
            .collect();

        if matched.is_empty() {
            return Ok(EvidenceLookup::NoEntry);
        }

        if matched
            .iter()
            .any(|entry| entry.status == EvidenceStatus::Missing)
        {
            return Ok(EvidenceLookup::Missing(matched));
        }

        Ok(EvidenceLookup::Entries(matched))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use tempfile::TempDir;

    fn temp_index_path() -> (TempDir, PathBuf) {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join(".ralph/evidence-index.jsonl");
        (temp, path)
    }

    #[test]
    fn evidence_index_entry_serializes_minimal_schema_without_display_fields() {
        let entry = EvidenceIndexEntry::new(
            "cap-1",
            EvidenceArtifactKind::CapabilityResultJson,
            ".ralph/capability-invocations/cap-1/result.json",
            "capability-invocation",
            EvidenceStatus::Success,
        )
        .with_session_id("session-1")
        .with_run_id("run-1")
        .with_parent_correlation_id("parent-run")
        .with_child_correlation_id("child-run");

        let json = serde_json::to_value(&entry).unwrap();
        assert_eq!(json["schema_version"], EVIDENCE_INDEX_SCHEMA_VERSION);
        assert_eq!(json["session_id"], "session-1");
        assert_eq!(json["run_id"], "run-1");
        assert_eq!(json["correlation_id"], "cap-1");
        assert_eq!(json["artifact_kind"], "capability_result_json");
        assert_eq!(
            json["artifact_path"],
            ".ralph/capability-invocations/cap-1/result.json"
        );
        assert_eq!(json["producer"], "capability-invocation");
        assert_eq!(json["status"], "success");
        assert_eq!(json["parent_correlation_id"], "parent-run");
        assert_eq!(json["child_correlation_id"], "child-run");
        assert!(json.get("created_at").is_some());

        let object = json.as_object().unwrap();
        assert!(!object.contains_key("result_topic"));
        assert!(!object.contains_key("summary"));
        assert!(!object.contains_key("inspect"));
        assert!(!object.contains_key("doctor"));
        assert!(!object.contains_key("diagnosis"));
        assert!(!object.contains_key("graph_layout"));
        assert!(!object.contains_key("rendered"));
    }

    #[test]
    fn evidence_index_writer_and_reader_lookup_by_correlation_id() {
        let (_temp, path) = temp_index_path();
        let mut writer = EvidenceIndexWriter::new(&path);

        writer
            .record(&EvidenceIndexEntry::new(
                "cap-1",
                EvidenceArtifactKind::CapabilityInvokeJson,
                ".ralph/capability-invocations/cap-1/invoke.json",
                "capability-invocation",
                EvidenceStatus::Success,
            ))
            .unwrap();
        writer
            .record(&EvidenceIndexEntry::new(
                "cap-1",
                EvidenceArtifactKind::CapabilityResultJson,
                ".ralph/capability-invocations/cap-1/result.json",
                "capability-invocation",
                EvidenceStatus::Success,
            ))
            .unwrap();
        writer
            .record(&EvidenceIndexEntry::new(
                "other",
                EvidenceArtifactKind::RecordSessionJsonl,
                "/tmp/session.jsonl",
                "record-session",
                EvidenceStatus::Success,
            ))
            .unwrap();

        let reader = EvidenceIndexReader::new(&path);
        let lookup = reader.find_by_correlation("cap-1").unwrap();
        let entries = lookup.entries();

        assert!(matches!(lookup, EvidenceLookup::Entries(_)));
        assert_eq!(entries.len(), 2);
        assert!(
            entries
                .iter()
                .any(|entry| entry.artifact_kind == EvidenceArtifactKind::CapabilityInvokeJson)
        );
        assert!(
            entries
                .iter()
                .any(|entry| entry.artifact_kind == EvidenceArtifactKind::CapabilityResultJson)
        );
        assert!(matches!(
            reader.find_by_correlation("missing-correlation").unwrap(),
            EvidenceLookup::NoEntry
        ));
    }

    #[test]
    fn missing_artifact_marker_is_distinct_from_no_entry() {
        let (_temp, path) = temp_index_path();
        let mut writer = EvidenceIndexWriter::new(&path);

        writer
            .record(&EvidenceIndexEntry::missing(
                "reply-404",
                EvidenceArtifactKind::ReplyEvent,
                ".ralph/events.jsonl",
                "event-logger",
            ))
            .unwrap();

        let reader = EvidenceIndexReader::new(&path);
        let lookup = reader.find_by_correlation("reply-404").unwrap();
        let entries = lookup.entries();

        assert!(matches!(lookup, EvidenceLookup::Missing(_)));
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].status, EvidenceStatus::Missing);
        assert_eq!(entries[0].artifact_kind, EvidenceArtifactKind::ReplyEvent);
        assert!(matches!(
            reader.find_by_correlation("never-written").unwrap(),
            EvidenceLookup::NoEntry
        ));
    }

    #[test]
    fn parent_child_links_use_correlation_ids_without_topology_fields() {
        let entry = EvidenceIndexEntry::new(
            "child-result-1",
            EvidenceArtifactKind::CapabilityResultJson,
            ".ralph/capability-invocations/cap-1/result.json",
            "capability-invocation",
            EvidenceStatus::Success,
        )
        .with_parent_correlation_id("parent-invocation-1")
        .with_child_correlation_id("child-result-1");

        let json = serde_json::to_value(&entry).unwrap();
        assert_eq!(json["parent_correlation_id"], "parent-invocation-1");
        assert_eq!(json["child_correlation_id"], "child-result-1");

        let object = json.as_object().unwrap();
        assert!(!object.contains_key("hat_registry"));
        assert!(!object.contains_key("event_loop"));
        assert!(!object.contains_key("supervisor_topology"));
        assert!(!object.contains_key("live_topology_mutation"));
    }

    #[test]
    fn existing_record_session_artifact_can_be_indexed_without_replacing_source() {
        let temp = TempDir::new().unwrap();
        let record_session_path = temp.path().join("session.jsonl");
        let index_path = temp.path().join(".ralph/evidence-index.jsonl");
        std::fs::write(
            &record_session_path,
            concat!(
                "{\"event\":\"_meta.session_start\",\"data\":{\"pid\":1},\"ts\":1}\n",
                "{\"event\":\"_meta.termination\",\"data\":{\"reason\":\"LoopComplete\"},\"ts\":2}\n"
            ),
        )
        .unwrap();

        let mut writer = EvidenceIndexWriter::new(&index_path);
        writer
            .record(
                &EvidenceIndexEntry::new(
                    "session-1",
                    EvidenceArtifactKind::RecordSessionJsonl,
                    record_session_path.display().to_string(),
                    "record-session",
                    EvidenceStatus::Success,
                )
                .with_session_id("session-1"),
            )
            .unwrap();

        let lookup = EvidenceIndexReader::new(&index_path)
            .find_by_correlation("session-1")
            .unwrap();
        assert!(matches!(lookup, EvidenceLookup::Entries(_)));
        assert_eq!(
            lookup.entries()[0].artifact_path,
            record_session_path.display().to_string()
        );

        // 直接读取原始 artifact 仍然可行,index 没有复制或替代 record-session 内容。
        let source = std::fs::read_to_string(&record_session_path).unwrap();
        let values: Vec<Value> = source
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(values.len(), 2);
        assert_eq!(values[0]["event"], "_meta.session_start");
        assert_eq!(values[1]["event"], "_meta.termination");
    }

    #[test]
    fn event_log_artifact_can_be_indexed_without_replacing_source() {
        let temp = TempDir::new().unwrap();
        let event_log_path = temp.path().join(".ralph/events.jsonl");
        let index_path = temp.path().join(".ralph/evidence-index.jsonl");

        let mut event_logger = crate::EventLogger::new(&event_log_path);
        event_logger
            .log_event(
                1,
                "replyer",
                &ralph_proto::Event::new("reply.hat.message", "answer")
                    .with_id("reply-1")
                    .with_reply("request-1"),
                None,
            )
            .unwrap();

        let mut writer = EvidenceIndexWriter::new(&index_path);
        writer
            .record(&EvidenceIndexEntry::new(
                "reply-1",
                EvidenceArtifactKind::EventLogJsonl,
                event_log_path.display().to_string(),
                "event-logger",
                EvidenceStatus::Success,
            ))
            .unwrap();

        let lookup = EvidenceIndexReader::new(&index_path)
            .find_by_correlation("reply-1")
            .unwrap();
        assert!(matches!(lookup, EvidenceLookup::Entries(_)));
        assert_eq!(
            lookup.entries()[0].artifact_path,
            event_log_path.display().to_string()
        );

        // 直接通过 EventHistory 读取原始 event log,证明 index 只是 artifact link,不是替代真相源。
        let records = crate::EventHistory::new(&event_log_path)
            .read_all()
            .unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].topic, "reply.hat.message");
        assert_eq!(records[0].id.as_deref(), Some("reply-1"));
        assert_eq!(records[0].reply.as_deref(), Some("request-1"));
    }

    #[test]
    fn runtime_graph_output_is_not_an_index_artifact_kind() {
        let kinds = [
            EvidenceArtifactKind::RecordSessionJsonl,
            EvidenceArtifactKind::EventLogJsonl,
            EvidenceArtifactKind::AgentsSnapshotJson,
            EvidenceArtifactKind::RuntimeDeliveryRecord,
            EvidenceArtifactKind::RuntimeLifecycleRecord,
            EvidenceArtifactKind::ReplyEvent,
            EvidenceArtifactKind::CapabilityInvokeJson,
            EvidenceArtifactKind::CapabilityResultJson,
            EvidenceArtifactKind::CapabilityFailedJson,
            EvidenceArtifactKind::ResolvedConfig,
            EvidenceArtifactKind::MissingArtifact,
        ];

        let serialized: Vec<String> = kinds
            .into_iter()
            .map(|kind| serde_json::to_string(&kind).unwrap())
            .collect();
        assert!(!serialized.iter().any(|kind| kind.contains("rerun")));
        assert!(!serialized.iter().any(|kind| kind.contains("runtime_graph")));
        assert!(!serialized.iter().any(|kind| kind.contains("graph_layout")));
    }

    #[test]
    fn dynamic_role_contract_hash_links_to_source_artifact_without_full_contract() {
        let (_temp, path) = temp_index_path();
        let mut writer = EvidenceIndexWriter::new(&path);

        writer
            .record(&EvidenceIndexEntry::dynamic_role_result_topic(
                "erc-aaaabbbbccccdddd",
                EvidenceArtifactKind::AgentsSnapshotJson,
                ".ralph/agents.json",
                "agents-snapshot",
                "spawn-1",
                "builder#2",
                "analysis.done",
            ))
            .unwrap();

        let lookup = EvidenceIndexReader::new(&path)
            .find_by_correlation("erc-aaaabbbbccccdddd")
            .unwrap();
        let entries = lookup.entries();
        assert!(matches!(lookup, EvidenceLookup::Entries(_)));
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].artifact_kind,
            EvidenceArtifactKind::AgentsSnapshotJson
        );
        assert_eq!(entries[0].artifact_path, ".ralph/agents.json");
        assert_eq!(entries[0].parent_correlation_id.as_deref(), Some("spawn-1"));
        assert_eq!(
            entries[0].child_correlation_id.as_deref(),
            Some("builder#2")
        );
        assert_eq!(entries[0].result_topic.as_deref(), Some("analysis.done"));

        let json = serde_json::to_value(&entries[0]).unwrap();
        let object = json.as_object().unwrap();
        assert!(!object.contains_key("role_name"));
        assert!(!object.contains_key("objective"));
        assert!(!object.contains_key("full_role_contract"));
    }

    #[test]
    fn spawn_request_lookup_lists_spawned_child_instances_and_missing_markers() {
        let (_temp, path) = temp_index_path();
        let mut writer = EvidenceIndexWriter::new(&path);

        for instance_id in ["builder#2", "builder#3"] {
            writer
                .record(&EvidenceIndexEntry::dynamic_spawn_request(
                    "spawn-1",
                    EvidenceArtifactKind::EventLogJsonl,
                    ".ralph/events.jsonl",
                    "parallel.supervisor.topology_spawn",
                    instance_id,
                ))
                .unwrap();
        }
        writer
            .record(&EvidenceIndexEntry::missing_dynamic_result(
                "missing-result:spawn-1:builder#4:analysis.done",
                ".ralph/events.jsonl",
                "record-summary.dynamic-result-coverage",
                "spawn-1",
                "erc-missing-role",
                "analysis.done",
            ))
            .unwrap();

        let lookup = EvidenceIndexReader::new(&path)
            .find_by_correlation("spawn-1")
            .unwrap();
        let spawned_children = lookup
            .entries()
            .iter()
            .filter(|entry| entry.status == EvidenceStatus::Success)
            .filter_map(|entry| entry.child_correlation_id.as_deref())
            .collect::<Vec<_>>();
        let missing_markers = lookup
            .entries()
            .iter()
            .filter(|entry| entry.status == EvidenceStatus::Missing)
            .collect::<Vec<_>>();
        assert!(matches!(lookup, EvidenceLookup::Missing(_)));
        assert_eq!(spawned_children, vec!["builder#2", "builder#3"]);
        assert_eq!(missing_markers.len(), 1);
        assert_eq!(
            missing_markers[0].correlation_id,
            "missing-result:spawn-1:builder#4:analysis.done"
        );
        assert!(
            lookup
                .entries()
                .iter()
                .all(|entry| entry.artifact_path == ".ralph/events.jsonl"),
            "display/summary format changes must not affect stored artifact paths"
        );
    }

    #[test]
    fn missing_dynamic_result_marker_is_distinct_from_no_entry() {
        let (_temp, path) = temp_index_path();
        let mut writer = EvidenceIndexWriter::new(&path);

        writer
            .record(&EvidenceIndexEntry::missing_dynamic_result(
                "missing-result:spawn-1:builder#2:analysis.done",
                ".ralph/events.jsonl",
                "record-summary.dynamic-result-coverage",
                "spawn-1",
                "erc-aaaabbbbccccdddd",
                "analysis.done",
            ))
            .unwrap();

        let lookup = EvidenceIndexReader::new(&path)
            .find_by_correlation("missing-result:spawn-1:builder#2:analysis.done")
            .unwrap();
        let entries = lookup.entries();
        assert!(matches!(lookup, EvidenceLookup::Missing(_)));
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].status, EvidenceStatus::Missing);
        assert_eq!(
            entries[0].artifact_kind,
            EvidenceArtifactKind::MissingArtifact
        );
        assert_eq!(entries[0].parent_correlation_id.as_deref(), Some("spawn-1"));
        assert_eq!(
            entries[0].child_correlation_id.as_deref(),
            Some("erc-aaaabbbbccccdddd")
        );
        assert_eq!(entries[0].result_topic.as_deref(), Some("analysis.done"));
        assert!(matches!(
            EvidenceIndexReader::new(&path)
                .find_by_correlation("missing-result:not-written")
                .unwrap(),
            EvidenceLookup::NoEntry
        ));
    }

    #[test]
    fn terminal_failed_dynamic_result_is_distinct_from_missing_marker() {
        let (_temp, path) = temp_index_path();
        let mut writer = EvidenceIndexWriter::new(&path);

        writer
            .record(
                &EvidenceIndexEntry::new(
                    "failed-result:spawn-1:builder#2:analysis.done",
                    EvidenceArtifactKind::EventLogJsonl,
                    ".ralph/events.jsonl",
                    "parallel.supervisor.topology_spawn",
                    EvidenceStatus::Failure,
                )
                .with_parent_correlation_id("spawn-1")
                .with_child_correlation_id("erc-failed-role")
                .with_result_topic("analysis.done"),
            )
            .unwrap();

        let lookup = EvidenceIndexReader::new(&path)
            .find_by_correlation("failed-result:spawn-1:builder#2:analysis.done")
            .unwrap();
        let entries = lookup.entries();
        assert!(matches!(lookup, EvidenceLookup::Entries(_)));
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].status, EvidenceStatus::Failure);
        assert_eq!(
            entries[0].artifact_kind,
            EvidenceArtifactKind::EventLogJsonl
        );
        assert_eq!(entries[0].result_topic.as_deref(), Some("analysis.done"));
    }
}
