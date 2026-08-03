//! # ralph-display
//!
//! 终端展示模块:把"后端进程输出"变成"用户可见的展示"。
//!
//! - `StreamHandler` trait 是进程输出 → 展示的 seam(适配层只依赖这个 trait)
//! - 4 种展示实现:控制台 / 静默 / TUI / Markdown 美化
//! - `DisplayTarget` + `make_stream_handler` 是窄入口,调用者只表达"我要什么"

pub mod colors;
pub mod stream_handler;

/// TUI 共享行缓冲(展示层类型真相源)。
pub type TuiLineBuffer = std::sync::Arc<std::sync::Mutex<Vec<ratatui::text::Line<'static>>>>;

pub use stream_handler::{
    ConsoleStreamHandler, DisplayTarget, DisplayVerbosity, MarkdownRenderMode,
    PrettyStreamHandler, QuietStreamHandler, SessionResult, StreamHandler, TuiStreamHandler,
    make_stream_handler, render_text_to_lines,
};
