# 任务计划: 方向B.3 - structured failure classes for parent branching policy

## [2026-05-16 23:14:00] [Session ID: omx-1778510695653-7pd7o2] 续档启动: capability B.3 收口

目标:
- 继续 capability invocation failure 演进线,把 `capability.failed` 从“只有错误文本”推进到“带结构化失败类型”。
- 证明 parent run 可以依据结构化 `failure_class` 做分支决策,而不是解析自由文本 `error`。
- 保持边界: child/micro-run 仍隔离执行, parent topology 不热改,最终 human-facing answer 仍必须显式发布 `reply.human.message`。

续档原因:
- 原 `task_plan.md` 已达到 1120 行,超过六文件规则的 1000 行阈值。
- 已将旧计划快照保存到 `archive/default_history/task_plan_2026-05-16_2314_capability_b3_prev.md`。
- 本文件只承接当前 B.3 收口,避免旧上下文继续污染注意力窗口。

六文件摘要:
- `task_plan.md` 旧档最新主线是 B.1/B.2/B.3: 从 answer evidence inspect UX,到 explicit human reply,到 multi-step capability orchestration,再到 failure fallback 和 structured failure class。
- `notes.md` 已记录 B.1/B.2 的边界: `capability.result` / `capability.failed` 是 parent-consumable runtime event, `reply.human.message` 才是对人的最终输出。
- `WORKLOG.md` 已记录 B.1/B.2 的完成证据和 OpenSpec archive 结果。
- `LATER_PLANS.md` 当前没有直接阻塞 B.3 的待办; 与 startup/bootstrap 相关的事项仍是后续线。
- `ERRORFIX.md` / `EPIPHANY_LOG.md` 没有新增会改变 B.3 当前实现方向的约束。

阶段计划:
- [x] 阶段1: 盘点现有 failure payload 与最适合结构化的失败分类
- [x] 阶段2: 写窄 OpenSpec change,定义 `failure_class` 与 parent branching policy 的最小边界
- [x] 阶段3: 实现结构化 failure class,并让现有 fallback gate 按 class 断言
- [x] 阶段4: 补 focused gate / unit tests / stable spec sync / archive
- [x] 阶段5: 收口验证、归档与本地提交

当前已知现场:
- 已新增 `CapabilityFailureClass` 与 `failure_class` 字段。
- 已补 core/cli 构造点和 focused/unit/integration 断言。
- OpenSpec change `capability-failure-class-branching-policy` 已存在且 `openspec validate --all --strict` 曾通过。
- 上一轮 `cargo test` 未看到最终完成输出; 当前进程检查没有 Ralph 仓库的 `cargo test` 在跑,所以必须重新跑 fresh gates。

即将执行:
- review 当前 diff,确认实现与 OpenSpec change 一致。
- 继续 OpenSpec archive,并复跑 focused gates、smoke 和 `cargo test`。
- 将 B.3 的 notes / WORKLOG / task_plan 收口后做本地 commit,不 push。

状态:
- **目前在阶段4** - 先 review diff 与 OpenSpec 任务状态,再 archive 和验证。

## [2026-05-16 23:16:00] [Session ID: omx-1778510695653-7pd7o2] 阶段动作: B.3 archive 前验证

即将执行:
- 先跑 change-level OpenSpec validate 与 focused tests,确认 `failure_class` 的静态 contract 和动态 gate 都成立。
- 再跑 `cargo fmt --all -- --check`、`git diff --check` 与 `cargo test`,作为 archive 前 repo-wide gate。

状态:
- **目前在阶段4** - 进入 archive 前验证。

## [2026-05-16 23:31:00] [Session ID: omx-1778510695653-7pd7o2] 阶段完成: B.3 archive 与验证已完成

已完成:
- `CapabilityFailureClass` 已进入 core runtime capability records。
- parent-visible `capability.failed` 与 child/micro-run `failed.json` 都带 `failure_class`。
- live fallback gate 已证明 parent 可以看见 `invalid_capability_id` 并继续 fallback。
- OpenSpec change 已 archive,稳定 spec 已同步。
- `notes.md`、`WORKLOG.md`、`EXPERIENCE.md` 已补记录。

验证证据:
- `openspec validate capability-failure-class-branching-policy --type change`
- `cargo test -p ralph-core capability::tests -- --nocapture`
- `cargo test -p ralph-cli capability::tests -- --nocapture`
- `cargo test -p ralph-cli --test integration_live_capability parallel_parent_run_can_fallback_after_capability_failed_before_final_human_reply`
- `cargo test -p ralph-cli --test integration_live_capability`
- `cargo test -p ralph-core smoke_runner`
- `cargo fmt --all -- --check`
- `git diff --check`
- `openspec validate --all --strict`
- `cargo test`
- archive 后复跑: `openspec validate --all --strict`, `git diff --check`, `cargo test -p ralph-cli --test integration_live_capability`, `cargo test -p ralph-core smoke_runner`, `cargo test`

待办更新:
- [x] 阶段1: 盘点现有 failure payload 与最适合结构化的失败分类
- [x] 阶段2: 写窄 OpenSpec change,定义 `failure_class` 与 parent branching policy 的最小边界
- [x] 阶段3: 实现结构化 failure class,并让现有 fallback gate 按 class 断言
- [x] 阶段4: 补 focused gate / unit tests / stable spec sync / archive
- [x] 阶段5: 收口验证、归档与本地提交

状态:
- **当前任务已完成实现与验证** - 下一步执行本地 commit,不 push。

## [2026-05-16 23:36:00] [Session ID: omx-1778510695653-7pd7o2] 新任务启动: B.4 - richer branching policy without retry engine

目标:
- 继续 B.3 的 `failure_class` 结构化分支输入。
- 证明 parent run 可以根据不同 `failure_class` 做不同后续策略,但不引入通用 retry engine、planner 或 live topology mutation。
- 保持边界: `capability.failed` 是 parent-consumable runtime event,最终 human-facing answer 仍必须显式发 `reply.human.message`。

阶段计划:
- [x] 阶段1: 盘点现有 failure classes、integration gate 与最小 branching matrix 缺口
- [x] 阶段2: 写窄 OpenSpec change,定义 B.4 richer branching policy 的最小边界和非目标
- [x] 阶段3: 设计 focused dogfood gate,优先复用 `integration_live_capability.rs`
- [x] 阶段4: 若实现面仍然窄,进入代码/测试实现
- [x] 阶段5: archive OpenSpec,跑 focused/smoke/full gates,本地提交

当前假设:
- B.4 最小价值不是实现一个 retry framework,而是固定 parent policy 可以区分至少两类失败并走不同策略。
- 候选矩阵:
  - `invalid_capability_id` -> fallback capability request
  - `child_run_failed` -> degraded explicit human reply,不再重复调用
- 备选解释:
  - 如果现有 prompt/event replay 不能稳定把 `child_run_failed` 暴露给 parent 后续 turn,则 B.4 应先修 evidence/prompt replay,不要硬写 branching gate。

推翻当前假设的证据:
- 如果 child/micro-run failure 没有进入 parent-visible `capability.failed`,或者后续 turn 看不到 `failure_class`,则当前 branching matrix 不成立。
- 如果需要新增通用 retry engine 才能证明 B.4,说明 scope 已经太大,应退回 OpenSpec 重新切分。

状态:
- **目前在阶段1** - 先做只读勘查,确认最小 branching matrix 是否可用现有 runtime 证明。

## [2026-05-16 23:42:00] [Session ID: omx-1778510695653-7pd7o2] 阶段推进: B.4 最小 branching matrix 已确定

勘查结论:
- `invalid_capability_id` 已有 live fallback gate,可以作为矩阵中的“可恢复 fallback”分支。
- `malformed_request` 在 core supervisor 中已经是 parent-visible `capability.failed`,可以用 live gate 证明 parent 看到 class 后选择 diagnostic `reply.human.message`,而不是重试。
- `child_run_failed` 当前有 artifact/unit 覆盖,但 live dry-run 子流程默认成功。为了避免引入测试专用 runtime 开关,B.4 暂不把它作为 live matrix 的必需分支。

决策:
- B.4 新增一条 `malformed_request -> diagnostic human reply, no retry` focused gate。
- B.4 不新增 retry engine、planner、child failure injection 开关或 topology mutation。

状态:
- **阶段1完成,进入阶段2** - 写窄 OpenSpec change。

## [2026-05-16 23:47:00] [Session ID: omx-1778510695653-7pd7o2] 阶段动作: B.4 focused gate 实现后验证

即将执行:
- 先跑新增的 malformed-request diagnostic live gate。
- 如果通过,再跑整组 `integration_live_capability` 和 OpenSpec 全量校验。

状态:
- **目前在阶段4** - 用最小动态证据验证 B.4 branching matrix。

## [2026-05-16 23:50:00] [Session ID: omx-1778510695653-7pd7o2] 遇到错误: B.4 fmt check 失败

现象:
- `cargo test -p ralph-cli --test integration_live_capability` 通过,5 passed。
- `openspec validate --all --strict` 通过,27 passed。
- `cargo fmt --all -- --check` 失败,指出新增 B.4 测试函数签名和长 assert 需要格式化。

处理:
- 运行 `cargo fmt --all`。
- 重新跑 focused/live/OpenSpec/fmt/diff check,确认格式化后无回归。

状态:
- **目前在阶段4** - 修复格式化门禁失败。

## [2026-05-16 23:55:00] [Session ID: omx-1778510695653-7pd7o2] 阶段推进: B.4 archive 前验证通过

已验证:
- 新增 malformed diagnostic gate 通过。
- 整组 `integration_live_capability` 通过,包含既有 invalid fallback 分支和新增 malformed no-retry diagnostic 分支。
- `openspec validate --all --strict` 通过。
- `cargo fmt --all -- --check` 通过。
- `git diff --check` 通过。
- `cargo test -p ralph-core smoke_runner` 通过。
- `cargo test` 通过。

状态:
- **阶段4完成,进入阶段5** - archive OpenSpec change 并做最终提交。

## [2026-05-17 00:05:00] [Session ID: omx-1778510695653-7pd7o2] 阶段完成: B.4 richer branching policy 已收口

已完成:
- OpenSpec change `capability-failure-branching-matrix` 已 archive。
- 稳定 spec 已同步 class-specific failure branching requirements。
- 新增 malformed-request diagnostic no-retry live gate。
- 既有 invalid capability id fallback gate 仍通过。
- `notes.md`、`WORKLOG.md`、`EXPERIENCE.md` 已补记录。

验证证据:
- `openspec validate capability-failure-branching-matrix --type change`
- `cargo test -p ralph-cli --test integration_live_capability parallel_parent_run_can_emit_diagnostic_reply_for_malformed_capability_request_without_retry -- --nocapture`
- `cargo test -p ralph-cli --test integration_live_capability parallel_parent_run_can_emit_diagnostic_reply_for_malformed_capability_request_without_retry`
- `cargo test -p ralph-cli --test integration_live_capability`
- `openspec validate --all --strict`
- `cargo fmt --all -- --check`
- `git diff --check`
- `cargo test -p ralph-core smoke_runner`
- `cargo test`
- archive 后复跑: `openspec validate --all --strict`, `cargo test -p ralph-cli --test integration_live_capability`, `cargo test -p ralph-core smoke_runner`, `cargo test`

待办更新:
- [x] 阶段1: 盘点现有 failure classes、integration gate 与最小 branching matrix 缺口
- [x] 阶段2: 写窄 OpenSpec change,定义 B.4 richer branching policy 的最小边界和非目标
- [x] 阶段3: 设计 focused dogfood gate,优先复用 `integration_live_capability.rs`
- [x] 阶段4: 若实现面仍然窄,进入代码/测试实现
- [x] 阶段5: archive OpenSpec,跑 focused/smoke/full gates,本地提交

状态:
- **当前任务已完成实现与验证** - 下一步执行本地 commit,不 push。
