//! CLI executor for running prompts through backends.
//!
//! Executes prompts via CLI tools with real-time streaming output.
//! Supports optional execution timeout (stall watchdog) with graceful SIGTERM termination.

use crate::cli_backend::CliBackend;
#[cfg(test)]
use crate::cli_backend::{OutputFormat, PromptMode};
#[cfg(unix)]
use nix::sys::signal::{Signal, kill};
#[cfg(unix)]
use nix::unistd::Pid;
use std::io::Write;
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tracing::{debug, warn};

#[derive(Debug, Clone, Copy)]
enum StreamKind {
    Stdout,
    Stderr,
}

/// Result of a CLI execution.
#[derive(Debug)]
pub struct ExecutionResult {
    /// The full output from the CLI.
    pub output: String,
    /// Whether the execution succeeded (exit code 0).
    pub success: bool,
    /// The exit code.
    pub exit_code: Option<i32>,
    /// Whether the execution was terminated due to timeout.
    ///
    /// 说明：
    /// - “超时”这里指的是“检测超时”（stall watchdog）：
    ///   - 到检测窗口不会立刻终止进程；
    ///   - 只有当输出在 `output_stale_timeout` 内没有变化，才会判定超时并终止。
    pub timed_out: bool,
}

/// Executor for running prompts through CLI backends.
#[derive(Debug)]
pub struct CliExecutor {
    backend: CliBackend,
}

impl CliExecutor {
    /// Creates a new executor with the given backend.
    pub fn new(backend: CliBackend) -> Self {
        Self { backend }
    }

    /// Executes a prompt and streams output to the provided writer.
    ///
    /// Output is streamed line-by-line to the writer while being accumulated
    /// for the return value.
    ///
    /// Timeout 语义：
    /// - `timeout` 是“检测窗口”（check interval），不是硬超时。
    /// - 当检测窗口到期时：
    ///   - 若 `output_stale_timeout` 存在且输出已停滞超过该阈值：判定超时并终止
    ///   - 否则判定通过，并把检测窗口重新从当前时刻开始计时
    ///
    /// When `verbose` is true, stderr output is also written to the output writer
    /// with a `[stderr]` prefix. When false, stderr is captured but not displayed.
    pub async fn execute<W: Write + Send>(
        &self,
        prompt: &str,
        mut output_writer: W,
        timeout: Option<Duration>,
        output_stale_timeout: Option<Duration>,
        verbose: bool,
    ) -> std::io::Result<ExecutionResult> {
        // Note: _temp_file is kept alive for the duration of this function scope.
        // For large prompts (>7000 chars), Claude reads from the temp file.
        let (cmd, args, stdin_input, _temp_file) = self.backend.build_command(prompt, false);

        let mut command = Command::new(&cmd);
        command.args(&args);
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        // Set working directory to current directory (mirrors PTY executor behavior)
        // Use fallback to "." if current_dir fails (e.g., E2E test workspaces)
        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        command.current_dir(&cwd);

        debug!(
            command = %cmd,
            args = ?args,
            cwd = ?cwd,
            "Spawning CLI command"
        );

        if stdin_input.is_some() {
            command.stdin(Stdio::piped());
        }

        let mut child = command.spawn()?;

        // Write to stdin if needed
        if let Some(input) = stdin_input
            && let Some(mut stdin) = child.stdin.take()
        {
            stdin.write_all(input.as_bytes()).await?;
            drop(stdin); // Close stdin to signal EOF
        }

        let mut output = String::new();
        let mut timed_out = false;
        let mut last_output_changed_at = Instant::now();

        // 并发读取 stdout/stderr，避免 pipe buffer deadlock，并提供“检测超时”所需的进展信号。
        let stdout_handle = child.stdout.take();
        let stderr_handle = child.stderr.take();

        let (line_tx, mut line_rx) = mpsc::channel::<(StreamKind, String)>(256);

        // stdout
        let stdout_tx = line_tx.clone();
        let stdout_task = tokio::spawn(async move {
            if let Some(stdout) = stdout_handle {
                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();
                while let Some(line) = lines.next_line().await? {
                    if stdout_tx.send((StreamKind::Stdout, line)).await.is_err() {
                        break;
                    }
                }
            }
            Ok::<(), std::io::Error>(())
        });

        // stderr
        let stderr_tx = line_tx.clone();
        let stderr_task = tokio::spawn(async move {
            if let Some(stderr) = stderr_handle {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                while let Some(line) = lines.next_line().await? {
                    if stderr_tx.send((StreamKind::Stderr, line)).await.is_err() {
                        break;
                    }
                }
            }
            Ok::<(), std::io::Error>(())
        });

        // 释放主 sender，让 rx 能在两个 reader 结束后自然关闭
        drop(line_tx);

        // 收集输出，直到流结束 or 超时触发
        match timeout.filter(|d| !d.is_zero()) {
            Some(check_interval) => {
                debug!(
                    timeout_secs = check_interval.as_secs(),
                    stale_timeout_secs = output_stale_timeout.map(|d| d.as_secs()),
                    "Executing with watchdog timeout"
                );

                // “检测超时”语义：
                // - check_interval 到期时不立刻 kill
                // - 只有当输出停滞超过 `output_stale_timeout` 才判定 timed_out
                // - 若输出仍在变化：判定通过，并把检测窗口从此刻重新计时
                let mut next_check_deadline = tokio::time::Instant::now() + check_interval;
                let sleep = tokio::time::sleep_until(next_check_deadline);
                tokio::pin!(sleep);

                loop {
                    tokio::select! {
                        // 检测窗口到期：根据“输出是否停滞”决定是否超时
                        _ = &mut sleep => {
                            match output_stale_timeout {
                                Some(stale_timeout) => {
                                    if last_output_changed_at.elapsed() >= stale_timeout {
                                        warn!(
                                            timeout_secs = check_interval.as_secs(),
                                            stale_timeout_secs = stale_timeout.as_secs(),
                                            "Watchdog timeout reached and output is stale; sending SIGTERM"
                                        );
                                        timed_out = true;
                                        Self::terminate_child(&mut child)?;
                                        break;
                                    }

                                    // 检测通过：检测窗口重新计时（从现在开始）
                                    next_check_deadline = tokio::time::Instant::now() + check_interval;
                                    sleep.as_mut().reset(next_check_deadline);
                                }
                                None => {
                                    // 兜底：若未提供 stale 阈值，则退化为“硬超时”
                                    warn!(
                                        timeout_secs = check_interval.as_secs(),
                                        "Watchdog timeout reached with no stale threshold; sending SIGTERM"
                                    );
                                    timed_out = true;
                                    Self::terminate_child(&mut child)?;
                                    break;
                                }
                            }
                        }
                        line = line_rx.recv() => {
                            let Some((stream, line)) = line else {
                                break;
                            };

                            last_output_changed_at = Instant::now();

                            match stream {
                                StreamKind::Stdout => {
                                    writeln!(output_writer, "{line}")?;
                                    output.push_str(&line);
                                    output.push('\n');
                                }
                                StreamKind::Stderr => {
                                    if verbose {
                                        writeln!(output_writer, "[stderr] {line}")?;
                                    }
                                    output.push_str("[stderr] ");
                                    output.push_str(&line);
                                    output.push('\n');
                                }
                            }

                            output_writer.flush()?;
                        }
                    }
                }
            }
            None => {
                while let Some((stream, line)) = line_rx.recv().await {
                    match stream {
                        StreamKind::Stdout => {
                            writeln!(output_writer, "{line}")?;
                            output.push_str(&line);
                            output.push('\n');
                        }
                        StreamKind::Stderr => {
                            if verbose {
                                writeln!(output_writer, "[stderr] {line}")?;
                            }
                            output.push_str("[stderr] ");
                            output.push_str(&line);
                            output.push('\n');
                        }
                    }

                    output_writer.flush()?;
                }
            }
        }

        let status = child.wait().await?;

        // 等待读取 task 收尾（best-effort）
        let _ = stdout_task.await;
        let _ = stderr_task.await;

        Ok(ExecutionResult {
            output,
            success: status.success() && !timed_out,
            exit_code: status.code(),
            timed_out,
        })
    }

    /// Terminates the child process with SIGTERM.
    fn terminate_child(child: &mut tokio::process::Child) -> std::io::Result<()> {
        #[cfg(not(unix))]
        {
            // SIGTERM doesn't exist on Windows. Best-effort termination:
            // On Unix this would be SIGKILL, on Windows it maps to process termination.
            child.start_kill()
        }

        #[cfg(unix)]
        if let Some(pid) = child.id() {
            #[allow(clippy::cast_possible_wrap)]
            let pid = Pid::from_raw(pid as i32);
            debug!(%pid, "Sending SIGTERM to child process");
            let _ = kill(pid, Signal::SIGTERM);
            Ok(())
        } else {
            Ok(())
        }
    }

    /// Executes a prompt without streaming (captures all output).
    ///
    /// Uses no timeout by default. For timed execution, use `execute_capture_with_timeout`.
    pub async fn execute_capture(&self, prompt: &str) -> std::io::Result<ExecutionResult> {
        self.execute_capture_with_timeout(prompt, None).await
    }

    /// Executes a prompt without streaming, with optional timeout.
    pub async fn execute_capture_with_timeout(
        &self,
        prompt: &str,
        timeout: Option<Duration>,
    ) -> std::io::Result<ExecutionResult> {
        // Use a sink that discards output for non-streaming execution
        // verbose=false since output is being discarded anyway
        let sink = std::io::sink();
        // 注意：capture 模式没有 stdout/stderr 进度展示，默认退化为“硬超时”更可预测。
        self.execute(prompt, sink, timeout, None, false).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_execute_echo() {
        // Use echo as a simple test backend
        let backend = CliBackend {
            command: "echo".to_string(),
            args: vec![],
            prompt_mode: PromptMode::Arg,
            prompt_flag: None,
            output_format: OutputFormat::Text,
        };

        let executor = CliExecutor::new(backend);
        let mut output = Vec::new();

        let result = executor
            .execute("hello world", &mut output, None, None, true)
            .await
            .unwrap();

        assert!(result.success);
        assert!(!result.timed_out);
        assert!(result.output.contains("hello world"));
    }

    #[tokio::test]
    async fn test_execute_stdin() {
        // Use cat to test stdin mode
        let backend = CliBackend {
            command: "cat".to_string(),
            args: vec![],
            prompt_mode: PromptMode::Stdin,
            prompt_flag: None,
            output_format: OutputFormat::Text,
        };

        let executor = CliExecutor::new(backend);
        let result = executor.execute_capture("stdin test").await.unwrap();

        assert!(result.success);
        assert!(result.output.contains("stdin test"));
    }

    #[tokio::test]
    async fn test_execute_failure() {
        let backend = CliBackend {
            command: "false".to_string(), // Always exits with code 1
            args: vec![],
            prompt_mode: PromptMode::Arg,
            prompt_flag: None,
            output_format: OutputFormat::Text,
        };

        let executor = CliExecutor::new(backend);
        let result = executor.execute_capture("").await.unwrap();

        assert!(!result.success);
        assert!(!result.timed_out);
        assert_eq!(result.exit_code, Some(1));
    }

    #[tokio::test]
    async fn test_execute_timeout() {
        // Use sleep to test timeout behavior
        // The sleep command ignores stdin, so we use PromptMode::Stdin
        // to avoid appending the prompt as an argument
        let backend = CliBackend {
            command: "sleep".to_string(),
            args: vec!["10".to_string()],   // Sleep for 10 seconds
            prompt_mode: PromptMode::Stdin, // Use stdin mode so prompt doesn't interfere
            prompt_flag: None,
            output_format: OutputFormat::Text,
        };

        let executor = CliExecutor::new(backend);

        // “检测超时”语义下：check_interval 到期时会先判断“输出是否停滞”。
        // sleep 不产生任何输出，所以当 stale 阈值足够小，应该在第一次检查时触发超时。
        let check_interval = Some(Duration::from_millis(100));
        let stale_timeout = Some(Duration::from_millis(50));
        let result = executor
            .execute("", std::io::sink(), check_interval, stale_timeout, false)
            .await
            .unwrap();

        assert!(result.timed_out, "Expected execution to time out");
        assert!(
            !result.success,
            "Timed out execution should not be successful"
        );
    }

    #[tokio::test]
    async fn test_execute_watchdog_does_not_timeout_when_output_is_active() {
        // 使用 sh 产生持续输出，验证“检测窗口到期时，只要输出在 stale 阈值内有变化，就不会超时”
        let backend = CliBackend {
            command: "sh".to_string(),
            args: vec![
                "-c".to_string(),
                "for i in 1 2 3 4 5; do echo tick-$i; sleep 0.05; done".to_string(),
            ],
            prompt_mode: PromptMode::Stdin,
            prompt_flag: None,
            output_format: OutputFormat::Text,
        };

        let executor = CliExecutor::new(backend);

        // 让检测窗口至少触发 1-2 次，同时保证输出频率足够高（< stale 阈值）
        let check_interval = Some(Duration::from_millis(100));
        // CI/高负载机器上调度抖动更明显，阈值过小会导致该用例偶发误判超时。
        let stale_timeout = Some(Duration::from_millis(400));
        let result = executor
            .execute("", std::io::sink(), check_interval, stale_timeout, false)
            .await
            .unwrap();

        assert!(
            !result.timed_out,
            "Active output should prevent watchdog timeout"
        );
        assert!(result.success, "Process should complete successfully");
        assert!(result.output.contains("tick-1"));
    }

    #[tokio::test]
    async fn test_execute_no_timeout_when_fast() {
        // Use echo which completes immediately
        let backend = CliBackend {
            command: "echo".to_string(),
            args: vec![],
            prompt_mode: PromptMode::Arg,
            prompt_flag: None,
            output_format: OutputFormat::Text,
        };

        let executor = CliExecutor::new(backend);

        // Execute with a generous timeout - should complete before timeout
        let timeout = Some(Duration::from_secs(10));
        let result = executor
            .execute_capture_with_timeout("fast", timeout)
            .await
            .unwrap();

        assert!(!result.timed_out, "Fast command should not time out");
        assert!(result.success);
        assert!(result.output.contains("fast"));
    }
}
