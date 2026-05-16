# Design: capability-result-human-reply-dogfood

## 1. Context

Current durable contracts already exist:

- parent runtime capability invocation produces `capability.request` and parent-visible `capability.result` / `capability.failed`
- isolated child or micro-run artifacts remain auditable through `.ralph/capability-invocations/<invocation_id>/...`
- human-facing answers remain explicit and must use `reply.human.message`

The missing proof is that these contracts compose cleanly in one runtime run.

## 2. Goal

Prove the smallest end-to-end product flow:

1. `ralph#1` requests a capability
2. runtime executes the capability in isolation
3. parent run receives `capability.result`
4. `ralph#1` explicitly emits `reply.human.message`
5. runtime artifacts preserve both the capability chain and the final human-visible answer

## 3. Non-goals

- No new routing topic
- No automatic conversion from `capability.result` into human reply
- No live topology mutation
- No new capability broker or answer aggregator
- No live external backend dependency for the minimal gate

## 4. Preferred test shape

The existing `integration_live_capability.rs` harness is already the best base.

Recommended focused flow:

- turn 1: `ralph#1` emits `capability.request`
- isolated invocation completes and runtime writes parent `capability.result`
- turn 2: `ralph#1` sees `capability.result` in its prompt and emits `reply.human.message`
- run exits with `LOOP_COMPLETE`

This gives one deterministic repo-native chain without adding app-server timing or external emit choreography.

## 5. Evidence contract

The new gate should prove three independent layers remain intact:

### 5.1 Capability runtime layer

- parent event log contains `capability.request`
- parent event log contains parent-visible `capability.result`
- invocation artifacts remain inspectable by invocation id

### 5.2 Human-facing answer layer

- parent event log contains explicit `reply.human.message`
- record-session preserves the human-facing reply publication evidence
- CLI output shows the final human-facing payload

### 5.3 Boundary layer

- `capability.result` does not itself count as a human-facing answer
- human-facing output only appears after explicit `reply.human.message`
- parent topology remains unchanged throughout the flow

## 6. Failure interpretation

Classify failures before changing code:

1. **Parent workflow failure**: `ralph#1` never emits explicit `reply.human.message` after seeing `capability.result`
2. **Display failure**: `reply.human.message` exists in artifacts but not in visible CLI output
3. **Durability failure**: visible human reply appears, but durable artifacts or record-session do not preserve it

Only the first category implies a runtime behavior gap. The other two point to output or recording surfaces.

## 7. Test strategy summary

Start with one focused CLI integration gate. Only add code fixes if the gate reveals an actual contract hole.
