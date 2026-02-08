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

---

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

---

## 2026-02-04 16:24 +0800｜apply 落盘：parallel-hat-solution-eval-example（example / fixture / smoke）

### 这次落盘了什么

- 新增并行“实验开发永动机”示例目录：
  - `examples/parallel-experimental-dev-engine/ralph.yml`
  - `examples/parallel-experimental-dev-engine/README.md`
- 补充仓库文档入口：
  - `README.md` 的 parallel 章节新增 runnable examples 链接
- 新增 replay fixture + smoke gates：
  - `crates/ralph-core/tests/fixtures/parallel_experimental_dev_engine.jsonl`
  - `crates/ralph-core/tests/smoke_runner.rs` 增加 fixture 存在性检查 + 关键 topic/归因前缀/patch/LOOP_COMPLETE 断言

### 细节与踩坑点

- README 里的 mermaid 图表已用 `mermaid-validator` 校验通过（避免括号 label parse error）。
- `cargo test -p ralph-core smoke_runner` 这个命令会按“名称过滤”测试，可能不会跑到 `tests/smoke_runner.rs` 的集成测试本体。
  - 如果你要“只跑 smoke_runner 这个集成测试文件”，更稳的方式是：
    - `cargo test -p ralph-core --test smoke_runner`

---

## 2026-02-04 20:13 +0800｜补充：为 parallel-experimental-dev-engine example 增加专用 ralph-e2e（Codex）场景

### 为什么要加（动机）

- smoke/replay 能验证“语义与回放确定性”，但它不覆盖“真后端（Codex）端到端跑一遍”的现实风险。
- 你要的是“比较硬”的 E2E，所以我选了：
  - **直跑 example**
  - **用 events.jsonl 做断言主依据**
  - **强制闭环：patch + integration.applied + LOOP_COMPLETE**

### 怎么做（关键实现点）

- 新增一个 E2E scenario：`ParallelExperimentalDevEngineExampleScenario`
  - 只支持 `Backend::Codex`（避免其它 backend 工具能力差异导致假失败）
- 在 E2E workspace 中“预填 EXPERIMENT_PLAN”
  - 因为这个 example 的本意就是“用户先填 plan 再运行”
  - 预填的 plan 选择“轻量、确定成功”的实验：只创建小文件 + `rg` 验证
  - 这样能最大化降低 flakey 的来源（不引入编译/网络/长跑命令）

### 断言强度（当前是偏硬的版本）

- 必须出现关键 topic 链路（并且数量至少等于预填实验数）：
  - `experiment.start` / `experiment.task` / `experiment.result` / `experiment.reviewed` / `integration.task` / `integration.applied` / `experiment.complete`
- `experiment.reviewed` 必须明确 `evidence_ok=true`（避免“证据不足也收敛”的回归）
- 必须看到 `patch`，且至少包含一次 `diff --git`（unified diff 形态）
- 禁止出现异常信号：
  - `gate.*`（example 默认 allow，不应触发审批）
  - `routing.escalate`（路由/target 失败应被捕获）

### 如果后续发现不适合（你说的“再改”方向）

- 如果 Codex 偶发不写 `diff --git` 或不稳定输出 `evidence_ok`：
  - 先把断言从“强内容匹配”降级到“topic 出现 + exit_code=0 + 不 timeout”
  - 或把 `EXPECTED_EXPERIMENTS` 从 2 调到 1（更宽松）
- 如果仍然 flakey：
  - 把“硬约束”迁移到 replay fixtures（增加 `needs_more_evidence` / `integration.rejected` 分支 fixture）

---

## 2026-02-04 21:05 +0800｜调整：parallel-experimental-dev-engine example 改为 PROMPT.md 驱动 + auditor 可放弃不理想实验

### 背景（你提出的约束）

- 你希望开发者日常使用时：
  - **实验计划/实验内容写在 PROMPT.md**，而不是写进 `ralph.yml`
  - 理论上不需要改 `ralph.yml`
- 你强调“实验就是实验”：
  - runner 跑出来可能不理想，这是并行探索的常态
  - 允许 `experiment_auditor` 对不理想结果明确放弃（reject/abandon）
  - 不要把流程写成“必须等所有实验都 OK 才能继续”
- 你指出 `starting_event: "experiment.start"` 也不应写死在配置里：
  - 应该不写，由 ralph 决定（结合 prompt 与 hats 拓扑）

### 我做了什么（落盘变更点）

- 把原先内联在 `event_loop.prompt` 的整段 prompt 迁移到独立文件：
  - 新增 `examples/parallel-experimental-dev-engine/PROMPT.md`
  - 增加 `EXPERIMENT_PLAN_START/END` 标记，方便人类编辑与 E2E 预填
- 更新示例配置：
  - `examples/parallel-experimental-dev-engine/ralph.yml` 改为 `event_loop.prompt_file: examples/parallel-experimental-dev-engine/PROMPT.md`
  - 移除 `event_loop.starting_event`，让 ralph 在 `task.start` 后自行选择入口事件
- 更新审计语义（使“放弃不理想实验”成为一等能力）：
  - `experiment_auditor` 的输出 `verdict` 扩展为：
    - `approved | rejected | needs_more_evidence`
  - 并明确：
    - 证据齐全但 `failed/blocked` => `rejected`
    - 证据不足 => `needs_more_evidence`
- 同步 example 文档叙事：
  - README 改为“编辑 PROMPT.md，而不是改 ralph.yml”
  - 最小成功标准改为：有 `approved` 候选即可进入 integration，不要求所有实验都 OK
- 同步 E2E 场景（避免示例结构变化导致 E2E 失真）：
  - 预填逻辑从“改 ralph.yml 里的 block scalar”改为“改 PROMPT.md 里 markers”
  - 在 E2E workspace 中复制 `examples/parallel-experimental-dev-engine/` 目录结构再运行

### 验证

- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅

---

## 2026-02-05 00:18 +0800｜补充：complete_publishes 的“配置自洽”规则与落盘位置

### 规则（你提醒我必须记住的点）

- 如果配置里定义了 `event_loop.complete_publishes = C`：
  - **最佳实践**是至少有一个 Hat 的 `publishes` 显式包含 `C`。
  - 否则：
    - `ralph hats graph --view logical` 会出现 `Complete[complete]` 但没有任何入边；
    - completion candidate 没有明确“生产者”，配置很容易写成“永远等不到的收敛事件”。
  - 只有当你明确约定由协调者（`ralph#1`）在 prompt 中自行发布 `C` 时，才可以接受“没有 Hat 声明发布”的情况。

### 我把它记录在哪里

- 记录在 spec：`specs/hats-graph-logical-view.spec.md`
  - 位置：G5（complete_publishes）备注区，紧挨着 `Complete[complete]` 与入边规则说明

### 同步修正（让 example 自身也符合这条规则）

- `examples/parallel-experimental-dev-engine/ralph.yml`：
  - `experiment_integrator.publishes` 增加 `experiment.complete`
  - integrator 成功时要求额外发布 `experiment.complete`（作为 `complete_publishes` 的候选事件）
- `examples/parallel-experimental-dev-engine/PROMPT.md`：
  - 收敛条件改为“观察到 experiment.complete -> 输出 LOOP_COMPLETE”
  - 并保留兜底：integration.applied 但缺失 experiment.complete 时允许补发

### 验证

- Mermaid：已用 `mermaid-validator` 校验 example README 的 flowchart 语法 ✅

---

## 2026-02-05 00:40 +0800｜hats graph：logical view 在 coordinator-driven workflow 下“看起来断开”的根因与修复方向

### 复现证据

- 在 `examples/parallel-experimental-dev-engine/` 下运行：
  - `ralph hats graph`（默认 logical view）
  - 现象：只剩 `experiment_runner -> experiment_auditor`，`experiment_integrator` 与 `complete` 变成孤岛
- `ralph hats graph --format mermaid` 可以更直观看到：图里没有 `ralph#1`（coordinator）节点，因此许多边天生画不出来。

### 根因（不是渲染器坏了，是“视图语义”差异）

- `ralph hats graph` 默认输出的是 **logical view**（见 `specs/hats-graph-logical-view.spec.md`）：
  - **隐藏**调度员 `ralph#1`
  - 只画 Hat→Hat（A publishes topic，B subscribes topic）
  - `complete_publishes` 会画成 `Complete[complete]` 终点锚点
- 但 `parallel-experimental-dev-engine` 这个 example 的工作流是典型的 **coordinator-driven**：
  - `experiment.task` / `integration.task` / `experiment.complete` 是由 `ralph#1` 发布（不属于任何 hat publishes）
  - `experiment.reviewed` / `integration.applied|rejected|blocked` 的消费方主要也是 `ralph#1`
- 所以在 logical view 里：
  - 只有 `experiment.result` 这种“hat↔hat 内部 topic”会显示成连线
  - 其余边都被“隐藏调度员”的规则裁掉，视觉上就像“断开”

### 解决方案（不改默认语义）

- 增加 `--view physical`（物理视图）：
  - 显式展示 `ralph#1 (coordinator)` 节点
  - 只在“边界 topic”（无内部发布者/无内部订阅者）上画 Ralph↔Hat 边
  - 让 coordinator-driven workflow 能在拓扑图里恢复“全貌视图”
- 默认仍是 `--view logical`，保持干净、确定性输出，不破坏既有 spec/回归测试。

### 落地细节（渲染稳定性）

- 在 physical view 初版里，我遇到过 `beautiful-mermaid-rs --ascii` 对“Ralph 同一对节点多条边”的不稳定（QuickJS exception）。
- 因此实现里对 **涉及 Ralph 的多条边** 做了折叠（label 用 `" / "` 拼接），让 unicode/ascii/compact 渲染稳定可用。
- `cargo test -p ralph-core smoke_runner` ✅

---

## 2026-02-05 09:12 +0800｜hats graph：让 Unicode/ASCII 图里 ralph#1 更靠左/靠上

### 现象

- `ralph hats graph --format unicode --view physical` 的 Unicode 图里：
  - `ralph#1 (coordinator)` 经常被布局到图的右侧或中下方；
  - 直觉上更希望它在图的左侧/上方，作为“调度员/起点”。

### 关键发现（最重要）

- `beautiful-mermaid-rs` 的 flowchart 布局对“Mermaid 节点声明顺序”非常敏感：
  - 同一张拓扑图，仅仅把 `Hat_ralph[...]` 放到 Mermaid 文本更前面，
    就能显著改变渲染布局；
  - 把 `Hat_ralph` 优先声明后，Unicode/ASCII 图里 `ralph#1` 更稳定地靠左/靠上（best-effort）。

### 落地做法

- 只改 physical view 的 Mermaid 生成：
  - `generate_mermaid_string_physical` 里先输出 `Hat_ralph[ralph#1 (coordinator)]`，
    再输出其它 hats 节点声明与边；
  - 并补回归测试，锁死“physical view 必须优先声明 Hat_ralph”这一约束。

### 验证

- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test -p ralph-core kiro` ✅

---

## 2026-02-05 15:55 +0800｜PROMPT 注入优先级 & `ralph_prompt` 语义澄清

### Prompt 内容来源优先级（`resolve_prompt_content`）

- 优先级（高 → 低）：
  1. CLI `-p "text"`（inline prompt）
  2. CLI `-P path`（prompt file）
  3. config `event_loop.prompt`（inline prompt）
  4. config `event_loop.prompt_file`（prompt file）
  5. 默认 `PROMPT.md`
- 结论：
  - 只要 `event_loop.prompt` 存在，就不会再读取/注入 `PROMPT.md`（或其它 prompt_file）。
  - 只要走 prompt_file，就不会再注入 `event_loop.prompt`（二选一）。

### `event_loop.ralph_prompt` 的定位

- `event_loop.ralph_prompt`：始终追加注入给 Ralph（协调者）。
- 并行模式：只注入给 `ralph#1`，不注入其它 hats（避免 prompt pollution）。

### example 约定

- `examples/parallel-experimental-dev-engine/PROMPT.md`：应是 Markdown 的实验计划 prompt（模板），不是 YAML 配置文件。

---

## 2026-02-05 15:10 +0800｜example：`parallel-experimental-dev-engine` 的 `PROMPT.md` 改为纯 YAML（无说明/无 marker）

### 变更点

- `examples/parallel-experimental-dev-engine/PROMPT.md` 现在是纯 YAML 模板：
  - 不包含任何 Markdown 说明段落
  - 不包含 `<!-- ... -->` marker
- `examples/parallel-experimental-dev-engine/ralph.yml` 的 `event_loop.ralph_prompt`：
  - 去掉了“不要拷贝 marker 行”的旧描述（因为已不存在 marker）
- `examples/parallel-experimental-dev-engine/README.md`：
  - 不再要求“编辑 marker 区间”，而是直接编辑 PROMPT.md 的 YAML 字段
- `crates/ralph-e2e/src/scenarios/parallel_experimental_dev_engine_example.rs`：
  - E2E 预填不再依赖 marker 截取模板，而是直接覆写 workspace 里的 PROMPT.md 为确定性 YAML

### 验证

- `cargo test` ✅

---

## 2026-02-05 11:15 +0800｜Example 结构调整：`parallel-experimental-dev-engine`（固定协议 -> `event_loop.ralph_prompt`）

### 目标（用户需求）

- 把 `examples/parallel-experimental-dev-engine/PROMPT.md` 中“开发者不需要改”的固定协议迁移到 `examples/parallel-experimental-dev-engine/ralph.yml` 的 `event_loop.ralph_prompt`。
- 让 `PROMPT.md` 只保留“演示型范例/模板”，告诉开发者应填写什么（`EXPERIMENT_PLAN` YAML）。

### 关键点（为什么这样做）

- `event_loop.ralph_prompt` 是 **Ralph-only 的追加注入**：
  - 并行模式下只进入 ralph#1 的 coordinator instructions；
  - 不会污染其它 hats 的 prompt（避免 prompt pollution）。
- `PROMPT.md` 只承载“可变的实验计划”，能把日常改动面压到最小：
  - 不易误改协议；
  - E2E 可以继续通过 marker 预填计划（`EXPERIMENT_PLAN_START/END`）。

### 实施摘要（做了什么）

- `examples/parallel-experimental-dev-engine/ralph.yml`：
  - 新增 `event_loop.ralph_prompt`，承载原 PROMPT.md 中的固定协议：
    - 强 backpressure 规则
    - task.start -> experiment.start 的入口约定
    - in-flight window / AIMD / P_max 推断
    - abandon/reject、integration、completion 的收敛语义
    - 最小 payload 字段与事件输出格式
- `examples/parallel-experimental-dev-engine/PROMPT.md`：
  - 精简为“计划模板文件”：
    - 只保留短说明 + `EXPERIMENT_PLAN` YAML（保留 marker 行，兼容 E2E 预填）
- `examples/parallel-experimental-dev-engine/README.md`：
  - 明确固定 vs 可变分工：
    - PROMPT.md 只改计划
    - ralph.yml 的 ralph_prompt 固定协议通常不改

### 验证

- `cargo test` ✅（确保不影响编译与现有测试）

---

## 2026-02-05 11:40 +0800｜parallel-experimental-dev-engine：用 `commit` 取代 `patch` 作为实验产物

### 背景 / 动机

- 旧约定要求 runner 在 `experiment.result` 里嵌入 `git diff` 的 unified diff patch 文本。
- 在真实改动里，patch 很容易变成几千行：
  - event payload 膨胀；
  - 模型输出易截断；
  - 审计/集成阶段反而难以“可搬运、可回放”。

### 新约定（commit-only）

- `experiment.result` **必须**包含 `commit`（git hash）。
- `integration.task` 通过 `commit` 传递候选产物，integrator 在主工作区执行 `git cherry-pick <hash>` 做集成与最终验收。
- 约定不再要求（也不建议）在 payload 里粘贴 patch 文本。

### Trade-offs

- 优点：
  - payload 很小；
  - 更贴近真实开发工作流（review / cherry-pick / 回滚）。
- 代价：
  - runner/integrator 需要能成功执行 `git commit`（存在 git 身份依赖）。
  - 推荐用“命令级 git 身份”避免依赖全局配置：
    - `git -c user.name="ralph" -c user.email="ralph@local" commit -m "..."`
- worktree 回收影响：
  - worktree 的改动若不提交，会随 worktree 回收而丢失；
  - 一旦提交，commit 对象落在共享 `.git` 的 object DB 中，worktree 回收不影响短期可用性。

### 受影响位置（同步点）

- example：`examples/parallel-experimental-dev-engine/ralph.yml` / `PROMPT.md` / `README.md`
- replay + smoke：`crates/ralph-core/tests/fixtures/parallel_experimental_dev_engine.jsonl`、`crates/ralph-core/tests/smoke_runner.rs`
- E2E（真后端场景）：`crates/ralph-e2e/src/scenarios/parallel_experimental_dev_engine_example.rs`
- OpenSpec change：`openspec/changes/parallel-hat-solution-eval-example/`（spec/design/proposal/tasks）

---

## 2026-02-05 10:44 +0800｜调研：prompt precedence 与 “只注入 Ralph” 的现有语义锚点

### 现状（代码事实）

- Prompt 来源优先级在 `crates/ralph-cli/src/loop_runner.rs` 的 `resolve_prompt_content()`：
  - 只要 `event_loop.prompt`（inline）有值，就会 **直接返回**，不会再读 `PROMPT.md`。
  - 只有当 inline 为空时，才会去读 `event_loop.prompt_file`（默认值就是 `PROMPT.md`）。
- 非并行（EventLoop）里，Ralph 的 prompt 是通过 `HatlessRalph::build_prompt()` 生成：
  - 入口：`crates/ralph-core/src/event_loop/mod.rs` 的 `EventLoop::build_prompt()`（hat_id == "ralph" 分支）
  - 组装：`crates/ralph-core/src/hatless_ralph.rs` 的 `HatlessRalph::core_prompt()` + `build_prompt()`
- 并行（ParallelSupervisor）里，已经有明确的“防 prompt pollution”规则：
  - `crates/ralph-core/src/parallel/instance.rs` 的 `build_prompt()`：**只给 ralph#1 注入 `prompt_prelude`**，其他 hat 强制为空字符串。
  - `crates/ralph-core/src/parallel/supervisor.rs` 会为 ralph#1 生成一份“强约束协调语义”的 instructions（`build_ralph_coordinator_instructions()`）。

### 推论（对本次需求的直接影响）

- 如果我们要新增 `event_loop.ralph_prompt` 并要求“始终注入给 Ralph”，最佳注入点是：
  - 非并行：HatlessRalph 组装 prompt 时插入（不会污染事件 payload）
  - 并行：把它拼进 ralph#1 的 coordinator instructions（或仅 ralph#1 prompt 组装路径），保持“只注入 Ralph”的污染防线不被破坏

---

## 2026-02-05 10:45 +0800｜`complete_publishes` 的“明确发布者”硬门禁（validate）+ example/E2E 对齐

### 背景

- 用户提出一条“配置自洽”规则：
  - 如果 `examples/.../ralph.yml` 里定义了 `event_loop.complete_publishes = C`，
    那么必须有一个 hat 的 `publishes` 声明包含同一个 `C`。
- 这条规则的动机很明确：
  - completion candidate 如果没有明确生产者，会把收敛信号变成隐式约定；
  - 最终表现为：workflow 卡死、拓扑图出现悬空终点、排查成本变高。

### 结论（硬门禁规则）

- 当且仅当存在自定义 hats（`hats` 非空）时：
  - 如果配置了 `event_loop.complete_publishes = C`，
    那么 **MUST** 至少有一个 hat 的 `publishes` 包含 `C`；
  - 否则 `RalphConfig::validate()` 直接报错拒绝配置。

### 并行模式下的 prompt 注入语义（确认“PROMPT.md 会注入给谁”）

- 关键代码在 `crates/ralph-core/src/parallel/instance.rs` 的 `build_prompt()` 注释里已经写死：
  - `"只有 ralph#1 注入 prompt_prelude"`
  - `"其他 hat 只看自己的 instructions + incoming events"`
- 所以：
  - `event_loop.prompt` / `event_loop.prompt_file` / 仓库根目录 `PROMPT.md`（默认 prompt_file）
    只会影响 `ralph#1`（协调者），不会污染其它 worker hats。

### 对齐动作（E2E + example）

- E2E：`parallel/hat_instances` 场景原先用 `complete_publishes: routing.escalate`，
  但 `routing.escalate` 本质是 supervisor 直投给 `ralph#1` 的升级事件。
  - 为了满足 hard gate，不改 completion candidate topic，
    让 `collector` 同时“声明并发出”一个 `routing.escalate`（使其具备明确 hat publisher）。
- Example：`parallel-experimental-dev-engine`
  - 实验计划迁移到 `examples/parallel-experimental-dev-engine/PROMPT.md`（用 marker 包裹，便于 E2E 预填）
  - 移除 `starting_event`（由 `ralph#1` 自行决定入口事件）
  - auditor verdict 增加 `rejected`（允许放弃不理想实验）
  - integrator 明确 `publishes` + 发布 `experiment.complete`（作为 `complete_publishes` 的 completion candidate）

---

## 2026-02-05 09:30 +0800｜调整：默认 physical view（取消必须写 `--view physical`），Radar 也默认 physical

### 结论

- `ralph hats graph` 默认 view 切到 physical：
  - 用户不需要再手写 `--view physical`。
  - `--view logical` 保留为“更干净的 Hat→Hat 逻辑边”视图。
- TUI 右上角 Hats Graph Radar 与 CLI 默认对齐：
  - 默认 physical view；
  - 同时支持单 topic 匹配 `"a / b / c"` 这类折叠 label（避免事件线动画匹配失败）。

### 复现/对照（命令行证据）

- physical（默认）：
  - `cargo run --bin ralph -- -c examples/parallel-experimental-dev-engine/ralph.yml hats graph --format mermaid`
- logical（更干净，但 coordinator-driven 会天然“断开”）：
  - `cargo run --bin ralph -- -c examples/parallel-experimental-dev-engine/ralph.yml hats graph --view logical`

### 验证

- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test -p ralph-core kiro` ✅

---

## 2026-02-05 16:35 +0800｜parallel-experimental-dev-engine：Experiments 为空时的 Auto-Plan 语义

### 需求摘要

- 当用户在 `PROMPT.md` 里不写（或只保留 TODO 占位）“实验列表（Experiments）”时：
  - 由 `ralph#1` 先做只读扫描（根据 objective/约束分析项目）。
  - 自动生成 2~5 个实验方案（多条路径）。
  - 再按窗口（AIMD）派发 `experiment.task` 给 runner 去跑。

### 落盘位置

- `examples/parallel-experimental-dev-engine/ralph.yml`：在 `event_loop.ralph_prompt` 里定义 Auto-Plan 触发条件与生成规则。
- `examples/parallel-experimental-dev-engine/PROMPT.md`：把 Experiments 标为可选，并补充可选约束（Constraints）字段。
- `examples/parallel-experimental-dev-engine/README.md`：同步使用说明（Experiments 可留空）。

---

## 2026-02-05 16:58 +0800｜PROMPT 实验任务：从“硬列表”改为“可选条目 + 默认 Auto-Plan”

- PROMPT.md 默认不再包含任何 `exp-001/exp-002` 这类实验条目。
- `ralph_prompt` 规则改为：
  - 用户写了可执行实验条目 → 优先按条目派发 `experiment.task`。
  - 用户没写（或全是 TODO 占位）→ Auto-Plan：先分析项目再生成 2~5 个实验。

## 2026-02-06 12:00 +0800｜hats graph Mermaid：`Node[label (x)]` 在 mermaid-cli 下会 Parse error

- `Hat_ralph[ralph#1 (coordinator)]` ❌；`Hat_ralph["ralph#1 (coordinator)"]` ✅
- 实现：`MermaidLabelMode::Strict` 下 `format_mermaid_node_label` 遇到 `(` / `)` 自动加引号（`crates/ralph-cli/src/hats.rs`）。
2026-02-07 12:37 +0800 | 修复笔记: AsciiRenderOptions 新增 max_width 后,显式字面量初始化必须补 ..Default::default() 或显式给 max_width,本次选择 ..Default::default() 以同时规避未来字段演进造成的 E0063。
