# Change: request-reply-answer-evidence

## Why

Phase 1A added a minimal runtime evidence index kernel, but no runtime path writes answer-return evidence into it yet.

Ralph already has an explicit `reply.hat.message` answer-return topic and a requester-return routing path. The remaining gap is that a failed or successful answer return is hard to audit from one place. The operator still needs to inspect event logs, routing records, and runtime graph output separately.

Phase 2 should close the smallest useful loop: when a request/reply answer-return path succeeds or fails, the runtime records correlation evidence that can be resolved through the evidence index while preserving the existing event log as the truth source.

## What Changes

- Add an OpenSpec capability for request/reply answer-return evidence.
- Define the minimal runtime contract for registering answer-return artifacts in the evidence index.
- Define success and failure evidence for `reply.hat.message` requester-return resolution.
- Define timeout/missing-answer markers as explicit evidence, not silent absence.
- Keep routing semantics in `hat-request-reply-channel`; this change only adds evidence linkage and minimal answer lifecycle visibility.

## Capabilities

### New Capabilities

- `request-reply-answer-evidence`: Defines how requester-return answer events are indexed, correlated, and audited.

### Modified Capabilities

- `hat-request-reply-channel`: Semantics are reused but not changed in Phase 2 OpenSpec planning.
- `runtime-evidence-index-kernel`: The evidence index becomes the lookup surface for request/reply answer artifacts.

## Impact

- Expected later code touchpoints:
  - `crates/ralph-core/src/parallel/supervisor/routing.rs`
  - `crates/ralph-core/src/evidence_index.rs`
  - `crates/ralph-core/src/event_logger.rs`
  - parallel supervisor routing tests
  - focused evidence index tests
- This OpenSpec-only step does not implement code.
- This change must not introduce a live topology mutation path.
- This change must not turn every reply attribute into human-visible output.
