# Testing & Validation

Comprehensive testing approaches for Ralph development and validation.

## Test Types

| Type | Purpose | Speed | Cost |
|------|---------|-------|------|
| Unit Tests | Test individual functions | Fast | Free |
| Smoke Tests | Replay recorded sessions | Fast | Free |
| E2E Tests | Validate against real backends | Slow | API costs |
| TUI Validation | Verify terminal rendering | Medium | Free |

## Running Tests

### All Tests

```bash
cargo test
```

This includes unit tests and smoke tests (344+ tests total).

### Smoke Tests Only

```bash
cargo test -p ralph-core smoke_runner
```

### Kiro-Specific Tests

```bash
cargo test -p ralph-core kiro
```

### E2E Tests

```bash
# All backends
cargo run -p ralph-e2e -- all

# Specific backend
cargo run -p ralph-e2e -- claude

# List scenarios
cargo run -p ralph-e2e -- --list
```

## Smoke Tests

Smoke tests use recorded JSONL fixtures instead of live API calls — fast, free, deterministic.

### How They Work

1. Record a session to JSONL
2. Replay during tests
3. Verify expected behavior

### Fixture Locations

```
crates/ralph-core/tests/fixtures/
├── basic_session.jsonl          # Claude CLI session
└── kiro/                         # Kiro sessions
    ├── basic.jsonl
    ├── tool_use.jsonl
    └── autonomous.jsonl
```

### Recording New Fixtures

```bash
# Record a session
ralph run -c ralph.yml --record-session session.jsonl -p "your prompt"

# Or capture raw CLI output
claude -p "your prompt" 2>&1 | tee output.txt
```

### Inspecting Recordings

After recording, you can quickly sanity-check the JSONL:

```bash
# Human-readable summary (strict parse)
ralph record summary session.jsonl
```

If you're recording a long session and want to confirm the file is still growing:

```bash
# Raw follow (tail -f style)
ralph record watch session.jsonl
```

If the session was started with `--record-session`, you can also omit `FILE` and let Ralph resolve it
via `.ralph/record-session.latest` from the workspace root:

```bash
ralph record watch
```

### Agent Workflow: Real-time validation with `record watch`

`ralph record watch` is designed as a low-abstraction, real-time tool for coding agents.
It helps distinguish "semantic bugs" from "display bugs", and makes progress visible while a run is still active.

Recommended workflow (two terminals):

1. **Terminal A**: run Ralph with recording enabled.

   ```bash
   ralph run --record-session /tmp/session.jsonl
   ```

2. **Terminal B**: follow the record file and watch evidence arrive line-by-line.

   ```bash
   ralph record watch /tmp/session.jsonl --from-start
   ```

Optional (agent automation probe):

```bash
# Exit 0 when the reply evidence arrives, exit 2 on timeout.
ralph record watch /tmp/session.jsonl --until-topic reply.human.message --timeout-secs 30 --quiet
```

What to look for:

- Early evidence: `_meta.session_start`, `_meta.loop_start`
- Live activity: `ux.terminal.write` and `bus.publish` lines continue to append
- User-facing reply: `reply.human.message` shows up in `bus.publish` and/or stdout tail records

When the UI "looks blank":

- If `record watch` shows `reply.human.message` or stdout contains `<event topic="reply.human.message">...`,
  it is very likely a **rendering/display** issue (not a routing/LLM issue).
- If `record watch` does not show reply evidence, it is more likely a **semantic/durability** issue:
  event routing, flush behavior, interrupt handling, or backend output.

As a final sanity check after the run ends:

```bash
ralph record summary /tmp/session.jsonl
```

### Fixture Format

JSONL with one event per line:

```json
{"type":"output","content":"Starting task...","timestamp":"2024-01-21T10:00:00Z"}
{"type":"tool_call","tool":"read_file","args":{"path":"src/lib.rs"}}
{"type":"tool_result","result":"...contents..."}
{"type":"output","content":"LOOP_COMPLETE"}
```

## E2E Tests

End-to-end tests validate against real AI backends.

### Why E2E can miss record-session durability issues

Most E2E scenarios are run-to-completion and don't frequently exercise Ctrl+C/SIGINT or SIGTERM.
But record-session durability problems often only show up when you interrupt a run and immediately
inspect the JSONL (e.g., buffered writes not flushed, or the process exits before writing termination).

To guard this class of issues, prefer dedicated contract tests that:

1. Start `ralph run --idle-start --no-tui --record-session ...`
2. Send SIGINT/SIGTERM
3. Assert the JSONL is line-by-line parseable and contains:
   - `_meta.session_start`
   - `_meta.loop_start`
   - `_meta.termination` with `reason=Interrupted`

You can run these contract tests with:

```bash
cargo test -p ralph-cli --test integration_record_session
```

### Test Tiers

| Tier | Focus | Scenarios |
|------|-------|-----------|
| 1 | Connectivity | Backend availability, auth |
| 2 | Orchestration | Single/multi iteration |
| 3 | Events | Parsing, routing |
| 4 | Capabilities | Tool use, streaming |
| 5 | Hat Collections | Workflows, routing |
| 6 | Memory | Add, search, inject |
| 7 | Error Handling | Timeout, limits |
| 8 | Parallel Runtime | Parallel hat instances (experimental) |

### Running E2E Tests

```bash
# All tests for Claude
cargo run -p ralph-e2e -- claude

# All available backends
cargo run -p ralph-e2e -- all

# Fast mode (skip analysis)
cargo run -p ralph-e2e -- claude --skip-analysis

# Debug mode (keep workspaces)
cargo run -p ralph-e2e -- claude --keep-workspace --verbose
```

### Parallel Hat Instances (experimental)

并行模式的 E2E 场景会启动多个 **headless** 后端进程（多实例并发）。
因此它更接近真实用户环境，也更容易暴露认证/限流/网络问题。

```bash
# 只跑并行实例场景（推荐配合 keep-workspace 方便排障）
cargo run -p ralph-e2e -- codex --filter parallel-hat-instances --keep-workspace --verbose
```

排障建议：
- 先看 `.e2e-tests/` 下对应 workspace 的 `.ralph/diagnostics/` 目录（E2E 默认开启 `RALPH_DIAGNOSTICS=1`）
- 再看 `.e2e-tests/` 下对应 workspace 的 `.ralph/events-*.jsonl`（确认事件路由与 `<event ...>` 解析是否正常）

### E2E Reports

Generated in `.e2e-tests/`:

```
.e2e-tests/
├── report.md      # Human-readable Markdown
├── report.json    # Machine-readable JSON
└── claude-connect/  # Test workspace (with --keep-workspace)
```

### E2E Orchestration

For E2E test development, use isolated config:

```bash
# E2E test development
ralph run -c ralph.e2e.yml -p "fix e2e tests"
```

This uses separate scratchpad to avoid pollution.

## TUI Validation

Validate Terminal UI rendering using LLM-as-judge.

### Quick Start

```bash
# Validate from captured output
/tui-validate file:output.txt criteria:ralph-header

# Validate live TUI via tmux
/tui-validate tmux:ralph-session criteria:ralph-full

# Custom criteria
/tui-validate command:"cargo run --example tui" criteria:"Shows header"
```

### Built-in Criteria

| Criteria | Validates |
|----------|-----------|
| `ralph-header` | Iteration count, elapsed time, hat display |
| `ralph-footer` | Activity indicator, event topic |
| `ralph-full` | Complete layout and hierarchy |
| `tui-basic` | Has content, no artifacts |

### Live TUI Capture

```bash
# 1. Start TUI in tmux
tmux new-session -d -s ralph-test -x 100 -y 30
tmux send-keys -t ralph-test "ralph run -p 'test'" Enter

# 2. Wait for render
sleep 3

# 3. Capture
tmux capture-pane -t ralph-test -p -e > tui-capture.txt

# 4. Validate
/tui-validate file:tui-capture.txt criteria:ralph-header
```

### Prerequisites

```bash
# Screenshot tool (Freeze)
brew install charmbracelet/tap/freeze

# 如果你的网络/代理环境导致 brew tap 失败，可用 Go 安装：
# GOPROXY=https://goproxy.cn,direct GOSUMDB=sum.golang.google.cn \
#   go install github.com/charmbracelet/freeze@latest

# Live capture
brew install tmux
```

## Linting

```bash
# Check formatting
cargo fmt --check

# Run clippy
cargo clippy --all-targets --all-features
```

## Pre-commit Hooks

Install hooks:

```bash
./scripts/setup-hooks.sh
```

Hooks run `cargo fmt --check` and `cargo clippy` before each commit.

## Testing Best Practices

### 1. Run Tests After Changes

```bash
cargo test  # Always run before declaring done
```

### 2. Prefer Smoke Tests

For new features, create replay fixtures rather than relying on live APIs.

### 3. Use E2E for Integration

E2E tests are expensive but catch integration issues.

### 4. Validate TUI Changes

After modifying `ralph-tui`, use TUI validation.

### 5. Keep Fixtures Updated

When behavior changes, update corresponding fixtures.

## Creating New Tests

### Unit Test

```rust
#[test]
fn test_event_parsing() {
    let input = r#"ralph emit "build.done" "tests pass""#;
    let event = parse_event(input).unwrap();
    assert_eq!(event.topic, "build.done");
}
```

### Smoke Test

1. Record session: `--record-session fixture.jsonl`
2. Place in `tests/fixtures/`
3. Add test case referencing fixture

### E2E Scenario

```rust
pub struct MyScenario;

impl E2EScenario for MyScenario {
    fn name(&self) -> &str { "my-scenario" }
    fn tier(&self) -> u8 { 3 }

    async fn run(&self, ctx: &E2EContext) -> E2EResult {
        // Test implementation
    }
}
```

## Next Steps

- Explore [Diagnostics](diagnostics.md) for debugging
- Learn about [Architecture](architecture.md)
- See the [Contributing Guide](../contributing/index.md)
