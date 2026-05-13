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

### Requirement: Supervisor shutdown terminates all worker CLI processes
When the parallel supervisor is stopping (due to user quit, interrupt, or cancellation), the system MUST terminate all headless worker CLI processes started for HatJobs, and MUST ensure no orphan processes remain.

#### Scenario: User quits TUI while workers are running
- **WHEN** the user presses `q` in the Supervisor TUI while at least one HatJob process is still running
- **THEN** the runtime terminates those CLI processes (graceful first, then force-kill after a timeout)
- **THEN** the Ralph process exits without leaving orphan worker processes

#### Scenario: Supervisor shutdown does not leak processes
- **WHEN** the supervisor transitions to a terminal shutdown state
- **THEN** all child HatJob processes are terminated and reaped before the supervisor returns

---

### Requirement: 并行模式下所有 hats 默认订阅 human.message
When `parallel.enabled: true`, the system MUST ensure every configured hat subscribes to topic `human.message`, even if it is not explicitly listed in `hats.<id>.triggers`.

说明：
- 目的：保证 Supervisor 的 strict target 校验下，`human.message(target_instance=writer#2)` 这种“实例直达”不会因为“hat 未订阅该 topic”而被拒绝。
- 该规则只要求“订阅存在”，并不要求 `human.message` 必须 broadcast；事件是否 fanout 仍由 `target_instance` / contracts / triggers 决定。

#### Scenario: 并行模式自动补齐 human.message 订阅
- **GIVEN** 配置启用了 `parallel.enabled: true`
- **AND** 某个 hat（例如 `writer`）未显式配置 `triggers: ["human.message"]`
- **WHEN** 系统启动并行 Supervisor
- **THEN** `writer` 在运行时视为已订阅 `human.message`

### Requirement: Parallel runtime supports session_strategy=app_server
在并行模式下,事件 MUST 支持显式声明 `session_strategy="app_server"`.
当某次 job 合并的 pending events 中存在任意 `app_server` 请求时,该 job MUST 以 `app_server` 会话形态执行.

同时系统 MUST 保持 sticky(只升级不降级)规则,按强弱排序:

`exec < mcp < app_server`.

#### Scenario: Event requests app_server session
- **WHEN** 某个 hat instance 输出 `<event topic="build.task" session_strategy="app_server">...</event>`
- **THEN** 并行运行时 MUST 将该事件解析为 `Event.session_strategy=app_server`
- **THEN** 该事件被路由到的实例在执行对应 job 时 MUST 选择 `app_server` 会话形态

---

### Requirement: App Server turn control supports steer and interrupt
在 `session_strategy=app_server` 下,系统 MUST 支持 turn 级控制语义:

- `turn_action="start"`: 新开 turn(默认行为).
- `turn_action="steer"`: 对 in-flight turn 追加输入,使用 App Server 的 `turn/steer`.
- `turn_action="interrupt"`: 中断当前 turn,使用 App Server 的 `turn/interrupt`.

#### Scenario: In-flight steer appends input to the same turn
- **GIVEN** 某个实例正在以 `session_strategy=app_server` 执行 job,并存在 in-flight turn
- **WHEN** 系统投递一条带 `turn_action="steer"` 的事件到该实例
- **THEN** 运行时 MUST 对该实例执行 `turn/steer`(而不是等本轮结束再新开 turn)

#### Scenario: Interrupt cancels only the active turn
- **GIVEN** 某个实例正在以 `session_strategy=app_server` 执行 job,并存在 in-flight turn
- **WHEN** 系统投递一条带 `turn_action="interrupt"` 的事件到该实例
- **THEN** 运行时 MUST 执行 `turn/interrupt` 来中断当前 turn

---

### Requirement: Steer degrades safely when no in-flight turn exists
当 `turn_action="steer"` 被投递到一个没有 in-flight turn 的实例时(例如实例空闲,或当前 job 非 app_server),系统 MUST 采取安全降级策略:

- 不丢消息.
- 允许该输入在后续 turn 被处理(例如作为下一次 job 的普通 pending event).

#### Scenario: Steer is queued when no active turn exists
- **GIVEN** 目标实例当前不存在 in-flight turn
- **WHEN** 系统投递一条带 `turn_action="steer"` 的事件到该实例
- **THEN** 系统 MUST 不丢弃该事件,并保证其仍会在后续执行中被处理(以 best-effort 方式)

### Requirement: External turn_action steer/interrupt are reserved for ExternalInput to ralph#1
For out-of-band external events (JSONL ingest via `ralph emit` or Supervisor TUI), the system MUST treat `turn_action=steer|interrupt` as a control-plane signal reserved for ExternalInput and deliverable only to `ralph#1`.

#### Scenario: Hat job cannot emit steer/interrupt via ralph emit
- **GIVEN** a headless hat job environment where `RALPH_HAT_INSTANCE_ID` is set
- **WHEN** the job runs `ralph emit human.message "..." --turn-action steer --target-instance ralph#1`
- **THEN** the `ralph emit` command MUST exit non-zero
- **THEN** the external events file MUST NOT contain a new event line with `turn_action="steer"`

#### Scenario: Valid control-plane event is delivered only to ralph#1
- **WHEN** the Supervisor ingests an external JSONL event with `turn_action="steer"` and `target_instance="ralph#1"`
- **THEN** the system MUST deliver the event to `ralph#1`
- **THEN** the system MUST NOT deliver the event to any non-`ralph#1` instance

---

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

## Change History
- 2026-01-28: Synced from `openspec/changes/parallel-hat-instances/specs/parallel-hat-instances/spec.md`.
- 2026-03-15: Synced from `openspec/changes/event-id-and-reply/specs/parallel-hat-instances/spec.md`.
