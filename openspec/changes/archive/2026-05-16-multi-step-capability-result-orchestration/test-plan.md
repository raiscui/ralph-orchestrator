# Test plan: multi-step-capability-result-orchestration

## Scope

This test plan covers the smallest repo-native proof that one parent run can orchestrate multiple isolated capability invocations in sequence and only emit a final human-facing answer after the chain completes.

## Test layers

### 1. Focused CLI integration gate

Goal: prove one runtime run can contain this sequence:

1. capability request A
2. capability result A
3. capability request B
4. capability result B
5. final explicit `reply.human.message`
6. normal completion

Planned assertions:

- run exits successfully
- parent event log contains both capability requests with distinct request ids
- parent event log contains both capability results
- each invocation id is independently inspectable
- CLI output contains the final human-facing payload only at the end
- record-session preserves the final human-facing reply publication

### 2. Boundary preservation assertions

Goal: prove multi-step orchestration does not weaken the current contracts.

Planned assertions:

- all capability executions remain isolated
- parent topology remains unchanged
- intermediate capability results are not treated as human-facing answers
- final human-facing answer still requires explicit `reply.human.message`

### 3. Failure classification

If the new gate fails, classify it before changing code:

- **Context propagation**: later turns do not show prior capability results in prompt context
- **Orchestration**: parent cannot successfully emit the next request after prior result
- **Durability/display**: final human-facing reply is missing from one artifact surface

### 4. Expected commands after implementation

```bash
cargo test -p ralph-cli --test integration_live_capability -- --nocapture
cargo test -p ralph-core smoke_runner
cargo test
openspec validate multi-step-capability-result-orchestration --type change
openspec validate --all --strict
```

## Stop conditions

Stop implementation and return to design if:

- later turns cannot see prior capability results in parent context
- the gate requires topology mutation or a scheduler/broker layer
- the gate requires automatic human reply synthesis
- the gate can only be proven with a live external backend
