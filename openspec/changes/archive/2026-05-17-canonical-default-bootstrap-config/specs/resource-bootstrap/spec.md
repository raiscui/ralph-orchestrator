## MODIFIED Requirements

### Requirement: Default startup bootstrap MUST resolve to parallel mode
Ralph MUST resolve the default no-config/no-prompt startup bootstrap configuration from a single canonical default bootstrap resource whose core runtime behavior matches the repository-maintained default parallel configuration for the user-visible bootstrap runtime fields.

This requirement applies only to implicit default startup bootstrap selection. Explicit configuration sources MUST continue to bypass the bootstrap selector.

The resolved bootstrap artifact does not need to be byte-for-byte identical to the repository `ralph.yml`, but it MUST match the canonical default configuration for the user-visible runtime fields that define backend execution and parallel startup behavior.

#### Scenario: no-config startup writes canonical default bootstrap config
- **GIVEN** the current workspace contains no `ralph.yml`
- **AND** the current workspace contains no `PROMPT.md`
- **WHEN** the user runs `ralph run` without explicit config or prompt input
- **THEN** Ralph MUST enter startup resource bootstrap
- **AND** `.ralph/resolved-config.yml` MUST match the canonical startup resource for `cli.backend`
- **AND** `.ralph/resolved-config.yml` MUST match the canonical startup resource for `cli.command`
- **AND** `.ralph/resolved-config.yml` MUST match the canonical startup resource for `cli.prompt_mode`
- **AND** `.ralph/resolved-config.yml` MUST match the canonical startup resource for `cli.args`
- **AND** `.ralph/resolved-config.yml` MUST match the canonical startup resource for `parallel.enabled`
- **AND** the real run MUST use that resolved canonical default bootstrap configuration

#### Scenario: explicit config still bypasses canonical bootstrap selection
- **GIVEN** the current workspace contains no `ralph.yml`
- **WHEN** the user runs `ralph run --config ralph.yml`
- **THEN** Ralph MUST NOT treat the missing explicit config as implicit canonical bootstrap selection
- **AND** Ralph MUST NOT write default bootstrap selection artifacts for that explicit config source

## ADDED Requirements

### Requirement: Startup bootstrap MUST keep one canonical source for default resource semantics
Ralph MUST keep a single canonical startup resource as the source of truth for implicit default bootstrap behavior, and the repository-maintained default `ralph.yml` MUST remain semantically synchronized with that canonical startup resource for the user-visible bootstrap runtime fields.

#### Scenario: selector default workflow matches canonical startup resource
- **WHEN** Ralph resolves the implicit default startup bootstrap workflow
- **THEN** the selector MUST choose the canonical default bootstrap workflow resource instead of the legacy `workflow:feature-minimal` resource

#### Scenario: repository default config stays semantically synchronized
- **WHEN** the repository updates its maintained default `ralph.yml`
- **THEN** the canonical startup bootstrap resource MUST be updated so that the user-visible bootstrap runtime fields remain semantically aligned

#### Scenario: repo-owned drift gate compares canonical sources
- **WHEN** the repository verification gate compares the maintained default `ralph.yml` with the canonical embedded startup resource
- **THEN** the gate MUST fail if the user-visible bootstrap runtime fields drift apart
