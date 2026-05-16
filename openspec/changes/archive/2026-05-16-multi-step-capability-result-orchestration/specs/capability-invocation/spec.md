## ADDED Requirements

### Requirement: Parent runs MUST support multi-step orchestration over multiple capability results
Ralph MUST allow a parent run to emit multiple distinct `capability.request` events across multiple turns, using earlier parent-visible capability results to inform later capability requests.

Each step MUST continue to use isolated child or micro-run execution, and the parent topology MUST remain unchanged across the sequence.

#### Scenario: second capability request follows the first capability result
- **GIVEN** a parent run has already emitted a valid `capability.request` with request id `req-step-1`
- **AND** the runtime has returned a parent-visible `capability.result` for `req-step-1`
- **WHEN** the coordinator chooses the next capability step
- **THEN** it MUST be able to emit a second valid `capability.request` with a different request id
- **AND** the runtime MUST execute that second capability request without mutating the parent topology

#### Scenario: multiple capability results remain separately auditable
- **GIVEN** a parent run emits multiple distinct capability requests in sequence
- **WHEN** the isolated invocations complete
- **THEN** each invocation MUST preserve its own invocation id and durable artifacts
- **AND** the resulting `capability.result` events MUST remain separately visible in the parent event log

### Requirement: Final human-facing answer after multi-step capability orchestration MUST remain explicit
Ralph MUST preserve the explicit human-facing reply contract after a multi-step capability orchestration chain.

A sequence of `capability.result` events MUST NOT become a human-visible answer unless a workflow actor explicitly emits `reply.human.message`.

#### Scenario: final reply is emitted only after multi-step chain completes
- **GIVEN** a parent run has received multiple parent-visible `capability.result` events
- **WHEN** the coordinator decides to present the final conclusion to the human user
- **THEN** it MUST explicitly emit `reply.human.message`
- **AND** the runtime MUST preserve the final human-facing reply as a separate event from the intermediate capability results

#### Scenario: intermediate capability results are not mistaken for human replies
- **GIVEN** a parent run has received an intermediate `capability.result`
- **WHEN** no explicit `reply.human.message` has been emitted yet
- **THEN** the runtime MUST NOT synthesize a human-facing reply
- **AND** the intermediate capability result MUST remain only a parent-consumable runtime event
