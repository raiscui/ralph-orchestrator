//! 并行 Supervisor TUI：Output 面板渲染（纯文本）。
//!
//! 说明：
//! - 并行模式需要按 job 分段展示输出，但渲染行为与串行的 ContentPane 尽量对齐：
//!   - 支持软换行（soft-wrap）
//!   - 支持搜索高亮
//!   - 支持框选高亮（由 App 的 copy/selection 逻辑复用该渲染器实现 WYSIWYG）
//! - stderr/stdout 的区分不通过左侧前缀列，而是通过上游渲染器把 stderr 行“弱化成灰色”。

use crate::state::parallel::output::ParallelOutputBuffer;
use crate::widgets::content::SelectionBounds;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub struct ParallelOutputPane<'a> {
    buffer: &'a ParallelOutputBuffer,
    search_query: Option<&'a str>,
    selection: Option<SelectionBounds>,
}

impl<'a> ParallelOutputPane<'a> {
    pub fn new(buffer: &'a ParallelOutputBuffer) -> Self {
        Self {
            buffer,
            search_query: None,
            selection: None,
        }
    }

    pub fn with_search(mut self, query: &'a str) -> Self {
        if !query.is_empty() {
            self.search_query = Some(query);
        }
        self
    }

    pub fn with_selection(mut self, selection: SelectionBounds) -> Self {
        self.selection = Some(selection);
        self
    }
}

impl Widget for ParallelOutputPane<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        // 与 ContentPane 保持一致：visible_lines 的数量上限按 viewport height 粗略裁剪；
        // soft-wrap 仍然在渲染阶段发生，因此最终显示的“逻辑行数”可能少于 height。
        let visible = self.buffer.visible_lines(area.height as usize);

        let selection_bg = Color::Blue;
        let mut y = area.y;

        for line in visible {
            if y >= area.y + area.height {
                break;
            }

            let rendered_line = if let Some(query) = self.search_query {
                highlight_search_matches(&line, query)
            } else {
                line
            };

            render_line_soft_wrapped(
                &rendered_line,
                area,
                area,
                buf,
                &mut y,
                selection_bg,
                self.selection,
            );
        }

        // Clear remaining rows below the content to prevent artifacts.
        while y < area.y + area.height {
            for x in area.x..area.x + area.width {
                let rel_x = x.saturating_sub(area.x);
                let rel_y = y.saturating_sub(area.y);
                let selected = self.selection.is_some_and(|s| s.contains(rel_x, rel_y));
                let style = if selected {
                    Style::default().bg(selection_bg)
                } else {
                    Style::default()
                };
                buf[(x, y)].set_char(' ').set_style(style);
            }
            y += 1;
        }
    }
}

fn render_line_soft_wrapped(
    line: &Line<'static>,
    widget_area: Rect,
    content_area: Rect,
    buf: &mut Buffer,
    y: &mut u16,
    selection_bg: Color,
    selection: Option<SelectionBounds>,
) {
    if *y >= widget_area.y + widget_area.height {
        return;
    }

    let mut x = content_area.x;
    for span in &line.spans {
        let content = span.content.as_ref();

        // Use grapheme clusters to correctly handle emojis and combining characters
        for grapheme in content.graphemes(true) {
            let width = grapheme.width() as u16;

            // Handle line wrapping when reaching end of content area
            if x + width > content_area.x + content_area.width && x > content_area.x {
                // Fill remaining space in current line with selection background if needed
                while x < content_area.x + content_area.width {
                    let rel_x = x.saturating_sub(widget_area.x);
                    let rel_y = (*y).saturating_sub(widget_area.y);
                    let selected = selection.is_some_and(|s| s.contains(rel_x, rel_y));
                    let style = if selected {
                        Style::default().bg(selection_bg)
                    } else {
                        Style::default()
                    };
                    buf[(x, *y)].set_char(' ').set_style(style);
                    x += 1;
                }

                *y += 1;
                if *y >= widget_area.y + widget_area.height {
                    return;
                }
                x = content_area.x;
            }

            // Skip if the grapheme doesn't fit (should only happen at line start with very wide chars)
            if width > content_area.width {
                continue;
            }

            // Render the grapheme
            let rel_x = x.saturating_sub(widget_area.x);
            let rel_y = (*y).saturating_sub(widget_area.y);
            let selected = selection.is_some_and(|s| s.contains(rel_x, rel_y));
            let style = if selected {
                span.style.bg(selection_bg)
            } else {
                span.style
            };

            buf[(x, *y)].set_symbol(grapheme).set_style(style);

            // Advance x position
            x += width;
        }
    }

    // Clear remaining space in the line
    while x < content_area.x + content_area.width {
        let rel_x = x.saturating_sub(widget_area.x);
        let rel_y = (*y).saturating_sub(widget_area.y);
        let selected = selection.is_some_and(|s| s.contains(rel_x, rel_y));
        let style = if selected {
            Style::default().bg(selection_bg)
        } else {
            Style::default()
        };
        buf[(x, *y)].set_char(' ').set_style(style);
        x += 1;
    }

    *y += 1;
}

fn highlight_search_matches(line: &Line<'static>, query: &str) -> Line<'static> {
    if query.is_empty() {
        return line.clone();
    }

    let query_lower = query.to_lowercase();
    let highlight_style = Style::default()
        .fg(Color::Black)
        .bg(Color::Yellow)
        .add_modifier(Modifier::REVERSED);

    let mut new_spans = Vec::new();

    for span in &line.spans {
        let content = span.content.as_ref();
        let content_lower = content.to_lowercase();
        let mut last_end = 0;

        for (match_start, _) in content_lower.match_indices(&query_lower) {
            let match_end = match_start + query.len();

            if match_start > last_end {
                new_spans.push(Span::styled(
                    content[last_end..match_start].to_string(),
                    span.style,
                ));
            }

            new_spans.push(Span::styled(
                content[match_start..match_end].to_string(),
                highlight_style,
            ));

            last_end = match_end;
        }

        if last_end < content.len() {
            new_spans.push(Span::styled(content[last_end..].to_string(), span.style));
        }
    }

    Line::from(new_spans)
}
