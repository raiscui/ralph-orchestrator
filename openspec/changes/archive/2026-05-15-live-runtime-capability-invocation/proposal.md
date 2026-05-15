## Why

Ralph can already list capabilities, run isolated capability invocations from the CLI, record evidence, and inspect that evidence by invocation id. The next step is to let a real parent run trigger a capability invocation and receive a structured result without mutating the parent topology.

This closes the loop from "capability invocation exists as a standalone tool" to "`ralph#1` can use capability invocation as a runtime action while preserving durable evidence."

## What Changes

- Add a parent-run trigger path for runtime capability invocation.
- The parent run MUST be able to request a capability by structured event/output, including capability id and input.
- Ralph MUST execute the request through the existing isolated child/micro-run invocation path.
- Ralph MUST emit a parent-consumable result or failure event that includes the invocation id and artifact paths.
- Ralph MUST preserve parent topology; it must not inject invoked capabilities into the live parent `HatRegistry` or replace the parent config.
- Dogfood validation MUST use `ralph tools capability inspect <invocation_id> --json` to verify the evidence chain.

## Capabilities

### New Capabilities

### Modified Capabilities
- `capability-invocation`: Add live parent-run trigger and result-return requirements for isolated capability invocation.

## Impact

- Runtime/CLI integration: likely `crates/ralph-cli/src/loop_runner.rs`, `crates/ralph-cli/src/capability.rs`, or the event/output parsing layer that processes parent run events.
- Core protocol: may reuse existing capability topics and records from `ralph-core` / `ralph-proto`; avoid adding broad new broker abstractions unless required.
- Tests: integration dogfood should run a deterministic parent run that emits a capability request, then assert parent events, child artifacts, evidence index entries, and Phase 3.1 inspect output.
- No external LLM dependency and no live topology mutation.
