## [2026-05-28 16:00:50] [Session ID: omx-1779954714247-oab9zc] 笔记: notes.md 超限续档与当前 recoverable retry 5.x 接续点

## 来源

### 来源1: 旧 notes 归档文件

- 路径: `archive/default_history/notes_2026-05-28_1559_pre_recoverable_retry_5x.md`
- 原始行数: 1109
- 续档原因: `notes.md` 已超过 1000 行,继续追加会违反六文件上下文规则。

### 来源2: 当前 task_plan 最新状态

- 当前 OpenSpec change: `agent-cli-recoverable-failure-retry`
- 已完成: 1.x core model,2.x ledger/replay,3.x runtime retry lifecycle,4.x manual continue control path。
- 正在进行: 5.x Human-facing evidence and observability。
- 当前卡点: `test_agents_command_prints_recoverable_summary` 断言 `stdout.contains("failure-wr")` 过强,因为表格会截断 `failure_id`。

## 综合发现

### 旧 notes 可复用结论

- 旧 notes 主要沉淀的是 clean live dogfood、dynamic role contract、completed dynamic instances、record-session 与 agents snapshot 差异。
- 这些长期经验已基本进入 `EXPERIENCE.md` 中的 runtime evidence / clean dogfood 相关条目。
- 本次续档没有发现必须新增 skill 的新规律。当前更重要的是让上下文文件回到可继续追加状态。

### 当前继续点

- 先修复 integration_agents 的过强断言,不要因为表格截断行为而让测试依赖不稳定字符串。
- 然后继续补齐 5.x: agents snapshot / `ralph agents` / record summary 或 evidence inspect 三个观察面。
- 最后进入 6.x integration guardrails 和最终 `cargo test`。

### 注意事项

- 后续若需要记录 bug 修复到 `ERRORFIX.md`,该文件当前已经到达 1000 行临界点,追加前应先续档或新开当前入口。
- 后续 Markdown 追加含反引号内容时继续使用 Python 写入或 quoted heredoc。

## [2026-05-28 16:17:30] [Session ID: omx-1779954714247-oab9zc] 笔记: recoverable summary 表格断言复现

## 现象

- focused 命令: `cargo test -p ralph-cli --test integration_agents test_agents_command_prints_recoverable_summary -- --exact --nocapture`。
- 结果: 失败,断言位置为 `crates/ralph-cli/tests/integration_agents.rs:179`,失败断言是 `stdout.contains("failu")`。
- 最小输出探针显示 `ralph agents` 的 Recoverable 列为 `retry_scheduled:1/3...`。

## 假设

### 主假设

- 现有测试绑定了紧凑表格里的 failure id 前缀,但表格列宽 20 字符会先保留 status 和 attempt,导致 failure id 被省略。
- 因此该断言过强,应该改成验证稳定的人类可见信号: `Recoverable` 列、实例 `writer#1`、状态 `retry_scheduled`、attempt `1/3`。

### 备选解释

- `ralph agents` 的紧凑列本身不满足 5.2 对 failure id / next retry timing 的可解释性。
- 推翻主假设的证据会是: agents snapshot 或 evidence inspect 没有完整 `failure_id` / `next_retry_at` / ledger path,导致只有紧凑表格可用。

## 当前验证结论

- `AgentRecoverableFailureSummary` 和 `.ralph/agents.json` 结构已保存完整 `failure_id`、attempt、`next_retry_at`、ledger path。
- `record_session` 的 Evidence Inspect 渲染测试已断言 scheduled / continued / exhausted 三类 full `failure_id` 与 status。
- 所以本轮对 `ralph agents` 集成测试应避免断言紧凑列中完整/前缀 failure id,但仍要断言它显示 recoverable 状态和 attempt。

## [2026-05-28 16:27:27] [Session ID: omx-1779954714247-oab9zc] 笔记: recoverable Evidence Inspect focused test 修复结果

## 已验证

- `record_session::tests::aggregate_collects_evidence_inspect` 正确 target 是 `cargo test -p ralph-cli --bin ralph ...`,不是 `--lib`。
- 编译错误来自 `TopologySpawnedInstance` fixture 上误放 `recoverable_failures` 字段。
- 删除误放字段后,focused test 通过。

## 结论

- recoverable scheduled / continued / exhausted 的 Evidence Inspect 断言仍在 `AgentInstanceSnapshot.recoverable_failures` 上覆盖。
- 该修复不改变 runtime,只是恢复测试 fixture 与真实数据结构一致。

## [2026-05-29 00:08:00] [Session ID: omx-1779954714247-oab9zc] 笔记: WORKLOG 超限续档前持续学习摘要

### 触发条件
- `WORKLOG.md` 已达到 1002 行,超过六文件 1000 行阈值。
- 当前主任务要 review 大范围未提交实现改动,继续追加会增加上下文风险。

### 六文件摘要
- 默认组当前主线: `agent-cli-recoverable-failure-retry` 已完成 5.x/6.x、OpenSpec 归档、guidance skill 同步和本地 guidance commit。
- `WORKLOG.md` 最近记录显示 recoverable failure retry 已通过 focused tests、package tests、workspace `cargo test --quiet`、OpenSpec strict validate 与 `git diff --check`。
- 最近新增经验: recoverable failure retry 的真相源是 `.ralph/recoverable-failures.jsonl`; agents snapshot 是 summary-only; stderr 只可观测不参与 event parsing; manual continue 复用同一 retry path。
- 本轮执行错误: 搜索包含反引号的 Markdown pattern 时必须用单引号,否则 zsh 会 command substitution。

### 可复用沉淀
- 已沉淀到 `.codex/skills/self-learning.ralph-agent-cli-recoverable-failure-retry/SKILL.md`。
- 已在 `AGENTS.md` Project Knowledge Index 中索引该 skill。
- `EXPERIENCE.md` 已更正旧 `no-delta` 阻断口径。

### 归档计划
- 将旧 `WORKLOG.md` 移至 `archive/default_history/WORKLOG_2026-05-29_0008_pre_review.md`。
- 新建轻量 `WORKLOG.md`,记录续档原因和当前 review 主线入口。

## [2026-05-29 00:16:00] [Session ID: omx-1779954714247-oab9zc] 笔记: recoverable failure retry 实现改动 focused review

### Review scope
- 只审查未提交工作区中与 `agent-cli-recoverable-failure-retry` 主链路直接相关的实现和观察面。
- 已排除本轮 guidance commit `2bf2aba5`。
- 当前 worktree 同时包含 topology/runtime evidence、TUI、E2E、docs、上下文归档等多条支线,不能作为单一 feature commit 直接提交。

### 静态证据
- `crates/ralph-core/src/recoverable_failure.rs`: ledger、classifier、retry policy 类型和 replay。
- `crates/ralph-core/src/parallel/instance.rs`: scheduled retry、manual continue、exhausted、recovered 状态机。
- `crates/ralph-core/src/parallel/supervisor.rs`: pending recoverable 阻止 completion promise、manual continue routing、agents snapshot summary。
- `crates/ralph-cli/src/record_session.rs`: Evidence Inspect 渲染 recoverable failures。
- `crates/ralph-cli/src/display.rs`: `ralph agents` compact recoverable summary。

### 动态验证
- `cargo test -p ralph-core --lib recoverable --quiet`: passed,32 tests。
- `cargo test -p ralph-core --lib parallel::instance::tests::recoverable_failure_schedules_retry_and_preserves_stdout_only_parsing -- --exact --nocapture`: passed。
- `cargo test -p ralph-core --lib parallel::instance::tests::manual_continue_appends_transition_and_uses_scheduled_retry_path -- --exact --nocapture`: passed。
- `cargo test -p ralph-core --lib parallel::instance::tests::recoverable_failure_exhaustion_becomes_terminal_with_ledger_pointer -- --exact --nocapture`: passed。
- `cargo test -p ralph-core --lib parallel::supervisor::routing_tests::agents_snapshot_includes_recoverable_failure_summaries -- --exact --nocapture`: passed。
- `cargo test -p ralph-cli --bin ralph record_session::tests::aggregate_collects_evidence_inspect -- --exact --nocapture`: passed。
- `git diff --check`: passed。

### Review findings
- 未发现 recoverable retry focused path 的直接 correctness blocker。
- 关注点 1: `.ralph/recoverable-failures.jsonl` append 没有显式跨 instance 写锁。当前测试覆盖顺序 append / replay / malformed line,但没有并发 append 压测。建议补一个并发 append regression test,或者通过 command queue / async mutex 串行化 ledger 写入。
- 关注点 2: `maybe_start_scheduled_retry()` 在 `scheduled.take()` 后才执行 worktree acquire。如果 acquire 失败,actor 会报 Failed,但 scheduled retry context 已被取走。建议在后续 hardening 中先 acquire 再 take,或失败时恢复 scheduled context。
- 关注点 3: `AgentCompletedDynamicInstanceSnapshot` 不包含 `recoverable_failures`; Evidence Inspect 的 `render_recoverable_failures()` 只遍历 current `snapshot.instances`。如果动态实例在 recoverable terminal 后被 tombstone,可能出现 completed tombstone 可见但 recoverable summary 不在该段展示的观察盲点。建议补动态实例 + exhausted recoverable + tombstone 场景测试,再决定是否把 recoverable summary 带入 tombstone 或单独按 ledger 渲染。

### 提交边界建议
- 单独提交 recoverable retry 实现本体: core recoverable module、config、instance/supervisor wiring、CLI display/record summary、对应 tests、OpenSpec archive/stable spec。
- 不要把 topology/runtime evidence、TUI display overhaul、E2E examples 大批量变更和该实现本体混成一个 commit。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 笔记: 4.x Manual continue control path 复核

### 现象
- 用户要求继续 `4.x: Manual continue control path`。
- 当前 active OpenSpec list 中已经没有 `agent-cli-recoverable-failure-retry`。
- 对应 change 位于 `openspec/changes/archive/2026-05-28-agent-cli-recoverable-failure-retry/`。
- archived `tasks.md` 中 4.x 的 5 个任务均为 `[x]`。

### 静态证据
- `openspec/changes/archive/2026-05-28-agent-cli-recoverable-failure-retry/tasks.md`:
  - 4.1 explicit `!continue` / `!continue <failure_id>` control intent 已完成。
  - 4.2 parser tests 已完成。
  - 4.3 explicit failure_id resolve/reject 已完成。
  - 4.4 bare `!continue` ambiguity handling 已完成。
  - 4.5 `continued_by_human` transition + same scheduler path 已完成。
- 当前代码符号位置:
  - `crates/ralph-core/src/parallel/supervisor.rs`: `handle_recoverable_continue_event`, `resolve_explicit_recoverable_continue`, `resolve_bare_recoverable_continue`。
  - `crates/ralph-core/src/parallel/instance.rs`: `continue_recoverable_failure`, `manual_continue_appends_transition_and_uses_scheduled_retry_path`。

### 动态验证
- 首轮尝试运行旧测试名时出现 `running 0 tests`,该结果不能算通过证据。
- 已改用当前真实测试名重跑:
  - `cargo test -p ralph-core --lib parallel::supervisor::routing_tests::explicit_recoverable_continue_accepts_only_waiting_failures -- --exact --nocapture`: 1 passed。
  - `cargo test -p ralph-core --lib parallel::supervisor::routing_tests::bare_recoverable_continue_uses_selected_instance_to_disambiguate -- --exact --nocapture`: 1 passed。
  - `cargo test -p ralph-core --lib parallel::supervisor::routing_tests::bare_recoverable_continue_falls_back_to_global_unique_when_selected_has_no_failure -- --exact --nocapture`: 1 passed。
  - `cargo test -p ralph-core --lib parallel::instance::tests::manual_continue_appends_transition_and_uses_scheduled_retry_path -- --exact --nocapture`: 1 passed。

### 结论
- 4.x Manual continue control path 当前不是未完成任务,而是已归档并通过 focused 验证的完成项。
- 如果用户继续推进这条线,更合理的下一步不是重做 4.x,而是处理未提交改动拆分提交 / hardening 点 / 或切到其他 active change。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 笔记: recoverable retry 实现本体提交前验证证据

### 目标
- hook 要求继续任务并收集 fresh verification evidence。
- 4.x 已确认完成,所以本轮顺着下一步做 recoverable retry 实现本体的 scoped pre-commit review。

### 当前工作区风险
- `git status --short | wc -l` 显示当前有 167 个改动项。
- 这些改动跨 recoverable retry、topology runtime evidence、TUI、E2E、docs、上下文归档等多条线。
- 因此不能整仓 stage/commit,只能按功能线拆分。

### recoverable retry 候选边界
- 核心实现:
  - `crates/ralph-core/src/recoverable_failure.rs`
  - `crates/ralph-core/src/config.rs`
  - `crates/ralph-core/src/lib.rs`
  - `crates/ralph-core/src/agents_snapshot.rs`
  - `crates/ralph-core/src/parallel/instance.rs`
  - `crates/ralph-core/src/parallel/supervisor.rs`
  - `crates/ralph-core/src/parallel/supervisor/routing.rs`
  - `crates/ralph-core/src/parallel/supervisor/routing_tests.rs`
- CLI/观察面:
  - `crates/ralph-cli/src/display.rs`
  - `crates/ralph-cli/src/record_session.rs`
  - `crates/ralph-cli/tests/integration_agents.rs`
  - `crates/ralph-cli/tests/integration_record_session.rs`
- 规格:
  - `openspec/specs/agent-cli-recoverable-failure-retry/spec.md`
  - `openspec/specs/parallel-hat-instances/spec.md`
  - `openspec/specs/supervisor-human-chat-gate/spec.md`
  - `openspec/changes/archive/2026-05-28-agent-cli-recoverable-failure-retry/`

### 新验证证据
- `cargo test -p ralph-core --lib recoverable --quiet`: 32 passed。
- `cargo test -p ralph-core --lib parallel::instance::tests::recoverable_failure_schedules_retry_and_preserves_stdout_only_parsing -- --exact --nocapture`: 1 passed。
- `cargo test -p ralph-core --lib parallel::instance::tests::manual_continue_appends_transition_and_uses_scheduled_retry_path -- --exact --nocapture`: 1 passed。
- `cargo test -p ralph-core --lib parallel::instance::tests::recoverable_failure_exhaustion_becomes_terminal_with_ledger_pointer -- --exact --nocapture`: 1 passed。
- `cargo test -p ralph-core --lib parallel::supervisor::routing_tests::agents_snapshot_includes_recoverable_failure_summaries -- --exact --nocapture`: 1 passed。
- `cargo test -p ralph-core --lib parallel::supervisor::routing_tests::explicit_recoverable_continue_accepts_only_waiting_failures -- --exact --nocapture`: 1 passed。
- `cargo test -p ralph-core --lib parallel::supervisor::routing_tests::bare_recoverable_continue_uses_selected_instance_to_disambiguate -- --exact --nocapture`: 1 passed。
- `cargo test -p ralph-core --lib parallel::supervisor::routing_tests::bare_recoverable_continue_falls_back_to_global_unique_when_selected_has_no_failure -- --exact --nocapture`: 1 passed。
- `cargo test -p ralph-cli --test integration_agents test_agents_command_prints_recoverable_summary -- --exact --nocapture`: 1 passed。
- `cargo test -p ralph-cli --bin ralph record_session::tests::aggregate_collects_evidence_inspect -- --exact --nocapture`: 1 passed。
- `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate --all --strict`: 28 passed,0 failed。
- `git diff --check`: passed。

### 结论
- recoverable retry 主链路 scoped gates 继续通过。
- 仍不建议直接整仓提交,因为工作区有大量无关支线改动。
- 下一步如果继续收口,应先生成 recoverable retry scoped file list,逐文件 diff review,再只 stage 该范围。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 笔记: recoverable retry scoped diff 边界审查

### 审查目标
- 当前工作区混有 167 个改动项,不能整仓提交。
- 本轮审查 recoverable retry 候选文件是否可以整文件 stage,还是需要 patch-stage / 排除。

### 可以整文件 stage 的候选
- `crates/ralph-core/src/recoverable_failure.rs`
  - 新增 recoverable failure ledger / classifier / retry delay / tests。
  - 静态扫描未发现 topology/capability/TUI 支线语义。
- `openspec/specs/agent-cli-recoverable-failure-retry/spec.md`
  - 新增稳定 spec,内容集中在 recoverable failure ledger / classifier / retry / manual continue。
  - 注意 Purpose 仍是 archive 生成的 `TBD`,可以后续清理,但不阻断当前 strict validate。
- `openspec/changes/archive/2026-05-28-agent-cli-recoverable-failure-retry/`
  - archived proposal/design/tasks/delta specs 内容均围绕 recoverable retry。
  - 这是 OpenSpec archive 的证据目录,应随实现一起提交。
- `openspec/specs/supervisor-human-chat-gate/spec.md`
  - diff 只新增 explicit recoverable continue control 相关 requirement。
  - 可作为 recoverable retry spec impact 一起 stage。

### 可以 stage,但需人工确认是否接受同文件内邻近结构变更
- `crates/ralph-core/src/config.rs`
  - recoverable retry 配置、验证和 ledger path resolver 属于本线。
  - 但 diff 里还出现 `capability.request` / `topology.spawn_group` 相关测试数据和 `allow_parent_runtime_capabilities` 字段。
  - 这说明该文件含 runtime capability / topology 支线改动,不能未经 patch review 整文件 stage。
- `crates/ralph-core/src/lib.rs`
  - recoverable exports 属于本线。
  - 同时包含 `mod topology_spawn` / `pub use topology_spawn` 等 topology 支线 export。
  - 必须 patch-stage recoverable 部分,或等 topology 线一起提交。
- `crates/ralph-core/src/agents_snapshot.rs`
  - `AgentRecoverableFailureSummary` 属于本线。
  - `completed_dynamic_instances` / role contract summary / capability result summary 属于 topology/dynamic role evidence 支线。
  - 必须 patch-stage recoverable 字段与 summary struct,不能整文件 stage。
- `crates/ralph-core/src/parallel/supervisor.rs`
  - recoverable failure transition map、completion gating、manual continue、agents summary 属于本线。
  - 同文件中还混有 topology runtime / capability invoker / dynamic spawn 相关改动。
  - 只能 patch-stage recoverable hunks;整文件 stage 会混入其它支线。
- `crates/ralph-core/src/parallel/supervisor/routing_tests.rs`
  - recoverable tests 属于本线。
  - 同文件大量 topology/capability/dynamic spawn tests 变更,不能整文件 stage。
- `crates/ralph-cli/src/record_session.rs`
  - Recoverable Failures evidence inspect 渲染属于本线。
  - 但同文件同时含 topology.spawn_group / capability.result / completed_dynamic_instances evidence inspect 大块新增。
  - 必须 patch-stage recoverable 渲染和 fixture 部分,不能整文件 stage。
- `crates/ralph-cli/tests/integration_agents.rs`
  - recoverable summary test 属于本线。
  - role_contract summary 和 completed dynamic instances tests 属于 dynamic role/topology 支线。
  - 必须 patch-stage recoverable test 与 fixture field 补齐。
- `crates/ralph-cli/tests/integration_record_session.rs`
  - recoverable field 补齐可能属于本线。
  - 当前新增主测试 `record_summary_agents_file_shows_current_and_completed_dynamic_evidence` 属于 completed dynamic evidence 支线。
  - 不应整文件 stage。
- `openspec/specs/parallel-hat-instances/spec.md`
  - 前半段 recoverable CLI failure requirement 属于本线。
  - 后半段 parent-visible dynamic spawn / partial topology outcomes / dogfood guardrail 属于 topology 支线。
  - 必须 patch-stage recoverable requirement,不能整文件 stage。

### 可以整文件 stage 的高置信实现文件
- `crates/ralph-core/src/parallel/instance.rs`
  - diff 主要集中在 HatInstanceCommand / Event / RecoverableRetryRuntime / scheduler / manual continue / tests。
  - 静态扫描只发现 recoverable retry 语义,没有明显 topology/capability 支线。
  - 可作为整文件 stage 候选。
- `crates/ralph-core/src/parallel/supervisor/routing.rs`
  - diff 主要是 `recoverable.continue` external human control action。
  - 可作为整文件 stage 候选。
- `crates/ralph-cli/src/display.rs`
  - diff 是 `ralph agents` 表格新增 Recoverable 列与 formatter。
  - 可作为整文件 stage 候选,但会与 role contract columns 同处一张表,提交前最好再看完整 diff。

### 当前不建议整文件 stage 的文件
- `crates/ralph-core/src/config.rs`
- `crates/ralph-core/src/lib.rs`
- `crates/ralph-core/src/agents_snapshot.rs`
- `crates/ralph-core/src/parallel/supervisor.rs`
- `crates/ralph-core/src/parallel/supervisor/routing_tests.rs`
- `crates/ralph-cli/src/record_session.rs`
- `crates/ralph-cli/tests/integration_agents.rs`
- `crates/ralph-cli/tests/integration_record_session.rs`
- `openspec/specs/parallel-hat-instances/spec.md`

### 推荐提交策略
1. 先提交可整文件 stage 且高置信的 recoverable-only 文件。
2. 对混线文件使用 `git add -p`,只 stage recoverable hunks。
3. stage 后必须运行:
   - `git diff --cached --check`
   - recoverable focused tests
   - `openspec validate --all --strict`
4. 如果 patch-stage 成本过高,更稳的路线是先做一个 `recoverable retry implementation + topology evidence pending split` 临时分支 review,但不要提交混线文件。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 笔记: recoverable retry 第一批 staged 文件验证

### 已 staged
- `crates/ralph-core/src/recoverable_failure.rs`
- `crates/ralph-core/src/parallel/instance.rs`
- `crates/ralph-core/src/parallel/supervisor/routing.rs`
- `openspec/specs/agent-cli-recoverable-failure-retry/spec.md`
- `openspec/specs/supervisor-human-chat-gate/spec.md`
- `openspec/changes/archive/2026-05-28-agent-cli-recoverable-failure-retry/`

### 主动降级为 patch-stage 的文件
- `crates/ralph-cli/src/display.rs`: 完整 diff 包含 role_contract / completed_dynamic_instances,不能整文件 stage。

### 发现并修复的问题
- `git diff --cached --check` 首次失败:
  - archived `design.md` 有 trailing whitespace。
  - stable spec 末尾有多余空白行。
- 已修复并重新 stage。
- 曾短暂把 `task_plan.md` stage 进 index,已用 `git restore --staged -- task_plan.md` 移出 index。

### fresh verification
- `cargo test -p ralph-core --lib recoverable --quiet`: 32 passed。
- `cargo test -p ralph-core --lib parallel::instance::tests::manual_continue_appends_transition_and_uses_scheduled_retry_path -- --exact --nocapture`: 1 passed。
- `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate --all --strict`: 28 passed,0 failed。
- `git diff --cached --check`: passed。

### 当前状态
- index 只是第一批 recoverable-only 高置信文件,不是完整 recoverable retry commit。
- 混线文件还需要 `git add -p` 精确 stage recoverable hunks。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 笔记: recoverable retry 第二批低风险 hunks staged

### 已完成
- 在第一批 staged 文件基础上,继续 staged 低风险 recoverable hunks:
  - `crates/ralph-core/src/config.rs`: recoverable retry policy / validation / ledger path resolver / config tests。
  - `crates/ralph-core/src/lib.rs`: `recoverable_failure` module 和 recoverable exports。
  - `openspec/specs/parallel-hat-instances/spec.md`: 只 stage recoverable CLI failure requirements,没有 stage 后半段 topology.spawn_group requirements。

### 错误与修正
- 首次 synthetic stage 使用过期 marker `test_agent_cli_recoverable_failures_defaults`,脚本失败。
- 失败后确认 index 仍保持第一批 staged 文件,`git diff --cached --check` 通过。
- 改用当前真实函数名和函数边界提取后,第二批 staged 成功。

### staged scan
- `config.rs` cached diff 只显示 `agent_cli_recoverable_failures` / ledger / retry policy 相关内容。
- `lib.rs` cached diff 只显示 `recoverable_failure` module/export 与 `AgentRecoverableFailureSummary` export。
- `parallel-hat-instances` cached diff 只显示 recoverable CLI failure requirements。

### fresh verification
- `cargo test -p ralph-core --lib recoverable --quiet`: 32 passed。
- `cargo test -p ralph-core --lib parallel::instance::tests::manual_continue_appends_transition_and_uses_scheduled_retry_path -- --exact --nocapture`: 1 passed。
- `cargo test -p ralph-core --lib config::tests::test_parse_agent_cli_recoverable_failures_policy_override -- --exact --nocapture`: 1 passed。
- `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate --all --strict`: 28 passed,0 failed。
- `git diff --cached --check`: passed。

### 当前剩余
- 仍需 patch-stage:
  - `crates/ralph-core/src/agents_snapshot.rs`
  - `crates/ralph-core/src/parallel/supervisor.rs`
  - `crates/ralph-core/src/parallel/supervisor/routing_tests.rs`
  - `crates/ralph-cli/src/display.rs`
  - `crates/ralph-cli/src/record_session.rs`
  - `crates/ralph-cli/tests/integration_agents.rs`
  - `crates/ralph-cli/tests/integration_record_session.rs`

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 笔记: 第三批 staged-only agents snapshot / supervisor recoverable 验证

### 本轮目标
- 继续 patch-stage recoverable retry commit 范围。
- 处理 `agents_snapshot.rs` 和 `supervisor.rs` recoverable 必要 hunks。
- 关键要求: 证明 staged index 本身可编译,不能依赖工作区未 staged 改动。

### 已 staged
- `crates/ralph-core/src/agents_snapshot.rs`:
  - 只 stage `recoverable_failures` 字段和 `AgentRecoverableFailureSummary` struct。
  - 未 stage `completed_dynamic_instances` / `child_runs` / role contract 字段。
- `crates/ralph-core/src/parallel/supervisor.rs`:
  - recoverable ledger/map。
  - `RecoverableFailureTransition` event handling。
  - completion promise pending recoverable gate。
  - recoverable continue resolution/dispatch。
  - agents snapshot recoverable summary。
- `crates/ralph-core/src/parallel/supervisor/routing.rs`:
  - 重新从整文件 stage 污染中收窄为 recoverable-only。
  - 只保留 `recoverable.continue` branch、dynamic spawn 传 recoverable policy/ledger、`escalate_delivery_failure pub(super)`。

### 被 staged-only 验证推翻的旧假设
- 旧假设: 第一批/第二批 staged index 已接近可编译。
- 反证: 临时 worktree 验证失败,暴露 `routing.rs` 混入 topology/role_contract 支线,以及 config resolver / supervisor pattern 缺口。
- 修正: 将 `routing.rs` 降级为 recoverable-only patch,补 config resolver,补 supervisor match/pattern。

### 修复过的 staged-only 问题
1. `routing.rs` 整文件 stage 污染:
   - 移除 topology / role_contract / runtime topic classifier 相关 staged hunks。
2. `config.rs` resolver 未进入 index:
   - 补 `CoreConfig::resolve_recoverable_failures_ledger_path()`。
3. `supervisor.rs` pattern 问题:
   - `JobCompleted` 改为 `job_id: _`。
   - drain match 补 `RecoverableFailureTransition { .. }`。
4. `routing.rs` 必要可见性与 spawn args:
   - `escalate_delivery_failure` 改为 `pub(super)`。
   - dynamic spawn path 给 `HatInstanceHandle::spawn` 补 recoverable policy / ledger。
5. `config.rs` 新增 tests 缺 `#[test]`:
   - 补回 attributes。
   - 重新验证 exact tests 不再是 0 tests。

### staged-only fresh verification
- 使用临时 worktree 从 HEAD 应用 `git diff --cached --binary`。
- 在临时 worktree 运行:
  - `cargo test -p ralph-core --lib recoverable --quiet`: 27 passed。
  - `cargo test -p ralph-core --lib parallel::instance::tests::manual_continue_appends_transition_and_uses_scheduled_retry_path -- --exact --nocapture`: 1 passed。
  - `cargo test -p ralph-core --lib config::tests::test_parse_agent_cli_recoverable_failures_policy_override -- --exact --nocapture`: 1 passed。
  - `cargo test -p ralph-core --lib config::tests::test_validate_recoverable_failures_policy_rejects_zero_attempts -- --exact --nocapture`: 1 passed。
  - `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate --all --strict`: 27 passed,0 failed。
  - `git diff --cached --check`: passed。

### 当前剩余
- CLI 观察面和 integration tests 仍未 stage:
  - `crates/ralph-cli/src/display.rs`
  - `crates/ralph-cli/src/record_session.rs`
  - `crates/ralph-cli/tests/integration_agents.rs`
  - `crates/ralph-cli/tests/integration_record_session.rs`
- core routing tests 仍未 stage:
  - `crates/ralph-core/src/parallel/supervisor/routing_tests.rs`

## [2026-05-29 17:06:44] [Session ID: native-codex-20260529] 笔记: recoverable retry scoped staged 验证完成

### staged 范围
- 已将 recoverable retry 主线补齐到 index:
  - core lifecycle / ledger / retry policy / supervisor gate / manual continue。
  - agents snapshot recoverable summary。
  - `ralph agents` Recoverable 列。
  - `ralph record summary --agents-file` recoverable Evidence Inspect。
  - CLI integration tests 和 routing tests。
- 明确没有 stage 上下文文件: `task_plan.md` / `notes.md` / `WORKLOG.md` / `ERRORFIX.md` / `LATER_PLANS.md` / `EPIPHANY_LOG.md`。

### staged-only verification
- 临时 worktree: `/tmp/ralph-staged-verify.rqLxTa/wt`。
- `git diff --cached --check`: passed。
- `cargo fmt --check`: passed。
- `cargo test -p ralph-core --lib recoverable --quiet`: 32 passed。
- `cargo test -p ralph-core --lib parallel::instance::tests::manual_continue_appends_transition_and_uses_scheduled_retry_path -- --exact --nocapture`: 1 passed。
- routing recoverable tests passed:
  - `pending_recoverable_failures_block_completion_gate`
  - `explicit_recoverable_continue_accepts_only_waiting_failures`
  - `bare_recoverable_continue_uses_selected_instance_to_disambiguate`
  - `bare_recoverable_continue_falls_back_to_global_unique_when_selected_has_no_failure`
  - `agents_snapshot_includes_recoverable_failure_summaries`
- `cargo test -p ralph-cli --test integration_agents --quiet`: 5 passed。
- `cargo test -p ralph-cli --test integration_record_session --quiet`: 6 passed。
- `cargo test -p ralph-cli --bin ralph record_session::tests --quiet`: 3 passed。
- `cargo test -p ralph-core smoke_runner --quiet`: 12 passed。
- `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate --all --strict`: 27 passed,0 failed。

### full cargo test 情况
- 纯 staged-only worktree 中直接运行 `cargo test --quiet` 失败在 `integration_examples`。
- 原因不是 recoverable patch,而是当前主工作区存在 24 个未被 Git 跟踪的 `examples/parallel-*/PROMPT.md`。
- 临时 worktree 从 HEAD 检出时缺这些未跟踪 fixtures。
- 将主工作区现有 prompt fixtures overlay 到临时 worktree 后,`cargo test --quiet` 通过。

### 额外修复
- 修复了 `integration_record_session` 中既有 watch 测试固定 sleep 200ms 的时序问题。
- 新写法等待 stdout 文件出现 `_meta.session_start` 或超时,解除 staged-only 环境下稳定失败。

## [2026-05-29 17:10:44] [Session ID: native-codex-20260529] 笔记: hook 触发 fresh staged-only 验证

### 来源
- OMX hook: `ultrawork is still active (phase: planning); continue the task and gather fresh verification evidence before stopping`。

### 边界检查
- `git diff --cached --check`: passed。
- staged index 未包含六文件上下文。
- 当前仍有 24 个 `examples/parallel-*/PROMPT.md` 存在于工作区但未被 Git 跟踪,该问题仍作为独立 LATER_PLANS 项保留。

### fresh staged-only verification
- 新临时 worktree: `/tmp/ralph-staged-fresh.L8b8iw/wt`。
- patch: `/tmp/ralph-staged-fresh-patch.htycfW`。
- `git diff --cached --check`: passed。
- `cargo fmt --check`: passed。
- `cargo test -p ralph-core --lib recoverable --quiet`: 32 passed。
- `cargo test -p ralph-core --lib parallel::instance::tests::manual_continue_appends_transition_and_uses_scheduled_retry_path -- --exact --nocapture`: 1 passed。
- `cargo test -p ralph-core --lib parallel::supervisor::routing_tests::agents_snapshot_includes_recoverable_failure_summaries -- --exact --nocapture`: 1 passed。
- `cargo test -p ralph-cli --test integration_agents test_agents_command_prints_recoverable_summary -- --exact --nocapture`: 1 passed。
- `cargo test -p ralph-cli --test integration_record_session record_summary_agents_file_shows_recoverable_failure_evidence -- --exact --nocapture`: 1 passed。
- `cargo test -p ralph-cli --bin ralph record_session::tests::evidence_inspect_renders_recoverable_failures_from_agents_snapshot -- --exact --nocapture`: 1 passed。
- `cargo test -p ralph-core smoke_runner --quiet`: 12 passed。
- `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate --all --strict`: 27 passed,0 failed。

### 当前结论
- recoverable retry scoped staged patch 仍然可独立验证。
- 本轮 hook 没有产生新的代码改动需求。
- 下一步若人类确认,可以做 scoped commit;否则保持 staged index 等待指令。

## [2026-05-29 17:16:28] [Session ID: native-codex-20260529] 笔记: 第二次 hook fresh verification

### 来源
- OMX hook 再次提示 ultrawork 仍 active,要求继续并收集 fresh verification evidence。

### 新验证环境
- staged-only worktree: `/tmp/ralph-hook-fresh.Pa6ZTr/wt`。
- patch: `/tmp/ralph-hook-fresh-patch.ByKDN1`。

### 验证命令与结果
- `git diff --cached --check`: passed。
- `cargo fmt --check`: passed。
- `cargo test -p ralph-core --lib recoverable --quiet`: 32 passed。
- `cargo test -p ralph-core --lib parallel::instance::tests::manual_continue_appends_transition_and_uses_scheduled_retry_path -- --exact --nocapture`: 1 passed。
- `cargo test -p ralph-cli --test integration_agents test_agents_command_prints_recoverable_summary -- --exact --nocapture`: 1 passed。
- `cargo test -p ralph-cli --test integration_record_session record_summary_agents_file_shows_recoverable_failure_evidence -- --exact --nocapture`: 1 passed。
- `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate --all --strict`: 27 passed,0 failed。

### 结论
- staged recoverable retry patch 仍可从 HEAD + cached patch 独立验证。
- 本轮没有发现新的 index 污染或实现缺口。

## [2026-05-29 17:21:03] [Session ID: native-codex-20260529] 笔记: 第三次 hook fresh staged-only verification

### 来源
- OMX hook 第三次提示 ultrawork still active,要求继续并收集 fresh verification evidence。

### 新验证环境
- staged-only worktree: `/tmp/ralph-hook3-fresh.PeT4ld/wt`。
- patch: `/tmp/ralph-hook3-fresh-patch.KlEdF5`。

### 验证命令与结果
- `git diff --cached --check`: passed。
- `cargo fmt --check`: passed。
- `cargo test -p ralph-core --lib recoverable --quiet`: 32 passed。
- `cargo test -p ralph-core --lib parallel::instance::tests::manual_continue_appends_transition_and_uses_scheduled_retry_path -- --exact --nocapture`: 1 passed。
- `cargo test -p ralph-cli --test integration_record_session record_summary_agents_file_shows_recoverable_failure_evidence -- --exact --nocapture`: 1 passed。
- `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate --all --strict`: 27 passed,0 failed。

### 当前结论
- cached patch 仍可从 HEAD 独立验证。
- 本轮没有新增实现需求。
- 继续等待人类明确是否 commit。

## [2026-05-29 17:24:54] [Session ID: native-codex-20260529] 笔记: 第四次 hook fresh staged-only verification

### 来源
- OMX hook 第四次提示 ultrawork still active,要求继续并收集 fresh verification evidence。

### 新验证环境
- staged-only worktree: `/tmp/ralph-hook4-fresh.6mas2Y/wt`。
- patch: `/tmp/ralph-hook4-fresh-patch.bwcn5C`。

### 验证命令与结果
- `git diff --cached --check`: passed。
- `cargo fmt --check`: passed。
- `cargo test -p ralph-core --lib recoverable --quiet`: 32 passed。
- `cargo test -p ralph-cli --test integration_agents test_agents_command_prints_recoverable_summary -- --exact --nocapture`: 1 passed。
- `cargo test -p ralph-cli --test integration_record_session record_summary_agents_file_shows_recoverable_failure_evidence -- --exact --nocapture`: 1 passed。
- `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate --all --strict`: 27 passed,0 failed。

### 当前结论
- cached patch 仍能从 HEAD 独立验证。
- 没有发现 staged index 污染。
- 当前仍等待明确 commit 指令。

## [2026-05-29 17:28:50] [Session ID: native-codex-20260529] 笔记: 第五次 hook fresh staged-only verification

### 来源
- OMX hook 第五次提示 ultrawork still active,要求继续并收集 fresh verification evidence。

### 新验证环境
- staged-only worktree: `/tmp/ralph-hook5-fresh.aNGSVL/wt`。
- patch: `/tmp/ralph-hook5-fresh-patch.ERJQ0b`。

### 验证命令与结果
- `git diff --cached --check`: passed。
- `cargo fmt --check`: passed。
- `cargo test -p ralph-core --lib recoverable --quiet`: 32 passed。
- `cargo test -p ralph-core --lib parallel::supervisor::routing_tests::pending_recoverable_failures_block_completion_gate -- --exact --nocapture`: 1 passed。
- `cargo test -p ralph-cli --bin ralph record_session::tests::evidence_inspect_renders_recoverable_failures_from_agents_snapshot -- --exact --nocapture`: 1 passed。
- `cargo test -p ralph-core smoke_runner --quiet`: 12 passed。
- `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate --all --strict`: 27 passed,0 failed。

### 当前结论
- cached patch 仍能从 HEAD 独立验证。
- 本轮没有发现 staged index 污染或新增实现缺口。
- 当前仍等待明确 commit 指令。

## [2026-05-29 17:32:21] [Session ID: native-codex-20260529] 笔记: 第六次 hook fresh staged-only verification

### 来源
- OMX hook 第六次提示 ultrawork still active,要求继续并收集 fresh verification evidence。

### 新验证环境
- staged-only worktree: `/tmp/ralph-hook6-fresh.pYr7tH/wt`。
- patch: `/tmp/ralph-hook6-fresh-patch.rWDlsv`。

### 验证命令与结果
- `git diff --cached --check`: passed。
- `cargo fmt --check`: passed。
- `cargo test -p ralph-core --lib parallel::supervisor::routing_tests::explicit_recoverable_continue_accepts_only_waiting_failures -- --exact --nocapture`: 1 passed。
- `cargo test -p ralph-core --lib parallel::supervisor::routing_tests::bare_recoverable_continue_uses_selected_instance_to_disambiguate -- --exact --nocapture`: 1 passed。
- `cargo test -p ralph-cli --test integration_record_session --quiet`: 6 passed。
- `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate --all --strict`: 27 passed,0 failed。

### 当前结论
- cached patch 仍能从 HEAD 独立验证。
- 本轮覆盖了 recoverable continue 的 explicit/bare disambiguation 和 record-session suite。
- 没有发现 staged index 污染或新增实现缺口。

## [2026-05-29 17:36:33] [Session ID: native-codex-20260529] 笔记: 第七次 hook fresh verification 与 ultrawork inactive

### 来源
- OMX hook 再次提示 ultrawork still active,要求继续并收集 fresh verification evidence。

### 新验证环境
- staged-only worktree: `/tmp/ralph-hook7-fresh.MS622a/wt`。
- patch: `/tmp/ralph-hook7-fresh-patch.Xnws28`。

### 验证命令与结果
- `git diff --cached --check`: passed。
- `cargo fmt --check`: passed。
- `cargo test -p ralph-core --lib parallel::instance::tests::recoverable_failure_exhaustion_becomes_terminal_with_ledger_pointer -- --exact --nocapture`: 1 passed。
- `cargo test -p ralph-core --lib parallel::instance::tests::recoverable_failure_schedules_retry_and_preserves_stdout_only_parsing -- --exact --nocapture`: 1 passed。
- `cargo test -p ralph-cli --test integration_agents --quiet`: 5 passed。
- `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate --all --strict`: 27 passed,0 failed。

### OMX 状态
- 已执行 `omx state write --input '{"mode":"ultrawork","active":false}' --json`。
- 输出: `success=true`,状态路径为 `.omx/state/sessions/019e3aba-b774-7381-bb13-dfb86660fc61/ultrawork-state.json`。

### 当前结论
- cached patch 仍能从 HEAD 独立验证。
- ultrawork 已按 skill lifecycle 标记 inactive,避免 hook 继续把当前已收敛任务视为 planning active。


## [2026-05-29 18:02:02] [Session ID: omx-1779004640353-blcixq] 笔记: continuous-learning 六文件摘要

## 六文件摘要（用于决定如何沉淀知识）
- 涉及的上下文集: 默认组, 支线后缀 `evolution_analysis`。
- 默认组任务目标: 收束 `agent-cli-recoverable-failure-retry`,完成 recoverable failure ledger、retry lifecycle、manual continue、agents snapshot / record summary 可观察性,并执行 scoped commit。
- 默认组关键决定:
  - recoverable lifecycle 以 append-only ledger 为真相源。
  - pending recoverable transition 不能被 completion promise freeze 覆盖。
  - mixed worktree 中 scoped commit 只信 staged index,不整仓 stage。
  - `task_plan.md` 超过 1000 行后,在 scoped commit 完成后的安全切点单独续档。
- 默认组关键发现:
  - staged-only 全量测试暴露 examples 下 24 个 `PROMPT.md` 未被 Git 跟踪,这应作为单独 fixture/governance 后续项。
  - repeated OMX hook 的直接运行态原因是 ultrawork state active,已用 `omx state write` 标记 inactive。
  - `integration_record_session` 里固定 200ms sleep 对 staged-only 临时 worktree 不稳定,已改为等待目标输出。
- 默认组实际变更:
  - 本轮刚完成 local commit `8bf37643 feat: add recoverable agent cli retry lifecycle`。
  - 该 commit 包含 23 个 staged 文件,提交后 index 为空,未提交上下文文件或 `.omx/state`。
- 支线组摘要 `evolution_analysis`:
  - 这是 2026-05-28 的只读项目演进分析支线。
  - 它把优先级排序为: recoverable retry 收口,大文件拆分,TUI mdfried spec-code reconciliation,旧 docs tree 搜索污染治理,release-fast gate 固化。
  - 支线内发现 `tui-mdfried-viewer` OpenSpec tasks 与当前 `ralph-tui` 实现可能漂移,后续不能直接按 tasks 已完成状态继续实现。
- 支线组活跃度判定:
  - 默认组: 当前活跃。
  - `evolution_analysis`: 最新记录为 2026-05-28,今天没有继续推进证据,判定为未轮转旧支线,本轮总结后应归档到 `archive/branch_contexts/evolution_analysis/`。
- 暂缓事项 / 后续方向:
  - 处理 example `PROMPT.md` fixture 真相源。
  - 拆分 `record_session.rs` 以及其它 runtime/TUI 大文件。
  - 对账 `tui-mdfried-viewer` spec/tasks 与当前实现。
  - 将 runtime/evidence release-fast gate 固化。
- 错误与根因:
  - 固定 sleep 导致 record watch 测试不稳定。
  - zsh 变量名 `path` 覆盖 `PATH` 是 shell 层踩坑。
  - 未加引号 heredoc 在含反引号正文时会误触发命令替换,后续应继续用单引号 heredoc 或 Python append。
- 重大风险 / 灾难点 / 重要规律:
  - recoverable retry 必须有 `recovered` / `exhausted` 终态,否则 observability 会出现永久 pending 假象。
  - TUI OpenSpec tasks 不能直接当成当前实现事实,必须看代码和依赖证据。
- 可复用点候选:
  1. mixed worktree scoped commit 的安全流程: precheck staged index -> commit -> verify empty index。
  2. recoverable failure 的 ledger-first 排查顺序。
  3. spec-code drift 的 evidence-first 对账方式。
- 最适合写到哪里: `EXPERIENCE.md`, 默认 `LATER_PLANS.md`, archive manifest; 不需要新建重复 skill。
- 需要同步的现有 docs/specs/plan 文档:
  - `LATER_PLANS.md` 需要承接 `evolution_analysis` 中仍有效的 P1-P4。
  - `EXPERIENCE.md` 需要补 scoped commit / spec-code drift 两条项目经验。
- 是否需要新增或更新 docs/specs/plan 文档: 需要更新 `LATER_PLANS.md` 和 `EXPERIENCE.md`; 不新增 docs/specs,因为 recoverable retry 已有 OpenSpec specs,本轮没有新的 runtime contract。
- 是否提取/更新 skill: 否。已有 `.codex/skills/self-learning.ralph-agent-cli-recoverable-failure-retry/SKILL.md` 覆盖 recoverable retry,scoped commit 更适合项目经验而非新 skill。


## [2026-05-29 18:48:49] [Session ID: omx-1779004640353-blcixq] 笔记: example PROMPT.md fixture 契约分析

### 现象
- `crates/ralph-cli/tests/integration_examples.rs` 明确断言 runnable example 目录必须存在同目录 `PROMPT.md`。
- 多个 `specs/parallel-real-world-examples-batch-*.spec.md` 明确写着每个 example 至少包含 `ralph.yml`、`PROMPT.md`、`README.md`。
- README 示例也直接指导用户用 `-P examples/<name>/PROMPT.md` 运行。
- 当前 `.gitignore` 全局忽略 `PROMPT.md`,只给 `examples/parallel-experimental-dev-engine/PROMPT.md` 开了例外。

### 候选假设
- 主假设: 24 个未跟踪 `examples/parallel-*/PROMPT.md` 是应提交的 example fixtures,当前只是 `.gitignore` 例外没同步扩展。
- 备选解释: 测试应改为只扫描 tracked examples,未跟踪 prompts 是用户本地资料包。

### 验证结果
- 这些 prompt 内容是通用示例 packet,没有看到凭据或机器特定数据。
- 测试函数不是扫描目录,而是显式列出这些 example 名称,说明它们是 repo 契约的一部分。
- 因此最正确修复是 tracking 这些 prompt fixtures,而不是削弱测试。

### 实施计划
- 将 `.gitignore` 中的单个 example prompt 例外改为 `!examples/parallel-*/PROMPT.md`。
- 将 24 个未跟踪 `examples/parallel-*/PROMPT.md` 纳入 scoped patch。
- 跑 `cargo test -p ralph-cli --test integration_examples --quiet` 和 staged-only 验证。

## [2026-08-01 11:05:00] [Session ID: /root/task1] 笔记: recoverable failure 生命周期追踪(Reviewer 任务)

### 静态证据(代码路径)
- 分类器: `classify_recoverable_failure`(recoverable_failure.rs:521)窄且确定;success/timeout/canceled 直接 None;retry limit 必须伴随 429;另有 4 条瞬时网络白名单。
- Ledger: `RecoverableFailureLedger`(recoverable_failure.rs:296)append-only;写前收紧 stderr excerpt;读时 malformed line 带行号报错;replay 按 failure_id 派生 snapshot。
- Instance 生命周期(parallel/instance.rs):
  - 失败 → `try_schedule_or_exhaust_recoverable_failure`(1952): 有结构化事件→terminal;policy 关闭或 attempt>=max→写 `Exhausted` + ledger pointer;否则写 `RetryScheduled` + due,state→Idle。
  - 到期 → `maybe_start_scheduled_retry`(1007): 全局 semaphore 限流;worktree 重新 acquire;写 `Retrying`;用 runtime-held job context 重新 spawn executor。
  - 人工 continue → `continue_recoverable_failure`(1104): failure_id 匹配校验;解除 completion freeze;写 `ContinuedByHuman`;把 due 提前为 now,真正执行仍复用 scheduled retry path。
  - 成功 → `finish_recoverable_success_if_needed`(2077): 写 `Recovered`,清空 runtime。
- stderr 边界: `HatJobResult.output_for_parsing` 是 stdout-only,`observed_stderr` 只可观测不参与 EventParser。
- Supervisor: 内存 snapshot map;`recoverable.continue` 事件支持显式 failure_id / bare / instance 限定;pending recoverable 会 block completion gate。
- 用户可见面: TUI `!continue [failure_id]` → recoverable.continue;`ralph agents` 读 `.ralph/agents.json`(parallel_runner.rs:249 写出);`record summary` 探测 agents.json(record_cli.rs:249-284)渲染 Evidence Inspect 三态(scheduled/continued/exhausted)并带 ledger 指针。
- OpenSpec change `2026-05-28-agent-cli-recoverable-failure-retry` 已归档。

### 动态证据(全部通过)
- `cargo test -p ralph-core --lib recoverable --quiet`: 32 passed
- `recoverable_failure_schedules_retry_and_preserves_stdout_only_parsing`: passed
- `manual_continue_appends_transition_and_uses_scheduled_retry_path`: passed
- `recoverable_failure_exhaustion_becomes_terminal_with_ledger_pointer`: passed
- `pending_recoverable_failures_block_completion_gate`: passed
- `cargo test -p ralph-cli --test integration_agents test_agents_command_prints_recoverable_summary`: passed
- `cargo test -p ralph-cli --bin ralph record_session::tests::aggregate_collects_evidence_inspect`: passed
- `cargo test -p ralph-core --test smoke_runner`: 40 passed
- `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate --all --strict`: 28 passed

### 发现的问题(候选假设,未改代码)
- skill 中 `cargo test -p ralph-core smoke_runner --quiet` 实际匹配 0 个测试(过滤器作用于测试名,而 smoke 测试名不含 "smoke_runner");正确命令是 `--test smoke_runner`。skill 存在轻微过时,建议修正。

## [2026-08-01 10:45:00] [Session ID: omx-1785579233065-awidzo] 笔记: 架构走查证据

### 规模数据
- ralph-core: 66 文件 22628 行; config.rs 3001 / hatless_ralph.rs 2120 / state_operations 1036 / workspace 1013 / recoverable_failure 987
- ralph-adapters: 10 文件 8317 行; stream_handler.rs 3465 / pty_executor.rs 1899 / cli_backend.rs 1573 / cli_executor 496
- ralph-cli: 22 文件 46484 行; main.rs 2865 / codex_app_server_session 2466 / parallel_runner 2058 / autopilot 2042 / hats 1674 / capability 1562 / record_session 1514 / doctor 1487 / loop_runner 1247
- ralph-tui: 17 文件 15003 行; app.rs 3968 / state.rs 2919 / state/parallel.rs 2689
- ralph-e2e: 35823 行; scenarios/memory.rs 2278 / reporter 2254 / scenarios/hats.rs 1745
- ralph-proto: 1926 行; event_bus 387 / hat 336 / routing 325 / event 315 / ux_event 256

### 候选1: 显示管线住在 adapters
- stream_handler.rs: MadSkin markdown 渲染、CodeBlockHighlighter(611行)、4 个 StreamHandler 实现(Pretty/Console/Quiet/Tui)、TuiStreamHandler 产出 ratatui Line
- pty_executor.rs:561 run_observe_streaming<H: StreamHandler> 依赖渲染 trait
- loop_runner.rs:646-944 handler 选择矩阵(verbose/TTY/mode)泄漏展示决策
- ralph-tui 已依赖 ralph-adapters(Cargo.toml 含 ralph-adapters)

### 候选2: CLI 是第二层应用
- codex_app_server_session 2466 行: turn/steer、turn/interrupt 运行时
- parallel_runner.rs:4 注释 "调度/路由交给 ralph-core::ParallelSupervisor",但 2058 行仍在 CLI
- autopilot.rs 2042 行: 自带 agent analysis 解析(parse_agent_analysis_output_from_stdout 1276)
- core 有 EventLoop + ParallelSupervisor(instance.rs 3704/supervisor.rs 2135/routing.rs 1791)

### 候选3: TUI 单块
- app.rs 3968 行: radar 渲染 537 行、hit_test 系列、clipboard 处理
- TuiState pub 方法 50+(600-1349 区间); Default 700+ 行(1349)
- update(700)/apply_update(770) 大方法
- state 已拆 parallel 子模块(方向对但 app.rs 仍是巨石)

### 候选4: 证据/记录管线碎片化
- JSONL 格式知识散布: core event_logger 776/event_reader 466/session_recorder 610/session_player 532/evidence_index 876/summary_writer 330; cli record_session 1514/record_cli; autopilot parse_record_session(848)+parse_agent_analysis(1276)
- find_file_in_parents(main.rs) 被 record_cli.rs:277/335 和 doctor.rs:1042 复用
- resolve_record_session_latest_pointer_in_parents(record_cli:334) 与 autopilot 重复

### 候选5: EventLoop interface 宽
- event_loop/mod.rs pub 方法 25+ (initialize/initialize_resume/next_hat/process_output/process_events_from_jsonl/check_termination/publish_terminate_event...)
- cli loop_runner run_loop_impl 1247 行直接操作这些方法;idle timeout/force-kill/guardrail 在 cli 侧

### 候选6: e2e 场景脚本化
- e2e 复用 core EventParser/SessionPlayer(好 seam 证据)
- scenarios/memory.rs 2278 行脚本式场景;reporter 2254
