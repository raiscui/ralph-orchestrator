//! Main application loop for the TUI.
//!
//! This module provides a read-only observation dashboard that displays
//! formatted output from the Ralph orchestrator, with iteration navigation,
//! scroll, and search functionality.

use crate::chat::{ChatSubmit, parse_chat_submit};
use crate::external_event_writer::ExternalEventWriter;
use crate::input::{Action, map_key};
use crate::state::{GateStatus, ParallelFocus, TuiMode, TuiState, TuiUpdate};
use crate::widgets::{content::ContentPane, footer, header, help, instances};
use anyhow::Result;
use crossterm::{
    cursor::Show,
    event::{
        DisableMouseCapture, EnableMouseCapture, Event, EventStream, KeyCode, KeyEventKind,
        KeyModifiers, MouseEventKind,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use futures::StreamExt;
use ralph_core::truncate_with_ellipsis;
use ralph_proto::{GateResolve, GateResolvedBy, TOPIC_GATE_RESOLVE};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use scopeguard::defer;
use std::io;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::sync::watch;
use tokio::time::{Duration, interval};
use tracing::info;

/// Dispatches an action to the TuiState.
///
/// Returns `true` if the action signals to quit the application.
pub fn dispatch_action(action: Action, state: &mut TuiState, viewport_height: usize) -> bool {
    match action {
        Action::Quit => return true,
        Action::ScrollDown => {
            if let Some(buffer) = state.current_output_buffer_mut() {
                buffer.scroll_down(viewport_height);
            }
        }
        Action::ScrollUp => {
            if let Some(buffer) = state.current_output_buffer_mut() {
                buffer.scroll_up();
            }
        }
        Action::ScrollTop => {
            if let Some(buffer) = state.current_output_buffer_mut() {
                buffer.scroll_top();
            }
        }
        Action::ScrollBottom => {
            if let Some(buffer) = state.current_output_buffer_mut() {
                buffer.scroll_bottom(viewport_height);
            }
        }
        Action::NextIteration => match state.mode {
            TuiMode::Serial => state.navigate_next(),
            TuiMode::Parallel => {
                if state.parallel.focus == ParallelFocus::Output {
                    state.parallel.select_next_job();
                }
            }
        },
        Action::PrevIteration => match state.mode {
            TuiMode::Serial => state.navigate_prev(),
            TuiMode::Parallel => {
                if state.parallel.focus == ParallelFocus::Output {
                    state.parallel.select_prev_job();
                }
            }
        },
        Action::ShowHelp => {
            state.show_help = true;
        }
        Action::DismissHelp => {
            state.show_help = false;
            state.search_state.search_mode = false;
            state.search_query.clear();
            state.clear_search();
        }
        Action::StartSearch => {
            state.search_state.search_mode = true;
            state.search_query.clear();
        }
        Action::SearchNext => {
            state.next_match();
        }
        Action::SearchPrev => {
            state.prev_match();
        }
        Action::None => {}
    }
    false
}

/// Main TUI application for read-only observation.
pub struct App {
    state: Arc<Mutex<TuiState>>,
    /// Receives notification when the underlying process terminates.
    /// This is the ONLY exit path for the TUI event loop (besides Action::Quit).
    terminated_rx: watch::Receiver<bool>,
    /// Channel to signal main loop on Ctrl+C.
    /// In raw terminal mode, SIGINT is not generated, so TUI must signal
    /// the main orchestration loop through this channel.
    interrupt_tx: Option<watch::Sender<bool>>,

    /// 并行模式：UI 更新通道（observer → channel → reducer）。
    update_rx: Option<mpsc::UnboundedReceiver<TuiUpdate>>,
}

impl App {
    /// Creates a new App with shared state, termination signal, and optional interrupt channel.
    pub fn new(
        state: Arc<Mutex<TuiState>>,
        terminated_rx: watch::Receiver<bool>,
        interrupt_tx: Option<watch::Sender<bool>>,
        update_rx: Option<mpsc::UnboundedReceiver<TuiUpdate>>,
    ) -> Self {
        Self {
            state,
            terminated_rx,
            interrupt_tx,
            update_rx,
        }
    }

    /// Runs the TUI event loop.
    pub async fn run(mut self) -> Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // CRITICAL: Ensure terminal cleanup on ANY exit path (normal, abort, or panic).
        // When cleanup_tui() calls handle.abort(), the task is cancelled immediately
        // at its current await point, skipping all code after the loop. This defer!
        // guard runs on Drop, which is guaranteed even during task cancellation.
        defer! {
            let _ = disable_raw_mode();
            let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture, Show);
        }

        // Event-driven architecture: input polling is the primary driver
        // Render is throttled to ~60fps via interval tick
        let mut events = EventStream::new();
        let mut render_tick = interval(Duration::from_millis(16));

        // Track viewport height for scroll calculations
        let mut viewport_height: usize = 24; // Default, updated on render

        // 并行模式的 state 更新通道（由 App 消费）
        let mut update_rx = self.update_rx.take();

        loop {
            // Use biased select to prioritize input over render ticks
            tokio::select! {
                biased;

                // Priority 1: Handle input events immediately for responsiveness
                maybe_event = events.next() => {
                    match maybe_event {
                        Some(Ok(event)) => {
                            match event {
                                // Handle Ctrl+C: signal main loop and exit.
                                // In raw mode, SIGINT is not generated, so we must signal the
                                // main orchestration loop through interrupt_tx channel.
                                Event::Key(key) if key.kind == KeyEventKind::Press
                                    && key.code == KeyCode::Char('c')
                                    && key.modifiers.contains(KeyModifiers::CONTROL) =>
                                {
                                    info!("Ctrl+C detected, signaling main loop");
                                    if let Some(ref tx) = self.interrupt_tx {
                                        let _ = tx.send(true);
                                    }
                                    break;
                                }
                                Event::Mouse(mouse) => {
                                    match mouse.kind {
                                        MouseEventKind::ScrollUp => {
                                            let mut state = self.state.lock().unwrap();
                                            if let Some(buffer) = state.current_output_buffer_mut() {
                                                for _ in 0..3 {
                                                    buffer.scroll_up();
                                                }
                                            }
                                        }
                                        MouseEventKind::ScrollDown => {
                                            let mut state = self.state.lock().unwrap();
                                            if let Some(buffer) = state.current_output_buffer_mut() {
                                                for _ in 0..3 {
                                                    buffer.scroll_down(viewport_height);
                                                }
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                                Event::Key(key) if key.kind == KeyEventKind::Press => {
                                    // Dismiss help on any key when help is showing
                                    {
                                        let mut state = self.state.lock().unwrap();
                                        if state.show_help {
                                            state.show_help = false;
                                            continue;
                                        }
                                    }

                                    // 串行/并行：按 mode 分流输入处理
                                    let mut state = self.state.lock().unwrap();

                                    // 搜索输入模式（串行/并行共用）。
                                    if state.search_state.search_mode {
                                        match key.code {
                                            KeyCode::Esc => {
                                                state.search_state.search_mode = false;
                                                state.search_query.clear();
                                                state.clear_search();
                                            }
                                            KeyCode::Backspace => {
                                                state.search_query.pop();
                                            }
                                            KeyCode::Enter => {
                                                let query = state.search_query.trim().to_string();
                                                state.search_state.search_mode = false;
                                                state.search_query.clear();

                                                if query.is_empty() {
                                                    state.clear_search();
                                                } else {
                                                    state.search(&query);
                                                }
                                            }
                                            KeyCode::Char(c) => {
                                                state.search_query.push(c);
                                            }
                                            _ => {}
                                        }
                                        continue;
                                    }

                                    match state.mode {
                                        TuiMode::Serial => {
                                            let action = map_key(key);
                                            if dispatch_action(action, &mut state, viewport_height) {
                                                break;
                                            }
                                        }
                                        TuiMode::Parallel => {
                                            // 3.x：并行模式的输入映射（焦点/导航/滚动/搜索）。
                                            // 5.x/6.x：chat/gate 交互（写外部事件 + 展示 gate 列表）。

                                            // Focus switching first (Tab / BackTab)
                                            if key.code == KeyCode::Tab {
                                                state.parallel.focus_next();
                                                continue;
                                            }
                                            if key.code == KeyCode::BackTab {
                                                state.parallel.focus_prev();
                                                continue;
                                            }

                                            // Global keys（注意：Chat 焦点下字符应当进入输入框，不应触发 quit/help）
                                            let focus = state.parallel.focus;
                                            if focus != ParallelFocus::Chat {
                                                if key.code == KeyCode::Char('q') {
                                                    break;
                                                }
                                                if key.code == KeyCode::Char('?') {
                                                    state.show_help = true;
                                                    continue;
                                                }
                                            }

                                            match focus {
                                                ParallelFocus::Instances => match key.code {
                                                    KeyCode::Up | KeyCode::Char('k') => {
                                                        state.parallel.select_prev_instance();
                                                    }
                                                    KeyCode::Down | KeyCode::Char('j') => {
                                                        state.parallel.select_next_instance();
                                                    }
                                                    KeyCode::Right | KeyCode::Char('l') | KeyCode::Enter => {
                                                        state.parallel.focus = ParallelFocus::Output;
                                                    }
                                                    _ => {}
                                                },
                                                ParallelFocus::Output => {
                                                    let action = map_key(key);
                                                    if dispatch_action(action, &mut state, viewport_height) {
                                                        break;
                                                    }
                                                }
                                                ParallelFocus::Chat => {
                                                    match key.code {
                                                        KeyCode::Esc => {
                                                            state.parallel.chat_input.clear();
                                                        }
                                                        KeyCode::Backspace => {
                                                            state.parallel.chat_input.pop();
                                                        }
                                                        KeyCode::Enter => {
                                                            let raw = state.parallel.chat_input.trim().to_string();
                                                            state.parallel.chat_input.clear();
                                                            if raw.is_empty() {
                                                                continue;
                                                            }

                                                            match parse_chat_submit(&raw) {
                                                                Ok(ChatSubmit::HumanMessage { target_instance, payload }) => {
                                                                    let writer = ExternalEventWriter::new();
                                                                    match writer.append("human.message", payload, target_instance) {
                                                                        Ok(()) => {
                                                                            state.parallel.chat_status = Some(format!(
                                                                                "sent human.message -> {}",
                                                                                writer.path().display()
                                                                            ));
                                                                        }
                                                                        Err(e) => {
                                                                            state.parallel.chat_status = Some(format!("send failed: {e:#}"));
                                                                        }
                                                                    }
                                                                }
                                                                Ok(ChatSubmit::GateResolve { gate_id, decision }) => {
                                                                    let requested_by = state
                                                                        .parallel
                                                                        .gates
                                                                        .get(&gate_id)
                                                                        .map(|g| g.request.requested_by.clone());

                                                                    let resolve = GateResolve {
                                                                        gate_id,
                                                                        resolved_by: GateResolvedBy::Human,
                                                                        decision,
                                                                        requested_by,
                                                                    };

                                                                    match serde_json::to_string(&resolve) {
                                                                        Ok(payload) => {
                                                                            let writer = ExternalEventWriter::new();
                                                                            match writer.append(TOPIC_GATE_RESOLVE, payload, None) {
                                                                                Ok(()) => {
                                                                                    state.parallel.chat_status = Some(format!(
                                                                                        "sent gate.resolve -> {}",
                                                                                        writer.path().display()
                                                                                    ));
                                                                                }
                                                                                Err(e) => {
                                                                                    state.parallel.chat_status = Some(format!("send failed: {e:#}"));
                                                                                }
                                                                            }
                                                                        }
                                                                        Err(e) => {
                                                                            state.parallel.chat_status = Some(format!("serialize failed: {e}"));
                                                                        }
                                                                    }
                                                                }
                                                                Err(e) => {
                                                                    state.parallel.chat_status = Some(format!("parse error: {e}"));
                                                                }
                                                            }
                                                        }
                                                        KeyCode::Char(c) => {
                                                            state.parallel.chat_input.push(c);
                                                        }
                                                        _ => {}
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                // Ignore other events (FocusGained, FocusLost, Paste, Resize, key releases)
                                _ => {}
                            }
                        }
                        Some(Err(e)) => {
                            // Log error but continue - transient errors shouldn't crash TUI
                            tracing::warn!("Event stream error: {}", e);
                        }
                        None => {
                            // Stream ended unexpectedly
                            break;
                        }
                    }
                }

                // Priority 2: Render at throttled rate (~60fps)
                _ = render_tick.tick() => {
                    let frame_size = terminal.size()?;
                    let frame_area = ratatui::layout::Rect::new(0, 0, frame_size.width, frame_size.height);
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(2),  // Header: content + bottom border
                            Constraint::Min(0),     // Content: flexible
                            Constraint::Length(2),  // Footer: top border + content
                        ])
                        .split(frame_area);

                    let content_area = chunks[1];
                    // viewport_height 代表“当前可滚动输出视图”的高度（串行=content， 并行=output inner）
                    // 注意：并行模式下 output 还有边框与底部 chat panel，因此需要做一次保守估计。
                    viewport_height = content_area.height as usize;

                    let mut state = self.state.lock().unwrap();

                    // Autoscroll（串行/并行）：如果用户没离开底部，就跟随输出
                    let effective_viewport_height = match state.mode {
                        TuiMode::Serial => content_area.height as usize,
                        TuiMode::Parallel => {
                            // content = main + bottom(7)
                            // output inner = main - borders(2)
                            let main_height = content_area.height.saturating_sub(7);
                            main_height.saturating_sub(2) as usize
                        }
                    };
                    if let Some(buffer) = state.current_output_buffer_mut()
                        && buffer.following_bottom
                    {
                        let max_scroll = buffer
                            .line_count()
                            .saturating_sub(effective_viewport_height);
                        buffer.scroll_offset = max_scroll;
                    }

                    let state = state; // Rebind as immutable for rendering
                    terminal.draw(|f| {
                        // Render header
                        f.render_widget(header::render(&state, chunks[0].width), chunks[0]);

                        match state.mode {
                            TuiMode::Serial => {
                                // Render content using ContentPane
                                if let Some(buffer) = state.current_iteration() {
                                    let mut content_widget = ContentPane::new(buffer);
                                    if let Some(query) = &state.search_state.query {
                                        content_widget = content_widget.with_search(query);
                                    }
                                    f.render_widget(content_widget, content_area);
                                }
                            }
                            TuiMode::Parallel => {
                                // 布局：上（实例列表 + 输出） / 下（chat + gate）
                                let vertical = Layout::default()
                                    .direction(Direction::Vertical)
                                    .constraints([
                                        Constraint::Min(0),     // main
                                        Constraint::Length(7),  // bottom panel（后续可做自适应）
                                    ])
                                    .split(content_area);

                                let main_area = vertical[0];
                                let bottom_area = vertical[1];

                                let horizontal = Layout::default()
                                    .direction(Direction::Horizontal)
                                    .constraints([
                                        Constraint::Length(30), // instances
                                        Constraint::Min(0),     // output
                                    ])
                                    .split(main_area);

                                let instances_area = horizontal[0];
                                let output_area = horizontal[1];

                                // 左：实例列表
                                f.render_widget(instances::render(&state.parallel), instances_area);

                                // 右：输出（选中实例的当前 job）
                                let output_focused = state.parallel.focus == crate::state::ParallelFocus::Output;
                                let output_border_style = if output_focused {
                                    Style::default().fg(Color::Cyan)
                                } else {
                                    Style::default()
                                };

                                let title = if let Some(id) = state.parallel.selected_instance_id() {
                                    if let Some(instance) = state.parallel.selected_instance() {
                                        let state_label = instance.state.to_string();
                                        let total = instance.jobs.len();
                                        if total > 0 {
                                            let current = instance.current_job.saturating_add(1);
                                            format!("Output ({id}) [{state_label}] [job {current}/{total}]")
                                        } else {
                                            format!("Output ({id}) [{state_label}]")
                                        }
                                    } else {
                                        format!("Output ({id})")
                                    }
                                } else {
                                    "Output".to_string()
                                };
                                let block = Block::default()
                                    .title(title)
                                    .borders(Borders::ALL)
                                    .border_style(output_border_style);
                                let inner = block.inner(output_area);
                                f.render_widget(block, output_area);

                                // 更新可滚动视图高度（给鼠标滚动/键盘滚动用）
                                viewport_height = inner.height as usize;

                                if let Some(instance) = state.parallel.selected_instance()
                                    && let Some(buffer) = instance.current_job_buffer()
                                {
                                    let mut content_widget = ContentPane::new(buffer);
                                    if let Some(query) = &state.search_state.query {
                                        content_widget = content_widget.with_search(query);
                                    }
                                    f.render_widget(content_widget, inner);
                                } else {
                                    let empty = Paragraph::new(Line::from(vec![
                                        Span::raw(" "),
                                        Span::styled("No instance selected", Style::default().fg(Color::DarkGray)),
                                    ]));
                                    f.render_widget(empty, inner);
                                }

                                // 下：chat + gate（human async chat + gate 面板）
                                let bottom_focused = state.parallel.focus == crate::state::ParallelFocus::Chat;
                                let bottom_border_style = if bottom_focused {
                                    Style::default().fg(Color::Cyan)
                                } else {
                                    Style::default()
                                };
                                let bottom_block = Block::default()
                                    .title("Chat / Gates")
                                    .borders(Borders::ALL)
                                    .border_style(bottom_border_style);
                                let bottom_inner = bottom_block.inner(bottom_area);
                                f.render_widget(bottom_block, bottom_area);

                                if bottom_inner.width > 0 && bottom_inner.height > 0 {
                                    // 上：输入框 / 中：状态提示 / 下：gate 列表
                                    let inner_chunks = Layout::default()
                                        .direction(Direction::Vertical)
                                        .constraints([
                                            Constraint::Length(1), // input
                                            Constraint::Length(1), // status
                                            Constraint::Min(0),    // gates
                                        ])
                                        .split(bottom_inner);

                                    let input_area = inner_chunks[0];
                                    let status_area = inner_chunks[1];
                                    let gates_area = inner_chunks[2];

                                    // 1) chat 输入行
                                    let prompt_style = if bottom_focused {
                                        Style::default().fg(Color::Cyan)
                                    } else {
                                        Style::default().fg(Color::DarkGray)
                                    };

                                    let input_text = if state.parallel.chat_input.is_empty() {
                                        Span::styled(
                                            "Type: @instance msg | !approve/!deny/!resolve ...",
                                            Style::default().fg(Color::DarkGray),
                                        )
                                    } else {
                                        Span::raw(state.parallel.chat_input.clone())
                                    };

                                    let input_line = Line::from(vec![
                                        Span::raw(" "),
                                        Span::styled(">", prompt_style),
                                        Span::raw(" "),
                                        input_text,
                                    ]);
                                    f.render_widget(Paragraph::new(input_line), input_area);

                                    // 2) 状态提示
                                let status = state
                                    .parallel
                                    .chat_status
                                    .as_deref()
                                    .unwrap_or(if bottom_focused {
                                        "Enter=send  Esc=clear  Tab=switch"
                                    } else {
                                        "Tab to focus chat"
                                    });
                                    let status_line = Line::from(vec![
                                        Span::raw(" "),
                                        Span::styled(status.to_string(), Style::default().fg(Color::DarkGray)),
                                    ]);
                                    f.render_widget(Paragraph::new(status_line), status_area);

                                    // 3) gate 列表（最新在上）
                                    let mut gate_lines: Vec<Line> = Vec::new();
                                    let max_lines = gates_area.height as usize;

                                    for gate_id in state.parallel.gate_order.iter().rev() {
                                        if gate_lines.len() >= max_lines {
                                            break;
                                        }

                                        let Some(g) = state.parallel.gates.get(gate_id) else {
                                            continue;
                                        };

                                        let kind = match g.request.kind {
                                            ralph_proto::GateKind::Consult => "consult",
                                            ralph_proto::GateKind::Approval => "approval",
                                        };

                                        let now = std::time::Instant::now();
                                        let (status_text, status_style) = match g.status_at(now) {
                                            GateStatus::Resolved => (
                                                "resolved".to_string(),
                                                Style::default().fg(Color::Green),
                                            ),
                                            GateStatus::Timeout => (
                                                "timeout".to_string(),
                                                Style::default().fg(Color::Yellow),
                                            ),
                                            GateStatus::Waiting { remaining_seconds } => (
                                                format!("T-{remaining_seconds}s"),
                                                Style::default().fg(Color::Cyan),
                                            ),
                                            GateStatus::Open => (
                                                "open".to_string(),
                                                Style::default().fg(Color::Cyan),
                                            ),
                                        };

                                        let prompt = truncate_with_ellipsis(&g.request.prompt, 48);

                                        gate_lines.push(Line::from(vec![
                                            Span::raw(" "),
                                            Span::styled(format!("[{kind}]"), Style::default().fg(Color::Magenta)),
                                            Span::raw(" "),
                                            Span::styled(
                                                gate_id.clone(),
                                                Style::default()
                                                    .add_modifier(ratatui::style::Modifier::BOLD),
                                            ),
                                            Span::raw(" "),
                                            Span::styled(status_text, status_style),
                                            Span::raw(" "),
                                            Span::styled(
                                                g.request.requested_by.to_string(),
                                                Style::default().fg(Color::DarkGray),
                                            ),
                                            Span::raw(" "),
                                            Span::raw(prompt),
                                        ]));
                                    }

                                    if gate_lines.is_empty() {
                                        gate_lines.push(Line::from(vec![
                                            Span::raw(" "),
                                            Span::styled("No gates", Style::default().fg(Color::DarkGray)),
                                        ]));
                                    }

                                    f.render_widget(Paragraph::new(gate_lines), gates_area);
                                }
                            }
                        }

                        // Render footer
                        f.render_widget(footer::render(&state), chunks[2]);

                        // Render help overlay if active
                        if state.show_help {
                            help::render(f, f.area());
                        }
                    })?;
                }

                // Priority 2.5: Apply updates from parallel runner (observer → channel)
                maybe_update = async {
                    if let Some(rx) = update_rx.as_mut() {
                        rx.recv().await
                    } else {
                        std::future::pending::<Option<TuiUpdate>>().await
                    }
                } => {
                    if let Some(update) = maybe_update {
                        let mut state = self.state.lock().unwrap();
                        state.apply_update(update);
                    }
                }

                // Priority 3: Handle termination signal
                _ = self.terminated_rx.changed() => {
                    if *self.terminated_rx.borrow() {
                        break;
                    }
                }
            }
        }

        // NOTE: Explicit cleanup removed - now handled by defer! guard above.
        // The guard ensures cleanup happens even on task abort or panic.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{Action, map_key};
    use crate::state::TuiState;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::text::Line;

    // =========================================================================
    // AC1: Events Reach State — TuiStreamHandler → IterationBuffer
    // =========================================================================

    #[test]
    fn dispatch_action_scroll_down_calls_scroll_down_on_current_buffer() {
        // Given TuiState with an iteration buffer containing content
        let mut state = TuiState::new();
        state.start_new_iteration();
        let buffer = state.current_iteration_mut().unwrap();
        for i in 0..20 {
            buffer.append_line(Line::from(format!("line {}", i)));
        }
        let initial_offset = state.current_iteration().unwrap().scroll_offset;
        assert_eq!(initial_offset, 0);

        // When dispatch_action with ScrollDown and viewport_height 10
        dispatch_action(Action::ScrollDown, &mut state, 10);

        // Then scroll_offset is incremented
        assert_eq!(
            state.current_iteration().unwrap().scroll_offset,
            1,
            "scroll_down should increment scroll_offset"
        );
    }

    // =========================================================================
    // AC2: Keyboard Triggers Actions — 'j' → scroll_down()
    // =========================================================================

    #[test]
    fn j_key_triggers_scroll_down_action() {
        // Given key press 'j'
        let key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);

        // When map_key is called
        let action = map_key(key);

        // Then Action::ScrollDown is returned
        assert_eq!(action, Action::ScrollDown);
    }

    #[test]
    fn dispatch_action_scroll_up_calls_scroll_up_on_current_buffer() {
        let mut state = TuiState::new();
        state.start_new_iteration();
        let buffer = state.current_iteration_mut().unwrap();
        for i in 0..20 {
            buffer.append_line(Line::from(format!("line {}", i)));
        }
        // Set initial scroll offset to 5
        state.current_iteration_mut().unwrap().scroll_offset = 5;

        dispatch_action(Action::ScrollUp, &mut state, 10);

        assert_eq!(
            state.current_iteration().unwrap().scroll_offset,
            4,
            "scroll_up should decrement scroll_offset"
        );
    }

    #[test]
    fn dispatch_action_scroll_top_jumps_to_top() {
        let mut state = TuiState::new();
        state.start_new_iteration();
        let buffer = state.current_iteration_mut().unwrap();
        for _ in 0..20 {
            buffer.append_line(Line::from("line"));
        }
        state.current_iteration_mut().unwrap().scroll_offset = 10;

        dispatch_action(Action::ScrollTop, &mut state, 10);

        assert_eq!(state.current_iteration().unwrap().scroll_offset, 0);
    }

    #[test]
    fn dispatch_action_scroll_bottom_jumps_to_bottom() {
        let mut state = TuiState::new();
        state.start_new_iteration();
        let buffer = state.current_iteration_mut().unwrap();
        for _ in 0..20 {
            buffer.append_line(Line::from("line"));
        }

        dispatch_action(Action::ScrollBottom, &mut state, 10);

        // max_scroll = 20 - 10 = 10
        assert_eq!(state.current_iteration().unwrap().scroll_offset, 10);
    }

    #[test]
    fn dispatch_action_next_iteration_navigates_forward() {
        let mut state = TuiState::new();
        state.start_new_iteration();
        state.start_new_iteration();
        state.start_new_iteration();
        state.current_view = 0;
        state.following_latest = false;

        dispatch_action(Action::NextIteration, &mut state, 10);

        assert_eq!(state.current_view, 1);
    }

    #[test]
    fn dispatch_action_prev_iteration_navigates_backward() {
        let mut state = TuiState::new();
        state.start_new_iteration();
        state.start_new_iteration();
        state.start_new_iteration();
        state.current_view = 2;

        dispatch_action(Action::PrevIteration, &mut state, 10);

        assert_eq!(state.current_view, 1);
    }

    #[test]
    fn dispatch_action_show_help_sets_show_help() {
        let mut state = TuiState::new();
        assert!(!state.show_help);

        dispatch_action(Action::ShowHelp, &mut state, 10);

        assert!(state.show_help);
    }

    #[test]
    fn dispatch_action_dismiss_help_clears_show_help() {
        let mut state = TuiState::new();
        state.show_help = true;

        dispatch_action(Action::DismissHelp, &mut state, 10);

        assert!(!state.show_help);
    }

    #[test]
    fn dispatch_action_search_next_calls_next_match() {
        let mut state = TuiState::new();
        state.start_new_iteration();
        let buffer = state.current_iteration_mut().unwrap();
        buffer.append_line(Line::from("find me"));
        buffer.append_line(Line::from("find me again"));
        state.search("find");
        assert_eq!(state.search_state.current_match, 0);

        dispatch_action(Action::SearchNext, &mut state, 10);

        assert_eq!(state.search_state.current_match, 1);
    }

    #[test]
    fn dispatch_action_search_prev_calls_prev_match() {
        let mut state = TuiState::new();
        state.start_new_iteration();
        let buffer = state.current_iteration_mut().unwrap();
        buffer.append_line(Line::from("find me"));
        buffer.append_line(Line::from("find me again"));
        state.search("find");
        state.search_state.current_match = 1;

        dispatch_action(Action::SearchPrev, &mut state, 10);

        assert_eq!(state.search_state.current_match, 0);
    }

    // =========================================================================
    // AC5: Quit Returns True to Exit Loop
    // =========================================================================

    #[test]
    fn dispatch_action_quit_returns_true() {
        let mut state = TuiState::new();
        let should_quit = dispatch_action(Action::Quit, &mut state, 10);
        assert!(should_quit, "Quit action should return true to signal exit");
    }

    #[test]
    fn dispatch_action_non_quit_returns_false() {
        let mut state = TuiState::new();
        state.start_new_iteration();
        let buffer = state.current_iteration_mut().unwrap();
        buffer.append_line(Line::from("line"));

        let should_quit = dispatch_action(Action::ScrollDown, &mut state, 10);
        assert!(!should_quit, "Non-quit actions should return false");
    }

    // =========================================================================
    // AC6: No PTY Code — Structural Test
    // =========================================================================

    #[test]
    fn no_pty_handle_in_app() {
        let source = include_str!("app.rs");
        let test_module_start = source.find("#[cfg(test)]").unwrap_or(source.len());
        let production_code = &source[..test_module_start];

        // Check for PTY-related imports/code
        assert!(
            !production_code.contains("PtyHandle"),
            "app.rs should not contain PtyHandle after refactor"
        );
        assert!(
            !production_code.contains("tui_term"),
            "app.rs should not contain tui_term references after refactor"
        );
        assert!(
            !production_code.contains("TerminalWidget"),
            "app.rs should not contain TerminalWidget after refactor"
        );
    }

    /// Regression test: TUI must NOT have tokio::signal::ctrl_c() handler.
    ///
    /// Raw mode prevents SIGINT, so tokio's signal handler never fires.
    /// TUI must detect Ctrl+C directly via crossterm events.
    #[test]
    fn no_tokio_signal_handler_in_app() {
        let source = include_str!("app.rs");
        let pattern = ["tokio", "::", "signal", "::", "ctrl_c", "()"].concat();
        let test_module_start = source.find("#[cfg(test)]").unwrap_or(source.len());
        let production_code = &source[..test_module_start];
        let occurrences: Vec<_> = production_code.match_indices(&pattern).collect();
        assert!(
            occurrences.is_empty(),
            "Found {} occurrence(s) of tokio::signal::ctrl_c() in production code. \
             This doesn't work in raw mode - use crossterm events instead.",
            occurrences.len()
        );
    }

    /// Verify Ctrl+C handling exists in production code.
    ///
    /// Since raw mode prevents SIGINT, we must handle Ctrl+C via crossterm events.
    /// TUI is observation-only, so Ctrl+C breaks out of the event loop.
    #[test]
    fn ctrl_c_handling_exists_in_app() {
        let source = include_str!("app.rs");
        let test_module_start = source.find("#[cfg(test)]").unwrap_or(source.len());
        let production_code = &source[..test_module_start];

        assert!(
            production_code.contains("KeyCode::Char('c')")
                && production_code.contains("KeyModifiers::CONTROL"),
            "Production code must detect Ctrl+C via crossterm events"
        );
    }
}
