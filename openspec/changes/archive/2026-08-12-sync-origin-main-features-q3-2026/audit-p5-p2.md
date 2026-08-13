# Audit Report P2 — 2026-08-13

Author: agent session `omx-1786419140441-df5ql8`
Status: code + tests delivered (commit pending).

## TL;DR

`PromptExecutor` port contract tests are landed in
`crates/ralph-core/tests/prompt_executor_contract.rs` (250 lines).
The new integration test exercises `EventLoop::run` end-to-end via a
deterministic stub executor and proves four invariants that any future
cherry-pick of upstream loop work or any new backend adapter must
preserve.

| Test | Asserted invariant |
|------|--------------------|
| `test_executor_round_trip_with_event_loop_run` | `on_iteration_started` + `execute_prompt` + `RunHooks::before_execute` / `after_execute` form a single sandwich per iteration; the coordinating hat id passed to the executor is `"ralph"` (not the sub-hat the prompt is about); `after_execute` sees the same `PromptOutput` that `execute_prompt` produced |
| `test_canceled_propagates_to_interrupted_termination` | `PromptOutput { canceled: true, .. }` causes `EventLoop::run` to return `TerminationReason::Interrupted`, and the executor *is* called once for that iteration (no early bypass) |
| `test_timed_out_propagates_to_stopped_termination` | `PromptOutput { timed_out: true, .. }` causes `EventLoop::run` to return `TerminationReason::Stopped` |

All three tests pass in 0.52s on the local machine:

```
running 3 tests
test test_executor_round_trip_with_event_loop_run ... ok
test test_canceled_propagates_to_interrupted_termination ... ok
test test_timed_out_propagates_to_stopped_termination ... ok

test result: ok. 3 passed; 0 failed
```

All 645 lib tests in `ralph-core` still pass (`cargo test -p
ralph-core --lib`, 3.16s wall). No production-code changes were
needed; the contract was already correctly expressed by the trait,
the new tests just pin it down.

## Static evidence — files touched

```
A  crates/ralph-core/tests/prompt_executor_contract.rs   (250 lines)

# nothing else

$ cargo check -p ralph-core --tests
warning: function `busy_ralph_primary_explicit_target_is_redirected_to_secondary` is never used
   --> crates/ralph-core/src/parallel/supervisor/routing_tests.rs:3438:14
   = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: `ralph-core` (lib test) generated 1 warning
```

The dead-code warning is pre-existing and unrelated to this audit.

## Dynamic evidence — what each test exercises

### Stub executor

`RecordingExecutor` is the single shared stub:

- `next_response: Mutex<PromptOutput>` — the canned response for the
  single `execute_prompt` call the test expects (one iteration, since
  `event_loop.max_iterations = 1`).
- `calls: Mutex<Vec<RecordedCall>>` — append-only ledger of all calls,
  so the test can assert on the parameters EventLoop passed in.
- `current_iteration: Mutex<u32>` — set by `on_iteration_started`, read
  by `execute_prompt`, gives the test a hook to verify that the trait
  method runs *before* `execute_prompt` *with the same iteration
  index*.

`#[async_trait]` is reused from the trait definition
(`PromptExecutor` is declared `#[async_trait::async_trait]`); the
workspace's `async-trait` and `tokio` (with `full` feature → `macros`)
dependencies were already present in `ralph-core/Cargo.toml`, so no
new dependencies were introduced.

### RunHooks sharing

`RunHooks::before_execute` and `after_execute` are boxed `FnMut`
closures with a `'a` lifetime. Tests share counters across the
closure and assertions via `Arc<Mutex<u32>>` (which is owned,
`Send + Sync`, and `'static`), avoiding the borrow-checker trap of
holding a `&mut` local across an async call.

### `hat_id` semantics — why the first test asserts `"ralph"`

`EventLoop::run` invokes `executor.execute_prompt(&hat_id, ...)` with
the *coordinating* hat id (which is `"ralph"` in multi-hat mode),
not the active sub-hat (`display_hat`). The active sub-hat's identity
is communicated through the *prompt text*, not the `hat_id` parameter.

This is verified to be intentional in `event_loop/mod.rs` around the
`execute_prompt` call site (`executor.execute_prompt(&prompt,
interactive, &hat_id, ...)`), and recorded in the test as a contract
assertion so a future refactor that swaps the meaning of `hat_id`
will be caught immediately rather than silently.

## Verdict

`PromptExecutor` port contract is now mechanically enforced.

A future cherry-pick that wants to change the `&hat_id` argument,
move `on_iteration_started` ordering, or short-circuit cancel/timeout
paths will fail at least one of these three tests — which is exactly
the back-pressure the proposal asked for.

No need for a rewrite, no extra dep, no production-code change.

## Closing recommendation

- tasks.md 5.2 (P2) marks complete once the commit lands.
- Future additions to this file should follow the same shape: a
  stub executor + `EventLoop::run` + an explicit invariant assertion.
- If the port grows additional methods (e.g. an explicit
  `on_iteration_finished` or a streaming-output variant), the same
  pattern handles them.
