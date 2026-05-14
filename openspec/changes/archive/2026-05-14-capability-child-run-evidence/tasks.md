# Tasks: capability-child-run-evidence

## 1. OpenSpec artifacts

- [x] Write proposal describing Phase 3 capability child-run evidence linkage.
- [x] Write design covering evidence entries, ordering, failure behavior, and non-goals.
- [x] Write delta spec for `capability-invocation`.
- [x] Write test plan.

## 2. Implementation

- [x] Add capability invocation evidence-index recording in `crates/ralph-cli/src/capability.rs`.
- [x] Register `invoke.json`, `resolved-config.yml`, `.ralph/events.jsonl`, and `result.json` on success.
- [x] Register `failed.json` with failure status on child-run failure.
- [x] Preserve parent topology and existing artifact paths.

## 3. Tests

- [x] Extend `crates/ralph-cli/tests/integration_capability.rs` to query `.ralph/evidence-index.jsonl` by invocation id.
- [x] Add or extend focused unit tests for success/failure evidence entries if needed.
- [x] Verify parent `ralph.yml` remains unchanged.

## 4. Validation

- [x] `openspec validate capability-child-run-evidence --type change`
- [x] `openspec validate --all --strict`
- [x] `cargo fmt --all -- --check`
- [x] `cargo test -p ralph-cli --test integration_capability`
- [x] `cargo test -p ralph-core smoke_runner`
- [x] `cargo test`
- [x] `git diff --check`
