# hat-request-reply-channel

## Purpose
Defines the explicit hat-to-hat answer-return channel for Ralph's parallel runtime.

This spec separates:
- ordinary workflow progression events
- requester-return answer events

It ensures a hat can return an answer to the original requesting hat instance without turning every `reply` association into an automatic return path.

## Requirements
### Requirement: Hat answer-return channel is explicit and opt-in
The system MUST treat `reply.hat.message` as the dedicated hat-to-hat answer-return topic.

Only events that explicitly use topic `reply.hat.message` and carry a non-empty `reply="<request_event_id>"` participate in requester-return semantics.
The system MUST NOT automatically convert ordinary workflow events into requester replies.

#### Scenario: Explicit answer-return enters requester-return routing
- **WHEN** a hat instance emits `<event topic="reply.hat.message" reply="writer#1:7">...</event>`
- **THEN** the runtime treats the event as a requester-return answer candidate

#### Scenario: Ordinary workflow event does not auto-return to requester
- **WHEN** a hat instance emits `<event topic="research.ready" reply="writer#1:7">...</event>`
- **THEN** the runtime MUST keep routing it as a normal workflow event
- **THEN** the runtime MUST NOT reinterpret it as a requester-return answer solely because `reply` is present

### Requirement: Answer-return targets the original requester instance
The system MUST resolve the delivery target for `reply.hat.message` from the referenced request event's `source_instance`.

If the referenced request event was originally published by `writer#1`, the returned answer MUST be delivered to `writer#1` and MUST NOT be broadcast to other hats.

#### Scenario: Answer returns to the requesting instance
- **GIVEN** `writer#1` previously published a request event with `id="req-1"`
- **WHEN** `explorer#1` emits `<event topic="reply.hat.message" reply="req-1">answer</event>`
- **THEN** the runtime MUST deliver that answer event to `writer#1`
- **THEN** the runtime MUST NOT fanout that answer to unrelated instances

### Requirement: Answer-return can coexist with workflow events
The system MUST allow a called hat to publish requester-return answers and ordinary workflow events without forcing them into the same channel.

This means a hat MAY answer the requester and also continue the broader workflow, and the runtime MUST route each event according to its own semantics.

#### Scenario: Callee answers requester and continues workflow
- **GIVEN** `planner#1` asks `researcher#1` for a market check using request event `id="req-2"`
- **WHEN** `researcher#1` emits both `<event topic="reply.hat.message" reply="req-2">market summary</event>` and `<event topic="research.ready">done</event>`
- **THEN** the runtime MUST deliver the first event back to `planner#1`
- **THEN** the runtime MUST route `research.ready` through normal workflow routing

### Requirement: Unresolvable answer-return fails closed
The system MUST fail closed when `reply.hat.message` cannot be resolved back to a requesting hat instance.

If the referenced event id is unknown, or the referenced event has no `source_instance`, the runtime MUST NOT broadcast, fanout, or silently reroute the answer as a normal workflow event.
The runtime MUST record that the requester-return resolution failed.

#### Scenario: Unknown reply id does not leak as a workflow event
- **WHEN** a hat instance emits `<event topic="reply.hat.message" reply="missing-id">answer</event>`
- **THEN** the runtime MUST NOT deliver that event to unrelated hats
- **THEN** the runtime MUST record that requester-return resolution failed for `missing-id`

#### Scenario: Reply to non-hat source does not auto-route to hats
- **GIVEN** the referenced event exists but has no `source_instance`
- **WHEN** a hat instance emits `<event topic="reply.hat.message" reply="external-1">answer</event>`
- **THEN** the runtime MUST NOT reinterpret the answer as a normal workflow event
- **THEN** the runtime MUST record that no requesting hat instance could be resolved

### Requirement: Delivered answer-return remains correlatable
The system MUST preserve the original `reply="<request_event_id>"` relationship on delivered `reply.hat.message` events so the requester can correlate the answer with its request.

The requester-facing incoming event MUST keep the topic `reply.hat.message` and the original `reply` value.

#### Scenario: Requester receives an answer with preserved correlation
- **GIVEN** `writer#1` published request event `id="req-3"`
- **WHEN** `explorer#1` emits `<event topic="reply.hat.message" reply="req-3">summary</event>` and the runtime delivers it to `writer#1`
- **THEN** `writer#1` MUST receive an incoming event with topic `reply.hat.message`
- **THEN** that incoming event MUST still contain `reply="req-3"`

## Change History
- 2026-03-13: Synced from `openspec/changes/hat-request-reply-channel/specs/hat-request-reply-channel/spec.md`.
