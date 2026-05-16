## ADDED Requirements

### Requirement: Startup bootstrap MUST have a repeatable live integration gate
Ralph MUST maintain a repeatable repository integration gate that exercises the real no-config/no-prompt startup bootstrap path through `ralph run`.

This gate MUST validate runtime artifacts from one repeatable repository-owned test flow rather than relying only on separate unit tests or manually collected `/tmp` dogfood evidence.

#### Scenario: Live gate proves bootstrap artifacts and parallel resolved config
- **GIVEN** a temporary workspace that contains no `ralph.yml`
- **AND** the workspace contains no `PROMPT.md`
- **WHEN** the repository live bootstrap integration gate performs its startup bootstrap flow
- **THEN** the bootstrap run MUST succeed
- **AND** `.ralph/bootstrap-selection.json` MUST be written
- **AND** `.ralph/resolved-config.yml` MUST be written
- **AND** the resolved config artifact MUST show `parallel.enabled=true`
