## Why

当前 Ralph 的 TUI/pretty 输出已经支持 Markdown 的 best-effort 渲染，但实现依赖 `termimad`（Markdown → ANSI）再叠加 `ansi_to_tui`（ANSI → ratatui 行），属于“二次转换”的渲染链路，维护成本偏高，也更容易出现换行/样式细节上的不一致。
你希望改用（或参考）`mdfried` 项目中的 `mdfrier`，用 `tree-sitter-md` 解析 Markdown 并直接生成“按宽度包装后的结构化行”，从而获得更可控、更一致的渲染效果。

## What Changes

- 将 Markdown 渲染引擎从 `termimad` 切换为 `mdfrier`（或以 `mdfrier` 的渲染/包装策略为对齐目标），用于把 AI code agent 的输出更稳定地渲染到终端/TUI。
- 保持现有“渲染模式”心智不变：
  - 默认 Rendered：对 Markdown 做 best-effort 渲染，突出信息层级（标题/引用/代码块等）。
  - `--plain`：禁用 Markdown 渲染，Markdown 控制符原样可见，用于排障/复制/对齐旧行为。
- 继续遵守安全降级：遇到不完整/无法解析的 Markdown 输入时，必须安全回退为纯文本显示，且不得丢内容或 panic。
- **BREAKING**：将本项目许可证从 MIT 调整为 GPLv3 兼容许可证（优先建议 `GPL-3.0-or-later`，以与 `mdfrier` 的 `GPL-3.0-or-later` 保持一致），并同步更新仓库元数据与文档说明。

## Capabilities

### New Capabilities
- （无）

### Modified Capabilities
- `parallel-supervisor-tui`: 明确“实例输出视图”的 Markdown 渲染要求（Rendered/Plain 模式、best-effort、安全降级），并把渲染引擎切换视为实现手段（行为以 specs 为准）。

## Impact

- 受影响代码：
  - `crates/ralph-adapters`：Markdown/ANSI → ratatui 行的渲染管线（替换渲染引擎、统一换行宽度策略、保持 ANSI 处理优先级）。
  - `crates/ralph-tui` / `crates/ralph-cli`：渲染模式开关的接线与帮助文案（如 `--plain`）需要保持一致。
- 受影响依赖：
  - 新增 `mdfrier`（GPL-3.0-or-later）依赖；并评估是否移除 `termimad` 以减少重复能力与行为分叉。
- 许可证影响（需要在 design/tasks 中明确可执行步骤）：
  - 更新根目录 `LICENSE`、`Cargo.toml` 的 `license` 字段、README 等对外说明。
  - 评估历史贡献者/第三方代码的再许可可行性与风险（如果存在多贡献者，需要明确授权路径）。
