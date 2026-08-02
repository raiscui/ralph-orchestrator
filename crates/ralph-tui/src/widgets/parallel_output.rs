//! 并行 Supervisor TUI：Output 面板渲染（纯文本）。
//!
//! 说明：
//! - 并行模式需要按 job 分段展示输出，但渲染行为与串行的 ContentPane 尽量对齐：
//!   - 支持软换行（soft-wrap）
//!   - 支持搜索高亮
//!   - 支持框选高亮（由 App 的 copy/selection 逻辑复用该渲染器实现 WYSIWYG）
//! - stderr/stdout 的区分不通过左侧前缀列，而是通过上游渲染器把 stderr 行“弱化成灰色”。

use crate::state::parallel::output::ParallelOutputBuffer;
use crate::state::parallel::{InstanceViewState, ParallelTuiState};
use crate::theme::TuiTheme;
use crate::widgets::content::SelectionBounds;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub struct ParallelOutputPane<'a> {
    buffer: &'a ParallelOutputBuffer,
    search_query: Option<&'a str>,
    selection: Option<SelectionBounds>,
}

/// Output pane 内部的正文区与底部状态区。
///
/// 说明:
/// - `content_area` 才是 stdout/stderr 正文、选择、复制、滚动的可视区域。
/// - `status_area` 专门放 evidence / act 状态,不得参与正文 viewport 计算。
/// - 把这个 split 放在 widget 模块里,是为了让 App 渲染、autoscroll 预计算和测试 harness
///   复用同一套几何规则,避免状态条再次“吃掉”正文最后几行。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParallelOutputAreas {
    pub content_area: Rect,
    pub status_area: Rect,
}

pub fn split_parallel_output_areas(inner: Rect) -> ParallelOutputAreas {
    let status_height = parallel_output_status_height(inner.height);
    if status_height == 0 {
        return ParallelOutputAreas {
            content_area: inner,
            status_area: Rect::default(),
        };
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(status_height)])
        .split(inner);

    ParallelOutputAreas {
        content_area: chunks[0],
        status_area: chunks[1],
    }
}

fn parallel_output_status_height(inner_height: u16) -> u16 {
    if inner_height >= 4 {
        2
    } else {
        inner_height.min(1)
    }
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

/// Output 面板底部的状态/证据条。
///
/// 说明:
/// - 第一优先级是把当前 activity 放到 Output 窗口最下方,让用户不需要回头找 footer。
/// - 第二优先级是把当前证据路径露出来,便于核对 runtime 的实际落盘位置。
pub struct ParallelOutputStatusPane<'a> {
    parallel: &'a ParallelTuiState,
    instance: Option<&'a InstanceViewState>,
    theme: TuiTheme,
}

impl<'a> ParallelOutputStatusPane<'a> {
    pub fn new(
        parallel: &'a ParallelTuiState,
        instance: Option<&'a InstanceViewState>,
        theme: TuiTheme,
    ) -> Self {
        Self {
            parallel,
            instance,
            theme,
        }
    }
}

impl Widget for ParallelOutputStatusPane<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let now = std::time::Instant::now();
        let evidence_text = match self.parallel.latest_child_run_evidence_text() {
            Some(child_run) => {
                format!(
                    "{} | child {child_run}",
                    self.parallel.evidence_paths.summary_text()
                )
            }
            None => self.parallel.evidence_paths.summary_text(),
        };
        let evidence_line = build_fitted_status_line(
            area.width,
            "evidence:",
            &evidence_text,
            self.theme.colors().surface2,
        );

        let activity_text = self
            .instance
            .and_then(|instance| instance.current_activity_summary(now))
            .or_else(|| {
                self.instance.map(|instance| {
                    format!(
                        "{} ({})",
                        instance.state,
                        instance
                            .current_job_short_summary()
                            .unwrap_or_else(|| "j-".to_string())
                    )
                })
            })
            .unwrap_or_else(|| "idle".to_string());

        let activity_line = build_fitted_status_line(
            area.width,
            "act:",
            &activity_text,
            self.theme.colors().green,
        );

        let lines = if area.height >= 2 {
            vec![evidence_line, activity_line]
        } else {
            vec![build_combined_status_line(
                area.width,
                &activity_text,
                &evidence_text,
                self.theme,
            )]
        };

        Paragraph::new(lines).render(area, buf);
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

fn build_fitted_status_line(
    width: u16,
    label: &str,
    value: &str,
    color: ratatui::style::Color,
) -> Line<'static> {
    let prefix = format!("{label} ");
    let text = fit_text_to_width(&format!("{prefix}{value}"), width);
    if text.is_empty() {
        return Line::from("");
    }

    let spans = if let Some(rest) = text.strip_prefix(&prefix) {
        vec![
            Span::styled(prefix, Style::default().fg(color)),
            Span::raw(rest.to_string()),
        ]
    } else {
        vec![Span::styled(text, Style::default().fg(color))]
    };

    Line::from(spans)
}

fn build_combined_status_line(
    width: u16,
    activity: &str,
    evidence: &str,
    theme: TuiTheme,
) -> Line<'static> {
    let line = format!("act: {activity} | evidence: {evidence}");
    let fitted = fit_text_to_width(&line, width);
    Line::from(vec![
        Span::styled("act:", Style::default().fg(theme.colors().green)),
        Span::raw(" "),
        Span::raw(fitted.trim_start_matches("act: ").to_string()),
    ])
}

fn fit_text_to_width(text: &str, width: u16) -> String {
    if width == 0 {
        return String::new();
    }

    let max_width = width as usize;
    if UnicodeWidthStr::width(text) <= max_width {
        return text.to_string();
    }

    if max_width <= 1 {
        return "…".to_string();
    }

    let mut out = String::new();
    let mut used = 0usize;
    for ch in text.chars() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + ch_width >= max_width {
            break;
        }
        out.push(ch);
        used += ch_width;
    }
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::parallel::ParallelEvidencePaths;
    use ralph_core::{HatJobOutputChunk, OutputStream};
    use ralph_proto::{HatInstanceId, HatInstanceState};
    use ratatui::{Terminal, backend::TestBackend};

    fn render_status_to_lines(parallel: &ParallelTuiState, width: u16, height: u16) -> Vec<String> {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let widget = ParallelOutputStatusPane::new(
                    parallel,
                    parallel.selected_instance(),
                    TuiTheme::default(),
                );
                f.render_widget(widget, f.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let width = buffer.area().width as usize;
        buffer
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>()
            .chars()
            .collect::<Vec<_>>()
            .chunks(width)
            .map(|chunk| chunk.iter().collect::<String>().trim_end().to_string())
            .collect()
    }

    #[test]
    fn output_status_pane_shows_evidence_paths() {
        let mut parallel = ParallelTuiState {
            evidence_paths: ParallelEvidencePaths {
                events_path: Some(".ralph/events-20260517-195000.jsonl".to_string()),
                evidence_index_path: Some(".ralph/evidence-index.jsonl".to_string()),
                agents_snapshot_path: Some(".ralph/agents.json".to_string()),
                record_session_path: Some("/tmp/record-session.jsonl".to_string()),
            },
            ..ParallelTuiState::default()
        };

        let instance_id = HatInstanceId::from("writer#1");
        parallel.register_instance(instance_id, HatInstanceState::Running);

        let text = render_status_to_lines(&parallel, 200, 2).join("\n");

        assert!(
            text.contains("evidence:"),
            "should show evidence label: {text}"
        );
        assert!(
            text.contains(".ralph/events-20260517-195000.jsonl"),
            "should show current events path: {text}"
        );
        assert!(
            text.contains(".ralph/evidence-index.jsonl"),
            "should show evidence index path: {text}"
        );
        assert!(
            text.contains(".ralph/agents.json"),
            "should show agents snapshot path: {text}"
        );
        assert!(
            text.contains("/tmp/record-session.jsonl"),
            "should show record-session path: {text}"
        );
    }

    #[test]
    fn output_status_pane_places_activity_on_bottom_line() {
        let mut parallel = ParallelTuiState::default();
        let instance_id = HatInstanceId::from("coder#1");

        parallel.register_instance(instance_id.clone(), HatInstanceState::Running);
        parallel.append_output(&HatJobOutputChunk {
            job_id: 9,
            instance_id,
            stream: OutputStream::Activity,
            line: "Inspecting current code behavior".to_string(),
        });

        let lines = render_status_to_lines(&parallel, 140, 2);
        let bottom = lines.last().cloned().unwrap_or_default();

        assert!(
            bottom.contains("act:"),
            "activity should live on the bottom status line: {bottom}"
        );
        assert!(
            bottom.contains("Inspecting current code behavior"),
            "should show Codex-style activity label: {bottom}"
        );
        assert!(
            bottom.contains("Ctrl+C to interrupt"),
            "should show interrupt hint next to activity: {bottom}"
        );
    }

    #[test]
    fn output_status_pane_shows_latest_child_run_artifact() {
        use crate::state::parallel::{ChildRunStatus, ChildRunViewState};

        let mut parallel = ParallelTuiState::default();
        parallel.child_run_order.push("cap-req-1".to_string());
        parallel.child_runs.insert(
            "cap-req-1".to_string(),
            ChildRunViewState {
                key: "cap-req-1".to_string(),
                request_id: Some("cap-req-1".to_string()),
                invocation_id: Some("cap-inv-1".to_string()),
                capability_id: "workflow:default-parallel".to_string(),
                status: ChildRunStatus::Done,
                summary: Some("done".to_string()),
                artifact: Some(".ralph/capability-invocations/cap-inv-1/result.json".to_string()),
                updated_at: std::time::Instant::now(),
            },
        );

        let text = render_status_to_lines(&parallel, 240, 2).join("\n");

        assert!(
            text.contains("child done:workflow:default-parallel:cap-inv-1"),
            "status pane should expose latest child-run identity, got: {text}"
        );
        assert!(
            text.contains(".ralph/capability-invocations/cap-inv-1/result.json"),
            "status pane should expose child-run artifact path, got: {text}"
        );
    }

    #[test]
    fn split_parallel_output_areas_reserves_status_rows_outside_content() {
        let inner = Rect::new(2, 3, 80, 10);

        let areas = split_parallel_output_areas(inner);

        assert_eq!(
            areas.content_area,
            Rect::new(2, 3, 80, 8),
            "正文 viewport 必须扣除底部 evidence/act 状态条"
        );
        assert_eq!(
            areas.status_area,
            Rect::new(2, 11, 80, 2),
            "状态条应稳定占据 Output inner 最下面两行"
        );
        assert_eq!(
            areas.content_area.height + areas.status_area.height,
            inner.height,
            "content/status split 不能凭空丢行或重叠"
        );
    }

    #[test]
    fn fit_text_to_width_never_exceeds_target_width() {
        let text = fit_text_to_width("act: Inspecting current code behavior", 12);

        assert!(
            UnicodeWidthStr::width(text.as_str()) <= 12,
            "fitted text should not wrap: {text}"
        );
        assert!(
            text.ends_with('…'),
            "long status should be truncated: {text}"
        );
    }
}
