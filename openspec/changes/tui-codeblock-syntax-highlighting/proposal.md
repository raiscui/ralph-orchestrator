## Why

当前 Ralph 的输出（TUI 与 stdout 的 pretty 模式）已经支持 Markdown 的语义样式（例如标题、粗体、引用、行内代码等），但 fenced code block（```lang）内部仍然是“纯色代码”。当 LLM 输出包含较长代码块时，可读性明显下降，用户需要额外复制到编辑器里才能快速定位关键结构与错误点。

同时，我们的输出是“流式追加”的：如果为了语法高亮引入全量重渲染或重型的 ANSI 往返解析，容易直接拖慢流式刷新与长内容滚动体验。因此需要一个“只在必要位置付出成本”的方案：只对 code block 做高亮，其他内容继续走现有 Markdown 渲染路径。

## What Changes

- 在 TUI 输出与 stdout pretty 输出中，为 Markdown fenced code block 提供语法高亮。
- 支持的语言集合（首期）：`rust`、`bash/sh`、`json`、`yaml`、`toml`、`python`、`js/ts`。
- 语法高亮只在 code block **闭合（出现结束 ```）后**触发；未闭合的 code block 仅以“统一 code 样式”显示，避免流式阶段反复高亮导致卡顿与闪烁。
- 保持非 code block 的 Markdown 渲染行为不变（继续由现有渲染器处理标题/列表/引用/粗斜体等语义样式）。
- 通过“分段冻结/缓存”的方式避免在每个流式 chunk 到来时对历史内容做全量重渲染，确保流式刷新与长内容滚动性能稳定。

## Capabilities

### New Capabilities

- `codeblock-syntax-highlighting`: 为流式 Markdown 输出中的 fenced code block 提供语法高亮（覆盖 TUI 与 stdout pretty 输出），并定义：
  - 支持语言集合与别名（如 `sh`/`bash`、`js`/`javascript`、`ts`/`typescript`）
  - 未闭合 code block 的显示策略（不做语法高亮，仅统一 code 样式）
  - 不支持/未知语言的降级策略（退回统一 code 样式）
  - stdout/TUI 两条渲染路径的输出一致性要求

### Modified Capabilities

（无）

## Impact

- 受影响代码区域：
  - `ralph-adapters`：流式输出渲染链路（TUI 与 pretty stdout）。
  - `ralph-cli`：stdout pretty 模式的渲染输出表现（保持行为一致）。
- 依赖影响：
  - 可能引入/扩展语法高亮相关依赖（例如 tree-sitter/syntect 体系），并带来二进制体积与构建时间的变化。
- 性能影响：
  - 增加 code block 闭合时的一次性高亮成本，但通过“只高亮 code block + 缓存冻结”使整体体验更流畅，特别是流式输出与长内容滚动。
