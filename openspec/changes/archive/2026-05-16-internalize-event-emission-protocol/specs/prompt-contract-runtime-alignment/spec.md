## ADDED Requirements

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
