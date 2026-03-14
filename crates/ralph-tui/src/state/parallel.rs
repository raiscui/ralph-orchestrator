//! 并行模式 TUI state（Supervisor TUI）。
//!
//! 设计目标（对齐 `openspec/changes/parallel-supervisor-tui/specs/*`）：
//! - 以 HatInstance 作为主视图维度：实例列表 → 实例详情。
//! - 以 HatJob 作为次级维度：实例内按 job 分段保存输出，便于回看与搜索。
//! - UI 只负责“展示 + 产生人类输入事件”，不把调度逻辑塞进 UI。

#[path = "parallel/output.rs"]
pub(crate) mod output;

use crate::theme::MUTED_FG;
use output::wrap_lines_to_width;
use ralph_adapters::{MarkdownRenderMode, render_text_to_lines};
use ralph_core::{HatJobOutputChunk, OutputStream};
use ralph_proto::{
    Event, GateRequest, GateResolve, GateTimeout, HatInstanceId, HatInstanceState,
    TOPIC_GATE_REQUEST, TOPIC_GATE_RESOLVE, TOPIC_GATE_TIMEOUT,
};
use ratatui::text::{Line, Span};
use std::collections::HashMap;
use std::time::Instant;
use tracing::warn;
use unicode_segmentation::UnicodeSegmentation;

/// 并行 TUI 的焦点区域（Tab 循环）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParallelFocus {
    /// 左侧实例列表。
    Instances,
    /// 右侧实例输出详情。
    Output,
    /// 底部 human chat / gate 输入。
    Chat,
}

/// 输出视图的“屏幕坐标”（相对 Output inner area）。
///
/// 说明：
/// - 这是第一版的最小模型：只关心屏幕上的 x/y，不尝试映射回逻辑文本坐标。
/// - 优点：实现简单，天然适配软换行与宽字符渲染；可满足“鼠标拖拽框选/多行选择”的可视化需求。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ScreenPos {
    pub x: u16,
    pub y: u16,
}

/// 输出视图的选择区域（anchor→cursor 形成矩形区域）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenSelection {
    pub anchor: ScreenPos,
    pub cursor: ScreenPos,
}

impl ScreenSelection {
    pub fn new(anchor: ScreenPos, cursor: ScreenPos) -> Self {
        Self { anchor, cursor }
    }

    pub fn bounds(&self) -> (u16, u16, u16, u16) {
        let min_x = self.anchor.x.min(self.cursor.x);
        let max_x = self.anchor.x.max(self.cursor.x);
        let min_y = self.anchor.y.min(self.cursor.y);
        let max_y = self.anchor.y.max(self.cursor.y);
        (min_x, max_x, min_y, max_y)
    }
}

/// Chat 输入框的逻辑光标位置（按“行 + grapheme 列”计数）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TextPos {
    pub row: usize,
    pub col: usize,
}

/// Chat 输入框的线性选择（anchor→cursor，end 为“光标位置”，按半开区间渲染）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextSelection {
    pub anchor: TextPos,
    pub cursor: TextPos,
}

impl TextSelection {
    pub fn new(anchor: TextPos, cursor: TextPos) -> Self {
        Self { anchor, cursor }
    }

    pub fn normalized(&self) -> (TextPos, TextPos) {
        if (self.anchor.row, self.anchor.col) <= (self.cursor.row, self.cursor.col) {
            (self.anchor, self.cursor)
        } else {
            (self.cursor, self.anchor)
        }
    }
}

/// 并行模式 Chat 输入编辑器（最小多行编辑模型）。
///
/// 设计目标：
/// - 多行输入（Shift+Enter 换行）
/// - 光标上下左右移动
/// - 线性选择（Shift+方向键 / 鼠标拖拽），并支持“输入替换所选内容”
#[derive(Debug, Clone)]
pub struct ChatEditorState {
    pub lines: Vec<String>,
    pub cursor: TextPos,
    pub selection: Option<TextSelection>,
    preferred_col: usize,
}

impl Default for ChatEditorState {
    fn default() -> Self {
        Self {
            lines: vec![String::new()],
            cursor: TextPos::default(),
            selection: None,
            preferred_col: 0,
        }
    }
}

impl ChatEditorState {
    pub fn is_empty(&self) -> bool {
        self.lines.len() == 1 && self.lines[0].is_empty()
    }

    pub fn clear(&mut self) {
        self.lines.clear();
        self.lines.push(String::new());
        self.cursor = TextPos::default();
        self.selection = None;
        self.preferred_col = 0;
    }

    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    pub fn clear_selection(&mut self) {
        self.selection = None;
    }

    pub fn has_selection(&self) -> bool {
        self.selection.is_some_and(|s| s.anchor != s.cursor)
    }

    fn line_grapheme_len(line: &str) -> usize {
        UnicodeSegmentation::graphemes(line, true).count()
    }

    fn grapheme_col_to_byte_idx(line: &str, col: usize) -> usize {
        if col == 0 {
            return 0;
        }
        for (i, (byte_idx, _g)) in UnicodeSegmentation::grapheme_indices(line, true).enumerate() {
            if i == col {
                return byte_idx;
            }
        }
        line.len()
    }

    fn clamp_cursor_in_bounds(&mut self) {
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        self.cursor.row = self.cursor.row.min(self.lines.len().saturating_sub(1));
        let line_len = Self::line_grapheme_len(&self.lines[self.cursor.row]);
        self.cursor.col = self.cursor.col.min(line_len);
    }

    fn delete_selection_if_any(&mut self) -> bool {
        let Some(sel) = self.selection else {
            return false;
        };

        let (start, end) = sel.normalized();
        if start == end {
            self.selection = None;
            return false;
        }

        self.cursor = start;

        if start.row == end.row {
            let line = &mut self.lines[start.row];
            let start_b = Self::grapheme_col_to_byte_idx(line, start.col);
            let end_b = Self::grapheme_col_to_byte_idx(line, end.col);
            if start_b <= end_b && end_b <= line.len() {
                line.replace_range(start_b..end_b, "");
            }
        } else {
            // 1) start 行：删除 start.col..end
            let start_prefix = {
                let line = &self.lines[start.row];
                let start_b = Self::grapheme_col_to_byte_idx(line, start.col);
                line[..start_b].to_string()
            };

            // 2) end 行：保留 0..end.col 之后的 suffix
            let end_suffix = {
                let line = &self.lines[end.row];
                let end_b = Self::grapheme_col_to_byte_idx(line, end.col);
                line[end_b..].to_string()
            };

            // 3) 合并为一行，并移除中间行
            self.lines[start.row] = format!("{start_prefix}{end_suffix}");
            let remove_count = end.row.saturating_sub(start.row);
            for _ in 0..remove_count {
                let idx = start.row + 1;
                if idx < self.lines.len() {
                    self.lines.remove(idx);
                }
            }
        }

        if self.lines.is_empty() {
            self.lines.push(String::new());
        }

        self.selection = None;
        self.preferred_col = self.cursor.col;
        true
    }

    pub fn insert_char(&mut self, c: char) {
        self.delete_selection_if_any();
        self.clamp_cursor_in_bounds();

        let line = &mut self.lines[self.cursor.row];
        let byte_idx = Self::grapheme_col_to_byte_idx(line, self.cursor.col);
        line.insert(byte_idx, c);
        self.cursor.col = self.cursor.col.saturating_add(1);
        self.preferred_col = self.cursor.col;
    }

    pub fn backspace(&mut self) {
        if self.delete_selection_if_any() {
            return;
        }
        self.clamp_cursor_in_bounds();

        if self.cursor.col > 0 {
            let line = &mut self.lines[self.cursor.row];
            let end_b = Self::grapheme_col_to_byte_idx(line, self.cursor.col);
            let start_b = Self::grapheme_col_to_byte_idx(line, self.cursor.col.saturating_sub(1));
            if start_b < end_b && end_b <= line.len() {
                line.replace_range(start_b..end_b, "");
                self.cursor.col = self.cursor.col.saturating_sub(1);
                self.preferred_col = self.cursor.col;
            }
            return;
        }

        // 行首 backspace：把当前行合并到上一行
        if self.cursor.row > 0 {
            let current = self.lines.remove(self.cursor.row);
            self.cursor.row = self.cursor.row.saturating_sub(1);
            let prev_len = Self::line_grapheme_len(&self.lines[self.cursor.row]);
            self.lines[self.cursor.row].push_str(&current);
            self.cursor.col = prev_len;
            self.preferred_col = self.cursor.col;
        }
    }

    pub fn delete(&mut self) {
        if self.delete_selection_if_any() {
            return;
        }
        self.clamp_cursor_in_bounds();

        let line_len = Self::line_grapheme_len(&self.lines[self.cursor.row]);
        if self.cursor.col < line_len {
            let line = &mut self.lines[self.cursor.row];
            let start_b = Self::grapheme_col_to_byte_idx(line, self.cursor.col);
            let end_b = Self::grapheme_col_to_byte_idx(line, self.cursor.col.saturating_add(1));
            if start_b < end_b && end_b <= line.len() {
                line.replace_range(start_b..end_b, "");
            }
            return;
        }

        // 行尾 delete：把下一行合并到当前行
        let next_idx = self.cursor.row.saturating_add(1);
        if next_idx < self.lines.len() {
            let next = self.lines.remove(next_idx);
            self.lines[self.cursor.row].push_str(&next);
        }
    }

    pub fn insert_newline(&mut self) {
        self.delete_selection_if_any();
        self.clamp_cursor_in_bounds();

        let line = &mut self.lines[self.cursor.row];
        let byte_idx = Self::grapheme_col_to_byte_idx(line, self.cursor.col);
        let suffix = line[byte_idx..].to_string();
        line.truncate(byte_idx);

        let next_row = self.cursor.row.saturating_add(1);
        self.lines.insert(next_row, suffix);
        self.cursor.row = next_row;
        self.cursor.col = 0;
        self.preferred_col = 0;
    }

    pub fn move_left(&mut self, selecting: bool) {
        self.clamp_cursor_in_bounds();
        self.apply_selecting(selecting);

        if self.cursor.col > 0 {
            self.cursor.col = self.cursor.col.saturating_sub(1);
        } else if self.cursor.row > 0 {
            self.cursor.row = self.cursor.row.saturating_sub(1);
            self.cursor.col = Self::line_grapheme_len(&self.lines[self.cursor.row]);
        }

        self.preferred_col = self.cursor.col;
        self.apply_selection_cursor_if_needed(selecting);
    }

    pub fn move_right(&mut self, selecting: bool) {
        self.clamp_cursor_in_bounds();
        self.apply_selecting(selecting);

        let line_len = Self::line_grapheme_len(&self.lines[self.cursor.row]);
        if self.cursor.col < line_len {
            self.cursor.col = self.cursor.col.saturating_add(1);
        } else if self.cursor.row.saturating_add(1) < self.lines.len() {
            self.cursor.row = self.cursor.row.saturating_add(1);
            self.cursor.col = 0;
        }

        self.preferred_col = self.cursor.col;
        self.apply_selection_cursor_if_needed(selecting);
    }

    pub fn move_up(&mut self, selecting: bool) {
        self.clamp_cursor_in_bounds();
        self.apply_selecting(selecting);

        if self.cursor.row == 0 {
            self.apply_selection_cursor_if_needed(selecting);
            return;
        }

        self.cursor.row = self.cursor.row.saturating_sub(1);
        let line_len = Self::line_grapheme_len(&self.lines[self.cursor.row]);
        self.cursor.col = self.preferred_col.min(line_len);

        self.apply_selection_cursor_if_needed(selecting);
    }

    pub fn move_down(&mut self, selecting: bool) {
        self.clamp_cursor_in_bounds();
        self.apply_selecting(selecting);

        if self.cursor.row.saturating_add(1) >= self.lines.len() {
            self.apply_selection_cursor_if_needed(selecting);
            return;
        }

        self.cursor.row = self.cursor.row.saturating_add(1);
        let line_len = Self::line_grapheme_len(&self.lines[self.cursor.row]);
        self.cursor.col = self.preferred_col.min(line_len);

        self.apply_selection_cursor_if_needed(selecting);
    }

    fn apply_selecting(&mut self, selecting: bool) {
        if selecting {
            if self.selection.is_none() {
                self.selection = Some(TextSelection::new(self.cursor, self.cursor));
            }
        } else {
            self.selection = None;
        }
    }

    fn apply_selection_cursor_if_needed(&mut self, selecting: bool) {
        if !selecting {
            return;
        }
        if let Some(sel) = self.selection.as_mut() {
            sel.cursor = self.cursor;
            if sel.anchor == sel.cursor {
                self.selection = None;
            }
        }
    }

    pub fn set_cursor(&mut self, pos: TextPos, selecting: bool) {
        self.clamp_cursor_in_bounds();
        self.apply_selecting(selecting);

        self.cursor = pos;
        self.clamp_cursor_in_bounds();
        self.preferred_col = self.cursor.col;
        self.apply_selection_cursor_if_needed(selecting);
    }

    pub fn set_mouse_selection(&mut self, anchor: TextPos, cursor: TextPos) {
        self.cursor = cursor;
        self.clamp_cursor_in_bounds();
        self.preferred_col = self.cursor.col;
        if anchor == self.cursor {
            self.selection = None;
        } else {
            self.selection = Some(TextSelection::new(anchor, self.cursor));
        }
    }

    /// 返回某一行上的选择范围（以 grapheme 列计数，end 为排他）。
    pub fn selection_range_for_row(&self, row: usize) -> Option<(usize, usize)> {
        let sel = self.selection?;
        let (start, end) = sel.normalized();
        if start == end {
            return None;
        }

        if row < start.row || row > end.row {
            return None;
        }

        let line_len = self
            .lines
            .get(row)
            .map(|s| Self::line_grapheme_len(s))
            .unwrap_or(0);

        if start.row == end.row {
            if row != start.row {
                return None;
            }
            return Some((start.col.min(line_len), end.col.min(line_len)));
        }

        if row == start.row {
            return Some((start.col.min(line_len), line_len));
        }
        if row == end.row {
            return Some((0, end.col.min(line_len)));
        }

        Some((0, line_len))
    }
}

/// 并行模式下的实例视图状态。
#[derive(Debug)]
pub struct InstanceViewState {
    /// 实例运行态（Created/Idle/Running/Failed/Done）。
    pub state: HatInstanceState,
    /// 最近一次收到输出的时间（用于 UI 列表展示“是否卡住”）。
    pub last_output_at: Option<Instant>,
    /// 该实例的 job 历史（按 job_id 分段）。
    pub jobs: Vec<JobViewState>,
    /// 当前正在查看的 job 索引。
    pub current_job: usize,
}

impl InstanceViewState {
    pub fn new(state: HatInstanceState) -> Self {
        Self {
            state,
            last_output_at: None,
            jobs: Vec::new(),
            current_job: 0,
        }
    }

    /// 返回当前 job buffer（若不存在则 None）。
    pub fn current_job_buffer(&self) -> Option<&output::ParallelOutputBuffer> {
        self.jobs.get(self.current_job).map(|j| &j.buffer)
    }

    /// 返回可变的当前 job buffer（若不存在则 None）。
    pub fn current_job_buffer_mut(&mut self) -> Option<&mut output::ParallelOutputBuffer> {
        self.jobs.get_mut(self.current_job).map(|j| &mut j.buffer)
    }
}

/// 单个 job 的视图状态（输出 + 时间线索）。
#[derive(Debug)]
pub struct JobViewState {
    /// 运行时 job id（来自并行运行时）。
    pub job_id: u64,
    /// 输出 buffer（独立的 Text/Image 滚动模型）。
    pub buffer: output::ParallelOutputBuffer,
    /// 保存该 job 的“原始输出行”（stdout/stderr，按收到顺序）。
    ///
    /// 说明：
    /// - 并行模式的输出是“按行 chunk”推送的。
    /// - 为了支持 fenced code block 等跨行 Markdown 语义，需要保留原始行并在追加时重新渲染。
    /// - 这里做 best-effort：不追求极致增量渲染，但必须保证不崩溃、不丢内容。
    raw_lines: Vec<JobOutputLine>,
    /// 该 job 首次出现输出的时间（best-effort）。
    pub first_output_at: Option<Instant>,
}

/// Job 输出的原始行（用于再次渲染）。
#[derive(Debug, Clone)]
struct JobOutputLine {
    stream: OutputStream,
    line: String,
}

impl JobViewState {
    pub fn new(job_id: u64, number: u32) -> Self {
        Self {
            job_id,
            buffer: output::ParallelOutputBuffer::new(number),
            raw_lines: Vec::new(),
            first_output_at: None,
        }
    }
}

/// gate 的 UI 视图状态（open/timeout/resolve）。
#[derive(Debug, Clone)]
pub struct GateViewState {
    pub request: GateRequest,
    pub opened_at: Instant,
    pub timed_out: bool,
    pub resolved: Option<GateResolve>,
    pub last_timeout: Option<GateTimeout>,
}

/// gate 在 UI 中的展示状态（用于倒计时与颜色）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateStatus {
    /// 普通 gate（无超时），等待 human。
    Open,
    /// 超时 gate，仍在等待窗口内。
    Waiting { remaining_seconds: u64 },
    /// 已超时（可能仍未 resolve）。
    Timeout,
    /// 已 resolve（human/timeout 自决等）。
    Resolved,
}

impl GateViewState {
    pub fn new(request: GateRequest) -> Self {
        Self {
            request,
            opened_at: Instant::now(),
            timed_out: false,
            resolved: None,
            last_timeout: None,
        }
    }

    /// 计算在给定时间点下的展示状态（测试友好：可注入 now）。
    pub fn status_at(&self, now: Instant) -> GateStatus {
        if self.resolved.is_some() {
            return GateStatus::Resolved;
        }
        if self.timed_out {
            return GateStatus::Timeout;
        }

        let Some(secs) = self.request.timeout_seconds else {
            return GateStatus::Open;
        };
        if secs == 0 {
            return GateStatus::Timeout;
        }

        let elapsed = now.saturating_duration_since(self.opened_at).as_secs();
        let remaining = secs.saturating_sub(elapsed);
        if remaining == 0 {
            GateStatus::Timeout
        } else {
            GateStatus::Waiting {
                remaining_seconds: remaining,
            }
        }
    }
}

// ============================================================================
// 输出渲染器：把 raw_lines -> Line<'static>
// ============================================================================
//
// 说明：
// - 这个渲染器刻意不持有 `ParallelTuiState` 的整体可变引用。
// - 以“连续 chunk”为单位渲染，减少跨流语义的复杂度，同时保留原始顺序。
// - 彻底移除 Big Headers / 图片块等 `mdfried` 相关渲染特性。
// - stderr 的区分由上游渲染器通过样式（灰色）完成，不再使用左侧前缀列。
struct ParallelOutputRenderer {
    mode: MarkdownRenderMode,
    width: u16,
}

impl ParallelOutputRenderer {
    fn new(mode: MarkdownRenderMode, width: u16) -> Self {
        Self {
            mode,
            width: width.max(1),
        }
    }

    /// 把 job 的原始输出行渲染为可展示的“逻辑行”。
    ///
    /// 设计要点：
    /// - 输出顺序以 raw_lines 的时间顺序为准（stdout/stderr 交错也能保持相对顺序）；
    /// - 以“连续 chunk”为单位渲染：stdout/stderr 分段渲染，降低跨流语义复杂度；
    /// - stderr 不再使用左侧前缀列，仅做灰色弱化。
    fn render_job_output_document(&self, job: &JobViewState) -> Vec<Line<'static>> {
        let mut out: Vec<Line<'static>> = Vec::new();

        let mut stdout_chunk: Vec<String> = Vec::new();
        let mut stderr_chunk: Vec<String> = Vec::new();

        for raw in &job.raw_lines {
            match raw.stream {
                OutputStream::Stdout => {
                    if !stderr_chunk.is_empty() {
                        out.extend(self.render_stream_chunk(&stderr_chunk, true));
                        stderr_chunk.clear();
                    }
                    stdout_chunk.push(raw.line.clone());
                }
                OutputStream::Stderr => {
                    if !stdout_chunk.is_empty() {
                        out.extend(self.render_stream_chunk(&stdout_chunk, false));
                        stdout_chunk.clear();
                    }
                    stderr_chunk.push(raw.line.clone());
                }
            }
        }

        if !stdout_chunk.is_empty() {
            out.extend(self.render_stream_chunk(&stdout_chunk, false));
        }
        if !stderr_chunk.is_empty() {
            out.extend(self.render_stream_chunk(&stderr_chunk, true));
        }

        out
    }

    fn render_stream_chunk(&self, lines: &[String], muted: bool) -> Vec<Line<'static>> {
        if lines.is_empty() {
            return Vec::new();
        }

        let text = lines.join("\n");
        let contains_ansi = text.contains("\x1b[");

        // 单独处理“只有一个空行”的情况：否则会被 `render_text_to_lines` 的 empty fast-path 吃掉。
        let mut rendered = if text.is_empty() {
            vec![Line::from(String::new())]
        } else {
            render_text_to_lines(&text, self.mode, self.width)
        };

        // 说明:
        // - stderr 默认用 muted 灰色弱化,提高可读性。
        // - 但当文本本身包含 ANSI 转义序列时,这些 ANSI 往往承载“语义色彩”(例如 codex prompt transcript),
        //   如果强制覆盖 fg 会把色彩吞掉,导致用户看不到关键提示。
        // - 因此: 含 ANSI 时不再强制 muted,保留原始色彩。
        if muted && !contains_ansi {
            rendered = rendered
                .into_iter()
                .map(|line| {
                    let spans: Vec<Span<'static>> = line
                        .spans
                        .into_iter()
                        .map(|span| Span::styled(span.content, span.style.fg(MUTED_FG)))
                        .collect();
                    Line::from(spans)
                })
                .collect();
        }

        // 关键修复:
        // - Output buffer 的滚动单位必须尽量贴近“屏幕上的可见行”.
        // - 如果这里保留未换行的逻辑行,后续 widget 再 soft-wrap,
        //   `row_count()/scroll_offset` 就会低估真实高度,导致 reply 虽然已经到了 buffer 尾部,
        //   视口却还停在上面的 prompt 包裹行里。
        wrap_lines_to_width(rendered, self.width)
    }
}

/// 并行模式的整体 UI state（挂到 `TuiState.parallel` 下）。
#[derive(Debug)]
pub struct ParallelTuiState {
    pub focus: ParallelFocus,

    /// 实例表（为了稳定展示顺序，额外维护 order 列表）。
    pub instances: HashMap<HatInstanceId, InstanceViewState>,
    pub instance_order: Vec<HatInstanceId>,
    pub selected_instance: usize,

    /// Output 视图的当前“光标”（用于 Shift+方向键选择的起点）。
    pub output_cursor: ScreenPos,
    /// Output 视图的选择区域（用于高亮显示）。
    pub output_selection: Option<ScreenSelection>,
    /// 鼠标是否正在 Output 视图内按下并拖拽（用于 Drag/Up 事件的选择更新）。
    pub output_selecting: bool,

    /// human chat 输入框内容（多行编辑器）。
    pub chat_editor: ChatEditorState,
    /// 最近一次写入外部事件文件的结果提示（仅用于 UI 展示）。
    pub chat_status: Option<String>,

    /// open gates（按 gate_id 索引）。
    pub gates: HashMap<String, GateViewState>,
    pub gate_order: Vec<String>,
    /// 当前选中的 gate（用于展示 gate 详情与快捷 actions）。
    pub selected_gate: Option<String>,

    /// 输出渲染模式：
    /// - 默认 Rendered：更适合阅读 AI code agent 的 Markdown 输出。
    /// - `--plain` 会切换为 Plain：便于排障/复制粘贴/对齐旧行为。
    pub output_render_mode: MarkdownRenderMode,

    /// Markdown 语义换行宽度（仅用于 Rendered 模式下的 stdout 渲染）。
    ///
    /// 说明：
    /// - `ContentPane` 本身会做“按字符的软换行”，但它不会在换行后的新行补齐 blockquote bar、
    ///   list marker 等结构性前缀；
    /// - 因此在使用 `termimad` 渲染 Markdown 时，需要提前按“输出面板可用宽度”做语义化换行，
    ///   让每一行都带齐必要前缀，从而在 UI 上保持语义一致。
    pub output_render_width: u16,

    /// 输出 buffer 的最大行数（超过即丢弃最旧的行）。
    pub max_buffer_lines: usize,
}

impl Default for ParallelTuiState {
    fn default() -> Self {
        Self {
            focus: ParallelFocus::Instances,
            instances: HashMap::new(),
            instance_order: Vec::new(),
            selected_instance: 0,
            output_cursor: ScreenPos::default(),
            output_selection: None,
            output_selecting: false,
            chat_editor: ChatEditorState::default(),
            chat_status: None,
            gates: HashMap::new(),
            gate_order: Vec::new(),
            selected_gate: None,
            output_render_mode: MarkdownRenderMode::Rendered,
            output_render_width: 80,
            max_buffer_lines: 10_000,
        }
    }
}

impl ParallelTuiState {
    pub fn clear_output_selection(&mut self) {
        self.output_selection = None;
        self.output_selecting = false;
    }

    /// 更新 Markdown 渲染的换行宽度，并在宽度变化时重渲染所有 job buffer。
    ///
    /// 说明：
    /// - 终端 resize 会改变输出面板的可用宽度；
    /// - 若不重渲染，旧宽度下的“语义换行结果”会导致 blockquote/list 等前缀错位或视觉不一致；
    /// - 为保证体验稳定，这里在宽度变化时对所有 job 做一次 best-effort 重渲染。
    pub fn set_output_render_width(&mut self, width: u16) {
        let width = width.max(1);
        if self.output_render_width == width {
            return;
        }

        self.output_render_width = width;

        let max_buffer_lines = self.max_buffer_lines;
        let mode = self.output_render_mode;
        let width = self.output_render_width.max(1);

        let renderer = ParallelOutputRenderer::new(mode, width);

        for instance in self.instances.values_mut() {
            for job in &mut instance.jobs {
                let lines = renderer.render_job_output_document(job);
                job.buffer.replace_content_capped(lines, max_buffer_lines);
            }
        }
    }

    pub fn set_output_cursor(&mut self, pos: ScreenPos) {
        self.output_cursor = pos;
    }

    pub fn start_output_selection(&mut self, pos: ScreenPos) {
        self.output_cursor = pos;
        self.output_selection = Some(ScreenSelection::new(pos, pos));
        self.output_selecting = true;
    }

    pub fn update_output_selection_cursor(&mut self, pos: ScreenPos) {
        self.output_cursor = pos;
        if let Some(sel) = self.output_selection.as_mut() {
            sel.cursor = pos;
        }
    }

    pub fn finish_output_selection(&mut self) {
        self.output_selecting = false;
    }

    pub fn extend_output_selection_by_delta(&mut self, dx: i16, dy: i16, max_x: u16, max_y: u16) {
        if max_x == 0 || max_y == 0 {
            return;
        }

        if self.output_selection.is_none() {
            self.output_selection =
                Some(ScreenSelection::new(self.output_cursor, self.output_cursor));
        }

        let Some(sel) = self.output_selection.as_mut() else {
            return;
        };

        let next_x = i32::from(sel.cursor.x).saturating_add(i32::from(dx));
        let next_y = i32::from(sel.cursor.y).saturating_add(i32::from(dy));

        let clamped_x = next_x.clamp(0, i32::from(max_x.saturating_sub(1))) as u16;
        let clamped_y = next_y.clamp(0, i32::from(max_y.saturating_sub(1))) as u16;

        sel.cursor = ScreenPos {
            x: clamped_x,
            y: clamped_y,
        };
        self.output_cursor = sel.cursor;
    }

    /// 消费并行 Supervisor 的事件（用于 gate 面板等“控制面 UI”）。
    ///
    /// 说明：
    /// - 该入口只做 UI 需要的最小状态更新（不参与调度）。
    /// - 解析失败时 best-effort 忽略，并打日志；避免 UI 因外部输入格式问题崩溃。
    pub fn apply_event(&mut self, event: &Event) {
        match event.topic.as_str() {
            "human.message" => {
                // 说明:
                // - external `human.message` 表示“人类刚刚提交了一条输入”.
                // - 在并行模式下,Supervisor 可能会把发往 busy `ralph#1` 的消息改投到 `ralph#2`.
                // - 如果 UI 直到 `reply.human.message` 才更新,用户会体感为“第二条没反应”.
                // - 因此这里在消息被系统接收后,就先提示它实际排队到了哪个实例,
                //   并 best-effort 切过去,让后续输出从一开始就可见.
                if event.source.is_none() && event.source_instance.is_none() {
                    if let Some(target_instance) = event.target_instance.as_ref() {
                        self.chat_status = Some(format!("queued @{target_instance}"));

                        if self.select_instance_by_id(target_instance)
                            && let Some(view) = self.instances.get_mut(target_instance)
                            && let Some(buf) = view.current_job_buffer_mut()
                        {
                            // 对“刚提交的人类消息”,也尽量跟随到底部,
                            // 避免用户停留在旧位置时看不到新一轮输出.
                            buf.following_bottom = true;
                        }
                    } else {
                        self.chat_status = Some("queued human.message".to_string());
                    }
                }
            }
            "reply.human.message" => {
                // 说明:
                // - `reply.human.message` 是“给人看的回复消息”.
                // - 如果用户当前没选中该实例(或 Output 在别的实例上),很容易误以为“没有回复”.
                // - 因此这里把它做成一个可见的 status 提示(仅提示来源,不在 Chat 区域展示正文),
                //   并 best-effort 切换到来源实例,
                //   让 reply 在 Output 面板里“肉眼可见”(尤其是用户通过外部 `ralph emit` 注入时)。
                let source = event
                    .source_instance
                    .as_ref()
                    .map(|id| id.as_str())
                    .unwrap_or("unknown");
                self.chat_status = Some(format!("reply @{source} (见 Output)"));

                // best-effort: 自动切换到来源实例,避免用户停留在旧实例时体感为“没有回复”。
                if let Some(source_instance) = event.source_instance.as_ref()
                    && self.select_instance_by_id(source_instance)
                    && let Some(view) = self.instances.get_mut(source_instance)
                    && let Some(buf) = view.current_job_buffer_mut()
                {
                    // 关键点:
                    // - 用户可能曾经滚动离开底部(following_bottom=false)。
                    // - 对 reply 这种“强语义可见性”内容,我们把它拉回到底部,减少误判。
                    buf.following_bottom = true;
                }
            }
            TOPIC_GATE_REQUEST => match serde_json::from_str::<GateRequest>(&event.payload) {
                Ok(req) => self.upsert_gate_request(req),
                Err(e) => warn!(error = %e, "Failed to parse gate.request payload"),
            },
            TOPIC_GATE_TIMEOUT => match serde_json::from_str::<GateTimeout>(&event.payload) {
                Ok(timeout) => self.apply_gate_timeout(timeout),
                Err(e) => warn!(error = %e, "Failed to parse gate.timeout payload"),
            },
            TOPIC_GATE_RESOLVE => match serde_json::from_str::<GateResolve>(&event.payload) {
                Ok(resolve) => self.apply_gate_resolve(resolve),
                Err(e) => warn!(error = %e, "Failed to parse gate.resolve payload"),
            },
            _ => {}
        }
    }

    fn upsert_gate_request(&mut self, request: GateRequest) {
        let gate_id = request.gate_id.clone();
        let exists = self.gates.contains_key(&gate_id);
        if !exists {
            self.gate_order.push(gate_id.clone());
            self.gates.insert(gate_id, GateViewState::new(request));
            return;
        }

        if let Some(existing) = self.gates.get_mut(&gate_id) {
            // 保留 opened_at/timed_out/resolved 等运行态，只更新最新 request 内容（prompt 等）。
            existing.request = request;
        }
    }

    fn apply_gate_timeout(&mut self, timeout: GateTimeout) {
        let Some(g) = self.gates.get_mut(&timeout.gate_id) else {
            return;
        };
        g.timed_out = true;
        g.last_timeout = Some(timeout);
    }

    fn apply_gate_resolve(&mut self, resolve: GateResolve) {
        let Some(g) = self.gates.get_mut(&resolve.gate_id) else {
            return;
        };
        g.resolved = Some(resolve);
    }

    /// 切换到下一个焦点区域（Tab 循环）。
    pub fn focus_next(&mut self) {
        self.focus = match self.focus {
            ParallelFocus::Instances => ParallelFocus::Output,
            ParallelFocus::Output => ParallelFocus::Chat,
            ParallelFocus::Chat => ParallelFocus::Instances,
        };
    }

    /// 切换到上一个焦点区域（Shift+Tab / BackTab 循环）。
    pub fn focus_prev(&mut self) {
        self.focus = match self.focus {
            ParallelFocus::Instances => ParallelFocus::Chat,
            ParallelFocus::Output => ParallelFocus::Instances,
            ParallelFocus::Chat => ParallelFocus::Output,
        };
    }

    /// 选择下一个实例（循环）。
    pub fn select_next_instance(&mut self) {
        if self.instance_order.is_empty() {
            return;
        }
        self.selected_instance = (self.selected_instance + 1) % self.instance_order.len();
        self.clear_output_selection();
    }

    /// 选择上一个实例（循环）。
    pub fn select_prev_instance(&mut self) {
        if self.instance_order.is_empty() {
            return;
        }
        if self.selected_instance == 0 {
            self.selected_instance = self.instance_order.len() - 1;
        } else {
            self.selected_instance -= 1;
        }
        self.clear_output_selection();
    }

    /// 选择指定实例（按 HatInstanceId 精确匹配）。
    ///
    /// 返回值：
    /// - `true`：找到了该实例并完成切换
    /// - `false`：当前实例列表中不存在该 id（不做任何修改）
    pub fn select_instance_by_id(&mut self, id: &HatInstanceId) -> bool {
        let Some(idx) = self.instance_order.iter().position(|x| x == id) else {
            return false;
        };
        self.selected_instance = idx;
        self.clear_output_selection();
        true
    }

    /// 选择下一个 job（饱和到末尾，不循环）。
    pub fn select_next_job(&mut self) {
        {
            let Some(instance) = self.selected_instance_mut() else {
                return;
            };
            if instance.jobs.is_empty() {
                return;
            }
            let max = instance.jobs.len().saturating_sub(1);
            instance.current_job = (instance.current_job + 1).min(max);
        }
        self.clear_output_selection();
    }

    /// 选择上一个 job（饱和到 0，不循环）。
    pub fn select_prev_job(&mut self) {
        {
            let Some(instance) = self.selected_instance_mut() else {
                return;
            };
            if instance.jobs.is_empty() {
                return;
            }
            instance.current_job = instance.current_job.saturating_sub(1);
        }
        self.clear_output_selection();
    }

    /// 注册一个实例（若已存在则只做 best-effort 更新）。
    pub fn register_instance(&mut self, instance_id: HatInstanceId, state: HatInstanceState) {
        let exists = self.instances.contains_key(&instance_id);
        if !exists {
            self.instances
                .insert(instance_id.clone(), InstanceViewState::new(state));
            self.instance_order.push(instance_id);
            self.instance_order
                .sort_by(|a, b| a.as_str().cmp(b.as_str()));
        } else if let Some(s) = self.instances.get_mut(&instance_id) {
            s.state = state;
        }

        // 兜底：确保 selected 不越界
        if self.selected_instance >= self.instance_order.len() && !self.instance_order.is_empty() {
            self.selected_instance = self.instance_order.len() - 1;
        }
    }

    pub fn set_instance_state(&mut self, instance_id: HatInstanceId, state: HatInstanceState) {
        self.register_instance(instance_id.clone(), state);
        if let Some(s) = self.instances.get_mut(&instance_id) {
            s.state = state;
        }
    }

    pub fn append_output(&mut self, chunk: &HatJobOutputChunk) {
        let instance_id = chunk.instance_id.clone();
        // 关键点：
        // - output chunk 可能先于 instance state 更新到达；
        // - 但一旦实例已存在（例如已经是 Running/Idle/Done），这里绝不能把 state 覆盖回 Created，
        //   否则会导致 UI 状态回退（典型症状：Running 高亮“闪一下就没了”）。
        if !self.instances.contains_key(&instance_id) {
            self.register_instance(instance_id.clone(), HatInstanceState::Created);
        }

        let max_buffer_lines = self.max_buffer_lines;
        let mode = self.output_render_mode;
        let width = self.output_render_width.max(1);

        let Some(instance) = self.instances.get_mut(&instance_id) else {
            return;
        };

        instance.last_output_at = Some(Instant::now());

        // 根据 job_id 分段：最后一个 job 不同则新建
        let needs_new_job = instance
            .jobs
            .last()
            .map(|j| j.job_id != chunk.job_id)
            .unwrap_or(true);

        if needs_new_job {
            let number = u32::try_from(instance.jobs.len() + 1).unwrap_or(u32::MAX);
            instance.jobs.push(JobViewState::new(chunk.job_id, number));
            instance.current_job = instance.jobs.len().saturating_sub(1);
        }

        if let Some(job) = instance.jobs.last_mut() {
            if job.first_output_at.is_none() {
                job.first_output_at = Some(Instant::now());
            }

            // 特殊值：0 表示不保留输出（极端省内存/降噪）。
            // 关键点：不仅 buffer 要清空，raw_lines 也必须保持为空，
            // 否则会导致 raw_lines 无限增长，反而更耗内存。
            if max_buffer_lines == 0 {
                job.raw_lines.clear();
                job.buffer.replace_content_capped(Vec::new(), 0);
                return;
            }

            // 先保存原始行，再统一渲染（支持跨行 Markdown 语义，例如 fenced code block）。
            job.raw_lines.push(JobOutputLine {
                stream: chunk.stream,
                line: chunk.line.clone(),
            });

            // 控制原始输入的上限，避免长期运行导致内存无限增长。
            if max_buffer_lines > 0 && job.raw_lines.len() > max_buffer_lines {
                let overflow = job.raw_lines.len().saturating_sub(max_buffer_lines);
                job.raw_lines.drain(0..overflow);
            }

            // 重新渲染本 job 的所有可见输出（best-effort）。
            let renderer = ParallelOutputRenderer::new(mode, width);
            let lines = renderer.render_job_output_document(job);
            job.buffer.replace_content_capped(lines, max_buffer_lines);
        }
    }

    // 输出渲染逻辑已抽到 `ParallelOutputRenderer`，以便在遍历 instances 时避免 &mut self 的重入借用。

    pub fn selected_instance_id(&self) -> Option<&HatInstanceId> {
        self.instance_order.get(self.selected_instance)
    }

    pub fn selected_instance(&self) -> Option<&InstanceViewState> {
        self.selected_instance_id()
            .and_then(|id| self.instances.get(id))
    }

    pub fn selected_instance_mut(&mut self) -> Option<&mut InstanceViewState> {
        let id = self.selected_instance_id()?.clone();
        self.instances.get_mut(&id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::parallel_output::ParallelOutputPane;
    use ralph_proto::{GateKind, GateResolvedBy, HatId};
    use ratatui::style::Color;
    use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};
    use std::time::Duration;

    fn make_request(id: &str, timeout_seconds: Option<u64>) -> GateRequest {
        GateRequest {
            gate_id: id.to_string(),
            thread_id: None,
            requested_by: HatInstanceId::from("writer#1"),
            kind: GateKind::Consult,
            timeout_seconds,
            prompt: "need decision".to_string(),
            proposed_default: None,
        }
    }

    #[test]
    fn gate_status_waiting_and_timeout_are_deterministic() {
        let opened_at = Instant::now();
        let mut gate = GateViewState::new(make_request("g1", Some(10)));
        gate.opened_at = opened_at;

        assert_eq!(
            gate.status_at(opened_at + Duration::from_secs(3)),
            GateStatus::Waiting {
                remaining_seconds: 7
            }
        );
        assert_eq!(
            gate.status_at(opened_at + Duration::from_secs(10)),
            GateStatus::Timeout
        );
    }

    #[test]
    fn gate_status_resolved_overrides_timeout() {
        let opened_at = Instant::now();
        let mut gate = GateViewState::new(make_request("g1", Some(1)));
        gate.opened_at = opened_at;

        gate.resolved = Some(GateResolve {
            gate_id: "g1".to_string(),
            resolved_by: GateResolvedBy::Human,
            decision: serde_json::Value::Bool(true),
            requested_by: Some(HatInstanceId::from("writer#1")),
        });

        assert_eq!(
            gate.status_at(opened_at + Duration::from_secs(10)),
            GateStatus::Resolved
        );
    }

    #[test]
    fn apply_event_registers_and_updates_gate() {
        let mut state = ParallelTuiState::default();

        let req = make_request("g1", Some(60));
        let payload = serde_json::to_string(&req).unwrap();
        let event = Event::new(TOPIC_GATE_REQUEST, payload);
        state.apply_event(&event);

        assert!(state.gates.contains_key("g1"));
        assert_eq!(state.gate_order, vec!["g1".to_string()]);

        let timeout = GateTimeout {
            gate_id: "g1".to_string(),
            requested_by: Some(HatInstanceId::from("writer#1")),
        };
        let timeout_payload = serde_json::to_string(&timeout).unwrap();
        state.apply_event(&Event::new(TOPIC_GATE_TIMEOUT, timeout_payload));
        assert!(state.gates.get("g1").unwrap().timed_out);

        let resolve = GateResolve {
            gate_id: "g1".to_string(),
            resolved_by: GateResolvedBy::Human,
            decision: serde_json::Value::String("ok".to_string()),
            requested_by: Some(HatInstanceId::from("writer#1")),
        };
        let resolve_payload = serde_json::to_string(&resolve).unwrap();
        state.apply_event(&Event::new(TOPIC_GATE_RESOLVE, resolve_payload));
        assert!(state.gates.get("g1").unwrap().resolved.is_some());
    }

    #[test]
    fn apply_event_reply_human_message_sets_chat_status_preview() {
        let mut state = ParallelTuiState::default();

        // 先注册两个实例,并确保当前选中的是非 ralph#1。
        let other = HatInstanceId::from("builder#1");
        let ralph = HatInstanceId::from("ralph#1");
        state.register_instance(other.clone(), HatInstanceState::Created);
        state.register_instance(ralph.clone(), HatInstanceState::Created);
        assert_eq!(
            state.selected_instance_id(),
            Some(&other),
            "precondition: selected instance should be non-ralph"
        );

        // 给 ralph#1 预置一点输出,并模拟用户滚动离开底部。
        state.append_output(&HatJobOutputChunk {
            job_id: 1,
            instance_id: ralph.clone(),
            stream: OutputStream::Stdout,
            line: "prelude".to_string(),
        });
        {
            let view = state.instances.get_mut(&ralph).expect("ralph exists");
            let buf = view.current_job_buffer_mut().expect("job buffer exists");
            buf.scroll_top(); // sets following_bottom=false
            assert!(
                !buf.following_bottom,
                "precondition: should not follow bottom"
            );
        }

        let event = Event::new("reply.human.message", "hello").with_source_instance("ralph#1");
        state.apply_event(&event);

        let got = state
            .chat_status
            .as_deref()
            .expect("chat_status should be set for reply");
        assert!(
            got.contains("reply @ralph#1"),
            "expected status to include source instance, got: {got}"
        );
        assert!(
            !got.contains("hello"),
            "reply 正文不应出现在 Chat status,应该只在 Output 面板可见, got: {got}"
        );

        assert_eq!(
            state.selected_instance_id(),
            Some(&ralph),
            "reply 到达时应 best-effort 切换到来源实例,避免 reply 不可见"
        );

        let view = state.instances.get(&ralph).expect("ralph exists");
        let buf = view.current_job_buffer().expect("job buffer exists");
        assert!(
            buf.following_bottom,
            "reply 到达时应把 output 拉回到底部,避免用户误判为“没有回复”"
        );
    }

    #[test]
    fn apply_event_external_human_message_shows_queued_target_and_switches_instance() {
        let mut state = ParallelTuiState::default();

        let other = HatInstanceId::from("builder#1");
        let target = HatInstanceId::from("ralph#2");
        state.register_instance(other.clone(), HatInstanceState::Created);
        state.register_instance(target.clone(), HatInstanceState::Created);
        assert_eq!(
            state.selected_instance_id(),
            Some(&other),
            "precondition: selected instance should start on non-target"
        );

        state.append_output(&HatJobOutputChunk {
            job_id: 1,
            instance_id: target.clone(),
            stream: OutputStream::Stdout,
            line: "queued output".to_string(),
        });
        {
            let view = state.instances.get_mut(&target).expect("target exists");
            let buf = view.current_job_buffer_mut().expect("job buffer exists");
            buf.scroll_top();
            assert!(
                !buf.following_bottom,
                "precondition: should not follow bottom"
            );
        }

        let event = Event::new("human.message", "你能做什么").with_target_instance("ralph#2");
        state.apply_event(&event);

        assert_eq!(
            state.chat_status.as_deref(),
            Some("queued @ralph#2"),
            "external human.message 应该立刻告诉用户实际排队到了哪个实例"
        );
        assert_eq!(
            state.selected_instance_id(),
            Some(&target),
            "external human.message 到达时应 best-effort 切到实际 target_instance"
        );

        let view = state.instances.get(&target).expect("target exists");
        let buf = view.current_job_buffer().expect("job buffer exists");
        assert!(
            buf.following_bottom,
            "external human.message 到达后应把 Output 拉回到底部,便于看后续输出"
        );
    }

    #[test]
    fn apply_event_hat_sourced_human_message_does_not_override_chat_status() {
        let mut state = ParallelTuiState::default();
        state.chat_status = Some("existing".to_string());

        let event = Event::new("human.message", "internal")
            .with_source(HatId::new("ralph"))
            .with_source_instance("ralph#1")
            .with_target_instance("ralph#2");
        state.apply_event(&event);

        assert_eq!(
            state.chat_status.as_deref(),
            Some("existing"),
            "hat-sourced human.message 只是观测噪音,不应伪装成用户输入已排队"
        );
    }

    // =========================================================================
    // Chat Editor: 基础编辑/多行/选择
    // =========================================================================

    #[test]
    fn chat_editor_shift_enter_inserts_newline() {
        let mut editor = ChatEditorState::default();
        editor.insert_char('h');
        editor.insert_char('i');
        editor.insert_newline(); // Shift+Enter
        editor.insert_char('w');
        editor.insert_char('o');
        editor.insert_char('w');

        assert_eq!(editor.text(), "hi\nwow");
        assert_eq!(editor.cursor.row, 1);
        assert_eq!(editor.cursor.col, 3);
    }

    #[test]
    fn chat_editor_arrow_movement_crosses_lines() {
        let mut editor = ChatEditorState::default();
        editor.insert_char('a');
        editor.insert_char('b');
        editor.insert_newline();
        editor.insert_char('c');
        editor.insert_char('d');

        // cursor at end of "cd"
        assert_eq!(editor.cursor, TextPos { row: 1, col: 2 });

        editor.move_left(false);
        editor.move_left(false);
        assert_eq!(editor.cursor, TextPos { row: 1, col: 0 });

        // 行首左移：跳到上一行行尾
        editor.move_left(false);
        assert_eq!(editor.cursor, TextPos { row: 0, col: 2 });
    }

    #[test]
    fn chat_editor_selection_is_replaced_by_typing() {
        let mut editor = ChatEditorState::default();
        editor.insert_char('h');
        editor.insert_char('e');
        editor.insert_char('l');
        editor.insert_char('l');
        editor.insert_char('o');
        assert_eq!(editor.text(), "hello");

        // 选中 "hello"
        editor.set_mouse_selection(TextPos { row: 0, col: 0 }, TextPos { row: 0, col: 5 });
        assert!(editor.has_selection());

        // 输入替换所选内容
        editor.insert_char('X');
        assert_eq!(editor.text(), "X");
        assert_eq!(editor.cursor, TextPos { row: 0, col: 1 });
        assert!(!editor.has_selection());
    }

    // =========================================================================
    // Parallel Instance State
    // =========================================================================

    #[test]
    fn parallel_append_output_does_not_override_instance_state() {
        let mut state = ParallelTuiState::default();

        let instance_id = HatInstanceId::from("builder#1");

        // 先模拟 supervisor 把实例标记为 Running（这是 Radar 蓝色高亮的依据）。
        state.set_instance_state(instance_id.clone(), HatInstanceState::Running);

        // 再追加一次 output chunk：不应把 Running 覆盖回 Created。
        state.append_output(&HatJobOutputChunk {
            job_id: 1,
            instance_id: instance_id.clone(),
            stream: OutputStream::Stdout,
            line: "hello".to_string(),
        });

        let view = state
            .instances
            .get(&instance_id)
            .expect("instance must exist");
        assert_eq!(
            view.state,
            HatInstanceState::Running,
            "append_output 不应覆盖实例的生命周期状态"
        );
    }

    // =========================================================================
    // Parallel Output Rendering: Markdown Rendered / Plain
    // =========================================================================

    fn collect_latest_job_text(state: &ParallelTuiState, instance_id: &HatInstanceId) -> String {
        let instance = state
            .instances
            .get(instance_id)
            .expect("instance must exist");
        let job = instance.jobs.last().expect("job must exist");
        job.buffer
            .lines
            .iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn parallel_output_rendered_hides_markdown_markers_best_effort() {
        let mut state = ParallelTuiState::default();
        state.output_render_mode = MarkdownRenderMode::Rendered;

        let instance_id = HatInstanceId::from("writer#1");
        let job_id = 1;

        // Header + blank line
        state.append_output(&HatJobOutputChunk {
            job_id,
            instance_id: instance_id.clone(),
            stream: OutputStream::Stdout,
            line: "## Section Title".to_string(),
        });
        state.append_output(&HatJobOutputChunk {
            job_id,
            instance_id: instance_id.clone(),
            stream: OutputStream::Stdout,
            line: String::new(),
        });

        // Blockquote
        state.append_output(&HatJobOutputChunk {
            job_id,
            instance_id: instance_id.clone(),
            stream: OutputStream::Stdout,
            line: "> quoted".to_string(),
        });

        // Fenced code block
        state.append_output(&HatJobOutputChunk {
            job_id,
            instance_id: instance_id.clone(),
            stream: OutputStream::Stdout,
            line: "```rust".to_string(),
        });
        state.append_output(&HatJobOutputChunk {
            job_id,
            instance_id: instance_id.clone(),
            stream: OutputStream::Stdout,
            line: "let x = 1;".to_string(),
        });
        state.append_output(&HatJobOutputChunk {
            job_id,
            instance_id: instance_id.clone(),
            stream: OutputStream::Stdout,
            line: "```".to_string(),
        });

        let text = collect_latest_job_text(&state, &instance_id);
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

        // Best-effort expectations: markdown control markers should not be shown verbatim.
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
    fn parallel_output_rendered_shows_reply_human_message_payload_instead_of_event_wrapper() {
        // 关键回归:
        // - Rendered 模式下,`termimad` 会把 `<event ...>` 当作 HTML 吞掉.
        // - 对 `reply.human.message` 我们必须显示 payload,否则用户体感为“无回复”.
        let mut state = ParallelTuiState::default();
        state.output_render_mode = MarkdownRenderMode::Rendered;

        let instance_id = HatInstanceId::from("ralph#1");
        let job_id = 1;

        state.append_output(&HatJobOutputChunk {
            job_id,
            instance_id: instance_id.clone(),
            stream: OutputStream::Stdout,
            line: "<event topic=\"reply.human.message\" reply=\"E1\">hello</event>".to_string(),
        });

        let text = collect_latest_job_text(&state, &instance_id);
        assert!(
            text.contains("hello"),
            "Rendered should show payload: {text}"
        );
        assert!(
            !text.contains("<event"),
            "Rendered should hide event wrapper: {text}"
        );
    }

    #[test]
    fn reply_remains_visible_when_previous_output_lines_wrap_heavily() {
        // 回归目标:
        // - 复现用户截图里的真实模式:
        //   前面有大量 prompt/transcript 长行,reply 已经到达,chat_status 也更新了,
        //   但 Output 因为"逻辑行数"与"屏幕显示行数"不一致,仍卡在上面的包裹行里。
        // - 这里用一个很窄的 output 宽度复现该条件,锁死"到底后必须看见 reply"。
        let mut state = ParallelTuiState::default();
        state.output_render_mode = MarkdownRenderMode::Rendered;
        state.set_output_render_width(10);

        let instance_id = HatInstanceId::from("ralph#1");
        let job_id = 1;

        for line in ["ABCDEFGHIJKLMNO", "PQRSTUVWXYZABCD", "EFGHIJKLMNOPQRS"] {
            state.append_output(&HatJobOutputChunk {
                job_id,
                instance_id: instance_id.clone(),
                stream: OutputStream::Stderr,
                line: line.to_string(),
            });
        }

        state.append_output(&HatJobOutputChunk {
            job_id,
            instance_id: instance_id.clone(),
            stream: OutputStream::Stdout,
            line: "<event topic=\"reply.human.message\" reply=\"E1\">hello</event>".to_string(),
        });

        state.apply_event(
            &Event::new("reply.human.message", "hello").with_source_instance("ralph#1"),
        );

        let viewport_height = 4usize;
        {
            let view = state
                .instances
                .get_mut(&instance_id)
                .expect("instance exists");
            let buffer = view.current_job_buffer_mut().expect("job buffer exists");
            assert!(
                buffer.following_bottom,
                "reply event should force follow-bottom"
            );
            let max_scroll = buffer.row_count().saturating_sub(viewport_height);
            buffer.set_scroll_offset_clamped(max_scroll);
        }

        let view = state.instances.get(&instance_id).expect("instance exists");
        let buffer = view.current_job_buffer().expect("job buffer exists");

        let area = Rect::new(0, 0, 10, viewport_height as u16);
        let mut surface = Buffer::empty(area);
        ParallelOutputPane::new(buffer).render(area, &mut surface);

        let rendered = (0..area.height)
            .map(|y| {
                (0..area.width)
                    .map(|x| surface[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");

        assert!(
            rendered.contains("hello"),
            "bottom-followed output should contain the final reply, got:\n{rendered}"
        );
    }

    #[test]
    fn parallel_output_stderr_markdown_rendering_matches_renderer_output() {
        // 关键点：
        // - stderr 不应该在“正文”里被拼接任何前缀（例如 "[stderr]" / "E "）。
        // - 否则会破坏 Markdown 行首语义（标题/引用/列表等）。
        let mut state = ParallelTuiState::default();
        state.output_render_mode = MarkdownRenderMode::Rendered;

        let instance_id = HatInstanceId::from("writer#1");
        let job_id = 1;

        let input = ["## Section Title", "", "> quoted", "- item"].join("\n");
        for line in input.split('\n') {
            state.append_output(&HatJobOutputChunk {
                job_id,
                instance_id: instance_id.clone(),
                stream: OutputStream::Stderr,
                line: line.to_string(),
            });
        }

        let got = collect_latest_job_text(&state, &instance_id);
        let expected = render_text_to_lines(
            &input,
            MarkdownRenderMode::Rendered,
            state.output_render_width,
        )
        .iter()
        .map(|l| l.to_string())
        .collect::<Vec<_>>()
        .join("\n");

        assert_eq!(
            got, expected,
            "stderr 内容应与 markdown 渲染器输出一致（不应注入任何 stderr 前缀）"
        );
    }

    #[test]
    fn parallel_output_plain_keeps_markdown_control_symbols_visible() {
        let mut state = ParallelTuiState::default();
        state.output_render_mode = MarkdownRenderMode::Plain;

        let instance_id = HatInstanceId::from("writer#1");
        let job_id = 1;

        state.append_output(&HatJobOutputChunk {
            job_id,
            instance_id: instance_id.clone(),
            stream: OutputStream::Stdout,
            line: "## Section Title".to_string(),
        });
        state.append_output(&HatJobOutputChunk {
            job_id,
            instance_id: instance_id.clone(),
            stream: OutputStream::Stdout,
            line: "> quoted".to_string(),
        });
        state.append_output(&HatJobOutputChunk {
            job_id,
            instance_id: instance_id.clone(),
            stream: OutputStream::Stdout,
            line: "```rust".to_string(),
        });
        state.append_output(&HatJobOutputChunk {
            job_id,
            instance_id: instance_id.clone(),
            stream: OutputStream::Stdout,
            line: "let x = 1;".to_string(),
        });
        state.append_output(&HatJobOutputChunk {
            job_id,
            instance_id: instance_id.clone(),
            stream: OutputStream::Stdout,
            line: "```".to_string(),
        });

        let text = collect_latest_job_text(&state, &instance_id);
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
    fn parallel_output_stderr_with_ansi_is_not_force_muted() {
        let renderer = ParallelOutputRenderer::new(MarkdownRenderMode::Plain, 120);

        // stderr 上的 ANSI 色彩是“语义信息”(例如 prompt transcript),不能被 stderr-muted 强行覆盖掉。
        let rendered = renderer.render_stream_chunk(&["\x1b[31mRED\x1b[0m ok".to_string()], true);
        let has_red = rendered
            .iter()
            .flat_map(|line| line.spans.iter())
            .any(|span| span.style.fg == Some(Color::Red));

        assert!(
            has_red,
            "Expected ANSI red span to be preserved under stderr-muted: {rendered:?}"
        );
    }

    #[test]
    fn parallel_output_single_empty_stdout_line_is_preserved() {
        let mut state = ParallelTuiState::default();
        state.output_render_mode = MarkdownRenderMode::Plain;

        let instance_id = HatInstanceId::from("writer#1");
        let job_id = 1;

        state.append_output(&HatJobOutputChunk {
            job_id,
            instance_id: instance_id.clone(),
            stream: OutputStream::Stdout,
            line: String::new(),
        });

        let text = collect_latest_job_text(&state, &instance_id);
        assert_eq!(
            text, "",
            "Single empty line should be preserved as empty text"
        );
    }
}
