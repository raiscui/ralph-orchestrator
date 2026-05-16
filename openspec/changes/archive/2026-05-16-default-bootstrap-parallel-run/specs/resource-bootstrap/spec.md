## ADDED Requirements

### Requirement: Default startup bootstrap MUST resolve to parallel mode
Ralph MUST resolve the default no-config/no-prompt startup bootstrap configuration with `parallel.enabled=true`.

This requirement applies only to implicit default startup bootstrap selection. Explicit configuration sources MUST continue to bypass the bootstrap selector.

#### Scenario: no-config startup writes a parallel resolved config
- **GIVEN** the current workspace contains no `ralph.yml`
- **AND** the current workspace contains no `PROMPT.md`
- **WHEN** the user runs `ralph run` without explicit config or prompt input
- **THEN** Ralph MUST enter startup resource bootstrap
- **AND** the resolved config artifact MUST include `parallel.enabled=true`
- **AND** the real run MUST use that resolved parallel configuration

#### Scenario: explicit config still bypasses bootstrap
- **GIVEN** the current workspace contains no `ralph.yml`
- **WHEN** the user runs `ralph run --config ralph.yml`
- **THEN** Ralph MUST NOT treat the missing explicit config as implicit default bootstrap selection
- **AND** Ralph MUST NOT write default bootstrap selection artifacts for that explicit config source
