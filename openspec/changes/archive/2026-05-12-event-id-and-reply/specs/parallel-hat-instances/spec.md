## ADDED Requirements

### Requirement: Every routed event has a stable id
In parallel mode, the system MUST ensure every routed event has a non-empty `Event.id` so that hats and the coordinator can reliably reference and correlate events.

If an agent does not explicitly provide an id, the runtime MUST generate one deterministically within the run before routing and logging.

#### Scenario: Runtime fills missing id for agent-emitted events
- **WHEN** a hat instance outputs `<event topic="build.done">...</event>` without an explicit `id="..."`
- **THEN** the routed event MUST have a non-empty `Event.id`

#### Scenario: Explicit id is preserved end-to-end
- **WHEN** a hat instance outputs `<event topic="build.done" id="custom-1">...</event>`
- **THEN** the routed event MUST have `Event.id="custom-1"`

---

### Requirement: Events can reply to a prior event id
The system MUST support an optional single-valued `Event.reply` field that expresses "this event is a reply to event `<id>`".

When an agent outputs `<event ... reply="<id>">`, the runtime MUST parse and route the `reply` value alongside the event.

#### Scenario: reply attribute is parsed from event tags
- **WHEN** a hat instance outputs `<event topic="review.done" reply="writer#1:7">APPROVED</event>`
- **THEN** the routed event MUST have `Event.reply="writer#1:7"`

#### Scenario: Unknown reply id does not block routing
- **WHEN** a hat instance outputs `<event topic="note" reply="unknown-id">...</event>`
- **THEN** the runtime MUST still route and log the event (reply is best-effort correlation only)

---

### Requirement: Incoming events prompt exposes event ids for referencing
When a job is dispatched to a hat instance in parallel mode, the job prompt MUST expose each incoming event's `id` so the hat can reference it in follow-up replies (`reply="<id>"`).

#### Scenario: Prompt includes incoming event id
- **WHEN** a hat instance is dispatched a job with at least one incoming event that has `Event.id="writer#2:3"`
- **THEN** the job prompt MUST include the substring `id=writer#2:3` for that incoming event
