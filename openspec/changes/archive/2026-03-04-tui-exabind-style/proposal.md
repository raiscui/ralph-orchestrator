## Why

当前 Ralph 的 `ratatui` TUI 视觉层次偏“默认样式”：框体字符与配色较朴素、缺少进入界面的启动打开动画。
随着并行模式 Supervisor TUI 成为主要的观测与调试界面，我们需要用更一致、更有品质感的视觉语言提升可读性与沉浸感。

## What Changes

- 引入一套共享的 TUI 视觉主题（Theme），参考 `junkdog/exabind` 的风格基线：
  - 框体使用自定义 border glyph set（例如 `▟▜▔▏▕`）形成更“锐利/现代”的框体质感
  - 配色使用 Catppuccin（Mocha）调色板，统一背景/文本/强调色/弱化色
- 为 TUI 增加启动打开动画（open animation），用于“进入 alternate screen 后的第一印象”：
  - 面板/框体以 sweep-in / expand 方式出现，形成更自然的启动反馈
  - 在非 TTY / 低性能 / 明确禁用的情况下可降级为无动画渲染
- 对现有组件的样式做一致化改造（不改变核心交互语义）：
  - Header/Footer/Instances/Output/Chat/Gates 等面板的边框、标题、focus 高亮、选中态、搜索高亮统一到主题

## Capabilities

### New Capabilities

- `tui-exabind-style`: 定义 Ralph TUI 的默认“框体风格 + 配色方案 + 启动打开动画”的规格与约束（作为可复用的视觉能力，而不是分散在各个 widget 里的硬编码样式）。

### Modified Capabilities

- `parallel-supervisor-tui`: 并行模式 Supervisor TUI 的 REQUIREMENTS 将补齐“视觉主题/框体风格/启动动画”的可验收要求（不仅仅是功能可用）。

## Impact

- 受影响代码：
  - `crates/ralph-tui`: 需要抽出 Theme/Frame 复用层，并统一各面板的 `Block`/border_set/title style 渲染
  - TUI 启动与渲染循环：需要保存动画状态并在 60fps tick 下推进动画帧
- 依赖影响：
  - 可能引入 `tachyonfx`（或实现等价的轻量动画系统）来实现 sweep-in 等效果
- 测试与验证：
  - 现有基于字符串的渲染测试可能需要更新（border 字符变化）
  - 对关键组件建议补充可重复的 TUI 视觉验证基线（避免未来回归）
