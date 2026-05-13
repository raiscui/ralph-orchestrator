# experience-scopes Specification

## Purpose
TBD - created by archiving change scoped-experience-system. Update Purpose after archive.
## Requirements
### Requirement: Ralph MUST distinguish runtime, instance, topic, role, and project knowledge scopes
The system MUST model persistent and semi-persistent knowledge as five distinct scopes:

- runtime work graph in `.agent/tasks.jsonl`
- instance context in `.ralph/log/<instance_id>/...`
- topic shared context in `task_plan__topic.md`, `notes__topic.md`, and `WORKLOG__topic.md`
- role experience in `.ralph/roles/<hat_id>/experience.md`
- project experience in project-root `experience.md`

The system MUST NOT treat these five scopes as interchangeable stores.

#### Scenario: Runtime and long-term scopes remain distinct
- **WHEN** Ralph evaluates whether a run is complete
- **THEN** it MUST use the runtime work graph instead of consulting role or project experience files

#### Scenario: Project experience is stored separately from topic files
- **WHEN** the system persists a cross-topic stable rule
- **THEN** it MUST store that rule in project-root `experience.md` rather than only in a topic shared file

---

### Requirement: Instance context MUST remain separate from role experience
The system MUST keep instance raw trajectory files separate from role-level stable experience.

Instance context MUST preserve execution-local notes and evidence, while `.ralph/roles/<hat_id>/experience.md` MUST preserve only role-level reusable knowledge.

#### Scenario: Parallel instances do not co-mingle raw logs in role experience
- **WHEN** two instances of the same hat execute in parallel
- **THEN** each instance MUST write its raw trajectory to its own instance context
- **THEN** the system MUST NOT merge those raw trajectory files directly into `.ralph/roles/<hat_id>/experience.md`

#### Scenario: Role experience stores reusable rules instead of one-off task details
- **WHEN** a hat discovers a rule that applies to future tasks for that same role
- **THEN** the system MAY persist that rule to `.ralph/roles/<hat_id>/experience.md`
- **THEN** the system MUST NOT persist one-off task state there as if it were reusable role knowledge

---

### Requirement: Role experience and project experience MUST share one entry structure
The system MUST use one consistent entry shape for role experience and project experience.

At minimum, each persisted experience entry MUST be able to represent:

- identity
- summary
- scope
- source topics
- source hats
- status
- confidence
- timestamps
- supersession linkage

#### Scenario: Same parser can read role and project experience
- **WHEN** the system loads `.ralph/roles/spec_reviewer/experience.md` and project-root `experience.md`
- **THEN** it MUST be able to parse both files using one shared entry protocol

#### Scenario: Promotion does not require entry shape conversion
- **WHEN** a role experience is promoted to project experience
- **THEN** the system MUST preserve the same entry shape instead of translating it to a different format

