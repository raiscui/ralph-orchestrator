use crate::state::{TuiMode, TuiState};
use crate::theme::{EXABIND_BORDER_SET, TuiTheme};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};
use std::time::Instant;

/// Footer widget that adapts to terminal width.
pub struct Footer<'a> {
    state: &'a TuiState,
    theme: TuiTheme,
}

impl<'a> Footer<'a> {
    pub fn new(state: &'a TuiState, theme: TuiTheme) -> Self {
        Self { state, theme }
    }
}

impl Widget for Footer<'_> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        // Render block with top border as separator
        let block = Block::default()
            .borders(Borders::TOP)
            .border_set(EXABIND_BORDER_SET)
            .border_style(Style::default().fg(self.theme.colors().surface0))
            .style(self.theme.app_bg());
        let inner_area = block.inner(area);
        block.render(area, buf);

        // Search input mode: show prompt even if query is empty.
        if self.state.search_state.search_mode {
            let line = Line::from(vec![
                Span::raw(" "),
                Span::styled(
                    format!("/{}", self.state.search_query),
                    Style::default().fg(self.theme.colors().yellow),
                ),
            ]);
            Paragraph::new(line).render(inner_area, buf);
            return;
        }

        // If search state has an active query, render search display
        if let Some(query) = &self.state.search_state.query {
            let match_info = if self.state.search_state.matches.is_empty() {
                "no matches".to_string()
            } else {
                format!(
                    "{}/{}",
                    self.state.search_state.current_match + 1,
                    self.state.search_state.matches.len()
                )
            };

            let line = Line::from(vec![
                Span::raw(" "),
                Span::styled(
                    format!("Search: {} ", query),
                    Style::default().fg(self.theme.colors().yellow),
                ),
                Span::styled(match_info, Style::default().fg(self.theme.colors().sky)),
            ]);

            Paragraph::new(line).render(inner_area, buf);
            return;
        }

        // Default footer with flexible layout.
        // Build left content: optional alert + elapsed time / activity summary.
        let mut left_spans = vec![Span::raw(" ")];

        // Show new iteration alert when viewing history and a new iteration arrived
        if let Some(iter_num) = self.state.new_iteration_alert
            && !self.state.following_latest
        {
            left_spans.push(Span::styled(
                format!("▶ New: iter {} ", iter_num),
                Style::default().fg(self.theme.colors().green),
            ));
            left_spans.push(Span::raw("│ "));
        }

        if self.state.mode != TuiMode::Parallel {
            let elapsed_display = if let Some(elapsed) = self.state.get_loop_elapsed() {
                let total_secs = elapsed.as_secs();
                let mins = total_secs / 60;
                let secs = total_secs % 60;
                format!("Total Time Elapsed: {mins:02}:{secs:02}")
            } else {
                "Total Time Elapsed: 00:00".to_string()
            };
            left_spans.push(Span::raw(elapsed_display));
        }

        if let Some(status) = &self.state.serial_output_status {
            left_spans.push(Span::raw(" │ "));
            left_spans.push(Span::styled(
                status.clone(),
                Style::default().fg(self.theme.colors().sky),
            ));
        }

        // 并行模式下,footer 直接展示“当前在做什么/当前看的是哪个实例/哪个 job/最近事件/渲染模式”。
        // 这些字段都来自现有 TUI state,避免为了展示层再维护第二套状态真相源。
        if self.state.mode == TuiMode::Parallel {
            let now = Instant::now();
            if let Some(instance) = self.state.parallel.selected_instance() {
                if let Some(activity) = instance.current_activity_summary(now) {
                    left_spans.push(Span::styled(
                        activity,
                        Style::default().fg(self.theme.colors().green),
                    ));
                } else {
                    let elapsed_display = if let Some(elapsed) = self.state.get_loop_elapsed() {
                        let total_secs = elapsed.as_secs();
                        let mins = total_secs / 60;
                        let secs = total_secs % 60;
                        format!("Total Time Elapsed: {mins:02}:{secs:02}")
                    } else {
                        "Total Time Elapsed: 00:00".to_string()
                    };
                    left_spans.push(Span::raw(elapsed_display));
                }

                left_spans.push(Span::raw(" │ "));
                left_spans.push(Span::styled(
                    self.state
                        .parallel
                        .selected_instance_id()
                        .map(|id| id.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                    Style::default().fg(self.theme.colors().sky),
                ));
                left_spans.push(Span::raw(" "));
                left_spans.push(Span::styled(
                    instance.state.to_string(),
                    Style::default().fg(self.theme.colors().sky),
                ));

                if let Some(job) = instance.current_job_short_summary() {
                    left_spans.push(Span::raw(" "));
                    left_spans.push(Span::styled(job, self.theme.muted()));
                }
            } else {
                let elapsed_display = if let Some(elapsed) = self.state.get_loop_elapsed() {
                    let total_secs = elapsed.as_secs();
                    let mins = total_secs / 60;
                    let secs = total_secs % 60;
                    format!("Total Time Elapsed: {mins:02}:{secs:02}")
                } else {
                    "Total Time Elapsed: 00:00".to_string()
                };
                left_spans.push(Span::raw(elapsed_display));
            }

            left_spans.push(Span::raw(" "));
            left_spans.push(Span::styled(
                format!("m:{}", self.state.parallel.output_view_mode.short_label()),
                Style::default().fg(self.theme.colors().yellow),
            ));

            if let Some(last_event) = &self.state.last_event {
                left_spans.push(Span::raw(" "));
                left_spans.push(Span::styled(
                    format!("e:{last_event}"),
                    Style::default().fg(self.theme.colors().sky),
                ));
            }
        }

        let indicator_text = if self.state.loop_completed {
            "■ DONE"
        } else {
            "◉ ACTIVE"
        };

        let indicator_style = if self.state.loop_completed {
            Style::default().fg(self.theme.colors().blue)
        } else {
            Style::default().fg(self.theme.colors().green)
        };

        // Calculate left content width for layout
        let left_content_width: usize = left_spans.iter().map(|s| s.width()).sum();

        // Use horizontal layout: left content | flexible spacer | right indicator
        let chunks = Layout::horizontal([
            Constraint::Length(left_content_width as u16), // Alert + " Last: event"
            Constraint::Fill(1),                           // Flexible spacer
            Constraint::Length((indicator_text.len() + 2) as u16), // "indicator "
        ])
        .split(inner_area);

        // Render left side (alert + last event)
        let left = Line::from(left_spans);
        Paragraph::new(left)
            .style(self.theme.text().bg(self.theme.app_bg_color()))
            .render(chunks[0], buf);

        // Render right side (indicator)
        let right = Line::from(vec![
            Span::styled(indicator_text, indicator_style),
            Span::raw(" "),
        ]);
        Paragraph::new(right)
            .style(self.theme.text().bg(self.theme.app_bg_color()))
            .render(chunks[2], buf);
    }
}

/// Convenience function for rendering the footer.
pub fn render(state: &TuiState, theme: TuiTheme) -> Footer<'_> {
    Footer::new(state, theme)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::TuiTheme;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn render_to_string(state: &TuiState) -> String {
        render_to_string_with_width(state, 80)
    }

    fn render_to_string_with_width(state: &TuiState, width: u16) -> String {
        // Height of 2: 1 for top border + 1 for content
        let backend = TestBackend::new(width, 2);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let widget = render(state, TuiTheme::default());
                f.render_widget(widget, f.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        buffer
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>()
    }

    // =========================================================================
    // Acceptance Criteria Tests (Task 06)
    // =========================================================================

    #[test]
    fn footer_shows_new_iteration_alert() {
        // Given new_iteration_alert = Some(5) and following_latest = false
        let mut state = TuiState::new();
        state.new_iteration_alert = Some(5);
        state.following_latest = false;

        // When footer renders
        let text = render_to_string(&state);

        // Then output contains "▶ New: iter 5"
        assert!(
            text.contains("▶ New: iter 5"),
            "should show new iteration alert, got: {}",
            text
        );
    }

    #[test]
    fn footer_no_alert_when_following() {
        // Given following_latest = true (even if new_iteration_alert has a value)
        let mut state = TuiState::new();
        state.new_iteration_alert = Some(5);
        state.following_latest = true;

        // When footer renders
        let text = render_to_string(&state);

        // Then no alert is shown
        assert!(
            !text.contains("▶ New:"),
            "should NOT show alert when following_latest=true, got: {}",
            text
        );
    }

    #[test]
    fn footer_shows_elapsed_time() {
        // Given loop_started is set (simulating 2 minutes 30 seconds elapsed)
        let mut state = TuiState::new();
        state.loop_started = Some(
            std::time::Instant::now()
                .checked_sub(std::time::Duration::from_secs(150))
                .unwrap(),
        );

        // When footer renders
        let text = render_to_string(&state);

        // Then output contains "Total Time Elapsed: MM:SS" format
        assert!(
            text.contains("Total Time Elapsed: 02:30"),
            "should show 'Total Time Elapsed: 02:30', got: {}",
            text
        );
    }

    #[test]
    fn footer_shows_active_indicator() {
        // Given pending_hat is set (task in progress)
        let mut state = TuiState::new();
        state.pending_hat = Some((ralph_proto::HatId::new("builder"), "🔨Builder".to_string()));

        // When footer renders
        let text = render_to_string(&state);

        // Then output contains ◉ ACTIVE
        assert!(
            text.contains('◉') && text.contains("ACTIVE"),
            "should show ACTIVE indicator, got: {}",
            text
        );
    }

    #[test]
    fn footer_shows_search_query() {
        // Given search_state has an active query
        let mut state = TuiState::new();
        state.search_state.query = Some("test".to_string());
        state.search_state.matches = vec![(0, 0), (1, 0)]; // 2 matches

        // When footer renders
        let text = render_to_string(&state);

        // Then output contains "Search: test 1/2"
        assert!(
            text.contains("Search: test"),
            "should show search query, got: {}",
            text
        );
        assert!(
            text.contains("1/2"),
            "should show match position, got: {}",
            text
        );
    }

    #[test]
    fn footer_shows_parallel_status_summary() {
        use crate::state::TuiUpdate;
        use ralph_core::{HatJobOutputChunk, OutputStream};
        use ralph_proto::{Event, HatInstanceId, HatInstanceState};

        let mut state = TuiState::new_parallel();
        let instance_id = HatInstanceId::from("writer#1");

        state.apply_update(TuiUpdate::ParallelRegisterInstance {
            instance_id: instance_id.clone(),
            state: HatInstanceState::Running,
        });
        state.apply_update(TuiUpdate::ParallelOutputChunk(HatJobOutputChunk {
            job_id: 7,
            instance_id,
            stream: OutputStream::Stdout,
            line: "hello".to_string(),
        }));
        state.apply_update(TuiUpdate::ParallelEvent(
            Event::new("reply.human.message", "done").with_source_instance("writer#1"),
        ));
        state.parallel.output_view_mode = crate::state::parallel::ParallelOutputViewMode::Plain;

        let text = render_to_string_with_width(&state, 140);

        assert!(
            text.contains("writer#1"),
            "should show selected instance, got: {}",
            text
        );
        assert!(
            text.contains("running"),
            "should show instance state, got: {}",
            text
        );
        assert!(
            text.contains("j1/1"),
            "should show job summary, got: {}",
            text
        );
        assert!(
            text.contains("Working") && text.contains("Ctrl+C to interrupt"),
            "should show current activity summary, got: {}",
            text
        );
        assert!(
            text.contains("e:reply.human.message"),
            "should show last event, got: {}",
            text
        );
        assert!(
            text.contains("m:P"),
            "should show render mode, got: {}",
            text
        );
    }

    #[test]
    fn footer_shows_codex_style_parallel_activity() {
        use crate::state::TuiUpdate;
        use ralph_core::{HatJobOutputChunk, OutputStream};
        use ralph_proto::{HatInstanceId, HatInstanceState};

        let mut state = TuiState::new_parallel();
        let instance_id = HatInstanceId::from("coder#1");

        state.apply_update(TuiUpdate::ParallelRegisterInstance {
            instance_id: instance_id.clone(),
            state: HatInstanceState::Running,
        });
        state.apply_update(TuiUpdate::ParallelOutputChunk(HatJobOutputChunk {
            job_id: 9,
            instance_id: instance_id.clone(),
            stream: OutputStream::Activity,
            line: "Inspecting current code behavior".to_string(),
        }));

        let view = state
            .parallel
            .instances
            .get_mut(&instance_id)
            .expect("instance should exist");
        let activity = view
            .current_activity
            .as_mut()
            .expect("activity should be set");
        activity.started_at = std::time::Instant::now()
            .checked_sub(std::time::Duration::from_secs(29))
            .expect("test clock should support subtraction");

        let text = render_to_string_with_width(&state, 140);

        assert!(
            text.contains("Inspecting current code behavior"),
            "should show Codex-style activity label, got: {}",
            text
        );
        assert!(
            text.contains("29s"),
            "should show activity elapsed time, got: {}",
            text
        );
        assert!(
            text.contains("Ctrl+C to interrupt"),
            "should show the real Ralph interrupt hint, got: {}",
            text
        );
    }

    #[test]
    fn footer_shows_audit_parallel_output_mode() {
        let mut state = TuiState::new_parallel();
        state.parallel.output_view_mode = crate::state::parallel::ParallelOutputViewMode::Audit;

        let text = render_to_string_with_width(&state, 120);

        assert!(
            text.contains("m:A"),
            "audit mode should be visible in footer, got: {text}"
        );
    }

    #[test]
    fn footer_shows_no_matches_when_empty() {
        // Given search with no matches
        let mut state = TuiState::new();
        state.search_state.query = Some("notfound".to_string());
        state.search_state.matches = vec![];

        // When footer renders
        let text = render_to_string(&state);

        // Then output contains "no matches"
        assert!(
            text.contains("no matches"),
            "should show no matches indicator, got: {}",
            text
        );
    }

    #[test]
    fn footer_shows_done_indicator_when_complete() {
        // Given loop_completed = true (task complete after loop.terminate)
        let mut state = TuiState::new();
        state.loop_completed = true;

        // When footer renders
        let text = render_to_string(&state);

        // Then output contains ■ DONE
        assert!(
            text.contains('■') && text.contains("DONE"),
            "should show DONE indicator, got: {}",
            text
        );
    }

    #[test]
    fn footer_shows_active_at_startup() {
        // Given fresh state (loop not yet completed)
        let state = TuiState::new();

        // When footer renders
        let text = render_to_string(&state);

        // Then output contains ◉ ACTIVE (not DONE)
        assert!(
            text.contains('◉') && text.contains("ACTIVE"),
            "should show ACTIVE indicator at startup, got: {}",
            text
        );
    }
}
