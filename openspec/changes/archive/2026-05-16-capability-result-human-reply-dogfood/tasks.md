## 1. OpenSpec artifacts

- [x] 1.1 Create proposal for `capability-result-human-reply-dogfood`.
- [x] 1.2 Create design for capability-result to explicit human-reply closure.
- [x] 1.3 Add delta spec for `capability-invocation`.
- [x] 1.4 Create test plan.
- [x] 1.5 Validate the change with `openspec validate capability-result-human-reply-dogfood --type change`.

## 2. Implementation preparation after approval

- [x] 2.1 Confirm that the existing `integration_live_capability.rs` harness can surface `capability.result` on a second parent turn.
- [x] 2.2 Confirm the smallest script flow that emits `reply.human.message` only after seeing capability-result context.
- [x] 2.3 Classify possible failures as parent-workflow, display, or durability issues.

## 3. Contract tests to implement after approval

- [x] 3.1 Add a focused live capability integration gate for `capability.result` followed by explicit `reply.human.message`.
- [x] 3.2 Assert `.ralph/events.jsonl` preserves `capability.request`, `capability.result`, and `reply.human.message` separately.
- [x] 3.3 Assert record-session preserves human-facing reply publication evidence.
- [x] 3.4 Assert CLI output shows the final human-facing payload.
- [x] 3.5 Assert invocation inspect still works for the same invocation id.

## 4. Verification to run after implementation approval

- [x] 4.1 Run focused `ralph-cli` live capability integration tests.
- [x] 4.2 Run focused unit tests if runtime display/recording behavior changes.
- [x] 4.3 Run `cargo test -p ralph-core smoke_runner` if runtime routing or record-session behavior changes.
- [x] 4.4 Run `cargo test` before declaring implementation complete.
- [x] 4.5 Run `openspec validate capability-result-human-reply-dogfood --type change` after code changes.
