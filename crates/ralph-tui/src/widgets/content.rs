//! Content pane widget for rendering iteration output.
//!
//! This widget replaces the VT100 terminal widget with a simpler line-based
//! renderer that displays formatted Lines from an IterationBuffer.

use crate::state::IterationBuffer;
use crate::theme::TuiTheme;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::Widget,
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// Widget that renders the content of an iteration buffer.
///
/// The widget displays the visible lines from the buffer (respecting scroll offset)
/// and optionally highlights search matches.
#[derive(Debug, Clone, Copy)]
pub struct SelectionBounds {
    pub min_x: u16,
    pub max_x: u16,
    pub min_y: u16,
    pub max_y: u16,
}

impl SelectionBounds {
    pub fn from_points(start_x: u16, start_y: u16, end_x: u16, end_y: u16) -> Self {
        let min_x = start_x.min(end_x);
        let max_x = start_x.max(end_x);
        let min_y = start_y.min(end_y);
        let max_y = start_y.max(end_y);
        Self {
            min_x,
            max_x,
            min_y,
            max_y,
        }
    }

    pub fn contains(&self, x: u16, y: u16) -> bool {
        x >= self.min_x && x <= self.max_x && y >= self.min_y && y <= self.max_y
    }
}

pub struct ContentPane<'a> {
    /// Reference to the iteration buffer to render
    buffer: &'a IterationBuffer,
    /// Optional search query for highlighting matches
    search_query: Option<&'a str>,
    /// Optional selection bounds (relative to the widget area)
    selection: Option<SelectionBounds>,
    /// 主题（用于 selection/search 的高亮样式）。
    theme: TuiTheme,
}

impl<'a> ContentPane<'a> {
    /// Creates a new ContentPane for the given iteration buffer.
    pub fn new(buffer: &'a IterationBuffer, theme: TuiTheme) -> Self {
        Self {
            buffer,
            search_query: None,
            selection: None,
            theme,
        }
    }

    /// Sets the search query for highlighting matches.
    pub fn with_search(mut self, query: &'a str) -> Self {
        if !query.is_empty() {
            self.search_query = Some(query);
        }
        self
    }

    /// Sets the selection bounds to highlight.
    pub fn with_selection(mut self, selection: SelectionBounds) -> Self {
        self.selection = Some(selection);
        self
    }
}

impl Widget for ContentPane<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        // =========================================================================
        // 背景/底色策略（非常关键）
        // =========================================================================
        //
        // 现象（用户可见）：
        // - Warp 透明背景（app bg=Reset）下，如果内容区 cell 最终留下了 bg=Reset，
        //   那么动画（tachyonfx sweep）在插值时会把 Reset 当作黑色参与计算，
        //   进而被用户感知为“整屏背景变动/闪烁”。
        // - 同时，pane 内部如果没有稳定底色，会导致文本可读性下降、扫入白条更刺眼。
        //
        // 根因：
        // - ContentPane 需要大量写入/清空 cell。
        // - 如果清空时使用 `Cell::reset()` 或写入 `Style::default()`，很容易把 bg 还原成 Reset，
        //   把外层 panel_block 刷出来的底色抹掉。
        //
        // 解决：
        // - 先从“当前区域左上角 cell”读取一个基准背景色（由外层决定：panel base / app crust / Reset）。
        // - 以此构造 base_style：默认文本色 + 基准背景。
        // - 全区域先铺一层 base_style（清空残影），后续渲染只在此之上叠加 span/search/selection。
        //
        // 这样：
        // - Output/Chat 等 pane 内部能稳定保留底色（base），更易读；
        // - Warp 模式下，外圈仍可保持 Reset（半透明），不会被 content 清空逻辑污染。
        let base_bg = buf[(area.x, area.y)].bg;
        let base_style = self.theme.text().bg(base_bg);

        let x_end = area.x.saturating_add(area.width);
        let y_end = area.y.saturating_add(area.height);

        // 先铺底：防止切换 iteration/滚动时留下上一帧残影。
        for y in area.y..y_end {
            for x in area.x..x_end {
                buf[(x, y)].set_char(' ').set_style(base_style);
            }
        }

        // Get visible lines from the buffer (now returns owned Vec due to interior mutability)
        let visible = self.buffer.visible_lines(area.height as usize);

        let selection_bg = self.theme.selection_bg();
        let mut y = area.y;
        for line in &visible {
            if y >= y_end {
                break;
            }

            // Apply search highlighting if we have a query
            let rendered_line = if let Some(query) = self.search_query {
                highlight_search_matches(line, query, self.theme)
            } else {
                line.clone()
            };

            // Render the line into the buffer with soft wrapping
            let mut x = area.x;
            for span in &rendered_line.spans {
                let content = span.content.as_ref();

                for grapheme in UnicodeSegmentation::graphemes(content, true)
                    .filter(|symbol| !symbol.contains(char::is_control))
                {
                    let width = grapheme.width() as u16;
                    if width == 0 {
                        continue;
                    }

                    // Keep behavior consistent with ratatui's `Buffer::set_stringn`: if a grapheme
                    // is wider than the viewport, it can't be drawn; skip it.
                    if width > area.width {
                        continue;
                    }

                    // Soft wrap: if the grapheme doesn't fit on this row, move to next row.
                    if x.saturating_add(width) > x_end {
                        y = y.saturating_add(1);
                        x = area.x;
                        if y >= y_end {
                            return;
                        }
                    }

                    // Key: write by grapheme cluster and advance by display width, so we don't
                    // write ASCII into a CJK/emoji continuation cell and "swallow" the next char.
                    //
                    // 注意：span.style 可能不包含 bg/fg。
                    // - bg：必须保留 base_bg（否则 pane 内部会被写回 Reset，导致透明与动画副作用）
                    // - fg：默认用主题 text 色，保证 UI 一致性
                    let style = base_style.patch(span.style);
                    buf[(x, y)].set_symbol(grapheme).set_style(style);

                    let next_symbol = x.saturating_add(width);
                    x = x.saturating_add(1);
                    while x < next_symbol {
                        // 宽字符的 continuation cell 必须写空 symbol（""），否则会破坏终端的宽度对齐。
                        buf[(x, y)].set_symbol("").set_style(base_style);
                        x = x.saturating_add(1);
                    }
                }
            }

            // Move to the next row for the next logical line
            y = y.saturating_add(1);
        }

        // Selection overlay：
        // - 选择区域需要覆盖“空白处”，否则用户拖拽选择时会感觉断层。
        // - 放到最后统一叠加，能保证 selection 优先级高于 search highlight。
        if let Some(selection) = self.selection {
            let max_x = area.width.saturating_sub(1);
            let max_y = area.height.saturating_sub(1);

            let sel_min_x = selection.min_x.min(max_x);
            let sel_max_x = selection.max_x.min(max_x);
            let sel_min_y = selection.min_y.min(max_y);
            let sel_max_y = selection.max_y.min(max_y);

            for rel_y in sel_min_y..=sel_max_y {
                let y = area.y.saturating_add(rel_y);
                for rel_x in sel_min_x..=sel_max_x {
                    let x = area.x.saturating_add(rel_x);
                    if x >= x_end || y >= y_end {
                        continue;
                    }
                    let cell = &mut buf[(x, y)];
                    let style = cell.style().bg(selection_bg);
                    cell.set_style(style);
                }
            }
        }
    }
}

/// Highlights search matches in a line with a distinct style.
fn highlight_search_matches(line: &Line<'static>, query: &str, theme: TuiTheme) -> Line<'static> {
    if query.is_empty() {
        return line.clone();
    }

    let query_lower = query.to_lowercase();
    let highlight_style = theme.search_hit();

    let mut new_spans = Vec::new();

    for span in &line.spans {
        let content = span.content.as_ref();
        let content_lower = content.to_lowercase();
        let mut last_end = 0;

        // Find all matches in this span's content
        for (match_start, _) in content_lower.match_indices(&query_lower) {
            let match_end = match_start + query.len();

            // Add the part before the match with original style
            if match_start > last_end {
                new_spans.push(Span::styled(
                    content[last_end..match_start].to_string(),
                    span.style,
                ));
            }

            // Add the matched part with highlight style
            new_spans.push(Span::styled(
                content[match_start..match_end].to_string(),
                highlight_style,
            ));

            last_end = match_end;
        }

        // Add any remaining content after the last match
        if last_end < content.len() {
            new_spans.push(Span::styled(content[last_end..].to_string(), span.style));
        } else if last_end == 0 {
            // No matches found, keep original span
            new_spans.push(span.clone());
        }
    }

    Line::from(new_spans)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::TuiTheme;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::style::{Color, Modifier, Style};

    /// Helper to render ContentPane and return buffer content as strings
    fn render_content_pane(
        buffer: &IterationBuffer,
        search: Option<&str>,
        width: u16,
        height: u16,
    ) -> Vec<String> {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let mut widget = ContentPane::new(buffer, TuiTheme::default());
                if let Some(q) = search {
                    widget = widget.with_search(q);
                }
                f.render_widget(widget, f.area());
            })
            .unwrap();

        let buf = terminal.backend().buffer();
        // Extract lines from the buffer
        (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| buf[(x, y)].symbol().to_string())
                    .collect::<String>()
            })
            .collect()
    }

    /// Helper to check if a cell has the highlight style
    fn has_highlight_style(
        buffer: &IterationBuffer,
        search: &str,
        width: u16,
        height: u16,
        x: u16,
        y: u16,
    ) -> bool {
        let theme = TuiTheme::default();
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let widget = ContentPane::new(buffer, theme).with_search(search);
                f.render_widget(widget, f.area());
            })
            .unwrap();

        let buf = terminal.backend().buffer();
        let cell = &buf[(x, y)];
        // Check for highlight: typically reverse or yellow background
        cell.modifier.contains(Modifier::REVERSED)
            || cell.bg == theme.colors().yellow
            || cell.fg == theme.colors().crust
    }

    fn has_selection_bg(
        buffer: &IterationBuffer,
        selection: SelectionBounds,
        width: u16,
        height: u16,
        x: u16,
        y: u16,
    ) -> bool {
        let theme = TuiTheme::default();
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let widget = ContentPane::new(buffer, theme).with_selection(selection);
                f.render_widget(widget, f.area());
            })
            .unwrap();

        let buf = terminal.backend().buffer();
        let cell = &buf[(x, y)];
        cell.bg == theme.selection_bg()
    }

    // =========================================================================
    // Acceptance Criteria 1: Renders Lines
    // =========================================================================

    #[test]
    fn renders_lines_when_viewport_fits_all() {
        // Given a buffer with 3 lines
        let mut buffer = IterationBuffer::new(1);
        buffer.append_line(Line::from("first line"));
        buffer.append_line(Line::from("second line"));
        buffer.append_line(Line::from("third line"));

        // When ContentPane renders with viewport height >= 3
        let lines = render_content_pane(&buffer, None, 40, 5);

        // Then all 3 lines are visible in the output
        assert!(
            lines[0].contains("first line"),
            "first line should be visible, got: {:?}",
            lines
        );
        assert!(
            lines[1].contains("second line"),
            "second line should be visible, got: {:?}",
            lines
        );
        assert!(
            lines[2].contains("third line"),
            "third line should be visible, got: {:?}",
            lines
        );
    }

    // =========================================================================
    // Selection: Highlights Selected Cells
    // =========================================================================

    #[test]
    fn selection_highlights_cells_with_blue_background() {
        let mut buffer = IterationBuffer::new(1);
        buffer.append_line(Line::from("hello world"));

        let selection = SelectionBounds::from_points(0, 0, 4, 0);
        assert!(
            has_selection_bg(&buffer, selection, 20, 2, 0, 0),
            "selected cell should have selection background"
        );
        assert!(
            has_selection_bg(&buffer, selection, 20, 2, 4, 0),
            "end of selection should have selection background"
        );
        assert!(
            !has_selection_bg(&buffer, selection, 20, 2, 6, 0),
            "outside selection should not have selection background"
        );
    }

    #[test]
    fn renders_lines_preserves_styling() {
        // Given a buffer with styled lines
        let mut buffer = IterationBuffer::new(1);
        buffer.append_line(Line::from(vec![
            Span::styled("error: ", Style::default().fg(Color::Red)),
            Span::raw("something went wrong"),
        ]));

        // When ContentPane renders
        let backend = TestBackend::new(40, 3);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let widget = ContentPane::new(&buffer, TuiTheme::default());
                f.render_widget(widget, f.area());
            })
            .unwrap();

        // Then the styled spans are rendered (check color of first cell)
        let buf = terminal.backend().buffer();
        // The 'e' in 'error' should be red
        assert_eq!(
            buf[(0, 0)].fg,
            Color::Red,
            "styled span should preserve color"
        );
    }

    // =========================================================================
    // Acceptance Criteria 2: Respects Scroll Offset
    // =========================================================================

    #[test]
    fn respects_scroll_offset() {
        // Given a buffer with 10 lines and scroll_offset 5
        let mut buffer = IterationBuffer::new(1);
        for i in 0..10 {
            buffer.append_line(Line::from(format!("line {}", i)));
        }
        buffer.scroll_offset = 5;

        // When ContentPane renders with viewport height 5
        let lines = render_content_pane(&buffer, None, 40, 5);

        // Then lines 5-9 are shown (not 0-4)
        assert!(
            lines[0].contains("line 5"),
            "should show line 5 first, got: {:?}",
            lines
        );
        assert!(
            lines[4].contains("line 9"),
            "should show line 9 last, got: {:?}",
            lines
        );
        assert!(
            !lines.iter().any(|l| l.contains("line 0")),
            "line 0 should not be visible"
        );
    }

    #[test]
    fn scroll_offset_at_end_shows_last_lines() {
        let mut buffer = IterationBuffer::new(1);
        for i in 0..10 {
            buffer.append_line(Line::from(format!("line {}", i)));
        }
        buffer.scroll_bottom(3); // viewport 3, should show lines 7-9

        let lines = render_content_pane(&buffer, None, 40, 3);

        assert!(
            lines[0].contains("line 7"),
            "first visible should be line 7, got: {:?}",
            lines
        );
        assert!(
            lines[2].contains("line 9"),
            "last visible should be line 9, got: {:?}",
            lines
        );
    }

    // =========================================================================
    // Acceptance Criteria 3: Search Highlight
    // =========================================================================

    #[test]
    fn search_highlights_matches() {
        // Given a buffer with lines containing "foo"
        let mut buffer = IterationBuffer::new(1);
        buffer.append_line(Line::from("this contains foo in the middle"));
        buffer.append_line(Line::from("no match here"));
        buffer.append_line(Line::from("foo at start"));

        // When ContentPane renders with search query "foo"
        // Then "foo" spans are highlighted (different style)
        // Check that the 'f' in 'foo' at position 14 (line 0) has highlight style
        assert!(
            has_highlight_style(&buffer, "foo", 40, 3, 14, 0),
            "search match 'foo' should be highlighted"
        );
    }

    #[test]
    fn search_highlights_multiple_matches_per_line() {
        let mut buffer = IterationBuffer::new(1);
        buffer.append_line(Line::from("foo and another foo here"));

        // Both occurrences should be highlighted
        assert!(
            has_highlight_style(&buffer, "foo", 40, 1, 0, 0),
            "first 'foo' should be highlighted"
        );
        assert!(
            has_highlight_style(&buffer, "foo", 40, 1, 16, 0),
            "second 'foo' should be highlighted"
        );
    }

    #[test]
    fn search_case_insensitive() {
        let mut buffer = IterationBuffer::new(1);
        buffer.append_line(Line::from("Contains FOO uppercase"));

        // Search for lowercase should match uppercase
        assert!(
            has_highlight_style(&buffer, "foo", 40, 1, 9, 0),
            "case-insensitive search should highlight FOO"
        );
    }

    #[test]
    fn empty_search_query_no_highlight() {
        let mut buffer = IterationBuffer::new(1);
        buffer.append_line(Line::from("some text"));

        // Empty search shouldn't highlight anything
        let backend = TestBackend::new(40, 1);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let widget = ContentPane::new(&buffer, TuiTheme::default()).with_search("");
                f.render_widget(widget, f.area());
            })
            .unwrap();

        let buf = terminal.backend().buffer();
        // No cells should have highlight modifier
        for x in 0..40 {
            assert!(
                !buf[(x, 0)].modifier.contains(Modifier::REVERSED),
                "empty search should not highlight"
            );
        }
    }

    // =========================================================================
    // Acceptance Criteria 4: Empty Buffer Handling
    // =========================================================================

    #[test]
    fn empty_buffer_renders_without_panic() {
        // Given an empty IterationBuffer
        let buffer = IterationBuffer::new(1);

        // When ContentPane renders
        // Then no panic occurs and empty area is shown
        let lines = render_content_pane(&buffer, None, 40, 5);

        // All lines should be empty (spaces)
        for line in &lines {
            assert!(
                line.trim().is_empty(),
                "empty buffer should render blank lines, got: {:?}",
                line
            );
        }
    }

    #[test]
    fn empty_buffer_with_search_renders_without_panic() {
        let buffer = IterationBuffer::new(1);

        // Should not panic even with search query on empty buffer
        let lines = render_content_pane(&buffer, Some("search"), 40, 5);

        for line in &lines {
            assert!(line.trim().is_empty());
        }
    }

    // =========================================================================
    // Acceptance Criteria 5: Widget Integration
    // =========================================================================

    #[test]
    fn widget_fills_provided_rect() {
        let mut buffer = IterationBuffer::new(1);
        buffer.append_line(Line::from("test"));

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        // Render into a specific sub-area
        let area = Rect::new(5, 5, 30, 10);
        terminal
            .draw(|f| {
                let widget = ContentPane::new(&buffer, TuiTheme::default());
                f.render_widget(widget, area);
            })
            .unwrap();

        // Content should be at position (5, 5), not (0, 0)
        let buf = terminal.backend().buffer();
        assert_eq!(buf[(5, 5)].symbol(), "t", "content should start at area.x");
    }

    #[test]
    fn widget_wraps_lines_at_area_width() {
        let mut buffer = IterationBuffer::new(1);
        buffer.append_line(Line::from(
            "this is a very long line that exceeds the width",
        ));

        // Render with narrow width and enough height for wrapping
        let lines = render_content_pane(&buffer, None, 20, 3);

        // First row should have first 20 chars
        assert!(
            lines[0].starts_with("this is a very long "),
            "first row should have first 20 chars, got: {:?}",
            lines[0]
        );
        // Second row should have continuation
        assert!(
            lines[1].starts_with("line that exceeds th"),
            "second row should have continuation, got: {:?}",
            lines[1]
        );
        // Third row should have the rest
        assert!(
            lines[2].starts_with("e width"),
            "third row should have remaining text, got: {:?}",
            lines[2]
        );
    }

    #[test]
    fn cjk_double_width_does_not_swallow_next_ascii_char() {
        // This test reproduces a subtle but common TUI issue:
        // when a double-width CJK character is immediately followed by ASCII, if the renderer
        // iterates with `chars()` and advances the cursor with `x += 1`,
        // the next column is actually a continuation cell that terminals skip, causing the ASCII
        // first letter to disappear.
        let mut buffer = IterationBuffer::new(1);
        buffer.append_line(Line::from("将search/notes"));

        let area = Rect::new(0, 0, 20, 1);
        let mut buf = Buffer::empty(area);

        let widget = ContentPane::new(&buffer, TuiTheme::default());
        widget.render(area, &mut buf);

        // Expected behavior: the CJK character (U+5C06) occupies two columns; x=1 should be a
        // continuation cell (empty symbol), and the ASCII 's' should start at x=2.
        assert_eq!(buf[(0, 0)].symbol(), "将");
        assert_eq!(buf[(1, 0)].symbol(), "");
        assert_eq!(buf[(2, 0)].symbol(), "s");
    }

    // =========================================================================
    // Acceptance Criteria 6: Buffer Clearing (Artifact Prevention)
    // =========================================================================

    #[test]
    fn clears_remaining_rows_when_content_shorter_than_viewport() {
        // Given a pre-filled ratatui buffer (simulating previous frame's content)
        let area = Rect::new(0, 0, 40, 10);
        let mut buf = Buffer::empty(area);

        // Pre-fill the buffer with "X" characters to simulate previous iteration content
        for y in 0..10 {
            for x in 0..40 {
                buf[(x, y)].set_char('X');
            }
        }

        // And an IterationBuffer with only 3 lines
        let mut iter_buffer = IterationBuffer::new(1);
        iter_buffer.append_line(Line::from("line one"));
        iter_buffer.append_line(Line::from("line two"));
        iter_buffer.append_line(Line::from("line three"));

        // When ContentPane renders (only 3 lines of content)
        let widget = ContentPane::new(&iter_buffer, TuiTheme::default());
        widget.render(area, &mut buf);

        // Then rows 0-2 should have the content
        assert!(
            buf[(0, 0)].symbol() == "l",
            "row 0 should have content, got: {}",
            buf[(0, 0)].symbol()
        );

        // And rows 3-9 should be cleared (no 'X' artifacts)
        for y in 3..10 {
            for x in 0..40 {
                let symbol = buf[(x, y)].symbol();
                assert!(
                    symbol != "X",
                    "row {} col {} should be cleared, but found artifact 'X'",
                    y,
                    x
                );
            }
        }
    }
}
