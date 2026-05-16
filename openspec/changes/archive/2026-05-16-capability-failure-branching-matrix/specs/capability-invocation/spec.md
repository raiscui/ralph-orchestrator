## ADDED Requirements

### Requirement: Parent capability failure branching MUST support class-specific policies without a retry engine
Ralph MUST support parent-side capability failure branching where different `failure_class` values can lead to different explicit parent actions without requiring a generic retry engine.

The parent policy MUST be able to treat recoverable selection failures differently from malformed request failures, while keeping the parent topology unchanged.

#### Scenario: invalid capability id can choose explicit fallback
- **GIVEN** a parent run receives `capability.failed` with `failure_class = invalid_capability_id`
- **WHEN** the parent decides this is recoverable
- **THEN** it MUST be able to emit a later explicit fallback `capability.request`
- **AND** that fallback path MUST remain auditable separately from the original failure

#### Scenario: malformed request can choose diagnostic no-retry reply
- **GIVEN** a parent run receives `capability.failed` with `failure_class = malformed_request`
- **WHEN** the parent decides the malformed request should not be retried
- **THEN** it MUST be able to emit an explicit diagnostic `reply.human.message`
- **AND** the runtime MUST NOT require a fallback capability invocation for the run to complete

### Requirement: Malformed capability request diagnostics MUST remain explicit human-facing replies
Ralph MUST preserve the explicit human-facing reply contract when a parent run handles a malformed capability request.

A `capability.failed` event with `failure_class = malformed_request` MUST remain parent-consumable runtime evidence unless the parent explicitly emits `reply.human.message`.

#### Scenario: malformed failure alone is not exposed as final answer
- **GIVEN** a malformed `capability.request` causes parent-visible `capability.failed`
- **WHEN** no `reply.human.message` has been emitted
- **THEN** the runtime MUST NOT synthesize a final human answer from the failure event
- **AND** the failure event MUST remain separately auditable in the parent event log

#### Scenario: diagnostic reply is separate from malformed failure
- **GIVEN** a parent run sees `failure_class = malformed_request`
- **WHEN** it emits a diagnostic `reply.human.message`
- **THEN** the parent event log MUST preserve both the malformed failure event and the diagnostic human reply as separate events
- **AND** no fallback capability result MUST be required for that diagnostic branch
