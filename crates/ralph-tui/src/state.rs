//! State management for the TUI.

use ratatui::text::Line;
use ralph_core::HatJobOutputChunk;
use ralph_proto::{Event, HatId, HatInstanceId, HatInstanceState};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

// ============================================================================
// 并行模式（Supervisor TUI）state
// ============================================================================

pub(crate) mod output;
pub(crate) mod parallel;
pub(crate) mod radar;
pub(crate) mod search;
pub(crate) mod task;

use output::OutputSlice;
pub use output::IterationBuffer;
use radar::RadarSlice;
pub use radar::{
    HatGraphRadar, HatGraphRadarEdgeAnimation, HatGraphRadarEdgeMeta, HatGraphRadarMeta,
    HatGraphRadarNodeMeta, HatGraphRadarPoint, HatGraphRadarRect, HatGraphRadarRecentEvent,
};
use parallel::output::ParallelOutputBuffer;
pub use parallel::{
    ChatEditorState, GateStatus, ParallelEvidencePaths, ParallelFocus, ParallelTuiState, ScreenPos,
    ScreenSelection, TextPos, TextSelection,
};

use search::SearchSlice;
use task::{TaskCounts, TaskSlice, TaskSummary};

/// 当前输出视图的只读 buffer 视图（串行/并行统一抽象）。
pub enum CurrentOutputBuffer<'a> {
    Serial(&'a IterationBuffer),
    Parallel(&'a ParallelOutputBuffer),
}

impl CurrentOutputBuffer<'_> {
    pub fn row_count(&self) -> usize {
        match self {
            Self::Serial(buf) => buf.line_count(),
            Self::Parallel(buf) => buf.row_count(),
        }
    }

    pub fn scroll_offset(&self) -> usize {
        match self {
            Self::Serial(buf) => buf.scroll_offset,
            Self::Parallel(buf) => buf.scroll_offset,
        }
    }

    pub fn following_bottom(&self) -> bool {
        match self {
            Self::Serial(buf) => buf.following_bottom,
            Self::Parallel(buf) => buf.following_bottom,
        }
    }
}

/// 当前输出视图的可变 buffer 视图（串行/并行统一抽象）。
pub enum CurrentOutputBufferMut<'a> {
    Serial(&'a mut IterationBuffer),
    Parallel(&'a mut ParallelOutputBuffer),
}

impl CurrentOutputBufferMut<'_> {
    pub fn row_count(&self) -> usize {
        match self {
            Self::Serial(buf) => buf.line_count(),
            Self::Parallel(buf) => buf.row_count(),
        }
    }

    pub fn scroll_offset(&self) -> usize {
        match self {
            Self::Serial(buf) => buf.scroll_offset,
            Self::Parallel(buf) => buf.scroll_offset,
        }
    }

    pub fn set_scroll_offset_clamped(&mut self, idx: usize) {
        match self {
            Self::Serial(buf) => {
                if buf.line_count() == 0 {
                    buf.scroll_offset = 0;
                } else {
                    buf.scroll_offset = idx.min(buf.line_count().saturating_sub(1));
                }
            }
            Self::Parallel(buf) => buf.set_scroll_offset_clamped(idx),
        }
    }

    pub fn following_bottom(&self) -> bool {
        match self {
            Self::Serial(buf) => buf.following_bottom,
            Self::Parallel(buf) => buf.following_bottom,
        }
    }

    pub fn scroll_up(&mut self) {
        match self {
            Self::Serial(buf) => buf.scroll_up(),
            Self::Parallel(buf) => buf.scroll_up(),
        }
    }

    pub fn scroll_down(&mut self, viewport_height: usize) {
        match self {
            Self::Serial(buf) => buf.scroll_down(viewport_height),
            Self::Parallel(buf) => buf.scroll_down(viewport_height),
        }
    }

    pub fn scroll_top(&mut self) {
        match self {
            Self::Serial(buf) => buf.scroll_top(),
            Self::Parallel(buf) => buf.scroll_top(),
        }
    }

    pub fn scroll_bottom(&mut self, viewport_height: usize) {
        match self {
            Self::Serial(buf) => buf.scroll_bottom(viewport_height),
            Self::Parallel(buf) => buf.scroll_bottom(viewport_height),
        }
    }
}

/// TUI 运行模式：串行（按 iteration）/ 并行（按 instance/job）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiMode {
    Serial,
    Parallel,
}

// =============================================================================
// Hat Graph Radar（右上角覆盖层）
// =============================================================================
//
// 设计目标：
// - 在 TUI 右上角提供一个类似“游戏雷达/小地图”的拓扑速览；
// - 内容来自 hats graph 的 Mermaid 拓扑，但在终端里以 ASCII 图展示；
// - 放大/还原只是 UI 行为，不影响 orchestration。
//
// 说明：
// - ASCII 图由 ralph-cli 在启动 TUI 时生成并注入，这里只做缓存 + 展示，
//   避免在 TUI 每帧渲染时重复做 Mermaid→ASCII 的转换。

pub enum TuiUpdate {
    /// 并行：注册初始实例（Created）。
    ParallelRegisterInstance {
        instance_id: HatInstanceId,
        state: HatInstanceState,
    },
    /// 并行：实例状态变更（Running/Idle/Done/...）。
    ParallelInstanceState {
        instance_id: HatInstanceId,
        state: HatInstanceState,
    },
    /// 并行：输出流式 chunk（按行）。
    ParallelOutputChunk(HatJobOutputChunk),
    /// 并行：控制面事件（gate.* / human.message 等）。
    ParallelEvent(Event),
    /// 并行：UI 内部提示（例如写事件文件失败）。
    ParallelStatus(String),
}

/// Observable state derived from loop events.
pub struct TuiState {
    /// 当前 TUI 模式（串行/并行）。
    pub mode: TuiMode,

    /// Which hat will process next event (ID + display name).
    pub pending_hat: Option<(HatId, String)>,
    /// Current iteration number (0-indexed, display as +1).
    pub iteration: u32,
    /// Previous iteration number (for detecting changes).
    pub prev_iteration: u32,
    /// When loop began.
    pub loop_started: Option<Instant>,
    /// When current iteration began.
    pub iteration_started: Option<Instant>,
    /// Most recent event topic.
    pub last_event: Option<String>,
    /// Timestamp of last event.
    pub last_event_at: Option<Instant>,
    /// Whether to show help overlay.
    pub show_help: bool,
    /// Whether in scroll mode.
    pub in_scroll_mode: bool,
    /// Maximum iterations from config.
    pub max_iterations: Option<u32>,
    /// Idle timeout countdown.
    pub idle_timeout_remaining: Option<Duration>,
    /// Map of event topics to hat display information (for custom hats).
    /// Key: event topic (e.g., "review.security")
    /// Value: (HatId, display name including emoji)
    hat_map: HashMap<String, (HatId, String)>,



    // ========================================================================
    // Completion State
    // ========================================================================
    /// Whether the loop has completed (received loop.terminate event).
    pub loop_completed: bool,
    /// Frozen elapsed time when loop completed (timer stops at this value).
    pub final_iteration_elapsed: Option<Duration>,



    // ========================================================================
    // 领域切片 (独立变化 + 独立测试)
    // ========================================================================
    /// 任务计数与活跃任务。
    pub task: TaskSlice,
    /// Hat Graph Radar 可视化状态机。
    pub radar: RadarSlice,
    /// 搜索状态与匹配导航。
    pub search: SearchSlice,
    /// 串行输出缓冲(迭代/浏览/选择)。
    pub output: OutputSlice,

    // ========================================================================
    // Parallel Mode State
    // ========================================================================
    /// 并行模式（Supervisor TUI）的状态（默认空）。
    pub parallel: ParallelTuiState,
}

impl TuiState {
    /// Creates empty state. Timer starts immediately at creation.
    pub fn new() -> Self {
        Self {
            mode: TuiMode::Serial,
            pending_hat: None,
            iteration: 0,
            prev_iteration: 0,
            loop_started: Some(Instant::now()),
            iteration_started: None,
            last_event: None,
            last_event_at: None,
            show_help: false,
            in_scroll_mode: false,
            max_iterations: None,
            idle_timeout_remaining: None,
            hat_map: HashMap::new(),
            task: TaskSlice::default(),
            radar: RadarSlice::default(),
            search: SearchSlice::default(),
            output: OutputSlice::default(),
            // Completion state
            loop_completed: false,
            final_iteration_elapsed: None,
            // Parallel mode
            parallel: ParallelTuiState::default(),
        }
    }

    /// 创建并行模式的初始 state（Supervisor TUI）。
    pub fn new_parallel() -> Self {
        let mut state = Self::new();
        state.mode = TuiMode::Parallel;
        state
    }

    /// Creates state with a custom hat map for dynamic topic-to-hat resolution.
    /// Timer starts immediately at creation.
    pub fn with_hat_map(hat_map: HashMap<String, (HatId, String)>) -> Self {
        Self {
            mode: TuiMode::Serial,
            pending_hat: None,
            iteration: 0,
            prev_iteration: 0,
            loop_started: Some(Instant::now()),
            iteration_started: None,
            last_event: None,
            last_event_at: None,
            show_help: false,
            in_scroll_mode: false,
            max_iterations: None,
            idle_timeout_remaining: None,
            hat_map,
            task: TaskSlice::default(),
            radar: RadarSlice::default(),
            search: SearchSlice::default(),
            output: OutputSlice::default(),
            // Completion state
            loop_completed: false,
            final_iteration_elapsed: None,
            // Parallel mode
            parallel: ParallelTuiState::default(),
        }
    }

    /// Updates state based on event topic.
    pub fn update(&mut self, event: &Event) {
        // 串行模式下：按 EventBus 事件更新 header/计时等信息。
        // 并行模式下：事件不走这里（用 apply_update + ParallelEvent）。
        if self.mode == TuiMode::Parallel {
            return;
        }

        let now = Instant::now();
        let topic = event.topic.as_str();

        self.last_event = Some(topic.to_string());
        self.last_event_at = Some(now);
        self.record_hat_graph_radar_event(event, now);

        // First, check if we have a custom hat mapping for this topic
        if let Some((hat_id, hat_display)) = self.hat_map.get(topic) {
            self.pending_hat = Some((hat_id.clone(), hat_display.clone()));
            // Handle iteration timing for custom hats
            if topic.starts_with("build.") {
                self.iteration_started = Some(now);
            }
            return;
        }

        // Fall back to hardcoded mappings for backward compatibility
        match topic {
            "task.start" => {
                // Save state we want to preserve across reset
                let saved_hat_map = std::mem::take(&mut self.hat_map);
                let saved_loop_started = self.loop_started; // Preserve timer from TUI init
                *self = Self::new();
                self.hat_map = saved_hat_map;
                self.loop_started = saved_loop_started; // Keep original timer
                self.pending_hat = Some((HatId::new("planner"), "📋Planner".to_string()));
                self.last_event = Some(topic.to_string());
                self.last_event_at = Some(now);
            }
            "task.resume" => {
                // Don't reset timer on resume - keep counting from TUI init
                self.pending_hat = Some((HatId::new("planner"), "📋Planner".to_string()));
            }
            "build.task" => {
                self.pending_hat = Some((HatId::new("builder"), "🔨Builder".to_string()));
                self.iteration_started = Some(now);
            }
            "build.done" => {
                self.pending_hat = Some((HatId::new("planner"), "📋Planner".to_string()));
                self.prev_iteration = self.iteration;
                self.iteration += 1;
            }
            "build.blocked" => {
                self.pending_hat = Some((HatId::new("planner"), "📋Planner".to_string()));
            }
            "loop.terminate" => {
                self.pending_hat = None;
                self.loop_completed = true;
                // Freeze the iteration timer at its current value
                self.final_iteration_elapsed = self.iteration_started.map(|start| start.elapsed());
            }
            _ => {
                // Unknown topic - don't change pending_hat
            }
        }
    }

    /// 应用一个 UI 更新事件（observer → channel → reducer）。
    ///
    /// 说明：
    /// - 串行模式继续沿用“直接锁 state 更新”的老路径（`Tui::observer()`）。
    /// - 并行模式统一走该 reducer，避免多处并发写状态造成撕裂。
    pub fn apply_update(&mut self, update: TuiUpdate) {
        if self.mode != TuiMode::Parallel {
            return;
        }

        match update {
            TuiUpdate::ParallelRegisterInstance { instance_id, state } => {
                self.parallel.register_instance(instance_id, state);
            }
            TuiUpdate::ParallelInstanceState { instance_id, state } => {
                // 说明：
                // - 你希望“哪个 box 进入 Running，就显示它的染色 + cause event 线路动画”；
                // - 因此我们需要捕捉“非 Running → Running”的跃迁点。
                let prev_state = self.parallel.instances.get(&instance_id).map(|s| s.state);
                self.parallel.set_instance_state(instance_id.clone(), state);

                let now = Instant::now();
                if let Some(hat_id) = instance_id.split_hat_id() {
                    if state == HatInstanceState::Running
                        && prev_state != Some(HatInstanceState::Running)
                    {
                        self.maybe_start_hat_graph_edge_animation_for_running_hat(
                            HatId::new(hat_id),
                            now,
                        );
                    }

                    // 如果某个实例退出 Running，需要判断该 hat 是否“已经没有任何 Running 实例”。
                    // 若是，则立刻取消该 hat 的线路动画（符合你“目标不 Running 就取消”的口径）。
                    if prev_state == Some(HatInstanceState::Running)
                        && state != HatInstanceState::Running
                        && !self.is_hat_running_parallel(hat_id)
                    {
                        self.radar.hat_graph_edge_animations.remove(&HatId::new(hat_id));
                    }
                }
            }
            TuiUpdate::ParallelOutputChunk(chunk) => {
                self.parallel.append_output(&chunk);
            }
            TuiUpdate::ParallelEvent(event) => {
                // 并行模式：同步更新“最近事件/活跃度”指标（与串行模式一致）。
                let now = Instant::now();
                self.last_event = Some(event.topic.as_str().to_string());
                self.last_event_at = Some(now);
                self.record_hat_graph_radar_event(&event, now);
                self.parallel.apply_event(&event);
            }
            TuiUpdate::ParallelStatus(msg) => {
                self.parallel.chat_status = Some(msg);
            }
        }
    }

    fn is_hat_running_parallel(&self, hat_id: &str) -> bool {
        self.parallel.instances.iter().any(|(instance_id, view)| {
            view.state == HatInstanceState::Running
                && instance_id.split_hat_id().is_some_and(|id| id == hat_id)
        })
    }

    fn record_hat_graph_radar_event(&mut self, event: &Event, now: Instant) {
        self.radar.record_event(event, now);
    }

    /// Running hat 跃迁时启动因果边动画(壳协调: 从 parallel 域拿 running 状态)。
    fn maybe_start_hat_graph_edge_animation_for_running_hat(
        &mut self,
        target_hat: HatId,
        now: Instant,
    ) {
        self.radar.maybe_start_edge_animation(target_hat, now);
    }

    /// 每帧推进 Radar 动画状态(壳计算 running_hats 注入)。
    pub(crate) fn tick_hat_graph_radar_animation(&mut self, now: Instant) {
        let running_hats = if self.mode == TuiMode::Parallel {
            let mut hats = HashSet::new();
            for (instance_id, view) in &self.parallel.instances {
                if view.state != HatInstanceState::Running {
                    continue;
                }
                if let Some(hat_id) = instance_id.split_hat_id() {
                    hats.insert(hat_id.to_string());
                }
            }
            Some(hats)
        } else {
            None
        };
        self.radar.tick(now, running_hats.as_ref());
    }

    pub fn get_pending_hat_display(&self) -> String {
        self.pending_hat
            .as_ref()
            .map_or_else(|| "—".to_string(), |(_, display)| display.clone())
    }

    /// Time since loop started.
    pub fn get_loop_elapsed(&self) -> Option<Duration> {
        self.loop_started.map(|start| start.elapsed())
    }

    /// Time since iteration started, or frozen value if loop completed.
    pub fn get_iteration_elapsed(&self) -> Option<Duration> {
        // Return frozen elapsed time if loop has completed
        if let Some(final_elapsed) = self.final_iteration_elapsed {
            return Some(final_elapsed);
        }
        self.iteration_started.map(|start| start.elapsed())
    }

    /// True if event received in last 2 seconds.
    pub fn is_active(&self) -> bool {
        self.last_event_at
            .is_some_and(|t| t.elapsed() < Duration::from_secs(2))
    }

    /// True if iteration changed since last check.
    pub fn iteration_changed(&self) -> bool {
        self.iteration != self.prev_iteration
    }

    // ========================================================================
    // Task Tracking Methods
    // ========================================================================

    /// Returns a reference to the current task counts.
    pub fn get_task_counts(&self) -> &TaskCounts {
        &self.task.task_counts
    }

    /// Returns a reference to the active task, if any.
    pub fn get_active_task(&self) -> Option<&TaskSummary> {
        self.task.active_task.as_ref()
    }

    /// Updates the task counts.
    pub fn set_task_counts(&mut self, counts: TaskCounts) {
        self.task.task_counts = counts;
    }

    /// Sets the active task.
    pub fn set_active_task(&mut self, task: Option<TaskSummary>) {
        self.task.active_task = task;
    }

    // ========================================================================
    // Hat Graph Radar Methods
    // ========================================================================

    /// 注入 hats graph radar 的 ASCII 渲染结果（由 CLI 在启动 TUI 时生成）。
    pub fn set_hat_graph_radar(&mut self, radar: HatGraphRadar) {
        self.radar.hat_graph_radar = Some(radar);
    }

    /// Returns true if there are any open tasks.
    pub fn has_open_tasks(&self) -> bool {
        self.task.task_counts.open > 0
    }

    /// Returns a formatted string for task progress display (e.g., "3/5 tasks").
    pub fn get_task_progress_display(&self) -> String {
        if self.task.task_counts.total == 0 {
            "No tasks".to_string()
        } else {
            format!(
                "{}/{} tasks",
                self.task.task_counts.closed, self.task.task_counts.total
            )
        }
    }

    // ========================================================================
    // Iteration Management Methods
    // ========================================================================

    /// Starts a new iteration, creating a new IterationBuffer.
    /// If following_latest is true, current_view is updated to the new iteration.
    /// If not following, sets the new_iteration_alert to notify the user.
    pub fn start_new_iteration(&mut self) {
        self.output.start_new_iteration();
    }

    pub fn current_iteration(&self) -> Option<&IterationBuffer> {
        self.output.current_iteration()
    }

    pub fn current_iteration_mut(&mut self) -> Option<&mut IterationBuffer> {
        self.output.current_iteration_mut()
    }

    pub fn clear_serial_output_selection(&mut self) {
        self.output.clear_serial_output_selection();
    }

    pub fn start_serial_output_selection(&mut self, pos: ScreenPos) {
        self.output.start_serial_output_selection(pos);
    }

    pub fn update_serial_output_selection_cursor(&mut self, pos: ScreenPos) {
        self.output.update_serial_output_selection_cursor(pos);
    }

    pub fn finish_serial_output_selection(&mut self) {
        self.output.finish_serial_output_selection();
    }

    pub fn extend_serial_output_selection_by_delta(
        &mut self,
        dx: i16,
        dy: i16,
        max_x: u16,
        max_y: u16,
    ) {
        self.output.extend_serial_output_selection_by_delta(dx, dy, max_x, max_y);
    }

    pub fn current_iteration_lines_handle(
        &self,
    ) -> Option<std::sync::Arc<std::sync::Mutex<Vec<Line<'static>>>>> {
        self.output.current_iteration_lines_handle()
    }

    pub fn latest_iteration_lines_handle(
        &self,
    ) -> Option<std::sync::Arc<std::sync::Mutex<Vec<Line<'static>>>>> {
        self.output.latest_iteration_lines_handle()
    }

    pub fn navigate_next(&mut self) {
        self.output.navigate_next();
    }

    pub fn navigate_prev(&mut self) {
        self.output.navigate_prev();
    }

    pub fn total_iterations(&self) -> usize {
        self.output.total_iterations()
    }

    /// 返回“当前可滚动输出视图”的 buffer。
    ///
    /// - 串行模式：当前 iteration 的 buffer
    /// - 并行模式：当前选中实例的当前 job buffer
    pub fn current_output_buffer(&self) -> Option<CurrentOutputBuffer<'_>> {
        match self.mode {
            TuiMode::Serial => self.current_iteration().map(CurrentOutputBuffer::Serial),
            TuiMode::Parallel => self
                .parallel
                .selected_instance()
                .and_then(|i| i.current_job_buffer())
                .map(CurrentOutputBuffer::Parallel),
        }
    }

    /// 返回“当前可滚动输出视图”的可变 buffer。
    pub fn current_output_buffer_mut(&mut self) -> Option<CurrentOutputBufferMut<'_>> {
        match self.mode {
            TuiMode::Serial => self
                .current_iteration_mut()
                .map(CurrentOutputBufferMut::Serial),
            TuiMode::Parallel => self
                .parallel
                .selected_instance_mut()
                .and_then(|i| i.current_job_buffer_mut())
                .map(CurrentOutputBufferMut::Parallel),
        }
    }

    /// 在当前迭代/并行 buffer 中搜索(壳协调: 收集行 → 切片计算 → 跳转)。
    pub fn search(&mut self, query: &str) {
        let lines = match self.mode {
            TuiMode::Serial => self
                .output
                .current_iteration()
                .map(|buffer| {
                    buffer
                        .lines
                        .lock()
                        .ok()
                        .map(|lines| {
                            lines
                                .iter()
                                .map(|line| {
                                    line.spans
                                        .iter()
                                        .map(|span| span.content.as_ref())
                                        .collect::<String>()
                                })
                                .collect::<Vec<String>>()
                        })
                        .unwrap_or_default()
                })
                .unwrap_or_default(),
            TuiMode::Parallel => {
                let Some(buffer) = self
                    .parallel
                    .selected_instance()
                    .and_then(|i| i.current_job_buffer())
                else {
                    self.search.clear();
                    return;
                };
                buffer
                    .lines
                    .iter()
                    .map(|line| {
                        line.spans
                            .iter()
                            .map(|span| span.content.as_ref())
                            .collect::<String>()
                    })
                    .collect::<Vec<String>>()
            }
        };

        self.search.search_lines(query, &lines);
        self.jump_to_current_match();
    }

    /// 前进到下一个匹配。
    pub fn next_match(&mut self) {
        self.search.next();
        self.jump_to_current_match();
    }

    /// 后退到上一个匹配。
    pub fn prev_match(&mut self) {
        self.search.prev();
        self.jump_to_current_match();
    }

    /// 清空搜索状态。
    pub fn clear_search(&mut self) {
        self.search.clear();
    }

    /// 跳到当前匹配位置(调整滚动)。
    fn jump_to_current_match(&mut self) {
        let Some((line_idx, _)) = self.search.current() else {
            return;
        };

        // 使用默认视口高度(渲染时会用真实高度覆盖)。
        let viewport_height = 20;
        if let Some(mut buffer) = self.current_output_buffer_mut() {
            if line_idx < buffer.scroll_offset() {
                buffer.set_scroll_offset_clamped(line_idx);
            } else if line_idx >= buffer.scroll_offset() + viewport_height {
                buffer.set_scroll_offset_clamped(line_idx.saturating_sub(viewport_height / 2));
            }
        }
    }
}

impl Default for TuiState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // IterationBuffer Tests
    // ========================================================================

    mod iteration_buffer {
        use super::*;
        use ratatui::text::Line;

        #[test]
        fn new_creates_buffer_with_correct_initial_state() {
            let buffer = IterationBuffer::new(1);
            assert_eq!(buffer.number, 1);
            assert_eq!(buffer.line_count(), 0);
            assert_eq!(buffer.scroll_offset, 0);
        }

        #[test]
        fn append_line_adds_lines_in_order() {
            let mut buffer = IterationBuffer::new(1);
            buffer.append_line(Line::from("first"));
            buffer.append_line(Line::from("second"));
            buffer.append_line(Line::from("third"));

            assert_eq!(buffer.line_count(), 3);
            // Verify order by checking raw content
            let lines = buffer.lines.lock().unwrap();
            assert_eq!(lines[0].spans[0].content, "first");
            assert_eq!(lines[1].spans[0].content, "second");
            assert_eq!(lines[2].spans[0].content, "third");
        }

        #[test]
        fn replace_lines_capped_replaces_entire_content() {
            let mut buffer = IterationBuffer::new(1);
            buffer.append_line(Line::from("old"));

            buffer.replace_lines_capped(vec![Line::from("new1"), Line::from("new2")], 100);

            let lines = buffer.lines.lock().unwrap();
            assert_eq!(lines.len(), 2);
            assert_eq!(lines[0].spans[0].content, "new1");
            assert_eq!(lines[1].spans[0].content, "new2");
        }

        #[test]
        fn replace_lines_capped_drops_overflow_from_front_and_adjusts_scroll() {
            let mut buffer = IterationBuffer::new(1);
            buffer.scroll_offset = 2;

            let new_lines = (0..5)
                .map(|i| Line::from(format!("line {i}")))
                .collect::<Vec<_>>();
            buffer.replace_lines_capped(new_lines, 3);

            let lines = buffer.lines.lock().unwrap();
            assert_eq!(lines.len(), 3);
            assert_eq!(lines[0].spans[0].content, "line 2");
            assert_eq!(buffer.scroll_offset, 0);
        }

        #[test]
        fn line_count_returns_correct_count() {
            let mut buffer = IterationBuffer::new(1);
            assert_eq!(buffer.line_count(), 0);

            for i in 0..10 {
                buffer.append_line(Line::from(format!("line {}", i)));
            }
            assert_eq!(buffer.line_count(), 10);
        }

        #[test]
        fn visible_lines_returns_correct_slice_without_scroll() {
            let mut buffer = IterationBuffer::new(1);
            for i in 0..10 {
                buffer.append_line(Line::from(format!("line {}", i)));
            }

            let visible = buffer.visible_lines(5);
            assert_eq!(visible.len(), 5);
            // Should be lines 0-4
            assert_eq!(visible[0].spans[0].content, "line 0");
            assert_eq!(visible[4].spans[0].content, "line 4");
        }

        #[test]
        fn visible_lines_returns_correct_slice_with_scroll() {
            let mut buffer = IterationBuffer::new(1);
            for i in 0..10 {
                buffer.append_line(Line::from(format!("line {}", i)));
            }
            buffer.scroll_offset = 3;

            let visible = buffer.visible_lines(5);
            assert_eq!(visible.len(), 5);
            // Should be lines 3-7
            assert_eq!(visible[0].spans[0].content, "line 3");
            assert_eq!(visible[4].spans[0].content, "line 7");
        }

        #[test]
        fn visible_lines_handles_viewport_larger_than_content() {
            let mut buffer = IterationBuffer::new(1);
            for i in 0..3 {
                buffer.append_line(Line::from(format!("line {}", i)));
            }

            let visible = buffer.visible_lines(10);
            assert_eq!(visible.len(), 3); // Only 3 lines exist
        }

        #[test]
        fn visible_lines_handles_empty_buffer() {
            let buffer = IterationBuffer::new(1);
            let visible = buffer.visible_lines(5);
            assert!(visible.is_empty());
        }

        #[test]
        fn scroll_down_increases_offset() {
            let mut buffer = IterationBuffer::new(1);
            for i in 0..10 {
                buffer.append_line(Line::from(format!("line {}", i)));
            }

            assert_eq!(buffer.scroll_offset, 0);
            buffer.scroll_down(5); // viewport height 5
            assert_eq!(buffer.scroll_offset, 1);
            buffer.scroll_down(5);
            assert_eq!(buffer.scroll_offset, 2);
        }

        #[test]
        fn scroll_up_decreases_offset() {
            let mut buffer = IterationBuffer::new(1);
            for _ in 0..10 {
                buffer.append_line(Line::from("line"));
            }
            buffer.scroll_offset = 5;

            buffer.scroll_up();
            assert_eq!(buffer.scroll_offset, 4);
            buffer.scroll_up();
            assert_eq!(buffer.scroll_offset, 3);
        }

        #[test]
        fn scroll_up_does_not_underflow() {
            let mut buffer = IterationBuffer::new(1);
            buffer.append_line(Line::from("line"));
            buffer.scroll_offset = 0;

            buffer.scroll_up();
            assert_eq!(buffer.scroll_offset, 0); // Should stay at 0
        }

        #[test]
        fn scroll_down_does_not_overflow() {
            let mut buffer = IterationBuffer::new(1);
            for _ in 0..10 {
                buffer.append_line(Line::from("line"));
            }
            // With 10 lines and viewport 5, max scroll is 5 (shows lines 5-9)
            buffer.scroll_offset = 5;

            buffer.scroll_down(5);
            assert_eq!(buffer.scroll_offset, 5); // Should stay at max
        }

        #[test]
        fn scroll_top_resets_to_zero() {
            let mut buffer = IterationBuffer::new(1);
            for _ in 0..10 {
                buffer.append_line(Line::from("line"));
            }
            buffer.scroll_offset = 5;

            buffer.scroll_top();
            assert_eq!(buffer.scroll_offset, 0);
        }

        #[test]
        fn scroll_bottom_sets_to_max() {
            let mut buffer = IterationBuffer::new(1);
            for _ in 0..10 {
                buffer.append_line(Line::from("line"));
            }

            buffer.scroll_bottom(5); // viewport height 5
            assert_eq!(buffer.scroll_offset, 5); // max = 10 - 5 = 5
        }

        #[test]
        fn scroll_bottom_handles_small_content() {
            let mut buffer = IterationBuffer::new(1);
            for _ in 0..3 {
                buffer.append_line(Line::from("line"));
            }

            buffer.scroll_bottom(5); // viewport larger than content
            assert_eq!(buffer.scroll_offset, 0); // Can't scroll
        }

        #[test]
        fn scroll_down_handles_empty_buffer() {
            let mut buffer = IterationBuffer::new(1);
            buffer.scroll_down(5);
            assert_eq!(buffer.scroll_offset, 0);
        }

        // =====================================================================
        // Auto-scroll (following_bottom) Tests
        // =====================================================================

        #[test]
        fn following_bottom_is_true_initially() {
            let buffer = IterationBuffer::new(1);
            assert!(
                buffer.following_bottom,
                "New buffer should start with following_bottom = true"
            );
        }

        #[test]
        fn scroll_up_disables_following_bottom() {
            let mut buffer = IterationBuffer::new(1);
            for _ in 0..10 {
                buffer.append_line(Line::from("line"));
            }
            buffer.scroll_offset = 5;
            assert!(buffer.following_bottom);

            buffer.scroll_up();

            assert!(
                !buffer.following_bottom,
                "scroll_up should disable following_bottom"
            );
        }

        #[test]
        fn scroll_top_disables_following_bottom() {
            let mut buffer = IterationBuffer::new(1);
            for _ in 0..10 {
                buffer.append_line(Line::from("line"));
            }
            assert!(buffer.following_bottom);

            buffer.scroll_top();

            assert!(
                !buffer.following_bottom,
                "scroll_top should disable following_bottom"
            );
        }

        #[test]
        fn scroll_bottom_enables_following_bottom() {
            let mut buffer = IterationBuffer::new(1);
            for _ in 0..10 {
                buffer.append_line(Line::from("line"));
            }
            buffer.following_bottom = false;

            buffer.scroll_bottom(5);

            assert!(
                buffer.following_bottom,
                "scroll_bottom should enable following_bottom"
            );
        }

        #[test]
        fn scroll_down_to_bottom_enables_following_bottom() {
            let mut buffer = IterationBuffer::new(1);
            for _ in 0..10 {
                buffer.append_line(Line::from("line"));
            }
            buffer.scroll_offset = 4; // One away from max (5 with viewport 5)
            buffer.following_bottom = false;

            buffer.scroll_down(5); // Now at max (5)

            assert!(
                buffer.following_bottom,
                "scroll_down to bottom should enable following_bottom"
            );
        }

        #[test]
        fn scroll_down_not_at_bottom_keeps_following_false() {
            let mut buffer = IterationBuffer::new(1);
            for _ in 0..10 {
                buffer.append_line(Line::from("line"));
            }
            buffer.scroll_offset = 0;
            buffer.following_bottom = false;

            buffer.scroll_down(5); // Now at 1, max is 5

            assert!(
                !buffer.following_bottom,
                "scroll_down not reaching bottom should keep following_bottom false"
            );
        }

        #[test]
        fn autoscroll_scenario_content_grows_past_viewport() {
            // This tests the core bug fix: content growing from small to large
            let mut buffer = IterationBuffer::new(1);

            // Start with small content that fits in viewport
            for _ in 0..5 {
                buffer.append_line(Line::from("line"));
            }

            // Simulate initial state: following_bottom = true, scroll_offset = 0
            let viewport = 20;
            assert!(buffer.following_bottom);
            assert_eq!(buffer.scroll_offset, 0);

            // Simulate auto-scroll logic: if following_bottom, scroll to bottom
            if buffer.following_bottom {
                let max_scroll = buffer.line_count().saturating_sub(viewport);
                buffer.scroll_offset = max_scroll;
            }
            assert_eq!(buffer.scroll_offset, 0); // max_scroll is 0 when content < viewport

            // Content grows past viewport size
            for _ in 0..25 {
                buffer.append_line(Line::from("more content"));
            }
            // Now we have 30 lines, viewport is 20, max_scroll = 10

            // The bug was: scroll_offset = 0, but old logic checked if 0 >= 10-1 (false)
            // With following_bottom flag, we just check the flag:
            if buffer.following_bottom {
                let max_scroll = buffer.line_count().saturating_sub(viewport);
                buffer.scroll_offset = max_scroll;
            }

            // Now scroll_offset should be at the bottom
            assert_eq!(
                buffer.scroll_offset, 10,
                "Auto-scroll should move to bottom when content grows past viewport"
            );
        }
    }

    // ========================================================================
    // TuiState Tests (existing)
    // ========================================================================

    #[test]
    fn iteration_changed_detects_boundary() {
        let mut state = TuiState::new();
        assert!(!state.iteration_changed(), "no change at start");

        // Simulate build.done event (increments iteration)
        let event = Event::new("build.done", "");
        state.update(&event);

        assert_eq!(state.iteration, 1);
        assert_eq!(state.prev_iteration, 0);
        assert!(state.iteration_changed(), "should detect iteration change");
    }

    #[test]
    fn iteration_changed_resets_after_check() {
        let mut state = TuiState::new();
        let event = Event::new("build.done", "");
        state.update(&event);

        assert!(state.iteration_changed());

        // Simulate clearing the flag (app.rs does this by updating prev_iteration)
        state.prev_iteration = state.iteration;
        assert!(!state.iteration_changed(), "flag should reset");
    }

    #[test]
    fn multiple_iterations_tracked() {
        let mut state = TuiState::new();

        for i in 1..=3 {
            let event = Event::new("build.done", "");
            state.update(&event);
            assert_eq!(state.iteration, i);
            assert!(state.iteration_changed());
            state.prev_iteration = state.iteration; // simulate app clearing flag
        }
    }

    #[test]
    fn serial_event_with_source_is_recorded_for_cause_inference() {
        // 说明：
        // - 新口径下，Radar 的线路动画不再是“全局最新事件”，而是“按 Running 目标触发的短动画”；
        // - 但我们仍需要记录最近业务事件，用于后续推断某个 hat 进入 Running 时的 cause event。
        let mut state = TuiState::new();

        let event = Event::new("build.task", "").with_source(HatId::new("builder"));
        state.update(&event);

        assert_eq!(state.radar.hat_graph_recent_events.len(), 1);
        let last = state
            .radar.hat_graph_recent_events
            .back()
            .expect("recent event should exist");
        assert_eq!(last.source_hat.as_str(), "builder");
        assert_eq!(last.topic, "build.task");
    }

    #[test]
    fn parallel_running_transition_starts_and_cancels_cause_edge_animation() {
        // 说明：
        // - 当某个 hat 从非 Running → Running 时，应启动“cause event”线路动画；
        // - 当该 hat 不再 Running 时，应立刻取消该线路动画。
        let mut state = TuiState::new_parallel();

        // 准备一张最小拓扑：planner --build.task--> builder
        state.set_hat_graph_radar(HatGraphRadar {
            ascii_compact: String::new(),
            ascii_full: String::new(),
            meta_compact: Some(HatGraphRadarMeta {
                nodes: vec![
                    HatGraphRadarNodeMeta {
                        id: "Hat_planner".to_string(),
                        label: "planner".to_string(),
                        box_rect: HatGraphRadarRect {
                            x: 0,
                            y: 0,
                            width: 1,
                            height: 1,
                        },
                    },
                    HatGraphRadarNodeMeta {
                        id: "Hat_builder".to_string(),
                        label: "builder".to_string(),
                        box_rect: HatGraphRadarRect {
                            x: 0,
                            y: 0,
                            width: 1,
                            height: 1,
                        },
                    },
                ],
                edges: vec![HatGraphRadarEdgeMeta {
                    from: "Hat_planner".to_string(),
                    to: "Hat_builder".to_string(),
                    label: "build.task".to_string(),
                    path: vec![
                        HatGraphRadarPoint { x: 0, y: 0 },
                        HatGraphRadarPoint { x: 1, y: 0 },
                        HatGraphRadarPoint { x: 2, y: 0 },
                    ],
                }],
            }),
            meta_full: None,
        });

        // 1) 先收到一个业务事件（用于 cause 推断）
        let event = Event::new("build.task", "").with_source(HatId::new("planner"));
        state.apply_update(TuiUpdate::ParallelEvent(event));

        // 2) builder#1 进入 Running：应触发边动画（planner -> builder）
        state.apply_update(TuiUpdate::ParallelRegisterInstance {
            instance_id: HatInstanceId::new("builder#1"),
            state: HatInstanceState::Created,
        });
        state.apply_update(TuiUpdate::ParallelInstanceState {
            instance_id: HatInstanceId::new("builder#1"),
            state: HatInstanceState::Running,
        });

        let anim = state
            .radar.hat_graph_edge_animations
            .get(&HatId::new("builder"))
            .expect("edge animation should be started when builder enters Running");
        assert_eq!(anim.source_hat.as_str(), "planner");
        assert_eq!(anim.target_hat.as_str(), "builder");
        assert_eq!(anim.topic, "build.task");

        // 说明：
        // - 线路在 reveal 完成后应该持续显示，直到目标 hat 退出 Running；
        // - 因此 tick 到很久之后（但仍保持 Running）也不应被自动清理。
        let future = anim.started_at + Duration::from_secs(120);
        state.tick_hat_graph_radar_animation(future);
        assert!(
            state
                .radar.hat_graph_edge_animations
                .contains_key(&HatId::new("builder")),
            "edge animation should persist while target hat remains Running"
        );

        // 3) builder#1 退出 Running：应立刻取消该 hat 的线路动画
        state.apply_update(TuiUpdate::ParallelInstanceState {
            instance_id: HatInstanceId::new("builder#1"),
            state: HatInstanceState::Idle,
        });
        assert!(
            !state
                .radar.hat_graph_edge_animations
                .contains_key(&HatId::new("builder")),
            "edge animation should be cancelled when builder is no longer Running"
        );
    }

    #[test]
    fn hat_graph_meta_matches_collapsed_multi_topic_labels() {
        // 回归测试：
        // - physical view 可能把同一对节点间的多条边折叠成一条（label 用 " / " 拼接）。
        // - Radar 做因果边匹配时必须能用“单个 topic”匹配到这类折叠 label。
        let meta = HatGraphRadarMeta {
            nodes: Vec::new(),
            edges: vec![HatGraphRadarEdgeMeta {
                from: "Hat_ralph".to_string(),
                to: "Hat_runner".to_string(),
                label: "integration.applied / integration.blocked / integration.rejected"
                    .to_string(),
                path: Vec::new(),
            }],
        };

        assert!(
            meta.matching_edges_exact("Hat_ralph", "integration.applied", "Hat_runner")
                .next()
                .is_some(),
            "expected topic to match within collapsed label"
        );

        assert!(
            meta.matching_edges_exact("Hat_ralph", "integration.unknown", "Hat_runner")
                .next()
                .is_none(),
            "unexpected topic should not match collapsed label"
        );
    }

    #[test]
    fn hat_graph_edge_render_plan_reveals_then_scans_until_cancelled_by_running_state() {
        // 说明：
        // - 这个测试只验证“渲染计划”本身：reveal -> full -> scanning；
        // - “何时取消”由上层逻辑决定（目标退出 Running 时会 remove animation），这里不在纯函数里处理。

        let path_len = 10;
        let reveal_step_ms = 10;
        let head_step_ms = 50;
        let head_len = 3;

        // 0ms：还没开始 reveal
        let plan = radar::plan_hat_graph_radar_edge_animation(
            Duration::from_millis(0),
            path_len,
            reveal_step_ms,
            head_step_ms,
            head_len,
        );
        assert_eq!(plan.base_steps, 0);
        assert_eq!(plan.head_start, None);

        // 25ms：reveal 了 2 个 cell，head 贴着前沿（不环绕）
        let plan = radar::plan_hat_graph_radar_edge_animation(
            Duration::from_millis(25),
            path_len,
            reveal_step_ms,
            head_step_ms,
            head_len,
        );
        assert_eq!(plan.base_steps, 2);
        assert_eq!(plan.head_start, Some(0));
        assert_eq!(plan.head_len, 2);
        assert!(!plan.head_wrap);

        // 100ms：reveal 刚好完成（10*10ms），进入扫描态：base 全亮，head 从起点开始跑
        let plan = radar::plan_hat_graph_radar_edge_animation(
            Duration::from_millis(100),
            path_len,
            reveal_step_ms,
            head_step_ms,
            head_len,
        );
        assert_eq!(plan.base_steps, path_len);
        assert_eq!(plan.head_start, Some(0));
        assert_eq!(plan.head_len, head_len);
        assert!(plan.head_wrap);

        // 150ms：after_reveal=50ms，head 前进 1 格
        let plan = radar::plan_hat_graph_radar_edge_animation(
            Duration::from_millis(150),
            path_len,
            reveal_step_ms,
            head_step_ms,
            head_len,
        );
        assert_eq!(plan.base_steps, path_len);
        assert_eq!(plan.head_start, Some(1));
        assert_eq!(plan.head_len, head_len);
        assert!(plan.head_wrap);
    }

    #[test]
    fn custom_hat_topics_update_pending_hat() {
        // Test that custom hat topics (not hardcoded) update pending_hat correctly
        use std::collections::HashMap;

        // Create a hat map for custom hats
        let mut hat_map = HashMap::new();
        hat_map.insert(
            "review.security".to_string(),
            (
                HatId::new("security_reviewer"),
                "🔒 Security Reviewer".to_string(),
            ),
        );
        hat_map.insert(
            "review.correctness".to_string(),
            (
                HatId::new("correctness_reviewer"),
                "🎯 Correctness Reviewer".to_string(),
            ),
        );

        let mut state = TuiState::with_hat_map(hat_map);

        // Publish review.security event
        let event = Event::new("review.security", "Review PR #123");
        state.update(&event);

        // Should update pending_hat to security reviewer
        assert_eq!(
            state.get_pending_hat_display(),
            "🔒 Security Reviewer",
            "Should display security reviewer hat for review.security topic"
        );

        // Publish review.correctness event
        let event = Event::new("review.correctness", "Check logic");
        state.update(&event);

        // Should update to correctness reviewer
        assert_eq!(
            state.get_pending_hat_display(),
            "🎯 Correctness Reviewer",
            "Should display correctness reviewer hat for review.correctness topic"
        );
    }

    #[test]
    fn unknown_topics_keep_pending_hat_unchanged() {
        // Test that unknown topics don't clear pending_hat
        let mut state = TuiState::new();

        // Set initial hat
        state.pending_hat = Some((HatId::new("planner"), "📋Planner".to_string()));

        // Publish unknown event
        let event = Event::new("unknown.topic", "Some payload");
        state.update(&event);

        // Should keep the planner hat
        assert_eq!(
            state.get_pending_hat_display(),
            "📋Planner",
            "Unknown topics should not clear pending_hat"
        );
    }

    #[test]
    fn loop_terminate_freezes_iteration_timer() {
        // Given a running iteration with elapsed time
        let mut state = TuiState::new();
        let start_event = Event::new("build.task", "");
        state.update(&start_event);

        // Verify timer is running
        assert!(state.iteration_started.is_some());
        let elapsed_before = state.get_iteration_elapsed().unwrap();
        assert!(elapsed_before.as_nanos() > 0);

        // When loop.terminate is received
        let terminate_event = Event::new("loop.terminate", "");
        state.update(&terminate_event);

        // Then the timer is frozen
        assert!(state.loop_completed);
        assert!(state.final_iteration_elapsed.is_some());

        // The elapsed time should be frozen (not increasing)
        let frozen_elapsed = state.get_iteration_elapsed().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let elapsed_after_sleep = state.get_iteration_elapsed().unwrap();

        assert_eq!(
            frozen_elapsed, elapsed_after_sleep,
            "Timer should be frozen after loop.terminate"
        );
    }

    // ========================================================================
    // TuiState Iteration Management Tests
    // ========================================================================

    mod tui_state_iterations {
        use super::*;

        #[test]
        fn start_new_iteration_creates_first_buffer() {
            // Given TuiState with 0 iterations
            let mut state = TuiState::new();
            assert_eq!(state.total_iterations(), 0);

            // When start_new_iteration() is called
            state.start_new_iteration();

            // Then iterations.len() == 1 and new IterationBuffer exists
            assert_eq!(state.total_iterations(), 1);
            assert_eq!(state.output.iterations[0].number, 1);
        }

        #[test]
        fn start_new_iteration_creates_subsequent_buffers() {
            let mut state = TuiState::new();
            state.start_new_iteration();
            state.start_new_iteration();
            state.start_new_iteration();

            assert_eq!(state.total_iterations(), 3);
            assert_eq!(state.output.iterations[0].number, 1);
            assert_eq!(state.output.iterations[1].number, 2);
            assert_eq!(state.output.iterations[2].number, 3);
        }

        #[test]
        fn current_iteration_returns_correct_buffer() {
            // Given TuiState with 3 iterations and current_view = 1
            let mut state = TuiState::new();
            state.start_new_iteration();
            state.start_new_iteration();
            state.start_new_iteration();
            state.output.current_view = 1;

            // When current_iteration() is called
            let current = state.current_iteration();

            // Then the buffer at index 1 is returned (iteration number 2)
            assert!(current.is_some());
            assert_eq!(current.unwrap().number, 2);
        }

        #[test]
        fn current_iteration_returns_none_when_empty() {
            let state = TuiState::new();
            assert!(state.current_iteration().is_none());
        }

        #[test]
        fn current_iteration_mut_allows_modification() {
            let mut state = TuiState::new();
            state.start_new_iteration();

            // Add a line via mutable reference
            if let Some(buffer) = state.current_iteration_mut() {
                buffer.append_line(Line::from("test line"));
            }

            // Verify modification persisted
            assert_eq!(state.current_iteration().unwrap().line_count(), 1);
        }

        #[test]
        fn navigate_next_increases_current_view() {
            // Given TuiState with current_view = 1 and 3 iterations
            let mut state = TuiState::new();
            state.start_new_iteration();
            state.start_new_iteration();
            state.start_new_iteration();
            state.output.current_view = 1;
            state.output.following_latest = false;

            // When navigate_next() is called
            state.navigate_next();

            // Then current_view == 2
            assert_eq!(state.output.current_view, 2);
        }

        #[test]
        fn navigate_prev_decreases_current_view() {
            // Given TuiState with current_view = 2
            let mut state = TuiState::new();
            state.start_new_iteration();
            state.start_new_iteration();
            state.start_new_iteration();
            state.output.current_view = 2;

            // When navigate_prev() is called
            state.navigate_prev();

            // Then current_view == 1
            assert_eq!(state.output.current_view, 1);
        }

        #[test]
        fn navigate_next_does_not_exceed_bounds() {
            // Given TuiState with current_view = 2 and 3 iterations (max index 2)
            let mut state = TuiState::new();
            state.start_new_iteration();
            state.start_new_iteration();
            state.start_new_iteration();
            state.output.current_view = 2;

            // When navigate_next() is called
            state.navigate_next();

            // Then current_view stays at 2
            assert_eq!(state.output.current_view, 2);
        }

        #[test]
        fn navigate_prev_does_not_go_below_zero() {
            // Given TuiState with current_view = 0
            let mut state = TuiState::new();
            state.start_new_iteration();
            state.output.current_view = 0;

            // When navigate_prev() is called
            state.navigate_prev();

            // Then current_view stays at 0
            assert_eq!(state.output.current_view, 0);
        }

        #[test]
        fn following_latest_initially_true() {
            // Given new TuiState
            // When created
            let state = TuiState::new();

            // Then following_latest == true
            assert!(state.output.following_latest);
        }

        #[test]
        fn following_latest_becomes_false_on_back_navigation() {
            // Given TuiState with following_latest = true and current_view = 2
            let mut state = TuiState::new();
            state.start_new_iteration();
            state.start_new_iteration();
            state.start_new_iteration();
            state.output.current_view = 2;
            state.output.following_latest = true;

            // When navigate_prev() is called
            state.navigate_prev();

            // Then following_latest == false
            assert!(!state.output.following_latest);
        }

        #[test]
        fn following_latest_restored_at_latest() {
            // Given TuiState with following_latest = false
            let mut state = TuiState::new();
            state.start_new_iteration();
            state.start_new_iteration();
            state.start_new_iteration();
            state.output.current_view = 1;
            state.output.following_latest = false;

            // When navigate_next() reaches the last iteration
            state.navigate_next(); // 1 -> 2 (last)

            // Then following_latest == true
            assert!(state.output.following_latest);
        }

        #[test]
        fn total_iterations_reports_count() {
            // Given TuiState with 3 iterations
            let mut state = TuiState::new();
            state.start_new_iteration();
            state.start_new_iteration();
            state.start_new_iteration();

            // When total_iterations() is called
            // Then 3 is returned
            assert_eq!(state.total_iterations(), 3);
        }

        #[test]
        fn start_new_iteration_auto_follows_latest() {
            let mut state = TuiState::new();
            state.output.following_latest = true;
            state.start_new_iteration();
            state.start_new_iteration();

            // When following latest, current_view should track new iterations
            assert_eq!(state.output.current_view, 1); // Index of second iteration
        }

        // ========================================================================
        // Per-Iteration Scroll Independence Tests (Task 08)
        // ========================================================================

        #[test]
        fn per_iteration_scroll_independence() {
            // Given iteration 1 with scroll_offset 5 and iteration 2 with scroll_offset 0
            let mut state = TuiState::new();
            state.start_new_iteration();
            state.start_new_iteration();

            // Set different scroll offsets for each iteration
            state.output.iterations[0].scroll_offset = 5;
            state.output.iterations[1].scroll_offset = 0;

            // When switching between iterations
            state.output.current_view = 0;
            assert_eq!(
                state.current_iteration().unwrap().scroll_offset,
                5,
                "iteration 1 should have scroll_offset 5"
            );

            state.navigate_next();
            assert_eq!(
                state.current_iteration().unwrap().scroll_offset,
                0,
                "iteration 2 should have scroll_offset 0"
            );

            // Then each iteration's scroll_offset is preserved
            state.navigate_prev();
            assert_eq!(
                state.current_iteration().unwrap().scroll_offset,
                5,
                "iteration 1 should still have scroll_offset 5 after switching back"
            );
        }

        #[test]
        fn scroll_within_iteration_does_not_affect_others() {
            // Given multiple iterations with different scroll offsets
            let mut state = TuiState::new();
            state.start_new_iteration();
            state.start_new_iteration();
            state.start_new_iteration();

            // Add content to each iteration
            for i in 0..3 {
                for j in 0..20 {
                    state.output.iterations[i].append_line(Line::from(format!(
                        "iter {} line {}",
                        i + 1,
                        j
                    )));
                }
            }

            // Set initial scroll offsets
            state.output.iterations[0].scroll_offset = 3;
            state.output.iterations[1].scroll_offset = 7;
            state.output.iterations[2].scroll_offset = 10;

            // When scrolling in iteration 2
            state.output.current_view = 1;
            state.current_iteration_mut().unwrap().scroll_down(10);

            // Then only iteration 2's scroll changed
            assert_eq!(
                state.output.iterations[0].scroll_offset, 3,
                "iteration 1 unchanged"
            );
            assert_eq!(
                state.output.iterations[1].scroll_offset, 8,
                "iteration 2 scrolled down"
            );
            assert_eq!(
                state.output.iterations[2].scroll_offset, 10,
                "iteration 3 unchanged"
            );
        }

        // ========================================================================
        // New Iteration Alert Tests (Task 07)
        // ========================================================================

        #[test]
        fn new_iteration_alert_set_when_not_following() {
            // Given following_latest = false and new iteration arrives
            let mut state = TuiState::new();
            state.start_new_iteration(); // Iteration 1
            state.start_new_iteration(); // Iteration 2
            state.navigate_prev(); // Go back to iteration 1, following_latest = false

            // When start_new_iteration() is called
            state.start_new_iteration(); // Iteration 3

            // Then new_iteration_alert is set to the new iteration number
            assert_eq!(state.output.new_iteration_alert, Some(3));
        }

        #[test]
        fn new_iteration_alert_not_set_when_following() {
            // Given following_latest = true
            let mut state = TuiState::new();
            state.output.following_latest = true;
            state.start_new_iteration();

            // When start_new_iteration() is called
            state.start_new_iteration();

            // Then new_iteration_alert remains None
            assert_eq!(state.output.new_iteration_alert, None);
        }

        #[test]
        fn alert_cleared_when_following_restored() {
            // Given new_iteration_alert = Some(5)
            let mut state = TuiState::new();
            state.start_new_iteration();
            state.start_new_iteration();
            state.start_new_iteration();
            state.output.current_view = 0;
            state.output.following_latest = false;
            state.output.new_iteration_alert = Some(3);

            // When navigation restores following_latest = true
            state.navigate_next(); // 0 -> 1
            state.navigate_next(); // 1 -> 2 (last, restores following)

            // Then new_iteration_alert is cleared to None
            assert_eq!(state.output.new_iteration_alert, None);
        }

        #[test]
        fn alert_not_cleared_on_partial_navigation() {
            // Given new_iteration_alert = Some(3) and not at last iteration
            let mut state = TuiState::new();
            state.start_new_iteration();
            state.start_new_iteration();
            state.start_new_iteration();
            state.output.current_view = 0;
            state.output.following_latest = false;
            state.output.new_iteration_alert = Some(3);

            // When navigate_next() but not reaching last
            state.navigate_next(); // 0 -> 1

            // Then alert is still set (not at latest yet)
            assert_eq!(state.output.new_iteration_alert, Some(3));
            assert!(!state.output.following_latest);
        }

        #[test]
        fn alert_updates_for_multiple_new_iterations() {
            // Given not following and multiple new iterations arrive
            let mut state = TuiState::new();
            state.start_new_iteration(); // 1
            state.start_new_iteration(); // 2
            state.navigate_prev(); // Go back, stop following

            state.start_new_iteration(); // 3 arrives
            assert_eq!(state.output.new_iteration_alert, Some(3));

            // When another iteration arrives
            state.start_new_iteration(); // 4 arrives

            // Then alert should show the newest
            assert_eq!(state.output.new_iteration_alert, Some(4));
        }
    }

    // ========================================================================
    // SearchState Tests (Task 09)
    // ========================================================================

    mod search_state {
        use super::*;

        #[test]
        fn search_finds_matches_in_lines() {
            // Given current iteration with "error" in 3 lines
            let mut state = TuiState::new();
            state.start_new_iteration();
            let buffer = state.current_iteration_mut().unwrap();
            buffer.append_line(Line::from("First error occurred"));
            buffer.append_line(Line::from("Normal line"));
            buffer.append_line(Line::from("Another error here"));
            buffer.append_line(Line::from("Final error message"));

            // When search("error") is called
            state.search("error");

            // Then matches.len() >= 3
            assert!(
                state.search.matches.len() >= 3,
                "expected at least 3 matches, got {}",
                state.search.matches.len()
            );
            assert_eq!(state.search.query, Some("error".to_string()));
        }

        #[test]
        fn search_is_case_insensitive() {
            // Given current iteration with "Error" and "error"
            let mut state = TuiState::new();
            state.start_new_iteration();
            let buffer = state.current_iteration_mut().unwrap();
            buffer.append_line(Line::from("Error in uppercase"));
            buffer.append_line(Line::from("error in lowercase"));
            buffer.append_line(Line::from("ERROR all caps"));

            // When search("error") is called
            state.search("error");

            // Then all 3 are found
            assert_eq!(
                state.search.matches.len(),
                3,
                "expected 3 case-insensitive matches"
            );
        }

        #[test]
        fn next_match_cycles_forward() {
            // Given 3 matches and current_match = 2 (last)
            let mut state = TuiState::new();
            state.start_new_iteration();
            let buffer = state.current_iteration_mut().unwrap();
            buffer.append_line(Line::from("match one"));
            buffer.append_line(Line::from("match two"));
            buffer.append_line(Line::from("match three"));
            state.search("match");
            state.search.current_match = 2;

            // When next_match() is called
            state.next_match();

            // Then current_match becomes 0 (cycles back)
            assert_eq!(state.search.current_match, 0);
        }

        #[test]
        fn prev_match_cycles_backward() {
            // Given 3 matches and current_match = 0 (first)
            let mut state = TuiState::new();
            state.start_new_iteration();
            let buffer = state.current_iteration_mut().unwrap();
            buffer.append_line(Line::from("match one"));
            buffer.append_line(Line::from("match two"));
            buffer.append_line(Line::from("match three"));
            state.search("match");
            state.search.current_match = 0;

            // When prev_match() is called
            state.prev_match();

            // Then current_match becomes 2 (cycles back)
            assert_eq!(state.search.current_match, 2);
        }

        #[test]
        fn search_jumps_to_match_line() {
            // Given match at line 50
            let mut state = TuiState::new();
            state.start_new_iteration();
            let buffer = state.current_iteration_mut().unwrap();
            for i in 0..60 {
                if i == 50 {
                    buffer.append_line(Line::from("target match here"));
                } else {
                    buffer.append_line(Line::from(format!("line {}", i)));
                }
            }

            // When search finds match at line 50
            state.search("target");

            // Then scroll_offset is updated so line 50 is visible
            let buffer = state.current_iteration().unwrap();
            // With viewport of ~20, scroll should position line 50 in view
            assert!(
                buffer.scroll_offset <= 50,
                "scroll_offset {} should position line 50 in view",
                buffer.scroll_offset
            );
        }

        #[test]
        fn clear_search_resets_state() {
            // Given active search
            let mut state = TuiState::new();
            state.start_new_iteration();
            let buffer = state.current_iteration_mut().unwrap();
            buffer.append_line(Line::from("search term here"));
            state.search("term");
            assert!(state.search.query.is_some());

            // When clear_search() is called
            state.clear_search();

            // Then query = None, matches cleared, search_mode = false
            assert!(state.search.query.is_none());
            assert!(state.search.matches.is_empty());
            assert!(!state.search.search_mode);
        }

        #[test]
        fn search_with_no_matches_sets_empty() {
            // Given iteration with no matching content
            let mut state = TuiState::new();
            state.start_new_iteration();
            let buffer = state.current_iteration_mut().unwrap();
            buffer.append_line(Line::from("hello world"));

            // When searching for non-existent term
            state.search("xyz");

            // Then matches is empty but query is set
            assert_eq!(state.search.query, Some("xyz".to_string()));
            assert!(state.search.matches.is_empty());
            assert_eq!(state.search.current_match, 0);
        }

        #[test]
        fn search_on_empty_iteration_handles_gracefully() {
            // Given empty iteration
            let mut state = TuiState::new();
            state.start_new_iteration();

            // When searching
            state.search("anything");

            // Then no panic, empty matches
            assert!(state.search.matches.is_empty());
        }

        #[test]
        fn next_match_with_no_matches_does_nothing() {
            // Given no active search or empty matches
            let mut state = TuiState::new();
            state.start_new_iteration();

            // When next_match is called
            state.next_match();

            // Then no panic, current_match stays 0
            assert_eq!(state.search.current_match, 0);
        }

        #[test]
        fn multiple_matches_on_same_line() {
            // Given line with multiple occurrences
            let mut state = TuiState::new();
            state.start_new_iteration();
            let buffer = state.current_iteration_mut().unwrap();
            buffer.append_line(Line::from("error error error"));

            // When searching
            state.search("error");

            // Then finds all 3 matches
            assert_eq!(
                state.search.matches.len(),
                3,
                "should find 3 matches on same line"
            );
        }

        #[test]
        fn next_match_updates_scroll_to_show_match() {
            // Given many lines with matches spread out
            let mut state = TuiState::new();
            state.start_new_iteration();
            let buffer = state.current_iteration_mut().unwrap();
            for i in 0..100 {
                if i % 30 == 0 {
                    buffer.append_line(Line::from("findme"));
                } else {
                    buffer.append_line(Line::from(format!("line {}", i)));
                }
            }
            state.search("findme");

            // Navigate to second match (at line 30)
            state.next_match();

            // Then scroll should position line 30 in view
            let buffer = state.current_iteration().unwrap();
            // Match at line 30, scroll should be adjusted
            assert!(buffer.scroll_offset <= 30, "scroll should show line 30");
        }

        #[test]
        fn latest_iteration_lines_handle_returns_newest_iteration() {
            // Given a user viewing iteration 1 while iteration 3 is executing
            let mut state = TuiState::new();
            state.start_new_iteration(); // iteration 1
            state.start_new_iteration(); // iteration 2
            state.start_new_iteration(); // iteration 3

            // User navigates back to iteration 1
            state.output.current_view = 0;
            state.output.following_latest = false;

            // When getting line handles
            let current_handle = state.current_iteration_lines_handle();
            let latest_handle = state.latest_iteration_lines_handle();

            // Then current_iteration_lines_handle returns iteration 1's buffer
            assert!(current_handle.is_some());
            // And latest_iteration_lines_handle returns iteration 3's buffer
            assert!(latest_handle.is_some());

            // Write to latest and verify it doesn't affect current view
            {
                let latest = latest_handle.unwrap();
                latest
                    .lock()
                    .unwrap()
                    .push(Line::from("output from iteration 3"));
            }

            // Current view (iteration 1) should be empty
            let current = state.current_iteration().unwrap();
            assert_eq!(
                current.lines.lock().unwrap().len(),
                0,
                "iteration 1 should have no lines"
            );

            // Latest (iteration 3) should have the output
            let latest_buffer = state.output.iterations.last().unwrap();
            assert_eq!(
                latest_buffer.lines.lock().unwrap().len(),
                1,
                "iteration 3 should have the output"
            );
        }

        #[test]
        fn output_goes_to_correct_iteration_when_user_reviewing_history() {
            // This reproduces the bug: user is on page 3 of 6, but active agent writes to page 3
            let mut state = TuiState::new();

            // Create 6 iterations
            for _ in 0..6 {
                state.start_new_iteration();
            }

            // User navigates to iteration 3 (index 2)
            state.output.current_view = 2;
            state.output.following_latest = false;

            // New iteration starts (iteration 7)
            state.start_new_iteration();

            // Get handle for writing output - MUST use latest, not current
            let lines_handle = state.latest_iteration_lines_handle();

            // Write output
            {
                let handle = lines_handle.unwrap();
                handle
                    .lock()
                    .unwrap()
                    .push(Line::from("iteration 7 output"));
            }

            // Verify: iteration 3 (what user is viewing) should be unaffected
            let iteration_3 = &state.output.iterations[2];
            assert_eq!(
                iteration_3.lines.lock().unwrap().len(),
                0,
                "iteration 3 (being viewed) should have no output"
            );

            // Verify: iteration 7 (latest) should have the output
            let iteration_7 = state.output.iterations.last().unwrap();
            assert_eq!(
                iteration_7.lines.lock().unwrap().len(),
                1,
                "iteration 7 (latest) should have the output"
            );
        }
    }
}
