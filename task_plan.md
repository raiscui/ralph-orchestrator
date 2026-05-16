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
