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

