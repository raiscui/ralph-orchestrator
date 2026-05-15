## ADDED Requirements

### Requirement: Parent coordinators MUST receive a runtime capability selection catalog
Ralph MUST provide `ralph#1` with a bounded runtime capability catalog when parent-side capability invocation is available.

The catalog MUST be visible in the parent coordinator context before the coordinator is expected to choose a capability. It MUST include enough structured information for the coordinator to identify callable capabilities and emit a valid `capability.request` event.

#### Scenario: parent context contains callable capability metadata
- **GIVEN** a parent run starts with runtime capability invocation available
- **WHEN** Ralph builds the context for `ralph#1`
- **THEN** the context MUST include a runtime capability catalog section
- **AND** the section MUST include at least one callable capability id when such capabilities exist
- **AND** each listed capability MUST include its kind and concise selection summary

#### Scenario: parent context includes the request event contract
- **GIVEN** a parent run starts with callable runtime capabilities
- **WHEN** Ralph builds the context for `ralph#1`
- **THEN** the context MUST include the `capability.request` topic
- **AND** the context MUST describe the required `request_id`, `capability_id`, and `input` payload fields

### Requirement: Parent capability selection metadata MUST be structured and bounded
Ralph MUST generate the parent-visible capability catalog from structured capability metadata rather than YAML comments or full instruction bodies.

The catalog MUST stay bounded by exposing concise summaries and input guidance. It MUST NOT inject every full workflow prompt, hat instruction body, or child topology into the parent context.

#### Scenario: missing comments do not remove selection metadata
- **GIVEN** a callable capability exists with structured metadata
- **WHEN** source comments are absent, stripped, or ignored
- **THEN** the parent-visible catalog MUST still include the capability using structured metadata

#### Scenario: full capability bodies are not loaded into parent context
- **GIVEN** a callable capability has a long workflow or hat instruction body
- **WHEN** Ralph builds the parent-visible catalog
- **THEN** the catalog MUST include only bounded selection metadata
- **AND** the full body MUST remain isolated to child or micro-run execution

### Requirement: Parent-side capability selection MUST preserve topology isolation
Ralph MUST treat parent-side capability selection as a selection/instruction surface only.

Selecting a capability from the parent-visible catalog MUST still invoke the existing isolated child or micro-run execution path. Ralph MUST NOT mutate the live parent `HatRegistry`, replace the parent configuration, or inject the selected capability into the parent topology.

#### Scenario: selected capability runs through existing isolated invocation
- **GIVEN** `ralph#1` selects a capability listed in the parent-visible catalog
- **WHEN** it emits a valid `capability.request`
- **THEN** Ralph MUST handle the request through the existing parent-triggered isolated invocation path
- **AND** the parent MUST receive the structured result or failure event defined by the capability invocation contract

#### Scenario: parent config remains unchanged after catalog-based selection
- **GIVEN** a parent run starts from a `ralph.yml`
- **WHEN** `ralph#1` selects and invokes a catalog-listed capability
- **THEN** the parent `ralph.yml` MUST remain byte-for-byte unchanged
- **AND** the invoked capability MUST be auditable through invocation artifacts and evidence-index entries
