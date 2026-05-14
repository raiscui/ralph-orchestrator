# 任务计划: rerun-runtime-graphs V2 durable replay graph

## 目标

完成 OpenSpec change `rerun-runtime-graphs` 的 V2 durable replay graph 任务: 为离线 replay graph 补齐 delivery / lifecycle durable evidence,并用测试证明 replay 不再只靠 live observer。

## 阶段

- [x] 阶段1: 建立支线上下文并回读 OpenSpec / EXPERIENCE
- [x] 阶段2: 盘点当前 durable 证据与 runtime 投递路径
- [x] 阶段3: 设计最小 durable record 模型和重建入口
- [x] 阶段4: 实现 durable records 与 replay graph 重建
- [x] 阶段5: 补测试 / 文档 / OpenSpec tasks 勾选
- [x] 阶段6: 运行验证并记录 WORKLOG / ERRORFIX

## 关键问题

1. 当前 `.ralph/events.jsonl` / `.ralph/agents.json` 已经有哪些字段,缺哪些 V2 必需证据?
2. delivery durable evidence 应该落在现有 event history,还是新增独立 artifact?
3. replay graph CLI 入口是复用 `--runtime-graph-rrd`,还是新增从已有 run artifacts 重建的入口?

## 做出的决定

- 决定: 继续使用 `rerun_runtime_graph_v2` 支线上下文。
  - 理由: 原 `memory_axes` 支线已经归档,本轮是明确的 V2 实施任务。
- 决定: 先完成 3.1 证据盘点,再写代码。
  - 理由: OpenSpec 明确要求不能把缺证据的 replay graph 宣称成 full-fidelity。

## 遇到错误

- 暂无。

## 状态

**目前在阶段2** - 接下来阅读 parallel supervisor routing、event history、agents state、runtime graph recorder 与 record-session 输出路径。

## [2026-04-30 09:31:04] [Session ID: 019ddbe6-a5ae-7950-8ba0-27b3b9f53219] [状态更新]: 继续 V2 durable replay graph 实现

- 当前动作: 重新确认 OpenSpec artifact 状态、工作树状态和上轮支线计划, 然后继续阶段2的 durable evidence 盘点。
- 当前结论: `rerun-runtime-graphs` 的 OpenSpec artifact 已完成, 本轮进入 implementation tasks 3.1-3.4。
- 风险提示: 工作树存在大量既有改动, 本轮只修改 V2 durable replay graph 所需文件, 不处理无关改动。
- 下一步:
  - [x] 聚焦阅读 `routing.rs` 的 direct / queue / fanout / reply 投递路径。
  - [x] 聚焦阅读 lifecycle / spawn / shutdown 相关路径。
  - [x] 将 3.1 证据缺口写入 `notes__rerun_runtime_graph_v2.md`。

## [2026-04-30 09:31:04] [Session ID: 019ddbe6-a5ae-7950-8ba0-27b3b9f53219] [阶段完成]: 阶段2 durable 证据盘点完成

- [x] 阶段2: 盘点当前 durable 证据与 runtime 投递路径。
- 关键结论: 当前 V1 live observer 有 final recipient,但 durable `EventRecord` 缺 direct / queue / fanout / reply 最终 recipient; lifecycle 控制边也缺 durable record。
- 下一阶段: 进入阶段3,设计并实现 `runtime.delivery` 与 `runtime.lifecycle` 两类 observer-only durable records,再补离线 replay CLI。

## [2026-04-30 09:31:04] [Session ID: 019ddbe6-a5ae-7950-8ba0-27b3b9f53219] [行动计划]: 编辑 durable record 协议与日志入口

- 当前动作: 修改 `ralph-proto`、`event_logger`、parallel supervisor routing / lifecycle 路径和 CLI runtime graph 模块。
- 设计决策:
  - delivery durable evidence 使用 `runtime.delivery` topic,一条真实投递写一条记录。
  - lifecycle durable evidence 使用 `runtime.lifecycle` topic,实例 create/spawn/state/control 动作写一条记录。
  - replay CLI 从 `.ralph/events.jsonl` 按行顺序重建 `.rrd`,并对缺少 V2 记录的历史 run 标记 approximate。
- 验证计划:
  - 先补 event_logger 单测和 runtime_graph 单测。
  - 再跑 focused `ralph-core` / `ralph-cli` 测试。

## [2026-04-30 09:31:04] [Session ID: 019ddbe6-a5ae-7950-8ba0-27b3b9f53219] [遇到错误]: `cargo check` 发现 registry 遍历借用冲突

- 错误: `crates/ralph-core/src/parallel/supervisor.rs` 在 `self.registry.all()` 的不可变遍历中调用 `log_runtime_lifecycle_created`,造成 `E0502`。
- 原因: lifecycle durable 记录需要写 event_logger,这是 `&mut self`; 但 registry iterator 还持有 `&self`。
- 修复方向: 在遍历中只收集需要落盘的 create records,遍历结束后统一写入。

## [2026-04-30 09:31:04] [Session ID: 019ddbe6-a5ae-7950-8ba0-27b3b9f53219] [遇到错误]: targeted cargo test 参数顺序错误

- 错误: `cargo test --package ... --lib test_path --exact` 被 Cargo 拒绝,提示 `--exact` 必须放在 `--` 之后。
- 原因: 本轮第一次 targeted test 命令把 test harness 参数当成 Cargo 参数传入了。
- 修复方向: 改用 `cargo test --package <pkg> --lib <test_path> -- --exact`。

## [2026-04-30 09:31:04] [Session ID: 019ddbe6-a5ae-7950-8ba0-27b3b9f53219] [遇到错误]: 扩展 spawn 测试时移动了 `events_path`

- 错误: `routing_tests.rs` 中 `events_path` 被 move 给 `make_supervisor`,后续读取 runtime lifecycle records 时触发 `E0382`。
- 原因: 原测试没有在后续继续使用该 PathBuf,本轮新增 durable record 断言后需要保留路径。
- 修复方向: 调用 `make_supervisor` 时传入 `events_path.clone()`。

## [2026-04-30 09:31:04] [Session ID: 019ddbe6-a5ae-7950-8ba0-27b3b9f53219] [阶段完成]: V2 durable records 与 replay CLI 已实现

- [x] 阶段3: 设计最小 durable record 模型和重建入口。
- [x] 阶段4: 实现 durable records 与 replay graph 重建。
- [x] 阶段5: 补测试 / 文档 / OpenSpec tasks 勾选。
- 实现摘要:
  - 新增 `runtime.delivery` / `runtime.lifecycle` observer-only durable topics。
  - direct / queue / fanout / reply 成功投递后写 `RuntimeDeliveryRecord`。
  - create / spawn / state / freeze / cancel / shutdown 写 `RuntimeLifecycleRecord`。
  - 新增 `ralph runtime-graph replay --events <events.jsonl> --output <runtime.rrd>`。
  - 已同步 `openspec/changes/rerun-runtime-graphs/design.md` 和 `tasks.md`。
- 下一步:
  - [ ] 跑 OpenSpec 校验。
  - [ ] 跑 focused tests 与 smoke tests。
  - [ ] 记录 WORKLOG / ERRORFIX。

## [2026-04-30 10:16:53] [Session ID: 019ddbe6-a5ae-7950-8ba0-27b3b9f53219] [行动计划]: 收尾记录与最终状态检查

- 当前动作: 基于已完成的实现和验证结果, 追加 `WORKLOG__rerun_runtime_graph_v2.md` 与 `ERRORFIX__rerun_runtime_graph_v2.md`。
- 动机: 上一轮已经完成 V2 durable replay graph 的代码、测试和 OpenSpec 校验, 当前 continuation 需要把交付证据、修复过的错误和阶段6状态写回支线上下文。
- 已重新确认:
  - `openspec status --change rerun-runtime-graphs --json` 显示 artifacts complete。
  - `openspec validate rerun-runtime-graphs --strict` 通过。
  - `git status --short` 显示工作树存在大量既有改动, 本次只处理 `__rerun_runtime_graph_v2` 支线记录文件。
- 下一步:
  - [ ] 追加 WORKLOG 交付记录。
  - [ ] 追加 ERRORFIX 错误修复记录。
  - [ ] 更新阶段6完成状态并做最终 git status 检查。

## [2026-04-30 10:18:20] [Session ID: 019ddbe6-a5ae-7950-8ba0-27b3b9f53219] [行动计划]: 重新采集当前 session 的验证证据

- 当前动作: 重新运行 `cargo test`, 并保留当前 session 能直接证明的验证结果。
- 动机: continuation 摘要说明上一轮完整测试通过, 但根据本项目的 context restore 经验, 长测试是否完整结束必须以当前可见 exit status 为准。
- 下一步:
  - [ ] 运行完整 `cargo test`。
  - [ ] 如果通过, 将当前 session 的验证结果写入 WORKLOG。
  - [ ] 如果失败, 先分析失败是否与本任务相关, 不直接忽略。

## [2026-04-30 10:20:58] [Session ID: 019ddbe6-a5ae-7950-8ba0-27b3b9f53219] [阶段完成]: 阶段6验证和支线记录完成

- [x] 跑 OpenSpec 校验。
- [x] 跑 focused tests 与 smoke tests。
- [x] 当前 session 重新运行完整 `cargo test`, exit code 0。
- [x] 追加 `WORKLOG__rerun_runtime_graph_v2.md`。
- [x] 追加 `ERRORFIX__rerun_runtime_graph_v2.md`。
- [x] 阶段6: 运行验证并记录 WORKLOG / ERRORFIX。
- LATER_PLANS 检查: 当前没有本任务明确延期项, 不创建空文件。
- EPIPHANY_LOG 检查: 当前没有新的架构级灾难点或必须单独保存的风险, 不创建空文件。
- 状态: V2 durable replay graph 已实现、已验证、已记录, 接下来只做最终 `git status --short` 检查并交付总结。

## [2026-04-30 10:21:23] [Session ID: 019ddbe6-a5ae-7950-8ba0-27b3b9f53219] [行动计划]: 处理 stop hook 的 ultrawork 未收尾状态

- 当前动作: hook 提示 `ultrawork` 仍为 active planning, 需要继续任务并重新采集验证证据后再停止。
- 已确认:
  - `.omx/state/sessions/019ddbe6-a5ae-7950-8ba0-27b3b9f53219/ultrawork-state.json` 存在, 且 `active: true`, `current_phase: planning`。
  - `omx_state` MCP 读状态失败, 错误为 `Transport closed`, 因此本轮用 `.omx` 状态文件作为 runtime 状态真相源。
  - `docs/shared/agent-tiers.md` 在当前 repo 未找到; 本轮没有子代理委派, 所以不需要继续依赖该文件。
- 验收标准:
  - [x] `openspec validate rerun-runtime-graphs --strict` 重新通过。
  - [x] runtime graph focused CLI 测试重新通过。
  - [x] integration runtime graph 测试重新通过。
  - [x] `.omx` ultrawork 状态从 active planning 收尾为 inactive finished。
  - [x] 支线 WORKLOG 追加 hook 收尾验证记录。
- fresh verification evidence:
  - `openspec validate rerun-runtime-graphs --strict`: passed。
  - `cargo test --package ralph-cli --bin ralph runtime_graph::tests`: 5 passed。
  - `cargo test --package ralph-cli --test integration_runtime_graph`: 3 passed。
- runtime 收尾证据:
  - `omx status`: `ultrawork: inactive (phase: complete)`。
  - `omx state get-status --input '{"mode":"ultrawork"}' --json`: `active:false`, `phase:"complete"`。
  - `omx state list-active --json`: active modes 不包含 `ultrawork`。
- 状态: hook 收尾验证完成, 业务代码无新增改动。

## [2026-04-30 10:27:30] [Session ID: 019ddbe6-a5ae-7950-8ba0-27b3b9f53219] [行动计划]: 继续到 OpenSpec archive 前检查

- 当前动作: 用户要求继续, 本轮接上已完成的 V2 durable replay graph, 进入 `rerun-runtime-graphs` OpenSpec 归档前检查。
- 动机: implementation、focused verification、完整测试和 ultrawork 收尾都已完成; 当前合理下一步是确认 active change 能否归档。
- 已确认:
  - `openspec status --change rerun-runtime-graphs --json` 显示 artifacts complete。
- 下一步:
  - [ ] 检查 `openspec/changes/rerun-runtime-graphs/tasks.md` 是否还有未完成任务。
  - [ ] 检查 delta specs 与主 specs 的同步状态。
  - [ ] 如需要, 先同步主 spec; 然后执行 archive。
  - [ ] archive 后重新运行 OpenSpec 校验和必要 focused tests。

## [2026-04-30 21:01:15] [Session ID: 019ddbe6-a5ae-7950-8ba0-27b3b9f53219] [行动计划]: 执行 archive

- 当前动作: 将 `openspec/changes/rerun-runtime-graphs` 归档到 `openspec/changes/archive/2026-04-30-rerun-runtime-graphs`。
- 归档前证据:
  - artifacts complete。
  - tasks 全部为 `[x]`。
  - delta spec 已同步到 `openspec/specs/runtime-graph-observability/spec.md`。
  - `openspec validate rerun-runtime-graphs --strict` passed。
  - `openspec validate runtime-graph-observability --strict` passed。
- 下一步:
  - [ ] 移动 change 到 archive。
  - [ ] 归档后运行 OpenSpec 校验。
  - [ ] 运行 runtime graph focused tests。

## [2026-04-30 21:27:10] [Session ID: 019ddbe6-a5ae-7950-8ba0-27b3b9f53219] [阶段完成]: OpenSpec archive 完成

- [x] 检查 `openspec/changes/rerun-runtime-graphs/tasks.md` 是否还有未完成任务。
- [x] 检查 delta specs 与主 specs 的同步状态。
- [x] 同步主 spec 到 `openspec/specs/runtime-graph-observability/spec.md`。
- [x] 移动 change 到 `openspec/changes/archive/2026-04-30-rerun-runtime-graphs`。
- [x] 归档后运行 OpenSpec 校验。
- [x] 运行 runtime graph focused tests。
- 归档后证据:
  - `openspec list --json`: active changes 不再包含 `rerun-runtime-graphs`。
  - `openspec validate --all --strict`: 16 passed, 0 failed。
  - `openspec validate runtime-graph-observability --strict`: valid。
  - `cargo test --package ralph-cli --bin ralph runtime_graph::tests`: 5 passed。
  - `cargo test --package ralph-cli --test integration_runtime_graph`: 3 passed。
  - `omx status`: `ultrawork: inactive (phase: complete)`。
  - `omx state get-status --input '{"mode":"ultrawork"}' --json`: `active:false`, `phase:"complete"`。
- 遇到错误:
  - `openspec validate 2026-04-30-rerun-runtime-graphs --strict` 返回 `Unknown item`。
  - 处理: 查询 `openspec validate --help` 后确认归档后应使用 `openspec validate --all --strict` 或主 spec 名校验, 已补跑并通过。
- 状态: `rerun-runtime-graphs` 已完成实现、验证、主 spec 同步和 archive。

## [2026-05-01 15:48:09] [Session ID: 019de280-f42a-7171-a1e8-63aed3aef17d] [续跑]: 复核 V2 durable replay graph 代码 + 测试

- 当前动作: 用户明确要求 "V2 durable replay graph 代码 + 测试", 本轮接续既有 `rerun_runtime_graph_v2` 支线上下文, 先复核当前工作树中的实现和测试, 再重新采集当前 session 的验证证据。
- 动机: 之前记录显示 V2 durable replay graph 已实现并归档, 但本轮不能只沿用历史结论, 需要重新读代码和跑测试。
- 已确认背景:
  - `CODEX_THREAD_ID=019de280-f42a-7171-a1e8-63aed3aef17d`。
  - 工作树有大量既有改动和未跟踪文件, 本轮只处理 V2 runtime graph 相关文件与支线上下文记录。
- 待办:
  - [ ] 复核 runtime graph CLI / recorder / replay 代码。
  - [ ] 复核 durable delivery / lifecycle record 的写入路径。
  - [ ] 复核 integration test 与 focused unit test。
  - [ ] 运行 OpenSpec / focused tests / integration tests。
  - [ ] 根据真实结果补修或记录交付。
- 状态: **目前在阶段2** - 正在刷新实现上下文, 然后进入验证。

## [2026-05-01 15:48:09] [Session ID: 019de280-f42a-7171-a1e8-63aed3aef17d] [状态更新]: 开始 V2 runtime graph 验证

- 已复核代码路径:
  - `crates/ralph-proto/src/routing.rs`: 定义 `runtime.delivery` / `runtime.lifecycle` durable record 协议。
  - `crates/ralph-core/src/event_logger.rs`: 对 runtime durable payload 禁止截断。
  - `crates/ralph-core/src/parallel/supervisor.rs`: 记录 create/spawn/state/freeze/cancel/shutdown lifecycle。
  - `crates/ralph-core/src/parallel/supervisor/routing.rs`: 在 direct/queue/fanout/reply 成功投递后记录 delivery。
  - `crates/ralph-cli/src/runtime_graph.rs`: 从 durable events replay 到 `.rrd`,并区分 `full_fidelity` / approximate。
- 接下来运行:
  - [ ] `openspec validate runtime-graph-observability --strict`
  - [ ] `cargo test --package ralph-cli --bin ralph runtime_graph::tests`
  - [ ] `cargo test --package ralph-core --lib parallel::supervisor::routing_tests::direct_delivery_writes_runtime_delivery_record --exact`
  - [ ] `cargo test --package ralph-core --lib parallel::supervisor::routing_tests::fanout_delivery_writes_one_runtime_delivery_record_per_recipient --exact`
  - [ ] `cargo test --package ralph-core --lib parallel::supervisor::routing_tests::queue_delivery_writes_runtime_delivery_record --exact`
  - [ ] `cargo test --package ralph-core --lib parallel::supervisor::routing_tests::reply_delivery_writes_runtime_delivery_record --exact`
  - [ ] `cargo test --package ralph-core --lib parallel::supervisor::routing_tests::lifecycle_controls_write_freeze_cancel_shutdown_records --exact`
  - [ ] `cargo test --package ralph-cli --test integration_runtime_graph`
- 状态: **目前在阶段4** - 用当前工作树重新采集验证证据。

## [2026-05-01 15:48:09] [Session ID: 019de280-f42a-7171-a1e8-63aed3aef17d] [验证进展]: focused V2 测试通过

- 已通过:
  - [x] `openspec validate runtime-graph-observability --strict`
  - [x] `cargo test --package ralph-cli --bin ralph runtime_graph::tests`：5 passed
  - [x] `cargo test --package ralph-core --lib parallel::supervisor::routing_tests::direct_delivery_writes_runtime_delivery_record -- --exact`
  - [x] `cargo test --package ralph-core --lib parallel::supervisor::routing_tests::fanout_delivery_writes_one_runtime_delivery_record_per_recipient -- --exact`
  - [x] `cargo test --package ralph-core --lib parallel::supervisor::routing_tests::queue_delivery_writes_runtime_delivery_record -- --exact`
  - [x] `cargo test --package ralph-core --lib parallel::supervisor::routing_tests::reply_delivery_writes_runtime_delivery_record -- --exact`
  - [x] `cargo test --package ralph-core --lib parallel::supervisor::routing_tests::spawn_instance_forces_new_dynamic_instance_and_delivers_direct -- --exact`
  - [x] `cargo test --package ralph-core --lib parallel::supervisor::routing_tests::lifecycle_controls_write_freeze_cancel_shutdown_records -- --exact`
  - [x] `cargo test --package ralph-core --lib event_logger::tests::test_runtime_durable_payloads_are_not_truncated -- --exact`
  - [x] `cargo test --package ralph-cli --test integration_runtime_graph`：3 passed
- 遇到的非代码错误:
  - 我第一次把 `--exact` 放在 cargo 参数位置, cargo 报 `unexpected argument '--exact'`; 已按正确格式 `-- --exact` 重跑。
  - 我第一次写错 dynamic spawn 测试名, cargo 显示 `0 tests`; 已查真实测试名并重跑通过。
- 接下来:
  - [ ] `cargo fmt --all --check`
  - [ ] `cargo test`
- 状态: **目前在阶段4** - focused gates 已过,进入全局验证。

## [2026-05-01 15:48:09] [Session ID: 019de280-f42a-7171-a1e8-63aed3aef17d] [阶段完成]: V2 durable replay graph 当前验证完成

- [x] 复核 runtime graph CLI / recorder / replay 代码。
- [x] 复核 durable delivery / lifecycle record 的写入路径。
- [x] 复核 integration test 与 focused unit test。
- [x] 运行 OpenSpec / focused tests / integration tests。
- [x] 运行 `cargo fmt --all --check`。
- [x] 运行 `cargo test`。
- 验证证据:
  - `openspec validate runtime-graph-observability --strict`: valid。
  - `cargo test --package ralph-cli --bin ralph runtime_graph::tests`: 5 passed。
  - `cargo test --package ralph-core --lib ...direct_delivery_writes_runtime_delivery_record -- --exact`: passed。
  - `cargo test --package ralph-core --lib ...fanout_delivery_writes_one_runtime_delivery_record_per_recipient -- --exact`: passed。
  - `cargo test --package ralph-core --lib ...queue_delivery_writes_runtime_delivery_record -- --exact`: passed。
  - `cargo test --package ralph-core --lib ...reply_delivery_writes_runtime_delivery_record -- --exact`: passed。
  - `cargo test --package ralph-core --lib ...spawn_instance_forces_new_dynamic_instance_and_delivers_direct -- --exact`: passed。
  - `cargo test --package ralph-core --lib ...lifecycle_controls_write_freeze_cancel_shutdown_records -- --exact`: passed。
  - `cargo test --package ralph-core --lib event_logger::tests::test_runtime_durable_payloads_are_not_truncated -- --exact`: passed。
  - `cargo test --package ralph-cli --test integration_runtime_graph`: 3 passed。
  - `cargo fmt --all --check`: passed。
  - `cargo test`: passed。
- 当前结论:
  - V2 durable replay graph 代码和测试已在当前工作树中存在并通过验证。
  - 本轮没有新增业务代码补丁,只做复核、验证和上下文记录。
- 状态: **阶段完成** - 接下来追加 WORKLOG / ERRORFIX,再做最终状态检查。

## [2026-05-01 15:48:09] [Session ID: 019de280-f42a-7171-a1e8-63aed3aef17d] [补充验证]: 运行 replay smoke tests

- 当前动作: 追加运行 `cargo test -p ralph-core smoke_runner`。
- 动机: 仓库规则要求代码改动后必须 smoke test; V2 durable replay graph 改动触及 event log / runtime routing,需要确认 replay fixtures 不受影响。
- 待办:
  - [x] `cargo test -p ralph-core smoke_runner`
- 验证证据:
  - `cargo test -p ralph-core smoke_runner`: 12 passed。
- 状态: **阶段完成** - V2 durable replay graph 代码、focused tests、integration tests、全量 tests、smoke tests 均已验证。
