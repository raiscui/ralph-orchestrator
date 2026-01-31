# WORKLOG

## 2026-01-30 15:25 +0800｜实施：tui-exabind-style（ratatui 外观升级 + 启动打开动画）

### 目标
- 把 Ralph 的 TUI 视觉风格升级到参考 `junkdog/exabind` 的基线：更锐利的框体、Catppuccin（Mocha）配色、启动打开动画。
- 保持“可用性优先”：动画可禁用/可降级，不影响输入与稳定性。

### 我做了什么
- 主题（Theme）
  - 新增 `TuiTheme`（语义化 roles），默认使用 Catppuccin Mocha。
  - 把散落在各 widget 的颜色/强调色，统一收敛到 Theme（避免漂移）。
- 框体（Frame）
  - 新增 exabind 风格的 `border::Set`（`▟▜▔▏▕`）。
  - 新增 `panel_block(title, focused, theme)` 统一面板：border_set、标题样式、focus 边框、背景色。
- 启动打开动画（Open Animation）
  - 引入 `tachyonfx`，在 `App` 渲染循环里维护 `EffectManager`。
  - 进入 alternate screen 后 **只播放一次** 打开动画（≤500ms），动画结束后进入 steady-state，不阻塞输入。
  - 降级策略：
    - `RALPH_TUI_REDUCED_MOTION=1|true|yes|on` 禁用动画
    - stdout 非 TTY 自动禁用
    - 终端窗口过小（<60x12）自动禁用
- 测试与回归
  - `insta` 快照做了“边框字符归一化”，避免仅 border glyph 变化导致大量无意义 churn。
  - 更新 `examples/validate_widgets.rs`：输出写入 `target/tui-validation/`，可作为 `/tui-validate` 的稳定输入。

### 关键文件
- `crates/ralph-tui/src/theme.rs`
- `crates/ralph-tui/src/animation.rs`
- `crates/ralph-tui/src/app.rs`
- `crates/ralph-tui/tests/common/mod.rs`
- `crates/ralph-tui/examples/validate_widgets.rs`
- `.envrc`
- `openspec/changes/tui-exabind-style/tasks.md`

### 验证
- `cargo test -p ralph-tui` ✅
- `cargo test` ✅

---

## 2026-01-30 20:51 +0800｜修复：Warp 外圈 padding 发灰（改为终端默认背景模式）

### 现象
- Warp 终端里，TUI 之外（窗口 padding / 圆角外圈）出现一圈偏灰的背景。
- 该问题是 “exabind 风格 + 主题背景色” 变更后出现的；之前 Warp 的半透明背景看起来是统一的。

### 根因（本质）
- 这块区域不在 ratatui 的字符栅格里，我们无法“直接画到 padding”。
- 但我们可以避免制造对比：当内容区被我们大量刷成显式 `bg`（crust/base）后，内容区变成不透明纯色；
  Warp 的 padding 仍然是半透明窗口背景，于是两者并排时就显得“外圈灰了一圈”。

### 修复（更可靠的策略）
- 放弃 `OSC 11/111`：它属于 best-effort，在 Warp 的 padding 上不稳定/不生效，无法保证观感一致。
- 改为在 Warp + TTY 下启用“终端默认背景模式”（`bg=Reset`）：
  - `crates/ralph-tui/src/theme.rs`：`TuiTheme` 增加 `use_terminal_default_bg`，提供 `app_bg_color()`/`panel_bg_color()`；
  - `crates/ralph-tui/src/app.rs`：检测到 Warp（`TERM_PROGRAM` 包含 `warp`）且 stdout 为 TTY 时启用该模式；
  - `crates/ralph-tui/src/widgets/header.rs`、`crates/ralph-tui/src/widgets/footer.rs`、`crates/ralph-tui/src/animation.rs`：背景统一改为 `theme.app_bg_color()`，避免 Warp 下强行刷纯色背景。

### 验证
- `cargo test -p ralph-tui` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅

### 后续建议
- 如果你仍然看到 padding 发灰：这通常就是 Warp 的窗口主题/透明度/blur/padding 叠加效果本身了（属于 UI 范围外）。
  这时更稳的做法是把 Warp 的窗口背景色/透明度与期望的 `crust`（`#11111b`）对齐。

---

## 2026-01-30 21:47 +0800｜修复：Warp 透明背景下 Output 重启动画导致“全屏背景变动”

### 现象
- 现在全局背景已经透明（Warp 的半透明背景能透出）。
- 但唯独 Output pane 在切换实例触发“重启动画”时，会让人感觉全屏背景在变暗/变色。

### 根因
- tachyonfx 的 `sweep_in/out` 会对 fg/bg 做颜色插值。
- 在 tachyonfx 内部，`Color::Reset` 被映射为 RGB(0,0,0)（黑色）参与插值；`sweep_in/out` 中间帧还会把 `cell.bg==Reset` 临时当作 Black 参与 lerp。
- Output pane 面积很大，遮罩覆盖范围大，因此这种“插值成黑色”的副作用会被放大感知为“全屏背景在动”。

### 修复
- `crates/ralph-tui/src/animation.rs`
  - 当 `theme.app_bg_color()==Color::Reset`（Warp 透明背景模式）时：
    - `output_reopen_effect` 改用 `dissolve_to + coalesce_from`（带 `SweepPattern::up_to_down`）
    - 避免任何颜色插值，从根上消除“Reset 被插值成黑色背景”的问题
  - 新增回归测试：`output_reopen_effect_terminal_default_bg_does_not_paint_black_background`

### 验证
- `cargo test -p ralph-tui` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test` ✅


---

## 2026-01-31 12:00 ｜修复：pane 底色稳定 + Instances/Output 间隙分离

### 诉求（来自你的反馈）
- block 需要有底色（提升可读性、降低动画白条眩光）。
- 但最外圈（Warp 的半透明背景）必须保持透明，不要被 Output 动画“带着变色”。
- Instances pane 与 Output pane 之间需要间隙，避免边框贴合像 “collapsing borders”。

### 我做了什么
- `crates/ralph-tui/src/widgets/content.rs`
  - ContentPane 改为“先铺底再渲染”，并以区域左上角 cell 的 `bg` 作为基准底色：
    - 在 pane inner 区域里，这个 `bg` 通常是 Catppuccin `base`；
    - 在 app 空白区域里，这个 `bg` 可能是 `Reset`（Warp 半透明）。
  - 渲染时用 `base_style.patch(span.style)` 合并样式，避免把 pane 底色写回 `Reset`。
  - 宽字符 continuation cell 统一写 `symbol==\"\"`，避免宽度对齐异常。
  - selection 最后统一 overlay，保证空白处也能高亮。
- `crates/ralph-tui/src/app.rs`
  - 并行模式布局加入 `PARALLEL_PANE_GAP_WIDTH=1`：`instances | gap | output`。
  - 同步更新渲染与 Output 重启动画触发处的区域计算，避免 hit-test/动画错位。

### 验证
- `cargo test -p ralph-tui` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test` ✅


---

## 2026-01-30 23:15 ｜修复：Output 动画期间外圈被“染色”（Warp 透明模式）

### 你要的效果
- pane 内部可读性更好：允许有底色（`base`）。
- 终端最外圈保持 Warp 半透明：外圈应始终是 `bg=Reset`（透明）。

### 根因（为什么 Instances 没事但 Output 有事）
- Instances 主要受启动动画影响（Warp 下用 `slide_in` 的 symbol 遮罩，不做颜色插值）。
- Output 会触发 `sweep` 重启动画（会改 fg/bg）。
- 我们的渲染顺序是“先 widget → 后 effects”，因此动画可能覆盖掉边框外圈的 `bg=Reset` 修正，导致外圈被染色。

### 修复
- `crates/ralph-tui/src/app.rs`
  - 在 `EffectManager::process_effects(...)` 之后，再对 Instances/Output/Bottom 三个 pane 执行一次 `patch_exabind_panel_border_bg`（仅 Warp 模式 `bg=Reset`）。
  - 目的：无论动画怎么改 fg/bg，最终边框外圈都被强制刷回 `Reset`，外圈始终透明。
- `crates/ralph-tui/src/theme.rs`
  - 新增回归测试：`patch_exabind_panel_border_bg_restores_border_after_bg_mutating_effect`。

### 验证
- `cargo test -p ralph-tui` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test` ✅


---

## 2026-01-31 03:05 ｜修复：切换 Instances 时 Output 不再“先显示一帧”闪烁

### 问题
- 你切换 Instances 选中项时，Output 会先把新内容画出来一帧，
  然后才消失并开始入场动画。
- 这会造成非常明显的闪烁。

### 根因（核心机制）
- `sweep_out` 的首帧是“完全可见态”（timer reversed → `alpha=1`）。
- 但我们在这一帧里已经切换到新实例并把新内容渲染出来了。
- 所以必然出现“先可见一帧 → 再被盖掉”的闪烁。

### 修复
- `crates/ralph-tui/src/animation.rs`
  - Output 重启动画从 `sweep_out + sweep_in` 改为 `sweep_in + fade_from_fg`（从隐藏态揭开）。
- `crates/ralph-tui/src/app.rs`
  - 当 Output 重启动画被添加的那一帧，强制 priming：`fx_delta=0`，保证首帧从隐藏态起步。
  - priming 方法也用于启动入场动画（统一解决“首帧大 delta 导致从中途开始”的闪烁）。

### 验证
- `cargo fmt --check` ✅
- `cargo test -p ralph-tui` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test` ✅

---

## 2026-01-30 21:55 +0800｜修复：Warp 透明背景下启动进场动画“先显示全部再逐块出场”

### 现象
- 你期望：在启动进场动画开始前，所有 panes/blocks 都不可见（空屏），再逐块出场。
- Warp 透明背景模式（`bg=Reset`）下，之前会先看到完整 UI，然后才逐块动画，观感很违和。

### 根因
- 原启动动画使用 `sweep_in + fade_from_fg`，它只改 fg/bg，不改 symbol：
  - 在非透明背景时，fg/bg 刷成同色能“看起来像隐藏”；
  - 在 `bg=Reset` 时，fg/bg=Reset 仍会显示终端默认前景色 → 内容依然可见。

### 修复
- `crates/ralph-tui/src/animation.rs`
  - 当 `theme.app_bg_color()==Color::Reset` 时：
    - `startup_open_effect` 改用 `slide_in`（基于 symbol 的遮罩，起步态是真空屏）
    - `startup_open_effect_parallel` 改用 `slide_in + prolong_start` 组合：
      - Instances(frame) → Instances(items) → Output → Chat/Gates 严格串行
      - Instances(items) 用延迟启动，确保“先框后字”

### 验证
- `cargo test -p ralph-tui` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test` ✅

---

## 2026-01-30 16:23 +0800｜改良：统一 exabind 竖边背景（左右侧不再发灰）

### 现象
- TUI 左右两侧（竖边/分割线）背景偏灰，和 chat / output 之间的深色分隔不一致。

### 根因
- `▏` / `▕` 这类“细竖条”字形只占 cell 的一部分，剩余空白会用 **cell 的 bg** 填充。
- panel 边框 cell 默认继承 panel 内部背景 `base`，相比分隔处/外侧的 `crust` 更亮，因此看起来“发灰”。

### 修复
- `crates/ralph-tui/src/theme.rs`
  - 扩展 `patch_exabind_panel_border_bg`：把左右边框列的 `bg` 也刷回 `crust`。
  - 更新单测，断言边框列背景被刷成 `crust`，内部区域保持 `base`。

### 验证
- `cargo test -p ralph-tui` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅

---

## 2026-01-30 16:30 +0800｜改良：Warp 下窗口 padding（UI 范围外）背景尽量对齐 crust

### 现象
- Warp 终端里，TUI 之外（窗口 padding / 圆角外圈）仍然发灰。
- 用户还观察到：在启动/Output 动画时，这个外圈似乎也会跟着变色。

### 关键判断
- 这部分不在 ratatui 的字符栅格里，理论上“不能靠改 widget 的 bg 直接画到”。
- 但 Warp 的 padding 背景可能会跟随“终端默认背景色”（或透明/blur/vibrancy 的合成结果），因此可以尝试用 xterm OSC 同步默认背景色。

### 修复（best-effort）
- `crates/ralph-tui/src/app.rs`
  - 仅在 `stdout` 是 TTY 且检测到 Warp（`TERM_PROGRAM` 包含 `warp`）时：
    - 进入 alternate screen 后发送 `OSC 11`，把终端默认背景色设置为主题 `crust`
    - 退出时发送 `OSC 111`，恢复终端主题默认背景色
  - 写失败不会影响 TUI 运行（避免把“观感改良”做成硬故障）。
- 增加单测验证 OSC 转义序列格式（RGB hex + ST terminator）。

### 验证
- `cargo test -p ralph-tui` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅

---

## 2026-01-30 16:26 +0800｜改良：exabind panel 外圈“整圈”背景统一为深色

### 需求
- 用户明确希望 panel 的“最外圈整圈”（top/bottom/left/right）都是一致的深色背景。

### 修复
- `crates/ralph-tui/src/theme.rs`
  - 扩展 `patch_exabind_panel_border_bg`：补齐顶边整行 `bg=crust`，从而形成完整的 border ring 深色外圈。
  - 单测补充顶边整行断言，防止回归。

### 验证
- `cargo test -p ralph-tui` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅

---

## 2026-01-30 16:00 +0800｜修复：启动进场动画首帧先“全隐藏”再逐块出场（消除闪烁）

### 现象
- 启动 TUI 时会先出现完整 UI，然后进场动画才开始扫入，观感闪烁、很怪。

### 根因
- tachyonfx 的执行顺序是“先推进 timer，再执行 shader”。
- 如果首帧渲染被输入事件拖慢，首帧的 `fx_delta` 会偏大，导致启动动画从中途开始。

### 修复
- `crates/ralph-tui/src/app.rs`
  - 启动动画被添加的那一帧强制 priming：`fx_delta=0`，并重置 `last_effect_tick`。
  - 让启动动画首帧从“全隐藏起步态”开始，下一帧再正常推进时间轴。
- 新增回归测试：`app::tests::startup_animation_first_frame_priming_prevents_full_ui_flash`

### 验证
- `cargo test -p ralph-tui` ✅
- `cargo test` ✅

---

## 2026-01-30 14:00 +0800｜改良：对齐 exabind 边框“斜切角/底边贴边”细节

### 现象
- 使用 exabind 风格边框（`▟▜▔▏▕`）后，本地终端左上角看起来像“锯齿/缺口被糊住”，与 exabind 网页 demo 观感不一致。

### 根因
- `▟` / `▔` 这类块元素字形内部存在空白区域，空白区域会使用 **cell 的背景色** 填充。
- 我们的 panel 内部背景是 `base`（略亮），外部背景是 `crust`（更暗），导致本应透出外侧背景的区域被 `base` 填满。

### 修复
- `crates/ralph-tui/src/theme.rs`
  - 新增 `patch_exabind_panel_border_bg`：在渲染后把左上角 cell 与底边整行的 `bg` 刷回 `crust`，形成更干净的斜切角与贴边底线。
  - 增加单元测试，防止回归。
- `crates/ralph-tui/src/widgets/instances.rs`、`crates/ralph-tui/src/app.rs`
  - 在 Instances / Output / Chat-Gates 面板渲染后调用该 patch。

### 验证
- `cargo test -p ralph-tui` ✅
- `cargo test` ✅

### 后续建议
- 若仍觉得角不够“斜/顺”，优先检查终端字体是否使用 `JetBrains Mono`（以及是否发生了 fallback 字体替换）。

### 后续建议
- 如果你希望“更像 exabind”，下一步可以加：面板按顺序 reveal 的 stagger、或在 focus 切换时做轻量闪烁/呼吸边框（但依然要走 reduced-motion 降级）。

---

## 2026-01-30 15:55 +0800｜追加：逐块出场 + Output 重启动画（按左→右、上→下）

### 我做了什么
- 启动出场动画升级为“逐块串行”：
  - Instances（框体）→ Instances（条目）→ Output → Chat/Gates（从左到右、从上到下）
  - 节奏放慢，总时长 < 2s
- Instances 条目出场：
  - 框体阶段用 `paint` 把 inner 文本涂成同色（先框后字）
  - 框体结束后再对 inner 做 sweep-in + fade，让条目逐行出现
- Output 切换实例重启动画：
  - 监听 `selected_instance_id` 变化触发 unique effect
  - Output 先 sweep-out 消失，再 sweep-in + fade 出场（像“重新打开”）

### 关键文件
- `crates/ralph-tui/src/animation.rs:14`
- `crates/ralph-tui/src/app.rs:560`
- `openspec/changes/tui-exabind-style/specs/parallel-supervisor-tui/spec.md:10`
- `openspec/changes/tui-exabind-style/tasks.md:10`

### 验证
- `cargo test -p ralph-tui` ✅
- `cargo test` ✅


---

## 2026-01-30 22:28 ｜修复：并行启动入场动画首帧为空屏（含 header/footer），再逐块出场

### 现象
- 启动时会先看到完整 UI，再开始逐块入场动画，观感闪烁。

### 修复
- `crates/ralph-tui/src/animation.rs`
  - `startup_open_effect_parallel` 把 `header_area` / `footer_area` 也纳入 `bg=Reset` 分支的 Stage 1 遮罩。
  - 确保首帧是“真正空屏”，随后才逐块 reveal。
  - 新增单测：`startup_open_effect_parallel_terminal_default_bg_starts_from_blank_screen`。
- `crates/ralph-tui/src/app.rs`
  - 传入 `chunks[0]` / `chunks[2]` 以覆盖 header/footer。
- `openspec/changes/tui-exabind-style/specs/tui-exabind-style/spec.md`
  - 补充：启动动画必须从 fully hidden/blank state 开始；`bg=Reset` 下用 symbol 遮罩。
- `openspec/changes/tui-exabind-style/specs/parallel-supervisor-tui/spec.md`
  - 补充：并行启动首帧必须空屏（no pre-flash）。
- `openspec/changes/tui-exabind-style/tasks.md`
  - 追加并完成 3.7：启动动画从空屏起步。

### 验证
- `cargo test -p ralph-tui` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test` ✅


---

## 2026-01-31 01:43 ｜改良：Warp(bg=Reset) 下“刷白条”更像以前（不牺牲背景稳定性）

### 问题
- 为了修复 Output 动画导致整屏背景变动，我们在 `bg=Reset` 下禁用了 `sweep_in/out`，改用 `dissolve/coalesce`。
- 但 dissolve 的噪点式出现/消失，不如以前的连续白条扫入好看。

### 改良
- `crates/ralph-tui/src/animation.rs`
  - 新增 `SYMBOL_SWEEP_GRADIENT_MAX=10`，限制 `slide_in/out` 的渐变带宽度。
  - `bg=Reset` 时：Output 重启动画从 `dissolve_to + coalesce_from` 改为 `slide_out + slide_in`，观感更接近“白条扫过”。
  - 启动入场动画在 `bg=Reset` 时同样使用收窄后的 gradient，避免白条过厚。

### 验证
- `cargo test -p ralph-tui` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test` ✅


---

## 2026-01-31 02:08 ｜改良：Warp 透明模式下恢复 sweep 渐变（不再白条刺眼）

### 诉求
- 你希望动画更像原来 `sweep` 的渐变质感。
- 现在的白色太晃眼。
- 允许 block/pane 内部不透明，有底色。

### 我做了什么
- `crates/ralph-tui/src/theme.rs`
  - Warp 模式（`with_terminal_default_bg`）只把 app 背景交给终端（`bg=Reset`）。
  - pane 内部背景固定为主题 `base`，提升可读性并降低动画眩光。
- `crates/ralph-tui/src/animation.rs`
  - Output 重启动画在 Warp 模式下恢复 `sweep_out/sweep_in` 渐变（faded_color=base）。
  - 动画只作用于 Output 的 inner area，避免边框 cell（可能是 `bg=Reset`）参与插值导致闪烁。
- `crates/ralph-tui/src/app.rs`
  - 触发 Output 重启动画时传入 inner area（而不是整个 output area）。
- OpenSpec 同步：Warp 模式下 app bg 使用 `Reset`，pane bg 保留 `base`。

### 验证
- `cargo test -p ralph-tui` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test` ✅
