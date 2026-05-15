# Tasks: live-runtime-capability-invocation

## 1. OpenSpec artifacts

- [x] 1.1 Write proposal for Phase 4 live runtime capability invocation.
- [x] 1.2 Write design covering structured request events, isolated execution reuse, result/failure return events, and topology non-mutation.
- [x] 1.3 Write delta spec for `capability-invocation`.
- [x] 1.4 Write test plan.

## 2. Implementation

- [x] 2.1 Define parent-run capability request parsing for structured `capability.request` events.
- [x] 2.2 Add an internal API that invokes a capability by id/input through the existing isolated child/micro-run path.
- [x] 2.3 Add parent-run request handling that invokes each request id at most once.
- [x] 2.4 Emit parent-visible structured result/failure events with request id and invocation id.
- [x] 2.5 Preserve parent topology and parent `ralph.yml` unchanged.

## 3. Tests

- [x] 3.1 Add integration dogfood where a deterministic parent run emits `capability.request`.
- [x] 3.2 Assert result/failure event includes request id and invocation id.
- [x] 3.3 Assert child artifacts and evidence index entries exist for invocation id.
- [x] 3.4 Dogfood Phase 3.1 inspect UX against the produced invocation id.
- [x] 3.5 Add focused duplicate request id coverage.
- [x] 3.6 Add malformed request coverage if implementation exposes a separable parser/handler.

## 4. Validation

- [x] 4.1 `openspec validate live-runtime-capability-invocation --type change`
- [x] 4.2 `openspec validate --all --strict`
- [x] 4.3 `cargo fmt --all -- --check`
- [x] 4.4 `cargo test -p ralph-cli --test integration_capability`
- [x] 4.5 `cargo test -p ralph-cli capability::tests`
- [x] 4.6 `cargo test -p ralph-core smoke_runner`
- [x] 4.7 `cargo test`
- [x] 4.8 `git diff --check`
