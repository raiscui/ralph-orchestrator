## ADDED Requirements

### Requirement: Parent-visible spawn groups MUST preserve dynamic role contract summaries
Parallel runtime MUST preserve a role contract summary for every parent-visible dynamic instance created by `topology.spawn_group`.

The summary MUST be attached to spawn result evidence, agents snapshot evidence, and completed dynamic tombstones when the instance is later reaped.

#### Scenario: spawn result includes per-instance role contract summaries
- **WHEN** a coordinator emits `topology.spawn_group` with five task-derived roles
- **THEN** `topology.spawn.result` MUST list each spawned instance id and role
- **AND** each spawned item MUST include a role contract summary with `role_contract_hash`, `identity_source`, and `source_spawn_request_id`

#### Scenario: completed dynamic instance keeps role identity
- **WHEN** a dynamic instance publishes its final result and is reaped from the active registry
- **THEN** the completed dynamic instance tombstone MUST retain the role contract summary
- **AND** `ralph agents` or equivalent snapshot display MUST not make the user infer the role from old stdout text

### Requirement: Spawn group partial outcomes MUST be explicitly represented
Parallel runtime MUST represent partial `topology.spawn_group` outcomes explicitly instead of collapsing them into either total success or silent failure.

Partial outcomes include member validation failure, successful spawn followed by delivery failure, spawned instance timeout, result-topic absence, and cleanup/reaping after failure.

#### Scenario: member spawn succeeds but delivery fails
- **WHEN** a `topology.spawn_group` member is created but the delivery topic cannot be delivered to that instance
- **THEN** the runtime MUST emit or record a partial outcome that names the instance id, request id, role, and failed phase
- **AND** the agents snapshot MUST either show the instance as failed-after-spawn or retain a completed/failed tombstone after cleanup

#### Scenario: one member fails while others continue
- **WHEN** one member of a non-atomic spawn group fails and other members can still run
- **THEN** the runtime MUST allow the successful members to continue
- **AND** the parent evidence MUST distinguish successful member results from failed member outcomes

### Requirement: Dynamic spawn dogfood MUST validate parent-visible result coverage
Parallel runtime MUST maintain a focused dynamic spawn dogfood or integration guardrail that validates parent-visible dynamic instance creation, result coverage, final reply durability, and completion evidence.

The guardrail MUST assert that spawned dynamic instances are visible either in the current agents registry or completed dynamic tombstones, and that expected result topics are covered by the spawned source instances.

#### Scenario: natural-language multi-angle dogfood creates dynamic roles
- **WHEN** a coordinator receives a user request to choose several analysis angles and create hats for parallel analysis
- **THEN** the runtime dogfood MUST show one `topology.spawn_group`, one `topology.spawn.result`, zero `topology.spawn.failed`, and result topics from each spawned dynamic instance
- **AND** record-session MUST include `_meta.termination.reason = CompletionPromise`
