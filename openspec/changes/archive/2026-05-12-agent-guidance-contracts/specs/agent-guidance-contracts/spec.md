## ADDED Requirements

### Requirement: Agent guidance schema MUST define required guidance asset sections
Ralph MUST provide a documented agent guidance schema for core agent-facing guidance assets.

The schema MUST distinguish root operating contracts, long-term experience, prompt contracts, skills, OpenSpec changes, reports, and runbooks.

#### Scenario: Maintainer can classify a guidance asset
- **WHEN** a maintainer adds a new agent-facing guidance file
- **THEN** the schema MUST explain which asset type it belongs to and what responsibility that type owns

#### Scenario: Schema avoids runtime feature creep
- **WHEN** the schema describes agent guidance governance
- **THEN** it MUST clarify that guidance governance is separate from live runtime topology changes

---

### Requirement: Prompt contract MUST define minimum agent response and evidence expectations
Ralph MUST provide a prompt contract document that defines the minimum behavior expectations for prompt-like assets, skills, hats, and final user responses.

The contract MUST cover outcome-first reporting, evidence before completion claims, scope boundaries, and escalation conditions.

#### Scenario: Prompt author checks final response requirements
- **WHEN** a prompt or skill asks an agent to report completion
- **THEN** the prompt contract MUST require the report to include outcome, evidence, and known gaps when applicable

#### Scenario: Prompt author checks escalation boundaries
- **WHEN** a prompt or skill reaches a destructive, credential-gated, or materially branching action
- **THEN** the prompt contract MUST require escalation instead of silent execution

---

### Requirement: Guidance manifest MUST be the single truth source for core guidance assets
Ralph MUST provide a machine-readable guidance asset manifest for core agent-facing guidance assets.

The manifest MUST include stable asset ids, asset types, repository-relative paths, status values, summaries, and whether the asset must be indexed by `AGENTS.md`.

#### Scenario: Core guidance asset is registered
- **WHEN** a core guidance asset is added or moved
- **THEN** the manifest MUST be updated with the asset id, type, path, status, summary, and index requirement

#### Scenario: Machine-readable metadata does not rely on comments
- **WHEN** a guidance asset contains comments or prose headers
- **THEN** manifest validation MUST rely on structured manifest fields rather than parsing comments as metadata

---

### Requirement: Guidance manifest verifier MUST fail on drift
Ralph MUST provide an automated verifier for the guidance asset manifest.

The verifier MUST fail when manifest entries point to missing active files, use invalid types or statuses, duplicate ids, escape the repository root, omit summaries, or require `AGENTS.md` indexing without the path appearing in `AGENTS.md`.

#### Scenario: Missing guidance asset fails verification
- **WHEN** an active manifest entry points to a missing file
- **THEN** the verifier MUST fail and identify the asset id and missing path

#### Scenario: Required AGENTS index is missing
- **WHEN** a manifest entry sets `required_in_agents_index = true`
- **AND** `AGENTS.md` does not contain the entry path
- **THEN** the verifier MUST fail and identify the missing index reference

#### Scenario: Verifier participates in normal test gates
- **WHEN** maintainers run the normal repository test gate
- **THEN** manifest drift MUST be checked without requiring a separate manual command
