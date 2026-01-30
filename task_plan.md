# 任务计划：彻底回退 mdfried 相关功能 + 许可证回退 + 移除 stderr 前缀列

## 目标

- 执行“彻底回退”（你选择了方案 A）：
  - 移除 Big Headers / 图片块等 `mdfried` 相关渲染特性
  - Output 面板回到纯文本渲染（继续使用 `termimad`）
- 许可证回退到“项目原本”许可（计划回到 MIT，并同步更新仓库元数据与文档）
- 并行 Output 面板取消左侧红色 `E` 前缀列
  - stderr 用“灰色弱化”区分即可
- 保证 `cargo fmt --check`、`cargo clippy -- -D warnings`、`cargo test`、replay smoke tests 全部通过

## 方案（至少二选一）

### 方案 A：彻底回退（你已选择）

- 移除 Big Headers / 图片块等 `mdfried` 相关渲染特性
- Output 面板完全使用 `termimad` 的文本渲染结果

### 方案 B：最小回退（先恢复 termimad，保留其他 UI 改良）

- 仅把“Markdown→ANSI/Line”渲染器切回 `termimad`
- 其余与渲染器无关的 TUI 结构改良（例如前缀列/并行面板布局）保持不动
- 后续如需再“彻底回退”，再做第二步

> 你已明确选择：方案 A（彻底回退）。

## 阶段

- [x] 阶段1：盘点需要撤回的功能点
- [x] 阶段2：移除 Big Headers / 图片渲染与相关依赖
- [x] 阶段3：移除 Output 左侧红色 `E` 前缀列
- [x] 阶段4：许可证回退到 MIT（含 docs/README）
- [x] 阶段5：全量验证与四文件记录

## 关键问题（默认先做最小回退）

1. 许可证目标：我按“回到 MIT”执行（与 docs 里 Historical Notice 一致）。
   - 若你要 `MIT OR Apache-2.0` 之类的双许可，需要你再明确一下。

## 遇到错误

- [记录] `cargo fmt --check` 发现末尾空行差异，已用 `cargo fmt` 修复并通过复检。

## 状态

**已完成**：
已彻底移除 Big Headers/图片渲染与左侧 `E` 前缀列，并将许可证回退到 MIT。
全量验证已通过（fmt/clippy/test/smoke）。

## 日志

### 2026-01-30 12:49 +0800

- [确认] 你要求执行：
  - 方案 A：彻底回退 Big Headers/图片块
  - 许可证回退（预期回到 MIT）
  - 取消 Output 面板左侧红色 `E` 前缀列

### 2026-01-30 13:34 +0800

- [完成] Big Headers/图片渲染与相关依赖已移除；Output 前缀列已移除（不再显示红色 `E`）。
- [完成] 许可证已回退到 MIT，并同步更新 `Cargo.toml` / `LICENSE` / README / docs。
- [验证] `cargo fmt --check` / `cargo clippy -- -D warnings` / `cargo test` / `cargo test -p ralph-core smoke_runner` / `cargo test -p ralph-core kiro` 全部通过。

---

# 任务计划：termimad 渲染的 H1 从居中改为左对齐

## 目标

- `termimad` 渲染 Markdown 时，H1（`# Title`）不再居中，改为靠左对齐。
- 行为在两条渲染路径保持一致：
  - stdout（Pretty 输出）
  - TUI（转为 `ratatui::Line`）
- 保证 `cargo fmt --check`、`cargo clippy --all-targets --all-features -- -D warnings`、`cargo test` 全部通过。

## 阶段

- [x] 阶段1：定位 termimad 的 H1 对齐配置点
- [x] 阶段2：实现自定义 `MadSkin`（只改 H1 对齐）
- [x] 阶段3：补回归测试（验证 H1 不再被填充左侧空格）
- [x] 阶段4：验证并提交变更

## 状态

**已完成**：
已把 termimad 渲染的 H1 对齐从居中改为左对齐，并保证 stdout/TUI 两条渲染路径一致生效；同时补充了回归测试并完成全量验证。

## 日志

### 2026-01-30 22:17 +0800

- [启动] 新需求：termimad 渲染的 H1 从“居中”改为“靠左对齐”。
- [计划] 先调研 `MadSkin` / header style 的对齐 API，再统一封装一个 skin builder，最后补测试并提交。

### 2026-01-30 22:22 +0800

- [完成] 在 `default_markdown_skin()` 中把 `headers[0].align` 设置为 `Alignment::Left`，H1 不再居中。
- [测试] 新增回归测试：`markdown_h1_is_left_aligned_in_rendered_mode`。
- [验证] `cargo fmt --check` / `cargo clippy --all-targets --all-features -- -D warnings` / `cargo test` 全部通过。
