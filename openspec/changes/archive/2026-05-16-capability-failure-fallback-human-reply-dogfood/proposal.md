## Why

Ralph can already do three adjacent things:

- return a structured parent-visible `capability.failed`
- let the parent coordinator emit later `capability.request` events
- keep the final human-facing answer explicit through `reply.human.message`

What is still missing is the product seam between them. We do not yet have a repo-native proof that a parent run can see `capability.failed`, choose a fallback capability step, and only then emit the final human-visible answer.

## What Changes

- Extend `capability-invocation` with a requirement for parent-side fallback orchestration after `capability.failed`.
- Add a focused live capability integration gate that proves this minimal flow:
  1. parent emits an invalid `capability.request`
  2. runtime returns parent-visible `capability.failed`
  3. parent emits a fallback valid `capability.request`
  4. runtime returns `capability.result`
  5. parent explicitly emits `reply.human.message`
- Keep the existing boundaries intact:
  - no topology mutation
  - no retry engine
  - no automatic synthesis from failure/result into human reply

## Impact

- Affected spec: `capability-invocation`
- Affected tests: `crates/ralph-cli/tests/integration_live_capability.rs`
- Runtime code impact: expected to be none or minimal; current goal is proof and guardrail, not a new failure subsystem
