# canonical-writer Specification

## Purpose
TBD - created by archiving change scoped-experience-system. Update Purpose after archive.
## Requirements
### Requirement: Topic shared context MUST have exactly one canonical writer at a time
The system MUST designate exactly one topic canonical writer for each active topic shared context.

Only the active topic canonical writer MAY update `task_plan__topic.md`, `notes__topic.md`, and `WORKLOG__topic.md`.
Other hats MUST contribute via instance context or event evidence rather than writing topic shared files directly.

#### Scenario: Non-owner hat cannot directly write topic shared files
- **WHEN** a non-owner hat contributes findings for an active topic
- **THEN** the system MUST route those findings through instance context or event evidence
- **THEN** the system MUST NOT treat that hat as an authorized writer of topic shared files

#### Scenario: Topic owner writes the official shared conclusion
- **WHEN** a workflow owner or finalizer hat is active for a topic
- **THEN** the system MUST treat that hat as the topic canonical writer unless a more specific override is configured

---

### Requirement: Role experience MUST have one canonical writer per role
The system MUST designate one role canonical writer for each role experience file.

Only the role canonical writer MAY update `.ralph/roles/<hat_id>/experience.md`.
Other instances of the same role MUST contribute evidence through their own instance context instead of modifying the role experience directly.

#### Scenario: Parallel role instances contribute evidence without dual-writing role experience
- **WHEN** multiple instances of `spec_reviewer` run in parallel
- **THEN** each instance MUST write its evidence to its own instance context
- **THEN** only the active role canonical writer MAY update `.ralph/roles/spec_reviewer/experience.md`

#### Scenario: Ralph can temporarily own role experience when no primary owner exists
- **WHEN** a role has no configured primary owner
- **THEN** the system MUST allow `ralph#1` to act as the temporary role canonical writer

---

### Requirement: Project experience MUST default to `ralph#1` as canonical writer
The system MUST treat `ralph#1` as the default canonical writer for project-root `experience.md`.

Ordinary hats MAY produce candidate evidence or promotion suggestions, but they MUST NOT directly update project-root `experience.md` under the default policy.

#### Scenario: Ordinary hat proposes but does not directly write project experience
- **WHEN** a worker hat discovers a possible cross-role rule
- **THEN** it MAY emit candidate evidence or a promotion suggestion
- **THEN** it MUST NOT directly append that rule to project-root `experience.md`

#### Scenario: Ralph writes the active project-level rule
- **WHEN** `ralph#1` validates that a rule belongs at project scope
- **THEN** `ralph#1` MUST be able to persist that rule to project-root `experience.md`

---

### Requirement: Canonical writer handoff MUST leave a resumable summary trail
The system MUST require a handoff summary before ownership of topic or role shared knowledge transfers from one canonical writer to another.

At minimum, the handoff summary MUST capture:

- current conclusion
- unfinished work
- relevant evidence sources
- reason for ownership transfer

#### Scenario: Topic writer transfer includes explicit handoff summary
- **WHEN** topic canonical writer ownership moves from `ralph#1` to a workflow owner
- **THEN** the previous writer MUST append a handoff summary before the new writer continues the topic shared files

#### Scenario: New writer resumes from handoff instead of re-deriving all state
- **WHEN** a replacement canonical writer takes over a topic or role file
- **THEN** it MUST read the most recent handoff summary before continuing writes

