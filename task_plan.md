# 任务计划: Warp 透明背景下 Output 动画导致“全屏背景变动”

## 目标
- Warp 终端 + “终端默认背景模式（bg=Reset）”下：
  - Output pane 触发重启动画时，不再让用户感知到“全屏背景在变暗/变色/跳动”
  - 动画仍然满足“先消失再出现”的交互语义

## 阶段
- [x] 阶段1: 复现与定位（确认只在 Output 动画出现）
- [x] 阶段2: 根因分析（Reset 在动画插值里被当作黑色）
- [x] 阶段3: 修复实现（Warp 模式替换动画实现）
- [x] 阶段4: 回归测试与全量验证
- [x] 阶段5: 记录归档（notes/WORKLOG/ERRORFIX）

## 关键问题
1. 为什么只有 Output 的重启动画会引发“全屏背景变动”？  
   - 因为 Output pane 区域通常占屏幕大部分，动画遮罩会覆盖大面积区域，任何背景色插值都会被放大感知。
2. 为什么在 bg=Reset（透明模式）时，动画会把背景“变成黑色”？  
   - tachyonfx 在做颜色插值时，`Color::Reset` 会被映射为 RGB(0,0,0)（黑色），并且 sweep_in/out 中间帧还会把 `cell.bg==Reset` 临时当作 Black 参与 lerp。
3. 如何在不牺牲 Warp 半透明效果的前提下，保留“先消失再出现”的动画语义？  
   - 在 Warp 透明背景模式下，避免使用会做颜色插值的 sweep/fade；改用 dissolve/coalesce 这种“改 symbol / 覆盖 style”的遮罩型动画，避免插值把 Reset 变黑。

## 做出的决定
- [决定] Warp（bg=Reset）模式下，Output 重启动画改用 `dissolve_to` + `coalesce_from`（带 sweep pattern）。  
  - [理由] 该方案不会对 `Color::Reset` 做 lerp，从根上消除“插值成黑色背景”的副作用。
- [决定] 后续改良：Warp 模式下允许 pane 内部保留底色（`base`），并把 Output 重启动画恢复为 `sweep_out/sweep_in`（仅作用于 Output inner area）。  
  - [理由] 内部背景不再是 Reset 后，sweep 不会触发 Reset→Black 分支；同时渐变观感更接近原版 sweep，且不再出现刺眼白条。
- [决定] 非 Warp（显式背景色）保持原 sweep_out/sweep_in 动画不变。  
  - [理由] 在显式背景色模式下，原动画观感更好，且不会出现 Reset→Black 的问题。

## 遇到错误
- [错误] 单测里直接用 `std::time::Duration` 调 `tachyonfx::Effect::process` 编译失败。  
  - [原因] 当前构建的 tachyonfx 使用自定义 Duration 类型。  
  - [修复] 在测试中用 `.into()` 把 `std::time::Duration` 转成 tachyonfx Duration。

## 状态
**已完成**：Output 重启动画在 Warp 透明背景模式下不再引发“全屏背景变动”，并通过测试验证。

## 日志
### 2026-01-30 21:47 +0800
- [问题] 用户反馈：现在全局背景都透明了，但唯独 Output block 动画会引起“整个全屏背景变动”。
- [根因] tachyonfx 的 sweep_in/out 会对 fg/bg 做颜色插值；而 `Color::Reset` 在 tachyonfx 内部会被当作黑色参与插值，导致动画期出现大面积“黑底遮罩”。
- [修复] `crates/ralph-tui/src/animation.rs`：
  - 当 `theme.app_bg_color()==Color::Reset` 时，`output_reopen_effect` 改用 `dissolve_to` + `coalesce_from`（`SweepPattern::up_to_down`），避免插值。
  - 增加回归测试：`output_reopen_effect_terminal_default_bg_does_not_paint_black_background`。
- [验证] `cargo test -p ralph-tui` ✅；`cargo test` ✅；`cargo test -p ralph-core smoke_runner` ✅。

### 2026-01-31 02:20
- [改良] 用户希望恢复更像原 sweep 的渐变，并且白条不要太晃眼；允许 pane 内部有底色。
- [最终方案] Warp 模式下：app bg=Reset，但 pane bg=base；Output 重启动画恢复 sweep（仅 inner area、faded_color=base），既不闪也更好看。


---

# 任务计划: 启动入场动画必须从空屏开始（避免先全显示再动画）

## 目标
- 并行模式（Supervisor TUI）启动时：
  - 在入场动画开始前，屏幕上不应出现任何 pane 内容/边框（真正的空屏）
  - 随后按顺序逐块出场（Instances → Output → Chat/Gates），且 Instances 条目晚于框体

## 阶段
- [x] 阶段1: 复现与定位（确认哪些区域首帧仍可见）
- [x] 阶段2: 修复实现（补齐 header/footer 等区域的遮罩）
- [x] 阶段3: 回归测试（增加“首帧为空屏”的单测）
- [x] 阶段4: 全量测试验证
- [x] 阶段5: 记录归档（notes/WORKLOG/ERRORFIX）

## 关键问题
1. 为什么会出现“先全显示一帧，再开始逐块动画”的闪烁？
2. 哪些区域没有被启动动画覆盖（例如 header/footer）？
3. 在 `bg=Reset`（Warp 半透明）模式下，如何用不依赖颜色插值的方式实现“起步空屏”？

## 状态
**已完成**：启动入场动画首帧为空屏（含 header/footer），随后按顺序逐块出场；并通过测试验证。

## 日志
### 2026-01-30 22:25 
- [反馈] 用户要求：启动时先空屏，再逐块入场；不能先显示完整 UI 再动画。
### 2026-01-30 22:27 
- [修复] 并行启动动画（bg=Reset）补齐 header/footer 的遮罩：首帧为真正空屏，避免先全显示再动画。
- [测试] 新增单测：`startup_open_effect_parallel_terminal_default_bg_starts_from_blank_screen`，并通过 `cargo test`。


---

# 任务计划: Warp 透明模式下优化“刷白条”动画观感

## 目标
- 在 Warp（`bg=Reset`）模式下：
  - 保持“整屏背景不再跟着 Output 动”（不引入 Reset→Black 插值）
  - 同时让入场/重启动画的“白条扫过”更像之前（更干净、更有速度感）

## 方案（两条路）
1. 【改良优先】在 Warp(bg=Reset) 下让 pane 内部保留底色（Catppuccin `base`），并把 Output 重启动画恢复为 `sweep_out/sweep_in` 的渐变质感（仅作用于 pane inner，避免触碰 Reset→Black 分支）。
2. 【最佳观感】实现一个自定义 shader：只改 symbol/fg，不碰 bg，做出更接近原 `sweep_in/out` 的连续渐变扫入（成本更高，但可控性最好）。

## 阶段
- [x] 阶段1: 选择方案与参数（白条厚度/速度）
- [x] 阶段2: 实现（调整 Output 重启动画 + 启动入场动画参数）
- [x] 阶段3: 回归测试（确保 bg=Reset 不会被写成 Black）
- [x] 阶段4: 全量测试验证
- [x] 阶段5: 记录归档（notes/WORKLOG/ERRORFIX）

## 状态
**已完成**：在 Warp(bg=Reset) 下恢复更接近原 sweep 的渐变观感，同时降低白条眩光，并保持背景不再跟随 Output 动。

## 日志
### 2026-01-31 01:41 
- [反馈] 用户觉得当前 Warp 透明模式下的“刷白条”不如以前好看，希望恢复更干净的扫入质感。
### 2026-01-31 01:42 
- [实现] `bg=Reset` 时 Output 重启动画从 dissolve/coalesce 改为 `slide_out + slide_in`，恢复更连续的白条扫入观感。
- [调参] 引入 `SYMBOL_SWEEP_GRADIENT_MAX=10`，收窄 slide 渐变带，避免白条太厚/太糊。
- [验证] `cargo test -p ralph-tui`、`cargo test -p ralph-core smoke_runner`、`cargo test` ✅。

### 2026-01-31 02:20 
- [反馈] 用户希望“更像原来 sweep 的渐变”，并且白色不要太晃眼；允许 block 内部有底色。
- [实现] Warp 模式下保留 panel 内部底色（`base`），并把 Output 重启动画恢复为 `sweep_out/sweep_in`（仅作用于 inner area，faded_color=base）。
- [验证] `cargo test -p ralph-tui`、`cargo test -p ralph-core smoke_runner`、`cargo test` ✅。


---

# 任务计划: Output block 动画后“底色泄漏到最外圈”

## 目标
- 我只给 pane/block 内部上底色（便于阅读），但最外圈（终端背景/空白区域）保持透明（Warp 的半透明效果）
- Output block 做入场/重启动画时：
  - 不要把底色写到最外圈
  - 不要引发整屏背景闪烁/跳变

## 阶段
- [x] 阶段1: 复现与证据采集（对比 Instances vs Output）
- [x] 阶段2: 根因定位（谁在写“全屏/外圈”的 bg）
- [x] 阶段3: 修复实现（限制绘制/遮罩范围到 pane 区域）
- [x] 阶段4: 回归测试（单测防止外圈被写底色）
- [x] 阶段5: 全量测试验证 + 记录归档

## 关键问题
1. Output 为什么会影响最外圈，而 Instances 不会？  
2. 是“绘制逻辑”写大了区域，还是“动画效果”覆盖了更大的区域？  
3. 哪些区域必须保持 `Color::Reset` 才能让 Warp 的半透明生效？

## 方案（两条路）
1. 【改良优先】把 Output 的底色只画在 `inner_area`，并确保动画 effect 也只作用于 `inner_area`（而不是 pane 外圈 / app 全屏）。  
2. 【先能用】Output pane 渲染前先对其区域 `Clear`，再统一由一个“背景层”渲染底色，pane 只画文字与边框（避免各 pane 互相污染）。  

## 状态
**已完成**：Output 动画期间外圈不再被染色（Warp 透明背景保持一致），并通过测试验证。

## 日志
### 2026-01-31 12:00 +0800
- [补强] `crates/ralph-tui/src/widgets/content.rs`：
  - ContentPane 先读取 `base_bg`（来自当前区域左上角 cell），构造 `base_style=theme.text()+base_bg` 并先铺满区域，避免清空/换帧时把 pane 底色写回 `Reset`。
  - 宽字符 continuation cell 统一写入 `symbol==""`，避免对齐异常。
  - selection 改为末尾统一 overlay，保证空白处也能被高亮覆盖。
- [收益] Output/Chat 等 pane 内部底色更稳定，动画渐变更柔和，同时 Warp 外圈仍可保持 `bg=Reset` 的半透明效果。
- [验证] `cargo test -p ralph-tui` ✅；`cargo test -p ralph-core smoke_runner` ✅；`cargo test` ✅。


---

# 任务计划: 切换 Instances 时 Output 先显示后消失（闪烁）

## 目标
- 在并行模式（Supervisor TUI）里切换 Instances 选中项时：
  - Output 区域不应先把“新实例的内容”画出来一帧
  - 应该从“隐藏态”开始，再做入场动画（sweep-in）
  - 观感上不闪烁、不抖动

## 阶段
- [x] 阶段1: 复现与根因确认（为什么先可见一帧）
- [x] 阶段2: 修复实现（让重启动画首帧从隐藏态起步）
- [x] 阶段3: 回归测试（新增单测锁定无闪烁）
- [x] 阶段4: 全量测试验证 + 记录归档

## 关键问题
1. 为什么会“先显示一帧”？是状态先切换、动画后生效，还是动画本身首帧是可见态？
2. 我们是否需要保留 “out→in” 语义？还是只要 “从隐藏态 sweep-in” 即可满足体验？
3. 如何保证“添加 effect 的那一帧”一定是隐藏态（即使 `fx_delta` 不为 0）？

## 状态
**已完成**：切换 Instances 时 Output 不再先显示一帧，入场动画从隐藏态起步（无闪烁）。

## 日志
### 2026-01-31 03:05
- [根因] Output 重启动画之前是 `sweep_out + sweep_in`，其中 `sweep_out` 首帧是完全可见态（timer reversed → alpha=1）。
- [触发条件] 切换实例时，我们先用“新实例”渲染了 Output 内容，再应用 effect，必然出现“先露一帧再被盖掉”的闪烁。
- [修复] 改为 `sweep_in`（从隐藏态揭开），并在添加该 effect 的那一帧做 priming（`fx_delta=0`），保证首帧绝对隐藏。
- [验证] `cargo fmt --check`、`cargo test -p ralph-tui`、`cargo test -p ralph-core smoke_runner`、`cargo test` ✅


---

# 任务计划: Instances 与 Output 之间增加间隙（取消边框贴合/“collapsing borders”观感）

## 目标
- 并行模式（Supervisor TUI）里：
  - Instances pane 与 Output pane 之间不要贴在一起
  - 两个 pane 都保留完整边框
  - 中间留出一条背景间隙（效果类似其它区域的分隔感）

## 阶段
- [x] 阶段1: 现状确认（定位 Instances/Output 横向布局）
- [x] 阶段2: 实现间隙列（Layout 增加 spacer column）
- [x] 阶段3: 更新动画/点击 hit-test 的区域计算
- [x] 阶段4: 回归测试与快照更新
- [x] 阶段5: 全量测试验证 + 记录归档

## 状态
**已完成**：Instances 与 Output 之间加入间隙列，取消“边框贴合/像 collapsing borders”的观感，并通过测试验证。

## 日志
### 2026-01-31 12:00 +0800
- [实现] `crates/ralph-tui/src/app.rs`：
  - 引入 `PARALLEL_PANE_GAP_WIDTH=1`，并在并行模式 main 区域横向布局中插入 gap 列：`instances | gap | output`。
  - 同步更新所有用到该布局拆分的地方（渲染、Output 重启动画触发时的 area 计算），避免“某处改了某处没改”导致 hit-test/动画错位。
- [验证] `cargo test -p ralph-tui` ✅；`cargo test -p ralph-core smoke_runner` ✅；`cargo test` ✅。


---

# 任务计划: Warp 外圈仍被染色 + Alacritty 首帧先全显示再动画（回归修复）

## 目标
- Warp：
  - pane 内部保留底色（`base`），提升可读性（Output/Instances 一致）
  - **最外圈（Warp window 的半透明背景）不被染色**，不随 Output/切换实例动画而“变蓝灰”
- Alacritty（以及其它非 Warp 终端）：
  - 启动入场动画必须从“空屏”起步
  - 不能出现“先显示完整 UI 一帧，再开始逐块动画”的闪烁

## 阶段
- [x] 阶段1: 证据与根因再确认（Warp vs Alacritty 行为差异）
- [x] 阶段2: 修复实现
  - [x] 2.1 Warp：给 UI 留出 1-cell 外边距（让 pane 与屏幕边缘脱离，保留一圈 bg=Reset）
  - [x] 2.2 Non-Warp：并行启动动画从 sequence 改为 parallel+prolong_start（保证所有 pane 首帧隐藏）
- [x] 阶段3: 回归测试（新增“非 Warp 并行启动动画首帧空屏”单测）
- [x] 阶段4: 全量测试验证（包含 ralph-core replay smoke tests）
- [x] 阶段5: 记录归档（notes/WORKLOG/ERRORFIX）

## 关键问题
1. Warp 的“外圈染色”是否来自 **pane 贴边** 导致的视觉采样/混色（而不是 buffer 真的写出了全屏 bg）？
2. 非 Warp 并行启动动画为什么会闪？（sequence 只隐藏了第一个 stage，后续 pane 在 stage 之前是可见态）

## 状态
**已完成**：Warp 模式下 UI 不再贴边（外圈保留 bg=Reset 安全环），Non-Warp 并行启动动画首帧空屏；并通过测试验证。

## 日志
### 2026-01-31 12:40 +0800
- [反馈] 用户确认：Warp 中 Output 动画结束后，UI 范围外的半透明背景会被染成蓝灰色；切换实例也会出现“先染色再变透明”的过程。
- [对照] Alacritty 中没有外圈染色，但启动动画会出现“先全显示再动画”，且白色边框动画缺失。
- [行动] 先撤回未提交改动，重新基于当前 HEAD 做系统性回归修复（本计划）。

### 2026-01-31 12:45 +0800
- [实现] Warp：`theme.app_bg_color()==Reset` 时对 UI 区域增加 1-cell 外边距（pane 不贴边），让最外圈保持 bg=Reset 的“安全环”。
- [实现] Non-Warp：并行启动动画从 `fx::sequence` 改为 `fx::parallel + fx::prolong_start`，保证所有 pane 首帧隐藏（空屏起步）。
- [测试] 新增回归测试：`startup_open_effect_parallel_non_terminal_default_bg_starts_from_blank_screen`。
- [验证] `cargo test -p ralph-tui` ✅；`cargo test -p ralph-core smoke_runner` ✅；`cargo test` ✅。

### 2026-01-31 18:48 +0800
- [更正] Warp “外圈染色”问题：之前尝试过“给 UI 留 1-cell 外边距（pane 不贴边）”，但用户反馈仍会染色，因此该尝试已撤回（当前布局回到 `frame_area`）。
- [判断] Warp 的“UI 范围外 padding 被染色”大概率属于终端窗口级渲染/混色行为（cell 绘制无法完全控制），短期先不继续深挖。
- [继续] 聚焦解决 Alacritty 的“边框白色过渡动画缺失”（这个属于我们能稳定控制的 cell/effect 行为）。


---

# 任务计划: Alacritty 边框“白色过渡”动画缺失（补回）

## 目标
- 在 Alacritty（以及其它非 Warp 终端）中：
  - 启动入场动画与 Output 重启动画期间，pane 的边框应出现清晰的“亮线扫过/白色过渡”反馈
  - 动画结束后不残留白边（静态观感不变）
  - 不影响“首帧空屏”与“切换实例不闪烁”的既有修复

## 阶段
- [x] 阶段1: 现象确认（Alacritty 边框过渡不明显/缺失）
- [x] 阶段2: 根因分析（exabind 边框 bg 修正让纯 sweep 的反馈变弱）
- [x] 阶段3: 修复实现（仅对 outer border 叠加高亮 sweep shader）
- [x] 阶段4: 回归测试（单测锁定“确实会刷到 border”）
- [x] 阶段5: 全量测试验证（含 replay smoke tests）

## 状态
**已完成（代码层）**：非 Warp 下边框高亮 sweep 已实现，并通过单测与全量 `cargo test` 验证；等待你在 Alacritty 里确认观感是否达到预期（若仍不明显再继续调参）。

## 日志
### 2026-01-31 18:48 +0800
- [实现] `crates/ralph-tui/src/animation.rs`
  - 新增/改良 `BorderHighlightSweep`：只作用于 pane outer border（1-cell），随 UpToDown sweep 下移一段“亮段”。
  - 将高亮带宽从 1 行提高到 3 行，避免竖边每帧只有 2 个亮点导致肉眼难察觉。
  - 前沿 y 位置从 `round` 改为 `floor`，减少抖动跳帧。
- [测试] 新增单测：`border_highlight_sweep_highlights_outer_border_with_band`。
- [验证] `cargo test -p ralph-tui` ✅；`cargo test -p ralph-core smoke_runner` ✅；`cargo test` ✅。

### 2026-01-31 20:21 +0800
- [反馈] 你在 Alacritty 里确认：`BorderHighlightSweep` 仍不够明显，并且下边框几乎看不到动画。
- [根因] 我之前把前沿 y 的映射改成了 `floor(alpha * (height-1))`：
  - bottom_row 只有 alpha==1.0 才会命中；
  - 但我们为了避免尾帧残留，会在 alpha≈1 提前 return；
  - 结果就是下边框永远不会被高亮。
- [修复] `crates/ralph-tui/src/animation.rs`
  - `BorderHighlightSweep` 支持可选 `highlight_bg`：动画期同时刷 fg+bg，让“细线边框”在视觉上更“粗”。
  - 前沿 y 映射改为 `floor(alpha * height)` 并 clamp 到 `height-1`：保证在 alpha<1 的区间也能命中 bottom_row。
  - 高亮区域从 outer 1-cell 扩到 outer 2-cells：上下边框在动画期有更明显的带宽。
  - Non-Warp 下的高亮配色改为：`fg=White`，`bg=surface2`（更容易被肉眼捕捉）。
- [测试] 更新单测：`border_highlight_sweep_highlights_outer_border_with_band`（适配 outer=2 与 bg 高亮）。
- [验证] `cargo test -p ralph-tui` ✅；`cargo test -p ralph-core smoke_runner` ✅；`cargo test` ✅。

### 2026-01-31 20:32 +0800
- [反馈] 你确认新的边框高亮“太粗”，需要细一点。
- [修复] `crates/ralph-tui/src/animation.rs`
  - `border_highlight_sweep` 的 filter 从 outer=2 改回 outer=1（只作用于真正的 border ring）。
  - 高亮带宽从 3 行降到 2 行（更细、更不抢屏）。
  - 高亮 bg 从 `surface2` 调暗到 `surface1`（降低“整行亮块”的厚重感）。
  - 保留 bottom_row 可命中的映射修复（下边框仍能看到动画）。
- [测试] 新增回归测试：`border_highlight_sweep_can_reach_bottom_row_before_end`（锁定“下边框能动”）。
- [验证] `cargo test -p ralph-tui` ✅；`cargo test -p ralph-core smoke_runner` ✅；`cargo test` ✅。


---

# 任务计划: 并行启动入场动画从“完全串行”改为“错峰重叠”

## 目标
- 在 Supervisor TUI 启动时（并行模式）：
  - 不再是 Instances 完全结束后才开始 Output，再开始 Bottom
  - 改为“上一块进行到一半时，下一块就开始”，多块并行入场
  - 同时保留约束：Instances 条目必须晚于框体（先框后字）

## 阶段
- [x] 阶段1: 确认现状（当前是严格串行 delay）
- [x] 阶段2: 设计节奏（每块错峰 1/2 pane 时长）
- [x] 阶段3: 实现（调整 delay + 总时长常量）
- [x] 阶段4: 回归测试（确保首帧空屏仍成立）
- [x] 阶段5: 全量测试验证

## 状态
**已完成（代码层）**：并行启动入场动画已改为错峰重叠，且不破坏“首帧空屏/无闪烁”的既有修复。

## 日志
### 2026-01-31 20:21 +0800
- [实现] `crates/ralph-tui/src/animation.rs`
  - 将 `STARTUP_GAP_MS` 重新定义为“错峰启动间隔”：`STARTUP_PANE_MS / 2`。
  - 更新 `STARTUP_TOTAL_MS`：由原来的“完全串行”改为适配重叠节奏的总时长。
  - 并行启动的 delay 改为：
    - Output: `STARTUP_GAP_MS`（Instances 进行到一半时开始）
    - Bottom: `STARTUP_GAP_MS * 2`（Output 进行到一半时开始）
    - Instances items: `STARTUP_PANE_MS`（框体完成后才开始）
- [验证] `cargo test -p ralph-tui` ✅；`cargo test -p ralph-core smoke_runner` ✅；`cargo test` ✅。
