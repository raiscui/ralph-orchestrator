# 任务计划：`ralph hats graph --format mermaid` 隐藏调度员节点（Ralph），输出“逻辑视图”拓扑

## 目标

- `ralph hats graph --format mermaid` 输出的 Mermaid 图中：
  - **不出现**调度员节点 `Ralph`（避免全连接导致线路混乱）。
  - **不出现**任何 `Hat -> Ralph` / `Ralph -> Hat` 的边（这些是内部调度细节）。
  - Hat 与 Hat 之间的“逻辑连线”用 **实线** `-->`（不使用虚线 `-.->`）。
- 保持输出尽量确定性（同一份配置，多次输出结构与顺序稳定）。
- 增加/更新回归测试，锁死该行为，避免回归。

## 方案（至少二选一）

### 方案 A：仅 Mermaid 隐藏 Ralph（改动更小）

- 仅 `--format mermaid` 改为“逻辑视图”（隐藏 Ralph、实线 Hat→Hat）。
- `--format unicode/ascii/compact` 保持当前“物理视图”（包含 Ralph 中心节点）。
- 优点：对默认输出（unicode）影响最小。
- 缺点：不同 format 的语义不一致，用户心智负担更大。

### 方案 B：所有 format 统一逻辑视图（更一致｜我将按此执行）

- Mermaid + unicode/ascii/compact 全部使用“逻辑视图”：
  - 不显示 Ralph 节点
  - 只展示 Hat→Hat 的 topic 传播关系（实线）
  - （可选）当配置 `event_loop.starting_event` 存在时，增加 `Start[task.start] -->|starting_event| Hat` 入口边
- 优点：语义一致，图更干净，符合“Ralph 在背后调度”的表达。
- 缺点：默认 unicode/ascii 的输出会变化（但更清爽）。

> 决定：选择 **方案 B**，因为你明确希望“明面上不要出现 Ralph”，并且 Hat→Hat 逻辑边应是实线。

## 阶段

- [x] 阶段1：需求确认 + 写入 spec
- [x] 阶段2：实现 mermaid 生成逻辑视图（隐藏 Ralph + 实线 Hat→Hat）
- [x] 阶段3：更新/补充回归测试
- [x] 阶段4：验证（fmt/clippy/test + replay smoke tests）
- [x] 阶段5：四文件记录（notes/WORKLOG/ERRORFIX）

## 关键问题

1. 是否需要保留入口节点 `Start[task.start]`？
   - 我倾向：仅在 `event_loop.starting_event` 明确设置时才显示入口边，避免孤立节点与多余噪声。
2. 图的“逻辑边”是否需要去重？
   - 我倾向：按 `(source_hat, topic, target_hat)` 去重，保证输出稳定且不重复刷边。

## 状态

**已完成**：
- Mermaid 输出改为“逻辑视图”，明面上不再出现 `Ralph`，Hat→Hat 统一实线 `-->`
- 当 `event_loop.starting_event` 显式存在时，补 `Start[task.start] -->|starting_event| Hat` 入口边
- 已更新回归测试，并完成全量验证（含 replay smoke tests）

---

## 2026-02-02 00:35 +0800｜追加需求：把 `complete_publishes` 标记为 `Complete[complete]` 终点节点

### 新目标

- 当配置 `event_loop.complete_publishes` 存在时：
  - Mermaid 逻辑视图里 **必须**出现一个固定终点节点：`Complete[complete]`
  - 任意 hat 若 `publishes` 包含该 topic，则必须画出：`Hat_X -->|complete_publishes| Complete`
- 这个 topic 可能没有任何 hat 订阅，因此不能只依赖 Hat→Hat 的订阅关系推导。

### 阶段（本小改动）

- [x] 阶段A：补充 spec（complete 节点规则）
- [x] 阶段B：实现（生成 Complete 节点与边）
- [x] 阶段C：更新回归测试
- [x] 阶段D：验证（fmt/clippy/test + replay smoke tests）
- [x] 阶段E：四文件追加记录

### 当前状态

**已完成**：
- `complete_publishes` 会显示为 `Complete[complete]`，并从发布该 topic 的 hat 画到 Complete
- 已补 spec、回归测试与全量门禁验证

---

## 2026-02-02 02:39 +0800｜需求：TUI 右上角 Hat Graph Radar（ASCII Mermaid），按键 `p` 放大/还原

### 目标

- TUI 界面右上角常驻一个 “Hat Graph Radar” 覆盖层：
  - 显示 **ASCII** 风格的 hats graph（由 Mermaid 结构渲染而来）。
  - 默认是小窗（类似游戏右上角小雷达）。
  - 按 `p` 放大为大窗，再按 `p` 还原为小窗（纯 UI 行为，不影响 orchestration）。
- 该覆盖层必须是“只读观察”：
  - 不发送事件、不修改运行逻辑。
  - 只改变显示尺寸。
- 在 `Warp` 等终端启用“终端默认背景（bg=Reset）”模式时，边框背景不能把外圈染成不透明。
- 增加/更新回归测试，锁死：
  - `p` 键映射与状态切换
  -（尽量）Overlay 位置与尺寸的基础约束（至少不 panic / 不越界）

### 方案（至少二选一）

#### 方案 A：只显示 Mermaid 源码（最省事）

- 右上角展示 `flowchart LR ...` 的 Mermaid 源码（纯文本）。
- 放大/还原只是“显示更多行”。
- 优点：
  - 完全不依赖 Mermaid 渲染器；
  - 实现最简单、风险最低。
- 缺点：
  - 不像“雷达图”，视觉上不够直观；
  - Mermaid 源码在小窗里可读性差。

#### 方案 B：复用 `beautiful-mermaid-rs` 渲染 ASCII 图（更像雷达｜我将按此执行）

- 复用 `ralph hats graph` 已有的 Mermaid 生成逻辑：
  - 生成 Mermaid（逻辑视图）。
  - 用 `beautiful-mermaid-rs` 渲染成 ASCII 图。
- 小窗用更紧凑的渲染参数（padding=0），大窗用正常参数（更清晰）。
- 优点：
  - 视觉上更接近“雷达/小地图”；
  - 与 CLI 的 `ralph hats graph --format ascii` 行为一致，可复用测试基线。
- 缺点：
  - 需要在 TUI 启动时注入预渲染字符串（或给 ralph-tui 增加依赖）。

> 决定：选择 **方案 B**。理由：你要的是“右上角雷达图”观感，ASCII 渲染更符合直觉，并且我们已经在 `ralph hats graph` 里跑通了 Mermaid→ASCII 的链路。

### 阶段

- [x] 阶段1：补充 spec（UI 行为/布局/按键/验收）
- [x] 阶段2：实现 TUI overlay（右上角渲染 + Warp bg=Reset 细节）
- [x] 阶段3：实现 `p` 键切换（串行/并行模式都可用；Chat 输入时不抢键）
- [x] 阶段4：CLI 注入图数据（复用 hats graph 渲染逻辑）
- [x] 阶段5：回归测试 + `cargo test`（含 replay smoke tests）
- [x] 阶段6：四文件记录（notes/WORKLOG；若有坑再写 ERRORFIX）

### 关键问题

1. 覆盖层在并行模式下是否也显示？
   - 我倾向：**显示**。同一份 config 的 hats 拓扑对并行/串行都成立，且“雷达图”更有价值。
2. `p` 键在并行模式的 Chat 输入框里怎么办？
   - 我倾向：Chat 聚焦时 `p` 作为文本输入，不触发缩放；否则作为全局缩放键。

### 状态

**已完成**：
- 已更新 spec：`specs/terminal-ui.spec.md`
- 已实现 TUI 右上角 Hat Graph Radar 覆盖层（小窗/放大切换）
- 已实现按键 `p` 切换（Chat 输入时不抢键）
- 已在 `ralph-cli` 启动 TUI 时注入 hats graph 的 ASCII 渲染（compact/full）
- 已验证：`cargo fmt` + `cargo clippy --all-targets --all-features` + `cargo test` + `cargo test -p ralph-core smoke_runner`
---

## 2026-02-02 16:06 +0800｜调研：Tenere（pythops/tenere）是如何做语法高亮的

### 目标

- 搞清楚 Tenere 的“语法高亮”具体依赖哪些库。
- 搞清楚它的渲染链路（从 LLM 输出到 TUI 显示）是怎么接起来的。
- 记录关键实现点，方便在 Ralph/TUI 里做同类能力时复用思路。

### 阶段（本次调研）

- [x] 阶段1：定位入口（Cargo.toml / formatter 模块）
- [x] 阶段2：阅读关键实现（Formatter / Chat 渲染）
- [x] 阶段3：形成结论（高亮链路 + 关键点 + 限制）
- [x] 阶段4：四文件记录（notes/WORKLOG）

### 状态

**已完成**：
- Tenere 语法高亮并不是自己实现的 tokenization，而是“借用 bat 产出 ANSI 颜色”。
- 通过 `ansi-to-tui` 把 ANSI 转成 `ratatui::text::Text`，再用 `Paragraph` 渲染到 TUI。
- 输入文件名固定为 `"text.md"`，让 bat 按 Markdown 处理，并对 fenced code block 做语言高亮。

---

## 2026-02-02 21:09 +0800｜OpenSpec FF：tui-codeblock-syntax-highlighting（产出 artifacts，未实现）

### 目标

- 把“只对 fenced code block 做语法高亮（TUI + stdout pretty）、未闭合不高亮、闭合后冻结缓存”的需求固化为 OpenSpec change artifacts。
- 让实现阶段可以直接按 tasks.md 逐条落地，而不再反复讨论范围与取舍。

### 阶段（本次 FF）

- [x] 阶段1：创建 change 目录（spec-driven schema）
- [x] 阶段2：编写 proposal / design / specs / tasks
- [x] 阶段3：`openspec status` 验证进入 apply-ready 状态

### 状态

**已完成**：
- 已生成 `openspec/changes/tui-codeblock-syntax-highlighting/` 下的全部 artifacts（proposal/design/specs/tasks）
- 当前 change 已处于 “All artifacts complete” 状态，可进入 `/opsx:apply` 开始实现

---

## 2026-02-02 21:09 +0800｜实施：tui-codeblock-syntax-highlighting（只高亮 fenced code block）

### 目标（来自 OpenSpec）

- 渲染模式为 Rendered 时：
  - 对 fenced code block 做语法高亮（支持：rust、bash/sh、json、yaml/yml、toml、python/py、js/javascript、ts/typescript）。
  - 未闭合 code block 不做语法高亮（只用统一 code 样式），闭合后一次性高亮并冻结。
  - TUI 与 stdout pretty 输出语义一致（都走同一套高亮与降级逻辑）。
- `--plain` 模式：
  - fences 原样可见，且不产生 code block 语法高亮 ANSI。
- 性能优先：
  - 避免每个流式 chunk 都全量重渲染历史内容（引入冻结块/缓存）。

### 阶段（按 tasks.md 拆分）

- [x] 阶段1：依赖与 queries（tree-sitter + 语法查询离线化）
- [x] 阶段2：分段器（跨 chunk fence 识别 + lang normalize + 降级）
- [x] 阶段3：高亮渲染器（闭合后一次性高亮 + ANSI 输出）
- [x] 阶段4：集成到 TUI/stdout（冻结块/缓存 + 一致性）
- [x] 阶段5：回归测试 + 门禁验证（fmt/clippy/test/smoke_runner）

### 当前状态

**已完成（All Done）**：
- Rendered 模式下 fenced code block 已支持 tree-sitter 语法高亮（闭合后一次性高亮）。
- 未闭合 code block 始终保持统一 code 样式（不高亮），closing fence 到来后才高亮。
- `TuiStreamHandler` 已引入“冻结块/缓存”，避免每个 chunk 全量重渲染历史内容。
- stdout pretty 已复用同一套渲染链路（ANSI 中间表示，行为与 TUI 一致）。
- 已补回归测试并通过 fmt/clippy/test + replay smoke runner。

---

## 2026-02-03 00:09 +0800｜撤销临时 workaround：beautiful-mermaid-rs 性能已修复，回退两笔历史修改

### 背景

- 你说明 `/Users/cuiluming/local_doc/l_dev/my/rust/beautiful-mermaid-rs` 的性能问题已经在上游修复。
- 因此我们需要把 Ralph 侧（或相关仓库）之前为规避该性能问题而引入的两笔修改撤回：
  - `e88d58944be2e3297662817ba9bfb8d653cc747a`
  - `e7c2e92d3b2619f01734ac1622e34c17ad5e25f2`

### 目标

- 回退上述两个 commit 的代码改动，使行为回到“未加 workaround”状态。
- 回退后通过全量门禁验证（fmt/clippy/test + smoke runner）。

### 阶段

- [x] 阶段1：定位两个 commit 属于哪个仓库 + 阅读 diff（避免误回退）
- [x] 阶段2：按正确顺序执行 revert（必要时解决冲突）
- [x] 阶段3：全量门禁验证（fmt/clippy/test + smoke runner）
- [x] 阶段4：四文件记录（notes/WORKLOG；如遇冲突或踩坑再写 ERRORFIX）

### 当前状态

**已完成（All Done）**：
- 已确认这两个 commit 都在 `ralph-orchestrator` 仓库内（不是 `beautiful-mermaid-rs`）。
- 已完成 revert：
  - `Revert "docs: record tui radar startup regression fix"`（`e88d589...`）
  - `Revert "tui: avoid slow mermaid-ascii radar generation"`（`e7c2e92...`）
- 已通过全量门禁验证：
  - `cargo fmt --check` ✅
  - `cargo clippy --all-targets --all-features -- -D warnings` ✅
  - `cargo test` ✅
  - `cargo test -p ralph-core smoke_runner` ✅

---

## 2026-02-03 00:42 +0800｜修正：Hat Graph Radar 对齐 `beautiful-mermaid-rs --ascii`（Unicode 线条字符文字图）

### 背景 / 现象

- `beautiful-mermaid-rs` 的 CLI：`--ascii` 默认会输出 **Unicode 线条字符**（┌─┐│└┘▶），而 `--ascii --use-ascii` 才会强制 “纯 ASCII（+--|）”。
- 目前 Ralph 的 Hat Graph Radar（右上角覆盖层）使用了 `use_ascii: Some(true)`，输出是 “纯 ASCII（+--|）”，因此与你期望的效果不一致。

### 目标

- Hat Graph Radar（compact + full）统一改为 **Unicode 线条字符文字图**（对齐 `beautiful-mermaid-rs --ascii`）。
- 同步 `specs/terminal-ui.spec.md`：把 “ASCII-only” 改为“文本图（默认 Unicode box-drawing）”，避免误导。
- 增加/调整回归测试，锁死：
  - Radar 的渲染结果包含 Unicode box-drawing 字符（例如 `┌` / `─` / `│`）。

### 阶段

- [x] 阶段1：补充/更新 spec（Hat Graph Radar 输出字符集语义）
- [x] 阶段2：实现（Radar 渲染 options：`use_ascii: Some(false)`）
- [x] 阶段3：回归测试（Unicode 断言）
- [x] 阶段4：门禁验证（fmt/clippy/test + replay smoke tests）
- [ ] 阶段5：四文件记录 + git 提交

### 当前状态

**进行中**：阶段5（四文件记录已追加，待 git commit）。
