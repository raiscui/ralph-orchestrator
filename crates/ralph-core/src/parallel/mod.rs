//! 并行 HatInstance 运行时（实验性）。
//!
//! 设计目标：
//! - 允许多个 hat / 同一 hat 多实例并行执行（headless job）
//! - 路由语义显式化（queue / fanout），关键决策可落盘、可回放
//! - orchestrator 保持“薄协调层”：复杂工作交给外部 CLI agent

mod command_queue;
mod executor;
mod instance;
mod job;
mod router;
mod supervisor;

pub use command_queue::{COMMAND_LANE_WORKSPACE_GIT, CommandQueue};
pub use executor::HatJobExecutor;
pub use instance::{HatInstanceCommand, HatInstanceEvent, HatInstanceHandle};
pub use job::{HatJob, HatJobControl, HatJobOutputChunk, HatJobResult, JobBackend, OutputStream};
pub use router::TopicContractStore;
pub use supervisor::{ParallelRunResult, ParallelSupervisor};
