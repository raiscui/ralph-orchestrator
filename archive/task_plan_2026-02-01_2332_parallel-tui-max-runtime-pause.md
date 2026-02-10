# 任务计划：并行 TUI 在 `LOOP_COMPLETE` 后重置并暂停 max_runtime 计时

## 目标

- 仅在 **parallel-tui（启用 TUI）** 下：
  - 当检测到 `LOOP_COMPLETE`（completion promise）后，进入“暂停态”。
  - 此时 `event_loop.max_runtime_seconds` 的计时 **重置**。
  - 在暂停态期间 **不计时**。
  - 直到任意 HatInstance 再次进入 `Running`，才开始重新计时。
- 在 **parallel-cli / CI / E2E（无 TUI）** 下：
  - 行为保持不变：`LOOP_COMPLETE` 仍然触发收敛退出（termination=CompletionPromise）。
  - `max_runtime_seconds` 仍然从 run 启动开始计时。
- 保持上一任务的交互体验：
  - TUI 下禁用动态实例 idle 回收，避免 instance 进入 `done` 造成 human message 不可达。
- 质量门禁：
  - `cargo fmt`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
  - replay smoke tests：`cargo test -p ralph-core smoke_runner`、`cargo test -p ralph-core kiro`

## 方案（至少二选一）

### 方案 A：只对 TUI 生效（你已选择）

- 把 max_runtime 视为“活跃运行窗口”的护栏：
  - `LOOP_COMPLETE` 之后进入停歇期，不应该因为无人输入而被 max_runtime 强行终止。
  - 有新的 job 开始跑（Running）才重新计时。

### 方案 B：全局重置（不推荐）

- parallel-cli 也改为暂停/重置 max_runtime。
- 会影响 CI/E2E 的确定性与无人值守退出策略，风险大。

> 你已明确选择：方案 A（只对 TUI 生效）。

## 阶段

- [x] 阶段1：更新 specs（明确计时语义）
- [x] 阶段2：实现计时状态机（pause/reset/resume）
- [x] 阶段3：补回归测试（暂停期不超时 + 恢复后按 Running 开始计时）
- [x] 阶段4：全量验证（含 smoke tests）
- [x] 阶段5：四文件记录 + 后续建议

## 关键问题

1. “开始重新计时”的定义是什么？
   - 采用你的口径：**任意实例进入 `HatInstanceState::Running`** 作为开始点。
2. 暂停态期间是否仍然允许外部事件（human.message）推进？
   - 必须允许：暂停态要继续消费 external events。

## 做出的决定

- [x] 仅 TUI 生效：`LOOP_COMPLETE` 后 max_runtime 重置并暂停，直到 Running 才恢复计时
- [x] 保持禁用动态实例回收（避免 done 断对话）

## 遇到错误

- （暂无）

## 状态

**已完成**
- 已实现：TUI 下 `LOOP_COMPLETE` 后 max_runtime 重置并暂停，直到任意实例 `Running` 才重新计时。
- 已完成：spec 同步、回归测试、全量验证、四文件记录。
