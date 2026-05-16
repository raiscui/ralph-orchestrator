//! # ralph-cli
//!
//! Binary entry point for the Ralph Orchestrator.
//!
//! This crate provides:
//! - CLI argument parsing using `clap`
//! - Application initialization and configuration
//! - Entry point to the headless orchestration loop
//! - Event history viewing via `ralph events`
//! - Project initialization via `ralph init`
//! - SOP-based planning via `ralph plan`
//! - Code task generation via `ralph code-task`
//! - Work item tracking via `ralph task`

mod answer;
mod autopilot;
mod capability;
mod codex_app_server_session;
mod codex_mcp_session;
mod display;
mod doctor;
mod hats;
mod init;
mod loop_runner;
mod memory;
mod parallel_runner;
mod presets;
mod record_cli;
mod record_session;
mod runtime_graph;
mod sop_runner;
mod startup_resources;
mod task_cli;
mod tools;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use ralph_adapters::detect_backend;
use ralph_core::{
    EventHistory, RalphConfig, StateClearRequest, StateMode, StateOperationStore,
    agent_guidance_manifest::{DEFAULT_AGENT_GUIDANCE_MANIFEST, verify_manifest_at_with_report},
};
use std::ffi::OsStr;
use std::fs;
use std::io::{IsTerminal, Write, stdout};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::{info, warn};

// Unix-specific process management for process group leadership
#[cfg(unix)]
mod process_management {
    use nix::unistd::{Pid, getpgrp, setpgid, tcgetpgrp};
    use std::io::{IsTerminal, stdin, stdout};
    use tracing::debug;

    /// Sets up process group leadership.
    ///
    /// Per spec: "The orchestrator must run as a process group leader. All spawned
    /// CLI processes (Claude, Kiro, etc.) belong to this group. On termination,
    /// the entire process group receives the signal, preventing orphans."
    pub fn setup_process_group() {
        let pid = Pid::this();
        let pgrp = getpgrp();

        // 尽量让自己成为进程组 leader（用于后续对子进程做“整组清理”）。
        // 但当我们被 wrapper（例如 `npx`）启动时，强行 setpgid 可能让我们脱离前台 TTY 进程组，
        // 从而导致 TUI 键盘输入失效/卡死。因此这里做一次“是否前台组”的保护判断。
        if pgrp == pid {
            debug!("Already process group leader: PID {pid}");
            return;
        }

        if is_foreground_tty_group(pgrp) {
            debug!("Skipping setpgid: keeping foreground process group {pgrp}");
            return;
        }

        if let Err(e) = setpgid(pid, pid) {
            // EPERM is OK - we're already a process group leader (e.g., started from shell)
            if e != nix::errno::Errno::EPERM {
                debug!("Note: Could not set process group ({e}), continuing anyway");
            }
        }
        debug!("Process group initialized: PID {pid}");
    }

    fn is_foreground_tty_group(current_pgrp: Pid) -> bool {
        // 优先用 stdin 判断前台进程组；不行再回退到 stdout。
        if stdin().is_terminal()
            && let Ok(foreground_pgrp) = tcgetpgrp(stdin())
        {
            return foreground_pgrp == current_pgrp;
        }

        if stdout().is_terminal()
            && let Ok(foreground_pgrp) = tcgetpgrp(stdout())
        {
            return foreground_pgrp == current_pgrp;
        }

        false
    }
}

#[cfg(not(unix))]
mod process_management {
    /// No-op on non-Unix platforms.
    pub fn setup_process_group() {}
}

/// Installs a panic hook that restores terminal state before printing panic info.
///
/// When a TUI application panics, the terminal can be left in a broken state:
/// - Raw mode enabled (input not line-buffered)
/// - Alternate screen buffer active (no scrollback)
/// - Cursor hidden
///
/// This hook ensures the terminal is restored so the panic message is visible
/// and the user can scroll/interact normally.
fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        // Restore terminal state before printing panic info
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(
            std::io::stdout(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::cursor::Show
        );
        // Call the default panic hook to print the panic message
        default_hook(panic_info);
    }));
}

/// Color output mode for terminal display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum ColorMode {
    /// Automatically detect if stdout is a TTY
    #[default]
    Auto,
    /// Always use colors
    Always,
    /// Never use colors
    Never,
}

impl ColorMode {
    /// Returns true if colors should be used based on mode and terminal detection.
    fn should_use_colors(self) -> bool {
        match self {
            ColorMode::Always => true,
            ColorMode::Never => false,
            ColorMode::Auto => stdout().is_terminal(),
        }
    }
}

/// Verbosity level for streaming output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Verbosity {
    /// Suppress all streaming output (for CI/scripting)
    Quiet,
    /// Show assistant text and tool invocations (default)
    #[default]
    Normal,
    /// Show everything including tool results and session summary
    Verbose,
}

impl Verbosity {
    /// Resolves verbosity from CLI args, env vars, and config.
    ///
    /// Precedence (highest to lowest):
    /// 1. CLI flags: `--verbose`/`-v` or `--quiet`/`-q`
    /// 2. Environment variables: `RALPH_VERBOSE=1` or `RALPH_QUIET=1`
    /// 3. Config file: (if supported in future)
    /// 4. Default: Normal
    fn resolve(cli_verbose: bool, cli_quiet: bool) -> Self {
        // CLI flags take precedence
        if cli_quiet {
            return Verbosity::Quiet;
        }
        if cli_verbose {
            return Verbosity::Verbose;
        }

        // Environment variables
        if std::env::var("RALPH_QUIET").is_ok() {
            return Verbosity::Quiet;
        }
        if std::env::var("RALPH_VERBOSE").is_ok() {
            return Verbosity::Verbose;
        }

        Verbosity::Normal
    }
}

/// 判断用户是否显式传入了全局 `--config` / `-c`。
///
/// 说明:
/// - startup bootstrap 只在“没有显式 config source”时触发。
/// - `Cli` 解析后的 `config: PathBuf` 无法区分默认值和显式传入同名路径。
/// - 因此这里在 clap 解析前扫描原始 argv,遇到 `--` 后停止,避免把 backend 自定义参数误判为 Ralph config。
fn cli_config_was_explicit<'a>(args: impl IntoIterator<Item = &'a OsStr>) -> bool {
    for arg in args.into_iter().skip(1) {
        if arg == OsStr::new("--") {
            return false;
        }

        let value = arg.to_string_lossy();
        if value == "--config" || value.starts_with("--config=") {
            return true;
        }
        if value == "-c" || (value.starts_with("-c") && value.len() > 2) {
            return true;
        }
    }

    false
}

/// Output format for events command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum OutputFormat {
    /// Human-readable table format
    #[default]
    Table,
    /// JSON format for programmatic access
    Json,
}

// Re-export colors from display module for use in this file
use display::colors;

/// Source for configuration: file path, builtin preset, or remote URL.
#[derive(Debug, Clone)]
pub enum ConfigSource {
    /// Local file path (default behavior)
    File(PathBuf),
    /// Builtin preset name (e.g., "builtin:tdd-red-green")
    Builtin(String),
    /// Remote URL (e.g., "http://example.com/preset.yml")
    Remote(String),
}

impl ConfigSource {
    /// Parse a config source string into its variant.
    ///
    /// Format:
    /// - `builtin:preset-name` → Builtin preset
    /// - `http://...` or `https://...` → Remote URL
    /// - Anything else → File path
    fn parse(s: &str) -> Self {
        if let Some(name) = s.strip_prefix("builtin:") {
            ConfigSource::Builtin(name.to_string())
        } else if s.starts_with("http://") || s.starts_with("https://") {
            ConfigSource::Remote(s.to_string())
        } else {
            ConfigSource::File(PathBuf::from(s))
        }
    }
}

/// Ralph Orchestrator - Multi-agent orchestration framework
#[derive(Parser, Debug)]
#[command(name = "ralph", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    // ─────────────────────────────────────────────────────────────────────────
    // Global options (available for all subcommands)
    // ─────────────────────────────────────────────────────────────────────────
    /// Path to configuration file
    #[arg(short, long, default_value = "ralph.yml", global = true)]
    config: PathBuf,

    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Color output mode (auto, always, never)
    #[arg(long, value_enum, default_value_t = ColorMode::Auto, global = true)]
    color: ColorMode,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run the orchestration loop (default if no subcommand given)
    Run(RunArgs),

    /// DEPRECATED: Use `ralph run --continue` instead.
    /// Resume a previously interrupted loop from existing scratchpad.
    #[command(hide = true)]
    Resume(ResumeArgs),

    /// View event history for debugging
    Events(EventsArgs),

    /// List active hat instances (parallel mode)
    Agents(AgentsArgs),

    /// Initialize a new ralph.yml configuration file
    Init(InitArgs),

    /// Clean up Ralph artifacts (.agent/ directory)
    Clean(CleanArgs),

    /// Emit an event to the current run's events file with proper JSON formatting
    Emit(EmitArgs),

    /// Start a Prompt-Driven Development planning session
    Plan(PlanArgs),

    /// Generate code task files from descriptions or plans
    CodeTask(CodeTaskArgs),

    /// Create code tasks (alias for code-task)
    Task(CodeTaskArgs),

    /// Ralph's runtime tools (agent-facing)
    Tools(tools::ToolsArgs),

    /// Inspect/validate configured hats and render topology diagrams
    Hats(hats::HatsArgs),

    /// Headless automation: run in a Git repo, record session, then judge JSONL
    Autopilot(autopilot::AutopilotArgs),

    /// Inspect record-session JSONL files (summary, watch)
    Record(record_cli::RecordArgs),

    /// Build Rerun runtime graph artifacts from live or durable evidence
    RuntimeGraph(RuntimeGraphArgs),

    /// Verify repository governance contracts and generated artifacts
    Verify(VerifyArgs),

    /// Inspect or clear runtime workflow state
    State(StateArgs),

    /// Diagnose common startup issues and provide safe fixes
    Doctor(doctor::DoctorArgs),
}

/// Arguments for the init subcommand.
#[derive(Parser, Debug)]
struct InitArgs {
    /// Backend to use (claude, kiro, gemini, codex, amp, custom).
    /// When used alone, generates minimal config.
    /// When used with --preset, overrides the preset's backend.
    #[arg(long, conflicts_with = "list_presets")]
    backend: Option<String>,

    /// Copy embedded preset to ralph.yml
    #[arg(long, conflicts_with = "list_presets")]
    preset: Option<String>,

    /// List all available embedded presets
    #[arg(long, conflicts_with = "backend", conflicts_with = "preset")]
    list_presets: bool,

    /// Overwrite existing ralph.yml if present
    #[arg(long)]
    force: bool,
}

/// Arguments for the run subcommand.
#[derive(Parser, Debug)]
struct RunArgs {
    /// Inline prompt text (mutually exclusive with -P/--prompt-file)
    #[arg(short = 'p', long = "prompt", conflicts_with = "prompt_file")]
    prompt_text: Option<String>,

    /// Override backend from config (cli > config > auto-detect)
    #[arg(short = 'b', long = "backend", value_name = "BACKEND")]
    backend: Option<String>,

    /// Prompt file path (mutually exclusive with -p/--prompt)
    #[arg(short = 'P', long = "prompt-file", conflicts_with = "prompt_text")]
    prompt_file: Option<PathBuf>,

    /// Override max iterations
    #[arg(long)]
    max_iterations: Option<u32>,

    /// Override completion promise
    #[arg(long)]
    completion_promise: Option<String>,

    /// Dry run - show what would be executed without running
    #[arg(long)]
    dry_run: bool,

    /// Continue from existing scratchpad (resume interrupted loop).
    /// Use this when a previous run was interrupted and you want to
    /// continue from where it left off.
    #[arg(long = "continue")]
    continue_mode: bool,

    // ─────────────────────────────────────────────────────────────────────────
    // Execution Mode Options
    // ─────────────────────────────────────────────────────────────────────────
    /// (Parallel mode) Idle start when prompt is missing/empty.
    ///
    /// 说明:
    /// - 用于 headless/CI/E2E: 在没有 `PROMPT.md` 且未传 `-p/-P` 时,仍允许启动并行 Supervisor 并待机。
    /// - 该模式下不会自动投递 `task.start`,因此可以做到 0 token 真待机。
    /// - 该模式下会关闭并行 Supervisor 的 `max_runtime_seconds` 护栏,适合常驻会话。
    /// - 仍保留其他护栏,例如 `job_timeout_secs` / `max_iterations` / 人工中断。
    /// - 仅当 prompt 缺失/为空时生效；若 prompt 存在则保持原有启动行为。
    /// - 与 `--continue` 冲突,避免与 resume 语义混淆。
    #[arg(long, conflicts_with = "continue_mode")]
    idle_start: bool,

    /// Disable TUI observation mode (TUI is enabled by default)
    #[arg(long, conflicts_with = "autonomous")]
    no_tui: bool,

    /// Disable Markdown rendering in output views (show raw text).
    ///
    /// Why: AI code agent 输出常包含 Markdown。默认渲染更易读；
    /// 当你需要排障/复制粘贴/对齐旧行为时，用 `--plain` 强制纯文本。
    #[arg(long)]
    plain: bool,

    /// Force autonomous mode (headless, non-interactive).
    /// Overrides default_mode from config.
    #[arg(short, long, conflicts_with = "no_tui")]
    autonomous: bool,

    /// Idle timeout in seconds for interactive mode (default: 30).
    /// Process is terminated after this many seconds of inactivity.
    /// Set to 0 to disable idle timeout.
    #[arg(long)]
    idle_timeout: Option<u32>,

    // ─────────────────────────────────────────────────────────────────────────
    // Verbosity Options
    // ─────────────────────────────────────────────────────────────────────────
    /// Enable verbose output (show tool results and session summary)
    #[arg(short = 'v', long, conflicts_with = "quiet")]
    verbose: bool,

    /// Suppress streaming output (for CI/scripting)
    #[arg(short = 'q', long, conflicts_with = "verbose")]
    quiet: bool,

    /// Record session to JSONL file for replay testing
    #[arg(long, value_name = "FILE")]
    record_session: Option<PathBuf>,

    /// (Parallel mode) Record a live runtime graph to a Rerun `.rrd` file.
    ///
    /// 说明:
    /// - 这是 V1 live runtime graph 的最小入口。
    /// - 录制结果可以用 `rerun <FILE>` 打开。
    /// - 目前只支持并行模式。
    #[arg(long, value_name = "FILE")]
    runtime_graph_rrd: Option<PathBuf>,

    /// Custom backend command and arguments (use after --)
    ///
    /// 示例：
    /// - `ralph run -b codex -- --model gpt-5.1-codex-max`
    /// - `ralph run -b claude -- --no-cache`
    #[arg(last = true)]
    custom_args: Vec<String>,

    /// (Parallel mode) Hide stderr (`:err:`) streaming lines (shown by default).
    ///
    /// Why: In parallel mode, stderr is often backend/CLI logs and echoes.
    /// When you want a quieter view, hide it explicitly.
    #[arg(long = "hide-stderr", action = clap::ArgAction::SetFalse, default_value_t = true)]
    show_stderr: bool,

    /// (Parallel mode) Only show streaming output from these instances (repeatable).
    ///
    /// Example: `--instance writer#1 --instance tester#1`
    #[arg(long, value_name = "HAT#KEY", action = clap::ArgAction::Append)]
    instance: Vec<String>,
}

/// Arguments for the resume subcommand.
///
/// Per spec: "When loop terminates due to safeguard (not completion promise),
/// user can run `ralph resume` to restart reading existing scratchpad."
#[derive(Parser, Debug)]
struct ResumeArgs {
    /// Override max iterations (from current position)
    #[arg(long)]
    max_iterations: Option<u32>,

    /// Disable TUI observation mode (TUI is enabled by default)
    #[arg(long, conflicts_with = "autonomous")]
    no_tui: bool,

    /// Disable Markdown rendering in output views (show raw text).
    #[arg(long)]
    plain: bool,

    /// Force autonomous mode
    #[arg(short, long, conflicts_with = "no_tui")]
    autonomous: bool,

    /// Idle timeout in seconds for TUI mode
    #[arg(long)]
    idle_timeout: Option<u32>,

    /// Enable verbose output (show tool results and session summary)
    #[arg(short = 'v', long, conflicts_with = "quiet")]
    verbose: bool,

    /// Suppress streaming output (for CI/scripting)
    #[arg(short = 'q', long, conflicts_with = "verbose")]
    quiet: bool,

    /// Record session to JSONL file for replay testing
    #[arg(long, value_name = "FILE")]
    record_session: Option<PathBuf>,

    /// (Parallel mode) Record a live runtime graph to a Rerun `.rrd` file.
    ///
    /// 说明:
    /// - 这是 V1 live runtime graph 的最小入口。
    /// - 录制结果可以用 `rerun <FILE>` 打开。
    /// - 目前只支持并行模式。
    #[arg(long, value_name = "FILE")]
    runtime_graph_rrd: Option<PathBuf>,

    /// (Parallel mode) Hide stderr (`:err:`) streaming lines (shown by default).
    #[arg(long = "hide-stderr", action = clap::ArgAction::SetFalse, default_value_t = true)]
    show_stderr: bool,

    /// (Parallel mode) Only show streaming output from these instances (repeatable).
    #[arg(long, value_name = "HAT#KEY", action = clap::ArgAction::Append)]
    instance: Vec<String>,
}

/// Arguments for the events subcommand.
#[derive(Parser, Debug)]
struct EventsArgs {
    /// Show only the last N events
    #[arg(long)]
    last: Option<usize>,

    /// Filter by topic (e.g., "build.blocked")
    #[arg(long)]
    topic: Option<String>,

    /// Filter by iteration number
    #[arg(long)]
    iteration: Option<u32>,

    /// Output format
    #[arg(long, value_enum, default_value_t = OutputFormat::Table)]
    format: OutputFormat,

    /// Path to events file (default: auto-detects current run)
    #[arg(long)]
    file: Option<PathBuf>,

    /// Clear the event history
    #[arg(long)]
    clear: bool,
}

/// Arguments for the agents subcommand.
#[derive(Parser, Debug)]
struct AgentsArgs {
    /// Output format
    #[arg(long, value_enum, default_value_t = OutputFormat::Table)]
    format: OutputFormat,

    /// Path to agents snapshot file (default: .ralph/agents.json)
    #[arg(long)]
    file: Option<PathBuf>,

    /// Watch agents snapshot and refresh periodically (table format only).
    ///
    /// 说明:
    /// - stdout 是 TTY 时,会清屏并原地刷新.
    /// - stdout 不是 TTY 时,不会输出清屏控制序列,改为分隔符追加输出,便于日志/CI.
    #[arg(long)]
    watch: bool,

    /// Refresh interval in milliseconds for --watch.
    #[arg(long, value_name = "MS", default_value_t = 1000)]
    watch_interval_ms: u64,
}

/// Arguments for the runtime-graph subcommand.
#[derive(Parser, Debug)]
struct RuntimeGraphArgs {
    #[command(subcommand)]
    command: RuntimeGraphCommands,
}

#[derive(Subcommand, Debug)]
enum RuntimeGraphCommands {
    /// Replay a finished run from durable `.ralph/events.jsonl` evidence.
    Replay(RuntimeGraphReplayArgs),
}

#[derive(Parser, Debug)]
struct RuntimeGraphReplayArgs {
    /// Path to events JSONL (default: auto-detects current run).
    #[arg(long)]
    events: Option<PathBuf>,

    /// Output Rerun `.rrd` file.
    #[arg(long, value_name = "FILE")]
    output: PathBuf,

    /// Keep only workflow / delivery records for this exact topic.
    #[arg(long)]
    topic: Option<String>,

    /// Keep only records related to this instance.
    #[arg(long, value_name = "HAT#KEY")]
    instance: Option<String>,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Table)]
    format: OutputFormat,
}

/// Arguments for the verify command group.
#[derive(Parser, Debug)]
struct VerifyArgs {
    #[command(subcommand)]
    command: VerifyCommands,
}

#[derive(Subcommand, Debug)]
enum VerifyCommands {
    /// Verify the agent guidance manifest and registered guidance assets.
    AgentGuidance(VerifyAgentGuidanceArgs),
}

/// Arguments for `ralph verify agent-guidance`.
#[derive(Parser, Debug)]
struct VerifyAgentGuidanceArgs {
    /// Path to the guidance manifest, relative to the repository root.
    #[arg(long, default_value = DEFAULT_AGENT_GUIDANCE_MANIFEST)]
    manifest: String,
}

/// Arguments for the state command group.
#[derive(Parser, Debug)]
struct StateArgs {
    #[command(subcommand)]
    command: StateCommands,
}

#[derive(Subcommand, Debug)]
enum StateCommands {
    /// Show runtime workflow state summaries.
    Status(StateStatusArgs),

    /// Read one runtime workflow state record.
    Read(StateReadArgs),

    /// Clear runtime workflow state for one mode.
    Clear(StateClearArgs),
}

/// Arguments for `ralph state status`.
#[derive(Parser, Debug)]
struct StateStatusArgs {
    /// Limit status to one supported state mode.
    #[arg(long)]
    mode: Option<String>,

    /// Read session-scoped state, falling back to global state when absent.
    #[arg(long)]
    session_id: Option<String>,

    /// Print machine-readable JSON.
    #[arg(long)]
    json: bool,
}

/// Arguments for `ralph state read <mode>`.
#[derive(Parser, Debug)]
struct StateReadArgs {
    /// Supported state mode: ralph, ralplan, team, deep-interview, capability-invocation.
    mode: String,

    /// Read session-scoped state, falling back to global state when absent.
    #[arg(long)]
    session_id: Option<String>,

    /// Print machine-readable JSON.
    #[arg(long)]
    json: bool,
}

/// Arguments for `ralph state clear <mode>`.
#[derive(Parser, Debug)]
struct StateClearArgs {
    /// Supported state mode: ralph, ralplan, team, deep-interview, capability-invocation.
    mode: String,

    /// Clear only this session-scoped state.
    #[arg(long, conflicts_with = "all_sessions")]
    session_id: Option<String>,

    /// Clear global state plus all session-scoped state for the mode.
    #[arg(long, conflicts_with = "session_id")]
    all_sessions: bool,
}

/// Arguments for the clean subcommand.
#[derive(Parser, Debug)]
struct CleanArgs {
    /// Preview what would be deleted without actually deleting
    #[arg(long)]
    dry_run: bool,

    /// Clean diagnostic logs instead of .agent directory
    #[arg(long)]
    diagnostics: bool,
}

/// Arguments for the emit subcommand.
#[derive(Parser, Debug)]
struct EmitArgs {
    /// Event topic (e.g., "build.done", "review.complete")
    pub topic: String,

    /// Event payload - string or JSON (optional, defaults to empty)
    #[arg(default_value = "")]
    pub payload: String,

    /// Parse payload as JSON object instead of string
    #[arg(long, short)]
    pub json: bool,

    /// Custom ISO 8601 timestamp (defaults to current time)
    #[arg(long)]
    pub ts: Option<String>,

    /// Path to events file (defaults to .ralph/events.jsonl)
    #[arg(long, default_value = ".ralph/events.jsonl")]
    pub file: PathBuf,

    /// Optional target instance for direct delivery (parallel mode).
    ///
    /// Example: `--target-instance writer#1`
    #[arg(long, value_name = "HAT#KEY")]
    pub target_instance: Option<String>,

    /// Optional target hat for delivery (parallel mode).
    ///
    /// Example: `--target writer`
    #[arg(long, value_name = "HAT")]
    pub target: Option<String>,

    /// Force spawn a fresh hat instance for this delivery (parallel mode).
    ///
    /// 说明:
    /// - 用于实现“new_instance”投递模式(上下文隔离)。
    /// - 需要同时提供 `--target <hat_id>`。
    /// - 与 `--target-instance` 互斥。
    #[arg(long, conflicts_with = "target_instance", requires = "target")]
    pub spawn_instance: bool,

    /// Optional workspace strategy override (parallel mode).
    ///
    /// 说明:
    /// - 这是“提示执行环境”的信号,最终仍需 capability/permission gate 判定。
    /// - 值为 snake_case: shared / patch / worktree
    #[arg(long, value_enum, value_name = "STRATEGY")]
    pub workspace_strategy: Option<EmitWorkspaceStrategy>,

    /// Optional session strategy override (parallel mode).
    ///
    /// 说明:
    /// - 用于显式指定本条事件的会话形态,确保 replay/诊断一致性。
    /// - 值为 snake_case: exec / mcp / app_server
    #[arg(long, value_enum, value_name = "STRATEGY")]
    pub session_strategy: Option<EmitSessionStrategy>,

    /// Optional turn action (App Server only).
    ///
    /// 说明:
    /// - 用于 steer/interrupt 这类“运行时控制信号”。
    /// - 值为 snake_case: start / steer / interrupt
    #[arg(long, value_enum, value_name = "ACTION")]
    pub turn_action: Option<EmitTurnAction>,
}

/// `ralph emit` 的 workspace_strategy 可选值.
///
/// 说明:
/// - 这些值必须与 `ralph_core::event_reader::Event.workspace_strategy` 的 snake_case 约定一致。
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "snake_case")]
enum EmitWorkspaceStrategy {
    Shared,
    Patch,
    Worktree,
}

impl std::fmt::Display for EmitWorkspaceStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let raw = match self {
            EmitWorkspaceStrategy::Shared => "shared",
            EmitWorkspaceStrategy::Patch => "patch",
            EmitWorkspaceStrategy::Worktree => "worktree",
        };
        write!(f, "{raw}")
    }
}

/// `ralph emit` 的 session_strategy 可选值.
///
/// 说明:
/// - 这些值必须与 `ralph_core::event_reader::Event.session_strategy` 的 snake_case 约定一致。
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "snake_case")]
enum EmitSessionStrategy {
    Exec,
    Mcp,
    AppServer,
}

impl std::fmt::Display for EmitSessionStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let raw = match self {
            EmitSessionStrategy::Exec => "exec",
            EmitSessionStrategy::Mcp => "mcp",
            EmitSessionStrategy::AppServer => "app_server",
        };
        write!(f, "{raw}")
    }
}

/// `ralph emit` 的 turn_action 可选值.
///
/// 说明:
/// - 这些值必须与 `ralph_core::event_reader::Event.turn_action` 的 snake_case 约定一致。
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "snake_case")]
enum EmitTurnAction {
    Start,
    Steer,
    Interrupt,
}

impl std::fmt::Display for EmitTurnAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let raw = match self {
            EmitTurnAction::Start => "start",
            EmitTurnAction::Steer => "steer",
            EmitTurnAction::Interrupt => "interrupt",
        };
        write!(f, "{raw}")
    }
}

/// Arguments for the plan subcommand.
///
/// Starts an interactive PDD (Prompt-Driven Development) session.
/// This is a thin wrapper that spawns the AI backend with the bundled
/// PDD SOP, bypassing Ralph's event loop entirely.
#[derive(Parser, Debug)]
struct PlanArgs {
    /// The rough idea to develop (optional - SOP will prompt if not provided)
    #[arg(value_name = "IDEA")]
    idea: Option<String>,

    /// Backend to use (overrides config and auto-detection)
    #[arg(short, long, value_name = "BACKEND")]
    backend: Option<String>,
}

/// Arguments for the task subcommand.
///
/// Starts an interactive code-task-generator session.
/// This is a thin wrapper that spawns the AI backend with the bundled
/// code-task-generator SOP, bypassing Ralph's event loop entirely.
#[derive(Parser, Debug)]
struct CodeTaskArgs {
    /// Input: description text or path to PDD plan file
    #[arg(value_name = "INPUT")]
    input: Option<String>,

    /// Backend to use (overrides config and auto-detection)
    #[arg(short, long, value_name = "BACKEND")]
    backend: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Install panic hook to restore terminal state on crash
    // This prevents the terminal from being left in raw mode or alternate screen
    install_panic_hook();

    let raw_args = std::env::args_os().collect::<Vec<_>>();
    let config_was_explicit = cli_config_was_explicit(raw_args.iter().map(|arg| arg.as_os_str()));
    let cli = Cli::parse_from(raw_args);

    // Detect if TUI mode is requested - TUI owns the terminal, so logs must not go to stdout
    // TUI is enabled by default unless --no-tui is specified or --autonomous is used
    let tui_enabled = match &cli.command {
        Some(Commands::Run(args)) => !args.no_tui && !args.autonomous,
        Some(Commands::Resume(args)) => !args.no_tui && !args.autonomous,
        _ => false,
    };

    // Initialize logging - suppress in TUI mode to avoid corrupting the display
    let filter = if cli.verbose { "debug" } else { "info" };

    // Check if diagnostics are enabled
    let diagnostics_enabled = std::env::var("RALPH_DIAGNOSTICS")
        .map(|v| v == "1")
        .unwrap_or(false);

    if tui_enabled {
        // TUI mode: logs would corrupt the display, so we suppress them entirely.
        // For debugging TUI issues, set RALPH_DEBUG_LOG=1 to write to .agent/ralph.log
        if std::env::var("RALPH_DEBUG_LOG").is_ok() {
            let log_path = std::path::Path::new(".agent").join("ralph.log");
            if let Ok(file) = std::fs::File::create(&log_path) {
                if diagnostics_enabled {
                    // TUI + diagnostics: logs to file + trace layer
                    use ralph_core::diagnostics::DiagnosticTraceLayer;
                    use tracing_subscriber::prelude::*;

                    if let Ok(collector) = ralph_core::diagnostics::DiagnosticsCollector::new(
                        std::path::Path::new("."),
                    ) && let Some(session_dir) = collector.session_dir()
                    {
                        if let Ok(trace_layer) = DiagnosticTraceLayer::new(session_dir) {
                            tracing_subscriber::registry()
                                .with(
                                    tracing_subscriber::fmt::layer()
                                        .with_writer(std::sync::Mutex::new(file))
                                        .with_ansi(false),
                                )
                                .with(tracing_subscriber::EnvFilter::new(filter))
                                .with(trace_layer)
                                .init();
                        } else {
                            // Fallback: just file logging
                            tracing_subscriber::fmt()
                                .with_env_filter(filter)
                                .with_writer(std::sync::Mutex::new(file))
                                .with_ansi(false)
                                .init();
                        }
                    }
                } else {
                    // TUI without diagnostics: just file logging
                    tracing_subscriber::fmt()
                        .with_env_filter(filter)
                        .with_writer(std::sync::Mutex::new(file))
                        .with_ansi(false)
                        .init();
                }
            }
        }
        // If RALPH_DEBUG_LOG is not set or file creation fails, no logging (default)
    } else {
        // Normal mode: logs go to stdout
        if diagnostics_enabled {
            // Normal mode + diagnostics: stdout + trace layer
            use ralph_core::diagnostics::DiagnosticTraceLayer;
            use tracing_subscriber::prelude::*;

            if let Ok(collector) =
                ralph_core::diagnostics::DiagnosticsCollector::new(std::path::Path::new("."))
                && let Some(session_dir) = collector.session_dir()
            {
                if let Ok(trace_layer) = DiagnosticTraceLayer::new(session_dir) {
                    tracing_subscriber::registry()
                        .with(tracing_subscriber::fmt::layer())
                        .with(tracing_subscriber::EnvFilter::new(filter))
                        .with(trace_layer)
                        .init();
                } else {
                    // Fallback: just stdout
                    tracing_subscriber::fmt().with_env_filter(filter).init();
                }
            } else {
                // Fallback: just stdout
                tracing_subscriber::fmt().with_env_filter(filter).init();
            }
        } else {
            // Normal mode without diagnostics: just stdout
            tracing_subscriber::fmt().with_env_filter(filter).init();
        }
    }

    match cli.command {
        Some(Commands::Run(args)) => {
            run_command(
                cli.config,
                config_was_explicit,
                cli.verbose,
                cli.color,
                args,
            )
            .await
        }
        Some(Commands::Resume(args)) => {
            resume_command(cli.config, cli.verbose, cli.color, args).await
        }
        Some(Commands::Events(args)) => events_command(cli.color, args),
        Some(Commands::Agents(args)) => agents_command(cli.color, args),
        Some(Commands::Init(args)) => init_command(cli.color, args),
        Some(Commands::Clean(args)) => clean_command(cli.config, cli.color, args),
        Some(Commands::Emit(args)) => emit_command(cli.color, args),
        Some(Commands::Plan(args)) => plan_command(cli.config, cli.color, args),
        Some(Commands::CodeTask(args)) => code_task_command(cli.config, cli.color, args),
        Some(Commands::Task(args)) => code_task_command(cli.config, cli.color, args),
        Some(Commands::Tools(args)) => tools::execute(args, cli.color.should_use_colors()),
        Some(Commands::Hats(args)) => {
            hats::execute(&cli.config, args, cli.color.should_use_colors())
        }
        Some(Commands::Autopilot(args)) => autopilot::execute(cli.config, args).await,
        Some(Commands::Record(args)) => record_cli::execute(args).await,
        Some(Commands::RuntimeGraph(args)) => runtime_graph_command(cli.color, args),
        Some(Commands::Verify(args)) => verify_command(cli.color, args),
        Some(Commands::State(args)) => state_command(args),
        Some(Commands::Doctor(args)) => {
            doctor::execute(cli.config, args, cli.color.should_use_colors()).await
        }
        None => {
            // Default to run with TUI enabled (new default behavior)
            let args = RunArgs {
                prompt_text: None,
                prompt_file: None,
                backend: None,
                max_iterations: None,
                completion_promise: None,
                dry_run: false,
                continue_mode: false,
                idle_start: false,
                no_tui: false, // TUI enabled by default
                plain: false,
                autonomous: false,
                idle_timeout: None,
                verbose: false,
                quiet: false,
                record_session: None,
                runtime_graph_rrd: None,
                custom_args: Vec::new(),
                show_stderr: true,
                instance: Vec::new(),
            };
            run_command(cli.config, false, cli.verbose, cli.color, args).await
        }
    }
}

async fn run_command(
    config_path: PathBuf,
    config_was_explicit: bool,
    verbose: bool,
    color_mode: ColorMode,
    args: RunArgs,
) -> Result<()> {
    // Parse config source (file, builtin, or remote)
    let config_source = ConfigSource::parse(config_path.to_string_lossy().as_ref());

    // Load configuration based on source type.
    //
    // 说明:
    // - startup bootstrap 只能发生在真实 EventLoop / Supervisor 初始化前。
    // - v1 selector 只在默认 `ralph.yml` 缺失、且用户没有显式 prompt/config 意图时运行。
    // - 这样既能支持“无 config / 无 prompt 启动”,也不会吞掉显式配置错误。
    let mut bootstrap_selection = None;
    let mut config = match config_source {
        ConfigSource::File(path) => {
            if path.exists() {
                RalphConfig::from_file(&path)
                    .with_context(|| format!("Failed to load config from {:?}", path))?
            } else if startup_resources::should_bootstrap_missing_default_config(
                &path,
                config_was_explicit,
                args.prompt_text.is_some(),
                args.prompt_file.is_some(),
                args.continue_mode,
            ) {
                let resolution = startup_resources::resolve_default_bootstrap()
                    .context("Failed to resolve startup resources")?;
                bootstrap_selection = Some(resolution.selection);
                resolution.config
            } else {
                warn!("Config file {:?} not found, using defaults", path);
                RalphConfig::default()
            }
        }
        ConfigSource::Builtin(name) => {
            let preset = presets::get_preset(&name).ok_or_else(|| {
                let available = presets::preset_names().join(", ");
                anyhow::anyhow!(
                    "Unknown preset '{}'. Run `ralph run --list-presets` to see available presets.\n\nAvailable: {}",
                    name,
                    available
                )
            })?;
            RalphConfig::parse_yaml(preset.content)
                .with_context(|| format!("Failed to parse builtin preset '{}'", name))?
        }
        ConfigSource::Remote(url) => {
            info!("Fetching config from {}", url);
            let response = reqwest::get(&url)
                .await
                .with_context(|| format!("Failed to fetch config from {}", url))?;

            if !response.status().is_success() {
                anyhow::bail!(
                    "Failed to fetch config from {}: HTTP {}",
                    url,
                    response.status()
                );
            }

            let content = response
                .text()
                .await
                .with_context(|| format!("Failed to read config content from {}", url))?;

            RalphConfig::parse_yaml(&content)
                .with_context(|| format!("Failed to parse config from {}", url))?
        }
    };

    // Normalize v1 flat fields into v2 nested structure
    config.normalize();

    // Set workspace_root to current directory (critical for E2E tests in isolated workspaces).
    // This must happen after config load because workspace_root has #[serde(skip)] and
    // defaults to cwd at deserialize time - but we need it set to the actual runtime cwd.
    config.core.workspace_root =
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

    // Handle --continue mode: check scratchpad exists before proceeding
    let resume = args.continue_mode;
    if resume {
        let scratchpad_path = std::path::Path::new(&config.core.scratchpad);
        if !scratchpad_path.exists() {
            anyhow::bail!(
                "Cannot continue: scratchpad not found at '{}'. \
                 Start a fresh run with `ralph run`.",
                config.core.scratchpad
            );
        }
        info!(
            "Found existing scratchpad at '{}', continuing from previous state",
            config.core.scratchpad
        );
    }

    // Apply CLI overrides (after normalization so they take final precedence)
    // Per spec: CLI -p and -P are mutually exclusive (enforced by clap)
    //
    // 并行 TUI 的“自动待机”仅在没有 CLI prompt 覆盖时允许触发:
    // - 避免用户显式指定了 prompt(-p/-P)但路径/内容有误时被静默吞掉。
    let allow_tui_auto_idle = args.prompt_text.is_none() && args.prompt_file.is_none();
    if let Some(text) = args.prompt_text {
        config.event_loop.prompt = Some(text);
        config.event_loop.prompt_file = String::new(); // Clear file path
    } else if let Some(path) = args.prompt_file {
        config.event_loop.prompt_file = path.to_string_lossy().to_string();
        config.event_loop.prompt = None; // Clear inline
    }
    if let Some(max_iter) = args.max_iterations {
        config.event_loop.max_iterations = max_iter;
    }
    if let Some(promise) = args.completion_promise {
        config.event_loop.completion_promise = promise;
    }
    if verbose {
        config.verbose = true;
    }

    // Apply execution mode overrides per spec
    // TUI is enabled by default (unless --no-tui is specified)
    if args.autonomous {
        config.cli.default_mode = "autonomous".to_string();
    } else if !args.no_tui {
        config.cli.default_mode = "interactive".to_string();
    }

    // Override idle timeout if specified
    if let Some(timeout) = args.idle_timeout {
        config.cli.idle_timeout_secs = timeout;
    }

    // Apply backend override from CLI (takes precedence over config)
    if let Some(backend) = args.backend {
        config.cli.backend = backend;
    }

    // Validate configuration and emit warnings
    let warnings = config
        .validate()
        .context("Configuration validation failed")?;
    for warning in &warnings {
        eprintln!("{warning}");
    }

    // Handle auto-detection if backend is "auto"
    if config.cli.backend == "auto" {
        let priority = config.get_agent_priority();
        let detected = detect_backend(&priority, |backend| {
            config.adapter_settings(backend).enabled
        });

        match detected {
            Ok(backend) => {
                info!("Auto-detected backend: {}", backend);
                config.cli.backend = backend;
            }
            Err(e) => {
                eprintln!("{e}");
                return Err(anyhow::Error::new(e));
            }
        }
    }

    if let Some(selection) = bootstrap_selection.as_ref() {
        // 说明:
        // - 这里写出的 artifact 必须代表真实 run 接下来会使用的最终配置。
        // - 因此它放在 CLI override、validate、backend auto-detect 之后。
        // - 仍然位于 dry-run/真实 EventLoop/Supervisor 初始化之前,保持 startup-only 边界。
        startup_resources::write_bootstrap_artifacts(
            &config.core.workspace_root,
            selection,
            &config,
        )
        .context("Failed to write startup bootstrap artifacts")?;
    }

    if args.dry_run {
        println!("Dry run mode - configuration:");
        println!(
            "  Hats: {}",
            if config.hats.is_empty() {
                "planner, builder (default)".to_string()
            } else {
                config.hats.keys().cloned().collect::<Vec<_>>().join(", ")
            }
        );

        // Show prompt source
        if let Some(ref inline) = config.event_loop.prompt {
            let preview = if inline.len() > 60 {
                format!("{}...", &inline[..60].replace('\n', " "))
            } else {
                inline.replace('\n', " ")
            };
            println!("  Prompt: inline text ({})", preview);
        } else {
            println!("  Prompt file: {}", config.event_loop.prompt_file);
        }

        println!(
            "  Completion promise: {}",
            config.event_loop.completion_promise
        );
        println!("  Max iterations: {}", config.event_loop.max_iterations);
        println!("  Max runtime: {}s", config.event_loop.max_runtime_seconds);
        println!("  Backend: {}", config.cli.backend);
        println!("  Verbose: {}", config.verbose);
        // Execution mode info
        println!("  Default mode: {}", config.cli.default_mode);
        if config.cli.default_mode == "interactive" {
            println!("  Idle timeout: {}s", config.cli.idle_timeout_secs);
        }
        if !warnings.is_empty() {
            println!("  Warnings: {}", warnings.len());
        }
        return Ok(());
    }

    // Run the orchestration loop and exit with proper exit code
    // TUI is enabled by default (unless --no-tui or --autonomous is specified)
    let enable_tui = !args.no_tui && !args.autonomous;
    let verbosity = Verbosity::resolve(verbose || args.verbose, args.quiet);
    let custom_args = args.custom_args.clone();

    // --idle-start 只允许在并行模式下使用(串行没有“外部事件驱动的待机”语义)。
    if args.idle_start && !config.parallel.enabled {
        anyhow::bail!("`--idle-start` requires `parallel.enabled=true` in config.");
    }
    if args.runtime_graph_rrd.is_some() && !config.parallel.enabled {
        anyhow::bail!("`--runtime-graph-rrd` requires `parallel.enabled=true` in config.");
    }

    let reason = if config.parallel.enabled {
        parallel_runner::run_parallel_loop_impl(
            config,
            color_mode,
            parallel_runner::ParallelLoopFlags {
                resume,
                enable_tui,
                plain: args.plain,
                show_stderr: args.show_stderr,
                idle_start: args.idle_start,
                allow_tui_auto_idle,
                runtime_graph_rrd: args.runtime_graph_rrd.clone(),
            },
            verbosity,
            args.record_session,
            args.instance.clone(),
            custom_args.clone(),
        )
        .await?
    } else {
        loop_runner::run_loop_impl(
            config,
            color_mode,
            resume,
            enable_tui,
            verbosity,
            args.plain,
            args.record_session,
            custom_args,
        )
        .await?
    };
    let exit_code = reason.exit_code();

    // Use explicit exit for non-zero codes to ensure proper exit status
    if exit_code != 0 {
        std::process::exit(exit_code);
    }

    Ok(())
}

/// Resume a previously interrupted loop from existing scratchpad.
///
/// DEPRECATED: Use `ralph run --continue` instead.
///
/// Per spec: "When loop terminates due to safeguard (not completion promise),
/// user can run `ralph run --continue` to restart reading existing scratchpad,
/// continuing from where it left off."
async fn resume_command(
    config_path: PathBuf,
    verbose: bool,
    color_mode: ColorMode,
    args: ResumeArgs,
) -> Result<()> {
    // Show deprecation warning
    eprintln!(
        "{}warning:{} `ralph resume` is deprecated. Use `ralph run --continue` instead.",
        colors::YELLOW,
        colors::RESET
    );

    // Load configuration
    let mut config = if config_path.exists() {
        RalphConfig::from_file(&config_path)
            .with_context(|| format!("Failed to load config from {:?}", config_path))?
    } else {
        warn!("Config file {:?} not found, using defaults", config_path);
        RalphConfig::default()
    };

    config.normalize();

    // Set workspace_root to current directory (critical for E2E tests in isolated workspaces).
    config.core.workspace_root =
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

    // Check that scratchpad exists (required for resume)
    let scratchpad_path = std::path::Path::new(&config.core.scratchpad);
    if !scratchpad_path.exists() {
        anyhow::bail!(
            "Cannot continue: scratchpad not found at '{}'. \
             Start a fresh run with `ralph run`.",
            config.core.scratchpad
        );
    }

    info!(
        "Found existing scratchpad at '{}', continuing from previous state",
        config.core.scratchpad
    );

    // Apply CLI overrides
    if let Some(max_iter) = args.max_iterations {
        config.event_loop.max_iterations = max_iter;
    }
    if verbose {
        config.verbose = true;
    }

    // Apply execution mode overrides
    // TUI is enabled by default (unless --no-tui is specified)
    if args.autonomous {
        config.cli.default_mode = "autonomous".to_string();
    } else if !args.no_tui {
        config.cli.default_mode = "interactive".to_string();
    }

    // Override idle timeout if specified
    if let Some(timeout) = args.idle_timeout {
        config.cli.idle_timeout_secs = timeout;
    }

    // Validate configuration
    let warnings = config
        .validate()
        .context("Configuration validation failed")?;
    for warning in &warnings {
        eprintln!("{warning}");
    }

    // Handle auto-detection if backend is "auto"
    if config.cli.backend == "auto" {
        let priority = config.get_agent_priority();
        let detected = detect_backend(&priority, |backend| {
            config.adapter_settings(backend).enabled
        });

        match detected {
            Ok(backend) => {
                info!("Auto-detected backend: {}", backend);
                config.cli.backend = backend;
            }
            Err(e) => {
                eprintln!("{e}");
                return Err(anyhow::Error::new(e));
            }
        }
    }

    // Run the orchestration loop in resume mode
    // The key difference: we publish task.resume instead of task.start,
    // signaling the planner to read the existing scratchpad
    // TUI is enabled by default (unless --no-tui or --autonomous is specified)
    let enable_tui = !args.no_tui && !args.autonomous;
    let verbosity = Verbosity::resolve(verbose || args.verbose, args.quiet);
    if args.runtime_graph_rrd.is_some() && !config.parallel.enabled {
        anyhow::bail!("`--runtime-graph-rrd` requires `parallel.enabled=true` in config.");
    }
    let reason = if config.parallel.enabled {
        parallel_runner::run_parallel_loop_impl(
            config,
            color_mode,
            parallel_runner::ParallelLoopFlags {
                resume: true,
                enable_tui,
                plain: args.plain,
                show_stderr: args.show_stderr,
                idle_start: false,
                allow_tui_auto_idle: false,
                runtime_graph_rrd: args.runtime_graph_rrd.clone(),
            },
            verbosity,
            args.record_session,
            args.instance.clone(),
            Vec::new(), // resume 不支持 `-- <custom args>`
        )
        .await?
    } else {
        loop_runner::run_loop_impl(
            config,
            color_mode,
            true,
            enable_tui,
            verbosity,
            args.plain,
            args.record_session,
            Vec::new(), // resume 不支持 `-- <custom args>`
        )
        .await?
    };
    let exit_code = reason.exit_code();

    if exit_code != 0 {
        std::process::exit(exit_code);
    }

    Ok(())
}

fn init_command(color_mode: ColorMode, args: InitArgs) -> Result<()> {
    let use_colors = color_mode.should_use_colors();

    // Handle --list-presets
    if args.list_presets {
        println!("{}", init::format_preset_list());
        return Ok(());
    }

    // Handle --preset (with optional --backend override)
    if let Some(preset) = args.preset {
        let backend_override = args.backend.as_deref();
        match init::init_from_preset(&preset, backend_override, args.force) {
            Ok(()) => {
                let msg = if let Some(backend) = backend_override {
                    format!(
                        "Created ralph.yml from '{}' preset with {} backend",
                        preset, backend
                    )
                } else {
                    format!("Created ralph.yml from '{}' preset", preset)
                };
                if use_colors {
                    println!("{}✓{} {}", colors::GREEN, colors::RESET, msg);
                    println!(
                        "\n{}Next steps:{}\n  1. Create PROMPT.md with your task\n  2. Run: ralph run",
                        colors::DIM,
                        colors::RESET
                    );
                } else {
                    println!("{}", msg);
                    println!(
                        "\nNext steps:\n  1. Create PROMPT.md with your task\n  2. Run: ralph run"
                    );
                }
                return Ok(());
            }
            Err(e) => {
                anyhow::bail!("{}", e);
            }
        }
    }

    // Handle --backend alone (minimal config)
    if let Some(backend) = args.backend {
        match init::init_from_backend(&backend, args.force) {
            Ok(()) => {
                if use_colors {
                    println!(
                        "{}✓{} Created ralph.yml with {} backend",
                        colors::GREEN,
                        colors::RESET,
                        backend
                    );
                    println!(
                        "\n{}Next steps:{}\n  1. Create PROMPT.md with your task\n  2. Run: ralph run",
                        colors::DIM,
                        colors::RESET
                    );
                } else {
                    println!("Created ralph.yml with {} backend", backend);
                    println!(
                        "\nNext steps:\n  1. Create PROMPT.md with your task\n  2. Run: ralph run"
                    );
                }
                return Ok(());
            }
            Err(e) => {
                anyhow::bail!("{}", e);
            }
        }
    }

    // No flag specified - show help
    println!("Initialize a new ralph.yml configuration file.\n");
    println!("Usage:");
    println!("  ralph init --backend <backend>   Generate minimal config for backend");
    println!("  ralph init --preset <preset>     Use an embedded preset");
    println!("  ralph init --list-presets        Show available presets\n");
    println!("Backends: claude, kiro, gemini, codex, amp, custom");
    println!("\nRun 'ralph init --list-presets' to see available presets.");

    Ok(())
}

fn events_command(color_mode: ColorMode, args: EventsArgs) -> Result<()> {
    let use_colors = color_mode.should_use_colors();

    // Read events path from marker file, fall back to default if marker doesn't exist
    // This ensures `ralph events` reads from the same events file as the active run
    let history = match args.file {
        Some(path) => EventHistory::new(path),
        None => {
            // 说明:
            // - 与 `ralph emit` 一致: 支持在子目录执行时自动定位到 active run 的 events 文件。
            // - 找不到 marker 时,回退到默认 `.ralph/events.jsonl`。
            let path =
                resolve_events_file_from_marker_in_parents(PathBuf::from(".ralph/events.jsonl"));
            EventHistory::new(path)
        }
    };

    // Handle clear command
    if args.clear {
        history.clear()?;
        if use_colors {
            println!("{}✓{} Event history cleared", colors::GREEN, colors::RESET);
        } else {
            println!("Event history cleared");
        }
        return Ok(());
    }

    if !history.exists() {
        if use_colors {
            println!(
                "{}No event history found.{} Run `ralph` to generate events.",
                colors::DIM,
                colors::RESET
            );
        } else {
            println!("No event history found. Run `ralph` to generate events.");
        }
        return Ok(());
    }

    // Read and filter events
    let mut records = history.read_all()?;

    // Apply filters in sequence
    if let Some(ref topic) = args.topic {
        records.retain(|r| r.topic == *topic);
    }

    if let Some(iteration) = args.iteration {
        records.retain(|r| r.iteration == iteration);
    }

    // Apply 'last' filter after other filters (to get last N of filtered results)
    if let Some(n) = args.last
        && records.len() > n
    {
        records = records.into_iter().rev().take(n).rev().collect();
    }

    if records.is_empty() {
        if use_colors {
            println!("{}No matching events found.{}", colors::DIM, colors::RESET);
        } else {
            println!("No matching events found.");
        }
        return Ok(());
    }

    match args.format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&records)?;
            println!("{json}");
        }
        OutputFormat::Table => {
            display::print_events_table(&records, use_colors);
        }
    }

    Ok(())
}

fn runtime_graph_command(color_mode: ColorMode, args: RuntimeGraphArgs) -> Result<()> {
    match args.command {
        RuntimeGraphCommands::Replay(args) => runtime_graph_replay_command(color_mode, args),
    }
}

fn verify_command(color_mode: ColorMode, args: VerifyArgs) -> Result<()> {
    match args.command {
        VerifyCommands::AgentGuidance(args) => verify_agent_guidance_command(color_mode, args),
    }
}

fn verify_agent_guidance_command(
    color_mode: ColorMode,
    args: VerifyAgentGuidanceArgs,
) -> Result<()> {
    // 当前命令只做本地静态验证,repo root 就是调用者当前目录。
    // 这样 CI、开发机和 agent 都能用同一个入口复查 guidance drift。
    let repo_root = Path::new(".");
    let report = verify_manifest_at_with_report(repo_root, &args.manifest)
        .with_context(|| "agent guidance manifest verification failed")?;

    if color_mode.should_use_colors() {
        println!(
            "{}✓{} Agent guidance manifest verified: {}",
            colors::GREEN,
            colors::RESET,
            report.manifest_path
        );
    } else {
        println!("Agent guidance manifest verified: {}", report.manifest_path);
    }

    println!("Assets checked: {}", report.asset_count);
    println!("Skills checked: {}", report.skill_count);

    Ok(())
}

fn state_command(args: StateArgs) -> Result<()> {
    match args.command {
        StateCommands::Status(args) => state_status_command(args),
        StateCommands::Read(args) => state_read_command(args),
        StateCommands::Clear(args) => state_clear_command(args),
    }
}

fn state_status_command(args: StateStatusArgs) -> Result<()> {
    // ─────────────────────────────────────────────────────────────────────
    // CLI 只负责把用户意图转换成 core operation 调用。
    // 状态路径、session fallback、malformed JSON 处理都继续归 core 层所有。
    // ─────────────────────────────────────────────────────────────────────
    let store = StateOperationStore::new(".");
    let modes = match args.mode {
        Some(mode) => vec![parse_state_mode(&mode)?],
        None => StateMode::all().to_vec(),
    };

    let mut entries = Vec::with_capacity(modes.len());
    for mode in modes {
        let statuses = store
            .state_get_status(Some(mode), args.session_id.as_deref())
            .with_context(|| format!("state status failed for mode `{mode}`"))?;
        let fallback_path = store
            .state_path(mode, args.session_id.as_deref())
            .with_context(|| format!("state status failed for mode `{mode}`"))?;
        entries.push(match statuses.get(&mode) {
            Some(status) => serde_json::json!({
                "mode": mode.as_str(),
                "exists": true,
                "active": status.active,
                "current_phase": status.current_phase,
                "run_outcome": status.run_outcome.map(|outcome| outcome.as_str()),
                "lifecycle_outcome": status.lifecycle_outcome.map(|outcome| outcome.as_str()),
                "path": status.path.display().to_string(),
                "error": status.error,
            }),
            None => serde_json::json!({
                "mode": mode.as_str(),
                "exists": false,
                "active": null,
                "current_phase": null,
                "run_outcome": null,
                "lifecycle_outcome": null,
                "path": fallback_path.display().to_string(),
                "error": null,
            }),
        });
    }

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({ "statuses": entries }))?
        );
        return Ok(());
    }

    for entry in entries {
        println!("mode: {}", entry["mode"].as_str().unwrap_or("<unknown>"));
        println!("  exists: {}", entry["exists"].as_bool().unwrap_or(false));
        println!(
            "  active: {}",
            state_json_field_to_text(entry.get("active").unwrap_or(&serde_json::Value::Null))
        );
        println!(
            "  current_phase: {}",
            state_json_field_to_text(
                entry
                    .get("current_phase")
                    .unwrap_or(&serde_json::Value::Null)
            )
        );
        println!(
            "  run_outcome: {}",
            state_json_field_to_text(entry.get("run_outcome").unwrap_or(&serde_json::Value::Null))
        );
        println!(
            "  lifecycle_outcome: {}",
            state_json_field_to_text(
                entry
                    .get("lifecycle_outcome")
                    .unwrap_or(&serde_json::Value::Null)
            )
        );
        println!("  path: {}", entry["path"].as_str().unwrap_or("<none>"));
        if !entry["error"].is_null() {
            println!(
                "  error: {}",
                state_json_field_to_text(entry.get("error").unwrap_or(&serde_json::Value::Null))
            );
        }
    }

    Ok(())
}

fn state_read_command(args: StateReadArgs) -> Result<()> {
    let mode = parse_state_mode(&args.mode)?;
    let store = StateOperationStore::new(".");
    let result = store
        .state_read(mode, args.session_id.as_deref())
        .with_context(|| format!("state read failed for mode `{mode}`"))?;
    let exists = result.exists();

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "mode": mode.as_str(),
                "exists": exists,
                "path": result.path.map(|path| path.display().to_string()),
                "record": result.record,
            }))?
        );
        return Ok(());
    }

    match result.record {
        Some(record) => {
            println!("State found for mode `{mode}`");
            if let Some(path) = result.path {
                println!("path: {}", path.display());
            }
            println!("{}", serde_json::to_string_pretty(&record)?);
        }
        None => {
            println!("No state found for mode `{mode}`");
        }
    }

    Ok(())
}

fn state_clear_command(args: StateClearArgs) -> Result<()> {
    let mode = parse_state_mode(&args.mode)?;
    let store = StateOperationStore::new(".");
    let mut request = StateClearRequest::new(mode);

    if let Some(session_id) = args.session_id {
        request = request.with_session_id(session_id);
    }
    if args.all_sessions {
        request = request.with_all_sessions(true);
    }

    let result = store
        .state_clear(request)
        .with_context(|| format!("state clear failed for mode `{mode}`"))?;
    let count = result.removed_paths.len();
    let suffix = if count == 1 { "" } else { "s" };
    println!("Cleared {count} state file{suffix}");
    for path in result.removed_paths {
        println!("- {}", path.display());
    }

    Ok(())
}

fn parse_state_mode(mode: &str) -> Result<StateMode> {
    mode.parse()
        .with_context(|| format!("invalid state mode `{mode}`"))
}

fn state_json_field_to_text(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "-".to_string(),
        serde_json::Value::String(text) => text.clone(),
        serde_json::Value::Bool(value) => value.to_string(),
        other => other.to_string(),
    }
}

fn runtime_graph_replay_command(color_mode: ColorMode, args: RuntimeGraphReplayArgs) -> Result<()> {
    let events_path = args.events.unwrap_or_else(|| {
        resolve_events_file_from_marker_in_parents(PathBuf::from(".ralph/events.jsonl"))
    });

    if !events_path.exists() {
        anyhow::bail!(
            "runtime graph replay events file does not exist: {}",
            events_path.display()
        );
    }

    let filter = runtime_graph::RuntimeGraphReplayFilter {
        topic: args.topic,
        instance: args.instance.map(ralph_proto::HatInstanceId::new),
    };
    let report = runtime_graph::RuntimeGraphRecorder::replay_from_events(
        &events_path,
        &args.output,
        filter,
    )?;

    match args.format {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::json!({
                    "output": report.output_path,
                    "events": events_path,
                    "records_read": report.records_read,
                    "workflow_records": report.workflow_records,
                    "delivery_records": report.delivery_records,
                    "lifecycle_records": report.lifecycle_records,
                    "lifecycle_control_records": report.lifecycle_control_records,
                    "full_fidelity": report.full_fidelity,
                })
            );
        }
        OutputFormat::Table => {
            let use_colors = color_mode.should_use_colors();
            let fidelity = if report.full_fidelity {
                "full"
            } else {
                "approximate"
            };

            if use_colors {
                println!(
                    "{}✓{} Runtime replay graph written: {}",
                    colors::GREEN,
                    colors::RESET,
                    report.output_path.display()
                );
            } else {
                println!(
                    "Runtime replay graph written: {}",
                    report.output_path.display()
                );
            }

            println!("Events: {}", events_path.display());
            println!("Fidelity: {fidelity}");
            println!("Records read: {}", report.records_read);
            println!("Workflow records: {}", report.workflow_records);
            println!("Delivery records: {}", report.delivery_records);
            println!("Lifecycle records: {}", report.lifecycle_records);
            println!(
                "Lifecycle control records: {}",
                report.lifecycle_control_records
            );

            if !report.full_fidelity {
                println!(
                    "Warning: replay graph is approximate because durable delivery or lifecycle control records are missing."
                );
            }
        }
    }

    Ok(())
}

fn agents_command(color_mode: ColorMode, args: AgentsArgs) -> Result<()> {
    let use_colors = color_mode.should_use_colors();

    if args.watch && args.format == OutputFormat::Json {
        anyhow::bail!("`ralph agents --watch` 目前只支持表格输出.请移除 `--format json`.");
    }

    if args.watch && args.watch_interval_ms < 1 {
        anyhow::bail!(
            "`--watch-interval-ms` 必须 >= 1,当前为 {}.",
            args.watch_interval_ms
        );
    }

    // 默认行为：在子目录执行时也能自动定位到最近的 `.ralph/agents.json`。
    // 这样用户不需要手工 `cd` 回 workspace root。
    //
    // watch 模式的特殊点:
    // - 如果用户尚未启动并行 run,快照文件可能暂时不存在.
    // - 因此在 watch 循环里,当 `--file` 未指定时我们会持续做“向上遍历探测”,一旦生成就自动发现.
    let mut path = match args.file.clone() {
        Some(path) => path,
        None => find_agents_snapshot_in_parents(".ralph/agents.json")
            .unwrap_or_else(|| PathBuf::from(".ralph/agents.json")),
    };

    if args.watch {
        let interval = Duration::from_millis(args.watch_interval_ms);
        let is_tty = stdout().is_terminal();

        loop {
            // watch 模式下的 auto-detect: 未指定 --file 时,每轮都向上探测一次.
            if args.file.is_none()
                && let Some(found) = find_agents_snapshot_in_parents(".ralph/agents.json")
            {
                path = found;
            }

            // =================================================================
            // 刷新策略:
            // - TTY: 清屏 + 光标归位,原地刷新.
            // - 非 TTY: 追加分隔符,不输出 ANSI 控制序列(便于日志/CI).
            // =================================================================
            if is_tty {
                // ANSI: clear screen + cursor home
                print!("\x1b[2J\x1b[H");
            } else {
                println!("\n---");
            }

            if use_colors {
                println!(
                    "{}Watching{} {} (every {}ms). Ctrl+C to quit.",
                    colors::DIM,
                    colors::RESET,
                    path.display(),
                    args.watch_interval_ms
                );
            } else {
                println!(
                    "Watching {} (every {}ms). Ctrl+C to quit.",
                    path.display(),
                    args.watch_interval_ms
                );
            }

            if !path.exists() {
                if use_colors {
                    println!(
                        "{}No agents snapshot found yet.{} Waiting for `.ralph/agents.json`...",
                        colors::DIM,
                        colors::RESET
                    );
                } else {
                    println!("No agents snapshot found yet. Waiting for `.ralph/agents.json`...");
                }
            } else {
                match fs::read_to_string(&path) {
                    Ok(content) => {
                        match serde_json::from_str::<ralph_core::AgentsSnapshot>(&content) {
                            Ok(snapshot) => {
                                display::print_agents_table(&snapshot, use_colors);
                            }
                            Err(e) => {
                                // 说明:
                                // - watch 模式下不因一次解析失败退出,避免“正在写入/临时损坏”导致体验差.
                                // - 下轮刷新可能就恢复了.
                                if use_colors {
                                    println!(
                                        "{}Invalid agents snapshot JSON:{} {}",
                                        colors::RED,
                                        colors::RESET,
                                        e
                                    );
                                } else {
                                    println!("Invalid agents snapshot JSON: {e}");
                                }
                            }
                        }
                    }
                    Err(e) => {
                        if use_colors {
                            println!("{}Failed to read:{} {}", colors::RED, colors::RESET, e);
                        } else {
                            println!("Failed to read: {e}");
                        }
                    }
                }
            }

            // Flush: 确保在被 kill/管道场景下尽量不丢输出.
            let _ = stdout().flush();
            std::thread::sleep(interval);
        }
    }

    if !path.exists() {
        if use_colors {
            println!(
                "{}No agents snapshot found.{} Run `ralph run` (parallel mode) to generate `.ralph/agents.json`.",
                colors::DIM,
                colors::RESET
            );
        } else {
            println!(
                "No agents snapshot found. Run `ralph run` (parallel mode) to generate `.ralph/agents.json`."
            );
        }
        return Ok(());
    }

    let content =
        fs::read_to_string(&path).with_context(|| format!("Failed to read: {}", path.display()))?;
    let snapshot: ralph_core::AgentsSnapshot = serde_json::from_str(&content)
        .with_context(|| format!("Invalid agents snapshot JSON: {}", path.display()))?;

    match args.format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&snapshot)?;
            println!("{json}");
        }
        OutputFormat::Table => {
            display::print_agents_table(&snapshot, use_colors);
        }
    }

    Ok(())
}

fn find_agents_snapshot_in_parents(relative: &str) -> Option<PathBuf> {
    find_file_in_parents(relative)
}

fn find_file_in_parents(relative: &str) -> Option<PathBuf> {
    // 说明：
    // - 从当前工作目录向上遍历父目录,寻找最近的目标文件。
    // - 这是 best-effort 的 UX 改良: 找不到就返回 None,由调用方决定回退策略。
    let cwd = std::env::current_dir().ok()?;
    find_file_in_parents_from(&cwd, relative)
}

fn find_file_in_parents_from(start: &Path, relative: &str) -> Option<PathBuf> {
    for dir in start.ancestors() {
        let candidate = dir.join(relative);
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

fn resolve_events_file_from_marker_in_parents(fallback: PathBuf) -> PathBuf {
    // 说明:
    // - 从当前目录向上查找 `.ralph/current-events` marker。
    // - 找到则解析 marker 的内容(支持绝对/相对)。
    // - 找不到或解析失败则回退到 fallback。
    let Ok(cwd) = std::env::current_dir() else {
        return fallback;
    };
    resolve_events_file_from_marker_in_parents_from(&cwd, fallback)
}

fn resolve_events_file_from_marker_in_parents_from(start: &Path, fallback: PathBuf) -> PathBuf {
    let Some(marker_path) = find_file_in_parents_from(start, ".ralph/current-events") else {
        return fallback;
    };
    resolve_events_file_from_marker(&marker_path).unwrap_or(fallback)
}

fn resolve_events_file_from_marker(marker_path: &Path) -> Option<PathBuf> {
    // 说明:
    // - marker 文件位于 `<workspace_root>/.ralph/current-events`。
    // - marker 内容是一行路径,通常是相对 `<workspace_root>` 的相对路径。
    // - 如果 marker 内容是绝对路径,则直接使用。
    let raw = fs::read_to_string(marker_path).ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let configured = PathBuf::from(trimmed);
    if configured.is_absolute() {
        return Some(configured);
    }

    // 注意: marker 内容的相对路径是相对 `<workspace_root>`(包含 `.ralph/` 的那个目录),
    // 而不是相对 `.ralph/` 目录本身。
    let workspace_root = marker_path.parent()?.parent()?;
    Some(workspace_root.join(configured))
}

fn clean_command(config_path: PathBuf, color_mode: ColorMode, args: CleanArgs) -> Result<()> {
    let use_colors = color_mode.should_use_colors();

    // If --diagnostics flag is set, clean diagnostics directory
    if args.diagnostics {
        let workspace_root = std::env::current_dir().context("Failed to get current directory")?;
        return ralph_cli::clean_diagnostics(&workspace_root, use_colors, args.dry_run);
    }

    // Otherwise, clean .agent directory (existing behavior)
    // Load configuration
    let config = if config_path.exists() {
        RalphConfig::from_file(&config_path)
            .with_context(|| format!("Failed to load config from {:?}", config_path))?
    } else {
        warn!("Config file {:?} not found, using defaults", config_path);
        RalphConfig::default()
    };

    // Extract the .agent directory path from scratchpad path
    let scratchpad_path = Path::new(&config.core.scratchpad);
    let agent_dir = scratchpad_path.parent().ok_or_else(|| {
        anyhow::anyhow!(
            "Could not determine parent directory from scratchpad path: {}",
            config.core.scratchpad
        )
    })?;

    // Check if directory exists
    if !agent_dir.exists() {
        // Not an error - just inform user
        if use_colors {
            println!(
                "{}Nothing to clean:{} Directory '{}' does not exist",
                colors::DIM,
                colors::RESET,
                agent_dir.display()
            );
        } else {
            println!(
                "Nothing to clean: Directory '{}' does not exist",
                agent_dir.display()
            );
        }
        return Ok(());
    }

    // Dry run mode - list what would be deleted
    if args.dry_run {
        if use_colors {
            println!(
                "{}Dry run mode:{} Would delete directory and all contents:",
                colors::CYAN,
                colors::RESET
            );
        } else {
            println!("Dry run mode: Would delete directory and all contents:");
        }
        println!("  {}", agent_dir.display());

        // List directory contents
        list_directory_contents(agent_dir, use_colors, 1)?;

        return Ok(());
    }

    // Perform actual deletion
    fs::remove_dir_all(agent_dir).with_context(|| {
        format!(
            "Failed to delete directory '{}'. Check permissions and try again.",
            agent_dir.display()
        )
    })?;

    // Success message
    if use_colors {
        println!(
            "{}✓{} Cleaned: Deleted '{}' and all contents",
            colors::GREEN,
            colors::RESET,
            agent_dir.display()
        );
    } else {
        println!(
            "Cleaned: Deleted '{}' and all contents",
            agent_dir.display()
        );
    }

    Ok(())
}

/// Emit an event to the current run's events file with proper JSON formatting.
///
/// This command provides a deterministic way for agents to emit events without
/// risking malformed JSONL from manual echo commands. All JSON serialization
/// is handled via serde_json, ensuring proper escaping of payloads.
///
/// Events are written to the path specified in `.ralph/current-events` marker file
/// (created by `ralph run`), or falls back to `.ralph/events.jsonl` if no marker exists.
fn emit_command(color_mode: ColorMode, args: EmitArgs) -> Result<()> {
    let use_colors = color_mode.should_use_colors();

    // 先做控制面 fail-closed 校验,避免把非法 turn_action 写入 external JSONL。
    // 说明:
    // - 这里是“最快反馈层”,让发起方(尤其是 hat 内工具调用)立刻看到可行动错误。
    // - Supervisor 侧仍会做最终裁判,形成 defense-in-depth。
    let hat_instance_env = std::env::var("RALPH_HAT_INSTANCE_ID").ok();
    validate_emit_control_plane_args(&args, hat_instance_env.as_deref())?;

    // Generate timestamp if not provided
    let ts = args.ts.unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

    // ------------------------------------------------------------------
    // 说明:
    // - 这些可选字段会直接写入外部事件 JSONL。
    // - 字段名与取值必须与 `ralph_core::event_reader::Event` 对齐,
    //   否则 Supervisor/EventReader 将读不到这些信号。
    // ------------------------------------------------------------------
    let workspace_strategy = args.workspace_strategy.map(|v| v.to_string());
    let session_strategy = args.session_strategy.map(|v| v.to_string());
    let turn_action = args.turn_action.map(|v| v.to_string());
    let spawn_instance = if args.spawn_instance {
        Some(true)
    } else {
        None
    };

    // Validate JSON payload if --json flag is set
    let payload = if args.json && !args.payload.is_empty() {
        // Validate it's valid JSON
        serde_json::from_str::<serde_json::Value>(&args.payload).context("Invalid JSON payload")?;
        args.payload
    } else {
        args.payload
    };

    // Build the event record
    // We use serde_json directly to ensure proper escaping
    let record = serde_json::json!({
        "topic": args.topic,
        "payload": if args.json && !payload.is_empty() {
            // Parse and embed as object
            serde_json::from_str::<serde_json::Value>(&payload)?
        } else if payload.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::Value::String(payload)
        },
        "ts": ts,
        "target": args.target,
        "target_instance": args.target_instance,
        "spawn_instance": spawn_instance,
        "workspace_strategy": workspace_strategy,
        "session_strategy": session_strategy,
        "turn_action": turn_action,
    });

    // Read events path from marker file, fall back to CLI arg if marker doesn't exist.
    //
    // 说明:
    // - marker 文件由 `ralph run` 创建: `.ralph/current-events`。
    // - 为了避免在子目录(例如 `.ralph/worktrees/...`)执行 `ralph emit` 时写错文件,
    //   这里会向上遍历父目录寻找最近的 marker,并把 marker 指向的 events 路径解析为绝对路径。
    let events_file = resolve_events_file_from_marker_in_parents(args.file.clone());

    // Ensure parent directory exists
    if let Some(parent) = events_file.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    // Append to file
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&events_file)
        .with_context(|| format!("Failed to open events file: {}", events_file.display()))?;

    // Write as single-line JSON (JSONL format)
    let json_line = serde_json::to_string(&record)?;
    writeln!(file, "{}", json_line)?;

    // Success message
    if use_colors {
        println!(
            "{}✓{} Event emitted: {}",
            colors::GREEN,
            colors::RESET,
            args.topic
        );
    } else {
        println!("Event emitted: {}", args.topic);
    }

    Ok(())
}

/// 校验 `ralph emit` 中 control-plane 相关参数的 fail-closed 规则。
///
/// 规则:
/// - 在 hat job 环境(`RALPH_HAT_INSTANCE_ID` 存在)下,禁止 `--turn-action steer|interrupt`。
/// - 使用 `--turn-action steer|interrupt` 时:
///   - 必须 `--target-instance ralph#1`
///   - 禁止 `--target`
///   - 禁止 `--spawn-instance`
fn validate_emit_control_plane_args(args: &EmitArgs, hat_instance_env: Option<&str>) -> Result<()> {
    let Some(turn_action) = args.turn_action else {
        return Ok(());
    };

    if !matches!(
        turn_action,
        EmitTurnAction::Steer | EmitTurnAction::Interrupt
    ) {
        return Ok(());
    }

    if let Some(instance_id) = hat_instance_env
        && !instance_id.trim().is_empty()
    {
        anyhow::bail!(
            "control-plane turn_action is reserved for ExternalInput: hat instance \"{instance_id}\" cannot use `--turn-action {turn_action}`. \
Please remove `--turn-action` and emit a data-plane topic instead (for example `subtask.request` / `subtask.result`)."
        );
    }

    if args.target.is_some() {
        anyhow::bail!(
            "invalid control-plane routing: `--turn-action {turn_action}` cannot be combined with `--target`. \
Use `--target-instance ralph#1` explicitly."
        );
    }

    if args.spawn_instance {
        anyhow::bail!(
            "invalid control-plane routing: `--turn-action {turn_action}` cannot be combined with `--spawn-instance`. \
Use `--target-instance ralph#1` to control an existing in-flight turn."
        );
    }

    let target_instance = args.target_instance.as_deref().map(str::trim);
    if target_instance != Some("ralph#1") {
        anyhow::bail!(
            "invalid control-plane target: `--turn-action {turn_action}` requires `--target-instance ralph#1` (got {}).",
            args.target_instance
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("<missing>")
        );
    }

    Ok(())
}

/// Starts a Prompt-Driven Development planning session.
///
/// This is a thin wrapper that bypasses Ralph's event loop entirely.
/// It spawns the AI backend with the bundled PDD SOP for interactive planning.
fn plan_command(config_path: PathBuf, color_mode: ColorMode, args: PlanArgs) -> Result<()> {
    use sop_runner::{Sop, SopRunConfig, SopRunError};

    let use_colors = color_mode.should_use_colors();

    // Show what we're starting
    if use_colors {
        println!(
            "{}🎯{} Starting {} session...",
            colors::CYAN,
            colors::RESET,
            Sop::Pdd.name()
        );
    } else {
        println!("Starting {} session...", Sop::Pdd.name());
    }

    let config = SopRunConfig {
        sop: Sop::Pdd,
        user_input: args.idea,
        backend_override: args.backend,
        config_path: Some(config_path),
    };

    sop_runner::run_sop(config).map_err(|e| match e {
        SopRunError::NoBackend(no_backend) => anyhow::Error::new(no_backend),
        SopRunError::UnknownBackend(name) => anyhow::anyhow!(
            "Unknown backend: {}\n\nValid backends: claude, kiro, gemini, codex, amp",
            name
        ),
        SopRunError::SpawnError(io_err) => anyhow::anyhow!("Failed to spawn backend: {}", io_err),
    })
}

/// Starts a code-task-generator session.
///
/// This is a thin wrapper that bypasses Ralph's event loop entirely.
/// It spawns the AI backend with the bundled code-task-generator SOP.
fn code_task_command(
    config_path: PathBuf,
    color_mode: ColorMode,
    args: CodeTaskArgs,
) -> Result<()> {
    use sop_runner::{Sop, SopRunConfig, SopRunError};

    let use_colors = color_mode.should_use_colors();

    // Show what we're starting
    if use_colors {
        println!(
            "{}📋{} Starting {} session...",
            colors::CYAN,
            colors::RESET,
            Sop::CodeTaskGenerator.name()
        );
    } else {
        println!("Starting {} session...", Sop::CodeTaskGenerator.name());
    }

    let config = SopRunConfig {
        sop: Sop::CodeTaskGenerator,
        user_input: args.input,
        backend_override: args.backend,
        config_path: Some(config_path),
    };

    sop_runner::run_sop(config).map_err(|e| match e {
        SopRunError::NoBackend(no_backend) => anyhow::Error::new(no_backend),
        SopRunError::UnknownBackend(name) => anyhow::anyhow!(
            "Unknown backend: {}\n\nValid backends: claude, kiro, gemini, codex, amp",
            name
        ),
        SopRunError::SpawnError(io_err) => anyhow::anyhow!("Failed to spawn backend: {}", io_err),
    })
}

/// Lists directory contents recursively for dry-run mode.
fn list_directory_contents(path: &Path, use_colors: bool, indent: usize) -> Result<()> {
    let entries = fs::read_dir(path)?;
    let indent_str = "  ".repeat(indent);

    for entry in entries {
        let entry = entry?;
        let entry_path = entry.path();
        let file_name = entry.file_name();

        if entry_path.is_dir() {
            if use_colors {
                println!(
                    "{}{}{}/{}",
                    indent_str,
                    colors::BLUE,
                    file_name.to_string_lossy(),
                    colors::RESET
                );
            } else {
                println!("{}{}/", indent_str, file_name.to_string_lossy());
            }
            list_directory_contents(&entry_path, use_colors, indent + 1)?;
        } else if use_colors {
            println!(
                "{}{}{}{}",
                indent_str,
                colors::DIM,
                file_name.to_string_lossy(),
                colors::RESET
            );
        } else {
            println!("{}{}", indent_str, file_name.to_string_lossy());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_args_show_stderr_defaults_to_true() {
        // 说明：并行模式默认显示 stderr（便于调试），只有显式 `--hide-stderr` 才隐藏。
        let args = RunArgs::parse_from(["run"]);
        assert!(args.show_stderr);
    }

    #[test]
    fn run_args_hide_stderr_sets_show_stderr_false() {
        // 说明：`--hide-stderr` 是显式降噪开关。
        let args = RunArgs::parse_from(["run", "--hide-stderr"]);
        assert!(!args.show_stderr);
    }

    #[test]
    fn resume_args_show_stderr_defaults_to_true() {
        // 说明：resume 与 run 的默认策略保持一致。
        let args = ResumeArgs::parse_from(["resume"]);
        assert!(args.show_stderr);
    }

    #[test]
    fn resume_args_hide_stderr_sets_show_stderr_false() {
        // 说明：resume 也支持显式隐藏 stderr。
        let args = ResumeArgs::parse_from(["resume", "--hide-stderr"]);
        assert!(!args.show_stderr);
    }

    #[test]
    fn run_args_plain_defaults_to_false() {
        let args = RunArgs::parse_from(["run"]);
        assert!(!args.plain);
    }

    #[test]
    fn run_args_plain_sets_true() {
        let args = RunArgs::parse_from(["run", "--plain"]);
        assert!(args.plain);
    }

    #[test]
    fn resume_args_plain_defaults_to_false() {
        let args = ResumeArgs::parse_from(["resume"]);
        assert!(!args.plain);
    }

    #[test]
    fn resume_args_plain_sets_true() {
        let args = ResumeArgs::parse_from(["resume", "--plain"]);
        assert!(args.plain);
    }

    #[test]
    fn test_verbosity_cli_quiet() {
        assert_eq!(Verbosity::resolve(false, true), Verbosity::Quiet);
    }

    #[test]
    fn test_verbosity_cli_verbose() {
        assert_eq!(Verbosity::resolve(true, false), Verbosity::Verbose);
    }

    #[test]
    fn test_verbosity_default() {
        assert_eq!(Verbosity::resolve(false, false), Verbosity::Normal);
    }

    #[test]
    fn cli_config_explicit_detector_handles_global_config_forms() {
        assert!(cli_config_was_explicit([
            OsStr::new("ralph"),
            OsStr::new("--config"),
            OsStr::new("ralph.yml"),
            OsStr::new("run"),
        ]));
        assert!(cli_config_was_explicit([
            OsStr::new("ralph"),
            OsStr::new("run"),
            OsStr::new("--config=custom.yml"),
        ]));
        assert!(cli_config_was_explicit([
            OsStr::new("ralph"),
            OsStr::new("-ccustom.yml"),
            OsStr::new("run"),
        ]));
    }

    #[test]
    fn cli_config_explicit_detector_ignores_backend_custom_args_after_separator() {
        assert!(!cli_config_was_explicit([
            OsStr::new("ralph"),
            OsStr::new("run"),
            OsStr::new("--"),
            OsStr::new("--config"),
            OsStr::new("backend.yml"),
        ]));
    }

    #[test]
    fn test_config_source_parse_builtin() {
        let source = ConfigSource::parse("builtin:tdd-red-green");
        match source {
            ConfigSource::Builtin(name) => assert_eq!(name, "tdd-red-green"),
            _ => panic!("Expected Builtin variant"),
        }
    }

    #[test]
    fn test_config_source_parse_remote_https() {
        let source = ConfigSource::parse("https://example.com/preset.yml");
        match source {
            ConfigSource::Remote(url) => assert_eq!(url, "https://example.com/preset.yml"),
            _ => panic!("Expected Remote variant"),
        }
    }

    #[test]
    fn test_config_source_parse_remote_http() {
        let source = ConfigSource::parse("http://example.com/preset.yml");
        match source {
            ConfigSource::Remote(url) => assert_eq!(url, "http://example.com/preset.yml"),
            _ => panic!("Expected Remote variant"),
        }
    }

    #[test]
    fn test_config_source_parse_file() {
        let source = ConfigSource::parse("ralph.yml");
        match source {
            ConfigSource::File(path) => assert_eq!(path, std::path::PathBuf::from("ralph.yml")),
            _ => panic!("Expected File variant"),
        }
    }

    #[test]
    fn resolve_current_events_marker_relative_path_is_workspace_root_relative() {
        // 说明:
        // - `.ralph/current-events` 的内容通常是相对路径(例如 `.ralph/events-<id>.jsonl`)。
        // - 该相对路径应当以 "workspace root"(包含 `.ralph/` 的目录)为基准解析,
        //   而不是以 `.ralph/` 目录为基准解析。
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let workspace_root = temp_dir.path();

        std::fs::create_dir_all(workspace_root.join(".ralph")).expect("mkdir .ralph");
        std::fs::write(
            workspace_root.join(".ralph/current-events"),
            ".ralph/events-123.jsonl\n",
        )
        .expect("write marker");

        let nested = workspace_root.join("a/b/c");
        std::fs::create_dir_all(&nested).expect("mkdir nested");

        let fallback = PathBuf::from("fallback.jsonl");
        let resolved = resolve_events_file_from_marker_in_parents_from(&nested, fallback);

        assert_eq!(resolved, workspace_root.join(".ralph/events-123.jsonl"));
    }

    #[test]
    fn resolve_current_events_marker_absolute_path_is_used_as_is() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let workspace_root = temp_dir.path();

        std::fs::create_dir_all(workspace_root.join(".ralph")).expect("mkdir .ralph");
        let absolute_events_path = workspace_root.join("events-abs.jsonl");
        std::fs::write(
            workspace_root.join(".ralph/current-events"),
            absolute_events_path.to_string_lossy().to_string(),
        )
        .expect("write marker");

        let nested = workspace_root.join("subdir");
        std::fs::create_dir_all(&nested).expect("mkdir nested");

        let fallback = PathBuf::from("fallback.jsonl");
        let resolved = resolve_events_file_from_marker_in_parents_from(&nested, fallback);

        assert_eq!(resolved, absolute_events_path);
    }

    #[test]
    fn resolve_current_events_marker_missing_falls_back() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let workspace_root = temp_dir.path();
        let nested = workspace_root.join("a/b");
        std::fs::create_dir_all(&nested).expect("mkdir nested");

        let fallback = PathBuf::from("fallback.jsonl");
        let resolved = resolve_events_file_from_marker_in_parents_from(&nested, fallback.clone());

        assert_eq!(resolved, fallback);
    }

    #[test]
    fn resolve_current_events_marker_blank_falls_back() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let workspace_root = temp_dir.path();

        std::fs::create_dir_all(workspace_root.join(".ralph")).expect("mkdir .ralph");
        std::fs::write(workspace_root.join(".ralph/current-events"), "   \n")
            .expect("write marker");

        let nested = workspace_root.join("a");
        std::fs::create_dir_all(&nested).expect("mkdir nested");

        let fallback = PathBuf::from("fallback.jsonl");
        let resolved = resolve_events_file_from_marker_in_parents_from(&nested, fallback.clone());

        assert_eq!(resolved, fallback);
    }

    #[test]
    fn emit_turn_action_rejects_hat_environment() {
        let args = EmitArgs {
            topic: "human.message".to_string(),
            payload: "hi".to_string(),
            json: false,
            ts: None,
            file: PathBuf::from(".ralph/events.jsonl"),
            target_instance: Some("ralph#1".to_string()),
            target: None,
            spawn_instance: false,
            workspace_strategy: None,
            session_strategy: None,
            turn_action: Some(EmitTurnAction::Steer),
        };

        let err = validate_emit_control_plane_args(&args, Some("writer#1"))
            .expect_err("hat env must be rejected");
        let message = err.to_string();
        assert!(message.contains("reserved for ExternalInput"));
        assert!(message.contains("cannot use `--turn-action steer`"));
    }

    #[test]
    fn emit_turn_action_rejects_missing_target_instance() {
        let args = EmitArgs {
            topic: "human.message".to_string(),
            payload: "hi".to_string(),
            json: false,
            ts: None,
            file: PathBuf::from(".ralph/events.jsonl"),
            target_instance: None,
            target: None,
            spawn_instance: false,
            workspace_strategy: None,
            session_strategy: None,
            turn_action: Some(EmitTurnAction::Interrupt),
        };

        let err = validate_emit_control_plane_args(&args, None)
            .expect_err("missing target_instance must be rejected");
        assert!(
            err.to_string()
                .contains("requires `--target-instance ralph#1`")
        );
    }

    #[test]
    fn emit_turn_action_rejects_non_ralph_primary_target() {
        let args = EmitArgs {
            topic: "human.message".to_string(),
            payload: "hi".to_string(),
            json: false,
            ts: None,
            file: PathBuf::from(".ralph/events.jsonl"),
            target_instance: Some("writer#1".to_string()),
            target: None,
            spawn_instance: false,
            workspace_strategy: None,
            session_strategy: None,
            turn_action: Some(EmitTurnAction::Steer),
        };

        let err = validate_emit_control_plane_args(&args, None)
            .expect_err("non-ralph target must be rejected");
        assert!(err.to_string().contains("got writer#1"));
    }

    #[test]
    fn emit_turn_action_rejects_target_hat_hint() {
        let args = EmitArgs {
            topic: "human.message".to_string(),
            payload: "hi".to_string(),
            json: false,
            ts: None,
            file: PathBuf::from(".ralph/events.jsonl"),
            target_instance: Some("ralph#1".to_string()),
            target: Some("writer".to_string()),
            spawn_instance: false,
            workspace_strategy: None,
            session_strategy: None,
            turn_action: Some(EmitTurnAction::Steer),
        };

        let err = validate_emit_control_plane_args(&args, None)
            .expect_err("target hint must be rejected");
        assert!(
            err.to_string()
                .contains("cannot be combined with `--target`")
        );
    }

    #[test]
    fn emit_turn_action_rejects_spawn_instance_hint() {
        let args = EmitArgs {
            topic: "human.message".to_string(),
            payload: "hi".to_string(),
            json: false,
            ts: None,
            file: PathBuf::from(".ralph/events.jsonl"),
            target_instance: Some("ralph#1".to_string()),
            target: None,
            spawn_instance: true,
            workspace_strategy: None,
            session_strategy: None,
            turn_action: Some(EmitTurnAction::Interrupt),
        };

        let err = validate_emit_control_plane_args(&args, None)
            .expect_err("spawn_instance hint must be rejected");
        assert!(
            err.to_string()
                .contains("cannot be combined with `--spawn-instance`")
        );
    }
}
