## ADDED Requirements

### Requirement: Parent runs MUST support fallback orchestration after parent-visible capability failures
Ralph MUST allow a parent run to continue orchestrating after it receives a structured parent-visible `capability.failed` event.

The parent MUST be able to use failure context from an earlier capability step to decide a later fallback capability request, while keeping each later capability step isolated and the parent topology unchanged.

#### Scenario: fallback capability request follows parent-visible failure
- **GIVEN** a parent run emits a `capability.request`
- **AND** the runtime returns a structured parent-visible `capability.failed`
- **WHEN** the coordinator decides on a fallback step
- **THEN** it MUST be able to emit a later valid fallback `capability.request`
- **AND** the runtime MUST execute that fallback step without mutating the parent topology

#### Scenario: failure and fallback success remain separately auditable
- **GIVEN** a parent run first receives `capability.failed` and later receives fallback `capability.result`
- **WHEN** the run completes normally
- **THEN** the parent event log MUST preserve the failure and fallback success as separate events
- **AND** any fallback invocation id that exists MUST remain inspectable through the existing capability evidence UX

### Requirement: Final human-facing answer after failure fallback MUST remain explicit
Ralph MUST preserve the explicit human-facing reply contract when a parent run recovers from `capability.failed` through a later fallback capability step.

Neither `capability.failed` nor a later `capability.result` MUST become a human-visible answer unless a workflow actor explicitly emits `reply.human.message`.

#### Scenario: final reply is emitted only after fallback branch completes
- **GIVEN** a parent run has received `capability.failed` for an earlier step and `capability.result` for a fallback step
- **WHEN** the coordinator decides to present the conclusion to the human user
- **THEN** it MUST explicitly emit `reply.human.message`
- **AND** the runtime MUST preserve the final human-facing reply as a separate event from both the failure and the fallback success

#### Scenario: failure event alone is not a human-facing answer
- **GIVEN** a parent run has received a structured `capability.failed`
- **WHEN** no workflow actor emits `reply.human.message`
- **THEN** the runtime MUST NOT synthesize a human-facing reply automatically
- **AND** the failure event MUST remain only a parent-consumable runtime event unless an explicit human-facing reply is published
