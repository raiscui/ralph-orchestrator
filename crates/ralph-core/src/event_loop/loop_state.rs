//! Loop state tracking for the event loop.
//!
//! This module contains the `LoopState` struct that tracks the current
//! state of the orchestration loop including iteration count, failures,
//! timing, and hat activation tracking.

use ralph_proto::HatId;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

/// Current state of the event loop.
#[derive(Debug)]
pub struct LoopState {
    /// Current iteration number (1-indexed).
    pub iteration: u32,
    /// Number of consecutive failures.
    pub consecutive_failures: u32,
    /// Cumulative cost in USD (if tracked).
    pub cumulative_cost: f64,
    /// When the loop started.
    pub started_at: Instant,
    /// The last hat that executed.
    pub last_hat: Option<HatId>,
    /// Consecutive blocked events from the same hat.
    pub consecutive_blocked: u32,
    /// Hat that emitted the last blocked event.
    pub last_blocked_hat: Option<HatId>,
    /// Per-task block counts for task-level thrashing detection.
    pub task_block_counts: HashMap<String, u32>,
    /// Tasks that have been abandoned after 3+ blocks.
    pub abandoned_tasks: Vec<String>,
    /// Count of times planner dispatched an already-abandoned task.
    pub abandoned_task_redispatches: u32,
    /// Number of consecutive completion confirmations (requires 2 for termination).
    pub completion_confirmations: u32,
    /// Consecutive malformed JSONL lines encountered (for validation backpressure).
    pub consecutive_malformed_events: u32,

    /// Per-hat activation counts (used for max_activations).
    pub hat_activation_counts: HashMap<HatId, u32>,

    /// Hats for which `<hat_id>.exhausted` has been emitted.
    pub exhausted_hats: HashSet<HatId>,

    /// Human guidance messages that must be acknowledged before completion.
    ///
    /// 说明：
    /// - `human.guidance` 事件 push 到这个队列。
    /// - `human.guidance.ack` 事件清空这个队列。
    /// - 队列非空时, 终止信号 (completion_promise / complete_publishes) 被拒,
    ///   走 reset → 重新下一轮。
    /// - 这与 lazy-model-completion (complete_publishes 硬终止) 正交,
    ///   跟本地 2-strike pattern 协同工作。
    pub unacknowledged_guidance: Vec<String>,
}

impl Default for LoopState {
    fn default() -> Self {
        Self {
            iteration: 0,
            consecutive_failures: 0,
            cumulative_cost: 0.0,
            started_at: Instant::now(),
            last_hat: None,
            consecutive_blocked: 0,
            last_blocked_hat: None,
            task_block_counts: HashMap::new(),
            abandoned_tasks: Vec::new(),
            abandoned_task_redispatches: 0,
            completion_confirmations: 0,
            consecutive_malformed_events: 0,
            hat_activation_counts: HashMap::new(),
            exhausted_hats: HashSet::new(),
            unacknowledged_guidance: Vec::new(),
        }
    }
}

impl LoopState {
    /// Creates a new loop state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the elapsed time since the loop started.
    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }
}
