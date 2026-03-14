## Context

当前 Ralph 的输出渲染链路（TUI 与 stdout pretty）已经支持 Markdown 的语义样式渲染，但 fenced code block（```lang）内部缺少语法高亮。

在代码层面，我们目前的核心渲染入口集中在：

- `crates/ralph-adapters/src/stream_handler.rs`
  - `PrettyStreamHandler`：stdout pretty 模式，使用 `termimad::MadSkin` 将 Markdown 渲染为 ANSI 文本后直接输出。
  - `TuiStreamHandler`：TUI 模式，将 Markdown 渲染为 ANSI 文本，再用 `ansi-to-tui` 解析回 `ratatui::Line` 进行显示。

需要特别关注的现状约束：

- **流式输出刷新频率高**：`TuiStreamHandler::on_text()` 会在每个 chunk 到来时更新 UI。
- **长内容滚动敏感**：如果每个 chunk 都对历史内容做全量重渲染（Markdown 解析、wrap、ANSI 解析），会出现明显卡顿，且随内容增长恶化。
- **一致性要求**：本变更要求 stdout pretty 与 TUI 的 code block 高亮行为一致，避免“同一段输出两种视觉语义”。

## Goals / Non-Goals

**Goals:**

- 为 Markdown fenced code block 提供语法高亮：
  - 覆盖 TUI 输出与 stdout pretty 输出。
  - 首期支持语言：`rust`、`bash/sh`、`json`、`yaml`、`toml`、`python`、`js/ts`。
- 保持非 code block 的 Markdown 渲染行为不变（继续复用 `termimad` 语义样式）。
- **性能优先**：
  - 流式阶段未闭合 code block 不做语法高亮（仅统一 code 样式）。
  - 已闭合 code block 的高亮结果“冻结/缓存”，后续 chunk 不重复高亮、不重复渲染历史。
- stdout/TUI 两条渲染路径共享同一套“语义与颜色映射”，保证一致性。

**Non-Goals:**

- 不为行内代码（inline code）做语法高亮（仍按现有 Markdown 样式即可）。
- 不追求完整 Markdown AST 解析；仅实现 fenced code block 的最小识别与分段。
- 不在 code block 未闭合阶段做“临时语法高亮”（避免流式抖动与重复开销）。
- 首期不承诺支持复杂嵌套场景（例如 quote/list 内嵌 fence）为“完美解析”；解析失败时允许安全降级为普通 Markdown 文本显示。

## Decisions

### 1) 仅对 code block 做语法高亮（而不是全量 Markdown 高亮）

**选择**：只对 fenced code block 做语法高亮；其他 Markdown 继续交给 `termimad`。

**理由**：

- LLM 输出中普通文本占比通常远高于代码块，收益最大化的做法是把高成本操作限定在 code block。
- 可以把“语法高亮成本”从“每个 chunk”变成“每个 code block 闭合一次”，极大改善流式刷新与长内容滚动体验。

**替代方案**：

- 使用 `bat`（类似 Tenere）对整段 Markdown 输出 ANSI，再解析回 TUI：实现快，但链路更重（ANSI 往返 + 全量渲染），对流式与长内容更不友好。

### 2) fenced code block 采用“流式状态机”识别（而不是完整 Markdown 解析器）

**选择**：实现一个轻量的 fenced code block 状态机，支持跨 chunk 的行级识别：

- `OUTSIDE`：累积普通 Markdown 文本
- `IN_CODE(lang)`：累积代码内容，直到遇到 closing fence

**理由**：

- 状态机可在 O(新增文本长度) 的成本下持续运行，适合高频流式更新。
- 能自然满足“未闭合不高亮，闭合后一次性高亮”的策略。

**替代方案**：

- 引入完整 Markdown parser（pulldown-cmark/comrak）：实现更“规范”，但复杂度与开销更高，且与 termimad 现有渲染并不天然兼容。

### 3) 通过“分段冻结/缓存”避免每个 chunk 全量重渲染

**选择**：把输出拆成可冻结的渲染块（Rendered Blocks），并只对“尾部未冻结段”做增量渲染。

建议的块类型：

- `MarkdownBlock`：一段不含 fence 的 Markdown 文本（已渲染为 ANSI；TUI 侧已解析为 `Vec<Line>`）
- `CodeBlock`：一个已闭合的 code block（已语法高亮并渲染；同样可缓存 ANSI 与 `Vec<Line>`）
- `NonTextLine`：工具调用/错误/summary 等单行结构输出（现有逻辑保持）

冻结时机（关键）：

- 当检测到 opening fence：先冻结此前累积的 MarkdownBlock
- 当检测到 closing fence：冻结 CodeBlock（此时才做语法高亮）
- 当发生 tool call/error/complete：冻结当前文本段，保持时间顺序（沿用现有 block 模型）

**理由**：

- 避免 `TuiStreamHandler::update_lines()` 在每个 chunk 到来时重渲染所有历史 blocks。
- 对于长输出，滚动体验更稳定（历史块不变，渲染成本与“尾部长度”相关，而非与“总长度”相关）。

**替代方案**：

- 继续全量重渲染，但加入 throttle/debounce：能缓解但治标不治本；长输出仍会变慢，并且会增加 UI 延迟感。

### 4) stdout/TUI 一致性：以 ANSI 作为共享的中间表示

**选择**：

- 对 stdout pretty：直接输出 ANSI 文本。
- 对 TUI：把 ANSI 文本解析为 `ratatui::Line`（继续复用 `ansi-to-tui`）。

并在“冻结块”级别缓存两种形态（ANSI + Lines），避免重复解析。

**理由**：

- 当前渲染链路已经在使用 ANSI 作为中间形态（termimad → ANSI；TUI 再 parse）。
- 复用该模式能最小化改动面，同时保证 stdout/TUI 输出一致。

**替代方案**：

- 以 `ratatui::Line/Span` 作为唯一输出形态，再为 stdout 实现一套 Span→ANSI 的渲染器：一致性强，但改动面更大（需要重构 stdout pretty 输出）。

### 5) 语法高亮引擎选择：优先 tree-sitter-highlight（性能优先）

**选择**：使用 tree-sitter highlight 体系作为 code block 的语法高亮引擎。

**理由**：

- 在“只高亮 code block + 冻结缓存”的前提下，tree-sitter 仍能提供更好的性能上限与结构化 token 类别。
- highlight group（keyword/string/comment/...）更易映射到我们已有的 Sublime Monokai Extended 调色板，保证视觉一致性。

**替代方案**：

- `syntect`：更容易直接输出 ANSI，但 scope/theme 映射更复杂，且对“严格对齐我们现有调色板”不如 tree-sitter 的 group 映射直观。

## Risks / Trade-offs

- **[依赖与体积]** 引入 tree-sitter grammars 与 highlight queries 可能增加二进制体积与编译时间 → **Mitigation**：只打包首期 7 种语言；查询文件按需 vendor；可考虑 feature gate。
- **[解析边界]** fence 识别在复杂嵌套 Markdown（quote/list）中可能出现误判 → **Mitigation**：优先保证“不崩溃/不丢内容”；解析失败回退为普通 Markdown 渲染；后续按真实样例补强。
- **[主题一致性]** code block 高亮颜色与现有 Markdown 主题不一致会造成割裂 → **Mitigation**：明确一套 highlight group → palette 的映射，并用回归测试锁定关键颜色。
