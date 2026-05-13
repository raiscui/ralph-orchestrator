# state-cli-adapter Specification

## Purpose
Defines the CLI adapter contract for inspecting and clearing Ralph runtime workflow state through `StateOperationStore`. This specification keeps `ralph state` commands focused on status/read/clear behavior, JSON output, scope handling, and the boundary that CLI code must not duplicate core state file semantics.

## Requirements
### Requirement: Ralph MUST expose state operation status through CLI
Ralph MUST expose a `ralph state status` CLI command that reads runtime workflow state summaries through the core state operation layer.

The command MUST support all-mode status and single-mode status. It MUST support session-scoped status with `--session-id`. It MUST provide a human-readable default output and a machine-readable `--json` output.

#### Scenario: User requests all state statuses
- **WHEN** a user runs `ralph state status --json`
- **THEN** Ralph MUST call the core state operation status API for all supported modes
- **AND** the JSON output MUST include one status entry per supported mode

#### Scenario: User requests one state status
- **WHEN** a user runs `ralph state status --mode team`
- **THEN** Ralph MUST report only the `team` state summary
- **AND** it MUST NOT directly parse `.ralph/state/team-state.json` in CLI code

---

### Requirement: Ralph MUST expose state operation reads through CLI
Ralph MUST expose a `ralph state read <mode>` CLI command that reads runtime workflow state through the core state operation layer.

The command MUST support `--session-id` and `--json`. Missing state MUST be treated as a valid empty result rather than an execution failure. Malformed state JSON MUST surface as a non-zero CLI failure.

#### Scenario: State exists
- **WHEN** a user runs `ralph state read team --json`
- **AND** the `team` state exists
- **THEN** Ralph MUST print JSON containing `exists: true`
- **AND** it MUST include the state record returned by the core operation layer

#### Scenario: State is missing
- **WHEN** a user runs `ralph state read team --json`
- **AND** no `team` state exists
- **THEN** Ralph MUST print JSON containing `exists: false`
- **AND** the command MUST exit successfully

#### Scenario: State file is malformed
- **WHEN** a user runs `ralph state read team`
- **AND** the resolved state file contains malformed JSON
- **THEN** Ralph MUST return a non-zero exit code
- **AND** the diagnostic MUST identify that state reading failed

---

### Requirement: Ralph MUST expose state operation clear through CLI
Ralph MUST expose a `ralph state clear <mode>` CLI command that clears runtime workflow state through the core state operation layer.

The command MUST support `--session-id` for session-scoped clear and `--all-sessions` for global plus all session scopes clear. The command MUST reject `--session-id` together with `--all-sessions` because those scopes conflict.

#### Scenario: User clears one mode state
- **WHEN** a user runs `ralph state clear team`
- **THEN** Ralph MUST call the core state clear API for `team`
- **AND** it MUST report the deleted paths returned by the core operation layer

#### Scenario: User clears all sessions for one mode
- **WHEN** a user runs `ralph state clear team --all-sessions`
- **THEN** Ralph MUST clear global and session-scoped `team` state through the core operation layer
- **AND** it MUST report all deleted paths returned by the core operation layer

#### Scenario: User requests conflicting scopes
- **WHEN** a user runs `ralph state clear team --session-id s1 --all-sessions`
- **THEN** Ralph MUST reject the command before invoking the core clear API
- **AND** it MUST return a non-zero exit code

---

### Requirement: Ralph CLI MUST NOT duplicate state file semantics
Ralph CLI MUST use `StateOperationStore` as the single state operation boundary for `ralph state` commands.

The CLI MUST NOT construct state file paths manually outside user-facing display formatting. It MUST NOT read, merge, write, or clear state JSON directly. File format, scope precedence, malformed JSON behavior, and clear semantics MUST remain owned by `ralph-core`.

#### Scenario: Maintainer reviews state CLI code
- **WHEN** a maintainer inspects the implementation of `ralph state status`, `ralph state read`, or `ralph state clear`
- **THEN** the command handlers MUST call `StateOperationStore`
- **AND** they MUST NOT reimplement state JSON parsing or path resolution logic

---

### Requirement: Ralph state CLI v1 MUST avoid manual state writes
Ralph state CLI v1 MUST NOT expose a user-facing `state write` command.

Runtime lifecycle state writes should come from runtime adapters or future explicitly scoped changes. The first CLI adapter MUST focus on inspection and cleanup to avoid creating a manual mutation surface before runtime ownership is defined.

#### Scenario: User checks state CLI help
- **WHEN** a user runs `ralph state --help`
- **THEN** the listed subcommands MUST include status, read, and clear
- **AND** they MUST NOT include write

