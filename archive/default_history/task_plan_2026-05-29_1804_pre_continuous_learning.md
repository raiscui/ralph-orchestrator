# 任务计划: 归档 clean-current-runtime-evidence-and-dynamic-role-contract

## [2026-05-25 14:20:00] [Session ID: omx-1779158263949-kticiv] 计划: task_plan 续档后继续 OpenSpec 归档

目标:
- 在不污染 retry 分支的前提下,归档已完成的 `clean-current-runtime-evidence-and-dynamic-role-contract` OpenSpec change。

背景:
- 旧 `task_plan.md` 已 995 行,继续追加会越过 1000 行边界。
- 已将旧计划续档为 `task_plan_2026-05-25_1420_pre_archive_clean_runtime_evidence.md`。
- 上一轮已完成 tasks.md 24/24,并通过 focused gates、full `cargo test --quiet`、preserved dogfood。

阶段:
- [x] 阶段1: 续档旧 `task_plan.md`,新建当前计划入口。
- [ ] 阶段2: 做最小 continuous-learning 摘要,把可复用经验沉淀到 `EXPERIENCE.md`。
- [ ] 阶段3: 检查 OpenSpec change 状态、任务完成状态和 delta specs 同步状态。
- [ ] 阶段4: 执行 archive,并验证 archive 后状态。
- [ ] 阶段5: 写 WORKLOG 并交付结果。

约束:
- 不处理 `agent-cli-recoverable-failure-retry`。
- 不新增 UI/retry 功能。
- 不回滚或触碰非本轮产生的既有改动。

## [2026-05-25 14:26:00] [Session ID: omx-1779158263949-kticiv] 进展: archive 前状态检查

现象:
- `openspec status --change clean-current-runtime-evidence-and-dynamic-role-contract --json` 显示 artifacts 全部 done。
- `openspec list --json` 显示该 change 状态为 complete,24/24。
- 命令结束后出现 PostHog telemetry `ECONNRESET`,这是遥测 flush 网络错误,不是 OpenSpec change 状态错误。

下一步:
- 用禁用遥测的环境变量重跑关键 OpenSpec validate/status。
- 做 delta specs 与主 specs 同步检查。

## [2026-05-26 00:10:00] [Session ID: omx-1779158263949-kticiv] 进展: 继续归档收尾并同步规格

当前承接:
- 上一动作已把接近 1000 行的旧计划续档到 `archive/default_history/task_plan_2026-05-25_1420_pre_archive_clean_runtime_evidence.md`。
- 当前 change 已确认 artifacts done 且 tasks 24/24。
- Delta 检查发现 5 个 capability 的新增 requirements 尚未同步到 `openspec/specs/` 主规格。

本轮行动:
- 将 runtime evidence closure 的可复用结论沉淀到 `EXPERIENCE.md`,完成本次续档触发的最小 continuous-learning。
- 将 5 个 delta specs 幂等同步到主规格,其中创建新的 `current-runtime-evidence-contract` 主 spec。
- 使用禁用 telemetry 的 OpenSpec 校验命令验证同步结果,再把完成的 change 归档到 2026-05-26 归档目录。

约束:
- `agent-cli-recoverable-failure-retry` 仍为独立未完成 change,不在本次修复范围内。

## [2026-05-26 00:16:00] [Session ID: omx-1779158263949-kticiv] 进展: 最小 continuous-learning 已沉淀

已完成:
- 向 `EXPERIENCE.md` 追加 `exp-20260526-runtime-evidence-closure-and-dynamic-role-index`。
- 该经验覆盖 evidence-index 作为导航索引、record-session semantic completion、dynamic role contract closure 和 preserved dogfood artifact policy。

阶段状态:
- [x] 阶段2: 做最小 continuous-learning 摘要,把可复用经验沉淀到 `EXPERIENCE.md`。
- [ ] 阶段3: 检查 OpenSpec change 状态、任务完成状态和 delta specs 同步状态。

## [2026-05-26 00:20:00] [Session ID: omx-1779158263949-kticiv] 进展: delta specs 已同步到主规格

同步结果:
- 创建 `openspec/specs/current-runtime-evidence-contract/spec.md`。
- 更新 `openspec/specs/parallel-hat-instances/spec.md`。
- 更新 `openspec/specs/prompt-contract-runtime-alignment/spec.md`。
- 更新 `openspec/specs/record-session-contract-and-watch/spec.md`。
- 更新 `openspec/specs/runtime-evidence-index-kernel/spec.md`。

阶段状态:
- [x] 阶段3: 检查 OpenSpec change 状态、任务完成状态和 delta specs 同步状态。
- [ ] 阶段4: 执行 archive,并验证 archive 后状态。

## [2026-05-26 00:28:00] [Session ID: omx-1779158263949-kticiv] 完成: OpenSpec change 已归档

完成内容:
- 已将 `openspec/changes/clean-current-runtime-evidence-and-dynamic-role-contract` 归档到 `openspec/changes/archive/2026-05-26-clean-current-runtime-evidence-and-dynamic-role-contract/`。
- 已创建归档 manifest: `archive/manifests/ARCHIVE_MANIFEST__default_task_plan_rollover_2026-05-26_0010.md`。
- 已同步 5 个 delta specs 到主规格。

验证:
- 5 个主规格单项 strict validate 均通过。
- 归档前 change strict validate 通过。
- 归档后 `openspec list --json` 不再列出该 change。
- `openspec validate --all --strict` 仍有 1 个 unrelated failure: `agent-cli-recoverable-failure-retry` no-delta。

阶段状态:
- [x] 阶段4: 执行 archive,并验证 archive 后状态。
- [ ] 阶段5: 写 WORKLOG 并交付结果。

## [2026-05-26 00:31:00] [Session ID: omx-1779158263949-kticiv] 完成: 归档收尾已记录

阶段状态:
- [x] 阶段1: 续档旧 `task_plan.md`,新建当前计划入口。
- [x] 阶段2: 做最小 continuous-learning 摘要,把可复用经验沉淀到 `EXPERIENCE.md`。
- [x] 阶段3: 检查 OpenSpec change 状态、任务完成状态和 delta specs 同步状态。
- [x] 阶段4: 执行 archive,并验证 archive 后状态。
- [x] 阶段5: 写 WORKLOG 并交付结果。

最终状态:
- `clean-current-runtime-evidence-and-dynamic-role-contract` 已归档。
- 当前剩余 OpenSpec active changes 是 `agent-cli-recoverable-failure-retry` 和 `tui-mdfried-viewer`。

## [2026-05-26 00:38:00] [Session ID: omx-1779158263949-kticiv] 计划: 继续 agent-cli-recoverable-failure-retry

目标:
- 继续 active OpenSpec change `agent-cli-recoverable-failure-retry`,先补齐缺失 specs delta,解除当前 `openspec validate --all --strict` 的 no-delta 阻断。

已知现象:
- active change 目前只有 `.openspec.yaml` 和 `proposal.md`。
- 全量 strict 当前失败在该 change: 没有 specs delta。

阶段:
- [ ] 阶段1: 读取 proposal 和 OpenSpec status/instructions。
- [ ] 阶段2: 创建 specs delta artifact。
- [ ] 阶段3: validate 该 change,确认 no-delta 已解除。
- [ ] 阶段4: 根据新 status 决定下一 artifact,后续继续 design/tasks/implementation。

约束:
- 不混入刚归档的 runtime evidence 主线。

## [2026-05-28 13:59:00] [Session ID: codex-20260528-135644] 索引: 启用项目演进分析支线上下文

原因:
- 用户请求分析项目有哪些可以演进的地方,这是横跨架构、测试、文档和运行态证据的只读分析任务。
- 默认 `notes.md` 已超过 1000 行,继续追加会触发续档与 continuous-learning,不适合把本轮分析混入当前主线。

支线:
- 后缀: `__evolution_analysis`
- 主题: `ralph-orchestrator` 项目演进机会分析。
- 本轮只读分析 Rust 源码、OpenSpec、任务、文档和测试覆盖,不实施代码修改。
- 保持这条线聚焦 agent CLI 429 / retry limit / recoverable failure lifecycle。

## [2026-05-26 00:48:00] [Session ID: omx-1779158263949-kticiv] 进展: agent-cli recoverable retry design 已创建

完成:
- 创建 `openspec/changes/agent-cli-recoverable-failure-retry/design.md`。
- 设计明确了 append-only ledger、窄分类器、显式 retry policy、paused/retryable 生命周期和 manual continue 控制面。

下一步:
- 创建 specs delta artifact,解除 no-delta validation failure。

## [2026-05-26 15:08:30] [Session ID: omx-1779158263949-kticiv] 进展: 接续 agent-cli recoverable retry specs delta

行动目的:
- 解除 `agent-cli-recoverable-failure-retry` 当前 `openspec validate --all --strict` 的 no-delta 阻断。

当前边界:
- 本轮先补 OpenSpec specs delta 和必要的任务 artifact。
- 暂不实现代码。
- 不改变 stderr/stdout event parsing 边界。

下一步:
- 读取 change status、instructions、proposal 和 design。
- 创建 `specs/*/spec.md` delta 文件并 strict validate。

## [2026-05-26 15:10:30] [Session ID: omx-1779158263949-kticiv] 进展: specs delta 已创建,准备 strict validate

已创建:
- `openspec/changes/agent-cli-recoverable-failure-retry/specs/agent-cli-recoverable-failure-retry/spec.md`
- `openspec/changes/agent-cli-recoverable-failure-retry/specs/parallel-hat-instances/spec.md`
- `openspec/changes/agent-cli-recoverable-failure-retry/specs/supervisor-human-chat-gate/spec.md`

验证计划:
- 运行当前 change strict validate。
- 如果通过,读取 status,确认 `tasks.md` 是否解锁。

## [2026-05-26 15:11:30] [Session ID: omx-1779158263949-kticiv] 完成: specs delta 解除 no-delta 阻断

完成内容:
- 创建 3 个 specs delta 文件,覆盖新 capability 和两个 modified capability。
- `openspec validate agent-cli-recoverable-failure-retry --type change --strict` 通过。
- `openspec validate --all --strict` 通过,结果为 28 passed,0 failed。

阶段状态:
- [x] 阶段1: 读取 proposal 和 OpenSpec status/instructions。
- [x] 阶段2: 创建 specs delta artifact。
- [x] 阶段3: validate 该 change,确认 no-delta 已解除。
- [x] 阶段4: 根据新 status 决定下一 artifact。

下一步:
- `tasks.md` 已从 blocked 变为 ready。
- 按 `openspec-continue-change` 的单次 artifact guardrail,本轮在创建 specs 后停止;下一轮应创建 `openspec/changes/agent-cli-recoverable-failure-retry/tasks.md`。

## [2026-05-26 15:17:30] [Session ID: omx-1779158263949-kticiv] 进展: 开始创建 agent-cli recoverable retry tasks artifact

行动目的:
- 创建 `openspec/changes/agent-cli-recoverable-failure-retry/tasks.md`,让该 OpenSpec change 从 artifact 阶段进入可实施阶段。

当前状态:
- proposal/design/specs 均已 done。
- tasks 当前 ready。

约束:
- 本步骤只写任务清单,不实现代码。
- 任务必须可验证,且按依赖顺序排列。

## [2026-05-26 15:20:30] [Session ID: omx-1779158263949-kticiv] 完成: agent-cli recoverable retry tasks artifact 已创建

完成内容:
- 创建 `openspec/changes/agent-cli-recoverable-failure-retry/tasks.md`。
- 任务清单按依赖拆成 6 组: core model/policy、ledger/replay、parallel lifecycle、manual continue、human-facing evidence、integration/final validation。
- 当前 change status 已变为 `isComplete: true`。

验证:
- `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate agent-cli-recoverable-failure-retry --type change --strict` 通过。
- `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate --all --strict` 通过,结果 28 passed,0 failed。

下一步:
- 如继续推进,应进入 OpenSpec apply/implementation 阶段,从 `tasks.md` 的 1.1 开始。
- 实现阶段需要保持 stdout-only event parsing 不变量,并先做 focused tests。

## [2026-05-28 09:47:45] [Session ID: omx-1779158263949-kticiv] 计划: 实现 agent-cli recoverable retry tasks 1.x

目标:
- 只实现 `openspec/changes/agent-cli-recoverable-failure-retry/tasks.md` 的 1.x: core recoverable failure model and policy。

范围:
- 新增 recoverable failure domain types。
- 新增 `.ralph/recoverable-failures.jsonl` SSOT path resolver。
- 新增 narrow deterministic classifier 和 focused tests。
- 新增 retry policy config defaults / parsing / validation tests。

边界:
- 暂不接入 parallel runtime retry lifecycle。
- 暂不实现 ledger append/replay IO。
- 保持 stderr 只作为分类证据,不进入 event parsing。

验证计划:
- 先运行 focused Rust tests。
- 再运行 OpenSpec change strict validate。
- 若改动触及配置结构,运行相关 config tests。

## [2026-05-28 09:59:30] [Session ID: omx-1779158263949-kticiv] 完成: agent-cli recoverable retry tasks 1.x

完成内容:
- 新增 `crates/ralph-core/src/recoverable_failure.rs`,包含 recoverable failure domain types、retry policy、transition/snapshot、HatJobResult-like 输入和 deterministic classifier。
- `RalphConfig` 新增顶层 `agent_cli_recoverable_failures` 配置,默认 enabled=true,max_attempts=3,initial_delay_ms=30000,backoff_multiplier=2.0,max_delay_ms=300000。
- `CoreConfig` 新增 `.ralph/recoverable-failures.jsonl` 的唯一解析入口 `resolve_recoverable_failures_ledger_path()`。
- `openspec/changes/agent-cli-recoverable-failure-retry/tasks.md` 已勾选 1.1-1.6。
- 顺手修复 `crates/ralph-core/src/event_emission_protocol.rs` 中 clippy 报出的 `unnecessary_join` / `format_collect` warning,无行为变更。

验证:
- `cargo test -p ralph-core --lib recoverable --quiet`: 16 passed。
- `cargo test -p ralph-core --lib config::tests::test_default_config -- --exact`: passed。
- `cargo test -p ralph-core --lib config::tests::test_core_config_resolves_scoped_experience_paths -- --exact`: passed。
- `cargo clippy -p ralph-core --quiet`: passed,无输出。
- `cargo test -p ralph-core --quiet`: package tests passed。
- `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate --all --strict`: 28 passed,0 failed。

遇到错误:
- 曾错误执行 `cargo test ... --exact` 导致 Cargo 提示 `unexpected argument '--exact' found`;已改用 `cargo test ... -- --exact` 重跑并通过。

当前进度:
- OpenSpec apply progress: 6/34 tasks complete。
- 下一步应进入 2.x: append-only ledger and replay snapshot。

## [2026-05-28 10:01:00] [Session ID: omx-1779158263949-kticiv] 复核: apply progress 与命令错误修正

复核结果:
- `openspec instructions apply --change agent-cli-recoverable-failure-retry --json` 显示 progress 为 34 total,6 complete,28 remaining。

遇到错误:
- 曾用错误的 shell pipe + heredoc 写法解析 apply JSON,导致 Python 报 `NameError: name 'true' is not defined`。
- 已改为临时文件方式读取 JSON 并成功解析。

结论:
- 该错误属于验证脚本写法问题,不是项目代码问题。

## [2026-05-28 10:28:30] [Session ID: omx-1779158263949-kticiv] 计划: 实现 agent-cli recoverable retry tasks 2.x

目标:
- 实现 `openspec/changes/agent-cli-recoverable-failure-retry/tasks.md` 的 2.x: append-only ledger and replay snapshot。

范围:
- 为 `.ralph/recoverable-failures.jsonl` 增加 append-only JSONL 写入。
- 增加 ledger replay,按 `failure_id` 派生最新 snapshot。
- 增加 append ordering、多 transition、malformed line、bounded stderr excerpt 和 compact metadata tests。

边界:
- 不接入 parallel runtime lifecycle。
- 不实现 scheduler / retry enqueue。
- 不存 full prompt,不复制 raw event stream。

验证计划:
- focused recoverable tests。
- `cargo clippy -p ralph-core --quiet`。
- `cargo test -p ralph-core --quiet`。
- `openspec validate --all --strict`。

## [2026-05-28 10:34:30] [Session ID: omx-1779158263949-kticiv] 完成: agent-cli recoverable retry tasks 2.x

完成内容:
- `RecoverableFailureLedger` 已支持 append-only JSONL 写入。
- `read_transitions()` 可严格读取 transition,缺失 ledger 返回空集合,malformed line 返回带行号错误。
- `replay_snapshots()` 可按 `failure_id` 派生最新 `RecoverableFailureSnapshot`。
- 新增 `stable_recoverable_failure_id()`,只使用 job/instance/failure_kind correlation metadata,不包含 prompt/payload/stderr。
- append 时会再次收紧 stderr excerpt,避免调用方误写大块 stderr 或 prompt transcript。
- `openspec/changes/agent-cli-recoverable-failure-retry/tasks.md` 已勾选 2.1-2.4。

验证:
- `cargo test -p ralph-core --lib recoverable --quiet`: 23 passed。
- `cargo clippy -p ralph-core --quiet`: passed,无输出。
- `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate agent-cli-recoverable-failure-retry --type change --strict`: passed。
- `cargo test -p ralph-core --quiet`: package tests passed,633 lib tests plus integration/doctests passed。
- `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate --all --strict`: 28 passed,0 failed。

当前进度:
- OpenSpec apply progress: 10/34 tasks complete。
- 下一步应进入 3.x: Parallel runtime retry lifecycle。

## [2026-05-28 11:20:47] [Session ID: omx-1779158263949-kticiv] 计划: 实现 agent-cli recoverable retry tasks 3.x

目标:
- 进入 `openspec/changes/agent-cli-recoverable-failure-retry/tasks.md` 的 3.x: Parallel runtime retry lifecycle。
- 把 recoverable failure 从 ledger/evidence 层接入 parallel runtime 的 job lifecycle。

范围:
- 3.1: failed `HatJobResult` 先分类 recoverable,再走普通 terminal failure。
- 3.2: 让 retry-aware lifecycle 对 supervisor 可见,至少覆盖 `retry_scheduled`, `paused_recoverable`, `retrying`, `exhausted`, `continued_by_human` 的 runtime transition 语义。
- 3.3: 增加可测试的 bounded backoff 调度。
- 3.4: retry 只能用 runtime-held job context 重新 enqueue,ledger 只做 metadata/correlation evidence。
- 3.5: recoverable pending/scheduled/retrying 时阻止 coordinator completion promise 直接收敛。
- 3.6: retry 耗尽后转为 terminal job failure,并带 ledger evidence pointer。
- 3.7: 保持 stdout-only event parsing,stderr 只可参与 retry classification,不得进入 `output_for_parsing`。

当前候选设计:
- 优先在 `HatInstanceActor` 内保存运行中 job context,因为 pending events 在 `maybe_start_job` 里会被 take 掉。
- 用新的 recoverable transition event 通知 supervisor,让 supervisor 用 in-memory pending map 阻止 completion。
- ledger 继续保持 append-only metadata,不存 prompt,不存 event stream。

验证计划:
- 先补 focused unit tests 锁住 scheduler/classification/runtime transition。
- 再跑 `cargo test -p ralph-core --lib recoverable --quiet` 和相关 parallel lifecycle tests。
- 保持 `cargo clippy -p ralph-core --quiet` 无输出。
- 最后跑 OpenSpec change strict validate。

风险:
- 现有 `HatInstanceState` 枚举只有 Created/Running/Idle/Done/Failed,直接扩 enum 可能牵动 UI/agents snapshot。3.x 先优先通过 transition event + supervisor map 表达 retry lifecycle,避免提前扩 5.x 可视化范围。
- 当前 status enum 没有 `recovered/resolved`,如果 retry 后成功但 ledger 仍停在 `retrying`,会造成 evidence 语义不闭环。实现时需要确认是否补一个 terminal recovered status。

## [2026-05-28 12:14:13] [Session ID: omx-1779158263949-kticiv] 进展: 3.x runtime retry lifecycle 初步接线完成

已完成:
- `recoverable_failure.rs` 增加 `Recovered` lifecycle status 和 bounded backoff helper。
- `HatInstanceActor` 已保存 runtime-held job context,并在 recoverable failure 后调度 retry。
- `HatInstanceActor` retry 成功会写入 `recovered` transition,失败耗尽会写入 `exhausted` 并把 ledger pointer 附到 terminal stderr evidence。
- `ParallelSupervisor` 已消费 recoverable transition,维护 live snapshot,并在 pending recoverable 存在时阻止 completion promise。
- 已处理 completion freeze 与 recoverable transition 的竞态: pending recoverable 出现时重新打开 supervisor loop,actor 调度 retry 时解除自身 freeze。

验证:
- `cargo check -p ralph-core --quiet`: 通过。
- `cargo test -p ralph-core --lib recoverable --quiet`: 27 passed。
- `cargo test -p ralph-core --lib parallel::instance::tests::recoverable_failure_schedules_retry_and_preserves_stdout_only_parsing -- --exact`: 通过。
- `cargo test -p ralph-core --lib parallel::supervisor::routing_tests::pending_recoverable_failures_block_completion_gate -- --exact`: 通过。
- `cargo clippy -p ralph-core --quiet`: 通过,无输出。

下一步:
- 将 `recovered` 终态同步到 OpenSpec design/spec,然后跑 package tests 与 OpenSpec validate。

## [2026-05-28 12:26:41] [Session ID: omx-1779158263949-kticiv] 完成: agent-cli recoverable retry tasks 3.x

完成内容:
- `openspec/changes/agent-cli-recoverable-failure-retry/tasks.md` 已勾选 3.1-3.7。
- parallel runtime 已能对 failed `HatJobResult` 先做 recoverable classification,再决定 scheduled retry 或 exhausted terminal failure。
- retry 调度使用 bounded backoff helper,测试中通过 Tokio paused time 做 deterministic 验证。
- retry 重新执行使用 `HatInstanceActor` 内存保存的 `HatJob` context,ledger 只保存 compact metadata/correlation evidence。
- Supervisor 已消费 `RecoverableFailureTransition`,以 live snapshot 阻止 pending recoverable 时的 completion promise。
- exhausted failure 会转为 terminal `JobCompleted`,并在 stderr evidence 里附带 `recoverable_failure_id` 与 ledger path。
- stderr 仍只参与 classification,没有进入 `output_for_parsing` 或 EventParser。
- 由于 retry 成功后需要关闭 lifecycle,实现补充了 `recovered` status,并同步到 OpenSpec design/spec。

验证:
- `cargo check -p ralph-core --quiet`: 通过。
- `cargo test -p ralph-core --lib recoverable --quiet`: 28 passed。
- `cargo test -p ralph-core --lib parallel::instance::tests::recoverable_failure_schedules_retry_and_preserves_stdout_only_parsing -- --exact`: 通过。
- `cargo test -p ralph-core --lib parallel::instance::tests::recoverable_failure_exhaustion_becomes_terminal_with_ledger_pointer -- --exact`: 通过。
- `cargo test -p ralph-core --lib parallel::supervisor::routing_tests::pending_recoverable_failures_block_completion_gate -- --exact`: 通过。
- `cargo clippy -p ralph-core --quiet`: 通过,无输出。
- `cargo test -p ralph-core --quiet`: 638 lib tests plus integration/doctests passed。
- `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate agent-cli-recoverable-failure-retry --type change --strict`: 通过。
- `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate --all --strict`: 28 passed,0 failed。

当前进度:
- OpenSpec apply progress: 17/34 tasks complete。
- 下一步应进入 4.x: Manual continue control path。

## [2026-05-28 12:32:55] [Session ID: omx-1779158263949-kticiv] 复核: OpenSpec apply progress 解析命令修正

遇到错误:
- 第一次使用内联 Python f-string 解析 `openspec instructions apply --json` 输出时,因为 shell quoting 导致 `SyntaxError`。
- 第二次用 `sed '1d'` 去掉前置日志时仍不稳,Python 收到额外数据并报 `JSONDecodeError: Extra data`。

修正:
- 改为先把 OpenSpec 输出写入临时文件,再从第一个 `{` 开始解析 JSON。
- 修正后输出: `progress=17/34 remaining=17`,下一项为 `4.1 Extend Supervisor chat parsing with explicit !continue and !continue <failure_id> control intent.`

结论:
- 错误属于验证脚本/管道写法问题,不是项目代码或 OpenSpec artifact 问题。

## [2026-05-28 14:11:22] [Session ID: omx-1779158263949-kticiv] 计划: 实现 agent-cli recoverable retry tasks 4.x

目标:
- 进入 `openspec/changes/agent-cli-recoverable-failure-retry/tasks.md` 的 4.x: Manual continue control path。
- 让 human 通过显式 `!continue` / `!continue <failure_id>` 控制动作恢复 recoverable retry lifecycle。

范围:
- 4.1: Supervisor chat parsing 支持明确 continue control intent。
- 4.2: plain chat 例如 `继续分析这个问题` 仍是普通 chat,不能隐式 retry。
- 4.3: 显式 `failure_id` 需要解析到 recoverable snapshot,unknown / terminal 要可见拒绝。
- 4.4: bare `!continue` 只在唯一 paused/scheduled recoverable failure 时生效,否则拒绝歧义。
- 4.5: accepted manual continue 先写 `continued_by_human`,再通过同一 scheduler/retry path 继续。

候选设计:
- 优先复用 3.x `HatInstanceActor` 的 retry scheduler path,不要另做一套 manual retry executor。
- Supervisor 负责把 human/external control 解析为对具体 instance 的命令。
- Instance 负责持有 runtime job context 并执行 retry,ledger 仍只做 metadata/correlation evidence。

验证计划:
- 先读 Supervisor human/external event 代码和 instance command 模型。
- 增加 parser tests 锁住 `!continue` 与普通中文聊天的边界。
- 增加 actor/supervisor focused tests 覆盖 explicit failure_id、bare ambiguity、terminal rejection 和 accepted continue。
- 跑 focused tests、`cargo clippy -p ralph-core --quiet`、`cargo test -p ralph-core --quiet`、OpenSpec strict validate。

风险:
- 3.x 当前 scheduled retry 发生在 instance 内部,Supervisor 只有 live snapshot。4.x 若要 manual continue,需要确保 Supervisor 能把 failure id 找回到对应 instance,同时不能从 ledger 重建 prompt。
- 如果 runtime-held context 已丢失,manual continue 必须拒绝或可见报错,不能偷偷根据 ledger 重建 job。

## [2026-05-28 14:18:30] [Session ID: omx-1779158263949-kticiv] 行动: 读取 4.x control path 代码入口

目的:
- 进入 4.x `Manual continue control path`,先确认 chat parser、Supervisor external event loop、instance command/retry scheduler 的真实代码路径。
- 避免把 `!continue` 做成绕过 3.x retry scheduler 的旁路。

将要做:
- 读取 OpenSpec apply instructions 与 4.x specs/tasks。
- 用 CodeGraph 获取 Supervisor/Instance/TUI chat parser 的结构上下文。
- 检查当前 git 状态,只触碰本轮相关文件,不处理无关改动。

## [2026-05-28 14:25:10] [Session ID: omx-1779158263949-kticiv] 行动: 增加 explicit recoverable continue 控制意图

目的:
- 先完成 4.1/4.2 的 control surface,把 `!continue` 与普通 chat 文本分开。

将要做:
- 增加 `recoverable.continue` topic 常量,作为 external JSONL control event。
- 扩展 TUI `ChatSubmit` 和 `parse_chat_submit`,支持 `!continue` / `!continue <failure_id>`。
- 补 parser tests,证明 `继续分析这个问题` 仍是普通 `human.message`。

## [2026-05-28 14:32:40] [Session ID: omx-1779158263949-kticiv] 行动: 接入 Supervisor manual continue 解析和目标解析

目的:
- 完成 4.3/4.4: `!continue <failure_id>` 必须按 live recoverable snapshot 解析,unknown/terminal 要可见拒绝; bare `!continue` 只能在唯一 pending failure 时继续。

将要做:
- Supervisor 在 `route_event` 早期消费 `recoverable.continue`。
- 增加解析 payload 与 pending snapshot helper。
- 接受时向对应 instance 发送 `ContinueRecoverableFailure`,拒绝时写 `routing.escalate` 作为可审计错误。

## [2026-05-28 14:38:20] [Session ID: omx-1779158263949-kticiv] 错误: 4.x 初次 cargo check 失败

现象:
- `cargo check -p ralph-core --quiet` 失败。
- 错误 1: `escalate_delivery_failure` 是 `routing.rs` 私有方法,`supervisor.rs` 无法调用。
- 错误 2: `TOPIC_RECOVERABLE_CONTINUE` import 未使用。

修复计划:
- 将 `escalate_delivery_failure` 改为 `pub(super)`,因为它仍只暴露给 `parallel::supervisor` 模块内部。
- 移除 `supervisor.rs` 中未使用 import。

## [2026-05-28 14:45:10] [Session ID: omx-1779158263949-kticiv] 进展: 4.x focused tests 初步通过

已验证:
- `cargo check -p ralph-core --quiet`: 通过。
- `cargo test -p ralph-tui chat --quiet`: 24 passed。
- `cargo test -p ralph-core --lib parallel::instance::tests::manual_continue_appends_transition_and_uses_scheduled_retry_path -- --exact`: 通过。
- `cargo test -p ralph-core --lib parallel::supervisor::routing_tests::explicit_recoverable_continue_accepts_only_waiting_failures -- --exact`: 通过。
- `cargo test -p ralph-core --lib parallel::supervisor::routing_tests::bare_recoverable_continue_uses_selected_instance_to_disambiguate -- --exact`: 通过。

继续调整:
- bare `!continue` 的解析应表达“selected instance 或 selected/global recoverable failure 不歧义”。
- 当前实现对 selected instance 无匹配时直接拒绝,会让 TUI 在选中 ralph#1 但全局只有一个 worker failure 时体验过硬。
- 下一步收紧为: selected 有唯一匹配则接受; selected 多匹配则拒绝; selected 无匹配则回退全局唯一判断。

## [2026-05-28 14:49:00] [Session ID: omx-1779158263949-kticiv] 行动: 运行 4.x 验证门禁

目的:
- 在勾选 4.1-4.5 前完成格式化、clippy、focused/package/OpenSpec 验证。

将要做:
- `cargo fmt`。
- `cargo clippy -p ralph-core --quiet` 和 `cargo clippy -p ralph-tui --quiet`。
- `cargo test -p ralph-core --quiet`。
- `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate agent-cli-recoverable-failure-retry --type change --strict`。

## [2026-05-28 14:55:30] [Session ID: omx-1779158263949-kticiv] 进展: 4.x 第一轮验证通过,补跑 TUI 与全量 OpenSpec

已通过:
- `cargo fmt`: 通过。
- `cargo clippy -p ralph-core --quiet`: 通过,无输出。
- `cargo clippy -p ralph-tui --quiet`: 通过,无输出。
- `cargo test -p ralph-core --quiet`: 642 lib tests plus integration/doctests passed。
- `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate agent-cli-recoverable-failure-retry --type change --strict`: passed。

继续验证:
- 因为本轮改了 `crates/ralph-tui/src/chat.rs` 和 `crates/ralph-tui/src/app.rs`,补跑 `cargo test -p ralph-tui --quiet`。
- 补跑 `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate --all --strict`。

## [2026-05-28 15:00:30] [Session ID: omx-1779158263949-kticiv] 完成: agent-cli recoverable retry tasks 4.x

完成内容:
- `openspec/changes/agent-cli-recoverable-failure-retry/tasks.md` 已勾选 4.1-4.5。
- TUI chat parser 支持 `!continue` 与 `!continue <failure_id>`,并把它们解析为明确 control intent。
- 普通聊天文本 `继续分析这个问题` 保持 `human.message`,不会触发 recoverable retry。
- TUI 将 continue control 写入 external JSONL topic `recoverable.continue`,并附带当前 selected instance 作为消歧提示。
- Supervisor 在普通路由前消费 `recoverable.continue`,根据 live recoverable snapshot 解析目标:
  - explicit failure id: unknown / non-waiting terminal 或 retrying 状态会拒绝并通过 `routing.escalate` 留证。
  - bare continue: selected instance 有唯一 waiting failure 则接受; selected 无匹配时回退全局唯一; 多个候选则拒绝歧义。
- Instance 增加 `ContinueRecoverableFailure` 命令,接受后先写 `continued_by_human`,再把已有 scheduled retry 的 due 时间提前为 now,复用 `maybe_start_scheduled_retry()` 路径执行。
- manual retry 仍复用 runtime-held `HatJob` context,没有从 ledger 重建 prompt/event stream。

验证:
- `cargo fmt`: 通过。
- `cargo clippy -p ralph-core --quiet`: 通过,无输出。
- `cargo clippy -p ralph-tui --quiet`: 通过,无输出。
- `cargo test -p ralph-tui chat --quiet`: 24 passed。
- `cargo test -p ralph-core --lib parallel::instance::tests::manual_continue_appends_transition_and_uses_scheduled_retry_path -- --exact`: 通过。
- `cargo test -p ralph-core --lib parallel::supervisor::routing_tests::explicit_recoverable_continue_accepts_only_waiting_failures -- --exact`: 通过。
- `cargo test -p ralph-core --lib parallel::supervisor::routing_tests::bare_recoverable_continue_uses_selected_instance_to_disambiguate -- --exact`: 通过。
- `cargo test -p ralph-core --lib parallel::supervisor::routing_tests::bare_recoverable_continue_falls_back_to_global_unique_when_selected_has_no_failure -- --exact`: 通过。
- `cargo test -p ralph-core --quiet`: 642 lib tests plus integration/doctests passed。
- `cargo test -p ralph-tui --quiet`: 239 lib tests plus integration/doctests passed。
- `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate agent-cli-recoverable-failure-retry --type change --strict`: passed。
- `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate --all --strict`: 28 passed,0 failed。

当前进度:
- OpenSpec apply progress: 22/34 tasks complete。
- 下一步应进入 5.x: Human-facing evidence and observability。

## [2026-05-28 15:05:20] [Session ID: omx-1779158263949-kticiv] 行动: 补跑 workspace cargo test

目的:
- 虽然 4.x 已通过 ralph-core / ralph-tui package tests 和 OpenSpec strict,但项目规则要求代码改动完成前运行 `cargo test`。

将要做:
- 执行 `cargo test --quiet`。
- 若出现 error/warning 相关失败,继续修复,不忽略。

## [2026-05-28 15:09:00] [Session ID: omx-1779158263949-kticiv] 验证: workspace cargo test 通过

验证:
- `cargo test --quiet`: workspace tests and doctests passed,exit code 0。

结论:
- 4.x manual continue control path 的代码与 OpenSpec tasks 已完成并通过当前门禁。
- OpenSpec apply progress 保持 22/34 complete。
- 下一步进入 5.x: Human-facing evidence and observability。

## [2026-05-28 15:14:30] [Session ID: omx-1779158263949-kticiv] 计划: 实现 agent-cli recoverable retry tasks 5.x

目标:
- 进入 `openspec/changes/agent-cli-recoverable-failure-retry/tasks.md` 的 5.x: Human-facing evidence and observability。
- 让 users 能在 agents snapshot / record summary / evidence inspect 里看见 recoverable retry lifecycle,包括 scheduled、continued、exhausted、recovered。

范围:
- 5.1: runtime observability surface 增加 recoverable summary fields,但不把 ledger 变成 prompt/event store。
- 5.2: `ralph agents` / agents snapshot 能说明 retry-aware/exhausted state,包含 failure_id、instance、attempt、next retry timing。
- 5.3: record-session summary/evidence inspect 能指向 recoverable ledger evidence。
- 5.4: 增加测试或 fixture,证明 failed attempt、manual continue、exhaustion 后均可观测。

候选设计:
- agents snapshot: 在每个 `AgentInstanceSnapshot` 增加可选 recoverable failure summaries,由 Supervisor 的 live `recoverable_failures` map 派生。
- record summary/evidence inspect: 读取 agents file 和/或 workspace `.ralph/recoverable-failures.jsonl` ledger,渲染 compact summary,只显示 metadata/path,不复制 prompt/event stream。
- 保持 ledger 为 SSOT,observability 只是派生视图。

验证计划:
- 先读 `agents_snapshot.rs` 和 `record_session.rs` 现有结构。
- 增加 focused tests 覆盖 snapshot 和 record summary render。
- 运行 `cargo fmt`, focused tests, `cargo test -p ralph-core --quiet`, `cargo test -p ralph-cli --quiet` 或相关 integration,OpenSpec strict。

## [2026-05-28 15:20:30] [Session ID: omx-1779158263949-kticiv] 行动: 实现 agents snapshot recoverable summary

目的:
- 完成 5.1/5.2 的第一部分: 让 `.ralph/agents.json` 对每个 instance 显示 recoverable retry lifecycle 摘要。

将要做:
- 在 `agents_snapshot.rs` 增加 summary-only `AgentRecoverableFailureSummary`。
- `AgentInstanceSnapshot` 增加 `recoverable_failures` 列表,只存 failure id/status/attempt/timing/ledger path 等 compact metadata。
- `ParallelSupervisor::build_agent_instance_snapshot` 从 live `recoverable_failures` map 派生实例级 summary。

## [2026-05-28 15:28:10] [Session ID: omx-1779158263949-kticiv] 行动: 补 5.x observability focused tests

目的:
- 证明 failed attempt / manual continue / exhaustion 等 recoverable 状态能在 human-facing evidence 面展示。

将要做:
- 在 Supervisor routing tests 中补 agents snapshot recoverable summary 派生测试。
- 在 record_session tests 中补 Evidence Inspect 渲染 recoverable failures。
- 在 agents integration 或 display 层补表格输出包含 Recoverable 摘要。

## [2026-05-28 15:36:20] [Session ID: omx-1779158263949-kticiv] 错误: 5.x agents recoverable 表格测试断言过强

现象:
- `cargo test -p ralph-cli --test integration_agents --quiet` 失败在 `test_agents_command_prints_recoverable_summary`。
- 失败断言: `stdout.contains("failure-wr")`。

原因:
- 表格列会对 `failure_id` 做 10 字符截断,测试不应假设完整前缀能原样显示。

修复:
- 改为断言稳定列名 `Recoverable`、状态 `retry_scheduled`、attempt `1/3` 和 job/instance 相关可见文本。

## [2026-05-28 15:59:08] [Session ID: omx-1779954714247-oab9zc] 行动: 处理 notes.md 超限并继续 recoverable retry 5.x

目的:
- 用户要求继续当前四步计划,第一步是先处理上下文续档风险。
- `notes.md` 已超过 1000 行,继续追加会违反六文件规则,需要先做续档和 continuous-learning 摘要。

将要做:
- 读取并摘要当前 `notes.md` 的最新有效内容,特别是与 recoverable retry / runtime evidence 相关的结论。
- 将旧 `notes.md` 轮转为带时间戳的历史文件,并创建新的 `notes.md` 当前入口。
- 必要时更新长期知识载体,然后继续修复 `test_agents_command_prints_recoverable_summary`。

停止条件:
- `notes.md` 回到可继续追加的当前文件。
- 旧 notes 有稳定归档路径和可追溯摘要。
- 不触碰无关业务改动,不回滚用户已有修改。

## [2026-05-28 16:01:12] [Session ID: omx-1779954714247-oab9zc] 进展: notes.md 超限续档完成,进入 5.x 测试修复

已完成:
- 旧 `notes.md` 已移动到 `archive/default_history/notes_2026-05-28_1559_pre_recoverable_retry_5x.md`。
- 新 `notes.md` 已创建,当前 35 行,包含 recoverable retry 5.x 接续摘要。
- 已创建归档 manifest: `archive/manifests/ARCHIVE_MANIFEST__default_notes_rollover_2026-05-28_1559.md`。

当前待办状态:
- [x] 1. 处理 `notes.md` 超限续档和 continuous-learning 摘要。
- [ ] 2. 修复 `test_agents_command_prints_recoverable_summary` 的过强断言。
- [ ] 3. 继续完成 5.x observability。
- [ ] 4. 进入 6.x integration guardrails 和最终验证。

下一步:
- 读取 `crates/ralph-cli/tests/integration_agents.rs` 中失败测试和相关 display 输出逻辑。
- 修改断言为稳定的人类可见字段,避免绑定表格截断细节。

## [2026-05-28 16:19:01] [Session ID: omx-1779954714247-oab9zc] 验证: integration_agents recoverable summary 断言修复通过

已修复:
- `crates/ralph-cli/tests/integration_agents.rs` 中 `test_agents_command_prints_recoverable_summary` 不再断言紧凑表格中必须出现 failure id 前缀。
- 新断言覆盖稳定人类可见字段: `Recoverable`、`writer#1`、`retry_scheduled`、`1/3`。

动态证据:
- `cargo test -p ralph-cli --test integration_agents test_agents_command_prints_recoverable_summary -- --exact --nocapture`: 1 passed。

当前待办状态:
- [x] 1. 处理 `notes.md` 超限续档和 continuous-learning 摘要。
- [x] 2. 修复 `test_agents_command_prints_recoverable_summary` 的过强断言。
- [ ] 3. 继续完成 5.x observability。
- [ ] 4. 进入 6.x integration guardrails 和最终验证。

下一步:
- 检查 5.1-5.4 的实现/测试覆盖是否已经满足 OpenSpec。
- 重点验证 agents snapshot / `ralph agents` / record-session evidence inspect 对 scheduled、continued、exhausted 的可观察性。

## [2026-05-28 16:20:54] [Session ID: omx-1779954714247-oab9zc] 行动: 补强 5.x agents snapshot JSON 可观察性测试

目的:
- 当前 `ralph agents` 紧凑表格只能显示 recoverable 状态和 attempt,完整 `failure_id` / `next_retry_at` 更适合由 agents snapshot JSON 承载。
- OpenSpec 5.2 要求 `ralph agents` 或 agents snapshot 能解释 retry-aware / exhausted state,包含 `failure_id`、affected instance、attempt、next retry timing。

将要做:
- 在 `integration_agents.rs` 中补充 `ralph agents --format json` 断言。
- 使用同一份 recoverable snapshot,断言 JSON 输出包含完整 `failure-writer-429`、`next_retry_at` 和 ledger path。
- 然后运行 focused integration_agents 与 record_session / supervisor focused tests。

## [2026-05-28 16:24:54] [Session ID: omx-1779954714247-oab9zc] 错误: record_session bin target 测试编译失败

现象:
- 命令: `cargo test -p ralph-cli --bin ralph record_session::tests::aggregate_collects_evidence_inspect -- --exact --nocapture`。
- 编译失败: `struct TopologySpawnedInstance has no field named recoverable_failures`。
- 位置: `crates/ralph-cli/src/record_session.rs:1440:17`。

初步判断:
- 这不是 runtime 行为根因,而是测试 fixture 在某个非 `AgentInstanceSnapshot` 结构体上误填了 `recoverable_failures` 字段。
- 需要读取该段 fixture,确认字段应该属于哪个结构体,再做最小修复。

额外上下文风险:
- `ERRORFIX.md` 已到 1000 行,后续记录本错误前必须先续档,避免继续超过阈值。

下一步:
- 轮转 `ERRORFIX.md` 到 `archive/default_history/` 并创建当前错误记录入口。
- 读取 `record_session.rs` 相关测试上下文,修正 fixture 字段位置。

## [2026-05-28 16:30:04] [Session ID: omx-1779954714247-oab9zc] 完成: agent-cli recoverable retry tasks 5.x

完成内容:
- `openspec/changes/agent-cli-recoverable-failure-retry/tasks.md` 已勾选 5.1-5.4。
- `AgentInstanceSnapshot.recoverable_failures` 保持 summary-only metadata,不复制 prompt 或 event stream。
- `ralph agents` 紧凑表格显示 recoverable 状态和 attempt。
- `ralph agents --format json` 集成测试证明完整 `failure_id`、`next_retry_at`、ledger path 留在 agents snapshot JSON 中。
- Evidence Inspect 渲染 scheduled / continued / exhausted 三类 recoverable failure,并指向 `.ralph/recoverable-failures.jsonl` ledger。

验证:
- `cargo test -p ralph-cli --test integration_agents test_agents_command_prints_recoverable_summary -- --exact --nocapture`: passed。
- `cargo test -p ralph-cli --test integration_agents --quiet`: 9 passed。
- `cargo test -p ralph-cli --bin ralph record_session::tests::aggregate_collects_evidence_inspect -- --exact --nocapture`: passed。
- `cargo test -p ralph-cli --bin ralph record_session::tests --quiet`: 6 passed。
- `cargo test -p ralph-core --lib recoverable --quiet`: 32 passed。
- `cargo test -p ralph-core --lib parallel::supervisor::routing_tests::agents_snapshot_includes_recoverable_failure_summaries -- --exact --nocapture`: passed。

当前待办状态:
- [x] 1. 处理 `notes.md` 超限续档和 continuous-learning 摘要。
- [x] 2. 修复 `test_agents_command_prints_recoverable_summary` 的过强断言。
- [x] 3. 完成 5.x observability。
- [ ] 4. 进入 6.x integration guardrails 和最终验证。

下一步:
- 进入 6.x,先确认是否已有 fake/custom backend 或 executor fixture 可复用,避免重复新增测试基础设施。

## [2026-05-28 16:31:50] [Session ID: omx-1779954714247-oab9zc] 行动: 执行 6.x integration guardrails focused 验证

观察:
- `parallel::instance` 测试中已有 `RecoverableThenSuccessExecutor`,首轮返回 `ERROR: exceeded retry limit, last status: 429 Too Many Requests`,第二轮成功。
- 同文件已有 `AlwaysRecoverableFailureExecutor`,用于验证 `max_attempts=1` 时耗尽并转 terminal evidence。
- 这些 executor fixture 比新增外部脚本更贴近 runtime-held `HatJob` context 和 scheduler 路径。

将要做:
- 重跑自动 delayed retry focused test。
- 重跑 manual `!continue` 对应的 instance retry path focused test。
- 重跑 exhausted terminal evidence focused test。
- 重跑 Supervisor continue parsing focused tests。
- 重跑 replay smoke tests和最终验证门禁后再勾选 6.x。

## [2026-05-28 16:40:35] [Session ID: omx-1779954714247-oab9zc] 行动: 清理 clippy warning 后继续最终门禁

现象:
- `cargo clippy -p ralph-core --quiet` exit 0,但输出 5 条 warning。
- `cargo clippy -p ralph-cli --quiet` exit 0,但输出上述 core warning 加 2 条 cli warning。

处理原则:
- warning 都是 clippy 提供的局部机械性建议,不涉及 recoverable retry 语义变更。
- 为避免最终验证带 warning 收尾,先小范围修复,再重跑 clippy。

将要做:
- 修 `useless_borrows_in_formatting`。
- 修 `unnecessary_sort_by`。
- 修 `duration_suboptimal_units`。
- 修 `some_filter`。
- 重跑 `cargo fmt`, `cargo clippy -p ralph-core --quiet`, `cargo clippy -p ralph-cli --quiet`。

## [2026-05-28 16:47:12] [Session ID: omx-1779954714247-oab9zc] 错误: integration_record_session AgentInstanceSnapshot fixture 漏新字段

现象:
- `cargo test -p ralph-cli --quiet` 编译失败。
- 错误: `missing field recoverable_failures in initializer of AgentInstanceSnapshot`。
- 位置: `crates/ralph-cli/tests/integration_record_session.rs:377:25`。

判断:
- 这是 5.x 给 `AgentInstanceSnapshot` 增加 `recoverable_failures` 后,外部 integration fixture 漏填默认空 Vec。
- 不涉及 runtime 语义,应补齐 fixture 字段并重跑 `cargo test -p ralph-cli --quiet`。

下一步:
- 读取相关 fixture,补 `recoverable_failures: Vec::new()`。
- 更新 ERRORFIX 并重跑 package tests。

## [2026-05-28 16:50:23] [Session ID: omx-1779954714247-oab9zc] 进展: 6.x focused guardrails 和 package tests 通过

已验证:
- 6.1/6.2: `RecoverableThenSuccessExecutor` 首轮 429 后自动 delayed retry 成功。
  - `cargo test -p ralph-core --lib parallel::instance::tests::recoverable_failure_schedules_retry_and_preserves_stdout_only_parsing -- --exact --nocapture`: passed。
- 6.3: manual continue 复用 scheduled retry path。
  - `cargo test -p ralph-core --lib parallel::instance::tests::manual_continue_appends_transition_and_uses_scheduled_retry_path -- --exact --nocapture`: passed。
- 6.4: exhausted recoverable failure 变成 terminal 并带 ledger evidence pointer。
  - `cargo test -p ralph-core --lib parallel::instance::tests::recoverable_failure_exhaustion_becomes_terminal_with_ledger_pointer -- --exact --nocapture`: passed。
- 6.5: `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate agent-cli-recoverable-failure-retry --type change --strict`: passed。
- 6.6: classifier/ledger/scheduler/parallel lifecycle/Supervisor continue focused tests 已通过。
- 6.7: `cargo test -p ralph-core smoke_runner --quiet`: 12 passed。
- 增量 package tests:
  - `cargo test -p ralph-core --quiet`: passed。
  - `cargo test -p ralph-cli --quiet`: passed。
- `cargo fmt`: passed。
- `cargo clippy -p ralph-core --quiet`: passed,无输出。
- `cargo clippy -p ralph-cli --quiet`: passed,无输出。

仍待:
- 6.8 最终 workspace `cargo test`。

## [2026-05-28 16:56:01] [Session ID: omx-1779954714247-oab9zc] 完成: agent-cli-recoverable-failure-retry implementation all_done

最终状态:
- `openspec/changes/agent-cli-recoverable-failure-retry/tasks.md`: 34/34 complete。
- OpenSpec apply state: `all_done`。

最终验证:
- `cargo fmt`: passed。
- `cargo clippy -p ralph-core --quiet`: passed,无输出。
- `cargo clippy -p ralph-cli --quiet`: passed,无输出。
- `cargo test -p ralph-core --quiet`: passed。
- `cargo test -p ralph-cli --quiet`: passed。
- `cargo test -p ralph-core smoke_runner --quiet`: 12 passed。
- `cargo test --quiet`: workspace passed。
- `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate agent-cli-recoverable-failure-retry --type change --strict`: passed。
- `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate --all --strict`: 28 passed,0 failed。
- `git diff --check`: passed。

当前待办状态:
- [x] 1. 处理 `notes.md` 超限续档和 continuous-learning 摘要。
- [x] 2. 修复 `test_agents_command_prints_recoverable_summary` 的过强断言。
- [x] 3. 完成 5.x observability。
- [x] 4. 完成 6.x integration guardrails 和最终验证。

后续建议:
- 下一步可以执行 OpenSpec archive: `openspec archive agent-cli-recoverable-failure-retry --yes`。
- 归档前建议先由用户确认当前大范围工作区改动边界,因为仓库里存在大量早前未提交/未归档改动。

## [2026-05-28 17:47:21] [Session ID: omx-1779954714247-oab9zc] 行动: 验证 recoverable failure retry 文档与 skill 同步状态

目标:
- 验证 `EXPERIENCE.md` 是否已经删除或更正旧的 `no-delta` 阻断说法。
- 验证 `.codex/skills/self-learning.ralph-agent-cli-recoverable-failure-retry/SKILL.md` 是否存在,且内容能覆盖已落地的新功能。
- 验证 `AGENTS.md` 的 Project Knowledge Index 是否已经索引该项目级 skill。

边界:
- 只处理用户明确指定的文档/skill 同步。
- 不触碰当前工作区里的大量无关代码改动。
- 如果发现三项已完成,只做验证和记录,不做重复改写。

状态:
- 正在进行三处文件的只读验证,必要时做局部补丁。

## [2026-05-28 17:51:07] [Session ID: omx-1779954714247-oab9zc] 完成: recoverable failure retry 文档与 skill 同步

已完成:
- [x] `EXPERIENCE.md` 已更正旧 `no-delta change` 阻断说法,改为 2026-05-28 已归档后的当前口径。
- [x] 已新增项目级 skill: `.codex/skills/self-learning.ralph-agent-cli-recoverable-failure-retry/SKILL.md`。
- [x] 已在 `AGENTS.md` Project Knowledge Index 增加该 skill 索引。

验证:
- `python3` 检查 skill frontmatter 和关键短语: passed。
- `git diff --check -- EXPERIENCE.md AGENTS.md .codex/skills/self-learning.ralph-agent-cli-recoverable-failure-retry/SKILL.md task_plan.md`: passed。
- 使用单引号 `rg` pattern 复核索引和 no-delta 口径: passed。

错误与处理:
- 曾用双引号 `rg` pattern 搜索带反引号文本,触发 shell command substitution,出现 `zsh:1: command not found: agent-cli-recoverable-failure-retry`。
- 已按 `self-learning.shell-heredoc-backtick-command-substitution` 改用单引号 pattern 重跑,验证通过。

当前状态:
- 本轮用户指定三项已完成。
- 未触碰当前工作区里的大量无关代码改动。

## [2026-05-29 00:00:54] [Session ID: omx-1779954714247-oab9zc] 行动: 收口本地提交

目标:
- 只提交 recoverable failure retry 文档与 skill 同步相关改动。
- 避免把当前工作区中大量无关 Rust / docs / archive 改动混入本次提交。

提交候选:
- `AGENTS.md`
- `EXPERIENCE.md`
- `.codex/skills/self-learning.ralph-agent-cli-recoverable-failure-retry/SKILL.md`

谨慎项:
- `task_plan.md` / `WORKLOG.md` / `ERRORFIX.md` 含本轮记录,但当前 diff 还包含大量历史上下文变动,不应未经审查整文件 stage。

验证计划:
- focused diff review。
- `git diff --check` 针对候选文件。
- skill frontmatter / 关键短语检查。
- 本地 commit,不 push。

## [2026-05-29 00:03:07] [Session ID: omx-1779954714247-oab9zc] 完成: scoped 本地提交

结果:
- 已创建本地 commit: 2bf2aba5 `docs: sync recoverable failure retry guidance`。
- 提交范围只包含 recoverable failure retry guidance 相关文件。
- 未 push。

提交文件:
- `.codex/skills/self-learning.ralph-agent-cli-recoverable-failure-retry/SKILL.md`
- `AGENTS.md`
- `EXPERIENCE.md`

验证:
- skill frontmatter / 关键短语检查: passed。
- `git diff --cached --check`: passed。
- `git commit`: passed。

边界:
- 当前 worktree 仍存在大量既有未提交改动,没有被本次 commit 混入。

## [2026-05-29 00:09:00] [Session ID: omx-1779954714247-oab9zc] 进展: WORKLOG 超限续档并修复 manifest quoting 错误

完成:
- 旧 `WORKLOG.md` 已移动到 `archive/default_history/WORKLOG_2026-05-29_0008_pre_review.md`。
- 新 `WORKLOG.md` 已创建,作为当前 review 主线的轻量入口。
- 已在 `notes.md` 写入最小 continuous-learning 摘要。
- 已创建并修复 `archive/manifests/ARCHIVE_MANIFEST__default_worklog_rollover_2026-05-29_0008.md`。

错误处理:
- manifest 首次写入误用未加引号 heredoc,触发反引号 command substitution。
- 已用单引号 heredoc 重写 manifest,并追加到 `ERRORFIX.md`。

下一步:
- 进入未提交实现改动 review,先生成 changed-files 分类和 review scope。

## [2026-05-29 00:16:00] [Session ID: omx-1779954714247-oab9zc] 完成: 未提交实现 focused review

完成:
- 已处理 `WORKLOG.md` 超限续档。
- 已对 recoverable retry 主链路完成 focused review。
- 已运行 focused tests 和 `git diff --check`。

结论:
- 未发现 focused path 的直接 correctness blocker。
- 发现 3 个非阻断 hardening/watch 点: ledger 并发 append、scheduled retry acquire 失败恢复、completed dynamic tombstone recoverable 可见性。
- 当前 worktree 变更必须拆分提交,不能直接整仓 stage。

下一步建议:
- 先按 recoverable retry 实现本体做 scoped commit。
- 再分别 review / commit topology runtime evidence、TUI、E2E、docs 等其它支线。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 行动: 复核 4.x Manual continue control path

目标:
- 用户要求继续进入 `4.x Manual continue control path`。
- 当前静态证据显示 `agent-cli-recoverable-failure-retry` 已归档,且 archived `tasks.md` 的 4.x 已全部完成。
- 本轮不重做已经完成的实现,而是用 focused tests 验证当前代码仍满足 4.x 契约。

计划:
- [x] 读取 OpenSpec archive 和上下文文件。
- [ ] 运行 manual continue parser / routing / instance focused tests。
- [ ] 若测试通过,记录证据并建议进入真正未完成或未提交的下一项。
- [ ] 若测试失败,按现象 -> 假设 -> 验证计划 -> 结论修复。

状态:
- 正在运行 4.x focused tests。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 错误记录: manual continue routing 测试名过期

现象:
- 多个 `cargo test --exact` 命令返回 `running 0 tests`。
- 这说明命令没有匹配到当前测试函数,不能作为通过证据。

处理:
- 立即查询当前真实测试名。
- 重跑真实存在且能执行的 4.x focused tests。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 完成: 4.x Manual continue control path 复核

已完成:
- [x] 读取 OpenSpec archive 和上下文文件。
- [x] 确认 `agent-cli-recoverable-failure-retry` 已归档,4.x tasks 全部完成。
- [x] 纠正旧测试名导致的 `running 0 tests` 无效验证。
- [x] 运行当前真实 4.x focused tests,全部通过。

验证命令:
- `cargo test -p ralph-core --lib parallel::supervisor::routing_tests::explicit_recoverable_continue_accepts_only_waiting_failures -- --exact --nocapture`
- `cargo test -p ralph-core --lib parallel::supervisor::routing_tests::bare_recoverable_continue_uses_selected_instance_to_disambiguate -- --exact --nocapture`
- `cargo test -p ralph-core --lib parallel::supervisor::routing_tests::bare_recoverable_continue_falls_back_to_global_unique_when_selected_has_no_failure -- --exact --nocapture`
- `cargo test -p ralph-core --lib parallel::instance::tests::manual_continue_appends_transition_and_uses_scheduled_retry_path -- --exact --nocapture`

结论:
- 4.x Manual continue control path 当前已完成,不应作为未完成实现继续推进。
- 下一步更适合收口当前未提交 recoverable retry 实现本体,或处理 focused review 中记录的 hardening 点。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 行动: recoverable retry 实现本体提交前验证

目标:
- hook 要求继续任务并收集新的验证证据。
- 当前工作区有 167 个改动项,不能整仓提交。
- 本轮只围绕 recoverable retry 主链路做 scoped gates 和提交边界判断。

候选范围:
- core recoverable module / config / snapshot / instance / supervisor / routing tests。
- CLI agents display / record summary / integration fixtures。
- OpenSpec archived change 与已同步稳定 spec。

验证计划:
- [ ] 运行 recoverable 模块测试。
- [ ] 运行 instance retry / manual continue / exhaustion tests。
- [ ] 运行 supervisor observability / continue routing tests。
- [ ] 运行 CLI agents 和 record-session focused tests。
- [ ] 运行 OpenSpec strict 与 diff check。

边界:
- 不自动 stage / commit。
- 不处理 topology / TUI / E2E / docs 大批量支线。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 完成: recoverable retry 实现本体提交前验证

已完成:
- [x] 运行 recoverable 模块测试。
- [x] 运行 instance retry / manual continue / exhaustion tests。
- [x] 运行 supervisor observability / continue routing tests。
- [x] 运行 CLI agents 和 record-session focused tests。
- [x] 运行 OpenSpec strict 与 diff check。

关键结论:
- scoped gates 全部通过。
- 当前工作区有 167 个改动项,不能整仓提交。
- 下一步应先做 recoverable retry scoped file list 的逐文件 diff review,再只 stage 该范围。

新的验证证据:
- recoverable 模块: 32 passed。
- instance focused tests: 3 个单测 passed。
- supervisor focused tests: 4 个单测 passed。
- CLI focused tests: 2 个单测 passed。
- OpenSpec strict: 28 passed,0 failed。
- `git diff --check`: passed。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 行动: recoverable retry scoped diff 逐文件审查

目标:
- 基于上一轮通过的 focused gates,继续做提交边界审查。
- 逐文件判断候选改动是否属于 recoverable retry 主线,还是夹带其它支线。

本轮只读动作:
- 用 CodeGraph 刷新 recoverable retry 相关入口。
- 生成候选文件 diff 摘要。
- 形成 include / caution / exclude 清单。

状态:
- 正在生成逐文件 diff 摘要。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 完成: recoverable retry scoped diff 逐文件审查

已完成:
- [x] 刷新候选 diff stat。
- [x] 用 CodeGraph 确认 recoverable retry 入口。
- [x] 扫描候选 diff 中的 topology / capability / role_contract / completed_dynamic 等混线关键词。
- [x] 生成 include / patch-stage / 不建议整文件 stage 清单。
- [x] 跑 fresh lightweight verification。

整文件 stage 高置信候选:
- `crates/ralph-core/src/recoverable_failure.rs`
- `crates/ralph-core/src/parallel/instance.rs`
- `crates/ralph-core/src/parallel/supervisor/routing.rs`
- `crates/ralph-cli/src/display.rs` 需要提交前看完整 diff,但本轮未发现明显非 recoverable 支线。
- `openspec/specs/agent-cli-recoverable-failure-retry/spec.md`
- `openspec/specs/supervisor-human-chat-gate/spec.md`
- `openspec/changes/archive/2026-05-28-agent-cli-recoverable-failure-retry/`

必须 patch-stage 或暂缓的混线文件:
- `crates/ralph-core/src/config.rs`
- `crates/ralph-core/src/lib.rs`
- `crates/ralph-core/src/agents_snapshot.rs`
- `crates/ralph-core/src/parallel/supervisor.rs`
- `crates/ralph-core/src/parallel/supervisor/routing_tests.rs`
- `crates/ralph-cli/src/record_session.rs`
- `crates/ralph-cli/tests/integration_agents.rs`
- `crates/ralph-cli/tests/integration_record_session.rs`
- `openspec/specs/parallel-hat-instances/spec.md`

fresh verification:
- `cargo test -p ralph-core --lib recoverable --quiet`: 32 passed。
- `cargo test -p ralph-core --lib parallel::instance::tests::manual_continue_appends_transition_and_uses_scheduled_retry_path -- --exact --nocapture`: 1 passed。
- `cargo test -p ralph-cli --test integration_agents test_agents_command_prints_recoverable_summary -- --exact --nocapture`: 1 passed。
- `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate --all --strict`: 28 passed,0 failed。
- `git diff --check`: passed。

下一步:
- 执行 patch-stage 计划,先只 stage recoverable hunks。
- stage 后运行 `git diff --cached --check` 和 focused tests。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 行动: stage recoverable-only 高置信文件

目标:
- 继续执行 patch-stage 计划。
- 先只 stage 高置信 recoverable-only 文件,不碰混线文件。

调整:
- `crates/ralph-cli/src/display.rs` 完整 diff 含 role_contract / completed_dynamic_instances,从整文件 stage 候选降级为 patch-stage。

本次 stage 范围:
- `crates/ralph-core/src/recoverable_failure.rs`
- `crates/ralph-core/src/parallel/instance.rs`
- `crates/ralph-core/src/parallel/supervisor/routing.rs`
- `openspec/specs/agent-cli-recoverable-failure-retry/spec.md`
- `openspec/specs/supervisor-human-chat-gate/spec.md`
- `openspec/changes/archive/2026-05-28-agent-cli-recoverable-failure-retry/`

验证计划:
- `git diff --cached --stat`
- `git diff --cached --check`
- focused recoverable tests 和 OpenSpec strict。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 错误记录: staged diff check 发现 whitespace 问题

现象:
- `git diff --cached --check` 失败。
- `openspec/changes/archive/2026-05-28-agent-cli-recoverable-failure-retry/design.md` 有 trailing whitespace。
- `openspec/specs/agent-cli-recoverable-failure-retry/spec.md` 有 new blank line at EOF。

处理:
- 修复已 staged 文件格式。
- 重新 stage 相关文件。
- 重跑 `git diff --cached --check`。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 进展: recoverable retry 第一批 staged 文件通过 cached gate

已完成:
- [x] stage 第一批 recoverable-only 高置信文件。
- [x] 发现 `display.rs` 混线,降级为 patch-stage。
- [x] 修复 OpenSpec archive / stable spec whitespace 问题。
- [x] 确认上下文文件未留在 index。
- [x] `git diff --cached --check` passed。
- [x] fresh focused gates passed。

fresh verification:
- `cargo test -p ralph-core --lib recoverable --quiet`: 32 passed。
- `cargo test -p ralph-core --lib parallel::instance::tests::manual_continue_appends_transition_and_uses_scheduled_retry_path -- --exact --nocapture`: 1 passed。
- `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate --all --strict`: 28 passed,0 failed。
- `git diff --cached --check`: passed。

当前状态:
- index 已有第一批 recoverable-only 文件。
- 还需要继续对混线文件做 `git add -p` 或等价精确 patch-stage。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 行动: 第二批 patch-stage 低风险 recoverable hunks

目标:
- 继续 patch-stage 计划。
- 先处理低风险且被第一批 staged 文件依赖的 recoverable hunks。

本轮范围:
- `crates/ralph-core/src/config.rs`: recoverable retry config / validation / ledger path resolver。
- `crates/ralph-core/src/lib.rs`: recoverable module/export 相关行。
- `openspec/specs/parallel-hat-instances/spec.md`: 只 stage recoverable CLI failure requirements,不 stage topology.spawn_group 后半段。

边界:
- 不 stage role reasoning / runtime capabilities / topology_spawn / prompt_surface 等其它支线。
- 不 stage `supervisor.rs` / `routing_tests.rs` / `record_session.rs` 等大块混线文件。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 错误记录: 第二批 synthetic stage marker 过期

现象:
- 第二批 synthetic stage 脚本失败: `ValueError: substring not found`。
- 失败原因是按旧测试函数名提取 `config.rs` recoverable tests,其中 `test_agent_cli_recoverable_failures_defaults` 当前不存在。

验证:
- `git diff --cached --name-only` 确认 index 仍只有第一批 staged 文件。
- `git diff --cached --check` 仍通过。

处理:
- 改用当前真实存在的 recoverable config test markers。
- 本轮只处理 `config.rs` / `lib.rs` / `parallel-hat-instances` 三个低风险文件。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 进展: recoverable retry 第二批低风险 hunks staged

已完成:
- [x] `config.rs` recoverable policy / validation / ledger path resolver staged。
- [x] `lib.rs` recoverable module/export staged。
- [x] `parallel-hat-instances` recoverable CLI failure requirements staged。
- [x] cached suspicious scan 未发现 topology/capability/role_contract 混入第二批 staged hunks。
- [x] fresh focused gates passed。

fresh verification:
- `cargo test -p ralph-core --lib recoverable --quiet`: 32 passed。
- `cargo test -p ralph-core --lib parallel::instance::tests::manual_continue_appends_transition_and_uses_scheduled_retry_path -- --exact --nocapture`: 1 passed。
- `cargo test -p ralph-core --lib config::tests::test_parse_agent_cli_recoverable_failures_policy_override -- --exact --nocapture`: 1 passed。
- `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate --all --strict`: 28 passed,0 failed。
- `git diff --cached --check`: passed。

下一步:
- 继续 patch-stage `agents_snapshot.rs` 与 `supervisor.rs` recoverable hunks。
- 这两者是 `AgentRecoverableFailureSummary` 观察面和 supervisor lifecycle 的必要组成。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 行动: 第三批 patch-stage agents snapshot recoverable 观察面

目标:
- 继续补齐 recoverable retry commit 的 staged index。
- 本轮优先 stage `agents_snapshot.rs` 中 recoverable summary 字段与 struct。
- 同时补齐所有必要的 `AgentInstanceSnapshot` initializer,否则 staged commit 会无法编译。

验证重点:
- 不只跑 working tree tests。
- 还要用临时 worktree 应用 `git diff --cached --binary`,验证 staged patch 本身。

边界:
- 不 stage `completed_dynamic_instances` / `child_runs` / `role_contract_summary` / `IdentitySource` 等非 recoverable 字段。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 行动: 第三批 staged-only patch 构造 agents_snapshot/supervisor recoverable 核心

目标:
- 只把 recoverable summary 和 supervisor lifecycle 必需 hunks 写入 index。
- 不把 completed_dynamic_instances / child_runs / role_contract / topology_runtime 等支线写入 index。

计划:
- 构造 `agents_snapshot.rs` staged 内容: 只新增 `recoverable_failures` 字段和 `AgentRecoverableFailureSummary`。
- 构造 `supervisor.rs` staged 内容: recoverable ledger/map、transition handling、completion gate、manual continue resolution、snapshot summaries。
- 不改工作区文件,只更新 index。
- 更新后先跑 `git diff --cached --check` 和 cached suspicious scan。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 行动: staged-only 临时 worktree 验证

目标:
- 验证当前 staged index 本身能否编译和通过 focused gates。
- 避免 working tree 未 staged 混线改动掩盖 staged commit 缺失依赖。

方法:
- 从 HEAD 创建临时 worktree。
- 导出当前 `git diff --cached --binary`。
- 在临时 worktree 应用 patch。
- 在临时 worktree 跑 focused tests 和 OpenSpec strict。

边界:
- 不提交。
- 不修改主工作区未 staged 文件。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 错误记录: staged-only 临时 worktree 验证失败

现象:
- 从 HEAD 创建临时 worktree,应用 `git diff --cached --binary` 后运行 focused tests 失败。
- 这说明当前 staged index 不是可独立编译的 recoverable retry commit。

关键错误:
- `routing.rs` 当前 staged 了 topology / role_contract 相关改动,但 `lib.rs` / `supervisor.rs` 没有 stage 对应依赖:
  - `EffectiveRoleContract` unresolved。
  - `runtime_topic_bypasses_strict_target` unresolved。
  - `handle_topology_spawn_group_event` missing。
  - `effective_role_contracts` field missing。
- `supervisor.rs` staged-only 版本还缺 recoverable 必要兼容:
  - `JobCompleted` pattern 没带 `job_id` 或 `..`。
  - `shutdown_instances` match 没覆盖 `RecoverableFailureTransition`。
- `config.rs` staged-only 版本里 `resolve_recoverable_failures_ledger_path` 未成功进入 index,导致 supervisor 调用缺方法。

当前假设:
- 主假设: 第一批把 `routing.rs` 整文件 stage 是错误的,它混入了 topology / role_contract 支线。应把 `routing.rs` 降级为 recoverable-only patch-stage 或先从 index 退回。
- 备选解释: recoverable retry 实现本身依赖 topology/role_contract 新基础设施,必须和那条支线一起提交。但从代码结构看,`recoverable.continue` 不应依赖 topology spawn,所以更可能是 stage 边界污染。

修复计划:
- 重新构造 `routing.rs` staged-only 内容,只保留 `recoverable.continue` topic routing,移除 topology/role_contract hunks。
- 修复 `supervisor.rs` staged-only 内容:
  - 确保 `resolve_recoverable_failures_ledger_path` 已在 staged `config.rs`。
  - `JobCompleted` pattern 使用 `job_id` 或 `..`。
  - 所有 match 覆盖 `RecoverableFailureTransition`。
- 重跑临时 worktree staged-only 验证。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 错误记录: staged-only 验证第二轮失败

现象:
- 临时 worktree 应用 staged patch 后编译失败。

关键错误:
- `escalate_delivery_failure` 在 routing 子模块中是 private,但 staged `supervisor.rs` 的 recoverable continue handler 需要调用它。
- routing.rs 中动态 spawn 调用 `HatInstanceHandle::spawn` 少传 recoverable retry policy / ledger 参数。
- `JobCompleted` pattern 绑定了未使用的 `job_id`,触发 warning。

结论:
- 当前 staged index 还缺两个 recoverable 必要兼容 hunk:
  - `routing.rs`: `escalate_delivery_failure` 需要 `pub(super)`,动态 spawn 需要传 retry policy/ledger。
  - `supervisor.rs`: `job_id` 改成 `job_id: _` 或使用。

修复:
- 精确修改 staged index,不 stage topology/role_contract 支线。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 错误记录: staged-only 验证输出含无效 config test

现象:
- 临时 worktree staged-only 验证命令最终 exit 0。
- 但输出显示新增 recoverable config tests 是 dead code,说明缺少 `#[test]` 属性。
- `cargo test ... config::tests::test_parse_agent_cli_recoverable_failures_policy_override -- --exact` 显示 `running 0 tests`。

结论:
- 这不是有效的 config focused test 证据。
- 必须修 staged `config.rs`,给新增 recoverable config test 函数补回 `#[test]`。

下一步:
- 修 index-only `config.rs`。
- 重新做临时 worktree staged-only 验证。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 完成: 第三批 staged-only agents snapshot / supervisor recoverable 验证

已完成:
- [x] `agents_snapshot.rs` recoverable-only staged。
- [x] `supervisor.rs` recoverable lifecycle staged。
- [x] `routing.rs` 从整文件 stage 污染收窄为 recoverable-only staged。
- [x] 修复 config resolver / config test attributes / supervisor match pattern / dynamic spawn args。
- [x] staged-only 临时 worktree focused verification passed。

staged-only verification:
- `cargo test -p ralph-core --lib recoverable --quiet`: 27 passed。
- `cargo test -p ralph-core --lib parallel::instance::tests::manual_continue_appends_transition_and_uses_scheduled_retry_path -- --exact --nocapture`: 1 passed。
- `cargo test -p ralph-core --lib config::tests::test_parse_agent_cli_recoverable_failures_policy_override -- --exact --nocapture`: 1 passed。
- `cargo test -p ralph-core --lib config::tests::test_validate_recoverable_failures_policy_rejects_zero_attempts -- --exact --nocapture`: 1 passed。
- `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate --all --strict`: 27 passed,0 failed。
- `git diff --cached --check`: passed。

下一步:
- 继续 patch-stage CLI 观察面:
  - `crates/ralph-cli/src/display.rs`
  - `crates/ralph-cli/src/record_session.rs`
  - `crates/ralph-cli/tests/integration_agents.rs`
  - `crates/ralph-cli/tests/integration_record_session.rs`
- 然后补 `routing_tests.rs` recoverable tests。

## [2026-05-29 13:37:41] [Session ID: native-codex-20260529] 状态更新: 继续 patch-stage CLI recoverable 观察面

目标:
- 只将 recoverable retry 相关 CLI 观察面和测试补入 staged index。
- 不混入 topology / role_contract / completed dynamic / child_runs 支线。

即将执行:
- 刷新 、、CLI integration tests、 的 diff。
- 用 HEAD 基线构造 staged-only patch,优先保证 index 独立可编译。

当前状态:
- 第三批 core recoverable staged-only 验证已通过。
- 现在进入 CLI observability / tests patch-stage。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 状态修正: heredoc quoting 错误已记录

现象:
- 上一条状态追加时,反引号文件名被 shell 误执行。
- 错误已记录到 `ERRORFIX.md`。

继续计划:
- 后续 Markdown 追加一律使用 `cat <<'EOF'`。
- 当前 staged index 不包含上下文文件,继续只处理 recoverable retry CLI 观察面和测试。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 错误记录: zsh path 变量污染

现象:
- CLI staged blob 更新脚本在 for 循环里使用 `path` 变量后,后续 `git` 命令不可见。

处理:
- 已记录到 `ERRORFIX.md`。
- 下一步改用 `target_file` 变量名,并先用 `command -v git` 验证 PATH 恢复。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 阶段进展: CLI 和 routing_tests recoverable-only staged

已完成:
- 已将 `display.rs` 的 Recoverable 列以 index-only blob 方式 stage。
- 已将 `record_cli.rs` 的 `--agents-file` 和 Evidence Inspect 调用以 recoverable 观察面方式 stage。
- 已将 `record_session.rs` 的 recoverable-only Evidence Inspect renderer 和 unit test stage。
- 已将 `integration_agents.rs` / `integration_record_session.rs` 的 recoverable tests stage。
- 已将 `routing_tests.rs` 的 recoverable gate / continue / snapshot tests stage。

下一步:
- 从 HEAD 创建临时 worktree。
- 应用 `git diff --cached --binary`。
- 在临时 worktree 内跑 focused tests / OpenSpec strict / diff check。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 验证失败: cargo fmt check

现象:
- staged-only 临时 worktree `cargo fmt --check` 失败。

处理:
- 用临时 worktree 运行 `cargo fmt`。
- 将格式化后的 staged Rust 文件回写 index。
- 重新跑 staged-only 验证。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 验证阻断: 全量 cargo test 的 unrelated fixture 缺失

现象:
- staged-only `cargo test --quiet` 失败在 `integration_examples`。

证据:
- 主工作区存在多个 `examples/*/PROMPT.md`,但 `git ls-files` 不跟踪它们。
- 临时 worktree 从 HEAD 检出后缺这些未跟踪文件。

决定:
- 不把未跟踪 example prompt fixtures 混入 recoverable retry commit。
- 本轮以 staged-only focused tests、smoke runner、OpenSpec strict、fmt/check 作为有效 gate。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 修复: watch 测试时序护栏

现象:
- `integration_record_session` 全量测试中,既有 watch 测试固定等待 200ms 后 kill 子进程,在 staged-only worktree 中稳定失败。

处理:
- 改为等待 stdout 文件包含 `_meta.session_start` 或 5 秒超时。
- 这是测试稳定性修复,用于解除验证阻断。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 完成: recoverable retry scoped staged patch

完成清单:
- [x] CLI recoverable 观察面 staged。
- [x] record summary recoverable Evidence Inspect staged。
- [x] routing_tests recoverable-only 回归测试 staged。
- [x] staged-only 临时 worktree focused gates passed。
- [x] smoke runner passed。
- [x] OpenSpec strict passed。
- [x] full cargo test 在 overlay 本地未跟踪 example prompt fixtures 后 passed。

保留说明:
- 纯 staged-only worktree 的 full cargo test 会因为未跟踪 `examples/parallel-*/PROMPT.md` fixture 缺失而失败。
- 该问题已记录到 `LATER_PLANS.md`,不混入当前 recoverable retry scoped patch。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] hook 验证完成: fresh evidence 已补充

完成:
- [x] 刷新 staged index 和上下文边界。
- [x] 新建 staged-only worktree `/tmp/ralph-staged-fresh.L8b8iw/wt`。
- [x] 运行 fresh recoverable / CLI / smoke / OpenSpec gates。

Stop condition:
- 当前任务没有新的未完成实现步骤。
- 没有自动 commit,等待人类明确提交指令。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] hook 再次继续: 轻量 fresh verification

来源:
- OMX hook 再次提示 ultrawork 仍处于 planning phase,要求继续并收集 fresh verification evidence。

执行计划:
- 不新增代码,不 commit。
- 重新检查 staged index 边界。
- 新建 staged-only worktree,运行一组轻量但覆盖核心契约的验证:
  - diff check。
  - fmt check。
  - recoverable core exact/模块测试。
  - CLI recoverable observation exact tests。
  - OpenSpec strict。

停止条件:
- 这些新鲜验证全部通过,且没有发现 staged index 被污染。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 第二次 hook fresh verification 完成

完成:
- [x] 读取 recoverable retry 与 verification-before-completion skill。
- [x] 新建 staged-only worktree `/tmp/ralph-hook-fresh.Pa6ZTr/wt`。
- [x] 运行 diff/fmt/recoverable/manual continue/CLI/OpenSpec gates。

Stop condition:
- 新验证通过。
- 没有新增代码需求。
- 没有自动 commit,等待人类明确提交指令。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 第三次 hook 继续: fresh staged-only verification

来源:
- OMX hook 再次提示 ultrawork 仍 active,要求继续并收集 fresh verification evidence。

执行计划:
- 不新增功能。
- 不自动 commit。
- 重新确认 staged index 未包含上下文文件。
- 新建 staged-only worktree,运行一组关键门禁:
  - `git diff --cached --check`
  - `cargo fmt --check`
  - recoverable core module tests
  - manual continue exact test
  - CLI record summary recoverable evidence exact test
  - OpenSpec strict

停止条件:
- 本轮 fresh gates 全部通过,且 staged index 边界仍干净。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 第三次 hook fresh verification 完成

完成:
- [x] staged index 预检通过。
- [x] 新建 staged-only worktree `/tmp/ralph-hook3-fresh.PeT4ld/wt`。
- [x] fresh diff/fmt/recoverable/manual continue/record summary/OpenSpec gates 通过。

Stop condition:
- 当前 staged patch 已有第三轮 fresh evidence。
- 没有新的实现缺口。
- 没有自动 commit,等待人类明确提交指令。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 第四次 hook 继续: fresh staged-only verification

来源:
- OMX hook 再次提示 ultrawork still active,要求继续任务并收集 fresh verification evidence。

执行计划:
- 不新增代码。
- 不自动 commit。
- 重新检查 staged index 边界。
- 新建 staged-only worktree,运行关键门禁:
  - `git diff --cached --check`
  - `cargo fmt --check`
  - `cargo test -p ralph-core --lib recoverable --quiet`
  - `cargo test -p ralph-cli --test integration_agents test_agents_command_prints_recoverable_summary -- --exact --nocapture`
  - `cargo test -p ralph-cli --test integration_record_session record_summary_agents_file_shows_recoverable_failure_evidence -- --exact --nocapture`
  - `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate --all --strict`

停止条件:
- 本轮 fresh gates 全部通过。
- 上下文文件仍未 staged。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 第四次 hook fresh verification 完成

完成:
- [x] staged index 预检通过,共 23 个 staged 文件。
- [x] 新建 staged-only worktree `/tmp/ralph-hook4-fresh.6mas2Y/wt`。
- [x] fresh diff/fmt/recoverable/agents/record-summary/OpenSpec gates 通过。

Stop condition:
- 当前 cached patch 已有第四轮 fresh staged-only evidence。
- 没有新的实现缺口。
- 没有自动 commit,等待人类明确提交指令。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 第五次 hook 继续: fresh staged-only verification

来源:
- OMX hook 再次提示 ultrawork still active,要求继续任务并收集 fresh verification evidence。

执行计划:
- 不新增代码。
- 不自动 commit。
- 重新确认 staged index 和上下文文件边界。
- 新建 staged-only worktree,运行一组关键门禁:
  - `git diff --cached --check`
  - `cargo fmt --check`
  - `cargo test -p ralph-core --lib recoverable --quiet`
  - `cargo test -p ralph-core --lib parallel::supervisor::routing_tests::pending_recoverable_failures_block_completion_gate -- --exact --nocapture`
  - `cargo test -p ralph-cli --bin ralph record_session::tests::evidence_inspect_renders_recoverable_failures_from_agents_snapshot -- --exact --nocapture`
  - `cargo test -p ralph-core smoke_runner --quiet`
  - `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate --all --strict`

停止条件:
- 本轮 fresh gates 全部通过,且上下文文件仍未 staged。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 第五次 hook fresh verification 完成

完成:
- [x] staged index 预检通过,共 23 个 staged 文件。
- [x] 新建 staged-only worktree `/tmp/ralph-hook5-fresh.aNGSVL/wt`。
- [x] fresh diff/fmt/recoverable/supervisor/record-session/smoke/OpenSpec gates 通过。

Stop condition:
- 当前 cached patch 已有第五轮 fresh staged-only evidence。
- 没有新的实现缺口。
- 没有自动 commit,等待人类明确提交指令。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 第六次 hook 继续: fresh staged-only verification

来源:
- OMX hook 再次提示 ultrawork still active,要求继续任务并收集 fresh verification evidence。

执行计划:
- 不新增代码。
- 不自动 commit。
- 重新确认 staged index 和上下文文件边界。
- 新建 staged-only worktree,运行关键门禁:
  - `git diff --cached --check`
  - `cargo fmt --check`
  - `cargo test -p ralph-core --lib parallel::supervisor::routing_tests::explicit_recoverable_continue_accepts_only_waiting_failures -- --exact --nocapture`
  - `cargo test -p ralph-core --lib parallel::supervisor::routing_tests::bare_recoverable_continue_uses_selected_instance_to_disambiguate -- --exact --nocapture`
  - `cargo test -p ralph-cli --test integration_record_session --quiet`
  - `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate --all --strict`

停止条件:
- 本轮 fresh gates 全部通过,且上下文文件仍未 staged。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 第六次 hook fresh verification 完成

完成:
- [x] staged index 预检通过,共 23 个 staged 文件。
- [x] 新建 staged-only worktree `/tmp/ralph-hook6-fresh.pYr7tH/wt`。
- [x] fresh diff/fmt/recoverable continue disambiguation/record-session/OpenSpec gates 通过。

额外观察:
- `task_plan.md` 已超过 1000 行,已记录到 `LATER_PLANS.md`。
- 不在当前 recoverable retry scoped patch 中执行上下文续档,避免污染提交边界。

Stop condition:
- 当前 cached patch 已有第六轮 fresh staged-only evidence。
- 没有新的实现缺口。
- 没有自动 commit,等待人类明确提交指令。

## [2026-05-29 17:55:02] [Session ID: omx-1779004640353-blcixq] scoped commit 执行计划: recoverable retry

来源:
- 用户明确允许执行 scoped commit。
- 当前目标只提交 recoverable retry 主线 staged index,不做 push。

执行计划:
- [ ] 检查 staged 文件列表和 diff check。
- [ ] 检查 staged index 不包含六文件上下文或 .omx/state。
- [ ] 检查 submodule 状态。
- [ ] 执行本地 scoped commit。
- [ ] commit 后用 git log -1 --stat 和 git status --short 确认边界。

约束:
- 不整仓 stage。
- 不提交 task_plan.md / notes.md / WORKLOG.md / ERRORFIX.md / LATER_PLANS.md / EPIPHANY_LOG.md。
- 不提交 .omx/state。
- 不 push。

状态:
** 目前在提交前边界检查阶段 **- 准备确认 staged index 和 submodule 状态。

## [2026-05-29 17:56:25] [Session ID: omx-1779004640353-blcixq] scoped commit 完成: recoverable retry

完成:
- [x] staged 文件列表检查完成,未包含六文件上下文或 .omx/state。
- [x] git diff --cached --check 通过。
- [x] submodule status 无输出,没有 submodule 指针变化。
- [x] 已创建本地 commit: 8bf37643 feat: add recoverable agent cli retry lifecycle。
- [x] commit 后 git diff --cached --name-status 为空,index 已清空。

状态:
** scoped commit 已完成 **- 剩余工作区改动仍保持未暂存,没有 push。

## [2026-05-29 17:57:58] [Session ID: omx-1779004640353-blcixq] continuous-learning 启动: scoped commit 后上下文续档

触发条件:
- 用户显式执行 continuous-learning。
- 默认 task_plan.md 已超过 1000 行。
- recoverable retry scoped commit 已完成,当前 index 为空,是安全学习切点。

执行计划:
- [ ] 列出默认六文件、支线六文件和根目录历史版本候选。
- [ ] 按上下文集分组并判断活跃度。
- [ ] 阅读并生成六文件摘要。
- [ ] 对需要续档的默认上下文做归档/新入口。
- [ ] 将可复用经验同步到 EXPERIENCE.md / AGENTS.md / skill 或其它长期载体。
- [ ] 最后确认没有误暂存 runtime 或支线代码。

状态:
** 目前在候选文件发现阶段 **- 准备列出根目录上下文文件。
