## ADDED Requirements

### Requirement: 输出视图默认渲染 Markdown
当 Supervisor TUI 展示某个 HatInstance 的输出内容时，系统 MUST 默认对该输出进行 Markdown 渲染，以提升可读性。
这里的“输出内容”指 TUI 详情区展示的 AI code agent CLI 输出文本本身。

Markdown 渲染 MUST 是 best-effort，并至少覆盖以下常见结构：
- `h1/h2` 标题（例如 `#` / `##`）
- `blockquote` 引用（例如行首 `>`）
- 代码（fenced code block 与 inline code）

当渲染成功时，详情区的呈现结果 MUST 体现 Markdown 语义并隐藏对应控制符（例如行首 `#` / `>`，以及 fenced code block 的 fence 标记）。
当渲染失败或遇到不支持/不完整结构时，系统 MUST 触发安全降级（见下一个 Requirement）。

#### Scenario: 默认渲染常见 Markdown 结构
- **WHEN** Supervisor TUI 启用且用户未传 `--plain`
- **WHEN** 选中实例输出包含标题、引用、以及代码块等 Markdown 结构
- **THEN** TUI 详情区以渲染后的富文本呈现该输出，并隐藏对应的 Markdown 控制符

---

### Requirement: Markdown 渲染失败必须安全降级
当输出包含不完整、无法解析或不支持的 Markdown 结构时，系统 MUST 安全降级为原始文本展示。
安全降级 MUST 满足：
- 不得 panic / 崩溃
- 不得丢失原始输出内容（允许原样展示 Markdown 控制符）
- 不得阻塞后续输出追加（仍可持续接收并展示后续输出）

#### Scenario: 不完整 Markdown 不导致崩溃且不丢内容
- **WHEN** Supervisor TUI 启用且用户未传 `--plain`
- **WHEN** 选中实例输出包含不完整或无法解析的 Markdown 片段（例如未闭合的代码块）
- **THEN** TUI 不崩溃，并展示该段原始输出内容（不丢失文本）

---

### Requirement: `--plain` 强制纯文本展示
当用户传入 `--plain` 时，Supervisor TUI MUST 禁用 Markdown 渲染，并按原始输出文本展示。
这意味着 Markdown 控制符（例如行首 `#`、行首 `>`、以及 fenced code block 的 fence 标记）MUST 按原样可见。

#### Scenario: `--plain` 时显示原始 Markdown
- **WHEN** 用户在启用 Supervisor TUI 的运行中传入 `--plain`
- **WHEN** 选中实例输出包含标题、引用、以及代码块等 Markdown 结构
- **THEN** TUI 详情区按原始文本展示输出，Markdown 控制符按原样可见

---

### Requirement: 渲染不破坏滚动与搜索
在默认 Markdown 渲染开启时，输出视图 MUST 仍然支持滚动与搜索（沿用既有的 `/` 搜索心智）。

#### Scenario: 渲染输出仍可搜索
- **WHEN** 默认渲染开启且输出中包含可见文本片段 `foo`
- **THEN** 用户使用 `/foo` 搜索后，系统能够命中该片段
