# Tasks: parent-capability-selection-ux

## 1. OpenSpec artifacts

- [x] 1.1 Write proposal for parent-side capability selection UX.
- [x] 1.2 Write design covering catalog injection, structured metadata, request contract, and topology isolation.
- [x] 1.3 Write delta spec for `capability-invocation`.
- [x] 1.4 Write test plan.

## 2. Discovery

- [x] 2.1 Locate existing capability list/summaries metadata builders.
- [x] 2.2 Locate parent coordinator prompt/context construction for `ralph#1` in parallel mode.
- [x] 2.3 Identify the narrowest injection point that reaches `ralph#1` without changing topology.

## 3. Implementation

- [x] 3.1 Add a bounded parent-visible runtime capability catalog builder.
- [x] 3.2 Inject the catalog and `capability.request` contract into `ralph#1` parent context when runtime invocation is available.
- [x] 3.3 Ensure catalog entries are based on structured metadata, not YAML comments.
- [x] 3.4 Preserve Phase 4 isolated invocation path and parent topology immutability.

## 4. Tests

- [x] 4.1 Add a focused test for catalog rendering: stable marker, capability id, kind, summary, and request payload fields.
- [x] 4.2 Add/extend integration dogfood so deterministic parent behavior reads/asserts the catalog before emitting a listed capability request.
- [x] 4.3 Assert parent `ralph.yml` remains byte-for-byte unchanged after catalog-based invocation.
- [x] 4.4 Dogfood Phase 3.1 inspect UX against the produced invocation id.

## 5. Validation

- [x] 5.1 `openspec validate parent-capability-selection-ux --type change`
- [x] 5.2 `openspec validate --all --strict`
- [x] 5.3 `cargo fmt --all -- --check`
- [x] 5.4 `cargo test -p ralph-cli --test integration_live_capability`
- [x] 5.5 `cargo test -p ralph-cli --test integration_capability`
- [x] 5.6 `cargo test -p ralph-cli capability::tests`
- [x] 5.7 `cargo test -p ralph-core smoke_runner`
- [x] 5.8 `cargo test`
- [x] 5.9 `git diff --check`
