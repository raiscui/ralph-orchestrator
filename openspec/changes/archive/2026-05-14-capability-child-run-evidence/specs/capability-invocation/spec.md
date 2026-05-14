## ADDED Requirements

### Requirement: Capability invocation MUST register child-run evidence index entries

Ralph MUST register evidence index entries for each isolated capability invocation so that the invocation id can be used to locate the durable child-run artifacts.

The evidence index MUST link to existing durable artifacts and MUST NOT replace those artifacts as the truth source.

#### Scenario: successful micro-run artifacts are discoverable by invocation id

- **GIVEN** `ralph tools capability invoke` executes a hat capability through an isolated micro-run
- **WHEN** the invocation succeeds
- **THEN** `.ralph/evidence-index.jsonl` MUST contain entries for the invocation id
- **AND** those entries MUST include `capability_invoke_json`, `capability_result_json`, `resolved_config`, and `event_log_jsonl`
- **AND** each entry MUST point to the durable artifact path written by the invocation

#### Scenario: failed child-run artifacts are discoverable by invocation id

- **GIVEN** an isolated capability invocation writes `failed.json`
- **WHEN** the invocation completes with a failure result
- **THEN** `.ralph/evidence-index.jsonl` MUST contain a `capability_failed_json` entry for the invocation id
- **AND** that entry MUST have failure status
- **AND** the invocation MUST still preserve the parent topology

### Requirement: Capability invocation evidence MUST preserve parent topology isolation

Ralph MUST NOT mutate the parent run topology while registering evidence for a capability invocation.

Evidence registration MUST describe the isolated invocation artifacts, not inject the child capability into the parent topology.

#### Scenario: parent config remains unchanged after evidence registration

- **GIVEN** the workspace contains a parent `ralph.yml`
- **WHEN** `ralph tools capability invoke` writes artifacts, events, and evidence index entries
- **THEN** the parent `ralph.yml` MUST remain byte-for-byte unchanged
- **AND** the invocation/result artifacts MUST report `parent_topology_unchanged=true`

### Requirement: Capability invocation evidence MUST fail visibly when evidence cannot be recorded

Ralph MUST treat evidence-index recording failure as an invocation failure rather than silently returning a successful audit report.

#### Scenario: evidence index write failure is not hidden

- **GIVEN** capability child artifacts have been produced
- **WHEN** Ralph cannot write `.ralph/evidence-index.jsonl`
- **THEN** the command MUST return an error
- **AND** the operator MUST not receive a successful JSON report that omits evidence-index linkage
