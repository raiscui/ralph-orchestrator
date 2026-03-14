## ADDED Requirements

### Requirement: External turn_action steer/interrupt are reserved for ExternalInput to ralph#1
For out-of-band external events (JSONL ingest via `ralph emit` or Supervisor TUI), the system MUST treat `turn_action=steer|interrupt` as a control-plane signal reserved for ExternalInput and deliverable only to `ralph#1`.

#### Scenario: Hat job cannot emit steer/interrupt via ralph emit
- **GIVEN** a headless hat job environment where `RALPH_HAT_INSTANCE_ID` is set
- **WHEN** the job runs `ralph emit human.message "..." --turn-action steer --target-instance ralph#1`
- **THEN** the `ralph emit` command MUST exit non-zero
- **THEN** the external events file MUST NOT contain a new event line with `turn_action="steer"`

#### Scenario: Valid control-plane event is delivered only to ralph#1
- **WHEN** the Supervisor ingests an external JSONL event with `turn_action="steer"` and `target_instance="ralph#1"`
- **THEN** the system MUST deliver the event to `ralph#1`
- **THEN** the system MUST NOT deliver the event to any non-`ralph#1` instance
