# ralph-e2e

End-to-end test harness for the Ralph Orchestrator. Validates Ralph's behavior against real AI backends (Claude, Kiro, OpenCode) to ensure the orchestration loop works correctly.

## Quick Start

```bash
# Run all tests for all available backends
cargo run -p ralph-e2e -- all

# Run tests for a specific backend
cargo run -p ralph-e2e -- claude

# List available scenarios
cargo run -p ralph-e2e -- --list

# Run only scenarios matching a filter (case-insensitive substring match)
cargo run -p ralph-e2e -- codex --filter parallel-starting-event-inference-multi-candidate

# Run with detailed output
cargo run -p ralph-e2e -- claude --verbose

# Keep workspaces for debugging
cargo run -p ralph-e2e -- claude --keep-workspace

# Skip meta-Ralph analysis for faster runs
cargo run -p ralph-e2e -- claude --skip-analysis
```

## Architecture

```text
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  TestRunner │────▶│  Scenarios  │────▶│  Executor   │
└─────────────┘     └─────────────┘     └─────────────┘
       │                                       │
       ▼                                       ▼
┌─────────────┐                         ┌─────────────┐
│  Reporter   │                         │   Backend   │
└─────────────┘                         └─────────────┘
```

### Components

| Component | Description |
|-----------|-------------|
| **TestRunner** | Orchestrates scenario execution and collects results |
| **Scenarios** | Define test cases via the `TestScenario` trait |
| **Executor** | Spawns `ralph run` processes and captures output |
| **Reporter** | Generates terminal, Markdown, and JSON reports |
| **Analyzer** | Uses meta-Ralph for rich failure diagnosis |
| **WorkspaceManager** | Isolates tests in `.e2e-tests/` directories |

## Test Scenarios

Scenarios are organized into 8 tiers:

### Tier 1: Connectivity
Basic backend availability tests.
- `ClaudeConnectScenario` - Claude backend connectivity
- `KiroConnectScenario` - Kiro backend connectivity
- `OpenCodeConnectScenario` - OpenCode backend connectivity

### Tier 2: Orchestration Loop
Full Ralph orchestration cycle validation.
- `ClaudeSingleIterScenario` - Single iteration completion
- `ClaudeMultiIterScenario` - Multi-iteration progression
- `ClaudeCompletionScenario` - `LOOP_COMPLETE` detection

### Tier 3: Events
Event parsing and routing.
- `ClaudeEventsScenario` - Event XML parsing
- `ClaudeBackpressureScenario` - `build.done` backpressure evidence

### Tier 4: Capabilities
Backend feature validation.
- `ClaudeToolUseScenario` - Tool invocation handling
- `ClaudeStreamingScenario` - NDJSON streaming output

### Tier 5: Hat Collections
Hat-based workflow testing.
- `HatSingleScenario` - Single hat execution
- `HatMultiWorkflowScenario` - Planner → Builder delegation
- `HatInstructionsScenario` - Hat instructions followed
- `HatEventRoutingScenario` - Events route to correct hat
- `HatBackendOverrideScenario` - Per-hat backend selection

### Tier 6: Memory System
Persistent memory validation.
- `MemoryAddScenario` - Memory creation via CLI
- `MemorySearchScenario` - Memory search functionality
- `MemoryInjectionScenario` - Auto-injection in prompts
- `MemoryPersistenceScenario` - Cross-run persistence

### Tier 7: Error Handling (RED phase)
Graceful failure modes.
- `TimeoutScenario` - Timeout termination
- `MaxIterationsScenario` - Max iterations limit
- `AuthFailureScenario` - Invalid credentials handling
- `BackendUnavailableScenario` - Missing CLI handling

### Tier 8: Parallel Runtime (experimental)
Validates the **parallel hat instances** runtime against real backends.
- `ParallelHatInstancesScenario` - Fanout routing + multi-instance attributed output
- `ParallelStartingEventInferenceScenario` - starting_event unset → ralph#1 infers workflow entry event
- `ParallelStartingEventInferenceScenario` (multi-candidate variant) - multiple entry candidates → ralph#1 chooses correct entry for required workflow
- `ParallelEmitSpawnInstanceScenario` - ralph emit --spawn-instance creates dynamic instance + ACK
- `ParallelAppServerSteerMultiTurnScenario` - 覆盖 Codex App Server 的 turn/steer 多轮注入场景,当前仍在推进
- `ParallelAppServerSteerMultiTurnLiveScenario` - 覆盖真实 codex app-server 的 turn/steer 多轮注入,并保留客户端 RPC trace,当前仍在推进
- `ParallelAppServerSteerLiveReplyMultiTurnScenario` - 覆盖真实 codex app-server 的 turn/steer 多轮注入,并要求看到可见回复输出
- `ParallelTriggerRoutingExampleScenario` - 直接覆盖 `examples/parallel-trigger-routing`
- `ParallelExperimentalDevEngineExampleScenario` - 直接覆盖 `examples/parallel-experimental-dev-engine`
- `ParallelPrReviewExampleScenario` - 直接覆盖 `examples/parallel-pr-review`
- `ParallelReleaseChecklistExampleScenario` - 直接覆盖 `examples/parallel-release-checklist`
- `ParallelHumanApprovalGateExampleScenario` - 直接覆盖 `examples/parallel-human-approval-gate`,并在运行中注入真实 `ralph emit` 批准事件
- `ParallelIncidentResponseWarRoomExampleScenario` - 直接覆盖 `examples/parallel-incident-response-war-room`
- `ParallelSecurityExceptionReviewExampleScenario` - 直接覆盖 `examples/parallel-security-exception-review`
- `ParallelCustomerRenewalDeskExampleScenario` - 直接覆盖 `examples/parallel-customer-renewal-desk`
- `ParallelAuditEvidencePackExampleScenario` - 直接覆盖 `examples/parallel-audit-evidence-pack`
- `ParallelFinanceCloseControlRoomExampleScenario` - 直接覆盖 `examples/parallel-finance-close-control-room`
- `ParallelHiringDebriefPanelExampleScenario` - 直接覆盖 `examples/parallel-hiring-debrief-panel`
- `ParallelCustomerOnboardingActivationExampleScenario` - 直接覆盖 `examples/parallel-customer-onboarding-activation`
- `ParallelSupportEscalationDeskExampleScenario` - 直接覆盖 `examples/parallel-support-escalation-desk`
- `ParallelPartnerLaunchCoordinationExampleScenario` - 直接覆盖 `examples/parallel-partner-launch-coordination`
- `ParallelFieldEnablementRolloutExampleScenario` - 直接覆盖 `examples/parallel-field-enablement-rollout`
- `ParallelRevopsQuoteDeskExampleScenario` - 直接覆盖 `examples/parallel-revops-quote-desk`
- `ParallelExecutiveBusinessReviewPrepExampleScenario` - 直接覆盖 `examples/parallel-executive-business-review-prep`
- `ParallelCustomerAdvisoryBoardPrepExampleScenario` - 直接覆盖 `examples/parallel-customer-advisory-board-prep`
- `ParallelRegionalOperatingReviewExampleScenario` - 直接覆盖 `examples/parallel-regional-operating-review`
- `ParallelRenewalRiskCalibrationExampleScenario` - 直接覆盖 `examples/parallel-renewal-risk-calibration`
- `ParallelMultiRegionPipelineSyncExampleScenario` - 直接覆盖 `examples/parallel-multi-region-pipeline-sync`
- `ParallelLaunchReadinessCommandExampleScenario` - 直接覆盖 `examples/parallel-launch-readiness-command`
- `ParallelMigrationRehearsalExampleScenario` - 直接覆盖 `examples/parallel-migration-rehearsal`
- `ParallelPostmortemActionBoardExampleScenario` - 直接覆盖 `examples/parallel-postmortem-action-board`
- `ParallelProposalAssemblyExampleScenario` - 直接覆盖 `examples/parallel-proposal-assembly`
- `ParallelVendorSecurityProcurementExampleScenario` - 直接覆盖 `examples/parallel-vendor-security-procurement`

## Reports

Reports are generated in `.e2e-tests/`:

```bash
.e2e-tests/
├── report.md      # Agent-readable Markdown report
├── report.json    # Machine-readable JSON report
└── claude-connect/  # Test workspace (if --keep-workspace)
    ├── ralph.yml
    ├── prompt.md
    └── .agent/
```

### Report Formats

```bash
# Markdown + JSON snapshot (default)
cargo run -p ralph-e2e -- --report markdown

# JSON only
cargo run -p ralph-e2e -- --report json

# Both formats
cargo run -p ralph-e2e -- --report both
```

## Library Usage

The crate can be used as a library for programmatic testing:

```rust
use ralph_e2e::{
    TestRunner, WorkspaceManager, RunConfig,
    ClaudeConnectScenario, TestScenario,
};

#[tokio::main]
async fn main() {
    let workspace = WorkspaceManager::new(".e2e-tests");
    let scenarios: Vec<Box<dyn TestScenario>> = vec![
        Box::new(ClaudeConnectScenario::new()),
    ];

    let runner = TestRunner::new(workspace, scenarios);
    let config = RunConfig::new();
    let results = runner.run(&config).await.unwrap();

    println!("Passed: {}", results.passed_count());
}
```

## Development

```bash
# Run unit tests
cargo test -p ralph-e2e

# Run clippy
cargo clippy -p ralph-e2e

# Generate docs
cargo doc -p ralph-e2e --open
```

### Adding New Scenarios — Declarative First

**新场景请写 YAML, 不要再写 Rust `TestScenario` impl.** Wave 2 (Q3 2026) 把 21 个
imperative scenarios 全部迁移为 declarative YAML (Coverage 65%→100%). 详细指南:

👉 **[`docs/e2e/declarative-migration.md`](docs/e2e/declarative-migration.md)** — schema 字段速查 + 4 个常见陷阱 + 验证 checklist。

快速步骤:

1. `crates/ralph-e2e/scenarios/<your-scenario>.yaml` (用仓库 60+ 已有 YAML 之一作模板)
2. 在 `crates/ralph-e2e/src/lib.rs::all_scenarios()` 的 `ScenarioKind::Declarative` 块中加 entry:
   ```rust
   (
       ScenarioKind::Declarative,
       "<id>",
       Box::new(crate::declarative::from_yaml(
           "<id>",
           include_str!("../scenarios/<your-scenario>.yaml"),
       )),
   ),
   ```
3. 跑 4 个验证: `cargo check` + `cargo test --lib` + `cargo run -p ralph-e2e -- --list` (YAML 反序列化, 必跑) + `cargo test --test declarative_coverage_gate`。

历史命令式 impl 保留 (已加 `#[deprecated(since = "2.3.0", ...)]`), **1 release cycle 后物理删除**。

### Adding New Scenarios (Legacy Imperative)

历史保留的 imperative 路径, 仅用于:
- 显式声明 `§2.5.0 explicit-keep` 的场景 (例如 `ParallelExperimentalDevEngineExampleScenario`)
- 复杂时序 / 自定义检查, declarative schema 当前无法表达的极少数场景 (需先开 schema 扩展 RFC)

如确需新增 imperative, 步骤:
1. Create a new file in `src/scenarios/` (e.g., `my_scenario.rs`)
2. Implement the `TestScenario` trait:

```rust
use crate::scenarios::{TestScenario, ScenarioError, Assertions};
use crate::{Backend, ScenarioConfig, ExecutionResult, TestResult};

pub struct MyScenario;

impl TestScenario for MyScenario {
    fn id(&self) -> &str { "my-scenario" }
    fn description(&self) -> &str { "Tests something important" }
    fn tier(&self) -> &str { "Tier N: Category" }
    fn backend(&self) -> Backend { Backend::Claude }

    fn setup(&self, workspace: &Path) -> Result<ScenarioConfig, ScenarioError> {
        // Create ralph.yml and prompt
    }

    fn assertions(&self, result: &ExecutionResult) -> Vec<TestResult> {
        // Validate execution results
    }
}
```

3. Register in `src/scenarios/mod.rs` and `src/lib.rs`
   - 如果这是“直接覆盖 examples/ 下 runnable example”的场景,优先放在 `src/scenarios/` 顶层
   - 如果它是通用并行 helper / 非 example 场景,再放到 `src/scenarios/parallel/`
4. Add to `get_all_scenarios()` in `src/main.rs`

## Environment Variables

| Variable | Description |
|----------|-------------|
| `ANTHROPIC_API_KEY` | Required for Claude backend |
| `KIRO_API_KEY` | Required for Kiro backend |
| `OPENCODE_API_KEY` | Required for OpenCode backend |

## License

Same as parent ralph-orchestrator project.
