//! # ralph-e2e
//!
//! End-to-end test harness for the Ralph Orchestrator.
//!
//! This binary validates Ralph's behavior against real AI backends (Claude, Kiro, OpenCode).
//! It exercises the full orchestration loop including:
//! - Backend connectivity and authentication
//! - Event parsing and routing
//! - Hat collection workflows
//! - Memory system functionality
//!
//! ## Usage
//!
//! ```bash
//! # Run all tests for all available backends
//! ralph-e2e all
//!
//! # Run tests for a specific backend
//! ralph-e2e claude
//!
//! # List available scenarios
//! ralph-e2e --list
//! ```

use clap::{Parser, Subcommand, ValueEnum};
use colored::Colorize;
use ralph_e2e::{
    AuthChecker,
    // Tier 7: Error Handling
    AuthFailureScenario,
    Backend as LibBackend,
    BackendUnavailableScenario,
    // Tier 3: Events
    // Tier 2: Orchestration Loop
    // Tier 1: Connectivity
    // Tier 5: Hat Collections
    HatBackendOverrideScenario,
    HatEventRoutingScenario,
    HatInstructionsScenario,
    HatMultiWorkflowScenario,
    HatSingleScenario,
    MaxIterationsScenario,
    // Tier 6: Memory System
    MemoryAddScenario,
    MemoryCorruptedFileScenario,
    MemoryInjectionScenario,
    MemoryLargeContentScenario,
    MemoryMissingFileScenario,
    MemoryPersistenceScenario,
    MemoryRapidWriteScenario,
    MemorySearchScenario,
    MockCliError,
    MockConfig,
    ParallelAppServerIdleStartScenario,
    // Tier 8: Parallel Runtime
    ParallelAppServerSteerLiveReplyMultiTurnScenario,
    ParallelAppServerSteerMultiTurnLiveScenario,
    ParallelAppServerSteerMultiTurnScenario,
    ParallelAuditEvidencePackExampleScenario,
    ParallelCustomerAdvisoryBoardPrepExampleScenario,
    ParallelCustomerOnboardingActivationExampleScenario,
    ParallelCustomerRenewalDeskExampleScenario,
    ParallelEmitSpawnInstanceScenario,
    ParallelExecutiveBusinessReviewPrepExampleScenario,
    ParallelExperimentalDevEngineExampleScenario,
    ParallelFieldEnablementRolloutExampleScenario,
    ParallelFinanceCloseControlRoomExampleScenario,
    ParallelHatInstancesScenario,
    ParallelHiringDebriefPanelExampleScenario,
    ParallelHumanApprovalGateExampleScenario,
    ParallelIncidentResponseWarRoomExampleScenario,
    ParallelLaunchReadinessCommandExampleScenario,
    ParallelMigrationRehearsalExampleScenario,
    ParallelMultiRegionPipelineSyncExampleScenario,
    ParallelPartnerLaunchCoordinationExampleScenario,
    ParallelPostmortemActionBoardExampleScenario,
    ParallelPrReviewExampleScenario,
    ParallelProposalAssemblyExampleScenario,
    ParallelRegionalOperatingReviewExampleScenario,
    ParallelReleaseChecklistExampleScenario,
    ParallelRenewalRiskCalibrationExampleScenario,
    ParallelRevopsQuoteDeskExampleScenario,
    ParallelSecurityExceptionReviewExampleScenario,
    ParallelStartingEventInferenceScenario,
    ParallelSupportEscalationDeskExampleScenario,
    ParallelTriggerRoutingExampleScenario,
    ParallelVendorSecurityProcurementExampleScenario,
    ReportFormat as LibReportFormat,
    ReportWriter,
    RunConfig,
    // Tier 4: Capabilities
    StreamingScenario,
    TerminalReporter,
    TestRunner,
    TestScenario,
    TimeoutScenario,
    ToolUseScenario,
    Verbosity,
    WorkspaceManager,
    create_incremental_progress_callback,
    resolve_ralph_binary,
    run_mock_cli,
};

/// Backend selection for E2E tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum Backend {
    /// Test all available backends
    #[default]
    All,
    /// Test Claude backend only
    Claude,
    /// Test Kiro backend only
    Kiro,
    /// Test Codex backend only
    Codex,
    /// Test OpenCode backend only
    Opencode,
}

impl std::fmt::Display for Backend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Backend::All => write!(f, "all"),
            Backend::Claude => write!(f, "claude"),
            Backend::Kiro => write!(f, "kiro"),
            Backend::Codex => write!(f, "codex"),
            Backend::Opencode => write!(f, "opencode"),
        }
    }
}

impl Backend {
    /// Converts CLI backend to library backend (if not All).
    fn to_lib_backend(self) -> Option<LibBackend> {
        match self {
            Backend::All => None,
            Backend::Claude => Some(LibBackend::Claude),
            Backend::Kiro => Some(LibBackend::Kiro),
            Backend::Codex => Some(LibBackend::Codex),
            Backend::Opencode => Some(LibBackend::OpenCode),
        }
    }
}

/// E2E test harness for Ralph orchestrator.
///
/// Validates Ralph's behavior against real AI backends to ensure
/// the orchestration loop works correctly before releases.
#[derive(Parser, Debug)]
#[command(name = "ralph-e2e")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// 子命令（用于 mock-cli 回放等“被 Ralph 当成后端调用”的模式）。
    #[command(subcommand)]
    pub command: Option<Command>,

    /// E2E 测试运行参数（默认路径）。
    #[command(flatten)]
    pub test_opts: TestOpts,
}

/// `ralph-e2e` 的子命令集合。
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Mock CLI 适配器：回放 cassette，作为自定义 backend 被 `ralph run` 调用。
    MockCli {
        /// 要回放的 cassette 文件路径（JSONL）
        #[arg(long)]
        cassette: std::path::PathBuf,

        /// 回放速度倍率（0.0=尽可能快；1.0=实时；10.0=10x）
        #[arg(long, default_value = "0.0")]
        speed: f32,

        /// 允许执行的命令前缀白名单（逗号分隔）
        ///
        /// 说明：
        /// - 仅当 cassette 中包含 `bus.*` 事件里提取出的“可执行命令”时才会用到
        /// - 可用环境变量 `RALPH_MOCK_ALLOW` 覆盖（用于 CI 注入）
        #[arg(long)]
        allow: Option<String>,
    },
}

/// E2E 测试运行参数。
#[derive(Parser, Debug)]
pub struct TestOpts {
    /// Backend to test
    #[arg(value_enum, default_value_t = Backend::All)]
    pub backend: Backend,

    /// Show detailed output during tests
    #[arg(short, long)]
    pub verbose: bool,

    /// Only show pass/fail summary
    #[arg(short, long)]
    pub quiet: bool,

    /// List available test scenarios without running them
    #[arg(long)]
    pub list: bool,

    /// Run only tests matching this pattern
    #[arg(long)]
    pub filter: Option<String>,

    /// Generate report in specified format
    #[arg(long, value_enum, default_value_t = ReportFormat::Markdown)]
    pub report: ReportFormat,

    /// Keep test workspaces after tests complete (for debugging)
    #[arg(long)]
    pub keep_workspace: bool,

    /// Skip meta-Ralph analysis (faster, raw results only)
    #[arg(long)]
    pub skip_analysis: bool,

    /// mock-mode：用 cassette 回放代替真实后端（零成本、确定性）
    #[arg(long)]
    pub mock: bool,

    /// mock-mode：回放速度倍率（0.0=尽可能快；10.0=10x）
    #[arg(long, default_value = "0.0")]
    pub mock_speed: f32,

    /// mock-mode：cassette 目录（默认：`cassettes/e2e`，会相对 repo root 解析）
    #[arg(long)]
    pub cassette_dir: Option<std::path::PathBuf>,

    /// mock-mode：允许执行的命令前缀白名单（逗号分隔）
    #[arg(long)]
    pub mock_allow: Option<String>,
}

/// Report output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum ReportFormat {
    /// Markdown format (agent-readable, and refreshes JSON snapshot)
    #[default]
    Markdown,
    /// JSON format (machine-readable)
    Json,
    /// Both markdown and JSON
    Both,
}

impl ReportFormat {
    /// Converts CLI report format to the effective library report format.
    ///
    /// Default markdown mode also refreshes `report.json` so both artifacts
    /// always reflect the same run and avoid stale JSON confusion.
    fn to_lib_format(self) -> LibReportFormat {
        match self {
            ReportFormat::Markdown => LibReportFormat::Both,
            ReportFormat::Json => LibReportFormat::Json,
            ReportFormat::Both => LibReportFormat::Both,
        }
    }
}

/// Returns all registered test scenarios.
fn get_all_scenarios() -> Vec<Box<dyn TestScenario>> {
    vec![
        // Tier 1: Connectivity (backend-agnostic)
        // connectivity 已声明化(候选6)
        Box::new(ralph_e2e::declarative::from_yaml(
            "connectivity",
            include_str!("../scenarios/connectivity.yaml"),
        )),
        // Tier 2: Orchestration Loop (backend-agnostic)
        // single-iter 已声明化(候选6 试点)
        Box::new(ralph_e2e::declarative::from_yaml(
            "single-iter",
            include_str!("../scenarios/single-iter.yaml"),
        )),
        // multi-iter 已声明化(候选6)
        Box::new(ralph_e2e::declarative::from_yaml(
            "multi-iter",
            include_str!("../scenarios/multi-iter.yaml"),
        )),
        // completion 已声明化(候选6)
        Box::new(ralph_e2e::declarative::from_yaml(
            "completion",
            include_str!("../scenarios/completion.yaml"),
        )),
        // Tier 3: Events (backend-agnostic)
        // events 已声明化(候选6)
        Box::new(ralph_e2e::declarative::from_yaml(
            "events",
            include_str!("../scenarios/events.yaml"),
        )),
        // backpressure 已声明化(候选6)
        Box::new(ralph_e2e::declarative::from_yaml(
            "backpressure",
            include_str!("../scenarios/backpressure.yaml"),
        )),
        // Tier 4: Capabilities (backend-agnostic)
        Box::new(ToolUseScenario::new()),
        Box::new(StreamingScenario::new()),
        // Tier 5: Hat Collections (backend-agnostic)
        Box::new(HatSingleScenario::new()),
        Box::new(HatMultiWorkflowScenario::new()),
        Box::new(HatInstructionsScenario::new()),
        Box::new(HatEventRoutingScenario::new()),
        Box::new(HatBackendOverrideScenario::new()),
        // Tier 6: Memory System (backend-agnostic)
        Box::new(MemoryAddScenario::new()),
        Box::new(MemorySearchScenario::new()),
        Box::new(MemoryInjectionScenario::new()),
        Box::new(MemoryPersistenceScenario::new()),
        // Tier 6: Memory System (Chaos Tests)
        Box::new(MemoryCorruptedFileScenario::new()),
        Box::new(MemoryMissingFileScenario::new()),
        Box::new(MemoryRapidWriteScenario::new()),
        Box::new(MemoryLargeContentScenario::new()),
        // Tier 7: Error Handling (backend-agnostic)
        Box::new(TimeoutScenario::new()),
        Box::new(MaxIterationsScenario::new()),
        Box::new(AuthFailureScenario::new()),
        Box::new(BackendUnavailableScenario::new()),
        // Tier 8: Parallel Runtime (experimental)
        Box::new(ParallelHatInstancesScenario::new()),
        Box::new(ParallelHatInstancesScenario::new_zh()),
        Box::new(ParallelStartingEventInferenceScenario::new()),
        Box::new(ParallelStartingEventInferenceScenario::new_multi_candidate()),
        Box::new(ParallelEmitSpawnInstanceScenario::new()),
        Box::new(ParallelAppServerIdleStartScenario::new()),
        // app-server-idle-start-live 已声明化(候选6 inject 试点)
        Box::new(ralph_e2e::declarative::from_yaml(
            "parallel-app-server-idle-start-live",
            include_str!("../scenarios/app-server-idle-start-live.yaml"),
        )),
        Box::new(ParallelAppServerSteerMultiTurnScenario::new()),
        Box::new(ParallelAppServerSteerMultiTurnLiveScenario::new()),
        Box::new(ParallelAppServerSteerLiveReplyMultiTurnScenario::new()),
        Box::new(ParallelTriggerRoutingExampleScenario::new()),
        Box::new(ParallelExperimentalDevEngineExampleScenario::new()),
        Box::new(ParallelPrReviewExampleScenario::new()),
        Box::new(ParallelReleaseChecklistExampleScenario::new()),
        Box::new(ParallelHumanApprovalGateExampleScenario::new()),
        Box::new(ParallelIncidentResponseWarRoomExampleScenario::new()),
        Box::new(ParallelSecurityExceptionReviewExampleScenario::new()),
        Box::new(ParallelCustomerRenewalDeskExampleScenario::new()),
        Box::new(ParallelAuditEvidencePackExampleScenario::new()),
        Box::new(ParallelFinanceCloseControlRoomExampleScenario::new()),
        Box::new(ParallelHiringDebriefPanelExampleScenario::new()),
        Box::new(ParallelCustomerOnboardingActivationExampleScenario::new()),
        Box::new(ParallelSupportEscalationDeskExampleScenario::new()),
        Box::new(ParallelPartnerLaunchCoordinationExampleScenario::new()),
        Box::new(ParallelFieldEnablementRolloutExampleScenario::new()),
        Box::new(ParallelRevopsQuoteDeskExampleScenario::new()),
        Box::new(ParallelExecutiveBusinessReviewPrepExampleScenario::new()),
        Box::new(ParallelCustomerAdvisoryBoardPrepExampleScenario::new()),
        Box::new(ParallelRegionalOperatingReviewExampleScenario::new()),
        Box::new(ParallelRenewalRiskCalibrationExampleScenario::new()),
        Box::new(ParallelMultiRegionPipelineSyncExampleScenario::new()),
        Box::new(ParallelLaunchReadinessCommandExampleScenario::new()),
        Box::new(ParallelMigrationRehearsalExampleScenario::new()),
        Box::new(ParallelPostmortemActionBoardExampleScenario::new()),
        Box::new(ParallelProposalAssemblyExampleScenario::new()),
        Box::new(ParallelVendorSecurityProcurementExampleScenario::new()),
    ]
}

fn main() {
    let cli = Cli::parse();

    // 子命令优先处理：mock-cli 会被 `ralph run` 当作 custom backend 调用。
    if let Some(command) = cli.command {
        match command {
            Command::MockCli {
                cassette,
                speed,
                allow,
            } => {
                // 说明：
                // - 环境变量优先用于 CI 注入。
                // - 但若该变量存在且为空字符串,应视为“未提供”(避免意外覆盖 CLI 的 allowlist)。
                let allow_from_env = std::env::var("RALPH_MOCK_ALLOW")
                    .ok()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty());
                // 环境变量优先（便于 CI 注入），否则使用 CLI 传入的 allow。
                let allow_effective = allow_from_env.as_deref().or(allow.as_deref());

                match run_mock_cli(&cassette, speed, allow_effective) {
                    Ok(()) => {}
                    Err(e) => {
                        // 参考 docs/mock-cli.md：为不同失败类型返回更可诊断的退出码。
                        let code = match &e {
                            MockCliError::CassetteOpen { .. } => 1,
                            MockCliError::CassetteParse(_) => 2,
                            MockCliError::ReplayError(_) => 3,
                            MockCliError::CommandError(_) => 4,
                        };

                        eprintln!("{} {}", "Error:".red().bold(), e);
                        std::process::exit(code);
                    }
                }
                return;
            }
        }
    }

    // Print header for test runs
    println!(
        "\n{} {}",
        "🧪 E2E Test Harness".bold(),
        format!("v{}", env!("CARGO_PKG_VERSION")).dimmed()
    );
    println!("{}", "━".repeat(40).dimmed());

    if cli.test_opts.mock {
        println!("{}", "Mode: Mock (cassette replay)".dimmed());
    }

    // Determine verbosity
    let verbosity = if cli.test_opts.quiet {
        Verbosity::Quiet
    } else if cli.test_opts.verbose {
        Verbosity::Verbose
    } else {
        Verbosity::Normal
    };

    // Run the tests
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    if cli.test_opts.list {
        rt.block_on(list_scenarios(&cli.test_opts, verbosity));
        return;
    }

    rt.block_on(run_tests(&cli.test_opts, verbosity));
}

async fn list_scenarios(opts: &TestOpts, verbosity: Verbosity) {
    // Check backend availability（mock-mode 不需要真实后端）
    if !opts.mock && verbosity != Verbosity::Quiet {
        println!("\n{}", "Checking backends...".dimmed());
        let checker = AuthChecker::new();
        let backends = checker.check_all().await;

        for info in backends {
            let status = match info.status_string().as_str() {
                s if s.contains("Authenticated") => format!("✅ {} - {}", info.backend, s).green(),
                s if s.contains("Not authenticated") => {
                    format!("⚠️  {} - {}", info.backend, s).yellow()
                }
                s => format!("❌ {} - {}", info.backend, s).red(),
            };
            println!("  {}", status);
        }
        println!();
    }

    // List scenarios
    let scenarios = get_all_scenarios();
    println!("{}\n", "Available scenarios:".bold());

    // Group by tier
    let mut current_tier = String::new();
    for scenario in &scenarios {
        // Filter by backend if specified
        if let Some(backend) = opts.backend.to_lib_backend()
            && !scenario.supported_backends().contains(&backend)
        {
            continue;
        }

        // Print tier header if changed
        if scenario.tier() != current_tier {
            current_tier = scenario.tier().to_string();
            println!("  {}", current_tier.bold().underline());
        }

        println!(
            "    {}  {}",
            scenario.id().cyan(),
            scenario.description().dimmed()
        );
    }

    if scenarios.is_empty() {
        println!("  {}", "No scenarios implemented yet".yellow());
    }

    println!(
        "\n  {}",
        format!(
            "Total: {} scenario{}",
            scenarios.len(),
            if scenarios.len() == 1 { "" } else { "s" }
        )
        .dimmed()
    );
}

async fn run_tests(opts: &TestOpts, verbosity: Verbosity) {
    // Check backend availability first（mock-mode 不需要真实后端）
    if !opts.mock && verbosity != Verbosity::Quiet {
        println!();
        let checker = AuthChecker::new();

        if let Some(backend) = opts.backend.to_lib_backend() {
            let info = checker.check(backend).await;
            let status = info.status_string();
            let status_fmt = if status.contains("Authenticated") {
                format!("{}: {} ✅", info.backend, status).green()
            } else if status.contains("Not authenticated") {
                format!("{}: {} ⚠️", info.backend, status).yellow()
            } else {
                format!("{}: {} ❌", info.backend, status).red()
            };
            println!("{}", status_fmt);
        } else {
            println!("{}", "Checking all backends...".dimmed());
            for info in checker.check_all().await {
                let status = match info.status_string().as_str() {
                    s if s.contains("Authenticated") => {
                        format!("  ✅ {} - {}", info.backend, s).green()
                    }
                    s if s.contains("Not authenticated") => {
                        format!("  ⚠️  {} - {}", info.backend, s).yellow()
                    }
                    s => format!("  ❌ {} - {}", info.backend, s).red(),
                };
                println!("{}", status);
            }
        }
    }

    // Set up workspace manager with absolute path
    // The PTY executor calls std::env::current_dir() which requires the workspace to exist.
    // Using absolute paths ensures the workspace is resolvable regardless of working directory changes.
    let workspace_path = std::env::current_dir()
        .expect("Failed to get current directory")
        .join(".e2e-tests");
    let workspace_mgr = WorkspaceManager::new(workspace_path.clone());

    // Get scenarios
    let scenarios = get_all_scenarios();

    // Build run configuration
    let mut config = RunConfig::new().keep_workspaces(opts.keep_workspace);

    if let Some(filter) = &opts.filter {
        config = config.with_filter(filter);
    }

    if let Some(backend) = opts.backend.to_lib_backend() {
        config = config.with_backend(backend);
    }

    // Configure mock mode if enabled
    if opts.mock {
        let mut mock_config = match &opts.cassette_dir {
            Some(dir) => MockConfig::new(dir),
            None => MockConfig::default(),
        };

        mock_config = mock_config.with_speed(opts.mock_speed);

        if let Some(allow) = &opts.mock_allow {
            mock_config = mock_config.with_allow_commands(allow);
        }

        config = config.with_mock(mock_config);
    }

    // Resolve the ralph binary to use (local build preferred over PATH)
    let ralph_binary = resolve_ralph_binary();
    if verbosity != Verbosity::Quiet {
        println!(
            "{}",
            format!("Using binary: {}", ralph_binary.display()).dimmed()
        );
    }

    // Create runner with incremental progress callback
    let runner = TestRunner::new(workspace_mgr, scenarios)
        .with_binary(ralph_binary)
        .on_progress(create_incremental_progress_callback(
            verbosity,
            workspace_path.clone(),
        ));

    // Notify about live report
    if verbosity != Verbosity::Quiet {
        println!(
            "{}",
            format!(
                "Live report: {}",
                workspace_path.join("report-live.md").display()
            )
            .dimmed()
        );
        println!();
    }

    // Run the tests
    let results = match runner.run(&config).await {
        Ok(results) => results,
        Err(e) => {
            eprintln!("\n{} {}", "Error:".red().bold(), e);
            std::process::exit(1);
        }
    };

    // Write reports to disk
    let report_writer = ReportWriter::new(workspace_path);
    match report_writer.write(&results, None, opts.report.to_lib_format()) {
        Ok(paths) => {
            if verbosity != Verbosity::Quiet {
                for path in &paths {
                    println!("{}", format!("Report written: {}", path.display()).dimmed());
                }
            }
        }
        Err(e) => {
            eprintln!("{} Failed to write report: {}", "Warning:".yellow(), e);
        }
    }

    // Print summary
    let reporter = TerminalReporter::with_verbosity(verbosity);

    if verbosity != Verbosity::Quiet {
        // Print failures in detail
        if !results.all_passed() {
            reporter.print_failures(&results);
        }
    }

    // Always print summary
    reporter.print_summary(&results);

    // Exit with appropriate code
    if !results.all_passed() {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_mode_also_writes_json_snapshot() {
        assert_eq!(
            ReportFormat::Markdown.to_lib_format(),
            LibReportFormat::Both
        );
    }

    #[test]
    fn json_mode_remains_json_only() {
        assert_eq!(ReportFormat::Json.to_lib_format(), LibReportFormat::Json);
    }

    #[test]
    fn both_mode_remains_both() {
        assert_eq!(ReportFormat::Both.to_lib_format(), LibReportFormat::Both);
    }
}
