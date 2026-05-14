# Design: request-reply-answer-evidence

## Context

Existing durable pieces:

- `Event.id` and `Event.reply` provide event-level correlation.
- `reply.hat.message` is already the explicit hat-to-hat answer-return topic.
- requester-return routing resolves the original requester from the referenced request event's `source_instance`.
- `routing.requester_return` records requester-return resolution outcomes.
- `runtime.delivery` records durable delivery decisions.
- `runtime-evidence-index-kernel` can index artifact links by correlation id.

The missing piece is a minimal evidence-writing contract that connects answer-return events to durable artifacts and failure markers.

## Goals

- Register successful answer-return events in the evidence index.
- Register requester-return failure evidence when resolution fails closed.
- Register missing-answer / timeout evidence when a request expects an answer but none arrives within the configured lifecycle.
- Preserve existing event log / runtime delivery records as truth sources.
- Keep Phase 2 minimal: evidence and tests first, no broad CLI UX.

## Non-goals

- No `ralph evidence summary` implementation.
- No request broker or queue service.
- No live `HatRegistry`, `EventLoop`, or `Supervisor` topology mutation.
- No automatic conversion of ordinary workflow events into answers.
- No global rule that every hat answer should be shown to the human.
- No multi-answer semantic aggregation beyond preserving multiple indexed answers for the same request id.

## Proposed evidence model

Phase 2 reuses `EvidenceIndexEntry` and adds runtime usage rules rather than expanding the schema.

Recommended correlation keys:

- request event id: primary lookup key for answers to one request.
- answer event id: direct lookup key for the answer event itself.
- requester-return resolution event id: lookup key for routing result evidence.

Recommended artifact kinds:

- `reply_event`: answer-return event stored in `.ralph/events.jsonl`.
- `event_log_jsonl`: event log artifact containing the answer and routing evidence.
- `runtime_delivery_record`: durable delivery record for successful requester-return delivery.
- `missing_artifact`: failure/timeout marker when expected answer evidence is absent.

If the existing enum is insufficient during implementation, update the Phase 1A spec first or prove that an existing kind can represent the new evidence without ambiguity.

## Runtime write contract

When requester-return receives `reply.hat.message` with a valid `reply`:

1. Resolve the request event id to the original requester instance.
2. Deliver only to that requester instance.
3. Persist normal event log / runtime delivery records as today.
4. Register evidence index entries that link:
   - request id -> reply event artifact
   - reply event id -> event log artifact
   - request id -> runtime delivery record, when durable delivery is available

When requester-return fails closed:

1. Do not fanout the answer as a normal workflow event.
2. Persist `routing.requester_return` failure evidence.
3. Register evidence index entries that link:
   - request id -> requester-return failure artifact
   - request id -> missing marker if no deliverable reply artifact exists

When answer timeout is introduced:

1. Timeout policy must be explicit and testable.
2. A timeout writes a missing marker for the expected answer correlation id.
3. Timeout evidence points to the event log or explicit timeout artifact, not to a graph layout.

## Read contract

A test or later tool can look up the request id and distinguish:

- answer delivered successfully
- answer emitted but requester-return resolution failed
- expected answer timed out / missing
- no answer lifecycle evidence exists

Phase 2 does not require a human-facing CLI summary, but the raw lookup result must be enough for tests to assert the state.

## Relationship to current specs

- `hat-request-reply-channel`: remains the semantic owner for answer-return routing.
- `runtime-evidence-index-kernel`: remains the schema and lookup owner.
- `runtime-graph-observability`: graph output may visualize the flow, but the durable source remains JSONL artifacts.
- `capability-invocation`: future capability calls can use the same request id / answer id evidence contract.

## Risks

### Risk: answer-return becomes a hidden workflow router

Mitigation: only `reply.hat.message` participates. Ordinary workflow events with `reply` stay normal workflow events.

### Risk: evidence index starts owning event content

Mitigation: index entries only point to `.ralph/events.jsonl` or explicit artifacts. The event log remains truth source.

### Risk: timeout semantics become vague

Mitigation: timeout must produce a missing marker with a correlation id and producer, or Phase 2 implementation is incomplete.

## Test strategy summary

See `test-plan.md` for detailed test gates. The first implementation should use focused unit tests around supervisor routing and evidence index writes before any live E2E.
