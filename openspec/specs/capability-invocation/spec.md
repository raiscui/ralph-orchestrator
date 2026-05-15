# capability-invocation Specification

## Purpose

Define how Ralph exposes, selects, and invokes runtime capabilities through isolated child or micro-run executions.

This spec keeps parent topology stable while requiring structured invocation artifacts, result or failure artifacts, and durable evidence-index linkage for later audit.
## Requirements
### Requirement: Runtime capability discovery MUST use structured metadata summaries
Ralph MUST expose workflow capabilities and hat capabilities to `ralph#1` through structured metadata summaries instead of relying on YAML comments at runtime.

The metadata MUST be sufficient for `ralph#1` to understand what each capability does, when to use it, and what input/output contract it expects.

#### Scenario: Coordinator sees lightweight capability summaries
- **WHEN** a run starts with runtime capability invocation enabled
- **THEN** `ralph#1` MUST be able to inspect lightweight capability summaries without loading every full workflow or hat instruction body into its startup context

#### Scenario: Missing YAML comments does not remove capability discoverability
- **WHEN** a workflow or hat capability exists but its source file comments are absent, stripped, or ignored by parsing
- **THEN** capability discovery MUST still work from structured metadata

---

### Requirement: Workflow capability invocation MUST run as an isolated child execution
When `ralph#1` selects a workflow capability during a live run, Ralph MUST execute that workflow capability through an isolated child execution rather than replacing the active topology of the parent run.

The isolated child execution MUST use its own resolved configuration artifact and MUST return a structured invocation result to the parent run.

#### Scenario: Parent topology remains stable during workflow invocation
- **WHEN** `ralph#1` invokes a workflow capability after the parent run has already started
- **THEN** the parent run's active topology MUST remain unchanged while the workflow capability executes in isolation

#### Scenario: Workflow capability returns a structured result
- **WHEN** an isolated workflow capability run completes
- **THEN** Ralph MUST return a structured capability result or failure artifact to the parent run

---

### Requirement: Hat capability invocation MUST use an isolated transient execution model
When `ralph#1` selects a hat capability during a live run, Ralph MUST execute it through an isolated transient execution model rather than mutating the live `HatRegistry` of the active parent run.

The transient execution MAY be implemented as a micro-run or equivalent isolated child session, but it MUST preserve the stability of the parent topology.

#### Scenario: Hat capability does not require live registry mutation
- **WHEN** `ralph#1` invokes a hat capability that was not part of the parent run's startup topology
- **THEN** Ralph MUST execute that capability without injecting a new live hat definition into the parent's active registry

#### Scenario: Hat capability produces a parent-consumable result
- **WHEN** an isolated hat capability execution completes
- **THEN** Ralph MUST emit a structured capability result or failure artifact that the parent run can consume

---

### Requirement: Runtime capability invocation MUST emit auditable invocation artifacts
Ralph MUST record auditable artifacts for runtime capability selection and execution so that a later review can determine which capability was invoked, with what input contract, and what result came back.

#### Scenario: Invocation records selected capability and inputs
- **WHEN** a capability invocation begins
- **THEN** Ralph MUST record which capability was selected and the structured input contract used for that invocation

#### Scenario: Invocation records completion or failure
- **WHEN** a capability invocation ends
- **THEN** Ralph MUST record a structured result or failure artifact for that invocation

### Requirement: Capability invocation MUST register child-run evidence index entries

Ralph MUST register evidence index entries for each isolated capability invocation so that the invocation id can be used to locate the durable child-run artifacts.

The evidence index MUST link to existing durable artifacts and MUST NOT replace those artifacts as the truth source.

#### Scenario: successful micro-run artifacts are discoverable by invocation id

- **GIVEN** `ralph tools capability invoke` executes a hat capability through an isolated micro-run
- **WHEN** the invocation succeeds
- **THEN** `.ralph/evidence-index.jsonl` MUST contain entries for the invocation id
- **AND** those entries MUST include `capability_invoke_json`, `capability_result_json`, `resolved_config`, and `event_log_jsonl`
- **AND** each entry MUST point to the durable artifact path written by the invocation

#### Scenario: failed child-run artifacts are discoverable by invocation id

- **GIVEN** an isolated capability invocation writes `failed.json`
- **WHEN** the invocation completes with a failure result
- **THEN** `.ralph/evidence-index.jsonl` MUST contain a `capability_failed_json` entry for the invocation id
- **AND** that entry MUST have failure status
- **AND** the invocation MUST still preserve the parent topology

### Requirement: Capability invocation evidence MUST preserve parent topology isolation

Ralph MUST NOT mutate the parent run topology while registering evidence for a capability invocation.

Evidence registration MUST describe the isolated invocation artifacts, not inject the child capability into the parent topology.

#### Scenario: parent config remains unchanged after evidence registration

- **GIVEN** the workspace contains a parent `ralph.yml`
- **WHEN** `ralph tools capability invoke` writes artifacts, events, and evidence index entries
- **THEN** the parent `ralph.yml` MUST remain byte-for-byte unchanged
- **AND** the invocation/result artifacts MUST report `parent_topology_unchanged=true`

### Requirement: Capability invocation evidence MUST fail visibly when evidence cannot be recorded

Ralph MUST treat evidence-index recording failure as an invocation failure rather than silently returning a successful audit report.

#### Scenario: evidence index write failure is not hidden

- **GIVEN** capability child artifacts have been produced
- **WHEN** Ralph cannot write `.ralph/evidence-index.jsonl`
- **THEN** the command MUST return an error
- **AND** the operator MUST not receive a successful JSON report that omits evidence-index linkage

### Requirement: Capability invocation inspect UX MUST locate evidence by invocation id
Ralph MUST provide a CLI inspect UX that locates capability invocation evidence-index entries by invocation id.

The inspect UX MUST read the existing `.ralph/evidence-index.jsonl` file and MUST NOT create a second evidence store.

#### Scenario: successful invocation evidence is inspectable

- **GIVEN** `ralph tools capability invoke --json` has produced an invocation id
- **WHEN** the operator runs the inspect UX with that invocation id
- **THEN** the command MUST return the evidence-index entries for that invocation id
- **AND** the returned entries MUST include the durable artifact paths recorded in `.ralph/evidence-index.jsonl`

#### Scenario: inspect supports machine-readable output

- **GIVEN** an invocation id has evidence-index entries
- **WHEN** the operator runs the inspect UX with `--json`
- **THEN** the command MUST emit valid JSON
- **AND** the JSON MUST include the invocation id, lookup status, and evidence entries

#### Scenario: inspect supports human-readable output

- **GIVEN** an invocation id has evidence-index entries
- **WHEN** the operator runs the inspect UX without `--json`
- **THEN** the command MUST print a concise human-readable summary of artifact kinds, paths, producers, and statuses

### Requirement: Capability invocation inspect UX MUST fail visibly for unknown invocation ids
Ralph MUST return a non-zero command result when the inspect UX cannot find any evidence-index entry for the requested invocation id.

The error message MUST identify the missing invocation id and the evidence index path used for lookup.

#### Scenario: unknown invocation id is not treated as success

- **GIVEN** `.ralph/evidence-index.jsonl` exists
- **WHEN** the operator inspects an invocation id that has no matching evidence entry
- **THEN** the command MUST fail
- **AND** the operator MUST receive a clear no-entry message

#### Scenario: explicit missing evidence markers remain visible

- **GIVEN** `.ralph/evidence-index.jsonl` contains explicit missing evidence markers for an invocation id
- **WHEN** the operator inspects that invocation id
- **THEN** the command MUST report the lookup status as missing
- **AND** the command MUST include the missing marker entries instead of hiding them

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

