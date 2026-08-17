---
status: pending
created: 2026-08-17
started: 2026-08-17
---
# Task: Context Window Utilization Telemetry (port of origin #218)

## Description
Implement per-session / per-hat context window token tracking and display.
Tracks peak and last input tokens across the loop, surfaces them in the summary
so operators can see when a long-running session is approaching context-window
limits.

## Background
Origin `d631ef7 feat(telemetry): track context window utilization` (#218) introduced:
- `LoopState` fields: `peak_input_tokens`, `last_input_tokens`, `hat_peak_input_tokens`
- `record_iteration_tokens(hat, tokens)` method
- `PtyExecutionResult.context_window` field (Claude token extraction)
- `resolve_context_window(backend)` helper in config.rs
- `loop_runner.rs` wires `outcome.context_tokens` into `record_iteration_tokens`
- `summary_writer.rs` surfaces context utilization in termination status
- New test fixture + ~5 unit tests

Local main already has `context_window_tokens: Option<u32>` in config.rs
(ADR-0001 verified out outcomes), so the config baseline is partially
present. The 452-line modification to `pty_executor.rs` is the bulk of
the work.

## Reference Documentation
**Required:**
- Origin commit: `d631ef7e00c2926e81f6e11d32661aa2719d17ab`
- Origin spec: `.ralph/specs/context-window-utilization.md` (433 lines) — not present locally, will port key requirements
- Origin task description: `.ralph/tasks/context-window-utilization.code-task.md`

**Architecture match (local vs origin):**
- Local `Config.context_window_tokens: Option<u32>` ✅ already present
- Local `PtyExecutor` exists, similar shape ✅ can add fields
- Local `claude_stream.rs` exists ✅
- Local `loop_runner.rs` exists ✅
- Missing in origin: `acp_executor.rs`, `json_rpc_handler.rs`, `pi_stream.rs`, `stream_handler.rs` — local has been refactored into job/{app_server,headless,mcp}.rs

## Scope

### In scope (this change)
1. `LoopState` add fields + `record_iteration_tokens` method
2. `PtyExecutionResult` add `context_window: u64` field
3. `PtyExecutor` extract Claude stream peak from SessionResult
4. `loop_runner.rs` call `event_loop.record_iteration_tokens(&hat_id, outcome.context_tokens)`
5. `config.rs` add `resolve_context_window(backend, overrides)` helper
6. `summary_writer.rs` surface context utilization in completion summary
7. `loop_runner.rs` log context peak at termination
8. 5+ unit tests + 1 fixture (`claude_stream_peak.jsonl`)

### Out of scope (separate changes)
- Frontend React changes (origin had a 1-line deletion; local TUI does not
  need it)
- ACP / non-Codex backend extraction (local already has stable job
  abstractions; tokens reported as 0 suppress the suffix)

## Verification

### Unit tests (target: +6 new, 670+ total)
- `record_iteration_tokens_tracks_per_hat_and_global_peak`
- `record_iteration_tokens_zero_tokens_is_noop`
- `record_iteration_tokens_per_hat_peak_independent_of_global`
- `pty_executor_extracts_peak_from_claude_session`
- `summary_writer_includes_context_peak_in_status`
- `config_resolve_context_window_prefers_explicit_override`

### Integration tests
- Existing `cargo run -p ralph-e2e -- codex --filter events,backpressure` continues to pass
- minimax live: `parallel-hat-instances` 4/4 still PASS (telemetry is additive, non-breaking)

### Build/lint
- `cargo check --workspace --quiet`
- `cargo clippy -p ralph-core --all-targets --all-features -- -D warnings`
- `cargo clippy -p ralph-cli --all-targets --all-features -- -D warnings`
- `cargo clippy -p ralph-adapters --all-targets --all-features -- -D warnings`

## Risk
- Touches `PtyExecutor` and `loop_runner.rs` termination path — must not
  regress lazy-model-completion (commit `620411ce`) or human-guidance
  (commit `7de0d939`).
- Token extraction from Claude session JSONL depends on the existing
  fixture format — add a regression test fixture to lock the shape.

## Migration
- New fields default to `0` / `None` / empty — no existing-hat break.
- Summary additions are additive; old `LOOP_COMPLETE` summaries remain
  identical except for the new context suffix.

## Decision (out of band)
- §18 is NOT in the Q3 plan Group 3/4 → moved to Group 4 §18 (per
  `group3-dryrun-log-2026-08-17.md`).
- Per Round 4 CONTEXT.md, this is a NEW OpenSpec change, not the Q3
  change.
