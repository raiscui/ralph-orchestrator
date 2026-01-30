//! 并行 Supervisor TUI：输出缓冲（按 job 分段）。
//!
//! 设计目标：
//! - 并行模式下，每个 job 都有独立的输出 buffer，便于回看与搜索；
//! - 只存储“逻辑行”（`ratatui::text::Line`），不引入 Big Headers / 图片块等额外渲染结构；
//! - stderr 与 stdout 的区分交给上游渲染器（例如把 stderr 行统一弱化成灰色）。
//!
//! 注意：
//! - 软换行（soft-wrap）依然在 widget 渲染阶段完成；
//! - 这里的 `scroll_offset` 是按“逻辑行”计数，而不是按渲染后的软换行行数计数。

use ratatui::text::Line;

/// 并行模式下的输出 buffer（纯文本行 + 滚动状态）。
#[derive(Debug)]
pub struct ParallelOutputBuffer {
    /// Job 序号（1-based，仅用于展示）。
    pub number: u32,
    /// 逻辑行列表（滚动单位）。
    pub lines: Vec<Line<'static>>,
    /// 当前滚动偏移（按 `lines` 计数）。
    pub scroll_offset: usize,
    /// 是否自动跟随底部。
    pub following_bottom: bool,
}

impl ParallelOutputBuffer {
    pub fn new(number: u32) -> Self {
        Self {
            number,
            lines: Vec::new(),
            scroll_offset: 0,
            following_bottom: true,
        }
    }

    pub fn row_count(&self) -> usize {
        self.lines.len()
    }

    pub fn set_scroll_offset_clamped(&mut self, idx: usize) {
        if self.lines.is_empty() {
            self.scroll_offset = 0;
            return;
        }

        self.scroll_offset = idx.min(self.lines.len().saturating_sub(1));
    }

    pub fn visible_lines(&self, viewport_height: usize) -> Vec<Line<'static>> {
        if self.lines.is_empty() {
            return Vec::new();
        }

        let start = self.scroll_offset.min(self.lines.len());
        let end = (start + viewport_height).min(self.lines.len());
        self.lines[start..end].to_vec()
    }

    /// 替换整段输出内容，并在必要时修正 scroll_offset，避免越界。
    pub fn replace_content(&mut self, new_lines: Vec<Line<'static>>) {
        self.lines = new_lines;

        if self.lines.is_empty() {
            self.scroll_offset = 0;
            return;
        }

        self.scroll_offset = self.scroll_offset.min(self.lines.len().saturating_sub(1));
    }

    /// 替换整段输出内容，并在超过上限时丢弃最旧的行。
    pub fn replace_content_capped(&mut self, mut new_lines: Vec<Line<'static>>, max_lines: usize) {
        if max_lines == 0 {
            self.lines.clear();
            self.scroll_offset = 0;
            return;
        }

        if new_lines.len() > max_lines {
            let overflow = new_lines.len().saturating_sub(max_lines);
            new_lines.drain(0..overflow);
            self.scroll_offset = self.scroll_offset.saturating_sub(overflow);
        }

        self.lines = new_lines;

        if self.lines.is_empty() {
            self.scroll_offset = 0;
            return;
        }

        self.scroll_offset = self.scroll_offset.min(self.lines.len().saturating_sub(1));
    }

    pub fn scroll_up(&mut self) {
        if self.scroll_offset == 0 {
            return;
        }
        self.following_bottom = false;
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    pub fn scroll_down(&mut self, viewport_height: usize) {
        if self.lines.is_empty() {
            return;
        }

        let max_scroll = self.lines.len().saturating_sub(viewport_height.max(1));

        if self.scroll_offset >= max_scroll {
            self.scroll_offset = max_scroll;
            self.following_bottom = true;
            return;
        }

        self.scroll_offset = self.scroll_offset.saturating_add(1).min(max_scroll);
        self.following_bottom = self.scroll_offset >= max_scroll;
    }

    pub fn scroll_top(&mut self) {
        self.scroll_offset = 0;
        self.following_bottom = false;
    }

    pub fn scroll_bottom(&mut self, viewport_height: usize) {
        if self.lines.is_empty() {
            self.scroll_offset = 0;
            self.following_bottom = true;
            return;
        }

        let max_scroll = self.lines.len().saturating_sub(viewport_height.max(1));
        self.scroll_offset = max_scroll;
        self.following_bottom = true;
    }
}
