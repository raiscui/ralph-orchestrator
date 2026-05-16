## ADDED Requirements

### Requirement: Human-facing answer return MUST remain explicit after internal answer-return delivery
Ralph MUST preserve the boundary between internal answer-return and human-facing reply when both occur in the same runtime workflow.

An internal `reply.hat.message` answer MUST NOT become a human-visible reply unless a workflow actor explicitly emits `reply.human.message`.

#### Scenario: coordinator explicitly turns internal answer into human-visible reply
- **GIVEN** a coordinator emits an internal request to another hat
- **AND** that worker returns the answer via `reply.hat.message`
- **WHEN** the coordinator decides to present the result to the human user
- **THEN** the coordinator MUST explicitly emit `reply.human.message`
- **AND** the runtime MUST preserve both the internal answer-return evidence and the human-facing reply as separate events

#### Scenario: internal answer-return alone is not a human-facing reply
- **GIVEN** a worker emits `reply.hat.message` that successfully returns an answer to the requester
- **WHEN** no workflow actor emits `reply.human.message`
- **THEN** the runtime MUST NOT synthesize a human-facing reply on its own
- **AND** the answer MUST remain only an internal requester-return event unless an explicit human-facing event is published

### Requirement: Explicit human-facing answer return MUST be end-to-end auditable in repo-native runtime artifacts
Ralph MUST make an explicit human-facing answer return observable through existing runtime artifacts when it follows an internal answer-return in the same run.

The audit path MUST use existing durable artifacts and MUST NOT require a runtime graph artifact or an external live backend.

#### Scenario: explicit human-facing answer is visible in durable runtime artifacts
- **GIVEN** a runtime run includes both a delivered `reply.hat.message` and a later explicit `reply.human.message`
- **WHEN** the run completes normally
- **THEN** `.ralph/events.jsonl` MUST contain the explicit `reply.human.message`
- **AND** record-session output MUST preserve evidence that the human-facing reply was published
- **AND** the implementation MUST remain testable through a repo-native integration gate without requiring a live app-server backend
