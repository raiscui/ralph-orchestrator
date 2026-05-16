# Test plan: capability-result-human-reply-dogfood

## Scope

This test plan covers the smallest repo-native proof that a parent-triggered capability invocation can become a human-visible answer only through an explicit `reply.human.message` step.

## Test layers

### 1. Focused CLI integration gate

Goal: prove one runtime run can contain the full chain:

1. `capability.request`
2. parent-visible `capability.result`
3. explicit `reply.human.message`
4. normal completion

Planned assertions:

- run exits successfully
- parent event log contains `capability.request`
- parent event log contains parent-visible `capability.result`
- parent event log contains explicit `reply.human.message`
- record-session preserves evidence of `reply.human.message`
- CLI output shows the final human-facing payload
- `ralph tools capability inspect <invocation_id> --json` still resolves the invocation artifacts

### 2. Boundary preservation assertions

Goal: prove the gate does not weaken the existing topology or reply semantics.

Planned assertions:

- `capability.result` remains a parent-consumable runtime event
- `reply.human.message` remains explicit
- no automatic synthesis is required
- parent topology remains unchanged
- no runtime graph or live external backend is required

### 3. Failure classification

If the new gate fails, classify it before changing code:

- **Parent workflow**: coordinator never emits `reply.human.message` after receiving `capability.result`
- **Display-only**: human-facing reply is in artifacts but not visible in CLI output
- **Durability-only**: human-facing reply is visible but not preserved in events or record-session

### 4. Expected commands after implementation

```bash
cargo test -p ralph-cli --test integration_live_capability -- --nocapture
cargo test -p ralph-core smoke_runner
cargo test
openspec validate capability-result-human-reply-dogfood --type change
openspec validate --all --strict
```

## Stop conditions

Stop implementation and return to design if:

- the gate can only pass by auto-synthesizing `reply.human.message`
- the gate requires topology mutation or a broker layer
- the gate depends on runtime graph output as the truth source
- the gate can only be proven with a live external backend
