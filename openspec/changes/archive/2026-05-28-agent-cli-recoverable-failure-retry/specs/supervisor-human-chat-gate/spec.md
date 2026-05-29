## ADDED Requirements

### Requirement: Chat supports explicit recoverable continue control
Supervisor human input MUST support an explicit continue control for retrying a paused recoverable agent CLI failure.

The command surface MUST support targeting a specific recoverable `failure_id`, and MUST support a default target selected from the current paused recoverable failure or selected instance when unambiguous. A localized UI label such as `继续` MAY submit this explicit control action, but ordinary free-form chat text MUST NOT implicitly trigger a retry.

#### Scenario: Continue command targets a failure id
- **GIVEN** `.ralph/recoverable-failures.jsonl` contains a paused recoverable failure with `failure_id="failure-123"`
- **WHEN** the human submits `!continue failure-123` through Supervisor chat
- **THEN** the system MUST treat the input as a recoverable retry control action
- **THEN** the system MUST append a `continued_by_human` transition for `failure-123`

#### Scenario: Continue command can use selected paused failure
- **GIVEN** the Supervisor has a selected instance with exactly one paused recoverable failure
- **WHEN** the human submits `!continue` through Supervisor chat
- **THEN** the system MUST resolve the command to that paused recoverable failure
- **THEN** the system MUST enqueue a retry through the recoverable failure scheduler path

### Requirement: Ordinary chat does not implicitly retry failures
Supervisor human input MUST NOT treat ordinary chat messages as recoverable retry controls unless they use the explicit continue command or an equivalent structured UI action.

This requirement prevents ambiguous text such as `继续分析` from accidentally restarting an agent CLI process with side effects.

#### Scenario: Plain continue text remains chat
- **GIVEN** a selected instance has a paused recoverable failure
- **WHEN** the human submits plain chat text `继续分析这个问题`
- **THEN** the system MUST write an ordinary `human.message` event according to the existing chat rules
- **THEN** the system MUST NOT append a `continued_by_human` ledger transition
- **THEN** the system MUST NOT enqueue a retry solely because of that plain text

### Requirement: Continue control is auditable in human-facing evidence
Supervisor continue control MUST be visible as an auditable control transition rather than hidden executor behavior.

When a continue command is accepted, the system MUST expose enough evidence for record-session summaries, agents snapshots, or reports to show which failure was continued and which instance/job was retried.

#### Scenario: Accepted continue appears in evidence
- **WHEN** the human continue control is accepted for a recoverable failure
- **THEN** the recoverable failure ledger MUST include the `continued_by_human` transition
- **THEN** human-facing evidence MUST be able to identify the affected `failure_id`, `instance_id`, and retry attempt
