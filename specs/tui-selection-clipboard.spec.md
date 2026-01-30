# Spec: TUI 选择区域复制/粘贴（Clipboard 集成）

## 背景 / 问题

并行 Supervisor TUI 支持鼠标框选输出（蓝色高亮）。
但用户在 macOS 上框选后按 `Command+C` / `Command+V` 无法完成复制粘贴闭环。

常见原因是：

- TUI 处于 raw mode 且开启 mouse capture 时，终端模拟器的“原生文本选择”不会生效；
- 应用内的“高亮选择”只是 UI 状态，除非应用主动写入剪贴板，否则 `Cmd+C` 并不会拿到任何文本；
- 若终端使用 bracketed paste，crossterm 可能产生 `Event::Paste`，若应用忽略该事件则粘贴无效。

---

## 目标（Goals）

### G1: 框选 → 可粘贴（Clipboard 闭环）

当用户在输出面板内完成一次鼠标框选（MouseUp）后，TUI **必须** best-effort 将“所见即所得”的选中文本写入系统剪贴板。

效果：

- 用户无需额外按键；
- 随后在任意应用中 `Command+V` 可粘贴出文本；
- 在终端内也可直接 `Command+V` 将文本粘贴回 chat 输入框。

### G2: 显式复制快捷键

TUI **必须**提供一个显式复制快捷键（例如 `y`），用于：

- 重新复制当前选择；
- 终端/环境不允许自动复制时的兜底；
- 提升可发现性（配合 help overlay / status line 提示）。

### G3: 支持 Paste 事件

当 crossterm 上报 `Event::Paste(text)` 时：

- 在 search 输入模式：追加到搜索输入；
- 在并行 chat 输入框聚焦时：把 `text` 插入到 chat editor（保持换行）；
- 其它场景：可忽略。

---

## 非目标（Non-Goals）

- 不要求实现复杂的“系统级鼠标选择”（那是终端模拟器能力，不是 TUI 应用能力）。
- 不追求跨平台 100% 一致的剪贴板行为；但必须保证在 macOS 上可靠可用。

---

## 设计要点（Design Notes）

1) **选中文本的获取**：输出面板的选择是矩形选择（屏幕坐标），应以“渲染后的字符网格”为准提取文本（所见即所得）。

2) **剪贴板写入策略**（best-effort，多后端兜底）：
   - 优先：OSC52（写入终端剪贴板，适合远程/跨平台）
   - macOS 兜底：`pbcopy`（写入系统剪贴板，保证 `Cmd+V` 可用）

3) **失败可解释**：复制失败时在 status line 提示失败原因；成功时提示复制的字符数（必要时提示截断）。

---

## 验收标准（Acceptance Criteria）

- 框选输出结束（MouseUp）后，能够在 macOS 上 `Cmd+V` 粘贴出所选文本。
- `y` 能把当前选择写入剪贴板（无选择时提示“no selection”）。
- `Event::Paste(text)` 在 chat 输入框聚焦时会插入文本；在 search 模式会追加搜索输入。
- `cargo test` 全通过（包含 replay smoke tests）。
