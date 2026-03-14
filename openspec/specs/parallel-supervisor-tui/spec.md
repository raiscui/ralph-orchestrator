# parallel-supervisor-tui

## Purpose
（TBD）并行模式（`parallel.enabled=true`）下的 Supervisor TUI 规格：
- TTY 环境启用 TUI，非 TTY 自动降级到日志模式
- 展示 HatInstance 列表并支持选择
- 展示选中实例输出，支持滚动与搜索
- 输出按 job 分段，并可在 job 历史间切换
## Requirements
### Requirement: 并行模式可启动 Supervisor TUI
当 `parallel.enabled=true` 且运行环境为可交互终端（TTY）时，系统 MUST 启动 Supervisor TUI 来展示并行实例状态与输出。
当不满足 TTY 条件时，系统 MUST 自动降级为日志模式，并给出清晰的降级原因。

#### Scenario: TTY 环境下启用 TUI
- **WHEN** 用户以并行模式运行，并显式启用 TUI（例如未传 `--no-tui` 且 stdin/stdout 为 TTY）
- **THEN** 系统启动 Supervisor TUI（而不是仅输出 “parallel mode has no TUI” 的警告）

#### Scenario: 非 TTY 环境下自动降级
- **WHEN** 用户在非交互环境运行（例如 stdout 不是 TTY）但请求启用 TUI
- **THEN** 系统不启动 TUI，并以日志提示“已降级为 log mode”与原因

---

### Requirement: Supervisor TUI 展示 HatInstance 列表并支持选择
Supervisor TUI MUST 展示当前已注册的 HatInstance 列表。
列表每一项 MUST 至少包含：

- `HatInstanceId`（例如 `writer#1`）
- 当前状态（例如 `Created/Idle/Running/Failed/Done`）
- 最近一次输出或事件的时间线索（用于“是否卡住”的快速判断）

Supervisor TUI MUST 支持通过键盘在实例列表中移动选择，并将“当前选中实例”同步到详情视图。

#### Scenario: 列表展示与选择同步
- **WHEN** Supervisor 已创建多个实例，并产生状态变更（Running/Idle/Done 等）
- **THEN** TUI 的实例列表可见这些实例及其状态
- **THEN** 用户切换选中项后，详情区展示对应实例的输出视图

---

### Requirement: Supervisor TUI 展示实例输出并支持滚动与搜索
Supervisor TUI MUST 按实例归因展示输出流。
输出视图 MUST 支持：

- 滚动查看历史输出
- 文本搜索（沿用现有 TUI 的 `/` 搜索心智）

#### Scenario: 输出实时追加且可搜索
- **WHEN** 某个实例持续产生 stdout/stderr 输出
- **THEN** TUI 详情区实时追加可见的输出内容
- **THEN** 用户输入 `/query` 后，TUI 能定位到匹配的输出行

---

### Requirement: 输出按 job 分段并可在 job 历史间切换
为了对齐并行模式的“多次 CLI invocation”语义，Supervisor TUI MUST 将每个实例的输出按 HatJob 分段。
同一实例的输出视图 MUST 支持在 job 历史之间切换，并明确显示当前正在查看的 job。

#### Scenario: 同一实例多次 job 的输出可分段查看
- **WHEN** 同一 HatInstance 连续执行两次及以上 HatJob
- **THEN** TUI 将其输出分成多个 job 段
- **THEN** 用户可以在这些 job 段之间切换查看

### Requirement: 鼠标点选实例
Supervisor TUI MUST 支持通过鼠标点击实例列表项来切换当前选中实例，并立即同步到输出视图。

#### Scenario: 点击实例列表切换选中项
- **WHEN** 用户用鼠标点击 instances 列表中的某一行
- **THEN** 该行对应的 `HatInstanceId` 成为当前选中实例
- **THEN** 输出面板标题与内容切换为该实例（并沿用该实例的当前 job 选择规则）

---

### Requirement: 输出视图支持文本选择与框选
Supervisor TUI MUST 在 Output 视图提供“文本选择”能力，允许用户选择跨多行的文本，并在界面中可视化高亮该选择区域。

#### Scenario: 鼠标拖拽框选多行文本
- **WHEN** 用户在 Output 视图按下鼠标并拖拽形成选择区域
- **THEN** TUI 用高亮样式标记被选择的文本（至少覆盖拖拽起止之间的多行）

#### Scenario: 键盘 Shift+方向键扩展选择
- **WHEN** 用户在 Output 视图通过键盘进入选择并使用 Shift+方向键扩展范围
- **THEN** 选择范围随光标移动而扩大或缩小，并在 UI 中持续可见

---

### Requirement: 选择状态与滚动/搜索可组合
Supervisor TUI MUST 允许在存在选择范围时继续使用滚动与搜索功能，并保证选择状态不会导致崩溃或输出错乱。

#### Scenario: 选择后滚动仍可用
- **WHEN** 用户已经在 Output 视图选中了一段文本
- **THEN** 用户继续滚动视图时，TUI 仍正常渲染并且不会 panic

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

### Requirement: Supervisor TUI uses the exabind-style theme
In parallel mode, the Supervisor TUI MUST apply the `tui-exabind-style` theme (colors + border glyph set) across header/footer and all major panes (Instances, Output, Chat, Gates).

#### Scenario: All panes share consistent framing and colors
- **WHEN** the Supervisor TUI is running in parallel mode
- **THEN** Instances/Output/Chat/Gates MUST use the same border glyph set and a consistent background strategy
- **THEN** focus border changes MUST follow the theme's focused/unfocused styles

#### Scenario: Adjacent panes are visually separated (no border collapsing)
- **WHEN** the Supervisor TUI is running in parallel mode
- **THEN** the Instances and Output panes MUST NOT have touching borders
- **THEN** there MUST be a visible gap between the two panes while preserving each pane's full border

#### Scenario: Warp preserves terminal-default background
- **WHEN** the Supervisor TUI is running in Warp (e.g., `TERM_PROGRAM` contains `"warp"`)
- **THEN** the TUI MUST use terminal-default background (`bg=Reset`) for the app background to preserve Warp's window transparency
- **THEN** panes MUST still use the theme panel background color for readability (e.g., Catppuccin `base`)
- **THEN** border glyphs and foreground colors MUST still follow the theme

---

### Requirement: Supervisor TUI plays an open animation on startup
When the Supervisor TUI starts (TTY + TUI enabled), it MUST run the `tui-exabind-style` startup open animation once before entering steady-state rendering.

#### Scenario: Startup animation is visible and bounded
- **WHEN** a user starts `ralph run` with `parallel.enabled=true` and TUI enabled
- **THEN** the TUI MUST show a brief open animation
- **THEN** the open animation MUST reveal panes sequentially (Instances → Output → Chat/Gates)
- **THEN** Instances list items MUST start appearing only after the Instances frame animation completes
- **THEN** after the animation completes, all panels MUST be fully rendered and interactive

#### Scenario: Startup begins from a blank screen (no pre-flash)
- **WHEN** a user starts `ralph run` with `parallel.enabled=true`, TUI enabled, and animations enabled
- **THEN** the first rendered frame of the open animation MUST be visually blank (no header/footer/panes visible)
- **THEN** panes MUST only become visible as the staged reveal progresses (never “fully visible first, then animated”)

---

### Requirement: Output pane reopens when switching instances
When the selected instance changes in the Supervisor TUI, the Output pane MUST play a re-open animation that hides the pane and then reveals it again.

#### Scenario: Switching instance triggers output re-open
- **WHEN** the user switches the selected instance in the Instances pane
- **THEN** the Output pane MUST briefly disappear
- **THEN** the Output pane MUST play an open animation to reveal the new output

#### Scenario: Output re-open begins from hidden state (no pre-flash)
- **WHEN** the user switches the selected instance in the Instances pane and animations are enabled
- **THEN** the Output pane MUST NOT render the new output fully visible before the re-open animation starts
- **THEN** the first frame of the re-open animation MUST be visually hidden (no “one-frame flash”)

## Change History
- 2026-01-28: Synced from `openspec/changes/archive/2026-01-28-parallel-supervisor-tui/specs/parallel-supervisor-tui/spec.md`.
