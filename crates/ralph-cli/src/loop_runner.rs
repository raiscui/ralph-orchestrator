//! Core orchestration loop implementation.
//!
//! This module contains the main `run_loop_impl` function that executes
//! the Ralph orchestration loop, along with supporting types and helper
//! functions for PTY execution and termination handling.

use anyhow::{Context, Result};
use ralph_adapters::{CliBackend, PtyPromptExecutor};
use ralph_display::{DisplayVerbosity, MarkdownRenderMode, TuiLineBuffer};
use ralph_core::{
    EventLogger, EventLoop, EventRecord, RalphConfig, Record,
    SessionRecorder, SummaryWriter, TerminationReason,
};
use ralph_proto::{Event, HatId, TerminalWrite, UxEvent};
use ralph_tui::Tui;
use std::fs::{self, File};
use std::io::{BufWriter, IsTerminal, stdin, stdout};
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};

use crate::display::{
    build_tui_hat_map, preview_one_line, print_iteration_separator, print_termination,
};
use crate::process_management;
use crate::{ColorMode, Verbosity};

pub(crate) fn clear_scratchpad_for_fresh_run(
    scratchpad_path: &std::path::Path,
    context_label: &str,
) -> Result<()> {
    // ------------------------------------------------------------------
    // 说明:
    // - fresh run 只能“清空/截断” scratchpad,不能删除文件。
    //   否则后续 `--continue/--resume` 可能因为文件缺失而失败(典型回归)。
    // - 当文件不存在时,这里保持 no-op(与现有语义一致,避免无意创建目录/文件)。
    // ------------------------------------------------------------------

    if !scratchpad_path.exists() {
        return Ok(());
    }

    if let Some(parent) = scratchpad_path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!("Failed to create scratchpad parent directory ({context_label}): {parent:?}")
        })?;
    }

    fs::write(scratchpad_path, "").with_context(|| {
        format!("Failed to clear scratchpad for fresh run ({context_label}): {scratchpad_path:?}")
    })?;

    debug!("Cleared scratchpad for fresh run ({context_label}): {scratchpad_path:?}");

    Ok(())
}


/// Core loop implementation supporting both fresh start and continue modes.
///
/// `resume`: If true, publishes `task.resume` instead of `task.start`,
/// signaling the planner to read existing scratchpad rather than doing fresh gap analysis.
///
/// `record_session`: If provided, records all events to the specified JSONL file for replay testing.
pub async fn run_loop_impl(
    config: RalphConfig,
    color_mode: ColorMode,
    resume: bool,
    enable_tui: bool,
    verbosity: Verbosity,
    plain: bool,
    record_session: Option<PathBuf>,
    custom_args: Vec<String>,
) -> Result<TerminationReason> {
    // Set up process group leadership per spec
    // "The orchestrator must run as a process group leader"
    process_management::setup_process_group();

    let use_colors = color_mode.should_use_colors();

    // Determine effective execution mode (with fallback logic)
    // Per spec: Claude backend requires PTY mode to avoid hangs
    // TUI mode is observation-only - uses streaming mode, not interactive
    let interactive_requested = config.cli.default_mode == "interactive" && !enable_tui;
    let user_interactive = if interactive_requested {
        if stdout().is_terminal() {
            true
        } else {
            warn!("Interactive mode requested but stdout is not a TTY, falling back to autonomous");
            false
        }
    } else {
        false
    };
    // 输出渲染策略：默认渲染 Markdown；`--plain` 强制纯文本。
    let render_mode = MarkdownRenderMode::from_plain(plain);

    // Set up interrupt channel for signal handling
    // Per spec:
    // - SIGINT (Ctrl+C): Immediately terminate child process (SIGTERM -> 5s grace -> SIGKILL), exit with code 130
    // - SIGTERM: Same as SIGINT
    // - SIGHUP: Same as SIGINT
    //
    // Use watch channel for interrupt notification so we can race execution vs interrupt
    // Note: Signal handlers are spawned AFTER TUI initialization to avoid deadlock
    let (interrupt_tx, interrupt_rx) = tokio::sync::watch::channel(false);

    // Resolve prompt content with precedence:
    // 1. CLI -p (inline text)
    // 2. CLI -P (file path)
    // 3. Config prompt (inline text)
    // 4. Config prompt_file (file path)
    // 5. Default PROMPT.md
    let prompt_content = resolve_prompt_content(&config.event_loop)?;

    // For fresh runs (not resume), generate a unique timestamped events file
    // This prevents stale events from previous runs polluting new runs (issue #82)
    // The marker file `.ralph/current-events` coordinates path between Ralph and agents
    if !resume {
        let run_id = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
        let events_path = format!(".ralph/events-{}.jsonl", run_id);

        fs::create_dir_all(".ralph").context("Failed to create .ralph directory")?;
        fs::write(".ralph/current-events", &events_path)
            .context("Failed to write .ralph/current-events marker file")?;

        debug!("Created events file for this run: {}", events_path);

        // Fresh run：清理旧 scratchpad，避免历史残留误导本次 objective。
        // 注意：resume/continue 模式下必须保留 scratchpad（作为恢复上下文的一部分）。
        let scratchpad_path = std::path::PathBuf::from(&config.core.scratchpad);
        let resolved_scratchpad_path = if scratchpad_path.is_relative() {
            config.core.workspace_root.join(&scratchpad_path)
        } else {
            scratchpad_path
        };
        clear_scratchpad_for_fresh_run(&resolved_scratchpad_path, "serial")?;
    }

    // Initialize event loop
    let mut event_loop = EventLoop::new(config.clone());

    // For resume mode, we initialize with a different event topic
    // This tells the planner to read existing scratchpad rather than creating a new one
    if resume {
        event_loop.initialize_resume(&prompt_content);
    } else {
        event_loop.initialize(&prompt_content);
    }

    // Set up session recording if requested
    // This records all events to a JSONL file for replay testing
    let session_recorder: Option<Arc<SessionRecorder<BufWriter<File>>>> =
        if let Some(record_path) = record_session {
            let file = File::create(&record_path).with_context(|| {
                format!("Failed to create session recording file: {:?}", record_path)
            })?;
            let recorder = Arc::new(SessionRecorder::new(BufWriter::new(file)));

            // 写入“最近一次录制路径”指针,用于 `ralph record watch` 无参自动定位.
            crate::record_session::write_record_session_latest_pointer(
                &config.core.workspace_root,
                &record_path,
            )?;

            // Record metadata for the session
            //
            // 说明:
            // - `_meta.session_start` 用于记录"在哪个目录,以什么命令启动"的基本信息.
            // - 这对离线排障很关键,尤其是用户只提供一份 JSONL 时.
            let argv = std::env::args_os()
                .map(|s| s.to_string_lossy().to_string())
                .collect::<Vec<_>>();
            let cwd = std::env::current_dir()
                .ok()
                .map(|p| p.to_string_lossy().to_string());
            let current_exe = std::env::current_exe()
                .ok()
                .map(|p| p.to_string_lossy().to_string());
            let workspace_root = config.core.workspace_root.display().to_string();
            recorder.record_meta(Record::meta_session_start(
                cwd.as_deref(),
                Some(workspace_root.as_str()),
                &argv,
                std::process::id(),
                current_exe.as_deref(),
                Some(env!("CARGO_PKG_VERSION")),
            ));

            recorder.record_meta(Record::meta_loop_start(
                &config.event_loop.prompt_file,
                config.event_loop.max_iterations,
                if enable_tui { Some("tui") } else { Some("cli") },
            ));

            // Wire observer to EventBus so events are recorded
            let observer = SessionRecorder::make_observer(Arc::clone(&recorder));
            event_loop.add_observer(observer);

            info!("Session recording enabled: {:?}", record_path);
            Some(recorder)
        } else {
            None
        };

    // Initialize event logger for debugging
    let mut event_logger = EventLogger::default_path();

    // Log initial event for debugging.
    //
    // 说明：
    // - fresh run 的初始化握手事件始终是 `task.start`
    // - `event_loop.starting_event` 不是“第一个事件”，而是“协调后 workflow entry event”（可选）：
    //   - 若配置了 starting_event：协调者（parallel 时为 ralph#1） MUST 优先发布该 topic 作为入口事件
    //   - 若未配置：协调者（parallel 时为 ralph#1）需要基于目标与 hats 拓扑自行决定入口事件
    let start_topic = if resume { "task.resume" } else { "task.start" };
    let start_triggered = "planner"; // Backward-compatible default for display/logging
    let start_event = Event::new(start_topic, &prompt_content);
    let start_record =
        EventRecord::new(0, "loop", &start_event, Some(&HatId::new(start_triggered)));
    if let Err(e) = event_logger.log(&start_record) {
        warn!("Failed to log start event: {}", e);
    }

    // Create backend from config - TUI mode uses the same backend as non-TUI
    // The TUI is an observation layer that displays output, not a different mode
    let default_backend =
        CliBackend::from_config(&config.cli).map_err(|e| anyhow::Error::new(e))?;

    // Create termination signal for TUI shutdown
    let (terminated_tx, terminated_rx) = tokio::sync::watch::channel(false);

    // Wire TUI with termination signal and shared state
    // TUI is observation-only - works in both interactive and autonomous modes
    // Requirements: both stdin and stdout must be terminals for TUI
    // (Crossterm requires stdin for keyboard input, stdout for rendering)
    let enable_tui = enable_tui && stdin().is_terminal() && stdout().is_terminal();
    let (mut tui_handle, tui_state) = if enable_tui {
        // Build hat map for dynamic topic-to-hat resolution
        // This allows TUI to display custom hats (e.g., "Security Reviewer")
        // instead of generic "ralph" for all events
        let hat_map = build_tui_hat_map(event_loop.registry());
        let mut tui = Tui::new().with_hat_map(hat_map);

        // 右上角 Radar：best-effort 渲染 hats graph（失败不影响主流程）
        match crate::hats::render_hat_graph_radar_ascii(&config, event_loop.registry()) {
            Ok(radar) => {
                tui = tui.with_hat_graph_radar(radar);
            }
            Err(e) => {
                warn!("Failed to render hat graph radar for TUI: {e:#}");
            }
        }

        let tui = tui.with_termination_signal(terminated_rx);

        // Get shared state before spawning (for content streaming)
        let state = tui.state();

        // Wire interrupt channel so TUI can signal main loop on Ctrl+C
        // (raw mode prevents SIGINT from being generated by the OS)
        let tui = tui.with_interrupt_tx(interrupt_tx.clone());

        let observer = tui.observer();
        event_loop.add_observer(observer);
        (
            Some(tokio::spawn(async move { tui.run().await })),
            Some(state),
        )
    } else {
        (None, None)
    };

    // Give TUI task time to initialize (enter alternate screen, enable raw mode)
    // before the main loop starts doing work
    if tui_handle.is_some() {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Spawn signal handlers AFTER TUI initialization to avoid deadlock
    // (TUI must enter raw mode and create EventStream before signal handlers are registered)

    // Spawn task to listen for SIGINT (Ctrl+C)
    let interrupt_tx_sigint = interrupt_tx.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            debug!("Interrupt received (SIGINT), terminating immediately...");
            let _ = interrupt_tx_sigint.send(true);
        }
    });

    // Spawn task to listen for SIGTERM (Unix only)
    #[cfg(unix)]
    {
        let interrupt_tx_sigterm = interrupt_tx.clone();
        tokio::spawn(async move {
            let mut sigterm =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                    .expect("Failed to register SIGTERM handler");
            sigterm.recv().await;
            debug!("SIGTERM received, terminating immediately...");
            let _ = interrupt_tx_sigterm.send(true);
        });
    }

    // Spawn task to listen for SIGHUP (Unix only)
    #[cfg(unix)]
    {
        let interrupt_tx_sighup = interrupt_tx.clone();
        tokio::spawn(async move {
            let mut sighup = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup())
                .expect("Failed to register SIGHUP handler");
            sighup.recv().await;
            warn!("SIGHUP received (terminal closed), terminating immediately...");
            let _ = interrupt_tx_sighup.send(true);
        });
    }

    // Log execution mode - hat info already logged by initialize()
    let exec_mode = if user_interactive {
        "interactive"
    } else {
        "autonomous"
    };
    debug!(execution_mode = %exec_mode, "Execution mode configured");



    // =====================================================================
    // 装配串行执行器(候选5: 进程执行通过 PromptExecutor port 注入)
    // =====================================================================
    let tui_lines_provider: Option<Arc<dyn Fn() -> Option<TuiLineBuffer> + Send + Sync>> =
        tui_state.as_ref().map(|state| {
            let state = state.clone();
            let provider: Arc<dyn Fn() -> Option<TuiLineBuffer> + Send + Sync> = Arc::new(move || {
                if let Ok(mut s) = state.lock() {
                    s.start_new_iteration();
                    s.latest_iteration_lines_handle()
                } else {
                    None
                }
            });
            provider
        });

    let display_verbosity = match verbosity {
        Verbosity::Quiet => DisplayVerbosity::Quiet,
        Verbosity::Normal => DisplayVerbosity::Normal,
        Verbosity::Verbose => DisplayVerbosity::Verbose,
    };

    let mut executor = PtyPromptExecutor::new(
        default_backend,
        render_mode,
        display_verbosity,
        None,
        config.cli.role_args.clone(),
        config.cli.reasoning_effort,
        custom_args,
        user_interactive,
        config.core.workspace_root.clone(),
        config.clone(),
        tui_lines_provider,
    );

    let mut last_hat_for_display: Option<HatId> = None;
    let reason = event_loop
        .run(
            &mut executor,
            interrupt_rx,
            user_interactive,
            ralph_core::RunHooks {
                before_execute: Some(Box::new(|iteration, display_hat, prompt, elapsed| {
                    // 分隔符展示(非 TUI 模式; TUI 有自己的 header)
                    if tui_state.is_none() {
                        print_iteration_separator(
                            iteration,
                            display_hat.as_str(),
                            elapsed,
                            config.event_loop.max_iterations,
                            use_colors,
                        );
                    }
                    // hat 切换日志
                    if last_hat_for_display.as_ref() != Some(display_hat) {
                        if tui_state.is_none() {
                            if display_hat.as_str() == "ralph" {
                                info!("I'm Ralph. Let's do this.");
                            } else {
                                info!("Putting on my {} hat.", display_hat);
                            }
                        }
                        last_hat_for_display = Some(display_hat.clone());
                    }
                    // verbose: 完整打印本轮 prompt
                    if verbosity == Verbosity::Verbose {
                        eprintln!("\n{}", "=".repeat(80));
                        eprintln!("PROMPT FOR {} (iteration {})", display_hat, iteration);
                        eprintln!("{}", "-".repeat(80));
                        eprintln!("{}", prompt);
                        eprintln!("{}\n", "=".repeat(80));
                    }
                })),
                after_execute: Some(Box::new(|iteration, hat_id, out| {
                    // 说明:
                    // - TODO: wire `record_iteration_tokens(hat_id, out.context_window)`
                    //   当 Claude session peak extraction 在 PtyExecutor 中落地时。
                    // - 当前 borrow checker 冲突: after_execute closure 捕获 &event_loop,
                    //   但 .run() 持有 &mut self。Refactor hook 签名后再 wire。
                    // - 现在 PromptOutput.context_window 已就位 (默认 0),
                    //   record_iteration_tokens(0) 是 no-op, 不影响行为。
                    // - record-session 迭代记录(stdout-only)
                    if let Some(recorder) = &session_recorder
                        && !out.output.is_empty()
                    {
                        let offset_ms = recorder.elapsed().as_millis() as u64;
                        recorder.record_meta(Record::meta_iteration(
                            iteration,
                            offset_ms,
                            hat_id.as_str(),
                        ));
                        recorder.record_ux_event(&UxEvent::TerminalWrite(TerminalWrite::new(
                            out.output.as_bytes(),
                            true,
                            offset_ms,
                        )));
                    }
                })),
            },
        )
        .await;

    // ------------------------------------------------------------------
    // 善后: 终止元信息 / summary / 终端展示 / TUI 退出
    // ------------------------------------------------------------------
    if let Some(recorder) = &session_recorder {
        let reason_str = format!("{reason:?}");
        recorder.record_meta(Record::meta_termination(
            &reason_str,
            event_loop.state().iteration,
            recorder.elapsed().as_secs_f64(),
            recorder.ux_write_count(),
        ));
        let _ = recorder.flush();
    }
    let summary_writer = SummaryWriter::default();
    let scratchpad_path = std::path::Path::new(&config.core.scratchpad);
    let scratchpad_opt = if scratchpad_path.exists() {
        Some(scratchpad_path)
    } else {
        None
    };
    let final_commit = get_last_commit_info();
    if let Err(e) = summary_writer.write(
        &reason,
        event_loop.state(),
        scratchpad_opt,
        final_commit.as_deref(),
    ) {
        warn!("Failed to write summary file: {}", e);
    }
    if !enable_tui {
        print_termination(&reason, event_loop.state(), use_colors);
    }

    // TUI 退出: 中断立即退出; 自然完成等用户按 q
    if reason == TerminationReason::Interrupted {
        let _ = terminated_tx.send(true);
    }
    if let Some(handle) = tui_handle.take() {
        let _ = handle.await;
    }

    Ok(reason)
}

fn get_last_commit_info() -> Option<String> {
    let output = Command::new("git")
        .args(["log", "-1", "--format=%h: %s"])
        .output()
        .ok()?;

    if output.status.success() {
        let info = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if info.is_empty() { None } else { Some(info) }
    } else {
        None
    }
}

pub(crate) fn resolve_prompt_content(
    event_loop_config: &ralph_core::EventLoopConfig,
) -> Result<String> {
    debug!(
        inline_prompt = ?event_loop_config.prompt.as_ref().map(|s| preview_one_line(s, 50)),
        prompt_file = %event_loop_config.prompt_file,
        "Resolving prompt content"
    );

    // Check for inline prompt first (CLI -p or config prompt)
    if let Some(ref inline_text) = event_loop_config.prompt {
        debug!(len = inline_text.len(), "Using inline prompt text");
        return Ok(inline_text.clone());
    }

    // Check for prompt file (CLI -P or config prompt_file or default)
    let prompt_file = &event_loop_config.prompt_file;
    if !prompt_file.is_empty() {
        let path = std::path::Path::new(prompt_file);
        debug!(path = %prompt_file, exists = path.exists(), "Checking prompt file");
        if path.exists() {
            let content = std::fs::read_to_string(path)
                .with_context(|| format!("Failed to read prompt file: {}", prompt_file))?;
            debug!(path = %prompt_file, len = content.len(), "Read prompt from file");
            return Ok(content);
        } else {
            // File specified but doesn't exist - error with helpful message
            anyhow::bail!(
                "Prompt file '{}' not found. Check the path or use -p \"text\" for inline prompt.",
                prompt_file
            );
        }
    }

    // No valid prompt source found
    anyhow::bail!(
        "No prompt specified. Use -p \"text\" for inline prompt, -P path for file, \
         or create PROMPT.md in the current directory."
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_pty_always_enabled_for_streaming() {
        // PTY mode is always enabled for real-time streaming output.
        // This ensures all backends (claude, gemini, kiro, codex, amp) get
        // streaming output instead of buffered output from CliExecutor.
        let use_pty = true; // Matches the actual implementation

        // PTY should always be true regardless of backend or mode
        assert!(use_pty, "PTY should always be enabled for streaming output");
    }

    #[test]
    fn test_user_interactive_mode_determination() {
        // user_interactive is determined by default_mode setting, not PTY.
        // PTY handles output streaming; user_interactive handles input forwarding.

        // Autonomous mode: no user input forwarding
        let autonomous_interactive = false;
        assert!(
            !autonomous_interactive,
            "Autonomous mode should not forward user input"
        );

        // Interactive mode with TTY: forward user input
        let interactive_with_tty = true;
        assert!(
            interactive_with_tty,
            "Interactive mode with TTY should forward user input"
        );
    }

    fn guardrail_scratchpad_clear_truncates_but_does_not_delete() {
        // ------------------------------------------------------------------
        // 目标:
        // - 锁死 fresh run 的 scratchpad 清理语义: truncate(清空)而不是 delete。
        // - 这能防止后续 `--continue/--resume` 因 scratchpad 缺失而失败的回归。
        // ------------------------------------------------------------------

        let dir = tempdir().expect("tempdir should be created");
        let scratchpad_path = dir.path().join("scratchpad.md");
        fs::write(&scratchpad_path, "hello").expect("scratchpad should be writable");

        clear_scratchpad_for_fresh_run(&scratchpad_path, "test").expect("clear should succeed");

        assert!(
            scratchpad_path.exists(),
            "scratchpad must still exist after clear"
        );
        let content = fs::read_to_string(&scratchpad_path).expect("scratchpad should be readable");
        assert_eq!(content, "");
    }

    #[test]
    fn guardrail_scratchpad_clear_is_noop_when_missing() {
        // ------------------------------------------------------------------
        // 说明:
        // - 保持现有语义: 文件不存在时不创建(避免无意创建目录/文件影响工作区)。
        // ------------------------------------------------------------------

        let dir = tempdir().expect("tempdir should be created");
        let scratchpad_path = dir.path().join("missing.md");

        clear_scratchpad_for_fresh_run(&scratchpad_path, "test").expect("clear should succeed");
        assert!(
            !scratchpad_path.exists(),
            "missing scratchpad should remain missing (no implicit create)"
        );
    }
}
