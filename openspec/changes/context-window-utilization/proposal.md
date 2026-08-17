# Proposal: Context Window Utilization Telemetry

## Why
Operators running long Ralph sessions have no visibility into context window
utilization. When a session approaches the model's token limit, the run
silently degrades (output truncation, refusal) without warning. This change
adds per-iteration and session-peak token tracking, surfaced in the
completion summary, so operators can detect and react to token pressure
before it bites.

## What changes

### LoopState telemetry (ralph-core/src/event_loop/loop_state.rs)
Add 3 fields:
- `peak_input_tokens: u64` — session-scoped peak across all iterations
- `last_input_tokens: Option<u64>` — last iteration's token count
- `hat_peak_input_tokens: HashMap<HatId, u64>` — per-hat session peak

Add method `record_iteration_tokens(hat, tokens)`:
- No-op when `tokens == 0` (suppresses non-token backends like ACP)
- Tracks per-hat peak and global peak
- Session-scoped peaks never reset on iteration boundaries

### PtyExecutionResult context_window (ralph-adapters/src/pty_executor.rs)
Add field `context_window: u64` (defaults to 0). `PtyExecutor::execute_with*`
extracts the peak token count from Claude stream JSONL output (when the
Claude session emits a `usage` event).

### loop_runner wiring (ralph-cli/src/loop_runner.rs)
After every `execute_pty` / `execute_app_server`, call:
```rust
event_loop.record_iteration_tokens(&hat_id, outcome.context_tokens);
```
This is in `run_loop_impl` and the per-hat execution branches.

### Config helper (ralph-core/src/config.rs)
Add `resolve_context_window(backend: Backend) -> u64`:
- Reads `context_window_tokens` from the active backend's adapter config
- Returns 0 if unset (no override)
- Caller wires this into `PtyExecutor::set_context_window()` at startup

### Summary writer (ralph-core/src/summary_writer.rs)
Include context peak in termination status:
```
Completed: LOOP_COMPLETE (context peak: 42,000 tokens)
```

### Frontend / TUI
Origin's 1-line frontend deletion is not needed locally (no TUI context
display). Skip per "改良胜过新增".

## Acceptance criteria
1. LoopState fields persist session-wide, not per-iteration
2. `record_iteration_tokens(0)` is a no-op (suppresses ACP / headless)
3. `PtyExecutor` extracts peak tokens from Claude JSONL when present
4. `loop_runner` wires peak tokens into LoopState on every iteration
5. `resolve_context_window` returns configured value, 0 if unset
6. Summary includes context peak suffix when peak > 0
8. All existing tests (lazy-model-completion, human-guidance, hat-imports)
   still pass

## Out of scope
- Frontend React context display (1 line in origin; local TUI does not need it)
- ACP / non-Codex backend token extraction (local job abstractions already stable)
- Real-time context warning during iteration (future change)

## Verification
- 6+ new unit tests
- Existing e2e suite (`events,backpressure,parallel-hat-instances`) still PASS
- minimax live: `parallel-hat-instances` 4/4 PASS (regression check)
- `cargo clippy --workspace -- -D warnings` clean

## Risk
- Touches `PtyExecutor` (452 lines additive) and `loop_runner.rs` termination path
- Must not regress lazy-model-completion (commit `620411ce`) or human-guidance (commit `7de0d939`)
- Token extraction depends on Claude session JSONL fixture shape — add regression test

## References
- Origin: `d631ef7 feat(telemetry): track context window utilization` (#218)
- Local baseline: `context_window_tokens: Option<u32>` in config.rs (already present)
- ADR-0001 verified out: context_window config field shipped before Group 3 work
