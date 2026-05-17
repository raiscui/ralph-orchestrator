//! 并行模式：实例列表面板（HatInstance 列表）。

use crate::state::{ParallelFocus, ParallelTuiState};
use crate::theme::{TuiTheme, panel_block, patch_exabind_panel_border_bg};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Widget},
};

/// 左侧实例列表面板。
pub struct InstancesPane<'a> {
    parallel: &'a ParallelTuiState,
    focused: bool,
    theme: TuiTheme,
}

impl<'a> InstancesPane<'a> {
    pub fn new(parallel: &'a ParallelTuiState, theme: TuiTheme) -> Self {
        Self {
            parallel,
            focused: parallel.focus == ParallelFocus::Instances,
            theme,
        }
    }
}

impl Widget for InstancesPane<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = panel_block("Instances", self.focused, &self.theme);
        let inner = block.inner(area);
        block.render(area, buf);
        // exabind 风格边框：需要把“外侧背景”刷回 crust，才能让左上斜切角与底边贴边。
        patch_exabind_panel_border_bg(buf, area, &self.theme);

        if inner.width == 0 || inner.height == 0 {
            return;
        }

        let now = std::time::Instant::now();
        let items: Vec<ListItem> = self
            .parallel
            .instance_order
            .iter()
            .map(|id| {
                let state = self
                    .parallel
                    .instances
                    .get(id)
                    .map(|s| s.state.to_string())
                    .unwrap_or_else(|| "-".to_string());
                let job_summary = self
                    .parallel
                    .instances
                    .get(id)
                    .and_then(|s| s.current_job_summary());
                let activity_summary = self
                    .parallel
                    .instances
                    .get(id)
                    .and_then(|s| s.current_activity_short_summary(now));
                let age = self
                    .parallel
                    .instances
                    .get(id)
                    .and_then(|s| s.last_output_at)
                    .map(|t| format!("{}s", t.elapsed().as_secs()))
                    .unwrap_or_else(|| "-".to_string());

                // 说明：
                // - 这里用简单的三列展示，宽度不足时会被 ratatui 截断
                // - 后续可再做更智能的宽度自适应
                let mut spans = vec![
                    Span::styled(
                        id.as_str().to_string(),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" "),
                    Span::raw(state),
                ];

                // job 摘要直接回答“当前在跑第几个 job”。
                // 这里复用 state 中已有的 job 分段,避免新增第二套状态源。
                if let Some(job_summary) = job_summary {
                    spans.push(Span::raw(" "));
                    spans.push(Span::styled(job_summary, self.theme.muted()));
                }

                if let Some(activity_summary) = activity_summary {
                    spans.push(Span::raw(" "));
                    spans.push(Span::styled(
                        format!("a:{activity_summary}"),
                        self.theme.muted(),
                    ));
                } else {
                    spans.extend([Span::raw(" "), Span::styled(age, self.theme.muted())]);
                }

                let line = Line::from(spans);
                ListItem::new(line)
            })
            .collect();

        let mut state = ListState::default();
        if !items.is_empty() {
            state.select(Some(self.parallel.selected_instance));
        }

        let list = List::new(items).highlight_style(
            Style::default()
                .bg(self.theme.selection_bg())
                .fg(self.theme.colors().crust)
                .add_modifier(Modifier::BOLD),
        );
        ratatui::widgets::StatefulWidget::render(list, inner, buf, &mut state);
    }
}

/// Convenience helper.
pub fn render(parallel: &ParallelTuiState, theme: TuiTheme) -> InstancesPane<'_> {
    InstancesPane::new(parallel, theme)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ralph_core::{HatJobOutputChunk, OutputStream};
    use ralph_proto::{HatInstanceId, HatInstanceState};
    use ratatui::{Terminal, backend::TestBackend};

    fn render_to_string(parallel: &ParallelTuiState) -> String {
        let backend = TestBackend::new(80, 4);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let widget = render(parallel, TuiTheme::default());
                f.render_widget(widget, f.area());
            })
            .unwrap();

        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>()
    }

    #[test]
    fn instances_pane_shows_current_job_summary() {
        let mut parallel = ParallelTuiState::default();
        let instance_id = HatInstanceId::from("writer#1");

        parallel.register_instance(instance_id.clone(), HatInstanceState::Running);
        parallel.append_output(&HatJobOutputChunk {
            job_id: 7,
            instance_id,
            stream: OutputStream::Stdout,
            line: "hello".to_string(),
        });

        let text = render_to_string(&parallel);

        assert!(text.contains("writer#1"), "should show instance id: {text}");
        assert!(text.contains("running"), "should show state: {text}");
        assert!(text.contains("job 1/1"), "should show job summary: {text}");
    }

    #[test]
    fn instances_pane_shows_current_activity_summary() {
        let mut parallel = ParallelTuiState::default();
        let instance_id = HatInstanceId::from("writer#1");

        parallel.register_instance(instance_id.clone(), HatInstanceState::Running);
        parallel.append_output(&HatJobOutputChunk {
            job_id: 7,
            instance_id,
            stream: OutputStream::Activity,
            line: "Inspecting current code behavior".to_string(),
        });

        let text = render_to_string(&parallel);

        assert!(
            text.contains("a:Inspecting current code behavior"),
            "should show activity summary: {text}"
        );
    }
}
