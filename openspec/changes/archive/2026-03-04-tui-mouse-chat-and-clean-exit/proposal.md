## Why

当前 Supervisor TUI 更偏向“只读查看”，缺少鼠标点选实例、输出文本的多行框选/选择，以及像终端一样可用的 chat 输入体验，导致并行调试与人工介入效率偏低。
同时，退出 TUI 时仍可能残留 worker 的 headless CLI 子进程，带来资源泄露与后续运行被污染的风险，所以需要一次把交互与退出语义补齐。

## What Changes

- Supervisor TUI 支持鼠标点击实例列表项来切换“当前选中实例”，并与现有键盘选择行为保持一致。
- 实例输出视图支持“像文字编辑器一样”的文本选择能力：支持鼠标/键盘选择、支持多行选择、支持鼠标框选。
- Chat 区域支持鼠标点击进入输入态；输入框提供清晰提示符（prompt），并支持像终端输入一样的编辑体验：
  - 支持键盘与鼠标的光标移动（左右/上下、鼠标定位）
  - 支持鼠标/键盘框选选择
  - 支持 `Shift+Enter` 换行（多行输入）
- 退出 Supervisor TUI 时，必须退出并清理所有并行 worker 的 CLI 子进程，避免残留与资源泄露。

## Capabilities

### New Capabilities
- （无）

### Modified Capabilities
- `parallel-supervisor-tui`: 增加鼠标点选实例、输出文本选择/框选等交互要求，并保证与既有滚动/搜索/键盘选择能力可组合使用。
- `supervisor-human-chat-gate`: 明确 chat 输入框的交互与编辑要求（提示符、鼠标点击进入、光标移动、多行输入、选择/框选、`Shift+Enter` 换行）。
- `parallel-hat-instances`: 补充“退出/取消”时的进程生命周期要求：当 Supervisor（含 TUI）退出时，必须确保所有 headless CLI 子进程被终止，不留下孤儿进程。

## Impact

- `crates/ralph-tui`: 需要引入/完善鼠标事件处理、焦点管理、文本选择模型与渲染；并增强 chat 输入组件的行编辑与多行输入支持。
- 并行运行时（`ralph-cli`/`ralph-core` 相关）：需要保证退出链路会触发可观测的 shutdown，并对 worker 子进程做可靠的终止与回收（避免残留进程与收尾 warning）。
- 测试与验证：可能需要新增/更新并行退出的回归测试；如涉及 UI 行为变更，建议补充 TUI 视觉/交互验证基准。
