# 任务计划: clean live dogfood 收尾后新入口

## [2026-05-22 12:11:00] [Session ID: omx-1779158263949-kticiv] 状态: task_plan 超限续档完成

目标:
- 保留 clean live dogfood 的最终结论入口。
- 旧计划文件已归档到 `archive/default_history/task_plan_2026-05-22_1211_prev_clean_live_dogfood.md`。
- 长期经验已沉淀到 `EXPERIENCE.md` 的 `exp-20260522-clean-live-dogfood-record-session-vs-agents-snapshot`。

当前任务结论:
- clean config 方案可用。
- record-session: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-114604.jsonl`。
- summary: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-114604.summary.txt`。
- `RUN_STATUS=0`。
- `Termination.reason=CompletionPromise`。
- `analysis.done: 3 source_instances=builder#2,builder#3,builder#4`。
- `topology.spawn_group: 1`, `topology.spawn.result: 1`, `topology.spawn.failed: 0`。

后续事项:
- 如继续实现,优先处理 agents snapshot 对已回收 dynamic instances 的历史表达。
- dogfood workspace/artifact policy 仍可单独立项。

## [2026-05-22 13:31:55] [Session ID: omx-1779158263949-kticiv] 计划: completed dynamic instances evidence 明确表达

目标:
- 让 Evidence Inspect / agents snapshot 明确表达 completed dynamic instances。
- 防止 dynamic instance 因 TTL/unregister 从 current registry 消失后,用户误判为实例没有跑。

阶段:
- [ ] 阶段1: 读取相关 spec / 数据结构 / summary 输出代码。
- [ ] 阶段2: 补充规格和测试用例。
- [ ] 阶段3: 实现 completed/tombstone dynamic instance 表达。
- [ ] 阶段4: 运行 focused tests、smoke / fmt / diff check。
- [ ] 阶段5: 更新六文件上下文并交付。

初步方案:
- 优先方案: 在 `AgentsSnapshot` 中增加 completed/retired dynamic instances tombstone,由 supervisor 在 unregister 前保留 summary,record summary 在 Evidence Inspect 中单独显示 `Completed Dynamic Instances`。
- 备选方案: 不改 agents sidecar schema,仅在 record summary 从 topology.spawn.result + Result Topics 合成历史视图。这个方案改动小,但 `ralph agents` 仍无法明确表达 completed dynamic instances。

当前状态:
- 正在阶段1,先读取现有结构和测试。

## [2026-05-22 13:44:07] [Session ID: omx-1779158263949-kticiv] 进展: completed dynamic instances focused tests 通过

已完成:
- `AgentsSnapshot` 增加 `completed_dynamic_instances` tombstone 区。
- Supervisor 在 dynamic instance unregister 前保存 summary-only tombstone。
- `ralph record summary` 的 Evidence Inspect 增加 `Completed Dynamic Instances` section。
- `ralph agents` 增加独立 Completed dynamic instances 展示,不混入 current instances。
- specs 已补充 current registry 与 completed tombstone 的语义边界。

已通过:
- `cargo test -p ralph-core parallel::supervisor::routing_tests::completed_dynamic_instance_remains_visible_as_agents_tombstone -- --exact --nocapture`
- `cargo test -p ralph-cli record_session::tests::evidence_inspect_renders_completed_dynamic_instances_from_agents_snapshot -- --exact --nocapture`
- `cargo test -p ralph-cli --test integration_agents test_agents_command_prints_completed_dynamic_instances_separately -- --exact --nocapture`

下一步:
- 跑相关测试套件、fmt、diff check、smoke_runner、全量 deny-warnings。

## [2026-05-22 13:45:32] [Session ID: omx-1779158263949-kticiv] 进展: 继续 completed dynamic instances 验证

行动:
- 先运行 `cargo fmt --all`,修正上一轮 `cargo fmt --all -- --check` 暴露的 rustfmt 差异。
- 随后重新运行 focused tests 和相关 integration tests,确认 completed dynamic instances tombstone 不只是静态可编译,而是 CLI / record summary / supervisor 三条观察面都稳定。

当前状态:
- 阶段1-3 已由前序实现完成。
- 现在进入阶段4: 格式化和验证。

## [2026-05-22 13:49:11] [Session ID: omx-1779158263949-kticiv] 进展: focused tests 格式化后通过

已通过:
- `cargo test -p ralph-core parallel::supervisor::routing_tests::completed_dynamic_instance_remains_visible_as_agents_tombstone -- --exact --nocapture`
- `cargo test -p ralph-cli record_session::tests::evidence_inspect_renders_completed_dynamic_instances_from_agents_snapshot -- --exact --nocapture`
- `cargo test -p ralph-cli --test integration_agents test_agents_command_prints_completed_dynamic_instances_separately -- --exact --nocapture`

下一步:
- 运行相关 integration / topology / record-session aggregate tests,确认邻近路径没有被 tombstone schema 破坏。

## [2026-05-22 13:49:40] [Session ID: omx-1779158263949-kticiv] 进展: 相关测试套件通过

已通过:
- `cargo test -p ralph-cli --test integration_agents -- --nocapture`: 8 passed。
- `cargo test -p ralph-cli --test integration_topology_spawn -- --nocapture`: 1 passed。
- `cargo test -p ralph-core topology_spawn_group -- --nocapture`: 15 passed。
- `cargo test -p ralph-cli record_session::tests::aggregate_collects_evidence_inspect -- --exact --nocapture`: passed。

下一步:
- 执行 final gates: `cargo fmt --all -- --check`, `git diff --check`, `cargo test -p ralph-core smoke_runner`, `RUSTFLAGS="-Dwarnings" cargo test --quiet`。

## [2026-05-22 13:51:02] [Session ID: omx-1779158263949-kticiv] 错误: 全量 deny-warnings 测试失败

现象:
- `RUSTFLAGS="-Dwarnings" cargo test --quiet` 在 `crates/ralph-cli/tests/integration_capability.rs` 失败。
- 失败测试: `tools_capability_invoke_materializes_default_parallel_workflow_config`。
- 断言期望 resolved workflow capability config 包含 `builder`,但实际 hats 是 `worker`, `confessor`, `confession_handler`。

验证计划:
- 读取失败测试和 workflow capability config 解析路径。
- 区分这是本次 completed dynamic instances 改动引发的回归,还是此前 default config 语义已改变但测试仍旧。
- 如果是测试期望过期,用 repo-grounded 证据修正测试;如果是配置解析路径错误,修实现而不是绕过。

## [2026-05-22 14:21:40] [Session ID: omx-1779158263949-kticiv] 修复: integration_capability 默认 workflow hat 期望过期

现象:
- 全量 gate 中 `tools_capability_invoke_materializes_default_parallel_workflow_config` 失败。
- resolved workflow capability config 的真实 hats 是 `worker`, `confessor`, `confession_handler`,但测试仍期望 `builder`。

已验证的静态证据:
- `resolve_workflow_capability_config()` 从 embedded startup catalog 读取 `workflow:default-parallel`。
- `workflow:default-parallel` 的内容来自 `include_str!("../../../ralph.yml")`。
- 当前仓库根 `ralph.yml` 的 canonical default workflow hats 是 `worker`, `confessor`, `confession_handler`。

修复:
- 将 integration test 的 expected hats 从 `builder` 改为 `worker`。
- 将 workflow mock backend 的实例分支从 `builder#1` 改为 `worker#1`,保持 dogfood execute test 与 canonical workflow 一致。

## [2026-05-22 14:23:40] [Session ID: omx-1779158263949-kticiv] 进展: integration_capability 修复后通过

已通过:
- `cargo test -p ralph-cli --test integration_capability -- --nocapture`: 8 passed。

下一步:
- 重新执行 final gates,确认格式、whitespace、smoke 与全量 deny-warnings 均通过。

## [2026-05-22 14:25:19] [Session ID: omx-1779158263949-kticiv] 完成: completed dynamic instances evidence 明确表达

阶段完成情况:
- [x] 阶段1: 读取相关 spec / 数据结构 / summary 输出代码。
- [x] 阶段2: 补充规格和测试用例。
- [x] 阶段3: 实现 completed/tombstone dynamic instance 表达。
- [x] 阶段4: 运行 focused tests、smoke / fmt / diff check。
- [x] 阶段5: 更新六文件上下文并交付。

最终结论:
- `AgentsSnapshot.instances` 表示 current registry。
- `AgentsSnapshot.completed_dynamic_instances` 表示已完成并 unregister 的 dynamic instance tombstone。
- Evidence Inspect 和 `ralph agents` 都会单独展示 completed dynamic instances。
- 全量 `RUSTFLAGS="-Dwarnings" cargo test --quiet` 已通过。

额外处理:
- 修复了 `integration_capability` 中 default workflow 仍期望旧 `builder` hat 的测试漂移问题。当前 canonical default workflow 使用 `worker`。

## [2026-05-22 15:21:48] [Session ID: omx-1779158263949-kticiv] 计划: 重新运行 clean 3-worker live dogfood 展示 completed dynamic instances

目标:
- 使用 clean config 重新运行真实 3-worker live dogfood。
- 生成新的 record-session / stdout / stderr / summary。
- 使用本次真实 `.ralph/agents.json` 作为 `ralph record summary --agents-file` 输入。
- 观察 Evidence Inspect / agents snapshot 是否明确展示 `completed_dynamic_instances`。

行动:
- 先检查 `/tmp/ralph-clean-task-derived-dogfood-20260522.yml` 与 prompt 是否仍存在且内容符合 clean dogfood 契约。
- 若存在,复用该 clean config;若不存在,重新生成临时 config/prompt。
- 使用 `./target/debug/ralph run -c <config> --no-tui --record-session <jsonl> -p <prompt>` 跑 live dogfood。
- 运行后执行 `./target/debug/ralph record summary <jsonl> --agents-file .ralph/agents.json`。

验证重点:
- `RUN_STATUS=0` 或明确记录失败状态。
- `Termination.reason=CompletionPromise`。
- `analysis.done` 来自 3 个 builder dynamic instances。
- `Agents Snapshot` 中 current registry 与 completed dynamic instances 的展示是否清楚。

## [2026-05-22 15:29:08] [Session ID: omx-1779158263949-kticiv] 完成: clean 3-worker live dogfood 展示验证

已完成:
- [x] 构建当前 `./target/debug/ralph`。
- [x] 运行 clean 3-worker live dogfood。
- [x] 生成 record summary with agents file。
- [x] 运行 `ralph agents` 查看 agents snapshot 展示。
- [x] 记录证据到 notes / WORKLOG。

关键结论:
-  已显示 。
- Instance        | Hat     | State    | Dynamic | Source            | Fixed Role       | Role Contract        | Last Input
---------------|---------|----------|---------|-------------------|------------------|----------------------|----------------------------------------
builder#1      | builder | idle     | no      | config-derived    | -                | -                    | -
ralph#1        | ralph   | idle     | no      | config-derived    | -                | -                    | analysis.done: {"role":"review","suggestions":["把当前演... 已显示 。
- current registry 保持 2 个静态实例,completed tombstone 保留 3 个已完成动态实例。

## [2026-05-22 15:30:56] [Session ID: omx-1779158263949-kticiv] 更正完成: clean 3-worker live dogfood 展示验证

更正说明:
- 上一条完成记录因未 quoted heredoc 被 shell 执行了反引号内容,部分正文损坏。
- 本条为更正后的最终状态。

已完成:
- [x] 构建当前 。
- [x] 运行 clean 3-worker live dogfood。
- [x] 生成 。
- [x] 运行 Instance        | Hat     | State    | Dynamic | Source            | Fixed Role       | Role Contract        | Last Input
---------------|---------|----------|---------|-------------------|------------------|----------------------|----------------------------------------
builder#1      | builder | idle     | no      | config-derived    | -                | -                    | -
ralph#1        | ralph   | idle     | no      | config-derived    | -                | -                    | analysis.done: {"role":"review","suggestions":["把当前演...

Completed dynamic instances: 3
Instance        | Hat     | Final    | Source            | Fixed Role       | Role Contract        | Last Input | Completed At
---------------|---------|----------|-------------------|------------------|----------------------|------------|--------------------------
builder#2      | builder | done     | task-derived      | -                | v1:temporary:erc-096c9f14:clean-task-derive... | build.task | 2026-05-22T07:24:04.225...
builder#3      | builder | done     | task-derived      | -                | v1:temporary:erc-dfd3922b:clean-task-derive... | build.task | 2026-05-22T07:24:17.818...
builder#4      | builder | done     | task-derived      | -                | v1:temporary:erc-6c5d8b99:clean-task-derive... | build.task | 2026-05-22T07:24:33.817... 查看真实 agents snapshot 展示。

关键结论:
-  已显示 。
- Instance        | Hat     | State    | Dynamic | Source            | Fixed Role       | Role Contract        | Last Input
---------------|---------|----------|---------|-------------------|------------------|----------------------|----------------------------------------
builder#1      | builder | idle     | no      | config-derived    | -                | -                    | -
ralph#1        | ralph   | idle     | no      | config-derived    | -                | -                    | analysis.done: {"role":"review","suggestions":["把当前演... 已显示 。
- current registry 保持 2 个静态实例,completed tombstone 保留 3 个已完成动态实例。
- 证据文件见:  与 。


## [2026-05-22 15:32:09] [Session ID: omx-1779158263949-kticiv] 最终更正完成: clean 3-worker live dogfood 展示验证

更正说明:
- 前两条完成记录因未 quoted heredoc 被 shell 执行了反引号内容,部分正文损坏。
- 本条由 Python 直接追加,是更正后的最终状态。

已完成:
- [x] 构建当前 `./target/debug/ralph`。
- [x] 运行 clean 3-worker live dogfood。
- [x] 生成 `ralph record summary --agents-file .ralph/agents.json`。
- [x] 运行 `./target/debug/ralph agents` 查看真实 agents snapshot 展示。

关键结论:
- `record summary --agents-file .ralph/agents.json` 已显示 `Completed Dynamic Instances`。
- `ralph agents` 已显示 `Completed dynamic instances: 3`。
- current registry 保持 2 个静态实例,completed tombstone 保留 3 个已完成动态实例。
- 证据文件见: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-152219.summary.txt` 与 `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-152219.agents.txt`。


## [2026-05-22 18:57:23] [Session ID: omx-1779158263949-kticiv] 计划: runtime 支持 stdout multi-line event parsing

目标:
- 让 runtime 能从 stdout 中解析完整跨行 `<event ...>...</event>`。
- 单行 event 行为保持不变。
- stderr 中的 event 仍不能被解析为真实事件。
- 不完整 event 不能 publish。
- `record summary` 的 `Result Topics` 能看到 multi-line `reply.human.message` / `analysis.done`。

阶段:
- [ ] 阶段1: 定位现有 stdout event parser / process_output / record-session 写入路径。
- [ ] 阶段2: 补 regression tests,覆盖 multi-line stdout、stderr ignore、incomplete no-publish、single-line unchanged。
- [ ] 阶段3: 实现 stdout-only multi-line buffering/parser。
- [ ] 阶段4: 运行 focused tests、record summary test、smoke、fmt、diff check、全量 deny-warnings。
- [ ] 阶段5: 更新 notes / WORKLOG / ERRORFIX / LATER_PLANS 并交付。

设计边界:
- 主修复在 runtime parser,不是只靠 prompt。
- prompt 单行约束最多作为辅助 guardrail。
- 解析只接受 stdout,避免 stderr / prompt echo 产生假事件。
- 只有完整闭合的 `<event ...>...</event>` 才能发布。


## [2026-05-22 19:06:59] [Session ID: omx-1779158263949-kticiv] 状态更新: multi-line reply 事件修复进入回归测试阶段

当前现象:
- live dogfood 中 multi-line reply.human.message 已出现在 stdout 记录里。
- record summary Result Topics 未显示 reply.human.message。

当前主假设:
- 核心 EventParser 可能已经支持跨行 event。
- parallel supervisor 在同一批输出中先识别 LOOP_COMPLETE,进入 stop_spawning,从而跳过 route_events_batch。
- 因 reply.human.message 的 observer 只在 route_event 内触发,最终没有进入 bus.publish / Result Topics。

最强备选解释:
- 后端 output_for_parsing 在进入 EventParser 前被截断或拆分,导致 parser 没拿到完整 event。
- 如果 focused supervisor test 直接给完整 output 仍复现缺失,则主假设成立;否则要回到 backend output assembly 继续查。

下一步:
- 先新增 EventParser 多行 reply 锁定测试。
- 再新增 supervisor 回归测试,证明 completion 同批 reply 必须被 observer 记录,但不能继续派生新 job。
- 测试失败后再做最小 runtime 修复。


## [2026-05-22 19:11:02] [Session ID: omx-1779158263949-kticiv] 验证结果: parser 假设被推翻,completion 同批 observer 缺失成立

已运行验证:
- cargo test -p ralph-core event_parser::tests::test_parse_reply_human_message_with_multiline_payload -- --exact --nocapture: passed。
- cargo test -p ralph-core parallel::supervisor::routing_tests::supervisor_observes_multiline_reply_human_message_in_completion_batch -- --exact --nocapture: failed as expected。

关键输出:
- failing test 的 observer 仅收到 task.start。
- 未收到 reply.human.message。

结论:
- 核心 EventParser 可以解析跨行 reply.human.message payload。
- 当前失败路径是 parallel supervisor 在同一批输出中看到 LOOP_COMPLETE 后跳过 route_events_batch,导致 reply.human.message 未触发 event_observer。

下一步:
- 在 completion stop_spawning 分支增加 observer-only drain。
- 只允许 reply.human.message 等输出型事件触发 observer,不允许 build.task/build.done 等事件派生新 job。


## [2026-05-22 19:13:05] [Session ID: omx-1779158263949-kticiv] 状态更新: observer-only drain 修复已通过 focused guardrails

已修改:
- 在 supervisor completion stop_spawning 分支增加 observer-only drain。
- observer-only drain 只放行 reply.human.message 与 hat-sourced human.message 这类不会派生新 job 的输出型事件。
- build.task / build.done 等 workflow topic 在 completion 后仍不路由。

已通过验证:
- 新增 supervisor multi-line reply + LOOP_COMPLETE 同批测试: passed。
- supervisor_does_not_route_new_events_after_completion_promise: passed。
- supervisor_freezes_prequeued_ralph_job_after_completion_promise: passed。

下一步:
- 检查 diff。
- 运行格式检查、smoke、全量 deny-warnings gate。


## [2026-05-22 19:23:13] [Session ID: omx-1779158263949-kticiv] 完成: runtime multi-line reply event observer 修复

阶段状态:
- [x] 阶段1: 定位现有 stdout event parser / process_output / record-session 写入路径。
- [x] 阶段2: 补 regression tests,覆盖 multi-line reply.human.message + LOOP_COMPLETE 同批输出。
- [x] 阶段3: 实现 completion 同批 observer-only drain。
- [x] 阶段4: 运行 focused tests、smoke、fmt、diff check、全量 deny-warnings。
- [x] 阶段5: 更新上下文并准备交付。

已验证结论:
- EventParser 已支持 multi-line event payload。
- 原失败路径是 parallel supervisor 在 completion_promise 后跳过 route_events_batch,导致 reply.human.message 没有触发 event_observer。
- 修复后,completion 同批里的 reply.human.message 会进入 observer,但 build.task/build.done 等 workflow topic 仍不会在 completion 后继续派生 job。

验证命令:
- cargo test -p ralph-core event_parser::tests::test_parse_reply_human_message_with_multiline_payload -- --exact --nocapture
- cargo test -p ralph-core parallel::supervisor::routing_tests::supervisor_observes_multiline_reply_human_message_in_completion_batch -- --exact --nocapture
- cargo test -p ralph-core parallel::supervisor::routing_tests::supervisor_does_not_route_new_events_after_completion_promise -- --exact --nocapture
- cargo test -p ralph-core parallel::supervisor::routing_tests::supervisor_freezes_prequeued_ralph_job_after_completion_promise -- --exact --nocapture
- cargo fmt --all -- --check
- git diff --check
- cargo test -p ralph-core smoke_runner
- RUSTFLAGS="-Dwarnings" cargo test --quiet

最终状态:
- 所有上述验证均 passed。


## [2026-05-22 20:16:44] [Session ID: omx-1779158263949-kticiv] 计划: 重跑 clean 3-worker live dogfood 验证 multi-line reply durable 结果

目标:
- 复用 `/tmp/ralph-clean-task-derived-dogfood-20260522.yml` 与 `/tmp/ralph-clean-task-derived-dogfood-20260522.prompt.md`。
- 构建当前 `./target/debug/ralph`。
- 重跑 clean 3-worker live dogfood。
- 生成新的 record-session / stdout / stderr / summary / agents 展示。
- 重点确认 `record summary Result Topics` 是否出现 `reply.human.message`。

验证重点:
- RUN_STATUS=0。
- Termination.reason=CompletionPromise。
- topology.spawn_group=1, topology.spawn.result=1, topology.spawn.failed=0。
- analysis.done=3, source_instances 包含 builder#2/#3/#4。
- completed_dynamic_instances=3。
- multi-line reply.human.message 进入 bus.publish / Result Topics。


## [2026-05-22 20:26:54] [Session ID: omx-1779158263949-kticiv] 完成: clean 3-worker live dogfood 重跑

已完成:
- [x] 构建当前 `./target/debug/ralph`。
- [x] 运行 clean 3-worker live dogfood。
- [x] 生成 `record summary --agents-file .ralph/agents.json`。
- [x] 运行 `ralph agents` 查看当前 agents snapshot 展示。
- [x] 直接解析 record-session 的 `bus.publish` topic counts。

证据文件:
- record-session: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-201931.jsonl`
- summary: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-201931.summary.txt`
- agents display: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-201931.agents.txt`
- stdout: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-201931.stdout.txt`
- stderr: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-201931.stderr.txt`
- status: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-201931.status.txt`

关键结论:
- RUN_STATUS=0。
- Termination.reason=CompletionPromise。
- Evidence Inspect: iterations=5, elapsed_secs=315.809。
- topology.spawn_group=1, topology.spawn.result=1, topology.spawn.failed=0。
- analysis.done=3, source_instances=builder#2,builder#3,builder#4。
- Completed Dynamic Instances=3。
- reply.human.message=1, source_instances=ralph#1。
- record-session 原始 `bus.publish` 统计也显示 `reply.human.message: 1 sources=[ralph#1]`。

结论:
- multi-line `reply.human.message` 已进入 durable bus.publish / Result Topics。
- 上一轮 runtime observer-only drain 修复通过真实 live dogfood 验证。


## [2026-05-23 14:45:53] [Session ID: omx-1779158263949-kticiv] 计划: agent CLI 可恢复失败归纳与 retry 机制

目标:
- 设计一个数据文件,记录并归纳 agent CLI 进程失败原因,例如 `429 Too Many Requests` / `exceeded retry limit`。
- 支持可恢复失败在 X 秒后自动重试。
- 支持用户输入“继续”后重试被暂停的可恢复 agent job。

当前观察:
- 并行 CLI executor 已收集 stdout/stderr/exit_code/timed_out/canceled 到 `HatJobResult`。
- `HatInstance` 会把 executor error 转成 failed `HatJobResult`。
- Supervisor 当前对 failed job 主要做状态记录,没有把 stderr 归一成 recoverable failure,也没有 retry lease / pending resume 数据文件。

候选设计方向:
- 数据文件作为单一真相源: `.ralph/recoverable-failures.jsonl` 或 `.ralph/agent-failures.jsonl`。
- 每条记录包含 instance/job/backend/error_kind/raw_excerpt/request_id/retry_after/attempt/status。
- runtime 根据分类决定 auto retry / pause until continue / terminal failure。
- TUI/CLI/record summary 都从该数据文件展示可恢复失败和下一次 retry 时间。

下一步:
- 创建 OpenSpec change `agent-cli-recoverable-failure-retry`。
- 先补 proposal/design/spec/test-plan,经确认后再实现。

## [2026-05-23 14:51:22] [Session ID: omx-1779158263949-kticiv] 计划: agent CLI 可恢复失败归纳与 retry/继续重试

目标:
- 为 agent CLI 停止原因建立一个数据文件,记录并归纳类似  /  的可恢复失败。
- 为这类失败提供两条恢复路径: x 秒后自动重试,或用户输入“继续”后手动重试。
- 先完成 repo-grounded spec 继续推进,不直接实现代码。

阶段:
- [ ] 阶段1: 读取现有变更状态、相关代码路径和已有 spec。
- [ ] 阶段2: 补齐 proposal / design / specs / tasks,钉死数据文件与 retry 语义。
- [ ] 阶段3: 如有必要,补充验证计划与回归测试边界。
- [ ] 阶段4: 交付当前规格结果,并更新六文件上下文。

当前状态:
- 现在先确认 OpenSpec change 的真实进度,以及 proposal 是否已经落盘。

## [2026-05-23 15:02:11] [Session ID: omx-1779158263949-kticiv] 进展: agent CLI recoverable failure proposal 已落盘

阶段更新:
- [x] 阶段1: 读取现有变更状态、相关代码路径和已有 spec。
- [x] 阶段2: 补齐 proposal / design / specs / tasks,钉死数据文件与 retry 语义中的 proposal。
- [ ] 阶段3: 如有必要,补充验证计划与回归测试边界。
- [ ] 阶段4: 交付当前规格结果,并更新六文件上下文。

已完成:
- 已创建 `openspec/changes/agent-cli-recoverable-failure-retry/proposal.md`。
- OpenSpec 状态已进入 proposal done,design/specs ready。

补充记录:
- 前一次追加计划时,因为 heredoc 没有做安全引用,示例里的反引号被 shell 当成命令替换,导致计划正文出现空白占位。
- 这次的 proposal 版本已经按安全方式落盘,以后继续推进时以 proposal 文件内容为准。

## [2026-05-23 15:12:00] [Session ID: omx-1779158263949-kticiv] 计划: 使用用户新 prompt 做 dynamic hats dogfood

目标:
- 使用用户给定 prompt: "我 需 要 你 从 多 个 方 面 角 度 去 分 析 项 目 下 一 步 演 进 的 话 ， 有 哪 些 值 得 做 的 方 向 ，  你 来 看 需 要 几 个 角 度 方 面 ， 就 创 建 几 个 hats 去 并 行 做 分 析"。
- 运行 live dogfood,验证 Ralph 是否能由 coordinator 自主决定角度数量并创建动态 hats/instances 并行分析。
- 收集 record-session、summary、agents snapshot、stdout/stderr/status 作为证据。

阶段:
- [ ] 阶段1: 检查并复用 clean dogfood config,确认 prompt/配置契约仍适用。
- [ ] 阶段2: 构建当前 debug ralph binary。
- [ ] 阶段3: 运行 live dogfood 并收集证据。
- [ ] 阶段4: 解析 record-session 与 agents snapshot,判断是否产生动态实例和可用分析结果。
- [ ] 阶段5: 写入 WORKLOG 并交付结论。

当前状态:
- 正在阶段1,先读取 `/tmp/ralph-clean-task-derived-dogfood-20260522.yml` 与旧 prompt,确认是否适合复用。

## [2026-05-23 15:16:12] [Session ID: omx-1779158263949-kticiv] 进展: dynamic hats dogfood 编译通过并开始 live run

阶段更新:
- [x] 阶段1: 检查并复用 clean dogfood config,确认 prompt/配置契约仍适用。
- [x] 阶段2: 构建当前 debug ralph binary。
- [ ] 阶段3: 运行 live dogfood 并收集证据。
- [ ] 阶段4: 解析 record-session 与 agents snapshot,判断是否产生动态实例和可用分析结果。
- [ ] 阶段5: 写入 WORKLOG 并交付结论。

已验证:
- `cargo build -p ralph-cli --bin ralph --quiet` 通过。
- `ralph run -p` 可覆盖 config 中的 prompt_file,因此本轮不需要修改 `/tmp` 旧 prompt 文件。

当前行动:
- 使用 `/tmp/ralph-clean-task-derived-dogfood-20260522.yml`。
- 使用用户新 prompt 直接作为 inline prompt。
- 输出证据到 `/tmp/ralph-dynamic-evolution-angle-dogfood-20260523-*.{jsonl,stdout,stderr,status}`。

## [2026-05-23 16:45:00] [Session ID: omx-1779158263949-kticiv] 完成: 用户自然语言 prompt dynamic hats dogfood

阶段完成情况:
- [x] 阶段1: 检查并复用 clean dogfood config,确认 prompt/配置契约仍适用。
- [x] 阶段2: 构建当前 debug ralph binary。
- [x] 阶段3: 运行 live dogfood 并收集证据。
- [x] 阶段4: 解析 record-session 与 agents snapshot,判断是否产生动态实例和可用分析结果。
- [x] 阶段5: 写入 WORKLOG 并交付结论。

验证命令与证据:
- build: `cargo build -p ralph-cli --bin ralph --quiet` passed。
- record-session: `/tmp/ralph-dynamic-evolution-angle-dogfood-20260523-151612.jsonl`。
- summary: `/tmp/ralph-dynamic-evolution-angle-dogfood-20260523-151612.summary.txt`。
- agents: `/tmp/ralph-dynamic-evolution-angle-dogfood-20260523-151612.agents.txt`。
- extracted result: `/tmp/ralph-dynamic-evolution-angle-dogfood-20260523-151612.results.md`。

结论:
- record-session 可解析,parse_errors=0。
- `_meta.termination.reason=CompletionPromise`,iterations=8,elapsed_secs≈539.8。
- `topology.spawn_group=1`,由 `ralph#1` 发出。
- `topology.spawn.result=1`,spawned builder#2..builder#6。
- `analysis.done=6`,其中动态实例 builder#2..builder#6 分别完成 protocol_architect/evidence_auditor/ux_reviewer/governance_reviewer/e2e_gatekeeper。
- `reply.human.message=6`,最终 reply 汇总了下一步最值得做的主线。

注意:
- 包装脚本里使用了 zsh 只读变量 `status`,导致外层 stderr 出现 `zsh: read-only variable: status`。这属于我的 shell wrapper 错误,不是 Ralph 语义失败。本轮语义完成以 record-session `_meta.termination` 为准。

## [2026-05-23 17:05:00] [Session ID: omx-1779158263949-kticiv] 计划: 新建 OpenSpec change clean-current-runtime-evidence-and-dynamic-role-contract

目标:
- 按 dynamic hats dogfood 推荐主线,新建 `clean-current-runtime-evidence-and-dynamic-role-contract` OpenSpec change。
- 先生成规格资产,不实现代码。
- 规格范围覆盖 runtime protocol SSOT、dynamic role contract evidence、topology.spawn_group partial/tombstone 语义、record-session/evidence inspect correlation、parallel runtime release gate。

阶段:
- [ ] 阶段1: 创建 OpenSpec change scaffold 并读取 artifact 状态。
- [ ] 阶段2: 生成 proposal,确定新增/修改 capabilities。
- [ ] 阶段3: 生成 specs,写清楚 MUST/SHALL requirement。
- [ ] 阶段4: 生成 design/tasks,整理测试和 dogfood 验证计划。
- [ ] 阶段5: 跑 openspec status/validate,写回 WORKLOG 并交付。

当前状态:
- 正在阶段1,先检查是否已有同名 change,然后创建或继续。

## [2026-05-23 17:24:00] [Session ID: omx-1779158263949-kticiv] 完成: OpenSpec change clean-current-runtime-evidence-and-dynamic-role-contract

阶段完成情况:
- [x] 阶段1: 创建 OpenSpec change scaffold 并读取 artifact 状态。
- [x] 阶段2: 生成 proposal,确定新增/修改 capabilities。
- [x] 阶段3: 生成 specs,写清楚 MUST/SHALL requirement。
- [x] 阶段4: 生成 design/tasks,整理测试和 dogfood 验证计划。
- [x] 阶段5: 跑 openspec status/validate,写回 WORKLOG 并交付。

已完成文件:
- `openspec/changes/clean-current-runtime-evidence-and-dynamic-role-contract/proposal.md`
- `openspec/changes/clean-current-runtime-evidence-and-dynamic-role-contract/design.md`
- `openspec/changes/clean-current-runtime-evidence-and-dynamic-role-contract/tasks.md`
- `openspec/changes/clean-current-runtime-evidence-and-dynamic-role-contract/specs/current-runtime-evidence-contract/spec.md`
- `openspec/changes/clean-current-runtime-evidence-and-dynamic-role-contract/specs/parallel-hat-instances/spec.md`
- `openspec/changes/clean-current-runtime-evidence-and-dynamic-role-contract/specs/record-session-contract-and-watch/spec.md`
- `openspec/changes/clean-current-runtime-evidence-and-dynamic-role-contract/specs/runtime-evidence-index-kernel/spec.md`
- `openspec/changes/clean-current-runtime-evidence-and-dynamic-role-contract/specs/prompt-contract-runtime-alignment/spec.md`

验证:
- `openspec status --change clean-current-runtime-evidence-and-dynamic-role-contract`: 4/4 artifacts complete。
- `openspec validate clean-current-runtime-evidence-and-dynamic-role-contract --type change --strict`: passed。

注意:
- `openspec validate --changes --strict` 会验证所有 open changes,当前因无关的 `agent-cli-recoverable-failure-retry` 未完成而总失败。本次新 change 单项验证已通过。

## [2026-05-23 18:00:56] [Session ID: omx-1779158263949-kticiv] 核验: clean-current-runtime-evidence-and-dynamic-role-contract spec 状态

目标:
- 按用户要求核对  是否已经生成完整 OpenSpec 规格。
- 本轮不实现代码,只核验规格资产和校验结果。

阶段:
- [ ] 阶段1: 查看 OpenSpec artifact status。
- [ ] 阶段2: 严格校验该 change。
- [ ] 阶段3: 如已完整,向用户交付文件清单和验证证据。

当前状态:
- 正在阶段1,准备运行 Change: clean-current-runtime-evidence-and-dynamic-role-contract
Schema: spec-driven
Progress: 4/4 artifacts complete

[x] proposal
[x] design
[x] specs
[x] tasks

All artifacts complete!。

## [2026-05-23 18:01:34] [Session ID: omx-1779158263949-kticiv] 修正: 上一条 task_plan 追加发生 shell 反引号替换

现象:
- 上一条 `task_plan.md` 追加时,正文包含反引号但使用了未加引号 heredoc。
- shell 执行了反引号中的内容,导致计划正文里的 change 名和命令被替换为空或 `openspec status` 输出。

影响:
- 没有修改 OpenSpec 文件。
- 只污染了 `task_plan.md` 的一条核验记录正文。

修正:
- 本条开始恢复使用 `cat <<'EOF'` 语义追加 Markdown。
- 后续继续执行 `openspec status` 与 `openspec validate` 核验。

阶段:
- [ ] 阶段1: 查看 OpenSpec artifact status。
- [ ] 阶段2: 严格校验该 change。
- [ ] 阶段3: 交付文件清单和验证证据。

当前状态:
- 正在阶段1,准备运行 `openspec status --change clean-current-runtime-evidence-and-dynamic-role-contract`。

## [2026-05-23 18:02:28] [Session ID: omx-1779158263949-kticiv] 完成: 核验 clean-current-runtime-evidence-and-dynamic-role-contract spec

阶段完成情况:
- [x] 阶段1: 查看 OpenSpec artifact status。
- [x] 阶段2: 严格校验该 change。
- [x] 阶段3: 交付文件清单和验证证据。

验证结果:
- `openspec status --change clean-current-runtime-evidence-and-dynamic-role-contract`: 4/4 artifacts complete。
- `openspec validate clean-current-runtime-evidence-and-dynamic-role-contract --type change --strict`: valid。

说明:
- 该 OpenSpec change 已经存在且完整,本轮没有重复创建同名 change。
- 本轮没有实现代码,符合“先 spec,再落地”的约束。
- 本轮发现并记录了一个 task_plan 追加时的 shell heredoc 操作错误: Markdown 正文含反引号时必须使用单引号 heredoc 或其他不会触发命令替换的写入方式。

## [2026-05-24 15:57:17] [Session ID: omx-1779158263949-kticiv] 计划: 落地 clean-current-runtime-evidence-and-dynamic-role-contract 聚焦任务

目标:
- 按用户要求从 `openspec/changes/clean-current-runtime-evidence-and-dynamic-role-contract/tasks.md` 开始落地。
- 本轮优先且仅聚焦 runtime protocol SSOT 与 dynamic role contract evidence。
- 明确不扩 UI 分支,不扩 agent CLI retry 分支,避免主线发散。

阶段:
- [ ] 阶段1: 读取 OpenSpec apply 指令、proposal/design/spec/tasks。
- [ ] 阶段2: 定位 runtime protocol、prompt boundary、topology spawn、agents snapshot 相关代码路径。
- [ ] 阶段3: 实现 runtime protocol SSOT / prompt boundary 测试与 dynamic role evidence。
- [ ] 阶段4: 运行 focused tests、OpenSpec validate,必要时补 smoke/integration gate。
- [ ] 阶段5: 更新 tasks.md、WORKLOG.md,交付修改清单和验证证据。

约束:
- 不修改 UI/TUI 视觉展示。
- 不实现 retry/recoverable failure 方案。
- 不撤回或覆盖非本轮产生的既有本地改动。

当前状态:
- 正在阶段1,准备运行 `openspec status --change clean-current-runtime-evidence-and-dynamic-role-contract --json` 与 `openspec instructions apply --change clean-current-runtime-evidence-and-dynamic-role-contract --json`。

## [2026-05-24 18:45:19] [Session ID: omx-1779158263949-kticiv] 行动: 实现 runtime protocol SSOT 最小补齐

现象:
- dynamic role contract evidence 已有 `EffectiveRoleContract`、`RoleContractSummary`、`topology.spawn.result` 摘要和 agents tombstone 相关实现。
- runtime protocol 相关判断仍分散在 `config.rs`、`topology_runtime.rs`、`routing.rs` 的局部函数或硬编码列表中。

当前假设:
- 需要把 reserved/control/result/observer-only topic 分类收敛到 `event_emission_protocol.rs` 这一运行时协议文件中。
- 然后由 config validation、topology role allowlist、strict-target bypass 复用同一分类 helper。

备选解释:
- 现有局部判断已经能跑通 dogfood,只要标记任务即可。
- 但这会继续留下协议漂移风险,不满足 OpenSpec 的 SSOT 要求。

接下来实际改动:
- 添加 runtime topic classification helper。
- 更新 prompt 文案插入 runtime topic matrix。
- 更新 config/topology/routing 使用 helper。
- 补 focused tests。

## [2026-05-24 18:53:28] [Session ID: omx-1779158263949-kticiv] 进展: runtime protocol SSOT 与 dynamic role evidence 已聚焦实现

阶段完成情况:
- [x] 阶段1: 读取 OpenSpec apply 指令、proposal/design/spec/tasks。
- [x] 阶段2: 定位 runtime protocol、prompt boundary、topology spawn、agents snapshot 相关代码路径。
- [x] 阶段3: 实现 runtime protocol SSOT / prompt boundary 测试与 dynamic role evidence 勾选。
- [ ] 阶段4: 运行 focused tests、OpenSpec validate,必要时补 smoke/integration gate。
- [ ] 阶段5: 更新 WORKLOG.md,交付修改清单和验证证据。

已完成:
- 在 `event_emission_protocol.rs` 增加 runtime topic classification SSOT。
- `config.rs` 改为复用 SSOT 拒绝 ordinary hat reserved triggers。
- `topology_runtime.rs` 改为复用 SSOT 裁剪/拒绝 role contract output allowlist。
- `routing.rs` 的 gate strict-target bypass 改为复用 SSOT helper。
- 补充 event protocol、config reserved trigger、dynamic worker prompt boundary 测试。
- `tasks.md` 已勾选 1.x 和 2.x。

已通过 focused tests:
- `cargo test -p ralph-core event_emission_protocol --lib --quiet`
- `cargo test -p ralph-core config::tests::test_reserved --lib --quiet`
- `cargo test -p ralph-core --lib topology_spawn_group_rejects_control_plane_output_topic --quiet`
- `cargo test -p ralph-core --lib dynamic_worker_prompt_contains_effective_role_contract --quiet`
- `cargo test -p ralph-core --lib completed_dynamic_instance_remains_visible_as_agents_tombstone --quiet`
- `cargo test -p ralph-cli --test integration_topology_spawn --quiet`

当前状态:
- 正在阶段4,下一步运行 OpenSpec 单项校验和 touched-module 更宽 focused test。

## [2026-05-24 18:57:15] [Session ID: omx-1779158263949-kticiv] 完成: runtime protocol SSOT 与 dynamic role contract evidence 聚焦落地

阶段完成情况:
- [x] 阶段1: 读取 OpenSpec apply 指令、proposal/design/spec/tasks。
- [x] 阶段2: 定位 runtime protocol、prompt boundary、topology spawn、agents snapshot 相关代码路径。
- [x] 阶段3: 实现 runtime protocol SSOT / prompt boundary 测试与 dynamic role evidence。
- [x] 阶段4: 运行 focused tests、OpenSpec validate、smoke、全仓库 `cargo test --quiet`。
- [x] 阶段5: 更新 `tasks.md`、`WORKLOG.md`,交付修改清单和验证证据。

本轮完成范围:
- 已完成 tasks.md 的 1.1-1.4 与 2.1-2.4。
- 未做 3.x partial/tombstone 深化、4.x record summary 扩展、5.x evidence-index 扩展、6.x release-fast dogfood。它们保留为后续任务。

验证结果:
- `cargo test -p ralph-core event_emission_protocol --lib --quiet`: passed。
- `cargo test -p ralph-core config::tests::test_reserved --lib --quiet`: passed。
- `cargo test -p ralph-core --lib topology_spawn_group_rejects_control_plane_output_topic --quiet`: passed。
- `cargo test -p ralph-core --lib dynamic_worker_prompt_contains_effective_role_contract --quiet`: passed。
- `cargo test -p ralph-core --lib completed_dynamic_instance_remains_visible_as_agents_tombstone --quiet`: passed。
- `cargo test -p ralph-cli --test integration_topology_spawn --quiet`: passed。
- `cargo test -p ralph-core --lib runtime_capability_catalog_is_injected_only_into_ralph_prompt --quiet`: passed。
- `cargo test -p ralph-core --lib worker_prompt_excludes_coordinator_only_sections --quiet`: passed。
- `cargo test -p ralph-core --lib prompt_surface --quiet`: passed。
- `openspec validate clean-current-runtime-evidence-and-dynamic-role-contract --type change --strict`: passed。
- `cargo test -p ralph-core smoke_runner --quiet`: passed。
- `cargo test --quiet`: passed。
- `git diff --check` on touched paths: passed。

## [2026-05-25 10:27:50] [Session ID: omx-1779158263949-kticiv] 计划: 落地 3.x Spawn group partial and tombstone lifecycle

目标:
- 继续 `clean-current-runtime-evidence-and-dynamic-role-contract`。
- 本轮只完成 3.1-3.4: partial outcome、non-atomic spawn continuation、failed/failed-after-spawn tombstone、events-log evidence。
- 不做 UI/TUI 展示扩展,不做 agent CLI retry。

阶段:
- [ ] 阶段1: 审计现有 `TopologySpawnFailedMember`、agents tombstone、runtime lifecycle/logging 路径。
- [ ] 阶段2: 实现 partial outcome shape 和 failed-after-spawn tombstone 写入。
- [ ] 阶段3: 补 focused tests,覆盖 validation failure + delivery failure after spawn + failed tombstone evidence。
- [ ] 阶段4: 跑 focused tests、OpenSpec validate、smoke/full gates。
- [ ] 阶段5: 勾选 tasks.md 3.x,写 WORKLOG,交付证据。

当前假设:
- 现有 `TopologySpawnFailedMember` 已能表达 index/role/error,但缺 request_id、instance_id、phase、recovery_hint。
- 现有 completed tombstone 只覆盖 done-after-reaping,还需要失败/failed-after-spawn 的 tombstone 写入路径。

备选解释:
- 也许当前 `topology.spawn.result.failed[]` 足够表达 partial,但它无法解释“实例已经创建后又失败/消失”的 lifecycle,所以仍需要 tombstone/evidence 加强。

当前状态:
- 正在阶段1,准备读取相关代码和测试。

## [2026-05-25 10:30:30] [Session ID: omx-1779158263949-kticiv] 行动: 继续 3.x partial outcome 与 tombstone 落地

目标:
- 在已经同意的 3.x 方案上继续执行,优先补 `TopologySpawnFailedMember` 的 request/phase/recovery evidence。
- 让 delivery failed after spawn 进入 agents tombstone,保持 runtime evidence 单一真相源。

当前约束:
- 不扩 UI/TUI。
- 不做 retry / recoverable CLI failure。
- 不重构 unrelated 路径,不回滚既有本地改动。

即将执行:
- 使用 OpenSpec apply 指令刷新剩余任务。
- 读取 topology spawn runtime、agents snapshot、routing tests 的当前实现。
- 做 focused patch,再运行 focused tests。

## [2026-05-25 10:35:20] [Session ID: omx-1779158263949-kticiv] 进展: 3.x 静态审计结论

已观察到的现象:
- `TopologySpawnFailedMember` 当前只有 `index`、`role`、`error`,不能表达 request id、instance id、failed phase、recovery hint。
- `topology.spawn.result` 已经有 `status = partial` 的载体,但 failed member 粒度不足。
- completed dynamic tombstone 已经存在,但 `retirement_reason` 固定为 `dynamic_instance_unregistered_after_done`。
- `spawn_group_members` 的 delivery error 分支已经会记录 failed member,但没有把已创建实例标记 failed,也没有写 failed tombstone。

当前假设:
- 3.x 的正确一刀是改良现有 result/tombstone 结构,而不是新增第二套 evidence 文件。
- 用 `topology.spawn.result.failed[]` 表达 partial phase evidence,用现有 `completed_dynamic_instances` 表达 failed tombstone。

备选解释:
- 可以新增专门的 `topology.spawn.partial_failed` topic,但这会制造第二个 truth source,与当前 change 的 evidence closure 目标冲突。

下一步:
- 扩展 `TopologySpawnFailedMember`。
- 增加带 reason 的 dynamic unregister helper。
- 在 validation/spawn/delivery failure 分支填充 phase/recovery evidence。
- 补 focused tests。

## [2026-05-25 10:39:10] [Session ID: omx-1779158263949-kticiv] 错误: partial continuation 测试断言范围过宽

现象:
- `cargo test -p ralph-core --lib topology_spawn_group_partial_failure_keeps_successful_members_running --quiet` 失败。
- 输出显示 executor seen 为 `["ralph#1", "builder#2"]`,断言期望只有 `["builder#2"]`。

已验证事实:
- `builder#2` 已经启动,说明 successful member 没被 failed member 阻断。
- 多出来的 `ralph#1` 来自 `topology.spawn.result` 回投给 coordinator,属于正常 runtime 行为。

修正计划:
- 测试断言改为过滤 `builder#` 动态实例,不把 coordinator result acknowledgement 当成 member delivery。

## [2026-05-25 10:42:00] [Session ID: omx-1779158263949-kticiv] 进展: 3.x 已实现并勾选 tasks

阶段完成情况:
- [x] 阶段1: 审计现有 partial/tombstone 代码路径。
- [x] 阶段2: 实现 partial outcome shape 和 failed-after-spawn tombstone 写入。
- [x] 阶段3: 补 focused tests,覆盖 validation failure、non-atomic continuation、failed tombstone evidence。
- [ ] 阶段4: 跑 OpenSpec validate、smoke/full gates。
- [ ] 阶段5: 写 WORKLOG,交付证据。

已通过 focused tests:
- `cargo test -p ralph-core --lib topology_spawn_group_result_serializes_partial_failure --quiet`
- `cargo test -p ralph-core --lib topology_spawn_group_rejects_control_plane_output_topic --quiet`
- `cargo test -p ralph-core --lib topology_spawn_group_partial_failure_keeps_successful_members_running --quiet`
- `cargo test -p ralph-core --lib failed_dynamic_instance_remains_visible_as_agents_tombstone --quiet`
- `cargo test -p ralph-core --lib completed_dynamic_instance_remains_visible_as_agents_tombstone --quiet`
- `cargo test -p ralph-core --lib topology_spawn_group_ --quiet`

当前状态:
- `openspec/.../tasks.md` 已勾选 3.1-3.4。
- 下一步运行 OpenSpec validate、核心库测试、smoke/full gate。

## [2026-05-25 10:48:20] [Session ID: omx-1779158263949-kticiv] 完成: 3.x Spawn group partial and tombstone lifecycle

阶段完成情况:
- [x] 阶段1: 审计现有 `TopologySpawnFailedMember`、agents tombstone、runtime lifecycle/logging 路径。
- [x] 阶段2: 实现 partial outcome shape 和 failed-after-spawn tombstone 写入。
- [x] 阶段3: 补 focused tests,覆盖 validation failure + non-atomic continuation + failed tombstone evidence。
- [x] 阶段4: 跑 focused tests、OpenSpec validate、smoke/full gates。
- [x] 阶段5: 勾选 tasks.md 3.x,写 WORKLOG,交付证据。

最终状态:
- `openspec/changes/clean-current-runtime-evidence-and-dynamic-role-contract/tasks.md` 已完成 3.1-3.4。
- 当前 change 单项 strict validation 通过。
- `cargo test -p ralph-core smoke_runner --quiet` 通过。
- `cargo test --quiet` 通过。

未纳入本轮:
- 4.x record summary/evidence correlation。
- 5.x evidence-index correlation。
- 6.x release-fast dogfood evidence。
- unrelated `agent-cli-recoverable-failure-retry` 的 OpenSpec delta 修复。

## [2026-05-25 12:44:40] [Session ID: omx-1779158263949-kticiv] 计划: 落地 4.x Record summary and evidence correlation

目标:
- 继续 `clean-current-runtime-evidence-and-dynamic-role-contract`。
- 本轮聚焦 4.1-4.4: 让 `ralph record summary` / `record summary --agents-file` 能展示 dynamic spawn request、spawned instances、role contract hash、result source coverage、current registry 与 completed dynamic tombstones。

阶段:
- [ ] 阶段1: 刷新 OpenSpec apply 状态,读取 4.x spec/design/tasks。
- [ ] 阶段2: 审计 `record_cli` / `record_session` / agents-file summary 当前实现和测试入口。
- [ ] 阶段3: 实现 summary evidence 展示,区分 semantic completion、wrapper exit、stdout tail/display state。
- [ ] 阶段4: 补 focused tests,覆盖 missing termination、missing dynamic result coverage、completed dynamic instance display。
- [ ] 阶段5: 勾选 tasks.md 4.x,运行 focused gates、OpenSpec validate、smoke/full tests,写 WORKLOG。

当前约束:
- 不扩 TUI/UI。
- 不实现 retry / recoverable CLI failure。
- 不新增 record-session 的替代 truth source,只做 durable evidence 的 inspect/correlation 展示。

当前状态:
- 正在阶段1,准备刷新 OpenSpec 状态并读取 record summary 代码路径。

## [2026-05-25 12:51:30] [Session ID: omx-1779158263949-kticiv] 进展: 4.x 静态审计结论

已观察到的现象:
- `record_session.rs` 已有 `EvidenceInspectAggregate` 和 `render_evidence_inspect`,能读取 `topology.spawn_group`、`topology.spawn.result`、`topology.spawn.failed`、result topics 和 agents snapshot。
- 当前输出已经显示 spawned instances、role contract hash、source_spawn_request_id、current registry、completed dynamic tombstones。
- 仍缺显式 per-dynamic-instance result coverage,用户需要自己把 spawned instance 和 Result Topics source_instances 对起来。
- Termination 已显示 `<missing>`,但没有明确写出“semantic completion 只来自 record-session `_meta.termination`,不能从 spawn success / stdout / display 推断”。
- partial failed member 目前只显示 failed count,没有展开 request_id / instance_id / phase / recovery_hint。

当前假设:
- 4.x 的正确改良点是增强现有 `Evidence Inspect` 文本,不新增新的 JSON truth source。
- 通过现有 `topology.spawn.result` 与 `result_topics` 聚合,可计算 dynamic result coverage 和 missing markers。

备选解释:
- 也可以把 coverage 写入 evidence-index,但那属于 5.x,本轮不提前扩展。

下一步:
- 在 `render_evidence_inspect` 中补 semantic completion source 文案、failed member detail、Dynamic Result Coverage 小节。
- 补 focused unit/integration tests。

## [2026-05-25 12:58:30] [Session ID: omx-1779158263949-kticiv] 进展: 4.x record summary evidence correlation 已实现

阶段完成情况:
- [x] 阶段1: 刷新 OpenSpec apply 状态,读取 4.x spec/design/tasks。
- [x] 阶段2: 审计 `record_cli` / `record_session` / agents-file summary 当前实现和测试入口。
- [x] 阶段3: 实现 summary evidence 展示,区分 semantic completion、wrapper exit、stdout tail/display state。
- [x] 阶段4: 补 focused tests,覆盖 missing termination、missing dynamic result coverage、completed dynamic instance display。
- [ ] 阶段5: 运行 focused gates、OpenSpec validate、smoke/full tests,写 WORKLOG。

已实现:
- Evidence Inspect 的 Termination 增加 `semantic_source: record-session _meta.termination`。
- 缺失 termination 时明确说明不能从 topology spawn success、stdout tail、wrapper exit status 或 display state 推断 completion。
- Topology spawn result 下增加 partial failed member 明细,展示 request_id、instance_id、phase、recovery_hint。
- 增加 Dynamic Result Coverage,逐个 dynamic spawned instance 展示 expected / covered / missing result topics。
- `record summary --agents-file` integration test 覆盖 current registry 与 completed dynamic tombstones 分区展示。

已通过 focused tests:
- `cargo test -p ralph-cli evidence_inspect --quiet`
- `cargo test -p ralph-cli --test integration_record_session record_summary_agents_file_shows_current_and_completed_dynamic_evidence --quiet`
- `cargo test -p ralph-cli --test integration_record_session --quiet`

当前状态:
- `openspec/.../tasks.md` 已勾选 4.1-4.4。
- 下一步运行 OpenSpec validate、ralph-cli tests、smoke/full gates。

## [2026-05-25 13:05:20] [Session ID: omx-1779158263949-kticiv] 完成: 4.x Record summary and evidence correlation

阶段完成情况:
- [x] 阶段1: 刷新 OpenSpec apply 状态,读取 4.x spec/design/tasks。
- [x] 阶段2: 审计 `record_cli` / `record_session` / agents-file summary 当前实现和测试入口。
- [x] 阶段3: 实现 summary evidence 展示,区分 semantic completion、wrapper exit、stdout tail/display state。
- [x] 阶段4: 补 focused tests,覆盖 missing termination、missing dynamic result coverage、completed dynamic instance display。
- [x] 阶段5: 勾选 tasks.md 4.x,运行 focused gates、OpenSpec validate、smoke/full tests,写 WORKLOG。

最终状态:
- `openspec/changes/clean-current-runtime-evidence-and-dynamic-role-contract/tasks.md` 已完成 1.x-4.x,当前进度 16/24。
- 当前 change 单项 strict validation 通过。
- `cargo test -p ralph-core smoke_runner --quiet` 通过。
- `cargo test --quiet` 通过。

未纳入本轮:
- 5.x evidence-index correlation。
- 6.x release-fast gate and dogfood evidence。
- unrelated `agent-cli-recoverable-failure-retry` 的 OpenSpec delta 修复。

## [2026-05-25 13:09:30] [Session ID: omx-1779158263949-kticiv] 计划: 落地 5.x Evidence index correlation

目标:
- 继续 `clean-current-runtime-evidence-and-dynamic-role-contract`。
- 本轮接着完成 5.1-5.3: evidence-index 支持 dynamic role contract hash、spawn request id、missing dynamic result markers 的关联查询。

阶段:
- [ ] 阶段1: 审计 `evidence_index.rs` 数据结构、writer/reader 和现有 lookup tests。
- [ ] 阶段2: 设计最小新增 entry,保持 evidence index 只做 artifact correlation,不复制完整 record-session / role contract。
- [ ] 阶段3: 实现 lookup by role_contract_hash / spawn_request_id / missing dynamic result marker。
- [ ] 阶段4: 补 focused tests 并勾选 5.1-5.3。
- [ ] 阶段5: 跑 focused gates,然后继续 6.x。

约束:
- 不把 evidence-index 做成新的 truth source。
- 不提前实现 retry 或 UI/TUI。
- 不修改 unrelated active change。

当前状态:
- 正在阶段1,准备读取 evidence index 实现和测试。

## [2026-05-25 13:18:00] [Session ID: omx-1779158263949-kticiv] 进展: 继续 5.x Evidence index correlation

当前承接:
- 用户要求“继续”,上一轮停在 5.x evidence-index correlation。
- 1.x-4.x 已完成,本轮继续 5.1-5.3。

本轮行动:
- 先运行 OpenSpec apply 状态命令,确认 pending tasks 和 context files。
- 读取 change 的 design/spec/tasks 与 `crates/ralph-core/src/evidence_index.rs` 当前实现。
- 跑 focused formatting/tests,根据失败结果修正 evidence-index kernel。

当前状态:
- 正在阶段1/2,目标是保持 evidence-index 为 artifact correlation,不成为新的 truth source。

## [2026-05-25 13:24:00] [Session ID: omx-1779158263949-kticiv] 决策: 5.x evidence-index 需要补齐 result topic 和 lineage lookup

现象:
- `EvidenceIndexEntry::dynamic_role_contract` 当前可保存 role_contract_hash、spawn_request_id、instance_id 和 artifact_path。
- `missing_dynamic_result` 当前把 request id 放在 parent,把 role_contract_hash 放在 child,但 `find_by_correlation` 只匹配主 correlation_id。
- OpenSpec 明确要求 dynamic role correlation entry 能 link spawn request id、role contract hash、instance id、produced result topic。

当前假设:
- 正确修正是扩展 index entry 的最小 correlation metadata,增加 optional result_topic。
- `find_by_correlation` 应匹配 primary/parent/child correlation id,这样 lookup by spawn request id 能同时看见 child artifacts 和 missing markers。

备选解释:
- 也可以新增专门的 `find_related_by_correlation`,但这会让调用方需要知道两套查找入口,增加误用风险。

下一步:
- 在 `EvidenceIndexEntry` 增加可选 `result_topic` 字段和 builder/helper。
- 扩展 lookup 语义并补 tests: role hash -> source artifact + result topic, spawn request -> children + missing marker, missing marker vs terminal failure vs no entry。

## [2026-05-25 13:31:00] [Session ID: omx-1779158263949-kticiv] 进展: 5.x evidence-index correlation 已实现并勾选

阶段完成情况:
- [x] 阶段1: 审计 `evidence_index.rs` 数据结构、writer/reader 和现有 lookup tests。
- [x] 阶段2: 设计最小新增 entry,保持 evidence index 只做 artifact correlation,不复制完整 record-session / role contract。
- [x] 阶段3: 实现 lookup by role_contract_hash / spawn_request_id / missing dynamic result marker。
- [x] 阶段4: 补 focused tests 并勾选 5.1-5.3。
- [ ] 阶段5: 跑 OpenSpec validate、diff check 和后续 release-fast gates。

已实现:
- `EvidenceIndexEntry` 增加 optional `result_topic`,只保存 produced/expected topic 名称。
- 增加 `dynamic_role_result_topic` helper,把 role_contract_hash、spawn_request_id、instance_id、result_topic 和 artifact path 关联起来。
- `find_by_correlation` 扩展为匹配 primary / parent / child / result_topic,让 request id 和 role hash 能查到 lineage 上的 missing marker。
- tests 覆盖 role hash source artifact、spawn request children + missing marker、missing marker 与 terminal failure 区分。

已通过:
- `cargo test -p ralph-core evidence_index --lib --quiet`: 11 passed。

## [2026-05-25 13:38:00] [Session ID: omx-1779158263949-kticiv] 进展: 进入 6.x release-fast gate

行动:
- 查找现有 release-fast / dynamic spawn guardrail 入口。
- 优先补 repo 内可复验命令或测试,不扩 UI/retry。

## [2026-05-25 13:49:00] [Session ID: omx-1779158263949-kticiv] 进展: 6.2 integration guardrail 已扩展

结果:
- topology spawn integration 现已验证 evidence-index request id、role hash、event log、agents snapshot artifact links。
- 曾遇到 macOS `/var` 与 `/private/var` 临时目录文本差异,已改为 artifact path 后缀断言。

## [2026-05-25 14:00:00] [Session ID: omx-1779158263949-kticiv] 进展: 执行 6.5 preserved dogfood

行动: 用 custom backend 在 `/tmp` 保留 topology spawn record-session、agents、events、evidence-index artifacts。
