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

---

## 2026-02-11 11:54 +0800 | 任务: PROMPT 模板提交 + 修补 worktree sandbox commit + 强化并行派发

### 我正在做什么 & 为什么

- 我正在把 `parallel-experimental-dev-engine` 的 PROMPT 文件恢复成可复用模板.
- 同时我在修补 worktree 在工具沙箱下无法 `git commit` 的问题.
- 这样你跑 example 时不会被卡在 commit 阶段,runner 也更容易真正并行跑起来.

### 阶段

- [x] 阶段1: PROMPT 模板整理,删除多余副本文件
- [x] 阶段2: 新增 `parallel.workspace.worktree_backend`(clone/worktree)
- [x] 阶段3: example 配置与文案同步(默认 clone + 强化派发规则)
- [x] 阶段4: fmt/clippy/test 验证
- [ ] 阶段5: git commit

### 遇到错误(来自你这次运行的反馈,已纳入修补范围)

- [记录写入错误]: 使用未加引号 heredoc 导致反引号命令替换,建议统一改为 `<<'EOF'`.
- [提交阻塞]: sandbox 禁止写入上级仓库 `.git/worktrees/.../index.lock`,导致 worktree 内 `git commit` 失败.
- [二次写入偏差]: 阻塞日志追加时再次触发反引号替换,建议固定使用带引号 heredoc.

### 状态

- 目前在阶段5: 准备提交代码与文档变更.

### 阶段完成(阶段5)

- 2026-02-11 11:56 +0800 已完成: git commit
  - `6bae384 fix(parallel): add clone worktree backend for sandboxed runners`
- 状态切换: 本任务已完成.
