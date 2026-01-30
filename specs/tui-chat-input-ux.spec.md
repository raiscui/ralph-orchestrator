# Spec: 并行 Supervisor TUI 的 Chat 输入体验改良

## 背景 / 问题

用户在并行 Supervisor TUI（`ralph run` + parallel + TUI）里使用 chat 输入框时，遇到 3 个体验问题：

1) `Shift+Enter` 期望用于“换行”，但在实际终端环境里无效（表现为：仍触发提交/发送）。

2) chat 输入框是多行高度，但当只输入一行（例如 `@writer#1 hello`）时，文本贴着输入框上沿，视觉上“太靠上”。

3) 并行模式的流式输出默认隐藏 stderr，不利于调试；期望默认显示 stderr。

---

## 目标（Goals）

### G1: 可用且稳定的多行输入

并行模式的 chat 输入框 **必须** 支持用户输入多行内容，并能明确区分“提交”和“换行”：

- `Enter`：提交（发送事件）
- `Shift+Enter`：换行（若终端能够区分）
- **必须**提供至少一种在常见终端里稳定可用的换行 fallback（例如 `Alt+Enter` 或 `Ctrl+J`）

### G2: 输入框内容下移（更符合聊天输入区直觉）

当输入内容行数不足输入框高度时，chat 输入框内的内容与光标位置 **应该**更靠近底部（下移），而不是贴着顶部渲染。

要求：

- 输入内容“靠近底部”不等于“贴着底线”：当输入框高度允许时，**应该**在底部保留 1 行呼吸留白，
  让输入内容不要紧贴下方的 `Targets:` 行（更符合聊天输入区的视觉节奏）。
- 光标所在行始终可见
- 鼠标 hit-test（点击定位/拖拽选择）与渲染对齐策略保持一致（避免“点到 A 选中 B”）

### G3: 默认显示 stderr

并行模式下，Supervisor 的输出观察者 **必须**默认把 stderr 流式行也送进展示层（TUI / log mode）。

同时 **必须**保留一个显式开关，让用户可以在需要“降噪”时隐藏 stderr。

---

## 非目标（Non-Goals）

- 不要求实现类似 IDE 的复杂多行编辑器能力（例如：跨行剪贴板、undo/redo、语法高亮）。
- 不改变并行模式的事件解析策略：事件解析仍以 stdout 为准（避免 stderr 的 `<event ...>` 假事件污染）。

---

## 设计要点（Design Notes）

1) 终端对 `Shift+Enter` 的支持存在差异，不能把“可换行”完全绑定在 `KeyModifiers::SHIFT` 能否被捕获上。

1.1) 为了尽可能让 `Shift+Enter` 在“支持的终端”里真正可被区分，TUI **应该** best-effort 启用 crossterm 的 kitty keyboard protocol：
- `PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)`
- 退出时配对 `PopKeyboardEnhancementFlags`，避免污染用户终端状态

2) 输入框“下移”的实现应以“渲染对齐”解决为主（不改 state 数据结构），同时保证 mouse hit-test 与渲染一致。

3) 默认显示 stderr 属于 CLI 层面的“默认值策略”调整；实现应尽量保持 runner/TUI 逻辑不复杂化。

---

## 验收标准（Acceptance Criteria）

- 在并行 Supervisor TUI 的 chat 输入框内：
  - `Enter` 会提交并清空输入（原行为保持）
  - 至少一种组合键能稳定插入换行（不提交）
  - 当输入只有 1 行时，内容视觉上位于输入框更靠下的位置（不是贴顶），并且与下方 `Targets:` 行之间有明显留白（不贴在一起）

- 并行模式默认展示 stderr：
  - 不带任何额外参数运行并行模式时，stderr 行会被显示
  - 仍存在一个开关可隐藏 stderr（开关名以实现为准，但必须可用且在 `--help` 中可发现）

- `cargo test` 全通过（包含 replay smoke tests）。
