## ADDED Requirements

### Requirement: Record summary MUST expose dynamic spawn correlation
`ralph record summary` MUST expose dynamic spawn correlation when a record-session contains parent-visible dynamic spawn events.

The summary MUST include `topology.spawn_group` count, `topology.spawn.result` count, `topology.spawn.failed` count, spawned instance ids, source instances for result topics, and final termination state.

#### Scenario: summary shows spawned instances and result coverage
- **WHEN** a record-session contains `topology.spawn.result` for `builder#2` through `builder#6` and matching `analysis.done` events
- **THEN** `ralph record summary` MUST show the spawned instance ids or enough source-instance coverage to verify the dynamic run
- **AND** it MUST show `analysis.done` source instances without requiring manual JSONL scanning

#### Scenario: summary distinguishes spawn success from workflow completion
- **WHEN** a record-session has `topology.spawn.result` but lacks `_meta.termination`
- **THEN** `ralph record summary` MUST make the missing termination visible
- **AND** it MUST NOT imply that spawn success alone means the workflow completed

### Requirement: Record summary MUST distinguish semantic completion from wrapper exit status
`ralph record summary` MUST treat record-session `_meta.termination` as the primary semantic completion signal for a recorded run.

Wrapper shell status, stdout tails, and TUI display state MAY be useful diagnostics, but they MUST NOT override a parseable record-session termination reason.

#### Scenario: wrapper script fails after record-session completion
- **WHEN** an outer shell wrapper fails after the Ralph run writes `_meta.termination.reason = CompletionPromise`
- **THEN** the summary MUST still report the semantic termination reason from the record-session
- **AND** a reviewer MUST be able to separate wrapper failure from runtime semantic failure

### Requirement: Record summary with agents file MUST distinguish current registry and completed dynamics
`ralph record summary --agents-file` MUST distinguish currently registered instances from completed dynamic tombstones when the agents sidecar contains both.

The summary MUST not present current registry snapshots as the complete history of dynamic instances unless completed tombstones or record-session spawn/result evidence are also consulted.

#### Scenario: completed dynamic instances are displayed separately
- **WHEN** dynamic instances have completed and been reaped before summary time
- **THEN** the summary MUST show completed dynamic instances separately from current registry instances
- **AND** the summary MUST still allow source-instance result coverage to be verified
