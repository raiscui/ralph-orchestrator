## MODIFIED Requirements

### Requirement: Trigger-driven fanout routing (parallel mode)
When `parallel.enabled=true`, the system MUST support trigger-driven routing as the default behavior.

If no **configured** `TopicContract` matches an event topic, the system MUST derive a TopicContract by matching `hats.*.triggers` against the event topic, using the same routing priority as the sequential EventBus:
- Specific subscriptions (non-global wildcard) MUST take precedence over global wildcard (`*`) fallback hats.
- If there are no specific subscribers, global wildcard subscribers MUST receive the event as fallback.
- If there are no subscribers at all (no specific AND no wildcard), the event MUST be treated as an orphan and MUST be routed to `ralph#1`.

For default routing, the system MUST fanout-deliver the event to **all** recipient hats (hat-level fanout).

#### Scenario: Two hats subscribed to the same topic run concurrently
- **WHEN** `parallel.enabled=true`, and both `spec_writer` and `spec_reviewer` subscribe to `spec.ready`, and an event with topic `spec.ready` is published
- **THEN** the system MUST dispatch jobs for both hats concurrently as separate headless CLI invocations (subject to the global concurrency cap)

#### Scenario: Manager wildcard subscriber prevents boss escalation
- **WHEN** a hat `manager` subscribes to `"*"` and no hat has a specific subscription for topic `unknown.topic`, and an event with topic `unknown.topic` is published
- **THEN** the event MUST be delivered to `manager` and MUST NOT be additionally delivered to `ralph#1`

#### Scenario: True orphan escalates to ralph#1
- **WHEN** no hat subscribes to topic `unknown.topic` (neither specifically nor via `"*"`) and an event with topic `unknown.topic` is published
- **THEN** the event MUST be delivered to `ralph#1` for coordination
