## Why

目前 Ralph 的 TUI 输出视图已经能 best-effort 渲染 Markdown（主要提升结构化文本的可读性）。
但整体仍然是“等宽小字号文本”的观感，与 `mdfried` 的 Big Headers / 图片预览差距很大。
当输出内容偏“规格/方案/长文档”时，阅读与扫读效率仍然不够好。

你明确选择了 **B：要尽可能接近 `mdfried` 的效果**。
因此本次变更的核心目标是：在 TUI 里引入“图形协议 + 图片渲染”，让 Markdown 标题能“更大”，并为图片内联预留能力。

## What Changes

- 在 Supervisor TUI 的输出视图中，引入接近 `mdfried` 的“deep fry”渲染效果：
  - Markdown 标题（至少 H1/H2/H3）在支持图形协议的终端中以“更大”的形式呈现（图片或缩放文本）。
  - 可选：支持 Markdown 图片语法 `![]()` 的内联预览（带安全降级与大小限制）。
- 保持稳定降级：
  - 当终端不支持图形协议时，必须自动降级为纯文本/现有 Markdown 渲染（不崩溃、可读）。
- 不改变现有并行 runtime 的关键语义：
  - event parsing 仍坚持 stdout-only（避免 stderr 回显造成假事件）。
  - 并行 supervisor 的收敛语义与 routing 逻辑不受影响。

## Capabilities

### New Capabilities
- （无）

### Modified Capabilities
- `parallel-supervisor-tui`: 输出视图的渲染能力升级为“文本 + 图片块”，并新增 Big Headers /（可选）图片内联的规格要求与降级规则。

## Impact

- 受影响代码：
  - `crates/ralph-tui`：输出视图渲染模型将从“纯文本行（Line/Span）”扩展为“文本块 + 图片块”的混合渲染。
  - `crates/ralph-adapters`：Markdown 解析仍可复用，但需要在渲染层提供“标题/图片块”的结构化输出。
  - `crates/ralph-cli`：如需新增开关/配置（例如禁用图片渲染），需要接线并保持默认行为合理。
- 受影响依赖：
  - 预期新增 `ratatui-image`（以及其协议探测/渲染链路）。
  - 可能新增 `image`/字体栅格化相关依赖；如启用 Chafa fallback，可能引入动态/静态链接策略取舍。
- 风险与注意事项：
  - 图片渲染可能阻塞 UI（resize/encode），需要缓存或后台线程/任务。
  - “图片不可复制”会影响框选复制的语义，需要明确 UI 行为（例如复制 alt 文本或跳过）。

