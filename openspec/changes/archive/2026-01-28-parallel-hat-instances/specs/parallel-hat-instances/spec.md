## ADDED Requirements

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
For any published event, the system MUST resolve an explicit TopicContract that defines delivery semantics (`queue` or `fanout`) and an audience selector.

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
For `delivery=fanout`, the system MUST deliver the event to all recipient instances.

#### Scenario: Fanout reaches all instances
- **WHEN** an event with `delivery=fanout` targets instances `[writer#1, writer#2]`
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
