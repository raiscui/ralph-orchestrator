## 1. OpenSpec artifacts

- [x] 1.1 Create proposal for `multi-step-capability-result-orchestration`.
- [x] 1.2 Create design for multi-step parent orchestration over capability results.
- [x] 1.3 Add delta spec for `capability-invocation`.
- [x] 1.4 Create test plan.
- [x] 1.5 Validate the change with `openspec validate multi-step-capability-result-orchestration --type change`.

## 2. Implementation preparation after approval

- [x] 2.1 Confirm that parent prompts on later turns contain prior `capability.result` context.
- [x] 2.2 Confirm the smallest deterministic script flow for request A -> result A -> request B -> result B -> final reply.
- [x] 2.3 Classify possible failures as context propagation, orchestration, or durability/display issues.

## 3. Contract tests to implement after approval

- [x] 3.1 Add a focused live capability integration gate covering two sequential capability requests.
- [x] 3.2 Assert `.ralph/events.jsonl` preserves both capability requests and both capability results separately.
- [x] 3.3 Assert both invocation ids remain inspectable through `ralph tools capability inspect`.
- [x] 3.4 Assert the final human-facing payload is emitted only after the second result.
- [x] 3.5 Assert record-session preserves the final `reply.human.message` publication evidence.

## 4. Verification to run after implementation approval

- [x] 4.1 Run focused `ralph-cli` live capability integration tests.
- [x] 4.2 Run focused unit tests if runtime prompt-context or record behavior changes.
- [x] 4.3 Run `cargo test -p ralph-core smoke_runner` if runtime routing or record-session behavior changes.
- [x] 4.4 Run `cargo test` before declaring implementation complete.
- [x] 4.5 Run `openspec validate multi-step-capability-result-orchestration --type change` after code changes.
