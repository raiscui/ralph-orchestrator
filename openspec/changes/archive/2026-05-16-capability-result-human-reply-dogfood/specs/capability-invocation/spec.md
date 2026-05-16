## ADDED Requirements

### Requirement: Parent-visible capability results MUST remain distinct from human-facing replies
Ralph MUST preserve the boundary between parent-consumable capability results and human-facing answers when both occur in the same parent run.

A parent-visible `capability.result` MUST NOT become a human-visible answer unless a workflow actor explicitly emits `reply.human.message`.

#### Scenario: coordinator explicitly turns capability result into human-visible reply
- **GIVEN** a parent run triggers an isolated capability invocation
- **AND** the runtime returns a parent-visible `capability.result`
- **WHEN** the coordinator decides to present that result to the human user
- **THEN** the coordinator MUST explicitly emit `reply.human.message`
- **AND** the runtime MUST preserve `capability.result` and `reply.human.message` as separate events

#### Scenario: capability result alone is not a human-facing answer
- **GIVEN** a parent run receives a valid `capability.result`
- **WHEN** no workflow actor emits `reply.human.message`
- **THEN** the runtime MUST NOT synthesize a human-facing answer automatically
- **AND** the capability result MUST remain only a parent-consumable runtime event unless an explicit human-facing reply is published

### Requirement: Explicit human-facing reply after capability invocation MUST be auditable through repo-native runtime artifacts
Ralph MUST make an explicit human-facing reply observable through existing runtime artifacts when it follows a parent-triggered capability invocation in the same run.

The audit path MUST use existing durable artifacts and MUST NOT require runtime graph artifacts or a live external backend.

#### Scenario: capability result and explicit human reply are both auditable
- **GIVEN** a parent run includes `capability.request`, a parent-visible `capability.result`, and a later explicit `reply.human.message`
- **WHEN** the run completes normally
- **THEN** `.ralph/events.jsonl` MUST preserve both `capability.result` and `reply.human.message`
- **AND** record-session MUST preserve evidence that the human-facing reply was published
- **AND** the invocation id MUST remain inspectable through the existing capability evidence UX
