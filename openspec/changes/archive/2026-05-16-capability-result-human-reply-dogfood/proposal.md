## Why

Ralph can already run a live parent-triggered capability invocation and return a structured `capability.result` back to the parent run. It can also already prove that human-visible answers must be emitted explicitly through `reply.human.message`.

What is still missing is the product seam between those two contracts. We need a repo-native proof that a parent coordinator can receive `capability.result` from an isolated capability invocation and then explicitly turn that result into a human-visible answer without mutating topology or inventing a new reply channel.

## What Changes

- Extend `capability-invocation` with a requirement for explicit human-facing reply after parent-visible capability results.
- Define a minimal repo-native dogfood path where:
  - `ralph#1` emits `capability.request`
  - runtime returns `capability.result`
  - `ralph#1` explicitly emits `reply.human.message`
  - existing artifacts prove the full chain
- Keep the boundary intact: `capability.result` is parent-consumable runtime data, not an automatic human reply.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `capability-invocation`: add a requirement for explicit human-facing answer after parent-triggered capability result delivery.

## Impact

- Expected touchpoints:
  - `crates/ralph-cli/tests/integration_live_capability.rs`
  - possibly small runtime/recording/display fixes only if the new focused gate exposes a real gap
  - `openspec/specs/capability-invocation/spec.md`
- No new runtime topology mutation.
- No automatic synthesis from `capability.result` into `reply.human.message`.
- No new evidence store or broker layer.
