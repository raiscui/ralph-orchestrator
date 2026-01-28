## ADDED Requirements

### Requirement: Trigger-driven fanout routing (parallel mode)
When `parallel.enabled=true`, the system MUST support trigger-driven routing as the default behavior.

If no explicit `TopicContract` applies to an event topic, the system MUST compute recipient hats by matching `hats.*.triggers` against the event topic, using the same routing priority as the sequential EventBus:
- Specific subscriptions (non-global wildcard) MUST take precedence over global wildcard (`*`) fallback hats.
- If there are no specific subscribers, global wildcard subscribers MAY receive the event as fallback.

For default routing, the system MUST fanout-deliver the event to **all** recipient hats (hat-level fanout).

#### Scenario: Two hats subscribed to the same topic run concurrently
- **WHEN** `parallel.enabled=true`, and both `spec_writer` and `spec_reviewer` subscribe to `spec.ready`, and an event with topic `spec.ready` is published
- **THEN** the system MUST dispatch jobs for both hats concurrently as separate headless CLI invocations (subject to the global concurrency cap)

### Requirement: Hat-level fanout, instance-level queue
When an event is routed to a recipient hat via trigger-driven fanout, the system MUST deliver the event to exactly one `HatInstance` of that hat.

The system MUST NOT fanout-deliver the same event to all instances of a single hat by default.

#### Scenario: A hat with multiple instances receives the event only once
- **WHEN** `writer` has instances `writer#1` and `writer#2`, and an event is routed to the `writer` hat (without `target_instance`)
- **THEN** exactly one of `writer#1` or `writer#2` MUST receive the event for execution

### Requirement: Optional TopicContract override
The system MUST treat `parallel.topic_contracts` as an optional override layer:
- If a `TopicContract` matches `event.topic`, routing MUST follow the contract’s semantics.
- If no `TopicContract` matches, routing MUST fall back to trigger-driven fanout (to hats) and instance-level queue (to one instance per hat).
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

### Requirement: Instance selection is idle-first and deterministic
When selecting a single instance for a recipient hat, the system MUST use an idle-first policy:
- Prefer instances in `Idle` or `Created` state over `Running`.
- If multiple candidates have the same “busy rank”, the system MUST break ties deterministically (e.g., by stable ordering of `HatInstanceId`).

#### Scenario: Idle instance is chosen over running instance
- **WHEN** `writer#1` is `Idle` and `writer#2` is `Running`, and an event is routed to the `writer` hat
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
