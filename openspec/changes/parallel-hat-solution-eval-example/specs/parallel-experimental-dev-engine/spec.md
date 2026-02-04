## ADDED Requirements

### Requirement: 提供可直接运行的并行实验开发配置（ralph.yml）
The repository MUST include a runnable example configuration at `examples/parallel-experimental-dev-engine/` that enables “parallel implement + batch verify + iterative experimentation”, and MUST include both `ralph.yml` and `README.md`.

#### Scenario: 示例文件齐备且可被直接引用
- **WHEN** a developer checks the repository tree
- **THEN** `examples/parallel-experimental-dev-engine/ralph.yml` MUST exist
- **THEN** `examples/parallel-experimental-dev-engine/README.md` MUST exist

---

### Requirement: 示例配置必须显式启用 parallel 并设置安全刹车
The example `ralph.yml` MUST enable `parallel.enabled=true`, and MUST set explicit safety rails for long-running exploratory work (global concurrency cap, idle TTL, gate timeout, max iterations, max runtime).

#### Scenario: 配置包含并行开关与关键 guardrails
- **WHEN** Ralph loads `examples/parallel-experimental-dev-engine/ralph.yml`
- **THEN** `parallel.enabled` MUST be `true`
- **THEN** `parallel.autoscale.max_running_jobs` MUST be set (explicitly or by documented default)
- **THEN** `parallel.autoscale.dynamic_idle_ttl_secs` MUST be set (explicitly or by documented default)
- **THEN** `parallel.gate.default_timeout_secs` MUST be set (explicitly or by documented default)
- **THEN** `event_loop.max_iterations` MUST be set
- **THEN** `event_loop.max_runtime_seconds` MUST be set

---

### Requirement: 示例配置必须定义可收敛的入口与完成语义
The example `ralph.yml` MUST define `event_loop.starting_event`, `event_loop.complete_publishes`, and `event_loop.completion_promise`, so the workflow can converge deterministically instead of stalling.

#### Scenario: workflow entry/exit 语义在配置中可见
- **WHEN** Ralph loads `examples/parallel-experimental-dev-engine/ralph.yml`
- **THEN** `event_loop.starting_event` MUST be set
- **THEN** `event_loop.complete_publishes` MUST be set
- **THEN** `event_loop.completion_promise` MUST be set

---

### Requirement: runner 必须把“实现 + 验证”做成强 backpressure 并上报结构化结果
The example workflow MUST define at least one multi-instance runner hat that is triggered by an “experiment task” topic, performs both implementation and verification, and MUST publish a structured result event that includes verification evidence.

#### Scenario: 单个实验任务必然产出可验证的结果事件
- **WHEN** an experiment task event is delivered to a runner hat instance
- **THEN** the runner MUST publish exactly one result event for that task (success or failure)
- **THEN** the result payload MUST include (at minimum): `run_id`, `experiment_id`, `status`, and `verification_evidence`

---

### Requirement: 并行隔离必须使用 worktree，且结果必须可带回主工作区
The runner hat in the example configuration MUST use `workspace.strategy=worktree` for isolation, and MUST export its work product as an auditable, portable artifact that can be applied in the main workspace. The minimum portable artifact MUST be a unified diff `patch` (commit MAY be included as optional metadata).

#### Scenario: worktree 隔离与产物导出在配置/事件中可观察
- **WHEN** Ralph loads `examples/parallel-experimental-dev-engine/ralph.yml`
- **THEN** the runner hat MUST set `workspace.strategy` to `worktree`
- **WHEN** a runner publishes a result event
- **THEN** the result payload MUST include a `patch` (unified diff)
- **AND** the result payload MAY include a `commit` (git hash)

---

### Requirement: README 必须明确权限与 gate 策略（生产建议：worktree ask，hooks allow）
The example `README.md` MUST document the permission and gate trade-offs, and MUST include a production/team suggestion snippet that sets `parallel.permissions.worktree=ask` while keeping `parallel.permissions.hooks=allow` by default.

#### Scenario: README 包含可复制的权限建议片段
- **WHEN** a developer reads `examples/parallel-experimental-dev-engine/README.md`
- **THEN** it MUST include an example snippet containing `parallel.permissions.worktree: ask`
- **THEN** it MUST include an example snippet containing `parallel.permissions.hooks: allow`

---

### Requirement: 必须引入独立 integrator 在主工作区做采纳与最终验收
The example workflow MUST define an independent integrator hat that is triggered by `integration.task`, applies the selected `patch` in the main workspace, runs final verification, and publishes `integration.applied` or `integration.rejected`. The runner MUST NOT perform main-workspace integration/acceptance work in this workflow.

#### Scenario: 配置包含 integrator 的触发与输出
- **WHEN** Ralph loads `examples/parallel-experimental-dev-engine/ralph.yml`
- **THEN** there MUST be a hat that subscribes to `integration.task`
- **THEN** that hat MUST publish `integration.applied` and `integration.rejected` (or an equivalent success/failure pair)

#### Scenario: integrator 必须在主工作区执行集成
- **WHEN** Ralph loads `examples/parallel-experimental-dev-engine/ralph.yml`
- **THEN** the integrator hat MUST use `workspace.strategy=shared` (main workspace) for integration and acceptance

#### Scenario: 集成结果必须携带可审计证据
- **WHEN** an `integration.applied` event is published
- **THEN** its payload MUST include (at minimum): `run_id`, `experiment_id`, and `verification_evidence`

---

### Requirement: 必须引入独立 auditor 做硬门槛审计（证据不足不得收敛）
The example workflow MUST define an independent auditor hat that is triggered by `experiment.result` and MUST publish `experiment.reviewed`. The auditor MUST enforce a hard evidence gate: missing evidence MUST be reported as `needs_more_evidence` (or an equivalent verdict) and MUST block convergence.

#### Scenario: 审计事件存在且包含最小结构
- **WHEN** an experiment result event is delivered to the auditor
- **THEN** the auditor MUST publish an `experiment.reviewed` event
- **THEN** the review payload MUST include (at minimum): `run_id`, `experiment_id`, and `evidence_ok`

#### Scenario: 证据不足时必须拒绝（硬门槛）
- **WHEN** an experiment result payload is missing required evidence (e.g., `verification_evidence` or `patch`)
- **THEN** the auditor MUST set `evidence_ok=false`
- **THEN** the auditor MUST include a verdict such as `needs_more_evidence` and list what is missing

---

### Requirement: ralph#1 必须自适应决定并行度，并按窗口分批派发实验任务
The workflow MUST implement adaptive concurrency in `ralph#1`: it MUST infer a concurrency ceiling (`P_max`) from the user-provided plan/prompt, MUST adjust the in-flight window `P` during runtime (aggressive start + AIMD-style control), and MUST NOT flood the system by dispatching all tasks at once.

#### Scenario: 并行窗口必须为控制面预留 slot
- **WHEN** `ralph#1` chooses a runtime in-flight window size `P`
- **THEN** `P` MUST satisfy `P <= parallel.autoscale.max_running_jobs - 2` (reserve capacity for `ralph#1` + auditor)

#### Scenario: 完成判定必须以审计为准
- **WHEN** `ralph#1` decides whether an experiment is “done”
- **THEN** it MUST treat `experiment.reviewed` with `evidence_ok=true` as the completion signal for that experiment (not `experiment.result` alone)

---

### Requirement: 收敛必须经过主工作区集成验收（integration.applied gate）
The workflow MUST require a main-workspace integration/acceptance gate: after all experiments have passed auditor review, `ralph#1` MUST publish an `integration.task` to the integrator, and MUST NOT converge (publish `experiment.complete` or output the completion promise) until an `integration.applied` event is observed.

#### Scenario: 通过审计后必须触发 integration.task
- **WHEN** all experiments in the user plan have produced `experiment.reviewed` with `evidence_ok=true`
- **THEN** `ralph#1` MUST publish an `integration.task` event

#### Scenario: 未集成通过前不得收敛
- **WHEN** `integration.applied` has not been published for the selected experiment
- **THEN** the workflow MUST NOT publish `experiment.complete`
- **AND** the workflow MUST NOT output the completion promise

---

### Requirement: 必须提供 replay fixture 用于端到端回放验证
The repository MUST include a replay fixture that replays an end-to-end run of the example workflow, so CI/smoke tests can validate the configuration deterministically without live backends.

#### Scenario: 存在可回放的 JSONL fixture
- **WHEN** a developer runs the replay-based smoke tests
- **THEN** the repository MUST include a JSONL fixture under `crates/ralph-core/tests/fixtures/` for this workflow

---

### Requirement: 必须提供该 example 的真后端 E2E 场景（Codex）
The repository MUST include a dedicated `ralph-e2e` scenario that directly runs `examples/parallel-experimental-dev-engine/` against the Codex backend, and MUST assert the workflow emits the critical topic chain, includes an auditable `patch`, and converges to `LOOP_COMPLETE`.

#### Scenario: E2E 场景存在且只支持 Codex
- **WHEN** a developer lists E2E scenarios
- **THEN** there MUST be an E2E scenario for `parallel-experimental-dev-engine`
- **AND** it MUST support `Backend::Codex` (and MAY restrict to Codex only)

#### Scenario: E2E 断言覆盖关键 topic 链路与 patch
- **WHEN** the E2E scenario runs the example workflow
- **THEN** it MUST observe events including:
  - `experiment.start`
  - `experiment.task`
  - `experiment.result` (payload MUST include `patch`)
  - `experiment.reviewed` (payload MUST indicate `evidence_ok=true`)
  - `integration.task`
  - `integration.applied`
  - `experiment.complete`
- **AND** the run MUST converge to `LOOP_COMPLETE` (exit successfully rather than timing out)
