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
                    Span::styled(age, self.theme.muted()),
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
