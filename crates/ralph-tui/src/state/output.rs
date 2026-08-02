//! 串行输出切片: TuiState 的 output 域。
//!
//! 说明:
//! - 迭代缓冲(浏览/滚动/选择/共享行句柄)独立成片。
//! - 串行/并行统一的 CurrentOutputBuffer 抽象留在壳(state.rs), 因为它要同时引用 parallel 域。

use ratatui::text::Line;
use std::sync::{Arc, Mutex};

use super::parallel::{ScreenPos, ScreenSelection};

#[derive(Debug)]
pub struct IterationBuffer {
    /// Iteration number (1-indexed for display)
    pub number: u32,
    /// Formatted lines of output (shared for streaming)
    pub lines: Arc<Mutex<Vec<Line<'static>>>>,
    /// Scroll position within this buffer
    pub scroll_offset: usize,
    /// Whether to auto-scroll to bottom as new content arrives.
    /// Starts true, becomes false when user scrolls up, restored when user
    /// scrolls to bottom (G key) or manually scrolls down to reach bottom.
    pub following_bottom: bool,
}

impl IterationBuffer {
    /// Creates a new buffer for the given iteration number.
    pub fn new(number: u32) -> Self {
        Self {
            number,
            lines: Arc::new(Mutex::new(Vec::new())),
            scroll_offset: 0,
            following_bottom: true, // Start following bottom for auto-scroll
        }
    }

    /// Returns a shared handle to the lines buffer for streaming.
    ///
    /// This allows stream handlers to write directly to the buffer,
    /// enabling real-time streaming to the TUI.
    pub fn lines_handle(&self) -> Arc<Mutex<Vec<Line<'static>>>> {
        Arc::clone(&self.lines)
    }

    /// Appends a line to the buffer.
    pub fn append_line(&mut self, line: Line<'static>) {
        if let Ok(mut lines) = self.lines.lock() {
            lines.push(line);
        }
    }

    /// 追加一行，并在超过上限时丢弃最旧的行（近似 ring buffer）。
    ///
    /// 说明：
    /// - 该策略用于“长跑并行输出”，避免内存无限增长。
    /// - 丢弃旧行时，需要同步调整 scroll_offset，避免越界。
    pub fn append_line_capped(&mut self, line: Line<'static>, max_lines: usize) {
        if max_lines == 0 {
            return;
        }

        let Ok(mut lines) = self.lines.lock() else {
            return;
        };

        lines.push(line);
        if lines.len() <= max_lines {
            return;
        }

        let overflow = lines.len().saturating_sub(max_lines);
        if overflow == 0 {
            return;
        }

        // 移除最旧的 overflow 行
        lines.drain(0..overflow);
        self.scroll_offset = self.scroll_offset.saturating_sub(overflow);
    }

    /// 替换整段输出内容，并在超过上限时丢弃最旧的行。
    ///
    /// 说明：
    /// - 并行 Supervisor TUI 需要“保留原始输出行 → 重新渲染”为 styled lines，
    ///   因此每次追加输出后，可能会对当前 job 的整段内容做一次全量重渲染。
    /// - 为了避免内存无限增长，这里同样需要一个按行上限的裁剪策略。
    pub fn replace_lines_capped(&mut self, mut new_lines: Vec<Line<'static>>, max_lines: usize) {
        if max_lines == 0 {
            return;
        }

        if new_lines.len() > max_lines {
            let overflow = new_lines.len().saturating_sub(max_lines);
            new_lines.drain(0..overflow);
            self.scroll_offset = self.scroll_offset.saturating_sub(overflow);
        }

        // 防御性：避免 scroll_offset 指向越界位置导致“看起来像空白输出”。
        if new_lines.is_empty() {
            self.scroll_offset = 0;
        } else {
            self.scroll_offset = self.scroll_offset.min(new_lines.len().saturating_sub(1));
        }

        if let Ok(mut lines) = self.lines.lock() {
            *lines = new_lines;
        }
    }

    /// Returns the total number of lines in the buffer.
    pub fn line_count(&self) -> usize {
        self.lines.lock().map(|l| l.len()).unwrap_or(0)
    }

    /// Returns a clone of the visible lines based on scroll offset and viewport height.
    ///
    /// Note: Returns owned Vec instead of slice due to interior mutability.
    pub fn visible_lines(&self, viewport_height: usize) -> Vec<Line<'static>> {
        let Ok(lines) = self.lines.lock() else {
            return Vec::new();
        };
        if lines.is_empty() {
            return Vec::new();
        }
        let start = self.scroll_offset.min(lines.len());
        let end = (start + viewport_height).min(lines.len());
        lines[start..end].to_vec()
    }

    /// Scrolls up by one line.
    /// Disables auto-scroll since user is moving away from bottom.
    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
        self.following_bottom = false;
    }

    /// Scrolls down by one line, respecting the viewport bounds.
    /// Re-enables auto-scroll if user reaches the bottom.
    pub fn scroll_down(&mut self, viewport_height: usize) {
        let max_scroll = self.max_scroll_offset(viewport_height);
        if self.scroll_offset < max_scroll {
            self.scroll_offset += 1;
        }
        // Re-enable following if user scrolled to or past the bottom
        if self.scroll_offset >= max_scroll {
            self.following_bottom = true;
        }
    }

    /// Scrolls to the top of the buffer.
    /// Disables auto-scroll since user is moving away from bottom.
    pub fn scroll_top(&mut self) {
        self.scroll_offset = 0;
        self.following_bottom = false;
    }

    /// Scrolls to the bottom of the buffer.
    /// Re-enables auto-scroll since user explicitly went to bottom.
    pub fn scroll_bottom(&mut self, viewport_height: usize) {
        self.scroll_offset = self.max_scroll_offset(viewport_height);
        self.following_bottom = true;
    }

    /// Calculates the maximum scroll offset for the given viewport height.
    fn max_scroll_offset(&self, viewport_height: usize) -> usize {
        self.lines
            .lock()
            .map(|l| l.len().saturating_sub(viewport_height))
            .unwrap_or(0)
    }
}

/// 串行输出域切片。
#[derive(Debug)]
pub struct OutputSlice {
    pub iterations: Vec<IterationBuffer>,
    pub current_view: usize,
    pub following_latest: bool,
    pub new_iteration_alert: Option<usize>,
    pub serial_output_cursor: ScreenPos,
    pub serial_output_selection: Option<ScreenSelection>,
    pub serial_output_selecting: bool,
    pub serial_output_status: Option<String>,
}

impl Default for OutputSlice {
    fn default() -> Self {
        Self {
            iterations: Vec::new(),
            current_view: 0,
            // 默认跟随最新迭代(与历史行为一致: TuiState::new 曾显式设为 true)
            following_latest: true,
            new_iteration_alert: None,
            serial_output_cursor: ScreenPos::default(),
            serial_output_selection: None,
            serial_output_selecting: false,
            serial_output_status: None,
        }
    }
}

impl OutputSlice {
    pub fn start_new_iteration(&mut self) {
        let number = (self.iterations.len() + 1) as u32;
        self.iterations.push(IterationBuffer::new(number));

        // Auto-follow if enabled
        if self.following_latest {
            self.current_view = self.iterations.len().saturating_sub(1);
        } else {
            // Alert user about new iteration when reviewing history
            self.new_iteration_alert = Some(number as usize);
        }
    }

    /// Returns a reference to the currently viewed iteration buffer.
    pub fn current_iteration(&self) -> Option<&IterationBuffer> {
        self.iterations.get(self.current_view)
    }

    /// Returns a mutable reference to the currently viewed iteration buffer.
    pub fn current_iteration_mut(&mut self) -> Option<&mut IterationBuffer> {
        self.iterations.get_mut(self.current_view)
    }

    pub fn clear_serial_output_selection(&mut self) {
        self.serial_output_selection = None;
        self.serial_output_selecting = false;
    }

    pub fn start_serial_output_selection(&mut self, pos: ScreenPos) {
        self.serial_output_cursor = pos;
        self.serial_output_selection = Some(ScreenSelection::new(pos, pos));
        self.serial_output_selecting = true;
    }

    pub fn update_serial_output_selection_cursor(&mut self, pos: ScreenPos) {
        self.serial_output_cursor = pos;
        if let Some(sel) = self.serial_output_selection.as_mut() {
            sel.cursor = pos;
        }
    }

    pub fn finish_serial_output_selection(&mut self) {
        self.serial_output_selecting = false;
    }

    pub fn extend_serial_output_selection_by_delta(
        &mut self,
        dx: i16,
        dy: i16,
        max_x: u16,
        max_y: u16,
    ) {
        if max_x == 0 || max_y == 0 {
            return;
        }

        if self.serial_output_selection.is_none() {
            self.serial_output_selection = Some(ScreenSelection::new(
                self.serial_output_cursor,
                self.serial_output_cursor,
            ));
        }

        let Some(sel) = self.serial_output_selection.as_mut() else {
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
        self.serial_output_cursor = sel.cursor;
    }

    /// Returns a shared handle to the current iteration's lines buffer.
    ///
    /// This allows stream handlers to write directly to the buffer,
    /// enabling real-time streaming to the TUI during execution.
    pub fn current_iteration_lines_handle(
        &self,
    ) -> Option<std::sync::Arc<std::sync::Mutex<Vec<Line<'static>>>>> {
        self.iterations
            .get(self.current_view)
            .map(|buffer| buffer.lines_handle())
    }

    /// Returns a shared handle to the latest iteration's lines buffer.
    ///
    /// This should be used when writing output from the currently executing
    /// iteration, regardless of which iteration the user is viewing.
    /// This prevents output from being written to the wrong iteration when
    /// the user is reviewing an older iteration.
    pub fn latest_iteration_lines_handle(
        &self,
    ) -> Option<std::sync::Arc<std::sync::Mutex<Vec<Line<'static>>>>> {
        self.iterations.last().map(|buffer| buffer.lines_handle())
    }

    /// Navigates to the next iteration (if not at the last one).
    /// If reaching the last iteration, re-enables following_latest and clears alerts.
    pub fn navigate_next(&mut self) {
        if self.iterations.is_empty() {
            return;
        }
        let max_index = self.iterations.len().saturating_sub(1);
        if self.current_view < max_index {
            self.current_view += 1;
            // Re-enable following when reaching the latest
            if self.current_view == max_index {
                self.following_latest = true;
                self.new_iteration_alert = None;
            }
        }
    }

    /// Navigates to the previous iteration (if not at the first one).
    /// Disables following_latest when navigating backwards.
    pub fn navigate_prev(&mut self) {
        if self.current_view > 0 {
            self.current_view -= 1;
            self.following_latest = false;
        }
    }

    /// Returns the total number of iterations.
    pub fn total_iterations(&self) -> usize {
        self.iterations.len()
    }

}
