# Design: capability-failure-fallback-human-reply-dogfood

## Context

Current runtime behavior already covers:

- parent-triggered `capability.request`
- parent-visible `capability.result`
- parent-visible `capability.failed`
- explicit human-facing answer through `reply.human.message`

The missing product proof is the failure branch where `ralph#1` continues after a failure instead of ending the run immediately.

## Goal

Prove the smallest parent-side fallback workflow:

1. turn 1: `ralph#1` emits an invalid capability request
2. runtime returns `capability.failed`
3. turn 2: parent prompt now contains that failure context
4. `ralph#1` emits a fallback valid capability request
5. runtime returns `capability.result`
6. turn 3: parent prompt contains the fallback success context
7. `ralph#1` emits final explicit `reply.human.message`

## Non-goals

- No automatic retry policy engine
- No dynamic topology mutation
- No broker that rewrites parent decisions
- No requirement that every failure create an invocation id
- No generalized planning DSL over failure branches

## Chosen test shape

Use the existing `integration_live_capability.rs` harness.

### Backend script behavior

For `ralph#1`:

- turn 1:
  - verify runtime capability catalog is present
  - emit invalid `capability.request` with a known `request_id`
- turn 2:
  - verify prompt contains `capability.failed`
  - verify prompt contains the failed request id and invalid capability id
  - emit fallback valid `capability.request`
- turn 3:
  - verify prompt contains fallback `capability.result`
  - emit explicit `reply.human.message`
  - emit `LOOP_COMPLETE`

This keeps the gate narrow while proving the parent can consume failure context and continue.

## Assertions

The focused gate should assert:

- parent event log contains one `capability.failed`
- the failed payload includes the original `request_id`
- the failure keeps `parent_topology_unchanged = true`
- parent event log later contains fallback `capability.result`
- fallback invocation id remains inspectable through `ralph tools capability inspect <id> --json`
- `.ralph/events.jsonl` preserves `capability.failed`, fallback `capability.result`, and final `reply.human.message` as separate events
- record-session preserves the final explicit human-facing reply

## Risks and mitigations

1. **Failure context not visible in later parent turn**
   - Symptom: turn 2 prompt does not contain `capability.failed`
   - Mitigation: fail the gate immediately and inspect prompt captures before changing runtime

2. **Invalid capability path produces too little metadata**
   - Symptom: parent cannot see failed request id or capability id
   - Mitigation: assert those exact fields in the captured prompt and event payload

3. **Failure branch accidentally becomes human-visible reply**
   - Symptom: CLI stdout shows failure payload before explicit final reply
   - Mitigation: assert the final visible payload is only the explicit `reply.human.message`
