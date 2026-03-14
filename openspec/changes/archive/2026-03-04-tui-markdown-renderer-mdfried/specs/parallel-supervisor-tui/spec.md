## ADDED Requirements

### Requirement: 实例输出默认渲染 Markdown
当输出文本不包含 ANSI 转义序列且未启用 `--plain` 时，Supervisor TUI 的实例输出视图 MUST 以 best-effort 方式渲染 Markdown，以提升信息层级与可读性。

#### Scenario: 默认渲染 Markdown
- **WHEN** 用户在并行模式下启动 Supervisor TUI，并查看某个实例的输出（stdout/stderr）
- **THEN** 对于不包含 ANSI 的输出片段，TUI 将 Markdown 控制符（例如 `#`、`>`、fenced code block）以样式化方式呈现，而不是原样展示控制符

---

### Requirement: `--plain` 禁用 Markdown 渲染
当用户启用 `--plain` 时，Supervisor TUI 的实例输出视图 MUST 禁用 Markdown 渲染，并让 Markdown 控制符原样可见（用于排障、复制、对齐旧行为）。

#### Scenario: `--plain` 下控制符原样可见
- **WHEN** 用户以 `--plain` 启动并行模式的 Supervisor TUI
- **THEN** 输出中的 `#`、`>`、fenced code block 等 Markdown 控制符保持原样可见

---

### Requirement: 渲染失败必须安全降级
当 Markdown 输入不完整或解析失败时，Supervisor TUI 的实例输出视图 MUST 安全降级为纯文本显示，且不得 panic、不得丢失原始内容。

#### Scenario: 不完整 fenced code block 不导致崩溃
- **WHEN** 输出包含未闭合的 fenced code block（例如只有开头 ``` 但缺少结尾 ```）
- **THEN** TUI 不会崩溃（不 panic），并且仍能显示原始内容（允许以纯文本形式回退）

---

### Requirement: ANSI 输出优先保留样式
当输出文本包含 ANSI 转义序列时，Supervisor TUI 的实例输出视图 MUST 优先保留 ANSI 样式并跳过 Markdown 渲染，以避免颜色/格式信息被吞掉或误解析。

#### Scenario: 含 ANSI 的输出不做 Markdown 渲染
- **WHEN** 某个实例输出包含 ANSI 颜色控制（例如 `\u001b[31m...\u001b[0m`）
- **THEN** TUI 显示的颜色样式与 ANSI 语义一致，并且不会把其中的 Markdown 控制符当作 Markdown 再次渲染
