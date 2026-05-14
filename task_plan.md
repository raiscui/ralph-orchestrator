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
