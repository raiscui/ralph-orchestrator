# Design: multi-step-capability-result-orchestration

## 1. Context

The current product surface already proves:

- one `capability.request` can trigger one isolated capability invocation
- one resulting `capability.result` can be consumed by `ralph#1`
- one explicit `reply.human.message` can be emitted after that result

The missing seam is orchestration memory across steps inside the same parent run.

## 2. Goal

Prove the smallest multi-step parent workflow:

1. `ralph#1` emits capability request A
2. runtime returns capability result A
3. `ralph#1` uses result A context to emit capability request B
4. runtime returns capability result B
5. `ralph#1` emits one final explicit `reply.human.message`
6. parent topology stays unchanged throughout

## 3. Non-goals

- No parallel fanout planner across many capability requests
- No new capability scheduler or queue manager
- No capability-result aggregation service beyond what `ralph#1` explicitly does in its own workflow
- No implicit human answer synthesis
- No live external backend requirement

## 4. Preferred gate shape

Reuse the existing `integration_live_capability.rs` harness with a deterministic custom backend.

Recommended deterministic parent turns:

- turn 1: verify capability catalog is present, emit request A
- turn 2: verify prompt now contains `capability.result` for request A, emit request B
- turn 3: verify prompt contains `capability.result` for request B, emit final `reply.human.message` and `LOOP_COMPLETE`

This keeps the test narrow and proves the parent can see the evolving multi-step result history.

## 5. Evidence contract

The gate should prove three layers:

### 5.1 Sequenced capability invocation layer

- `.ralph/events.jsonl` preserves request A and request B separately
- `.ralph/events.jsonl` preserves result A and result B separately
- each invocation id remains inspectable independently

### 5.2 Parent orchestration layer

- later capability requests are emitted only after prior capability results become parent-visible
- the parent run uses distinct `request_id` values for each step
- topology remains unchanged during all steps

### 5.3 Human-facing closure layer

- final `reply.human.message` is emitted only after the multi-step chain finishes
- CLI output and record-session preserve the final human-facing payload

## 6. Failure interpretation

Classify failures before changing runtime code:

1. **Context propagation failure**: later parent turns do not show prior `capability.result` context
2. **Orchestration failure**: parent cannot issue a second valid capability request after the first result
3. **Durability/display failure**: final human-facing reply exists in one surface but not the others

Only the first two imply a runtime or prompt-context gap.

## 7. Test strategy summary

Start with one focused CLI integration gate and only widen implementation if the gate reveals a real contract hole.
