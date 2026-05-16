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
- **AND** the record-session evidence MUST show termination reason `CompletionPromise`

