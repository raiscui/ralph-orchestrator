## 1. Spec and test plan
- [x] 1.1 Create proposal for `capability-failure-fallback-human-reply-dogfood`.
- [x] 1.2 Create design for parent fallback orchestration after `capability.failed`.
- [x] 1.3 Add delta spec for fallback orchestration and explicit final human reply.
- [x] 1.4 Write focused test plan for the failure -> fallback -> reply flow.
- [x] 1.5 Validate the change with `openspec validate capability-failure-fallback-human-reply-dogfood --type change`.

## 2. Harness and behavior proof
- [x] 2.1 Confirm the existing `integration_live_capability.rs` harness can surface `capability.failed` back into a later parent turn.
- [x] 2.2 Define the smallest deterministic failure sample, preferably invalid capability id followed by valid fallback id.
- [x] 2.3 Confirm the later parent prompt contains enough failure context to make fallback assertions stable.

## 3. Focused implementation and assertions
- [x] 3.1 Add a focused live capability integration gate for failure -> fallback -> explicit human reply.
- [x] 3.2 Assert parent event log preserves `capability.failed`, fallback `capability.result`, and final `reply.human.message` separately.
- [x] 3.3 Assert fallback success keeps `parent_topology_unchanged = true`.
- [x] 3.4 Assert fallback invocation evidence remains inspectable.
- [x] 3.5 Assert record-session preserves the final explicit human-facing reply.

## 4. Verification and archive
- [x] 4.1 Run `cargo test -p ralph-cli --test integration_live_capability`.
- [x] 4.2 Run `cargo test -p ralph-core smoke_runner`.
- [x] 4.3 Run `openspec validate --all --strict`.
- [x] 4.4 Run `cargo test`.
- [x] 4.5 Run `openspec validate capability-failure-fallback-human-reply-dogfood --type change` after code changes.
- [x] 4.6 Archive the change and sync stable spec if the gate proves the contract.
