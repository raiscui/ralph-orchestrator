//! Job 执行域: 编排 port(`HatJobExecutor`)的后端适配实现。
//!
//! 说明:
//! - core 定义 `HatJobExecutor` port(编排 → 进程执行的 seam)。
//! - 这里实现三种后端形态:
//!   - [`headless`]: 一次性 CLI 进程(headless exec)
//!   - [`app_server`]: Codex App Server 常驻会话(turn/steer/interrupt)
//!   - [`mcp`]: Codex MCP 常驻会话
//! - [`CliHatJobExecutor`] 是选择器: 按 backend / session_strategy 路由到具体形态。
//!
//! 路由优先级: app_server > mcp > headless(见 `should_use_*`)。

pub mod app_server;
mod headless;
pub mod mcp;

pub use app_server::CodexAppServerRuntime;
pub use mcp::CodexMcpRuntime;

use crate::cli_backend::CliBackend;
use ralph_core::{HatJob, HatJobControl, HatJobExecutor, HatJobOutputChunk, HatJobResult, JobBackend, RoleArgsConfig, RoleReasoningEffortConfig};
use crate::cli_backend::CliExecutionRole;
use ralph_proto::SessionStrategy;
use std::sync::Arc;

pub struct CliHatJobExecutor {
    pub default_backend: CliBackend,
    /// Role-aware extra CLI args copied from `config.cli`.
    pub role_args: RoleArgsConfig,
    /// Role-aware reasoning effort defaults copied from `config.cli`.
    pub role_reasoning_effort: RoleReasoningEffortConfig,
    /// `ralph run -- <custom args>`：按次追加到实际执行的 backend args。
    ///
    /// 说明：
    /// - 这对并行模式同样重要（否则行为与串行不一致）。
    /// - 追加顺序：backend 默认 args / hat-level args 在前，custom_args 在后（更像“命令行最终覆盖”）。
    pub custom_args: Vec<String>,
    /// Ralph 实例专用: Codex MCP 常驻会话运行时。
    pub codex_mcp_runtime: Arc<CodexMcpRuntime>,
    /// Codex App Server 常驻会话运行时（支持 turn/steer/interrupt）。
    pub codex_app_server_runtime: Arc<CodexAppServerRuntime>,
}

#[async_trait::async_trait]
impl HatJobExecutor for CliHatJobExecutor {
    async fn execute(
        &self,
        job: HatJob,
        output_tx: tokio::sync::mpsc::Sender<HatJobOutputChunk>,
        cancel_rx: tokio::sync::watch::Receiver<bool>,
        control_rx: tokio::sync::mpsc::Receiver<HatJobControl>,
    ) -> anyhow::Result<HatJobResult> {
        let mut backend = match &job.backend {
            JobBackend::Default => self.default_backend.clone(),
            JobBackend::Hat(hat_backend) => CliBackend::from_hat_backend(hat_backend)
                .map_err(|e| anyhow::anyhow!("Invalid hat backend config: {e}"))?,
        };

        // 角色化 reasoning 默认:
        // - Ralph/coordinator job 默认 medium,用于快速分发/收敛。
        // - non-Ralph worker 默认 high,用于具体执行/审查。
        // - adapter 会保留 hat-level/custom args 中的显式 override。
        let cli_role = if job.hat_id.as_str() == "ralph" {
            CliExecutionRole::Coordinator
        } else {
            CliExecutionRole::Worker
        };
        Self::apply_role_backend_overlays(
            &mut backend,
            &self.role_args,
            &self.custom_args,
            self.role_reasoning_effort,
            cli_role,
        );

        if Self::should_use_codex_app_server(&job, &backend) {
            return self
                .codex_app_server_runtime
                .execute_job(&job, &backend, output_tx, cancel_rx, control_rx)
                .await;
        }

        if Self::should_use_codex_mcp(&job, &backend) {
            // 当前 Codex MCP runtime 不支持 in-flight steer；控制消息会在 core 侧被降级为普通事件入队。
            return self
                .codex_mcp_runtime
                .execute_job(&job, &backend, output_tx, cancel_rx)
                .await;
        }

        // 非 app_server 的后端不支持 in-flight steer: 避免 control_rx 堵塞,直接丢弃即可。
        let _ = control_rx;

        // headless 执行: spawn 外部 CLI 进程, 流式采集, 处理超时/取消。
        headless::spawn_headless_job(&backend, &job, output_tx, cancel_rx).await
    }
}


impl CliHatJobExecutor {
    fn apply_role_backend_overlays(
        backend: &mut CliBackend,
        role_args: &RoleArgsConfig,
        custom_args: &[String],
        role_reasoning_effort: RoleReasoningEffortConfig,
        cli_role: CliExecutionRole,
    ) {
        backend.apply_role_args(role_args, cli_role);

        if !custom_args.is_empty() {
            backend.args.extend(custom_args.iter().cloned());
        }

        backend.apply_role_reasoning_effort_defaults(&role_reasoning_effort, cli_role);
    }
    fn should_use_codex_mcp(job: &HatJob, backend: &CliBackend) -> bool {
        // ------------------------------------------------------------------
        // 说明:
        // - 默认走一次性 exec.
        // - 只有当事件显式请求 `session_strategy=mcp` 时才升级为 Codex MCP 常驻模式.
        // - 方案1(只升级,不降级): instance 一旦进入 mcp,后续 job 会 sticky 到 mcp(由 core 侧合并).
        // ------------------------------------------------------------------
        if backend.command != "codex" {
            return false;
        }

        // 显式请求 app_server 时,必须让 app_server 通道接管（优先级高于 mcp）。
        if job.session_strategy == SessionStrategy::AppServer {
            return false;
        }

        job.session_strategy == SessionStrategy::Mcp
    }
    fn should_use_codex_app_server(job: &HatJob, backend: &CliBackend) -> bool {
        if backend.command != "codex" {
            return false;
        }

        job.session_strategy == SessionStrategy::AppServer
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli_backend::CliBackend;

    #[test]
    fn parallel_role_backend_overlays_apply_coordinator_hooks_only() {
        let role_args = RoleArgsConfig {
            coordinator: vec!["-c".to_string(), "features.hooks=false".to_string()],
            worker: Vec::new(),
        };
        let custom_args: Vec<String> = Vec::new();

        let mut coordinator_backend = CliBackend::codex();
        CliHatJobExecutor::apply_role_backend_overlays(
            &mut coordinator_backend,
            &role_args,
            &custom_args,
            RoleReasoningEffortConfig::default(),
            CliExecutionRole::Coordinator,
        );

        assert!(
            coordinator_backend
                .args
                .contains(&"features.hooks=false".to_string()),
            "coordinator should receive hooks override"
        );

        let mut worker_backend = CliBackend::codex();
        CliHatJobExecutor::apply_role_backend_overlays(
            &mut worker_backend,
            &role_args,
            &custom_args,
            RoleReasoningEffortConfig::default(),
            CliExecutionRole::Worker,
        );

        assert!(
            !worker_backend
                .args
                .contains(&"features.hooks=false".to_string()),
            "worker should not receive coordinator-only hooks override"
        );
    }
}
