//! 并行 HatInstance 运行器（ralph-cli 侧）。
//!
//! 说明：
//! - 该模块把“调度/路由”交给 `ralph-core::ParallelSupervisor`。
//! - 这里实现 `HatJobExecutor`：spawn 外部 headless CLI 进程，流式采集 stdout/stderr。

use crate::codex_app_server_session::CodexAppServerRuntime;
use crate::codex_mcp_session::CodexMcpRuntime;
use anyhow::{Context, Result};
use ralph_adapters::CliBackend;
use ralph_core::{
    HatJob, HatJobControl, HatJobExecutor, HatJobOutputChunk, HatJobResult, JobBackend,
    OutputStream,
};
use ralph_core::{
    HatRegistry, ParallelSupervisor, RalphConfig, Record, SessionRecorder, TerminationReason,
};
use ralph_proto::{HatInstanceId, HatInstanceState, SessionStrategy, TerminalWrite, UxEvent};
use ralph_tui::{Tui, TuiUpdate};
use std::fs::File;
use std::io::{IsTerminal, Write, stdin, stdout};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::watch;
use tracing::{debug, warn};

use crate::display::colors;
use crate::process_management;
use crate::{ColorMode, Verbosity};

fn should_forward_event_to_tui(event: &ralph_proto::Event) -> bool {
    // 事件转发策略（并行模式）：
    // - gate.* / human.message / reply.human.message：用于控制面 UI（Gate 面板/提示等）；
    // - source_instance 存在：用于运行态可视化（Hat Graph Radar 边动画可据此推导发布者 hat）。
    let topic = event.topic.as_str();
    topic.starts_with("gate.")
        || topic == "human.message"
        || topic == "reply.human.message"
        || event.source_instance.is_some()
        || event.source.is_some()
}

/// ralph-cli 的 HatJobExecutor：使用外部 CLI 后端执行 prompt（headless）。
#[derive(Debug, Clone)]
struct CliHatJobExecutor {
    default_backend: CliBackend,
    /// `ralph run -- <custom args>`：按次追加到实际执行的 backend args。
    ///
    /// 说明：
    /// - 这对并行模式同样重要（否则行为与串行不一致）。
    /// - 追加顺序：backend 默认 args / hat-level args 在前，custom_args 在后（更像“命令行最终覆盖”）。
    custom_args: Vec<String>,
    /// Ralph 实例专用: Codex MCP 常驻会话运行时。
    codex_mcp_runtime: Arc<CodexMcpRuntime>,
    /// Codex App Server 常驻会话运行时（支持 turn/steer/interrupt）。
    codex_app_server_runtime: Arc<CodexAppServerRuntime>,
}

#[async_trait::async_trait]
impl HatJobExecutor for CliHatJobExecutor {
    async fn execute(
        &self,
        job: HatJob,
        output_tx: tokio::sync::mpsc::Sender<HatJobOutputChunk>,
        mut cancel_rx: tokio::sync::watch::Receiver<bool>,
        control_rx: tokio::sync::mpsc::Receiver<HatJobControl>,
    ) -> anyhow::Result<HatJobResult> {
        let mut backend = match &job.backend {
            JobBackend::Default => self.default_backend.clone(),
            JobBackend::Hat(hat_backend) => CliBackend::from_hat_backend(hat_backend)
                .map_err(|e| anyhow::anyhow!("Invalid hat backend config: {e}"))?,
        };

        if !self.custom_args.is_empty() {
            backend.args.extend(self.custom_args.iter().cloned());
        }

        if Self::should_use_codex_app_server(&job, &backend) {
            return self
                .codex_app_server_runtime
                .execute_job(&job, &backend, output_tx, cancel_rx, control_rx)
                .await;
        }

        if Self::should_use_codex_mcp(&job, &backend) {
            // 当前 Codex MCP runtime 不支持 in-flight steer；控制消息会在 core 侧被降级为普通事件入队。
            let _ = control_rx;
            return self
                .codex_mcp_runtime
                .execute_job(&job, &backend, output_tx, cancel_rx)
                .await;
        }

        // 非 app_server 的后端不支持 in-flight steer: 避免 control_rx 堵塞,直接丢弃即可。
        let _ = control_rx;

        let (cmd, args, stdin_input, _temp_file) = backend.build_command(&job.prompt, false);

        let mut command = Command::new(&cmd);
        command.args(&args);
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        // 并行模式回放/诊断：把实例信息传给后端（custom backend 可用来做输出分流）
        command.env("RALPH_HAT_INSTANCE_ID", job.instance_id.as_str());
        command.env("RALPH_HAT_ID", job.hat_id.as_str());

        if let Some(workdir) = &job.workdir {
            command.current_dir(workdir);
        }

        if stdin_input.is_some() {
            command.stdin(Stdio::piped());
        }

        debug!(
            instance = %job.instance_id,
            hat = %job.hat_id,
            command = %cmd,
            args = ?args,
            workdir = ?job.workdir,
            "Spawning headless job"
        );

        let mut child = command.spawn().with_context(|| {
            format!(
                "Failed to spawn backend process: cmd={cmd} args={args:?} workdir={:?}",
                job.workdir
            )
        })?;

        // 写 stdin（如需要）
        if let Some(input) = stdin_input
            && let Some(mut stdin) = child.stdin.take()
        {
            stdin.write_all(input.as_bytes()).await?;
            drop(stdin);
        }

        // 并发读取 stdout/stderr，避免 pipe buffer deadlock
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let (line_tx, mut line_rx) = tokio::sync::mpsc::channel::<(OutputStream, String)>(256);

        let spawn_reader =
            |stream: OutputStream,
             handle: Option<tokio::process::ChildStdout>,
             tx: tokio::sync::mpsc::Sender<(OutputStream, String)>| async move {
                if let Some(handle) = handle {
                    let reader = BufReader::new(handle);
                    let mut lines = reader.lines();
                    while let Some(line) = lines.next_line().await? {
                        if tx.send((stream, line)).await.is_err() {
                            break;
                        }
                    }
                }
                Ok::<(), std::io::Error>(())
            };

        // stdout
        let tx1 = line_tx.clone();
        let stdout_task = tokio::spawn(spawn_reader(OutputStream::Stdout, stdout, tx1));
        // stderr（类型不同：ChildStderr）
        let stderr_tx = line_tx.clone();
        let stderr_task = tokio::spawn(async move {
            if let Some(stderr) = stderr {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                while let Some(line) = lines.next_line().await? {
                    if stderr_tx.send((OutputStream::Stderr, line)).await.is_err() {
                        break;
                    }
                }
            }
            Ok::<(), std::io::Error>(())
        });

        // 释放主 sender，让 rx 能在两个 task 结束后自然关闭
        drop(line_tx);

        let mut output = String::new();
        let mut timed_out = false;
        let mut canceled = false;
        let mut last_output_changed_at = std::time::Instant::now();

        // 收集输出，直到流结束 or 被取消/超时
        match job.timeout.filter(|d| !d.is_zero()) {
            Some(check_interval) => {
                // “检测超时”语义：
                // - check_interval 到期时不立刻 kill
                // - 只有当输出停滞超过 `output_stale_timeout` 才判定 timed_out
                // - 若输出仍在变化：判定通过，并把检测窗口从此刻重新计时
                let mut next_check_deadline = tokio::time::Instant::now() + check_interval;
                let sleep = tokio::time::sleep_until(next_check_deadline);
                tokio::pin!(sleep);

                loop {
                    tokio::select! {
                        biased;
                        // 取消优先
                        changed = cancel_rx.changed() => {
                            if changed.is_ok() && *cancel_rx.borrow() {
                                canceled = true;
                                Self::terminate_child(&mut child, "canceled").await?;
                                break;
                            }
                        }
                        // 检测窗口到期：根据“输出是否停滞”决定是否超时
                        _ = &mut sleep => {
                            match job.output_stale_timeout {
                                Some(stale_timeout) => {
                                    if last_output_changed_at.elapsed() >= stale_timeout {
                                        timed_out = true;
                                        Self::terminate_child(&mut child, "timed_out").await?;
                                        break;
                                    }

                                    // 检测通过：检测窗口重新计时（从现在开始）
                                    next_check_deadline = tokio::time::Instant::now() + check_interval;
                                    sleep.as_mut().reset(next_check_deadline);
                                }
                                None => {
                                    // 兼容兜底：若未提供 stale 阈值，则退化为“硬超时”
                                    timed_out = true;
                                    Self::terminate_child(&mut child, "timed_out").await?;
                                    break;
                                }
                            }
                        }
                        line = line_rx.recv() => {
                            let Some((stream, line)) = line else {
                                break;
                            };

                            last_output_changed_at = std::time::Instant::now();
                            Self::handle_output_line(job.job_id, &job.instance_id, &output_tx, &mut output, stream, line).await;
                        }
                    }
                }
            }
            None => loop {
                tokio::select! {
                    biased;
                    changed = cancel_rx.changed() => {
                        if changed.is_ok() && *cancel_rx.borrow() {
                            canceled = true;
                            Self::terminate_child(&mut child, "canceled").await?;
                            break;
                        }
                    }
                    line = line_rx.recv() => {
                        let Some((stream, line)) = line else {
                            break;
                        };
                        Self::handle_output_line(job.job_id, &job.instance_id, &output_tx, &mut output, stream, line).await;
                    }
                }
            },
        }

        // 确保子进程被 reap（避免僵尸进程）
        let status = child.wait().await?;

        // 等待读取 task 收尾（best-effort）
        let _ = stdout_task.await;
        let _ = stderr_task.await;

        Ok(HatJobResult {
            output_for_parsing: output,
            observed_stderr: String::new(),
            success: status.success() && !timed_out && !canceled,
            exit_code: status.code(),
            timed_out,
            canceled,
        })
    }
}

impl CliHatJobExecutor {
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

    async fn handle_output_line(
        job_id: u64,
        instance_id: &HatInstanceId,
        output_tx: &tokio::sync::mpsc::Sender<HatJobOutputChunk>,
        output: &mut String,
        stream: OutputStream,
        line: String,
    ) {
        // 1) 流式输出给 Supervisor（带 instance_id 归因）
        let _ = output_tx
            .send(HatJobOutputChunk {
                job_id,
                instance_id: instance_id.clone(),
                stream,
                line: line.clone(),
            })
            .await;

        // 2) 组装完整 output（用于 event parser）
        match stream {
            OutputStream::Stdout => {
                output.push_str(&line);
                output.push('\n');
            }
            OutputStream::Stderr => {
                // 重要：并行模式下，stderr 往往包含“后端自身的日志”（例如 Codex 会回显 user prompt、
                // MCP 启动日志、warnings 等）。这些内容可能包含 `<event ...>` 字样（来自 prompt 本身），
                // 如果把 stderr 拼进 `output`，会导致 EventParser 把“输入/日志”误判为“已发出事件”，
                // 从而造成重复路由、假阳性 completion、E2E 波动等问题。
                //
                // 因此：
                // - stderr 仍然会通过 `output_tx` 传给 Supervisor 做可观测输出（`[hat#n:err] ...`）
                // - 但不会进入 `HatJobResult.output`，以保证 event parsing 只基于 stdout（模型最终输出）
            }
        }
    }

    /// 尽最大努力终止子进程：
    /// 1) SIGTERM
    /// 2) 5s grace
    /// 3) SIGKILL（或 start_kill）
    async fn terminate_child(
        child: &mut tokio::process::Child,
        reason: &str,
    ) -> std::io::Result<()> {
        #[cfg(unix)]
        {
            use nix::sys::signal::{Signal, kill};
            use nix::unistd::Pid;

            if let Some(pid) = child.id() {
                #[allow(clippy::cast_possible_wrap)]
                let pid = Pid::from_raw(pid as i32);
                let _ = kill(pid, Signal::SIGTERM);
            }
        }

        #[cfg(not(unix))]
        {
            let _ = child.start_kill();
        }

        // grace period
        let wait_res = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
        match wait_res {
            Ok(Ok(_status)) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(_) => {
                warn!(%reason, "Job did not exit after SIGTERM, forcing kill");
                let _ = child.start_kill();
                let _ = child.wait().await;
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod guardrail_tests {
    use super::*;

    #[tokio::test]
    async fn parallel_output_for_event_parsing_is_stdout_only() {
        // ------------------------------------------------------------------
        // 目标:
        // - 锁死 parallel 模式的关键不变量: EventParser 的输入必须是 stdout-only.
        // - stderr 可能包含 prompt transcript / MCP 日志 / warnings 等,它们经常含 `<event ...>` 字样。
        //   一旦把 stderr 拼进 `HatJobResult.output`,就会产生“假事件/假 completion/重复路由”的回归。
        // ------------------------------------------------------------------

        let (output_tx, mut output_rx) = tokio::sync::mpsc::channel::<HatJobOutputChunk>(8);
        let instance_id = HatInstanceId::from("writer#1");
        let job_id = 42_u64;
        let mut output = String::new();

        // 1) stdout: 必须进入 output(用于事件解析).
        CliHatJobExecutor::handle_output_line(
            job_id,
            &instance_id,
            &output_tx,
            &mut output,
            OutputStream::Stdout,
            "hello".to_string(),
        )
        .await;

        assert_eq!(output, "hello\n");
        let chunk = output_rx.recv().await.expect("should receive stdout chunk");
        assert_eq!(chunk.job_id, job_id);
        assert_eq!(chunk.instance_id, instance_id);
        assert_eq!(chunk.stream, OutputStream::Stdout);
        assert_eq!(chunk.line, "hello");

        // 2) stderr: 仍要流式转发给 supervisor 做可观测输出,但绝不能污染 output.
        let stderr_line = "<event topic=\"build.task\">should_not_parse</event>".to_string();
        CliHatJobExecutor::handle_output_line(
            job_id,
            &instance_id,
            &output_tx,
            &mut output,
            OutputStream::Stderr,
            stderr_line.clone(),
        )
        .await;

        assert_eq!(
            output, "hello\n",
            "REGRESSION: stderr must NOT be appended to output used for event parsing"
        );
        let chunk = output_rx.recv().await.expect("should receive stderr chunk");
        assert_eq!(chunk.job_id, job_id);
        assert_eq!(chunk.instance_id, instance_id);
        assert_eq!(chunk.stream, OutputStream::Stderr);
        assert_eq!(chunk.line, stderr_line);
    }
}

/// 运行并行 HatInstance 调度器。
///
/// 说明：
/// - 目前 parallel 模式先走“日志输出”路径，TUI 仍沿用旧串行实现。
#[derive(Debug, Clone, Copy)]
pub struct ParallelLoopFlags {
    pub resume: bool,
    pub enable_tui: bool,
    pub plain: bool,
    pub show_stderr: bool,
    /// Headless/CI/E2E: 允许在 prompt 缺失/为空时启动并待机。
    pub idle_start: bool,
    /// 并行 TUI: 仅当没有 CLI prompt 覆盖时,才允许自动待机。
    pub allow_tui_auto_idle: bool,
}

pub async fn run_parallel_loop_impl(
    config: RalphConfig,
    color_mode: ColorMode,
    flags: ParallelLoopFlags,
    verbosity: Verbosity,
    record_session: Option<PathBuf>,
    instance_filters: Vec<String>,
    custom_args: Vec<String>,
) -> Result<TerminationReason> {
    process_management::setup_process_group();

    let resume = flags.resume;
    let plain = flags.plain;
    let show_stderr = flags.show_stderr;

    // TUI 需要 stdin/stdout 都是 TTY（crossterm 既要读键盘也要写屏幕）
    let enable_tui = flags.enable_tui && stdin().is_terminal() && stdout().is_terminal();
    let session_recorder: Option<Arc<SessionRecorder<std::io::BufWriter<File>>>> =
        if let Some(record_path) = record_session {
            let file = File::create(&record_path).with_context(|| {
                format!(
                    "Failed to create session recording file (parallel): {:?}",
                    record_path
                )
            })?;
            let recorder = Arc::new(SessionRecorder::new(std::io::BufWriter::new(file)));

            // 录制元信息：parallel 模式用更明确的 ux_mode，便于诊断
            recorder.record_meta(Record::meta_loop_start(
                &config.event_loop.prompt_file,
                config.event_loop.max_iterations,
                Some(if enable_tui {
                    "parallel-tui"
                } else {
                    "parallel-cli"
                }),
            ));

            debug!("Session recording enabled (parallel): {:?}", record_path);
            Some(recorder)
        } else {
            None
        };

    let use_colors = color_mode.should_use_colors();

    // prompt 解析(并行模式):
    //
    // 说明：
    // - 默认与串行保持一致：必须有 prompt。
    // - 但为了支持“并行 TUI 启动后待机(0 token)”体验,我们允许在特定条件下进入 idle_start:
    //   - headless/CI/E2E: 显式 `--idle-start`
    //   - 交互式 TUI: 默认 `PROMPT.md` 缺失/为空 + 无 CLI prompt 覆盖时自动待机
    //
    // 重要：
    // - idle_start 只在 fresh run 生效；resume/continue 不支持该语义。
    // - 即使进入 idle_start,我们仍会创建 `.ralph/current-events` marker 以支持 `ralph emit` 注入。
    let mut idle_start = false;
    let prompt_content = if resume {
        crate::loop_runner::resolve_prompt_content(&config.event_loop)?
    } else {
        // 1) 优先 inline prompt
        let inline = config
            .event_loop
            .prompt
            .as_deref()
            .map(|s| s.to_string())
            .unwrap_or_default();

        if !inline.trim().is_empty() {
            inline
        } else {
            // 2) prompt_file
            let prompt_file = config.event_loop.prompt_file.as_str();
            let default_prompt_file = prompt_file == "PROMPT.md";

            // 并行 TUI 的“自动待机”只在默认 PROMPT.md 丢失/为空时触发,避免吞掉显式配置错误。
            let can_auto_idle = enable_tui && flags.allow_tui_auto_idle && default_prompt_file;

            if prompt_file.is_empty() {
                if flags.idle_start {
                    idle_start = true;
                    String::new()
                } else {
                    crate::loop_runner::resolve_prompt_content(&config.event_loop)?
                }
            } else {
                let path = std::path::Path::new(prompt_file);
                if path.exists() {
                    let content = std::fs::read_to_string(path).with_context(|| {
                        format!("Failed to read prompt file (parallel): {}", prompt_file)
                    })?;

                    if content.trim().is_empty() && (flags.idle_start || can_auto_idle) {
                        idle_start = true;
                        String::new()
                    } else if content.trim().is_empty() {
                        anyhow::bail!(
                            "Prompt file '{}' is empty. Add content, or use `ralph run --idle-start` (parallel) to start idle.",
                            prompt_file
                        );
                    } else {
                        content
                    }
                } else if flags.idle_start || can_auto_idle {
                    idle_start = true;
                    String::new()
                } else {
                    crate::loop_runner::resolve_prompt_content(&config.event_loop)?
                }
            }
        }
    };

    // Create termination + interrupt signals for TUI lifecycle
    let (terminated_tx, terminated_rx) = watch::channel(false);
    let (interrupt_tx, mut interrupt_rx) = watch::channel(false);

    // Fresh run：创建本轮的 current-events marker，供 `ralph emit` 追加事件
    if !resume {
        let run_id = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
        let events_path = format!(".ralph/events-{}.jsonl", run_id);

        std::fs::create_dir_all(".ralph").context("Failed to create .ralph directory")?;
        std::fs::write(".ralph/current-events", &events_path)
            .context("Failed to write .ralph/current-events marker file")?;

        // Fresh run：清理旧 scratchpad，避免历史残留误导本次 objective。
        // 注意：resume/continue 模式下必须保留 scratchpad（作为恢复上下文的一部分）。
        let scratchpad_path = config.core.resolve_path(&config.core.scratchpad);
        crate::loop_runner::clear_scratchpad_for_fresh_run(&scratchpad_path, "parallel")?;
    }

    // headless(无 TUI)下的 idle_start 需要一个“明显信号”,否则看起来像卡住。
    // 注意：这是本地输出,不触发任何后端调用,不消耗 token。
    if idle_start && !enable_tui && !matches!(verbosity, Verbosity::Quiet) {
        let mut out = std::io::stdout().lock();
        let _ = writeln!(
            out,
            "[supervisor] idle_start 已启用: 正在等待外部事件. 例如: ralph emit human.message \"你的任务\" --target-instance ralph#1"
        );
    }

    let default_backend = CliBackend::from_config(&config.cli)
        .map_err(|e| anyhow::anyhow!("Failed to create backend from config: {e}"))?;
    let codex_mcp_runtime = Arc::new(CodexMcpRuntime::default());
    let codex_app_server_runtime = Arc::new(CodexAppServerRuntime::new(use_colors));

    let executor = Arc::new(CliHatJobExecutor {
        default_backend,
        custom_args,
        codex_mcp_runtime: Arc::clone(&codex_mcp_runtime),
        codex_app_server_runtime: Arc::clone(&codex_app_server_runtime),
    });

    let instance_filter_set: std::collections::HashSet<String> =
        instance_filters.into_iter().collect();

    // 初始化实例列表（用于 log 模式打印 / TUI 模式预注册）
    let mut initial_instances = Vec::new();
    for (hat_id, hat_cfg) in &config.hats {
        let n = hat_cfg.instances.max(1);
        for i in 1..=n {
            initial_instances.push(format!("{hat_id}#{i}"));
        }
    }
    initial_instances.push("ralph#1".to_string());
    initial_instances.sort();

    // TUI（并行 Supervisor UI）
    let (mut tui_handle, tui_update_tx) = if enable_tui {
        let mut tui = Tui::new_parallel()
            .with_parallel_markdown_rendering(!plain)
            .with_parallel_max_buffer_lines(config.tui.max_buffer_lines)
            .with_termination_signal(terminated_rx)
            .with_interrupt_tx(interrupt_tx.clone());

        // 右上角 Radar：best-effort 渲染 hats graph（失败不影响主流程）
        let registry = HatRegistry::from_config(&config);
        match crate::hats::render_hat_graph_radar_ascii(&config, &registry) {
            Ok(radar) => {
                tui = tui.with_hat_graph_radar(radar);
            }
            Err(e) => {
                warn!("Failed to render hat graph radar for parallel TUI: {e:#}");
            }
        }

        let update_tx = tui
            .update_sender()
            .expect("Tui::new_parallel() must have update_sender()");

        // 预注册实例（Created），让 UI 一启动就能看到列表
        for id in &initial_instances {
            let _ = update_tx.send(TuiUpdate::ParallelRegisterInstance {
                instance_id: HatInstanceId::from(id.as_str()),
                state: HatInstanceState::Created,
            });
        }

        (
            Some(tokio::spawn(async move { tui.run().await })),
            Some(update_tx),
        )
    } else {
        (None, None)
    };

    // 给 TUI 一点初始化时间（进入 alternate screen / raw mode）
    if tui_handle.is_some() {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // 并行模式下的输出/状态/gate 观察者
    type OutputObserver = Arc<dyn Fn(&HatJobOutputChunk) + Send + Sync>;
    type StateObserver = Arc<dyn Fn(&HatInstanceId, HatInstanceState) + Send + Sync>;
    type EventObserver = Arc<dyn Fn(&ralph_proto::Event) + Send + Sync>;

    let (observer, state_observer, event_observer): (
        OutputObserver,
        StateObserver,
        Option<EventObserver>,
    ) = if enable_tui {
        let update_tx = tui_update_tx.clone().expect("tui_update_tx must exist");

        let output_tx = update_tx.clone();
        let recorder_for_output = session_recorder.clone();
        let observer: OutputObserver = Arc::new(move |chunk: &HatJobOutputChunk| {
            // best-effort：把 stdout/stderr 写入 cassette（并行回放用）
            if let Some(recorder) = &recorder_for_output {
                let offset_ms = recorder.elapsed().as_millis() as u64;
                let mut line = chunk.line.clone();
                line.push('\n');
                let is_stdout = matches!(chunk.stream, OutputStream::Stdout);
                recorder.record_ux_event(&UxEvent::TerminalWrite(
                    TerminalWrite::new(line.as_bytes(), is_stdout, offset_ms)
                        .with_instance_id(chunk.instance_id.to_string()),
                ));
            }

            // 默认显示 stderr，便于调试。
            // 如需降噪，用 `ralph run --hide-stderr` 显式隐藏。
            //
            // 注意:
            // - 录制 cassette 与 "是否显示" 是两件事。
            // - 因此即使 hide stderr,我们仍然会把 stderr 写入 cassette(用于回放/诊断)。
            if !show_stderr && matches!(chunk.stream, OutputStream::Stderr) {
                return;
            }

            let _ = output_tx.send(TuiUpdate::ParallelOutputChunk(chunk.clone()));
        });

        let state_tx = update_tx.clone();
        let state_observer: StateObserver = Arc::new(move |instance_id: &HatInstanceId, state| {
            let _ = state_tx.send(TuiUpdate::ParallelInstanceState {
                instance_id: instance_id.clone(),
                state,
            });
        });

        let event_tx = update_tx;
        let recorder_for_events = session_recorder.clone();
        let event_observer: EventObserver = Arc::new(move |event: &ralph_proto::Event| {
            // best-effort：把 bus.publish 写入 cassette（便于诊断/提取命令）
            if let Some(recorder) = &recorder_for_events {
                recorder.record_bus_event(event);
            }
            // 事件转发策略：
            // - 控制面事件（gate.* / human.message）必须进 UI（Gate 面板/活跃度等）；
            // - 运行态可视化（Hat Graph Radar 边动画）需要“带 source 的业务事件”进 UI；
            // - 其余事件不转发，避免 UI 被高频噪音刷爆。
            if should_forward_event_to_tui(event) {
                let _ = event_tx.send(TuiUpdate::ParallelEvent(event.clone()));
            }
        });

        (observer, state_observer, Some(event_observer))
    } else {
        // 日志模式：沿用原有 stdout 打印
        // 6.1：实例列表（日志模式下的最小可用展示）
        if !matches!(verbosity, Verbosity::Quiet) {
            let mut out = std::io::stdout().lock();
            let _ = writeln!(out, "[supervisor] instances (initial=created):");
            for id in &initial_instances {
                let _ = writeln!(out, "  - {id}");
            }
        }

        // 输出观察者：按实例归因输出
        let stderr_hidden_hint_printed = Arc::new(AtomicBool::new(false));
        let recorder_for_output = session_recorder.clone();
        let observer: OutputObserver = Arc::new(move |chunk: &HatJobOutputChunk| {
            // best-effort：把 stdout/stderr 写入 cassette（并行回放用）
            if let Some(recorder) = &recorder_for_output {
                let offset_ms = recorder.elapsed().as_millis() as u64;
                let mut line = chunk.line.clone();
                line.push('\n');
                let is_stdout = matches!(chunk.stream, OutputStream::Stdout);
                recorder.record_ux_event(&UxEvent::TerminalWrite(
                    TerminalWrite::new(line.as_bytes(), is_stdout, offset_ms)
                        .with_instance_id(chunk.instance_id.to_string()),
                ));
            }

            if matches!(verbosity, Verbosity::Quiet) {
                return;
            }

            if !instance_filter_set.is_empty()
                && !instance_filter_set.contains(chunk.instance_id.as_str())
            {
                return;
            }

            // 默认显示 stderr 行，便于调试；如需降噪可显式隐藏。
            if !show_stderr && matches!(chunk.stream, OutputStream::Stderr) {
                // 仅在第一次“确实出现 stderr”时提醒一次，避免用户困惑但又不刷屏。
                if !stderr_hidden_hint_printed.swap(true, Ordering::Relaxed) {
                    let mut out = std::io::stdout().lock();
                    let _ = writeln!(
                        out,
                        "[supervisor] stderr streaming lines are hidden (via `--hide-stderr`); omit it to show them."
                    );
                }
                return;
            }

            // 使用 stdout 直接写，避免 tracing 与输出混用导致顺序错乱
            let mut out = std::io::stdout().lock();

            let stream_tag = match chunk.stream {
                OutputStream::Stdout => "out",
                OutputStream::Stderr => "err",
            };

            let line = format!(
                "[{}:{}:job={}] {}",
                chunk.instance_id, stream_tag, chunk.job_id, chunk.line
            );

            // stderr 用灰色显示，提高可读性。
            //
            // 注意:
            // - 如果 stderr 行本身带 ANSI(例如我们回显的 prompt transcript,或后端自身彩色日志),
            //   外层再包一层 GRAY 会破坏原始色彩语义。
            let is_stderr = matches!(chunk.stream, OutputStream::Stderr);
            let line_has_ansi = chunk.line.contains("\x1b[");
            if use_colors && is_stderr && !line_has_ansi {
                let _ = writeln!(out, "{}{}{}", colors::GRAY, line, colors::RESET);
            } else {
                let _ = writeln!(out, "{line}");
            }
        });

        // 6.1：状态变更展示（日志模式）
        let state_observer: StateObserver = Arc::new(move |instance_id: &HatInstanceId, state| {
            if matches!(verbosity, Verbosity::Quiet) {
                return;
            }
            let mut out = std::io::stdout().lock();
            let _ = writeln!(out, "[{}:state] {}", instance_id, state);
        });

        // 日志模式默认不展示事件；但若开启了 session recording，则仍记录 bus.publish
        let event_observer: Option<EventObserver> = session_recorder.clone().map(|recorder| {
            Arc::new(move |event: &ralph_proto::Event| {
                recorder.record_bus_event(event);
            }) as EventObserver
        });

        (observer, state_observer, event_observer)
    };

    // Spawn signal handlers AFTER TUI initialization to avoid deadlock
    // (TUI must enter raw mode and create EventStream before signal handlers are registered)
    let interrupt_tx_sigint = interrupt_tx.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            let _ = interrupt_tx_sigint.send(true);
        }
    });

    #[cfg(unix)]
    {
        let interrupt_tx_sigterm = interrupt_tx.clone();
        tokio::spawn(async move {
            let mut sigterm =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                    .expect("Failed to register SIGTERM handler");
            sigterm.recv().await;
            let _ = interrupt_tx_sigterm.send(true);
        });
    }

    #[cfg(unix)]
    {
        let interrupt_tx_sighup = interrupt_tx.clone();
        tokio::spawn(async move {
            let mut sighup = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup())
                .expect("Failed to register SIGHUP handler");
            sighup.recv().await;
            let _ = interrupt_tx_sighup.send(true);
        });
    }

    // Supervisor
    let mut supervisor = ParallelSupervisor::new(config, prompt_content, executor)?
        .with_agents_snapshot_to_default_path()
        .with_output_observer(observer)
        .with_instance_state_observer(state_observer)
        // 并行 TUI：completion promise（LOOP_COMPLETE）进入“暂停”而不是“退出”，并禁用动态实例回收，
        // 这样 human message 可以在会话中持续驱动下一轮对话/工作，而不会被 done/回收打断。
        .with_pause_on_completion_promise(enable_tui)
        .with_disable_dynamic_instance_reap(enable_tui)
        .with_idle_start(idle_start);
    if let Some(event_observer) = event_observer {
        supervisor = supervisor.with_event_observer(event_observer);
    }

    // Supervisor 与 interrupt 竞态：确保 Ctrl+C 能硬退出并清理子进程组
    let supervisor_future = supervisor.run(resume);
    tokio::pin!(supervisor_future);

    let result = loop {
        tokio::select! {
            res = &mut supervisor_future => break res,
            changed = interrupt_rx.changed() => {
                if changed.is_ok() && *interrupt_rx.borrow() {
                    #[cfg(unix)]
                    {
                        use nix::sys::signal::{killpg, Signal};
                        use nix::unistd::getpgrp;
                        let pgid = getpgrp();
                        let _ = killpg(pgid, Signal::SIGTERM);
                        tokio::time::sleep(Duration::from_millis(250)).await;
                        let _ = killpg(pgid, Signal::SIGKILL);
                    }

                    // 让 TUI 立即退出（如果还在跑）
                    let _ = terminated_tx.send(true);

                    // best-effort：即使被 Ctrl+C/SIGTERM/SIGHUP 打断,也要把 cassette 尾部刷盘,
                    // 否则用户会看到 record-session JSONL “没有回答就结束”。
                    if let Some(recorder) = &session_recorder {
                        let reason = TerminationReason::Interrupted;
                        let reason_str = format!("{reason:?}");
                        recorder.record_meta(Record::meta_termination(
                            &reason_str,
                            0, // 并行模式的 iteration 语义与串行不同，这里先写 0 做占位
                            recorder.elapsed().as_secs_f64(),
                            recorder.ux_write_count(),
                        ));
                        let _ = recorder.flush();
                    }

                    codex_mcp_runtime.shutdown_all().await;
                    codex_app_server_runtime.shutdown_all().await;
                    return Ok(TerminationReason::Interrupted);
                }
            }
        }
    };
    let result = match result {
        Ok(result) => result,
        Err(e) => {
            // best-effort：错误退出时也尽量刷盘,保留可审计证据.
            if let Some(recorder) = &session_recorder {
                let reason_str = format!("Error: {e:#}");
                recorder.record_meta(Record::meta_termination(
                    &reason_str,
                    0,
                    recorder.elapsed().as_secs_f64(),
                    recorder.ux_write_count(),
                ));
                let _ = recorder.flush();
            }
            codex_mcp_runtime.shutdown_all().await;
            codex_app_server_runtime.shutdown_all().await;
            return Err(e);
        }
    };

    // 6.1：结束时打印最终状态快照
    if !enable_tui && !matches!(verbosity, Verbosity::Quiet) {
        let mut pairs = result.instance_states.into_iter().collect::<Vec<_>>();
        pairs.sort_by(|a, b| a.0.as_str().cmp(b.0.as_str()));

        let mut out = std::io::stdout().lock();
        let _ = writeln!(out, "[supervisor] final states:");
        for (id, state) in pairs {
            let _ = writeln!(out, "  - {id}: {state}");
        }
    }

    let reason = result.termination.unwrap_or(TerminationReason::Stopped);

    // 自然结束：如果 TUI 开着，让用户按 q 退出（与串行模式对齐）
    if let Some(handle) = tui_handle.take() {
        let _ = handle.await;
    }

    // best-effort：写入 termination 元信息，便于 cassette 诊断/回放
    if let Some(recorder) = &session_recorder {
        let reason_str = format!("{reason:?}");
        recorder.record_meta(Record::meta_termination(
            &reason_str,
            0, // 并行模式的 iteration 语义与串行不同，这里先写 0 做占位
            recorder.elapsed().as_secs_f64(),
            recorder.ux_write_count(),
        ));
        let _ = recorder.flush();
    }

    codex_mcp_runtime.shutdown_all().await;
    codex_app_server_runtime.shutdown_all().await;

    Ok(reason)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ralph_proto::{Event, HatId};

    #[test]
    fn parallel_tui_event_forwarding_allows_gate_topics() {
        // gate.* 属于控制面事件：必须转发到 TUI（否则 Gate 面板无法更新）。
        let event = Event::new("gate.request", "");
        assert!(should_forward_event_to_tui(&event));
    }

    #[test]
    fn parallel_tui_event_forwarding_allows_human_message() {
        // human.message 会影响 TUI 的活跃度与最近事件展示。
        let event = Event::new("human.message", "");
        assert!(should_forward_event_to_tui(&event));
    }

    #[test]
    fn parallel_tui_event_forwarding_allows_events_with_source() {
        // 允许带 source 的事件进入 UI（兼容外部注入/特殊来源事件）。
        let event = Event::new("build.task", "").with_source(HatId::new("builder"));
        assert!(should_forward_event_to_tui(&event));
    }

    #[test]
    fn parallel_tui_event_forwarding_allows_events_with_source_instance() {
        // 并行模式下，业务事件通常只有 source_instance（发布者实例），TUI 可据此推导 hat_id。
        let event = Event::new("build.task", "").with_source_instance("builder#1");
        assert!(should_forward_event_to_tui(&event));
    }

    #[test]
    fn parallel_tui_event_forwarding_filters_noise_without_source_or_instance() {
        // 既没有 source，也没有 source_instance，且不是 gate.* / human.message：认为是 UI 噪音，不转发。
        let event = Event::new("build.task", "");
        assert!(!should_forward_event_to_tui(&event));
    }
}
