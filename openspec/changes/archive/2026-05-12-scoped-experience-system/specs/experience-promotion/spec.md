## ADDED Requirements

### Requirement: Experience promotion MUST follow a topic-first, scope-narrow default path
The system MUST treat topic shared context as the default first home for newly discovered candidate knowledge.

If the correct long-term scope is uncertain, the system MUST choose the narrower reusable scope before choosing a broader one.

#### Scenario: Uncertain reusable rule promotes to role before project
- **WHEN** the system determines that a discovered rule is reusable but may only apply to one role
- **THEN** it MUST promote that rule to role experience rather than directly to project experience

#### Scenario: Topic retains rule until broader reuse is demonstrated
- **WHEN** the system cannot yet show that a rule is reusable beyond the current topic
- **THEN** it MUST keep that rule in topic shared context instead of promoting it immediately

---

### Requirement: Topic findings MUST promote to role experience only when role-specific reuse is justified
The system MUST promote a topic-derived finding to `.ralph/roles/<hat_id>/experience.md` only when that finding represents stable, reusable guidance for one role.

Role promotion MUST NOT be used for one-off task details or topic-local state.

#### Scenario: Stable role rule becomes role experience
- **WHEN** `cab_program_lead` repeatedly requires host, agenda, and logistics readiness before final confirmation
- **THEN** the system MUST allow that rule to be promoted from topic context to `.ralph/roles/cab_program_lead/experience.md`

#### Scenario: Topic-local detail does not become role experience
- **WHEN** a topic records a cohort choice that only applies to one customer advisory board run
- **THEN** the system MUST keep that detail in topic context instead of promoting it to role experience

---

### Requirement: Project experience MUST require cross-role or pre-routing value
The system MUST promote a rule to project-root `experience.md` only when that rule has project-wide value.

At minimum, the system MUST justify project-scope promotion by one or more of:

- demonstrated cross-role reuse
- `ralph#1` needing the rule before workflow or hat selection
- the rule being a project-level collaboration constraint

#### Scenario: Cross-role coordination rule becomes project experience
- **WHEN** the system confirms that only a canonical writer may update shared topic files across multiple workflows
- **THEN** it MUST allow that rule to be promoted to project-root `experience.md`

#### Scenario: Single-role technique does not jump directly to project scope
- **WHEN** a rule only improves one role's internal review flow
- **THEN** the system MUST NOT promote that rule directly to project-root `experience.md`

---

### Requirement: Demotion MUST preserve auditability instead of hard-deleting prior knowledge
The system MUST preserve an audit trail when a previously promoted experience is no longer valid at its current scope.

At minimum, demotion MUST support marking an entry as deprecated and linking it to the narrower replacement or source context.

#### Scenario: Project rule demotes to role-specific guidance
- **WHEN** the system learns that a project experience actually only applies to one role
- **THEN** it MUST mark the project entry as deprecated
- **THEN** it MUST preserve a link to the corresponding role experience or replacement entry

#### Scenario: Role rule demotes back to topic-local history
- **WHEN** the system learns that a role experience was only a temporary topic workaround
- **THEN** it MUST mark the role entry as deprecated instead of physically deleting it
- **THEN** it MUST preserve its source-topic traceability
