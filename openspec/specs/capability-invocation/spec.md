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

### Requirement: Parent-visible capability failures MUST include a structured failure class
Ralph MUST include a structured `failure_class` in parent-visible `capability.failed` payloads.

The `failure_class` MUST be the preferred parent branching input for capability failure handling. Parent policy MUST be able to depend on that field instead of parsing free-form error strings.

#### Scenario: invalid capability id is classified before fallback
- **GIVEN** a parent run emits a `capability.request` with an invalid capability id
- **WHEN** the runtime returns `capability.failed`
- **THEN** the failure payload MUST include `failure_class = invalid_capability_id`
- **AND** the parent MUST be able to see that structured class in a later turn before choosing fallback behavior

#### Scenario: child execution failure remains distinguishable from pre-invocation failure
- **GIVEN** an isolated capability child or micro-run starts and later fails
- **WHEN** the runtime returns failure records or artifacts
- **THEN** the failure MUST remain distinguishable from pre-invocation selection failures through a structured failure class
- **AND** any created invocation id or failure artifact links MUST remain auditable

### Requirement: Parent branching policy MUST prefer structured failure class over free-form error parsing
Ralph MUST preserve a product contract where parent-side capability branching decisions can be driven by structured failure classification.

Free-form `error` text MAY still be present for human diagnosis, but it MUST NOT be the only stable signal available for parent orchestration.

#### Scenario: fallback branch keys off invalid capability class
- **GIVEN** a parent run receives `capability.failed` with `failure_class = invalid_capability_id`
- **WHEN** the parent decides how to continue
- **THEN** it MUST be able to emit an explicit fallback capability request based on that structured class
- **AND** the later fallback success and final `reply.human.message` MUST remain separately auditable

