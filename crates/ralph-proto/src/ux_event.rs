//! UX event types for terminal and TUI capture.
//!
//! These events enable recording and replaying terminal output with timing
//! and color information preserved. The design follows the observer pattern:
//! events are captured during execution and can be replayed later.

use serde::{Deserialize, Serialize};

/// A UX event captured during session recording.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum UxEvent {
    /// Raw bytes written to terminal (stdout/stderr).
    #[serde(rename = "ux.terminal.write")]
    TerminalWrite(TerminalWrite),

    /// Terminal resize event.
    #[serde(rename = "ux.terminal.resize")]
    TerminalResize(TerminalResize),

    /// Color mode detection result.
    #[serde(rename = "ux.terminal.color_mode")]
    TerminalColorMode(TerminalColorMode),

    /// TUI frame capture (reserved for future ralph-tui integration).
    #[serde(rename = "ux.tui.frame")]
    TuiFrame(TuiFrame),
}

/// Raw bytes written to stdout or stderr.
///
/// Bytes are stored as base64 to preserve ANSI escape sequences
/// and binary data without JSON escaping issues.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalWrite {
    /// Base64-encoded raw bytes.
    pub bytes: String,

    /// 可读文本(用于诊断).
    ///
    /// 说明:
    /// - 这是对原始 bytes 的 UTF-8 lossy 视图,便于直接阅读 JSONL.
    /// - 回放与事件解析仍应以 `bytes` 为准,不要依赖该字段做字节级语义.
    /// - 旧 cassette 可能没有该字段,因此保持为可选.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    /// True for stdout, false for stderr.
    pub stdout: bool,

    /// Milliseconds since session start.
    pub offset_ms: u64,

    /// 可选：输出归因到具体 HatInstance（并行模式回放需要）。
    ///
    /// 说明：
    /// - 串行模式下通常为 None
    /// - 并行模式下建议为 Some("writer#1") 这类值
    /// - 作为“UX 录制”的扩展字段，不影响旧 cassette 的解析
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
}

impl TerminalWrite {
    /// Creates a new terminal write event.
    pub fn new(raw_bytes: &[u8], stdout: bool, offset_ms: u64) -> Self {
        use base64::Engine;
        Self {
            bytes: base64::engine::general_purpose::STANDARD.encode(raw_bytes),
            text: Some(String::from_utf8_lossy(raw_bytes).into_owned()),
            stdout,
            offset_ms,
            instance_id: None,
        }
    }

    /// 为 TerminalWrite 设置 instance_id（用于并行回放分流）。
    #[must_use]
    pub fn with_instance_id(mut self, instance_id: impl Into<String>) -> Self {
        self.instance_id = Some(instance_id.into());
        self
    }

    /// Decodes the base64 bytes back to raw bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if the base64 data is malformed.
    pub fn decode_bytes(&self) -> Result<Vec<u8>, base64::DecodeError> {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.decode(&self.bytes)
    }
}

/// Terminal dimension change event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalResize {
    /// Terminal width in columns.
    pub width: u16,

    /// Terminal height in rows.
    pub height: u16,

    /// Milliseconds since session start.
    pub offset_ms: u64,
}

impl TerminalResize {
    /// Creates a new resize event.
    pub fn new(width: u16, height: u16, offset_ms: u64) -> Self {
        Self {
            width,
            height,
            offset_ms,
        }
    }
}

/// Color mode detection result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalColorMode {
    /// Requested color mode (auto, always, never).
    pub mode: String,

    /// Actual detected mode after auto-detection.
    pub detected: String,

    /// Milliseconds since session start.
    pub offset_ms: u64,
}

impl TerminalColorMode {
    /// Creates a new color mode event.
    pub fn new(mode: impl Into<String>, detected: impl Into<String>, offset_ms: u64) -> Self {
        Self {
            mode: mode.into(),
            detected: detected.into(),
            offset_ms,
        }
    }
}

/// TUI frame capture for future ralph-tui integration.
///
/// This is a placeholder for when TUI support is implemented.
/// Frame buffers will be captured from ratatui's `TestBackend`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuiFrame {
    /// Sequential frame identifier.
    pub frame_id: u64,

    /// Frame width in columns.
    pub width: u16,

    /// Frame height in rows.
    pub height: u16,

    /// Serialized cell buffer (format TBD when TUI is implemented).
    pub cells: String,

    /// Milliseconds since session start.
    pub offset_ms: u64,
}

impl TuiFrame {
    /// Creates a new TUI frame event.
    pub fn new(frame_id: u64, width: u16, height: u16, cells: String, offset_ms: u64) -> Self {
        Self {
            frame_id,
            width,
            height,
            cells,
            offset_ms,
        }
    }
}

/// Abstract interface for capturing rendered output.
///
/// Implementations capture frames from either CLI mode (terminal bytes)
/// or TUI mode (ratatui frame buffers). Both produce the same `UxEvent`
/// format for unified replay and export.
pub trait FrameCapture: Send + Sync {
    /// Returns captured events and clears the internal buffer.
    fn take_captures(&mut self) -> Vec<UxEvent>;

    /// Returns true if any events have been captured.
    fn has_captures(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_write_roundtrip() {
        let original = b"Hello, \x1b[32mWorld\x1b[0m!";
        let write = TerminalWrite::new(original, true, 100);

        assert!(write.stdout);
        assert_eq!(write.offset_ms, 100);

        let decoded = write.decode_bytes().unwrap();
        assert_eq!(decoded, original);

        // 诊断字段: 便于直接阅读 JSONL,不影响回放字节保真.
        let expected = String::from_utf8_lossy(original);
        assert_eq!(write.text.as_deref(), Some(expected.as_ref()));
    }

    #[test]
    fn test_terminal_write_backward_compat_without_text() {
        // 旧 cassette 里没有 text 字段,必须仍然可解析.
        let json = r#"{"bytes":"SGVsbG8=","stdout":true,"offset_ms":0}"#;
        let write: TerminalWrite = serde_json::from_str(json).unwrap();
        assert!(write.text.is_none());
        assert_eq!(write.decode_bytes().unwrap(), b"Hello");
    }

    #[test]
    fn test_ux_event_serialization() {
        let event = UxEvent::TerminalWrite(TerminalWrite::new(b"test", true, 0));
        let json = serde_json::to_string(&event).unwrap();

        assert!(json.contains("ux.terminal.write"));
        assert!(json.contains("\"text\""));

        let parsed: UxEvent = serde_json::from_str(&json).unwrap();
        if let UxEvent::TerminalWrite(write) = parsed {
            assert!(write.stdout);
        } else {
            panic!("Expected TerminalWrite variant");
        }
    }

    #[test]
    fn test_terminal_resize_serialization() {
        let event = UxEvent::TerminalResize(TerminalResize::new(120, 30, 500));
        let json = serde_json::to_string(&event).unwrap();

        assert!(json.contains("ux.terminal.resize"));
        assert!(json.contains("120"));
        assert!(json.contains("30"));
    }
}
