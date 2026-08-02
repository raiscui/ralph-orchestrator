## ADDED Requirements

### Requirement: Runtime protocol SSOT MUST define control-plane and result-plane boundaries
Ralph MUST maintain a single runtime protocol contract that defines reserved control-plane topics, workflow entry hints, result topics, reply topics, and stdout-only event parsing boundaries.

The contract MUST state that `task.start` and `task.resume` are runtime entry events, that `event_loop.starting_event` is only a workflow entry hint after coordination, and that ordinary hats MUST NOT treat reserved control-plane topics as business triggers unless a specific runtime contract grants that authority.

#### Scenario: control-plane topic is not treated as ordinary worker trigger
- **WHEN** a parallel runtime prompt or validator encounters `task.start`, `task.resume`, `topology.*`, `capability.*`, `runtime.*`, `gate.*`, `human.message`, or `reply.human.message`
- **THEN** the protocol contract MUST identify whether the topic is reserved, observer-only, coordinator-only, or worker-publishable
- **AND** worker prompt generation MUST NOT present reserved topics as normal business result topics

#### Scenario: starting event remains a coordinator workflow hint
- **WHEN** `event_loop.starting_event` is configured as `build.task`
- **THEN** the runtime MUST still publish `task.start` as the first runtime event
- **AND** only the coordinator decision after `task.start` MAY publish the configured workflow entry topic

### Requirement: Dynamic role contract evidence MUST be part of the runtime evidence closure
Ralph MUST expose task-derived dynamic role contracts as auditable runtime evidence, not only as prompt text.

For each dynamic role contract created from `topology.spawn_group`, the evidence closure MUST include at least the source spawn request id, instance id, role name, canonical objective preview, allowed result topics, role contract hash, persistence class, and any normalization warnings.

#### Scenario: spawned dynamic instance has role contract evidence
- **WHEN** `ralph#1` emits `topology.spawn_group` and the runtime spawns a dynamic instance
- **THEN** the spawn result and agents snapshot evidence MUST expose a role contract summary for that instance
- **AND** the summary MUST include a stable `role_contract_hash` that can be correlated later

#### Scenario: prompt text is not the only role evidence
- **WHEN** a reviewer inspects a completed dynamic instance after it has been reaped from the active registry
- **THEN** they MUST be able to see the retained role contract summary without reading the worker prompt transcript

### Requirement: Runtime evidence inspection MUST correlate protocol, role, and result evidence
Ralph MUST provide an evidence inspection path that correlates runtime protocol events, dynamic role contract summaries, result topics, reply events, agents snapshots, and record-session termination state.

The inspection path MUST preserve record-session JSONL, `.ralph/events.jsonl`, `.ralph/agents.json`, and evidence-index entries as separate truth sources. It MUST NOT synthesize a single replacement truth source that hides disagreements between them.

#### Scenario: reviewer traces a dynamic spawn from request to result
- **WHEN** a record-session contains `topology.spawn_group`, `topology.spawn.result`, dynamic `build.task` deliveries, and `analysis.done` results
- **THEN** evidence inspection MUST show the spawn `request_id`, spawned instance ids, role contract hashes, result source instances, and final termination reason
- **AND** the reviewer MUST be able to tell which evidence came from record-session and which came from agents snapshot sidecars

#### Scenario: incomplete evidence remains visible as incomplete
- **WHEN** a dynamic role was spawned but no matching result topic or terminal failure topic is present
- **THEN** evidence inspection MUST report the missing result coverage instead of implying success from the spawn result alone

### Requirement: Runtime release-fast gate MUST prove protocol and evidence closure
Ralph MUST define a release-fast gate for the runtime/evidence lane that proves protocol correctness, dynamic role evidence, record-session durability, and parent-visible dynamic spawn behavior before a change is declared done.

The gate MUST include OpenSpec validation, Rust tests for the touched modules, replay smoke tests for deterministic behavior, and at least one focused live or replay dogfood that retains record-session and agents snapshot evidence.

#### Scenario: release-fast gate has durable artifacts
- **WHEN** the release-fast gate completes for a runtime/evidence change
- **THEN** the output MUST include commands run, pass/fail status, record-session path if a runtime dogfood was run, and agents snapshot path if dynamic instances were involved
- **AND** a `CompletionPromise` termination in record-session MUST be preferred over wrapper process text as the semantic completion signal

#### Scenario: gate rejects display-only proof
- **WHEN** a terminal screenshot or stdout tail appears to show success but record-session lacks `_meta.termination` or expected result topics
- **THEN** the gate MUST treat the run as incomplete until durable evidence is present
