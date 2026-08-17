//! 串行模式的执行器适配: `PtyPromptExecutor`。
//!
//! 说明:
//! - 实现 `ralph_core::PromptExecutor` port: 后端选择、角色参数、展示 handler、
//!   进程执行(PTY / 缓冲)全部收在这里。
//! - cli 只负责装配(显示参数/角色参数/自定义参数)与观察者。

use crate::cli_backend::{CliBackend, CliExecutionRole};
use crate::cli_executor::CliExecutor;
use crate::pty_executor::PtyConfig;
use crate::PtyExecutor;
use ralph_core::{
    HatBackend, PromptExecutor, PromptOutput, RalphConfig, RoleArgsConfig,
    RoleReasoningEffortConfig,
};
use ralph_display::{
    DisplayTarget, DisplayVerbosity, MarkdownRenderMode, TuiLineBuffer,
    make_stream_handler,
};
use ralph_proto::HatId;
use std::io::{self, IsTerminal};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;

/// 串行模式执行器: PTY 优先, 结构化后端走 CliExecutor。
pub struct PtyPromptExecutor {
    /// PTY 执行器(懒创建, TUI 连接时复用)。
    pty: Option<PtyExecutor>,
    /// 全局默认 backend(hat-level 未覆盖时使用)。
    default_backend: CliBackend,
    /// 展示参数(渲染模式 / 详细度 / TUI 行缓冲)。
    render_mode: MarkdownRenderMode,
    display_verbosity: DisplayVerbosity,
    tui_lines: Option<TuiLineBuffer>,
    /// TUI 行缓冲换代回调(每轮迭代由 cli 注入, 返回最新迭代的 lines)。
    tui_lines_provider: Option<Arc<dyn Fn() -> Option<TuiLineBuffer> + Send + Sync>>,
    /// 角色参数与一次性自定义参数。
    role_args: RoleArgsConfig,
    role_reasoning_effort: RoleReasoningEffortConfig,
    custom_args: Vec<String>,
    /// 交互模式: idle timeout 语义(交互时 idle 表示"迭代完成"而非终止)。
    interactive: bool,
    workspace_root: std::path::PathBuf,
    /// 全局配置(adapter settings / cli idle timeout 来源)。
    config: RalphConfig,
}

impl PtyPromptExecutor {
    /// 装配串行执行器。
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        default_backend: CliBackend,
        render_mode: MarkdownRenderMode,
        display_verbosity: DisplayVerbosity,
        tui_lines: Option<TuiLineBuffer>,
        role_args: RoleArgsConfig,
        role_reasoning_effort: RoleReasoningEffortConfig,
        custom_args: Vec<String>,
        interactive: bool,
        workspace_root: std::path::PathBuf,
        config: RalphConfig,
        tui_lines_provider: Option<Arc<dyn Fn() -> Option<TuiLineBuffer> + Send + Sync>>,
    ) -> Self {
        Self {
            pty: None,
            default_backend,
            render_mode,
            display_verbosity,
            tui_lines,
            tui_lines_provider,
            role_args,
            role_reasoning_effort,
            custom_args,
            interactive,
            workspace_root,
            config,
        }
    }

    /// 解析本轮实际使用的 backend(hat-level 覆盖全局)。
    fn resolve_backend(&self, hat_backend: Option<&HatBackend>) -> anyhow::Result<CliBackend> {
        match hat_backend {
            Some(hb) => crate::cli_backend::CliBackend::from_hat_backend(hb)
                .map_err(|e| anyhow::anyhow!("Invalid hat backend config: {e}")),
            None => Ok(self.default_backend.clone()),
        }
    }

    /// 应用角色化参数(与并行模式 CliHatJobExecutor 一致)。
    fn apply_role_args(&self, backend: &mut CliBackend, hat_id: &HatId) {
        let cli_role = if hat_id.as_str() == "ralph" {
            CliExecutionRole::Coordinator
        } else {
            CliExecutionRole::Worker
        };
        backend.apply_role_args(&self.role_args, cli_role);
        if !self.custom_args.is_empty() {
            backend.args.extend(self.custom_args.iter().cloned());
        }
        backend.apply_role_reasoning_effort_defaults(&self.role_reasoning_effort, cli_role);
    }
}

#[async_trait::async_trait]
impl PromptExecutor for PtyPromptExecutor {
    fn on_iteration_started(&mut self, _iteration: u32) {
        // TUI 模式下每轮换代行缓冲(指向最新迭代的 lines)。
        if let Some(provider) = &self.tui_lines_provider {
            self.tui_lines = provider();
        }
    }

    async fn execute_prompt(
        &mut self,
        prompt: &str,
        interactive: bool,
        hat_id: &HatId,
        hat_backend: Option<&HatBackend>,
        interrupt_rx: watch::Receiver<bool>,
    ) -> anyhow::Result<PromptOutput> {
        let mut backend = self.resolve_backend(hat_backend)?;
        self.apply_role_args(&mut backend, hat_id);

        // 结构化后端(Gemini 等)在自动化路径下走 CliExecutor:
        // stdout/stderr 可分离, 可提取最终 response, 避免 PTY 混合。
        let should_force_buffered_headless = backend.emits_structured_response()
            && !interactive
            && self.tui_lines.is_none();

        if should_force_buffered_headless {
            let executor = CliExecutor::new(backend.clone());
            let adapter_settings = self.config.adapter_settings(&backend.command);
            let timeout = (adapter_settings.timeout > 0)
                .then(|| Duration::from_secs(adapter_settings.timeout));
            let output_stale_timeout = (adapter_settings.output_stale_timeout_secs > 0)
                .then(|| Duration::from_secs(adapter_settings.output_stale_timeout_secs));
            let result = executor
                .execute(
                    prompt,
                    io::stdout(),
                    timeout,
                    output_stale_timeout,
                    self.display_verbosity == DisplayVerbosity::Verbose,
                )
                .await?;
            return Ok(PromptOutput {
                output: result.output,
                timed_out: false,
                canceled: false,
                success: result.success,
                // CliExecutor 非 PTY 路径不报告 context window (Gemini 等)
                context_window: 0,
            });
        }

        // PTY 路径: 懒创建 executor(首轮)或复用(后续轮次需更新 backend)。
        if self.pty.is_none() {
            let idle_timeout_secs = if self.interactive {
                self.config.cli.idle_timeout_secs
            } else {
                0
            };
            let pty_config = PtyConfig {
                interactive: self.interactive,
                idle_timeout_secs,
                workspace_root: self.workspace_root.clone(),
                ..PtyConfig::from_env()
            };
            self.pty = Some(PtyExecutor::new(backend.clone(), pty_config));
        }
        let exec = self.pty.as_mut().expect("pty just created");
        exec.set_backend(backend.clone());
        if self.tui_lines.is_some() {
            exec.set_tui_mode(true);
        }

        // 交互 + 无 TUI: 裸交互模式(PTY 自身处理 raw mode)。
        if interactive && self.tui_lines.is_none() {
            let result = exec.run_interactive(prompt, interrupt_rx).await?;
            return Ok(to_prompt_output(result, interactive));
        }

        // 其余: 展示意图交给 display 工厂(选择矩阵不再泄漏到调用者)。
        let target = match &self.tui_lines {
            Some(lines) => DisplayTarget::Tui(lines.clone()),
            None => DisplayTarget::Console {
                stream_json: backend.output_format == crate::cli_backend::OutputFormat::StreamJson,
                tty: io::stdout().is_terminal(),
            },
        };
        let mut handler = make_stream_handler(target, self.display_verbosity, self.render_mode);
        let result = exec
            .run_observe_streaming(prompt, interrupt_rx, &mut handler)
            .await?;
        Ok(to_prompt_output(result, interactive))
    }
}

/// 将 PTY 执行结果归一化为 PromptOutput(交互 idle timeout = 迭代完成)。
fn to_prompt_output(
    result: crate::pty_executor::PtyExecutionResult,
    interactive: bool,
) -> PromptOutput {
    // 交互 idle timeout 表示"本轮迭代完成"(继续处理输出);
    // 非交互 idle timeout 表示循环终止。
    let timed_out = matches!(
        result.termination,
        crate::pty_executor::TerminationType::IdleTimeout
    ) && !interactive;
    let canceled = matches!(
        result.termination,
        crate::pty_executor::TerminationType::UserInterrupt
            | crate::pty_executor::TerminationType::ForceKill
    );

    let output = if result.extracted_text.is_empty() {
        result.stripped_output
    } else {
        result.extracted_text
    };

    PromptOutput {
        output,
        timed_out,
        canceled,
        success: result.success,
        context_window: result.context_window,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pty_executor::{PtyExecutionResult, TerminationType};

    fn result_with(termination: TerminationType) -> PtyExecutionResult {
        PtyExecutionResult {
            output: String::new(),
            stripped_output: "output".to_string(),
            extracted_text: String::new(),
            success: true,
            exit_code: Some(0),
            termination,
            context_window: 0,
        }
    }

    #[test]
    fn idle_timeout_interactive_continues() {
        let out = to_prompt_output(result_with(TerminationType::IdleTimeout), true);
        assert!(!out.timed_out && !out.canceled && out.success);
    }

    #[test]
    fn idle_timeout_autonomous_stops() {
        let out = to_prompt_output(result_with(TerminationType::IdleTimeout), false);
        assert!(out.timed_out);
    }

    #[test]
    fn natural_always_continues() {
        let out = to_prompt_output(result_with(TerminationType::Natural), false);
        assert!(!out.timed_out && !out.canceled);
    }

    #[test]
    fn interrupt_and_force_kill_cancel() {
        for t in [TerminationType::UserInterrupt, TerminationType::ForceKill] {
            let out = to_prompt_output(result_with(t.clone()), false);
            assert!(out.canceled, "{t:?} should cancel");
        }
    }
}
