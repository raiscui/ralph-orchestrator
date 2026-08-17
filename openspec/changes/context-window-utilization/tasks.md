# Tasks — context-window-utilization

## 1. LoopState telemetry

- [ ] 1.1 Add fields: `peak_input_tokens: u64`, `last_input_tokens: Option<u64>`, `hat_peak_input_tokens: HashMap<HatId, u64>`
- [ ] 1.2 Default impl initializes all 3 to empty/zero
- [ ] 1.3 Add `record_iteration_tokens(hat, tokens)` method (no-op on `tokens == 0`)
- [ ] 1.4 Add 3 unit tests

## 2. PtyExecutor context_window

- [ ] 2.1 Add `context_window: u64` field to `PtyExecutionResult` (default 0)
- [ ] 2.2 Add `set_context_window(u64)` setter on `PtyExecutor`
- [ ] 2.3 Extract Claude session peak in `execute_*` (when Claude stream has `usage` event)
- [ ] 2.4 Add fixture `claude_stream_peak.jsonl` (locked shape for regression)
- [ ] 2.5 Add 1-2 unit tests

## 3. loop_runner wiring

- [ ] 3.1 In `run_loop_impl`, after `execute_*`, call `event_loop.record_iteration_tokens(&hat_id, outcome.context_tokens)`
- [ ] 3.2 At termination log, surface peak context token count

## 4. config.rs helper

- [ ] 4.1 Add `resolve_context_window(backend: Backend) -> u64` in `config.rs`
- [ ] 4.2 Read from `config.adapters.<backend>.context_window_tokens` if Some
- [ ] 4.3 Default 0 otherwise (suppresses suffix downstream)

## 5. Summary writer

- [ ] 5.1 In `status_text` (or equivalent), include peak context suffix when > 0
- [ ] 5.2 Format: `Completed: LOOP_COMPLETE (context peak: 42,000 tokens)`

## 6. Verification

- [ ] 6.1 `cargo test -p ralph-core --lib` (target: 670+ passed, +6 new)
- [ ] 6.2 `cargo test -p ralph-adapters --lib` (target: 130+ passed)
- [ ] 6.3 `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] 6.4 E2E regression: `cargo run -p ralph-e2e -- codex --filter events,backpressure` PASS
- [ ] 6.5 minimax live regression: `parallel-hat-instances` 4/4 PASS

## 7. Documentation sync

- [ ] 7.1 CONTEXT.md: add "Context telemetry" domain entry
- [ ] 7.2 docs/solutions/minimax-full-auto-compat/README.md: no change needed
