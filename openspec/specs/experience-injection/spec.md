# experience-injection Specification

## Purpose
TBD - created by archiving change scoped-experience-system. Update Purpose after archive.
## Requirements
### Requirement: Ordinary hats MUST inject experience in descending scope order
The system MUST inject reusable knowledge for ordinary hats in descending scope order:

1. project experience
2. role experience
3. topic shared context summary
4. instance context summary
5. runtime task state

The system MUST NOT reverse this order by default.

#### Scenario: Ordinary hat sees global rules before local noise
- **WHEN** a non-coordinator hat begins work on a known role and topic
- **THEN** it MUST receive project experience before role experience
- **THEN** it MUST receive role experience before topic and instance summaries

#### Scenario: Runtime tasks remain last in knowledge injection
- **WHEN** the system builds a prompt for an ordinary hat
- **THEN** runtime work state MUST be injected after reusable experience layers

---

### Requirement: `ralph#1` MUST use metadata-first injection before selecting a workflow or hat
The system MUST let `ralph#1` choose workflows or hats using project experience plus lightweight workflow or hat descriptions before loading narrower role-specific knowledge.

Before workflow selection is resolved, the system MUST NOT eagerly inject all role experience files into `ralph#1`.

#### Scenario: Ralph selects from metadata before reading role experience
- **WHEN** `ralph#1` receives a user message in a workspace with no explicit `ralph.yml`
- **THEN** it MUST read project experience and candidate workflow or hat descriptions before reading a specific role experience file

#### Scenario: Ralph loads owner role experience only after workflow choice narrows scope
- **WHEN** `ralph#1` resolves the active workflow owner
- **THEN** it MAY load that owner's role experience
- **THEN** it MUST NOT treat all unrelated role experience files as mandatory first-pass context

---

### Requirement: Experience loading MUST be summary-first and on-demand
The system MUST prefer summaries over full-file eager loading for topic shared context and instance context.

The system MUST only read full detail when summary information is insufficient for the current decision.

#### Scenario: Topic summary satisfies normal routing decision
- **WHEN** `ralph#1` needs to know the current state of an active topic
- **THEN** it MUST be able to rely on the latest topic summary without reading all instance logs by default

#### Scenario: Evidence read escalates only when summary is insufficient
- **WHEN** a summary does not explain a conflict or missing conclusion
- **THEN** the system MAY read the relevant instance logs or detailed topic history on demand

---

### Requirement: System MUST avoid eager global loading of unrelated experience
The system MUST NOT eagerly inject all topic files, all role experiences, or all instance logs into every prompt by default.

This guardrail MUST apply even when the workspace contains many historical topics or many role definitions.

#### Scenario: Unrelated role experience stays out of a worker prompt
- **WHEN** a `spec_reviewer` hat starts work on a specification topic
- **THEN** the system MUST NOT automatically inject unrelated role experience such as `cab_program_lead` guidance

#### Scenario: Historical topics do not flood initial context
- **WHEN** a new topic starts in a workspace containing many archived or inactive topic files
- **THEN** the system MUST avoid injecting those unrelated topic files into the initial prompt by default

