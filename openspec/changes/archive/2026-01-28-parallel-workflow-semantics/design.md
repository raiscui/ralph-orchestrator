## Context

当前 Ralph 在“启动 / 委派 / 收敛退出 / orphan 兜底”这几个概念上，存在 **实现、文档、示例** 三者语义不一致的问题：

- runtime 实际启动时，串行与并行都会先发布 `task.start`（或 resume 时 `task.resume`）。
- `event_loop.starting_event` 在代码里定义为“Ralph 协调完成后发布的 workflow entry event”，但部分文档/示例叙事容易把它误解成“第一条事件”。
- parallel 的默认 trigger 路由目前会在“没有 specific 订阅者”时把 ralph#1 作为 fallback 一并加入，这会导致：
  - 有经理（wildcard）时也会打扰老板（ralph#1）
  - 当 hats 数量规模很大时，ralph#1 负载与注意力会被大量非关键事件稀释
- parallel 的退出条件目前是“只认 ralph#1 输出 completion_promise”，worker hat 输出不会终止 run。
  - 这本身是一个合理的“强约束协调语义”，但如果 completion 逻辑只写在 demo prompt 里，就会让使用者困惑。

这次 change 的目标，是把上述语义固化为“单一权威定义”，并让默认推荐拓扑更符合你选择的链式模式：**老板兜底、不过度过问**。

## Goals / Non-Goals

**Goals:**
- 明确区分并固化 3 个概念：
  1. runtime handshake start event：`task.start` / `task.resume`
  2. workflow entry event：`event_loop.starting_event`（可选；协调后发布）
  3. workflow completion：由 ralph#1 输出 `event_loop.completion_promise` 结束
- 新增 `event_loop.complete_publishes`（唯一 topic）作为 workflow 级“完成候选事件”声明，把“什么时候该结束”从 demo prompt 迁移到 config+spec。
- 收敛 orphan 边界：仅当事件 **无任何接受者** 时才升级给 ralph#1（链式拓扑的老板兜底语义）。
- 对齐 docs/examples：把 `starting_event` 的“官方定义”写清楚，避免再把它描述成“first event published”。

**Non-Goals:**
- 不在本 change 内引入新的“平台化 orchestrator 功能”（例如复杂的重试 DSL、全自动 manager 层级生成等）。
- 不把退出条件改成“任意 worker 输出 completion_promise 都能退出”（会破坏并行的强约束收敛语义）。
- 不在本 change 内重做 TUI/可观测性体系（已有 spec 覆盖 `parallel-supervisor-tui`）。

## Decisions

### Decision 1: 明确 runtime start 与 workflow entry 的分层

**选择：**
- runtime 的第一条事件固定为 `task.start` / `task.resume`。
- `event_loop.starting_event` 定义为“Ralph 完成初始协调后发布的 workflow entry event topic”，不是 runtime 的第一条事件。
- 在 parallel 模式下，`task.start`/`task.resume` 作为控制面 topic，必须路由到 `ralph#1`，避免 top-level prompt 污染其他 hats。

**理由：**
- 与现有实现一致（串行/并行都先 publish `task.start`）。
- 保持 “top-level prompt 只影响协调者” 的原则，避免角色污染导致 worker 偏离职责。

**备选方案：**
- 把 `starting_event` 解释成“第一条事件”并让 Supervisor 直接发布它。
  - 放弃原因：与现有实现/代码注释相悖，并且会让 `task.start` 的 objective 注入缺位或重复。

### Decision 2: 引入 event_loop.complete_publishes 作为 workflow 级完成候选事件

**选择：**
- 新增 `event_loop.complete_publishes: "<topic>"`（唯一、可选）。
- 该 topic 表达“workflow 已经到达可收敛点”的业务信号，但 **是否结束** 仍由 ralph#1 裁决，最终以 ralph#1 输出 `event_loop.completion_promise` 为准。

**理由：**
- 把“结束语义”从 demo prompt 里搬到 config+spec，减少使用者困惑。
- 保持并行模式的强约束：只有 ralph#1 可以输出 completion_promise，避免多 hat 并发下出现不一致退出。

**备选方案：**
- 仅靠 `completion_promise` 字符串（完全不引入 completion topic）。
  - 放弃原因：用户侧只能把“什么时候结束”写进 prompt/demo，很难成为“官方语义”。
- 让 Supervisor 直接把 completion topic 作为机械退出条件（看到就停）。
  - 放弃原因：你明确希望“由 agent 决定”，而不是机械化程序直接结束。

### Decision 3: Orphan 兜底语义收敛为“真 orphan 才找老板”（链式拓扑）

**选择：**
- triggers 默认路由下：
  - 有 specific subscriber → 只发给 specific
  - 无 specific 但有 wildcard subscriber → 只发给 wildcard（例如经理）
  - 没有任何 subscriber → 才视为 orphan，升级给 ralph#1
- 这是一次 **BREAKING** 行为调整：不再在“存在 wildcard subscriber”的情况下额外同时投递给 ralph#1。

**理由：**
- 你明确选择“链式、老板兜底、不什么都过问”的长期方向。
- 当 hats 数量增至 100+ 时，这个边界是 ralph#1 不被打爆的必要条件。

**备选方案：**
- 保持现状：无 specific 时把 ralph#1 永远加入 fallback。
  - 放弃原因：老板会被大量非关键事件持续打断，链式拓扑无法成立。

### Decision 4: 控制面 topic 命名约定使用 ralph.*

**选择：**
- 允许并推荐使用 `ralph.*` 作为控制面 topic 前缀（例如 orphan 升级、workflow 控制、内部可观测性事件）。

**理由：**
- 清晰区分“业务事件”和“控制面事件”，便于路由、文档、测试与运维排查。

## Risks / Trade-offs

- [Risk] 现有并行 workflow 可能依赖“ralph 总能看到 fallback 事件”来做收敛 → Mitigation：
  - 对需要强制老板介入的 topic，使用显式 TopicContract（或让经理显式 publish `ralph.*` 升级事件）。
- [Risk] `event_loop.complete_publishes` 的 completion candidate 事件可能因被其他 hats 订阅而不再是 orphan，导致 ralph#1 看不到 → Mitigation：
  - 在文档与 preset 中约定：completion topic 应尽量保持“终端信号语义”，避免被非协调者消费；需要观察可用 logger hat，但不要改变路由边界。
  - 必要时用 TopicContract 显式把 completion topic 也投递给 ralph#1。
- [Trade-off] 更强的语义约束意味着更少“隐式魔法”与更高可预测性，但也意味着部分 demo/prompt 需要更新以匹配新规则。

## Migration Plan

1. 先落地 spec 与 docs（本 change 的输出），把术语和语义统一。
2. 代码实现阶段按顺序推进：
   - 配置层：增加 `event_loop.complete_publishes` 字段与解析/校验。
   - parallel 路由层：调整 triggers fallback/orphan 计算逻辑，符合“真 orphan 才升级”。
   - parallel 启动：确保 `task.start/task.resume` 作为控制面 topic 投递给 `ralph#1`。
   - ralph#1 指令：在并行模式下把 `starting_event` / `complete_publishes` 的约束写进协调者 prompt（对齐 HatlessRalph 的可预测语义）。
   - docs/examples：修正文档表述，并更新示例说明“prompt.md 是 demo，不是 runtime 依赖”。
3. 用 replay smoke tests 固化行为，避免语义回退。

## Open Questions

- `event_loop.complete_publishes` 是否需要在 runtime 层做“强保证可达 ralph#1”（例如自动合成 TopicContract），还是仅通过“真 orphan 才升级 + completion topic 不被订阅”的约定来保证？
- `ralph.*` 是否需要加入 control-plane allowlist（类似 gate.*）以绕过 strict target 校验？
- 并行模式下 ralph#1 的 prompt 结构要对齐 HatlessRalph 到什么程度（完整复用 builder vs 只对齐关键段落）？
