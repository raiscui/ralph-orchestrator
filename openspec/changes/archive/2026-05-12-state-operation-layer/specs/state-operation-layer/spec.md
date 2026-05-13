## ADDED Requirements

### Requirement: Ralph MUST define a unified runtime state operation layer
Ralph MUST define one unified operation layer for runtime workflow state reads, writes, clears, active-mode listing, and status summaries.

The operation layer MUST include the operations `state_read`, `state_write`, `state_clear`, `state_list_active`, and `state_get_status`. Runtime workflow state adapters, including future CLI or MCP adapters, MUST use this layer instead of reading or writing state JSON directly.

#### Scenario: Future adapter reads workflow state
- **WHEN** a future CLI, MCP, or runtime adapter needs workflow state
- **THEN** it MUST call the state operation layer
- **AND** it MUST NOT parse or write the state file directly

#### Scenario: Standard operations are discoverable
- **WHEN** maintainers inspect the state operation contract
- **THEN** the contract MUST name `state_read`, `state_write`, `state_clear`, `state_list_active`, and `state_get_status`

---

### Requirement: State writes MUST be atomic and serialized per path
Ralph MUST write runtime workflow state atomically and serialize writes that target the same resolved path.

A complete state write MUST write JSON to a temporary file and then rename it into the resolved state path. Concurrent writes to the same path MUST be queued or otherwise serialized so two writers cannot interleave bytes in the same state file.

#### Scenario: State write replaces a file atomically
- **WHEN** `state_write` updates a runtime workflow state file
- **THEN** it MUST write a complete JSON document through a temporary file and rename
- **AND** readers MUST NOT observe a partially written target file

#### Scenario: Concurrent writes target one state path
- **WHEN** two `state_write` calls resolve to the same path at the same time
- **THEN** Ralph MUST serialize those writes for that path
- **AND** the final target file MUST remain valid JSON

---

### Requirement: Runtime state MUST use explicit mode and scope boundaries
Ralph MUST validate runtime state mode and scope before reading or writing state.

The v1 contract MUST support only the modes `ralph`, `ralplan`, `team`, `deep-interview`, and `capability-invocation`. State paths MUST distinguish global scope from session scope. A session-scoped read SHOULD prefer the session state and MAY fall back to global state when the session state does not exist.

#### Scenario: Unsupported mode is rejected
- **WHEN** a caller requests state for an unsupported mode
- **THEN** the operation layer MUST return a structured error
- **AND** it MUST NOT create or modify a state file

#### Scenario: Session scope is explicit
- **WHEN** a caller writes state with a `session_id`
- **THEN** the state MUST be written under a session-scoped path
- **AND** it MUST NOT overwrite the global state path

---

### Requirement: State records MUST expose stable lifecycle fields
Ralph MUST define stable lifecycle fields for runtime workflow state records.

A state record MUST support `mode`, `active`, `current_phase`, `updated_at`, `run_outcome`, `lifecycle_outcome`, `session_id`, and `state`. The `state` field MAY hold mode-specific custom data, but standard fields MUST remain top-level so status summaries can be produced without mode-specific parsing.

#### Scenario: Status summary reads standard fields
- **WHEN** `state_get_status` summarizes a state file
- **THEN** it MUST report `active`, `current_phase`, `run_outcome`, `lifecycle_outcome`, and `path` from standard fields

#### Scenario: Custom state is preserved separately
- **WHEN** `state_write` receives a custom `state` object
- **THEN** mode-specific data MUST be stored under `state`
- **AND** it MUST NOT replace the standard lifecycle fields unless those fields are explicitly provided as standard fields

---

### Requirement: State operation layer MUST NOT replace existing Ralph evidence or agent truth sources
Ralph MUST keep runtime workflow state separate from existing memories, tasks, event logs, record-session files, and diagnostics.

The state operation layer MUST NOT replace `.agent/memories.md`, `.agent/tasks.jsonl`, `.agent/scratchpad.md`, `.ralph/events*.jsonl`, `.ralph/current-events`, `.ralph/record-session.latest`, `--record-session` JSONL, or `.ralph/diagnostics/*`. Event logs and record-session JSONL remain evidence streams, while state operation records represent current workflow lifecycle state.

#### Scenario: Events remain the evidence stream
- **WHEN** Ralph records bus events or replay evidence
- **THEN** it MUST continue using the event logger or record-session contracts
- **AND** it MUST NOT rely on state operation files as the only evidence stream

#### Scenario: Memories and tasks remain their own truth sources
- **WHEN** agents manage persistent memories or runtime task lists
- **THEN** they MUST continue using the memories/tasks contracts
- **AND** state operation files MUST NOT become the storage format for those records
