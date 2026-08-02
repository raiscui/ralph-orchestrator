## Why

The latest dynamic-hats dogfood proved that Ralph can let a coordinator derive five parent-visible analysis roles and run them to completion, but it also showed that the next useful evolution is not another feature layer. The runtime now needs a single, testable contract that ties protocol semantics, dynamic role identity, durable evidence, and release gates together so future changes cannot drift across prompts, record-session summaries, agents snapshots, and E2E checks.

## What Changes

- Establish a runtime protocol SSOT for reserved control topics, workflow entry hints, reply topics, and stdout-only event parsing.
- Promote task-derived dynamic role contracts from prompt-only guidance into auditable runtime evidence, including canonical role contract summaries, hashes, source spawn request ids, allowlists, and warnings.
- Specify `topology.spawn_group` partial/tombstone behavior so parent-visible dynamic instances remain explainable when spawn, delivery, completion, or cleanup is only partially successful.
- Extend evidence inspection expectations so a reviewer can correlate `request_id`, event ids, reply ids, source/target instances, `role_contract_hash`, record-session entries, agents snapshots, and result topics without treating any display surface as a replacement truth source.
- Define a release-fast gate for this runtime lane, combining OpenSpec validation, Rust tests, replay smoke tests, live parallel Codex focused E2E, and record-session/agents artifact retention.
- Defer unrelated upper-layer work such as mdfried image rendering, recoverable CLI retry implementation, and manifest schema v2 unless they are needed to satisfy this contract.

## Capabilities

### New Capabilities
- `current-runtime-evidence-contract`: Defines the unified runtime/evidence closure contract for protocol SSOT, dynamic role contract evidence, correlation inspection, and release-fast gates.

### Modified Capabilities
- `parallel-hat-instances`: Adds stricter parent-visible dynamic spawn, partial/tombstone, and task-derived role lifecycle requirements.
- `record-session-contract-and-watch`: Adds evidence inspection requirements for dynamic role and spawn correlation summaries.
- `runtime-evidence-index-kernel`: Adds correlation expectations for runtime protocol and dynamic role artifacts while preserving original truth sources.
- `prompt-contract-runtime-alignment`: Adds prompt-surface consistency requirements for runtime protocol and task-derived role contracts.

## Impact

- Runtime protocol and prompt generation paths in `crates/ralph-core/src/event_emission_protocol.rs`, `crates/ralph-core/src/instructions.rs`, `crates/ralph-core/src/prompt_surface.rs`, and related prompt overlay logic.
- Parent-visible dynamic spawn flow in `crates/ralph-core/src/topology_spawn.rs`, `crates/ralph-core/src/parallel/supervisor/topology_runtime.rs`, `crates/ralph-core/src/parallel/supervisor.rs`, and `crates/ralph-core/src/agents_snapshot.rs`.
- Record summary and evidence inspection surfaces in `crates/ralph-cli/src/record_cli.rs`, `crates/ralph-cli/src/record_session.rs`, and `.ralph/agents.json` sidecar handling.
- E2E/replay validation in `crates/ralph-e2e`, `crates/ralph-core` smoke fixtures, and any repo-local script that becomes the release-fast gate.
- No direct code implementation is part of this proposal; implementation must follow the accepted spec and tasks.
