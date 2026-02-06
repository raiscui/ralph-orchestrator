# Spec: `ralph hats graph --view logical` 输出“逻辑视图”（隐藏调度员 Ralph）

## 背景 / 问题

当前 `ralph hats graph --format mermaid` 会把调度员 `Ralph` 作为中心节点输出，并画出：

- `Ralph -> Hat`（表示：Hat 订阅了某个 topic）
- `Hat -> Ralph`（表示：Hat 发布了某个 topic）
- `Hat -.-> Hat`（表示：虽然内部经 Ralph 调度，但逻辑上 A 发布的 topic 会触发 B）

这在 hat 数量增多时会形成“近似全连接”的视觉噪声。
同时，用户侧希望表达的是 **Hat 与 Hat 之间的逻辑关系**。
调度员在背后存在即可，不应出现在图的“明面上”。

> 备注：
> - 默认 view 是 `--view physical`（包含 coordinator，便于看全貌/看路由）。
> - 本 spec 只约束 `--view logical`（可选：用于隐藏 coordinator，让图更干净）。

---

## 目标（Goals）

### G1：隐藏调度员（Ralph）

`ralph hats graph --format mermaid --view logical` 输出中 **必须**不出现 `Ralph` 节点。
同时 **必须**不出现任何 `Hat -> Ralph` 或 `Ralph -> Hat` 的边。

### G2：以 Hat→Hat 实线展示逻辑关系

图中用于表达“topic 传播”的边 **必须**使用实线 `-->`。
不再使用虚线 `-.->`。

### G3：保持确定性（Deterministic Output）

同一份配置，多次运行输出的 Mermaid 文本结构与顺序 **应该**保持稳定：

- Hat 节点声明顺序稳定（按 `hat.id` 排序）
- Hat→Hat 边的输出顺序稳定（按 `(source_hat_id, topic, target_hat_id)` 排序/去重）

### G4：入口事件（可选展示）

当配置里显式设置 `event_loop.starting_event` 时：

- Mermaid 图 **应该**展示 `Start[task.start] -->|starting_event| Hat` 的入口边
- 入口边的目标 Hat 是所有订阅了 `starting_event` 的 Hat

当 `event_loop.starting_event` 未设置时：

- Mermaid 图 **不应该**输出孤立的 `Start[task.start]` 节点（避免噪声）

### G5：结束事件（complete_publishes）

当配置里显式设置 `event_loop.complete_publishes` 时：

- Mermaid 图 **必须**展示一个固定终点节点：`Complete[complete]`
- 对所有 `publishes` 包含该 topic 的 hats：
  - Mermaid 图 **必须**展示 `Hat -->|complete_publishes| Complete` 的边

备注：
- `complete_publishes` 作为“工作流完成候选事件”，它 **可能没有**任何 hat 订阅。
  因此它不能只依赖 Hat→Hat 的订阅关系推导，否则会在图上“消失”。
- 配置硬门禁（config validate）：
  - 当 `hats` 非空且你设置了 `event_loop.complete_publishes = C`，那么**必须**至少有一个 Hat 的 `publishes` 声明包含 `C`。
  - 否则 Mermaid 图会出现 `Complete[complete]` 但没有任何入边，且 completion candidate 没有明确“生产者”。
  - 为了避免这种“隐式收敛信号”导致 workflow 卡死，Ralph 会直接拒绝该配置并报错。

---

## 非目标（Non-Goals）

- 不要求把所有 external topics（例如 `human.message`、`tool.*`）都画成入口节点。
  这会把图重新拉回“全连接/噪声”状态。
- 不改变实际运行时的事件调度行为：运行时依旧由 Ralph 在背后路由。

---

## 设计要点（Design Notes）

1) Mermaid 的节点 ID 兼容性差异较大。
为兼容 ASCII/Unicode 渲染器（例如 `beautiful-mermaid-rs`），节点 ID 与展示 label **必须**分离：

- 节点 ID：使用 ASCII 安全的 `hat.id`（必要时 sanitize），并加 `Hat_` 前缀
- 节点 label：使用 `hat.name`（可包含中文/emoji），并做最小必要转义

2) “逻辑视图”的边定义：

- 若 Hat A 的 `publishes` 包含 topic `T`
- 且 Hat B 的 `subscriptions` 包含 topic `T`
- 且 A != B
- 则图中应存在边：`A -->|T| B`

3) `complete_publishes` 的边定义：

- 若配置存在 `event_loop.complete_publishes = C`
- 且 Hat A 的 `publishes` 包含 topic `C`
- 则图中应存在边：`A -->|C| Complete`

---

## 验收标准（Acceptance Criteria）

- `ralph hats graph --format mermaid --view logical` 输出中：
  - 不包含字符串 `Ralph`
  - 不包含 `-.->`
  - 对于任意 `(A publishes T)` 与 `(B subscribes T)` 的组合，存在 `A -->|T| B`
  - 当 `event_loop.complete_publishes` 存在且某个 Hat 发布该 topic 时：
    - 存在 `Complete[complete]`
    - 存在 `Hat -->|complete_publishes| Complete`

- 回归测试覆盖：
  - 基础两节点：A 发布 mid，B 订阅 mid，输出必须包含 `Hat_A -->|mid| Hat_B`
  - complete_publishes：当配置指定 `complete_publishes = end` 且 Hat_B 发布 end 时，输出必须包含 `Hat_B -->|end| Complete`
  - 中文/emoji hat 名称：渲染后的 unicode/ascii 输出中必须包含中文/emoji label（证明 ID/label 分离仍有效）

- `cargo fmt --check`、`cargo clippy --all-targets --all-features -- -D warnings`、`cargo test`（含 replay smoke tests）全部通过。
