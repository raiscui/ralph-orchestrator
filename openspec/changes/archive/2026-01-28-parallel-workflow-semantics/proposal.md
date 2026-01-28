## Why

目前 `starting_event`、并行启动（`task.start`/`task.resume`）、并行结束条件（`completion_promise`）以及“orphan 事件兜底给谁”之间的语义在 **实现 / 文档 / 示例** 里不一致。
这会让并行模式看起来像“靠 demo prompt 才能闭环”，造成团队理解分裂与使用困惑；同时也阻碍我们把 parallel 拓展到大量 hats 的链式拓扑（老板兜底、不过度打扰）。

## What Changes

- 明确定义 **workflow 启动语义**：
  - runtime 的第一条事件永远是 `task.start`（或 resume 时 `task.resume`）。
  - `event_loop.starting_event` 的含义是“协调后 workflow entry event（可选）”，而不是“第一条事件”。
- 引入/固化 **workflow 级完成信号**：在 `ralph.yml` 的 `event_loop` 下增加一个唯一配置项 `complete_publishes: "<topic>"`，用来声明该工作流“最终完成事件 topic”。
- 明确定义 **并行结束语义**：
  - 并行 runtime 的硬退出仍以 `event_loop.completion_promise` 为准（Supervisor 只认 ralph#1 输出的 promise）。
  - `event_loop.complete_publishes` 作为“完成事件 topic”，用于驱动 ralph#1 在看到完成条件后决定是否输出 `completion_promise`（例如 `LOOP_COMPLETE`），从而结束 run。
- **BREAKING**：收敛 “orphan 兜底” 的边界为“真 orphan 才升级给老板”：
  - 只有当某事件在路由计算后 **没有任何接受者** 时，才会升级到 ralph#1（或未来的经理链路）。
  - 若已经存在可接收者（例如经理 hat / wildcard hat），则不再额外把同一事件同时送给 ralph#1。
- 约定控制面 topic 命名：允许使用统一前缀 `ralph.*`（例如兜底升级、工作流控制、可观测性事件），并把其语义写入规格与文档。
- 对齐文档与示例叙事：`examples/parallel-trigger-routing` 将“目标 prompt”内联在 `event_loop.prompt`，entry/exit 语义用 `starting_event/complete_publishes` 表达，避免让示例看起来像“必须靠额外 prompt 文件才能闭环”。

## Capabilities

### New Capabilities
- (none)

### Modified Capabilities
- `parallel-hat-instances`: 补齐 workflow entry/exit 的官方语义（`task.start` vs `starting_event`、`complete_publishes`、并行结束只由 ralph#1 决定等）。
- `parallel-trigger-routing`: 调整/明确 orphan 与 fallback 的边界（真 orphan 才升级给 ralph#1），并补充 `ralph.*` 控制面 topic 的约定。

## Impact

- 配置与解析：
  - `ralph.yml` schema/解析需要新增 `complete_publishes`（并明确其作用域与默认行为）。
  - `starting_event` 的文档定义需要修正为“协调后 entry event”。
- 并行路由语义（行为变更）：
  - triggers 默认路由下的 fallback/orphan 处理需要对齐“链式拓扑 + 老板兜底”的目标。
  - 完成事件（`complete_publishes`）的可达性需要保证 ralph#1 能做出最终收敛决策。
- 文档与示例同步：
  - 修正文档中把 `starting_event` 描述成 “first event published” 的表述。
  - 在示例中明确区分“demo prompt 驱动的确定性闭环”和“runtime 语义”。
- 测试：
  - 增加/更新 replay smoke tests，覆盖“有经理订阅者时不打扰 ralph#1”、“无订阅者时升级给 ralph#1”、“完成事件触发收敛”的行为。
