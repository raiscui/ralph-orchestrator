## Context

Ralph 的 TUI 输出区会展示 AI code agent 的 CLI 输出。
这类输出经常包含 Markdown（例如 `#`/`##` 标题、`>` 引用、代码块等）。
当前如果以纯文本展示，信息层级不明显，阅读与排障效率偏低。

本项目已经在 workspace 里引入 `termimad`（MIT），并在 `crates/ralph-adapters/src/stream_handler.rs` 中存在“ANSI + Markdown → ratatui Lines”的转换逻辑。
因此本次设计倾向于**改良现有渲染管线**，而不是引入一个全新渲染系统。

同时，我们调研了 `benjajaja/mdfried` / `mdfrier`：
它们提供了“Markdown → styled terminal lines”的能力，并使用 `tree-sitter-md` 做解析。
但其许可证为 `GPL-3.0-or-later`，与本项目 MIT License 不兼容，因此我们只能“学习技术思路”，不能直接依赖或复用其代码。

## Goals / Non-Goals

**Goals:**
- Supervisor TUI 的实例输出视图默认 best-effort 渲染 Markdown，提高可读性。
- 至少覆盖常见结构：`h1/h2`、`blockquote`（`>`）、代码（fenced/inline）。
- 提供 `--plain` 参数：强制关闭 Markdown 渲染，让 Markdown 控制符原样可见。
- 渲染失败/不完整输入必须安全降级：不 panic、不丢内容、不阻塞后续输出。
- 不破坏既有滚动与搜索心智（`/` 搜索依旧可用）。

**Non-Goals:**
- 完整实现 CommonMark/GFM 的全部语义（例如表格、脚注、任务列表等完整覆盖）。
- 图片渲染、语法高亮、富交互（这些属于未来增强）。
- 改动 TUI 启用/降级逻辑（TTY 检测、`--no-tui` 等保持不变）。

## Decisions

1) **依赖与许可证策略**
- 决策：不引入 `mdfried` / `mdfrier`（GPL-3.0-or-later）。
- 决策：优先复用现有 `termimad` 渲染能力（项目已依赖），必要时只做 skin/样式改良。
- 备选：若 `termimad` 无法满足“标题/引用/代码块”的效果或性能要求，再评估引入宽松许可证的 Markdown 解析/渲染库（例如 `pulldown-cmark` + 自研映射，或 `tui-markdown` 等），但必须先做许可证确认。

2) **渲染管线（默认渲染 vs `--plain`）**
- 决策：新增一个明确的“Markdown 渲染模式”开关（例如 `MarkdownRenderMode::{Rendered, Plain}` 或 `render_markdown: bool`）。
- 决策：默认值为 Rendered；当 CLI 指定 `--plain` 时强制进入 Plain。
- 决策：Plain 模式仅禁用 Markdown 语义渲染，不改变既有 ANSI 处理（如果输出包含 ANSI，仍按现有逻辑解析为样式，而不是把转义序列原样打印出来）。
  - 理由：`--plain` 的目标是“不要隐藏 Markdown 控制符，便于排障/复制”，而不是让用户看到不可读的 ANSI 控制码。

3) **流式输出与缓存策略**
- 决策：保留“原始输出文本 buffer”作为单一事实来源。
- 决策：当 buffer 追加、终端宽度变化、或渲染模式切换时，重新生成用于渲染的 `Vec<ratatui::text::Line>`（或等价结构）。
- 决策：解析失败时直接回退到“按 `\n` 分行的原始文本渲染”，确保稳定性优先。

4) **搜索的匹配文本**
- 决策：搜索逻辑优先在“用户可见文本”上工作（Rendered 模式用渲染后的可见文本，Plain 模式用原始文本）。
- 备选：若实现复杂度过高，允许先在原始文本上搜索（仍能满足 `foo` 这类文本片段命中），但需要确保 UI 不会因行折叠/换行差异导致崩溃。

## Risks / Trade-offs

- [Risk] 现有 `termimad` 的段落/换行规则可能与用户预期不同 → Mitigation：在 `MadSkin` 与换行参数上做最小可控改良，并用回归测试锁定行为。
- [Risk] 输出是“流式拼接”，可能出现未闭合 fenced code block 等不完整 Markdown → Mitigation：要求 best-effort + 安全降级，并补充专门回归测试覆盖。
- [Risk] `--plain` 的作用范围（只影响 TUI 还是也影响非 TUI pretty 输出）可能引发预期差异 → Mitigation：在 tasks 中明确实现范围，并在 CLI help 文案里写清楚。

## Migration Plan

- 增量上线：先加 `--plain` 并保持默认行为不变（默认仍渲染）。
- 回滚策略：若渲染造成严重问题，可临时在默认配置中切回 Plain（或在代码里保留快速切换开关），同时保持 `--plain` 仍可用。

## Open Questions

- `--plain` 是否只作用于 Supervisor TUI 的“实例输出视图”，还是同时作用于非 TUI 的 pretty 输出？（倾向：保持 TUI/非 TUI 的一致性，但需要你确认期望）
- 对标题（`#`/`##`）的视觉呈现，是否需要更强的“分隔线/强调”风格来贴近 mdfried 的观感？还是只要粗体/颜色就足够？
