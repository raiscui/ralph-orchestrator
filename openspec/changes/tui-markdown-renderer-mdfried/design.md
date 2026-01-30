## Context

当前项目的 Markdown 渲染链路（以 TUI 输出为主）大致是：

- 先检测输出文本是否包含 ANSI 转义序列
  - 若包含：直接走 `ansi_to_tui`，优先保留既有颜色/格式
  - 若不包含：在 Rendered 模式下用 `termimad` 把 Markdown 渲染为 ANSI 文本，再由 `ansi_to_tui` 转为 `ratatui` 的行/样式
- `--plain` 语义是“禁用 Markdown 渲染”，但依然会做 ANSI 解析以避免把 ESC 控制符原样打印出来

你希望改用（或对齐）`mdfried`/`mdfrier` 的渲染策略：
`mdfrier` 使用 `tree-sitter-md` 解析 Markdown，并在给定 width 下做“基于 Markdown 上下文”的换行包装，再输出结构化 `Line/Span` 列表；同时它提供 `ratatui` feature，可直接把 markdown span 的语义（粗体/斜体/代码/引用/表格等）映射为 `ratatui` 的 `Style`。

关键约束是许可证：
`mdfrier` 的许可证是 `GPL-3.0-or-later`，要“直接依赖并使用”它，就需要把本项目许可证切换到 GPLv3 兼容（本次默认选择 `GPL-3.0-or-later`）。

## Goals / Non-Goals

**Goals:**
- 将 Markdown 渲染引擎切换为 `mdfrier`（`ratatui` feature），减少“Markdown → ANSI → ratatui”的二次转换复杂度。
- 保持用户心智稳定：继续支持 Rendered/Plain（`--plain`）两种模式，且遇到异常/不完整输入必须安全降级，不 panic、不丢内容。
- 维持 ANSI 优先级：当文本包含 ANSI 转义序列时，优先保留 ANSI 样式（避免“Markdown 渲染吞掉颜色”）。
- 渲染宽度与真实终端宽度对齐：TUI 使用面板宽度；non-TUI pretty 输出使用实际终端宽度（或合理的 fallback）。
- 完成许可证切换所需的工程动作清单（在 tasks 中可执行、可验证）。

**Non-Goals:**
- 不追求一次性覆盖 CommonMark/GFM 的全部语义（以 `mdfrier` 能力为上限，额外增强留到后续）。
- 不引入“代码块语法高亮”等更重的渲染能力（除非现有项目已具备且不增加复杂度）。
- 不改变 TUI 布局/交互（本次只聚焦渲染引擎与许可证）。

## Decisions

1) **许可证策略：切换为 `GPL-3.0-or-later`**
- 决策：把仓库许可证从 MIT 切换为 `GPL-3.0-or-later`（与你说的“GPL3”一致且与 `mdfrier` 兼容）。
- 理由：只有许可证兼容后，才能直接引入并使用 `mdfrier`（避免“只能参考思路但不能用代码/依赖”的尴尬）。
- 备选：保持 MIT + 自研/换用宽松许可证渲染库（不符合你当前指令，作为 fallback 记录在案）。

2) **依赖策略：引入 `mdfrier` + `ratatui` feature，并计划移除 `termimad`**
- 决策：在 workspace 依赖中加入 `mdfrier`，启用其 `ratatui` feature（获得 `Theme` 与 ratatui 行转换）。
- 决策：当切换完成且测试通过后，移除 `termimad` 以减少“两个 Markdown 引擎并存”导致的行为分叉。
- 权衡：移除 `termimad` 可能影响既有 wrap 细节与测试断言，需要在 tasks 中明确“以新的 specs/fixtures 为准”。

3) **渲染管线：保留 ANSI 优先，Markdown 只处理“非 ANSI 文本”**
- 决策：延续现有策略：`contains_ansi(text)` 为 true 时，跳过 Markdown 渲染，直接走 ANSI → ratatui。
  - 理由：ANSI 与 Markdown 混合解析成本高且容易产生风格冲突；现有策略更稳定、也更符合“颜色输出工具优先”的直觉。
- 决策：当 `contains_ansi(text)` 为 false 且 Rendered 模式启用时：
  - 使用 `mdfrier::MdFrier::parse(width, text, theme)` 生成 markdown 行
  - 若解析失败/返回空结果（且输入非空），必须回退到“按 `\n` 分行的纯文本渲染”，确保内容不丢
- 决策：Plain 模式保持现有语义：Markdown 控制符原样可见，但仍会做 ANSI 解析（避免 ESC 控制符外露）。

4) **宽度来源：TUI 面板宽度 vs non-TUI 终端宽度**
- 决策：TUI 渲染时用“输出面板可用宽度”（扣除边框/滚动条等）作为 `mdfrier` 的 `width`。
- 决策：non-TUI pretty 输出使用当前终端宽度；若无法获取则 fallback 到一个明确的默认值（例如 80），并在 design/tasks 中把这个默认值固定为“可测试的常量”。

5) **Theme 策略：先落地可用，再对齐 Ralph 主题**
- 决策：先使用 `mdfrier::ratatui` 提供的 `DefaultTheme` 跑通端到端（保证功能闭环与正确性）。
- 决策：随后在同一 change 中引入 `RalphMarkdownTheme`（实现 `mdfrier::ratatui::Theme`），把颜色/强调风格对齐 Ralph 现有 TUI 主题（避免过亮/不统一）。

## Risks / Trade-offs

- [Risk] 许可证切换会影响下游使用/分发方式（对外是实质性 breaking change）
  - Mitigation：在 README/CHANGELOG（或 release notes）中明确说明；版本号按语义化升级；并在 tasks 中列出“需要更新的元数据点”。
- [Risk] 仓库存在多贡献者时，可能需要确认再许可权限
  - Mitigation：在 tasks 中加“贡献者/CLA/历史提交”检查项；如果无法确认，则在实施阶段停在“许可证切换”门槛前。
- [Risk] `mdfrier` 解析失败时返回空 Vec，可能导致输出被吞
  - Mitigation：对非空输入若输出为空，强制回退为纯文本分行渲染（并加回归测试锁定）。
- [Risk] 换行/空行细节变化会导致现有测试或 smoke fixtures 失败
  - Mitigation：以 specs 为准更新断言与 fixtures；新增针对“空行/尾部换行/不完整 fenced code block”的回归测试覆盖。
- [Risk] `mdfrier` 的 `rust-version` 为 `1.86.0`，若项目/CI 低于此版本会构建失败
  - Mitigation：在 tasks 中明确 toolchain 要求与 CI 变更点（若项目当前实际要求更高，则记录为兼容通过）。

## Migration Plan

1. 先落盘 specs（把用户可见行为锁死：Rendered/Plain、降级策略、ANSI 优先）。
2. 实施时先做“许可证切换 + 依赖接入”（确保 legal/编译门槛过关）。
3. 替换渲染管线（TUI 与 non-TUI 尽量走同一条路径），并补齐单测。
4. 更新 smoke fixtures（如有）与文档说明，跑 `cargo test` 全量验证。

## Open Questions

- 渲染范围：只替换 Supervisor TUI 的输出渲染，还是同时替换 non-TUI pretty 输出？（本次倾向统一，以减少分叉）
- Theme 风格：是否需要“尽量复刻 mdfried 的视觉风格”，还是“对齐 Ralph 现有 TUI 主题”即可？
