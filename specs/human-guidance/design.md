# spec: human guidance tracking + explicit completion (port of origin #326)

## Goal

Allow a human operator to inject guidance mid-loop that must be explicitly
acknowledged before `ralph` can emit the completion promise. Blocks silent
completion that would ignore outstanding guidance.

## Source

- origin commit: `a4b6d456ba86c763c73dddca01017bd1be77bc56 fix(runtime): require explicit completion after guidance (#326)`
- PR: https://github.com/mikeyobrien/ralph-orchestrator/pull/326 (verify before land)

## Schema (no Rust struct changes)

Two new event topics, consumed by the event_loop:

| Topic | Effect |
|---|---|
| `human.guidance` | Push payload to `unacknowledged_guidance` queue. |
| `human.guidance.ack` | Clear `unacknowledged_guidance` queue. |

Both topics are valid in any iteration; they pass through to the validator
without further processing.

## Behavior

### Guard: unacknowledged guidance blocks completion

When `completion_detected == true` (ralph output contains the completion promise
**and** scratchpad/memory verification passes), additionally check:

- If `LoopState::unacknowledged_guidance` is non-empty, **reject** the completion:
  - Reset `completion_confirmations = 0` (same as pending-tasks rejection).
  - Publish `task.resume` event listing the unacknowledged guidance count.
  - Continue the loop (do not terminate).

This composes cleanly with the local main **2-strike pattern**: when guidance
is unacknowledged, the confirmation counter resets, requiring 2 fresh
confirmations after the ack before termination.

### Guard: default_publishes cannot silently complete

When `check_default_publishes` is about to inject a default event whose topic
equals the configured `completion_promise`:

- Skip the injection
- Publish a `task.resume` event to that hat asking for explicit completion
  evidence

This prevents a misconfigured `default_publishes` from triggering silent
completion when the hat produced no events.

### HatlessRalph guidance text

Prepend a blocking notice to the `## ROBOT GUIDANCE` section:

> This guidance is blocking. You MUST address it before declaring completion.
> After addressing it, emit `human.guidance.ack` with a brief summary.

## Architecture

### New field on LoopState

```rust
pub struct LoopState {
    // ... existing fields ...
    /// Human guidance messages that must be acknowledged before completion.
    pub unacknowledged_guidance: Vec<String>,
}
```

Default: empty `Vec`.

### Event parser additions

In `EventLoop::process_output` (or the helper that handles validated events),
recognize the two new topics and apply the side effects:

```rust
if event.topic == "human.guidance" {
    self.state.unacknowledged_guidance.push(payload.clone());
    validated_events.push(event);
    continue;
}

if event.topic == "human.guidance.ack" {
    self.state.unacknowledged_guidance.clear();
    validated_events.push(event);
    continue;
}
```

### Completion gate

In the existing `match verification_result { Ok(true) => { ... } }` arm,
before incrementing `completion_confirmations`:

```rust
if !self.state.unacknowledged_guidance.is_empty() {
    let count = self.state.unacknowledged_guidance.len();
    warn!(guidance_count = count, "Completion rejected: unacknowledged human.guidance");
    self.state.completion_confirmations = 0;
    self.bus.publish(Event::new(
        "task.resume",
        format!("Completion rejected: {count} human.guidance message(s) unacknowledged"),
    ));
    // 不进入 Ok(true) 路径，保持后续正常处理。
} else {
    // 现有的 2-strike 确认逻辑
}
```

### default_publishes guard

In `check_default_publishes`:

```rust
if default_topic == self.config.event_loop.completion_promise {
    warn!(...);
    let resume_event = Event::new("task.resume", ...).with_target(hat_id.clone());
    self.state.record_event(&resume_event);
    self.bus.publish(resume_event);
    return;
}
// 现有注入逻辑
```

### summary_writer test fixture

Add `unacknowledged_guidance: Vec::new()` to the test LoopState construction
so existing tests keep compiling.

## Out of scope (intentionally)

- Auto- ack on first `human.guidance` (origin does not do this; explicit ack is required)
- Re-validation of guidance content
- Multi-user guidance tracking
- Carrying guidance payloads to parallel hat instances (parallel supervisor
  is out of scope for this change)

## Acceptance criteria

1. `human.guidance` event pushed to `unacknowledged_guidance`
2. `human.guidance.ack` event clears `unacknowledged_guidance`
3. Completion with non-empty `unacknowledged_guidance` → rejected, `task.resume` published, counter reset
4. Completion with empty `unacknowledged_guidance` → existing 2-strike behavior preserved
5. `default_publishes == completion_promise` → `task.resume` instead of silent completion
6. HatlessRalph prompt contains the blocking notice
7. `LoopState::default()` initializes empty `unacknowledged_guidance`
8. summary_writer test fixture still compiles

## Compatibility

- No `HatConfig` change
- No new public API
- Existing tests must still pass (2-strike, lazy-model-completion, scratchpad verification)

## Risk

- Touches the same termination path as lazy-model-completion fix (commits
  620411ce, d275c7e6, 39c4a0df). Care needed to avoid regressing those.
- 2-strike pattern + unacknowledged-guidance reset interact: after guidance is
  sent, completion is rejected, so the next loop iteration starts fresh.

## Verification

- Unit tests in `event_loop/tests.rs` (port origin's 163-line test suite):
  - `human.guidance` adds to queue
  - `human.guidance.ack` clears queue
  - Completion blocked when queue non-empty
  - Completion allowed when queue empty
  - `default_publishes == completion_promise` skips injection
  - `default_publishes != completion_promise` still injects (regression)
- `cargo test -p ralph-core --lib` green
- `cargo test -p ralph-cli --lib` green (HatlessRalph prompt tests)
- minimax live E2E: not required for this change (no live-only behavior)
