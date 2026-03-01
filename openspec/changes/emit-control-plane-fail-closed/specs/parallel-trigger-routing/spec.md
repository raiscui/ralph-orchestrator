## ADDED Requirements

### Requirement: External control-plane turn actions require explicit targeting
When ingesting external events, the system MUST reject any event carrying `turn_action=steer|interrupt` unless it explicitly targets `target_instance="ralph#1"`.

#### Scenario: Missing target_instance is rejected
- **WHEN** the Supervisor ingests an external JSONL event with `turn_action="steer"` and no `target_instance`
- **THEN** the system MUST reject the event
- **THEN** the system MUST NOT deliver the event to any hat instance

#### Scenario: Non-ralph target_instance is rejected
- **WHEN** the Supervisor ingests an external JSONL event with `turn_action="interrupt"` and `target_instance="writer#1"`
- **THEN** the system MUST reject the event
- **THEN** the system MUST NOT deliver the event to `writer#1`

#### Scenario: Turn-action events are not redirected to secondary ralph
- **GIVEN** `ralph#1` is `Running` and a secondary `ralph#2` exists
- **WHEN** the Supervisor ingests an external JSONL event with `turn_action="steer"` and `target_instance="ralph#1"`
- **THEN** the system MUST deliver the event to `ralph#1` (not rewritten to `ralph#2`)
