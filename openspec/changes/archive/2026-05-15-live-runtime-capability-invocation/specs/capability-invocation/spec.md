## ADDED Requirements

### Requirement: Parent runs MUST trigger runtime capability invocation from structured requests
Ralph MUST allow a live parent run to trigger a runtime capability invocation from a structured capability request emitted by `ralph#1`.

The request MUST identify the capability to invoke and the input to pass to the isolated invocation.

#### Scenario: parent run emits a capability request

- **GIVEN** a parent run is processing output from `ralph#1`
- **WHEN** that output contains a structured capability request
- **THEN** Ralph MUST parse the request as a runtime capability invocation request
- **AND** Ralph MUST select the requested capability without changing the parent topology

#### Scenario: duplicate request id is not invoked twice

- **GIVEN** a parent run has already handled a capability request id
- **WHEN** the same request id appears again in later processed output
- **THEN** Ralph MUST NOT start a second isolated invocation for the duplicate request id

### Requirement: Parent-triggered capability invocation MUST use isolated execution
Ralph MUST execute parent-triggered capability requests through the isolated child or micro-run execution model.

Ralph MUST NOT mutate the live parent `HatRegistry`, replace the parent configuration, or inject the invoked capability into the parent topology.

#### Scenario: parent topology stays stable during parent-triggered invocation

- **GIVEN** a parent run has a fixed startup topology
- **WHEN** it triggers a capability invocation
- **THEN** the invocation MUST run as an isolated child or micro-run
- **AND** the parent topology MUST remain unchanged

#### Scenario: child artifacts are produced for parent-triggered invocation

- **GIVEN** a parent-triggered capability invocation starts
- **WHEN** the isolated execution completes or fails
- **THEN** Ralph MUST preserve invocation artifacts under `.ralph/capability-invocations/<invocation_id>/`
- **AND** Ralph MUST register evidence-index entries for the invocation id

### Requirement: Parent-triggered capability invocation MUST return structured result events
Ralph MUST return a structured result or failure event to the parent run after a parent-triggered capability invocation completes.

The returned event MUST include the original request id, the invocation id, the capability id, parent topology isolation status, and enough artifact references for later audit.

#### Scenario: successful parent-triggered invocation returns result event

- **GIVEN** a parent run triggers a capability invocation
- **WHEN** the isolated invocation succeeds
- **THEN** the parent run MUST receive a structured capability result event
- **AND** the event MUST include the request id and invocation id
- **AND** `ralph tools capability inspect <invocation_id> --json` MUST locate the evidence entries

#### Scenario: failed parent-triggered invocation returns failure event

- **GIVEN** a parent run triggers a capability invocation
- **WHEN** the isolated invocation fails
- **THEN** the parent run MUST receive a structured capability failure event
- **AND** the event MUST include the request id when available
- **AND** the failure MUST be auditable through artifacts or evidence-index entries when an invocation id was created
