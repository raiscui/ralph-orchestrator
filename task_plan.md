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

---

## 2026-02-11 16:00 +0800 | 任务: parallel-experimental-dev-engine 收敛语义与 topic 约束修正

### 我正在做什么 & 为什么

- 我正在修正 `parallel-experimental-dev-engine` 示例中的收敛提示语义与事件路由语义.
- 你指出了两个关键风险: `LOOP_COMPLETE` 前缺少完成总结,以及误发无人接收的 `experiment.start`.
- 这些问题会直接影响示例可复现性和可读性,所以我会优先把配置提示词与文档口径对齐.

### 方案方向(两条路)

- 方案A(不惜代价,最佳方案,推荐): 同步修改 `ralph.yml` + `README.md` + 相关说明文档,并补回归验证.
  - 优点: 口径一致,后续不容易再次误发 topic.
  - 缺点: 改动文件略多.
- 方案B(先能用,后面再优雅): 仅改 `ralph.yml` 提示词.
  - 优点: 改动最小.
  - 缺点: README 仍可能继续误导使用者.

### 决策

- 采用方案A. 原因: 你的诉求是示例行为约束,需要配置与文档同时收敛.

### 阶段

- [ ] 阶段1: 定位所有涉及 `LOOP_COMPLETE` 与 `experiment.start` 的示例语句
- [ ] 阶段2: 修改示例配置与文档,明确"先总结再 LOOP_COMPLETE"且禁止无人接收 topic
- [ ] 阶段3: 运行最小验证并复查改动一致性
- [ ] 阶段4: 追加记录到 `notes.md` 与 `WORKLOG.md`,交付结果

### 状态

**目前在阶段1** - 正在定位示例文件中的目标语句.

### 阶段进展更新(2026-02-11 16:06 +0800)

- [x] 阶段1: 定位所有涉及 `LOOP_COMPLETE` 与 `experiment.start` 的示例语句
- [ ] 阶段2: 修改示例配置与文档,明确"先总结再 LOOP_COMPLETE"且禁止无人接收 topic
- [ ] 阶段3: 运行最小验证并复查改动一致性
- [ ] 阶段4: 追加记录到 `notes.md` 与 `WORKLOG.md`,交付结果

### 当前状态

**目前在阶段2** - 开始编辑 `examples/parallel-experimental-dev-engine/ralph.yml` 与 `examples/parallel-experimental-dev-engine/README.md`.

### 阶段进展更新(2026-02-11 16:10 +0800)

- [x] 阶段1: 定位所有涉及 `LOOP_COMPLETE` 与 `experiment.start` 的示例语句
- [x] 阶段2: 修改示例配置与文档,明确"先总结再 LOOP_COMPLETE"且禁止无人接收 topic
- [x] 阶段3: 运行最小验证并复查改动一致性
- [ ] 阶段4: 追加记录到 `notes.md` 与 `WORKLOG.md`,交付结果

### 阶段3验证结果

- `cargo fmt --check` 通过.
- `cargo test -p ralph-core smoke_runner` 通过.
- 示例文件检索确认: 不再把 `experiment.start` 作为工作流入口指令.

### 阶段进展更新(2026-02-11 16:12 +0800)

- [x] 阶段1: 定位所有涉及 `LOOP_COMPLETE` 与 `experiment.start` 的示例语句
- [x] 阶段2: 修改示例配置与文档,明确"先总结再 LOOP_COMPLETE"且禁止无人接收 topic
- [x] 阶段3: 运行最小验证并复查改动一致性
- [x] 阶段4: 追加记录到 `notes.md` 与 `WORKLOG.md`,交付结果

### 状态

**已完成** - 示例协议已按本次要求收敛,且完成最小验证.

---

## 2026-02-11 16:20 +0800 | 任务: 为所有 hat/ralph 注入 ralph_hat_instance_id

### 我正在做什么 & 为什么

- 我正在实现: 给所有 hat（包括 `ralph`）的 prompt 注入统一字段 `ralph_hat_instance_id`.
- 这样每个运行实例都能在 prompt 中明确识别自己的身份,避免多实例场景下角色漂移.

### 方案方向(两条路)

- 方案A(不惜代价,最佳方案,推荐): 在 prompt 构建层统一注入字段,覆盖串行 EventLoop 与并行 Supervisor/Instance 两条路径,并补测试.
  - 优点: 语义一致,不依赖某个 preset 或单个流程.
  - 缺点: 涉及文件略多.
- 方案B(先能用,后面再优雅): 仅在并行路径注入.
  - 优点: 改动少.
  - 缺点: 串行/非并行路径不一致.

### 决策

- 采用方案A,保证全局一致.

### 阶段

- [ ] 阶段1: 定位所有 prompt 构建入口与现有实例标识字段
- [ ] 阶段2: 实现 `ralph_hat_instance_id` 注入(含 ralph)
- [ ] 阶段3: 补充/调整测试并运行验证
- [ ] 阶段4: 记录到 notes/WORKLOG 并交付

### 状态

**目前在阶段1** - 正在定位 prompt 构建代码与测试覆盖点.

### 阶段进展更新(2026-02-11 16:27 +0800)

- [x] 阶段1: 定位所有 prompt 构建入口与现有实例标识字段
- [ ] 阶段2: 实现 `ralph_hat_instance_id` 注入(含 ralph)
- [ ] 阶段3: 补充/调整测试并运行验证
- [ ] 阶段4: 记录到 notes/WORKLOG 并交付

### 当前状态

**目前在阶段2** - 开始修改运行时 prompt 组装逻辑,统一注入 `ralph_hat_instance_id`.

### 阶段进展更新(2026-02-11 16:34 +0800)

- [x] 阶段1: 定位所有 prompt 构建入口与现有实例标识字段
- [x] 阶段2: 实现 `ralph_hat_instance_id` 注入(含 ralph)
- [ ] 阶段3: 补充/调整测试并运行验证
- [ ] 阶段4: 记录到 notes/WORKLOG 并交付

### 阶段2实现点

- 串行 EventLoop: 在 `build_prompt` 与 `build_ralph_prompt` 顶部注入 `ralph_hat_instance_id`.
- 并行 HatInstance: 在 `build_prompt` 顶部注入当前 `instance_id`.
- 并行 LLM dispatch decider: 注入 `ralph_hat_instance_id:"ralph#decider-<job_id>"`.

### 阶段进展更新(2026-02-11 16:37 +0800)

- [x] 阶段1: 定位所有 prompt 构建入口与现有实例标识字段
- [x] 阶段2: 实现 `ralph_hat_instance_id` 注入(含 ralph)
- [x] 阶段3: 补充/调整测试并运行验证
- [ ] 阶段4: 记录到 notes/WORKLOG 并交付

### 阶段3验证结果

- `cargo fmt --check` ✅
- 定向测试 ✅
  - `cargo test -p ralph-core test_ralph_prompt_includes_ghuntley_style`
  - `cargo test -p ralph-core parallel_injects_event_loop_ralph_prompt_only_for_ralph`
- `cargo test`(全量) ✅

### 阶段进展更新(2026-02-11 16:39 +0800)

- [x] 阶段1: 定位所有 prompt 构建入口与现有实例标识字段
- [x] 阶段2: 实现 `ralph_hat_instance_id` 注入(含 ralph)
- [x] 阶段3: 补充/调整测试并运行验证
- [x] 阶段4: 记录到 notes/WORKLOG 并交付

### 状态

**已完成** - 运行时已为所有 hat(含 ralph)注入 `ralph_hat_instance_id`.

---

## 2026-02-11 18:52 +0800 | 任务: 加载 `config/all_hat.md` 并注入所有 hat prompt

### 我正在做什么 & 为什么

- 我正在把 `config/all_hat.md` 作为所有 hat(包括 ralph)共享补充提示内容,在 prompt 构建时统一注入.
- 这样可以让项目级约束在所有角色上保持一致,减少角色间行为偏差.

### 方案方向(两条路)

- 方案A(不惜代价,最佳方案,推荐): 启动时加载 `config/all_hat.md`,串行/并行/decider 三条 prompt 路径统一注入.
  - 优点: 行为一致,覆盖完整.
  - 缺点: 需要改动多个文件与测试.
- 方案B(先能用,后面再优雅): 只在某一条路径(如并行实例)注入.
  - 优点: 改动小.
  - 缺点: 串行和并行行为不一致,后续容易踩坑.

### 决策

- 采用方案A,保证所有 hat 一致生效.

### 阶段

- [ ] 阶段1: 梳理 prompt 注入点与加载时机
- [ ] 阶段2: 实现 `config/all_hat.md` 加载与统一注入
- [ ] 阶段3: 补测试并运行验证
- [ ] 阶段4: 更新 notes/WORKLOG 并交付

### 状态

**目前在阶段1** - 正在梳理注入路径(EventLoop/Parallel/Decider).

### 阶段进展更新(2026-02-11 18:59 +0800)

- [x] 阶段1: 梳理 prompt 注入点与加载时机
- [x] 阶段2: 实现 `config/all_hat.md` 加载与统一注入
- [ ] 阶段3: 补测试并运行验证
- [ ] 阶段4: 更新 notes/WORKLOG 并交付

### 阶段2实现点

- 新增 `prompt_overlay` 模块: 负责加载 `config/all_hat.md` 与统一注入格式.
- EventLoop: 启动时加载 overlay,并在 ralph/非ralph prompt 统一注入.
- ParallelSupervisor + HatInstance + LLM decider: 统一注入 overlay.

### 阶段进展更新(2026-02-11 19:04 +0800)

- [x] 阶段1: 梳理 prompt 注入点与加载时机
- [x] 阶段2: 实现 `config/all_hat.md` 加载与统一注入
- [x] 阶段3: 补测试并运行验证
- [ ] 阶段4: 更新 notes/WORKLOG 并交付

### 阶段3验证结果

- `cargo fmt --check` ✅
- 定向测试 ✅
  - `cargo test -p ralph-core test_ralph_prompt_includes_all_hat_overlay_from_workspace_config`
  - `cargo test -p ralph-core parallel_injects_event_loop_ralph_prompt_only_for_ralph`
- `cargo test`(全量) ✅

### 阶段进展更新(2026-02-11 19:06 +0800)

- [x] 阶段1: 梳理 prompt 注入点与加载时机
- [x] 阶段2: 实现 `config/all_hat.md` 加载与统一注入
- [x] 阶段3: 补测试并运行验证
- [x] 阶段4: 更新 notes/WORKLOG 并交付

### 状态

**已完成** - `config/all_hat.md` 已在加载时注入到所有 hat prompt.

---

## 2026-02-11 19:01 +0800 | 任务: 复核并回答 LOOP_COMPLETE/注入变量问题

### 我正在做什么 & 为什么

- 我正在对你连续提出的 5 个关键点做一次最终代码复核与测试回归.
- 这样可以把答复建立在当前仓库真实实现上,避免口头偏差.

### 阶段

- [x] 阶段1: 读取四文件并确认历史阶段已收敛
- [x] 阶段2: 逐文件核对实现(`event_parser`/`event_loop`/`parallel`/example)
- [x] 阶段3: 执行全量 `cargo test` 回归
- [x] 阶段4: 追加记录并形成交付答复

### 阶段3验证结果

- `cargo test` 全量通过.
- 未出现 error 或失败用例.

### 状态

**已完成** - 可基于当前实现给出确定结论.
