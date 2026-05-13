//! Help overlay widget.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::theme::MUTED_FG;

/// Renders help overlay centered on screen.
pub fn render(f: &mut Frame, area: Rect) {
    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black).fg(Color::White));

    let help_text = vec![
        Line::from(Span::styled(
            "Navigation:",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(vec![
            Span::styled("  h/←", Style::default().fg(Color::Cyan)),
            Span::raw("    Previous iteration"),
        ]),
        Line::from(vec![
            Span::styled("  l/→", Style::default().fg(Color::Cyan)),
            Span::raw("    Next iteration"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Scrolling:",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(vec![
            Span::styled("  j/↓", Style::default().fg(Color::Cyan)),
            Span::raw("    Scroll down"),
        ]),
        Line::from(vec![
            Span::styled("  k/↑", Style::default().fg(Color::Cyan)),
            Span::raw("    Scroll up"),
        ]),
        Line::from(vec![
            Span::styled("  g", Style::default().fg(Color::Cyan)),
            Span::raw("      Scroll to top"),
        ]),
        Line::from(vec![
            Span::styled("  G", Style::default().fg(Color::Cyan)),
            Span::raw("      Scroll to bottom"),
        ]),
        Line::from(""),
        Line::from(Span::styled("Search:", Style::default().fg(Color::Yellow))),
        Line::from(vec![
            Span::styled("  /", Style::default().fg(Color::Cyan)),
            Span::raw("      Start search"),
        ]),
        Line::from(vec![
            Span::styled("  n/N", Style::default().fg(Color::Cyan)),
            Span::raw("    Next/prev match"),
        ]),
        Line::from(""),
        Line::from(Span::styled("Mouse:", Style::default().fg(Color::Yellow))),
        Line::from(vec![
            Span::styled("  Click", Style::default().fg(Color::Cyan)),
            Span::raw("  Select output / instance / gate / focus chat"),
        ]),
        Line::from(vec![
            Span::styled("  Drag", Style::default().fg(Color::Cyan)),
            Span::raw("   Select output/chat text (auto-copies)"),
        ]),
        Line::from(vec![
            Span::styled("  Click @chip", Style::default().fg(Color::Cyan)),
            Span::raw("  Set default chat target"),
        ]),
        Line::from(vec![
            Span::styled("  Click !chip", Style::default().fg(Color::Cyan)),
            Span::raw("  Prefill gate command (no send)"),
        ]),
        Line::from(vec![
            Span::styled("  Wheel", Style::default().fg(Color::Cyan)),
            Span::raw("  Scroll output"),
        ]),
        Line::from(""),
        Line::from(Span::styled("Other:", Style::default().fg(Color::Yellow))),
        Line::from(vec![
            Span::styled("  q", Style::default().fg(Color::Cyan)),
            Span::raw("      Quit (stops the run)"),
        ]),
        Line::from(vec![
            Span::styled("  y", Style::default().fg(Color::Cyan)),
            Span::raw("      Copy current output selection"),
        ]),
        Line::from(vec![
            Span::styled("  p", Style::default().fg(Color::Cyan)),
            Span::raw("      Toggle hat graph zoom"),
        ]),
        Line::from(vec![
            Span::styled("  Tab", Style::default().fg(Color::Cyan)),
            Span::raw("    Switch focus (parallel)"),
        ]),
        Line::from(vec![
            Span::styled("  Shift+Enter", Style::default().fg(Color::Cyan)),
            Span::raw("  Newline in chat (parallel)"),
        ]),
        Line::from(vec![
            Span::styled("  Alt+Enter", Style::default().fg(Color::Cyan)),
            Span::raw("    Newline in chat (parallel)"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+J", Style::default().fg(Color::Cyan)),
            Span::raw("      Newline in chat (parallel)"),
        ]),
        Line::from(vec![
            Span::styled("  ?", Style::default().fg(Color::Cyan)),
            Span::raw("      Show this help"),
        ]),
        Line::from(vec![
            Span::styled("  Esc", Style::default().fg(Color::Cyan)),
            Span::raw("    Dismiss/cancel"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Press Esc to dismiss",
            Style::default().fg(MUTED_FG),
        )),
    ];

    let paragraph = Paragraph::new(help_text)
        .block(block)
        .alignment(Alignment::Left);

    let popup_area = centered_rect(50, 60, area);
    f.render_widget(Clear, popup_area);
    f.render_widget(paragraph, popup_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
