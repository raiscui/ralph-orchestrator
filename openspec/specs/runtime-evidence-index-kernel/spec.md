# runtime-evidence-index-kernel Specification

## Purpose

Define Ralph's minimal runtime evidence index kernel. The kernel links durable artifacts to stable correlation ids so tests and later runtime features can locate record-session, event-log, reply, and capability invocation evidence without replacing those original truth sources or introducing evidence CLI / doctor UX.

## Requirements
### Requirement: Evidence index kernel MUST define a minimal artifact link schema

The evidence index kernel MUST define a versioned entry schema that can link a run/session to durable artifacts using a stable correlation id.

The minimal schema MUST include session id or run id, artifact kind, artifact path, producer, correlation id, status, and optional parent-child correlation fields.

#### Scenario: index entry links an artifact to a correlation id

- **GIVEN** a run has produced a durable artifact
- **WHEN** the artifact is registered in the evidence index
- **THEN** the index entry MUST include a correlation id
- **AND** the index entry MUST include an artifact kind and artifact path
- **AND** the index entry MUST include a producer and status

#### Scenario: schema does not depend on CLI display fields

- **WHEN** the minimal evidence index schema is used by a test
- **THEN** the test MUST be able to resolve artifact links without requiring evidence summary, inspect, or doctor display fields

### Requirement: Evidence index kernel MUST register existing evidence artifacts without replacing their truth sources

The evidence index kernel MUST register links to existing evidence artifacts while keeping each original artifact stream as its own truth source.

Record-session JSONL, event log JSONL, runtime delivery/lifecycle durable records, reply events, and capability invocation artifacts MUST remain readable without the index.

#### Scenario: record-session artifact remains the source document

- **GIVEN** a record-session JSONL file exists
- **WHEN** the file is registered in the evidence index
- **THEN** the index MUST point to the record-session artifact path
- **AND** the index MUST NOT duplicate the full record-session contents as its own replacement truth source

#### Scenario: runtime delivery remains durable evidence

- **GIVEN** an event log contains a runtime delivery durable record
- **WHEN** that record is registered in the evidence index
- **THEN** the index MUST point to the event log or durable artifact
- **AND** the runtime delivery record MUST remain the replay evidence source

### Requirement: Evidence index kernel MUST support correlation lookup

The evidence index kernel MUST support lookup by correlation id so tests and later runtime features can find related artifacts without scanning every evidence stream manually.

Lookup results MUST distinguish success artifacts, failure artifacts, missing artifact markers, and no index entry.

#### Scenario: lookup finds capability invocation artifacts

- **GIVEN** a capability invocation writes `invoke.json` and `result.json`
- **WHEN** a reader looks up the invocation id
- **THEN** the reader MUST find the invocation artifact entries linked to that invocation id

#### Scenario: lookup reports missing marker

- **GIVEN** an expected artifact was not produced
- **WHEN** a missing artifact marker is registered for its correlation id
- **THEN** lookup MUST return a result that is distinguishable from both success artifacts and no index entry

### Requirement: Evidence index kernel MUST support parent-child artifact links

The evidence index kernel MUST support a minimal parent-child link so isolated child runs and micro-runs can be associated with the parent run that invoked them.

The parent-child link MUST use correlation identifiers rather than mutating the parent run topology.

#### Scenario: child capability result links to parent invocation

- **GIVEN** a parent run invokes an isolated capability
- **WHEN** the child run writes a result or failure artifact
- **THEN** the index entry MUST be able to link the child artifact correlation id to the parent invocation correlation id
- **AND** the parent run topology MUST NOT need to change for the link to exist

### Requirement: Evidence index kernel MUST preserve Phase 1A and Phase 1B boundaries

The evidence index kernel MUST NOT require evidence CLI, doctor UX, or diagnosis taxonomy fields to satisfy the Phase 1A contract.

Phase 1A MUST remain limited to artifact links, correlation lookup, status markers, and parent-child links.

#### Scenario: CLI evidence UX is not required for kernel tests

- **WHEN** Phase 1A contract tests run
- **THEN** they MUST pass without invoking `ralph evidence summary`, `ralph evidence inspect`, or `ralph doctor evidence`

#### Scenario: graph display is not treated as durable truth source

- **WHEN** runtime graph or Rerun output exists for a run
- **THEN** the evidence index MUST NOT treat graph layout or live observer output as the durable replay truth source
- **AND** it MUST prefer durable JSONL records or explicit artifact files for indexed evidence

---

### Requirement: Evidence index MUST correlate dynamic role contracts without replacing source artifacts
The evidence index kernel MUST support correlation entries for dynamic role contract evidence while preserving record-session JSONL, event logs, and agents snapshots as the source artifacts.

A dynamic role correlation entry MUST be able to link a spawn request id, role contract hash, instance id, and produced result topic to the artifact paths that contain the evidence.

#### Scenario: lookup by role contract hash finds source artifacts
- **WHEN** a dynamic instance is spawned with `role_contract_hash = erc-abc123`
- **THEN** an evidence lookup by that hash MUST be able to find the record-session or agents snapshot artifact containing the role contract summary
- **AND** the lookup MUST NOT require duplicating the full role contract text into the index

#### Scenario: lookup by spawn request id shows children
- **WHEN** a parent-visible spawn group has `request_id = evolution-analysis-5-lenses`
- **THEN** an evidence lookup by request id MUST be able to list the spawned instance artifact links or missing markers for expected members

### Requirement: Evidence index MUST support missing dynamic result markers
The evidence index kernel MUST support missing result markers for expected dynamic role outputs.

A missing marker MUST distinguish between no index entry, a known missing result, and a terminal failed result.

#### Scenario: spawned role has no result topic
- **WHEN** a dynamic role was spawned and expected to publish `analysis.done` but no matching event appears before termination
- **THEN** the index MAY register a missing result marker
- **AND** lookup MUST report that marker distinctly from a successful result artifact

### Requirement: Evidence index boundaries MUST remain Phase-aware
The evidence index kernel MUST keep correlation storage separate from higher-level evidence UX, diagnosis taxonomy, and release policy.

The index MAY provide links and status markers, but release-fast gate decisions and human-readable diagnosis MUST remain in higher-level CLI/report layers.

#### Scenario: CLI display changes do not invalidate index entries
- **WHEN** `ralph record summary` changes its display format
- **THEN** existing evidence index entries MUST remain valid if their artifact paths and correlation ids still resolve
