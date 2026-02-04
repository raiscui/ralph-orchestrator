//! State management for the TUI.

use ralph_core::HatJobOutputChunk;
use ralph_proto::{Event, HatId, HatInstanceId, HatInstanceState};
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};

// ============================================================================
// 并行模式（Supervisor TUI）state
// ============================================================================

pub(crate) mod parallel;
use parallel::output::ParallelOutputBuffer;
pub use parallel::{
    ChatEditorState, GateStatus, ParallelFocus, ParallelTuiState, ScreenPos, ScreenSelection,
    TextPos, TextSelection,
};

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

/// Hat Graph Radar 的“坐标点”（以终端 cell 为单位）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HatGraphRadarPoint {
    pub x: u16,
    pub y: u16,
}

/// Hat Graph Radar 的矩形区域（以终端 cell 为单位）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HatGraphRadarRect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

/// Hat Graph Radar 的节点 meta：用于把“某个 hat 的 box”映射到字符画坐标。
#[derive(Debug, Clone)]
pub struct HatGraphRadarNodeMeta {
    /// Mermaid node id（parser identity），例如 `Hat_planner`。
    pub id: String,
    /// 节点展示 label（可能包含 emoji/中文）。
    pub label: String,
    /// 节点 box 的矩形范围（含边框）。
    pub box_rect: HatGraphRadarRect,
}

/// Hat Graph Radar 的边 meta：用于按最新 event 做“逐段点亮”动画。
#[derive(Debug, Clone)]
pub struct HatGraphRadarEdgeMeta {
    pub from: String,
    pub to: String,
    pub label: String,
    /// 有序 path 坐标序列（包含拐点/箭头/box-start marker 等关键格子）。
    pub path: Vec<HatGraphRadarPoint>,
}

/// Hat Graph Radar 的完整 meta（nodes + edges）。
#[derive(Debug, Clone, Default)]
pub struct HatGraphRadarMeta {
    pub nodes: Vec<HatGraphRadarNodeMeta>,
    pub edges: Vec<HatGraphRadarEdgeMeta>,
}

impl HatGraphRadarMeta {
    pub fn find_node(&self, id: &str) -> Option<&HatGraphRadarNodeMeta> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn matching_edges(
        &self,
        from: &str,
        label: &str,
    ) -> impl Iterator<Item = &HatGraphRadarEdgeMeta> {
        self.edges
            .iter()
            .filter(move |e| e.from == from && e.label == label)
    }

    pub fn matching_edges_exact(
        &self,
        from: &str,
        label: &str,
        to: &str,
    ) -> impl Iterator<Item = &HatGraphRadarEdgeMeta> {
        self.edges
            .iter()
            .filter(move |e| e.from == from && e.label == label && e.to == to)
    }
}

// =============================================================================
// Hat Graph Radar：事件线动画（按 Running 目标驱动）
// =============================================================================
//
// 你最新口径（2026-02-03）：
// - 线路需要先做 progressive reveal（从 source → target 逐段点亮）；
// - reveal 完成后，线路应保持“全亮”并持续显示，直到目标 hat 退出 Running（进入 Idle/Done/Failed）；
// - “指向的目标 box 不再 Running”时，必须立刻取消该线路高亮（不要残留）。
//
// 设计取舍：
// - cause event 采用 best-effort 推断：从“最近收到的业务事件”里找一条能够在 hats graph
//   中连到该 target hat 的边（from+topic+to 完全匹配）。
// - 动画本身是纯 UI 行为，不影响 orchestration。

/// Hat Graph Radar 边动画速度：每多少毫秒“点亮一个 cell”。
pub(crate) const HAT_GRAPH_EDGE_ANIMATION_STEP_MS: u64 = 30;
/// progressive reveal 的最大时长：用于把“很长的路径”加速到一个可读的时间窗口内。
const HAT_GRAPH_EDGE_ANIMATION_MAX_REVEAL_MS: u64 = 800;

/// Hat Graph Radar：扫描头（跑动高亮段）的移动速度（每多少毫秒前进一个 cell）。
///
/// 说明：
/// - 这是 reveal 完成后的“锦上添花”动效，目的是让用户一眼看出“这条边仍在生效/仍在运行态”；
/// - 速度不应跟随 reveal 的 step_ms（reveal 会为长路径自动加速，否则扫描会快到看不见）。
pub(crate) const HAT_GRAPH_EDGE_HEAD_STEP_MS: u64 = 60;

/// Hat Graph Radar：扫描头的长度（以 cell 数计）。
pub(crate) const HAT_GRAPH_EDGE_HEAD_LEN: usize = 16;

/// 推断“cause event”的回看窗口：只在这个时间范围内找最近事件（避免匹配到过旧的事件）。
const HAT_GRAPH_CAUSE_LOOKBACK: Duration = Duration::from_secs(10);

/// 保存最近事件的上限（按条数），避免无限增长。
const HAT_GRAPH_RECENT_EVENT_MAX: usize = 64;

/// Radar 侧用于推断 “cause event” 的最近事件记录（只存必要信息）。
#[derive(Debug, Clone)]
pub struct HatGraphRadarRecentEvent {
    pub source_hat: HatId,
    pub topic: String,
    pub observed_at: Instant,
}

/// 某个 target hat 当前正在播放的“cause event 边动画”。
#[derive(Debug, Clone)]
pub struct HatGraphRadarEdgeAnimation {
    pub target_hat: HatId,
    pub source_hat: HatId,
    pub topic: String,
    pub started_at: Instant,
    pub step_ms: u64,
}

/// Radar 边动画在“当前帧”应如何渲染（纯渲染计划，可单测）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HatGraphRadarEdgeRenderPlan {
    /// 用 base 色从 path[0..base_steps] 做高亮（reveal 阶段为部分，reveal 后为全量）。
    pub base_steps: usize,
    /// 扫描头的起点（沿 path 的索引）。
    pub head_start: Option<usize>,
    /// 扫描头的长度（以 cell 数计）。
    pub head_len: usize,
    /// 扫描头是否允许环绕（reveal 完成后为 true）。
    pub head_wrap: bool,
}

/// 计算 Radar 边动画的渲染计划。
///
/// 规则：
/// - reveal 阶段：base 只亮到当前进度；head 贴着 reveal 前沿（更亮、更醒目）
/// - reveal 完成后：base 全亮；head 以固定速度循环移动（直到目标 hat 退出 Running 才会被上层清理）
pub(crate) fn plan_hat_graph_radar_edge_animation(
    elapsed: Duration,
    path_len: usize,
    reveal_step_ms: u64,
    head_step_ms: u64,
    head_len: usize,
) -> HatGraphRadarEdgeRenderPlan {
    if path_len == 0 {
        return HatGraphRadarEdgeRenderPlan {
            base_steps: 0,
            head_start: None,
            head_len: 0,
            head_wrap: false,
        };
    }

    let elapsed_ms = elapsed.as_millis();
    let reveal_step_ms = reveal_step_ms.max(1);
    let head_step_ms = head_step_ms.max(1);

    let total_steps = (elapsed_ms / u128::from(reveal_step_ms)) as usize;
    let revealed = total_steps.min(path_len);

    // reveal 阶段：head 贴着前沿，不环绕
    if revealed < path_len {
        if revealed == 0 {
            return HatGraphRadarEdgeRenderPlan {
                base_steps: 0,
                head_start: None,
                head_len: 0,
                head_wrap: false,
            };
        }

        let head_len = head_len.min(revealed);
        let head_start = revealed.saturating_sub(head_len);
        return HatGraphRadarEdgeRenderPlan {
            base_steps: revealed,
            head_start: Some(head_start),
            head_len,
            head_wrap: false,
        };
    }

    // reveal 完成：base 全亮；head 循环扫描
    let reveal_total_ms =
        u128::from(reveal_step_ms).saturating_mul(u128::try_from(path_len).unwrap_or(u128::MAX));
    let after_reveal_ms = elapsed_ms.saturating_sub(reveal_total_ms);
    let head_ticks = (after_reveal_ms / u128::from(head_step_ms)) as usize;
    let head_start = head_ticks % path_len;
    let head_len = head_len.min(path_len);

    HatGraphRadarEdgeRenderPlan {
        base_steps: path_len,
        head_start: Some(head_start),
        head_len,
        head_wrap: true,
    }
}

fn sanitize_mermaid_identifier(raw: &str) -> String {
    // 说明：
    // - Radar 的 meta 里边/节点引用的是 Mermaid “节点 ID”（例如 Hat_builder）；
    // - 该规则必须与 `ralph-cli` / `ralph-tui::app.rs` 的生成逻辑一致，否则匹配不到边。
    //
    // 规则：保守地只允许 ASCII [A-Za-z0-9_]，其余字符全部移除。
    let mut out = String::new();
    for c in raw.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            out.push(c);
        }
    }

    if out.is_empty() {
        "hat".to_string()
    } else {
        out
    }
}

fn mermaid_hat_node_id(hat_id: &str) -> String {
    // 与 `crates/ralph-cli/src/hats.rs#mermaid_hat_node_id` / `crates/ralph-tui/src/app.rs` 保持一致：
    // - 加前缀避免与 Start/Complete 等节点名冲突；
    // - 避免 hat_id 以数字开头触发 Mermaid 标识符解析歧义。
    format!("Hat_{}", sanitize_mermaid_identifier(hat_id))
}

#[derive(Debug, Clone)]
pub struct HatGraphRadar {
    /// 小窗（雷达）展示：更紧凑的 ASCII 图（通常 padding=0）。
    pub ascii_compact: String,
    /// 大窗（放大）展示：更可读的 ASCII 图（通常默认 padding）。
    pub ascii_full: String,
    /// compact 视图的 meta（可选：渲染器不支持/注入失败时允许降级为无高亮/无动画）。
    pub meta_compact: Option<HatGraphRadarMeta>,
    /// full 视图的 meta（可选：同上）。
    pub meta_full: Option<HatGraphRadarMeta>,
}

/// TUI 的状态更新事件（用于 observer → channel → reducer）。
#[derive(Debug, Clone)]
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

// ============================================================================
// TaskSummary - Summary of a single task for TUI display
// ============================================================================

/// Summary of a task for TUI display.
/// Contains only the fields needed for rendering.
#[derive(Debug, Clone, Default)]
pub struct TaskSummary {
    /// Task identifier (e.g., "task-1737372000-a1b2").
    pub id: String,
    /// Task title/description.
    pub title: String,
    /// Task status (e.g., "open", "closed", "blocked").
    pub status: String,
}

impl TaskSummary {
    /// Creates a new task summary.
    pub fn new(id: impl Into<String>, title: impl Into<String>, status: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            status: status.into(),
        }
    }
}

// ============================================================================
// TaskCounts - Aggregate task statistics for TUI display
// ============================================================================

/// Aggregate task statistics for TUI display.
#[derive(Debug, Clone, Default)]
pub struct TaskCounts {
    /// Total number of tasks.
    pub total: usize,
    /// Number of open tasks.
    pub open: usize,
    /// Number of closed tasks.
    pub closed: usize,
    /// Number of ready (unblocked) tasks.
    pub ready: usize,
}

impl TaskCounts {
    /// Creates new task counts.
    pub fn new(total: usize, open: usize, closed: usize, ready: usize) -> Self {
        Self {
            total,
            open,
            closed,
            ready,
        }
    }
}

// ============================================================================
// SearchState - Search functionality for TUI content
// ============================================================================

/// Search state for finding and navigating matches in TUI content.
/// Tracks the current query, match positions, and navigation index.
#[derive(Debug, Default)]
pub struct SearchState {
    /// Current search query (None when no active search).
    pub query: Option<String>,
    /// Match positions as (line_index, char_offset) pairs.
    pub matches: Vec<(usize, usize)>,
    /// Index into matches vector for current match.
    pub current_match: usize,
    /// Whether search input mode is active (user is typing query).
    pub search_mode: bool,
}

impl SearchState {
    /// Creates a new empty search state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Clears all search state.
    pub fn clear(&mut self) {
        self.query = None;
        self.matches.clear();
        self.current_match = 0;
        self.search_mode = false;
    }
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
    /// Current search query (if in search input mode).
    pub search_query: String,
    /// Search direction (true = forward, false = backward).
    pub search_forward: bool,
    /// Maximum iterations from config.
    pub max_iterations: Option<u32>,
    /// Idle timeout countdown.
    pub idle_timeout_remaining: Option<Duration>,
    /// Map of event topics to hat display information (for custom hats).
    /// Key: event topic (e.g., "review.security")
    /// Value: (HatId, display name including emoji)
    hat_map: HashMap<String, (HatId, String)>,

    // ========================================================================
    // Iteration Management (new fields for TUI refactor)
    // ========================================================================
    /// Content buffers for each iteration.
    pub iterations: Vec<IterationBuffer>,
    /// Index of the iteration currently being viewed (0-indexed).
    pub current_view: usize,
    /// Whether to automatically follow the latest iteration.
    pub following_latest: bool,
    /// Alert about a new iteration (shown when viewing history and new iteration arrives).
    /// Contains the iteration number to alert about. Cleared when navigating to latest.
    pub new_iteration_alert: Option<usize>,

    // ========================================================================
    // Search State
    // ========================================================================
    /// Search state for finding and navigating matches in iteration content.
    pub search_state: SearchState,

    // ========================================================================
    // Completion State
    // ========================================================================
    /// Whether the loop has completed (received loop.terminate event).
    pub loop_completed: bool,
    /// Frozen elapsed time when loop completed (timer stops at this value).
    pub final_iteration_elapsed: Option<Duration>,

    // ========================================================================
    // Task Tracking State
    // ========================================================================
    /// Aggregate task counts for display in TUI widgets.
    pub task_counts: TaskCounts,
    /// Currently active task (if any) for display in TUI widgets.
    pub active_task: Option<TaskSummary>,

    // ========================================================================
    // Hat Graph Radar (Top-right overlay)
    // ========================================================================
    /// 右上角 hats 拓扑雷达图（若 CLI 注入则显示）。
    pub hat_graph_radar: Option<HatGraphRadar>,
    /// 是否处于“放大”视图（按键 `p` 切换）。
    pub hat_graph_zoomed: bool,
    /// Radar 的“最近业务事件”（用于推断某个 Running hat 的 cause event）。
    pub hat_graph_recent_events: VecDeque<HatGraphRadarRecentEvent>,
    /// Radar 的“按 Running 目标驱动”的边动画（target_hat -> animation）。
    pub hat_graph_edge_animations: HashMap<HatId, HatGraphRadarEdgeAnimation>,

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
            search_query: String::new(),
            search_forward: true,
            max_iterations: None,
            idle_timeout_remaining: None,
            hat_map: HashMap::new(),
            // Iteration management
            iterations: Vec::new(),
            current_view: 0,
            following_latest: true,
            new_iteration_alert: None,
            // Search state
            search_state: SearchState::new(),
            // Completion state
            loop_completed: false,
            final_iteration_elapsed: None,
            // Task tracking state
            task_counts: TaskCounts::default(),
            active_task: None,
            // Hat graph radar
            hat_graph_radar: None,
            hat_graph_zoomed: false,
            hat_graph_recent_events: VecDeque::new(),
            hat_graph_edge_animations: HashMap::new(),
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
            search_query: String::new(),
            search_forward: true,
            max_iterations: None,
            idle_timeout_remaining: None,
            hat_map,
            // Iteration management
            iterations: Vec::new(),
            current_view: 0,
            following_latest: true,
            new_iteration_alert: None,
            // Search state
            search_state: SearchState::new(),
            // Completion state
            loop_completed: false,
            final_iteration_elapsed: None,
            // Task tracking state
            task_counts: TaskCounts::default(),
            active_task: None,
            // Hat graph radar
            hat_graph_radar: None,
            hat_graph_zoomed: false,
            hat_graph_recent_events: VecDeque::new(),
            hat_graph_edge_animations: HashMap::new(),
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
                        self.hat_graph_edge_animations.remove(&HatId::new(hat_id));
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
        // 说明：
        // - 你明确指出 event 线路动画应是“因果可视化”，不是 gate/human 这类控制面噪音；
        // - 因此这里只记录“业务事件”，并且必须能推导出发布者 hat（source/source_instance）。
        let topic = event.topic.as_str();
        if topic.starts_with("gate.") || topic == "human.message" {
            return;
        }

        let source_hat = if let Some(source_hat) = event.source.clone() {
            source_hat
        } else if let Some(source_instance) = event.source_instance.as_ref()
            && let Some(hat_id) = source_instance.split_hat_id()
        {
            HatId::new(hat_id)
        } else {
            return;
        };

        self.hat_graph_recent_events
            .push_back(HatGraphRadarRecentEvent {
                source_hat,
                topic: topic.to_string(),
                observed_at: now,
            });

        // 容量上限：按条数裁剪（保证常数级内存）。
        while self.hat_graph_recent_events.len() > HAT_GRAPH_RECENT_EVENT_MAX {
            let _ = self.hat_graph_recent_events.pop_front();
        }
    }

    fn maybe_start_hat_graph_edge_animation_for_running_hat(
        &mut self,
        target_hat: HatId,
        now: Instant,
    ) {
        // 说明：
        // - 只有 Radar + meta 存在时，才有条件做“因果边动画”；
        // - 这里使用 meta 做“结构匹配”，避免靠字符串/ANSI 解析导致脆弱。
        let Some(radar) = self.hat_graph_radar.as_ref() else {
            return;
        };
        let Some(meta) = radar.meta_full.as_ref().or(radar.meta_compact.as_ref()) else {
            return;
        };

        // 目标节点：Hat_{id}
        let target_node_id = mermaid_hat_node_id(target_hat.as_str());

        // 从最近事件里倒序找：谁能在图上连到 target（from+topic+to 完全匹配）。
        let mut cause: Option<(HatId, String)> = None;
        for e in self.hat_graph_recent_events.iter().rev() {
            if now.saturating_duration_since(e.observed_at) > HAT_GRAPH_CAUSE_LOOKBACK {
                break;
            }

            let from_node_id = mermaid_hat_node_id(e.source_hat.as_str());
            let topic = e.topic.as_str();
            let matches = meta.edges.iter().any(|edge| {
                edge.from == from_node_id && edge.to == target_node_id && edge.label == topic
            });
            if matches {
                cause = Some((e.source_hat.clone(), e.topic.clone()));
                break;
            }
        }

        let Some((source_hat, topic)) = cause else {
            return;
        };

        // 计算 step_ms：
        // - 默认 `HAT_GRAPH_EDGE_ANIMATION_STEP_MS`（30ms / cell）
        // - 如果路径很长，则加速（缩小 step_ms），让 reveal 在一个合理窗口内完成
        let from_node_id = mermaid_hat_node_id(source_hat.as_str());
        let max_len = meta
            .matching_edges_exact(&from_node_id, topic.as_str(), &target_node_id)
            .map(|edge| edge.path.len())
            .max()
            .unwrap_or(0);

        let step_ms = if max_len == 0 {
            HAT_GRAPH_EDGE_ANIMATION_STEP_MS
        } else {
            let adaptive = HAT_GRAPH_EDGE_ANIMATION_MAX_REVEAL_MS / max_len as u64;
            adaptive.clamp(1, HAT_GRAPH_EDGE_ANIMATION_STEP_MS.max(1))
        };

        self.hat_graph_edge_animations.insert(
            target_hat.clone(),
            HatGraphRadarEdgeAnimation {
                target_hat,
                source_hat,
                topic,
                started_at: now,
                step_ms,
            },
        );
    }

    /// 每帧（render tick）推进 Radar 的可视化状态：
    /// - 清理过旧的 recent events（用于 cause 推断）
    /// - 清理无效的边动画（目标不再 Running）
    pub(crate) fn tick_hat_graph_radar_animation(&mut self, now: Instant) {
        // 1) recent events：只保留 lookback 窗口内的（越界的直接丢弃）
        while let Some(front) = self.hat_graph_recent_events.front() {
            if now.saturating_duration_since(front.observed_at) > HAT_GRAPH_CAUSE_LOOKBACK {
                let _ = self.hat_graph_recent_events.pop_front();
            } else {
                break;
            }
        }

        // 2) edge animations：目标不 Running 时移除
        let running_hats: HashSet<String> = if self.mode == TuiMode::Parallel {
            let mut hats = HashSet::new();
            for (instance_id, view) in &self.parallel.instances {
                if view.state != HatInstanceState::Running {
                    continue;
                }
                if let Some(hat_id) = instance_id.split_hat_id() {
                    hats.insert(hat_id.to_string());
                }
            }
            hats
        } else {
            HashSet::new()
        };

        self.hat_graph_edge_animations.retain(|target_hat, _anim| {
            if self.mode == TuiMode::Parallel {
                return running_hats.contains(target_hat.as_str());
            }
            true
        });
    }

    /// Returns formatted hat display (emoji + name).
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
        &self.task_counts
    }

    /// Returns a reference to the active task, if any.
    pub fn get_active_task(&self) -> Option<&TaskSummary> {
        self.active_task.as_ref()
    }

    /// Updates the task counts.
    pub fn set_task_counts(&mut self, counts: TaskCounts) {
        self.task_counts = counts;
    }

    /// Sets the active task.
    pub fn set_active_task(&mut self, task: Option<TaskSummary>) {
        self.active_task = task;
    }

    // ========================================================================
    // Hat Graph Radar Methods
    // ========================================================================

    /// 注入 hats graph radar 的 ASCII 渲染结果（由 CLI 在启动 TUI 时生成）。
    pub fn set_hat_graph_radar(&mut self, radar: HatGraphRadar) {
        self.hat_graph_radar = Some(radar);
    }

    /// Returns true if there are any open tasks.
    pub fn has_open_tasks(&self) -> bool {
        self.task_counts.open > 0
    }

    /// Returns a formatted string for task progress display (e.g., "3/5 tasks").
    pub fn get_task_progress_display(&self) -> String {
        if self.task_counts.total == 0 {
            "No tasks".to_string()
        } else {
            format!(
                "{}/{} tasks",
                self.task_counts.closed, self.task_counts.total
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
        let number = (self.iterations.len() + 1) as u32;
        self.iterations.push(IterationBuffer::new(number));

        // Auto-follow if enabled
        if self.following_latest {
            self.current_view = self.iterations.len().saturating_sub(1);
        } else {
            // Alert user about new iteration when reviewing history
            self.new_iteration_alert = Some(number as usize);
        }
    }

    /// Returns a reference to the currently viewed iteration buffer.
    pub fn current_iteration(&self) -> Option<&IterationBuffer> {
        self.iterations.get(self.current_view)
    }

    /// Returns a mutable reference to the currently viewed iteration buffer.
    pub fn current_iteration_mut(&mut self) -> Option<&mut IterationBuffer> {
        self.iterations.get_mut(self.current_view)
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

    /// Returns a shared handle to the current iteration's lines buffer.
    ///
    /// This allows stream handlers to write directly to the buffer,
    /// enabling real-time streaming to the TUI during execution.
    pub fn current_iteration_lines_handle(
        &self,
    ) -> Option<std::sync::Arc<std::sync::Mutex<Vec<Line<'static>>>>> {
        self.iterations
            .get(self.current_view)
            .map(|buffer| buffer.lines_handle())
    }

    /// Returns a shared handle to the latest iteration's lines buffer.
    ///
    /// This should be used when writing output from the currently executing
    /// iteration, regardless of which iteration the user is viewing.
    /// This prevents output from being written to the wrong iteration when
    /// the user is reviewing an older iteration.
    pub fn latest_iteration_lines_handle(
        &self,
    ) -> Option<std::sync::Arc<std::sync::Mutex<Vec<Line<'static>>>>> {
        self.iterations.last().map(|buffer| buffer.lines_handle())
    }

    /// Navigates to the next iteration (if not at the last one).
    /// If reaching the last iteration, re-enables following_latest and clears alerts.
    pub fn navigate_next(&mut self) {
        if self.iterations.is_empty() {
            return;
        }
        let max_index = self.iterations.len().saturating_sub(1);
        if self.current_view < max_index {
            self.current_view += 1;
            // Re-enable following when reaching the latest
            if self.current_view == max_index {
                self.following_latest = true;
                self.new_iteration_alert = None;
            }
        }
    }

    /// Navigates to the previous iteration (if not at the first one).
    /// Disables following_latest when navigating backwards.
    pub fn navigate_prev(&mut self) {
        if self.current_view > 0 {
            self.current_view -= 1;
            self.following_latest = false;
        }
    }

    /// Returns the total number of iterations.
    pub fn total_iterations(&self) -> usize {
        self.iterations.len()
    }

    // ========================================================================
    // Search Methods
    // ========================================================================

    /// Searches for the given query in the current iteration's content.
    /// Populates matches with (line_index, char_offset) pairs.
    /// Search is case-insensitive.
    pub fn search(&mut self, query: &str) {
        self.search_state.query = Some(query.to_string());
        self.search_state.matches.clear();
        self.search_state.current_match = 0;

        let query_lower = query.to_lowercase();

        // Collect matches first (avoid borrow conflicts)
        let matches: Vec<(usize, usize)> = match self.mode {
            TuiMode::Serial => self
                .iterations
                .get(self.current_view)
                .and_then(|buffer| buffer.lines.lock().ok().map(|lines| lines.clone()))
                .map(|lines| {
                    let mut found = Vec::new();
                    for (line_idx, line) in lines.iter().enumerate() {
                        let line_text: String =
                            line.spans.iter().map(|s| s.content.as_ref()).collect();
                        let line_lower = line_text.to_lowercase();

                        let mut search_start = 0;
                        while let Some(pos) = line_lower[search_start..].find(&query_lower) {
                            let char_offset = search_start + pos;
                            found.push((line_idx, char_offset));
                            search_start = char_offset + query_lower.len();
                        }
                    }
                    found
                })
                .unwrap_or_default(),
            TuiMode::Parallel => {
                let Some(buffer) = self
                    .parallel
                    .selected_instance()
                    .and_then(|i| i.current_job_buffer())
                else {
                    self.search_state.matches = Vec::new();
                    return;
                };

                let mut found = Vec::new();
                for (line_idx, line) in buffer.lines.iter().enumerate() {
                    let line_text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
                    let line_lower = line_text.to_lowercase();

                    let mut search_start = 0;
                    while let Some(pos) = line_lower[search_start..].find(&query_lower) {
                        let char_offset = search_start + pos;
                        found.push((line_idx, char_offset));
                        search_start = char_offset + query_lower.len();
                    }
                }
                found
            }
        };

        self.search_state.matches = matches;

        // Jump to first match if any exist
        if !self.search_state.matches.is_empty() {
            self.jump_to_current_match();
        }
    }

    /// Navigates to the next match, cycling back to the first if at the end.
    pub fn next_match(&mut self) {
        if self.search_state.matches.is_empty() {
            return;
        }

        self.search_state.current_match =
            (self.search_state.current_match + 1) % self.search_state.matches.len();
        self.jump_to_current_match();
    }

    /// Navigates to the previous match, cycling to the last if at the beginning.
    pub fn prev_match(&mut self) {
        if self.search_state.matches.is_empty() {
            return;
        }

        if self.search_state.current_match == 0 {
            self.search_state.current_match = self.search_state.matches.len() - 1;
        } else {
            self.search_state.current_match -= 1;
        }
        self.jump_to_current_match();
    }

    /// Clears the search state.
    pub fn clear_search(&mut self) {
        self.search_state.clear();
    }

    /// Jumps to the current match by adjusting scroll_offset to show the match line.
    fn jump_to_current_match(&mut self) {
        if self.search_state.matches.is_empty() {
            return;
        }

        let (line_idx, _) = self.search_state.matches[self.search_state.current_match];

        // Adjust scroll to show the match line
        // Use a default viewport height for calculation (will be overridden by actual render)
        let viewport_height = 20;
        if let Some(mut buffer) = self.current_output_buffer_mut() {
            // If the match line is above the current view, scroll up to it
            if line_idx < buffer.scroll_offset() {
                buffer.set_scroll_offset_clamped(line_idx);
            }
            // If the match line is below the current view, scroll down to show it
            else if line_idx >= buffer.scroll_offset() + viewport_height {
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

// ============================================================================
// IterationBuffer - Content storage for a single iteration
// ============================================================================

use ratatui::text::Line;
use std::sync::{Arc, Mutex};

/// Stores formatted output content for a single Ralph iteration.
/// Each iteration has its own buffer with independent scroll state.
///
/// The `lines` field is wrapped in `Arc<Mutex<>>` to allow sharing
/// with stream handlers during execution, enabling real-time streaming
/// to the TUI instead of batch transfer after execution completes.
#[derive(Debug)]
pub struct IterationBuffer {
    /// Iteration number (1-indexed for display)
    pub number: u32,
    /// Formatted lines of output (shared for streaming)
    pub lines: Arc<Mutex<Vec<Line<'static>>>>,
    /// Scroll position within this buffer
    pub scroll_offset: usize,
    /// Whether to auto-scroll to bottom as new content arrives.
    /// Starts true, becomes false when user scrolls up, restored when user
    /// scrolls to bottom (G key) or manually scrolls down to reach bottom.
    pub following_bottom: bool,
}

impl IterationBuffer {
    /// Creates a new buffer for the given iteration number.
    pub fn new(number: u32) -> Self {
        Self {
            number,
            lines: Arc::new(Mutex::new(Vec::new())),
            scroll_offset: 0,
            following_bottom: true, // Start following bottom for auto-scroll
        }
    }

    /// Returns a shared handle to the lines buffer for streaming.
    ///
    /// This allows stream handlers to write directly to the buffer,
    /// enabling real-time streaming to the TUI.
    pub fn lines_handle(&self) -> Arc<Mutex<Vec<Line<'static>>>> {
        Arc::clone(&self.lines)
    }

    /// Appends a line to the buffer.
    pub fn append_line(&mut self, line: Line<'static>) {
        if let Ok(mut lines) = self.lines.lock() {
            lines.push(line);
        }
    }

    /// 追加一行，并在超过上限时丢弃最旧的行（近似 ring buffer）。
    ///
    /// 说明：
    /// - 该策略用于“长跑并行输出”，避免内存无限增长。
    /// - 丢弃旧行时，需要同步调整 scroll_offset，避免越界。
    pub fn append_line_capped(&mut self, line: Line<'static>, max_lines: usize) {
        if max_lines == 0 {
            return;
        }

        let Ok(mut lines) = self.lines.lock() else {
            return;
        };

        lines.push(line);
        if lines.len() <= max_lines {
            return;
        }

        let overflow = lines.len().saturating_sub(max_lines);
        if overflow == 0 {
            return;
        }

        // 移除最旧的 overflow 行
        lines.drain(0..overflow);
        self.scroll_offset = self.scroll_offset.saturating_sub(overflow);
    }

    /// 替换整段输出内容，并在超过上限时丢弃最旧的行。
    ///
    /// 说明：
    /// - 并行 Supervisor TUI 需要“保留原始输出行 → 重新渲染”为 styled lines，
    ///   因此每次追加输出后，可能会对当前 job 的整段内容做一次全量重渲染。
    /// - 为了避免内存无限增长，这里同样需要一个按行上限的裁剪策略。
    pub fn replace_lines_capped(&mut self, mut new_lines: Vec<Line<'static>>, max_lines: usize) {
        if max_lines == 0 {
            return;
        }

        if new_lines.len() > max_lines {
            let overflow = new_lines.len().saturating_sub(max_lines);
            new_lines.drain(0..overflow);
            self.scroll_offset = self.scroll_offset.saturating_sub(overflow);
        }

        // 防御性：避免 scroll_offset 指向越界位置导致“看起来像空白输出”。
        if new_lines.is_empty() {
            self.scroll_offset = 0;
        } else {
            self.scroll_offset = self.scroll_offset.min(new_lines.len().saturating_sub(1));
        }

        if let Ok(mut lines) = self.lines.lock() {
            *lines = new_lines;
        }
    }

    /// Returns the total number of lines in the buffer.
    pub fn line_count(&self) -> usize {
        self.lines.lock().map(|l| l.len()).unwrap_or(0)
    }

    /// Returns a clone of the visible lines based on scroll offset and viewport height.
    ///
    /// Note: Returns owned Vec instead of slice due to interior mutability.
    pub fn visible_lines(&self, viewport_height: usize) -> Vec<Line<'static>> {
        let Ok(lines) = self.lines.lock() else {
            return Vec::new();
        };
        if lines.is_empty() {
            return Vec::new();
        }
        let start = self.scroll_offset.min(lines.len());
        let end = (start + viewport_height).min(lines.len());
        lines[start..end].to_vec()
    }

    /// Scrolls up by one line.
    /// Disables auto-scroll since user is moving away from bottom.
    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
        self.following_bottom = false;
    }

    /// Scrolls down by one line, respecting the viewport bounds.
    /// Re-enables auto-scroll if user reaches the bottom.
    pub fn scroll_down(&mut self, viewport_height: usize) {
        let max_scroll = self.max_scroll_offset(viewport_height);
        if self.scroll_offset < max_scroll {
            self.scroll_offset += 1;
        }
        // Re-enable following if user scrolled to or past the bottom
        if self.scroll_offset >= max_scroll {
            self.following_bottom = true;
        }
    }

    /// Scrolls to the top of the buffer.
    /// Disables auto-scroll since user is moving away from bottom.
    pub fn scroll_top(&mut self) {
        self.scroll_offset = 0;
        self.following_bottom = false;
    }

    /// Scrolls to the bottom of the buffer.
    /// Re-enables auto-scroll since user explicitly went to bottom.
    pub fn scroll_bottom(&mut self, viewport_height: usize) {
        self.scroll_offset = self.max_scroll_offset(viewport_height);
        self.following_bottom = true;
    }

    /// Calculates the maximum scroll offset for the given viewport height.
    fn max_scroll_offset(&self, viewport_height: usize) -> usize {
        self.lines
            .lock()
            .map(|l| l.len().saturating_sub(viewport_height))
            .unwrap_or(0)
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

        assert_eq!(state.hat_graph_recent_events.len(), 1);
        let last = state
            .hat_graph_recent_events
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
            .hat_graph_edge_animations
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
                .hat_graph_edge_animations
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
                .hat_graph_edge_animations
                .contains_key(&HatId::new("builder")),
            "edge animation should be cancelled when builder is no longer Running"
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
        let plan = plan_hat_graph_radar_edge_animation(
            Duration::from_millis(0),
            path_len,
            reveal_step_ms,
            head_step_ms,
            head_len,
        );
        assert_eq!(plan.base_steps, 0);
        assert_eq!(plan.head_start, None);

        // 25ms：reveal 了 2 个 cell，head 贴着前沿（不环绕）
        let plan = plan_hat_graph_radar_edge_animation(
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
        let plan = plan_hat_graph_radar_edge_animation(
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
        let plan = plan_hat_graph_radar_edge_animation(
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
            assert_eq!(state.iterations[0].number, 1);
        }

        #[test]
        fn start_new_iteration_creates_subsequent_buffers() {
            let mut state = TuiState::new();
            state.start_new_iteration();
            state.start_new_iteration();
            state.start_new_iteration();

            assert_eq!(state.total_iterations(), 3);
            assert_eq!(state.iterations[0].number, 1);
            assert_eq!(state.iterations[1].number, 2);
            assert_eq!(state.iterations[2].number, 3);
        }

        #[test]
        fn current_iteration_returns_correct_buffer() {
            // Given TuiState with 3 iterations and current_view = 1
            let mut state = TuiState::new();
            state.start_new_iteration();
            state.start_new_iteration();
            state.start_new_iteration();
            state.current_view = 1;

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
            state.current_view = 1;
            state.following_latest = false;

            // When navigate_next() is called
            state.navigate_next();

            // Then current_view == 2
            assert_eq!(state.current_view, 2);
        }

        #[test]
        fn navigate_prev_decreases_current_view() {
            // Given TuiState with current_view = 2
            let mut state = TuiState::new();
            state.start_new_iteration();
            state.start_new_iteration();
            state.start_new_iteration();
            state.current_view = 2;

            // When navigate_prev() is called
            state.navigate_prev();

            // Then current_view == 1
            assert_eq!(state.current_view, 1);
        }

        #[test]
        fn navigate_next_does_not_exceed_bounds() {
            // Given TuiState with current_view = 2 and 3 iterations (max index 2)
            let mut state = TuiState::new();
            state.start_new_iteration();
            state.start_new_iteration();
            state.start_new_iteration();
            state.current_view = 2;

            // When navigate_next() is called
            state.navigate_next();

            // Then current_view stays at 2
            assert_eq!(state.current_view, 2);
        }

        #[test]
        fn navigate_prev_does_not_go_below_zero() {
            // Given TuiState with current_view = 0
            let mut state = TuiState::new();
            state.start_new_iteration();
            state.current_view = 0;

            // When navigate_prev() is called
            state.navigate_prev();

            // Then current_view stays at 0
            assert_eq!(state.current_view, 0);
        }

        #[test]
        fn following_latest_initially_true() {
            // Given new TuiState
            // When created
            let state = TuiState::new();

            // Then following_latest == true
            assert!(state.following_latest);
        }

        #[test]
        fn following_latest_becomes_false_on_back_navigation() {
            // Given TuiState with following_latest = true and current_view = 2
            let mut state = TuiState::new();
            state.start_new_iteration();
            state.start_new_iteration();
            state.start_new_iteration();
            state.current_view = 2;
            state.following_latest = true;

            // When navigate_prev() is called
            state.navigate_prev();

            // Then following_latest == false
            assert!(!state.following_latest);
        }

        #[test]
        fn following_latest_restored_at_latest() {
            // Given TuiState with following_latest = false
            let mut state = TuiState::new();
            state.start_new_iteration();
            state.start_new_iteration();
            state.start_new_iteration();
            state.current_view = 1;
            state.following_latest = false;

            // When navigate_next() reaches the last iteration
            state.navigate_next(); // 1 -> 2 (last)

            // Then following_latest == true
            assert!(state.following_latest);
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
            state.following_latest = true;
            state.start_new_iteration();
            state.start_new_iteration();

            // When following latest, current_view should track new iterations
            assert_eq!(state.current_view, 1); // Index of second iteration
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
            state.iterations[0].scroll_offset = 5;
            state.iterations[1].scroll_offset = 0;

            // When switching between iterations
            state.current_view = 0;
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
                    state.iterations[i].append_line(Line::from(format!(
                        "iter {} line {}",
                        i + 1,
                        j
                    )));
                }
            }

            // Set initial scroll offsets
            state.iterations[0].scroll_offset = 3;
            state.iterations[1].scroll_offset = 7;
            state.iterations[2].scroll_offset = 10;

            // When scrolling in iteration 2
            state.current_view = 1;
            state.current_iteration_mut().unwrap().scroll_down(10);

            // Then only iteration 2's scroll changed
            assert_eq!(
                state.iterations[0].scroll_offset, 3,
                "iteration 1 unchanged"
            );
            assert_eq!(
                state.iterations[1].scroll_offset, 8,
                "iteration 2 scrolled down"
            );
            assert_eq!(
                state.iterations[2].scroll_offset, 10,
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
            assert_eq!(state.new_iteration_alert, Some(3));
        }

        #[test]
        fn new_iteration_alert_not_set_when_following() {
            // Given following_latest = true
            let mut state = TuiState::new();
            state.following_latest = true;
            state.start_new_iteration();

            // When start_new_iteration() is called
            state.start_new_iteration();

            // Then new_iteration_alert remains None
            assert_eq!(state.new_iteration_alert, None);
        }

        #[test]
        fn alert_cleared_when_following_restored() {
            // Given new_iteration_alert = Some(5)
            let mut state = TuiState::new();
            state.start_new_iteration();
            state.start_new_iteration();
            state.start_new_iteration();
            state.current_view = 0;
            state.following_latest = false;
            state.new_iteration_alert = Some(3);

            // When navigation restores following_latest = true
            state.navigate_next(); // 0 -> 1
            state.navigate_next(); // 1 -> 2 (last, restores following)

            // Then new_iteration_alert is cleared to None
            assert_eq!(state.new_iteration_alert, None);
        }

        #[test]
        fn alert_not_cleared_on_partial_navigation() {
            // Given new_iteration_alert = Some(3) and not at last iteration
            let mut state = TuiState::new();
            state.start_new_iteration();
            state.start_new_iteration();
            state.start_new_iteration();
            state.current_view = 0;
            state.following_latest = false;
            state.new_iteration_alert = Some(3);

            // When navigate_next() but not reaching last
            state.navigate_next(); // 0 -> 1

            // Then alert is still set (not at latest yet)
            assert_eq!(state.new_iteration_alert, Some(3));
            assert!(!state.following_latest);
        }

        #[test]
        fn alert_updates_for_multiple_new_iterations() {
            // Given not following and multiple new iterations arrive
            let mut state = TuiState::new();
            state.start_new_iteration(); // 1
            state.start_new_iteration(); // 2
            state.navigate_prev(); // Go back, stop following

            state.start_new_iteration(); // 3 arrives
            assert_eq!(state.new_iteration_alert, Some(3));

            // When another iteration arrives
            state.start_new_iteration(); // 4 arrives

            // Then alert should show the newest
            assert_eq!(state.new_iteration_alert, Some(4));
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
                state.search_state.matches.len() >= 3,
                "expected at least 3 matches, got {}",
                state.search_state.matches.len()
            );
            assert_eq!(state.search_state.query, Some("error".to_string()));
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
                state.search_state.matches.len(),
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
            state.search_state.current_match = 2;

            // When next_match() is called
            state.next_match();

            // Then current_match becomes 0 (cycles back)
            assert_eq!(state.search_state.current_match, 0);
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
            state.search_state.current_match = 0;

            // When prev_match() is called
            state.prev_match();

            // Then current_match becomes 2 (cycles back)
            assert_eq!(state.search_state.current_match, 2);
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
            assert!(state.search_state.query.is_some());

            // When clear_search() is called
            state.clear_search();

            // Then query = None, matches cleared, search_mode = false
            assert!(state.search_state.query.is_none());
            assert!(state.search_state.matches.is_empty());
            assert!(!state.search_state.search_mode);
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
            assert_eq!(state.search_state.query, Some("xyz".to_string()));
            assert!(state.search_state.matches.is_empty());
            assert_eq!(state.search_state.current_match, 0);
        }

        #[test]
        fn search_on_empty_iteration_handles_gracefully() {
            // Given empty iteration
            let mut state = TuiState::new();
            state.start_new_iteration();

            // When searching
            state.search("anything");

            // Then no panic, empty matches
            assert!(state.search_state.matches.is_empty());
        }

        #[test]
        fn next_match_with_no_matches_does_nothing() {
            // Given no active search or empty matches
            let mut state = TuiState::new();
            state.start_new_iteration();

            // When next_match is called
            state.next_match();

            // Then no panic, current_match stays 0
            assert_eq!(state.search_state.current_match, 0);
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
                state.search_state.matches.len(),
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
            state.current_view = 0;
            state.following_latest = false;

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
            let latest_buffer = state.iterations.last().unwrap();
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
            state.current_view = 2;
            state.following_latest = false;

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
            let iteration_3 = &state.iterations[2];
            assert_eq!(
                iteration_3.lines.lock().unwrap().len(),
                0,
                "iteration 3 (being viewed) should have no output"
            );

            // Verify: iteration 7 (latest) should have the output
            let iteration_7 = state.iterations.last().unwrap();
            assert_eq!(
                iteration_7.lines.lock().unwrap().len(),
                1,
                "iteration 7 (latest) should have the output"
            );
        }
    }
}
