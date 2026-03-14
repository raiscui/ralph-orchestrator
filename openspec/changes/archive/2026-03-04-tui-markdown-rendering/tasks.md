## 1. CLI 开关与配置贯通

- [x] 1.1 在 `ralph run`（以及会启动 Supervisor TUI 的路径）新增命令行参数 `--plain`，并补充清晰的 help 文案（说明其用于禁用 Markdown 渲染）。
- [x] 1.2 将 `--plain` 贯通到 TUI 运行配置（例如 `TuiConfig` / `App` 初始化参数），并保证默认行为不变：默认渲染，`--plain` 强制纯文本。

## 2. 输出渲染模式（Rendered / Plain）

- [x] 2.1 为 TUI 输出渲染增加显式模式开关（例如 `render_markdown: bool` 或 `MarkdownRenderMode`），默认 Rendered。
- [x] 2.2 复用现有的 Markdown/ANSI 渲染能力：Rendered 模式继续走 `termimad`（best-effort），Plain 模式跳过 Markdown 渲染并保留 Markdown 控制符原样可见（同时保持 ANSI 处理逻辑不变）。
- [x] 2.3 将 Supervisor TUI 的“实例输出视图”统一接入同一套渲染函数与模式开关，避免并行/串行两套渲染逻辑分叉。

## 3. 回归测试（与 spec 场景对齐）

- [x] 3.1 增加单测：Rendered 模式下 `h1/h2`、`blockquote`、代码块能产生“语义化样式/隐藏控制符”的效果（以现有渲染库可观测的 style 断言为准）。
- [x] 3.2 增加单测：Plain 模式下 `#` / `>` / fenced code block 的控制符必须原样可见（不被渲染器吞掉）。
- [x] 3.3 增加单测：不完整/无法解析的 Markdown 输入时不会 panic，且内容不丢失（安全降级路径覆盖）。
- [x] 3.4 增加 CLI 层测试：传入 `--plain` 时渲染模式被正确设置并影响 TUI 输出渲染路径。

## 4. 验证与文档

- [x] 4.1 运行 `cargo test`（包含 replay smoke tests），确保无回归。
- [x] 4.2 补充文档：在 README 或相关 docs 中增加 `--plain` 的说明与使用场景（排障/复制粘贴/对齐旧行为）。
