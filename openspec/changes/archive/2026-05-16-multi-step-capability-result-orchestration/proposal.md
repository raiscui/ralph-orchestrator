## Why

Ralph can already prove two smaller products seams:

- a parent run can trigger one isolated capability invocation and receive a parent-visible `capability.result`
- a parent run can explicitly turn one capability result into one human-visible `reply.human.message`

What is still missing is the next orchestration step: a parent run that uses earlier capability results to decide the next capability request, and only emits the final human-facing answer after multiple isolated capability steps have completed.

## What Changes

- Extend `capability-invocation` with a requirement for multi-step parent orchestration over multiple capability results.
- Define a minimal repo-native flow where one parent run emits at least two distinct `capability.request` events in sequence, each informed by prior `capability.result` context.
- Require the final human-facing answer to remain explicit through `reply.human.message` after the multi-step chain completes.
- Keep all capability executions isolated and keep parent topology unchanged.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `capability-invocation`: add a requirement for multi-step orchestration over multiple parent-visible capability results.

## Impact

- Expected touchpoints:
  - `crates/ralph-cli/tests/integration_live_capability.rs`
  - possibly small runtime or prompt-context fixes only if the new focused gate exposes a real gap
  - `openspec/specs/capability-invocation/spec.md`
- No new broker layer.
- No topology mutation.
- No automatic human reply synthesis.
