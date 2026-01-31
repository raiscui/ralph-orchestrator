//! Test harness for TUI integration tests.
//!
//! Provides `TuiTestHarness` that drives TUI state with event sequences,
//! enabling deterministic snapshot testing of state transitions.

use ralph_proto::Event;
use ralph_tui::state::{TuiMode, TuiState, TuiUpdate};
use ralph_tui::theme::{TuiTheme, panel_block};
use ralph_tui::widgets::{content::ContentPane, footer, header, instances};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::text::{Line, Span};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Test harness that drives TUI with event sequences.
///
/// Unlike ReplayBackend (which replays terminal output bytes), this harness
/// works at the event level, simulating the observer pattern used in production.
pub struct TuiTestHarness {
    state: Arc<Mutex<TuiState>>,
    events: Vec<Event>,
    event_cursor: usize,
    terminal_width: u16,
    terminal_height: u16,
}

impl TuiTestHarness {
    /// Creates a new harness with an empty event queue.
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(TuiState::new())),
            events: Vec::new(),
            event_cursor: 0,
            terminal_width: 100,
            terminal_height: 30,
        }
    }

    /// Creates a new harness for parallel mode (Supervisor TUI).
    pub fn new_parallel() -> Self {
        Self {
            state: Arc::new(Mutex::new(TuiState::new_parallel())),
            events: Vec::new(),
            event_cursor: 0,
            terminal_width: 100,
            terminal_height: 30,
        }
    }

    /// Creates a harness from a sequence of events.
    pub fn with_events(events: Vec<Event>) -> Self {
        Self {
            state: Arc::new(Mutex::new(TuiState::new())),
            events,
            event_cursor: 0,
            terminal_width: 100,
            terminal_height: 30,
        }
    }

    /// Creates a harness from a JSONL fixture file.
    /// The fixture name should be relative to the tests/fixtures directory.
    pub fn from_fixture(name: &str) -> Self {
        let path = fixture_path(name);
        let events = load_events_from_jsonl(path);
        Self::with_events(events)
    }

    /// Sets the terminal size for rendering tests.
    #[must_use]
    pub fn with_terminal_size(mut self, width: u16, height: u16) -> Self {
        self.terminal_width = width;
        self.terminal_height = height;
        self
    }

    /// Adds content lines to a specific iteration (for testing rendering).
    pub fn add_iteration_content(&mut self, iteration: usize, lines: Vec<Line<'static>>) {
        let mut state = self.state.lock().unwrap();

        // Ensure we have enough iterations
        while state.iterations.len() <= iteration {
            state.start_new_iteration();
        }

        // Add lines to the specified iteration
        if let Some(buffer) = state.iterations.get_mut(iteration) {
            for line in lines {
                buffer.append_line(line);
            }
        }
    }

    /// Process all events up to the specified iteration number.
    /// Iteration numbers are 1-indexed (matching display).
    pub fn advance_to_iteration(&mut self, target_iter: u32) {
        let mut state = self.state.lock().unwrap();

        while self.event_cursor < self.events.len() {
            let event = &self.events[self.event_cursor];
            state.update(event);
            self.event_cursor += 1;

            // Check if we've reached the target iteration
            // Iteration is 0-indexed internally, but increments on build.done
            if state.iteration >= target_iter {
                break;
            }
        }
    }

    /// Process events until a specific topic is encountered.
    pub fn process_until_event(&mut self, target_topic: &str) {
        let mut state = self.state.lock().unwrap();

        while self.event_cursor < self.events.len() {
            let event = &self.events[self.event_cursor];
            let matches = event.topic.as_str() == target_topic;
            state.update(event);
            self.event_cursor += 1;

            if matches {
                break;
            }
        }
    }

    /// Process all remaining events.
    pub fn process_all(&mut self) {
        let mut state = self.state.lock().unwrap();

        while self.event_cursor < self.events.len() {
            let event = &self.events[self.event_cursor];
            state.update(event);
            self.event_cursor += 1;
        }
    }

    /// Navigate to next iteration (simulates pressing →/l).
    pub fn navigate_next(&mut self) {
        self.state.lock().unwrap().navigate_next();
    }

    /// Navigate to previous iteration (simulates pressing ←/h).
    pub fn navigate_prev(&mut self) {
        self.state.lock().unwrap().navigate_prev();
    }

    /// Scroll down in current iteration (simulates pressing j/↓).
    pub fn scroll_down(&mut self, viewport_height: usize) {
        if let Some(buffer) = self.state.lock().unwrap().current_iteration_mut() {
            buffer.scroll_down(viewport_height);
        }
    }

    /// Scroll up in current iteration (simulates pressing k/↑).
    pub fn scroll_up(&mut self) {
        if let Some(buffer) = self.state.lock().unwrap().current_iteration_mut() {
            buffer.scroll_up();
        }
    }

    /// Execute search in current iteration.
    pub fn search(&mut self, query: &str) {
        self.state.lock().unwrap().search(query);
    }

    /// Navigate to next search match.
    pub fn search_next(&mut self) {
        self.state.lock().unwrap().next_match();
    }

    /// Navigate to previous search match.
    pub fn search_prev(&mut self) {
        self.state.lock().unwrap().prev_match();
    }

    /// Clear search state.
    pub fn clear_search(&mut self) {
        self.state.lock().unwrap().clear_search();
    }

    /// Get current scroll offset.
    pub fn current_scroll_offset(&self) -> usize {
        self.state
            .lock()
            .unwrap()
            .current_iteration()
            .map(|b| b.scroll_offset)
            .unwrap_or(0)
    }

    /// Capture current TUI state as a snapshot-friendly struct.
    /// Excludes timing-dependent fields for deterministic snapshots.
    pub fn capture_state(&self) -> TuiSnapshot {
        let state = self.state.lock().unwrap();

        TuiSnapshot {
            iteration: state.iteration,
            current_view: state.current_view,
            total_iterations: state.total_iterations(),
            following_latest: state.following_latest,
            has_alert: state.new_iteration_alert.is_some(),
            alert_iteration: state.new_iteration_alert,
            search_query: state.search_state.query.clone(),
            search_matches: state.search_state.matches.len(),
            search_current_match: state.search_state.current_match,
            pending_hat: state.get_pending_hat_display(),
            show_help: state.show_help,
        }
    }

    /// Get direct access to state for advanced assertions.
    pub fn state(&self) -> &Arc<Mutex<TuiState>> {
        &self.state
    }

    /// Applies a parallel-mode update (observer → channel → reducer).
    ///
    /// 注意：该方法只在 `TuiState::new_parallel()` 创建的 harness 上有意义。
    pub fn apply_parallel_update(&mut self, update: TuiUpdate) {
        self.state.lock().unwrap().apply_update(update);
    }

    /// Get configured terminal width.
    #[allow(dead_code)]
    pub fn width(&self) -> u16 {
        self.terminal_width
    }

    /// Get configured terminal height.
    #[allow(dead_code)]
    pub fn height(&self) -> u16 {
        self.terminal_height
    }

    /// Render header widget and return as string.
    /// Uses configured terminal width for responsive rendering.
    /// Height is 2: content + bottom border.
    pub fn render_header(&self) -> String {
        let state = self.state.lock().unwrap();
        let theme = TuiTheme::default();
        let backend = TestBackend::new(self.terminal_width, 2);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let widget = header::render(&state, theme, self.terminal_width);
                f.render_widget(widget, f.area());
            })
            .unwrap();

        buffer_to_string(terminal.backend().buffer())
    }

    /// Render footer widget and return as string.
    /// Height is 2: top border + content.
    pub fn render_footer(&self) -> String {
        let state = self.state.lock().unwrap();
        let theme = TuiTheme::default();
        let backend = TestBackend::new(self.terminal_width, 2);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let widget = footer::render(&state, theme);
                f.render_widget(widget, f.area());
            })
            .unwrap();

        buffer_to_string(terminal.backend().buffer())
    }

    /// Render full TUI layout and return as string.
    /// Layout: header (2 lines: content + border) | content (flexible) | footer (2 lines: border + content)
    pub fn render_full(&self) -> String {
        let state = self.state.lock().unwrap();
        let theme = TuiTheme::default();
        let backend = TestBackend::new(self.terminal_width, self.terminal_height);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                // 背景先铺一层，避免快照里残留旧帧内容。
                f.render_widget(
                    ratatui::widgets::Block::new().style(theme.app_bg()),
                    f.area(),
                );

                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(2), // Header: content + bottom border
                        Constraint::Min(0),    // Content
                        Constraint::Length(2), // Footer: top border + content
                    ])
                    .split(f.area());

                // Render header
                f.render_widget(header::render(&state, theme, chunks[0].width), chunks[0]);

                // Render content (serial vs parallel)
                match state.mode {
                    TuiMode::Serial => {
                        if let Some(buffer) = state.current_iteration() {
                            let content = ContentPane::new(buffer, theme);
                            f.render_widget(content, chunks[1]);
                        }
                    }
                    TuiMode::Parallel => {
                        let content_area = chunks[1];

                        // 布局：上（实例列表 + 输出） / 下（chat + gate）
                        let vertical = Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([
                                Constraint::Min(0),    // main
                                Constraint::Length(9), // bottom panel（对齐真实渲染：3 行输入 + 1 行状态 + gate）
                            ])
                            .split(content_area);

                        let main_area = vertical[0];
                        let bottom_area = vertical[1];

                        let horizontal = Layout::default()
                            .direction(Direction::Horizontal)
                            .constraints([
                                Constraint::Length(30), // instances
                                Constraint::Length(1),  // gap（避免边框贴合）
                                Constraint::Min(0),     // output
                            ])
                            .split(main_area);

                        // 左：实例列表
                        f.render_widget(instances::render(&state.parallel, theme), horizontal[0]);

                        // 右：输出（选中实例的当前 job）
                        let output_area = horizontal[2];
                        let block = panel_block("Output", false, &theme);
                        let inner = block.inner(output_area);
                        f.render_widget(block, output_area);

                        if let Some(instance) = state.parallel.selected_instance()
                            && let Some(buffer) = instance.current_job_buffer()
                        {
                            let mut content_widget = ContentPane::new(buffer, theme);
                            if let Some(query) = &state.search_state.query {
                                content_widget = content_widget.with_search(query);
                            }
                            f.render_widget(content_widget, inner);
                        }

                        // 下：chat/gates（尽量贴近真实渲染，便于回归）
                        let bottom_block = panel_block("Chat / Gates", false, &theme);
                        let bottom_inner = bottom_block.inner(bottom_area);
                        f.render_widget(bottom_block, bottom_area);

                        if bottom_inner.width > 0 && bottom_inner.height > 0 {
                            let inner_chunks = Layout::default()
                                .direction(Direction::Vertical)
                                .constraints([
                                    Constraint::Length(3), // input（多行）
                                    Constraint::Length(1), // status
                                    Constraint::Min(0),    // gates
                                ])
                                .split(bottom_inner);

                            let input_area = inner_chunks[0];
                            let status_area = inner_chunks[1];
                            let gates_area = inner_chunks[2];

                            // 输入区（快照用：不渲染颜色，只验证文字/布局）
                            //
                            // 说明：
                            // - 真实实现里输入框是 3 行（支持 Shift+Enter 换行）
                            // - 提示符约定：首行用 ">"，后续行用 "|"，便于区分多行
                            let bottom_focused =
                                state.parallel.focus == ralph_tui::state::ParallelFocus::Chat;
                            let editor = &state.parallel.chat_editor;
                            let mut input_lines: Vec<Line> = Vec::new();

                            if editor.is_empty() && !bottom_focused {
                                input_lines.push(Line::from(vec![
                                    Span::raw(" "),
                                    Span::raw(">"),
                                    Span::raw(" "),
                                    Span::raw("Type: @instance msg | !approve/!deny/!resolve ..."),
                                ]));
                            } else {
                                let total_lines = editor.lines.len().max(1);
                                let viewport_rows = input_area.height as usize;
                                for i in 0..viewport_rows {
                                    if i >= total_lines {
                                        input_lines.push(Line::from(""));
                                        continue;
                                    }

                                    let prefix_symbol = if i == 0 { ">" } else { "|" };
                                    let line_text =
                                        editor.lines.get(i).map(|s| s.as_str()).unwrap_or("");
                                    input_lines.push(Line::from(vec![
                                        Span::raw(" "),
                                        Span::raw(prefix_symbol),
                                        Span::raw(" "),
                                        Span::raw(line_text),
                                    ]));
                                }
                            }

                            // 填满剩余行，避免旧帧残影
                            while input_lines.len() < input_area.height as usize {
                                input_lines.push(Line::from(""));
                            }

                            f.render_widget(
                                ratatui::widgets::Paragraph::new(input_lines),
                                input_area,
                            );

                            // 状态行
                            let status = state
                                .parallel
                                .chat_status
                                .clone()
                                .unwrap_or_else(|| "ready".to_string());
                            let status_line = Line::from(vec![Span::raw(" "), Span::raw(status)]);
                            f.render_widget(
                                ratatui::widgets::Paragraph::new(status_line),
                                status_area,
                            );

                            // gate 列表（最多渲染可见高度）
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

                                let status = match g.status_at(std::time::Instant::now()) {
                                    ralph_tui::state::GateStatus::Resolved => {
                                        "resolved".to_string()
                                    }
                                    ralph_tui::state::GateStatus::Timeout => "timeout".to_string(),
                                    ralph_tui::state::GateStatus::Waiting { remaining_seconds } => {
                                        format!("T-{remaining_seconds}s")
                                    }
                                    ralph_tui::state::GateStatus::Open => "open".to_string(),
                                };

                                gate_lines.push(Line::from(vec![
                                    Span::raw(" "),
                                    Span::raw(format!("[{kind}] ")),
                                    Span::raw(gate_id),
                                    Span::raw(" "),
                                    Span::raw(status),
                                    Span::raw(" "),
                                    Span::raw(g.request.requested_by.to_string()),
                                ]));
                            }

                            if gate_lines.is_empty() {
                                gate_lines
                                    .push(Line::from(vec![Span::raw(" "), Span::raw("No gates")]));
                            }

                            f.render_widget(
                                ratatui::widgets::Paragraph::new(gate_lines),
                                gates_area,
                            );
                        }
                    }
                }

                // Render footer
                f.render_widget(footer::render(&state, theme), chunks[2]);
            })
            .unwrap();

        buffer_to_multiline_string(terminal.backend().buffer(), self.terminal_height)
    }
}

impl Default for TuiTestHarness {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert a ratatui Buffer to a single-line string.
fn buffer_to_string(buffer: &ratatui::buffer::Buffer) -> String {
    let raw = buffer
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    normalize_snapshot_text(raw.trim_end())
}

/// Convert a ratatui Buffer to a multi-line string.
fn buffer_to_multiline_string(buffer: &ratatui::buffer::Buffer, height: u16) -> String {
    let width = buffer.area().width as usize;
    let content: String = buffer.content().iter().map(|cell| cell.symbol()).collect();

    let raw = content
        .chars()
        .collect::<Vec<_>>()
        .chunks(width)
        .take(height as usize)
        .map(|chunk| chunk.iter().collect::<String>().trim_end().to_string())
        .collect::<Vec<_>>()
        .join("\n");

    normalize_snapshot_text(&raw)
}

/// 规范化快照文本，避免“仅边框字符变化”导致大量无意义的快照 churn。
///
/// 背景：
/// - 本次引入了 exabind 风格边框字符集（例如 `▟▜▔▏▕`）。
/// - 但我们的快照测试主要关注布局与关键文本，不希望被边框 glyph 绑死。
fn normalize_snapshot_text(text: &str) -> String {
    text.chars()
        .map(|ch| match ch {
            // exabind：horizontal/bottom/corner 统一用 `▔`，快照里归一化成 box-drawing 的横线。
            '▔' => '─',
            // exabind：左右边框更“锐利”，快照里归一化成 box-drawing 的竖线。
            '▏' | '▕' => '│',
            // exabind：上角使用斜切角，快照里归一化成 box-drawing 的角。
            '▟' => '┌',
            '▜' => '┐',
            _ => ch,
        })
        .collect()
}

/// Snapshot-friendly representation of TUI state.
/// Excludes timing fields (Instant) that would make snapshots non-deterministic.
#[derive(Debug, Serialize, PartialEq)]
pub struct TuiSnapshot {
    /// Current iteration number (internal counter, 0-indexed).
    pub iteration: u32,
    /// Index of iteration being viewed (0-indexed).
    pub current_view: usize,
    /// Total number of iterations.
    pub total_iterations: usize,
    /// Whether auto-following latest iteration.
    pub following_latest: bool,
    /// Whether there's a new iteration alert.
    pub has_alert: bool,
    /// The iteration number in the alert (if any).
    pub alert_iteration: Option<usize>,
    /// Current search query.
    pub search_query: Option<String>,
    /// Number of search matches.
    pub search_matches: usize,
    /// Current match index.
    pub search_current_match: usize,
    /// Pending hat display string.
    pub pending_hat: String,
    /// Whether help overlay is shown.
    pub show_help: bool,
}

/// Builder for creating event sequences for testing.
#[allow(dead_code)]
pub struct EventSequenceBuilder {
    events: Vec<Event>,
}

#[allow(dead_code)]
impl EventSequenceBuilder {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    /// Add task.start event (initializes loop).
    pub fn task_start(mut self) -> Self {
        self.events.push(Event::new("task.start", "Start"));
        self
    }

    /// Add build.task event (starts builder iteration).
    pub fn build_task(mut self, description: &str) -> Self {
        self.events.push(Event::new("build.task", description));
        self
    }

    /// Add build.done event (completes iteration).
    pub fn build_done(mut self) -> Self {
        self.events.push(Event::new("build.done", "Done"));
        self
    }

    /// Add build.blocked event (iteration blocked).
    pub fn build_blocked(mut self, reason: &str) -> Self {
        self.events.push(Event::new("build.blocked", reason));
        self
    }

    /// Add loop.terminate event (ends loop).
    pub fn loop_terminate(mut self) -> Self {
        self.events.push(Event::new("loop.terminate", "Complete"));
        self
    }

    /// Add custom event.
    pub fn event(mut self, topic: &str, payload: &str) -> Self {
        self.events.push(Event::new(topic, payload));
        self
    }

    /// Build the event sequence.
    pub fn build(self) -> Vec<Event> {
        self.events
    }
}

impl Default for EventSequenceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to create a multi-iteration event sequence.
pub fn multi_iteration_events(num_iterations: u32) -> Vec<Event> {
    let mut builder = EventSequenceBuilder::new().task_start();

    for i in 0..num_iterations {
        builder = builder.build_task(&format!("Task {}", i + 1)).build_done();
    }

    builder.build()
}

/// Helper to create content lines for testing.
pub fn create_test_lines(count: usize, prefix: &str) -> Vec<Line<'static>> {
    (0..count)
        .map(|i| Line::raw(format!("{} line {}", prefix, i + 1)))
        .collect()
}

/// JSONL event format for fixtures.
#[derive(Debug, Deserialize)]
struct JsonlEvent {
    topic: String,
    payload: String,
}

/// Load events from a JSONL fixture file.
pub fn load_events_from_jsonl<P: AsRef<Path>>(path: P) -> Vec<Event> {
    let file = File::open(path).expect("Failed to open fixture file");
    let reader = BufReader::new(file);

    reader
        .lines()
        .filter_map(|line| {
            let line = line.ok()?;
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            let jsonl_event: JsonlEvent = serde_json::from_str(line).ok()?;
            Some(Event::new(
                jsonl_event.topic.as_str(),
                jsonl_event.payload.as_str(),
            ))
        })
        .collect()
}

/// Get the path to a fixture file relative to the tests directory.
pub fn fixture_path(name: &str) -> std::path::PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(manifest_dir)
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harness_processes_events() {
        let events = EventSequenceBuilder::new()
            .task_start()
            .build_task("Task 1")
            .build_done()
            .build();

        let mut harness = TuiTestHarness::with_events(events);
        harness.process_all();

        let snapshot = harness.capture_state();
        assert_eq!(snapshot.iteration, 1);
    }

    #[test]
    fn harness_advance_to_iteration() {
        let events = multi_iteration_events(3);
        let mut harness = TuiTestHarness::with_events(events);

        harness.advance_to_iteration(2);
        let snapshot = harness.capture_state();
        assert_eq!(snapshot.iteration, 2);
    }

    #[test]
    fn harness_captures_navigation_state() {
        let mut harness = TuiTestHarness::new();

        // Create 3 iterations manually
        {
            let mut state = harness.state.lock().unwrap();
            state.start_new_iteration();
            state.start_new_iteration();
            state.start_new_iteration();
        }

        // Navigate back
        harness.navigate_prev();

        let snapshot = harness.capture_state();
        assert_eq!(snapshot.current_view, 1);
        assert!(!snapshot.following_latest);
    }

    #[test]
    fn event_sequence_builder_works() {
        let events = EventSequenceBuilder::new()
            .task_start()
            .build_task("Do something")
            .build_done()
            .loop_terminate()
            .build();

        assert_eq!(events.len(), 4);
        assert_eq!(events[0].topic.as_str(), "task.start");
        assert_eq!(events[3].topic.as_str(), "loop.terminate");
    }
}
