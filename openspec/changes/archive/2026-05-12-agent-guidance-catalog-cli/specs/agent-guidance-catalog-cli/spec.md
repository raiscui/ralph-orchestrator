## ADDED Requirements

### Requirement: Agent guidance manifest MUST catalog project-owned skills
Ralph MUST catalog project-owned skills in the agent guidance manifest.

The catalog MUST support `skill` assets whose paths point to repository-relative `SKILL.md` files under approved project skill roots.

#### Scenario: Project-owned skill is listed in the manifest
- **WHEN** a project-owned skill is intended to be part of Ralph's maintained guidance surface
- **THEN** the manifest MUST include a `skill` asset entry with stable id, repository-relative path, lifecycle status, summary, and index requirement

#### Scenario: Skill path outside approved roots fails
- **WHEN** a manifest `skill` asset points outside `.agents/skills/*/SKILL.md` or `.codex/skills/*/SKILL.md`
- **THEN** verification MUST fail and identify the offending skill asset

---

### Requirement: Agent guidance verifier MUST validate skill frontmatter
Ralph MUST validate required metadata for manifest `skill` assets.

A non-archived skill asset MUST have YAML-style frontmatter with non-empty `name` and `description` fields.

#### Scenario: Skill frontmatter is complete
- **WHEN** a non-archived skill asset has frontmatter containing non-empty `name` and `description`
- **THEN** verification MUST accept the skill metadata

#### Scenario: Skill frontmatter name is missing
- **WHEN** a non-archived skill asset omits the `name` field or leaves it empty
- **THEN** verification MUST fail and identify the skill path

#### Scenario: Skill frontmatter description is missing
- **WHEN** a non-archived skill asset omits the `description` field or leaves it empty
- **THEN** verification MUST fail and identify the skill path

---

### Requirement: Agent guidance verifier MUST reject duplicate skill identifiers
Ralph MUST prevent duplicate skill identities inside the guidance manifest.

The verifier MUST reject duplicate manifest asset ids and duplicate parsed skill names among active or draft skill assets.

#### Scenario: Duplicate manifest id fails
- **WHEN** two manifest assets use the same `id`
- **THEN** verification MUST fail and identify the duplicate id

#### Scenario: Duplicate skill name fails
- **WHEN** two non-archived skill assets parse to the same frontmatter `name`
- **THEN** verification MUST fail and identify the duplicate skill name

---

### Requirement: Agent guidance verifier CLI MUST provide standalone validation
Ralph MUST provide a standalone CLI entry point for agent guidance verification.

The CLI MUST run the same verifier as the repository test gate, print a concise human-readable summary on success or failure, and return a non-zero exit code on verification failure.

#### Scenario: CLI verification succeeds
- **WHEN** a maintainer runs the guidance verifier CLI in a valid repository
- **THEN** the command MUST exit successfully and report the verified manifest path and checked asset counts

#### Scenario: CLI verification fails
- **WHEN** the manifest has invalid guidance assets
- **THEN** the command MUST exit with a non-zero status and report the verifier error without requiring a full `cargo test` run
