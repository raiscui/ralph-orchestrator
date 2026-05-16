# Test plan: capability-failure-fallback-human-reply-dogfood

## Goal

Prove that a parent run can recover from `capability.failed`, invoke a fallback capability step, and still only expose the final answer through explicit `reply.human.message`.

## Main flow

1. invalid `capability.request`
2. parent-visible `capability.failed`
3. fallback valid `capability.request`
4. fallback parent-visible `capability.result`
5. explicit final `reply.human.message`

## Required assertions

- parent event log contains `capability.failed`
- failed payload contains the original failed `request_id`
- failed payload preserves `parent_topology_unchanged = true`
- parent event log contains fallback `capability.result`
- fallback invocation id is inspectable with `ralph tools capability inspect <invocation_id> --json`
- `.ralph/events.jsonl` preserves failure, fallback success, and final human reply separately
- record-session preserves the final explicit human-facing reply
- CLI stdout exposes only the explicit final human-facing payload

## Guardrails

Goal: prove failure fallback does not weaken current contracts.

Must continue to prove:
- `capability.failed` remains a parent-consumable runtime event
- fallback capability execution still uses isolated child/micro-run path
- parent topology remains unchanged
- final human-facing answer remains explicit

Must not introduce:
- automatic retry engine semantics
- topology mutation
- automatic synthesis from failure/result into human reply

## Failure modes to catch

- **Failure context missing**: turn 2 prompt does not show `capability.failed` or failed request metadata
- **Fallback never happens**: parent stops after failure instead of emitting a later valid request
- **Failure leaked to human output**: CLI stdout shows failure payload before explicit final reply
- **Fallback success not auditable**: inspect cannot resolve fallback invocation id

## Commands

```bash
openspec validate capability-failure-fallback-human-reply-dogfood --type change
cargo test -p ralph-cli --test integration_live_capability
cargo test -p ralph-core smoke_runner
openspec validate --all --strict
cargo test
```

## Exit criteria

- all commands above pass
- parent prompt captures prove later turns can see `capability.failed`
- final human-facing answer still only appears after explicit `reply.human.message`
