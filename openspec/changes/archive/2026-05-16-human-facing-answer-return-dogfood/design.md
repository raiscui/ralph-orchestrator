# Design: human-facing-answer-return-dogfood

## 1. Context

The current runtime already separates two reply channels:

- `reply.hat.message`: explicit hat-to-hat answer return, routed back to the original requester instance.
- `reply.human.message`: explicit hat-to-human output topic, observed for UI/logging and not routed back into hats.

This separation is correct and already guarded by unit tests. The missing piece is a single repo-native runtime proof that both channels can appear in one workflow run without blurring their responsibilities.

## 2. Goal

Add the smallest dogfood gate that proves this sequence:

1. A coordinator asks another hat for information.
2. The worker returns the answer via `reply.hat.message`.
3. The coordinator decides what to tell the human.
4. The coordinator explicitly emits `reply.human.message`.
5. The human-visible answer is auditable in durable runtime artifacts.

## 3. Non-goals

- No new routing primitive.
- No automatic synthesis from internal answer return into human-visible reply.
- No request broker, mailbox layer, or answer aggregation service.
- No topology mutation or runtime hot-loading.
- No new general-purpose evidence CLI.
- No live Codex or app-server dependency for the minimal gate.

## 4. Preferred minimal runtime shape

The most stable repo-native gate should reuse the existing custom-backend integration harness.

Recommended test flow:

1. `ralph#1` starts and emits an internal request event, for example `research.request`.
2. `researcher#1` receives it and returns `reply.hat.message reply=<request_id>`.
3. `ralph#1` receives that internal answer on the next turn.
4. `ralph#1` explicitly emits `reply.human.message` with the final human-facing payload.
5. The run terminates via `LOOP_COMPLETE`.

This shape keeps the gate deterministic and avoids introducing external `human.message` injection or app-server timing as a prerequisite.

## 5. Evidence contract for the gate

The gate should prove three things at once:

### 5.1 Internal answer-return remains internal

- `.ralph/events.jsonl` contains the internal `reply.hat.message` and requester-return delivery evidence.
- The internal answer remains auditable via the existing answer evidence index contract.

### 5.2 Human-facing reply remains explicit

- `.ralph/events.jsonl` contains a separate `reply.human.message` event emitted by the coordinator.
- That event is not synthesized by runtime routing; it is produced explicitly by the workflow actor.

### 5.3 Human-facing reply is actually visible to runtime consumers

At least the following surfaces should show evidence of the final human reply:

- `.ralph/events.jsonl`
- record-session JSONL (`bus.publish(topic=reply.human.message)` or equivalent runtime record)
- CLI output surface for the run, showing the human-facing payload rather than requiring manual evidence-index inspection

## 6. Why this is the right boundary

This design keeps the single-source-of-truth layering intact:

- event log remains the durable truth source
- evidence index remains the lookup layer for internal answer-return artifacts
- `reply.human.message` remains the explicit user-facing output contract

The gate does not invent a new abstraction. It only proves the existing abstractions compose correctly.

## 7. Failure interpretation

If the dogfood gate fails, interpret it in this order:

1. **Display failure**: `reply.human.message` exists in event log / record-session but is not visible in CLI output.
2. **Durability failure**: `reply.human.message` is visible in live stdout but missing from durable artifacts.
3. **Workflow failure**: `ralph#1` never emits the explicit human-facing reply after receiving the internal answer.

Only the third category would justify a runtime behavior change. The first two categories are more likely recording or rendering gaps.

## 8. Test strategy summary

Implementation should start with one focused CLI integration gate, then only add smaller unit fixes if the gate exposes a real contract gap.

See `test-plan.md` for the detailed assertions and stop conditions.
