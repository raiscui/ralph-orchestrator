## Why

Ralph already has the two runtime contracts that matter here, but they are currently proven in isolation:

- `reply.hat.message` already closes the internal requester-return loop and writes answer evidence.
- `reply.human.message` already exists as the explicit human-facing reply topic and display/record surface.

What is still missing is a small repo-native dogfood path that proves these two contracts can participate in the same runtime run without collapsing their boundary. We need a focused gate that shows an internal answer can flow back to a coordinator and then become a human-visible answer only through an explicit `reply.human.message` decision.

## What Changes

- Extend `request-reply-answer-evidence` with an explicit human-facing answer-return requirement.
- Define a minimal end-to-end runtime flow where:
  - an internal request is made,
  - a worker returns an internal answer via `reply.hat.message`,
  - the coordinator explicitly emits `reply.human.message`,
  - and the resulting human-visible answer is auditable through existing runtime artifacts.
- Add a repo-native dogfood gate plan for this explicit two-step answer path.
- Keep the existing boundary intact: internal answer return does not automatically synthesize human reply.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `request-reply-answer-evidence`: add a requirement for explicit human-facing answer return after internal answer-return delivery.

## Impact

- Expected code and test touchpoints after approval:
  - `crates/ralph-cli/tests/integration_answer_evidence.rs`
  - possibly small runtime/recording/display fixes only if the new gate exposes a real gap
  - `openspec/specs/request-reply-answer-evidence/spec.md`
- No new evidence store.
- No request broker.
- No live topology mutation.
- No automatic conversion from `reply.hat.message` to `reply.human.message`.
