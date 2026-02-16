//! Ralph execution for E2E tests.
//!
//! This module provides functionality to execute `ralph run` with test configurations
//! and capture all output including stdout, stderr, exit code, and artifacts from
//! the `.agent/` directory.
//!
//! # Example
//!
//! ```no_run
//! use ralph_e2e::executor::{RalphExecutor, ScenarioConfig, PromptSource};
//! use std::path::PathBuf;
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() {
//!     let executor = RalphExecutor::new(PathBuf::from(".e2e-tests/test-scenario"));
//!
//!     let config = ScenarioConfig {
//!         config_file: PathBuf::from("ralph.yml"),
//!         prompt: PromptSource::Inline("Say hello".to_string()),
//!         max_iterations: 1,
//!         timeout: Duration::from_secs(60),
//!         extra_args: vec![],
//!     };
//!
//!     let result = executor.run(&config).await.unwrap();
//!     println!("Exit code: {:?}", result.exit_code);
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;

/// Configuration for a test scenario.
#[derive(Debug, Clone)]
pub struct ScenarioConfig {
    /// Path to ralph.yml for this test (relative to workspace).
    pub config_file: PathBuf,

    /// Prompt to send to the agent.
    pub prompt: PromptSource,

    /// Maximum iterations for this test.
    pub max_iterations: u32,

    /// Timeout for the entire test.
    pub timeout: Duration,

    /// Additional CLI arguments.
    pub extra_args: Vec<String>,
}

impl ScenarioConfig {
    /// Creates a minimal config for basic connectivity tests.
    pub fn minimal(prompt: impl Into<String>) -> Self {
        Self {
            config_file: PathBuf::from("ralph.yml"),
            prompt: PromptSource::Inline(prompt.into()),
            max_iterations: 1,
            timeout: Duration::from_secs(300), // 5 minutes - Claude iterations can take 60-120s
            extra_args: vec![],
        }
    }
}

/// Source of the prompt for a test.
#[derive(Debug, Clone)]
pub enum PromptSource {
    /// Prompt loaded from a file.
    File(PathBuf),
    /// Inline prompt string.
    Inline(String),
    /// Use the prompt defined in the scenario's `ralph.yml` (do not pass `-p`).
    ///
    /// 说明：
    /// - 用于“直接跑仓库 example 配置”的场景：我们希望验证示例本身可用，
    ///   而不是被 E2E runner 的额外提示词影响。
    /// - 该模式下，E2E 仍会通过 `--max-iterations` / `--no-tui` 等 CLI 参数提供测试护栏。
    Config,
}

/// Result of executing Ralph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Exit code from the ralph process (None if killed by signal).
    pub exit_code: Option<i32>,

    /// Full stdout output.
    pub stdout: String,

    /// Full stderr output.
    pub stderr: String,

    /// How long the execution took.
    #[serde(with = "duration_serde")]
    pub duration: Duration,

    /// Content of scratchpad after execution, if present.
    pub scratchpad: Option<String>,

    /// Events parsed from JSONL logs (primary source for assertions).
    pub events: Vec<EventRecord>,

    /// Number of iterations completed.
    pub iterations: u32,

    /// Reason for termination, if detected.
    pub termination_reason: Option<String>,

    /// Whether the execution timed out.
    pub timed_out: bool,
}

/// A recorded event from Ralph execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecord {
    /// Event topic (e.g., "build.done", "task.complete").
    pub topic: String,

    /// Event payload content.
    pub payload: String,

    /// Event source instance id (parallel mode), e.g. "writer#1".
    ///
    /// 说明：
    /// - 该字段来自 `.ralph/events.jsonl` 的 `source_instance`（可选）。
    /// - 串行/历史事件可能没有该字段，因此保持可选。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_instance: Option<String>,
}

/// Errors that can occur during Ralph execution.
#[derive(Debug, Error)]
pub enum ExecutorError {
    /// Failed to spawn the ralph process.
    #[error("failed to spawn ralph: {0}")]
    SpawnError(#[from] std::io::Error),

    /// Workspace directory doesn't exist.
    #[error("workspace does not exist: {0}")]
    WorkspaceNotFound(PathBuf),

    /// Config file doesn't exist.
    #[error("config file does not exist: {0}")]
    ConfigNotFound(PathBuf),

    /// Ralph binary not found.
    #[error("ralph binary not found")]
    RalphNotFound,

    /// Execution timed out.
    #[error("execution timed out after {0:?}")]
    Timeout(Duration),
}

/// 从当前目录向上查找 workspace root（以 `Cargo.toml` 为锚点）。
///
/// 说明：
/// - 这个函数用于 mock-mode 下解析 cassette 目录等“相对 repo root 的路径”。
/// - 找不到时返回 `None`，上层再决定如何处理（比如退回到相对路径）。
pub fn find_workspace_root() -> Option<PathBuf> {
    let mut current = std::env::current_dir().ok()?;

    loop {
        if current.join("Cargo.toml").exists() {
            return Some(current);
        }

        current = current.parent()?.to_path_buf();
    }
}

/// Resolves the path to the ralph binary.
///
/// Resolution order:
/// 1. `target/release/ralph` (prefer optimized builds)
/// 2. `target/debug/ralph` (development builds)
/// 3. Falls back to "ralph" (PATH lookup)
///
/// This ensures e2e tests run against the locally built code, not a system-installed version.
pub fn resolve_ralph_binary() -> PathBuf {
    if let Some(root) = find_workspace_root() {
        // Check for release binary first (faster)
        let release_binary = root.join("target/release/ralph");
        if release_binary.exists() {
            return release_binary;
        }

        // Fall back to debug binary
        let debug_binary = root.join("target/debug/ralph");
        if debug_binary.exists() {
            return debug_binary;
        }
    }

    // Fall back to PATH lookup
    PathBuf::from("ralph")
}

/// Executes Ralph with test configurations.
#[derive(Debug, Clone)]
pub struct RalphExecutor {
    /// Path to the workspace directory for this scenario.
    workspace: PathBuf,

    /// Optional path to the ralph binary (defaults to finding it in PATH).
    ralph_binary: Option<PathBuf>,
}

impl RalphExecutor {
    /// Creates a new executor for the given workspace.
    ///
    /// The workspace should already exist and contain a ralph.yml config file.
    pub fn new(workspace: PathBuf) -> Self {
        Self {
            workspace,
            ralph_binary: None,
        }
    }

    /// Creates a new executor with a specific ralph binary path.
    pub fn with_binary(workspace: PathBuf, ralph_binary: PathBuf) -> Self {
        Self {
            workspace,
            ralph_binary: Some(ralph_binary),
        }
    }

    /// Returns the workspace path.
    pub fn workspace(&self) -> &PathBuf {
        &self.workspace
    }

    /// Returns the ralph binary that will be used.
    pub fn ralph_binary(&self) -> PathBuf {
        self.ralph_binary
            .clone()
            .unwrap_or_else(|| PathBuf::from("ralph"))
    }

    /// Executes ralph with the given configuration.
    pub async fn run(&self, config: &ScenarioConfig) -> Result<ExecutionResult, ExecutorError> {
        self.run_with_timeout(config, config.timeout).await
    }

    /// Executes ralph with a specific timeout.
    pub async fn run_with_timeout(
        &self,
        config: &ScenarioConfig,
        timeout: Duration,
    ) -> Result<ExecutionResult, ExecutorError> {
        use std::process::Stdio;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::process::Command;
        use tokio::time::Instant;

        // Verify workspace exists
        if !self.workspace.exists() {
            return Err(ExecutorError::WorkspaceNotFound(self.workspace.clone()));
        }

        let config_path = self.workspace.join(&config.config_file);
        if !config_path.exists() {
            return Err(ExecutorError::ConfigNotFound(config_path));
        }

        let start = Instant::now();

        // Build the command
        // Note: Pass config_file (not full config_path) because current_dir is set to workspace
        let mut cmd = Command::new(self.ralph_binary());
        cmd.arg("run")
            .arg("-c")
            .arg(&config.config_file)
            .arg("--max-iterations")
            .arg(config.max_iterations.to_string())
            .current_dir(&self.workspace)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            // Always enable diagnostics for E2E tests to aid debugging
            .env("RALPH_DIAGNOSTICS", "1")
            // Pass workspace root so Ralph resolves paths correctly in E2E tests
            .env("RALPH_WORKSPACE_ROOT", &self.workspace)
            // Use Haiku for faster, cheaper E2E tests
            .env("CLAUDE_MODEL", "haiku");

        // Unix: make Ralph the leader of a new process group.
        //
        // Why:
        // - Ralph 会在运行期继续 spawn backend 子进程（claude/codex/…）。
        // - E2E 超时/卡死时必须能“一刀切”杀掉整组，避免子进程残留污染下一次测试。
        // - 只有在新进程组里，`kill(-pgid, SIGTERM)` 才可靠。
        //
        // 备注：
        // - 本仓库禁止使用 `unsafe`，因此这里使用标准库提供的安全 API。
        #[cfg(unix)]
        cmd.process_group(0);

        // Handle prompt
        match &config.prompt {
            PromptSource::File(path) => {
                cmd.arg("-p").arg(format!("@{}", path.display()));
            }
            PromptSource::Inline(prompt) => {
                cmd.arg("-p").arg(prompt);
            }
            PromptSource::Config => {
                // 不传 `-p`：让 `ralph run` 使用 `ralph.yml` 里的 `event_loop.prompt`。
            }
        }

        // Add extra args
        for arg in &config.extra_args {
            cmd.arg(arg);
        }

        // Spawn the process
        let mut child = cmd.spawn()?;

        // Close stdin to signal no more input
        if let Some(mut stdin) = child.stdin.take() {
            stdin.shutdown().await.ok();
        }

        // Capture stdout/stderr concurrently so we can:
        // - preserve partial output on timeout (useful for debugging)
        // - avoid `wait_with_output()` which consumes the child and prevents killing on timeout
        let stdout_handle = child.stdout.take();
        let stderr_handle = child.stderr.take();

        let stdout_task = tokio::spawn(async move {
            let mut buf = Vec::new();
            if let Some(mut stdout) = stdout_handle {
                let _ = stdout.read_to_end(&mut buf).await;
            }
            buf
        });

        let stderr_task = tokio::spawn(async move {
            let mut buf = Vec::new();
            if let Some(mut stderr) = stderr_handle {
                let _ = stderr.read_to_end(&mut buf).await;
            }
            buf
        });

        let mut timed_out = false;
        let status = match tokio::time::timeout(timeout, child.wait()).await {
            Ok(Ok(status)) => Some(status),
            Ok(Err(e)) => return Err(ExecutorError::SpawnError(e)),
            Err(_) => {
                timed_out = true;
                // 强制终止整个进程组，避免 backend 子进程残留（硬门槛）。
                self.terminate_process_group(&mut child, "e2e timeout")
                    .await
            }
        };

        let duration = start.elapsed();

        let stdout_bytes = stdout_task.await.unwrap_or_default();
        let stderr_bytes = stderr_task.await.unwrap_or_default();

        let stdout = String::from_utf8_lossy(&stdout_bytes).to_string();
        let stderr = String::from_utf8_lossy(&stderr_bytes).to_string();

        // -----------------------------------------------------------------
        // E2E 调试产物: 落盘 stdout/stderr
        //
        // 说明：
        // - E2E runner 的报告通常只展示“断言摘要”，stdout 的全文不一定会写入 report.json。
        // - 当场景在真实后端上漂移/失败时，stdout/stderr 是定位根因的最高价值证据。
        // - 因此这里做 best-effort 落盘(失败不影响测试结果)。
        // -----------------------------------------------------------------
        self.write_e2e_output_artifacts(&stdout, &stderr).await;

        // Read scratchpad if it exists
        let scratchpad = self.read_scratchpad().await;

        // Read events from JSONL file (primary source)
        let events = self.read_events_from_jsonl().await;

        // Count iterations from output
        let iterations = self.count_iterations(&stdout);

        // Detect termination reason
        let termination_reason = if timed_out {
            Some("TIMEOUT".to_string())
        } else {
            self.detect_termination_reason(&stdout)
        };

        Ok(ExecutionResult {
            exit_code: status.and_then(|s| s.code()),
            stdout,
            stderr,
            duration,
            scratchpad,
            events,
            iterations,
            termination_reason,
            timed_out,
        })
    }

    async fn write_e2e_output_artifacts(&self, stdout: &str, stderr: &str) {
        // 说明：
        // - 防止极端情况下 stdout/stderr 爆量把 workspace 撑大，这里做一个字符级上限。
        // - 200k 对排障足够(包含多个实例的归因前缀 + 关键事件),同时避免无意义的超大文件。
        const MAX_CHARS: usize = 200_000;

        let dir = self.workspace.join(".e2e");
        if tokio::fs::create_dir_all(&dir).await.is_err() {
            return;
        }

        let stdout_path = dir.join("stdout.txt");
        let stderr_path = dir.join("stderr.txt");

        let stdout_text = truncate_with_notice(stdout, MAX_CHARS);
        let stderr_text = truncate_with_notice(stderr, MAX_CHARS);

        let _ = tokio::fs::write(stdout_path, stdout_text).await;
        let _ = tokio::fs::write(stderr_path, stderr_text).await;
    }

    /// 终止 `ralph run` 的进程组（Unix）或单进程（非 Unix）。
    ///
    /// 说明：
    /// - Ralph 会成为进程组 leader，并在组内继续 spawn backend 子进程。
    /// - E2E timeout 时必须强杀整个进程组，避免后台残留影响下一次测试。
    async fn terminate_process_group(
        &self,
        child: &mut tokio::process::Child,
        _reason: &str,
    ) -> Option<std::process::ExitStatus> {
        #[cfg(unix)]
        {
            use nix::sys::signal::{Signal, kill};
            use nix::unistd::{Pid, getpgid};
            use std::time::Duration;

            if let Some(pid_u32) = child.id() {
                // 优先通过 OS 查询 pgid（更可靠），失败再回退到 pid。
                #[allow(clippy::cast_possible_wrap)]
                let pid = pid_u32 as i32;
                let pgid = getpgid(Some(Pid::from_raw(pid)))
                    .map(|p| p.as_raw())
                    .unwrap_or(pid);

                let _ = kill(Pid::from_raw(-pgid), Signal::SIGTERM);

                // grace period
                let wait_res = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
                match wait_res {
                    Ok(Ok(status)) => return Some(status),
                    _ => {
                        let _ = kill(Pid::from_raw(-pgid), Signal::SIGKILL);
                        return tokio::time::timeout(Duration::from_secs(5), child.wait())
                            .await
                            .ok()
                            .and_then(|r| r.ok());
                    }
                }
            }
        }

        // Non-unix or missing pid: best-effort kill the child process.
        let _ = child.start_kill();
        child.wait().await.ok()
    }

    /// Reads the scratchpad file from the workspace.
    async fn read_scratchpad(&self) -> Option<String> {
        let scratchpad_path = self.workspace.join(".agent").join("scratchpad.md");
        tokio::fs::read_to_string(scratchpad_path).await.ok()
    }

    /// Reads events from .ralph/events.jsonl file.
    ///
    /// 说明：
    /// - `.ralph/events.jsonl`：Ralph 自己写入的“调试事件日志”（包含内部事件）
    /// - `.ralph/current-events` 指向的文件：`ralph emit` 追加的“外部事件输入”（可能为空）
    ///
    /// E2E 断言主要依赖 `.ralph/events.jsonl`，但这里也会 best-effort 合并外部事件文件，
    /// 方便需要 `ralph emit` 的场景（以及排障时的完整性）。
    async fn read_events_from_jsonl(&self) -> Vec<EventRecord> {
        let debug_events_path = self.workspace.join(".ralph/events.jsonl");
        let events_marker = self.workspace.join(".ralph").join("current-events");

        let mut paths = vec![debug_events_path.clone()];
        if let Ok(rel_path) = tokio::fs::read_to_string(&events_marker).await {
            let marker_path = self.workspace.join(rel_path.trim());
            if marker_path != debug_events_path && tokio::fs::metadata(&marker_path).await.is_ok() {
                paths.push(marker_path);
            }
        }

        let mut events = Vec::new();
        for path in paths {
            if let Ok(content) = tokio::fs::read_to_string(&path).await {
                for line in content.lines().filter(|l| !l.trim().is_empty()) {
                    let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
                        continue;
                    };
                    let Some(topic) = value.get("topic").and_then(|v| v.as_str()) else {
                        continue;
                    };

                    let payload = match value.get("payload") {
                        Some(serde_json::Value::String(s)) => s.clone(),
                        Some(serde_json::Value::Null) | None => String::new(),
                        Some(other) => {
                            serde_json::to_string(other).unwrap_or_else(|_| other.to_string())
                        }
                    };

                    let source_instance = value
                        .get("source_instance")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    events.push(EventRecord {
                        topic: topic.to_string(),
                        payload,
                        source_instance,
                    });
                }
            }
        }

        events
    }

    /// Counts iterations from the output.
    ///
    /// Ralph outputs iteration markers like "[Iteration 1]" or similar.
    fn count_iterations(&self, output: &str) -> u32 {
        // Look for patterns like "[Iteration N]" or "Iteration N" or "[iter N]"
        let iter_regex = regex::Regex::new(r"(?i)\[?\s*iter(?:ation)?\s*(\d+)\s*\]?").unwrap();

        let mut max_iter = 0;
        for cap in iter_regex.captures_iter(output) {
            if let Some(num) = cap.get(1)
                && let Ok(n) = num.as_str().parse::<u32>()
            {
                max_iter = max_iter.max(n);
            }
        }

        max_iter
    }

    /// Detects the termination reason from output.
    fn detect_termination_reason(&self, output: &str) -> Option<String> {
        if output.contains("LOOP_COMPLETE") {
            return Some("LOOP_COMPLETE".to_string());
        }
        if output.contains("max iterations") || output.contains("max-iterations") {
            return Some("MAX_ITERATIONS".to_string());
        }
        if output.contains("timeout") || output.contains("timed out") {
            return Some("TIMEOUT".to_string());
        }
        None
    }
}

/// Serde helper for Duration serialization.
mod duration_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_secs_f64().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = f64::deserialize(deserializer)?;
        Ok(Duration::from_secs_f64(secs))
    }
}

fn truncate_with_notice(input: &str, max_chars: usize) -> String {
    // 说明：
    // - 这里按“字符数”裁剪,避免把多字节 UTF-8 截断成非法字节序列。
    // - 为了让排障者知道这是被截断的,会追加一个固定尾注。
    let count = input.chars().count();
    if count <= max_chars {
        return input.to_string();
    }

    let mut out = input.chars().take(max_chars).collect::<String>();
    out.push_str(&format!(
        "\n... [truncated for e2e artifact, {} chars total]\n",
        count
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::path::Path;

    /// Creates a unique test workspace path.
    fn test_workspace(test_name: &str) -> PathBuf {
        env::temp_dir().join(format!(
            "ralph-e2e-executor-{}-{}",
            test_name,
            std::process::id()
        ))
    }

    /// Sets up a test workspace with a minimal ralph.yml.
    fn setup_workspace(path: &Path) {
        fs::create_dir_all(path.join(".agent")).unwrap();
        fs::write(
            path.join("ralph.yml"),
            r"cli:
  backend: claude
  max_iterations: 1
",
        )
        .unwrap();
    }

    /// Cleans up a test workspace.
    fn cleanup_workspace(path: &PathBuf) {
        if path.exists() {
            fs::remove_dir_all(path).ok();
        }
    }

    #[test]
    fn test_resolve_ralph_binary_finds_local_or_path() {
        let binary = super::resolve_ralph_binary();
        // Should return something - either a local build or "ralph" for PATH
        let binary_str = binary.to_string_lossy();
        assert!(
            binary_str.contains("target/debug/ralph")
                || binary_str.contains("target/release/ralph")
                || binary_str == "ralph",
            "Expected local build path or 'ralph', got: {}",
            binary_str
        );
    }

    #[test]
    fn test_executor_new() {
        let workspace = PathBuf::from("/tmp/test-workspace");
        let executor = RalphExecutor::new(workspace.clone());
        assert_eq!(executor.workspace(), &workspace);
        assert_eq!(executor.ralph_binary(), PathBuf::from("ralph"));
    }

    #[test]
    fn test_executor_with_binary() {
        let workspace = PathBuf::from("/tmp/test-workspace");
        let binary = PathBuf::from("/usr/local/bin/ralph");
        let executor = RalphExecutor::with_binary(workspace.clone(), binary.clone());
        assert_eq!(executor.workspace(), &workspace);
        assert_eq!(executor.ralph_binary(), binary);
    }

    #[test]
    fn test_scenario_config_minimal() {
        let config = ScenarioConfig::minimal("Say hello");
        assert_eq!(config.config_file, PathBuf::from("ralph.yml"));
        assert!(matches!(config.prompt, PromptSource::Inline(p) if p == "Say hello"));
        assert_eq!(config.max_iterations, 1);
        assert_eq!(config.timeout, Duration::from_secs(300));
        assert!(config.extra_args.is_empty());
    }

    #[test]
    fn test_count_iterations_none() {
        let executor = RalphExecutor::new(PathBuf::from("/tmp"));
        let count = executor.count_iterations("no iteration markers here");
        assert_eq!(count, 0);
    }

    #[test]
    fn test_count_iterations_single() {
        let executor = RalphExecutor::new(PathBuf::from("/tmp"));
        let count = executor.count_iterations("[Iteration 1] Starting...");
        assert_eq!(count, 1);
    }

    #[test]
    fn test_count_iterations_multiple() {
        let executor = RalphExecutor::new(PathBuf::from("/tmp"));
        let output = "[Iteration 1] First\n[Iteration 2] Second\n[Iteration 3] Third";
        let count = executor.count_iterations(output);
        assert_eq!(count, 3);
    }

    #[test]
    fn test_count_iterations_short_format() {
        let executor = RalphExecutor::new(PathBuf::from("/tmp"));
        let output = "[iter 1] First\n[iter 2] Second";
        let count = executor.count_iterations(output);
        assert_eq!(count, 2);
    }

    #[test]
    fn test_detect_termination_loop_complete() {
        let executor = RalphExecutor::new(PathBuf::from("/tmp"));
        let reason = executor.detect_termination_reason("Task done. LOOP_COMPLETE");
        assert_eq!(reason, Some("LOOP_COMPLETE".to_string()));
    }

    #[test]
    fn test_detect_termination_max_iterations() {
        let executor = RalphExecutor::new(PathBuf::from("/tmp"));
        let reason = executor.detect_termination_reason("Reached max iterations, stopping");
        assert_eq!(reason, Some("MAX_ITERATIONS".to_string()));
    }

    #[test]
    fn test_detect_termination_none() {
        let executor = RalphExecutor::new(PathBuf::from("/tmp"));
        let reason = executor.detect_termination_reason("normal output");
        assert!(reason.is_none());
    }

    #[tokio::test]
    async fn test_run_workspace_not_found() {
        let workspace = PathBuf::from("/nonexistent/workspace");
        let executor = RalphExecutor::new(workspace.clone());
        let config = ScenarioConfig::minimal("test");

        let result = executor.run(&config).await;
        assert!(matches!(result, Err(ExecutorError::WorkspaceNotFound(_))));
    }

    #[tokio::test]
    async fn test_run_config_not_found() {
        let workspace = test_workspace("config-not-found");
        fs::create_dir_all(&workspace).unwrap();

        let executor = RalphExecutor::new(workspace.clone());
        let config = ScenarioConfig::minimal("test");

        let result = executor.run(&config).await;
        assert!(matches!(result, Err(ExecutorError::ConfigNotFound(_))));

        cleanup_workspace(&workspace);
    }

    #[tokio::test]
    async fn test_execution_result_serialization() {
        let result = ExecutionResult {
            exit_code: Some(0),
            stdout: "hello".to_string(),
            stderr: String::new(),
            duration: Duration::from_secs_f64(1.5),
            scratchpad: Some("# Notes".to_string()),
            events: vec![EventRecord {
                topic: "build.done".to_string(),
                payload: "success".to_string(),
                source_instance: None,
            }],
            iterations: 2,
            termination_reason: Some("LOOP_COMPLETE".to_string()),
            timed_out: false,
        };

        // Serialize to JSON
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"exit_code\":0"));
        assert!(json.contains("\"stdout\":\"hello\""));
        assert!(json.contains("\"duration\":1.5"));

        // Deserialize back
        let parsed: ExecutionResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.exit_code, Some(0));
        assert_eq!(parsed.stdout, "hello");
        assert_eq!(parsed.iterations, 2);
    }

    // Integration test that requires ralph binary - skip in CI
    #[tokio::test]
    #[ignore = "requires ralph binary"]
    async fn test_run_real_ralph() {
        let workspace = test_workspace("real-ralph");
        setup_workspace(&workspace);

        let executor = RalphExecutor::new(workspace.clone());
        let config = ScenarioConfig::minimal("Say 'test passed'");

        let result = executor.run(&config).await;

        // Clean up regardless of result
        cleanup_workspace(&workspace);

        // Verify execution
        let result = result.expect("ralph should execute");
        assert!(
            !result.stdout.is_empty() || !result.stderr.is_empty(),
            "should have output"
        );
    }
}
