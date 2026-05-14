# Tasks: capability-evidence-inspect-ux

## 1. OpenSpec artifacts

- [x] 1.1 Write proposal for Phase 3.1 capability invocation evidence inspect UX.
- [x] 1.2 Write design covering command placement, lookup semantics, JSON/human output, and non-goals.
- [x] 1.3 Write delta spec for `capability-invocation`.
- [x] 1.4 Write test plan.

## 2. Implementation

- [x] 2.1 Add `inspect` subcommand under `ralph tools capability`.
- [x] 2.2 Read `.ralph/evidence-index.jsonl` through `EvidenceIndexReader::find_by_correlation(...)`.
- [x] 2.3 Emit stable JSON output with invocation id, lookup status, index path, and evidence entries.
- [x] 2.4 Emit concise human-readable output with artifact kind, path, producer, and status.
- [x] 2.5 Return a clear non-zero error for `EvidenceLookup::NoEntry`.
- [x] 2.6 Preserve `Missing` lookup status when explicit missing markers exist.

## 3. Tests

- [x] 3.1 Extend `crates/ralph-cli/tests/integration_capability.rs` to inspect a real invocation id with `--json`.
- [x] 3.2 Add integration coverage for human output.
- [x] 3.3 Add integration coverage for unknown invocation id failure.
- [x] 3.4 Add focused unit coverage for inspect output mapping if useful.

## 4. Validation

- [x] 4.1 `openspec validate capability-evidence-inspect-ux --type change`
- [x] 4.2 `openspec validate --all --strict`
- [x] 4.3 `cargo fmt --all -- --check`
- [x] 4.4 `cargo test -p ralph-cli --test integration_capability`
- [x] 4.5 `cargo test -p ralph-cli capability::tests`
- [x] 4.6 `cargo test -p ralph-core smoke_runner`
- [x] 4.7 `cargo test`
- [x] 4.8 `git diff --check`
