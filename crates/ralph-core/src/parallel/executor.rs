//! HatJob 执行器抽象。
//!
//! 说明：
//! - ralph-core 只定义“怎么调度/怎么路由/怎么落盘”，不直接绑定某个具体 LLM SDK。
//! - 真正的执行由 ralph-cli（或其他前端）实现：spawn 外部 headless CLI 进程并流式采集输出。

use super::{HatJob, HatJobControl, HatJobOutputChunk, HatJobResult};
use async_trait::async_trait;
use tokio::sync::mpsc;

/// HatJob 执行器（由上层实现，例如 ralph-cli）。
#[async_trait]
pub trait HatJobExecutor: Send + Sync {
    /// 执行一个 job，并把 stdout/stderr 以 chunk 的形式流式写入 `output_tx`。
    ///
    /// 注意：
    /// - `cancel_rx` 为 true 时，执行器必须尽最大努力终止子进程（SIGTERM -> grace -> SIGKILL）。
    async fn execute(
        &self,
        job: HatJob,
        output_tx: mpsc::Sender<HatJobOutputChunk>,
        cancel_rx: tokio::sync::watch::Receiver<bool>,
        control_rx: mpsc::Receiver<HatJobControl>,
    ) -> anyhow::Result<HatJobResult>;
}
