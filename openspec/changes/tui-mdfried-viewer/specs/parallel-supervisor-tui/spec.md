## ADDED Requirements

### Requirement: 输出视图支持 Big Headers（接近 mdfried）
Supervisor TUI 的实例输出视图 MUST 在启用 Markdown 渲染（非 `--plain` 且内容不含 ANSI）时，将 Markdown 标题（至少 H1/H2/H3）渲染为“更大”的标题块，以提升长文档的扫读效率。

#### Scenario: 支持图形协议时标题占用多行
- **WHEN** 用户在并行模式的 Supervisor TUI 中查看某个实例输出，且当前终端支持图形协议或等效的大标题渲染能力
- **THEN** H1/H2/H3 标题在输出视图中呈现为“明显大于普通文本”的标题块（例如占用多行高度），而不是仅以普通一行文本显示

#### Scenario: 不支持图形协议时稳定降级
- **WHEN** 用户在并行模式的 Supervisor TUI 中查看某个实例输出，但终端不支持任何图形协议（或探测失败）
- **THEN** 系统自动降级为现有的纯文本/Markdown 样式渲染（不崩溃、不空白、不丢内容）

---

### Requirement: 输出流标识不得破坏 Markdown 解析
Supervisor TUI MUST 将输出流标识（stdout/stderr）视为“展示元信息”，并且不得把该标识拼接进参与 Markdown 解析的正文内容中。

#### Scenario: stderr 前缀不影响标题/列表语义
- **WHEN** 某条 stderr 输出行以 `#` / `>` / `-` 等 Markdown 行首语法开头
- **THEN** 输出视图仍能按 Markdown 语义正确渲染该行（例如标题/引用/列表），并且用户仍能通过 UI 看出它来自 stderr

---

### Requirement: 图片内联默认关闭且可安全降级
Supervisor TUI MUST 默认关闭 Markdown 图片语法 `![]()` 的远程图片下载与内联渲染能力，并在显式开启后仍必须具备安全降级（不崩溃、不阻塞 UI）。

#### Scenario: 默认不下载远程图片
- **WHEN** 输出包含 `![](https://...)` 或其他远程图片链接
- **THEN** 系统默认不发起网络下载，并以 alt 文本/链接占位（或纯文本）呈现

#### Scenario: 显式开启后渲染失败可回退
- **WHEN** 用户显式开启图片内联渲染，但图片下载/解码/渲染失败（或超出大小上限）
- **THEN** 系统回退为文本占位显示，并保持 TUI 可交互（不出现长时间卡顿）

