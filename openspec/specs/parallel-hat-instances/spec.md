# parallel-hat-instances

## Purpose
Defines the core requirements for Ralph's parallel runtime:
- parallel execution across hats and instances
- stable instance addressing (`HatInstanceId`)
- headless job execution (external CLI processes)
- explicit routing contracts and audience overrides
- replayable decision logging
- async human gates (optional timeout)
- workspace strategy + permissions enforcement
- replay-based smoke testing
## Requirements
### Requirement: Parallel hat execution
The system MUST support true parallel execution across different hats and across multiple instances of the same hat.

#### Scenario: Writer and tester run concurrently
- **WHEN** `writer#1` is running a job and `tester#1` is dispatched a job at the same time
- **THEN** the system MUST run both jobs concurrently as separate headless CLI invocations

### Requirement: HatInstance addressing
The system MUST assign each running worker a stable `HatInstanceId` in the form `{hat_id}#{instance_key}` for routing, logging, and UI.

#### Scenario: Create a second writer instance
- **WHEN** the system creates a second instance of the `writer` hat
- **THEN** it MUST produce a distinct `HatInstanceId` such as `writer#2` that can be referenced by events

### Requirement: Headless job execution
Each job MUST be executed as a headless external CLI agent process (not a second interactive TUI/PTY), and the system MUST capture stdout/stderr as structured output.

#### Scenario: Job produces stream output
- **WHEN** a job is started for `reviewer#1`
- **THEN** the system MUST capture its stdout/stderr and attribute the output to `reviewer#1`

### Requirement: Explicit topic contracts
For any published event, the system MUST resolve a TopicContract that defines delivery semantics (`queue` or `fanout`) and an audience selector.

The resolved TopicContract MAY come from:
- configured `parallel.topic_contracts`, or
- trigger-derived defaults (see `openspec/specs/parallel-trigger-routing/spec.md`).

#### Scenario: Dispatch uses an explicit contract
- **WHEN** an event with topic `build.task` is published
- **THEN** the system MUST route it according to a TopicContract that explicitly declares `delivery=queue|fanout` and an audience selector

### Requirement: Audience selection and overrides
The system MUST compute the final recipients as `TopicContract.audience ∩ Event.audience_override` (when override is present).

#### Scenario: Audience override narrows recipients
- **WHEN** TopicContract audience selects all `writer#*` instances and an event specifies `audience_override.instances=["writer#2"]`
- **THEN** the final recipients MUST be narrowed to `writer#2` if it exists

### Requirement: Best-effort audience override
If `audience_override.instances=[...]` references a missing instance, the system MUST treat it as best-effort by default and apply `missing_instance_policy` (e.g., spawn/queue/escalate/drop).

#### Scenario: Missing instance does not hard-fail by default
- **WHEN** an event targets `audience_override.instances=["writer#99"]` and `writer#99` does not exist
- **THEN** routing MUST continue using `missing_instance_policy` without treating the event as a delivery failure

### Requirement: Require-delivery override
If an event sets `audience_override.require_delivery=true`, missing targeted instances MUST be treated as a delivery failure and MUST escalate (e.g., by opening a human gate or requesting spawn).

#### Scenario: Require-delivery escalates on missing instance
- **WHEN** an event targets `audience_override.instances=["tester#ci-smoke"]` with `require_delivery=true` and that instance does not exist
- **THEN** the system MUST escalate the failure instead of silently rerouting

### Requirement: Queue selection and decision logging
For `delivery=queue`, the system MUST select exactly one recipient instance. If multiple candidates exist, selection MUST follow `queue_selection=llm|deterministic`, and the system MUST log the candidate set and final selection into the event log for replay.

#### Scenario: LLM queue selection is replayable
- **WHEN** an event with `delivery=queue` has candidates `[writer#1, writer#2]` and uses `queue_selection=llm`
- **THEN** the system MUST record candidates and the chosen instance into the events log so replay does not re-run selection

### Requirement: Fanout delivery
For `delivery=fanout`, the system MUST deliver the event to all recipients selected by the TopicContract audience selector (after applying any audience override).

#### Scenario: Fanout reaches all instances
- **WHEN** an event with `delivery=fanout` targets instances `[writer#1, writer#2]` via the resolved TopicContract audience
- **THEN** both `writer#1` and `writer#2` MUST receive the event

### Requirement: Human gate protocol (async, optional timeout)
The system MUST support an event-based human gate protocol that can (a) wait for human input, and (b) optionally timeout and proceed with a recorded decision, without blocking other HatInstances.

#### Scenario: Gate times out and proceeds
- **WHEN** a gate request is opened with a 60s timeout and no human response arrives within 60s
- **THEN** the system MUST emit a timeout resolution event and proceed with a recorded decision

### Requirement: Workspace strategy and permissions
The system MUST support job-level workspace strategies (at least shared and worktree) and MUST enforce capability/permission checks before acquiring isolated workspaces or performing destructive actions.

#### Scenario: Worktree requires capability/permission
- **WHEN** a hat job requests a worktree workspace
- **THEN** the system MUST verify the hat capability and runtime permission policy before creating or upgrading the worktree

### Requirement: Replay-based smoke testing
The system MUST support replay-based smoke tests using recorded JSONL fixtures to validate parallel orchestration behavior deterministically.

#### Scenario: Replay does not require live backends
- **WHEN** a smoke test replays a recorded fixture
- **THEN** it MUST validate behavior without requiring a live AI backend or re-running LLM decisions

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

## Change History
- 2026-01-28: Synced from `openspec/changes/parallel-hat-instances/specs/parallel-hat-instances/spec.md`.
