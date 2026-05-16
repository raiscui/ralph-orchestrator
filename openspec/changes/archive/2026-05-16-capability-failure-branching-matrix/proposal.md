## Why

B.3 made `capability.failed.failure_class` available as a structured parent policy input.

The next useful product step is to prove that parent-side policy can make different choices for different classes without turning the runtime into a generic retry engine. Otherwise, `failure_class` risks becoming a nicer label that the parent still treats as one undifferentiated failure.

## What Changes

- Define a minimal class-specific branching matrix:
  - `invalid_capability_id` may recover through an explicit fallback `capability.request`.
  - `malformed_request` should fail fast into an explicit diagnostic `reply.human.message` without retrying the malformed capability request.
- Add a focused live gate for the `malformed_request` diagnostic branch.
- Keep the existing B.3 fallback gate as the evidence for the `invalid_capability_id` branch.

## Impact

- Affected spec: `capability-invocation`
- Affected tests:
  - `crates/ralph-cli/tests/integration_live_capability.rs`
- Product effect: parent-side capability policy can branch by structured failure class while preserving explicit human reply semantics and static parent topology.
