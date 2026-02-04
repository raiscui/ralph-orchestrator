# 笔记：Ralph Orchestrator
#
# 说明：
# - 本文件用于记录“本次改动过程中的发现与决策”。
# - 当文件超过 1000 行会自动轮换为 `notes_YYYY-MM-DD_HHMM.md`，避免变成巨无霸难以检索。
#
# 上一个轮换文件：
# - `notes_2026-02-03_1344.md`

---

## 2026-02-03 13:44 +0800｜Radar 再向下偏移：避免遮挡 Output 面板边线

### 结论
- 将 Radar 的纵向 inset 从 `2` 调整为 `3`。
- 目的：让 Radar 的 top border 不再落在并行模式 Output 面板的 top border 同一行，避免覆盖边线。

### 改动点
- `crates/ralph-tui/src/app.rs`
  - `HAT_GRAPH_RADAR_INSET_Y: 2 → 3`
  - 同步更新回归测试的 clamp 预期（可用高度随 inset 变化）
- `specs/terminal-ui.spec.md`
  - 补充说明：inset 的目的也包括避免覆盖重要 pane 边框（例如 Output top border）

### 验证
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅

## 2026-02-03 23:41 +0800｜调整：取消 Radar 扫描头“高对比模式”（c）

### 你提出的新口径（我为什么要改）

- 你希望取消高对比模式：不再提供 `c` 切换。
- 目标是减少 UI 分支与心智负担，让 Radar 动效更确定。

### 实现要点

- 移除高对比开关与键位：
  - `crates/ralph-tui/src/state.rs`：删除 `hat_graph_high_contrast`
  - `crates/ralph-tui/src/input.rs`：删除 `ToggleHatGraphHighContrast` 与 `c` 键映射
  - `crates/ralph-tui/src/app.rs`：
    - 删除 reducer 分支与并行模式全局 `c` 快捷键处理
- 简化扫描头渲染：
  - `crates/ralph-tui/src/app.rs`：`apply_hat_graph_radar_scan_head` 去掉 `high_contrast` 参数与配色分支，只保留默认 stop
- 测试与文档同步：
  - `crates/ralph-tui/src/app.rs`：删除高对比分支测试，保留默认渐变测试
  - `crates/ralph-tui/src/widgets/help.rs`：移除 `c` 的 help 提示
  - `specs/terminal-ui.spec.md`：移除 `c` 对比模式说明（避免“幽灵功能”）

### 验证

- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅

---

## 2026-02-03 23:40 +0800｜调优：拖尾加长 + 扫描头提亮 + base 高亮边变暗（不压暗拖尾）

### 你最新反馈（我为什么要继续调）

- 拖尾太短。
- 需要整体提亮。
- “线段原色”要变暗，但不是让拖尾变暗（拖尾需要保持亮度层次）。

### 改动点

- 拖尾加长：
  - `crates/ralph-tui/src/state.rs`：`HAT_GRAPH_EDGE_HEAD_LEN: 16`
- 扫描头提亮（去掉 tail 的 DIM，改为更亮的渐变 stop + 更宽的 BOLD 区间）：
  - `crates/ralph-tui/src/app.rs`：`apply_hat_graph_radar_scan_head`
  - normal：`blue -> lavender -> text`
  - high-contrast：`maroon -> peach -> yellow`
  - `BOLD` 区间：从 `t >= 0.80` 调到 `t >= 0.70`
- base 高亮边变暗（降低抢眼程度，把注意力交给扫描头）：
  - `crates/ralph-tui/src/app.rs`：`edge_base_fg: sapphire -> overlay1`

### 回归测试

- `crates/ralph-tui/src/app.rs`
  - `hat_graph_radar_scan_head_uses_truecolor_gradient_without_bg_normal`
  - `hat_graph_radar_scan_head_high_contrast_uses_warm_palette_without_bg`
  - 锁死点：bg 不应被改写；拖尾应发生渐变；tip 必须 BOLD 且更亮。

### 验证

- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅

---

## 2026-02-03 23:10 +0800｜调整：去掉扫描头 bg；truecolor 渐变拖尾；高对比模式（c 切换）

### 你提出的新口径（我为什么要继续改）

- 轻微发光底色（bg）要去掉。
- 扫描头要更“高级”：tip 更亮、tail 更长更柔和。
- 需要一档“高对比模式”，在不同终端观感下能快速切换更醒目的扫描头配色。

### 实现要点

- 去掉 bg：扫描头不再设置 `Style::bg(...)`，只改前景色与 `DIM/BOLD` 修饰（保持底色由面板/终端决定）。
- 更长拖尾：
  - `crates/ralph-tui/src/state.rs`：`HAT_GRAPH_EDGE_HEAD_LEN: 6 → 10`
  - tail 段用 `DIM` 拉长“柔和拖尾”的观感
- truecolor 渐变（两段插值）：
  - `crates/ralph-tui/src/app.rs`：`apply_hat_graph_radar_scan_head`
  - normal：冷色系渐变（更贴近 base 线路）
  - high-contrast：暖色系渐变（与 base 差异更大，更醒目）
- 高对比模式开关（纯 UI 偏好）：
  - `crates/ralph-tui/src/state.rs`：新增 `hat_graph_high_contrast: bool`
  - `crates/ralph-tui/src/input.rs`：新增 Action 并绑定按键 `c`
  - `crates/ralph-tui/src/app.rs`：
    - 串行：走 `map_key → dispatch_action`
    - 并行：非 Chat 输入场景下 `c` 直接切换（Chat 里 `c` 仍然当作输入字符）
  - Radar 标题增加显示：`c: std/HC`，便于一眼确认当前模式

### 回归测试

- `crates/ralph-tui/src/input.rs`：`c_returns_toggle_hat_graph_high_contrast`
- `crates/ralph-tui/src/app.rs`：
  - `hat_graph_radar_scan_head_uses_truecolor_gradient_without_bg_normal`
  - `hat_graph_radar_scan_head_high_contrast_uses_warm_palette_without_bg`
  - 关键点：断言“bg 不变”（避免回归把 bg 又染上去）

### 文档同步

- `specs/terminal-ui.spec.md`：补充 `c` 的对比模式切换语义，以及在输入上下文中不触发 toggle 的约束。

### 验证

- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅

## 2026-02-03 21:25 +0800｜Radar：扫描头动效（常亮边 + 跑动高亮头）

### 你确认的需求（我为什么要做这个）
- 你确认需要“更像动画”的反馈：线路不只是 reveal 一次。
- 你希望：reveal 完成后线路保持全亮，并且有一个“高亮头”沿线路循环跑动。
  这样用户一眼能看出“这条边还在跑 / 目标 hat 仍在 Running”。

### 实现要点（我做了什么）
- 我把“每一帧该怎么画”的逻辑抽成 **纯函数渲染计划**，先保证可测试、可回归：
  - `crates/ralph-tui/src/state.rs`：`plan_hat_graph_radar_edge_animation`
  - 这样渲染层只负责按 plan 上色，不再把时间/阶段/边界条件散落在 UI 代码里。
- 渲染层（TUI）按两层上色：
  1) base：`sapphire`（全亮底色/路径）
  2) head：`sky + BOLD`（短段扫描头）
  - reveal 阶段：head 贴着 reveal 前沿
  - reveal 完成后：head 以固定速度循环跑动（直到目标退出 Running 才会被上层清理）

### 对应代码位置
- 常量与渲染计划：
  - `crates/ralph-tui/src/state.rs`：`HAT_GRAPH_EDGE_HEAD_STEP_MS` / `HAT_GRAPH_EDGE_HEAD_LEN` / `plan_hat_graph_radar_edge_animation`
- 渲染 overlay（cell-level 上色，不塞 ANSI）：
  - `crates/ralph-tui/src/app.rs`：`apply_fg_to_hat_graph_radar_path_segment` + Radar edge 渲染循环
- 文档同步：
  - `specs/terminal-ui.spec.md`：补充 “highlight head looping” 的行为描述

### task_plan 轮换（避免超过 1000 行）
- `task_plan.md` 超过 1000 行后已轮换：
  - 旧文件归档：`task_plan_2026-02-03_2105.md`
  - 新任务计划：`task_plan.md`

### 验证
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅

---

## 2026-02-03 20:45 +0800｜Radar：edge.path 补点（关键点 -> 连续线段）以修复“连线动画半截消失”

### 你反馈的现象
- 你看到 event 线路动画“显示一半就不显示了”。

### 关键发现（为什么会这样）
- `beautiful-mermaid-rs` 的 `AsciiRenderMetaEdge.path` 语义是“关键格子”（拐点/箭头等），并不保证包含线段上的每一个 cell。
- 如果 TUI 直接按这些“关键点”逐段上色，肉眼会觉得“线段断了/只亮到一半”。

### 修复策略（低侵入改良）
- 在 **CLI 注入 radar meta** 时把 `edge.path` 做“补点”：
  - 对相邻关键点之间的水平/垂直段补齐为逐 cell 的连续序列
  - 非正交段保守回退为“只连接关键点”

### 对应代码位置
- `crates/ralph-cli/src/hats.rs`：`densify_hat_graph_radar_path`（补点逻辑）

### 验证
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅

## 2026-02-03 18:15 +0800｜Radar：事件线改为“按 Running 目标驱动”的短动画（取消 60s/循环逻辑）

### 你重新澄清的口径（关键）
- event 线路绘制不需要持续很久。
- 如果线路指向的目标 node box 已经不在 Running，则取消该线路动画。
- 如果有新的 box 进入 Running，则：
  - 新 box 染色（Running 高亮）
  - 并且展示“导致它 Running 的 event”的线路动画（同时出现）
- 你看到的现象：
  - 线路显示持续很久；
  - 即使新 event 出现，仍然在显示旧线路（被旧逻辑卡住）。

### 根因（为什么旧线路会“拖很久”）
- 我们此前实现的是“全局最新 event”的动画状态：
  - 并且加入了“循环播放 + 60 秒驻留”的规则（避免闪烁）。
  - 这会导致：新 event 到来时不一定能立刻替换旧动画，从而出现“旧线路一直在播”的观感。

### 新实现思路（按 Running 目标驱动）
- 改为两层状态：
  1) `hat_graph_recent_events`：记录最近业务事件（source_hat + topic），用于推断 cause event。
  2) `hat_graph_edge_animations`：以 target_hat 为 key 的“短动画”，只在 hat 进入 Running 时启动。
- 触发点从“收到 event”改为“实例状态跃迁到 Running”：
  - 这样能保证“新 box 进入 Running”时，动画与 box 高亮同步出现。
- 取消条件：
  - target_hat 没有任何 Running 实例时，立刻移除该 hat 的边动画。
- 动画时长：
  - 一次 progressive reveal + 短 hold；
  - 同时设置上限，避免路径很长时拖太久（符合你“不是必须持续很久”的要求）。

### 相关代码位置（便于你 review）
- spec 更新（删除 60s/循环，改为 Running 驱动的短动画）：`specs/terminal-ui.spec.md`
- 状态机与 cause 推断：`crates/ralph-tui/src/state.rs`
- 渲染（按 target_hat 画边、目标不 Running 直接隐藏）：`crates/ralph-tui/src/app.rs`

---

## 2026-02-03 17:49 +0800｜Radar：向内偏移（避免遮挡 Output 边线）+ 事件线动画“循环播放”+ 60s 驻留

### 你最新补充的需求

- Radar 覆盖层从右上角“向内偏移”（朝左下角），并且**还要再向下偏移一点**，避免遮挡 Output 的边线。
- 线动画：
  - 没有新 event 时也要一直循环播放；
  - event 来得太快也不能闪烁：每条动画至少 60 秒驻留。
- 事件归因：
  - 你指出 `.ralph/events.jsonl` 里是有 `source_instance` / `hat` 的；
  - 因此不应该“自动填当前实例 hat_id”，而应使用真实字段来做匹配。

### 关键信息：`.ralph/events.jsonl` 的字段含义（引用示例）

来自 `examples/parallel-trigger-routing/.ralph/events.jsonl` 的一行（节选）：

```json
{"iteration":0,"hat":"spec_writer","source_instance":"spec_writer#1","topic":"spec.ready","triggered":"spec_reviewer", ...}
```

- `hat`：Ralph 记录的“发布该事件时的活跃 hat”（用于人类可读日志/回放观察）。
- `source_instance`：并行模式下的实例归因（`hat_id#n`），用于精确回放/路由/可视化。
- `triggered`：这条事件最终触发的 hat（event loop 在路由时填入，agent 自己不写）。

### 现状定位（为什么你会感觉“看不到动画/闪一下就没了”）

- `crates/ralph-tui/src/app.rs` 里，边动画当前是“播完就停”的：
  - 只在 `elapsed <= total_ms` 时才刷高亮；
  - `total_ms` 通常只有几百毫秒到几秒，视觉上容易被认为“没有/闪一下”。
- `crates/ralph-tui/src/state.rs` 里虽然有 `tick_hat_graph_radar_animation(...)`（用于 60s 后切换 pending），
  但如果渲染循环没有调用它，pending 永远不会生效。

### 预期修复方向（方案 A：只改 ralph-tui）

- 渲染侧：把“播完就停”改成“按步进取模循环播放”。
- 状态侧：在每帧 render tick 调用 `tick_hat_graph_radar_animation(now)`，让 pending 能在 60s 后切换。
- 布局侧：把 Radar 的 `inset_y` 再加 1 行（你说的“再向下偏移一点”）。

---

## 2026-02-03 16:55 +0800｜更正：并行模式不填 event.source；Radar 动画用 source_instance 推导发布者

### 为什么要更正
- 你指出：`examples/parallel-trigger-routing/.ralph/events.jsonl` 的记录里没有 `source` 字段，
  但有 `hat`（发布者）与 `source_instance`，因此“自动填充 event.source”为当前实例 hat_id 不符合你的预期。

### 关键澄清
- `events.jsonl` 的每行是 `EventRecord`，其发布者字段叫 `hat`，来源是 Supervisor 调用
  `EventLogger::log_event(iteration, hat_id, event, triggered)` 时传入的 `hat_id`，
  **不是**从 `Event.source` 推导出来的。
- `ralph_proto::Event.source` 在并行模式下可能为空，这是因为 `<event ...>` 文本协议本身不携带 source，
  而我们选择让协议更“原样”，不在 HatInstance 内自动补齐该字段。

### 当前实现（与预期对齐的方案）
- `crates/ralph-core/src/parallel/instance.rs`
  - 仍会补齐 `event.source_instance` 与 `event.id`（用于归因与回放），但不再自动补齐 `event.source`
- `crates/ralph-cli/src/parallel_runner.rs`
  - 并行模式事件转发：`gate.*` / `human.message` / `source_instance` / `source` 事件都会进入 TUI
- `crates/ralph-tui/src/state.rs`
  - Radar 动画触发：优先用 `event.source`；否则用 `event.source_instance.split_hat_id()` 推导发布者 hat

### 验证
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅

---

## 2026-02-03 19:47 +0800｜并行实验开发永动机：配置范式关键点（给未来复用）

### 1) 为什么必须把“派发”与“执行”拆成两个 hats

- 派发器（dispatcher）只做结构化转发：
  - 输入：`experiment.start`（payload=EXPERIMENT_PLAN）
  - 输出：N 条 `experiment.task`
  - 不跑工具、不改文件，避免引入不确定性。
- 执行器（runner）才承担真实改动与验证：
  - 把“实现 + 验证”放在同一个 job/worktree 里，避免跨 hat 共享 workspace 造成污染。

### 2) 为什么 runner 结果必须带 patch/commit（否则会“丢改动”）

- 并行 worktree 的生命周期通常是“job 级别”：
  - job 结束后 worktree 可能被回收。
  - 如果 runner 不导出产物（`patch` 或 `commit`），主工作区拿不到任何可审阅/可落盘的改动。
- 因此把“产物导出”上升为协议要求：
  - `experiment.result` 必须包含 `verification_evidence` + (`patch` 或 `commit`)。

### 3) 如何让探索型工作流“可收敛”，而不是无限循环

- 通过配置层强制声明入口/完成/结束条件：
  - `starting_event`：明确 workflow 入口（例：`experiment.start`）
  - `complete_publishes`：明确“收敛完成事件”（例：`experiment.complete`）
  - `completion_promise`：明确 CLI 结束信号（例：`LOOP_COMPLETE`）
- 同时加硬护栏：
  - `max_iterations` / `max_runtime_seconds`
  - 并行并发上限与 idle TTL
  - 单任务 `job_timeout_secs`

### 4) 生产建议（与示例默认不同）

- 为了“一条命令跑通”，example 默认 `parallel.permissions.*=allow`。
- 真正在团队里跑时建议改为 `ask`：
  - 通过 gate 人工审批，把“高风险操作”显式暴露出来。

---

## 2026-02-03 19:53 +0800｜纠错：需求仅补充到 OpenSpec change（不落盘实现）

- 本次需求只需要把配置方案补充到 change：`openspec/changes/parallel-hat-solution-eval-example/`。
- 因此我已撤回主仓库实现层的 example/fixture/smoke test 落盘内容。
- 方案草案目前放在：
  - `openspec/changes/parallel-hat-solution-eval-example/design.md`（Appendix：`ralph.yml` + `README.md` 草案）

---

## 2026-02-03 15:05 +0800｜Hat Graph Radar：Running hats 蓝色高亮 + 最新 event 边动画（逐段点亮）

### 关键澄清：Radar 画图不是 exec CLI，而是直接调用 crate

- Ralph 侧生成 Radar 字符图时，直接调用 `beautiful-mermaid-rs` crate 的函数：
  - 例如：`use beautiful_mermaid_rs::{AsciiRenderOptions, render_mermaid_ascii_with_meta};`
  - 以及：`render_mermaid_ascii_with_meta(&diagram, &AsciiRenderOptions { ... })`
- 这意味着要做“node box 蓝色高亮 / edge 动画”，最稳的路径不是在终端里改字符串，
  而是让渲染器输出坐标 meta（nodes/edges/path），TUI 用 buffer cell-level 叠加上色。

### 方案落地（最终采用）

- TS 核心（`/Users/cuiluming/local_doc/l_dev/ref/typescript/beautiful-mermaid`）
  - 新增 API：`renderMermaidAsciiWithMeta(text, options) -> { text, meta }`
  - meta 输出包含：
    - node box bounds（用于高亮 box）
    - edge stroke path（有序坐标序列，用于逐段点亮动画）
- Rust 绑定层（`/Users/cuiluming/local_doc/l_dev/my/rust/beautiful-mermaid-rs`）
  - 同步 vendor bundle（让 `beautifulMermaid.renderMermaidAsciiWithMeta` 在 QuickJS 可用）
  - 新增 Rust API：`render_mermaid_ascii_with_meta(...) -> AsciiRenderWithMeta`
- Ralph TUI（`ralph-orchestrator`）
  - CLI 启动时注入 Radar：`ascii_compact/ascii_full + meta_compact/meta_full`
  - UI 渲染时：
    - Running hats：box 前景蓝色（并行模式高亮所有 Running）
    - 最新 event：从发布者 hat 到订阅者 hats 的边做“逐段点亮”动画

### 验证
- TS：`bun test src/__tests__/` ✅
- beautiful-mermaid-rs：`cargo test` ✅
- ralph-orchestrator：
  - `cargo fmt --check` ✅
  - `cargo clippy --all-targets --all-features -- -D warnings` ✅
  - `cargo test` ✅
  - `cargo test -p ralph-core smoke_runner` ✅

---

## 2026-02-03 16:21 +0800｜Bugfix：Radar 高亮闪烁 + event 边动画不触发

### 现象
- 运行中 hat 的 box 蓝色高亮只闪一下就没了（看起来像被回退成 created）。
- Radar 看不到任何 event 相关的边动画效果。

### 根因
- 根因 A：`ParallelTuiState::append_output()` 每次收到 output chunk 都会调用
  `register_instance(..., Created)`，把已经是 Running/Idle 的实例状态覆盖回 Created。
- 根因 B：并行模式 `parallel_runner` 的 event_observer 只把 `gate.*` / `human.message` 转发进 TUI，
  带 source 的业务事件（用于从发布者 hat 启动边动画）根本到不了 UI reducer。

### 修复
- `crates/ralph-tui/src/state/parallel.rs`
  - `append_output` 仅在实例不存在时才注册 Created，实例存在时绝不覆盖 state。
  - 新增单测：`parallel_append_output_does_not_override_instance_state`
- `crates/ralph-cli/src/parallel_runner.rs`
  - 放宽转发条件：`gate.*` / `human.message` / `event.source.is_some()` 都转发到 TUI。
  - 抽出 `should_forward_event_to_tui` 并新增单测锁死策略。

### 验证
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅

---

## 2026-02-03 16:45 +0800｜Radar：Running box 颜色改为 #a9dc76；并行事件补齐 source 以触发边动画

### 你反馈的问题（为什么我要继续改）
- 你希望 Running hat 的 box 高亮色改为 `#a9dc76`（而不是蓝色）。
- 你仍然没看到 event 的线段动画。

### 根因定位（为什么动画仍不可见）
- 虽然我们在 `parallel_runner` 放宽了“转发到 TUI 的事件过滤条件”，
  但并行模式下 hat 输出的 `<event ...>` 在解析后 **没有注入 `event.source`**：
  - `crates/ralph-core/src/parallel/instance.rs`：`EventParser::new().parse(&result.output)`
  - 随后仅补齐 `source_instance` 与 `id`，导致 `source` 仍为 None
- 结果：
  - TUI 的 `maybe_start_hat_graph_animation` 因缺少 source 无法启动动画；
  - 你自然看不到“逐段点亮”的边动画。

### 修复点
- 颜色：
  - `crates/ralph-tui/src/theme.rs`：新增语义化方法 `hat_graph_running_hat_fg() -> #a9dc76`
  - `crates/ralph-tui/src/app.rs`：Running hats 的 box 高亮使用该语义色
- 并行事件归因：
  - `crates/ralph-core/src/parallel/instance.rs`：在 `decorate_outgoing_event` 中补齐 `event.source=hat_id`
  - 语义与串行模式对齐：所有由 hat instance 发出的事件都应携带发布者 hat_id

### 验证
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅

---

## 2026-02-03 21:40 +0800｜增强：Radar 扫描头渐变 + 轻微发光底色（更强对比度）

### 目标

- 扫描头从“单色 + BOLD”升级为“渐变 + 轻微发光底色（bg）”。
- 与 base 线路（常亮边）拉开对比度。
- 让“流动方向 / 仍在运行”的视觉提示更明显。

### 实现要点

- 只改渲染层，不改状态机与事件语义：
  - 代码入口：`crates/ralph-tui/src/app.rs`：`apply_hat_graph_radar_scan_head`
- 渐变策略（tail -> tip）：
  - tail：`sapphire`（贴近 base）
  - mid：`sky`
  - near-tip：`lavender`（加粗）
  - tip：`text`（最亮 + 加粗）
- 轻微“发光底色”：
  - tail/mid：`bg=surface0`
  - tip：`bg=surface1`

### 回归测试

- `crates/ralph-tui/src/app.rs`：`hat_graph_radar_scan_head_uses_gradient_and_glow_bg`
  - 锁死规则：tip 必须“最亮 + BOLD + 更亮 bg”，tail 必须“更暗 + 轻微 bg”。

### 验证

- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅

---

## 2026-02-04 00:27 +0800｜并行实验开发永动机：`parallel-hat-solution-eval-example` 配置范式要点

### 你要解决的问题（现象）

- 真实项目里经常出现：一个目标可以走多条实现路径。
- 多条路径都“看起来可行”，但性能/体验/副作用不确定。
- 必须把多条路径都跑出来，并且对每条路径做同强度验证。
- 如果串行探索，会非常慢，而且很容易“忘了验证”导致越改越乱。

### 结论（本质）

这类任务不是“拆分目标成分支演示”。
而是要一套可复用的 `ralph.yml` 范式：
把“并行实现 + 批量验证 + 多轮实验探索”变成可持续推进、可回放、可收敛的工作流。

### 配置范式（核心机制）

1) **窗口化派发（in-flight window）**
- 不能一次性把所有实验都发出去（洪水式派发会导致队列膨胀/吞吐下降）。
- 只允许同时在途 `P` 个实验任务。
- 一个实验“完成”的定义以审计为准：
  - 收到 `experiment.reviewed` 且 `evidence_ok=true` 才释放一个 slot。

2) **自适应并行度（激进 + AIMD）**
- `P_max` 由 `ralph#1` 根据用户给的 plan/prompt 推断（你选择了让 Ralph 自己判断）。
- 运行中动态调参（AIMD）：
  - 顺利：`P += 1`（上限 `P_max`）
  - 拥塞/失败信号：`P = floor(P/2)`（快速刹车）
- 强护栏：必须给控制面留 slot：
  - `P <= parallel.autoscale.max_running_jobs - 2`（为 `ralph#1` + `auditor` 预留）

3) **独立审计（auditor）把 backpressure 变成硬门禁**
- runner 产出 `experiment.result`（必须带验证证据 + **patch**；`commit` 仅可选补充信息）。
- auditor 独立消费 result，输出 `experiment.reviewed`：
  - 证据不足 => `needs_more_evidence`，并阻断收敛
  - 证据充分 => `evidence_ok=true`
- 这样可以抵抗长跑中的“模型漂移”（口头说通过，但证据不充分）。

4) **独立集成验收（integrator）：采纳/合并不由 runner 做**
- `experiment_runner` 不负责“是否采纳/合并/最终验收”。
- `ralph#1` 在所有实验都通过审计后，发布 `integration.task`：
  - 选择一个候选实验结果（通常基于 selection_criteria）
  - 携带 patch + `final_verification`（主工作区最终验收命令）
- `experiment_integrator` 是主工作区单写者：
  - apply patch（必要时解决冲突）
  - 跑最终验收命令
  - 输出 `integration.applied` / `integration.rejected` / `integration.blocked`
- `ralph#1` 只有在收到 `integration.applied` 后才允许收敛（发布 `experiment.complete` + 输出 `LOOP_COMPLETE`）。

5) **权限与 gate（你问的 ask + timeout）**
- `parallel.permissions.*=ask` 会触发 `gate.request`。
- `parallel.gate.default_timeout_secs` 控制默认超时（0 表示不超时）。
- 超时后会产生 `gate.timeout`（并投递给 `ralph#1`），后续应由决策型 job 产出 `gate.resolve` 再继续推进。
- 你额外确认的口径：hooks 默认不需要批准（避免频繁打断），worktree 可切到 ask。

### 相关参考（在哪里对齐语义）

- 并行 runtime / gate timeout / 事件协议的既有定义：
  - `specs/parallel-hat-instances.spec.md`

### 文档坑（已修复）

- Mermaid flowchart 的 edge label 包含括号时可能触发 parse error。
- 规避方式：给 label 加引号，例如 `-->|\"experiment.task (windowed)\"|`。
