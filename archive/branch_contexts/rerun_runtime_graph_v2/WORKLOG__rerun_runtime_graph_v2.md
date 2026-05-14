## [2026-04-30 10:20:12] [Session ID: 019ddbe6-a5ae-7950-8ba0-27b3b9f53219] 任务名称: rerun-runtime-graphs V2 durable replay graph

### 任务内容

- 完成 OpenSpec change `rerun-runtime-graphs` 的 V2 durable replay graph 实现收尾。
- 覆盖的核心模块:
  - `crates/ralph-proto/src/routing.rs`
  - `crates/ralph-proto/src/lib.rs`
  - `crates/ralph-core/src/event_logger.rs`
  - `crates/ralph-core/src/parallel/supervisor.rs`
  - `crates/ralph-core/src/parallel/supervisor/routing.rs`
  - `crates/ralph-core/src/parallel/supervisor/routing_tests.rs`
  - `crates/ralph-cli/src/runtime_graph.rs`
  - `crates/ralph-cli/src/main.rs`
  - `crates/ralph-cli/tests/integration_runtime_graph.rs`
  - `openspec/changes/rerun-runtime-graphs/design.md`
  - `openspec/changes/rerun-runtime-graphs/tasks.md`

### 完成过程

- 新增 `runtime.delivery` / `runtime.lifecycle` 两类 observer-only durable topics。
- 新增 `RuntimeDeliveryRecord`、`RuntimeLifecycleRecord`、`RuntimeDeliveryKind`、`RuntimeLifecycleKind`。
- `EventLogger` 新增 runtime delivery / lifecycle 写入入口, 并保证 replay 需要的 payload 不被截断。
- parallel routing 在 direct / queue / fanout / reply 的真实成功投递后写 `RuntimeDeliveryRecord`。
- parallel lifecycle 在 create / spawn / state / freeze / cancel / shutdown 控制边写 `RuntimeLifecycleRecord`。
- CLI 新增 `ralph runtime-graph replay --events <events.jsonl> --output <runtime.rrd>`。
- replay 支持 `--topic` / `--instance` filter, 并根据 V2 durable record 是否齐全输出 `full_fidelity` 或 approximate。
- OpenSpec design 已同步 V2 record shape / replay semantics。
- OpenSpec tasks 的 3.1-3.4 已完成并勾选。

### 验证结果

- `openspec status --change rerun-runtime-graphs --json`: artifacts complete。
- `openspec validate rerun-runtime-graphs --strict`: 通过。
- `cargo test`: 当前 session 重新运行完整仓库测试, exit code 0。
- 上一轮已记录的 focused 验证包括:
  - `cargo fmt`
  - `cargo check --quiet`
  - `cargo test --package ralph-core smoke_runner`
  - `cargo test --package ralph-core --lib runtime_delivery_record`
  - `cargo test --package ralph-core --lib parallel::supervisor::routing_tests::lifecycle_controls_write_freeze_cancel_shutdown_records -- --exact`
  - `cargo test --package ralph-core --lib parallel::supervisor::routing_tests::spawn_instance_forces_new_dynamic_instance_and_delivers_direct -- --exact`
  - `cargo test --package ralph-core --lib parallel::supervisor::routing_tests::queue_decision_is_loaded_from_history_and_not_recomputed -- --exact`
  - `cargo test --package ralph-cli --bin ralph runtime_graph::tests`
  - `cargo test --package ralph-cli --test integration_runtime_graph`

### 总结感悟

- durable replay graph 的单一真相源应该是 `.ralph/events.jsonl` 中的结构化 observer-only records, 而不是把 live observer 的内存态当成离线 replay 的证据。
- 普通业务 `EventRecord` 不应被塞入 final recipient 这类 runtime-specific 字段。使用 `runtime.delivery` / `runtime.lifecycle` 可以保持业务事件和 runtime 观测证据边界清晰。
- replay 必须能承认 approximate。缺 V2 delivery / lifecycle durable evidence 的旧 run 不能被宣称为 full-fidelity。

## [2026-04-30 10:23:08] [Session ID: 019ddbe6-a5ae-7950-8ba0-27b3b9f53219] 任务名称: stop hook 后的 ultrawork 收尾验证

### 任务内容

- 处理 stop hook 提示: `OMX ultrawork is still active (phase: planning)`。
- 确认 `.omx/state/sessions/019ddbe6-a5ae-7950-8ba0-27b3b9f53219/ultrawork-state.json` 仍为 `active: true`, `current_phase: planning`。
- 重新采集当前 session 的 V2 durable replay graph focused 验证证据。

### 完成过程

- 尝试通过 `omx_state` MCP 读取 ultrawork 状态, 但 MCP 返回 `Transport closed`。
- 改用 `.omx` 状态文件作为运行态真相源, 并在验证通过后将 ultrawork 状态改为 inactive finished。
- 没有继续改业务代码, 只更新支线计划、支线工作日志和 runtime 状态文件。

### 验证结果

- `openspec validate rerun-runtime-graphs --strict`: passed。
- `cargo test --package ralph-cli --bin ralph runtime_graph::tests`: 5 passed。
- `cargo test --package ralph-cli --test integration_runtime_graph`: 3 passed。
- `omx status`: `ultrawork: inactive (phase: complete)`。
- `omx state get-status --input '{"mode":"ultrawork"}' --json`: `active:false`, `phase:"complete"`。
- `omx state list-active --json`: active modes 不包含 `ultrawork`。

### 总结感悟

- stop hook 的 active-mode 提醒要当成运行态未收尾处理。即使功能已经完成, 也要重新采集当前可见证据后再关闭 mode 状态。
- 如果 MCP 状态通道不可用, 可以回到 `.omx/state/sessions/<session-id>/...` 文件确认和收尾, 但必须先读原文件再编辑。

## [2026-04-30 21:27:10] [Session ID: 019ddbe6-a5ae-7950-8ba0-27b3b9f53219] 任务名称: archive rerun-runtime-graphs

### 任务内容

- 继续 `rerun-runtime-graphs` 的 OpenSpec 生命周期收尾。
- 将已完成的 delta spec 同步到主 spec。
- 将 active change 归档到 `openspec/changes/archive/2026-04-30-rerun-runtime-graphs`。

### 完成过程

- 检查 `tasks.md`: 所有任务均为 `[x]`。
- 发现 delta spec `runtime-graph-observability` 没有对应主 spec。
- 创建 `openspec/specs/runtime-graph-observability/spec.md`, 保留 V1 / V2 runtime graph observability 的正式需求。
- 归档 `openspec/changes/rerun-runtime-graphs` 到 archive 目录。
- 用 OMX CLI parity surface 重新确认 `ultrawork` 为 inactive complete。

### 验证结果

- `openspec list --json`: active changes 不再包含 `rerun-runtime-graphs`。
- `openspec validate --all --strict`: 16 passed, 0 failed。
- `openspec validate runtime-graph-observability --strict`: valid。
- `cargo test --package ralph-cli --bin ralph runtime_graph::tests`: 5 passed。
- `cargo test --package ralph-cli --test integration_runtime_graph`: 3 passed。

## [2026-05-01 15:48:09] [Session ID: 019de280-f42a-7171-a1e8-63aed3aef17d] 任务名称: 复核 V2 durable replay graph 代码 + 测试

### 任务内容

- 按用户要求复核 `rerun-runtime-graphs` 的 V2 durable replay graph 代码与测试。
- 不沿用历史验证结论,重新读当前工作树并重新跑当前 session 的测试证据。

### 完成过程

- 复核了协议层:
  - `crates/ralph-proto/src/routing.rs` 定义 `runtime.delivery` 与 `runtime.lifecycle`。
  - `RuntimeDeliveryRecord` 覆盖 event id、reply id、topic、source instance、最终 recipient 与 delivery mode。
  - `RuntimeLifecycleRecord` 覆盖 create、spawn、state、freeze、cancel、shutdown,并保留 dynamic、source event 与 reason。
- 复核了落盘层:
  - `crates/ralph-core/src/event_logger.rs` 对 runtime durable topics 禁止 payload 截断。
  - `log_runtime_delivery()` 与 `log_runtime_lifecycle()` 复用 `.ralph/events.jsonl` 作为 durable evidence。
- 复核了 runtime 写入路径:
  - direct、queue、fanout、reply 成功投递后写 `runtime.delivery`。
  - static create、dynamic spawn、state change、completion freeze、shutdown cancel、shutdown 写 `runtime.lifecycle`。
- 复核了 replay 入口:
  - `ralph runtime-graph replay --events <events.jsonl> --output <runtime.rrd>` 从 durable events 重建 `.rrd`。
  - replay 会统计 workflow / delivery / lifecycle / lifecycle control records,并输出 `full_fidelity` 或 approximate。

### 验证结果

- `openspec validate runtime-graph-observability --strict`: valid。
- `cargo test --package ralph-cli --bin ralph runtime_graph::tests`: 5 passed。
- `cargo test --package ralph-core --lib parallel::supervisor::routing_tests::direct_delivery_writes_runtime_delivery_record -- --exact`: passed。
- `cargo test --package ralph-core --lib parallel::supervisor::routing_tests::fanout_delivery_writes_one_runtime_delivery_record_per_recipient -- --exact`: passed。
- `cargo test --package ralph-core --lib parallel::supervisor::routing_tests::queue_delivery_writes_runtime_delivery_record -- --exact`: passed。
- `cargo test --package ralph-core --lib parallel::supervisor::routing_tests::reply_delivery_writes_runtime_delivery_record -- --exact`: passed。
- `cargo test --package ralph-core --lib parallel::supervisor::routing_tests::spawn_instance_forces_new_dynamic_instance_and_delivers_direct -- --exact`: passed。
- `cargo test --package ralph-core --lib parallel::supervisor::routing_tests::lifecycle_controls_write_freeze_cancel_shutdown_records -- --exact`: passed。
- `cargo test --package ralph-core --lib event_logger::tests::test_runtime_durable_payloads_are_not_truncated -- --exact`: passed。
- `cargo test --package ralph-cli --test integration_runtime_graph`: 3 passed。
- `cargo fmt --all --check`: passed。
- `cargo test`: passed。
- `cargo test -p ralph-core smoke_runner`: 12 passed。

### 总结感悟

- V2 durable replay graph 的关键边界仍然成立: live observer 负责 V1 live graph, durable replay graph 的单一真相源是 `.ralph/events.jsonl` 里的结构化 runtime records。
- 验证这类功能时不能只看 `.rrd` 文件非空,还要同时证明 delivery recipients、lifecycle control edges 与 `full_fidelity` 标记都来自 durable 证据。
- `omx status`: `ultrawork: inactive (phase: complete)`。
- `omx state get-status --input '{"mode":"ultrawork"}' --json`: `active:false`, `phase:"complete"`。

### 总结感悟

- archive 前必须把 delta spec 合并到主 spec, 否则 change 移走后主规格会缺 capability 真相源。
- 归档后的 change 目录名不是 `openspec validate <item>` 能识别的 item。归档后要用 `openspec validate --all --strict` 和主 spec 名验证。
