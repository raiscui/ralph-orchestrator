## ADDED Requirements

### Requirement: Parallel workflow always starts from task.start / task.resume
When `parallel.enabled=true`, the system MUST publish and route exactly one start event before any other workflow events:

- Fresh start MUST publish `task.start`
- Resume start MUST publish `task.resume`

The start event payload MUST contain the top-level prompt prelude (e.g. content from `-P PROMPT.md`).

#### Scenario: Fresh run publishes task.start first
- **WHEN** a user runs `ralph run` with `parallel.enabled=true` in non-resume mode
- **THEN** the first routed event MUST be `task.start` and its payload MUST contain the top-level prompt prelude

#### Scenario: Resume publishes task.resume first
- **WHEN** a user runs `ralph resume` (or equivalent) with `parallel.enabled=true`
- **THEN** the first routed event MUST be `task.resume` and its payload MUST contain the top-level prompt prelude

### Requirement: task.start and task.resume are control-plane topics routed to ralph#1
In parallel mode, `task.start` and `task.resume` MUST be treated as control-plane topics.

The system MUST route these topics to `ralph#1` (the coordinator) so that:
- The coordinator always sees the user objective
- Other hats do not get polluted by the top-level prompt prelude

#### Scenario: task.start is delivered to ralph#1 only
- **WHEN** `parallel.enabled=true` and `task.start` is published
- **THEN** the event MUST be delivered to `ralph#1`

### Requirement: starting_event is the workflow entry event after coordination
The `event_loop.starting_event` field MUST be defined as an optional **workflow entry event topic**.

It MUST NOT be treated as the runtime’s first event.
The runtime’s first event is always `task.start` (or `task.resume`).

#### Scenario: starting_event does not change the runtime start topic
- **WHEN** `parallel.enabled=true` and `event_loop.starting_event` is set to `"science.start"`
- **THEN** the first routed event MUST still be `task.start` (or `task.resume` in resume mode), not `science.start`

### Requirement: Coordinator publishes the workflow entry event on fresh start
On a fresh start (i.e. after receiving `task.start`), the coordinator (`ralph#1`) MUST publish at least one workflow entry event to begin the hat workflow.

If `event_loop.starting_event` is configured, the first workflow entry event MUST use that topic.

#### Scenario: starting_event drives the first delegated event
- **WHEN** `parallel.enabled=true` and `event_loop.starting_event` is set to `"science.start"`, and the coordinator receives `task.start`
- **THEN** the coordinator MUST publish an event with topic `science.start` to begin the workflow

### Requirement: Workflow completion topic is declared by event_loop.complete_publishes
The config MUST support a single workflow completion event topic under `event_loop`:

`event_loop.complete_publishes: "<topic>"`

This value MUST be treated as the workflow’s “completion candidate event”.

#### Scenario: complete_publishes declares a single completion topic
- **WHEN** a workflow config sets `event_loop.complete_publishes: "fix.applied"`
- **THEN** the system MUST treat `fix.applied` as the workflow completion candidate event topic for that run

### Requirement: Only ralph#1 can end a parallel run via completion_promise
In parallel mode, the runtime MUST treat the loop as complete only when the coordinator (`ralph#1`) outputs the configured `event_loop.completion_promise`.

Output from any non-ralph hat MUST NOT be treated as satisfying `completion_promise`.

#### Scenario: Worker output cannot terminate the run
- **WHEN** `parallel.enabled=true` and a non-ralph hat outputs the string equal to `event_loop.completion_promise`
- **THEN** the runtime MUST NOT terminate solely based on that output

#### Scenario: Ralph output terminates the run
- **WHEN** `parallel.enabled=true` and `ralph#1` outputs the string equal to `event_loop.completion_promise`
- **THEN** the runtime MUST terminate after completing any configured drain window

### Requirement: Coordinator decides whether completion candidate ends the workflow
When the coordinator (`ralph#1`) observes an event whose topic matches `event_loop.complete_publishes`, it MUST decide whether to end the run by outputting `event_loop.completion_promise` (e.g. `LOOP_COMPLETE`).

#### Scenario: Completion event triggers coordinator-controlled shutdown
- **WHEN** `event_loop.complete_publishes: "fix.applied"` and an event with topic `fix.applied` is delivered to `ralph#1`
- **THEN** `ralph#1` MUST either (a) output `event_loop.completion_promise` to end the run, or (b) publish follow-up events and continue the workflow
