## 1. Theme & Frame Primitives

- [x] 1.1 在 `crates/ralph-tui` 新增 Catppuccin（Mocha）调色板与 `TuiTheme`（语义化 style roles：bg/text/muted/accent/border/selection/search）
- [x] 1.2 新增 exabind 风格 `ratatui::symbols::border::Set`（例如 `▟▜▔▏▕`）并实现 `panel_block(title, focused)` 之类的复用 helper（统一 border_set/title/border_style/bg）
- [x] 1.3 支持“终端默认背景模式”（app bg=`Reset`），并在 Warp + TTY 下自动启用以保留半透明窗口背景；同时 panes 允许保留主题底色（base）以提升可读性、降低动画白条眩光

## 2. Apply Theme Across Widgets

- [x] 2.1 将 `crates/ralph-tui/src/widgets/instances.rs` 迁移到 Theme/Block helper：统一面板背景、标题样式、focus 边框与选中态
- [x] 2.2 将 `crates/ralph-tui/src/widgets/header.rs` 与 `crates/ralph-tui/src/widgets/footer.rs` 的分隔线与颜色迁移到 Theme，并更新对应渲染测试以适配新边框/新样式
- [x] 2.3 将 `crates/ralph-tui/src/widgets/content.rs` 的 selection/search highlight 从硬编码颜色改为 Theme roles（避免主题漂移）
- [x] 2.4 将并行模式底部面板（Chat/Gates）渲染统一到 Theme/Block helper（包括 prompt、chips、gate 列表与 actions 的颜色层级）

## 3. Startup Open Animation

- [x] 3.1 设计并落地“动画可禁用/可降级”开关（配置或环境变量），并在小窗口/非交互场景下自动禁用以保证可用性
- [x] 3.2 引入并接线动画引擎（优先 `tachyonfx`）：在 `App` 渲染循环中维护 `EffectManager`，并能对 `Frame` buffer 应用效果
- [x] 3.3 实现一次性启动打开动画（sweep-in / expand + fade）：进入 alternate screen 后播放一次，完成后进入 steady-state 正常渲染且不阻塞输入
- [x] 3.4 将启动打开动画改为“逐块出场”（从左到右、从上到下）：Instances（框体）→ Instances（条目）→ Output → Chat/Gates，并适当放慢节奏（整体 ≤ 2s）
- [x] 3.5 实现 Instances 条目出场：必须在 Instances 框体出场动画完成后，再开始条目逐行/渐显出现
- [x] 3.6 实现 Output 重新打开：切换实例时 Output 先消失（sweep_out / fade_out），再播放出场动画出现（sweep_in / fade_in）
- [x] 3.7 启动动画必须从“空屏”起步：首帧不应出现完整 UI（包含 header/footer），在 `bg=Reset`（Warp 半透明）模式下用 symbol 遮罩而非颜色插值避免闪烁

## 4. Verification

- [x] 4.1 调整/补充测试：避免对 border glyph 过度敏感，确保 `cargo test` 全通过（特别是 header/footer 的渲染测试）
- [x] 4.2 补充一条可重复的 TUI 视觉回归验证路径（例如捕获输出并用既有 TUI validate criteria 校验 header/footer/full layout）
