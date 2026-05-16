## 1. OpenSpec artifacts

- [x] 1.1 Create proposal for `human-facing-answer-return-dogfood`.
- [x] 1.2 Create design for the minimal explicit human-facing answer path.
- [x] 1.3 Add delta spec for `request-reply-answer-evidence`.
- [x] 1.4 Create test plan.
- [x] 1.5 Validate the change with `openspec validate human-facing-answer-return-dogfood --type change`.

## 2. Implementation preparation after approval

- [x] 2.1 Confirm the smallest custom-backend flow that produces internal answer-return followed by explicit human-facing reply.
- [x] 2.2 Decide whether the existing `integration_answer_evidence.rs` harness can absorb the new gate cleanly.
- [x] 2.3 Identify whether any failure would be display-only, durability-only, or real workflow behavior drift.

## 3. Contract tests to implement after approval

- [x] 3.1 Add a focused CLI integration gate for internal answer-return followed by explicit `reply.human.message`.
- [x] 3.2 Assert `.ralph/events.jsonl` preserves both internal and human-facing reply events separately.
- [x] 3.3 Assert record-session preserves human-facing reply publication evidence.
- [x] 3.4 Assert the final human-facing payload is visible on the CLI output surface.
- [x] 3.5 Keep the existing guardrail that internal `reply.hat.message` alone does not synthesize `reply.human.message`.

## 4. Verification to run after implementation approval

- [x] 4.1 Run focused `ralph-cli` integration tests for answer evidence.
- [x] 4.2 Run any newly added focused unit tests if runtime display or recording code changes.
- [x] 4.3 Run `cargo test -p ralph-core smoke_runner` if runtime routing or recording behavior changes.
- [x] 4.4 Run `cargo test` before declaring implementation complete.
- [x] 4.5 Run `openspec validate human-facing-answer-return-dogfood --type change` after code changes.
