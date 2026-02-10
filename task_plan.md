# 任务计划: git 提交(2026-02-11 00:15 +0800)

## 目标
把当前工作区变更整理成一次可追溯的提交.
提交前后都要保证仓库处于一致状态(含 submodule).

## 我正在做什么 & 为什么

- 我正在处理你刚说的"git 提交".
- 我会先盘点当前仓库的改动范围,再决定如何 stage/commit.
- 这样做可以避免漏提(submodule 常见)或误提(把临时文件也提交进去).

## 方案方向(两条路)

- 方案A(不惜代价,最佳方案,推荐): diff 审查(含 submodule) -> fmt/clippy/test -> 再提交.
  - 优点: 提交更稳,后续回滚/定位更容易.
  - 缺点: 更耗时.
- 方案B(先能用,后面再优雅): 先提交(依赖 pre-commit hook) -> 再补验证.
  - 优点: 快.
  - 缺点: hook 失败或测试失败时,需要返工重新提交.

## 阶段

- [x] 阶段0: 续档 task_plan 并做 continuous-learning(因旧文件>1000行)
- [x] 阶段1: 检查 git 状态与 diff(含 submodule)
- [x] 阶段2: 运行 fmt/clippy/test(按改动范围选择,至少跑 cargo test)
- [x] 阶段3: stage 变更(排除临时文件)并处理 submodule 提交
- [x] 阶段4: 执行 git commit(写清晰的 message)
- [x] 阶段5: 追加记录到 WORKLOG.md 收尾

## 关键问题
1. 当前是否存在 submodule 改动? 如果有,需要先在 submodule 内提交,再提交父仓库指针.
2. 本次提交是否只包含"我生成/修改"的变更? 如果混入用户本地改动,需要在 message 里说明范围.
3. 是否存在会被误提交的文件(例如构建产物、日志、会话记录)? 需要在 stage 前剔除.

## 做出的决定
- 默认采用方案A. 原因: 提交失败越早暴露,返工成本越低.

## 遇到错误
- (暂无)

## 状态
**已完成** - 已完成盘点/验证/提交,并把过程追加记录到 `WORKLOG.md`.

## 结果(2026-02-11 00:26 +0800)

- 已提交:`2b9e508 chore: archive four-file history and update example prompt`
- 已验证:
  - `cargo fmt --check` ✅
  - `cargo clippy --all-targets --all-features -- -D warnings` ✅
  - `cargo test` ✅
- 未纳入提交:
  - 未跟踪文件:`examples/parallel-experimental-dev-engine/PROMPT copy.md`
