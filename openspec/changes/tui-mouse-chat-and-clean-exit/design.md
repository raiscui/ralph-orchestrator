## Context

当前并行模式的 Supervisor TUI 主要目标是“可观测 + 最小控制面”，实现位于：

- `crates/ralph-tui/src/app.rs`：输入事件循环 + 布局渲染
- `crates/ralph-tui/src/state/parallel.rs`：并行模式状态（实例列表/输出/job/gates/chat）
- `crates/ralph-cli/src/parallel_runner.rs`：并行运行时 + TUI 生命周期（terminated/interrupt 信号）

现状关键点（与本次变更强相关）：

- 鼠标目前只用于滚轮滚动（`MouseEventKind::ScrollUp/ScrollDown`），不支持点击选择实例、也不支持点击进入 chat。
- chat 输入是单行字符串（`ParallelTuiState.chat_input: String`），不支持多行、光标移动、选择/框选。
- 输出渲染是只读 `ContentPane`（`crates/ralph-tui/src/widgets/content.rs`），支持滚动与搜索高亮，但没有“文本选择”模型。
- 并行 runner 的 Ctrl+C 会通过 `interrupt_tx` 触发 killpg（SIGTERM → SIGKILL）来清理子进程组。
  但在 TUI 内按 `q` 退出时，目前只会退出 TUI，本身不会自动触发 supervisor/worker 的统一退出。

约束：

- TUI 仅在 stdin/stdout 都是 TTY 时启用（并行 runner 已强制）。
- 需要尽量复用现有架构（observer → `TuiUpdate` → reducer），避免把调度逻辑塞进 UI。
- 需要兼容并行模式的日志输出（`--no-tui`）路径，且不改变外部事件格式（`human.message` / `gate.resolve`）。

## Goals / Non-Goals

**Goals:**

- 支持鼠标点选实例列表项，切换“当前选中实例”，并与现有键盘选择逻辑一致。
- 支持输出视图的“文本选择”：
  - 支持多行选择
  - 支持鼠标框选与键盘选择（最小可用：Shift+方向键扩展选择）
- 支持点击 chat 区域进入输入态，并将 chat 输入升级为“像终端一样的输入窗口”：
  - 显示提示符（prompt）
  - 支持鼠标/键盘移动光标（左右/上下，鼠标点击定位）
  - 支持鼠标/键盘框选选择
  - 支持 `Shift+Enter` 换行（多行输入）
- 当用户从 TUI 主动退出（例如按 `q`）时，必须退出所有并行 worker 的 headless CLI 子进程，避免残留。

**Non-Goals:**

- 不做“完整文本编辑器”（例如 Vim 全套命令、复杂撤销树、语法高亮等）。
- 不引入复杂的 UI 组件系统（右键菜单、拖拽窗口、弹窗编辑器等）。
- 不强制实现跨平台系统剪贴板集成（如后续确有需要，再单独设计与实现）。

## Decisions

### 1) chat 输入：自研最小多行编辑模型（而非引入外部 textarea 依赖）

**选择：** 在 `ralph-tui` 内实现一个最小可用的多行编辑状态机（例如 `ChatEditorState`），替换 `ParallelTuiState.chat_input: String`。

**原因：**

- 本项目已有清晰的 state 模型（`ParallelTuiState`），自研可以把“光标/选择/多行”作为一等状态，便于测试。
- 避免引入新依赖带来的维护与行为差异（尤其是键位/鼠标事件的跨终端差异）。

**备选：**

- 引入第三方 textarea 组件库（可显著减少编辑器工作量）。
  但会带来额外依赖、键位/渲染一致性风险，以及与现有 focus/事件循环的整合成本。

**落地要点（最小集合）：**

- 缓冲区：`Vec<String>` 或 `String + 行索引`（推荐 `Vec<String>`，便于上下移动光标）。
- 光标：`(row, col)`，`col` 以 grapheme cluster 计数（避免 CJK/emoji 宽度错位）。
- 选择：可选 `Selection { anchor: (row, col), cursor: (row, col), mode: Linear|Rect }`
- 提交规则：`Enter` 提交消息；`Shift+Enter` 插入换行。

### 2) 鼠标事件路由：基于“最后一次布局 Rect”的 hit-test

**选择：** 渲染时计算 `instances_area/output_area/bottom_area` 的 `Rect`，保存在 App 的局部状态中。
输入事件到来时，用鼠标坐标做 hit-test：

- 点击实例列表区：计算点击的行 → 更新 `selected_instance` → 聚焦保持或切到 Instances
- 点击输出区：聚焦 Output（不改变实例选择）
- 点击 chat 区：聚焦 Chat；若点在输入框区域内，同时更新 chat 光标位置

**原因：**

- 当前布局在 `app.rs` 内集中定义，Rect 已经天然可得。
- 不需要在 state 内引入布局依赖，保持 reducer 的“纯数据”特性。

### 3) 输出选择：先实现“屏幕坐标系选择”，再演进到“逻辑文本坐标系”

**选择：** 第一版把输出选择建模为“当前可视区域的屏幕坐标选择”（x/y，相对 output inner area）。
渲染 `ContentPane` 时，根据选择范围对 cell 施加反色/背景色样式，达到“可见可选”的效果。

**原因：**

- 当前 `ContentPane` 做了软换行与 grapheme 渲染，想精确映射回原始逻辑文本会复杂很多。
- 屏幕坐标的选择能满足“框选/多行选择”的最小需求，并且实现路径清晰。

**备选：**

- 直接对 `IterationBuffer` 的逻辑行/列做选择并映射渲染。
  这需要对软换行、宽字符占位、不可见控制字符过滤等规则做统一抽象，工作量更大。

### 4) 退出语义：TUI quit 与 interrupt 走同一条“全局终止”通道

**选择：** 当用户在 TUI 内触发退出（`q`）时：

- 不仅退出 UI，还应通过 `interrupt_tx` 通知并行 runner
- 并行 runner 复用既有 interrupt 分支：killpg（SIGTERM → SIGKILL）清理整个子进程组
- 同时 `terminated_tx.send(true)`，确保 TUI 不会卡住等待

**原因：**

- 已有“可靠清理子进程组”的实现（Ctrl+C 分支）。
- 复用同一路径能减少特殊情况，并保证退出后不会遗留 worker CLI 进程。

### 5) Chat/Gate 快捷交互：默认目标 + Targets chips + Gate actions

**选择：** 将并行 TUI 的 chat 从“纯文本输入框”升级为“可点击的 human-in-loop 控制面”，核心包含三点：

1) **默认消息目标**：用户不写 `@...` 前缀时，消息默认发给当前 `selected_instance`。  
2) **Targets chips（展示全部实例）**：在 chat 面板展示所有实例（如 `@writer#2`），鼠标点击即可切换 `selected_instance`。  
3) **Gate 交互增强**：
   - gate 列表行可点击，点击后把该 gate 设为 `selected_gate`；
   - 同时自动切换 `selected_instance = gate.requested_by`，让用户立刻看到该实例输出并回复；
   - 在 chat 面板展示当前 gate 的关键信息（prompt 等），并提供 `!approve/!deny/!resolve` 的可点击 actions chips，点击后只做“预填输入框”（不自动发送）。

**原因：**

- human-in-loop 的关键成本不在“能不能写事件”，而在“人类指令如何快速、准确、可控地路由到正确的并行实例/正确的 gate”。
- 让默认消息带 `target_instance` 能避免 `human.message` 发生意外广播（即便未来所有 hats 默认订阅该 topic）。
- gate 点击联动 `selected_instance` 能减少上下文切换：选 gate → 看输出 → 回复 gate，一次完成。

**落地要点（实现提示）：**

- 并行模式下为所有 hats 注入 `human.message` 订阅（运行时补齐），以通过 strict target 校验。
- Targets/Gate actions 的点击命中（hit-test）沿用“渲染时记录布局快照”的模式，避免把布局塞进 state。

## Risks / Trade-offs

- **[风险] 不同终端对鼠标/Shift 组合键支持不一致** → **缓解**：提供纯键盘路径；关键交互（选择实例/发送 chat/退出）必须有键盘兜底。
- **[风险] 输出选择与软换行/宽字符导致坐标映射复杂** → **缓解**：第一版锁定“屏幕坐标选择”，并明确只保证对“当前可见区域”的选择正确。
- **[风险] 退出时 killpg 可能误伤同一进程组内的其他子进程** → **缓解**：保持“ralph 自己建立独立进程组”的前提（`process_management::setup_process_group()`），并确保 worker 进程都在该组内。

## Migration Plan

- 无需数据迁移。
- 需要更新 TUI 帮助页/提示文案，说明：
  - 鼠标可点击实例与 chat
  - `Shift+Enter` 换行
  - `q` 退出将终止所有 worker CLI 进程

## Open Questions

- 输出选择的“复制/导出”动作是否需要在本次一并提供？
  - 例如：提供一个快捷键把选中文本写入剪贴板或写入文件并提示路径。
- chat 输入的提交键是否需要兼容更多习惯（例如 `Ctrl+Enter` 提交、`Enter` 换行）？
  - 目前按用户需求默认：`Enter` 提交，`Shift+Enter` 换行。
