## Context

在 `tui-markdown-renderer-mdfried` 这个 change 里，我们完成了“Markdown → ratatui::Line”的渲染链路替换：
用 `mdfrier` 做语义解析与换行，然后在 TUI 里以 `Line/Span` 形式展示。

但 `mdfried` 的核心卖点并不在“语义样式”本身，而在更强的视觉层级：

- **Big Headers™**：标题不是普通字号，而是更大（图片或缩放文本）
- **图片内联**：`![]()` 可以在终端里直接预览

这两点都需要“终端图形协议”能力（Kitty / iTerm2 / Sixel 等）或可靠的降级方案（halfblocks / Chafa）。
因此要实现你选的方案 B（最接近 mdfried），本质上必须引入图片渲染层，而不是继续只在 `mdfrier` 上调颜色。

当前 Ralph 的关键约束：

- 并行 runtime 的事件解析必须保持 **stdout-only**（stderr 可能包含 prompt 回显或示例 `<event>`，会造成假事件）
- TUI 的输出视图目前是“纯文本行”的模型（`IterationBuffer` + `ContentPane`），包含滚动/搜索/框选复制等语义

## Goals / Non-Goals

**Goals:**
- 在 **Supervisor TUI 的输出视图**中支持接近 `mdfried` 的 Big Headers：
  - 至少支持 H1/H2/H3（更大、更醒目）
  - 支持图形协议时尽可能用图片/缩放文本呈现
  - 不支持图形协议时稳定降级为现有 Markdown 文本渲染
- 为图片内联预留能力，并以“可控、安全”为前提逐步开放：
  - 支持 `![]()` 的结构识别
  - 在明确开关/配置允许的前提下，才做下载/缓存/渲染
- 性能与稳定性：
  - 不允许因为图片 resize/encode 阻塞而导致 TUI 卡顿
  - 必须有缓存（避免每帧重复 encode）

**Non-Goals:**
- 不追求完全复刻 `mdfried` 的所有交互（例如字体选择 UI、打开链接、全屏 viewer 的导航体验）。
- 不在本次 change 里把“所有 TUI 输出”都升级为图片渲染。
  - 先聚焦并行 Supervisor 的 Output 面板，后续再扩展到串行。
- 不改变并行 supervisor 的 routing / completion 语义。

## Decisions

1) **依赖选择：采用 `ratatui-image` 作为图形协议统一层**
- 决策：引入 `ratatui-image`，用于：
  - 探测终端支持的图形协议（Kitty / iTerm2 / Sixel）
  - 探测字体像素尺寸（用于 cell ↔ pixel 映射）
  - 在 ratatui 里渲染图片（Image/StatefulImage）
- 理由：`mdfried` 自身即基于它，能力边界与兼容矩阵更贴近你要的效果。
- 备选：自己实现 iTerm2/Kitty/Sixel 协议（成本高，且易引入终端兼容坑）。

2) **渲染模型升级：从“纯 Line 列表”升级为“富块（Text/Image）列表”**
- 决策：为输出视图引入中间表示（例如 `OutputBlock`）：
  - `Text(Vec<Line>)`
  - `Image(ImageBlock)`（包含协议状态/尺寸/占位高度/降级文本）
- 理由：
  - Big Header/图片天然不是 1 行文本，必须占用多个 cell 行；
  - 继续强行塞进 `Line` 会导致滚动/布局/覆盖关系不可控。
- 影响：`ContentPane` 需要重构为“按块布局 + 渲染”，并且滚动单位从“行”扩展为“块内行”。

3) **Big Headers 的实现路径：优先图片渲染，必要时降级**
- 决策：先把 H1/H2/H3 标题渲染为图片块（使用 `ratatui-image` 的协议/尺寸映射）。
- 对齐 `mdfried` 的经验：
  - 标题渲染依赖字体栅格化（`mdfried` 使用 `cosmic-text` + `image`）。
  - 本项目优先复用同类方案，避免“选字体 UI”之前就出现明显错位。
- 降级策略：
  - 协议探测失败 / 字体像素尺寸不可得 / 渲染失败 → 回退到现有 Markdown 文本渲染（`mdfrier`）。

4) **图片内联：默认关闭，显式开启才下载/渲染**
- 决策：`![]()` 的图片下载/渲染默认关闭。
- 理由（安全 + 可控）：
  - AI 输出可能包含不可信 URL；
  - 下载/解码图片可能引入性能与安全风险；
  - 需要明确缓存目录、大小上限、协议/格式白名单策略。
- 计划：在 tasks 里拆成独立阶段（先识别并占位，再做下载/缓存/渲染）。

5) **性能：避免在 UI 线程做重计算**
- 决策：图片 resize/encode 必须被缓存，且必要时使用后台线程/任务。
- 参考 `ratatui-image` 的建议：
  - `StatefulImage` 的 resize/encode 可能阻塞；
  - 推荐 thread/tokio offload（`examples/thread.rs` / `examples/tokio.rs`）。
- 本项目倾向：
  - 对“大标题”使用缓存（同一个标题在同一宽度下只 encode 一次）
  - 宽度变化时才重新 encode

6) **stderr 不再污染 Markdown 内容（流标识与内容分离）**
- 背景：当前并行 TUI 会把 stderr 行拼接成 `"[stderr] {line}"` 再渲染。
  - 这会破坏 Markdown 的“行首语义”（例如 `#`/`>`/`-` 必须出现在行首才生效）。
  - 这也是你截图里“看起来不像 mdfried”的直接原因之一。
- 决策：
  - 输出流（stdout/stderr）属于“元信息”，不再写进 Markdown 内容里。
  - UI 上仍然要让用户看得出这是 stderr，但要用独立的 UI 前缀/列/状态标识来呈现，
    不能影响 Markdown 解析输入。
- 额外约束：
  - 这不改变 event parsing 的 stdout-only 语义；
  - 仅影响 TUI 展示层的“如何标识 stream”。

## Risks / Trade-offs

- [Risk] UI 复杂度显著提升（滚动/选择/复制语义被影响）
  - Mitigation：分阶段交付：
    - 第一步只做 Big Headers（且只在 stdout Rendered 模式）
    - 图片内联放到后续任务，并明确默认关闭

- [Risk] 不同终端对图形协议支持差异很大
  - Mitigation：把“探测 + 降级”写成一等逻辑：
    - 探测失败就自动回到纯文本渲染
    - 允许用户强制禁用图片渲染（排障/兼容）

- [Risk] 图片不可复制，影响“框选复制”体验
  - Mitigation：定义复制语义：
    - 复制时跳过图片块或复制其 alt 文本/链接
    - 在 specs 里明确行为，避免“看起来随机”

- [Risk] 依赖与二进制体积增长
  - Mitigation：通过 features 控制：
    - 优先不启用 Chafa 依赖（先用 halfblocks fallback）
    - 图片内联下载依赖（如 `reqwest`）仅在启用图片功能时引入
