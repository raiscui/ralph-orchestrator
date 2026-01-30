## Why

当前 Supervisor TUI 的“实例输出/详情区”主要展示 AI code agent 的终端输出。
这些输出里经常包含 Markdown（例如 `#` / `##` 标题、`>` 引用、代码块等）。
不渲染时可读性较差，关键信息不突出，影响快速审阅与排障效率。
但在某些场景下（排障、复制粘贴、与旧行为对齐），用户也需要一键回退到纯文本输出。

## What Changes

- Supervisor TUI 的“实例输出/详情区”默认对 AI code agent 输出进行 Markdown 渲染，至少覆盖：`h1/h2` 标题、`> blockquote`、代码块（含 fenced code block/inline code）。
- 对其它无法完全枚举的 Markdown 结构，尽可能渲染；当遇到不支持/不完整/解析失败时，必须安全降级为原始文本显示，且不得丢失内容或导致崩溃。
- 新增命令行参数 `--plain`：强制关闭 Markdown 渲染，按原始纯文本展示（用于排障与兼容场景）。
- 不改变现有 TUI 启用/降级逻辑（TTY 检测、`--no-tui` 等行为保持不变）。

## Capabilities

### New Capabilities

- (none)

### Modified Capabilities

- `parallel-supervisor-tui`: 实例输出视图默认渲染 Markdown；新增 `--plain` 禁用渲染并回退原始输出；渲染失败需安全降级，避免崩溃与内容丢失。

## Impact

- 受影响代码主要集中在：TUI 输出展示组件（渲染管线/样式策略），以及 CLI 参数解析与接线（新增 `--plain`）。
- 依赖与实现路径需要在 design 阶段明确：直接使用 `mdfried` 或学习其渲染策略并在本项目实现；同时评估许可证、依赖体积、ANSI 样式兼容性与性能。
- 测试需要覆盖：默认渲染分支与 `--plain` 分支；以及“异常/不完整 Markdown 输入”下的安全降级（禁止 panic，输出不丢失）。
