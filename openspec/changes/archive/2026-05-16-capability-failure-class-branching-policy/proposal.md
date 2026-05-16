## Why

Ralph can already return structured `capability.failed` events and it can already prove one concrete fallback flow after failure.

What is still missing is a stable structured input for parent branching policy. Right now, a parent coordinator could still be forced to infer failure meaning from free-form `error` text, which is brittle and not a good source of truth for product behavior.

## What Changes

- Extend `capability-invocation` so parent-visible `capability.failed` includes a structured `failure_class`.
- Define `failure_class` as the preferred parent policy input instead of parsing free-form error strings.
- Strengthen the existing failure-fallback dogfood gate so the parent explicitly branches after seeing `failure_class=invalid_capability_id`.

## Impact

- Affected spec: `capability-invocation`
- Affected code:
  - `crates/ralph-core/src/capability.rs`
  - `crates/ralph-core/src/parallel/supervisor/capability_runtime.rs`
  - `crates/ralph-cli/src/capability.rs`
  - `crates/ralph-cli/tests/integration_live_capability.rs`
- Product effect: parent-side branching can key off structured failure classes instead of error text heuristics
