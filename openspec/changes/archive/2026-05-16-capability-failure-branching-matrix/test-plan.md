# Test plan: capability-failure-branching-matrix

## Goal

Prove that parent policy can choose different actions for different structured capability failure classes without introducing a generic retry engine.

## Assertions

- Existing invalid capability id branch still recovers through explicit fallback capability request.
- New malformed request branch produces parent-visible `capability.failed` with `failure_class=malformed_request`.
- Parent sees `malformed_request` in a later turn before emitting diagnostic reply.
- Diagnostic reply is explicit `reply.human.message`.
- Malformed branch does not require a fallback `capability.result`.
- Parent topology remains unchanged.

## Commands

```bash
openspec validate capability-failure-branching-matrix --type change
cargo test -p ralph-cli --test integration_live_capability parallel_parent_run_can_fallback_after_capability_failed_before_final_human_reply
cargo test -p ralph-cli --test integration_live_capability parallel_parent_run_can_emit_diagnostic_reply_for_malformed_capability_request_without_retry
cargo test -p ralph-cli --test integration_live_capability
cargo test -p ralph-core smoke_runner
openspec validate --all --strict
cargo fmt --all -- --check
git diff --check
cargo test
```
