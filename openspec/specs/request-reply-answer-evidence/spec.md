# request-reply-answer-evidence Specification

## Purpose

Define the runtime evidence contract for explicit `reply.hat.message` answer-return delivery.

This spec ensures requester replies, fail-closed requester-return resolution, and missing-answer markers are registered in Ralph's durable evidence index while preserving the event log as the truth source and keeping ordinary workflow routing unchanged.
## Requirements
### Requirement: Answer-return evidence MUST index successful requester replies

The runtime MUST register evidence index entries when an explicit `reply.hat.message` answer is delivered back to the original requester.

The indexed evidence MUST preserve the request event id and answer event id correlation while keeping the event log as the truth source.

#### Scenario: delivered answer is indexed by request id

- **GIVEN** a request event with id `req-1` was published by `writer#1`
- **WHEN** `explorer#1` emits `reply.hat.message` with `reply="req-1"`
- **THEN** the answer MUST be delivered only to `writer#1`
- **AND** the evidence index MUST contain an entry that can be found by correlation id `req-1`
- **AND** that entry MUST point to the durable event log or answer artifact containing the reply

#### Scenario: answer event remains directly traceable

- **GIVEN** a delivered answer event has id `answer-1`
- **WHEN** the evidence index is queried by `answer-1`
- **THEN** the lookup MUST identify the reply event artifact or event log that contains that answer event

### Requirement: Answer-return evidence MUST record fail-closed requester-return resolution

The runtime MUST register failure evidence when `reply.hat.message` cannot resolve back to the requesting hat instance.

The runtime MUST NOT broadcast, fanout, or silently reinterpret the unresolved answer as a normal workflow event.

#### Scenario: unknown request id writes failure evidence

- **WHEN** a hat emits `reply.hat.message` with `reply="missing-req"`
- **THEN** the runtime MUST fail closed
- **AND** it MUST persist requester-return failure evidence
- **AND** the evidence index MUST contain a lookup result for `missing-req` that is distinguishable from a successful answer delivery

#### Scenario: request without source instance writes failure evidence

- **GIVEN** a referenced request event exists but has no `source_instance`
- **WHEN** a hat emits `reply.hat.message` for that request id
- **THEN** the runtime MUST fail closed
- **AND** the evidence index MUST record why no requester instance could be resolved

### Requirement: Answer-return evidence MUST support missing or timeout markers

When the runtime expects an answer for a request and the answer is not produced within the configured lifecycle, it MUST be possible to register a missing or timeout marker in the evidence index.

The marker MUST use the request event id as the correlation id and MUST be distinguishable from no index entry.

#### Scenario: missing expected answer is auditable

- **GIVEN** request id `req-timeout-1` expects an answer
- **WHEN** no answer artifact is produced before the answer lifecycle closes
- **THEN** the evidence index MUST contain a missing marker for `req-timeout-1`
- **AND** lookup by `req-timeout-1` MUST distinguish the marker from no entry

### Requirement: Answer-return evidence MUST preserve existing routing boundaries

The evidence contract MUST NOT change the existing `hat-request-reply-channel` routing semantics.

Only explicit `reply.hat.message` events with non-empty `reply` participate in requester-return answer evidence.

#### Scenario: ordinary workflow event with reply attribute is not answer-return evidence

- **WHEN** a hat emits `research.ready` with `reply="req-2"`
- **THEN** the runtime MUST route it as a normal workflow event
- **AND** the evidence index MUST NOT classify it as a delivered answer-return event solely because the `reply` attribute exists

#### Scenario: human-visible answer remains explicit

- **WHEN** a hat emits an internal `reply.hat.message`
- **THEN** the runtime MUST NOT automatically publish `reply.human.message`
- **AND** any human-visible answer MUST remain an explicit event or workflow decision

### Requirement: Answer-return evidence MUST remain graph-independent

The evidence index MUST use durable JSONL records or explicit artifacts for answer-return evidence and MUST NOT treat live runtime graph or Rerun graph layout as the truth source.

#### Scenario: graph visualization is not required for answer evidence lookup

- **GIVEN** no runtime graph output has been rendered
- **WHEN** a delivered answer is indexed
- **THEN** lookup by request id MUST still locate durable JSONL evidence
- **AND** the test MUST NOT require a Rerun graph artifact

### Requirement: Answer evidence inspect UX MUST locate answer-return evidence by correlation id
Ralph MUST provide a CLI inspect UX that locates explicit answer-return evidence-index entries by request id or answer id.

The inspect UX MUST read the existing `.ralph/evidence-index.jsonl` file and MUST NOT create a second evidence store.

#### Scenario: request-id answer evidence is inspectable
- **GIVEN** a request id has answer-return evidence entries
- **WHEN** the operator runs the answer inspect UX with that request id
- **THEN** the command MUST return the evidence-index entries for that correlation id
- **AND** the returned entries MUST include the durable artifact paths already recorded in `.ralph/evidence-index.jsonl`

#### Scenario: answer-id evidence is inspectable
- **GIVEN** an answer event id has answer-return evidence entries
- **WHEN** the operator runs the answer inspect UX with that answer id
- **THEN** the command MUST return the evidence-index entries for that correlation id
- **AND** the command MUST NOT require runtime graph artifacts to succeed

### Requirement: Answer evidence inspect UX MUST preserve missing vs no-entry semantics
Ralph MUST preserve the difference between explicit missing markers and no evidence entry when inspecting answer-return evidence.

#### Scenario: explicit missing evidence marker remains visible
- **GIVEN** `.ralph/evidence-index.jsonl` contains explicit missing answer evidence for a request id
- **WHEN** the operator runs the answer inspect UX with that request id
- **THEN** the command MUST succeed
- **AND** it MUST report lookup status `missing`
- **AND** it MUST include the missing marker entries

#### Scenario: unknown correlation id fails visibly
- **GIVEN** `.ralph/evidence-index.jsonl` has no matching entry for a correlation id
- **WHEN** the operator runs the answer inspect UX with that correlation id
- **THEN** the command MUST fail with a non-zero exit code
- **AND** the error MUST identify the missing correlation id and evidence index path
