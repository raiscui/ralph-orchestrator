//! Event logging for debugging and post-mortem analysis.
//!
//! Logs all events to `.ralph/events.jsonl` as specified in the event-loop spec.
//! The observer pattern allows hooking into the event bus without modifying routing.

use crate::TOPIC_CAPABILITY_INVOKE;
use crate::{TOPIC_CAPABILITY_FAILED, TOPIC_CAPABILITY_REQUEST, TOPIC_CAPABILITY_RESULT};
use ralph_proto::{
    Event, HatId, QueueDecisionRecord, RuntimeDeliveryRecord, RuntimeLifecycleRecord,
    TOPIC_DISPATCH_DECISION, TOPIC_RUNTIME_DELIVERY, TOPIC_RUNTIME_LIFECYCLE,
};
use serde::{Deserialize, Deserializer, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

/// Custom deserializer that accepts both String and structured JSON payloads.
///
/// Agents sometimes write structured data as JSON objects instead of strings.
/// This deserializer accepts both formats:
/// - `"payload": "string"` → `"string"`
/// - `"payload": {...}` → `"{...}"` (serialized to JSON string)
/// - `"payload": null` or missing → `""` (empty string)
fn deserialize_flexible_payload<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum FlexiblePayload {
        String(String),
        Object(serde_json::Value),
    }

    let opt = Option::<FlexiblePayload>::deserialize(deserializer)?;
    Ok(opt
        .map(|flex| match flex {
            FlexiblePayload::String(s) => s,
            FlexiblePayload::Object(serde_json::Value::Null) => String::new(),
            FlexiblePayload::Object(obj) => {
                // Serialize the object back to a JSON string
                serde_json::to_string(&obj).unwrap_or_else(|_| obj.to_string())
            }
        })
        .unwrap_or_default())
}

/// A logged event record for debugging.
///
/// Supports two schemas:
/// 1. Rich internal format (logged by Ralph):
///    `{"ts":"2024-01-15T10:23:45Z","iteration":1,"hat":"loop","topic":"task.start","triggered":"planner","payload":"..."}`
/// 2. Simple agent format (written by agents):
///    `{"topic":"build.task","payload":"...","ts":"2024-01-15T10:24:12Z"}`
///
/// Fields that don't exist in the agent format default to sensible values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecord {
    /// ISO 8601 timestamp.
    pub ts: String,

    /// Loop iteration number (0 if not provided by agent-written events).
    #[serde(default)]
    pub iteration: u32,

    /// Hat that was active when event was published (empty string if not provided).
    #[serde(default)]
    pub hat: String,

    /// Hat instance that published this event (if available).
    ///
    /// 说明：
    /// - 并行 HatInstance 模式下，这个字段用于把事件与具体实例关联起来。
    /// - 串行/历史事件可能没有该字段，因此保持可选。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_instance: Option<String>,

    /// Optional stable identifier for this event.
    ///
    /// 说明：
    /// - 该字段用于把"同一条事件"在不同层面(路由、日志、prompt)关联起来。
    /// - 旧 events.jsonl 可能不存在该字段，因此保持可选并提供默认值。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Optional in-reply-to event id (single value).
    ///
    /// 说明：
    /// - 用于把本条事件与"被回复的事件"建立关联（in-reply-to）。
    /// - 该字段为单值：一次只回复一条 event.id。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply: Option<String>,

    /// Event topic.
    pub topic: String,

    /// Hat that will be triggered by this event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub triggered: Option<String>,

    /// Event content (truncated if large). Defaults to empty string for agent events without payload.
    /// Accepts both string and object payloads - objects are serialized to JSON strings.
    #[serde(default, deserialize_with = "deserialize_flexible_payload")]
    pub payload: String,

    /// How many times this task has blocked (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_count: Option<u32>,
}

impl EventRecord {
    /// Maximum payload length before truncation.
    const MAX_PAYLOAD_LEN: usize = 500;

    /// 是否允许截断该 topic 的 payload。
    ///
    /// 说明：
    /// - 大多数事件 payload 只是“人类可读摘要”，截断能显著降低 events.jsonl 体积。
    /// - 但少数事件 payload 是“replay 需要的结构化数据”（例如 queue 决策记录），
    ///   一旦截断就可能导致 JSON 不可解析，从而破坏回放确定性。
    fn should_truncate_payload(topic: &str) -> bool {
        !matches!(
            topic,
            TOPIC_DISPATCH_DECISION
                | TOPIC_RUNTIME_DELIVERY
                | TOPIC_RUNTIME_LIFECYCLE
                | TOPIC_CAPABILITY_INVOKE
                | TOPIC_CAPABILITY_REQUEST
                | TOPIC_CAPABILITY_RESULT
                | TOPIC_CAPABILITY_FAILED
        )
    }

    /// Creates a new event record.
    pub fn new(
        iteration: u32,
        hat: impl Into<String>,
        event: &Event,
        triggered: Option<&HatId>,
    ) -> Self {
        let payload = if Self::should_truncate_payload(event.topic.as_str())
            && event.payload.len() > Self::MAX_PAYLOAD_LEN
        {
            // Find a valid UTF-8 char boundary at or before MAX_PAYLOAD_LEN.
            // We walk backwards from the limit until we find a char boundary.
            let mut truncate_at = Self::MAX_PAYLOAD_LEN;
            while truncate_at > 0 && !event.payload.is_char_boundary(truncate_at) {
                truncate_at -= 1;
            }
            format!(
                "{}... [truncated, {} chars total]",
                &event.payload[..truncate_at],
                event.payload.chars().count()
            )
        } else {
            event.payload.clone()
        };

        Self {
            ts: chrono::Utc::now().to_rfc3339(),
            iteration,
            hat: hat.into(),
            source_instance: event.source_instance.as_ref().map(|id| id.to_string()),
            id: event.id.clone(),
            reply: event.reply.clone(),
            topic: event.topic.to_string(),
            triggered: triggered.map(|h| h.to_string()),
            payload,
            blocked_count: None,
        }
    }

    /// Sets the blocked count for this record.
    pub fn with_blocked_count(mut self, count: u32) -> Self {
        self.blocked_count = Some(count);
        self
    }
}

/// Logger that writes events to a JSONL file.
pub struct EventLogger {
    /// Path to the events file.
    path: PathBuf,

    /// File handle for appending.
    file: Option<File>,
}

impl EventLogger {
    /// Default path for the events file.
    pub const DEFAULT_PATH: &'static str = ".ralph/events.jsonl";

    /// Creates a new event logger.
    ///
    /// The `.ralph/` directory is created if it doesn't exist.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            file: None,
        }
    }

    /// Creates a logger with the default path.
    pub fn default_path() -> Self {
        Self::new(Self::DEFAULT_PATH)
    }

    /// Ensures the parent directory exists and opens the file.
    fn ensure_open(&mut self) -> std::io::Result<&mut File> {
        if self.file.is_none() {
            if let Some(parent) = self.path.parent() {
                fs::create_dir_all(parent)?;
            }
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)?;
            self.file = Some(file);
        }
        Ok(self.file.as_mut().unwrap())
    }

    /// Logs an event record.
    pub fn log(&mut self, record: &EventRecord) -> std::io::Result<()> {
        let file = self.ensure_open()?;

        // ---------------------------------------------------------------------
        // 说明：events.jsonl 需要“尽量原子”的按行追加写入
        //
        // 背景：
        // - JSONL 的基本不变量是：每一行都是一条完整 JSON。
        // - 如果在写入过程中进程被打断（崩溃/强杀/断电），就可能出现“半行 JSON”，
        //   这会导致 `ralph events` 或 replay 读取时解析失败。
        //
        // 策略：
        // - 先把整行内容组装成一个 String（JSON + '\n'），再一次性写入。
        // - 这样至少可以避免 `writeln!` 在格式化过程中产生多次写入导致的行破损概率。
        // ---------------------------------------------------------------------
        let mut json_line = serde_json::to_string(record)?;
        json_line.push('\n');
        file.write_all(json_line.as_bytes())?;
        file.flush()?;
        debug!(topic = %record.topic, iteration = record.iteration, "Event logged");
        Ok(())
    }

    /// Convenience method to log an event directly.
    pub fn log_event(
        &mut self,
        iteration: u32,
        hat: &str,
        event: &Event,
        triggered: Option<&HatId>,
    ) -> std::io::Result<()> {
        let record = EventRecord::new(iteration, hat, event, triggered);
        self.log(&record)
    }

    /// 记录一次 queue 路由决策（用于 replay，不允许截断）。
    ///
    /// 说明：
    /// - 这是一种“observer-only”记录事件：topic 固定为 `dispatch.decision`
    /// - payload 为 `QueueDecisionRecord` 的 JSON 字符串
    pub fn log_queue_decision(
        &mut self,
        iteration: u32,
        hat: &str,
        decision: &QueueDecisionRecord,
    ) -> std::io::Result<()> {
        let payload = serde_json::to_string(decision).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to serialize QueueDecisionRecord: {e}"),
            )
        })?;

        let event = Event::new(TOPIC_DISPATCH_DECISION, payload);
        self.log_event(iteration, hat, &event, None)
    }

    /// 记录一次真实 runtime delivery（用于 V2 durable replay graph，不允许截断）。
    pub fn log_runtime_delivery(
        &mut self,
        iteration: u32,
        hat: &str,
        delivery: &RuntimeDeliveryRecord,
    ) -> std::io::Result<()> {
        let payload = serde_json::to_string(delivery).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to serialize RuntimeDeliveryRecord: {e}"),
            )
        })?;

        let event = Event::new(TOPIC_RUNTIME_DELIVERY, payload);
        self.log_event(iteration, hat, &event, None)
    }

    /// 记录一次 runtime lifecycle / control 动作（用于 V2 durable replay graph，不允许截断）。
    pub fn log_runtime_lifecycle(
        &mut self,
        iteration: u32,
        hat: &str,
        lifecycle: &RuntimeLifecycleRecord,
    ) -> std::io::Result<()> {
        let payload = serde_json::to_string(lifecycle).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to serialize RuntimeLifecycleRecord: {e}"),
            )
        })?;

        let event = Event::new(TOPIC_RUNTIME_LIFECYCLE, payload);
        self.log_event(iteration, hat, &event, None)
    }

    /// Returns the path to the log file.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Reader for event history files.
pub struct EventHistory {
    path: PathBuf,
}

impl EventHistory {
    /// Creates a new history reader.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Creates a reader for the default path.
    pub fn default_path() -> Self {
        Self::new(EventLogger::DEFAULT_PATH)
    }

    /// Returns true if the history file exists.
    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    /// Reads all event records from the file.
    pub fn read_all(&self) -> std::io::Result<Vec<EventRecord>> {
        if !self.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);
        let mut records = Vec::new();

        for (line_num, line) in reader.lines().enumerate() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str(&line) {
                Ok(record) => records.push(record),
                Err(e) => {
                    warn!(line = line_num + 1, error = %e, "Failed to parse event record");
                }
            }
        }

        Ok(records)
    }

    /// Reads the last N event records.
    pub fn read_last(&self, n: usize) -> std::io::Result<Vec<EventRecord>> {
        let all = self.read_all()?;
        let start = all.len().saturating_sub(n);
        Ok(all[start..].to_vec())
    }

    /// Reads events filtered by topic.
    pub fn filter_by_topic(&self, topic: &str) -> std::io::Result<Vec<EventRecord>> {
        let all = self.read_all()?;
        Ok(all.into_iter().filter(|r| r.topic == topic).collect())
    }

    /// Reads events filtered by iteration.
    pub fn filter_by_iteration(&self, iteration: u32) -> std::io::Result<Vec<EventRecord>> {
        let all = self.read_all()?;
        Ok(all
            .into_iter()
            .filter(|r| r.iteration == iteration)
            .collect())
    }

    /// Clears the event history file.
    pub fn clear(&self) -> std::io::Result<()> {
        if self.exists() {
            fs::remove_file(&self.path)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ralph_proto::{HatInstanceId, HatInstanceState, RuntimeDeliveryKind, RuntimeLifecycleKind};
    use tempfile::TempDir;

    fn make_event(topic: &str, payload: &str) -> Event {
        Event::new(topic, payload)
    }

    #[test]
    fn test_log_and_read() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("events.jsonl");

        let mut logger = EventLogger::new(&path);

        // Log some events
        let event1 = make_event("task.start", "Starting task");
        let event2 = make_event("build.done", "Build complete");

        logger
            .log_event(1, "loop", &event1, Some(&HatId::new("planner")))
            .unwrap();
        logger
            .log_event(2, "builder", &event2, Some(&HatId::new("planner")))
            .unwrap();

        // Read them back
        let history = EventHistory::new(&path);
        let records = history.read_all().unwrap();

        assert_eq!(records.len(), 2);
        assert_eq!(records[0].topic, "task.start");
        assert_eq!(records[0].iteration, 1);
        assert_eq!(records[0].hat, "loop");
        assert_eq!(records[0].triggered, Some("planner".to_string()));
        assert_eq!(records[1].topic, "build.done");
    }

    #[test]
    fn test_read_last() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("events.jsonl");

        let mut logger = EventLogger::new(&path);

        for i in 1..=10 {
            let event = make_event("test", &format!("Event {}", i));
            logger.log_event(i, "hat", &event, None).unwrap();
        }

        let history = EventHistory::new(&path);
        let last_3 = history.read_last(3).unwrap();

        assert_eq!(last_3.len(), 3);
        assert_eq!(last_3[0].iteration, 8);
        assert_eq!(last_3[2].iteration, 10);
    }

    #[test]
    fn test_filter_by_topic() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("events.jsonl");

        let mut logger = EventLogger::new(&path);

        logger
            .log_event(1, "hat", &make_event("build.done", "a"), None)
            .unwrap();
        logger
            .log_event(2, "hat", &make_event("build.blocked", "b"), None)
            .unwrap();
        logger
            .log_event(3, "hat", &make_event("build.done", "c"), None)
            .unwrap();

        let history = EventHistory::new(&path);
        let blocked = history.filter_by_topic("build.blocked").unwrap();

        assert_eq!(blocked.len(), 1);
        assert_eq!(blocked[0].iteration, 2);
    }

    #[test]
    fn test_payload_truncation() {
        let long_payload = "x".repeat(1000);
        let event = make_event("test", &long_payload);
        let record = EventRecord::new(1, "hat", &event, None);

        assert!(record.payload.len() < 1000);
        assert!(record.payload.contains("[truncated"));
    }

    #[test]
    fn test_payload_truncation_with_multibyte_chars() {
        // Create a payload with multi-byte UTF-8 characters (✅ is 3 bytes)
        // Place emoji near the truncation boundary to trigger the bug
        let mut payload = "x".repeat(498);
        payload.push_str("✅✅✅"); // 3 emojis at bytes 498-506
        payload.push_str(&"y".repeat(500));

        let event = make_event("test", &payload);
        // This should NOT panic
        let record = EventRecord::new(1, "hat", &event, None);

        assert!(record.payload.contains("[truncated"));
        // Verify the payload is valid UTF-8 (would panic on iteration if not)
        for _ in record.payload.chars() {}
    }

    #[test]
    fn test_event_record_preserves_id_and_reply() {
        let event = make_event("reply.hat.message", "answer")
            .with_id("evt-answer-1")
            .with_reply("evt-request-1");
        let record = EventRecord::new(1, "responder", &event, None);

        assert_eq!(record.id.as_deref(), Some("evt-answer-1"));
        assert_eq!(record.reply.as_deref(), Some("evt-request-1"));

        let json = serde_json::to_string(&record).unwrap();
        let parsed: EventRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id.as_deref(), Some("evt-answer-1"));
        assert_eq!(parsed.reply.as_deref(), Some("evt-request-1"));
    }

    #[test]
    fn test_creates_parent_directory() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("nested/dir/events.jsonl");

        let mut logger = EventLogger::new(&path);
        let event = make_event("test", "payload");
        logger.log_event(1, "hat", &event, None).unwrap();

        assert!(path.exists());
    }

    #[test]
    fn test_empty_history() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("nonexistent.jsonl");

        let history = EventHistory::new(&path);
        assert!(!history.exists());

        let records = history.read_all().unwrap();
        assert!(records.is_empty());
    }

    #[test]
    fn test_agent_written_events_without_iteration() {
        // Agent events use simple format: {"topic":"...","payload":"...","ts":"..."}
        // They don't include iteration, hat, or triggered fields
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("events.jsonl");

        // Write agent-style events (without iteration field)
        let mut file = File::create(&path).unwrap();
        writeln!(
            file,
            r#"{{"topic":"build.task","payload":"Implement auth","ts":"2024-01-15T10:00:00Z"}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"topic":"build.done","ts":"2024-01-15T10:30:00Z"}}"#
        )
        .unwrap();

        // Should read without warnings (iteration defaults to 0)
        let history = EventHistory::new(&path);
        let records = history.read_all().unwrap();

        assert_eq!(records.len(), 2);
        assert_eq!(records[0].topic, "build.task");
        assert_eq!(records[0].payload, "Implement auth");
        assert_eq!(records[0].iteration, 0); // Defaults to 0
        assert_eq!(records[0].hat, ""); // Defaults to empty string
        assert_eq!(records[1].topic, "build.done");
        assert_eq!(records[1].payload, ""); // Defaults to empty when not provided
    }

    #[test]
    fn test_mixed_event_formats() {
        // Test that both agent-written and Ralph-logged events can coexist
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("events.jsonl");

        // Write a Ralph-logged event (full format)
        let mut logger = EventLogger::new(&path);
        let event = make_event("task.start", "Initial task");
        logger
            .log_event(1, "loop", &event, Some(&HatId::new("planner")))
            .unwrap();

        // Write an agent-style event (simple format)
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        writeln!(
            file,
            r#"{{"topic":"build.task","payload":"Agent wrote this","ts":"2024-01-15T10:05:00Z"}}"#
        )
        .unwrap();

        // Should read both without warnings
        let history = EventHistory::new(&path);
        let records = history.read_all().unwrap();

        assert_eq!(records.len(), 2);
        // First is Ralph's full-format event
        assert_eq!(records[0].topic, "task.start");
        assert_eq!(records[0].iteration, 1);
        assert_eq!(records[0].hat, "loop");
        // Second is agent's simple format
        assert_eq!(records[1].topic, "build.task");
        assert_eq!(records[1].iteration, 0); // Defaulted
        assert_eq!(records[1].hat, ""); // Defaulted
    }

    #[test]
    fn test_object_payload_from_ralph_emit_json() {
        // Test that `ralph emit --json` object payloads are parsed correctly
        // This was the root cause of "invalid type: map, expected a string" errors
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("events.jsonl");

        let mut file = File::create(&path).unwrap();

        // String payload (normal case)
        writeln!(
            file,
            r#"{{"ts":"2024-01-15T10:00:00Z","topic":"task.start","payload":"implement feature"}}"#
        )
        .unwrap();

        // Object payload (from `ralph emit --json`)
        writeln!(
            file,
            r#"{{"topic":"task.complete","payload":{{"status":"verified","tasks":["auth","api"]}},"ts":"2024-01-15T10:30:00Z"}}"#
        )
        .unwrap();

        // Nested object payload
        writeln!(
            file,
            r#"{{"topic":"loop.recovery","payload":{{"status":"recovered","evidence":{{"tests":"pass"}}}},"ts":"2024-01-15T10:45:00Z"}}"#
        )
        .unwrap();

        let history = EventHistory::new(&path);
        let records = history.read_all().unwrap();

        assert_eq!(records.len(), 3);

        // String payload unchanged
        assert_eq!(records[0].topic, "task.start");
        assert_eq!(records[0].payload, "implement feature");

        // Object payload converted to JSON string
        assert_eq!(records[1].topic, "task.complete");
        assert!(records[1].payload.contains("\"status\""));
        assert!(records[1].payload.contains("\"verified\""));
        // Should be valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&records[1].payload).unwrap();
        assert_eq!(parsed["status"], "verified");

        // Nested object also works
        assert_eq!(records[2].topic, "loop.recovery");
        let parsed: serde_json::Value = serde_json::from_str(&records[2].payload).unwrap();
        assert_eq!(parsed["evidence"]["tests"], "pass");
    }

    #[test]
    fn test_queue_decision_payload_is_not_truncated() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("events.jsonl");

        let mut logger = EventLogger::new(&path);

        // 构造一个足够长的候选集，确保序列化后的 JSON 明显超过截断阈值。
        let candidates: Vec<HatInstanceId> = (0..200)
            .map(|i| HatInstanceId::from(format!("writer#{i}")))
            .collect();

        let decision = QueueDecisionRecord::new(
            "evt-1",
            candidates.clone(),
            candidates[0].clone(),
            Some("prefer least-busy".to_string()),
        );

        logger
            .log_queue_decision(1, "router", &decision)
            .expect("Should log queue decision record");

        let history = EventHistory::new(&path);
        let records = history.read_all().unwrap();

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].topic, TOPIC_DISPATCH_DECISION);

        // 不允许被 EventRecord 的默认截断逻辑砍掉。
        assert!(
            records[0].payload.len() > EventRecord::MAX_PAYLOAD_LEN,
            "Queue decision payload should not be truncated (len={})",
            records[0].payload.len()
        );

        // payload 必须是可解析的 JSON（保证 replay 可读）。
        let parsed: QueueDecisionRecord =
            serde_json::from_str(&records[0].payload).expect("Payload should be valid JSON");
        assert_eq!(parsed.chosen_instance, candidates[0]);
        assert_eq!(parsed.candidates.len(), candidates.len());
    }

    #[test]
    fn test_runtime_durable_payloads_are_not_truncated() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("events.jsonl");

        let mut logger = EventLogger::new(&path);
        let long_topic = format!("build.{}", "very_long_topic_".repeat(80));
        let delivery = RuntimeDeliveryRecord::new(
            Some("event-runtime-delivery".to_string()),
            None,
            long_topic,
            Some(HatInstanceId::new("planner#1")),
            HatInstanceId::new("writer#1"),
            RuntimeDeliveryKind::Fanout,
        );
        let lifecycle = RuntimeLifecycleRecord::new(
            HatInstanceId::new("writer#2"),
            RuntimeLifecycleKind::Spawn,
        )
        .with_state(HatInstanceState::Created)
        .with_dynamic(true)
        .with_source_event_id("event-runtime-delivery")
        .with_reason("explicit_spawn_instance");

        logger
            .log_runtime_delivery(0, "supervisor", &delivery)
            .unwrap();
        logger
            .log_runtime_lifecycle(0, "supervisor", &lifecycle)
            .unwrap();

        let history = EventHistory::new(&path);
        let records = history.read_all().unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].topic, TOPIC_RUNTIME_DELIVERY);
        assert_eq!(records[1].topic, TOPIC_RUNTIME_LIFECYCLE);
        assert!(!records[0].payload.contains("[truncated"));
        assert!(!records[1].payload.contains("[truncated"));

        let parsed_delivery: RuntimeDeliveryRecord =
            serde_json::from_str(&records[0].payload).unwrap();
        let parsed_lifecycle: RuntimeLifecycleRecord =
            serde_json::from_str(&records[1].payload).unwrap();
        assert_eq!(parsed_delivery.recipient, HatInstanceId::new("writer#1"));
        assert_eq!(parsed_delivery.mode, RuntimeDeliveryKind::Fanout);
        assert_eq!(parsed_lifecycle.instance_id, HatInstanceId::new("writer#2"));
        assert_eq!(parsed_lifecycle.kind, RuntimeLifecycleKind::Spawn);
        assert_eq!(parsed_lifecycle.state, Some(HatInstanceState::Created));
    }
}
