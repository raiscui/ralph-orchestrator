# WORKLOG.md

## [2026-05-20 07:48:00] [Session ID: omx-1779158263949-kticiv] 任务名称: continuous-learning parallel_rec 支线续档与经验沉淀

### 任务内容

- 用户显式触发 `$continuous-learning`。
- 回读默认六文件和 3 个支线六文件组。
- 处理 `notes__parallel_rec_analysis.md` 超过 1000 行的问题。
- 同步长期经验到 `EXPERIENCE.md`、spec、plan 和 runbook。
- 归档已总结支线与旧默认 `WORKLOG.md`。

### 完成过程

- 按上下文集分组读取默认组、`display_info_evidence`、`multi_agent_collab_evidence`、`parallel_rec_analysis`。
- 将支线中仍然有效的后续事项提升到根 `LATER_PLANS.md`。
- 在 `EXPERIENCE.md` 增补:
  - `exp-20260520-topology-spawn-result-ack-guardrail`
  - `exp-20260520-multi-agent-collaboration-evidence-layers`
- 更新 `specs/parent-visible-topology-spawn-observability.spec.md`,明确 `topology.spawn.result` 不能重新投递原始任务。
- 更新 `docs/plans/2026-05-19-parent-visible-topology-spawn-and-child-run-observability.md`,补充 dogfood 后 guardrail 和 record-session timeline 验证脚本。
- 更新 `docs/runbook/runtime-capabilities.md`,明确 capability lane 与 topology lane 的边界。
- 将支线上下文移入 `archive/branch_contexts/`。
- 将旧默认 `WORKLOG.md` 移入 `archive/default_history/WORKLOG_2026-05-20_0748_pre_continuous_learning.md`。

### 验证证据

- `beautiful-mermaid-rs --ascii` 成功渲染 `specs/parent-visible-topology-spawn-observability.spec.md` 的 2 个 Mermaid code block。
- 归档 manifest: `archive/manifests/ARCHIVE_MANIFEST__continuous_learning_parallel_rec_2026-05-20_0748.md`。

### 总结感悟

- `topology.spawn.result` 这种成功 ack 也必须有明确后续语义,否则 coordinator 会把它误当成普通 event 再派发。
- 多智能体协作证据必须分层: protocol tests、scenario registration 和 live backend E2E 不能互相冒充。
- 支线归档前要先把仍然有效的后续事项提升到根 `LATER_PLANS.md`,否则 archive 会把未来路线埋掉。

## [2026-05-20 17:14:00] [Session ID: omx-1779158263949-kticiv] 任务名称: dogfood worker MaxRuntime 调试与 parallel default_publishes 修复

### 任务内容
- 继续处理 parent-visible topology spawn dogfood 中 worker `MaxRuntime` 问题。
- 区分 worker 无结果、parser 未识别、runtime 未路由、coordinator completion 预算不足等候选原因。
- 补齐 parallel path 的 `default_publishes` 等价语义和 focused regression。

### 完成过程
- 解析 `/tmp/ralph-topology-dogfood-guardrail-record.jsonl`,确认原始 run 没有 `analysis.done` bus.publish。
- 运行 bounded worker 实验,确认 stdout event 能出现,并排除“parser 完全无法识别 worker event”的解释。
- 发现 serial path 有 `check_default_publishes`,parallel `JobCompleted` 路径缺少等价 fallback。
- 在 `crates/ralph-core/src/parallel/supervisor.rs` 增加成功无 event 时的 `default_publishes` fallback 注入。
- 在 `crates/ralph-core/src/parallel/supervisor/routing_tests.rs` 增加 focused regression。
- 运行 90 秒和 180 秒 live dogfood,确认 180 秒预算下三实例自然收敛到 `CompletionPromise`。

### 验证证据
- `/tmp/ralph-topology-dogfood-bounded-180-record.jsonl`: 3 条 `analysis.done`,最终 `_meta.termination.reason=CompletionPromise`。
- `cargo test -p ralph-core parallel::supervisor::routing_tests::parallel_default_publishes_injects_when_worker_finishes_without_event -- --exact --nocapture`: passed。
- `cargo test -p ralph-core smoke_runner`: passed。
- `cargo fmt --all -- --check`: passed。
- `git diff --check`: passed。
- `cargo test`: passed。

### 总结感悟
- 这次不是单一根因: 原始 dogfood 同时暴露了 worker 任务过开放、runtime 预算不足,以及 parallel path 缺少 serial fallback 语义。
- record-session 中必须区分 stdout terminal write 与 `bus.publish`; stdout 看到 event 不等于 runtime 已经路由。
- 对 live Codex 三 worker repo-grounded 分析,稳定 dogfood 应同时具备 bounded worker contract、足够 `max_runtime_seconds`,以及 record-session 作为主证据。

## [2026-05-20 19:01:00] [Session ID: omx-1779158263949-kticiv] 任务名称: 3-worker live dogfood 复跑与结果整理

### 任务内容
- 继续等待上一轮已经启动的 3-worker live dogfood,没有重新启动第二个并发 run。
- 解析 `/tmp/ralph-topology-dogfood-bounded-180-rerun-20260520-185717.jsonl`。
- 整理 `analyst#2/#3/#4` 三个 worker 的 `analysis.done` 输出,并评估这些结果是否有用。

### 完成过程
- 通过后台 session 确认复跑命令退出码为 0。
- 用 `./target/debug/ralph record summary` 复核 record-session。
- 用 Python 解析 `bus.publish` 顺序、`analysis.done` payload 和 `_meta.termination`。
- 读取 `.ralph/agents.json`,确认 parent-visible dynamic instances 和 fixed-role metadata。

### 关键结果
- `Termination reason=CompletionPromise`, `elapsed_secs=85.605838208`, `iterations=4`。
- `analysis.done=3/3`。
- `analysis.task=3/3`,分别投递给 `analyst#2/#3/#4`。
- `.ralph/agents.json` 包含 `analyst#2/#3/#4` dynamic instances,其中 `analyst#4` 因 `fixed_role=true` 写入 `fixed_role_label=review`。
- `topology.spawn.result` 后没有重复 `analysis.task`,说明 acknowledgement guardrail 在本轮证据中成立。

### 总结感悟
- 这轮 dogfood 的 worker 输出有实际价值,尤其是 evidence inspect、运行图 / 生命周期查询、TUI/plain 显示契约和 acknowledgement guardrail。
- stream-json adapter 方向有价值,但应拆成独立后续任务,不要混进 parent-visible spawn 收尾。

## [2026-05-21 07:25:47] [Session ID: omx-1779158263949-kticiv] 任务名称: unified evidence inspect 实装

### 任务内容
- 按用户确认,优先实装“一个能证明 topology / child-run / agents / result / termination 的统一 evidence inspect”。
- 优先改良既有 `ralph record summary`,而不是新增分散命令。

### 完成过程
- 新增 `specs/unified-evidence-inspect.spec.md`,并用 `beautiful-mermaid-rs --ascii` 验证 Mermaid 语法。
- 在 `crates/ralph-cli/src/record_session.rs` 增加 `EvidenceInspectAggregate` 和 `render_evidence_inspect`。
- 在 `crates/ralph-cli/src/record_cli.rs` 接入 `Evidence Inspect` section,并新增 `--agents-file FILE`。
- 修复 `record_watch_auto_locates_latest_pointer_and_streams_lines` 的强杀竞态,改为 `--until-event` 自然退出。
- 用 3-worker live dogfood 的真实 record-session 验证输出。

### 验证证据
- `cargo fmt --all -- --check`: passed。
- `git diff --check`: passed。
- `cargo test -p ralph-cli --test integration_record_session -- --nocapture`: passed。
- `cargo test -p ralph-cli record_session::tests::aggregate_collects_evidence_inspect -- --exact --nocapture`: passed。
- `cargo test -p ralph-core smoke_runner`: passed。
- `cargo test`: passed。
- 真实验证命令: `./target/debug/ralph record summary /tmp/ralph-topology-dogfood-bounded-180-rerun-20260520-185717.jsonl --agents-file .ralph/agents.json`。

### 总结感悟
- 当前最大问题不是再加调度智能,而是把已经存在的运行态真相收束成可信 evidence surface。
- record-session 应继续作为主证据,agents snapshot 作为 sidecar,这样可以明确区分 durable timeline 和当前运行态快照。

## [2026-05-21 07:38:00] [Session ID: omx-1779158263949-kticiv] 任务名称: TUI/plain 显示验收

### 任务内容
- 给 parallel no-tui/plain 模式补 topology/capability 控制面事件摘要。
- 复用已有 TUI footer / instances / output status strip 测试验证显示层没有回归。
- 更新 `specs/unified-evidence-inspect.spec.md`,把 plain runtime control-plane evidence 和 TUI display guardrails 写入同一证据规格。

### 完成过程
- 在 `crates/ralph-cli/src/parallel_runner.rs` 增加 `maybe_write_parallel_cli_event_summary`。
- no-tui event observer 改为始终存在: 有 recorder 时写 record-session,非 quiet 时输出 `[supervisor:event] ...` 摘要。
- 新增 guardrail tests 覆盖 `topology.spawn.result`、`capability.result`、`capability.failed`、非相关 topic 忽略、quiet 不输出。
- 用完整测试路径重跑 TUI focused tests,避免 0 tests 的假阳性。

### 验证
- `cargo test -p ralph-cli parallel_runner::guardrail_tests -- --nocapture`: passed,9 tests。
- `cargo test -p ralph-tui widgets::footer::tests::footer_shows_parallel_child_run_summary -- --exact --nocapture`: passed。
- `cargo test -p ralph-tui widgets::parallel_output::tests::output_status_pane_shows_latest_child_run_artifact -- --exact --nocapture`: passed。
- `cargo test -p ralph-tui widgets::instances::tests::instances_pane_shows_topology_spawn_role_label -- --exact --nocapture`: passed。
- `cargo test -p ralph-tui widgets::parallel_output::tests::split_parallel_output_areas_reserves_status_rows_outside_content -- --exact --nocapture`: passed。
- `cargo test -p ralph-tui app::tests::split_parallel_output_areas_reserves_bottom_status_rows -- --exact --nocapture`: passed。
- `cargo fmt --all -- --check`: passed。
- `git diff --check`: passed。
- `cargo test -p ralph-core smoke_runner`: passed。
- `cargo test`: passed。

### 总结感悟
- 运行中显示和离线 evidence inspect 应互补: plain/TUI 负责即时可观测,record-session 负责耐久审计。
- focused test 如果显示 0 tests,必须立刻纠正过滤路径,否则会制造验收假阳性。

## [2026-05-21 07:43:00] [Session ID: omx-1779158263949-kticiv] 任务名称: TUI/plain 显示验收 warning 收尾

### 任务内容
- 处理最终验证中出现的 `dead_code` warning。
- 将测试专用 helper 限定到 `#[cfg(test)]`。

### 完成过程
- 发现 warning 后没有继续宣称完成。
- 修正后重新跑 focused tests、TUI tests、smoke_runner 和 deny-warnings 全量测试。

### 验证
- `RUSTFLAGS="-Dwarnings" cargo test --quiet`: passed。
- 说明最终状态没有 warning。

### 总结感悟
- Rust 测试通过但有 warning 时,仍不能作为交付证据。
- 对只服务测试的辅助函数,应显式使用 `#[cfg(test)]` 避免污染 release/bin 编译面。

## [2026-05-21 08:10:00] [Session ID: omx-1779158263949-kticiv] 任务名称: parent-visible spawn replay/integration guardrail

### 任务内容
- 补一个真实 CLI integration guardrail,锁住 `topology.spawn_group` 的 parent-visible 动态实例物化链路。
- 覆盖 `.ralph/events.jsonl`、`.ralph/agents.json`、stdout、record-session 和 `record summary --agents-file`。

### 完成过程
- 新增 `crates/ralph-cli/tests/integration_topology_spawn.rs`。
- 测试用 custom backend 脚本模拟 `ralph#1` 发 `topology.spawn_group`,以及三个 dynamic builder worker 产出 `analysis.done`。
- 断言 `topology.spawn.result` 是 acknowledgement,不会在结果之后再次 redeliver 原始 `build.task`。
- 更新 `specs/parent-visible-topology-spawn-observability.spec.md` 的验证建议,把 CLI integration guardrail 写入长期规格。

### 验证
- `cargo test -p ralph-cli --test integration_topology_spawn -- --nocapture`: passed。
- `cargo test -p ralph-core parallel::supervisor::routing_tests::topology_spawn_group_creates_three_dynamic_instances_and_delivers_direct -- --exact --nocapture`: passed。
- `cargo test -p ralph-core parallel::supervisor::routing_tests::topology_spawn_group_is_idempotent_by_request_id -- --exact --nocapture`: passed。
- `cargo test -p ralph-core smoke_runner`: passed。
- `RUSTFLAGS="-Dwarnings" cargo test --quiet`: passed。
- `cargo fmt --all -- --check`: passed。
- `git diff --check`: passed。

### 总结感悟
- replay/integration guardrail 不能只看 core helper。必须跨到真实 binary 和落盘 evidence,否则容易漏掉 config、record-session、agents sidecar 或 stdout display 的边界问题。
- 并发 worker 的完成顺序不稳定,测试应对结果集合排序后比较,但 direct delivery 的 runtime 顺序仍可以作为 deterministic evidence 断言。

## [2026-05-21 19:03:10] [Session ID: omx-1779158263949-kticiv] 任务名称: ralplan 生成 task-derived dynamic hat identity / role contract 方案

### 任务内容
- 按 `` 共识规划流程生成 task-derived dynamic hat identity / role contract 的最终方案。
- 本轮只生成方案,没有实现代码。
- 方案覆盖 topology.spawn_group、EffectiveRoleContract、prompt isolation、agents snapshot、record summary、TUI/plain display、测试和 dogfood 验证。

### 完成过程
- 创建 repo-grounded context snapshot: .omx/context/task-derived-dynamic-hat-identity-role-contract-20260521T000000Z.md。
- 生成 Planner draft。
- 依次完成 Architect -> Critic 评审。
- 根据 ITERATE 反馈修订到 rev3。
- 最终 Architect rev3 与 Critic rev3 均 APPROVE。
- 保存最终方案到 .omx/plans/task-derived-dynamic-hat-identity-role-contract.md。

### 总结感悟
- 最重要的架构收敛是: raw spawn payload 不能直接成为权限真相源。
- runtime canonical EffectiveRoleContract 应作为 downstream 唯一 contract。
- objective 冲突策略必须可测试: canonical objective 永远取 member.task,raw objective 只进入 warning/evidence。

## [2026-05-21 21:02:00] [Session ID: omx-1779158263949-kticiv] 任务名称: task-derived dynamic hat identity / role contract 落地

### 任务内容
- 按 `.omx/plans/task-derived-dynamic-hat-identity-role-contract.md` 落地 task-derived dynamic hat identity / role contract。
- 覆盖 `topology.spawn_group` raw role_contract hint、runtime canonical `EffectiveRoleContract`、worker prompt、agents snapshot、record summary、plain/TUI display、focused/integration tests。
- 根据 live dogfood 暴露的问题,补强 coordinator event protocol 中 `role_contract` sibling field 的 schema guidance。

### 完成过程
- 新增/接入 `EffectiveRoleContract`、`RoleContractSummary`、`RolePersistence` 与 stable `role_contract_hash`。
- 扩展 `TopologySpawnMember.role_contract` 作为 raw hint,并在 runtime 侧 canonicalize。
- 在 supervisor 中保存 per-instance effective role contract,并只将 summary 写入 agents snapshot / display / record summary。
- dynamic worker prompt 注入 canonical `### ROLE CONTRACT`,并通过测试确认不继承 coordinator-only prompt surface。
- no-tui plain summary、`ralph agents`、TUI Instances 和 `record summary --agents-file` 都能显示 role contract summary。
- live dogfood 证明 parent-visible spawn 成功,同时发现首轮 LLM 把 `role_contract` 错放到 `input` object。已据此补强 `event_emission_protocol.rs` 和回归断言。
- 修复一个额外暴露的测试环境依赖: `event_loop_ralph` object payload 测试不再读取工作区 `.ralph/events.jsonl`,改用临时 fixture。

### 验证证据
- `cargo test -p ralph-core event_emission_protocol::tests::topology_spawn_prompt_documents_parent_visible_group_spawn_contract -- --exact --nocapture`: passed。
- `cargo test -p ralph-core parallel::supervisor::routing_tests::runtime_capability_catalog_is_injected_only_into_ralph_prompt -- --exact --nocapture`: passed。
- `cargo test -p ralph-core parallel::supervisor::routing_tests::topology_spawn_group -- --nocapture`: 9 passed。
- `cargo test -p ralph-core parallel::supervisor::routing_tests::dynamic_worker_prompt_contains_effective_role_contract -- --exact --nocapture`: passed。
- `cargo test -p ralph-cli --test integration_topology_spawn -- --nocapture`: passed。
- `cargo test -p ralph-core --test event_loop_ralph test_reads_actual_events_jsonl_with_object_payloads -- --exact --nocapture`: passed。
- `cargo fmt --all -- --check`: passed。
- `git diff --check`: passed。
- `cargo test -p ralph-core smoke_runner`: 12 passed。
- `RUSTFLAGS="-Dwarnings" cargo test --quiet`: passed。

### Dogfood 证据
- record-session: `/tmp/ralph-task-derived-role-contract-dogfood-20260521-202623.jsonl`。
- `topology.spawn.result` 成功创建 `builder#2/#3/#4`,均显示 `identity_source=task-derived`, `persistence=fixed`, `contract_schema_version=1`, `role_contract_hash`, `source_spawn_request_id`。
- 本次 live dogfood 的终止状态不是自然完成: `RUN_STATUS=124`, `Termination.reason=Interrupted`,elapsed 约 419.970 秒。
- 因此只能作为 parent-visible spawn / role contract evidence,不能作为 3-worker live collaboration 自然收敛证据。

### 总结感悟
- runtime schema 正确不等于 LLM 会按正确 payload 发事件。对于 coordinator-only 控制面协议,必须给 schema-literate 示例,并用 prompt regression test 锁住。
- `EffectiveRoleContract` 作为唯一真相源是对的: raw hint 被限制在输入层,下游只看 canonical contract 或 summary。
- live dogfood 暴露出 worker artifact 写入和超时稳定性问题,需要作为后续独立任务处理,不能混进本次 contract 主线。

## [2026-05-22 12:09:52] [Session ID: omx-1779158263949-kticiv] 任务名称: clean live dogfood 验证 3-worker task-derived role contract 自然收敛

### 任务内容
- 按用户要求给 live dogfood 制作专门 clean config。
- 关闭 confessor / confession_handler,避免 confession phase 拉长收敛链路。
- 让目标 `builder` hat 明确 publishes `analysis.done`。
- 保持 `ralph#1` coordinator 使用 `-c features.hooks=false`,worker 不加该 role_args,继续正常带 hooks。
- 强约束 worker final event 只能通过最终 assistant stdout 输出 `analysis.done`。

### 完成过程
- 生成临时配置 `/tmp/ralph-clean-task-derived-dogfood-20260522.yml`。
- 生成临时 prompt `/tmp/ralph-clean-task-derived-dogfood-20260522.prompt.md`。
- 首次运行发现 `complete_publishes` 配置校验失败,已移除临时 config 中的该字段。
- 重新运行真实 live dogfood,并保存 record-session / stdout / stderr / summary。
- 进一步核验 `.ralph/agents.json` 少 `builder#4` 的原因,确认 record-session durable stream 已证明 `builder#4` 完整运行并发布结果。

### 验证证据
- run command:
  - `./target/debug/ralph run -c /tmp/ralph-clean-task-derived-dogfood-20260522.yml --no-tui --record-session /tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-114604.jsonl -p ...`
- record-session: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-114604.jsonl`
- summary: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-114604.summary.txt`
- stdout: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-114604.stdout.txt`
- stderr: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-114604.stderr.txt`
- 关键结果:
  - `RUN_STATUS=0`
  - `Termination.reason=CompletionPromise`
  - `elapsed_secs=49.620`
  - `topology.spawn_group: 1`
  - `topology.spawn.result: 1`
  - `parent_topology_unchanged=false`
  - `topology.spawn.failed: 0`
  - `analysis.done: 3 source_instances=builder#2,builder#3,builder#4`
  - `capability.request/result/failed: 0`
  - 最终 `reply.human.message` 确认 3/3 全部收到。

### 总结感悟
- clean dogfood 的关键不是修改长期 `ralph.yml`,而是给 live 验证单独配置最小拓扑,让目标 topic、收敛信号和干扰 phase 明确。
- `record-session` 是历史真相源,`.ralph/agents.json` 是当前 registry 观察面。动态实例被 TTL 回收后,sidecar 可能不再显示它,不能据此否定它曾经跑过。
- 后续如果要让用户更容易确认 parent-visible dynamic instance 全生命周期,应补 tombstone/final historical agents view 或在 summary 里显式标注 Agents Snapshot 的语义边界。

## [2026-05-22 14:25:19] [Session ID: omx-1779158263949-kticiv] 任务名称: completed dynamic instances evidence 明确表达

### 任务内容
- 让 Evidence Inspect / agents snapshot 明确表达 completed dynamic instances。
- 防止动态实例完成后被 TTL/unregister 回收,导致用户在最终 `.ralph/agents.json` 或 `ralph agents` 中误判“实例没跑起来”。

### 完成过程
- 在 agents snapshot schema 中新增 `completed_dynamic_instances` tombstone 区。
- 在 supervisor unregister dynamic instance 前保存 summary-only completed snapshot。
- 在 Evidence Inspect 中新增 `Completed Dynamic Instances` section,并标注 current registry 语义。
- 在 `ralph agents` 中新增独立 `Completed dynamic instances` 表,不混入 active/current instance 表。
- 更新相关 specs,明确 completed tombstone 的 observability contract。
- 补充 supervisor focused test、record summary unit test、CLI integration test。
- 修复全量 gate 中暴露的过期 integration test: default workflow 当前执行 hat 是 `worker`,不是旧的 `builder`。

### 验证证据
- `cargo test -p ralph-core parallel::supervisor::routing_tests::completed_dynamic_instance_remains_visible_as_agents_tombstone -- --exact --nocapture`: passed。
- `cargo test -p ralph-cli record_session::tests::evidence_inspect_renders_completed_dynamic_instances_from_agents_snapshot -- --exact --nocapture`: passed。
- `cargo test -p ralph-cli --test integration_agents test_agents_command_prints_completed_dynamic_instances_separately -- --exact --nocapture`: passed。
- `cargo test -p ralph-cli --test integration_agents -- --nocapture`: 8 passed。
- `cargo test -p ralph-cli --test integration_topology_spawn -- --nocapture`: 1 passed。
- `cargo test -p ralph-core topology_spawn_group -- --nocapture`: 15 passed。
- `cargo test -p ralph-cli record_session::tests::aggregate_collects_evidence_inspect -- --exact --nocapture`: passed。
- `cargo test -p ralph-cli --test integration_capability -- --nocapture`: 8 passed。
- `cargo fmt --all -- --check`: passed。
- `git diff --check`: passed。
- `cargo test -p ralph-core smoke_runner`: 12 passed。
- `RUSTFLAGS="-Dwarnings" cargo test --quiet`: passed。

### 总结感悟
- current registry 与 historical completed tombstone 必须分开表达,否则调度语义和观察语义会互相污染。
- 对 parent-visible dynamic spawn,最终解释链应同时看 record-session durable stream 和 agents sidecar,不能只看 registry 当前快照。

## [2026-05-22 15:29:08] [Session ID: omx-1779158263949-kticiv] 任务名称: 重新运行 clean 3-worker live dogfood 并验证 completed dynamic instances 展示

### 任务内容
- 按用户要求重新运行 clean 3-worker live dogfood。
- 使用真实  和  检查最终展示效果。

### 完成过程
- 复用 clean config: `/tmp/ralph-clean-task-derived-dogfood-20260522.yml`。
- 复用 clean prompt: `/tmp/ralph-clean-task-derived-dogfood-20260522.prompt.md`。
- 先执行 `cargo build -p ralph-cli --bin ralph --quiet`,确保 `./target/debug/ralph` 是当前代码。
- 运行 live dogfood,保存 record-session/stdout/stderr/summary。
- 执行 `./target/debug/ralph agents`,保存真实 agents CLI 展示。

### 验证证据
- record-session: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-152219.jsonl`。
- summary: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-152219.summary.txt`。
- agents display: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-152219.agents.txt`。
- RUN_STATUS=0。
- Termination.reason=CompletionPromise。
- topology.spawn_group=1。
- topology.spawn.result=1。
- topology.spawn.failed=0。
- analysis.done=3, source_instances=builder#2,builder#3,builder#4。
-  current registry instances=2, completed_dynamic_instances=3。
- Instance        | Hat     | State    | Dynamic | Source            | Fixed Role       | Role Contract        | Last Input
---------------|---------|----------|---------|-------------------|------------------|----------------------|----------------------------------------
builder#1      | builder | idle     | no      | config-derived    | -                | -                    | -
ralph#1        | ralph   | idle     | no      | config-derived    | -                | -                    | analysis.done: {"role":"review","suggestions":["把当前演... 输出了独立的  表。

### 总结感悟
- 这次真实 live dogfood 证明 tombstone 方案解决了“动态实例完成后从 current registry 消失,用户看不见”的问题。
- 后续如果要让最终 human summary 也进入 Result Topics,需要收紧  为单行 XML event 或增强 multi-line event parsing。

## [2026-05-22 15:30:56] [Session ID: omx-1779158263949-kticiv] 更正记录: clean 3-worker live dogfood completed_dynamic_instances 展示验证

### 更正说明
- 上一条同主题 WORKLOG 写入时误用了未 quoted heredoc,正文中部分反引号内容被 shell 执行。
- 本条为完整更正版,后续引用本次 live dogfood 证据以本条为准。

### 任务内容
- 重新运行 clean 3-worker live dogfood。
- 使用真实  和  检查 completed dynamic instances 展示。

### 验证证据
- record-session: 。
- summary: 。
- agents display: 。
- RUN_STATUS=0。
- Termination.reason=CompletionPromise。
- topology.spawn_group=1。
- topology.spawn.result=1。
- topology.spawn.failed=0。
- analysis.done=3, source_instances=builder#2,builder#3,builder#4。
-  current registry instances=2, completed_dynamic_instances=3。
- Instance        | Hat     | State    | Dynamic | Source            | Fixed Role       | Role Contract        | Last Input
---------------|---------|----------|---------|-------------------|------------------|----------------------|----------------------------------------
builder#1      | builder | idle     | no      | config-derived    | -                | -                    | -
ralph#1        | ralph   | idle     | no      | config-derived    | -                | -                    | analysis.done: {"role":"review","suggestions":["把当前演... 输出独立  表。

### 总结感悟
- tombstone 方案在真实 live dogfood 中可用,能明确表达已完成并从 current registry 回收的 dynamic instances。
-  多行 event 未进入 Result Topics 是后续可单独收紧的观察点。


## [2026-05-22 15:32:09] [Session ID: omx-1779158263949-kticiv] 最终更正记录: clean 3-worker live dogfood completed_dynamic_instances 展示验证

### 更正说明
- 前两条同主题 WORKLOG / notes 写入时误用了未 quoted heredoc,正文中部分反引号内容被 shell 执行。
- 本条由 Python 直接追加,是完整可信版本。

### 任务内容
- 重新运行 clean 3-worker live dogfood。
- 使用真实 `.ralph/agents.json` 和 `ralph record summary --agents-file .ralph/agents.json` 检查 completed dynamic instances 展示。

### 验证证据
- record-session: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-152219.jsonl`。
- summary: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-152219.summary.txt`。
- agents display: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-152219.agents.txt`。
- RUN_STATUS=0。
- Termination.reason=CompletionPromise。
- topology.spawn_group=1。
- topology.spawn.result=1。
- topology.spawn.failed=0。
- analysis.done=3, source_instances=builder#2,builder#3,builder#4。
- `.ralph/agents.json` current registry instances=2, completed_dynamic_instances=3。
- `ralph agents` 输出独立 `Completed dynamic instances: 3` 表。

### 总结感悟
- tombstone 方案在真实 live dogfood 中可用,能明确表达已完成并从 current registry 回收的 dynamic instances。
- `reply.human.message` 多行 event 未进入 Result Topics 是后续可单独收紧的观察点。


## [2026-05-22 19:23:13] [Session ID: omx-1779158263949-kticiv] 任务名称: runtime multi-line reply event parsing / completion observer 修复

### 任务内容
- 修复 multi-line `reply.human.message` 与 `LOOP_COMPLETE` 同一批 stdout 输出时,事件只出现在 stdout 而没有进入 bus.publish / Result Topics 的问题。
- 不改成依赖 prompt 单行输出,而是在 runtime 层保证已解析的最终回复事件能被 observer 记录。

### 完成过程
- 先补 `EventParser` 多行 reply payload 测试,验证 parser 能解析跨行 payload,且 completion promise 在 event 外仍能识别。
- 再补 supervisor 回归测试,构造同批输出: multi-line `reply.human.message` 后跟 `LOOP_COMPLETE`。
- 测试先失败,observer 只收到 `task.start`,证明问题不是 parser,而是 completion 同批 route 被跳过。
- 在 supervisor 的 completion stop_spawning 分支加入 observer-only drain。
- drain 只放行 `reply.human.message` 与 hat-sourced `human.message` 这类 route_event 已定义为不投递给 hat 的输出型事件。
- 保持原有 completion lockdown: `build.task` / `build.done` 等 workflow topic 在 completion 后仍不会派生新 job。

### 验证证据
- `cargo test -p ralph-core event_parser::tests::test_parse_reply_human_message_with_multiline_payload -- --exact --nocapture`: passed。
- `cargo test -p ralph-core parallel::supervisor::routing_tests::supervisor_observes_multiline_reply_human_message_in_completion_batch -- --exact --nocapture`: 先失败后通过。
- `cargo test -p ralph-core parallel::supervisor::routing_tests::supervisor_does_not_route_new_events_after_completion_promise -- --exact --nocapture`: passed。
- `cargo test -p ralph-core parallel::supervisor::routing_tests::supervisor_freezes_prequeued_ralph_job_after_completion_promise -- --exact --nocapture`: passed。
- `cargo fmt --all -- --check`: passed。
- `git diff --check`: passed。
- `cargo test -p ralph-core smoke_runner`: 12 passed。
- `RUSTFLAGS="-Dwarnings" cargo test --quiet`: passed。

### 总结感悟
- 这次根因不是 multi-line parser 缺失,而是 completion 同批路由策略把 observer 也一起跳过了。
- completion 后禁止派生 job 和记录最终人类回复不是冲突关系,需要把“routing”与“observer-only evidence”分开。


## [2026-05-22 20:26:54] [Session ID: omx-1779158263949-kticiv] 任务名称: 重跑 clean 3-worker live dogfood 验证 multi-line reply durable 结果

### 任务内容
- 按用户要求重跑 clean 3-worker live dogfood。
- 使用当前 `./target/debug/ralph` 和现有 clean config/prompt。
- 验证 runtime multi-line `reply.human.message` 是否进入 durable `bus.publish` / `record summary Result Topics`。

### 完成过程
- 重新执行 `cargo build -p ralph-cli --bin ralph --quiet`。
- 复用 `/tmp/ralph-clean-task-derived-dogfood-20260522.yml`。
- 复用 `/tmp/ralph-clean-task-derived-dogfood-20260522.prompt.md`。
- 运行 `./target/debug/ralph run -c ... --no-tui --record-session ... -p <prompt>`。
- 生成 `ralph record summary <record> --agents-file .ralph/agents.json`。
- 生成 `ralph agents` 展示。
- 用 Python 直接解析 record-session 原始 JSONL 的 `bus.publish` topics。

### 验证证据
- record-session: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-201931.jsonl`。
- summary: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-201931.summary.txt`。
- agents display: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-201931.agents.txt`。
- stdout: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-201931.stdout.txt`。
- stderr: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-201931.stderr.txt`。
- status: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-201931.status.txt`。
- RUN_STATUS=0。
- `Termination.reason=CompletionPromise`。
- `analysis.done: 3 source_instances=builder#2,builder#3,builder#4`。
- `reply.human.message: 1 source_instances=ralph#1`。
- `topology.spawn_group: 1`, `topology.spawn.result: 1`, `topology.spawn.failed: 0`。
- `completed_dynamic_instances: 3`。
- 原始 record-session `bus.publish` 统计: `reply.human.message: 1 sources=['ralph#1']`。

### 总结感悟
- 真实 live dogfood 已证明: multi-line final human reply 不再只是 stdout 可见,而是进入了 durable Result Topics。
- 这说明 completion 同批 observer-only drain 的修复方向正确: 不恢复 completion 后的 job 派生,只恢复最终回复事件的证据记录。

## [2026-05-23 16:45:00] [Session ID: omx-1779158263949-kticiv] 任务名称: 用户自然语言 prompt dynamic hats live dogfood

### 任务内容
- 使用用户给定自然语言 prompt 做 live dogfood。
- 验证 Ralph 是否能自己决定拆分角度数量,并创建多个 parent-visible dynamic hats/instances 并行分析。
- 收集 record-session、summary、agents snapshot、stdout/stderr 和抽取后的结果文件。

### 完成过程
- 使用 clean dogfood config: `/tmp/ralph-clean-task-derived-dogfood-20260522.yml`。
- 使用 inline `-p` 覆盖 config 中的 prompt_file,没有修改仓库内 prompt 文件。
- 先运行 `cargo build -p ralph-cli --bin ralph --quiet`,确认当前 debug binary 可构建。
- 运行 `./target/debug/ralph run -c /tmp/ralph-clean-task-derived-dogfood-20260522.yml --no-tui --color never --record-session /tmp/ralph-dynamic-evolution-angle-dogfood-20260523-151612.jsonl -p <用户 prompt>`。
- 运行 `ralph record summary ... --agents-file .ralph/agents.json` 和 `ralph agents`。
- 用 Python 直接解析 record-session,确认 topic counts、spawn payload、spawn result、analysis.done source_instances 与最终 reply。

### 验证证据
- record-session: `/tmp/ralph-dynamic-evolution-angle-dogfood-20260523-151612.jsonl`。
- summary: `/tmp/ralph-dynamic-evolution-angle-dogfood-20260523-151612.summary.txt`。
- agents display: `/tmp/ralph-dynamic-evolution-angle-dogfood-20260523-151612.agents.txt`。
- extracted result: `/tmp/ralph-dynamic-evolution-angle-dogfood-20260523-151612.results.md`。
- record parse errors: 0。
- termination: `CompletionPromise`, iterations=8, elapsed_secs≈539.8。
- topic counts: `topology.spawn_group=1`, `topology.spawn.result=1`, `topology.spawn.failed=0`, `analysis.done=6`, `reply.human.message=6`。
- dynamic roles: `protocol_architect(builder#2)`, `evidence_auditor(builder#3)`, `ux_reviewer(builder#4)`, `governance_reviewer(builder#5)`, `e2e_gatekeeper(builder#6)`。
- final summary recommended mainline: `clean-current-runtime-evidence-and-dynamic-role-contract`。

### 总结感悟
- 自然语言 prompt 没有直接要求固定三个角色时,coordinator 会先用静态 `builder#1` 做一次角度拆分建议,再基于结果发 `topology.spawn_group` 创建 5 个 parent-visible dynamic instances。
- 这个行为符合“让智能体自己决定需要几个角度”的产品期望,但耗时较长,本轮约 540 秒。
- 结果质量有用: 五个角色覆盖 runtime protocol、evidence、UX、governance、E2E/release gate,最终建议优先收敛 runtime protocol + evidence + release gate,而不是继续堆新功能。
- 本轮外层 wrapper 使用 zsh 只读变量 `status` 导致状态文件失败,后续脚本应改用 `run_status` 之类变量名。

## [2026-05-23 17:24:00] [Session ID: omx-1779158263949-kticiv] 任务名称: 创建 clean-current-runtime-evidence-and-dynamic-role-contract OpenSpec

### 任务内容
- 按用户要求,基于 live dogfood 推荐主线创建新的 OpenSpec change。
- 本轮只创建规格资产,不实现代码。
- 范围覆盖 runtime protocol SSOT、dynamic role contract evidence、topology.spawn_group partial/tombstone、record-session/evidence inspect correlation、parallel runtime release gate。

### 完成过程
- 使用 `openspec new change clean-current-runtime-evidence-and-dynamic-role-contract` 创建 spec-driven change。
- 创建 proposal,明确新增 capability `current-runtime-evidence-contract` 与修改能力 `parallel-hat-instances`、`record-session-contract-and-watch`、`runtime-evidence-index-kernel`、`prompt-contract-runtime-alignment`。
- 创建 5 个 spec 文件,所有 Requirement 首句使用 MUST/SHALL 口径。
- 创建 design.md,明确 record-session 是 semantic completion truth、evidence index 只做 correlation、canonical role contract 是 downstream authority、partial spawn 是一等证据、release-fast gate 是 focused gate。
- 创建 tasks.md,拆分为 6 组 implementation tasks。

### 验证证据
- `openspec status --change clean-current-runtime-evidence-and-dynamic-role-contract`: 4/4 artifacts complete。
- `openspec validate clean-current-runtime-evidence-and-dynamic-role-contract --type change --strict`: passed。
- `openspec validate --changes --strict` 的全局结果因另一个 open change `agent-cli-recoverable-failure-retry` 未完成而失败,不代表本次新 change 失败。

### 总结感悟
- 这条 spec 把 dynamic hats dogfood 的结论固定成了可实现、可验证的 OpenSpec 资产。
- 关键设计是“不增加新真相源”: record-session、events log、agents snapshot、evidence index 分别保留职责,由 evidence inspect 做 correlation。
- 后续实现应按 tasks.md 逐步推进,优先做 runtime protocol SSOT 和 dynamic role contract evidence,不要先扩 UX 或 retry 上层功能。

## [2026-05-23 18:02:28] [Session ID: omx-1779158263949-kticiv] 任务名称: 核验 clean-current-runtime-evidence-and-dynamic-role-contract OpenSpec

### 任务内容
- 根据用户要求核对 `clean-current-runtime-evidence-and-dynamic-role-contract` 是否已经作为新的 OpenSpec spec/change 完整落地。
- 本轮不实现代码,只做规格资产存在性与 OpenSpec 校验。

### 完成过程
- 检查同名 change 文件清单。
- 运行 `openspec status --change clean-current-runtime-evidence-and-dynamic-role-contract`。
- 运行 `openspec validate clean-current-runtime-evidence-and-dynamic-role-contract --type change --strict`。
- 确认 proposal/design/specs/tasks 均已完成。

### 总结感悟
- 这条 change 已经可以作为后续实现入口。
- 后续应从 `tasks.md` 开始按 spec-driven 流程推进,不要跳过 runtime protocol SSOT 和 dynamic role contract evidence 的基础约束。

## [2026-05-24 18:57:15] [Session ID: omx-1779158263949-kticiv] 任务名称: 落地 runtime protocol SSOT 与 dynamic role contract evidence

### 任务内容
- 按用户要求从 `openspec/changes/clean-current-runtime-evidence-and-dynamic-role-contract/tasks.md` 开始落地。
- 本轮只做 runtime protocol SSOT 和 dynamic role contract evidence。
- 明确没有扩 UI/TUI 分支,也没有实现 agent CLI recoverable retry 分支。

### 完成过程
- 在 `event_emission_protocol.rs` 增加 runtime topic classification SSOT,覆盖 runtime entry、coordinator-only control、observer-only、human input、human reply、hat reply、workflow result。
- `config.rs` 改为复用 SSOT 拒绝 ordinary hat 使用 reserved runtime/control triggers。
- `topology_runtime.rs` 改为复用 SSOT 裁剪/拒绝 task-derived role contract 的 output allowlist。
- `routing.rs` 的 gate strict-target bypass 改为复用 runtime protocol helper,避免局部复制控制面 topic 列表。
- 保留并验证已有 `EffectiveRoleContract` / `RoleContractSummary` / `topology.spawn.result` / `.ralph/agents.json` tombstone evidence 路径。
- 补充 dynamic worker prompt boundary 测试,确保 task-derived worker 不继承 coordinator-only topology spawn 指令。
- 更新 `tasks.md`,勾选 1.1-1.4 与 2.1-2.4。

### 验证证据
- `cargo test -p ralph-core event_emission_protocol --lib --quiet`: 5 passed。
- `cargo test -p ralph-core config::tests::test_reserved --lib --quiet`: 3 passed。
- `cargo test -p ralph-core --lib topology_spawn_group_rejects_control_plane_output_topic --quiet`: passed。
- `cargo test -p ralph-core --lib dynamic_worker_prompt_contains_effective_role_contract --quiet`: passed。
- `cargo test -p ralph-core --lib completed_dynamic_instance_remains_visible_as_agents_tombstone --quiet`: passed。
- `cargo test -p ralph-cli --test integration_topology_spawn --quiet`: passed。
- `cargo test -p ralph-core --lib runtime_capability_catalog_is_injected_only_into_ralph_prompt --quiet`: passed。
- `cargo test -p ralph-core --lib worker_prompt_excludes_coordinator_only_sections --quiet`: passed。
- `cargo test -p ralph-core --lib prompt_surface --quiet`: 6 passed。
- `openspec validate clean-current-runtime-evidence-and-dynamic-role-contract --type change --strict`: valid。
- `cargo test -p ralph-core smoke_runner --quiet`: 12 smoke tests passed。
- `cargo test --quiet`: full workspace passed。
- `git diff --check` on touched paths: passed。

### 总结感悟
- 这次没有新增第二套 evidence truth source,而是把现有协议分类和 dynamic role evidence 路径收敛到更清晰的单一来源。
- dynamic role evidence 这条线已经有较完整的运行时证据闭环: spawn result、agents current snapshot、completed dynamic tombstone、record summary integration guardrail 都能覆盖核心字段。
- 后续如果继续,应该从 3.x partial/tombstone lifecycle 或 4.x record summary correlation 接着做,不要把 UI 或 retry 混进这条主线。

## [2026-05-25 10:47:10] [Session ID: omx-1779158263949-kticiv] 任务名称: 落地 3.x Spawn group partial and tombstone lifecycle

### 任务内容
- 继续 `clean-current-runtime-evidence-and-dynamic-role-contract` OpenSpec。
- 本轮只落地 3.1-3.4: spawn group partial outcome、non-atomic continuation、failed-after-spawn tombstone、events-log evidence。
- 没有扩 UI/TUI,没有实现 agent CLI retry / recoverable failure 分支。

### 完成过程
- 扩展 `TopologySpawnFailedMember`,增加 `request_id`、`instance_id`、`phase`、`recovery_hint` 证据字段。
- 增加 topology spawn failure phase 常量,覆盖 validation、spawn、delivery failed after spawn、timeout、missing result、failed cleanup/reaping 的稳定命名。
- `spawn_group_members` 在 member validation failure、spawn failure、delivery failed after spawn 分支写出 request/phase/recovery evidence。
- 对 delivery failed after spawn 分支,将已经创建的 dynamic instance 标记为 `failed`,并复用 `completed_dynamic_instances` tombstone 保存失败态和 role contract summary。
- 增加带 retirement reason 的 dynamic unregister helper,保持 done tombstone 与 failed tombstone 走同一套 agents snapshot truth source。
- 补充 tests: validation failure evidence、non-atomic partial continuation、failed dynamic tombstone、runtime.lifecycle failed state record。
- 更新 `openspec/changes/clean-current-runtime-evidence-and-dynamic-role-contract/tasks.md`,勾选 3.1-3.4。

### 验证证据
- `cargo test -p ralph-core --lib topology_spawn_group_result_serializes_partial_failure --quiet`: passed。
- `cargo test -p ralph-core --lib topology_spawn_group_rejects_control_plane_output_topic --quiet`: passed。
- `cargo test -p ralph-core --lib topology_spawn_group_partial_failure_keeps_successful_members_running --quiet`: passed。
- `cargo test -p ralph-core --lib failed_dynamic_instance_remains_visible_as_agents_tombstone --quiet`: passed。
- `cargo test -p ralph-core --lib completed_dynamic_instance_remains_visible_as_agents_tombstone --quiet`: passed。
- `cargo test -p ralph-core --lib topology_spawn_group_ --quiet`: 16 passed。
- `openspec validate clean-current-runtime-evidence-and-dynamic-role-contract --type change --strict`: valid。
- `git diff --check` on touched paths: passed。
- `cargo test -p ralph-core --lib --quiet`: 606 passed。
- `cargo test -p ralph-core smoke_runner --quiet`: smoke passed。
- `cargo test --quiet`: full workspace passed。

### 注意事项
- `openspec validate --all --strict` 仍失败,但失败项是 unrelated active change `agent-cli-recoverable-failure-retry`,原因是该 change 没有 specs delta。本轮已记录到 `LATER_PLANS.md`,没有扩到 retry 分支。

### 总结感悟
- 这次遵循了“不新增第二套 truth source”的方向: partial failure 仍在 `topology.spawn.result.failed[]`,失败后的实例历史仍在 `completed_dynamic_instances` tombstone。
- 对 parent-visible dynamic spawn 来说,最重要的是让“创建了谁、谁失败在哪个阶段、是否还能在 agents snapshot 里追溯”成为同一条 evidence 链。

## [2026-05-25 13:05:20] [Session ID: omx-1779158263949-kticiv] 任务名称: 落地 4.x Record summary and evidence correlation

### 任务内容
- 继续 `clean-current-runtime-evidence-and-dynamic-role-contract` OpenSpec。
- 本轮只落地 4.1-4.4: `ralph record summary` / `record summary --agents-file` 的 dynamic spawn correlation 和 agents tombstone evidence 展示。
- 没有扩 UI/TUI,没有实现 retry / recoverable CLI failure,没有新增 record-session 的替代 truth source。

### 完成过程
- 在 `Evidence Inspect` 的 Termination 区域增加 `semantic_source: record-session _meta.termination`。
- 当 `_meta.termination` 缺失时,明确显示不能从 topology spawn success、stdout tail、wrapper exit status 或 display state 推断 workflow completion。
- 在 `topology.spawn.result` 输出里展开 partial failed member 明细,包括 request_id、instance_id、phase、recovery_hint 和 error。
- 增加 `Dynamic Result Coverage` 小节,逐个 spawned dynamic instance 展示 expected / covered / missing result topics。
- 保留 agents snapshot 当前 registry 与 completed dynamic tombstones 分区展示,并增加 `record summary --agents-file` integration test 覆盖。
- 更新 `openspec/changes/clean-current-runtime-evidence-and-dynamic-role-contract/tasks.md`,勾选 4.1-4.4。

### 验证证据
- `cargo test -p ralph-cli evidence_inspect --quiet`: 4 related tests passed。
- `cargo test -p ralph-cli --test integration_record_session record_summary_agents_file_shows_current_and_completed_dynamic_evidence --quiet`: passed。
- `cargo test -p ralph-cli --test integration_record_session --quiet`: 6 passed。
- `openspec validate clean-current-runtime-evidence-and-dynamic-role-contract --type change --strict`: valid。
- `cargo test -p ralph-cli --quiet`: passed。
- `cargo test -p ralph-core smoke_runner --quiet`: smoke passed。
- `git diff --check` on touched paths: passed。
- `cargo test --quiet`: full workspace passed。

### 注意事项
- 本轮发现 `crates/ralph-cli/src/record_session.rs` 已超过 1000 行,后续可以拆分 aggregate / renderer / pointer 模块。本轮已记录到 `LATER_PLANS.md`,没有在当前任务中重构。
- `openspec validate --all --strict` 的 unrelated `agent-cli-recoverable-failure-retry` 问题仍按上轮记录保留,本轮没有处理 retry 分支。

### 总结感悟
- 这次继续遵循 evidence closure 原则: record summary 只做 durable evidence 的关联展示,不替代 record-session / agents snapshot / events log。
- 4.x 之后,用户不用手动扫 JSONL 就能看到 dynamic spawn 创建了哪些实例、哪些 result topic 被哪些 source instance 覆盖、哪些 dynamic 结果缺失。

## [2026-05-25 14:12:00] [Session ID: omx-1779158263949-kticiv] 任务名称: 完成 clean-current-runtime-evidence-and-dynamic-role-contract 5.x-6.x

### 任务内容
- 继续并完成 `openspec/changes/clean-current-runtime-evidence-and-dynamic-role-contract/tasks.md` 的 5.x Evidence index correlation 和 6.x Release-fast gate / dogfood evidence。
- 本轮没有扩 UI/TUI,没有实现 retry / recoverable CLI failure 分支,没有处理 unrelated `agent-cli-recoverable-failure-retry`。

### 完成过程
- 扩展 `EvidenceIndexEntry`,增加 optional `result_topic`,用于关联 produced / expected result topic,不保存 result payload。
- 增加 dynamic role / spawn request / missing result marker helper,保持 evidence-index 只保存 artifact path 和 correlation id。
- 扩展 `find_by_correlation`,让 lookup 能匹配 primary / parent / child / result_topic,从而 request id 和 role hash 能查到 lineage 上的 missing marker。
- 将 parent-visible `topology.spawn_group` 真实运行路径接入 evidence-index,写出 request id -> child instance,role hash -> event log / agents snapshot + result topic 链接。
- 扩展 `integration_topology_spawn` guardrail,验证 `.ralph/evidence-index.jsonl` 中的 dynamic spawn/role hash artifact links。
- 在 `docs/runbook/testing-and-evidence.md` 增加 Runtime Evidence Lane Release-Fast Gate 命令集。
- 勾选 `tasks.md` 5.1-6.5,当前 change 进度 24/24。

### 验证证据
- `cargo test -p ralph-core evidence_index --lib --quiet`: 11 passed。
- `cargo test -p ralph-cli --test integration_topology_spawn parallel_parent_visible_spawn_materializes_dynamic_agents_without_redelivery --quiet`: passed。
- `cargo test -p ralph-cli --test integration_answer_evidence --quiet`: 3 passed。
- `cargo test -p ralph-core smoke_runner --quiet`: 12 passed。
- `uvx --from mkdocs --with mkdocs-material --with mkdocs-minify-plugin --with mkdocs-material-extensions --with pymdown-extensions mkdocs build --strict`: passed。
- `openspec validate clean-current-runtime-evidence-and-dynamic-role-contract --type change --strict`: valid。
- `openspec validate --all --strict`: 27 passed,1 failed; failed item is unrelated `change/agent-cli-recoverable-failure-retry` with no delta,本轮未处理。
- `git diff --check` on touched paths: passed。
- `cargo test --quiet`: full workspace passed。
- Preserved dogfood verifier: `DOGFOOD_OK`。
  - workspace: `/tmp/ralph-runtime-evidence-lane-dogfood-20260525-140000`
  - record-session: `/tmp/ralph-runtime-evidence-lane-dogfood-20260525-140000/session.jsonl`
  - events: `/tmp/ralph-runtime-evidence-lane-dogfood-20260525-140000/.ralph/events.jsonl`
  - agents snapshot: `/tmp/ralph-runtime-evidence-lane-dogfood-20260525-140000/.ralph/agents.json`
  - evidence index: `/tmp/ralph-runtime-evidence-lane-dogfood-20260525-140000/.ralph/evidence-index.jsonl`
  - summary: `/tmp/ralph-runtime-evidence-lane-dogfood-20260525-140000/summary.txt`

### 注意事项
- dogfood 首次 verifier 失败是检查脚本期待 `spawned: 3`,但当前 summary 实际格式是 `requested_instances=3` 和 `spawned=[...] failed=0`;已修正 verifier 并重跑通过。
- integration test 中 agents snapshot artifact path 在 macOS 下可能出现 `/var` 与 `/private/var` 文本差异,已改为后缀断言以避免路径 symlink 误判。

### 总结感悟
- evidence-index 应该是导航索引,不是事实副本。最有价值的是让 request id、role hash、instance id、result topic 能回到 record-session/events/agents snapshot。
- runtime dogfood 必须保留 artifact,否则只能证明测试瞬间通过,不能支撑后续复盘。

## [2026-05-26 00:30:00] [Session ID: omx-1779158263949-kticiv] 任务名称: 归档 clean-current-runtime-evidence-and-dynamic-role-contract

### 任务内容
- 继续上一轮完成后的收尾动作: 归档 `clean-current-runtime-evidence-and-dynamic-role-contract` OpenSpec change。
- 因 `task_plan.md` 已接近 1000 行,先执行计划文件续档和最小 continuous-learning。
- 同步 delta specs 到主规格,再归档 change。

### 完成过程
- 将旧 `task_plan.md` 续档为 `archive/default_history/task_plan_2026-05-25_1420_pre_archive_clean_runtime_evidence.md`。
- 在 `EXPERIENCE.md` 追加 `exp-20260526-runtime-evidence-closure-and-dynamic-role-index`。
- 创建归档 manifest: `archive/manifests/ARCHIVE_MANIFEST__default_task_plan_rollover_2026-05-26_0010.md`。
- 同步 5 个 delta specs 到主规格:
  - `openspec/specs/current-runtime-evidence-contract/spec.md` 新建。
  - `openspec/specs/parallel-hat-instances/spec.md` 更新。
  - `openspec/specs/prompt-contract-runtime-alignment/spec.md` 更新。
  - `openspec/specs/record-session-contract-and-watch/spec.md` 更新。
  - `openspec/specs/runtime-evidence-index-kernel/spec.md` 更新。
- 将 change 移动到 `openspec/changes/archive/2026-05-26-clean-current-runtime-evidence-and-dynamic-role-contract/`。
- 更新 `LATER_PLANS.md`,标记 dynamic hats runtime evidence 主线已完成并归档。

### 验证证据
- `openspec validate current-runtime-evidence-contract --type spec --strict`: valid。
- `openspec validate parallel-hat-instances --type spec --strict`: valid。
- `openspec validate prompt-contract-runtime-alignment --type spec --strict`: valid。
- `openspec validate record-session-contract-and-watch --type spec --strict`: valid。
- `openspec validate runtime-evidence-index-kernel --type spec --strict`: valid。
- `openspec validate clean-current-runtime-evidence-and-dynamic-role-contract --type change --strict`: valid before archive。
- `openspec list --json`: active changes no longer include `clean-current-runtime-evidence-and-dynamic-role-contract`。
- `openspec validate --all --strict`: 27 passed,1 failed; remaining failure is unrelated active change `agent-cli-recoverable-failure-retry` with no delta。
- `git diff --check` on touched archive/spec/context paths: passed。

### 总结感悟
- OpenSpec archive 前必须先同步 delta specs,否则实现完成但主规格缺失会留下长期漂移。
- OpenSpec CLI telemetry 的正确关闭方式是 `OPENSPEC_TELEMETRY=0` 或 `DO_NOT_TRACK=1`; `POSTHOG_DISABLED=1` 不足以关闭本地 OpenSpec 的 PostHog flush。

## [2026-05-26 15:11:30] [Session ID: omx-1779158263949-kticiv] 任务名称: agent-cli-recoverable-failure-retry specs delta

### 任务内容
- 为 OpenSpec change `agent-cli-recoverable-failure-retry` 创建 specs delta artifact。
- 覆盖 1 个新 capability: `agent-cli-recoverable-failure-retry`。
- 覆盖 2 个 modified capability 的新增要求: `parallel-hat-instances`, `supervisor-human-chat-gate`。

### 完成过程
- 读取 change status、specs instructions、proposal 和 design。
- 按 proposal 的 Capabilities 创建 3 个 delta spec 文件。
- 使用 `ADDED Requirements` 表达新增行为,避免误用 `MODIFIED` 覆盖已有主规格。
- 运行 `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate agent-cli-recoverable-failure-retry --type change --strict`,通过。
- 运行 `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate --all --strict`,通过,结果为 28 passed,0 failed。

### 总结感悟
- 本轮 no-delta 阻断已经解除,`tasks.md` 已解锁为 ready。
- 下一轮应继续按 OpenSpec artifact 顺序创建 tasks,暂不直接实现代码。

## [2026-05-26 15:20:30] [Session ID: omx-1779158263949-kticiv] 任务名称: agent-cli-recoverable-failure-retry tasks artifact

### 任务内容
- 创建 `openspec/changes/agent-cli-recoverable-failure-retry/tasks.md`。
- 将 recoverable failure retry change 拆成可实施、可验证的小任务。

### 完成过程
- 读取当前 change status,确认 `tasks` artifact 为 ready。
- 读取 tasks instructions、design 和 3 个 delta specs。
- 快速查看相关代码入口,包括 `HatJobResult`、parallel runner、Supervisor chat parser、agents snapshot 和配置结构。
- 创建 6 组任务: core model/policy、append-only ledger、parallel runtime lifecycle、manual continue、human-facing evidence、integration/final validation。
- 运行当前 change strict validate 和全量 OpenSpec strict validate,均通过。

### 总结感悟
- 该 change 的 OpenSpec artifact 阶段已经 complete。
- 下一步可以安全进入 implementation,但必须先做 classifier/ledger/scheduler 的 focused tests,再接入并行 runtime。

## [2026-05-28 09:59:30] [Session ID: omx-1779158263949-kticiv] 任务名称: agent-cli-recoverable-failure-retry 1.x implementation

### 任务内容
- 实现 `openspec/changes/agent-cli-recoverable-failure-retry/tasks.md` 的 1.1-1.6。
- 交付 recoverable failure 核心类型、确定性分类器、retry policy 配置和 focused tests。

### 完成过程
- 使用 OpenSpec apply workflow 确认当前 change 为 `agent-cli-recoverable-failure-retry`。
- 新增 `crates/ralph-core/src/recoverable_failure.rs`:
  - `RecoverableFailureKind`
  - `RecoverableFailureStatus`
  - `RecoverableFailureTransition`
  - `RecoverableFailureSnapshot`
  - `RecoverableFailureInput`
  - `RecoverableFailureClassification`
  - `AgentCliRecoverableFailuresConfig`
  - `classify_recoverable_failure` / `classify_hat_job_result`
- 在 `RalphConfig` 顶层加入 `agent_cli_recoverable_failures`。
- 在 `CoreConfig` 增加 `.ralph/recoverable-failures.jsonl` 的 SSOT path resolver。
- 增加分类器与配置解析/校验测试。
- 勾选 `tasks.md` 的 1.1-1.6。
- clippy 暴露 `event_emission_protocol.rs` 的字符串累积 warning,已用 `writeln!` + `fold` 消除,无行为变更。

### 验证证据
- `cargo test -p ralph-core --lib recoverable --quiet`: 16 passed。
- `cargo test -p ralph-core --lib config::tests::test_default_config -- --exact`: passed。
- `cargo test -p ralph-core --lib config::tests::test_core_config_resolves_scoped_experience_paths -- --exact`: passed。
- `cargo clippy -p ralph-core --quiet`: passed,无输出。
- `cargo test -p ralph-core --quiet`: package tests passed。
- `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate --all --strict`: 28 passed,0 failed。

### 总结感悟
- 1.x 只建立 retry 语义真相源,没有改变 runtime 行为。
- 下一步 2.x 应实现 ledger append/replay,仍需继续坚持“不存 full prompt / 不复制 event stream”的边界。

## [2026-05-28 10:34:30] [Session ID: omx-1779158263949-kticiv] 任务名称: agent-cli-recoverable-failure-retry 2.x ledger implementation

### 任务内容
- 实现 `openspec/changes/agent-cli-recoverable-failure-retry/tasks.md` 的 2.1-2.4。
- 将 recoverable failure 从纯类型/分类器推进为可落盘、可回放的 append-only evidence ledger。

### 完成过程
- 在 `crates/ralph-core/src/recoverable_failure.rs` 增加 `RecoverableFailureLedger`。
- 实现 `append_transition()`:
  - 自动创建父目录。
  - 组装完整 JSON line 后一次性追加写入。
  - flush 后返回。
  - 写入前收紧 `stderr_excerpt`。
- 实现 `read_transitions()`:
  - ledger 不存在时返回空集合。
  - 空行跳过。
  - malformed JSON line 返回带 path 和 line_number 的错误。
- 实现 `replay_snapshots()`:
  - 顺序 replay transition。
  - 同一个 `failure_id` 后写入的 transition 覆盖为最新 snapshot。
- 增加 `stable_recoverable_failure_id()`,只使用 job id、instance id 和 failure kind。
- 增加 focused tests 覆盖 append ordering、多 transition replay、missing file、malformed line、bounded stderr excerpt 和 compact metadata。
- 勾选 tasks 2.1-2.4。

### 验证证据
- `cargo test -p ralph-core --lib recoverable --quiet`: 23 passed。
- `cargo clippy -p ralph-core --quiet`: passed,无输出。
- `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate agent-cli-recoverable-failure-retry --type change --strict`: passed。
- `cargo test -p ralph-core --quiet`: package tests passed。
- `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate --all --strict`: 28 passed,0 failed。

### 总结感悟
- 2.x 仍未改变 runtime 行为,只是补齐了 durable evidence substrate。
- 下一步 3.x 才应把 failed HatJobResult 接到 classifier + ledger + retry-aware lifecycle。

## [2026-05-28 12:26:41] [Session ID: omx-1779158263949-kticiv] 任务名称: agent-cli-recoverable-failure-retry 3.x parallel runtime retry lifecycle

### 任务内容
- 实现 `openspec/changes/agent-cli-recoverable-failure-retry/tasks.md` 的 3.1-3.7。
- 将 recoverable failure 从 classifier/ledger 层接入 parallel runtime job lifecycle。

### 完成过程
- 在 `recoverable_failure.rs` 增加 `Recovered` lifecycle status 和 `recoverable_retry_delay_ms()` bounded backoff helper。
- 在 `HatInstanceActor` 中加入 recoverable retry runtime:
  - 保存 runtime-held `HatJob` context 和 source event ids。
  - classified recoverable failure 时写 ledger transition 并调度 retry。
  - retry due 后复用内存 job context 重新执行。
  - retry 成功写入 `recovered` transition。
  - attempts exhausted 时写入 `exhausted`,并把 ledger pointer 附到 terminal stderr evidence。
- 在 `ParallelSupervisor` 中加入 recoverable live snapshot:
  - 消费 `RecoverableFailureTransition`。
  - pending recoverable 存在时阻止 coordinator completion promise。
  - 如果 recoverable transition 出现在 completion drain 期间,重新打开 supervisor loop,避免 scheduled retry 被 freeze 吃掉。
- 同步 OpenSpec design/spec,补充 retry 成功后的 `recovered` 终态。
- 勾选 tasks.md 的 3.1-3.7。

### 验证证据
- `cargo check -p ralph-core --quiet`: passed。
- `cargo test -p ralph-core --lib recoverable --quiet`: 28 passed。
- `cargo test -p ralph-core --lib parallel::instance::tests::recoverable_failure_schedules_retry_and_preserves_stdout_only_parsing -- --exact`: passed。
- `cargo test -p ralph-core --lib parallel::instance::tests::recoverable_failure_exhaustion_becomes_terminal_with_ledger_pointer -- --exact`: passed。
- `cargo test -p ralph-core --lib parallel::supervisor::routing_tests::pending_recoverable_failures_block_completion_gate -- --exact`: passed。
- `cargo clippy -p ralph-core --quiet`: passed,无输出。
- `cargo test -p ralph-core --quiet`: 638 lib tests plus integration/doctests passed。
- `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate agent-cli-recoverable-failure-retry --type change --strict`: passed。
- `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate --all --strict`: 28 passed,0 failed。

### 总结感悟
- retry lifecycle 不能只靠 ledger replay,Supervisor 还需要 live transition map 来阻止 completion 提前收敛。
- retry 成功必须有 `recovered` 终态,否则 ledger snapshot 会长期停在 `retrying`,后续 evidence inspect 和 completion gate 都无法证明该 lifecycle 已解决。
- stderr 作为 retry classifier evidence 与 stdout-only EventParser 必须继续保持类型层隔离。

## [2026-05-28 15:00:30] [Session ID: omx-1779158263949-kticiv] 任务名称: agent-cli-recoverable-failure-retry 4.x manual continue control path

### 任务内容
- 实现 `openspec/changes/agent-cli-recoverable-failure-retry/tasks.md` 的 4.1-4.5。
- 为 recoverable agent CLI failure 增加显式人工 `!continue` 控制路径。

### 完成过程
- 增加 `TOPIC_RECOVERABLE_CONTINUE = "recoverable.continue"`,作为 Supervisor 消费的 external control topic。
- 扩展 TUI `ChatSubmit`:
  - `!continue` -> bare recoverable continue。
  - `!continue <failure_id>` -> explicit recoverable continue。
  - 普通 `继续分析这个问题` 仍是 ordinary `human.message`。
- TUI 写入 `recoverable.continue` external JSONL,并把当前 selected instance 作为 bare continue 的消歧提示。
- Supervisor 在 `route_event` 早期消费 `recoverable.continue`,不把它交给普通 TopicContract / hat 路由。
- Supervisor 用 live `recoverable_failures` snapshot 解析 continue 目标:
  - explicit id unknown 或状态不允许 continue 时拒绝。
  - bare continue 先按 selected instance 唯一候选解析,否则按全局唯一 pending 解析,仍歧义则拒绝。
  - 拒绝通过 `routing.escalate` 留下可见/auditable 证据。
- `HatInstanceCommand` 增加 `ContinueRecoverableFailure { failure_id }`。
- Instance 接受 manual continue 后先 append/publish `continued_by_human`,再把已有 scheduled retry 的 due 时间提前,复用 `maybe_start_scheduled_retry()` 执行真实 retry。
- 保持 3.x 边界: retry 只复用 runtime-held `HatJob` context,不从 ledger 重建 prompt/event stream。
- 勾选 tasks.md 的 4.1-4.5。

### 验证证据
- `cargo fmt`: passed。
- `cargo clippy -p ralph-core --quiet`: passed。
- `cargo clippy -p ralph-tui --quiet`: passed。
- `cargo test -p ralph-tui chat --quiet`: 24 passed。
- `cargo test -p ralph-core --lib parallel::instance::tests::manual_continue_appends_transition_and_uses_scheduled_retry_path -- --exact`: passed。
- `cargo test -p ralph-core --lib parallel::supervisor::routing_tests::explicit_recoverable_continue_accepts_only_waiting_failures -- --exact`: passed。
- `cargo test -p ralph-core --lib parallel::supervisor::routing_tests::bare_recoverable_continue_uses_selected_instance_to_disambiguate -- --exact`: passed。
- `cargo test -p ralph-core --lib parallel::supervisor::routing_tests::bare_recoverable_continue_falls_back_to_global_unique_when_selected_has_no_failure -- --exact`: passed。
- `cargo test -p ralph-core --quiet`: 642 lib tests plus integration/doctests passed。
- `cargo test -p ralph-tui --quiet`: 239 lib tests plus integration/doctests passed。
- `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate agent-cli-recoverable-failure-retry --type change --strict`: passed。
- `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate --all --strict`: 28 passed,0 failed。

### 总结感悟
- `!continue` 必须是显式 control intent,不能作为普通 chat payload 的自然语言推断。
- completion pending 状态和 manual-continueable 状态需要分开: `retrying` 会阻止 completion,但不应该接受人工 continue。
- manual continue 的正确落点不是新 executor,而是把 scheduled retry 的 due 时间提前,继续复用现有 scheduler path。

## [2026-05-28 16:30:04] [Session ID: omx-1779954714247-oab9zc] 任务名称: agent-cli-recoverable-failure-retry 5.x human-facing observability

### 任务内容
- 完成 `openspec/changes/agent-cli-recoverable-failure-retry/tasks.md` 的 5.1-5.4。
- 让 recoverable retry lifecycle 在 agents snapshot、`ralph agents` 和 record-session Evidence Inspect 中可见。

### 完成过程
- 修复 `test_agents_command_prints_recoverable_summary` 对紧凑表格 failure id 前缀的过强断言。
- 补强 `ralph agents --format json` 集成断言,证明完整 `failure_id`、attempt、`next_retry_at` 和 ledger path 保留在 agents snapshot JSON 中。
- 修复 `record_session` 测试 fixture 字段误放,让 recoverable evidence fixture 保持在 `AgentInstanceSnapshot.recoverable_failures` 上。
- 确认 Evidence Inspect 会渲染 scheduled、continued、exhausted 三类 recoverable failure,并指向 `.ralph/recoverable-failures.jsonl`。

### 验证证据
- `cargo test -p ralph-cli --test integration_agents --quiet`: 9 passed。
- `cargo test -p ralph-cli --bin ralph record_session::tests --quiet`: 6 passed。
- `cargo test -p ralph-core --lib recoverable --quiet`: 32 passed。
- `cargo test -p ralph-core --lib parallel::supervisor::routing_tests::agents_snapshot_includes_recoverable_failure_summaries -- --exact --nocapture`: passed。

### 总结感悟
- 紧凑 CLI 表格适合显示状态摘要,完整 failure evidence 应由 agents snapshot JSON 和 Evidence Inspect 承载。
- 测试断言要锁住语义字段,不要绑定列宽截断后的偶然字符串。

## [2026-05-28 16:56:01] [Session ID: omx-1779954714247-oab9zc] 任务名称: agent-cli-recoverable-failure-retry 6.x guardrails and final validation

### 任务内容
- 完成 `openspec/changes/agent-cli-recoverable-failure-retry/tasks.md` 的 6.1-6.8。
- 为 recoverable retry 的自动 retry、manual continue、exhaustion 和最终回归门禁提供证据。

### 完成过程
- 复用 `RecoverableThenSuccessExecutor` 作为 fake executor fixture,覆盖首轮 `ERROR: exceeded retry limit, last status: 429 Too Many Requests` 后 retry 成功。
- 复用 `AlwaysRecoverableFailureExecutor` 覆盖 attempts exhausted 后 terminal evidence pointer。
- 运行 Supervisor continue parsing focused tests,确认 explicit/bare continue 的解析和拒绝逻辑稳定。
- 修复新增 `AgentInstanceSnapshot.recoverable_failures` 后暴露出的旧测试 fixture 漏字段问题。
- 清理 clippy warning,确保最终 clippy 无输出。
- 勾选 6.1-6.8,确认 OpenSpec apply state 为 `all_done`。

### 验证证据
- `cargo test -p ralph-core --lib parallel::instance::tests::recoverable_failure_schedules_retry_and_preserves_stdout_only_parsing -- --exact --nocapture`: passed。
- `cargo test -p ralph-core --lib parallel::instance::tests::manual_continue_appends_transition_and_uses_scheduled_retry_path -- --exact --nocapture`: passed。
- `cargo test -p ralph-core --lib parallel::instance::tests::recoverable_failure_exhaustion_becomes_terminal_with_ledger_pointer -- --exact --nocapture`: passed。
- `cargo test -p ralph-core smoke_runner --quiet`: 12 passed。
- `cargo fmt`: passed。
- `cargo clippy -p ralph-core --quiet`: passed,无输出。
- `cargo clippy -p ralph-cli --quiet`: passed,无输出。
- `cargo test -p ralph-core --quiet`: passed。
- `cargo test -p ralph-cli --quiet`: passed。
- `cargo test --quiet`: workspace passed。
- `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate agent-cli-recoverable-failure-retry --type change --strict`: passed。
- `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate --all --strict`: 28 passed,0 failed。
- `git diff --check`: passed。

### 总结感悟
- 6.x 不需要再新增另一套外部 backend 脚本;已有 executor fixture 更直接验证 runtime-held job context、scheduler、ledger 和 stdout-only parsing invariant。
- `AgentInstanceSnapshot` 增加字段后,所有 integration fixture 都必须显式补默认空 Vec,否则 bin/integration target 才会暴露漏项。

## [2026-05-28 17:51:07] [Session ID: omx-1779954714247-oab9zc] 任务名称: recoverable failure retry 文档与项目级 skill 同步

### 任务内容
- 更新 `EXPERIENCE.md`,删除/更正旧的 `agent-cli-recoverable-failure-retry` no-delta 阻断口径。
- 新增项目级 self-learning skill: `.codex/skills/self-learning.ralph-agent-cli-recoverable-failure-retry/SKILL.md`。
- 在 `AGENTS.md` 的 Project Knowledge Index 中增加该 skill 索引。

### 完成过程
- 先验证交接摘要,发现 skill 文件不存在、AGENTS 未索引、EXPERIENCE 仍有旧 no-delta 说法。
- 读取相邻 self-learning skill 风格和已归档的 `agent-cli-recoverable-failure-retry` stable spec。
- 将 `EXPERIENCE.md` 中旧口径改为 2026-05-28 已归档后的当前判断: 后续 validate 失败应按实时输出重新定位,不要默认归因到 recoverable retry。
- 新 skill 记录 ledger 真相源、窄分类器、有界 retry policy、stderr 不解析、manual continue 复用 retry path、agents snapshot 摘要边界和验证锚点。
- `AGENTS.md` 增加 skill 索引,确保后续 agent 能在 429 / `exceeded retry limit` / recoverable ledger / Evidence Inspect 场景直接找到该经验。

### 验证证据
- `python3` frontmatter/关键短语检查: passed。
- `git diff --check -- EXPERIENCE.md AGENTS.md .codex/skills/self-learning.ralph-agent-cli-recoverable-failure-retry/SKILL.md task_plan.md`: passed。
- `rg -n 'no-delta change 阻断|当前可能被 unrelated `agent-cli-recoverable-failure-retry` no-delta|ralph-agent-cli-recoverable-failure-retry|Recoverable agent CLI failure retry' ...`: 使用单引号 pattern 重跑后 passed。

### 总结感悟
- 这次交接摘要声称三项已完成,但真实文件状态显示并未完成。以后接手压缩摘要后,必须先验证文件系统真相。
- 搜索带反引号的 Markdown 文本时,必须用单引号 pattern,否则 zsh 会把反引号内容当命令执行。

## [2026-05-29 00:03:07] [Session ID: omx-1779954714247-oab9zc] 任务名称: recoverable failure retry guidance scoped commit

### 任务内容
- 将 recoverable failure retry 文档 / skill 同步结果做本地提交。
- 严格控制提交范围,避免混入当前 worktree 里大量既有代码改动。

### 完成过程
- staged: `.codex/skills/self-learning.ralph-agent-cli-recoverable-failure-retry/SKILL.md`, `AGENTS.md`, `EXPERIENCE.md`。
- 运行 skill 结构检查和 staged diff whitespace 检查。
- 创建本地 commit: 2bf2aba5 `docs: sync recoverable failure retry guidance`。

### 总结感悟
- 在大脏工作区里收口提交时,先看 `git diff --cached --name-status`,再 commit,可以有效避免把无关代码改动带入文档提交。
