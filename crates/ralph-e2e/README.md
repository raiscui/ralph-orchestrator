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
- `connectivity` - Backend availability + auth + prompt/response round-trip (declarative YAML)

### Tier 2: Orchestration Loop
Full Ralph orchestration cycle validation.
- `single-iter` - Single iteration completes
- `multi-iter` - Multi-iteration event progression
- `completion` - `LOOP_COMPLETE` detection via completion promise

### Tier 3: Events
Event parsing and routing.
- `events` - `<event topic="...">` XML parsing
- `backpressure` - `build.done` backpressure evidence in agent output

### Tier 4: Capabilities
Backend feature validation.
- `tool-use` - Tool invocation + response handling
- `streaming` - NDJSON streaming output parsing

### Tier 5: Hat Collections
Hat-based workflow testing.
- `hat-single` - Single hat execution with correct persona
- `hat-multi-workflow` - Planner → Builder delegation chain
- `hat-instructions` - Hat `instructions` followed by agent
- `hat-event-routing` - Events route to correct subscribing hat
- `hat-backend-override` - Per-hat `backend_override` config

### Tier 6: Memory System
Persistent memory validation.
- `memory-add` - Memory creation via `ralph tools memory add`
- `memory-search` - Memory search via `ralph tools memory search`
- `memory-injection` - Auto-injected into agent prompts
- `memory-persistence` - Persists across `.agent/memories.md` runs
- `memory-corrupted` / `memory-missing` / `memory-rapid-write` / `memory-large-content` - Chaos variants (YAML)

### Tier 7: Error Handling
Graceful failure modes.
- `timeout-handling` - Idle timeout termination
- `max-iterations` - Termination at max iterations
- `auth-failure` - Invalid credentials / auth error handling
- `backend-unavailable` - Missing CLI backend handling

### Tier 8: Parallel Runtime (experimental)
Validates the **parallel hat instances** runtime against real backends.
- `parallel-hat-instances` - Fanout routing + multi-instance attributed output
- `parallel-starting-event-inference` - starting_event unset → ralph#1 infers workflow entry event
- `parallel-starting-event-inference-multi-candidate` - multiple entry candidates → ralph#1 picks the correct one for the required workflow
- `parallel-emit-spawn-instance` - `ralph emit --spawn-instance` creates dynamic instance + `spawn.done` ACK
- `parallel-app-server-steer-multi-turn` - Codex App Server turn/steer multi-round injection (fake codex shim)
- `parallel-app-server-steer-multi-turn-live` - Real codex app-server turn/steer multi-round injection, preserves RPC trace
- `parallel-app-server-steer-live-reply-multi-turn` - Real codex app-server turn/steer with visible `answer` output
- `parallel-trigger-routing-example` - Runs `examples/parallel-trigger-routing`
- `parallel-experimental-dev-engine-example` - Runs `examples/parallel-experimental-dev-engine` (the one `ImperativeExplicitKeep`)
- `parallel-pr-review-example` - 直接覆盖 `examples/parallel-pr-review`
- `parallel-release-checklist-example` - 直接覆盖 `examples/parallel-release-checklist`
- `parallel-human-approval-gate-example` - 直接覆盖 `examples/parallel-human-approval-gate`,并在运行中注入真实 `ralph emit` 批准事件
- `parallel-incident-response-war-room-example` - 直接覆盖 `examples/parallel-incident-response-war-room`
- `parallel-security-exception-review-example` - 直接覆盖 `examples/parallel-security-exception-review`
- `parallel-customer-renewal-desk-example` - 直接覆盖 `examples/parallel-customer-renewal-desk`
- `parallel-audit-evidence-pack-example` - 直接覆盖 `examples/parallel-audit-evidence-pack`
- `parallel-finance-close-control-room-example` - 直接覆盖 `examples/parallel-finance-close-control-room`
- `parallel-hiring-debrief-panel-example` - 直接覆盖 `examples/parallel-hiring-debrief-panel`
- `parallel-customer-onboarding-activation-example` - 直接覆盖 `examples/parallel-customer-onboarding-activation`
- `parallel-support-escalation-desk-example` - 直接覆盖 `examples/parallel-support-escalation-desk`
- `parallel-partner-launch-coordination-example` - 直接覆盖 `examples/parallel-partner-launch-coordination`
- `parallel-field-enablement-rollout-example` - 直接覆盖 `examples/parallel-field-enablement-rollout`
- `parallel-revops-quote-desk-example` - 直接覆盖 `examples/parallel-revops-quote-desk`
- `parallel-executive-business-review-prep-example` - 直接覆盖 `examples/parallel-executive-business-review-prep`
- `parallel-customer-advisory-board-prep-example` - 直接覆盖 `examples/parallel-customer-advisory-board-prep`
- `parallel-regional-operating-review-example` - 直接覆盖 `examples/parallel-regional-operating-review`
- `parallel-renewal-risk-calibration-example` - 直接覆盖 `examples/parallel-renewal-risk-calibration`
- `parallel-multi-region-pipeline-sync-example` - 直接覆盖 `examples/parallel-multi-region-pipeline-sync`
- `parallel-launch-readiness-command-example` - 直接覆盖 `examples/parallel-launch-readiness-command`
- `parallel-migration-rehearsal-example` - 直接覆盖 `examples/parallel-migration-rehearsal`
- `parallel-postmortem-action-board-example` - 直接覆盖 `examples/parallel-postmortem-action-board`
- `parallel-proposal-assembly-example` - 直接覆盖 `examples/parallel-proposal-assembly`
- `parallel-vendor-security-procurement-example` - 直接覆盖 `examples/parallel-vendor-security-procurement`

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
    TestRunner, WorkspaceManager, RunConfig, all_scenarios, TestScenario,
};

#[tokio::main]
async fn main() {
    let workspace = WorkspaceManager::new(".e2e-tests");
    // Pull every registered scenario from the lib (single source of truth).
    let scenarios: Vec<Box<dyn TestScenario>> = all_scenarios()
        .into_iter()
        .map(|(_kind, _id, scenario)| scenario)
        .collect();

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

**新场景请写 YAML, 不要再写 Rust `TestScenario` impl.** Wave 2 + Wave 3.4 (Q3 2026) 把 34 个
原 imperative scenarios 全部迁移并物理删除, gate 当前 100% PASS. 详细指南:

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

历史命令式 impl 保留 (已加 `#[deprecated(since = "2.3.0", ...)]`), **Wave 3.4 (2026-08-17) 已经把 22 个 deprecated struct 全部物理删除**——继续保留就是累积技术债。

### Adding New Scenarios (Legacy Imperative)

历史保留的 imperative 路径, 仅用于:
- 显式声明 `§2.5.0 explicit-keep` 的场景 (例如 `parallel-experimental-dev-engine-example`)
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

3. Register in `src/scenarios/mod.rs` (`mod xxx;` + 必要的 `pub use`)
   - 如果这是“直接覆盖 examples/ 下 runnable example”的场景,优先放在 `src/scenarios/` 顶层
   - 如果它是通用并行 helper / 非 example 场景,再放到 `src/scenarios/parallel/`
4. Register in `src/lib.rs::all_scenarios()` (`main.rs` 的 `get_all_scenarios()` 已经是
   `all_scenarios().into_iter().map(...).collect()` 的薄壳)

## Environment Variables

| Variable | Description |
|----------|-------------|
| `ANTHROPIC_API_KEY` | Required for Claude backend |
| `KIRO_API_KEY` | Required for Kiro backend |
| `OPENCODE_API_KEY` | Required for OpenCode backend |

## License

Same as parent ralph-orchestrator project.
