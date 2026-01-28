## Why

当前并行模式（`parallel.enabled: true`）把 `parallel.topic_contracts` 作为硬门槛：

> Parallel mode requires **explicit topic routing contracts** (`parallel.topic_contracts`). There is no implicit broadcast.
>
> —— `README.md`

并且在运行时也会直接拒绝启动：

> parallel.enabled=true 但 parallel.topic_contracts 为空：并行模式要求每个 topic 都能解析到显式 TopicContract。建议先配置一个 "*" 作为兜底（仍属于显式配置）。
>
> —— `crates/ralph-core/src/parallel/supervisor.rs`

这会带来两个问题：

1. 用户需要同时维护两套“订阅语义”：
   - `hats.*.triggers`（人类直觉的事件订阅）
   - `parallel.topic_contracts.*.audience`（并行路由必填）
2. 并行模式的默认行为无法做到“只写 triggers 就能跑”，也不符合我们想要的体验：
   - 同一 topic 的多个订阅者 hat 能真正并发启动（而不是被协调器串行化）。
   - 默认 fanout 到 hats（而不是强制 queue/contract 驱动）。

本 change 的目标是：把并行模式的默认路由语义，收敛为“触发器驱动（trigger-driven）”，让 `topic_contracts` 回归为**可选的显式覆盖**，而不是必填门槛。

## What Changes

- 并行模式新增默认路由：当事件没有被显式 TopicContract 覆盖时，按 `hats.*.triggers` 计算订阅者 hats，并 **fanout 到所有订阅 hats**（粒度=hat，而不是实例）。
- 对每个订阅 hat，**只选择 1 个实例执行**（实例级 queue），不对该 hat 的所有实例 fanout。
- 自动扩缩容（默认开启）：
  - 优先选择空闲实例（Idle/Created）。
  - 若该 hat 的实例都在 Running，则允许动态创建新实例。
  - 全局并发上限默认 **4**（安全刹车，防止进程/成本爆炸）。
  - 动态实例空闲超过 **30s** 自动回收。
  - 实例 key 单调递增且永不复用（方案 A，避免“复活同名实例”歧义）。
- 事件协议扩展（按 Event 字段表达，不编码进 topic 字符串）：
  - 增加 per-event 的 `workspace_strategy` override：`shared | patch | worktree`。
  - 多个事件合并为同一 job 时，采用“最强隔离优先”的合并规则：`worktree > patch > shared`。
- 严格校验与容错：
  - `event.target` / `event.target_instance` 必须是该 topic 的订阅者，否则视为配置/事件错误（warn，并 drop 或 escalate）。
  - 允许少数控制面 topic 走特例路径（例如 gate 类事件），避免把系统控制信号误判为非法投递。
- **BREAKING**：
  - `parallel.enabled: true` 不再强制要求配置 `parallel.topic_contracts`（topic_contracts 从必填门槛调整为可选覆盖）。
  - 并行模式下将存在“触发器驱动的隐式 fanout（到 hats）”，这会改变当前依赖 contracts 的默认行为与文档描述。

## Capabilities

### New Capabilities

- `parallel-trigger-routing`: 在并行运行时提供 trigger-driven 的默认 fanout（到 hats）与实例级 queue（到单实例），并包含 autoscale（max=4、idle=30s）与 workspace override 合并规则。

### Modified Capabilities

<!-- 无（本仓库的 openspec/specs 目前为空；本 change 以新增 capability 方式承载行为变更） -->

## Impact

- crates：
  - `ralph-core`：ParallelSupervisor 路由语义、实例生命周期（动态扩缩容/回收）、job workspace 决策。
  - `ralph-proto`：Event 协议扩展（workspace override 字段）与相关序列化。
  - `ralph-cli`：并行模式的可观测输出（实例创建/回收日志），以及配置校验提示。
- 配置与文档：
  - `README.md`：并行模式不再“必须显式 topic_contracts”，需更新 breaking behavior 说明与示例。
  - `specs/parallel-hat-instances.spec.md`：需要补充/修正“默认路由语义”（从 contracts 强依赖转为 triggers 默认）与 autoscale/workspace override 章节。
- 测试：
  - 新增/更新 replay-based smoke fixture，覆盖“无 topic_contracts 仍可按 triggers 并发运行”的最小闭环。
  - 更新并行 E2E 场景：允许最小配置（只写 hats.triggers），并验证 fanout 到多个 hats 的并发启动与事件落盘。
