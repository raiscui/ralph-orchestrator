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
- `cargo test -p ralph-core kiro` ✅

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
