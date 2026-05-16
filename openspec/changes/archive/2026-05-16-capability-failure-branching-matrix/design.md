# Design: capability failure branching matrix

## Context

Ralph now returns parent-visible `capability.failed` events with `failure_class`.
The runtime also already has two useful pre-invocation classes:

- `invalid_capability_id`: request shape is valid, but selected capability is unknown.
- `malformed_request`: request payload is missing required structured fields.

B.4 should prove that parent policy can use these classes differently.

## Minimal matrix

| failure_class | parent policy shape | retry engine? | final human reply? |
| --- | --- | --- | --- |
| `invalid_capability_id` | emit explicit fallback `capability.request` | no | after fallback success, explicit `reply.human.message` |
| `malformed_request` | emit diagnostic `reply.human.message` | no | immediately explicit diagnostic reply |

## Non-goals

- No generic retry engine.
- No planner or broker.
- No live parent topology mutation.
- No test-only child failure injection switch.
- No automatic conversion from `capability.failed` to a human-visible answer.

## Evidence strategy

- Existing B.3 live gate continues proving `invalid_capability_id -> fallback request -> capability.result -> reply.human.message`.
- New B.4 live gate emits a malformed `capability.request`, waits until parent sees `malformed_request`, then emits explicit diagnostic `reply.human.message` and completes.
- The new gate asserts there is no fallback `capability.result` for the malformed branch.
