# adapter-contract-tests Specification

## Purpose
TBD - created by archiving change adapter-contract-tests. Update Purpose after archive.
## Requirements
### Requirement: Adapter streams MUST keep stdout as the default event parsing input
Adapter streams MUST treat stdout as the default semantic event parsing input and stderr as diagnostics-only input.

#### Scenario: stderr event text is diagnostics only
- **WHEN** a backend writes `<event topic="build.done">...</event>` to stderr
- **THEN** Ralph MUST NOT route that stderr text as a business event by default
- **AND** the stderr bytes MAY still be recorded as diagnostic evidence

#### Scenario: stdout event text is parsed
- **WHEN** a backend writes `<event topic="build.done">...</event>` to stdout
- **THEN** Ralph MUST parse and route that event according to the normal event contract

---

### Requirement: Prompt transport modes MUST be adapter-contract visible
Adapter execution MUST preserve the selected prompt transport mode so `stdin` mode writes prompt text to child stdin without appending it as an argv tail.

#### Scenario: stdin mode does not append prompt argv
- **WHEN** a custom backend is configured with `prompt_mode=stdin`
- **THEN** the spawned backend argv MUST NOT include the prompt as a trailing argument
- **AND** the prompt MUST be available through stdin

---

### Requirement: Adapter evidence MUST preserve event and stream attribution
Adapter evidence MUST preserve stable event and stream attribution fields that are needed for replay and diagnostics.

#### Scenario: event records preserve id and reply
- **WHEN** an event has `id` and `reply`
- **THEN** event logging MUST preserve both fields in the JSONL record

#### Scenario: terminal writes preserve instance id
- **WHEN** a parallel backend stream belongs to a hat instance
- **THEN** recorded `ux.terminal.write` evidence MUST include that `instance_id`

---

### Requirement: Record-session critical records MUST be strict-parseable after flush
Record-session critical evidence MUST be written as strict JSONL records that can be parsed after the writer flushes or terminates.

#### Scenario: critical record sequence is parseable
- **WHEN** a record-session file contains `_meta.session_start`, `_meta.loop_start`, `ux.terminal.write`, `bus.publish`, and `_meta.termination`
- **THEN** `ralph record summary` / strict session loading MUST parse the file without line-level JSON errors
