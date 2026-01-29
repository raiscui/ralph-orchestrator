//! 并行模式 TUI state（Supervisor TUI）。
//!
//! 设计目标（对齐 `openspec/changes/parallel-supervisor-tui/specs/*`）：
//! - 以 HatInstance 作为主视图维度：实例列表 → 实例详情。
//! - 以 HatJob 作为次级维度：实例内按 job 分段保存输出，便于回看与搜索。
//! - UI 只负责“展示 + 产生人类输入事件”，不把调度逻辑塞进 UI。

use crate::state::IterationBuffer;
use ralph_core::HatJobOutputChunk;
use ralph_proto::{
    Event, GateRequest, GateResolve, GateTimeout, HatInstanceId, HatInstanceState,
    TOPIC_GATE_REQUEST, TOPIC_GATE_RESOLVE, TOPIC_GATE_TIMEOUT,
};
use ratatui::style::{Color, Style};
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
        self.selection.map_or(false, |s| s.anchor != s.cursor)
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
    pub fn current_job_buffer(&self) -> Option<&IterationBuffer> {
        self.jobs.get(self.current_job).map(|j| &j.buffer)
    }

    /// 返回可变的当前 job buffer（若不存在则 None）。
    pub fn current_job_buffer_mut(&mut self) -> Option<&mut IterationBuffer> {
        self.jobs.get_mut(self.current_job).map(|j| &mut j.buffer)
    }
}

/// 单个 job 的视图状态（输出 + 时间线索）。
#[derive(Debug)]
pub struct JobViewState {
    /// 运行时 job id（来自并行运行时）。
    pub job_id: u64,
    /// 输出 buffer（复用现有 IterationBuffer 的滚动/渲染能力）。
    pub buffer: IterationBuffer,
    /// 该 job 首次出现输出的时间（best-effort）。
    pub first_output_at: Option<Instant>,
}

impl JobViewState {
    pub fn new(job_id: u64, number: u32) -> Self {
        Self {
            job_id,
            buffer: IterationBuffer::new(number),
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
            max_buffer_lines: 5_000,
        }
    }
}

impl ParallelTuiState {
    pub fn clear_output_selection(&mut self) {
        self.output_selection = None;
        self.output_selecting = false;
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
        self.register_instance(instance_id.clone(), HatInstanceState::Created);

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

            // 注意：并行模式输出需要带 stream 线索（避免 stderr 混淆）。
            let prefix = match chunk.stream {
                ralph_core::OutputStream::Stdout => "",
                ralph_core::OutputStream::Stderr => "[stderr] ",
            };
            let content = format!("{prefix}{}", chunk.line);

            // stderr 用灰色显示，避免和 stdout 混在一起时抢眼。
            let line = match chunk.stream {
                ralph_core::OutputStream::Stdout => Line::from(content),
                ralph_core::OutputStream::Stderr => Line::from(vec![Span::styled(
                    content,
                    Style::default().fg(Color::DarkGray),
                )]),
            };

            job.buffer.append_line_capped(line, self.max_buffer_lines);
        }
    }

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
    use ralph_proto::{GateKind, GateResolvedBy};
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
}
