## 1. 依赖与主题映射

- [ ] 1.1 为 `ralph-adapters` 增加 code block 语法高亮所需依赖（tree-sitter + highlight + 7 种语言 grammar）
- [ ] 1.2 为 7 种语言引入/落地对应的 highlights queries（确保离线可用、构建可重复）
- [ ] 1.3 定义 highlight group → `sublime_monokai_extended` 调色板的映射表（保证 stdout/TUI 颜色一致）

## 2. fenced code block 流式分段器（state machine）

- [ ] 2.1 实现跨 chunk 的 fenced code block 识别与分段（含行缓存，避免边界切割导致误判）
- [ ] 2.2 实现语言标签 normalize（`sh↔bash`、`yml↔yaml`、`py↔python`、`js/javascript`、`ts/typescript`）
- [ ] 2.3 实现未知语言与异常 fence 的安全降级（不 panic、不丢内容，回退统一 code 样式）

## 3. code block 高亮渲染器（闭合后一次性高亮）

- [ ] 3.1 实现 `CodeBlockHighlighter`：仅在 closing fence 到达后生成高亮输出（未闭合阶段不高亮）
- [ ] 3.2 实现高亮结果的 ANSI 输出（stdout pretty 直接写 ANSI；TUI 复用 `ansi-to-tui` 解析）

## 4. TUI 渲染链路集成（避免全量重渲染）

- [ ] 4.1 改造 `TuiStreamHandler`：引入“冻结块/缓存”，避免每个 chunk 重渲染全部历史内容
- [ ] 4.2 确保 `--plain` 模式下禁用 Markdown 与 code block 高亮（但仍保留现有 ANSI 输入解析优先级）

## 5. stdout pretty 渲染链路集成（与 TUI 一致）

- [ ] 5.1 改造 `PrettyStreamHandler`：在 flush 阶段复用同一分段器/高亮器，输出与 TUI 语义一致
- [ ] 5.2 确保 stdout pretty 的支持语言/别名/降级行为与 TUI 完全一致

## 6. 回归测试与门禁验证

- [ ] 6.1 增加测试：支持语言的闭合 code block 会产生“可观测的高亮”（stdout 至少包含 ANSI；TUI 至少存在非默认样式 span）
- [ ] 6.2 增加测试：未闭合 code block 不触发语法高亮，closing fence 到来后才高亮
- [ ] 6.3 增加测试：已闭合 code block 渲染结果冻结，后续 chunk 不会改变历史块（避免重复高亮/重渲染）
- [ ] 6.4 增加测试：`--plain` 模式下 fences 原样可见且不产生高亮 ANSI
- [ ] 6.5 跑全量门禁：`cargo fmt --check`、`cargo clippy --all-targets --all-features -- -D warnings`、`cargo test`、`cargo test -p ralph-core smoke_runner`
