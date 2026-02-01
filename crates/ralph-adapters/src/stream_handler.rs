//! Stream handler trait and implementations for processing Claude stream events.
//!
//! The `StreamHandler` trait abstracts over how stream events are displayed,
//! allowing for different output strategies (console, quiet, TUI, etc.).

use ansi_to_tui::IntoText;
use crossterm::{
    QueueableCommand,
    style::{self, Color},
    terminal,
};
use ratatui::{
    style::{Color as RatatuiColor, Style},
    text::{Line, Span},
};
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use termimad::{Alignment, MadSkin};

/// Detects if text contains ANSI escape sequences.
///
/// Checks for the common ANSI escape sequence prefix `\x1b[` (ESC + `[`)
/// which is used for colors, formatting, and cursor control.
#[inline]
pub(crate) fn contains_ansi(text: &str) -> bool {
    text.contains("\x1b[")
}

// ============================================================================
// Markdown Rendering (termimad)
//
// 设计目标：
// - Rendered 模式：尽可能把 Markdown 变成“带语义样式”的终端/TUI输出（隐藏控制符）。
// - Plain 模式：禁用 Markdown 渲染（控制符原样可见），但依然需要保留 ANSI 解析能力，
//   避免把 ESC 控制序列原样打印出来。
// - 对于 TUI：我们希望在“给定宽度”下提前完成语义化换行（例如 blockquote 前缀需要出现在换行后每一行）。
//   这样 ContentPane 的软换行就不会破坏语义前缀。
// ============================================================================

/// 当无法获取真实终端宽度时的 fallback（与历史默认行为一致，且可稳定用于测试）。
const DEFAULT_MARKDOWN_WRAP_WIDTH: u16 = 80;

#[inline]
fn normalize_wrap_width(width: u16) -> u16 {
    if width == 0 {
        DEFAULT_MARKDOWN_WRAP_WIDTH
    } else {
        width
    }
}

#[inline]
fn terminal_wrap_width() -> u16 {
    terminal::size()
        .ok()
        .map(|(w, _)| w)
        .filter(|w| *w > 0)
        .unwrap_or(DEFAULT_MARKDOWN_WRAP_WIDTH)
}

// ============================================================================
// Markdown Theme: Sublime Monokai Extended
//
// 说明：
// - 这套颜色来自 `jonschlinkert/sublime-monokai-extended`（Monokai Extended.tmTheme）。
// - 我们把它限制在 termimad 的 `MadSkin` 里，只影响 Markdown 的“语义样式”渲染：
//   - stdout（Pretty 输出）
//   - TUI（转为 `ratatui::Line`）
// - 这样 stdout 与 TUI 两条渲染路径会自然保持一致（都复用同一个 skin），
//   同时不会把全局 TUI 主题（面板底色/边框/标题栏）也一起改掉，避免范围膨胀。
// ============================================================================
mod sublime_monokai_extended {
    // 注意：这里必须使用 `termimad` 自己依赖的 `crossterm::Color` 类型，
    // 而不是工作区直接依赖的 `crossterm::Color`。
    //
    // 原因：
    // - `termimad` 的 `MadSkin`/`LineStyle`/`CompoundStyle` 方法参数使用的是它的 `Color`；
    // - 工作区里同时存在两个不同版本的 crossterm（例如 0.28 vs 0.29），
    //   直接用 `crossterm::style::Color` 会触发类型不匹配（E0308）。
    use termimad::crossterm::style::Color;

    // =====================================================================
    // 全局色彩微调：统一混入 3% 的 #4493f8
    //
    // 你希望所有 Markdown 颜色都“带一点点蓝调”，因此我们对调色板做一个统一的轻微偏色：
    // - 对每个 RGB 通道：new = base * 97% + mix * 3%（四舍五入）
    // - 但“白色正文”保持不变（我们不对 `FOREGROUND` 做混合）
    //
    // 说明：
    // - 这样做比“逐个手调每个颜色”更稳定：你以后想把 3% 改成 2%/5%，只改这一处权重。
    // - 使用整数权重而非浮点，保证 const 下可编译且结果确定。
    // =====================================================================
    const MIX_R: u8 = 0x44;
    const MIX_G: u8 = 0x93;
    const MIX_B: u8 = 0xf8;

    const MIX_WEIGHT_PERCENT: u16 = 3;
    const BASE_WEIGHT_PERCENT: u16 = 100 - MIX_WEIGHT_PERCENT;

    const fn mix_channel(base: u8, mix: u8) -> u8 {
        // 四舍五入：+50 再 /100
        ((base as u16 * BASE_WEIGHT_PERCENT + mix as u16 * MIX_WEIGHT_PERCENT + 50) / 100) as u8
    }

    const fn mix_rgb(r: u8, g: u8, b: u8) -> Color {
        Color::Rgb {
            r: mix_channel(r, MIX_R),
            g: mix_channel(g, MIX_G),
            b: mix_channel(b, MIX_B),
        }
    }

    pub const FOREGROUND: Color = Color::Rgb {
        r: 0xf8,
        g: 0xf8,
        b: 0xf2,
    };

    // 主要结构色（从 tmTheme 的 base 等提取）
    pub const DIMMED: Color = mix_rgb(0x63, 0x60, 0x50);
    pub const DIMMED2: Color = mix_rgb(0x56, 0x56, 0x56);

    // Markdown scope 色（从 tmTheme 的 markup/markdown scope 提取）
    //
    // 注意：Monokai Extended.tmTheme 里 heading 更偏橙（#fd971f）。
    // 这里按你的偏好把 heading 的“基色”覆盖为 #fc9867（随后会统一叠加 3% 的 #4493f8 混合）。
    //
    // 另外：你希望“标题（H1）”更偏亮黄一些，因此 H1 的“基色”单独用 #ffd866（同样会叠加混合）。
    pub const TITLE: Color = mix_rgb(0xff, 0xd8, 0x66);
    pub const HEADING: Color = mix_rgb(0xfc, 0x98, 0x67);
    pub const QUOTE: Color = mix_rgb(0x66, 0xd9, 0xef);
    pub const RAW_INLINE: Color = mix_rgb(0x78, 0xdc, 0xe8);
    // 说明：
    // - 主题原始 bold（Monokai Extended）更偏粉红（例如 #f92672）。
    // - 但你希望“初始化/步骤标签”这类强调文本更偏“完成/可执行”的绿色，因此这里把 bold 的“基色”覆盖为 #a9dc76（同样会叠加混合）。
    pub const BOLD: Color = mix_rgb(0xa9, 0xdc, 0x76);
    pub const ITALIC: Color = mix_rgb(0xe4, 0x2e, 0x70);
    pub const STRIKE: Color = mix_rgb(0xcc, 0x42, 0x73);
    pub const LIST_PUNCT: Color = mix_rgb(0x77, 0x77, 0x77);
}

fn default_markdown_skin() -> MadSkin {
    // =========================================================================
    // termimad 默认 skin 会把 H1（headers[0]）设置为居中：
    // - `skin.headers[0].align = Alignment::Center`
    //
    // 在 Ralph 的输出场景里（日志/代码/事件流），H1 居中会带来两个实际问题：
    // 1) 复制粘贴到文件后，标题左侧会多出空格，影响对齐与 diff 可读性；
    // 2) 与其他行（列表、代码块、引用等）混排时，会产生“锯齿感”的视觉不一致。
    //
    // 因此我们把 H1 改为左对齐，而不改变 H2/H3... 的默认行为。
    // =========================================================================
    let mut skin = MadSkin::default();
    skin.headers[0].align = Alignment::Left;

    // =========================================================================
    // Sublime Monokai Extended 配色映射（Markdown 内部配色）
    //
    // 设计取舍：
    // - 正文：只设置前景色，不强制背景色，避免覆盖用户终端/TUI 面板底色。
    // - 行内代码：取消背景色，减少“色块噪音”，通过前景色进行区分即可。
    // - 代码块：取消背景色，减少大段输出时的“色块噪音”，同时避免与面板底色冲突。
    // - 标题：H1 与 H2+ 分层配色，但仍保持“少数强调色”，避免彩虹标题过于花哨。
    // =========================================================================
    skin.paragraph.set_fg(sublime_monokai_extended::FOREGROUND);

    skin.inline_code
        .set_fg(sublime_monokai_extended::RAW_INLINE);
    // termimad 默认会给 inline code 设置背景色；这里按你的要求取消背景（不铺底）。
    skin.inline_code.object_style.background_color = None;
    skin.code_block.set_fg(sublime_monokai_extended::RAW_INLINE);
    // termimad 默认会给 code block 设置背景色；这里按你的要求取消背景（不铺底）。
    skin.code_block.compound_style.object_style.background_color = None;

    // 标题分层（统一叠加 3% 的 #4493f8 微调；下列为“基色”）：
    // - H1（标题）：#ffd866（混合后约为 #f9d66a）
    // - H2-H6（heading）：#fc9867（混合后约为 #f6986b）
    skin.headers[0].set_fg(sublime_monokai_extended::TITLE);
    for header in &mut skin.headers[1..] {
        header.set_fg(sublime_monokai_extended::HEADING);
    }

    // 结构符号（项目符号/引用竖线/分割线/表格）采用“更弱或更低饱和”的颜色，
    // 目的是让它们提供结构信息，但不抢正文与代码的视觉注意力。
    skin.bullet.set_fg(sublime_monokai_extended::LIST_PUNCT);
    skin.quote_mark.set_fg(sublime_monokai_extended::QUOTE);
    skin.horizontal_rule
        .set_fg(sublime_monokai_extended::DIMMED2);
    skin.table.set_fg(sublime_monokai_extended::DIMMED2);

    // 行内语义（bold/italic/strike）按主题的 markup scope 配色。
    skin.bold.set_fg(sublime_monokai_extended::BOLD);
    skin.italic.set_fg(sublime_monokai_extended::ITALIC);
    skin.strikeout.set_fg(sublime_monokai_extended::STRIKE);

    // 这些符号在输出里出现频率不高，但出现时仍希望“看起来像同一套主题”。
    skin.ellipsis.set_fg(sublime_monokai_extended::DIMMED);

    skin
}

/// Session completion result data.
#[derive(Debug, Clone)]
pub struct SessionResult {
    pub duration_ms: u64,
    pub total_cost_usd: f64,
    pub num_turns: u32,
    pub is_error: bool,
}

/// Markdown 渲染模式。
///
/// - `Rendered`：best-effort 渲染 Markdown（默认），并保留 ANSI 样式。
/// - `Plain`：禁用 Markdown 渲染，按原始文本展示（但仍会解析 ANSI，避免输出 ESC 控制符）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkdownRenderMode {
    Rendered,
    Plain,
}

impl MarkdownRenderMode {
    #[must_use]
    pub const fn from_plain(plain: bool) -> Self {
        if plain { Self::Plain } else { Self::Rendered }
    }
}

/// Renders streaming output with colors and markdown.
pub struct PrettyStreamHandler {
    stdout: io::Stdout,
    verbose: bool,
    /// Buffer for accumulating text before markdown rendering
    text_buffer: String,
    /// 输出渲染模式（默认渲染 Markdown；`--plain` 时禁用）。
    render_mode: MarkdownRenderMode,
}

impl PrettyStreamHandler {
    /// Creates a new pretty handler.
    pub fn new(verbose: bool) -> Self {
        Self::new_with_mode(verbose, MarkdownRenderMode::Rendered)
    }

    /// Creates a new pretty handler with explicit render mode.
    pub fn new_with_mode(verbose: bool, render_mode: MarkdownRenderMode) -> Self {
        Self {
            stdout: io::stdout(),
            verbose,
            text_buffer: String::new(),
            render_mode,
        }
    }

    /// Flush buffered text as rendered markdown.
    fn flush_text_buffer(&mut self) {
        if self.text_buffer.is_empty() {
            return;
        }

        let text = std::mem::take(&mut self.text_buffer);

        // 说明：
        // - Plain：原样输出（Markdown 控制符可见）。
        // - 含 ANSI：直接原样输出，避免二次渲染吞掉样式/控制码。
        if self.render_mode == MarkdownRenderMode::Plain || contains_ansi(&text) {
            let _ = self.stdout.write_all(text.as_bytes());
            let _ = self.stdout.flush();
            return;
        }

        let wrap_width = usize::from(terminal_wrap_width()).max(3);

        // termimad 会把 Markdown 渲染成带 ANSI 的终端文本（并在给定宽度下 hard-wrap）。
        // 对于非 TUI（stdout）场景，直接输出 ANSI 最简单，
        // 也避免了“ratatui::Line → ANSI”的二次转换带来的额外开销与差异。
        let skin = default_markdown_skin();
        let rendered = skin.text(&text, Some(wrap_width)).to_string();

        let _ = self.stdout.write_all(rendered.as_bytes());
        let _ = self.stdout.flush();
    }
}

impl StreamHandler for PrettyStreamHandler {
    fn on_text(&mut self, text: &str) {
        // Buffer text for markdown rendering
        // Text is flushed when: tool calls arrive, on_complete is called, or on_error is called
        // This works well for StreamJson backends (Claude) which have natural flush points
        // Text format backends should use ConsoleStreamHandler for immediate output
        self.text_buffer.push_str(text);
    }

    fn on_tool_result(&mut self, _id: &str, output: &str) {
        if self.verbose {
            let _ = self
                .stdout
                .queue(style::SetForegroundColor(Color::DarkGrey));
            let _ = self
                .stdout
                .write(format!(" \u{2713} {}\n", truncate(output, 200)).as_bytes());
            let _ = self.stdout.queue(style::ResetColor);
            let _ = self.stdout.flush();
        }
    }

    fn on_error(&mut self, error: &str) {
        let _ = self.stdout.queue(style::SetForegroundColor(Color::Red));
        let _ = self
            .stdout
            .write(format!("\n\u{2717} Error: {}\n", error).as_bytes());
        let _ = self.stdout.queue(style::ResetColor);
        let _ = self.stdout.flush();
    }

    fn on_complete(&mut self, result: &SessionResult) {
        // Flush any remaining buffered text
        self.flush_text_buffer();

        let _ = self.stdout.write(b"\n");
        let color = if result.is_error {
            Color::Red
        } else {
            Color::Green
        };
        let _ = self.stdout.queue(style::SetForegroundColor(color));
        let _ = self.stdout.write(
            format!(
                "Duration: {}ms | Cost: ${:.4} | Turns: {}\n",
                result.duration_ms, result.total_cost_usd, result.num_turns
            )
            .as_bytes(),
        );
        let _ = self.stdout.queue(style::ResetColor);
        let _ = self.stdout.flush();
    }

    fn on_tool_call(&mut self, name: &str, _id: &str, input: &serde_json::Value) {
        // Flush any buffered text before showing tool call
        self.flush_text_buffer();

        // ⚙️ [ToolName]
        let _ = self.stdout.queue(style::SetForegroundColor(Color::Blue));
        let _ = self.stdout.write(format!("\u{2699} [{}]", name).as_bytes());

        if let Some(summary) = format_tool_summary(name, input) {
            let _ = self
                .stdout
                .queue(style::SetForegroundColor(Color::DarkGrey));
            let _ = self.stdout.write(format!(" {}\n", summary).as_bytes());
        } else {
            let _ = self.stdout.write(b"\n");
        }
        let _ = self.stdout.queue(style::ResetColor);
        let _ = self.stdout.flush();
    }
}

/// Handler for streaming output events from Claude.
///
/// Implementors receive events as Claude processes and can format/display
/// them in various ways (console output, TUI updates, logging, etc.).
pub trait StreamHandler: Send {
    /// Called when Claude emits text.
    fn on_text(&mut self, text: &str);

    /// Called when Claude invokes a tool.
    ///
    /// # Arguments
    /// * `name` - Tool name (e.g., "Read", "Bash", "Grep")
    /// * `id` - Unique tool invocation ID
    /// * `input` - Tool input parameters as JSON (file paths, commands, patterns, etc.)
    fn on_tool_call(&mut self, name: &str, id: &str, input: &serde_json::Value);

    /// Called when a tool returns results (verbose only).
    fn on_tool_result(&mut self, id: &str, output: &str);

    /// Called when an error occurs.
    fn on_error(&mut self, error: &str);

    /// Called when session completes (verbose only).
    fn on_complete(&mut self, result: &SessionResult);
}

/// Writes streaming output to stdout/stderr.
///
/// In normal mode, displays assistant text and tool invocations.
/// In verbose mode, also displays tool results and session summary.
pub struct ConsoleStreamHandler {
    verbose: bool,
    stdout: io::Stdout,
    stderr: io::Stderr,
}

impl ConsoleStreamHandler {
    /// Creates a new console handler.
    ///
    /// # Arguments
    /// * `verbose` - If true, shows tool results and session summary.
    pub fn new(verbose: bool) -> Self {
        Self {
            verbose,
            stdout: io::stdout(),
            stderr: io::stderr(),
        }
    }
}

impl StreamHandler for ConsoleStreamHandler {
    fn on_text(&mut self, text: &str) {
        let _ = write!(self.stdout, "{}", text);
    }

    fn on_tool_call(&mut self, name: &str, _id: &str, input: &serde_json::Value) {
        match format_tool_summary(name, input) {
            Some(summary) => {
                let _ = writeln!(self.stdout, "[Tool] {}: {}", name, summary);
            }
            None => {
                let _ = writeln!(self.stdout, "[Tool] {}", name);
            }
        }
    }

    fn on_tool_result(&mut self, _id: &str, output: &str) {
        if self.verbose {
            let _ = writeln!(self.stdout, "[Result] {}", truncate(output, 200));
        }
    }

    fn on_error(&mut self, error: &str) {
        // Write to both stdout (inline) and stderr (for separation)
        let _ = writeln!(self.stdout, "[Error] {}", error);
        let _ = writeln!(self.stderr, "[Error] {}", error);
    }

    fn on_complete(&mut self, result: &SessionResult) {
        if self.verbose {
            let _ = writeln!(
                self.stdout,
                "\n--- Session Complete ---\nDuration: {}ms | Cost: ${:.4} | Turns: {}",
                result.duration_ms, result.total_cost_usd, result.num_turns
            );
        }
    }
}

/// Suppresses all streaming output (for CI/silent mode).
pub struct QuietStreamHandler;

impl StreamHandler for QuietStreamHandler {
    fn on_text(&mut self, _: &str) {}
    fn on_tool_call(&mut self, _: &str, _: &str, _: &serde_json::Value) {}
    fn on_tool_result(&mut self, _: &str, _: &str) {}
    fn on_error(&mut self, _: &str) {}
    fn on_complete(&mut self, _: &SessionResult) {}
}

fn plain_text_to_lines(text: &str) -> Vec<Line<'static>> {
    text.split('\n')
        .map(|line| Line::from(line.to_string()))
        .collect()
}

fn ansi_text_to_lines(text: &str) -> Vec<Line<'static>> {
    match text.into_text() {
        Ok(parsed_text) => parsed_text
            .lines
            .into_iter()
            .map(|line| {
                let owned_spans: Vec<Span<'static>> = line
                    .spans
                    .into_iter()
                    .map(|span| Span::styled(span.content.into_owned(), span.style))
                    .collect();
                Line::from(owned_spans)
            })
            .collect(),
        Err(_) => plain_text_to_lines(text),
    }
}

fn render_markdown_to_lines_best_effort(text: &str, wrap_width: u16) -> Vec<Line<'static>> {
    render_markdown_to_lines(text, wrap_width).unwrap_or_else(|| plain_text_to_lines(text))
}

fn render_markdown_to_lines(text: &str, wrap_width: u16) -> Option<Vec<Line<'static>>> {
    let wrap_width = usize::from(normalize_wrap_width(wrap_width)).max(3);

    // termimad 会把 Markdown 渲染成带 ANSI 的终端文本（并在给定宽度下 hard-wrap）。
    // 然后我们再把 ANSI 解析回 ratatui Lines，以便在 TUI 内渲染。
    let skin = default_markdown_skin();
    let rendered = skin.text(text, Some(wrap_width)).to_string();

    match rendered.as_str().into_text() {
        Ok(parsed_text) => Some(
            parsed_text
                .lines
                .into_iter()
                .map(|line| {
                    let owned_spans: Vec<Span<'static>> = line
                        .spans
                        .into_iter()
                        .map(|span| Span::styled(span.content.into_owned(), span.style))
                        .collect();
                    Line::from(owned_spans)
                })
                .collect(),
        ),
        Err(_) => None,
    }
}

/// 把文本转换为带样式的 `ratatui` 行（`Line<'static>`），同时处理 ANSI 与 Markdown。
///
/// 说明：
/// - 若文本本身包含 ANSI 转义序列（例如某些 CLI 工具的彩色输出），则直接走 ANSI → ratatui 的解析，
///   以保留颜色/格式。
/// - 否则：
///   - `Rendered`：使用 `termimad` best-effort 渲染 Markdown，并按 `wrap_width` 做语义化换行。
///   - `Plain`：保持原始文本（Markdown 控制符原样可见）。
/// - 当 `termimad` 渲染结果无法解析为 ratatui 行时，必须安全降级为纯文本显示（不 panic、不丢内容）。
pub fn render_text_to_lines(
    text: &str,
    mode: MarkdownRenderMode,
    wrap_width: u16,
) -> Vec<Line<'static>> {
    if text.is_empty() {
        return Vec::new();
    }

    let wrap_width = normalize_wrap_width(wrap_width);

    if contains_ansi(text) {
        return ansi_text_to_lines(text);
    }

    match mode {
        MarkdownRenderMode::Rendered => render_markdown_to_lines_best_effort(text, wrap_width),
        MarkdownRenderMode::Plain => plain_text_to_lines(text),
    }
}

/// A content block in the chronological stream.
///
/// Used to preserve ordering between text and non-text content (tool calls, errors).
#[derive(Clone)]
enum ContentBlock {
    /// Markdown/ANSI text that was accumulated before being frozen
    Text(String),
    /// A single non-text line (tool call, error, completion summary, etc.)
    NonText(Line<'static>),
}

/// Renders streaming output as ratatui Lines for TUI display.
///
/// This handler produces output visually equivalent to `PrettyStreamHandler`
/// but stores it as `Line<'static>` objects for rendering in a ratatui-based TUI.
///
/// Text content is parsed as markdown, producing styled output for bold, italic,
/// code, headers, etc. Tool calls and errors bypass markdown parsing to preserve
/// their explicit styling.
///
/// **Chronological ordering**: When a tool call arrives, the current text buffer
/// is "frozen" into a content block, preserving the order in which events arrived.
pub struct TuiStreamHandler {
    /// Buffer for accumulating current markdown text (not yet frozen)
    current_text_buffer: String,
    /// Chronological sequence of content blocks (frozen text + non-text events)
    blocks: Vec<ContentBlock>,
    /// Verbose mode (show tool results)
    verbose: bool,
    /// 输出渲染模式（默认渲染 Markdown；`--plain` 时禁用）。
    render_mode: MarkdownRenderMode,
    /// Collected output lines for rendering
    lines: Arc<Mutex<Vec<Line<'static>>>>,
}

impl TuiStreamHandler {
    /// Creates a new TUI handler.
    ///
    /// # Arguments
    /// * `verbose` - If true, shows tool results and session summary.
    pub fn new(verbose: bool) -> Self {
        Self::new_with_mode(verbose, MarkdownRenderMode::Rendered)
    }

    /// Creates a new TUI handler with explicit render mode.
    pub fn new_with_mode(verbose: bool, render_mode: MarkdownRenderMode) -> Self {
        Self {
            current_text_buffer: String::new(),
            blocks: Vec::new(),
            verbose,
            render_mode,
            lines: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Creates a TUI handler with shared lines storage.
    ///
    /// Use this to share output lines with the TUI application.
    pub fn with_lines(verbose: bool, lines: Arc<Mutex<Vec<Line<'static>>>>) -> Self {
        Self::with_lines_and_mode(verbose, lines, MarkdownRenderMode::Rendered)
    }

    /// Creates a TUI handler with shared lines storage and explicit render mode.
    pub fn with_lines_and_mode(
        verbose: bool,
        lines: Arc<Mutex<Vec<Line<'static>>>>,
        render_mode: MarkdownRenderMode,
    ) -> Self {
        Self {
            current_text_buffer: String::new(),
            blocks: Vec::new(),
            verbose,
            render_mode,
            lines,
        }
    }

    /// Returns a clone of the collected lines.
    pub fn get_lines(&self) -> Vec<Line<'static>> {
        self.lines.lock().unwrap().clone()
    }

    /// Flushes any buffered markdown text by re-parsing and updating lines.
    pub fn flush_text_buffer(&mut self) {
        self.update_lines();
    }

    /// Freezes the current text buffer into a content block.
    ///
    /// This is called when a non-text event (tool call, error) arrives,
    /// ensuring that text before the event stays before it in the output.
    fn freeze_current_text(&mut self) {
        if !self.current_text_buffer.is_empty() {
            self.blocks
                .push(ContentBlock::Text(self.current_text_buffer.clone()));
            self.current_text_buffer.clear();
        }
    }

    /// Re-renders all content blocks and updates the shared lines.
    ///
    /// Iterates through frozen blocks in chronological order, then appends
    /// any current (unfrozen) text buffer content. This preserves the
    /// interleaved ordering of text and non-text content.
    fn update_lines(&mut self) {
        let mut all_lines = Vec::new();
        let wrap_width = terminal_wrap_width();

        // Render frozen blocks in chronological order
        for block in &self.blocks {
            match block {
                ContentBlock::Text(text) => {
                    all_lines.extend(render_text_to_lines(text, self.render_mode, wrap_width));
                }
                ContentBlock::NonText(line) => {
                    all_lines.push(line.clone());
                }
            }
        }

        // Render current (unfrozen) text buffer for real-time updates
        if !self.current_text_buffer.is_empty() {
            all_lines.extend(render_text_to_lines(
                &self.current_text_buffer,
                self.render_mode,
                wrap_width,
            ));
        }

        // Note: Long lines are NOT truncated here. The TUI's ContentPane widget
        // handles soft-wrapping at viewport boundaries, preserving full content.

        // Update shared lines
        *self.lines.lock().unwrap() = all_lines;
    }

    /// Adds a non-text line (tool call, error, etc.) and updates display.
    ///
    /// First freezes any pending text buffer to preserve chronological order.
    fn add_non_text_line(&mut self, line: Line<'static>) {
        self.freeze_current_text();
        self.blocks.push(ContentBlock::NonText(line));
        self.update_lines();
    }
}

impl StreamHandler for TuiStreamHandler {
    fn on_text(&mut self, text: &str) {
        // Append text to current buffer
        self.current_text_buffer.push_str(text);

        // Re-parse and update lines on each text chunk
        // This handles streaming markdown correctly
        self.update_lines();
    }

    fn on_tool_call(&mut self, name: &str, _id: &str, input: &serde_json::Value) {
        // Build spans: ⚙️ [ToolName] summary
        let mut spans = vec![Span::styled(
            format!("\u{2699} [{}]", name),
            Style::default().fg(RatatuiColor::Blue),
        )];

        if let Some(summary) = format_tool_summary(name, input) {
            spans.push(Span::styled(
                format!(" {}", summary),
                Style::default().fg(RatatuiColor::DarkGray),
            ));
        }

        self.add_non_text_line(Line::from(spans));
    }

    fn on_tool_result(&mut self, _id: &str, output: &str) {
        if self.verbose {
            let line = Line::from(Span::styled(
                format!(" \u{2713} {}", truncate(output, 200)),
                Style::default().fg(RatatuiColor::DarkGray),
            ));
            self.add_non_text_line(line);
        }
    }

    fn on_error(&mut self, error: &str) {
        let line = Line::from(Span::styled(
            format!("\n\u{2717} Error: {}", error),
            Style::default().fg(RatatuiColor::Red),
        ));
        self.add_non_text_line(line);
    }

    fn on_complete(&mut self, result: &SessionResult) {
        // Flush any remaining buffered text
        self.flush_text_buffer();

        // Add blank line
        self.add_non_text_line(Line::from(""));

        // Add summary with color based on error status
        let color = if result.is_error {
            RatatuiColor::Red
        } else {
            RatatuiColor::Green
        };
        let summary = format!(
            "Duration: {}ms | Cost: ${:.4} | Turns: {}",
            result.duration_ms, result.total_cost_usd, result.num_turns
        );
        let line = Line::from(Span::styled(summary, Style::default().fg(color)));
        self.add_non_text_line(line);
    }
}

/// Extracts the most relevant field from tool input for display.
///
/// Returns a human-readable summary (file path, command, pattern, etc.) based on the tool type.
/// Returns `None` for unknown tools or if the expected field is missing.
fn format_tool_summary(name: &str, input: &serde_json::Value) -> Option<String> {
    match name {
        "Read" | "Edit" | "Write" => input.get("file_path")?.as_str().map(|s| s.to_string()),
        "Bash" => {
            let cmd = input.get("command")?.as_str()?;
            Some(truncate(cmd, 60))
        }
        "Grep" => input.get("pattern")?.as_str().map(|s| s.to_string()),
        "Glob" => input.get("pattern")?.as_str().map(|s| s.to_string()),
        "Task" => input.get("description")?.as_str().map(|s| s.to_string()),
        "WebFetch" => input.get("url")?.as_str().map(|s| s.to_string()),
        "WebSearch" => input.get("query")?.as_str().map(|s| s.to_string()),
        "LSP" => {
            let op = input.get("operation")?.as_str()?;
            let file = input.get("filePath")?.as_str()?;
            Some(format!("{} @ {}", op, file))
        }
        "NotebookEdit" => input.get("notebook_path")?.as_str().map(|s| s.to_string()),
        "TodoWrite" => Some("updating todo list".to_string()),
        _ => None,
    }
}

/// Truncates a string to approximately `max_len` characters, adding "..." if truncated.
///
/// Uses `char_indices` to find a valid UTF-8 boundary, ensuring we never slice
/// in the middle of a multi-byte character.
fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        // Find the byte index of the max_len-th character
        let byte_idx = s
            .char_indices()
            .nth(max_len)
            .map(|(idx, _)| idx)
            .unwrap_or(s.len());
        format!("{}...", &s[..byte_idx])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_console_handler_verbose_shows_results() {
        let mut handler = ConsoleStreamHandler::new(true);
        let bash_input = json!({"command": "ls -la"});

        // These calls should not panic
        handler.on_text("Hello");
        handler.on_tool_call("Bash", "tool_1", &bash_input);
        handler.on_tool_result("tool_1", "output");
        handler.on_complete(&SessionResult {
            duration_ms: 1000,
            total_cost_usd: 0.01,
            num_turns: 1,
            is_error: false,
        });
    }

    #[test]
    fn test_console_handler_normal_skips_results() {
        let mut handler = ConsoleStreamHandler::new(false);
        let read_input = json!({"file_path": "src/main.rs"});

        // These should not show tool results
        handler.on_text("Hello");
        handler.on_tool_call("Read", "tool_1", &read_input);
        handler.on_tool_result("tool_1", "output"); // Should be silent
        handler.on_complete(&SessionResult {
            duration_ms: 1000,
            total_cost_usd: 0.01,
            num_turns: 1,
            is_error: false,
        }); // Should be silent
    }

    #[test]
    fn test_quiet_handler_is_silent() {
        let mut handler = QuietStreamHandler;
        let empty_input = json!({});

        // All of these should be no-ops
        handler.on_text("Hello");
        handler.on_tool_call("Read", "tool_1", &empty_input);
        handler.on_tool_result("tool_1", "output");
        handler.on_error("Something went wrong");
        handler.on_complete(&SessionResult {
            duration_ms: 1000,
            total_cost_usd: 0.01,
            num_turns: 1,
            is_error: false,
        });
    }

    #[test]
    fn test_truncate_helper() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("this is a long string", 10), "this is a ...");
    }

    #[test]
    fn test_truncate_utf8_boundaries() {
        // Arrow → is 3 bytes (U+2192: E2 86 92)
        let with_arrows = "→→→→→→→→→→";
        // Should truncate at character boundary, not byte boundary
        assert_eq!(truncate(with_arrows, 5), "→→→→→...");

        // Mixed ASCII and multi-byte
        let mixed = "a→b→c→d→e";
        assert_eq!(truncate(mixed, 5), "a→b→c...");

        // Emoji (4-byte characters)
        let emoji = "🎉🎊🎁🎈🎄";
        assert_eq!(truncate(emoji, 3), "🎉🎊🎁...");
    }

    #[test]
    fn test_format_tool_summary_file_tools() {
        assert_eq!(
            format_tool_summary("Read", &json!({"file_path": "src/main.rs"})),
            Some("src/main.rs".to_string())
        );
        assert_eq!(
            format_tool_summary("Edit", &json!({"file_path": "/path/to/file.txt"})),
            Some("/path/to/file.txt".to_string())
        );
        assert_eq!(
            format_tool_summary("Write", &json!({"file_path": "output.json"})),
            Some("output.json".to_string())
        );
    }

    #[test]
    fn test_format_tool_summary_bash_truncates() {
        let short_cmd = json!({"command": "ls -la"});
        assert_eq!(
            format_tool_summary("Bash", &short_cmd),
            Some("ls -la".to_string())
        );

        let long_cmd = json!({"command": "this is a very long command that should be truncated because it exceeds sixty characters"});
        let result = format_tool_summary("Bash", &long_cmd).unwrap();
        assert!(result.ends_with("..."));
        assert!(result.len() <= 70); // 60 chars + "..."
    }

    #[test]
    fn test_format_tool_summary_search_tools() {
        assert_eq!(
            format_tool_summary("Grep", &json!({"pattern": "TODO"})),
            Some("TODO".to_string())
        );
        assert_eq!(
            format_tool_summary("Glob", &json!({"pattern": "**/*.rs"})),
            Some("**/*.rs".to_string())
        );
    }

    #[test]
    fn test_format_tool_summary_unknown_tool_returns_none() {
        assert_eq!(
            format_tool_summary("UnknownTool", &json!({"some_field": "value"})),
            None
        );
    }

    #[test]
    fn test_format_tool_summary_missing_field_returns_none() {
        // Read without file_path
        assert_eq!(
            format_tool_summary("Read", &json!({"wrong_field": "value"})),
            None
        );
        // Bash without command
        assert_eq!(format_tool_summary("Bash", &json!({})), None);
    }

    // ========================================================================
    // TuiStreamHandler Tests
    // ========================================================================

    mod tui_stream_handler {
        use super::*;
        use ratatui::style::{Color, Modifier};

        /// Helper to collect lines from TuiStreamHandler
        fn collect_lines(handler: &TuiStreamHandler) -> Vec<ratatui::text::Line<'static>> {
            handler.lines.lock().unwrap().clone()
        }

        #[test]
        fn text_creates_line_on_newline() {
            // Given TuiStreamHandler
            let mut handler = TuiStreamHandler::new(false);

            // When on_text("hello\n") is called
            handler.on_text("hello\n");

            // Then a Line with "hello" content is produced
            // Note: termimad (like non-TUI mode) doesn't create empty line for trailing \n
            let lines = collect_lines(&handler);
            assert_eq!(
                lines.len(),
                1,
                "termimad doesn't create trailing empty line"
            );
            assert_eq!(lines[0].to_string(), "hello");
        }

        #[test]
        fn partial_text_buffering() {
            // Given TuiStreamHandler
            let mut handler = TuiStreamHandler::new(false);

            // When on_text("hel") then on_text("lo\n") is called
            // Note: With markdown parsing, partial text is rendered immediately
            // (markdown doesn't require newlines for paragraphs)
            handler.on_text("hel");
            handler.on_text("lo\n");

            // Then the combined "hello" text is present
            let lines = collect_lines(&handler);
            let full_text: String = lines.iter().map(|l| l.to_string()).collect();
            assert!(
                full_text.contains("hello"),
                "Combined text should contain 'hello'. Lines: {:?}",
                lines
            );
        }

        #[test]
        fn tool_call_produces_formatted_line() {
            // Given TuiStreamHandler
            let mut handler = TuiStreamHandler::new(false);

            // When on_tool_call("Read", "id", &json!({"file_path": "src/main.rs"})) is called
            handler.on_tool_call("Read", "tool_1", &json!({"file_path": "src/main.rs"}));

            // Then a Line starting with "⚙️" and containing "Read" and file path is produced
            let lines = collect_lines(&handler);
            assert_eq!(lines.len(), 1);
            let line_text = lines[0].to_string();
            assert!(
                line_text.contains('\u{2699}'),
                "Should contain gear emoji: {}",
                line_text
            );
            assert!(
                line_text.contains("Read"),
                "Should contain tool name: {}",
                line_text
            );
            assert!(
                line_text.contains("src/main.rs"),
                "Should contain file path: {}",
                line_text
            );
        }

        #[test]
        fn tool_result_verbose_shows_content() {
            // Given TuiStreamHandler with verbose=true
            let mut handler = TuiStreamHandler::new(true);

            // When on_tool_result(...) is called
            handler.on_tool_result("tool_1", "file contents here");

            // Then result content appears in output
            let lines = collect_lines(&handler);
            assert_eq!(lines.len(), 1);
            let line_text = lines[0].to_string();
            assert!(
                line_text.contains('\u{2713}'),
                "Should contain checkmark: {}",
                line_text
            );
            assert!(
                line_text.contains("file contents here"),
                "Should contain result content: {}",
                line_text
            );
        }

        #[test]
        fn tool_result_quiet_is_silent() {
            // Given TuiStreamHandler with verbose=false
            let mut handler = TuiStreamHandler::new(false);

            // When on_tool_result(...) is called
            handler.on_tool_result("tool_1", "file contents here");

            // Then no output is produced
            let lines = collect_lines(&handler);
            assert!(
                lines.is_empty(),
                "verbose=false should not produce tool result output"
            );
        }

        #[test]
        fn error_produces_red_styled_line() {
            // Given TuiStreamHandler
            let mut handler = TuiStreamHandler::new(false);

            // When on_error("fail") is called
            handler.on_error("Something went wrong");

            // Then a Line with red foreground style is produced
            let lines = collect_lines(&handler);
            assert_eq!(lines.len(), 1);
            let line_text = lines[0].to_string();
            assert!(
                line_text.contains('\u{2717}'),
                "Should contain X mark: {}",
                line_text
            );
            assert!(
                line_text.contains("Error"),
                "Should contain 'Error': {}",
                line_text
            );
            assert!(
                line_text.contains("Something went wrong"),
                "Should contain error message: {}",
                line_text
            );

            // Check style is red
            let first_span = &lines[0].spans[0];
            assert_eq!(
                first_span.style.fg,
                Some(Color::Red),
                "Error line should have red foreground"
            );
        }

        #[test]
        fn long_lines_preserved_without_truncation() {
            // Given TuiStreamHandler
            let mut handler = TuiStreamHandler::new(false);

            // When on_text() receives a very long string (500+ chars)
            let long_string: String = "a".repeat(500) + "\n";
            handler.on_text(&long_string);

            // Then content is preserved fully (termimad may wrap at terminal width)
            // Note: termimad wraps at ~80 chars by default, so 500 chars = multiple lines
            let lines = collect_lines(&handler);

            // Verify total content is preserved (all 500 'a's present)
            let total_content: String = lines.iter().map(|l| l.to_string()).collect();
            let a_count = total_content.chars().filter(|c| *c == 'a').count();
            assert_eq!(
                a_count, 500,
                "All 500 'a' chars should be preserved. Got {}",
                a_count
            );

            // Should not have truncation ellipsis
            assert!(
                !total_content.contains("..."),
                "Content should not have ellipsis truncation"
            );
        }

        #[test]
        fn multiple_lines_in_single_text_call() {
            // When text contains multiple newlines
            let mut handler = TuiStreamHandler::new(false);
            handler.on_text("line1\nline2\nline3\n");

            // Then all text content is present
            // Note: Markdown parsing may combine lines into paragraphs differently
            let lines = collect_lines(&handler);
            let full_text: String = lines
                .iter()
                .map(|l| l.to_string())
                .collect::<Vec<_>>()
                .join(" ");
            assert!(
                full_text.contains("line1")
                    && full_text.contains("line2")
                    && full_text.contains("line3"),
                "All lines should be present. Lines: {:?}",
                lines
            );
        }

        #[test]
        fn termimad_parity_with_non_tui_mode() {
            // Verify that TUI mode (using termimad) matches non-TUI mode output
            // This ensures the "★ Insight" box renders consistently in both modes
            let text = "Some text before:★ Insight ─────\nKey point here";

            let mut handler = TuiStreamHandler::new(false);
            handler.on_text(text);

            let lines = collect_lines(&handler);

            // termimad wraps after "★ Insight " putting dashes on their own line
            // This matches PrettyStreamHandler (non-TUI) behavior
            assert!(
                lines.len() >= 2,
                "termimad should produce multiple lines. Got: {:?}",
                lines.iter().map(|l| l.to_string()).collect::<Vec<_>>()
            );

            // Content should be preserved
            let full_text: String = lines.iter().map(|l| l.to_string()).collect();
            assert!(
                full_text.contains("★ Insight"),
                "Content should contain insight marker"
            );
        }

        #[test]
        fn tool_call_flushes_text_buffer() {
            // Given buffered text
            let mut handler = TuiStreamHandler::new(false);
            handler.on_text("partial text");

            // When tool call arrives
            handler.on_tool_call("Read", "id", &json!({}));

            // Then buffered text is flushed as a line before tool call line
            let lines = collect_lines(&handler);
            assert_eq!(lines.len(), 2);
            assert_eq!(lines[0].to_string(), "partial text");
            assert!(lines[1].to_string().contains('\u{2699}'));
        }

        #[test]
        fn interleaved_text_and_tools_preserves_chronological_order() {
            // Given: text1 → tool1 → text2 → tool2
            // Expected output order: text1, tool1, text2, tool2
            // NOT: text1 + text2, then tool1 + tool2 (the bug we fixed)
            let mut handler = TuiStreamHandler::new(false);

            // Simulate Claude's streaming output pattern
            handler.on_text("I'll start by reviewing the scratchpad.\n");
            handler.on_tool_call("Read", "id1", &json!({"file_path": "scratchpad.md"}));
            handler.on_text("I found the task. Now checking the code.\n");
            handler.on_tool_call("Read", "id2", &json!({"file_path": "main.rs"}));
            handler.on_text("Done reviewing.\n");

            let lines = collect_lines(&handler);

            // Find indices of key content
            let text1_idx = lines
                .iter()
                .position(|l| l.to_string().contains("reviewing the scratchpad"));
            let tool1_idx = lines
                .iter()
                .position(|l| l.to_string().contains("scratchpad.md"));
            let text2_idx = lines
                .iter()
                .position(|l| l.to_string().contains("checking the code"));
            let tool2_idx = lines.iter().position(|l| l.to_string().contains("main.rs"));
            let text3_idx = lines
                .iter()
                .position(|l| l.to_string().contains("Done reviewing"));

            // All content should be present
            assert!(text1_idx.is_some(), "text1 should be present");
            assert!(tool1_idx.is_some(), "tool1 should be present");
            assert!(text2_idx.is_some(), "text2 should be present");
            assert!(tool2_idx.is_some(), "tool2 should be present");
            assert!(text3_idx.is_some(), "text3 should be present");

            // Chronological order must be preserved
            let text1_idx = text1_idx.unwrap();
            let tool1_idx = tool1_idx.unwrap();
            let text2_idx = text2_idx.unwrap();
            let tool2_idx = tool2_idx.unwrap();
            let text3_idx = text3_idx.unwrap();

            assert!(
                text1_idx < tool1_idx,
                "text1 ({}) should come before tool1 ({}). Lines: {:?}",
                text1_idx,
                tool1_idx,
                lines.iter().map(|l| l.to_string()).collect::<Vec<_>>()
            );
            assert!(
                tool1_idx < text2_idx,
                "tool1 ({}) should come before text2 ({}). Lines: {:?}",
                tool1_idx,
                text2_idx,
                lines.iter().map(|l| l.to_string()).collect::<Vec<_>>()
            );
            assert!(
                text2_idx < tool2_idx,
                "text2 ({}) should come before tool2 ({}). Lines: {:?}",
                text2_idx,
                tool2_idx,
                lines.iter().map(|l| l.to_string()).collect::<Vec<_>>()
            );
            assert!(
                tool2_idx < text3_idx,
                "tool2 ({}) should come before text3 ({}). Lines: {:?}",
                tool2_idx,
                text3_idx,
                lines.iter().map(|l| l.to_string()).collect::<Vec<_>>()
            );
        }

        #[test]
        fn on_complete_flushes_buffer_and_shows_summary() {
            // Given buffered text and verbose mode
            let mut handler = TuiStreamHandler::new(true);
            handler.on_text("final output");

            // When on_complete is called
            handler.on_complete(&SessionResult {
                duration_ms: 1500,
                total_cost_usd: 0.0025,
                num_turns: 3,
                is_error: false,
            });

            // Then buffer is flushed and summary line appears
            let lines = collect_lines(&handler);
            assert!(lines.len() >= 2, "Should have at least 2 lines");
            assert_eq!(lines[0].to_string(), "final output");

            // Find summary line
            let summary = lines.last().unwrap().to_string();
            assert!(
                summary.contains("1500"),
                "Should contain duration: {}",
                summary
            );
            assert!(
                summary.contains("0.0025"),
                "Should contain cost: {}",
                summary
            );
            assert!(summary.contains('3'), "Should contain turns: {}", summary);
        }

        #[test]
        fn on_complete_error_uses_red_style() {
            let mut handler = TuiStreamHandler::new(true);
            handler.on_complete(&SessionResult {
                duration_ms: 1000,
                total_cost_usd: 0.01,
                num_turns: 1,
                is_error: true,
            });

            let lines = collect_lines(&handler);
            assert!(!lines.is_empty());

            // Last line should be red styled for error
            let last_line = lines.last().unwrap();
            assert_eq!(
                last_line.spans[0].style.fg,
                Some(Color::Red),
                "Error completion should have red foreground"
            );
        }

        #[test]
        fn on_complete_success_uses_green_style() {
            let mut handler = TuiStreamHandler::new(true);
            handler.on_complete(&SessionResult {
                duration_ms: 1000,
                total_cost_usd: 0.01,
                num_turns: 1,
                is_error: false,
            });

            let lines = collect_lines(&handler);
            assert!(!lines.is_empty());

            // Last line should be green styled for success
            let last_line = lines.last().unwrap();
            assert_eq!(
                last_line.spans[0].style.fg,
                Some(Color::Green),
                "Success completion should have green foreground"
            );
        }

        #[test]
        fn tool_call_with_no_summary_shows_just_name() {
            let mut handler = TuiStreamHandler::new(false);
            handler.on_tool_call("UnknownTool", "id", &json!({}));

            let lines = collect_lines(&handler);
            assert_eq!(lines.len(), 1);
            let line_text = lines[0].to_string();
            assert!(line_text.contains("UnknownTool"));
            // Should not crash or show "null" for missing summary
        }

        #[test]
        fn get_lines_returns_clone_of_internal_lines() {
            let mut handler = TuiStreamHandler::new(false);
            handler.on_text("test\n");

            let lines1 = handler.get_lines();
            let lines2 = handler.get_lines();

            // Both should have same content
            assert_eq!(lines1.len(), lines2.len());
            assert_eq!(lines1[0].to_string(), lines2[0].to_string());
        }

        // =====================================================================
        // Markdown Rendering Tests
        // =====================================================================

        #[test]
        fn markdown_bold_text_renders_with_bold_modifier() {
            // Given TuiStreamHandler
            let mut handler = TuiStreamHandler::new(false);

            // When on_text("**important**\n") is called
            handler.on_text("**important**\n");

            // Then the text "important" appears with BOLD modifier
            let lines = collect_lines(&handler);
            assert!(!lines.is_empty(), "Should have at least one line");

            // Find a span containing "important" and check it's bold
            let has_bold = lines.iter().any(|line| {
                line.spans.iter().any(|span| {
                    span.content.contains("important")
                        && span.style.add_modifier.contains(Modifier::BOLD)
                })
            });
            assert!(
                has_bold,
                "Should have bold 'important' span. Lines: {:?}",
                lines
            );
        }

        #[test]
        fn markdown_italic_text_renders_with_italic_modifier() {
            // Given TuiStreamHandler
            let mut handler = TuiStreamHandler::new(false);

            // When on_text("*emphasized*\n") is called
            handler.on_text("*emphasized*\n");

            // Then the text "emphasized" appears with ITALIC modifier
            let lines = collect_lines(&handler);
            assert!(!lines.is_empty(), "Should have at least one line");

            let has_italic = lines.iter().any(|line| {
                line.spans.iter().any(|span| {
                    span.content.contains("emphasized")
                        && span.style.add_modifier.contains(Modifier::ITALIC)
                })
            });
            assert!(
                has_italic,
                "Should have italic 'emphasized' span. Lines: {:?}",
                lines
            );
        }

        #[test]
        fn markdown_inline_code_renders_with_distinct_style() {
            // Given TuiStreamHandler
            let mut handler = TuiStreamHandler::new(false);

            // When on_text("`code`\n") is called
            handler.on_text("`code`\n");

            // Then the text "code" appears with distinct styling (different from default)
            let lines = collect_lines(&handler);
            assert!(!lines.is_empty(), "Should have at least one line");

            let has_code_style = lines.iter().any(|line| {
                line.spans.iter().any(|span| {
                    span.content.contains("code")
                        && (span.style.fg.is_some() || span.style.bg.is_some())
                })
            });
            assert!(
                has_code_style,
                "Should have styled 'code' span. Lines: {:?}",
                lines
            );
        }

        #[test]
        fn markdown_inline_code_uses_sublime_monokai_extended_palette() {
            // 目的：锁定 “sublime-monokai-extended” 的关键配色不会被未来改回 termimad 默认灰度。
            //
            // 说明：
            // - 由于此仓库测试环境默认设置了 `NO_COLOR=1`，crossterm 会抑制彩色 ANSI 输出。
            // - 因此这里不通过“渲染后的 ANSI → 解析后的 Span 样式”来断言颜色，
            //   而是直接断言 `MadSkin` 内部配置，保证主题映射本身不会回归。
            //
            // - 前景：#78dce8（基色；叠加 3% #4493f8 后约为 #76dae8）
            // - 背景：None（按需求取消铺底）
            use termimad::crossterm::style::Color as TermimadColor;
            let skin = default_markdown_skin();

            assert_eq!(
                skin.inline_code.get_fg(),
                Some(TermimadColor::Rgb {
                    r: 0x76,
                    g: 0xda,
                    b: 0xe8
                }),
                "Inline code fg should be ~#76dae8 (after 3% #4493f8 mix)"
            );
            assert_eq!(
                skin.inline_code.get_bg(),
                None,
                "Inline code bg should be None (no background)"
            );
        }

        #[test]
        fn markdown_fenced_code_block_uses_sublime_monokai_extended_palette() {
            // 目的：确保 fenced code block 的“前景色正确 + 背景被取消”，避免回退到 termimad 默认铺底。
            //
            // - 前景：#78dce8（基色；叠加 3% #4493f8 后约为 #76dae8）
            // - 背景：None（按需求取消铺底）
            use termimad::crossterm::style::Color as TermimadColor;
            let skin = default_markdown_skin();

            assert_eq!(
                skin.code_block.compound_style.get_fg(),
                Some(TermimadColor::Rgb {
                    r: 0x76,
                    g: 0xda,
                    b: 0xe8
                }),
                "Code block fg should be ~#76dae8 (after 3% #4493f8 mix)"
            );
            assert_eq!(
                skin.code_block.compound_style.get_bg(),
                None,
                "Code block bg should be None (no background)"
            );
        }

        #[test]
        fn markdown_heading_uses_custom_orange() {
            // 目的：锁定 heading（H2+）基色为你指定的 #fc9867，并验证 3% #4493f8 混合后的最终值。
            use termimad::crossterm::style::Color as TermimadColor;
            let skin = default_markdown_skin();

            assert_eq!(
                skin.headers[1].compound_style.get_fg(),
                Some(TermimadColor::Rgb {
                    r: 0xf6,
                    g: 0x98,
                    b: 0x6b
                }),
                "Heading fg should be ~#f6986b (after 3% #4493f8 mix)"
            );
        }

        #[test]
        fn markdown_h1_uses_custom_yellow() {
            // 目的：锁定标题（H1）基色为你指定的 #ffd866，并验证 3% #4493f8 混合后的最终值。
            use termimad::crossterm::style::Color as TermimadColor;
            let skin = default_markdown_skin();

            assert_eq!(
                skin.headers[0].compound_style.get_fg(),
                Some(TermimadColor::Rgb {
                    r: 0xf9,
                    g: 0xd6,
                    b: 0x6a
                }),
                "H1 fg should be ~#f9d66a (after 3% #4493f8 mix)"
            );
        }

        #[test]
        fn markdown_bold_uses_custom_green() {
            // 目的：锁定“强调/标签类（**bold**）”基色为你指定的 #a9dc76，并验证 3% #4493f8 混合后的最终值。
            use termimad::crossterm::style::Color as TermimadColor;
            let skin = default_markdown_skin();

            assert_eq!(
                skin.bold.get_fg(),
                Some(TermimadColor::Rgb {
                    r: 0xa6,
                    g: 0xda,
                    b: 0x7a
                }),
                "Bold fg should be ~#a6da7a (after 3% #4493f8 mix)"
            );
        }

        #[test]
        fn markdown_header_renders_content() {
            // Given TuiStreamHandler
            let mut handler = TuiStreamHandler::new(false);

            // When on_text("## Section Title\n") is called
            handler.on_text("## Section Title\n");

            // Then "Section Title" appears in the output
            // Note: termimad applies ANSI styling to headers
            let lines = collect_lines(&handler);
            assert!(!lines.is_empty(), "Should have at least one line");

            let has_header_content = lines.iter().any(|line| {
                line.spans
                    .iter()
                    .any(|span| span.content.contains("Section Title"))
            });
            assert!(
                has_header_content,
                "Should have header content. Lines: {:?}",
                lines
            );
        }

        #[test]
        fn markdown_h1_is_left_aligned_in_rendered_mode() {
            // 目的：防止 termimad 默认 H1 居中（左侧填充空格）的行为回归。
            //
            // 说明：
            // - 我们在 `default_markdown_skin()` 里把 `headers[0].align` 改为 `Left`，
            //   以提升日志/代码输出的可读性和复制粘贴后的对齐体验。
            let lines = render_text_to_lines("# Title\n", MarkdownRenderMode::Rendered, 20);

            let title_line = lines
                .iter()
                .find(|l| l.to_string().contains("Title"))
                .expect("Should render H1 content line");

            let text = title_line.to_string();
            assert!(
                text.starts_with("Title"),
                "H1 should be left aligned (no leading spaces). Got: {text:?}"
            );
        }

        #[test]
        fn markdown_plain_mode_keeps_control_symbols_visible() {
            // Given TuiStreamHandler in Plain mode
            let mut handler = TuiStreamHandler::new_with_mode(false, MarkdownRenderMode::Plain);

            // When markdown text is streamed
            handler.on_text("## Section Title\n> quoted\n```rust\nlet x = 1;\n```\n");

            // Then markdown control symbols remain visible (not rendered away)
            let lines = collect_lines(&handler);
            let text = lines
                .iter()
                .map(|l| l.to_string())
                .collect::<Vec<_>>()
                .join("\n");

            assert!(
                text.contains("## Section Title"),
                "Plain should keep header markers: {text}"
            );
            assert!(
                text.contains("> quoted"),
                "Plain should keep blockquote markers: {text}"
            );
            assert!(
                text.contains("```"),
                "Plain should keep fence markers: {text}"
            );
        }

        #[test]
        fn markdown_rendered_mode_hides_control_symbols_best_effort() {
            // Given TuiStreamHandler in Rendered mode
            let mut handler = TuiStreamHandler::new_with_mode(false, MarkdownRenderMode::Rendered);

            // When markdown text is streamed
            handler.on_text("## Section Title\n> quoted\n```rust\nlet x = 1;\n```\n");

            // Then the content remains, but control symbols are not shown verbatim.
            let lines = collect_lines(&handler);
            let text = lines
                .iter()
                .map(|l| l.to_string())
                .collect::<Vec<_>>()
                .join("\n");

            assert!(
                text.contains("Section Title"),
                "Rendered should keep content: {text}"
            );
            assert!(
                text.contains("quoted"),
                "Rendered should keep content: {text}"
            );
            assert!(
                text.contains("let x = 1;"),
                "Rendered should keep content: {text}"
            );

            assert!(
                !text.contains("## "),
                "Rendered should hide header markers: {text}"
            );
            assert!(
                !text.contains("> quoted"),
                "Rendered should hide blockquote markers: {text}"
            );
            assert!(
                !text.contains("```"),
                "Rendered should hide fence markers: {text}"
            );
        }

        #[test]
        fn markdown_streaming_continuity_handles_split_formatting() {
            // Given TuiStreamHandler
            let mut handler = TuiStreamHandler::new(false);

            // When markdown arrives in chunks: "**bo" then "ld**\n"
            handler.on_text("**bo");
            handler.on_text("ld**\n");

            // Then the complete "bold" text renders with BOLD modifier
            let lines = collect_lines(&handler);

            let has_bold = lines.iter().any(|line| {
                line.spans
                    .iter()
                    .any(|span| span.style.add_modifier.contains(Modifier::BOLD))
            });
            assert!(
                has_bold,
                "Split markdown should still render bold. Lines: {:?}",
                lines
            );
        }

        #[test]
        fn markdown_mixed_content_renders_correctly() {
            // Given TuiStreamHandler
            let mut handler = TuiStreamHandler::new(false);

            // When on_text() receives mixed markdown
            handler.on_text("Normal **bold** and *italic* text\n");

            // Then appropriate spans have appropriate styling
            let lines = collect_lines(&handler);
            assert!(!lines.is_empty(), "Should have at least one line");

            let has_bold = lines.iter().any(|line| {
                line.spans.iter().any(|span| {
                    span.content.contains("bold")
                        && span.style.add_modifier.contains(Modifier::BOLD)
                })
            });
            let has_italic = lines.iter().any(|line| {
                line.spans.iter().any(|span| {
                    span.content.contains("italic")
                        && span.style.add_modifier.contains(Modifier::ITALIC)
                })
            });

            assert!(has_bold, "Should have bold span. Lines: {:?}", lines);
            assert!(has_italic, "Should have italic span. Lines: {:?}", lines);
        }

        #[test]
        fn markdown_tool_call_styling_preserved() {
            // Given TuiStreamHandler with markdown text then tool call
            let mut handler = TuiStreamHandler::new(false);

            // When markdown text followed by tool call
            handler.on_text("**bold**\n");
            handler.on_tool_call("Read", "id", &json!({"file_path": "src/main.rs"}));

            // Then tool call still has blue styling
            let lines = collect_lines(&handler);
            assert!(lines.len() >= 2, "Should have at least 2 lines");

            // Last line should be the tool call with blue color
            let tool_line = lines.last().unwrap();
            let has_blue = tool_line
                .spans
                .iter()
                .any(|span| span.style.fg == Some(Color::Blue));
            assert!(
                has_blue,
                "Tool call should preserve blue styling. Line: {:?}",
                tool_line
            );
        }

        #[test]
        fn markdown_error_styling_preserved() {
            // Given TuiStreamHandler with markdown text then error
            let mut handler = TuiStreamHandler::new(false);

            // When markdown text followed by error
            handler.on_text("**bold**\n");
            handler.on_error("Something went wrong");

            // Then error still has red styling
            let lines = collect_lines(&handler);
            assert!(lines.len() >= 2, "Should have at least 2 lines");

            // Last line should be the error with red color
            let error_line = lines.last().unwrap();
            let has_red = error_line
                .spans
                .iter()
                .any(|span| span.style.fg == Some(Color::Red));
            assert!(
                has_red,
                "Error should preserve red styling. Line: {:?}",
                error_line
            );
        }

        #[test]
        fn markdown_partial_formatting_does_not_crash() {
            // Given TuiStreamHandler
            let mut handler = TuiStreamHandler::new(false);

            // When incomplete markdown is sent and flushed
            handler.on_text("**unclosed bold");
            handler.flush_text_buffer();

            // Then no panic occurs and text is not lost（安全降级）
            let lines = collect_lines(&handler);
            let text = lines
                .iter()
                .map(|l| l.to_string())
                .collect::<Vec<_>>()
                .join("\n");
            assert!(
                text.contains("unclosed bold"),
                "Partial markdown should not lose content: {text}"
            );
        }

        #[test]
        fn markdown_unclosed_fenced_code_block_safely_degrades() {
            // 这个用例对应 OpenSpec：
            // - Requirement: 渲染失败必须安全降级
            // - Scenario: 不完整 fenced code block 不导致崩溃
            //
            // 我们不强制要求“必须显示 ``` 控制符”，因为 Rendered 模式允许 best-effort 隐藏控制符。
            // 但必须保证：不 panic，且 fenced code 的实际内容不丢失。

            // Given TuiStreamHandler
            let mut handler = TuiStreamHandler::new(false);

            // When a fenced code block starts but never closes
            handler.on_text("```rust\nlet x = 1;\n");
            handler.flush_text_buffer();

            // Then no panic occurs and the code content is still visible.
            let lines = collect_lines(&handler);
            let text = lines
                .iter()
                .map(|l| l.to_string())
                .collect::<Vec<_>>()
                .join("\n");
            assert!(
                text.contains("let x = 1;"),
                "Unclosed fenced code block should keep content: {text}"
            );
        }

        // =====================================================================
        // ANSI Color Preservation Tests
        // =====================================================================

        #[test]
        fn ansi_output_skips_markdown_parsing_and_preserves_styles() {
            // 这个用例对应 OpenSpec：
            // - Requirement: ANSI 输出优先保留样式
            // - Scenario: 含 ANSI 的输出不做 Markdown 渲染
            //
            // 关键断言：
            // 1) ANSI 的颜色样式被保留（这里用 Red）
            // 2) 因为“检测到 ANSI → 跳过 Markdown 渲染”，所以 `**` 这类 Markdown 控制符必须原样可见，
            //    且不应凭空出现“由 Markdown 渲染产生”的 BOLD 样式（除非 ANSI 自己声明了 bold）。

            // Given TuiStreamHandler (Rendered mode by default)
            let mut handler = TuiStreamHandler::new(false);

            // When text contains ANSI *and* Markdown markers
            handler.on_text("\x1b[31m**bold**\x1b[0m\n");

            let lines = collect_lines(&handler);
            let text = lines
                .iter()
                .map(|l| l.to_string())
                .collect::<Vec<_>>()
                .join("\n");

            assert!(
                text.contains("**bold**"),
                "ANSI path should not strip markdown markers: {text}"
            );

            let has_red = lines.iter().any(|line| {
                line.spans
                    .iter()
                    .any(|span| span.style.fg == Some(Color::Red))
            });
            assert!(
                has_red,
                "ANSI red style should be preserved. Lines: {lines:?}"
            );

            let has_bold_modifier = lines.iter().any(|line| {
                line.spans.iter().any(|span| {
                    span.content.contains("bold")
                        && span.style.add_modifier.contains(Modifier::BOLD)
                })
            });
            assert!(
                !has_bold_modifier,
                "ANSI-only input should not gain markdown bold modifier. Lines: {lines:?}"
            );
        }

        #[test]
        fn ansi_green_text_produces_green_style() {
            // Given TuiStreamHandler
            let mut handler = TuiStreamHandler::new(false);

            // When on_text receives ANSI green text
            handler.on_text("\x1b[32mgreen text\x1b[0m\n");

            // Then the text should have green foreground color
            let lines = collect_lines(&handler);
            assert!(!lines.is_empty(), "Should have at least one line");

            let has_green = lines.iter().any(|line| {
                line.spans
                    .iter()
                    .any(|span| span.style.fg == Some(Color::Green))
            });
            assert!(
                has_green,
                "Should have green styled span. Lines: {:?}",
                lines
            );
        }

        #[test]
        fn ansi_bold_text_produces_bold_modifier() {
            // Given TuiStreamHandler
            let mut handler = TuiStreamHandler::new(false);

            // When on_text receives ANSI bold text
            handler.on_text("\x1b[1mbold text\x1b[0m\n");

            // Then the text should have BOLD modifier
            let lines = collect_lines(&handler);
            assert!(!lines.is_empty(), "Should have at least one line");

            let has_bold = lines.iter().any(|line| {
                line.spans
                    .iter()
                    .any(|span| span.style.add_modifier.contains(Modifier::BOLD))
            });
            assert!(has_bold, "Should have bold styled span. Lines: {:?}", lines);
        }

        #[test]
        fn ansi_mixed_styles_preserved() {
            // Given TuiStreamHandler
            let mut handler = TuiStreamHandler::new(false);

            // When on_text receives mixed ANSI styles (bold + green)
            handler.on_text("\x1b[1;32mbold green\x1b[0m normal\n");

            // Then the text should have appropriate styles
            let lines = collect_lines(&handler);
            assert!(!lines.is_empty(), "Should have at least one line");

            // Check for green color
            let has_styled = lines.iter().any(|line| {
                line.spans.iter().any(|span| {
                    span.style.fg == Some(Color::Green)
                        || span.style.add_modifier.contains(Modifier::BOLD)
                })
            });
            assert!(
                has_styled,
                "Should have styled span with color or bold. Lines: {:?}",
                lines
            );
        }

        #[test]
        fn ansi_plain_text_renders_without_crash() {
            // Given TuiStreamHandler
            let mut handler = TuiStreamHandler::new(false);

            // When on_text receives plain text (no ANSI)
            handler.on_text("plain text without ansi\n");

            // Then text renders normally (fallback to markdown)
            let lines = collect_lines(&handler);
            assert!(!lines.is_empty(), "Should have at least one line");

            let full_text: String = lines.iter().map(|l| l.to_string()).collect();
            assert!(
                full_text.contains("plain text"),
                "Should contain the text. Lines: {:?}",
                lines
            );
        }

        #[test]
        fn ansi_red_error_text_produces_red_style() {
            // Given TuiStreamHandler
            let mut handler = TuiStreamHandler::new(false);

            // When on_text receives ANSI red text (like error output)
            handler.on_text("\x1b[31mError: something failed\x1b[0m\n");

            // Then the text should have red foreground color
            let lines = collect_lines(&handler);
            assert!(!lines.is_empty(), "Should have at least one line");

            let has_red = lines.iter().any(|line| {
                line.spans
                    .iter()
                    .any(|span| span.style.fg == Some(Color::Red))
            });
            assert!(has_red, "Should have red styled span. Lines: {:?}", lines);
        }

        #[test]
        fn ansi_cyan_text_produces_cyan_style() {
            // Given TuiStreamHandler
            let mut handler = TuiStreamHandler::new(false);

            // When on_text receives ANSI cyan text
            handler.on_text("\x1b[36mcyan text\x1b[0m\n");

            // Then the text should have cyan foreground color
            let lines = collect_lines(&handler);
            assert!(!lines.is_empty(), "Should have at least one line");

            let has_cyan = lines.iter().any(|line| {
                line.spans
                    .iter()
                    .any(|span| span.style.fg == Some(Color::Cyan))
            });
            assert!(has_cyan, "Should have cyan styled span. Lines: {:?}", lines);
        }

        #[test]
        fn ansi_underline_produces_underline_modifier() {
            // Given TuiStreamHandler
            let mut handler = TuiStreamHandler::new(false);

            // When on_text receives ANSI underlined text
            handler.on_text("\x1b[4munderlined\x1b[0m\n");

            // Then the text should have UNDERLINED modifier
            let lines = collect_lines(&handler);
            assert!(!lines.is_empty(), "Should have at least one line");

            let has_underline = lines.iter().any(|line| {
                line.spans
                    .iter()
                    .any(|span| span.style.add_modifier.contains(Modifier::UNDERLINED))
            });
            assert!(
                has_underline,
                "Should have underlined styled span. Lines: {:?}",
                lines
            );
        }

        #[test]
        fn ansi_multiline_preserves_colors() {
            // Given TuiStreamHandler
            let mut handler = TuiStreamHandler::new(false);

            // When on_text receives multiple ANSI-colored lines
            handler.on_text("\x1b[32mline 1 green\x1b[0m\n\x1b[31mline 2 red\x1b[0m\n");

            // Then both colors should be present
            let lines = collect_lines(&handler);
            assert!(lines.len() >= 2, "Should have at least two lines");

            let has_green = lines.iter().any(|line| {
                line.spans
                    .iter()
                    .any(|span| span.style.fg == Some(Color::Green))
            });
            let has_red = lines.iter().any(|line| {
                line.spans
                    .iter()
                    .any(|span| span.style.fg == Some(Color::Red))
            });

            assert!(has_green, "Should have green line. Lines: {:?}", lines);
            assert!(has_red, "Should have red line. Lines: {:?}", lines);
        }
    }
}

// =========================================================================
// ANSI Detection Tests (module-level)
// =========================================================================

#[cfg(test)]
mod ansi_detection_tests {
    use super::*;

    #[test]
    fn contains_ansi_with_color_code() {
        assert!(contains_ansi("\x1b[32mgreen\x1b[0m"));
    }

    #[test]
    fn contains_ansi_with_bold() {
        assert!(contains_ansi("\x1b[1mbold\x1b[0m"));
    }

    #[test]
    fn contains_ansi_plain_text_returns_false() {
        assert!(!contains_ansi("hello world"));
    }

    #[test]
    fn contains_ansi_markdown_returns_false() {
        assert!(!contains_ansi("**bold** and *italic*"));
    }

    #[test]
    fn contains_ansi_empty_string_returns_false() {
        assert!(!contains_ansi(""));
    }

    #[test]
    fn contains_ansi_with_escape_in_middle() {
        assert!(contains_ansi("prefix \x1b[31mred\x1b[0m suffix"));
    }
}
