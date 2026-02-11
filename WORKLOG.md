## 2026-02-07 13:34 +0800 | WORKLOG 续档

- 旧文件因超过 1000 行已归档为 `WORKLOG_$(date '+%Y-%m-%d_%H%M').md`。
- 新日志从本文件继续追加。

## 2026-02-07 13:33 +0800 | 按用户要求: 默认 `ralph hats graph` 也改为拆分多边

- 用户要求: 不要 `experiment.complete / +3 more`,不要合并,要拆分多边。
- 实施:
  - 移除 physical view 下 TerminalPretty 的 Ralph 边折叠逻辑。
  - 统一 Strict/TerminalPretty 均输出一 topic 一条边。
  - 删除摘要函数与相关测试,并改写 TerminalPretty 回归测试为“必须拆分多边”。
- 验证:
  - 定向测试通过(Strict/TerminalPretty 两条)。
  - `cargo build --release` 通过。
  - `cargo test` 全量通过。
  - `cargo fmt --check` 通过。

## 2026-02-08 15:24 +0800 | git 提交: hats graph 默认拆分 Ralph 多 topic 边

- 变更点:
  - hats graph(physical view): Strict/TerminalPretty 均保持一 topic 一条边,不再折叠 Ralph 多 topic.
  - AsciiRenderOptions: 统一构造入口,用 Default + 覆盖的方式消除 clippy needless_update,并补回归测试.
  - WORKLOG: 超 1000 行续档,旧文件已归档为 WORKLOG_2026-02-07_1552.md.
- 验证:
  - cargo fmt --check ✅
  - cargo clippy --all-targets --all-features -- -D warnings ✅
  - cargo test ✅

## 2026-02-11 00:26 +0800 | git 提交: 归档四文件历史版本 + 更新 example PROMPT

- 变更点:
  - 将根目录历史四文件版本文件移动到 `archive/`(git rename),降低根目录噪音,便于后续检索.
  - `task_plan.md` 已按"超过 1000 行续档"规则重新开始,旧版本存档为 `archive/task_plan_2026-02-11_001538.md`.
  - `examples/parallel-experimental-dev-engine/PROMPT.md` 从 TODO 模板更新为一份具体的实验目标与约束示例.
- 验证:
  - `cargo fmt --check` ✅
  - `cargo clippy --all-targets --all-features -- -D warnings` ✅
  - `cargo test` ✅
- 提交:
  - `2b9e508 chore: archive four-file history and update example prompt`
- 备注:
  - 当前仍存在未跟踪文件:`examples/parallel-experimental-dev-engine/PROMPT copy.md`(本次未提交).

## 2026-02-11 11:54 +0800 | 修补: parallel worktree sandbox commit + 整理 PROMPT 模板

- 背景:
  - 你反馈在 `parallel-experimental-dev-engine` 的 worktree 中 `git commit` 会因为 sandbox 权限失败.
  - 同时你希望 runner 真正并行跑起来,而不是把多个实验塞到一个 event 里导致串行.

- 实施:
  - 新增 `parallel.workspace.worktree_backend`:
    - `worktree`: 继续用 `git worktree`.
    - `clone`: 用 `git clone --no-hardlinks` 创建独立 `.git`,并在回收前把 clone 的 HEAD fetch 进主仓库(refs/ralph/workspaces/...).
  - example 同步:
    - `examples/parallel-experimental-dev-engine/PROMPT.md` 恢复为可复用模板.
    - `examples/parallel-experimental-dev-engine/ralph.yml` 默认启用 `worktree_backend: clone`.
    - `event_loop.ralph_prompt` 增强: 明确 1 个 `experiment.task` 只能包含 1 个实验;批次派发必须输出多个 `<event ...>` block.
    - `examples/parallel-experimental-dev-engine/README.md` 补充 `worktree_backend` 的取舍与切换方法.

- 验证:
  - `cargo fmt` ✅
  - `cargo clippy --all-targets --all-features -- -D warnings` ✅
  - `cargo test` ✅

- 提交:
  - `6bae384 fix(parallel): add clone worktree backend for sandboxed runners`

## 2026-02-11 16:11 +0800 | 修正示例协议: 完成总结先于 LOOP_COMPLETE + 禁止 experiment.start 空转

- 变更背景:
  - 用户指出 `parallel-experimental-dev-engine` 示例应满足:
    - 发 `LOOP_COMPLETE` 前必须有完成总结.
    - 不要发没有接收器的 `experiment.start`.

- 实施内容:
  - `examples/parallel-experimental-dev-engine/ralph.yml`
    - 改写 Auto-Plan 触发口径: 从 `task.start` payload 直接解析计划.
    - 入口处理改为首发 `experiment.task`，并明确禁止发布 `experiment.start` 这类无人接收 topic.
    - 收敛规则改为“先完成总结,后单独一行 LOOP_COMPLETE”。
  - `examples/parallel-experimental-dev-engine/README.md`
    - 核心 topic 与成功标准同步更新上述规则.

- 验证:
  - `cargo fmt --check` ✅
  - `cargo test -p ralph-core smoke_runner` ✅

## 2026-02-11 16:39 +0800 | 新增统一运行时身份字段: `ralph_hat_instance_id`

- 背景:
  - 需要让所有 hat(含 ralph)在 prompt 中可识别自己的运行时身份.

- 实施:
  - 串行 EventLoop:
    - `build_prompt`(ralph/非ralph)统一注入 `ralph_hat_instance_id:"<hat_id>"`.
    - `build_ralph_prompt` 注入 `ralph_hat_instance_id:"ralph"`.
  - 并行 Instance:
    - `HatInstanceActor::build_prompt` 注入 `ralph_hat_instance_id:"<instance_id>"`.
  - 并行 LLM decider:
    - dispatch decider prompt 注入 `ralph_hat_instance_id:"ralph#decider-<job_id>"`.

- 测试:
  - 更新 event_loop/event_loop_ralph/parallel routing 相关断言.
  - 全量 `cargo test` 通过.

## 2026-02-11 19:06 +0800 | 功能: `config/all_hat.md` 统一注入所有 hat prompt

- 背景:
  - 需要把项目级共享提示(位于 `config/all_hat.md`)注入给所有 hat(含 ralph),并在运行时统一生效.

- 实施:
  - 新增 `prompt_overlay` 模块:
    - 负责加载 `${workspace_root}/config/all_hat.md`.
    - 负责把内容以固定标题段落注入 prompt.
  - EventLoop:
    - 启动时一次加载 overlay,缓存到 `all_hat_prompt`.
    - `build_prompt` 与 `build_ralph_prompt` 统一注入.
  - Parallel:
    - Supervisor 启动时加载 overlay 并下发到所有实例.
    - Instance prompt 与 dispatch decider prompt 都会注入 overlay.

- 测试:
  - 新增 EventLoop overlay 注入测试.
  - 扩展并行 routing 测试,确认 `ralph#1` 与普通 hat 均收到 overlay.

- 验证:
  - `cargo fmt --check` ✅
  - `cargo test` ✅

## 2026-02-11 19:01 +0800 | 复核交付: LOOP_COMPLETE 规则 + prompt 注入范围确认

- 复核内容:
  - `LOOP_COMPLETE` 检测规则是否要求“独占整行/结尾”.
  - `parallel-experimental-dev-engine` 是否已强制“先总结再 LOOP_COMPLETE”且禁止 `experiment.start`.
  - `ralph_hat_instance_id` 是否已覆盖所有 hat(含 ralph).
  - 是否存在“注入给所有 hat 但不包括 ralph”的变量.
  - `config/all_hat.md` 是否已注入全部路径.

- 结论:
  - `LOOP_COMPLETE` 当前是“event block 外的子串匹配”,非“整行唯一匹配”.
  - example 已写死“先完成总结,最后一行 LOOP_COMPLETE”,且禁止无人接收 topic(`experiment.start`).
  - `ralph_hat_instance_id` 已覆盖串行/并行/decider,并包含 ralph.
  - 不存在“全 hat 但排除 ralph”的通用注入变量.
  - `config/all_hat.md` 已注入 EventLoop + Parallel Instance + Parallel Decider.

- 验证:
  - `cargo test` 全量通过.
