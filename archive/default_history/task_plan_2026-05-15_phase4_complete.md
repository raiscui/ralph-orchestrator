# 任务计划: Phase 2 request/reply answer evidence 实现

## [2026-05-14 12:28:45] [Session ID: codex-20260514-phase2] 记录类型: Phase 2 实现启动与上下文续档

### 目标

按照 OpenSpec change `request-reply-answer-evidence` 实现 Phase 2 最小闭环: 让 `reply.hat.message` 的 requester-return 成功 / 失败 / missing 证据写入 Phase 1A evidence index,同时保持现有 routing 语义不漂移。

### 上下文续档

- 旧 `task_plan.md` 已超过 1000 行,按项目规则续档为 `task_plan_2026-05-14_phase1a_phase2_prev.md`。
- 旧计划最后状态显示:
  - Phase 1A evidence index kernel 已实现、验证、归档并提交。
  - Phase 2 OpenSpec change 已创建并验证,但尚未实现代码。
- 因当前主任务是进入 Phase 2 实现,continuous-learning 作为超过 1000 行触发项先登记为安全点待办,不阻塞本轮实现。

### 阶段

- [x] 阶段1: 续档旧 task_plan 并建立 Phase 2 当前计划入口。
- [ ] 阶段2: 重读 Phase 2 OpenSpec、稳定 spec 与相关 routing/evidence 代码。
- [ ] 阶段3: 先补 focused tests,覆盖 success / failure / missing / routing boundary。
- [ ] 阶段4: 实现最小 evidence index 写入,不引入 CLI UX 或 live topology 热改。
- [ ] 阶段5: 更新 OpenSpec tasks,运行 focused tests、smoke、全量测试与 OpenSpec gate。
- [ ] 阶段6: 审查 diff,本地提交,不 push。

### 关键边界

- 不实现 CLI/doctor/evidence summary UX。
- 不把所有 reply 或所有 answer 自动变成人类可见输出。
- 不修改 live topology。
- Evidence index 只做索引,原始事件日志仍是真相源。
- 不使用 `git add .`,不触碰无关用户改动。

### 当前状态

**目前在阶段2** - 下一步重读 OpenSpec 与代码锚点,先建立测试切口,再进入实现。

## [2026-05-14 12:29:30] [Session ID: codex-20260514-phase2] 状态更新: 开始读取 Phase 2 OpenSpec 与 apply 指令

- 当前阶段: 阶段2。
- 即将执行:
  - 读取 `openspec status --change request-reply-answer-evidence --json`。
  - 读取 `openspec instructions apply --change request-reply-answer-evidence --json`。
  - 按 contextFiles 重读 proposal/design/spec/tasks/test-plan。
  - 同步读取稳定 spec 与 routing/evidence 代码锚点。

## [2026-05-14 12:31:20] [Session ID: codex-20260514-phase2] 状态更新: 开始映射 routing 与 evidence 代码锚点

- 当前阶段: 阶段2。
- 观察到的规格事实:
  - 只有显式 `reply.hat.message` 且带非空 `reply` 参与 answer-return evidence。
  - success 要索引 request id 与 answer event id。
  - fail-closed 和 missing/timeout 必须可查询且区别于无 entry。
- 即将执行:
  - 读取 `crates/ralph-core/src/parallel/supervisor/routing.rs` 与测试。
  - 读取 `crates/ralph-core/src/evidence_index.rs`。
  - 读取 protocol topic 常量和 event model。
  - 判断最小实现是否应落在 routing 内部 helper,避免把 evidence 写入散落到调用方。

## [2026-05-14 12:34:00] [Session ID: codex-20260514-phase2] 阶段2 决策: evidence 写入落在 supervisor 内部 helper

- 静态证据:
  - `route_event()` 已在顶部专门收口 `reply.hat.message`。
  - success 分支会调用 `deliver_to_instance_id(..., RuntimeDeliveryMode::Reply)` 并已有 `routing.requester_return` 日志。
  - fail-closed 分支都集中调用 `log_requester_return_resolution()`。
  - `EventLogger` 默认真相源是 `.ralph/events.jsonl`,Phase 1A index 只保存 artifact link。
- 当前决策:
  - 在 `ParallelSupervisor` 内增加 evidence index writer 与一组私有 helper。
  - success 分支写 request id 与 answer event id 两类索引。
  - fail-closed 分支在 `log_requester_return_resolution()` 里写 failure/missing 索引。
  - missing/timeout marker 用一个显式私有 helper 暴露给测试,先不新增 broad broker 或 runtime timeout service。
- 备选解释/风险:
  - 若 runtime_delivery durable record id 无法稳定定位,不要伪造独立 artifact;先索引到 `.ralph/events.jsonl`。
  - 若 evidence 写入失败,不应改变 routing 结果,但必须 tracing warn,避免 evidence 层反向破坏运行时。

## [2026-05-14 12:36:10] [Session ID: codex-20260514-phase2] 阶段完成: OpenSpec 与代码锚点已重读

已完成:
- [x] 读取 `request-reply-answer-evidence` proposal/design/spec/tasks/test-plan。
- [x] 读取稳定 `runtime-evidence-index-kernel` 与 `hat-request-reply-channel` spec。
- [x] 定位 `route_event()` 的 `reply.hat.message` success/fail-closed 分支。
- [x] 定位 `log_requester_return_resolution()` 与 `deliver_to_instance_id()`。
- [x] 读取现有 routing tests,确认已有 request/reply 基础语义测试。

当前待办列表:
- [x] 阶段1: 续档旧 task_plan 并建立 Phase 2 当前计划入口。
- [x] 阶段2: 重读 Phase 2 OpenSpec、稳定 spec 与相关 routing/evidence 代码。
- [ ] 阶段3: 先补 focused tests,覆盖 success / failure / missing / routing boundary。
- [ ] 阶段4: 实现最小 evidence index 写入,不引入 CLI UX 或 live topology 热改。
- [ ] 阶段5: 更新 OpenSpec tasks,运行 focused tests、smoke、全量测试与 OpenSpec gate。
- [ ] 阶段6: 审查 diff,本地提交,不 push。

当前状态:
- 进入阶段3,先写 focused tests。

## [2026-05-14 12:39:20] [Session ID: codex-20260514-phase2] 状态更新: fmt gate 发现格式差异

- 现象: `cargo fmt --all -- --check` 返回格式 diff。
- 结论: 这是 rustfmt 格式化差异,不是业务逻辑失败。
- 即将执行:
  - 运行 `cargo fmt --all`。
  - 重跑 focused routing test。

## [2026-05-14 12:43:10] [Session ID: codex-20260514-phase2] 状态更新: focused tests 初跑通过,补齐无 reply guardrail

- 已通过:
  - success evidence lookup by request / answer id。
  - unknown request id fail-closed evidence。
  - missing requester source fail-closed evidence。
  - missing expected answer marker。
  - ordinary workflow event with reply 不被当作 answer-return evidence。
  - internal reply.hat.message 不自动生成 reply.human.message。
- 发现 test-plan 里还有一个边角: `reply.hat.message` without non-empty `reply` fails closed。
- 即将执行: 补充该 focused test,然后跑 routing_tests 相关集合。

## [2026-05-14 12:46:20] [Session ID: codex-20260514-phase2] 阶段完成: routing focused tests 已通过

已完成:
- [x] 新增 success evidence lookup 测试。
- [x] 新增 unknown request id fail-closed evidence 测试。
- [x] 新增 request without source_instance fail-closed evidence 测试。
- [x] 新增 no reply fail-closed evidence 测试。
- [x] 新增 missing expected answer marker 测试。
- [x] 新增 ordinary workflow event with reply boundary 测试。
- [x] 新增 reply.hat.message 不自动 reply.human.message 测试。
- [x] `cargo test --package ralph-core --lib parallel::supervisor::routing_tests`: 48 passed。

当前待办列表:
- [x] 阶段1: 续档旧 task_plan 并建立 Phase 2 当前计划入口。
- [x] 阶段2: 重读 Phase 2 OpenSpec、稳定 spec 与相关 routing/evidence 代码。
- [x] 阶段3: 先补 focused tests,覆盖 success / failure / missing / routing boundary。
- [x] 阶段4: 实现最小 evidence index 写入,不引入 CLI UX 或 live topology 热改。
- [ ] 阶段5: 更新 OpenSpec tasks,运行 focused tests、smoke、全量测试与 OpenSpec gate。
- [ ] 阶段6: 审查 diff,本地提交,不 push。

当前状态:
- 进入阶段5,先更新 OpenSpec tasks,再跑验证门。

## [2026-05-14 12:49:00] [Session ID: codex-20260514-phase2] 状态更新: 修正 producer 字段语义

- 现象: 初版实现把 fail/missing reason 放进了 `EvidenceIndexEntry.producer`。
- 判断: 这会让 producer 字段混入原因文本,破坏 schema 的语义清晰度。
- 修正方向:
  - `producer` 固定为 runtime 写入者身份。
  - 失败/缺失原因保留在 `routing.requester_return` JSONL artifact 中。
  - missing lifecycle helper 先写 diagnostic event,再写 missing marker。

## [2026-05-14 12:52:00] [Session ID: codex-20260514-phase2] 状态更新: focused evidence gates 已通过

- `cargo test --package ralph-core --lib parallel::supervisor::routing_tests`: 48 passed。
- `cargo test --package ralph-core --lib evidence_index::tests`: 7 passed。
- OpenSpec tasks 已勾选 4.1 与 4.2。
- 下一步运行 smoke tests。

## [2026-05-14 12:55:10] [Session ID: codex-20260514-phase2] 状态更新: 处理 smoke gate warning

- 现象: `cargo test -p ralph-core smoke_runner` 通过,但编译输出 warning: `record_missing_answer_evidence` 和 `log_missing_answer_evidence_marker` dead_code。
- 判断: 不能忽略 warning。Phase 2 规格要求 missing/timeout marker 可注册,所以该能力不应只作为 test-only 私有 helper 存在。
- 修正方向: 将 `record_missing_answer_evidence` 暴露为 `ParallelSupervisor` 的公开最小方法,保留内部诊断日志 helper 私有。

## [2026-05-14 12:58:30] [Session ID: codex-20260514-phase2] 状态更新: smoke gate 已通过,进入全量测试

- `cargo test -p ralph-core smoke_runner`: 12 passed,无 warning。
- OpenSpec tasks 已勾选 4.3。
- 即将执行: `cargo test`。

## [2026-05-14 13:04:00] [Session ID: codex-20260514-phase2] 阶段完成: Phase 2 验证门已通过

已通过验证:
- `cargo test --package ralph-core --lib parallel::supervisor::routing_tests`: 48 passed。
- `cargo test --package ralph-core --lib evidence_index::tests`: 7 passed。
- `cargo test -p ralph-core smoke_runner`: 12 passed,无 warning。
- `cargo test`: workspace tests and doctests passed。
- `cargo fmt --all -- --check`: passed。
- `git diff --check`: passed。
- `openspec validate request-reply-answer-evidence --type change`: valid。
- `openspec validate --all --strict`: 26 passed,0 failed。

当前待办列表:
- [x] 阶段1: 续档旧 task_plan 并建立 Phase 2 当前计划入口。
- [x] 阶段2: 重读 Phase 2 OpenSpec、稳定 spec 与相关 routing/evidence 代码。
- [x] 阶段3: 先补 focused tests,覆盖 success / failure / missing / routing boundary。
- [x] 阶段4: 实现最小 evidence index 写入,不引入 CLI UX 或 live topology 热改。
- [x] 阶段5: 更新 OpenSpec tasks,运行 focused tests、smoke、全量测试与 OpenSpec gate。
- [ ] 阶段6: 审查 diff,本地提交,不 push。

当前状态:
- 进入阶段6,审查 diff 并本地提交。

## [2026-05-14 13:31:00] [Session ID: codex-20260514-archive-learning] 任务启动: Phase 2 OpenSpec archive 与续档 continuous-learning 收口

# 目标

把已完成的 `request-reply-answer-evidence` OpenSpec change 收口归档, 完成 `task_plan.md` 续档触发的 continuous-learning, 然后进入下一条低风险演进线。

# 阶段

- [ ] 阶段1: 重新验证当前仓库、OpenSpec 与工作区状态。
- [ ] 阶段2: archive `request-reply-answer-evidence`, 同步主 spec, 并运行 OpenSpec gate。
- [ ] 阶段3: 执行 `task_plan` 续档触发的 continuous-learning, 回读六文件和历史版本, 沉淀长期知识并归档历史文件。
- [ ] 阶段4: 基于现有 Phase 2 能力选择下一演进线, 优先做 live runtime answer evidence dogfood, 只在证据不足时转 Phase 3 spec。
- [ ] 阶段5: 验证、审查 diff, 必要时本地提交, 不 push。

# 关键约束

- 不动无关 `tui-mdfried-viewer` change。
- 不把 archive/continuous-learning 扩成新的 runtime 大改。
- continuous-learning 只沉淀已验证经验, 不抢下一阶段实现权。
- 如果开始 dogfood, 必须用 record-session / evidence index / OpenSpec gate 形成可查证结果。

# 当前状态

**目前在阶段1** - 已确认工作区初始干净, 正在重新读取 OpenSpec 状态与 archive 输入。

## [2026-05-14 13:33:00] [Session ID: codex-20260514-archive-learning] 状态更新: OpenSpec list 输出完整但命令超时

- 现象: `timeout 15s openspec list --json` 已输出 active changes JSON,随后进程未及时退出,最终返回 124。
- 已观察事实:
  - `request-reply-answer-evidence`: 19/19 complete。
  - `tui-mdfried-viewer`: 13/15 in-progress,无关本轮。
- 当前处理:
  - 不把 124 当作成功静默跳过。
  - 后续 OpenSpec 命令改为单命令限时执行,采信完整输出,并用 `openspec validate --all --strict` 作为最终 gate。
  - 若 validate 也超时或失败,必须停下来处理。

## [2026-05-14 13:36:00] [Session ID: codex-20260514-archive-learning] 阶段1完成: archive 输入事实已复核

已完成:
- [x] 重新确认初始 `git status --short --untracked-files=all` 干净。
- [x] `request-reply-answer-evidence` 在 OpenSpec list 输出中为 19/19 complete。
- [x] `tasks.md` 无未勾选任务。
- [x] delta spec 位于 `openspec/changes/request-reply-answer-evidence/specs/request-reply-answer-evidence/spec.md`。
- [x] OpenSpec CLI `archive` 命令支持自动更新 main specs。

当前待办列表:
- [x] 阶段1: 重新验证当前仓库、OpenSpec 与工作区状态。
- [ ] 阶段2: archive `request-reply-answer-evidence`, 同步主 spec, 并运行 OpenSpec gate。
- [ ] 阶段3: 执行 `task_plan` 续档触发的 continuous-learning, 回读六文件和历史版本, 沉淀长期知识并归档历史文件。
- [ ] 阶段4: 基于现有 Phase 2 能力选择下一演进线, 优先做 live runtime answer evidence dogfood, 只在证据不足时转 Phase 3 spec。
- [ ] 阶段5: 验证、审查 diff, 必要时本地提交, 不 push。

当前状态:
- 进入阶段2,执行 OpenSpec archive 并检查稳定 spec。

## [2026-05-14 13:39:00] [Session ID: codex-20260514-archive-learning] 状态更新: 修正 archive 生成的 Purpose TBD

- 现象: `openspec archive` 已创建 `openspec/specs/request-reply-answer-evidence/spec.md`,但 Purpose 是 `TBD - created by archiving change...`。
- 判断: 这会降低稳定 spec 的可读性,也和之前 Phase 1A archive 收尾经验一致。
- 即将执行:
  - 用正式 Purpose 替换 TBD。
  - 运行 OpenSpec validation gate。

## [2026-05-14 13:42:00] [Session ID: codex-20260514-archive-learning] 阶段2完成: request-reply-answer-evidence 已归档并通过 OpenSpec gate

已完成:
- [x] `openspec archive request-reply-answer-evidence --yes` 创建稳定 spec 并移动 change 到 archive。
- [x] 修正 `openspec/specs/request-reply-answer-evidence/spec.md` 的 `Purpose TBD`。
- [x] `openspec validate --all --strict`: 26 passed,0 failed。
- [x] `git diff --check`: passed。
- [x] archive 后 active change 只剩 `tui-mdfried-viewer`。

注意:
- OpenSpec CLI 仍输出 PostHog flush network error,但命令退出码为 0,规格验证内容通过。

当前待办列表:
- [x] 阶段1: 重新验证当前仓库、OpenSpec 与工作区状态。
- [x] 阶段2: archive `request-reply-answer-evidence`, 同步主 spec, 并运行 OpenSpec gate。
- [ ] 阶段3: 执行 `task_plan` 续档触发的 continuous-learning, 回读六文件和历史版本, 沉淀长期知识并归档历史文件。
- [ ] 阶段4: 基于现有 Phase 2 能力选择下一演进线, 优先做 live runtime answer evidence dogfood, 只在证据不足时转 Phase 3 spec。
- [ ] 阶段5: 验证、审查 diff, 必要时本地提交, 不 push。

当前状态:
- 进入阶段3,执行 continuous-learning 的六文件候选发现与分组。

## [2026-05-14 14:01:00] [Session ID: codex-20260514-archive-learning] 阶段3完成: task_plan 续档 continuous-learning 已收口

已完成:
- [x] 按 continuous-learning skill 列出根目录默认六文件、历史版本和旧支线候选。
- [x] 按上下文组判断活跃度,当前根目录只保留默认六文件。
- [x] 在 `notes.md` 写入本轮六文件摘要。
- [x] 在 `EXPERIENCE.md` 新增 `exp-20260514-request-reply-answer-evidence-boundary`。
- [x] 将已覆盖的默认历史版本移入 `archive/default_history/`。
- [x] 将旧支线组移入 `archive/branch_contexts/<topic>/`。
- [x] 写入 `archive/manifests/ARCHIVE_MANIFEST__task_plan_rollover_2026-05-14_1358.md`。
- [x] 清理 `LATER_PLANS.md` 中已完成的 continuous-learning 待办。

当前待办列表:
- [x] 阶段1: 重新验证当前仓库、OpenSpec 与工作区状态。
- [x] 阶段2: archive `request-reply-answer-evidence`, 同步主 spec, 并运行 OpenSpec gate。
- [x] 阶段3: 执行 `task_plan` 续档触发的 continuous-learning, 回读六文件和历史版本, 沉淀长期知识并归档历史文件。
- [ ] 阶段4: 基于现有 Phase 2 能力选择下一演进线, 优先做 live runtime answer evidence dogfood, 只在证据不足时转 Phase 3 spec。
- [ ] 阶段5: 验证、审查 diff, 必要时本地提交, 不 push。

当前状态:
- 进入阶段4,先判断 live runtime answer evidence dogfood 是否能在当前仓库以低风险方式完成。

## [2026-05-14 14:04:00] [Session ID: codex-20260514-archive-learning] 阶段4行动: 优先尝试 live runtime answer evidence dogfood

- 决策: 先尝试 dogfood Phase 2,因为它能直接证明 evidence index 服务真实 runtime 链路,比马上开 Phase 3 更贴近上一阶段产物。
- 成功条件:
  - 使用本地、低成本、可重复的 Ralph run 或 integration fixture 触发 `reply.hat.message` requester-return。
  - 产生 `.ralph/events.jsonl` 或 record-session JSONL。
  - 产生 `.ralph/evidence-index.jsonl`,且可按 request id / answer id 查到 answer evidence。
- 退路:
  - 如果当前 CLI/config 没有稳定低成本入口,不硬造功能;改为创建 Phase 3 capability invocation / child run evidence OpenSpec。
- 即将执行:
  - 读取现有 parallel runtime config、custom/mock backend、fixtures、integration tests。
  - 找最小 dogfood 命令或测试入口。

## [2026-05-14 14:13:00] [Session ID: codex-20260514-archive-learning] 阶段4错误: dogfood 测试误用 EvidenceIndexReader API

- 现象: `cargo test -p ralph-cli --test integration_answer_evidence -- --nocapture` 编译失败。
- 错误: `EvidenceIndexReader` 没有 `lookup` 方法。
- 当前结论: 这是新集成测试误用 API,不是 Phase 2 runtime 行为失败。
- 即将执行:
  - 读取 `crates/ralph-core/src/evidence_index.rs` reader API。
  - 修正测试调用真实方法。
  - 重跑 focused dogfood test。

## [2026-05-14 14:16:00] [Session ID: codex-20260514-archive-learning] 阶段4完成: live runtime answer evidence dogfood 已落成集成测试

已完成:
- [x] 新增 `crates/ralph-cli/tests/integration_answer_evidence.rs`。
- [x] 测试通过真实 `ralph run --no-tui --record-session` 启动 parallel runtime。
- [x] custom backend 按 `RALPH_HAT_INSTANCE_ID` 输出:
  - `ralph#1` 第一次发布 `research.request`。
  - `researcher#1` 发布 `reply.hat.message reply="req-dogfood-1"`。
  - `ralph#1` 第二次输出 `LOOP_COMPLETE`。
- [x] 测试断言 `.ralph/evidence-index.jsonl` 可按 request id 和 answer id 查到 success evidence。
- [x] 测试断言 `.ralph/events.jsonl` 包含 delivered requester-return record,record-session 包含 CompletionPromise termination。
- [x] `cargo test -p ralph-cli --test integration_answer_evidence -- --nocapture`: 1 passed。

当前待办列表:
- [x] 阶段1: 重新验证当前仓库、OpenSpec 与工作区状态。
- [x] 阶段2: archive `request-reply-answer-evidence`, 同步主 spec, 并运行 OpenSpec gate。
- [x] 阶段3: 执行 `task_plan` 续档触发的 continuous-learning, 回读六文件和历史版本, 沉淀长期知识并归档历史文件。
- [x] 阶段4: 基于现有 Phase 2 能力选择下一演进线, 优先做 live runtime answer evidence dogfood, 只在证据不足时转 Phase 3 spec。
- [ ] 阶段5: 验证、审查 diff, 必要时本地提交, 不 push。

当前状态:
- 进入阶段5,运行 focused gates、smoke、OpenSpec gate 与 diff 审查。

## [2026-05-14 14:18:00] [Session ID: codex-20260514-archive-learning] 阶段5错误: git diff --check 发现 EXPERIENCE.md EOF 空白行

- 现象: `git diff --check` 报 `EXPERIENCE.md:127: new blank line at EOF.`。
- 当前结论: 这是本轮追加 `EXPERIENCE.md` 时留下的末尾空白,不是业务逻辑失败。
- 修复: 删除文件末尾多余空白行。
- 即将执行: 修复 EOF,重跑 `git diff --check` 与后续 OpenSpec / 全量测试门禁。

## [2026-05-14 14:28:00] [Session ID: omx-1778510695653-7pd7o2] 阶段5接手: 验证、审查、提交收口

我从上一轮交接继续,当前目标不是重做已完成工作,而是确认最终门禁、补齐日志、审查 diff,并在验证通过后做本地提交。

当前待办列表:
- [x] 阶段1: 重新验证当前仓库、OpenSpec 与工作区状态。
- [x] 阶段2: archive `request-reply-answer-evidence`, 同步主 spec, 并运行 OpenSpec gate。
- [x] 阶段3: 执行 `task_plan` 续档触发的 continuous-learning, 回读六文件和历史版本, 沉淀长期知识并归档历史文件。
- [x] 阶段4: 基于现有 Phase 2 能力完成 live runtime answer evidence dogfood。
- [ ] 阶段5: 验证、审查 diff, 补 WORKLOG / ERRORFIX, 本地提交, 不 push。

即将执行:
- 检查是否仍有 `cargo test` 进程或遗留验证结果。
- 运行必要的最终门禁: `cargo test`, `cargo fmt --all -- --check`, `cargo test -p ralph-core smoke_runner`, `openspec validate --all --strict`, `git diff --check`。
- 审查 `git status` / `git diff --stat` / 关键 diff。
- 写入 WORKLOG 与必要的 ERRORFIX。
- 验证通过后按 Lore Commit Protocol 做本地提交。

## [2026-05-14 14:36:00] [Session ID: omx-1778510695653-7pd7o2] 阶段5进展: 最终门禁和 diff 审查已通过,准备日志收口

已完成:
- [x] `cargo test`: workspace tests and doctests passed,exit 0。
- [x] `cargo fmt --all -- --check`: passed。
- [x] `cargo test -p ralph-core smoke_runner`: 12 passed,0 failed。
- [x] `openspec validate --all --strict`: 26 passed,0 failed。
- [x] `git diff --check`: passed。
- [x] 复核 `crates/ralph-cli/tests/integration_answer_evidence.rs`,并补充中文注释说明 runtime dogfood 的真相源边界。
- [x] 重跑 `cargo test -p ralph-cli --test integration_answer_evidence`: 1 passed。

当前待办列表:
- [x] 阶段1: 重新验证当前仓库、OpenSpec 与工作区状态。
- [x] 阶段2: archive `request-reply-answer-evidence`, 同步主 spec, 并运行 OpenSpec gate。
- [x] 阶段3: 执行 `task_plan` 续档触发的 continuous-learning, 回读六文件和历史版本, 沉淀长期知识并归档历史文件。
- [x] 阶段4: 基于现有 Phase 2 能力完成 live runtime answer evidence dogfood。
- [ ] 阶段5: 补 WORKLOG / ERRORFIX,最终复核 staged diff,本地提交,不 push。

即将执行:
- 追加 WORKLOG.md 和 ERRORFIX.md。
- 检查是否需要 EPIPHANY_LOG.md;如果没有新的架构级风险,不追加。
- stage 全部相关移动与新增文件,复核 staged diff,然后本地提交。

## [2026-05-14 14:42:00] [Session ID: omx-1778510695653-7pd7o2] 阶段5行动: stage 并本地提交

准备提交前复核:
- 根目录六文件候选只剩当前默认六文件。
- `git diff --check`: passed。
- `openspec validate --all --strict`: 26 passed,0 failed。
- `cargo test`: 已完整通过。
- `cargo test -p ralph-cli --test integration_answer_evidence`: 已通过。
- `cargo test -p ralph-core smoke_runner`: 已通过。
- `git submodule status`: 没有列出 submodule。

即将执行:
- `git add -A`。
- 查看 `git diff --cached --stat` / `git diff --cached --name-status` / `git diff --cached --check`。
- 如果 staged diff 仍符合目标范围,做本地提交,不 push。

## [2026-05-14 14:45:00] [Session ID: omx-1778510695653-7pd7o2] 阶段5错误: staged diff check 发现 stable spec EOF 空白行

- 现象: `git diff --cached --check` 报 `openspec/specs/request-reply-answer-evidence/spec.md:91: new blank line at EOF.`。
- 当前结论: 这是 OpenSpec archive 生成/整理稳定 spec 时留下的末尾空白,不是规格内容失败。
- 修复: 移除多余空白行,保留一个标准 EOF newline。
- 即将执行: 重新 `git add -A`,再跑 `git diff --cached --check` 和 OpenSpec gate。

## [2026-05-14 14:51:00] [Session ID: omx-1778510695653-7pd7o2] 阶段5完成: 最终验证通过,执行本地提交

最终验证证据:
- [x] `cargo fmt --all -- --check`: passed。
- [x] `cargo test -p ralph-cli --test integration_answer_evidence`: 1 passed,0 failed。
- [x] `cargo test -p ralph-core smoke_runner`: 12 passed,0 failed。
- [x] `cargo test`: workspace tests and doctests passed,exit 0。
- [x] `openspec validate --all --strict`: 26 passed,0 failed。
- [x] `git diff --cached --check`: passed。
- [x] `git diff --check`: passed。

当前待办列表:
- [x] 阶段1: 重新验证当前仓库、OpenSpec 与工作区状态。
- [x] 阶段2: archive `request-reply-answer-evidence`, 同步主 spec, 并运行 OpenSpec gate。
- [x] 阶段3: 执行 `task_plan` 续档触发的 continuous-learning, 回读六文件和历史版本, 沉淀长期知识并归档历史文件。
- [x] 阶段4: 基于现有 Phase 2 能力完成 live runtime answer evidence dogfood。
- [x] 阶段5: 验证、审查 diff,补 WORKLOG / ERRORFIX,本地提交,不 push。

当前状态:
- 准备创建本地 commit。
- 本轮没有发现需要写入 EPIPHANY_LOG.md 的新架构级风险; Phase 3 方向已在 WORKLOG 总结里作为后续建议记录。

## [2026-05-14 14:58:00] [Session ID: omx-1778510695653-7pd7o2] 新任务启动: Phase 3 capability invocation / child run evidence 真实串联

目标:
- 进入 Phase 3,把 capability invocation / isolated child run 的证据链真实串到 durable JSONL 和 evidence index。
- 保持 Phase 2 的边界: 不热改 live topology,不把 evidence index 当事实源,不新增大平台式 broker。

初始状态:
- `git status --short --untracked-files=all`: clean。
- 上一轮 Phase 2 已提交为 `27dab0d Dogfood answer evidence through runtime`。
- 当前根目录六文件行数未超过 1000 行,无需再次触发续档。

阶段计划:
- [ ] 阶段1: 读取现有 capability invocation spec / code / tests,确认 Phase 3 最小契约。
- [ ] 阶段2: 创建 OpenSpec change 和 test plan,并通过 OpenSpec gate。
- [ ] 阶段3: 实现 isolated child run evidence wiring,优先改良现有 capability invocation 路径,避免新增平行机制。
- [ ] 阶段4: 增加 focused tests / integration dogfood,证明 artifacts、events、evidence-index 三者闭环。
- [ ] 阶段5: 跑 cargo fmt / focused tests / smoke / cargo test / openspec / diff check,审查并本地提交。

关键约束:
- 只做 `capability invocation / child run evidence` 最小真实串联。
- 不实现 live topology 热修改。
- 不把普通 runtime event 自动当 capability result。
- 不引入外部服务或真实 LLM E2E,优先使用 deterministic custom/fake runner 证明 runtime contract。

即将执行:
- 读取 OpenSpec 相关 skill 和现有 `capability-invocation` 稳定 spec。
- 查找 capability invoke 当前实现和测试。

## [2026-05-14 15:11:00] [Session ID: omx-1778510695653-7pd7o2] 阶段1完成: Phase 3 最小改良点确认

已完成:
- [x] 读取 `openspec/specs/capability-invocation/spec.md`。
- [x] 读取 `crates/ralph-core/src/capability.rs`。
- [x] 读取 `crates/ralph-cli/src/capability.rs`。
- [x] 读取 `crates/ralph-cli/tests/integration_capability.rs`。
- [x] 读取 `crates/ralph-core/src/evidence_index.rs`。

当前结论:
- 现有 capability invocation 已产生 isolated child/micro-run artifacts 和 events。
- Phase 3 最小缺口是没有写 `.ralph/evidence-index.jsonl`。
- 最小正确实现应改良现有 `invoke_isolated_with_runner()`,在 artifact 写出和 event logging 同路径注册 evidence index。

当前待办列表:
- [x] 阶段1: 读取现有 capability invocation spec / code / tests,确认 Phase 3 最小契约。
- [ ] 阶段2: 创建 OpenSpec change 和 test plan,并通过 OpenSpec gate。
- [ ] 阶段3: 实现 isolated child run evidence wiring,优先改良现有 capability invocation 路径,避免新增平行机制。
- [ ] 阶段4: 增加 focused tests / integration dogfood,证明 artifacts、events、evidence-index 三者闭环。
- [ ] 阶段5: 跑 cargo fmt / focused tests / smoke / cargo test / openspec / diff check,审查并本地提交。

即将执行:
- 创建 OpenSpec change `capability-child-run-evidence`。
- 写 proposal/design/spec/tasks/test-plan。

## [2026-05-14 15:19:00] [Session ID: omx-1778510695653-7pd7o2] 阶段2完成: Phase 3 OpenSpec 已创建并通过验证

已完成:
- [x] 创建 `openspec/changes/capability-child-run-evidence/`。
- [x] 写入 `proposal.md`。
- [x] 写入 `design.md`。
- [x] 写入 delta spec `specs/capability-invocation/spec.md`。
- [x] 写入 `tasks.md` 和 `test-plan.md`。
- [x] `openspec validate capability-child-run-evidence --type change`: valid。
- [x] `openspec validate --all --strict`: 27 passed,0 failed。

当前待办列表:
- [x] 阶段1: 读取现有 capability invocation spec / code / tests,确认 Phase 3 最小契约。
- [x] 阶段2: 创建 OpenSpec change 和 test plan,并通过 OpenSpec gate。
- [ ] 阶段3: 实现 isolated child run evidence wiring,优先改良现有 capability invocation 路径,避免新增平行机制。
- [ ] 阶段4: 增加 focused tests / integration dogfood,证明 artifacts、events、evidence-index 三者闭环。
- [ ] 阶段5: 跑 cargo fmt / focused tests / smoke / cargo test / openspec / diff check,审查并本地提交。

即将执行:
- 先扩展 `integration_capability` 和 capability 单元测试,让 evidence-index 缺失暴露出来。
- 再在 `invoke_isolated_with_runner()` 的既有 artifact 写出路径中注册 evidence index。

## [2026-05-14 15:26:00] [Session ID: omx-1778510695653-7pd7o2] 阶段4红灯验证: integration capability 暴露 evidence 缺口

现象:
- `cargo test -p ralph-cli --test integration_capability -- --nocapture` 失败。
- 失败断言: `matches!(evidence_lookup, EvidenceLookup::Entries(_))`。

结论:
- 测试成功暴露当前 Phase 3 缺口: capability invocation 已有 artifacts/events,但 evidence index lookup by invocation id 没有返回 entries。
- 这不是新测试误报,而是本阶段要实现的功能缺失。

当前待办列表:
- [x] 阶段1: 读取现有 capability invocation spec / code / tests,确认 Phase 3 最小契约。
- [x] 阶段2: 创建 OpenSpec change 和 test plan,并通过 OpenSpec gate。
- [ ] 阶段3: 实现 isolated child run evidence wiring,优先改良现有 capability invocation 路径,避免新增平行机制。
- [ ] 阶段4: 增加 focused tests / integration dogfood,证明 artifacts、events、evidence-index 三者闭环。
- [ ] 阶段5: 跑 cargo fmt / focused tests / smoke / cargo test / openspec / diff check,审查并本地提交。

即将执行:
- 在 `crates/ralph-cli/src/capability.rs` 引入 `EvidenceIndexWriter` / `EvidenceIndexEntry`。
- artifact 写出后立即写 evidence entry。
- 成功路径记录 invoke/result/resolved-config/event-log。
- 失败路径记录 invoke/failed/resolved-config/event-log。

## [2026-05-14 15:39:00] [Session ID: omx-1778510695653-7pd7o2] 阶段3/4完成: capability invocation evidence 已串入真实 CLI 路径

已完成:
- [x] 在 `crates/ralph-cli/src/capability.rs` 中复用现有 `invoke_isolated_with_runner()` 路径写 evidence index。
- [x] 成功路径记录:
  - `resolved-config.yml` -> `resolved_config`
  - `invoke.json` -> `capability_invoke_json`
  - `.ralph/events.jsonl` -> `event_log_jsonl`
  - `result.json` -> `capability_result_json`
- [x] 失败路径记录:
  - `failed.json` -> `capability_failed_json` + failure status
- [x] 扩展 capability 单元测试覆盖成功/失败 evidence entries。
- [x] 扩展 integration capability 测试通过真实 CLI 查 `.ralph/evidence-index.jsonl`。

验证:
- [x] 红灯: `cargo test -p ralph-cli --test integration_capability -- --nocapture` 曾失败在 missing evidence lookup。
- [x] 绿灯: `cargo test -p ralph-cli capability::tests -- --nocapture`: 4 passed。
- [x] 绿灯: `cargo test -p ralph-cli --test integration_capability -- --nocapture`: 2 passed。

当前待办列表:
- [x] 阶段1: 读取现有 capability invocation spec / code / tests,确认 Phase 3 最小契约。
- [x] 阶段2: 创建 OpenSpec change 和 test plan,并通过 OpenSpec gate。
- [x] 阶段3: 实现 isolated child run evidence wiring,优先改良现有 capability invocation 路径,避免新增平行机制。
- [x] 阶段4: 增加 focused tests / integration dogfood,证明 artifacts、events、evidence-index 三者闭环。
- [ ] 阶段5: 跑 cargo fmt / focused tests / smoke / cargo test / openspec / diff check,审查并本地提交。

即将执行:
- 更新 OpenSpec tasks 为已完成实现项。
- 运行格式化、focused gates、smoke、全量测试、OpenSpec strict 和 diff check。

## [2026-05-14 15:53:00] [Session ID: omx-1778510695653-7pd7o2] 阶段5验证完成: 准备 diff 审查与提交

已完成验证:
- [x] `cargo fmt --all -- --check`: passed。
- [x] `cargo test -p ralph-cli capability::tests`: 4 passed。
- [x] `cargo test -p ralph-cli --test integration_capability`: 2 passed。
- [x] `cargo test -p ralph-core smoke_runner`: 12 passed。
- [x] `openspec validate capability-child-run-evidence --type change`: valid。
- [x] `openspec validate --all --strict`: 27 passed,0 failed。
- [x] `git diff --check`: passed。
- [x] `cargo test`: workspace tests and doctests passed。

当前待办列表:
- [x] 阶段1: 读取现有 capability invocation spec / code / tests,确认 Phase 3 最小契约。
- [x] 阶段2: 创建 OpenSpec change 和 test plan,并通过 OpenSpec gate。
- [x] 阶段3: 实现 isolated child run evidence wiring,优先改良现有 capability invocation 路径,避免新增平行机制。
- [x] 阶段4: 增加 focused tests / integration dogfood,证明 artifacts、events、evidence-index 三者闭环。
- [ ] 阶段5: 审查 diff,stage,最终 diff check,本地提交。

即将执行:
- 查看 `git status` / `git diff --stat` / 关键 diff。
- 若范围符合 Phase 3,stage 后提交,不 push。

## [2026-05-14 16:00:00] [Session ID: omx-1778510695653-7pd7o2] 阶段5补充: 覆盖 evidence 写失败场景

审查发现:
- OpenSpec 明确要求 evidence-index 记录失败不能被隐藏。
- 初始实现已经在代码路径上会返回错误,但缺少 focused test 锁住这个契约。

已完成:
- [x] 增加 `isolated_invocation_fails_when_evidence_index_cannot_be_recorded`。
- [x] 通过创建目录占用 `.ralph/evidence-index.jsonl` 路径来模拟 evidence writer 无法写入。
- [x] 断言 runner 不会继续启动,且错误包含 `Failed to record evidence for`。

验证:
- [x] `cargo fmt --all -- --check`: passed。
- [x] `cargo test -p ralph-cli capability::tests -- --nocapture`: 5 passed。
- [x] `cargo test -p ralph-cli --test integration_capability -- --nocapture`: 2 passed。
- [x] `git diff --check`: passed。

即将执行:
- 再跑 `openspec validate --all --strict` 和 `cargo test`,确保补充测试后仍全局通过。

## [2026-05-14 16:08:00] [Session ID: omx-1778510695653-7pd7o2] 阶段5行动: archive 已完成的 Phase 3 OpenSpec change

当前状态:
- Phase 3 implementation 和 dogfood tests 已完成。
- `cargo test` 已通过。
- `openspec validate --all --strict` 已通过。
- `capability-child-run-evidence` tasks 全部勾选完成。

决策:
- 归档已完成的 `capability-child-run-evidence`,避免 active OpenSpec changes 堆积。
- archive 后要修正稳定 spec 的 `Purpose TBD`,并重新跑 OpenSpec gate。

即将执行:
- `openspec archive capability-child-run-evidence --yes`。
- 检查 `openspec/specs/capability-invocation/spec.md`。
- 修正 Purpose。
- 重跑 `openspec validate --all --strict` 与 `git diff --check`。

## [2026-05-14 16:13:00] [Session ID: omx-1778510695653-7pd7o2] 阶段5归档完成: Phase 3 OpenSpec 已同步稳定 spec

已完成:
- [x] `openspec archive capability-child-run-evidence --yes`。
- [x] active OpenSpec changes 只剩既有无关 `tui-mdfried-viewer`。
- [x] 修正 `openspec/specs/capability-invocation/spec.md` 中历史遗留的 `Purpose TBD`。

即将执行:
- 最终验证: fmt, focused tests, smoke, cargo test, OpenSpec strict, diff checks。
- 审查 status 和 staged diff。
- 本地提交,不 push。

## [2026-05-14 16:22:00] [Session ID: omx-1778510695653-7pd7o2] 阶段5完成: Phase 3 验证通过并准备提交

最终验证证据:
- [x] `cargo fmt --all -- --check`: passed。
- [x] `cargo test -p ralph-cli capability::tests`: 5 passed。
- [x] `cargo test -p ralph-cli --test integration_capability`: 2 passed。
- [x] `cargo test -p ralph-core smoke_runner`: 12 passed。
- [x] `cargo test`: workspace tests and doctests passed。
- [x] `openspec validate --all --strict`: 26 passed,0 failed。
- [x] `git diff --check`: passed。

当前待办列表:
- [x] 阶段1: 读取现有 capability invocation spec / code / tests,确认 Phase 3 最小契约。
- [x] 阶段2: 创建 OpenSpec change 和 test plan,并通过 OpenSpec gate。
- [x] 阶段3: 实现 isolated child run evidence wiring,优先改良现有 capability invocation 路径,避免新增平行机制。
- [x] 阶段4: 增加 focused tests / integration dogfood,证明 artifacts、events、evidence-index 三者闭环。
- [x] 阶段5: 跑 cargo fmt / focused tests / smoke / cargo test / openspec / diff check,审查并本地提交。

当前状态:
- 准备 stage 和本地提交。
- 本轮没有新增需要写入 EPIPHANY_LOG.md 的架构级风险。

## [2026-05-14 16:48:00] [Session ID: omx-1778510695653-7pd7o2] 新任务: Phase 3.1 capability invocation evidence UX + Phase 4 排期

目标:
- 先推进 Phase 3.1: 为 capability invocation evidence 增加用户/agent 可用的查询 UX。
- 将 Phase 4: live runtime capability invocation 登记为依赖 Phase 3.1 的下一条演进线,避免和查询 UX 混成一个不可验证的大改动。

执行边界:
- Phase 3.1 优先复用现有 evidence index reader 和 capability invocation artifacts。
- 不新增第二套 evidence store。
- 不把 Phase 4 的 live runtime 调用语义提前塞进 Phase 3.1。
- 仍遵循 OpenSpec: spec/test-plan 先行,实现后再 focused tests / smoke / cargo test / archive。

当前待办列表:
- [ ] 阶段1: 探查现有 CLI/evidence/capability 入口,确认最小 UX 落点。
- [ ] 阶段2: 创建 Phase 3.1 OpenSpec change,写 proposal/spec/design/tasks/test-plan。
- [ ] 阶段3: 跑 OpenSpec strict gate,用测试计划定义红灯。
- [ ] 阶段4: 实现 Phase 3.1 查询 UX 与 focused tests。
- [ ] 阶段5: 运行 fmt / focused tests / smoke / cargo test / OpenSpec / diff check。
- [ ] 阶段6: archive Phase 3.1 change,本地提交。
- [ ] 阶段7: 将 Phase 4 作为下一条 OpenSpec 候选登记到 LATER_PLANS.md。

即将执行:
- 使用 `omx explore` / `rg` 查现有 `evidence` CLI、`EvidenceIndexReader`、`capability invoke` 测试结构。

## [2026-05-14 17:02:00] [Session ID: omx-1778510695653-7pd7o2] 阶段2完成: Phase 3.1 OpenSpec 已通过 gate

已完成:
- [x] 创建 OpenSpec change `capability-evidence-inspect-ux`。
- [x] 写入 proposal/design/delta spec/tasks/test-plan。
- [x] `openspec validate capability-evidence-inspect-ux --type change`: valid。
- [x] `openspec validate --all --strict`: 27 passed,0 failed。

当前待办列表:
- [x] 阶段1: 探查现有 CLI/evidence/capability 入口,确认最小 UX 落点。
- [x] 阶段2: 创建 Phase 3.1 OpenSpec change,写 proposal/spec/design/tasks/test-plan。
- [x] 阶段3: 跑 OpenSpec strict gate,用测试计划定义红灯。
- [ ] 阶段4: 实现 Phase 3.1 查询 UX 与 focused tests。
- [ ] 阶段5: 运行 fmt / focused tests / smoke / cargo test / OpenSpec / diff check。
- [ ] 阶段6: archive Phase 3.1 change,本地提交。
- [ ] 阶段7: 将 Phase 4 作为下一条 OpenSpec 候选登记到 LATER_PLANS.md。

即将执行:
- 回读 `crates/ralph-cli/src/capability.rs` 和 integration test。
- 先补 inspect integration 红灯,再实现 CLI。

## [2026-05-14 17:15:00] [Session ID: omx-1778510695653-7pd7o2] 阶段4完成: capability inspect UX 已实现并 focused 通过

已完成:
- [x] 新增 `ralph tools capability inspect <invocation_id>`。
- [x] `--json` 输出 `invocation_id`、`index_path`、lookup `status` 和 `entries`。
- [x] human 输出展示 invocation、index path、status、artifact kind/path/producer/status。
- [x] `NoEntry` 返回非零错误,错误消息包含 invocation id 和 `.ralph/evidence-index.jsonl`。
- [x] explicit missing marker 保留为 `missing` lookup status。

验证:
- [x] 红灯: `cargo test -p ralph-cli --test integration_capability -- --nocapture` 失败于 `unrecognized subcommand 'inspect'`。
- [x] 绿灯: `cargo test -p ralph-cli --test integration_capability -- --nocapture`: 4 passed。
- [x] 绿灯: `cargo test -p ralph-cli capability::tests -- --nocapture`: 6 passed。

当前待办列表:
- [x] 阶段1: 探查现有 CLI/evidence/capability 入口,确认最小 UX 落点。
- [x] 阶段2: 创建 Phase 3.1 OpenSpec change,写 proposal/spec/design/tasks/test-plan。
- [x] 阶段3: 跑 OpenSpec strict gate,用测试计划定义红灯。
- [x] 阶段4: 实现 Phase 3.1 查询 UX 与 focused tests。
- [ ] 阶段5: 运行 fmt / focused tests / smoke / cargo test / OpenSpec / diff check。
- [ ] 阶段6: archive Phase 3.1 change,本地提交。
- [ ] 阶段7: 将 Phase 4 作为下一条 OpenSpec 候选登记到 LATER_PLANS.md。

即将执行:
- 完整验证链: fmt check, focused tests, smoke, cargo test, OpenSpec strict, diff check。

## [2026-05-14 17:23:00] [Session ID: omx-1778510695653-7pd7o2] 阶段5进展: 第一批验证通过,继续全量测试与归档

已完成验证:
- [x] `cargo fmt --all -- --check`: passed。
- [x] `cargo test -p ralph-cli --test integration_capability`: 4 passed。
- [x] `cargo test -p ralph-cli capability::tests`: 6 passed。
- [x] `cargo test -p ralph-core smoke_runner`: 12 passed。
- [x] `openspec validate capability-evidence-inspect-ux --type change`: valid。
- [x] `openspec validate --all --strict`: 27 passed,0 failed。
- [x] `git diff --check`: passed。

当前待办列表:
- [x] 阶段1: 探查现有 CLI/evidence/capability 入口,确认最小 UX 落点。
- [x] 阶段2: 创建 Phase 3.1 OpenSpec change,写 proposal/spec/design/tasks/test-plan。
- [x] 阶段3: 跑 OpenSpec strict gate,用测试计划定义红灯。
- [x] 阶段4: 实现 Phase 3.1 查询 UX 与 focused tests。
- [ ] 阶段5: 运行全量 `cargo test`,然后更新 tasks 验证项。
- [ ] 阶段6: archive Phase 3.1 change,本地提交。
- [ ] 阶段7: 将 Phase 4 作为下一条 OpenSpec 候选登记到 LATER_PLANS.md。

即将执行:
- `cargo test`。
- 如果通过,更新 OpenSpec tasks 和 WORKLOG。

## [2026-05-14 17:40:00] [Session ID: omx-1778510695653-7pd7o2] 阶段6行动: 归档 Phase 3.1 OpenSpec change

当前验证状态:
- `cargo test` 已通过 workspace tests 和 doctests。
- Phase 3.1 OpenSpec tasks 已全部勾选。
- 当前准备执行 archive,并在 archive 后检查稳定 spec 与 OpenSpec strict gate。

即将执行:
- `openspec archive capability-evidence-inspect-ux --yes`。
- 检查 `openspec/specs/capability-invocation/spec.md` 中新增 inspect UX requirement。
- 运行 `openspec validate --all --strict` 和 `git diff --check`。

## [2026-05-14 17:50:00] [Session ID: omx-1778510695653-7pd7o2] 阶段6/7完成: Phase 3.1 已归档并登记 Phase 4

已完成:
- [x] `openspec archive capability-evidence-inspect-ux --yes`。
- [x] 稳定 spec `openspec/specs/capability-invocation/spec.md` 已新增 inspect UX requirements。
- [x] `openspec validate --all --strict`: 26 passed,0 failed。
- [x] `git diff --check`: passed。
- [x] Phase 4 live runtime capability invocation 已写入 `LATER_PLANS.md`。
- [x] 本轮交付已写入 `WORKLOG.md`。

当前待办列表:
- [x] 阶段1: 探查现有 CLI/evidence/capability 入口,确认最小 UX 落点。
- [x] 阶段2: 创建 Phase 3.1 OpenSpec change,写 proposal/spec/design/tasks/test-plan。
- [x] 阶段3: 跑 OpenSpec strict gate,用测试计划定义红灯。
- [x] 阶段4: 实现 Phase 3.1 查询 UX 与 focused tests。
- [x] 阶段5: 运行 fmt / focused tests / smoke / cargo test / OpenSpec / diff check。
- [x] 阶段6: archive Phase 3.1 change。
- [x] 阶段7: 将 Phase 4 作为下一条 OpenSpec 候选登记到 LATER_PLANS.md。

即将执行:
- 最终审查 diff/status。
- 运行最终 `openspec validate --all --strict`、`git diff --check`、`git diff --cached --check`。
- 本地提交,不 push。

## [2026-05-14 17:58:00] [Session ID: omx-1778510695653-7pd7o2] 错误记录: commit hook 要求固定 OmX co-author trailer

现象:
- 第一次 `git commit` 被 PreToolUse hook 阻止。
- 提示: `git commit is blocked until the inline commit message satisfies the Lore format and includes the required OmX co-author trailer`。
- 第二次使用 `Co-authored-by: OmX <omx@local>` 仍被阻止。

原因:
- 本地 hook 需要固定 trailer: `Co-authored-by: OmX <omx@oh-my-codex.dev>`。

处理:
- 重新暂存本条记录。
- 用正确 co-author trailer 重新提交。

## [2026-05-15 09:20:00] [Session ID: omx-1778510695653-7pd7o2] 新任务: Phase 4 live runtime capability invocation

目标:
- 按 `LATER_PLANS.md` 推进 Phase 4: 让真实 parent run 中的 `ralph#1` 能选择并触发 capability invocation。
- capability 执行仍走 isolated child/micro-run。
- result/failure artifact 要回传 parent run。
- parent topology 不能热改。
- dogfood 时必须用 Phase 3.1 的 `ralph tools capability inspect <invocation_id>` 查询证据链。

执行边界:
- 先创建 OpenSpec change 和测试计划,再实现。
- 优先复用 Phase 3/3.1 的 `capability` module、artifact writer、evidence index、inspect UX。
- 不新增第二套 runtime broker。
- 不引入 live external LLM E2E;优先用 deterministic custom backend / dry-run path 做 dogfood。

当前待办列表:
- [ ] 阶段1: 创建 OpenSpec change `live-runtime-capability-invocation` 并写 proposal/design/spec/tasks/test-plan。
- [ ] 阶段2: 探查 parent run / ralph#1 / event parsing / capability module 当前代码路径。
- [ ] 阶段3: 实现 parent run 触发 isolated capability invocation 的最小闭环。
- [ ] 阶段4: 用 integration dogfood 验证 parent event/result/evidence/inspect。
- [ ] 阶段5: 跑 fmt / focused tests / smoke / cargo test / OpenSpec / diff checks。
- [ ] 阶段6: archive OpenSpec change,本地提交。

即将执行:
- 创建 OpenSpec change `live-runtime-capability-invocation`。
- 生成完整 OpenSpec artifacts 后再读 runtime 代码。

## [2026-05-15 09:27:00] [Session ID: omx-1778510695653-7pd7o2] 工具异常记录: openspec status 输出 JSON 后未退出

现象:
- `openspec status --change live-runtime-capability-invocation --json` 已打印完整 JSON。
- 但 Node 进程没有退出,导致 shell session 挂住。

处理:
- 清理匹配的 `openspec status` Node 进程。
- 当前 OpenSpec scaffold 已存在,继续按已打印的 artifact 状态推进。

状态:
- `proposal` ready。
- `design/specs/tasks` 依赖 proposal。

## [2026-05-15 09:36:00] [Session ID: omx-1778510695653-7pd7o2] 阶段1完成: Phase 4 OpenSpec gate 已通过

已完成:
- [x] 创建 OpenSpec change `live-runtime-capability-invocation`。
- [x] 写入 proposal/design/delta spec/tasks/test-plan。
- [x] `openspec validate live-runtime-capability-invocation --type change`: valid。
- [x] `openspec validate --all --strict`: 27 passed,0 failed。

当前待办列表:
- [x] 阶段1: 创建 OpenSpec change `live-runtime-capability-invocation` 并写 proposal/design/spec/tasks/test-plan。
- [ ] 阶段2: 探查 parent run / ralph#1 / event parsing / capability module 当前代码路径。
- [ ] 阶段3: 实现 parent run 触发 isolated capability invocation 的最小闭环。
- [ ] 阶段4: 用 integration dogfood 验证 parent event/result/evidence/inspect。
- [ ] 阶段5: 跑 fmt / focused tests / smoke / cargo test / OpenSpec / diff checks。
- [ ] 阶段6: archive OpenSpec change,本地提交。

即将执行:
- 查 `loop_runner`、`parallel_runner`、`event_logger`、`EventParser` 和 integration run 测试。
- 找一个最小可测试接入点。

## [2026-05-15 10:56:22] [Session ID: omx-1778510695653-7pd7o2] 阶段2继续: Phase 4 实现前刷新上下文

当前状态:
- Phase 4 OpenSpec change `live-runtime-capability-invocation` 已存在且前一轮验证通过。
- 当前继续从未完成的阶段2开始: 探查 parent run / `ralph#1` / event parsing / capability module 当前代码路径。

当前待办列表:
- [x] 阶段1: 创建 OpenSpec change `live-runtime-capability-invocation` 并写 proposal/design/spec/tasks/test-plan。
- [ ] 阶段2: 探查 parent run / ralph#1 / event parsing / capability module 当前代码路径。
- [ ] 阶段3: 实现 parent run 触发 isolated capability invocation 的最小闭环。
- [ ] 阶段4: 用 integration dogfood 验证 parent event/result/evidence/inspect。
- [ ] 阶段5: 跑 fmt / focused tests / smoke / cargo test / OpenSpec / diff checks。
- [ ] 阶段6: archive OpenSpec change,本地提交。

即将执行:
- 读取 OpenSpec artifacts,确认实现边界。
- 读取 `loop_runner` / `event_loop` / `capability` / integration tests,找到最小接入点。
- 先写 focused integration dogfood 的失败测试,再实现。

## [2026-05-15 11:18:09] [Session ID: omx-1778510695653-7pd7o2] 阶段2-4进展: Phase 4 最小闭环已通过 focused dogfood

已完成:
- [x] 阶段2: 探查 parent run / `ralph#1` / event parsing / capability module 当前代码路径。
- [x] 阶段3: 实现 parent run 触发 isolated capability invocation 的最小闭环。
- [x] 阶段4: 用 integration dogfood 验证 parent event/result/evidence/inspect。

当前实现口径:
- core `ParallelSupervisor` 只识别 `ralph#1` 输出的 `capability.request`。
- CLI 注入 `RuntimeCapabilityInvoker`,复用现有 isolated child/micro-run invocation path。
- parent result/failure 作为 `capability.result` / `capability.failed` event 回写 parent event log。
- parent topology 不热改,adapter 只写 artifacts / evidence index。

已运行验证:
- `cargo check -p ralph-core -p ralph-cli`: passed。
- `cargo test -p ralph-core capability_request -- --nocapture`: 5 passed。
- `cargo test -p ralph-cli --test integration_live_capability -- --nocapture`: 1 passed。
- `cargo test -p ralph-cli --test integration_capability -- --nocapture`: 4 passed。
- `cargo test -p ralph-cli capability::tests -- --nocapture`: 6 passed。

当前待办列表:
- [x] 阶段1: 创建 OpenSpec change `live-runtime-capability-invocation` 并写 proposal/design/spec/tasks/test-plan。
- [x] 阶段2: 探查 parent run / ralph#1 / event parsing / capability module 当前代码路径。
- [x] 阶段3: 实现 parent run 触发 isolated capability invocation 的最小闭环。
- [x] 阶段4: 用 integration dogfood 验证 parent event/result/evidence/inspect。
- [ ] 阶段5: 跑 fmt / focused tests / smoke / cargo test / OpenSpec / diff checks。
- [ ] 阶段6: archive OpenSpec change,本地提交。

即将执行:
- 更新 OpenSpec tasks 的实现/测试项。
- 跑 smoke runner、全量 cargo test、OpenSpec 和 diff checks。

## [2026-05-15 11:23:44] [Session ID: omx-1778510695653-7pd7o2] 阶段5完成: Phase 4 验证 gate 全部通过

已完成验证:
- [x] `openspec validate live-runtime-capability-invocation --type change`: valid。
- [x] `openspec validate --all --strict`: 27 passed,0 failed。
- [x] `cargo fmt --all -- --check`: passed。
- [x] `cargo test -p ralph-cli --test integration_capability`: 4 passed。
- [x] `cargo test -p ralph-cli --test integration_live_capability`: 1 passed。
- [x] `cargo test -p ralph-cli capability::tests`: 6 passed。
- [x] `cargo test -p ralph-core smoke_runner`: 12 passed。
- [x] `cargo test`: passed workspace tests and doctests。
- [x] `git diff --check` / `git diff --cached --check`: passed。

当前待办列表:
- [x] 阶段1: 创建 OpenSpec change `live-runtime-capability-invocation` 并写 proposal/design/spec/tasks/test-plan。
- [x] 阶段2: 探查 parent run / ralph#1 / event parsing / capability module 当前代码路径。
- [x] 阶段3: 实现 parent run 触发 isolated capability invocation 的最小闭环。
- [x] 阶段4: 用 integration dogfood 验证 parent event/result/evidence/inspect。
- [x] 阶段5: 跑 fmt / focused tests / smoke / cargo test / OpenSpec / diff checks。
- [ ] 阶段6: archive OpenSpec change,本地提交。

即将执行:
- archive OpenSpec change `live-runtime-capability-invocation`。
- archive 后重新跑 OpenSpec strict 和 diff check。
- 审查 git diff/status,再本地提交。

## [2026-05-15 11:28:04] [Session ID: omx-1778510695653-7pd7o2] 阶段6完成: Phase 4 已归档并准备提交

已完成:
- [x] `openspec archive live-runtime-capability-invocation --yes`。
- [x] 稳定 spec `openspec/specs/capability-invocation/spec.md` 已同步 Phase 4 requirements。
- [x] 已清理 `LATER_PLANS.md` 中 Phase 4 延期项,避免重复待办。
- [x] archive 后 `openspec validate --all --strict`: 26 passed,0 failed。
- [x] archive 后 `cargo fmt --all -- --check`: passed。
- [x] archive 后 `git diff --check` / `git diff --cached --check`: passed。

当前待办列表:
- [x] 阶段1: 创建 OpenSpec change `live-runtime-capability-invocation` 并写 proposal/design/spec/tasks/test-plan。
- [x] 阶段2: 探查 parent run / ralph#1 / event parsing / capability module 当前代码路径。
- [x] 阶段3: 实现 parent run 触发 isolated capability invocation 的最小闭环。
- [x] 阶段4: 用 integration dogfood 验证 parent event/result/evidence/inspect。
- [x] 阶段5: 跑 fmt / focused tests / smoke / cargo test / OpenSpec / diff checks。
- [x] 阶段6: archive OpenSpec change,本地提交准备。

EPIPHANY_LOG 检查:
- 本轮没有发现需要单独长期讨论的灾难级风险。
- 已经把“child lifecycle result 与 parent-return result 同 topic,测试必须按 request_id 区分”的问题写入 `ERRORFIX.md`。

即将执行:
- `git status` 最终审查。
- `git add` 本轮相关文件。
- 本地提交,不 push。

## [2026-05-15 11:40:00] [Session ID: omx-1778510695653-7pd7o2] 交接复核: Phase 4 完成态确认

当前目标:
- 复核上一轮交接中声称已完成的 Phase 4 live runtime capability invocation。
- 重点确认 Git 工作区是否干净、最近提交是否为 Phase 4、本地是否未 push。

当前待办列表:
- [x] 阶段1: 创建并完成 OpenSpec change `live-runtime-capability-invocation`。
- [x] 阶段2: 实现 parent `ralph#1` 触发 isolated child/micro-run invocation。
- [x] 阶段3: 回传 `capability.result` / `capability.failed` artifact 到 parent run。
- [x] 阶段4: 使用 Phase 3.1 inspect UX dogfood evidence chain。
- [x] 阶段5: focused tests / smoke tests / cargo test / OpenSpec / diff checks。
- [x] 阶段6: archive change 并本地提交。
- [ ] 阶段7: 最终交接复核并向用户汇报。

即将执行:
- `git status --short --untracked-files=all`
- `git log -1 --oneline`
- `git show -1 --stat --format=fuller`

## [2026-05-15 11:41:30] [Session ID: omx-1778510695653-7pd7o2] 阶段7完成: Phase 4 交接复核完成

复核结果:
- `git log -1 --oneline`: `ebb5c45 Let parent runs invoke isolated capabilities`。
- 最近提交包含 Phase 4 core protocol、parallel supervisor runtime hook、CLI adapter、integration dogfood、OpenSpec archive、上下文记录。
- 当前只有本次复核追加的 `task_plan.md` 记录是未提交变更。

当前待办列表:
- [x] 阶段1: 创建并完成 OpenSpec change `live-runtime-capability-invocation`。
- [x] 阶段2: 实现 parent `ralph#1` 触发 isolated child/micro-run invocation。
- [x] 阶段3: 回传 `capability.result` / `capability.failed` artifact 到 parent run。
- [x] 阶段4: 使用 Phase 3.1 inspect UX dogfood evidence chain。
- [x] 阶段5: focused tests / smoke tests / cargo test / OpenSpec / diff checks。
- [x] 阶段6: archive change 并本地提交。
- [x] 阶段7: 最终交接复核并向用户汇报。

EPIPHANY_LOG 检查:
- 本次复核没有发现新的架构级灾难风险。
- Phase 4 的主要风险已经由提交中的 `Directive` 和 `ERRORFIX.md` 记录: 查询 `capability.result` 时必须用 `request_id` 区分 parent-return result。
