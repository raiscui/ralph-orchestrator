## [2026-05-18 19:16:59] [Session ID: multi-agent-collab#1] 笔记: multi-agent collaboration 证据调查

## 来源

### 来源1: `crates/ralph-core/src/parallel`

- `mod.rs` 明确目标: 多个 hat / 同一 hat 多实例并行执行,路由决策可落盘、可回放。
- `supervisor.rs` 维护 `instances`、`instances_by_hat`、`instance_states`、`queue_decisions`、`request_reply_origins`、`.ralph/agents.json` 快照和 runtime delivery observer。
- `instance.rs` 定义 actor inbox/outbox: Supervisor 投递 `Deliver`,实例回传 `StateChanged` / `JobCompleted` / `Published`。
- `routing.rs` 是事件路由主体:
  - `reply.hat.message` 根据 `reply=<request_event_id>` 回送给原请求实例。
  - 没有 `topic_contracts` 时回退到 `hats.*.triggers`。
  - `fanout` 会给每个 recipient 写 runtime delivery。
  - `queue` 会写 `dispatch.decision`,并从历史读取以便 replay 不重算。
  - `spawn_instance` / autoscale 可创建动态实例。

### 来源2: `crates/ralph-e2e/src/scenarios/parallel*`

- `parallel/mod.rs` 标记 Tier 8 Parallel Runtime,说明这些场景验证真实后端上的 parallel hat instances。
- `parallel/hat_instances.rs` 断言:
  - stdout 出现 `[supervisor] instances`。
  - stdout 有 `writer#1`、`writer#2`、`tester#1` 前缀。
  - events 里有 `build.task` / `build.done` / `test.done`。
  - 非法 target 会写 `routing.escalate`。
  - `LOOP_COMPLETE` 后不能再出现新 job_id。
- `parallel_trigger_routing_example.rs` 直接跑 `examples/parallel-trigger-routing`,断言 `spec_writer=2`、`spec_reviewer=2`、`spec_logger=3`,并检查 `.ralph/agents.json` 包含关键 hats。
- `parallel_human_approval_gate_example.rs` 在 run 执行中等待 `approval.requested`,再并发执行 `ralph emit approval.granted --target-instance ralph#1`,断言 topic 顺序和最终 `deployment.ready`。
- `parallel/emit_spawn_instance.rs` 验证 `ralph emit --spawn-instance` 创建动态 worker,并用 `.ralph/agents.json` + `.ralph/events.jsonl` 做交叉证据。

### 来源3: docs / specs / examples

- `specs/parallel-hat-instances/e2e.md` 写明 E2E 目标: 多实例启动、trigger fanout、归因输出、events 落盘、`routing.escalate`。
- `crates/ralph-e2e/README.md` Tier 8 列出 parallel-hat-instances、parallel-emit-spawn-instance、parallel-trigger-routing-example、parallel-human-approval-gate-example 等场景。
- `examples/parallel-trigger-routing/ralph.yml` 是可运行配置: `parallel.enabled: true`,三种 hats,`spec_logger.instances: 2`,以 `spec.start -> spec.ready -> spec.rejected/spec.approved` 展示协作闭环。
- `examples/parallel-trigger-routing/README.md` 解释默认路由语义: `topic -> hats` fanout,`hat -> instance` 单实例排队。

## Fresh verification evidence

- `cargo test --package ralph-core --lib parallel::supervisor::routing_tests::fanout_delivery_writes_one_runtime_delivery_record_per_recipient -- --exact`
  - 输出: `1 passed; 0 failed`。
  - 验证: fanout 投递会为每个 recipient 写 runtime delivery record。
- `cargo test --package ralph-core --lib parallel::supervisor::routing_tests::queue_delivery_writes_runtime_delivery_record -- --exact`
  - 输出: `1 passed; 0 failed`。
  - 验证: queue 投递路径会写 runtime delivery record。
- `cargo test --package ralph-core --lib parallel::supervisor::routing_tests::spawn_instance_forces_new_dynamic_instance_and_delivers_direct -- --exact`
  - 输出: `1 passed; 0 failed`。
  - 验证: `spawn_instance` 会创建动态实例并直达投递。
- `cargo test --package ralph-e2e --lib scenarios::parallel_trigger_routing_example::tests::example_config_does_not_embed_raw_event_blocks -- --exact`
  - 输出: `1 passed; 0 failed`。
  - 验证: example config 不嵌入 raw event tags,降低模型照抄伪事件风险。
- `cargo test --package ralph-e2e --lib scenarios::parallel_trigger_routing_example::tests::example_config_does_not_embed_placeholder_payload_templates -- --exact`
  - 输出: `1 passed; 0 failed`。
  - 验证: example config 不教学占位 payload。
- `cargo test --package ralph-e2e --lib scenarios::parallel_human_approval_gate_example::tests::example_config_does_not_embed_raw_event_blocks -- --exact`
  - 输出: `1 passed; 0 failed`。
  - 验证: human approval example 也有 prompt pollution guard。
- `cargo run -p ralph-e2e -- --list | rg 'parallel-hat-instances|parallel-trigger-routing|parallel-human-approval-gate|parallel-emit-spawn-instance'`
  - 输出包含:
    - `parallel-hat-instances`
    - `parallel-hat-instances-zh`
    - `parallel-emit-spawn-instance`
    - `parallel-trigger-routing-example`
    - `parallel-human-approval-gate-example`
  - 验证: 关键 collaboration E2E 场景已注册到 harness。

## 综合发现

- 当前仓库有真实 multi-agent collaboration runtime,不是只有文档: `ParallelSupervisor` + `HatInstanceActor` + routing module 是实现入口。
- 事件流核心是: event 被解析后进入 Supervisor,根据 TopicContract 或 triggers 计算 recipient,再投递给具体 `hat#instance`,并写 runtime delivery / agents snapshot。
- 测试层分两级:
  - core 单测用 fake executor 验证机械协议。
  - ralph-e2e 场景面向真实后端,但本轮只验证注册和静态 guard,没有跑 live Codex E2E。
- 风险:
  - 本轮没有实际跑 `cargo run -p ralph-e2e -- codex --filter ...`,所以没有 fresh live LLM 协作结果。
  - E2E 文档里也承认真实后端更慢、更贵,主要覆盖认证、网络、模型漂移。
  - 目前 checked evidence 证明协议/状态机和场景入口成立,不等同于证明每个 real-world example 在当前模型下都稳定通过。
