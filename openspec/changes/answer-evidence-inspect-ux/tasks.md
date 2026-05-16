## 1. OpenSpec artifacts
- [x] 1.1 Write proposal for answer evidence inspect UX.
- [x] 1.2 Write design for command placement, lookup semantics, and output contract.
- [x] 1.3 Add delta spec for `request-reply-answer-evidence`.
- [x] 1.4 Write focused test plan.

## 2. Implementation
- [x] 2.1 Add `ralph tools answer inspect <correlation_id>`.
- [x] 2.2 Reuse `.ralph/evidence-index.jsonl` and `EvidenceIndexReader::find_by_correlation(...)`.
- [x] 2.3 Support `entries` and `missing` as successful lookup states.
- [x] 2.4 Fail clearly on `NoEntry`.
- [x] 2.5 Keep JSON output as the stable automation contract.

## 3. Tests
- [x] 3.1 Extend `integration_answer_evidence` to inspect request-id evidence as JSON.
- [x] 3.2 Extend `integration_answer_evidence` to inspect answer-id evidence as human output.
- [x] 3.3 Add coverage for unknown correlation id failure.
- [x] 3.4 Add focused unit coverage for explicit missing marker mapping.

## 4. Validation
- [x] 4.1 `openspec validate answer-evidence-inspect-ux --type change`
- [x] 4.2 `openspec validate --all --strict`
- [x] 4.3 `cargo fmt --all -- --check`
- [x] 4.4 `cargo test -p ralph-cli --test integration_answer_evidence`
- [x] 4.5 `cargo test -p ralph-cli answer`
- [x] 4.6 `cargo test -p ralph-core smoke_runner`
- [x] 4.7 `cargo test`
- [x] 4.8 `git diff --check`
