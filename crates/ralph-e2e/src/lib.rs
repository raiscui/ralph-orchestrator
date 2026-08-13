//! # ralph-e2e
//!
//! End-to-end test harness library for the Ralph Orchestrator.
//!
//! This crate provides the core functionality for validating Ralph's behavior
//! against real AI backends. It is designed to be used both as a CLI tool
//! and as a library for programmatic test execution.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
//! │  TestRunner │────▶│  Scenarios  │────▶│  Executor   │
//! └─────────────┘     └─────────────┘     └─────────────┘
//!        │                                       │
//!        ▼                                       ▼
//! ┌─────────────┐                         ┌─────────────┐
//! │  Reporter   │                         │   Backend   │
//! └─────────────┘                         └─────────────┘
//! ```
//!
//! ## Modules (to be implemented)
//!
//! - `workspace`: Manages isolated test workspaces in `.e2e-tests/`
//! - `auth`: Checks backend availability and authentication
//! - `executor`: Invokes `ralph run` with test configurations
//! - `scenarios`: Defines test scenarios (TestScenario trait)
//! - `runner`: Orchestrates test execution
//! - `reporter`: Generates terminal and file reports
//! - `analyzer`: Meta-Ralph analysis for rich diagnostics

// Re-export common types for library consumers
pub use crate::analyzer::{
    AnalyzedResult, AnalyzerConfig, AnalyzerError, Diagnosis, FailedAnalysis, FailureType,
    MetaRalphAnalyzer, Optimization, PassedAnalysis, PassedTestAnalysis, Pattern, PotentialFix,
    QualityScore, Recommendation, Severity, TestMetrics, Warning, WarningCategory,
};
pub use crate::auth::{AuthChecker, BackendInfo};
pub use crate::backend::Backend;
pub use crate::executor::{
    EventRecord, ExecutionResult, ExecutorError, PromptSource, RalphExecutor, ScenarioConfig,
    resolve_ralph_binary,
};
pub use crate::mock::{
    CassetteError, CassetteResolver, DEFAULT_CASSETTE_DIR, MockConfig, build_mock_cli_args,
};
pub use crate::mock_cli::{MockCliError, run as run_mock_cli};
pub use crate::models::{Assertion, ReportFormat, TestResult};
pub use crate::reporter::{
    AnalyzedResultData, BackendSummary, JsonReporter, MarkdownReporter, QualityBreakdown,
    ReportSummary, ReportWriter, ReporterError, TerminalReporter, TestReport, TierSummary,
    Verbosity, create_incremental_progress_callback, create_progress_callback,
};
pub use crate::runner::{
    ProgressCallback, ProgressEvent, RunConfig, RunResults, RunnerError, TestRunner,
};
pub use crate::scenarios::{
    // Core traits and helpers
    Assertions,
    // Tier 8: Error Handling (backend-agnostic)
    AuthFailureScenario,
    BackendUnavailableScenario,
    // Tier 3: Events (backend-agnostic)
    BackpressureScenario,
    // Tier 7: Incremental Development (backend-agnostic)
    ChainedLoopScenario,
    // Tier 2: Orchestration Loop (backend-agnostic)
    CompletionScenario,
    // Tier 1: Connectivity (backend-agnostic)
    ConnectivityScenario,
    EventsScenario,
    // Tier 5: Hat Collections (backend-agnostic)
    HatBackendOverrideScenario,
    HatEventRoutingScenario,
    HatInstructionsScenario,
    HatMultiWorkflowScenario,
    HatSingleScenario,
    IncrementalFeatureScenario,
    MaxIterationsScenario,
    // Tier 6: Memory System (backend-agnostic)
    MemoryAddScenario,
    MemoryCorruptedFileScenario,
    MemoryInjectionScenario,
    MemoryLargeContentScenario,
    MemoryMissingFileScenario,
    MemoryPersistenceScenario,
    MemoryRapidWriteScenario,
    MemorySearchScenario,
    MultiIterScenario,
    // Tier 8: Parallel Runtime (experimental)
    ParallelAppServerIdleStartLiveScenario,
    ParallelAppServerIdleStartScenario,
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
    ScenarioError,
    SingleIterScenario,
    // Tier 4: Capabilities (backend-agnostic)
    StreamingScenario,
    // Tier 6: Task System (backend-agnostic)
    TaskAddScenario,
    TaskCloseScenario,
    TaskCompletionScenario,
    TaskReadyScenario,
    TestScenario,
    TimeoutScenario,
    ToolUseScenario,
};
pub use crate::workspace::WorkspaceManager;

pub mod analyzer;
pub mod auth;
mod backend;
pub mod declarative;
pub mod executor;
pub mod mock;
pub mod mock_cli;
mod models;
pub mod reporter;
pub mod runner;
pub mod scenarios;
pub mod workspace;

/// Library version, matching the crate version.
/// 场景类型,用于 90 % 声明式覆盖率 CI gate。
///
/// - `Declarative`: 由 `ralph_e2e::declarative::from_yaml` 加载的 YAML 场景。
/// - `Imperative`: 由具体 `TestScenario` impl 注册的命令式场景,被 gate test
///   计入分母。
/// - `ImperativeExplicitKeep`: 命令式场景,但已在 registry 注释中标为「保留
///   命令式」(例如依赖复杂 git seed/commit 工作流)。gate test 显式将其从
///   分母中扣除,使 90 % 阈值在设计上可达。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenarioKind {
    /// 由 YAML 文件驱动的声明式场景。
    Declarative,
    /// 由 Rust `TestScenario` impl 驱动的命令式场景。
    Imperative,
    /// 命令式场景,但显式标记为「保留命令式」,不计入覆盖率分母。
    ImperativeExplicitKeep,
}

/// 暴露给 lib 消费者的完整场景注册表(含 kind 标签),作为 CI gate test 与
/// CLI 列表/运行路径的单一真相源。
///
/// 返回元组 `(ScenarioKind, &'static str, Box<dyn TestScenario>)`:
/// - 元组第一项是该条目的 `ScenarioKind`(CI gate test 据此区分声明式/命令式)。
/// - 元组第二项是稳定场景 id,用于诊断输出、drift 日志与 gate failure 消息。
/// - 元组第三项是 `TestScenario` trait object,可直接传给现有 CLI 列表/运行代码。
///
/// 顺序与 `main.rs` 旧版 `get_all_scenarios()` 保持一致;任何 reorder
/// 都需要同步刷新 `audit-p5-p1.md` 与本 change 的 `tasks.md` 行号映射。

pub fn all_scenarios() -> Vec<(ScenarioKind, &'static str, Box<dyn TestScenario>)> {
    vec![
        // Tier 1: Connectivity (backend-agnostic)
        // connectivity 已声明化(候选6)
        (
            ScenarioKind::Declarative,
            "connectivity",
            Box::new(crate::declarative::from_yaml(
                "connectivity",
                include_str!("../scenarios/connectivity.yaml"),
            )),
        ),
        // Tier 2: Orchestration Loop (backend-agnostic)
        // single-iter 已声明化(候选6 试点)
        (
            ScenarioKind::Declarative,
            "single-iter",
            Box::new(crate::declarative::from_yaml(
                "single-iter",
                include_str!("../scenarios/single-iter.yaml"),
            )),
        ),
        // multi-iter 已声明化(候选6)
        (
            ScenarioKind::Declarative,
            "multi-iter",
            Box::new(crate::declarative::from_yaml(
                "multi-iter",
                include_str!("../scenarios/multi-iter.yaml"),
            )),
        ),
        // completion 已声明化(候选6)
        (
            ScenarioKind::Declarative,
            "completion",
            Box::new(crate::declarative::from_yaml(
                "completion",
                include_str!("../scenarios/completion.yaml"),
            )),
        ),
        // Tier 3: Events (backend-agnostic)
        // events 已声明化(候选6)
        (
            ScenarioKind::Declarative,
            "events",
            Box::new(crate::declarative::from_yaml(
                "events",
                include_str!("../scenarios/events.yaml"),
            )),
        ),
        // backpressure 已声明化(候选6)
        (
            ScenarioKind::Declarative,
            "backpressure",
            Box::new(crate::declarative::from_yaml(
                "backpressure",
                include_str!("../scenarios/backpressure.yaml"),
            )),
        ),
        // Tier 4: Capabilities (backend-agnostic)
        (
            ScenarioKind::Declarative,
            "tool-use",
            Box::new(crate::declarative::from_yaml(
                "tool-use",
                include_str!("../scenarios/tool-use.yaml"),
            )),
        ),
        (
            ScenarioKind::Declarative,
            "streaming",
            Box::new(crate::declarative::from_yaml(
                "streaming",
                include_str!("../scenarios/streaming.yaml"),
            )),
        ),
        // Tier 5: Hat Collections (backend-agnostic)
        (
            ScenarioKind::Declarative,
            "hat-single",
            Box::new(crate::declarative::from_yaml(
                "hat-single",
                include_str!("../scenarios/hat-single.yaml"),
            )),
        ),
        (
            ScenarioKind::Declarative,
            "hat-multi-workflow",
            Box::new(crate::declarative::from_yaml(
                "hat-multi-workflow",
                include_str!("../scenarios/hat-multi-workflow.yaml"),
            )),
        ),
        (
            ScenarioKind::Declarative,
            "hat-instructions",
            Box::new(crate::declarative::from_yaml(
                "hat-instructions",
                include_str!("../scenarios/hat-instructions.yaml"),
            )),
        ),
        (
            ScenarioKind::Declarative,
            "hat-event-routing",
            Box::new(crate::declarative::from_yaml(
                "hat-event-routing",
                include_str!("../scenarios/hat-event-routing.yaml"),
            )),
        ),
        (
            ScenarioKind::Declarative,
            "hat-backend-override",
            Box::new(crate::declarative::from_yaml(
                "hat-backend-override",
                include_str!("../scenarios/hat-backend-override.yaml"),
            )),
        ),
        // Tier 6: Memory System (backend-agnostic)
        (
            ScenarioKind::Declarative,
            "memory-add",
            Box::new(crate::declarative::from_yaml(
                "memory-add",
                include_str!("../scenarios/memory-add.yaml"),
            )),
        ),
        (
            ScenarioKind::Declarative,
            "memory-search",
            Box::new(crate::declarative::from_yaml(
                "memory-search",
                include_str!("../scenarios/memory-search.yaml"),
            )),
        ),
        (
            ScenarioKind::Declarative,
            "memory-injection",
            Box::new(crate::declarative::from_yaml(
                "memory-injection",
                include_str!("../scenarios/memory-injection.yaml"),
            )),
        ),
        (
            ScenarioKind::Declarative,
            "memory-persist",
            Box::new(crate::declarative::from_yaml(
                "memory-persist",
                include_str!("../scenarios/memory-persistence.yaml"),
            )),
        ),
        // Tier 6: Memory System (Chaos Tests)
        (
            ScenarioKind::Declarative,
            "memory-corrupted",
            Box::new(crate::declarative::from_yaml(
                "memory-corrupted",
                include_str!("../scenarios/memory-corrupted-file.yaml"),
            )),
        ),
        (
            ScenarioKind::Declarative,
            "memory-missing",
            Box::new(crate::declarative::from_yaml(
                "memory-missing",
                include_str!("../scenarios/memory-missing-file.yaml"),
            )),
        ),
        (
            ScenarioKind::Declarative,
            "memory-rapid-write",
            Box::new(crate::declarative::from_yaml(
                "memory-rapid-write",
                include_str!("../scenarios/memory-rapid-write.yaml"),
            )),
        ),
        (
            ScenarioKind::Imperative,
            "memory-large-content",
            Box::new(MemoryLargeContentScenario::new()),
        ),
        // Tier 7: Error Handling (backend-agnostic)
        (
            ScenarioKind::Declarative,
            "timeout-handling",
            Box::new(crate::declarative::from_yaml(
                "timeout-handling",
                include_str!("../scenarios/timeout.yaml"),
            )),
        ),
        (
            ScenarioKind::Declarative,
            "max-iterations",
            Box::new(crate::declarative::from_yaml(
                "max-iterations",
                include_str!("../scenarios/max-iterations.yaml"),
            )),
        ),
        (
            ScenarioKind::Declarative,
            "auth-failure",
            Box::new(crate::declarative::from_yaml(
                "auth-failure",
                include_str!("../scenarios/auth-failure.yaml"),
            )),
        ),
        (
            ScenarioKind::Declarative,
            "backend-unavailable",
            Box::new(crate::declarative::from_yaml(
                "backend-unavailable",
                include_str!("../scenarios/backend-unavailable.yaml"),
            )),
        ),
        // Tier 8: Parallel Runtime (experimental)
        // parallel-hat-instances(en) 已声明化(候选6)
        (
            ScenarioKind::Declarative,
            "parallel-hat-instances",
            Box::new(crate::declarative::from_yaml(
                "parallel-hat-instances",
                include_str!("../scenarios/hat-instances.yaml"),
            )),
        ),
        // parallel-hat-instances-zh 已声明化(候选6)
        (
            ScenarioKind::Declarative,
            "parallel-hat-instances-zh",
            Box::new(crate::declarative::from_yaml(
                "parallel-hat-instances-zh",
                include_str!("../scenarios/hat-instances-zh.yaml"),
            )),
        ),
        // starting-event-inference 已声明化(候选6)
        (
            ScenarioKind::Declarative,
            "parallel-starting-event-inference",
            Box::new(crate::declarative::from_yaml(
                "parallel-starting-event-inference",
                include_str!("../scenarios/starting-event-inference.yaml"),
            )),
        ),
        // starting-event-inference-multi-candidate 已声明化(候选6)
        (
            ScenarioKind::Declarative,
            "parallel-starting-event-inference-multi-candidate",
            Box::new(crate::declarative::from_yaml(
                "parallel-starting-event-inference-multi-candidate",
                include_str!("../scenarios/starting-event-inference-multi-candidate.yaml"),
            )),
        ),
        // emit-spawn-instance 已声明化(候选6); {model} 占位符与命令式等价
        (
            ScenarioKind::Declarative,
            "parallel-emit-spawn-instance",
            Box::new(crate::declarative::from_yaml(
                "parallel-emit-spawn-instance",
                include_str!("../scenarios/emit-spawn-instance.yaml"),
            )),
        ),
        (
            ScenarioKind::Imperative,
            "parallel-app-server-idle-start",
            Box::new(ParallelAppServerIdleStartScenario::new()),
        ),
        // app-server-idle-start-live 已声明化(候选6 inject 试点)
        (
            ScenarioKind::Declarative,
            "parallel-app-server-idle-start-live",
            Box::new(crate::declarative::from_yaml(
                "parallel-app-server-idle-start-live",
                include_str!("../scenarios/app-server-idle-start-live.yaml"),
            )),
        ),
        (
            ScenarioKind::Imperative,
            "parallel-app-server-steer-multi-turn",
            Box::new(ParallelAppServerSteerMultiTurnScenario::new()),
        ),
        // steer-multi-turn-live 已声明化(候选6)
        (
            ScenarioKind::Declarative,
            "parallel-app-server-steer-multi-turn-live",
            Box::new(crate::declarative::from_yaml(
                "parallel-app-server-steer-multi-turn-live",
                include_str!("../scenarios/steer-multi-turn-live.yaml"),
            )),
        ),
        // steer-live-reply-multi-turn 已声明化(候选6)
        (
            ScenarioKind::Declarative,
            "parallel-app-server-steer-live-reply-multi-turn",
            Box::new(crate::declarative::from_yaml(
                "parallel-app-server-steer-live-reply-multi-turn",
                include_str!("../scenarios/steer-live-reply-multi-turn.yaml"),
            )),
        ),
        // trigger-routing-example 已声明化(候选6, example 引用试点)
        (
            ScenarioKind::Declarative,
            "parallel-trigger-routing-example",
            Box::new(crate::declarative::from_yaml(
                "parallel-trigger-routing-example",
                include_str!("../scenarios/parallel-trigger-routing-example.yaml"),
            )),
        ),
        // 以下 example 场景已声明化(候选6): example 引用 + 事件链/payload/gates 断言
        (
            ScenarioKind::Declarative,
            "parallel-pr-review-example",
            Box::new(crate::declarative::from_yaml(
                "parallel-pr-review-example",
                include_str!("../scenarios/pr-review-example.yaml"),
            )),
        ),
        (
            ScenarioKind::Declarative,
            "parallel-release-checklist-example",
            Box::new(crate::declarative::from_yaml(
                "parallel-release-checklist-example",
                include_str!("../scenarios/release-checklist-example.yaml"),
            )),
        ),
        (
            ScenarioKind::Declarative,
            "parallel-audit-evidence-pack-example",
            Box::new(crate::declarative::from_yaml(
                "parallel-audit-evidence-pack-example",
                include_str!("../scenarios/audit-evidence-pack-example.yaml"),
            )),
        ),
        (
            ScenarioKind::Declarative,
            "parallel-customer-advisory-board-prep-example",
            Box::new(crate::declarative::from_yaml(
                "parallel-customer-advisory-board-prep-example",
                include_str!("../scenarios/customer-advisory-board-prep-example.yaml"),
            )),
        ),
        (
            ScenarioKind::Declarative,
            "parallel-customer-onboarding-activation-example",
            Box::new(crate::declarative::from_yaml(
                "parallel-customer-onboarding-activation-example",
                include_str!("../scenarios/customer-onboarding-activation-example.yaml"),
            )),
        ),
        (
            ScenarioKind::Declarative,
            "parallel-customer-renewal-desk-example",
            Box::new(crate::declarative::from_yaml(
                "parallel-customer-renewal-desk-example",
                include_str!("../scenarios/customer-renewal-desk-example.yaml"),
            )),
        ),
        (
            ScenarioKind::Declarative,
            "parallel-executive-business-review-prep-example",
            Box::new(crate::declarative::from_yaml(
                "parallel-executive-business-review-prep-example",
                include_str!("../scenarios/executive-business-review-prep-example.yaml"),
            )),
        ),
        (
            ScenarioKind::Declarative,
            "parallel-field-enablement-rollout-example",
            Box::new(crate::declarative::from_yaml(
                "parallel-field-enablement-rollout-example",
                include_str!("../scenarios/field-enablement-rollout-example.yaml"),
            )),
        ),
        (
            ScenarioKind::Declarative,
            "parallel-finance-close-control-room-example",
            Box::new(crate::declarative::from_yaml(
                "parallel-finance-close-control-room-example",
                include_str!("../scenarios/finance-close-control-room-example.yaml"),
            )),
        ),
        (
            ScenarioKind::Declarative,
            "parallel-hiring-debrief-panel-example",
            Box::new(crate::declarative::from_yaml(
                "parallel-hiring-debrief-panel-example",
                include_str!("../scenarios/hiring-debrief-panel-example.yaml"),
            )),
        ),
        (
            ScenarioKind::Declarative,
            "parallel-incident-response-war-room-example",
            Box::new(crate::declarative::from_yaml(
                "parallel-incident-response-war-room-example",
                include_str!("../scenarios/incident-response-war-room-example.yaml"),
            )),
        ),
        (
            ScenarioKind::Declarative,
            "parallel-launch-readiness-command-example",
            Box::new(crate::declarative::from_yaml(
                "parallel-launch-readiness-command-example",
                include_str!("../scenarios/launch-readiness-command-example.yaml"),
            )),
        ),
        (
            ScenarioKind::Declarative,
            "parallel-migration-rehearsal-example",
            Box::new(crate::declarative::from_yaml(
                "parallel-migration-rehearsal-example",
                include_str!("../scenarios/migration-rehearsal-example.yaml"),
            )),
        ),
        (
            ScenarioKind::Declarative,
            "parallel-multi-region-pipeline-sync-example",
            Box::new(crate::declarative::from_yaml(
                "parallel-multi-region-pipeline-sync-example",
                include_str!("../scenarios/multi-region-pipeline-sync-example.yaml"),
            )),
        ),
        (
            ScenarioKind::Declarative,
            "parallel-partner-launch-coordination-example",
            Box::new(crate::declarative::from_yaml(
                "parallel-partner-launch-coordination-example",
                include_str!("../scenarios/partner-launch-coordination-example.yaml"),
            )),
        ),
        (
            ScenarioKind::Declarative,
            "parallel-postmortem-action-board-example",
            Box::new(crate::declarative::from_yaml(
                "parallel-postmortem-action-board-example",
                include_str!("../scenarios/postmortem-action-board-example.yaml"),
            )),
        ),
        (
            ScenarioKind::Declarative,
            "parallel-proposal-assembly-example",
            Box::new(crate::declarative::from_yaml(
                "parallel-proposal-assembly-example",
                include_str!("../scenarios/proposal-assembly-example.yaml"),
            )),
        ),
        (
            ScenarioKind::Declarative,
            "parallel-regional-operating-review-example",
            Box::new(crate::declarative::from_yaml(
                "parallel-regional-operating-review-example",
                include_str!("../scenarios/regional-operating-review-example.yaml"),
            )),
        ),
        (
            ScenarioKind::Declarative,
            "parallel-renewal-risk-calibration-example",
            Box::new(crate::declarative::from_yaml(
                "parallel-renewal-risk-calibration-example",
                include_str!("../scenarios/renewal-risk-calibration-example.yaml"),
            )),
        ),
        (
            ScenarioKind::Declarative,
            "parallel-revops-quote-desk-example",
            Box::new(crate::declarative::from_yaml(
                "parallel-revops-quote-desk-example",
                include_str!("../scenarios/revops-quote-desk-example.yaml"),
            )),
        ),
        (
            ScenarioKind::Declarative,
            "parallel-security-exception-review-example",
            Box::new(crate::declarative::from_yaml(
                "parallel-security-exception-review-example",
                include_str!("../scenarios/security-exception-review-example.yaml"),
            )),
        ),
        (
            ScenarioKind::Declarative,
            "parallel-support-escalation-desk-example",
            Box::new(crate::declarative::from_yaml(
                "parallel-support-escalation-desk-example",
                include_str!("../scenarios/support-escalation-desk-example.yaml"),
            )),
        ),
        (
            ScenarioKind::Declarative,
            "parallel-vendor-security-procurement-example",
            Box::new(crate::declarative::from_yaml(
                "parallel-vendor-security-procurement-example",
                include_str!("../scenarios/vendor-security-procurement-example.yaml"),
            )),
        ),
        // human-approval-gate 已声明化(候选6: wait_event 注入 + --json emit + 事件顺序断言)
        (
            ScenarioKind::Declarative,
            "parallel-human-approval-gate-example",
            Box::new(crate::declarative::from_yaml(
                "parallel-human-approval-gate-example",
                include_str!("../scenarios/human-approval-gate-example.yaml"),
            )),
        ),
        // experimental-dev-engine 保留命令式: 依赖复杂 git seed/commit 工作流, 不适合声明化。
        (
            ScenarioKind::ImperativeExplicitKeep,
            "parallel-experimental-dev-engine-example",
            Box::new(ParallelExperimentalDevEngineExampleScenario::new()),
        ),
    ]
}

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
