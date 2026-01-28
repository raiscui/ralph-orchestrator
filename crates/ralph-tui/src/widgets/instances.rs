//! 并行模式：实例列表面板（HatInstance 列表）。

use crate::state::{ParallelFocus, ParallelTuiState};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Widget},
};

/// 左侧实例列表面板。
pub struct InstancesPane<'a> {
    parallel: &'a ParallelTuiState,
    focused: bool,
}

impl<'a> InstancesPane<'a> {
    pub fn new(parallel: &'a ParallelTuiState) -> Self {
        Self {
            parallel,
            focused: parallel.focus == ParallelFocus::Instances,
        }
    }
}

impl Widget for InstancesPane<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let border_style = if self.focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        };

        let block = Block::default()
            .title("Instances")
            .borders(Borders::ALL)
            .border_style(border_style);
        let inner = block.inner(area);
        block.render(area, buf);

        if inner.width == 0 || inner.height == 0 {
            return;
        }

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
                let line = Line::from(vec![
                    Span::styled(
                        id.as_str().to_string(),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" "),
                    Span::raw(state),
                    Span::raw(" "),
                    Span::styled(age, Style::default().fg(Color::DarkGray)),
                ]);
                ListItem::new(line)
            })
            .collect();

        let mut state = ListState::default();
        if !items.is_empty() {
            state.select(Some(self.parallel.selected_instance));
        }

        let list = List::new(items).highlight_style(
            Style::default()
                .add_modifier(Modifier::REVERSED)
                .fg(Color::Yellow),
        );
        ratatui::widgets::StatefulWidget::render(list, inner, buf, &mut state);
    }
}

/// Convenience helper.
pub fn render(parallel: &ParallelTuiState) -> InstancesPane<'_> {
    InstancesPane::new(parallel)
}
