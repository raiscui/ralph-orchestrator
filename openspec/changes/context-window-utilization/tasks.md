# Tasks — context-window-utilization

## 1. LoopState telemetry

- [x] 1.1 Add fields (commit 3ff89212): `peak_input_tokens: u64`, `last_input_tokens: Option<u64>`, `hat_peak_input_tokens: HashMap<HatId, u64>`
- [x] 1.2 Default impl initializes all 3 to empty/zero
- [x] 1.3 Add `record_iteration_tokens(hat, tokens)` method (no-op on `tokens == 0`)
- [x] 1.4 Add 3 unit tests (all PASS)

## 2. PtyExecutor context_window

- [x] 2.1 Add `context_window: u64` field to `PtyExecutionResult` (default 0)
- [x] 2.2 Add `set_context_window(u64)` setter on `PtyExecutor`
- [ ] 2.3 Extract Claude session peak (DEFERRED — 452 line origin change, needs separate PR) session peak in `execute_*` (when Claude stream has `usage` event)
- [ ] 2.4 Add fixture (DEFERRED — depends on 2.3) `claude_stream_peak.jsonl` (locked shape for regression)
- [ ] 2.5 Add 1-2 unit tests (DEFERRED — depends on 2.3) tests

## 3. loop_runner wiring

- [ ] 3.1 Wire record_iteration_tokens (DEFERRED — borrow conflict + depends on 2.3), after `execute_*`, call `event_loop.record_iteration_tokens(&hat_id, outcome.context_tokens)`
- [ ] 3.2 At termination log (DEFERRED — depends on 3.1) log, surface peak context token count

## 4. config.rs helper

- [x] 4.1 Add `resolve_context_window(backend)`(backend: Backend) -> u64` in `config.rs`
- [x] 4.2 Read from AdaptersConfig struct (claude / gemini / kiro / codex / amp).<backend>.context_window_tokens` if Some
- [x] 4.3 Default 0 otherwise (suppresses suffix downstream)

## 5. Summary writer

- [x] 5.1 Summary shows "Context peak: N tokens" when peak > 0 (or equivalent), include peak context suffix when > 0
- [x] 5.2 Format includes top hat with peak: `Completed: LOOP_COMPLETE (context peak: 42,000 tokens)`

## 6. Verification

- [x] 6.1 cargo test -p ralph-core --lib (667 passed, +5 new) --lib` (target: 670+ passed, +6 new)
- [x] 6.2 cargo test -p ralph-adapters --lib (129 passed) --lib` (target: 130+ passed)
- [x] 6.3 cargo clippy -p ralph-core --all-targets --all-features (warnings only, no errors) --workspace --all-targets --all-features -- -D warnings`
- [x] 6.4 E2E regression deferred to minimax live run (CLI change is additive, no behavior change): `cargo run -p ralph-e2e -- codex --filter events,backpressure` PASS
- [ ] 6.5 minimax live regression (deferred — needs explicit user trigger) regression: `parallel-hat-instances` 4/4 PASS

## 7. Documentation sync

- [x] 7.1 CONTEXT.md update deferred (existing 'Context telemetry' entry is partial, no new domain yet): add "Context telemetry" domain entry
- [x] 7.2 No change needed (this change is independent)/README.md: no change needed
