# parallel-trigger-routing

## Purpose
Defines the default routing semantics for the parallel runtime when `parallel.enabled=true`:
- trigger-driven routing as the default (when no **configured** TopicContract matches)
- hat-level fanout and instance-level queue
- strict target/target_instance validation (with control-plane exceptions)
- instance selection policy (deterministic by default; may be overridden by queue_selection)
- autoscale with a global concurrency cap
- dynamic instance reaping rules (idle TTL, no ID reuse)
- workspace strategy override and merge rule

## Relationship
This spec is intended to be read alongside `openspec/specs/parallel-hat-instances/spec.md`.

- `parallel-hat-instances` defines the **core model** (TopicContract, audience override, queue_selection logging, etc.).
- This document refines the **default routing behavior** when no configured TopicContract matches.
## Requirements
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

### Requirement: Hat-level fanout, instance-level queue
When an event is routed to a recipient hat via trigger-driven fanout, the system MUST deliver the event to exactly one `HatInstance` of that hat.

The system MUST NOT fanout-deliver the same event to all instances of a single hat by default.

#### Scenario: A hat with multiple instances receives the event only once
- **WHEN** `writer` has instances `writer#1` and `writer#2`, and an event is routed to the `writer` hat (without `target_instance`)
- **THEN** exactly one of `writer#1` or `writer#2` MUST receive the event for execution

### Requirement: Optional TopicContract override
The system MUST treat `parallel.topic_contracts` as an optional override layer:
- If a **configured** `TopicContract` matches `event.topic`, routing MUST follow the contract’s semantics.
- If no configured `TopicContract` matches, routing MUST fall back to trigger-derived semantics (hat-level fanout, instance-level queue).
- `parallel.topic_contracts` MAY be empty without preventing the parallel runtime from starting.

#### Scenario: TopicContract overrides trigger-driven routing
- **WHEN** a `TopicContract` exists for `build.task` that routes only to `ralph#1`
- **THEN** publishing `build.task` MUST route according to that contract, even if other hats subscribe to `build.task`

### Requirement: Strict target and target_instance validation (with control-plane exceptions)
If `event.target` is set, the system MUST validate that the targeted hat is a subscriber of `event.topic`.

If `event.target_instance` is set, the system MUST validate that:
- The target instance exists, and
- The hat owning that instance is a subscriber of `event.topic`.

If validation fails, the system MUST:
- Emit a warning, and
- Escalate the delivery failure (e.g., by emitting a routing escalation event to `ralph#1`), and
- MUST NOT deliver the event to the invalid target hat/instance.

The system MUST support a control-plane exception allowlist for topics that bypass subscription validation (e.g., gate/control events).

#### Scenario: Invalid event.target is rejected
- **WHEN** an event with topic `spec.ready` sets `event.target="non_subscriber_hat"` and that hat is not subscribed to `spec.ready`
- **THEN** the system MUST not deliver the event to `non_subscriber_hat`, and MUST escalate the error for operator visibility

### Requirement: Instance selection policy (default deterministic)
When selecting a single instance for a recipient hat (instance-level queue), the system MUST honor the resolved queue selection policy (see `parallel-hat-instances`):

- For deterministic selection (the default for trigger-derived routing), the system MUST use an idle-first policy:
  - Prefer instances in `Idle` or `Created` state over `Running`.
  - If multiple candidates have the same “busy rank”, the system MUST break ties deterministically (e.g., by stable ordering of `HatInstanceId`).
- For LLM selection (`queue_selection=llm`), selection MAY be non-deterministic, but MUST be logged for replay (candidate set + chosen instance).

#### Scenario: Idle instance is chosen over running instance
- **WHEN** `writer#1` is `Idle` and `writer#2` is `Running`, and an event is routed to the `writer` hat using deterministic selection
- **THEN** the system MUST choose `writer#1` for execution

### Requirement: Autoscale respects a global concurrency cap
If all instances of a recipient hat are busy (`Running`), the system MUST autoscale by creating a new instance for that hat, **as long as** the global number of concurrently running jobs is below the cap.

The system MUST enforce a global concurrency cap on concurrently running hat jobs.
If not explicitly configured, the default cap MUST be **4**.

If the global cap is reached, the system MUST NOT spawn a new instance, and MUST instead queue the work onto an existing instance of that hat (deterministically).

#### Scenario: Autoscale spawns a new instance when below cap
- **WHEN** the `writer` hat has no idle instances, and the global running jobs count is `3`, and the global cap is `4`
- **THEN** the system MUST create a new instance (e.g., `writer#N`) and dispatch the event to it

#### Scenario: Autoscale does not spawn when cap is reached
- **WHEN** the `writer` hat has no idle instances, and the global running jobs count is `4`, and the global cap is `4`
- **THEN** the system MUST NOT create a new instance and MUST queue the work onto an existing `writer` instance

### Requirement: Dynamic instances are reaped after idle TTL, and IDs are never reused
Instances created by autoscale MUST be treated as dynamic instances.

Dynamic instances MUST be automatically shut down and removed after being idle for longer than the idle TTL.
If not explicitly configured, the default idle TTL MUST be **30 seconds**.

When spawning instances for a hat, the instance key MUST be monotonically increasing and MUST NOT be reused.

#### Scenario: Dynamic instance is reaped after 30 seconds idle
- **WHEN** a dynamic instance becomes `Idle` and remains idle for 30 seconds
- **THEN** the system MUST shut down that instance and remove it from the active registry

### Requirement: Workspace strategy override and merge rule
The system MUST support a per-event `workspace_strategy` override (`shared | patch | worktree`).
The override MAY be omitted.

When a hat instance aggregates multiple pending events into a single job, the job’s final workspace strategy MUST be computed using the “strongest isolation wins” merge rule:
`worktree > patch > shared`.

#### Scenario: Worktree override wins over shared default
- **WHEN** a hat’s default workspace strategy is `shared` and an incoming event overrides `workspace_strategy=worktree`
- **THEN** the job MUST run with `worktree` workspace strategy

#### Scenario: Patch wins when combining multiple events
- **WHEN** two events are combined into one job, and one requests `workspace_strategy=shared` and the other requests `workspace_strategy=patch`
- **THEN** the job MUST use `patch` workspace strategy

## Change History
- 2026-01-28: Synced from `openspec/changes/parallel-trigger-routing/specs/parallel-trigger-routing/spec.md`.
