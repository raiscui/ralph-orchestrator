# Test plan: human-facing-answer-return-dogfood

## Scope

This test plan covers the minimal repo-native proof that an internal answer-return can become a human-visible reply only through an explicit `reply.human.message` step.

It assumes the existing `request-reply-answer-evidence` kernel and answer inspect UX remain unchanged.

## Test layers

### 1. Focused CLI integration gate

Goal: prove one runtime run can contain both internal answer-return and explicit human-facing reply.

Planned flow:

1. coordinator emits an internal request
2. worker replies with `reply.hat.message`
3. coordinator emits explicit `reply.human.message`
4. run exits with `LOOP_COMPLETE`

Planned assertions:

- run exits successfully
- `.ralph/events.jsonl` contains the delivered internal answer-return evidence
- `.ralph/events.jsonl` contains a separate `reply.human.message`
- record-session contains publication evidence for `reply.human.message`
- CLI output contains the final human-facing payload

### 2. Boundary preservation assertions

Goal: prove the new gate does not weaken the existing separation of responsibilities.

Planned assertions:

- `reply.hat.message` remains the internal requester-return channel
- `reply.human.message` remains an explicit human-facing output topic
- the gate does not rely on implicit synthesis
- the gate does not require runtime graph artifacts
- the gate does not require a live Codex/app-server backend

### 3. Failure classification

If the new gate fails, classify it before changing code:

- **Display-only**: durable artifacts contain `reply.human.message`, but visible CLI output does not
- **Durability-only**: stdout shows the human-facing reply, but durable artifacts do not preserve it
- **Workflow**: the coordinator never emits explicit `reply.human.message` after receiving the internal answer

Only the third case justifies changing workflow/runtime behavior.

### 4. Expected commands after implementation

```bash
cargo test -p ralph-cli --test integration_answer_evidence -- --nocapture
cargo test -p ralph-cli answer -- --nocapture
cargo test -p ralph-core smoke_runner
cargo test
openspec validate human-facing-answer-return-dogfood --type change
openspec validate --all --strict
```

## Stop conditions

Stop implementation and return to design if:

- the gate can only pass by implicitly synthesizing `reply.human.message`
- the gate requires topology mutation or a new request broker
- the gate depends on runtime graph output as the truth source
- the gate can only be proven with a live external app-server backend
