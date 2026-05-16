## ADDED Requirements

### Requirement: Answer evidence inspect UX MUST locate answer-return evidence by correlation id
Ralph MUST provide a CLI inspect UX that locates explicit answer-return evidence-index entries by request id or answer id.

The inspect UX MUST read the existing `.ralph/evidence-index.jsonl` file and MUST NOT create a second evidence store.

#### Scenario: request-id answer evidence is inspectable
- **GIVEN** a request id has answer-return evidence entries
- **WHEN** the operator runs the answer inspect UX with that request id
- **THEN** the command MUST return the evidence-index entries for that correlation id
- **AND** the returned entries MUST include the durable artifact paths already recorded in `.ralph/evidence-index.jsonl`

#### Scenario: answer-id evidence is inspectable
- **GIVEN** an answer event id has answer-return evidence entries
- **WHEN** the operator runs the answer inspect UX with that answer id
- **THEN** the command MUST return the evidence-index entries for that correlation id
- **AND** the command MUST NOT require runtime graph artifacts to succeed

### Requirement: Answer evidence inspect UX MUST preserve missing vs no-entry semantics
Ralph MUST preserve the difference between explicit missing markers and no evidence entry when inspecting answer-return evidence.

#### Scenario: explicit missing evidence marker remains visible
- **GIVEN** `.ralph/evidence-index.jsonl` contains explicit missing answer evidence for a request id
- **WHEN** the operator runs the answer inspect UX with that request id
- **THEN** the command MUST succeed
- **AND** it MUST report lookup status `missing`
- **AND** it MUST include the missing marker entries

#### Scenario: unknown correlation id fails visibly
- **GIVEN** `.ralph/evidence-index.jsonl` has no matching entry for a correlation id
- **WHEN** the operator runs the answer inspect UX with that correlation id
- **THEN** the command MUST fail with a non-zero exit code
- **AND** the error MUST identify the missing correlation id and evidence index path
