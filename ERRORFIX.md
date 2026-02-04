# ERRORFIX

> 说明：历史 ERRORFIX 记录已归档到 `ERRORFIX_2026-02-01_1538.md`（文件超过 1000 行自动轮转）。

## 2026-02-01 15:28 +0800｜hats graph：中文/emoji hat 名称导致 unicode/ascii 只剩 task.start→Ralph

### 现象
- 在 `examples/parallel-trigger-routing/ralph.yml` 这类配置下：
  - `ralph hats graph --format mermaid` 输出完整 hats 拓扑
  - 但 `ralph hats graph --format unicode/ascii/compact` 只剩 task.start→Ralph

### 根因
- Mermaid 图生成时把节点 ID 直接用 `hat.name`（中文/emoji）拼出来。
- `beautiful-mermaid-rs` 对 Unicode 节点 ID 兼容性不足，会吞边/吞节点但不报错。

### 修复
- `crates/ralph-cli/src/hats.rs`：
  - Mermaid 输出改为“节点 ID / label 分离”：
    - ID：`Hat_{sanitize(hat.id)}`（ASCII `[A-Za-z0-9_]` + 前缀避免冲突）
    - label：继续用 `hat.name`（保留中文/emoji）
  - hats 按 `hat.id` 排序，降低 HashMap 迭代顺序导致的布局波动。
  - 新增回归测试：unicode 渲染结果必须包含中文 hat 名称，避免再次回退为“只剩 Start→Ralph”。

### 验证
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅

---

## 2026-02-03 18:15 +0800｜tui hat graph radar：旧事件线拖太久且新事件不替换（需求口径改为 Running 驱动）

### 现象
- event 线路动画会持续很久（甚至新 event 出现仍在播旧线路）。
- 你明确说：event 线路不需要持续很久；并且应以 Running 状态为准：
  - 目标 box 不再 Running 就取消线路动画；
  - 新 box 进入 Running，则显示该 box 高亮 + “导致它 Running 的 event”线路动画。

### 根因
- 之前实现的是“全局最新 event”的动画状态 + 额外的“循环播放/驻留”策略：
  - 这会让旧线路在视觉上持续存在；
  - 同时新事件可能被节流/延后，从而出现“新 event 来了仍在播旧线”的观感。

### 修复
- 规范同步：
  - `specs/terminal-ui.spec.md`：删除“循环 + 60s 驻留”要求，改为 Running 目标驱动的短动画规则。
- TUI 状态机重构（按目标 hat）：
  - `crates/ralph-tui/src/state.rs`
    - 记录最近业务事件：`hat_graph_recent_events`（用于推断 cause event）。
    - 保存目标 hat 的短动画：`hat_graph_edge_animations`。
    - 在 `ParallelInstanceState` 中捕捉“进入 Running”跃迁并启动动画；
      在退出 Running 且该 hat 已无 Running 实例时立刻取消动画。
    - tick 负责清理过期事件/动画与目标不 Running 的动画。
- 渲染改造：
  - `crates/ralph-tui/src/app.rs`
    - 不再渲染“全局最新 event”；
    - 只渲染 `hat_graph_edge_animations`，并且目标不 Running 直接隐藏。

### 验证
- `cargo fmt` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅

---

## 2026-02-03 15:14 +0800｜开发过程踩坑记录：meta API 测试用例与 clippy 门禁

### 1) TS roundtrip 测试在极端紧凑参数下失败

- 现象：
  - 新增 `renderMermaidAsciiWithMeta` 后，为了模拟 Radar compact（padding=0），
    我在测试里用了 `{ paddingX: 0, paddingY: 0, boxBorderPadding: 0 }`。
  - 结果 `reverseFlowchartAsciiToMermaid(text)` 反解出来的边为空。
- 根因：
  - 反向解析并不保证在“极端压缩”参数下仍可稳定识别箭头/出边 marker（线段可能退化/覆盖）。
- 修复：
  - 将该测试改为使用默认 spacing，仅验证：
    - `text` 与旧 API 完全一致
    - `meta` 非空
    - 默认渲染仍可 roundtrip（逻辑一致）

### 2) `cargo clippy -D warnings` 阻塞：unused import

- 现象：
  - `crates/ralph-tui/src/app.rs` 引入了 `HatGraphRadarMeta`，但实现里只用到了方法调用，
    没有直接引用类型名，导致 clippy 报 unused import。
- 修复：
  - 删除未使用的 import 后重新通过 clippy。

---

## 2026-02-03 12:55 +0800｜tui hat graph radar：贴右上角（消除 2 行空隙）+ zoom 尺寸自适配字符图

### 现象
- TUI 右上角 Hat Graph Radar 面板距离终端右上角有 2 行空隙。
- `p` 放大（zoom）后，面板尺寸不适配字符图：
  - 有时字符图被裁切（面板不够大）。
  - 有时留白很多（面板过大）。

### 根因
- `crates/ralph-tui/src/app.rs`
  - Header 固定高度=2（内容 1 行 + bottom border 1 行）。
  - Radar 之前用 `content_area` 作为 bounds：
    - `content_area.y == 2`，因此 Radar 从 y=2 开始绘制，视觉上“离右上角空 2 行”。
  - Radar 之前的尺寸策略与字符图尺寸无关：
    - mini：固定上限（<=36x10）
    - zoom：按 content_area 的 2/3 + 上限（<=120x40）

### 修复
- `crates/ralph-tui/src/app.rs`
  - Radar 的 bounds 改为“整屏去掉 footer”（高度截止到 footer 起始行）：
    - 让 Radar y=0，真正贴到右上角，同时避免覆盖 footer。
  - 新增 `measure_text_diagram_size`：
    - 用 `unicode_width::UnicodeWidthStr` 测量每行 display width（避免 emoji/东亚字符宽度误判）。
  - `hat_graph_radar_area` 调整为“优先适配字符图尺寸”：
    - zoom：按 `diagram_size + border` 自适配，再按 bounds 裁剪。
    - mini：保留雷达上限（<=36x10），但图更小时会收缩，减少留白。
  - 新增回归测试，锁死：
    - Unicode display width 计算正确
    - area 锚定到 bounds 的 y=0
    - zoom 自适配与 bounds clamp
- `specs/terminal-ui.spec.md`
  - 更新 Radar 锚点语义（frame 去掉 footer）。
  - 补充 zoom 尺寸 SHOULD 自适配字符图尺寸。

### 验证
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test -p ralph-core kiro` ✅

---

## 2026-02-03 19:54 +0800｜流程纠错：误把“补充 change”当成“进入实现阶段”

### 现象
- 用户要的是：把 `ralph.yml` 配置方案补充到 OpenSpec change：`parallel-hat-solution-eval-example`。
- 我误把它当成可以直接进入 apply/实现阶段，导致把 example/fixture/tests 落盘到了主仓库实现层。

### 根因
- 没有把需求约束锁定在 OpenSpec workflow 的“artifact 产出”层。
- 对“补充到 change”语义理解偏差：把“方案内容”当成“需要马上实现的仓库文件”。

### 修复
- 回滚实现层落盘内容（不在主仓库新增 example/fixture/tests）。
- 把配置草案与使用说明补充回 change artifacts：
  - `openspec/changes/parallel-hat-solution-eval-example/design.md`（Appendix：`ralph.yml` + `README.md` 草案）

### 验证
- `cargo test -p ralph-core --test smoke_runner` ✅

---

## 2026-02-03 13:16 +0800｜tui hat graph radar：澄清“偏移”语义（向内偏移到左下）

### 现象
- 我把“与右上角间隔两行字符”误解成要“消除空隙”，导致 Radar 被顶到右上角贴边（y=0）。
- 你实际想要的是：Radar **从右上角向内偏移**（往左下移），保留留白。

### 根因
- 缺少对“偏移方向”的明确定义（是贴边对齐，还是向内 inset）。

### 修复
- `crates/ralph-tui/src/app.rs`
  - 增加 `HAT_GRAPH_RADAR_INSET_X/Y = 2`，让 Radar 从右上角向内（左下）偏移。
  - 同时用 `bounds - inset` 计算可用空间，避免越界；并保持 zoom 按字符图尺寸自适配。
- `specs/terminal-ui.spec.md`
  - 补充：Radar SHOULD inset from top-right（shift inward）。
- 回归测试
  - 从期望 `y=0` 改为期望 `y=HAT_GRAPH_RADAR_INSET_Y`，并同步 x/width/height 的 clamp 预期。

### 验证
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅

---

## 2026-02-01 23:32 +0800｜hats graph：Mermaid 输出不应出现 Ralph（调度员应隐藏），Hat→Hat 应为实线

### 现象
- `ralph hats graph --format mermaid` 输出包含 `Ralph` 节点：
  - 订阅/发布都表现为 `Ralph <-> Hat`
  - Hat→Hat 的逻辑关系用虚线 `-.->`
- 当 hats 多时，视觉上接近“全连接”，阅读体验很差。

### 根因
- Mermaid 生成逻辑把“内部调度拓扑（经 Ralph 路由）”直接暴露给用户：
  - 既画了 `Ralph -> Hat`（订阅）
  - 也画了 `Hat -> Ralph`（发布）
  - 同时又额外用虚线再画一遍 Hat→Hat
- 这导致图包含过多“实现细节”，噪声远大于信息量。

### 修复
- `crates/ralph-cli/src/hats.rs`：
  - Mermaid 输出改为“逻辑视图”：
    - 不再输出 `Ralph` 节点
    - 不再输出任何 `Ralph <-> Hat` 的边
    - Hat→Hat 传播关系统一用实线 `-->`
  - 当 `event_loop.starting_event` 显式存在时：
    - 增加 `Start[task.start] -->|starting_event| Hat` 入口边（否则不输出 Start，避免孤立节点）
  - 边集合按 `(source_id, topic, target_id)` 排序并去重，确保输出确定性。
- 新增/更新回归测试断言：
  - Mermaid 输出必须包含 `Hat_A -->|mid| Hat_B`
  - Mermaid 输出不得包含 `Ralph` 与 `-.->`

### 验证
- `cargo fmt` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test -p ralph-core kiro` ✅

---

## 2026-02-03 20:45 +0800｜TUI Radar：事件连线动画“只亮一半/像断线”

### 现象
- Hat Graph Radar 的 event 连线动画看起来“显示一半就不显示了”。

### 根因
- `beautiful-mermaid-rs` 提供的 meta 中，`AsciiRenderMetaEdge.path` 是“关键点序列”（拐点/箭头等），不是“线段上每个 cell”。
- TUI 侧如果直接按关键点逐段上色，就会造成肉眼观感上的“线段缺失/只亮半截”。

### 修复
- `crates/ralph-cli/src/hats.rs`：
  - 在 `convert_ascii_meta_to_radar_meta` 注入 meta 时，对每条 edge 做 `edge.path` 补点：
    - 水平/垂直段补齐为逐 cell 的连续路径
    - 非正交段保守回退为“只连接关键点”
  - 新增回归测试：`densify_hat_graph_radar_path_fills_horizontal_and_vertical_segments`

### 验证
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅

---

## 2026-02-03 17:55 +0800｜tui hat graph radar：边动画“播完就停/一闪而过” + pending 永不切换 + 遮挡 Output 边线

### 现象
- Radar 覆盖层会遮挡 Output 的边线（尤其是并行模式 Output panel 的 top border 附近）。
- 最新 event 的线段动画看不到，或只是一闪而过。
- 你要求：没有新 event 时也要一直循环播放；新 event 来太快也必须至少展示 60 秒再切换。

### 根因
- `crates/ralph-tui/src/app.rs`
  - 边动画之前用 `elapsed <= total_ms` 做“播完就停”，`total_ms` 通常很短，导致观感像“闪一下就没了”。
- `crates/ralph-tui/src/state.rs`
  - 虽然实现了 `tick_hat_graph_radar_animation(...)`（用于 60 秒后 pending→current 切换），
    但渲染主循环没有调用它，导致 pending 永远不会生效。
- `crates/ralph-core/src/parallel/instance.rs`
  - 并行模式下解析 `<event ...>` 后只补齐了 `source_instance` 与 `id`，没有补齐 `event.source`，
    导致 UI/诊断在“发布者 hat”维度的归因不够直接。

### 修复
- Radar 位置：
  - `crates/ralph-tui/src/app.rs`：`HAT_GRAPH_RADAR_INSET_Y` 从 `3` 调整为 `4`，把覆盖层整体再下移 1 行。
- 边动画循环播放：
  - `crates/ralph-tui/src/app.rs`：改为“按步进取模”的循环渲染逻辑（progressive reveal + hold + repeat），不再播完就停。
- 60 秒驻留切换：
  - `crates/ralph-tui/src/app.rs`：在每帧 render tick 调用 `state.tick_hat_graph_radar_animation(Instant::now())`，让 pending 能在驻留到期后切换。
- 事件发布者归因：
  - `crates/ralph-core/src/parallel/instance.rs`：`decorate_outgoing_event` 在缺失时补齐 `event.source=hat_id`，并继续补齐 `source_instance`。
- 回归测试同步：
  - `crates/ralph-tui/src/app.rs`：因为 inset_y 增大导致可用高度减少，更新了相关断言（高度从 7 → 6）。

### 验证
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅

---

## 2026-02-03 16:55 +0800｜tui hat graph radar：并行模式不应自动填充 event.source；改用 source_instance 驱动边动画

### 现象 / 反馈
- 你指出：把 `event.source` 自动填为“当前实例的 hat_id”不正确。
- 你提供的 `.ralph/events.jsonl` 记录显示：publisher 信息在 `hat` 字段里，而不是 `source` 字段里。

### 根因
- `EventRecord.hat` 与 `Event.source` 是两套概念：
  - `EventRecord.hat` 由 Supervisor 在 `EventLogger::log_event(iteration, hat_id, event, triggered)` 处写入；
  - 并行模式下 `<event ...>` 文本协议本身不携带 `source`，因此 `Event.source` 允许为空。
- 我们之前让 Radar 动画强依赖 `Event.source`，导致并行模式下容易“漏触发”。

### 修复
- `crates/ralph-core/src/parallel/instance.rs`
  - 回滚：不再自动补齐 `event.source`
  - 保留：继续补齐 `event.source_instance` 与 `event.id`
- `crates/ralph-cli/src/parallel_runner.rs`
  - 事件转发过滤：加入 `event.source_instance.is_some()`（避免因 source 为空漏事件）
- `crates/ralph-tui/src/state.rs`
  - Radar 动画触发：`source` 优先；否则用 `source_instance.split_hat_id()` 推导发布者 hat

### 验证
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅

---

## 2026-02-03 16:21 +0800｜tui hat graph radar：Running 高亮闪烁 + event 边动画不触发

### 现象
- 并行模式下，Running hat 的 box 蓝色高亮“闪一下就没了”（看起来被回退到 created）。
- Radar 看不到任何 event 边动画（线段逐段点亮没有出现）。

### 根因
- 根因 A：`crates/ralph-tui/src/state/parallel.rs` 的 `ParallelTuiState::append_output()`：
  - 每次 output chunk 都会 `register_instance(..., Created)`；
  - 导致已进入 Running/Idle 的实例状态被覆盖回 Created。
- 根因 B：`crates/ralph-cli/src/parallel_runner.rs` 的 event_observer：
  - 只转发 `gate.*` / `human.message` 到 TUI；
  - 带 `source` 的业务事件无法进入 UI reducer，因此 `hat_graph_animation` 无法启动。

### 修复
- `crates/ralph-tui/src/state/parallel.rs`
  - `append_output` 仅在实例不存在时才注册 Created；实例存在时不再覆盖 state。
  - 新增回归单测：`parallel_append_output_does_not_override_instance_state`
- `crates/ralph-cli/src/parallel_runner.rs`
  - 放宽并行模式事件转发：`gate.*` / `human.message` / `event.source.is_some()` 都转发到 TUI。
  - 抽出 `should_forward_event_to_tui` 并新增单测锁死策略。

### 验证
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅

---

## 2026-02-03 16:45 +0800｜tui hat graph radar：Running 高亮色改为 #a9dc76；并行事件补齐 source 修复边动画不可见

### 现象
- 你要求：Running hat 的 box 高亮色从“蓝色”改成 `#a9dc76`。
- 你仍然观察不到 event 的线段动画。

### 根因
- 并行模式下 hat 输出的 `<event ...>` 被解析后没有 `event.source`（仅有 `source_instance`）：
  - `crates/ralph-core/src/parallel/instance.rs` 的 `decorate_outgoing_event` 之前只补齐了 `source_instance`；
  - TUI 侧动画触发依赖 `event.source`（发布者 hat），因此动画无法启动。

### 修复
- 颜色改动：
  - `crates/ralph-tui/src/theme.rs`：新增语义化颜色 `TuiTheme::hat_graph_running_hat_fg()`，固定为 `#a9dc76`
  - `crates/ralph-tui/src/app.rs`：Running hats 的 box 高亮改用该语义色
- 并行事件归因修复：
  - `crates/ralph-core/src/parallel/instance.rs`：在 `decorate_outgoing_event` 中补齐 `event.source=hat_id`（若原本为空）
  - 新增回归单测：锁死 `source` 与 `source_instance` 都必须存在

### 验证
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅

---

## 2026-02-03 13:44 +0800｜tui hat graph radar：再下移 1 行，避免遮挡 Output top border

### 现象
- Radar 已向内偏移，但仍会盖住并行模式 Output 面板的顶部边线（border）。

### 根因
- Output 面板 top border 通常位于 y=2（header 高度=2，content_area 从 y=2 开始）。
- Radar 纵向 inset 之前为 2，导致 Radar 的 top border 也落在 y=2，发生覆盖。

### 修复
- `crates/ralph-tui/src/app.rs`
  - `HAT_GRAPH_RADAR_INSET_Y: 2 → 3`，让 Radar top border 从 y=2 下移到 y=3。
  - 更新 clamp 回归测试的期望值（可用高度随 inset_y 变化）。
- `specs/terminal-ui.spec.md`
  - 补充说明：inset 也用于避免覆盖关键 pane 边框（例如 Output top border）。

### 验证
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅

---

## 2026-02-03 00:45 +0800｜tui hat graph radar：对齐 `beautiful-mermaid-rs --ascii` 默认输出（Unicode 文字图，不要纯 ASCII）

### 现象
- TUI 右上角 Hat Graph Radar 的拓扑“文字图”目前是纯 ASCII（+--|）。
- 这与你期望的 `beautiful-mermaid-rs --ascii` 默认效果（Unicode box-drawing：┌─┐│└┘▶）不一致。

### 根因
- `crates/ralph-cli/src/hats.rs` 的 `render_hat_graph_radar_ascii(...)`：
  - compact 渲染使用 `use_ascii: Some(true)`，等价于强制 `--use-ascii`；
  - full 渲染走 `GraphFormat::Ascii`，同样是纯 ASCII。

### 修复
- `crates/ralph-cli/src/hats.rs`：
  - compact/full 统一改为 `use_ascii: Some(false)`，输出 Unicode box-drawing 文字图，语义对齐 `beautiful-mermaid-rs --ascii`。
  - 新增回归测试 `test_render_hat_graph_radar_uses_unicode_box_drawing`，锁死该行为。
- `specs/terminal-ui.spec.md`：
  - 把 Hat Graph Radar 的 “ASCII-only” 修正为“文本图（默认 Unicode box-drawing）”，避免再次误读。
- `crates/ralph-tui/src/lib.rs`：
  - 更新注释，明确 Radar 注入的是“文字图”，默认 Unicode。

### 验证
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅

---

## 2026-02-02 00:35 +0800｜hats graph：complete_publishes（如 spec.approved）无订阅者时在逻辑视图里“消失”

### 现象
- 配置 `event_loop.complete_publishes: "spec.approved"` 后：
  - Mermaid 逻辑视图里看不到 `spec.approved`
  - 因为没有任何 hat 订阅该 topic，图上缺少“结束”路径

### 根因
- 逻辑视图只画 Hat→Hat 订阅关系：
  - `(A publishes T) && (B subscribes T)` 才画边
- `complete_publishes` 是工作流的“结束候选事件”，不要求被 hat 订阅。
  因此会被上述规则过滤掉。

### 修复
- `crates/ralph-cli/src/hats.rs`：
  - 当 `event_loop.complete_publishes = C` 存在时：
    - 固定输出 `Complete[complete]`
    - 对所有发布 `C` 的 hat 画 `Hat_X -->|C| Complete`
- `specs/hats-graph-logical-view.spec.md`：补充 `G5` 规范
- 增加回归测试：`test_generate_mermaid_string_includes_complete_publishes`

### 验证
- `cargo fmt` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test -p ralph-core kiro` ✅
