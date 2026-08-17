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

    /// Session-scoped peak context-token count across all iterations.
    pub peak_input_tokens: u64,

    /// Last iteration's context-token count (if any).
    pub last_input_tokens: Option<u64>,

    /// Per-hat session-scoped peak context-token count.
    pub hat_peak_input_tokens: HashMap<HatId, u64>,
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
            peak_input_tokens: 0,
            last_input_tokens: None,
            hat_peak_input_tokens: HashMap::new(),
            unacknowledged_guidance: Vec::new(),
        }
    }
}

impl LoopState {
    /// Creates a new loop state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record this iteration's context-token usage for the hat that ran it.
    ///
    /// 说明:
    /// - `tokens` 是 iteration 的 adapter-reported live context occupancy。
    /// - 当 `tokens == 0` 时 no-op (ACP / non-token backends suppressed)。
    /// - Peaks session-scoped — 从不在 iteration boundary 重置。
    pub fn record_iteration_tokens(&mut self, hat: &HatId, tokens: u64) {
        if tokens == 0 {
            return;
        }
        let entry = self.hat_peak_input_tokens.entry(hat.clone()).or_insert(0);
        *entry = (*entry).max(tokens);
        self.peak_input_tokens = self.peak_input_tokens.max(tokens);
        self.last_input_tokens = Some(tokens);
    }

    /// Returns the elapsed time since the loop started.
    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }
}

#[cfg(test)]
mod context_window_tests {
    use super::*;

    #[test]
    fn record_iteration_tokens_tracks_per_hat_and_global_peak() {
        let mut state = LoopState::new();
        let builder = HatId::new("builder");
        let critic = HatId::new("critic");

        state.record_iteration_tokens(&builder, 10_000);
        assert_eq!(state.hat_peak_input_tokens.get(&builder).copied(), Some(10_000));
        assert_eq!(state.peak_input_tokens, 10_000);
        assert_eq!(state.last_input_tokens, Some(10_000));

        // Critic peaks higher than builder — global peak tracks max, per-hat stays independent
        state.record_iteration_tokens(&critic, 20_000);
        assert_eq!(state.hat_peak_input_tokens.get(&critic).copied(), Some(20_000));
        assert_eq!(state.peak_input_tokens, 20_000);
        assert_eq!(state.last_input_tokens, Some(20_000));
    }

    #[test]
    fn record_iteration_tokens_zero_tokens_is_noop() {
        let mut state = LoopState::new();
        let builder = HatId::new("builder");

        // Non-token backend (ACP, etc.) reports 0
        state.record_iteration_tokens(&builder, 0);
        assert_eq!(state.peak_input_tokens, 0);
        assert_eq!(state.last_input_tokens, None);
        assert!(state.hat_peak_input_tokens.is_empty());
    }

    #[test]
    fn record_iteration_tokens_per_hat_peak_independent_of_global() {
        let mut state = LoopState::new();
        let writer = HatId::new("writer");
        let tester = HatId::new("tester");

        // writer spikes high once, then goes back down
        state.record_iteration_tokens(&writer, 50_000);
        state.record_iteration_tokens(&writer, 1_000);
        assert_eq!(state.hat_peak_input_tokens.get(&writer).copied(), Some(50_000));

        // tester reports a smaller value — global peak should still be 50_000 (from writer)
        state.record_iteration_tokens(&tester, 5_000);
        assert_eq!(state.peak_input_tokens, 50_000);
        assert_eq!(state.hat_peak_input_tokens.get(&tester).copied(), Some(5_000));
    }
}
