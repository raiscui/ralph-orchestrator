//! 并行 Supervisor TUI：输出缓冲（按 job 分段）。
//!
//! 设计目标：
//! - 并行模式下，每个 job 都有独立的输出 buffer，便于回看与搜索；
//! - 只存储“可直接显示的行”（`ratatui::text::Line`），不引入 Big Headers / 图片块等额外渲染结构；
//! - stderr 与 stdout 的区分交给上游渲染器（例如把 stderr 行统一弱化成灰色）。
//!
//! 注意：
//! - 这里的 `lines` 在进入 buffer 前已经按 Output 宽度做过一轮 grapheme 级预换行；
//!   这样 `scroll_offset` / `row_count` 就能和屏幕上的“可见行数”保持一致。
//! - widget 仍可做兜底 soft-wrap,但正常情况下不应再依赖它决定“底部是否可见”。

use ratatui::text::Line;
use ratatui::text::Span;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// 并行模式下的输出 buffer（纯文本行 + 滚动状态）。
#[derive(Debug)]
pub struct ParallelOutputBuffer {
    /// Job 序号（1-based，仅用于展示）。
    pub number: u32,
    /// 已按当前 Output 宽度预换行后的显示行列表（滚动单位）。
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

/// 把一组 `Line` 按给定宽度预换行成“显示行”。
///
/// 为什么要在进入 buffer 前做这一步:
/// - 并行 Output 的自动滚动/到底判断依赖 `row_count()` 与 `scroll_offset`。
/// - 如果只存“逻辑行”,但真正渲染时再 soft-wrap,就会出现:
///   - 状态以为已经到底了
///   - 屏幕上仍停在上面的包裹行里
///   - 最后的 reply 被顶到视口外,用户体感就是"没有回复"
///
/// 这里按 grapheme + display width 预换行,让 buffer 的“行数”尽量贴近最终可见行数。
pub fn wrap_lines_to_width(lines: Vec<Line<'static>>, width: u16) -> Vec<Line<'static>> {
    let width = width.max(1);
    lines
        .into_iter()
        .flat_map(|line| wrap_line_to_width(line, width))
        .collect()
}

fn wrap_line_to_width(line: Line<'static>, width: u16) -> Vec<Line<'static>> {
    if width == 0 {
        return Vec::new();
    }

    let mut wrapped: Vec<Line<'static>> = Vec::new();
    let mut row_spans: Vec<Span<'static>> = Vec::new();
    let mut span_text = String::new();
    let mut span_style = None;
    let mut row_width: u16 = 0;
    let mut saw_visible_grapheme = false;

    let flush_span = |row_spans: &mut Vec<Span<'static>>,
                      span_text: &mut String,
                      span_style: &mut Option<ratatui::style::Style>| {
        if span_text.is_empty() {
            return;
        }
        let style = span_style.take().unwrap_or_default();
        row_spans.push(Span::styled(std::mem::take(span_text), style));
    };

    let flush_row = |wrapped: &mut Vec<Line<'static>>, row_spans: &mut Vec<Span<'static>>| {
        wrapped.push(Line::from(std::mem::take(row_spans)));
    };

    for span in line.spans {
        let style = span.style;
        for grapheme in span
            .content
            .as_ref()
            .graphemes(true)
            .filter(|symbol| !symbol.contains(char::is_control))
        {
            let grapheme_width = grapheme.width() as u16;
            if grapheme_width == 0 {
                continue;
            }
            if grapheme_width > width {
                continue;
            }

            if row_width > 0 && row_width.saturating_add(grapheme_width) > width {
                flush_span(&mut row_spans, &mut span_text, &mut span_style);
                flush_row(&mut wrapped, &mut row_spans);
                row_width = 0;
            }

            if span_style != Some(style) {
                flush_span(&mut row_spans, &mut span_text, &mut span_style);
                span_style = Some(style);
            }

            span_text.push_str(grapheme);
            row_width = row_width.saturating_add(grapheme_width);
            saw_visible_grapheme = true;
        }
    }

    flush_span(&mut row_spans, &mut span_text, &mut span_style);
    if !row_spans.is_empty() || !saw_visible_grapheme {
        flush_row(&mut wrapped, &mut row_spans);
    }

    if wrapped.is_empty() {
        vec![Line::from(String::new())]
    } else {
        wrapped
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::{Color, Style};
    use ratatui::text::{Line, Span};

    #[test]
    fn wrap_lines_to_width_expands_long_line_into_multiple_display_rows() {
        let lines = vec![Line::from("ABCDEFGHIJKLMNO")];
        let wrapped = wrap_lines_to_width(lines, 10);
        let got = wrapped
            .iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>();

        assert_eq!(got, vec!["ABCDEFGHIJ".to_string(), "KLMNO".to_string()]);
    }

    #[test]
    fn wrap_lines_to_width_preserves_span_styles_across_wrapped_rows() {
        let line = Line::from(vec![
            Span::styled("ABCDE", Style::default().fg(Color::Red)),
            Span::styled("FGHIJ", Style::default().fg(Color::Blue)),
        ]);
        let wrapped = wrap_lines_to_width(vec![line], 6);

        assert_eq!(wrapped.len(), 2);
        assert_eq!(wrapped[0].to_string(), "ABCDEF");
        assert_eq!(wrapped[1].to_string(), "GHIJ");

        assert_eq!(wrapped[0].spans[0].style.fg, Some(Color::Red));
        assert_eq!(wrapped[0].spans[1].style.fg, Some(Color::Blue));
        assert_eq!(wrapped[1].spans[0].style.fg, Some(Color::Blue));
    }
}
