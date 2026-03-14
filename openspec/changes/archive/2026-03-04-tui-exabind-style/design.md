## Context

当前项目的 TUI 位于 `crates/ralph-tui`，基于 `ratatui` + `crossterm`：
- 组件边框主要使用 `Block::default().borders(...)` 的默认边框字符
- 配色以零散的 `Color::Cyan/Yellow/Blue/...` 为主，缺少统一的主题（Theme）层
- 渲染循环已经以 `interval(Duration::from_millis(16))` 约 60fps 刷新，具备承载动画的节奏基础

本次变更希望把 TUI 的“视觉语言”升级为参考 `junkdog/exabind` 的风格基线：
1) 自定义框体字符集（更有质感的边框）
2) Catppuccin（Mocha）配色方案
3) 启动打开动画（open animation），让进入界面更有反馈

## Goals / Non-Goals

**Goals:**
- 提供一个集中、可复用的 TUI Theme 层：统一背景、正文、弱化、强调、focus、选中态、搜索高亮等样式
- 引入 exabind 风格的框体边框字符集（border glyph set），并在主要面板（Header/Footer/Instances/Output/Chat/Gates）一致应用
- 增加启动打开动画：进入 alternate screen 后，面板/框体以 sweep-in / expand 的方式出现；动画结束后进入常规渲染
- 动画可降级：非 TTY / 终端尺寸过小 / 明确禁用时，保持“无动画但正常可用”

**Non-Goals:**
- 不改动现有交互语义与布局结构（翻页、滚动、搜索、chat、gate 操作等保持原样）
- 不做完整的“多主题切换”系统（本次只落地一套默认主题）
- 不追求复刻 exabind 的所有特效（例如键盘可视化、LED 动画），仅聚焦“框体 + 配色 + 启动打开动画”

## Decisions

1) **Theme 数据结构：使用 Catppuccin（Mocha）作为颜色 token**

- 选择：在 `crates/ralph-tui` 内新增 `theme` 模块（或 `styling` 模块），提供：
  - Catppuccin Mocha 的 RGB 常量（例如 `crust/mantle/base/text/overlay/surface/...`）
  - 面向 UI 语义的 `Style` 生成器（例如 `panel_bg()`、`panel_border(focused)`、`title()`、`muted()`、`search_hit()`、`selection()`）
- 理由：把“颜色选择”从业务渲染逻辑里抽离出来，后续调整配色不需要在各 widget 里到处改 `Color::*`。

2) **框体边框：引入 exabind 风格的 `ratatui::symbols::border::Set`**

- 选择：定义一个全局复用的 border set（参考 exabind 的字符集，例如 `▟▜▔▏▕`），并通过
  - `Block::bordered().border_set(EXABIND_BORDER_SET)` 或等价方式
  在所有需要框体的面板里统一应用。
- 理由：边框字符集决定了 TUI 的“骨架质感”，统一之后才能形成一致的视觉品牌。
- 备选方案：继续使用 `Borders::ALL` 的默认字符集。
  - 未选理由：很难达到“像 exabind”的第一观感目标。

3) **面板渲染：用 helper 消除重复与特殊情况**

- 选择：新增一个面板构建 helper（例如 `fn panel_block(title, focused) -> Block`）：
  - 统一 border_set、border_style、title_style、背景色
  - focused/active 状态只通过参数决定（避免每个 widget 自己搞一套 if/else）
- 理由：这是典型的 cross-cutting 视觉改造，如果不抽 helper 会导致重复与后续漂移。

4) **启动打开动画：优先采用 `tachyonfx` 来做 buffer-level 效果**

- 选择：引入 `tachyonfx`（与 exabind 同一类思路）来实现：
  - sweep-in（UpToDown）/ fade 等组合效果
  - 用 `EffectManager` 在 60fps tick 上推进，并在每帧把 effect 应用到 `Frame` 的 buffer 上
- 理由：相比手写“逐帧裁剪/重排布局”，buffer-level 的 effect 组合更直接，也更接近 exabind 的效果与实现范式。
- 备选方案：不新增依赖，手写一个简化动画（按时间插值裁剪区域/逐行 reveal）。
  - 未选理由：实现复杂度高、很难把边框/标题/内容一起做出统一的 sweep 质感。

5) **动画降级与可控性：提供可禁用策略**

- 选择：设计上预留“禁用动画”的入口（例如配置项或环境变量），并在以下情况下自动禁用：
  - 终端尺寸过小（避免动画期间出现强烈闪烁或 layout artifact）
  - 非 TTY / 非交互环境（本来就不应该启用 TUI）
- 理由：动画属于“锦上添花”，必须保证在所有环境下“不影响可用性”。

## Risks / Trade-offs

- [Risk] 引入 `tachyonfx` 增加依赖与编译成本 → Mitigation：将依赖限制在 `crates/ralph-tui`；必要时做 feature gate（默认开，CI/瘦构建可关）
- [Risk] 边框字符（`▟▜▔▏▕` 等）在部分字体/终端渲染不佳 → Mitigation：提供 fallback border set（默认边框），允许在降级条件下切换
- [Risk] 现有基于渲染字符串的测试对 border 字符敏感 → Mitigation：更新测试关注“关键信息文本”而不是边框字符；必要时用 TUI 视觉验证做回归基线

## Open Questions

- 启动打开动画的“验收边界”需要更明确：我们更偏向 sweep-in（由上到下）还是 expand（由中心/四周展开）？
- 是否需要在 CLI 参数层提供 `--no-anim`（或统一到 `--no-tui` 体系）来显式关闭动画？
