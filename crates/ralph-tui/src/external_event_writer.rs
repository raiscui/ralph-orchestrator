//! 外部事件写入器（给并行 Supervisor 注入 human 输入）。
//!
//! 设计目标：
//! - 不 spawn 子进程（例如 `ralph emit`），避免 TUI 输入卡顿与依赖复杂化。
//! - 直接追加写入 JSONL，格式对齐 `ralph-core::EventReader` 的解析结构。
//! - 路径解析遵循并行 Supervisor 的约定：优先读取 `.ralph/current-events` marker。

use anyhow::{Context, Result};
use chrono::Utc;
use ralph_core::{Event as JsonlEvent, EventLogger};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// 将 human 的输入追加写入外部事件 JSONL。
#[derive(Debug, Clone)]
pub struct ExternalEventWriter {
    path: PathBuf,
}

impl ExternalEventWriter {
    /// 解析当前 run 的外部事件 JSONL 路径（以当前工作目录为根）。
    pub fn resolve_events_path() -> PathBuf {
        Self::resolve_events_path_in(Path::new("."))
    }

    /// 解析当前 run 的外部事件 JSONL 路径（以指定 root 为根）。
    ///
    /// 说明：
    /// - `.ralph/current-events` 的内容是“外部事件 JSONL 的路径”（一行文本）。
    /// - 若 marker 内容是相对路径，则相对 `root` 解析。
    /// - marker 不存在时，回退到 `root/.ralph/events.jsonl`（与 Supervisor 保持一致）。
    pub fn resolve_events_path_in(root: &Path) -> PathBuf {
        let marker = root.join(".ralph/current-events");
        let from_marker = fs::read_to_string(&marker).ok().and_then(|s| {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                return None;
            }
            let p = PathBuf::from(trimmed);
            Some(if p.is_absolute() { p } else { root.join(p) })
        });

        from_marker.unwrap_or_else(|| root.join(EventLogger::DEFAULT_PATH))
    }

    /// 创建 writer（best-effort：不要求目标文件已存在）。
    pub fn new() -> Self {
        Self {
            path: Self::resolve_events_path(),
        }
    }

    /// 直接指定外部事件 JSONL 的路径（测试/特殊场景使用）。
    #[cfg(test)]
    pub fn for_path(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 追加写入一个外部事件（JSONL 一行）。
    pub fn append_event(&self, event: &JsonlEvent) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "Failed to create parent directory for external events file: {}",
                    parent.display()
                )
            })?;
        }

        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .with_context(|| {
                format!(
                    "Failed to open external events file for append: {}",
                    self.path.display()
                )
            })?;

        let json = serde_json::to_string(event).context("Failed to serialize external event")?;
        file.write_all(json.as_bytes())
            .context("Failed to write external event line")?;
        file.write_all(b"\n")
            .context("Failed to write external event newline")?;
        file.flush().ok(); // best-effort

        Ok(())
    }

    /// 便捷方法：写入一个带 payload 的外部事件。
    pub fn append(
        &self,
        topic: &str,
        payload: String,
        target_instance: Option<String>,
    ) -> Result<()> {
        let event = JsonlEvent {
            topic: topic.to_string(),
            payload: Some(payload),
            ts: Utc::now().to_rfc3339(),
            target_instance,
            workspace_strategy: None,
        };
        self.append_event(&event)
    }
}

impl Default for ExternalEventWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ralph_core::Event as JsonlEvent;
    use std::fs;

    #[test]
    fn resolve_events_path_in_prefers_marker() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join(".ralph")).unwrap();
        fs::write(root.join(".ralph/current-events"), "custom.jsonl\n").unwrap();

        let resolved = ExternalEventWriter::resolve_events_path_in(root);
        assert_eq!(resolved, root.join("custom.jsonl"));
    }

    #[test]
    fn resolve_events_path_in_falls_back_to_default() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        // marker 不存在
        let resolved = ExternalEventWriter::resolve_events_path_in(root);
        assert_eq!(resolved, root.join(EventLogger::DEFAULT_PATH));
    }

    #[test]
    fn append_writes_valid_jsonl_event() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");

        let writer = ExternalEventWriter::for_path(&path);
        writer
            .append(
                "human.message",
                "hello".to_string(),
                Some("writer#1".to_string()),
            )
            .unwrap();

        let content = fs::read_to_string(&path).unwrap();
        let line = content.lines().next().unwrap();
        let parsed: JsonlEvent = serde_json::from_str(line).unwrap();

        assert_eq!(parsed.topic, "human.message");
        assert_eq!(parsed.payload, Some("hello".to_string()));
        assert_eq!(parsed.target_instance, Some("writer#1".to_string()));
        assert!(
            parsed.ts.len() >= 10,
            "ts should be non-empty rfc3339 string"
        );
    }
}
