# spec: context window utilization telemetry (port of origin #218)

## Goal
Operators running long Ralph sessions have no visibility into context window
utilization. When a session approaches the model's token limit, the run
silently degrades. This change adds per-iteration and session-peak token
tracking, surfaced in the completion summary.

## Source
- origin commit: `d631ef7e00c2926e81f6e11d32661aa2719d17ab feat(telemetry): track context window utilization` (#218)
- local baseline: `context_window_tokens: Option<u32>` in `config.rs` (ADR-0001 verified out, shipped before Group 3)

## Behavior

### LoopState telemetry
- `peak_input_tokens: u64` — session-scoped peak across all iterations
- `last_input_tokens: Option<u64>` — last iteration's token count
- `hat_peak_input_tokens: HashMap<HatId, u64>` — per-hat session peak
- `record_iteration_tokens(hat, tokens)`:
  - No-op when `tokens == 0` (suppresses ACP / non-token backends)
  - Updates per-hat peak (max of existing and new)
  - Updates global peak (max of all observed)
  - Sets `last_input_tokens = Some(tokens)`

### PtyExecutor context_window
- `PtyExecutionResult.context_window: u64` (default 0)
- `PtyExecutor::set_context_window(u64)` — sets the context limit for display
- During `execute_*`, if Claude stream emits `usage` events, extract the
  `input_tokens` peak and populate `context_window` field

### loop_runner wiring
After `execute_pty` / `execute_app_server` returns:
```rust
event_loop.record_iteration_tokens(&hat_id, outcome.context_tokens);
```
At termination log, include `peak_input_tokens` and the most demanding hat
in the human-readable termination summary.

### Config helper
- `resolve_context_window(backend: Backend) -> u64`
- Reads from `config.adapters.<backend>.context_window_tokens`
- Returns 0 if unset
- Caller wires into `PtyExecutor::set_context_window()` at startup

### Summary writer
Add context suffix to completion status:
```
Completed: LOOP_COMPLETE (context peak: 42,000 / 200,000 tokens)
```
Format includes the limit when known; otherwise just `peak tokens`.

## Architecture

### New module / module additions

```
crates/ralph-core/src/event_loop/loop_state.rs
  + peak_input_tokens: u64
  + last_input_tokens: Option<u64>
  + hat_peak_input_tokens: HashMap<HatId, u64>
  + record_iteration_tokens(hat, tokens)

crates/ralph-adapters/src/pty_executor.rs
  + PtyExecutionResult.context_window: u64
  + PtyExecutor::set_context_window(u64)
  ~ Claude stream JSONL parsing: extract usage.input_tokens peak

crates/ralph-cli/src/loop_runner.rs
  ~ After execute_*: record_iteration_tokens
  ~ Termination log: include peak context

crates/ralph-core/src/config.rs
  + pub fn resolve_context_window(backend: Backend) -> u64

crates/ralph-core/src/summary_writer.rs
  ~ status_text: include context peak suffix
```

## Acceptance criteria
1. LoopState fields persist session-wide, not per-iteration
2. `record_iteration_tokens(0)` is a no-op
3. `PtyExecutor` extracts peak tokens from Claude JSONL when present
4. `loop_runner` wires peak tokens into LoopState on every iteration
5. `resolve_context_window` returns configured value, 0 if unset
6. Summary includes context peak suffix when peak > 0
7. All existing tests (lazy-model-completion, human-guidance, hat-imports) still pass

## Risk
- Touches `PtyExecutor` (452 lines additive) and `loop_runner.rs` termination path
- Must not regress lazy-model-completion (commit `620411ce`) or human-guidance (commit `7de0d939`)
- Token extraction depends on Claude session JSONL fixture shape — add regression fixture

## Verification
- 6+ new unit tests
- `cargo test -p ralph-core --lib` (target: 670+ passed)
- `cargo test -p ralph-adapters --lib` (target: 130+ passed)
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- E2E regression: `cargo run -p ralph-e2e -- codex --filter events,backpressure` PASS
- minimax live regression: `parallel-hat-instances` 4/4 PASS
