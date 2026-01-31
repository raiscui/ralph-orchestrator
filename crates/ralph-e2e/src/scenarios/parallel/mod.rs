//! Tier 8: Parallel Runtime (experimental) test scenarios.
//!
//! 说明：
//! - 这些场景用于验证 **parallel hat instances** 在“真实后端”上的端到端行为。
//! - 与 replay smoke tests 的差异：
//!   - E2E 会覆盖真实 CLI、真实认证、真实网络与真实模型漂移带来的风险
//!   - 代价更高、速度更慢，因此场景应尽量“短、稳、可排障”

mod hat_instances;
mod job_run_counts;
mod starting_event_inference;

pub use hat_instances::ParallelHatInstancesScenario;
pub use starting_event_inference::ParallelStartingEventInferenceScenario;

// 说明：
// - 这些 helper 目前会被 `parallel_trigger_routing_example` 复用。
// - 可见性限制在 `crate::scenarios`，避免扩散到整个 crate。
pub(in crate::scenarios) use job_run_counts::{JobRunCounts, parse_parallel_job_line};
