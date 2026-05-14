## [2026-04-30 09:31:04] [Session ID: 019ddbe6-a5ae-7950-8ba0-27b3b9f53219] 笔记: V2 durable replay graph 证据缺口盘点

## 来源

### 来源1: OpenSpec design / spec

- `openspec/changes/rerun-runtime-graphs/design.md` 明确 V2 必须补齐 delivery 级 durable 记录。
- `openspec/changes/rerun-runtime-graphs/specs/runtime-graph-observability/spec.md` 明确 replay graph 必须来自 durable artifacts,不能只靠 live observers。
- V2 至少需要 durable evidence:
  - 最终 delivery recipients,包括 `target_instance` 和 fanout recipients。
  - source / target instance 边。
  - create/spawn lineage。
  - lifecycle control edges,包括 freeze、cancel、shutdown。

### 来源2: 当前 event durable log

- `crates/ralph-core/src/event_logger.rs` 的 `EventRecord` 已有 `source_instance`、`id`、`reply`、`topic`、`payload`。
- `EventRecord` 当前没有最终 recipient 字段,也没有 fanout recipients 字段。
- `dispatch.decision` 已经是 replay 用 observer-only durable event 的先例。
- `dispatch.decision` 的 payload 不允许截断,说明 replay 级结构化 payload 可以复用同一个事件日志通道。

### 来源3: 当前 live runtime graph

- `crates/ralph-cli/src/runtime_graph.rs` 的 V1 live graph 通过 `RuntimeDeliveryObservation` 获取最终投递边。
- `RuntimeDeliveryObservation` 只来自 live observer,字段包括 topic、source_instance、recipient、mode。
- 离线 replay 当前无法重放这些 delivery edges,因为 events.jsonl 里没有同等信息。

### 来源4: 当前真实投递路径

- direct: `route_event` 发现 `event.target_instance` 后调用 `deliver_to_instance_id(..., Direct)`。
- queue: `deliver_queue` 会先写 `dispatch.decision`,再调用 `deliver_to_instance_id(..., Queue)`。
- fanout: `deliver_fanout` 循环给每个 recipient 发 `HatInstanceCommand::Deliver`,但没有 durable recipients 记录。
- reply: `reply.hat.message` 解析 requester 后调用 `deliver_to_instance_id(..., Reply)`。

### 来源5: 当前 lifecycle / spawn 路径

- 静态实例创建在 `spawn_instances` 内直接写入 `instance_states` 并通知 live observer。
- 动态实例创建在 `spawn_dynamic_instance` / `spawn_instance` 内完成。
- completion promise 后通过 `freeze_pending_on_all_instances` 冻结 pending。
- Supervisor 退出时 `shutdown_instances` 会先发送 `CancelCurrentJob`,再发送 `Shutdown`。
- 这些控制边目前没有 durable record,离线 replay 无法知道 freeze/cancel/shutdown 发生过。

## 综合发现

### 3.1 证据缺口

- 缺 final recipient durable evidence:
  - direct / queue / reply 最终只在 live observer 中有 recipient。
  - fanout recipients 只存在于运行时 `recipients` slice 和 live observer 回调。
- 缺 lifecycle durable evidence:
  - 静态 create、动态 spawn、completion freeze、shutdown cancel、shutdown 本身都没有结构化 durable event。
- 缺 offline replay 入口:
  - `--runtime-graph-rrd` 只能录 live graph。
  - 当前没有从 `.ralph/events.jsonl` 重建 `.rrd` 的 CLI。

### 实现方向

- 采用新的 observer-only durable topics,而不是改普通业务 event schema:
  - `runtime.delivery`: 一条真实投递对应一条 durable delivery record。
  - `runtime.lifecycle`: 一条实例 lifecycle 或控制动作对应一条 durable lifecycle record。
- 这两个 topic 的 payload 必须像 `dispatch.decision` 一样不截断。
- replay graph 按 events.jsonl 行顺序重建 runtime_step。
- topic filter 只过滤 workflow / delivery topic; lifecycle record 是 topicless,默认保留,除非 instance filter 排除。
- instance filter 保留与该实例相关的 source / recipient / lifecycle record。
