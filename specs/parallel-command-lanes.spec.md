# Spec: Parallel Command Lanes(lane + generation + draining)

## 背景

Ralph 的并行运行时(Parallel Supervisor + HatInstance)会在同一进程内并发跑多个 HatInstance job.
其中有一类动作不属于 LLM agent 的“业务工作”,而是 orchestrator 自己必须执行的“副作用动作”,典型是:

- workspace=worktree: `git worktree add/remove`(以及必要的清理)
- workspace=clone: `git clone` 与目录清理
- workspace hooks: `on_acquire` / `on_release`(由 orchestrator 执行,否则无法在正确时机保证运行)

这些动作在并行场景下有两个高概率风险:

1. 并发 flaky:
   - 多个实例并发执行 `git worktree add/remove` 时,容易出现 git 锁/竞态,导致偶发失败.
2. 收尾不确定:
   - early-exit/shutdown 时,如果直接丢弃 in-flight 动作,会留下 worktree 目录或 git worktree 记录,污染后续运行.

openclaw 在网关层引入了一个很小的 in-process `command-queue` 抽象,核心思路是:

- lane: 把不同风险/交互面的任务隔离,允许低风险并行,并避免高风险互相踩踏.
- generation: reset/early-exit 时 bump generation,忽略 stale completion,避免队列永久卡死.
- draining: 明确拒绝新任务,避免“排队了但进程退出时被静默 kill”.

本 spec 目标是把这个思路以“最小 Rust 版本”落地到 Ralph 的 parallel runtime 里,优先解决 workspace/git 的并发与收尾问题.

## 目标

- 提供一个最小 `CommandQueue`(in-process)用于 parallel runtime 的副作用动作排队:
  - 支持 lanes.
  - 支持 generation(reset 后忽略旧 permit 的 release).
  - 支持 draining(拒绝新 acquire).
  - 支持 clear lane(拒绝排队中的 waiter).
- 把 `git worktree add/remove` 串行化:
  - 同一时刻最多允许 1 个实例执行 worktree 的 add/remove(避免 git 锁冲突).
- 改良 HatInstance shutdown 收尾确定性:
  - `Shutdown` 进入 draining: 停止启动新 job,取消当前 job(best-effort).
  - 在退出前 MUST best-effort 释放已 acquire 的 worktree.
  - shutdown 收尾 MUST 跳过 workspace hooks(避免长时间 drain).

## 非目标

- 不做“跨进程持久化队列”(不写入磁盘,不做恢复).
- 不引入新的用户配置项(先用内置默认 lane 与并发度).
- 不改变 LLM job 的并行调度语义(只治理 orchestrator 自己做的副作用动作).

## 设计

### 1) CommandQueue: acquire-based lanes

`CommandQueue` 是一个 in-process 的最小队列,提供:

- `acquire(lane) -> Permit`:
  - 当 lane 未饱和时立即返回.
  - 当 lane 饱和时排队等待.
- `set_lane_concurrency(lane, max_concurrent)`:
  - 默认 `max_concurrent=1`.
- `mark_draining()`:
  - 后续新 `acquire()` MUST 失败,返回专用错误(用于“明确拒绝新任务”).
- `clear_lane(lane)`:
  - 拒绝 lane 中仍在排队的 waiter,返回专用错误(用于“明确取消排队中的工作”).
- `reset_all_lanes()`:
  - bump generation + 清空 active.
  - 旧 generation 的 permit drop/release MUST 被忽略,避免 stale completion 影响新一代计数.

### 2) workspace.git lane

在 parallel runtime 中引入内置 lane:

- `workspace.git`:
  - 用于所有会修改主仓库 git 元数据的操作,至少包含:
    - `git worktree add`
    - `git worktree remove`
    - (可选) acquire 前的残留 workdir cleanup(remove worktree record)
  - 默认 `max_concurrent=1`.

### 3) HatInstance shutdown draining

HatInstance 收到 `HatInstanceCommand::Shutdown` 时:

- MUST 进入 draining(内部状态即可,无需新增外部 state 枚举).
- draining 状态下:
  - MUST 不再启动新 job(丢弃 pending 事件队列).
  - MUST cancel 当前 job(best-effort).
  - 若已 acquire worktree,退出前 MUST best-effort release.
    - release MUST 跳过 hooks(即使配置了 hooks,shutdown 仍不执行),保证退出可控.
    - `WorktreeBackend::Clone` 在 shutdown 收尾时 SHOULD 只做目录清理,不做 clone HEAD import(避免触碰主仓库 refs,并降低 drain 时长).

## 验收标准

### 单元测试: CommandQueue

- 同 lane 默认串行:
  - 并发启动 2 个 `acquire("workspace.git")` 的任务时,peak concurrency MUST 为 1.
- `mark_draining`:
  - 调用后新 `acquire()` MUST 立刻失败,且错误类型可区分.
- `clear_lane`:
  - 调用后仍在排队的 waiter MUST 被拒绝,且错误类型可区分.
- generation reset:
  - `reset_all_lanes()` 后,旧 permit 的 release 不应影响新一代 lane 的 active 计数(不应导致负数/卡死).

### 回归: workspace git 串行化

- 在 `WorktreeBackend::Worktree` 下,worktree acquire/release 的 git 操作 MUST 在 `workspace.git` lane 内执行.
  - 该点通过单元测试覆盖(观察 lane 的串行特性),并通过代码结构保证(调用点集中在 instance.rs 的 worktree 方法中).
