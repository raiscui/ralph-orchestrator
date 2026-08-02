# prompt-contract-runtime-alignment Specification

## Purpose
TBD - created by archiving change prompt-contract-runtime-alignment. Update Purpose after archive.
## Requirements
### Requirement: Runtime hat prompts MUST include the prompt contract output fields
Ralph MUST include the prompt contract output fields in runtime prompts generated for custom hats.

The runtime prompt MUST explicitly name outcome, evidence, changed files, known gaps, and next suggestions as the expected report fields when a hat reports results or completion.

#### Scenario: Custom hat prompt includes output contract fields
- **WHEN** `InstructionBuilder::build_custom_hat` generates instructions for a custom hat
- **THEN** the REPORT section MUST include outcome, evidence, changed files, known gaps, and next suggestions

#### Scenario: Runtime event loop uses aligned custom hat prompt
- **WHEN** `EventLoop::build_prompt` builds a prompt for a custom hat
- **THEN** the prompt MUST include the same output contract fields from `InstructionBuilder`

---

### Requirement: Runtime prompts MUST preserve evidence-before-completion semantics
Ralph MUST preserve the existing evidence-before-completion semantics while adding output contract fields.

The prompt update MUST NOT weaken test/build/evidence requirements, task closure requirements, or must-publish requirements for hats with publishable events.

#### Scenario: Evidence requirements remain present
- **WHEN** a runtime hat prompt includes the output contract fields
- **THEN** it MUST still require verification before reporting done
- **AND** it MUST still forbid reporting completion without evidence

#### Scenario: Publish requirements remain present
- **WHEN** a custom hat has publishable events
- **THEN** the prompt MUST still require publishing one of those events and MUST NOT allow ending the iteration without publishing

---

### Requirement: Prompt contract documentation MUST identify runtime prompt test anchors
Ralph MUST document that the prompt contract output fields are runtime prompt test anchors.

The documentation MUST explain that prompt-like assets and generated runtime prompts should preserve these field names so tests can detect drift.

#### Scenario: Maintainer changes prompt wording
- **WHEN** a maintainer updates prompt-like instructions or runtime prompt templates
- **THEN** `docs/prompt-contract.md` MUST tell them to preserve the output contract anchors or update tests deliberately

### Requirement: Runtime prompts MUST include a built-in event emission protocol
Ralph MUST include a built-in event emission protocol section in runtime prompts for parallel hat instances that are expected to publish workflow events.

The section MUST describe the canonical event envelope and enough stable attributes for hats to emit events without copying generic protocol instructions into execution-directory `ralph.yml` files.

#### Scenario: parallel hat prompt contains event emission protocol
- **GIVEN** a parallel run contains a hat with publishable workflow events
- **WHEN** Ralph builds the runtime prompt for that hat instance
- **THEN** the prompt MUST include a stable event emission protocol section
- **AND** the section MUST include the `<event` envelope concept
- **AND** the section MUST include the `topic` attribute
- **AND** the section MUST include the stdout-only emission rule

#### Scenario: event emission protocol includes stable routing attributes
- **GIVEN** Ralph builds the event emission protocol section
- **WHEN** the section is rendered
- **THEN** it MUST document `id`, `reply`, `target`, `target_instance`, `session_strategy`, `workspace_strategy`, `turn_action`, and `spawn_instance` as supported event attributes

### Requirement: Workflow configs MUST NOT be the source of truth for generic event envelope syntax
Ralph MUST treat generic event envelope syntax as a built-in runtime prompt contract, not as local workflow configuration knowledge.

Execution-directory `ralph.yml` files SHOULD define workflow-specific topics, payload fields, and backpressure rules, but they MUST NOT be required to restate the generic event envelope for the runtime to behave correctly.

#### Scenario: workflow config omits generic event-format tutorial
- **GIVEN** a workflow config defines hats, triggers, publishes, and workflow-specific payload requirements
- **AND** the config does not restate a generic `<event topic="...">payload</event>` tutorial in each hat instruction
- **WHEN** Ralph builds runtime prompts for the hats
- **THEN** the prompts MUST still include the built-in event emission protocol section
- **AND** hats MUST still have enough protocol guidance to publish their configured topics

#### Scenario: workflow-specific payload requirements remain local
- **GIVEN** a workflow requires a domain payload field such as `experiment_id` or `verification_evidence`
- **WHEN** Ralph builds runtime prompts
- **THEN** Ralph MUST NOT invent those workflow-specific payload requirements from the generic event emission protocol
- **AND** the workflow config or prompt MUST remain responsible for those domain fields

### Requirement: Built-in event examples MUST avoid accidental event replay from shared overlays
Ralph MUST prevent shared all-hat overlay documentation from being parsed or copied as accidental live workflow events.

Raw emittable event examples MAY appear in role-specific runtime instructions that intentionally teach a hat how to emit events, but generic shared overlays MUST render examples as non-emittable documentation or otherwise guard them from accidental replay.

#### Scenario: all-hat overlay examples are non-emittable
- **GIVEN** the compiled all-hat overlay contains event documentation examples
- **WHEN** Ralph injects the overlay into a hat prompt
- **THEN** raw shared-overlay examples MUST NOT remain as directly emittable `<event ...>` blocks

#### Scenario: role-specific event protocol can show the real envelope
- **GIVEN** Ralph renders the built-in event emission protocol for a publishing hat
- **WHEN** the rendered protocol is inserted into that hat's runtime prompt
- **THEN** it MAY show the real event envelope where that envelope is explicitly part of the hat's output contract
- **AND** tests MUST distinguish this intentional protocol section from escaped shared-overlay examples

### Requirement: The built-in event emission protocol MUST be proven in a live startup gate
Ralph MUST maintain a repeatable live startup integration gate that proves the built-in event emission protocol appears in the real coordinator prompt generated by the startup bootstrap path.

The gate MUST inspect the captured live `ralph#1` prompt and MUST also confirm record-session convergence facts for the live run that reuses the bootstrap-resolved configuration artifact.

#### Scenario: Live gate captures the real coordinator prompt and convergence evidence
- **GIVEN** the live startup integration gate runs a real `ralph run` from a workspace with no `ralph.yml` and no `PROMPT.md`
- **WHEN** the gate captures the generated `ralph#1` prompt and the run's record-session artifact
- **THEN** the prompt MUST include `Act as Ralph's startup bootstrap coordinator`
- **AND** the prompt MUST include `## RALPH EVENT EMISSION PROTOCOL`
- **AND** the prompt MUST include `reply.human.message`
- **AND** the record-session evidence MUST show `parallel-cli`
- **AND** the record-session evidence MUST show termination reason `CompletionPromise`---

### Requirement: Prompt surfaces MUST share a protocol and role-contract matrix
Ralph MUST define a prompt-surface matrix that states which runtime protocol and role-contract fields belong in coordinator prompts, shared all-hat overlays, configured hat prompts, and task-derived dynamic worker prompts.

The matrix MUST prevent coordinator-only responsibilities from leaking into ordinary workers while still giving workers enough protocol guidance to publish their allowed result topics through stdout.

#### Scenario: dynamic worker prompt contains canonical role contract only
- **WHEN** a task-derived worker prompt is generated from `topology.spawn_group`
- **THEN** the worker prompt MUST include the canonical role contract summary or role section generated by runtime normalization
- **AND** it MUST NOT treat raw spawn payload fields as a second authority over allowed topics or responsibilities

#### Scenario: coordinator-only instructions stay out of worker prompt
- **WHEN** the coordinator prompt includes instructions for `topology.spawn_group` or workflow completion decisions
- **THEN** ordinary dynamic worker prompts MUST NOT inherit those coordinator-only instructions unless explicitly authorized by the role contract

### Requirement: Shared overlays MUST not provide live-emittable accidental events
Ralph MUST keep shared prompt overlays from containing accidental live-emittable workflow events that could be copied or parsed as runtime output.

Generic examples MAY explain event syntax, but shared overlays MUST escape, fence, or otherwise mark examples so they are not confused with authorized final output events.

#### Scenario: all-hat overlay example is non-emittable
- **WHEN** the all-hat overlay documents an example event envelope
- **THEN** the rendered shared prompt MUST not contain an unguarded live workflow event that a worker can accidentally copy as its final answer

#### Scenario: role-specific final event contract remains real
- **WHEN** a worker role requires publishing `analysis.done`
- **THEN** the role-specific prompt MAY show the exact real event envelope for that result topic
- **AND** tests MUST distinguish this authorized role-specific example from generic shared overlay examples

### Requirement: Prompt alignment tests MUST cover dynamic role workers
Ralph MUST test prompt alignment for task-derived dynamic workers, not only configured custom hats.

The tests MUST assert that dynamic worker prompts include allowed result topics, stdout-only output guidance, role identity, and forbidden responsibilities derived from the canonical role contract.

#### Scenario: dynamic worker prompt has output and boundary anchors
- **WHEN** runtime builds a prompt for `builder#2` from a task-derived `protocol_architect` role
- **THEN** the prompt MUST include the allowed output topic such as `analysis.done`
- **AND** the prompt MUST include forbidden responsibility boundaries such as not coordinating globally or spawning other instances

