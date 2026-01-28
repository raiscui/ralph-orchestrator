## Why

当前并行模式（`parallel.enabled=true`）已经能“真并发”跑多个 HatInstance，但上层交互仍停留在 **stdout 日志**。
这导致几个核心痛点：

- 无法像串行模式 TUI 那样“快速定位某个实例正在做什么、看历史输出、搜索”。
- human async chat 与 gate 目前缺少一个**统一的、可交互的面板**，人类介入成本高。
- spec 里已经写了 `Supervisor TUI（实例列表 + 实例详情 + human async chat + gate）`，但还没有工程落地，形成“文档语义 vs 实际体验”的断层。

因此需要把 Supervisor TUI 做成并行模式的一等入口，让并发的可观测性与 human-in-the-loop 能力真正可用。

## What Changes

- 并行模式新增 **Supervisor TUI**：
  - 左侧：HatInstance 列表（状态/最近事件/最后输出时间/工作区策略等）。
  - 右侧：实例详情输出（按 job 分段，支持滚动/搜索/跳转/查看历史）。
  - 底部：human async chat + gate 面板（展示待处理 gate、支持快捷指令批准/拒绝/回复）。
- 并行 runner 不再在 `--tui` 下输出 “no TUI” 警告，而是启动真实 TUI（在 TTY 环境下）。
- 事件与回放约束保持不变：TUI 只做“展示 + 交互输入的事件生成”，不破坏并行调度的可回放性与 backpressure 语义。

## Capabilities

### New Capabilities

- `parallel-supervisor-tui`: 并行模式 Supervisor TUI 的端到端交互规范（实例列表/实例详情/job 历史/搜索/滚动/快捷键）。
- `supervisor-human-chat-gate`: human async chat + gate 面板的交互协议（定向消息、gate 展示/倒计时、approve/deny/resolve 指令与事件落盘）。

### Modified Capabilities

<!-- 无 -->

## Impact

- crates：
  - `crates/ralph-cli`：并行运行器启用 TUI 的入口与生命周期管理。
  - `crates/ralph-tui`：需要扩展 state/widgets/input，支持并行的 instance/job 维度 + chat/gate pane。
  - `crates/ralph-core`/`crates/ralph-proto`：可能需要补齐少量“UI 友好”的事件字段或观察接口（以最小改动为原则）。
- 测试：
  - 需要增加 TUI 渲染验证（用 `/tui-validate` 做主观 UI 回归的二值背压）。
  - 需要补充并行模式下 human chat / gate 的 replay fixtures（确保回放确定性）。
