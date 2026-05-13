## ADDED Requirements

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
