# 任务计划：Markdown 内部配色微调（Sublime Monokai Extended）

## 目标

- 调整 Ralph 的 Markdown 渲染配色（stdout + TUI 同步），满足你最新偏好：
  - code block：取消背景；前景 `#78dce8`
  - inline code：取消背景；前景 `#78dce8`
  - 标题（H1）：`#ffd866`
  - heading（H2-H6）：`#fc9867`
  - Markdown 红色：`#ff6188`
  - Markdown “强调/标签类”（通常是 `**bold**`）改为绿色：`#a9dc76`
- 对 Markdown 内部所有颜色统一做轻微“偏蓝”微调：
  - 所有颜色混入 3% 的 `#4493f8`
  - 白色（正文）保持不变

## 方案（至少二选一）

### 方案 A（先能用，后面再优雅｜我将先按此执行）

- 只改 `crates/ralph-adapters/src/stream_handler.rs` 的 `default_markdown_skin()` 与 palette 常量。
- 更新/新增回归测试锁定颜色与“无背景”要求。
- 优点：改动范围最小；stdout/TUI 自动一致；风险低。

### 方案 B（不惜代价，最佳方案）

- 在方案 A 基础上，把同一套配色同步到：
  - `crates/ralph-tui` 的全局主题（标题栏/边框/面板强调色）
  - （可选）docs 站点的 CSS（如果你也希望文档站点一致）
- 优点：视觉一致性最好；代价：影响面更大，需要额外 TUI/文档验证。

## 阶段

- [x] 阶段1：更新 Markdown skin 映射
- [x] 阶段2：更新/新增回归测试
- [x] 阶段3：验证（fmt/clippy/test/smoke）
- [x] 阶段4：四文件记录（notes/WORKLOG）

## 关键问题

1. “标题”我默认按 H1（`# Title`）理解；“heading”按 H2-H6（`##...`）理解。
2. 你这条反馈里“初始化”属于 `**bold**` 的强调/标签类，我会把 `skin.bold` 的前景色改为 `#a9dc76`。

## 做出的决定

- [x] 先按方案 A 落地（范围收敛、风险最低）。如你希望 docs/TUI 全局也同步，再切到方案 B。

## 状态

**已完成**：Markdown 颜色已统一混入 3% 的 `#4493f8`（白色正文不变），并完成测试与全量验证。

## 日志

### 2026-02-01 11:40 +0800

- [启动] 收到你的最新配色要求：使用 Sublime Monokai Extended，并按指定颜色/无背景规则微调。

### 2026-02-01 11:43 +0800

- [完成] 更新 `default_markdown_skin()`：
  - H1（标题）改为 `#ffd866`
  - H2-H6（heading）保持 `#fc9867`
- [完成] 更新/新增回归测试，锁定 H1/H2 的颜色配置。

### 2026-02-01 11:44 +0800

- [验证] 通过：
  - `cargo fmt --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
  - `cargo test -p ralph-core smoke_runner`
  - `cargo test -p ralph-core kiro`

### 2026-02-01 11:46 +0800

- [记录] 已追加更新 `notes.md` / `WORKLOG.md`（本次 Markdown 配色微调的原因、修复点、验证命令）。

### 2026-02-01 11:49 +0800

- [整理] 同步一条注释，使其与“H1/H2+ 分层标题色”的实现一致。
- [复检] `cargo fmt --check` ✅

### 2026-02-01 12:00 +0800

- [追加需求] 你反馈类似 `"1. 初 始 化 ： 入 口 命 令 启 动"` 里，“初始化”这类强调（bold）现在是红色，希望改为 `#a9dc76`。

### 2026-02-01 12:02 +0800

- [完成] `**bold**`（强调/标签类）前景色改为 `#a9dc76`（用于“初始化/步骤标签”这类文本）。
- [测试] 新增回归测试锁定 bold 颜色，避免未来回退。
- [验证] 通过：
  - `cargo fmt --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
  - `cargo test -p ralph-core smoke_runner`
  - `cargo test -p ralph-core kiro`

### 2026-02-01 12:17 +0800

- [追加需求] 你希望把 Markdown 的所有颜色统一混入 3% 的 `#4493f8`，让整体色相略微偏蓝；但“白色正文”保持不变。

### 2026-02-01 12:19 +0800

- [完成] 已对 Markdown palette 做统一 3% `#4493f8` 混合；`FOREGROUND`（正文白色）保持不变。
- [测试] 回归测试期望值已更新为混合后的最终 RGB。
- [验证] 通过：
  - `cargo fmt --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
  - `cargo test -p ralph-core smoke_runner`
  - `cargo test -p ralph-core kiro`
