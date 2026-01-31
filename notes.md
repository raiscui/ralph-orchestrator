# 笔记：TUI 背景色一致性（左右侧发灰 vs 面板间深色）

## 状态
- 创建时间：2026-01-30 16:23 +0800
- 说明：旧的 `notes.md` 已达到行数阈值，已轮转为 `notes_2026-01-30_1623.md`。

## 现象
- 用户反馈：TUI 左右两侧背景“发灰”，与 chat / output 之间的深色背景不一致。
- 期望：统一为一致的深色（终端不支持 alpha，“半透明”用更深的同色系背景近似）。

## 初步假设（待代码核对）
- 很可能是 exabind 边框字符（`▏` / `▕`）所在 cell 的 `bg` 仍在使用 panel 内部背景 `base`。
- 而 panel 外侧/分隔区域使用了 app 背景 `crust`，导致对比下左右边框一条“偏灰”。

## 代码核对（确认根因）
- `panel_block(...).style(theme.panel_bg())` 会把整个 panel area（含边框 cell）背景刷成 `base`。
- 目前 `patch_exabind_panel_border_bg` 只把“底边整行 + 左上角”刷回 `crust`。
- 因此：竖边（`▏`/`▕`）所在列仍是 `base`，视觉上更“灰”，与已经变深的分隔线（bottom row=crust）不一致。

## 需求补充（2026-01-30 16:26）
- 用户明确：需要“最外圈的整个一圈”都是深色。
- 解释：这等价于把 panel 的 **top row / bottom row / left col / right col** 全部刷成外侧背景（目前用 `crust` 近似“半透明深色”）。

## 观察：截图中的“灰色外圈”可能不在字符栅格内
- 用户截图里，灰色区域更像是终端/窗口的 padding（非字符栅格区域），而不是 ratatui 的 cell。
- ratatui 只能控制“整格 cell”的前景/背景色；如果灰色带是像素级连续边缘（不按整格变化），那它就不可能通过 TUI 代码改掉。
- 需要确认用户使用的终端（iTerm2/WezTerm/Kitty/Warp 等）是否开启了 window padding / 透明背景；这类 padding 区域使用终端自身背景色/透明度。

## 方案2细节：xterm OSC 背景色
- 设置默认背景色：`ESC ] 11 ; #RRGGBB ESC \\`
- 恢复默认背景色：`ESC ] 111 ESC \\`
- 备注：`ESC \\` 是 ST 终止符（相当于 BEL 终止），更适合避免铃声。

## Warp：为什么“UI 外圈”会突然变灰（推断）
- Warp 的 padding 属于“字符栅格之外”，理论上不受 ratatui 控制。
- 但用户反馈“以前半透明正常、exabind 改造后变灰”，更像是：
  - 之前我们较多使用 `Color::Reset`（不画显式背景），Warp 的窗口半透明效果会覆盖内容区与 padding；
  - 现在我们在内容区大量使用显式 `bg`（crust/base），Warp 仍保持 padding 的半透明，但内容区变成纯色不透明，二者一对比就像外圈灰了一圈。
- 因此更稳的修复方向是：Warp 环境下让内容区回到“使用终端默认背景”（bg=Reset），让半透明再次统一。

## 结论与落地（2026-01-30 20:51 +0800）
- 结论：Warp 的“外圈灰色”本质上是 **内容区被我们画成不透明纯色** 之后，和 Warp 自带的半透明 padding 产生了对比错觉。
- 放弃：尝试用 xterm `OSC 11/111` 去改默认背景色属于 best-effort，但在 Warp 的 padding 上不稳定/不生效，不能作为可靠修复。
- 落地：在 Warp + TTY 下启用“终端默认背景模式”（`bg=Reset`）：
  - 内容区与 padding 共享同一套 Warp 半透明背景；
  - 同时保留 exabind 边框 glyph 与其余语义色（fg/selection/search）不变。
- 代码要点：
  - `crates/ralph-tui/src/theme.rs`：新增 `use_terminal_default_bg`，并集中提供 `app_bg_color()`/`panel_bg_color()`
  - `crates/ralph-tui/src/app.rs`：检测 Warp（`TERM_PROGRAM` 包含 `warp`）且 `stdout` 为 TTY 时启用该模式
  - `crates/ralph-tui/src/widgets/header.rs`、`crates/ralph-tui/src/widgets/footer.rs`、`crates/ralph-tui/src/animation.rs`：背景统一改用 `theme.app_bg_color()`，避免 Warp 下强行刷纯色
- 验证：`cargo test -p ralph-tui` ✅；`cargo test` ✅；`cargo test -p ralph-core smoke_runner` ✅。

## 补充（2026-01-30 21:47 +0800）：为什么 Output 重启动画会“带动全屏背景变动”
- 现象：在 Warp 透明背景模式（`bg=Reset`）下，只有 Output pane 的“重启动画”会让人感觉全屏背景在变暗/变色。
- 关键原因不是“Output pane 的 area 画错了”，而是 **tachyonfx 对 `Color::Reset` 的处理方式**：
  - tachyonfx 在做颜色插值时会把 `Color::Reset` 映射为 RGB(0,0,0)（黑色）。
  - `sweep_in/out` 的中间帧还会把 `cell.bg==Reset` 临时当作 Black 来参与 lerp（否则无法插值）。
- 结果：当 Output pane 占屏幕大部分时，动画遮罩会把大面积区域短暂“插值成黑色背景”，肉眼就会认为“全屏背景在动”。
- 修复策略：在 `bg=Reset` 模式下，Output 重启动画改为 `dissolve_to + coalesce_from`（带 `SweepPattern`），避免任何颜色插值。

## 补充（2026-01-30 21:55 +0800）：为什么启动进场动画在 Warp 下会“先显示全部再逐个动画”
- 现象：你希望“在进场动画开始前，所有 block 都是不可见的”；但在 Warp 透明背景模式下，会先看到完整 UI，然后才逐块出场，观感像“先显示 → 再遮罩/再揭开”。
- 根因（机制）：我们原来的启动动画使用 `sweep_in + fade_from_fg`，它本质只改 fg/bg，不改 symbol：
  - 在非透明背景（bg=crust/base）里，`fg/bg` 被刷成同色时文字会“融进背景”，看起来像隐藏；
  - 但在透明背景（`bg=Reset`）里，`fg/bg=Reset` 仍会显示终端默认前景色 → 内容依然可见，所以就出现了“先看到全部”的现象。
- 修复策略：在 `bg=Reset` 模式下，用“基于 symbol 的遮罩”替代“基于颜色的遮罩”：
  - 改用 `slide_in`（它会把未揭开的区域的 cell 逐步变为空格），因此起步态是真正的空屏；
  - 并用 `prolong_start` 做严格串行：Instances(frame) → Instances(items) → Output → Chat/Gates。


## 补充（2026-01-30 22:28 ）：启动入场动画必须从空屏开始

- 用户期望：刚打开时先不显示任何 block（包含 header/footer/panes），随后再逐块入场。
- 问题：并行模式的启动动画只覆盖 content panes，header/footer 首帧仍会先被渲染出来。
- 修复：把 header/footer 也纳入 `startup_open_effect_parallel` 的遮罩范围（bg=Reset 下使用 `slide_in` 做 symbol 遮罩），并增加单测锁定“首帧空屏”。


## 补充（2026-01-31 01:43 ）：优化 Warp 透明模式下的“刷白条”观感

- 背景：为了避免 `Color::Reset` 在 tachyonfx 的 `sweep_in/out` 里被当作黑色插值，之前把 Output 重启动画换成了 `dissolve/coalesce`。
- 代价：`dissolve/coalesce` 是噪点式的消失/出现，观感不像以前那种连续的白条扫入。
- 改良：在 `bg=Reset` 模式下把 Output 重启动画换成 `slide_out + slide_in`：
  - 仍然是 symbol 遮罩（不会触发 Reset→Black）
  - 但视觉上更像“白条扫过”而不是随机噪点。
- 额外调参：给 slide 的 `gradient_length` 加上限（`SYMBOL_SWEEP_GRADIENT_MAX=10`），避免渐变带太厚导致“糊”。


## 补充（2026-01-31 02:08 ）：恢复更像原 sweep 的渐变（降低眩光）

- 目标：既要保留 Warp 透明背景下“整屏不闪”，又要让 Output 重启动画更像原来的 `sweep` 渐变，并且不要出现刺眼的纯白条。
- 关键改动：Warp 模式下不再把 pane 内部背景设为 `Reset`，而是保留主题底色（Catppuccin `base`）。
  - 好处1：文字阅读更稳定（不受 Warp 背景纹理/blur 影响）。
  - 好处2：tachyonfx 的 `sweep_in/out` 不再遇到 `cell.bg==Reset` 的分支，从而避免 Reset→Black 导致的“全屏背景变动”。
- 动画改动：`output_reopen_effect` 在 Warp 模式下恢复 `sweep_out/sweep_in`（faded_color=base），并只作用于 Output 的 inner area，避免边框 cell（bg=Reset）参与插值。


## 补充（2026-01-30 23:10 ）：为什么 Output 动画会把“最外圈”也染上底色（推断）

- 现象：用户希望“pane 内部有底色，但最外圈保持透明”；Instances pane 动画完成后外圈仍透明，但 Output 动画期间最外圈会被染成同样的底色。
- 关键机制：在 Ralph 里动画是“后处理（shader-like）”，顺序是：
  1) 先渲染 widgets（包括 exabind 边框与 `patch_exabind_panel_border_bg`）
  2) 再由 `EffectManager::process_effects` 对 buffer 做动画处理
- 这意味着：如果某个 effect 在某一帧写到了 pane 边框 cell（尤其是边框的 `bg=Reset` 区域），它会覆盖掉我们前面刚刷回去的 `bg=Reset`，导致外圈被染色。
- Instances 不会“长期触发”这一问题，通常是因为：
  - 它没有像 Output 那样频繁触发“重启动画”，且 Warp 模式下启动进场用的是 `slide_in`（symbol 遮罩，不做 Reset→Black 颜色插值）。
- 下一步验证点（准备通过代码与单测锁死）：
  - 在应用 effects 之后，再执行一次 `patch_exabind_panel_border_bg`，确保外圈背景永远以 `theme.app_bg_color()` 为准（Warp 下就是 `Reset`）。


## 补充（2026-01-31 03:05 ）：切换 Instances 时 Output “先显示再消失”闪烁

### 现象
- 切换 Instances 选中项时：
  - Output 会先把“新实例的内容”显示出来一帧
  - 然后再进入“消失→入场”的动画
- 观感就是你说的“闪一下”，尤其是输出内容很亮/很多行时非常明显。

### 根因（机制层）
- Output 重启动画之前是 `sweep_out + sweep_in`：
  - `sweep_out` 是 `sweep_in(...).reversed()`，timer 反向导致首帧 `alpha=1`（也就是“完全可见”）。
- 我们的渲染顺序是：先用“新选中实例”渲染 Output 内容 → 再应用 effect：
  - 所以首帧会先看到新内容；
  - 接着 `sweep_out` 才开始把它盖掉，于是形成“先显示一帧再消失”的闪烁。

### 修复思路
- 关键不是“换别的动画”，而是保证 **动画刚添加的那一帧**：
  - effect 必须从初始态起步（不能被 `fx_delta` 推到中途）
  - 并且首帧应该是隐藏态

### 落地实现
- 把 Output 重启动画简化为 `sweep_in`（从隐藏态揭开）。
- 在添加 Output 重启动画的那一帧，强制 priming：
  - 把 `fx_delta=0`，确保首帧就是隐藏态，不会先露出一帧内容。


## 补充（2026-01-31 12:00 +0800）：ContentPane 清空逻辑会“抹掉 pane 底色”

### 现象（用户感知）
- 你希望 “block 有底色，但外圈保持透明”。
- 实际上 Output 在动画后，有时会让外圈看起来也被同样的底色影响。
- 同时你反馈“刷白条太晃眼”，这通常发生在底色不稳定（bg=Reset）+ 动画插值强对比时。

### 根因（代码层）
- `panel_block(...).style(theme.panel_bg())` 会先把 pane area 铺成 `base` 底色。
- 但 `ContentPane` 在渲染时，会大量“清空/覆盖”cell：
  - 如果清空时使用 `Cell::reset()` 或写入 `Style::default()`，很容易把 bg 恢复成 `Reset`，
    导致 pane 内部底色被抹掉。
- 一旦 bg 回到 `Reset`，在 Warp 透明模式下就会：
  - 变得更“通透/发灰”（底色丢失）；
  - 动画（sweep）插值更容易产生高对比白条，甚至触发 Reset→Black 的副作用。

### 修复策略（可验证）
- ContentPane 先从“当前区域左上角 cell”读取 `base_bg`（由外层决定：base/crust/Reset）。
- 用 `base_style = theme.text().bg(base_bg)` 先铺满区域，防止残影且不破坏底色。
- 渲染每个 grapheme 时用 `base_style.patch(span.style)` 合并样式：
  - span 若没有 bg，就保留 `base_bg`；
  - span 若没有 fg，就使用主题默认 text 色。
- 宽字符 continuation cell 写入 `symbol==""`，避免终端宽度对齐异常。
- selection 最后统一 overlay，确保空白处也能正确高亮。

### 验证
- `cargo test -p ralph-tui` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test` ✅
