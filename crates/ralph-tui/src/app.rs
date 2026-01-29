//! Main application loop for the TUI.
//!
//! This module provides a read-only observation dashboard that displays
//! formatted output from the Ralph orchestrator, with iteration navigation,
//! scroll, and search functionality.

use crate::chat::{ChatSubmit, parse_chat_submit};
use crate::external_event_writer::ExternalEventWriter;
use crate::input::{Action, map_key};
use crate::state::{GateStatus, ParallelFocus, TuiMode, TuiState, TuiUpdate};
use crate::widgets::{
    content::{ContentPane, SelectionBounds},
    footer, header, help, instances,
};
use anyhow::Result;
use crossterm::{
    cursor::Show,
    event::{
        DisableMouseCapture, EnableMouseCapture, Event, EventStream, KeyCode, KeyEventKind,
        KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
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
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// 并行模式下用于鼠标 hit-test 的布局快照。
///
/// 说明：
/// - 该结构只在 `App::run` 的局部变量里保存“最近一次渲染”的 Rect。
/// - 这样输入事件可以在下一帧到来前做 hit-test，而无需把布局塞进 state（保持 reducer 纯净）。
#[derive(Debug, Clone, Copy)]
struct ParallelLayoutSnapshot {
    instances_inner: ratatui::layout::Rect,
    output_inner: ratatui::layout::Rect,
    bottom_inner: ratatui::layout::Rect,
    chat_input_area: ratatui::layout::Rect,
    chat_targets_area: ratatui::layout::Rect,
    gate_list_area: ratatui::layout::Rect,
    gate_actions_area: ratatui::layout::Rect,
}

/// gate 快捷操作（actions chips）的枚举（用于 hit-test 后预填输入框）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GateActionChip {
    Approve,
    Deny,
    Resolve,
}

fn contains_point(area: ratatui::layout::Rect, x: u16, y: u16) -> bool {
    x >= area.x
        && x < area.x.saturating_add(area.width)
        && y >= area.y
        && y < area.y.saturating_add(area.height)
}

fn inner_block(area: ratatui::layout::Rect) -> ratatui::layout::Rect {
    // Block::inner 的等价计算（borders=ALL）。
    // 注意：width/height 可能小于 2，这里用 saturating_* 避免 underflow。
    ratatui::layout::Rect {
        x: area.x.saturating_add(1),
        y: area.y.saturating_add(1),
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    }
}

fn request_interrupt(tx: Option<&watch::Sender<bool>>) {
    if let Some(tx) = tx {
        let _ = tx.send(true);
    }
}

fn clamp_to_area(value: u16, start: u16, len: u16) -> u16 {
    if len == 0 {
        return start;
    }
    let end = start.saturating_add(len.saturating_sub(1));
    value.clamp(start, end)
}

fn clamp_usize(value: usize, max_exclusive: usize) -> usize {
    if max_exclusive == 0 {
        return 0;
    }
    value.min(max_exclusive.saturating_sub(1))
}

fn hit_test_chat_editor(
    editor: &crate::state::ChatEditorState,
    area: ratatui::layout::Rect,
    x: u16,
    y: u16,
) -> crate::state::TextPos {
    if area.width == 0 || area.height == 0 {
        return crate::state::TextPos::default();
    }

    // 约定：prompt 占 3 个 cell（" " + ">" + " "）
    let prefix_cells: u16 = 3;
    let content_width = area.width.saturating_sub(prefix_cells);

    let viewport_rows = area.height as usize;
    let total_lines = editor.lines.len().max(1);
    let cursor_row = editor.cursor.row.min(total_lines.saturating_sub(1));
    let start_row = cursor_row.saturating_sub(viewport_rows.saturating_sub(1));

    let rel_y = y.saturating_sub(area.y) as usize;
    let mut row = start_row.saturating_add(rel_y);
    row = row.min(total_lines.saturating_sub(1));

    let rel_x = x.saturating_sub(area.x);
    let content_x = rel_x.saturating_sub(prefix_cells);

    let line_text = editor.lines.get(row).map(|s| s.as_str()).unwrap_or("");
    let graphemes: Vec<&str> = UnicodeSegmentation::graphemes(line_text, true).collect();
    let widths: Vec<u16> = graphemes
        .iter()
        .map(|g| UnicodeWidthStr::width(*g) as u16)
        .collect();

    let line_len = graphemes.len();

    // 仅对“当前光标行”应用水平滚动（对齐渲染逻辑）
    let scroll_cell = if row == cursor_row && content_width > 0 {
        let cursor_col = editor.cursor.col.min(line_len);
        let cursor_cell = widths.iter().take(cursor_col).copied().sum::<u16>();
        if cursor_cell >= content_width {
            cursor_cell.saturating_sub(content_width.saturating_sub(1))
        } else {
            0
        }
    } else {
        0
    };

    // 找到可视起点（按 grapheme 边界）
    let mut start_idx = 0usize;
    let mut cell_acc = 0u16;
    for (idx, w) in widths.iter().enumerate() {
        if cell_acc.saturating_add(*w) > scroll_cell {
            start_idx = idx;
            break;
        }
        cell_acc = cell_acc.saturating_add(*w);
        start_idx = idx.saturating_add(1);
    }

    // 将 content_x（cell）映射到 grapheme col
    let mut col = start_idx.min(line_len);
    let mut cell = 0u16;
    for idx in start_idx..line_len {
        let w = widths.get(idx).copied().unwrap_or(0);
        if cell.saturating_add(w) > content_x {
            col = idx;
            break;
        }
        cell = cell.saturating_add(w);
        col = idx.saturating_add(1);
    }

    crate::state::TextPos { row, col }
}

fn hit_test_targets_chip(
    instance_order: &[ralph_proto::HatInstanceId],
    area: ratatui::layout::Rect,
    x: u16,
    y: u16,
) -> Option<usize> {
    if !contains_point(area, x, y) || area.width == 0 || area.height == 0 {
        return None;
    }

    // 说明：Targets 行渲染格式固定：
    // " Targets: @writer#1 @writer#2 ..."
    let rel_x = x.saturating_sub(area.x);
    let mut cursor_x: u16 = 0;

    // 前缀：" " + "Targets:" + " "
    cursor_x = cursor_x.saturating_add(1);
    cursor_x = cursor_x.saturating_add(UnicodeWidthStr::width("Targets:") as u16);
    cursor_x = cursor_x.saturating_add(1);

    for (idx, id) in instance_order.iter().enumerate() {
        let label = format!("@{id}");
        let w = UnicodeWidthStr::width(label.as_str()) as u16;
        let start = cursor_x;
        let end = cursor_x.saturating_add(w);

        if rel_x >= start && rel_x < end {
            return Some(idx);
        }

        cursor_x = end.saturating_add(1);
        if cursor_x >= area.width {
            break;
        }
    }

    None
}

fn hit_test_gate_action_chip(
    area: ratatui::layout::Rect,
    x: u16,
    y: u16,
) -> Option<GateActionChip> {
    if !contains_point(area, x, y) || area.width == 0 || area.height == 0 {
        return None;
    }

    // 说明：Actions 行渲染格式固定：
    // " Actions: !approve !deny !resolve"
    let rel_x = x.saturating_sub(area.x);
    let mut cursor_x: u16 = 0;

    // 前缀：" " + "Actions:" + " "
    cursor_x = cursor_x.saturating_add(1);
    cursor_x = cursor_x.saturating_add(UnicodeWidthStr::width("Actions:") as u16);
    cursor_x = cursor_x.saturating_add(1);

    let items = [
        (GateActionChip::Approve, "!approve"),
        (GateActionChip::Deny, "!deny"),
        (GateActionChip::Resolve, "!resolve"),
    ];

    for (action, label) in items {
        let w = UnicodeWidthStr::width(label) as u16;
        let start = cursor_x;
        let end = cursor_x.saturating_add(w);
        if rel_x >= start && rel_x < end {
            return Some(action);
        }

        cursor_x = end.saturating_add(1);
        if cursor_x >= area.width {
            break;
        }
    }

    None
}

fn resolve_human_message_target_instance(
    explicit: Option<String>,
    selected_instance_id: Option<&ralph_proto::HatInstanceId>,
) -> Option<String> {
    // 规则：
    // - 若用户显式写了 @instance，则以显式 target 为准
    // - 否则默认定向到当前 selected_instance（避免意外 broadcast）
    explicit.or_else(|| selected_instance_id.map(|id| id.to_string()))
}

fn handle_parallel_mouse_down(
    mouse: &MouseEvent,
    state: &mut TuiState,
    layout: ParallelLayoutSnapshot,
    chat_drag_anchor: &mut Option<crate::state::TextPos>,
) {
    let x = mouse.column;
    let y = mouse.row;

    // 只处理左键点击（其余按钮先忽略）。
    if !matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
        return;
    }

    // 1) 点击实例列表：切换选中实例，并把焦点切回 Instances（与“列表可操作”的心智一致）
    if contains_point(layout.instances_inner, x, y) {
        let rel_y = y.saturating_sub(layout.instances_inner.y) as usize;
        let max = state.parallel.instance_order.len();
        if max > 0 {
            state.parallel.selected_instance = clamp_usize(rel_y, max);
            state.parallel.focus = ParallelFocus::Instances;
            state.parallel.clear_output_selection();
        }
        *chat_drag_anchor = None;
        return;
    }

    // 2) 点击 Chat 面板：进入输入态（后续会在输入区内支持鼠标定位光标/框选）
    if contains_point(layout.chat_input_area, x, y) {
        state.parallel.focus = ParallelFocus::Chat;
        state.parallel.finish_output_selection();

        let pos = hit_test_chat_editor(&state.parallel.chat_editor, layout.chat_input_area, x, y);
        state.parallel.chat_editor.set_cursor(pos, false);
        *chat_drag_anchor = Some(pos);
        return;
    }

    // 3) 点击 Targets chips：切换“默认目标实例”（保持在 Chat 焦点，便于继续输入）。
    if let Some(idx) = hit_test_targets_chip(
        &state.parallel.instance_order,
        layout.chat_targets_area,
        x,
        y,
    ) {
        state.parallel.selected_instance = idx;
        state.parallel.focus = ParallelFocus::Chat;
        state.parallel.clear_output_selection();
        *chat_drag_anchor = None;
        return;
    }

    // 4) 点击 Gate actions chips：预填输入框（不自动发送）。
    if let Some(action) = hit_test_gate_action_chip(layout.gate_actions_area, x, y) {
        let Some(gate_id) = state.parallel.selected_gate.clone() else {
            *chat_drag_anchor = None;
            return;
        };

        let prefill = match action {
            GateActionChip::Approve => format!("!approve {gate_id}"),
            GateActionChip::Deny => format!("!deny {gate_id}"),
            // 注意：末尾保留一个空格，方便继续输入 resolve 文本。
            GateActionChip::Resolve => format!("!resolve {gate_id} "),
        };

        state.parallel.focus = ParallelFocus::Chat;
        state.parallel.finish_output_selection();
        state.parallel.chat_editor.clear();
        for ch in prefill.chars() {
            if ch == '\n' {
                state.parallel.chat_editor.insert_newline();
            } else {
                state.parallel.chat_editor.insert_char(ch);
            }
        }
        *chat_drag_anchor = None;
        return;
    }

    // 5) 点击 gate 列表行：选中 gate，并联动切换 selected_instance=requested_by。
    if contains_point(layout.gate_list_area, x, y) {
        let rel_y = y.saturating_sub(layout.gate_list_area.y) as usize;
        let max_lines = layout.gate_list_area.height as usize;

        let mut line_idx = 0usize;
        for gate_id in state.parallel.gate_order.iter().rev() {
            if line_idx >= max_lines {
                break;
            }
            let Some(g) = state.parallel.gates.get(gate_id) else {
                continue;
            };

            if line_idx == rel_y {
                state.parallel.selected_gate = Some(gate_id.clone());
                let requested_by = g.request.requested_by.clone();
                let _ = state.parallel.select_instance_by_id(&requested_by);

                state.parallel.focus = ParallelFocus::Chat;
                state.parallel.finish_output_selection();
                *chat_drag_anchor = None;
                return;
            }

            line_idx += 1;
        }

        *chat_drag_anchor = None;
        return;
    }

    if contains_point(layout.bottom_inner, x, y) {
        state.parallel.focus = ParallelFocus::Chat;
        state.parallel.finish_output_selection();
        *chat_drag_anchor = None;
        return;
    }

    // 3) 点击 Output 面板：切换焦点到 Output（后续会在输出区内支持拖拽框选）
    if contains_point(layout.output_inner, x, y) {
        let rel_x = x.saturating_sub(layout.output_inner.x);
        let rel_y = y.saturating_sub(layout.output_inner.y);
        state.parallel.focus = ParallelFocus::Output;
        state
            .parallel
            .start_output_selection(crate::state::ScreenPos { x: rel_x, y: rel_y });
        *chat_drag_anchor = None;
    }
}

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

        // 并行模式：底部控制面板（chat/gates）的固定高度。
        // 说明：先用固定高度满足“多行输入 + gate 列表可见”的最小需求，后续可再做自适应。
        const PARALLEL_BOTTOM_PANEL_HEIGHT: u16 = 12;
        const PARALLEL_CHAT_INPUT_HEIGHT: u16 = 3;

        // 并行模式的 state 更新通道（由 App 消费）
        let mut update_rx = self.update_rx.take();

        // 并行模式：保存“最近一次渲染”的布局快照，用于鼠标 hit-test
        let mut parallel_layout: Option<ParallelLayoutSnapshot> = None;
        // 并行模式：Chat 区域的鼠标拖拽选择锚点（Down→Drag→Up）。
        let mut chat_drag_anchor: Option<crate::state::TextPos> = None;

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
                                    request_interrupt(self.interrupt_tx.as_ref());
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
                                        MouseEventKind::Down(MouseButton::Left) => {
                                            let mut state = self.state.lock().unwrap();
                                            if matches!(state.mode, TuiMode::Parallel)
                                                && let Some(layout) = parallel_layout
                                            {
                                                handle_parallel_mouse_down(
                                                    &mouse,
                                                    &mut state,
                                                    layout,
                                                    &mut chat_drag_anchor,
                                                );
                                            }
                                        }
                                        // 说明：Drag/Up 的选择逻辑会在后续任务里补齐（输出框选 / chat 框选）。
                                        // 这里先做结构化分发，避免未来再大改输入循环。
                                        MouseEventKind::Drag(MouseButton::Left) => {
                                            let mut state = self.state.lock().unwrap();
                                            if matches!(state.mode, TuiMode::Parallel)
                                                && let Some(layout) = parallel_layout
                                            {
                                                let x = mouse.column;
                                                let y = mouse.row;

                                                // Output：拖拽更新选择区域（屏幕坐标）。
                                                if state.parallel.output_selecting {
                                                    let clamped_x = clamp_to_area(
                                                        x,
                                                        layout.output_inner.x,
                                                        layout.output_inner.width,
                                                    );
                                                    let clamped_y = clamp_to_area(
                                                        y,
                                                        layout.output_inner.y,
                                                        layout.output_inner.height,
                                                    );
                                                    let rel_x = clamped_x.saturating_sub(layout.output_inner.x);
                                                    let rel_y = clamped_y.saturating_sub(layout.output_inner.y);
                                                    state.parallel.focus = ParallelFocus::Output;
                                                    state.parallel.update_output_selection_cursor(crate::state::ScreenPos {
                                                        x: rel_x,
                                                        y: rel_y,
                                                    });
                                                    continue;
                                                }

                                                // Chat：拖拽更新线性选择（TextPos）。
                                                if let Some(anchor) = chat_drag_anchor {
                                                    let clamped_x = clamp_to_area(
                                                        x,
                                                        layout.chat_input_area.x,
                                                        layout.chat_input_area.width,
                                                    );
                                                    let clamped_y = clamp_to_area(
                                                        y,
                                                        layout.chat_input_area.y,
                                                        layout.chat_input_area.height,
                                                    );
                                                    let pos = hit_test_chat_editor(
                                                        &state.parallel.chat_editor,
                                                        layout.chat_input_area,
                                                        clamped_x,
                                                        clamped_y,
                                                    );
                                                    state.parallel.focus = ParallelFocus::Chat;
                                                    state.parallel.chat_editor.set_mouse_selection(anchor, pos);
                                                    continue;
                                                }

                                                // 兜底：拖拽落在哪个区域，就把焦点切过去（后续 chat 框选会复用）。
                                                if contains_point(layout.output_inner, x, y) {
                                                    state.parallel.focus = ParallelFocus::Output;
                                                } else if contains_point(layout.bottom_inner, x, y) {
                                                    state.parallel.focus = ParallelFocus::Chat;
                                                }
                                            }
                                        }
                                        MouseEventKind::Up(MouseButton::Left) => {
                                            let mut state = self.state.lock().unwrap();
                                            if matches!(state.mode, TuiMode::Parallel)
                                                && let Some(layout) = parallel_layout
                                            {
                                                let _ = layout;

                                                // Output：结束拖拽选择。
                                                if state.parallel.output_selecting {
                                                    state.parallel.finish_output_selection();
                                                }

                                                // Chat：结束拖拽选择（保留 selection 结果，仅清理锚点）。
                                                chat_drag_anchor = None;
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
                                            if action == Action::Quit {
                                                // 说明：
                                                // - 对用户来说，退出 TUI 等价于“我不再需要这个 run 继续执行”。
                                                // - 在 raw mode 下，TUI 是唯一能可靠捕获用户退出意图的地方。
                                                // 因此这里复用 interrupt_tx 通道，触发主循环走统一的 shutdown 清理路径。
                                                request_interrupt(self.interrupt_tx.as_ref());
                                                break;
                                            }
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
                                                    // 并行模式：退出 TUI 时必须退出所有 worker CLI 子进程。
                                                    // 复用 interrupt_tx，让并行 runner 走 killpg(SIGTERM→SIGKILL) 的统一清理路径。
                                                    request_interrupt(self.interrupt_tx.as_ref());
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
                                                    // Esc：清空输出选择（避免与 search-mode 的 Esc 冲突：
                                                    // search-mode 已在上方分支提前 continue 处理）。
                                                    if key.code == KeyCode::Esc {
                                                        state.parallel.clear_output_selection();
                                                        continue;
                                                    }

                                                    // Shift+方向键：扩展输出选择（最小可用键盘选择）。
                                                    if key.modifiers.contains(KeyModifiers::SHIFT) {
                                                        let (dx, dy) = match key.code {
                                                            KeyCode::Left => (-1, 0),
                                                            KeyCode::Right => (1, 0),
                                                            KeyCode::Up => (0, -1),
                                                            KeyCode::Down => (0, 1),
                                                            _ => (0, 0),
                                                        };

                                                        if (dx, dy) != (0, 0)
                                                            && let Some(layout) = parallel_layout
                                                        {
                                                            state.parallel.focus = ParallelFocus::Output;
                                                            state.parallel.extend_output_selection_by_delta(
                                                                dx,
                                                                dy,
                                                                layout.output_inner.width,
                                                                layout.output_inner.height,
                                                            );
                                                            continue;
                                                        }
                                                    }

                                                    let action = map_key(key);
                                                    if dispatch_action(action, &mut state, viewport_height) {
                                                        break;
                                                    }
                                                }
                                                ParallelFocus::Chat => {
                                                    match key.code {
                                                        KeyCode::Esc => {
                                                            // Esc：优先清空选择；若没有选择，则清空输入内容。
                                                            if state.parallel.chat_editor.has_selection() {
                                                                state.parallel.chat_editor.clear_selection();
                                                            } else {
                                                                state.parallel.chat_editor.clear();
                                                            }
                                                        }
                                                        KeyCode::Backspace => {
                                                            state.parallel.chat_editor.backspace();
                                                        }
                                                        KeyCode::Delete => {
                                                            state.parallel.chat_editor.delete();
                                                        }
                                                        KeyCode::Enter => {
                                                            // Shift+Enter：换行；Enter：提交。
                                                            if key.modifiers.contains(KeyModifiers::SHIFT) {
                                                                state.parallel.chat_editor.insert_newline();
                                                                continue;
                                                            }

                                                            let raw = state.parallel.chat_editor.text();
                                                            state.parallel.chat_editor.clear();
                                                            if raw.trim().is_empty() {
                                                                continue;
                                                            }

                                                            match parse_chat_submit(&raw) {
                                                                Ok(ChatSubmit::HumanMessage { target_instance, payload }) => {
                                                                    // 默认消息（不写 @...）需要定向到当前选中实例，
                                                                    // 避免 human.message 在并行模式下“意外广播”。
                                                                    let resolved_target = resolve_human_message_target_instance(
                                                                        target_instance,
                                                                        state.parallel.selected_instance_id(),
                                                                    );
                                                                    if resolved_target.is_none() {
                                                                        state.parallel.chat_status =
                                                                            Some("send failed: no instance selected".to_string());
                                                                        continue;
                                                                    }
                                                                    let writer = ExternalEventWriter::new();
                                                                    match writer.append("human.message", payload, resolved_target) {
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
                                                        KeyCode::Left => {
                                                            state
                                                                .parallel
                                                                .chat_editor
                                                                .move_left(key.modifiers.contains(KeyModifiers::SHIFT));
                                                        }
                                                        KeyCode::Right => {
                                                            state
                                                                .parallel
                                                                .chat_editor
                                                                .move_right(key.modifiers.contains(KeyModifiers::SHIFT));
                                                        }
                                                        KeyCode::Up => {
                                                            state
                                                                .parallel
                                                                .chat_editor
                                                                .move_up(key.modifiers.contains(KeyModifiers::SHIFT));
                                                        }
                                                        KeyCode::Down => {
                                                            state
                                                                .parallel
                                                                .chat_editor
                                                                .move_down(key.modifiers.contains(KeyModifiers::SHIFT));
                                                        }
                                                        KeyCode::Char(c) => {
                                                            state.parallel.chat_editor.insert_char(c);
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
                            // content = main + bottom_panel(PARALLEL_BOTTOM_PANEL_HEIGHT)
                            // output inner = main - borders(2)
                            let main_height = content_area
                                .height
                                .saturating_sub(PARALLEL_BOTTOM_PANEL_HEIGHT);
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
                                        Constraint::Length(PARALLEL_BOTTOM_PANEL_HEIGHT),  // bottom panel（后续可做自适应）
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
                                    if let Some(sel) = state.parallel.output_selection {
                                        content_widget = content_widget.with_selection(SelectionBounds::from_points(
                                            sel.anchor.x,
                                            sel.anchor.y,
                                            sel.cursor.x,
                                            sel.cursor.y,
                                        ));
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
                                    // 上：输入框 / Targets / 状态提示 / gate（详情 + 列表）
                                    let inner_chunks = Layout::default()
                                        .direction(Direction::Vertical)
                                        .constraints([
                                            Constraint::Length(PARALLEL_CHAT_INPUT_HEIGHT), // input（多行）
                                            Constraint::Length(1), // targets
                                            Constraint::Length(1), // status
                                            Constraint::Min(0),    // gates（详情 + 列表）
                                        ])
                                        .split(bottom_inner);

                                    let input_area = inner_chunks[0];
                                    let targets_area = inner_chunks[1];
                                    let status_area = inner_chunks[2];
                                    let gates_area = inner_chunks[3];

                                    // gate 详情/快捷 actions 与 gate 列表的区域划分：
                                    // - 只有存在 selected_gate 时，才占用 gates_area 的顶部行。
                                    // - 终端高度太小的情况下，会自动降级：优先保证 gate 列表仍可显示。
                                    let mut gate_info_area = ratatui::layout::Rect::default();
                                    let mut gate_prompt_area = ratatui::layout::Rect::default();
                                    let mut gate_actions_area = ratatui::layout::Rect::default();
                                    let mut gate_list_area = gates_area;
                                    if let Some(gate_id) = state.parallel.selected_gate.as_deref()
                                        && state.parallel.gates.contains_key(gate_id)
                                        && gates_area.height >= 4
                                    {
                                        let chunks = Layout::default()
                                            .direction(Direction::Vertical)
                                            .constraints([
                                                Constraint::Length(1),
                                                Constraint::Length(1),
                                                Constraint::Length(1),
                                                Constraint::Min(0),
                                            ])
                                            .split(gates_area);
                                        gate_info_area = chunks[0];
                                        gate_prompt_area = chunks[1];
                                        gate_actions_area = chunks[2];
                                        gate_list_area = chunks[3];
                                    }

                                    // 保存布局快照，用于鼠标点击/拖拽做 hit-test
                                    parallel_layout = Some(ParallelLayoutSnapshot {
                                        instances_inner: inner_block(instances_area),
                                        output_inner: inner,
                                        bottom_inner,
                                        chat_input_area: input_area,
                                        chat_targets_area: targets_area,
                                        gate_list_area,
                                        gate_actions_area,
                                    });

                                    // 1) chat 输入行
                                    let prompt_style = if bottom_focused {
                                        Style::default().fg(Color::Cyan)
                                    } else {
                                        Style::default().fg(Color::DarkGray)
                                    };

                                    let selection_style = Style::default().bg(Color::Blue);

                                    // 约定：prompt 占 3 个 cell（" " + ">" + " "）
                                    let prefix_cells: u16 = 3;
                                    let content_width = input_area.width.saturating_sub(prefix_cells);

                                    let editor = &state.parallel.chat_editor;
                                    let mut input_lines: Vec<Line> = Vec::new();
                                    let mut cursor_pos: Option<(u16, u16)> = None;

                                    if editor.is_empty() && !bottom_focused {
                                        // 未聚焦且为空：显示占位提示
                                        input_lines.push(Line::from(vec![
                                            Span::raw(" "),
                                            Span::styled(">", prompt_style),
                                            Span::raw(" "),
                                            Span::styled(
                                                "Type: msg (-> selected) | @instance msg | !approve/!deny/!resolve ...",
                                                Style::default().fg(Color::DarkGray),
                                            ),
                                        ]));
                                    } else {
                                        let total_lines = editor.lines.len().max(1);
                                        let cursor_row = editor.cursor.row.min(total_lines.saturating_sub(1));
                                        let viewport_rows = input_area.height as usize;
                                        let start_row = cursor_row.saturating_sub(viewport_rows.saturating_sub(1));

                                        for i in 0..viewport_rows {
                                            let row = start_row.saturating_add(i);
                                            if row >= total_lines {
                                                input_lines.push(Line::from(""));
                                                continue;
                                            }

                                            let prefix_symbol = if row == 0 { ">" } else { "|" };

                                            let line_text = editor.lines.get(row).map(|s| s.as_str()).unwrap_or("");
                                            let graphemes: Vec<&str> =
                                                UnicodeSegmentation::graphemes(line_text, true).collect();
                                            let widths: Vec<u16> = graphemes
                                                .iter()
                                                .map(|g| UnicodeWidthStr::width(*g) as u16)
                                                .collect();

                                            let line_len = graphemes.len();
                                            let selection_range = editor.selection_range_for_row(row);

                                            // 光标所在行：做水平滚动，保证光标可见
                                            let is_cursor_row = row == cursor_row;
                                            let cursor_col = if is_cursor_row {
                                                editor.cursor.col.min(line_len)
                                            } else {
                                                0
                                            };
                                            let cursor_cell = widths.iter().take(cursor_col).copied().sum::<u16>();

                                            let scroll_cell = if is_cursor_row && content_width > 0 {
                                                if cursor_cell >= content_width {
                                                    cursor_cell.saturating_sub(content_width.saturating_sub(1))
                                                } else {
                                                    0
                                                }
                                            } else {
                                                0
                                            };

                                            // 根据 scroll_cell 找到可视起点（按 grapheme 边界）
                                            let mut start_idx = 0usize;
                                            let mut start_cell = 0u16;
                                            for (idx, w) in widths.iter().enumerate() {
                                                if start_cell.saturating_add(*w) > scroll_cell {
                                                    start_idx = idx;
                                                    break;
                                                }
                                                start_cell = start_cell.saturating_add(*w);
                                                start_idx = idx.saturating_add(1);
                                            }

                                            // 找到可视终点
                                            let mut end_idx = start_idx;
                                            let mut used_cells = 0u16;
                                            for idx in start_idx..line_len {
                                                let w = widths.get(idx).copied().unwrap_or(0);
                                                if used_cells.saturating_add(w) > content_width {
                                                    break;
                                                }
                                                used_cells = used_cells.saturating_add(w);
                                                end_idx = idx.saturating_add(1);
                                            }

                                            let vis_start = start_idx.min(line_len);
                                            let vis_end = end_idx.min(line_len).max(vis_start);

                                            // 构造 content spans（带选择高亮）
                                            let mut content_spans: Vec<Span> = Vec::new();
                                            if let Some((sel_start, sel_end)) = selection_range {
                                                let inter_start = sel_start.max(vis_start);
                                                let inter_end = sel_end.min(vis_end);

                                                if inter_start < inter_end {
                                                    let before = graphemes[vis_start..inter_start].concat();
                                                    let selected = graphemes[inter_start..inter_end].concat();
                                                    let after = graphemes[inter_end..vis_end].concat();

                                                    if !before.is_empty() {
                                                        content_spans.push(Span::raw(before));
                                                    }
                                                    if !selected.is_empty() {
                                                        content_spans.push(Span::styled(selected, selection_style));
                                                    }
                                                    if !after.is_empty() {
                                                        content_spans.push(Span::raw(after));
                                                    }
                                                } else {
                                                    content_spans.push(Span::raw(graphemes[vis_start..vis_end].concat()));
                                                }
                                            } else {
                                                content_spans.push(Span::raw(graphemes[vis_start..vis_end].concat()));
                                            }

                                            let mut spans = vec![
                                                Span::raw(" "),
                                                Span::styled(prefix_symbol, prompt_style),
                                                Span::raw(" "),
                                            ];
                                            spans.extend(content_spans);
                                            input_lines.push(Line::from(spans));

                                            // 计算 cursor 的屏幕位置（聚焦时才显示）
                                            if bottom_focused && is_cursor_row {
                                                let cursor_x_cells = prefix_cells.saturating_add(
                                                    cursor_cell.saturating_sub(start_cell),
                                                );
                                                let cursor_x = input_area
                                                    .x
                                                    .saturating_add(cursor_x_cells.min(input_area.width.saturating_sub(1)));
                                                let cursor_y = input_area.y.saturating_add(i as u16);
                                                cursor_pos = Some((cursor_x, cursor_y));
                                            }
                                        }
                                    }

                                    // 填满剩余行，避免旧帧残影
                                    while input_lines.len() < input_area.height as usize {
                                        input_lines.push(Line::from(""));
                                    }

                                    f.render_widget(Paragraph::new(input_lines), input_area);
                                    if let Some((x, y)) = cursor_pos {
                                        f.set_cursor_position((x, y));
                                    }

                                    // 2) Targets chips（默认消息目标选择）
                                    let selected_id = state.parallel.selected_instance_id();
                                    let mut targets_spans: Vec<Span> = vec![
                                        Span::raw(" "),
                                        Span::styled("Targets:", Style::default().fg(Color::DarkGray)),
                                        Span::raw(" "),
                                    ];
                                    if state.parallel.instance_order.is_empty() {
                                        targets_spans.push(Span::styled(
                                            "(none)",
                                            Style::default().fg(Color::DarkGray),
                                        ));
                                    } else {
                                        for id in &state.parallel.instance_order {
                                            let label = format!("@{id}");
                                            let is_selected = selected_id == Some(id);
                                            let chip_style = if is_selected {
                                                Style::default()
                                                    .fg(Color::Black)
                                                    .bg(Color::Cyan)
                                                    .add_modifier(ratatui::style::Modifier::BOLD)
                                            } else {
                                                Style::default().fg(Color::Cyan)
                                            };
                                            targets_spans.push(Span::styled(label, chip_style));
                                            targets_spans.push(Span::raw(" "));
                                        }
                                    }
                                    let targets_line = Line::from(targets_spans);
                                    f.render_widget(Paragraph::new(targets_line), targets_area);

                                    // 3) 状态提示
                                    let status = state
                                        .parallel
                                        .chat_status
                                        .as_deref()
                                        .unwrap_or(if bottom_focused {
                                            "Enter=send  Shift+Enter=newline  Arrows=move  Esc=clear  Tab=switch"
                                        } else {
                                            "Tab to focus chat"
                                        });
                                    let status_line = Line::from(vec![
                                        Span::raw(" "),
                                        Span::styled(status.to_string(), Style::default().fg(Color::DarkGray)),
                                    ]);
                                    f.render_widget(Paragraph::new(status_line), status_area);

                                    // 4) 当前 gate 详情（点击 gate 列表行后显示）
                                    if let Some(gate_id) = state.parallel.selected_gate.as_deref()
                                        && let Some(g) = state.parallel.gates.get(gate_id)
                                    {
                                        let kind = match g.request.kind {
                                            ralph_proto::GateKind::Consult => "consult",
                                            ralph_proto::GateKind::Approval => "approval",
                                        };

                                        if gate_info_area.height > 0 {
                                            let info_line = Line::from(vec![
                                                Span::raw(" "),
                                                Span::styled("Gate:", Style::default().fg(Color::DarkGray)),
                                                Span::raw(" "),
                                                Span::styled(
                                                    gate_id.to_string(),
                                                    Style::default()
                                                        .fg(Color::Magenta)
                                                        .add_modifier(ratatui::style::Modifier::BOLD),
                                                ),
                                                Span::raw(" "),
                                                Span::styled(
                                                    format!("[{kind}]"),
                                                    Style::default().fg(Color::Magenta),
                                                ),
                                                Span::raw(" "),
                                                Span::styled("by=", Style::default().fg(Color::DarkGray)),
                                                Span::styled(
                                                    g.request.requested_by.to_string(),
                                                    Style::default().fg(Color::Cyan),
                                                ),
                                            ]);
                                            f.render_widget(Paragraph::new(info_line), gate_info_area);
                                        }

                                        if gate_prompt_area.height > 0 {
                                            // 尽量按可视宽度截断，避免 prompt 把一行撑爆。
                                            let prefix_cells =
                                                1 + UnicodeWidthStr::width("Prompt:") as u16 + 1;
                                            let max_chars = gate_prompt_area
                                                .width
                                                .saturating_sub(prefix_cells) as usize;
                                            let prompt = truncate_with_ellipsis(&g.request.prompt, max_chars);

                                            let prompt_line = Line::from(vec![
                                                Span::raw(" "),
                                                Span::styled("Prompt:", Style::default().fg(Color::DarkGray)),
                                                Span::raw(" "),
                                                Span::raw(prompt),
                                            ]);
                                            f.render_widget(Paragraph::new(prompt_line), gate_prompt_area);
                                        }

                                        if gate_actions_area.height > 0 {
                                            let action_style = Style::default()
                                                .fg(Color::Cyan)
                                                .add_modifier(ratatui::style::Modifier::BOLD);
                                            let actions_line = Line::from(vec![
                                                Span::raw(" "),
                                                Span::styled("Actions:", Style::default().fg(Color::DarkGray)),
                                                Span::raw(" "),
                                                Span::styled("!approve", action_style),
                                                Span::raw(" "),
                                                Span::styled("!deny", action_style),
                                                Span::raw(" "),
                                                Span::styled("!resolve", action_style),
                                            ]);
                                            f.render_widget(Paragraph::new(actions_line), gate_actions_area);
                                        }
                                    }

                                    // 5) gate 列表（最新在上）
                                    let mut gate_lines: Vec<Line> = Vec::new();
                                    let max_lines = gate_list_area.height as usize;

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
                                        let is_selected_gate =
                                            state.parallel.selected_gate.as_deref() == Some(gate_id.as_str());
                                        let marker = if is_selected_gate { ">" } else { " " };
                                        let marker_style = if is_selected_gate {
                                            Style::default()
                                                .fg(Color::Cyan)
                                                .add_modifier(ratatui::style::Modifier::BOLD)
                                        } else {
                                            Style::default()
                                        };

                                        gate_lines.push(Line::from(vec![
                                            Span::styled(marker, marker_style),
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

                                    f.render_widget(Paragraph::new(gate_lines), gate_list_area);
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
    use crossterm::event::{
        KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    };
    use ralph_proto::{
        Event, GateKind, GateRequest, HatInstanceId, HatInstanceState, TOPIC_GATE_REQUEST,
    };
    use ratatui::text::Line;
    use tokio::sync::watch;

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

    #[test]
    fn request_interrupt_sets_watch_signal() {
        let (tx, rx) = watch::channel(false);
        request_interrupt(Some(&tx));
        assert!(
            *rx.borrow(),
            "request_interrupt should set the watch channel to true"
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

    // =========================================================================
    // Parallel TUI: Targets/Gates 快捷交互（chips + 默认目标）
    // =========================================================================

    #[test]
    fn resolve_human_message_target_instance_prefers_explicit_target() {
        let selected = HatInstanceId::from("writer#1");
        let got =
            resolve_human_message_target_instance(Some("writer#2".to_string()), Some(&selected));
        assert_eq!(got, Some("writer#2".to_string()));
    }

    #[test]
    fn resolve_human_message_target_instance_defaults_to_selected_instance() {
        let selected = HatInstanceId::from("writer#2");
        let got = resolve_human_message_target_instance(None, Some(&selected));
        assert_eq!(got, Some("writer#2".to_string()));
    }

    #[test]
    fn mouse_click_targets_chip_switches_selected_instance() {
        let mut state = TuiState::new_parallel();
        state
            .parallel
            .register_instance(HatInstanceId::from("writer#1"), HatInstanceState::Idle);
        state
            .parallel
            .register_instance(HatInstanceId::from("writer#2"), HatInstanceState::Idle);

        // 初始选中 writer#1
        assert_eq!(
            state.parallel.selected_instance_id().unwrap().as_str(),
            "writer#1"
        );

        let layout = ParallelLayoutSnapshot {
            instances_inner: ratatui::layout::Rect::new(0, 0, 10, 10),
            output_inner: ratatui::layout::Rect::new(0, 0, 0, 0),
            bottom_inner: ratatui::layout::Rect::new(20, 0, 60, 10),
            chat_input_area: ratatui::layout::Rect::new(20, 0, 60, 3),
            chat_targets_area: ratatui::layout::Rect::new(20, 3, 60, 1),
            gate_list_area: ratatui::layout::Rect::new(20, 5, 60, 3),
            gate_actions_area: ratatui::layout::Rect::new(20, 4, 60, 1),
        };

        // 点击第二个 chip（@writer#2）。
        // 说明：Targets 行格式固定：" Targets: @writer#1 @writer#2 ..."
        let click_x = layout.chat_targets_area.x + 20;
        let click_y = layout.chat_targets_area.y;
        let mouse = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: click_x,
            row: click_y,
            modifiers: KeyModifiers::empty(),
        };
        let mut anchor = None;
        handle_parallel_mouse_down(&mouse, &mut state, layout, &mut anchor);

        assert_eq!(
            state.parallel.selected_instance_id().unwrap().as_str(),
            "writer#2"
        );
        assert_eq!(state.parallel.focus, ParallelFocus::Chat);
    }

    #[test]
    fn mouse_click_gate_row_selects_gate_and_switches_selected_instance() {
        let mut state = TuiState::new_parallel();
        state
            .parallel
            .register_instance(HatInstanceId::from("writer#1"), HatInstanceState::Idle);
        state
            .parallel
            .register_instance(HatInstanceId::from("writer#2"), HatInstanceState::Idle);

        // 两个 gate：g1(by writer#1) → g2(by writer#2)（列表渲染“最新在上”）。
        let req1 = GateRequest {
            gate_id: "g1".to_string(),
            thread_id: None,
            requested_by: HatInstanceId::from("writer#1"),
            kind: GateKind::Consult,
            timeout_seconds: None,
            prompt: "p1".to_string(),
            proposed_default: None,
        };
        let req2 = GateRequest {
            gate_id: "g2".to_string(),
            thread_id: None,
            requested_by: HatInstanceId::from("writer#2"),
            kind: GateKind::Approval,
            timeout_seconds: None,
            prompt: "p2".to_string(),
            proposed_default: None,
        };
        let ev1 = Event::new(
            TOPIC_GATE_REQUEST,
            serde_json::to_string(&req1).unwrap().as_str(),
        );
        let ev2 = Event::new(
            TOPIC_GATE_REQUEST,
            serde_json::to_string(&req2).unwrap().as_str(),
        );
        state.parallel.apply_event(&ev1);
        state.parallel.apply_event(&ev2);

        let layout = ParallelLayoutSnapshot {
            instances_inner: ratatui::layout::Rect::new(0, 0, 10, 10),
            output_inner: ratatui::layout::Rect::new(0, 0, 0, 0),
            bottom_inner: ratatui::layout::Rect::new(20, 0, 60, 10),
            chat_input_area: ratatui::layout::Rect::new(20, 0, 60, 3),
            chat_targets_area: ratatui::layout::Rect::new(20, 3, 60, 1),
            gate_actions_area: ratatui::layout::Rect::new(20, 4, 60, 1),
            gate_list_area: ratatui::layout::Rect::new(20, 5, 60, 3),
        };

        // 点击 gate 列表第 0 行（最新的 g2）。
        let mouse = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: layout.gate_list_area.x,
            row: layout.gate_list_area.y,
            modifiers: KeyModifiers::empty(),
        };
        let mut anchor = None;
        handle_parallel_mouse_down(&mouse, &mut state, layout, &mut anchor);

        assert_eq!(state.parallel.selected_gate.as_deref(), Some("g2"));
        assert_eq!(
            state.parallel.selected_instance_id().unwrap().as_str(),
            "writer#2"
        );
    }

    #[test]
    fn mouse_click_gate_action_chip_prefills_input_without_sending() {
        let mut state = TuiState::new_parallel();
        state.parallel.selected_gate = Some("g2".to_string());

        // 先塞一点旧内容，验证点击会覆盖。
        state.parallel.chat_editor.insert_char('x');
        assert_eq!(state.parallel.chat_editor.text(), "x");

        let layout = ParallelLayoutSnapshot {
            instances_inner: ratatui::layout::Rect::new(0, 0, 10, 10),
            output_inner: ratatui::layout::Rect::new(0, 0, 0, 0),
            bottom_inner: ratatui::layout::Rect::new(20, 0, 60, 10),
            chat_input_area: ratatui::layout::Rect::new(20, 0, 60, 3),
            chat_targets_area: ratatui::layout::Rect::new(20, 3, 60, 1),
            gate_actions_area: ratatui::layout::Rect::new(20, 4, 60, 1),
            gate_list_area: ratatui::layout::Rect::new(20, 5, 60, 3),
        };

        // 点击 `!resolve`（actions 第 3 个）。
        let click_x = layout.gate_actions_area.x + 25;
        let click_y = layout.gate_actions_area.y;
        let mouse = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: click_x,
            row: click_y,
            modifiers: KeyModifiers::empty(),
        };
        let mut anchor = None;
        handle_parallel_mouse_down(&mouse, &mut state, layout, &mut anchor);

        assert_eq!(state.parallel.chat_editor.text(), "!resolve g2 ");
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
