# 并行触发路由（Parallel Trigger Routing，应用示例）

这是一个可运行的端到端示例：**parallel-trigger-routing**。

它用于演示在 `parallel.enabled: true` 且**没有**配置 `parallel.topic_contracts` 时的默认路由语义：

- `topic -> hats`：把事件扇出（fanout）给所有订阅该 topic 的 hat（`hats.*.triggers`）
- `hat -> instance`：对每个 hat，把事件排队给且仅给一个实例（优先空闲实例，其次轮询）

## 你应该看到什么

这个示例会刻意产生**两次** `spec.ready` 事件：

1. `spec_writer` 发出 `spec.ready`，并带上 `version: 1`
2. `spec_reviewer` 拒绝（`spec.rejected`）
3. `spec_writer` 修订后再次发出 `spec.ready`，并带上 `version: 2`
4. `spec_reviewer` 通过（`spec.approved`）
5. Ralph 收到 `spec.approved` 后输出 `LOOP_COMPLETE`

`spec.ready` 同时被**两个 hat** 订阅（`spec_reviewer` 和 `spec_logger`），因此它应该会同时触发两者。

`spec_logger` 配置为 `instances: 2`，所以两次 `spec.ready` 通常会分别由下面两个实例处理：

- `spec_logger#1`（第一次 `spec.ready`）
- `spec_logger#2`（第二次 `spec.ready`）

## 运行

在仓库根目录执行：

```bash
# 只使用配置（目标 prompt 已通过 event_loop.prompt 内联）
cargo run --bin ralph -- run \
  -c examples/parallel-trigger-routing/ralph.yml \
  --no-tui
```

可选：在 CLI 上覆盖 backend（如果你默认 backend 没配好，建议显式指定）：

```bash
cargo run --bin ralph -- run \
  -c examples/parallel-trigger-routing/ralph.yml \
  -b codex \
  --no-tui
```

## 备注

- 这个示例是刻意做成"触发器驱动"（trigger-driven）的，不使用 `parallel.topic_contracts`。
- 如果你需要更明确的投递/受众规则，可以添加 topic contracts；它们的优先级高于 triggers。
- 这个示例把工作流的入口/出口语义写在配置里（这是官方并行语义）：
  - `event_loop.starting_event: "spec.start"`
  - `event_loop.complete_publishes: "spec.approved"`
- 目标 prompt 内联在 `event_loop.prompt` 中，所以这个示例不依赖额外的 prompt 文件。
- 如果你需要给协调者 ralph#1 注入一段“固定语义锚点/行为约束”，可以使用：
  - `event_loop.ralph_prompt: | ...`
  - 它只会注入给 ralph（协调者），不会污染其他 hats 的 prompt（避免 prompt pollution）。
