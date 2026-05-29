## Why

Some agent CLI jobs stop on failures that are usually temporary rather than terminal, such as `429 Too Many Requests` or `exceeded retry limit`. Today these failures are only visible as generic non-zero exits plus stderr text, so downstream runtime and human operators cannot reliably tell which failures are worth retrying, when to retry them, or whether a paused job is waiting for a manual continue.

## What Changes

- Add a single append-only recoverable-failure data file under `.ralph/` to record and aggregate recoverable agent CLI failures as machine-readable evidence.
- Classify a small, deterministic set of recoverable CLI failure kinds from structured job result fields and stderr excerpts, instead of treating every non-zero exit as the same kind of failure.
- Add configurable retry scheduling for recoverable failures, including delay, attempt count, and next-retry metadata.
- Allow a paused recoverable failure to be resumed by an explicit continue action from the human side, so a user can trigger retry without reconstructing the job manually.
- Keep terminal failures terminal; this change only introduces retry behavior for explicitly classified recoverable failures.

## Capabilities

### New Capabilities
- `agent-cli-recoverable-failure-retry`: records recoverable CLI failures, classifies them deterministically, persists retry state, and supports delayed or manual retry.

### Modified Capabilities
- `parallel-hat-instances`: job completion and failure handling need to emit/persist recoverable failure state and support retry-aware instance/job lifecycle transitions.
- `supervisor-human-chat-gate`: human-facing input needs a clear continue path for resuming a paused recoverable failure, without conflating it with ordinary chat.

## Impact

- Runtime job execution paths in `crates/ralph-adapters/src/cli_executor.rs`, `crates/ralph-cli/src/parallel_runner.rs`, and the parallel instance/supervisor flow in `crates/ralph-core/src/parallel/`.
- A new `.ralph/` sidecar data file for durable failure evidence and retry state.
- Human-facing retry control surface in the Supervisor/TUI flow.
- Tests for deterministic failure classification, retry scheduling, and manual continue handling.
