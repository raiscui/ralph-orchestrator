//! 串行模式的执行器 port。
//!
//! 说明:
//! - EventLoop 负责"编排状态机"(选 hat / 构建 prompt / 处理输出 / 判定终止),
//!   进程执行通过本 port 注入, 依赖方向保持 core ← adapters。
//! - 与并行模式的 `HatJobExecutor` 对称: core 定义 port, adapter 实现, cli 装配。

use crate::config::HatBackend;
use tokio::sync::watch;

/// 一次 prompt 执行的输出(供事件解析与终止判定)。
#[derive(Debug, Clone, Default)]
pub struct PromptOutput {
    /// 供事件解析的正文(与并行模式一致: stdout-only 或已抽取的响应文本)。
    pub output: String,
    /// 是否因超时终止。
    pub timed_out: bool,
    /// 是否被取消(外部中断触发)。
    pub canceled: bool,
    /// 进程是否成功(exit 0 且未超时/未取消)。
    pub success: bool,
}

/// 串行模式执行器 port。
///
/// 实现方(ralph-adapters::PtyPromptExecutor)负责:
/// - 后端选择(hat-level 覆盖全局)与角色参数应用
/// - PTY / 缓冲执行器选择(结构化后端走 CliExecutor)
/// - 展示 handler 选择(display 工厂)与实时流式输出
#[async_trait::async_trait]
pub trait PromptExecutor: Send {
    /// 执行一次 prompt, 返回解析用输出与终止信息。
    async fn execute_prompt(
        &mut self,
        prompt: &str,
        interactive: bool,
        hat_id: &ralph_proto::HatId,
        backend: Option<&HatBackend>,
        interrupt_rx: watch::Receiver<bool>,
    ) -> anyhow::Result<PromptOutput>;

    /// 每轮迭代开始时的执行器上下文通知(实现方按需更新内部状态,
    /// 例如 TUI 行缓冲的换代)。默认无操作。
    fn on_iteration_started(&mut self, _iteration: u32) {}
}

/// `EventLoop::run` 的迭代钩子(cli 注入的展示/记录副作用)。
pub struct RunHooks<'a> {
    /// 迭代开始前(打印分隔符 / verbose prompt 等展示逻辑)。
    /// 参数: iteration, display_hat, prompt, loop_elapsed。
    pub before_execute:
        Option<Box<dyn FnMut(u32, &ralph_proto::HatId, &str, std::time::Duration) + 'a>>,
    /// 迭代完成后(record-session 落盘等记录逻辑)。
    pub after_execute: Option<Box<dyn FnMut(u32, &ralph_proto::HatId, &PromptOutput) + 'a>>,
}

impl RunHooks<'_> {
    pub fn none() -> Self {
        Self {
            before_execute: None,
            after_execute: None,
        }
    }
}
