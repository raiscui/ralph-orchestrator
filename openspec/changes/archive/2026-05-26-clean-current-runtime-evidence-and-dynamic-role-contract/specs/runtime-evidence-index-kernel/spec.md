## ADDED Requirements

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
