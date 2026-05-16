# Test plan: capability-failure-class-branching-policy

## Goal

Prove that parent-visible capability failures expose a structured `failure_class`, and that the parent can branch on that class instead of relying on free-form error text.

## Assertions

- parent-visible `capability.failed` includes `failure_class`
- invalid capability id is classified as `invalid_capability_id`
- child execution failure artifact records include `child_run_failed`
- the live failure-fallback gate only proceeds after the parent prompt contains `invalid_capability_id`
- fallback success and final explicit human reply remain separately auditable

## Commands

```bash
openspec validate capability-failure-class-branching-policy --type change
cargo test -p ralph-core capability::tests -- --nocapture
cargo test -p ralph-cli capability::tests -- --nocapture
cargo test -p ralph-cli --test integration_live_capability parallel_parent_run_can_fallback_after_capability_failed_before_final_human_reply
cargo test -p ralph-cli --test integration_live_capability
cargo test -p ralph-core smoke_runner
openspec validate --all --strict
cargo test
```
