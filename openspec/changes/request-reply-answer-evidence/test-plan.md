# Test plan: request-reply-answer-evidence

## Scope

This test plan covers Phase 2 planning for request/reply answer-return evidence. It assumes Phase 1A `runtime-evidence-index-kernel` exists and remains the artifact lookup kernel.

## Test layers

### 1. Successful answer-return evidence tests

Goal: prove a delivered `reply.hat.message` leaves evidence index entries.

Planned assertions:

- A request event id is remembered with its original requester instance.
- A `reply.hat.message` with `reply=<request_id>` is delivered only to the original requester.
- Evidence index lookup by request id returns answer event evidence.
- Evidence index lookup by answer event id returns the answer artifact or event log artifact.
- Event log remains parseable directly.

### 2. Fail-closed evidence tests

Goal: prove unresolved answers do not leak into normal workflow routing and still leave audit evidence.

Planned assertions:

- Unknown `reply` request id produces requester-return failure evidence.
- Request event without `source_instance` produces requester-return failure evidence.
- Failure lookup is distinguishable from success and no entry.
- The answer is not broadcast/fanout to unrelated hats.

### 3. Missing / timeout marker tests

Goal: prove expected but absent answers can be audited.

Planned assertions:

- Missing expected answer registers `EvidenceStatus::Missing` or equivalent missing marker.
- Lookup by request id distinguishes missing marker from no entry.
- Missing marker points to an event log or explicit timeout/missing artifact.
- No graph artifact is required.

### 4. Routing boundary guardrail tests

Goal: preserve `hat-request-reply-channel` semantics.

Planned assertions:

- Ordinary workflow event with `reply` is not indexed as delivered answer-return evidence.
- `reply.hat.message` without non-empty `reply` fails closed.
- Internal `reply.hat.message` does not automatically publish `reply.human.message`.
- Multiple replies to the same request remain multiple indexed entries instead of hidden aggregation.

### 5. Regression and smoke gates

Expected command shape after implementation:

```bash
cargo test --package ralph-core --lib parallel::supervisor::routing::tests::<focused_test> -- --exact
cargo test --package ralph-core --lib evidence_index::tests
cargo test -p ralph-core smoke_runner
cargo test
openspec validate request-reply-answer-evidence --type change
openspec validate --all --strict
```

## Stop conditions

Stop implementation and return to design if:

- Answer evidence requires live topology mutation.
- Ordinary workflow events with `reply` become answer-return events.
- Evidence lookup depends on runtime graph / Rerun graph layout.
- Human-visible reply output becomes implicit instead of explicit.
- Timeout semantics cannot write a durable missing marker.
