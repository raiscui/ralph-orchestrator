## ADDED Requirements

### Requirement: Capability invocation inspect UX MUST locate evidence by invocation id
Ralph MUST provide a CLI inspect UX that locates capability invocation evidence-index entries by invocation id.

The inspect UX MUST read the existing `.ralph/evidence-index.jsonl` file and MUST NOT create a second evidence store.

#### Scenario: successful invocation evidence is inspectable

- **GIVEN** `ralph tools capability invoke --json` has produced an invocation id
- **WHEN** the operator runs the inspect UX with that invocation id
- **THEN** the command MUST return the evidence-index entries for that invocation id
- **AND** the returned entries MUST include the durable artifact paths recorded in `.ralph/evidence-index.jsonl`

#### Scenario: inspect supports machine-readable output

- **GIVEN** an invocation id has evidence-index entries
- **WHEN** the operator runs the inspect UX with `--json`
- **THEN** the command MUST emit valid JSON
- **AND** the JSON MUST include the invocation id, lookup status, and evidence entries

#### Scenario: inspect supports human-readable output

- **GIVEN** an invocation id has evidence-index entries
- **WHEN** the operator runs the inspect UX without `--json`
- **THEN** the command MUST print a concise human-readable summary of artifact kinds, paths, producers, and statuses

### Requirement: Capability invocation inspect UX MUST fail visibly for unknown invocation ids
Ralph MUST return a non-zero command result when the inspect UX cannot find any evidence-index entry for the requested invocation id.

The error message MUST identify the missing invocation id and the evidence index path used for lookup.

#### Scenario: unknown invocation id is not treated as success

- **GIVEN** `.ralph/evidence-index.jsonl` exists
- **WHEN** the operator inspects an invocation id that has no matching evidence entry
- **THEN** the command MUST fail
- **AND** the operator MUST receive a clear no-entry message

#### Scenario: explicit missing evidence markers remain visible

- **GIVEN** `.ralph/evidence-index.jsonl` contains explicit missing evidence markers for an invocation id
- **WHEN** the operator inspects that invocation id
- **THEN** the command MUST report the lookup status as missing
- **AND** the command MUST include the missing marker entries instead of hiding them
