## [2026-05-17 22:58:00] [Session ID: omx-1779004640353-blcixq] 笔记: parallel_rec child_run panic 与 UTF-8 截断

## 来源

### 来源1: parallel_rec.jsonl

- 记录文件: `/Users/cuiluming/local_doc/l_dev/my/rust/ralph-orchestrator/parallel_rec.jsonl`
- 关键事件:
  - `topic=capability.failed`
  - `failure_class=child_run_failed`
  - `error` 包含 `byte index 60 is not a char boundary`
  - `panic` 指向 `crates/ralph-cli/src/main.rs:1256:41`

### 来源2: 运行态证据

- `parallel_rec.jsonl` 仍然在被 `ralph run --record-session parallel_rec.jsonl` 写入时检查到,说明这是活跃会话的动态失败,不是静态归档。
- 进程 `15429` 仍在运行,并持有该 record 文件句柄。

## 综合发现

### 现象

- 外层会话持续有 reasoning / thinking 轨迹。
- 但最终结果没有正常落地成 `reply.human.message` 或 `_meta.termination`。
- 中途有一个 child run 直接 panic,并返回 `capability.failed`。

### 候选假设

- 最强假设: `main.rs` 在处理用户输入时按 byte 进行截断/切片,而输入里含有中文字符,导致 UTF-8 边界错误。
- 备选解释: 不是结果逻辑本身坏,而是子运行在 payload 预处理阶段崩溃,父层继续做分析性输出,所以看起来像“只会思考不会出结果”。

### 当前判断

- 目前已经能确认的问题层级是 child_run 级别的 panic,不是简单的显示问题。
- 还不能直接宣称根因已经完全确认,因为还没读到 `main.rs:1256` 附近的具体实现。

### 下一步验证

- 读取 `crates/ralph-cli/src/main.rs` 1256 附近。
- 搜索所有可能的 byte slicing / truncate 代码。
- 对照 record 里的 `capability.request` 原文,确认中文 prompt 是如何被截断的。

## [2026-05-17 23:31:00] [Session ID: omx-1779004640353-blcixq] 笔记: screenshot 中的 `Preparing patch content` 属于活跃 turn 而非崩溃

## 来源

### 来源1: 用户截图

- 画面底部显示 `act: Preparing patch content... (17m 11s • Ctrl+C to interrupt)`。
- 主视图仍在输出 `ralph#1:out:job=2` 的历史回显和状态文字。

### 来源2: `parallel_rec.jsonl` / 运行态快照

- `record summary` 显示 `Termination reason: Interrupted`。
- `.ralph/agents.json` 显示 `ralph#1` 仍处于 `running`, `ralph#2` 也处于 `running`。
- `parallel_rec.jsonl` 里最新可见主题仍是 `human.message` 和前面的 `capability.failed`, 没有新的终止事件。

## 综合发现

### 现象

- TUI 看起来像“卡住”,但实际是活动状态长时间停留在 `Preparing patch content`。
- 没有证据表明这里发生了新一轮 panic 或 IO 死锁。

### 候选解释

- 最强解释: 这是一个仍在生成/整理 patch 的长 turn,所以状态一直停留在 patch preparation。
- 备选解释: 上下文过大,模型在 patch 生成前迟迟收不敛,因此 UI 看起来没有推进。

### 当前判断

- 更像是“长时间未完成的 turn / reasoning stage”,不是“程序已经崩掉却没显示错误”。
- 由于当前 `act` 仍指向 patch preparation,后续要么继续等它输出,要么主动 interrupt 再重启这轮。

## [2026-05-17 23:59:00] [Session ID: omx-1779004640353-blcixq] 笔记: workflow capability materialization 红绿验证

## 来源

### 来源1: 最小动态复现命令

- 命令: `cargo run --quiet -p ralph-cli --bin ralph -- tools capability invoke --id workflow:default-parallel --input '请分析中文能力' --workspace /tmp/ralph-capability-materialization-before --json`
- 修复前输出摘要:
  - `parallel.enabled=false`
  - `hats.count=0`
  - `event_loop.prompt` 仍是 capability stub 包装文本

### 来源2: 新增回归测试

- 测试: `tools_capability_invoke_materializes_default_parallel_workflow_config`
- 修复前结果: failed。
- 失败点: `resolved-config.yml` 里 `parallel.enabled=false` 且 `hats: {}`。
- 修复后结果: passed。

## 综合发现

### 现象

- `workflow:default-parallel` 能从 catalog 暴露出来,但 invocation 写出的 resolved config 没有使用该 workflow preset 的 YAML 内容。
- 这解释了为什么 artifact 里仍然是 `hats: {}` / `parallel.enabled: false`。

### 已验证结论

- 失败路径真实发生在 `invoke_isolated_with_runner()` 写 artifact 前调用的 `resolved_config_for_capability()`。
- 修复点不是 parent topology 热更新,而是 isolated child 的 resolved config materialization。
- 改为从 `startup_resources::embedded_catalog()` 找 workflow preset 并解析其内容后,`workflow:default-parallel` 的 resolved config 可以保留真实 `hats` 和 `parallel.enabled=true`。
## [2026-05-18 07:01:08] [Session ID: omx-1779004640353-blcixq] 笔记: 最新 live capability 复跑已经转绿

## 来源

### 来源1: 全量测试复跑

- 命令: `cargo test --quiet`
- 结果: 全量测试通过,没有再复现之前提到的 `parallel_parent_run_*` 失败。

### 来源2: live capability 专门测试

- 命令: `cargo test -p ralph-cli --test integration_live_capability --quiet`
- 结果: `5 passed; 0 failed`。

### 来源3: 代码路径快速复核

- 复核了 `crates/ralph-core/src/parallel/supervisor/capability_runtime.rs` 与 `crates/ralph-cli/src/capability.rs`。
- 看到 parent-return 事件已经会被 supervisor 收进 `capability_return_events`,再写回事件日志并继续路由。

## 综合发现

### 现象

- 之前怀疑“持续输出 thinking 但没有结果”的 live capability 问题,在当前代码状态下已经不再复现。
- `integration_live_capability` 的 5 个用例现在全绿。

### 当前判断

- 先前那组失败更像是两层历史问题叠加:
  - child run 的 UTF-8 预览 panic。
  - workflow capability 的 resolved config 物化为 stub。
- 这两个层面都已经被前序修复覆盖,当前没有新证据指向 parent result 回写链路继续坏着。

### 后续建议

- 如果后面再出现“只思考不出结果”,优先先看 `events.jsonl` 里有没有 `capability.result` / `capability.failed`,再区分是 child 失败还是 parent 回写问题。
- 对 live capability 再加一个小型 smoke 脚本,固定验证 `parallel_parent_run_triggers_live_capability_invocation_and_inspect_evidence`。

## [2026-05-18 07:18:00] [Session ID: omx-1779004640353-blcixq] 笔记: 为什么会思考太多轮才发 event

## 来源

### 来源1: `parallel_rec.jsonl` 动态统计

- 总记录数: 1372。
- `ux.terminal.write`: 1366 条。
- `bus.publish`: 3 条。
- stdout 里真正可路由的事件标签只有 4 个左右,而 stderr/思考流里有大量 `I’m thinking...`/`I'm thinking...` 之类的元认知输出。
- 关键时间跨度: session start 到最后一次记录约 2,285,560 ms,也就是接近 38 分钟。

### 来源2: prompt / routing 代码

- `crates/ralph-core/src/parallel/supervisor.rs` 会给 `ralph#1` 注入:
  - runtime capability catalog.
  - event emission protocol.
  - HUMAN CHAT / KEY SEMANTICS / HATS TOPOLOGY / WHAT TO DO.
- `render_event_emission_protocol()` 允许输出很长的自然语言说明,但没有硬性限制“先发 event 再解释”。

### 来源3: 现场输出内容

- 记录里明显看到模型在反复思考:
  - memory 如何读。
  - task_plan / notes / WORKLOG 是否要先更新。
  - 是否要用 multi_tool_use.parallel。
  - 该先补哪个证据再收口。
- 这些都是流程性元问题,不是业务问题本身。

## 综合发现

### 现象

- 这次慢,不是因为问题本体复杂到必须做很多轮推理。
- 而是模型被一整套协作/文件上下文/证据/规划/技能协议牵着走,先做了很多元工作,才进入真正的事件输出。

### 候选解释

- 主解释: prompt 面太宽,`ralph#1` 同时承担分析员、协调员、记忆整理员、文件记录员、验证执行者等角色,导致它在简单问题上也会先做大量资格检查和上下文梳理。
- 备选解释: 没有“单轮必须产出结构化事件”的硬门槛,所以模型会继续延长自然语言思考,直到自己认为证据足够。

### 当前判断

- 这不是纯粹的“代码创建失败”问题。
- 代码失败只是第一层,而 token 浪费的真正来源是 prompt / orchestration 过宽,以及缺少一个对简单问题的 event-first 快速路径。

### 后续建议

- 给 `ralph#1` 加一个更硬的快路径: 简单问题优先一轮内产出一个结构化 event 或简短结论,不要先做全套文件治理。
- 把“协调/记忆治理”和“回答当前问题”拆成两个更窄的 prompt surface。
- 如果要继续保留这套宽 prompt,至少加一个 turn budget 或 validator,避免简单问题被流程性元任务拖长。
## [2026-05-18 10:44:57] [Session ID: omx-1779004640353-blcixq] 笔记: Ralph prompt 分层设计图已落盘

## 来源

### 来源1: 用户设计约束

- Ralph 的任务是决定任务如何分发、安排、分配,而不是真正解决问题。
- 非 Ralph hat 不应该和 Ralph 共享太多 prompt,否则职责会不清楚。
- Ralph 创建 hat instance 时,来源可以是项目模板、`ralph.yml` 静态 hats,也可以是 task-derived dynamic hat。

### 来源2: 设计产物

- 文档: `specs/ralph-prompt-role-layering.md`
- Mermaid 验证: flowchart 和 sequenceDiagram 均通过 `beautiful-mermaid-rs --ascii`。

## 综合发现

### 设计结论

- Ralph 使用 coordinator prompt。
- 非 Ralph hat 使用 worker prompt。
- 两者只共享最小 shared protocol。
- dynamic hat 是一等身份来源,但必须携带最小 role contract,不能复制 Ralph 的完整 prompt。

## [2026-05-18 11:15:00] [Session ID: omx-1779004640353-blcixq] 笔记: role-aware reasoning effort 注入点

## 来源

### 来源1: `crates/ralph-core/src/config.rs`

- `CliConfig` 目前只有 `backend` / `command` / `prompt_mode` / `default_mode` / `idle_timeout_secs` / `args` / `prompt_flag`。
- 当前没有语义化的 role-aware reasoning effort 字段。

### 来源2: `crates/ralph-cli/src/parallel_runner.rs`

- `CliHatJobExecutor::execute()` 会根据 `HatJob.backend` 选择默认 backend 或 hat-level backend。
- `HatJob` 含 `hat_id` 与 `instance_id`,因此这里能区分 `ralph` coordinator 与 non-Ralph worker。
- 现有 `custom_args` 会追加到 backend args 末尾,所以默认 reasoning effort 应在追加 custom args 后再做“若无显式配置则注入”。

### 来源3: `crates/ralph-cli/src/loop_runner.rs`

- 串行/hatless Ralph 主循环里,实际执行者通常是 `ralph`。
- 但 backend selection 使用 `display_hat`,这意味着如果当前活跃的是 non-Ralph hat,也能用 worker role 推导默认 reasoning effort。

### 来源4: `crates/ralph-cli/src/codex_app_server_session.rs`

- app-server 参数解析已经支持从 backend args 读取 `--config model_reasoning_effort="..."` 并转发给 `codex app-server`。
- 因此运行时只要把 role-aware 默认注入到 `CliBackend.args`,exec 与 app-server 两条 Codex 路径都能受益。

## 综合发现

### 现象

- 当前仓库里 `model_reasoning_effort` 只在 E2E 场景里手写进 args,不是 Ralph runtime 的角色默认。

### 当前假设

- 最合适的修复点是建立 `cli.reasoning_effort.{coordinator,worker}` 语义配置,默认值分别为 `medium` / `high`。
- 然后在 runtime 选择出最终 backend 后,根据当前 role 注入 Codex CLI 的 `--config model_reasoning_effort="..."`。

### 验证计划

- 新增 config 解析测试: 默认 medium/high,显式覆盖能解析。
- 新增 adapter helper 测试: coordinator 注入 medium,worker 注入 high,已有显式 override 时不重复注入。
- 新增 runner 相关测试或 focused tests,确认 app-server parser 能看到注入后的 config override。

## [2026-05-18 11:30:57] [Session ID: omx-1779004640353-blcixq] 笔记: role-aware reasoning effort 已实现并验证

## 来源

### 来源1: 代码改动

- `crates/ralph-core/src/config.rs`
- `crates/ralph-core/src/lib.rs`
- `crates/ralph-adapters/src/cli_backend.rs`
- `crates/ralph-adapters/src/lib.rs`
- `crates/ralph-cli/src/loop_runner.rs`
- `crates/ralph-cli/src/parallel_runner.rs`
- `crates/ralph-cli/src/autopilot.rs`
- `crates/ralph-cli/tests/integration_startup_resources.rs`
- `ralph.yml`
- `specs/ralph-prompt-role-layering.md`

### 来源2: 验证结果

- `cargo test -p ralph-core config::tests::test_cli_reasoning_effort_defaults_are_role_aware -- --exact`
- `cargo test -p ralph-core config::tests::test_parse_yaml_with_cli_reasoning_effort_overrides -- --exact`
- `cargo test -p ralph-core config::tests::test_parse_yaml_with_cli_reasoning_effort_partial_override -- --exact`
- `cargo test -p ralph-adapters codex_reasoning_defaults -- --nocapture`
- `cargo test -p ralph-cli --bin ralph parallel_runner::tests::finalize_output_for_parsing_keeps_text_backend_stdout_only -- --exact`
- `cargo test -p ralph-cli --bin ralph codex_app_server_session::tests::parse_codex_app_server_options_maps_full_auto_and_model -- --exact`
- `cargo test -p ralph-cli --test integration_startup_resources --quiet`
- `cargo test -p ralph-cli --test integration_capability --quiet`
- `cargo test --quiet`

## 综合发现

### 现象

- `cli.reasoning_effort` 现在成为语义配置层,默认值对 coordinator / worker 分离。
- 运行时只在最终 backend 是 Codex 时注入 `--config model_reasoning_effort=...`。
- hat-level / command-line 已显式给出的 reasoning config 不会被默认值覆盖。

### 当前结论

- `ralph.yml` 与无配置 bootstrap 已一致获得 `coordinator=medium`、`worker=high` 的默认语义。
- parallel runner 能按 `hat_id` 区分 Ralph coordinator 与 non-Ralph worker。
- Codex app-server parser 可以接住注入后的 `--config` 参数,所以 exec 与 app-server 两条路径都可用。

## [2026-05-18 11:47:10] [Session ID: omx-1779004640353-blcixq] 笔记: Architect 对 prompt role layering 计划的关键修正

## 来源

### 来源1: Architect 审阅反馈

- 反馈结论是 `ITERATE`。
- 关键补丁点是:
  - `all_hat_prompt` 必须视为 shared protocol surface,不能作为 coordinator-only 内容的后门。
  - `task-derived dynamic hat` 与 `runtime-autoscale` 必须分开 provenance。
  - `coordinator.no_event_first_turn` 必须进入 durable diagnostic,不能仅靠 trace。

## 综合发现

### 设计风险

- 如果只做 prompt contains/excludes 测试,而不建立 surface/provenance 真相源,后续新增 all-hat overlay 或 dynamic role 时,很容易再次污染 worker。

### 当前修正方向

- 先建小而薄的语义层:
  - `PromptAudience`
  - `PromptSurface`
  - `IdentitySource`
- 再用这些语义层驱动 prompt 渲染、artifact、测试断言和诊断输出。

## [2026-05-18 12:40:00] [Session ID: omx-1779004640353-blcixq] 笔记: prompt role layering 的实现收束

## 来源

### 来源1: `.omx/plans/ralph-prompt-role-layering-consensus-plan.md`

- 计划明确要求建立薄语义层,而不是只补字符串 contains/excludes。
- `all_hat_prompt` 必须只保留 shared protocol / universal safety,不能继续夹带 coordinator-only surface。
- `IdentitySource` 必须区分 `config-derived` / `template-derived` / `task-derived` / `runtime-autoscale`。
- `coordinator.no_event_first_turn` 必须是 durable diagnostic,不能只是 tracing。

### 来源2: 当前代码阅读

- `config/all_hat.md` 目前仍混有 coordinator-only 叙事,包括 Ralph coordinator、task 分发、topology、`ralph emit` 等内容。
- `prompt_overlay::load_all_hat_prompt()` 目前只是读取并注入,没有 surface 审计。
- `HatlessRalph` 和 `InstructionBuilder` 已经分开承担 coordinator / worker prompt,真正的问题在全局 overlay 污染。
- `ParallelSupervisor` 的 `JobCompleted` 已经提供了可落点,可以挂 first-turn diagnostic,但需要补 job_id / 输入 topic 这类上下文。

## 综合发现

### 实现策略

- 先把 `config/all_hat.md` 收敛成真正的 shared-only overlay,并在 `prompt_overlay` 加 shared-only 审计。
- 再补一个很薄的 `prompt_surface` 语义层,用来集中定义 coordinator-only / worker-only / shared-protocol marker,让测试不再散落硬编码字符串。
- `IdentitySource` 先落到 capability invocation artifact 和 agents snapshot,不要试图一次把所有 runtime 路径都重写成新的平台。
- first-turn diagnostic 直接写入 `events.jsonl`,这样既耐久又不必伪造业务 reply event。

## [2026-05-18 14:12:00] [Session ID: omx-1779004640353-blcixq] 笔记: prompt role layering 实现与 deslop 收束

## 来源

### 来源1: 本轮代码改动

- `config/all_hat.md`: 从 coordinator-heavy overlay 收敛为 shared-only protocol overlay。
- `crates/ralph-core/src/prompt_surface.rs`: 新增 prompt surface / audience / identity source / role contract 薄语义层。
- `crates/ralph-core/src/prompt_overlay.rs`: all-hat prompt 加载时执行 shared-only 审计。
- `crates/ralph-core/src/parallel/supervisor.rs`: first coordinator turn 无结构化 event 时写 durable diagnostic。
- `crates/ralph-core/src/parallel/instance.rs`: `JobCompleted` 携带 `job_id`。
- `crates/ralph-core/src/agents_snapshot.rs`: agents snapshot 写出 `identity_source`。
- `crates/ralph-cli/src/capability.rs`: hat capability invocation 写出 task-derived `role_contract`。
- `crates/ralph-cli/src/display.rs`: `ralph agents` 表格展示 Source。

### 来源2: 验证命令

- `cargo test -p ralph-core prompt_surface -- --nocapture`
- `cargo test -p ralph-core prompt_overlay -- --nocapture`
- `cargo test -p ralph-core ralph_prompt_contains_coordinator_only_sections -- --nocapture`
- `cargo test -p ralph-core worker_prompt_excludes_coordinator_only_sections -- --nocapture`
- `cargo test -p ralph-core autoscale_spawns_below_cap_and_stops_at_cap -- --nocapture`
- `cargo test -p ralph-core coordinator_no_event_first_turn_diagnostic_is_durable -- --nocapture`
- `cargo test -p ralph-core simple_task_dispatches_on_first_turn -- --nocapture`
- `cargo test -p ralph-cli --test integration_agents --quiet`
- `cargo test -p ralph-cli --test integration_capability --quiet`
- `cargo fmt --all -- --check && git diff --check`
- `cargo test --quiet`

## 综合发现

### 已验证结论

- `all_hat_prompt` 现在有 shared-only 审计,compiled / inline / file 三条输入路径都会拒绝 coordinator-only / worker-only heading。
- `ralph#1` prompt 保留 coordinator-only surface,包括 runtime capability catalog、topology、human reply policy 等。
- worker prompt 不再继承 coordinator-only surface,但仍保留 shared event protocol 和 all-hat shared overlay。
- `hat:focused-reviewer` 这类 capability micro-run invocation artifact 会记录 `role_contract.identity_source = task-derived`。
- autoscale 出来的 runtime 实例在 agents snapshot 中记录 `identity_source = runtime-autoscale`,不会和 task-derived 混用。
- 第一轮 `ralph#1` 若只输出 prose/thinking、没有任何结构化 event,运行时会写 `coordinator.no_event_first_turn` durable diagnostic。
- 第一轮如果已经输出 `reply.human.message` 等结构化 event,不会写 no-event diagnostic,也不会伪造业务 reply。

### AI slop cleanup report

Scope:
- 本轮 Ralph-owned 文件列表,未扩大到已有 TUI / reasoning effort / 记忆系统前序改动。

Behavior Lock:
- 先完成 focused tests 和全量 `cargo test --quiet`,再做 deslop 小修。

Fallback Findings:
- 发现本轮新增的 `input_topic` 用 `"<unknown>"` 静默默认值不够证据化。
- 分类: masking fallback risk。
- 处理: 改成 `input_topic: Option<String>` 序列化,并增加 `input_topic_missing` 字段,避免掩盖状态采集缺失。

Passes Completed:
- Fallback-like code resolution gate: 已修复。
- Dead code deletion: 移除了 capability role_contract helper 的未使用 `_input` 参数。
- Duplicate removal: 本轮未发现需要合并的重复逻辑。
- Naming/error handling cleanup: no-event diagnostic payload 字段更明确。
- Test reinforcement: 保留/新增 focused regression tests。

Quality Gates:
- Regression tests: PASS。
- Format/diff check: PASS。
- Full tests: PASS。

Remaining Risks:
- 真正的 workflow capability execute 模式和更完整的 task-derived dynamic hat 生成策略仍是后续工程,不能算本轮完全落地。

## [2026-05-18 15:06:00] [Session ID: omx-1779004640353-blcixq] 笔记: release/产品路径误入 dry-run 的只读审计

## 来源

### 来源1: 全局 dry-run 搜索

- 命令:
  - `rg -n --fixed-strings -- '--dry-run' crates/ralph-e2e crates/ralph-cli examples scripts specs config ralph.yml`
  - `rg -n "dry_run\s*:|dry_run\b" crates/ralph-cli/src crates/ralph-core/src crates/ralph-e2e/src -g'*.rs'`
  - `rg -n "command\s*=\s*Some\(\"true\"|command:\s*true|\btrue\b.*prompt_mode" crates config examples specs -g'*.rs' -g'*.yml' -g'*.md'`
- 结果:
  - runtime code 的 dry-run 入口只集中在 `crates/ralph-cli/src/main.rs`、`crates/ralph-cli/src/capability.rs`、`crates/ralph-cli/src/lib.rs`。
  - E2E executor / release checklist example 没有传 `--dry-run`。
  - `cli.command = Some("true")` 的 no-op stub 只出现在 `resolved_micro_run_config_for_capability()`。

### 来源2: `crates/ralph-cli/src/capability.rs`

- `child_run_mode_for_capability()` 当前为:
  - `WorkflowCapability => Execute`
  - `HatCapability => DryRun`
- `child_run_args()` 只有在 `CapabilityChildRunMode::DryRun` 时加入 `--dry-run` 和 `--prompt`。
- `resolved_micro_run_config_for_capability()` 对 hat capability 写入 `cli.command = Some("true")`,并禁用 runtime capabilities。

### 来源3: `crates/ralph-cli/src/main.rs`

- `RunArgs.dry_run` 是 CLI flag。
- 无 subcommand 时构造默认 `RunArgs` 明确设置 `dry_run: false`。
- `if args.dry_run { ... return Ok(()) }` 是显式配置预览分支。

### 来源4: E2E release / release checklist 路径

- `crates/ralph-e2e/src/executor.rs` 选择 `target/release/ralph` 或 `target/debug/ralph`,但实际命令构造为 `ralph run -c <config> --max-iterations <n>`。
- executor 只追加 scenario 的 `extra_args`,release checklist scenario 没有 `--dry-run`。
- `examples/parallel-release-checklist/ralph.yml` 里没有 CLI dry-run;`PROMPT.md` 里的 `migration dry-run: pass` 是业务 evidence 文本,不是 Ralph CLI flag。

## 综合发现

### 现象

- 仓库中出现了多个 `release` 字样,也出现了多个 `dry-run` 字样。
- 但把两者连到真实执行路径时,没有看到 release checklist / release binary selection 自动带入 `--dry-run`。

### 候选假设

- 主假设: 除 `hat:*` capability micro-run 仍硬编码 dry-run 外,没有另一个 release/实际产品运行误入 dry-run 的代码路径。
- 备选解释: startup bootstrap 或 E2E release binary selection 可能误导阅读,但代码证据显示它们分别是显式 bootstrap preview 和二进制选择,不是产品 dry-run。

### 已验证结论

- `workflow:*` capability 已走 Execute,不会在 child args 里加 `--dry-run`。
- `hat:*` capability 仍走 DryRun,这是当前唯一一个产品入口可触达的 dry-run child path。
- `run --dry-run`、`clean --dry-run`、startup bootstrap dry-run tests、behavior specs 都是显式 preview/test/inspection 路径,不是 release 正式运行路径。

## [2026-05-18 17:22:00] [Session ID: omx-1779004640353-blcixq] 笔记: `hat:*` execute 不应嵌套 Ralph loop

## 来源

### 来源1: focused integration 失败输出

- 命令:
  - `cargo test -p ralph-cli --test integration_capability tools_capability_invoke_hat_executes_by_default_and_preview_is_explicit -- --nocapture`
- 观察到的现象:
  - `result.stdout_summary` 出现 `ITERATION 1 | ? ralph`。
  - `result.stdout_summary` 出现 `I'm Ralph. Let's do this.`。
  - child exit code 是 2。

### 来源2: 保留 artifact 的最小复现

- 复现目录:
  - `/tmp/ralph-capability-execute-repro`
- 关键输出:
  - child backend 实际输出了 `focused reviewer executed real child path`。
  - child backend 实际输出了 `LOOP_COMPLETE`。
  - Ralph loop 随后记录 `Completion detected but requires consecutive confirmation - continuing`。
  - 最终因为 `max_iterations = 1` 触发 `max_iterations`,exit code 2。
- child prompt 证据:
  - 既包含 task-derived prompt: `Runtime hat capability invocation: hat:focused-reviewer`。
  - 也包含 Ralph coordinator prompt: `You are Ralph. You have fresh context each iteration.`。

## 综合发现

### 现象

- `hat:*` 默认 execute 路径已经不再是 dry-run,但旧实现仍用 `ralph run --config ... --no-tui` 嵌套启动。
- 因此 child 不是一个 transient worker,而是一个新的 Ralph coordinator loop。

### 假设与验证

- 主假设: `hat:*` execute 失败不是 backend 没跑,而是 child 被 Ralph loop 双确认机制拦住。
- 动态证据已验证:
  - backend stdout 出现预期短输出和 `LOOP_COMPLETE`。
  - loop 日志明确写出需要 consecutive confirmation。
  - max_iterations=1 导致 exit 2。
- 备选解释被排除:
  - resolved config 没继承 command: 已由 resolved config 看到 `cli.command` 是 custom backend。
  - backend 没收到 input: prompt 文件里存在 `review this patch`。

### 修复结论

- `hat:*` capability execute 应直接调用底层 CLI backend,而不是嵌套 `ralph run`。
- `workflow:*` capability 仍保留 isolated child `ralph run`,因为它代表一个完整 workflow。
- 这能同时解决:
  - 默认 execute 仍像 dry-run/no-op 的产品语义问题。
  - `hat:*` child 被 coordinator prompt 污染的问题。
  - 简单 worker 因 loop 双确认多耗一轮的问题。


## [2026-05-18 17:39:46] [Session ID: omx-1779004640353-blcixq] 笔记: `workflow:*` record-session dogfood 收敛证据

## 来源

### 来源1: direct child workflow dogfood

- 路径: `/tmp/ralph-workflow-child-dogfood-unified`
- 要点:
  - `run.stdout` 已出现 `build.task -> build.done -> confession.clean -> LOOP_COMPLETE`。
  - `.ralph/events.jsonl` 已记录 `build.task` 投递 `builder#1`, `build.done` 投递 `confessor#1`, `confession.clean` 投递 `confession_handler#1`。
  - `record summary child-session.jsonl` 的 Termination 为 `<missing>`。
  - 这证明 workflow child 真执行了,但没有自然退出。

### 来源2: 静态代码

- 文件: `crates/ralph-core/src/parallel/supervisor.rs`
- 要点:
  - completion 判断要求 `hat_id.as_str() == "ralph"`。
  - 非 Ralph worker 输出 `LOOP_COMPLETE` 只会成为普通 stdout,不会触发 `TerminationReason::CompletionPromise`。
- 文件: `ralph.yml`
- 要点:
  - 修复前默认 workflow 没有 `event_loop.complete_publishes`。
  - `confession.clean` 有具体订阅者 `confession_handler`,所以不会作为 orphan 自动交回 `ralph#1`。

### 来源3: capability execute dogfood

- 路径: `/tmp/ralph-workflow-capability-record-dogfood-final`
- 命令:
  - `target/debug/ralph tools capability invoke --id workflow:default-parallel --input "workflow dogfood backend probe" --json`
- 关键证据:
  - invocation id: `cap-1779097096972`
  - child record-session: `/tmp/ralph-workflow-capability-record-dogfood-final/.ralph/capability-invocations/cap-1779097096972/child-record-session.jsonl`
  - record summary Termination: `CompletionPromise`
  - Topics: `task.start`, `build.task`, `build.done`, `confession.clean`, `workflow.complete`
  - evidence index 包含 `record_session_jsonl` 指向 child record-session。

## 综合发现

### 现象

- workflow child 能真实启动并路由到 configured hats。
- 但修复前由 `confession_handler#1` 直接输出 `LOOP_COMPLETE`,不会让 run 终止。

### 主假设

- `workflow:default-parallel` 的完成语义不自洽: 完成 token 的唯一有效生产者是 `ralph#1`,但默认 workflow 没有让完成候选事件回到 `ralph#1`。

### 备选解释

- fake backend 协议不完整导致 app-server turn 没完成。
- 该解释被事件链和所有 worker idle 状态削弱: worker 已完成输出,缺的是 termination 元信息。

### 已验证结论

- 修复后 default workflow 使用 `complete_publishes: workflow.complete`。
- `confession_handler` 发布 `workflow.complete`,再由 `ralph#1` 输出 `LOOP_COMPLETE`。
- capability execute 自动传 `--record-session .../child-record-session.jsonl`,并把它登记为 `record_session_jsonl` evidence。
- focused tests 和全量 `cargo test --quiet` 均通过。

## [2026-05-18 19:06:00] [Session ID: omx-1779004640353-blcixq] 笔记: build.task 反馈后没有立即出现三个新实例

## 来源

### 来源1: `parallel_rec.jsonl` 快照

- 快照: `/tmp/parallel_rec_snapshot_20260518_185928.jsonl`
- 原文件仍被 `ralph run --record-session parallel_rec.jsonl` 的 PID 66646 持有。
- 快照 line count: 308。
- `parallel_rec.jsonl` 5 秒内没有增长: before=308, after=308。
- 顶层 `bus.publish` 只有 1 条,topic 是 `human.message`,id 是 `sBPsqrBV0oqU`。
- 终端文本里虽然出现了 `<event topic="build.task" reply="sBPsqrBV0oqU">...`,但它在 record 层不是新的 `bus.publish`。

### 来源2: `.ralph/events.jsonl`

- 最新相关事件:
  - line 544: `build.task`,id=`9by8YGtaM1rm`,reply=`sBPsqrBV0oqU`。
  - line 545: `capability.request`,请求 `workflow:default-parallel`。
  - line 546: `capability.request`,请求 `hat:focused-reviewer`。
  - line 547: `capability.invoke`,invocation=`cap-1779101957035`,`parent_topology_unchanged=true`。
  - line 548-551: 只创建了 `confessor#1`、`builder#1`、`confession_handler#1`、`ralph#1`,原因分别为 `configured` 或 `fallback`。
  - line 552: 只把 `task.start` 投递给 child `ralph#1`。
- `.ralph/agents.json` 最新快照只有 4 个 config-derived 实例: `builder#1`、`confessor#1`、`confession_handler#1`、`ralph#1`。

### 来源3: capability invocation 目录

- 路径: `.ralph/capability-invocations/cap-1779101957035/`
- 当前只有:
  - `invoke.json`
  - `resolved-config.yml`
  - `child-record-session.jsonl`
- 没有 `result.json` 和 `failed.json`,说明 parent 还没收到 child result/failure。
- `invoke.json` 明确写了 `parent_topology_unchanged=true`。
- `child-record-session.jsonl` 的 `ralph record summary` 显示:
  - Topics top 10 只有 `task.start: 1`。
  - Termination reason 是 `<missing>`。
  - child pid 是 53483,仍持有 child record 文件。

### 来源4: child stdout / stderr

- child `ralph#1` 曾输出示例或候选事件:
  - `<event topic="build.task" id="..." spawn_instance="3">... </event>`
  - `<event topic="build.task" spawn_instance="builder#1,builder#2,builder#3">...`
- 实际用于三实例的那段 event 没有闭合 `</event>`,后续 thinking 文本直接进入了同一段上下文。
- child 过程中还出现 Codex tool router error:
  - `Full-history forked agents inherit the parent agent type, model, and reasoning effort; omit agent_type, model, and reasoning_effort, or spawn without a full-history fork.`
- 这说明 child 还尝试过 tool-layer agent spawning,但失败且不等同于 Ralph runtime hat instance。

### 来源5: 代码契约

- `crates/ralph-core/src/event_reader.rs` 中 `spawn_instance` 是 `Option<bool>`。
- `crates/ralph-core/src/event_parser.rs` 的测试使用的是 `spawn_instance="true"`。
- `crates/ralph-core/src/parallel/supervisor/routing.rs` 的显式 spawn 逻辑要求:
  - `spawn_instance=true`
  - 同时提供 `target="<hat_id>"`
  - 不能同时提供 `target_instance`
  - target hat 必须订阅该 topic
- `ralph.yml` 里 `build.task` 订阅者是 `builder`,默认 static topology 只有 `builder#1`。

## 综合发现

### 现象

- 用户认为已经反馈了 `build.task` event,但 UI 没有立即看到三个新实例。
- 实际 record 里 parent 层只明确看到 `human.message` 进入 bus。
- `.ralph/events.jsonl` 里后续确实出现了 `build.task`,但它触发的是 `workflow:default-parallel` capability child run。
- child run 只启动了默认 workflow 的 static/config-derived 实例,没有出现 task-derived 的三个新视角实例。

### 主假设

- 这不是 UI 刷新延迟,而是事件语义不匹配:
  - `build.task` 在当前 `ralph.yml` 中是交给 `builder` 的任务事件。
  - 它不会按 payload 文字自动把"三个视角"解释成三个新 hat 身份。
  - `workflow:default-parallel` 是 isolated child run,并且 `invoke.json` 明确保留 `parent_topology_unchanged=true`。

### 备选解释

- 备选解释1: 子 workflow 已经生成实例,只是 UI 没刷新。
  - 被 `.ralph/agents.json` 和 lifecycle events 推弱: 最新快照只有 4 个 config-derived 实例。
- 备选解释2: `spawn_instance` 已经请求了三个实例,只是还没完成。
  - 被代码契约推弱: 当前 `spawn_instance` 是 boolean,不是数量或实例列表。
  - child 真实用于三实例的 event 没闭合,record summary 也没有 `build.task` topic。
- 备选解释3: Codex tool-layer subagent 已经创建了三个 agent。
  - 被 child stderr 推弱: tool router 报错,且这类 tool agent 不会体现在 Ralph `.ralph/agents.json` 中。

### 已验证结论

- `parallel_rec.jsonl` 当前没有证据显示 top-level Ralph runtime 立即创建了三个新 hat 实例。
- 当前实际发生的是:
  1. human message 进 parent `ralph#1`。
  2. parent 产出了 `build.task` 和 `capability.request`。
  3. `workflow:default-parallel` capability 启动 isolated child run。
  4. child run 创建默认 config-derived topology: `builder#1`、`confessor#1`、`confession_handler#1`、`ralph#1`。
  5. child `ralph#1` 还在思考/尝试生成三实例任务,但没有产出可解析、可路由、语义合法的三实例事件。
- 因此用户没有立即看到三个新实例是符合当前实现的,但不符合用户对"Ralph 根据任务实时创建三个任务派生 hat"的产品期望。

## 后续建议

- 若当前只想让它动起来,应发送三条合法 event,每条都使用当前协议支持的形态,例如 `target="builder" spawn_instance="true"`,并确保每条 event 完整闭合。
- 若要满足产品期望,需要设计 task-derived dynamic hat 的正式协议:
  - 允许 `ralph#1` 基于任务生成 role contract。
  - 将 role contract 纳入 `.ralph/agents.json` / lifecycle evidence。
  - 明确哪些字段能生成一个新 hat instance,哪些只是普通 payload。
  - 不再让模型猜 `spawn_instance="3"` 或 `spawn_instance="builder#1,builder#2,builder#3"` 这种当前不支持的语法。

## [2026-05-18 19:24:00] [Session ID: omx-1779004640353-blcixq] 笔记: OMX 技能 / hook 对 Ralph 运行的影响边界

## 来源

### 来源1: `parallel_rec.jsonl` 与 parent `.ralph/events.jsonl`

- parent record 显示 Ralph 顶层 runtime 仍按 `human.message -> ralph#1 -> build.task / capability.request` 路由。
- `.ralph/events.jsonl` 显示没有立即生成 3 个 task-derived hats,原因仍是 `build.task` / `spawn_instance` 协议语义不匹配。
- 这部分不是 OMX skill 直接改写 Ralph routing 造成的。

### 来源2: child record-session

- 文件: `.ralph/capability-invocations/cap-1779101957035/child-record-session.jsonl`
- 关键动态证据:
  - `hook: Stop`
  - `hook: Stop Blocked`
  - 后续文本出现: `我继续收尾这个 hook 指出的点...把 ultrawork 状态从 planning 推到已完成...`
  - 后续 tool command 真的执行了 `omx state read` / `omx state write`。
- 这说明 Codex/OMX hook 进入了 Ralph child hat 的 backend 会话,并影响了该 hat 的后续行为。

## 综合发现

### 现象

- Ralph child workflow 里本来应该由 `confession_handler#1` 处理 workflow 事件。
- 但 Stop hook 注入后,该 hat 转而处理 OMX `ultrawork` active/planning 状态,并写 `.omx/state/.../ultrawork-state.json`。

### 已验证结论

- OMX skill/hook 没有直接修改 Ralph binary 的路由选择和 instance creation 逻辑。
- 但 OMX hook 确实影响了 Ralph 调用的 Codex backend session:
  - 它能阻止 hat 的自然停止。
  - 它能把额外任务注入到 hat 的后续行为里。
  - 它能让 hat 执行 `omx state` 清理,从而污染/延长 child workflow 的业务轨迹。
- 因此,对"Ralph 运行"这个词要分层判断:
  - 控制面/runtime code 层: 主要没有被 OMX 改写。
  - agent backend 行为层: 明确受到了 OMX hook/skill 的影响。

### 风险

- 如果 Ralph 用的是 Codex CLI/app-server 作为 backend,而该 Codex 环境启用了 OMX hooks,那么每个 hat session 都可能继承这些 hook 行为。
- 这会让 record-session 里出现与 Ralph workflow 无关的 `omx state` 操作,并可能导致 Stop hook 循环、长 thinking、或者任务偏离。

### 后续建议

- 给 Ralph backend 增加一个可控隔离选项,例如运行 hat backend 时禁用 OMX hooks 或指定 clean Codex home。
- 至少在 record/session summary 里标识 backend hook activity,避免把 hook 注入误判成 Ralph workflow 自己的决策。

## [2026-05-18 22:24:12] [Session ID: omx-1779004640353-blcixq] 笔记: 只让 ralph 禁用 hooks 的可表达性

## 来源

### 来源1: `crates/ralph-core/src/config.rs`

- 网址: 本地仓库文件
- 要点:
  - `HatConfig` 已经有 `backend: Option<HatBackend>`.
  - `HatBackend` 支持 `Custom { command, args }`.
  - 说明普通 hats 的 backend 级别参数是能单独配的。

### 来源2: `crates/ralph-core/src/parallel/supervisor/routing.rs`

- 网址: 本地仓库文件
- 要点:
  - `spawn_instance()` 对 `hat_id == "ralph"` 走特例分支.
  - 这个分支直接构造 synthetic Ralph,并把 `hat_config` 传成 `None`.
  - 说明 fallback coordinator 不会读取 `hats.ralph.backend`.

### 来源3: `crates/ralph-core/src/parallel/instance.rs`

- 网址: 本地仓库文件
- 要点:
  - `HatInstanceHandle::spawn()` 会把 `hat_config.backend` 转成 `JobBackend::Hat`.
  - `maybe_start_job()` 只在 `hat_config` 存在时才会把 job backend 设成 hat-level backend.
  - 所以普通 hats 的 per-hat backend 会生效.

### 来源4: `crates/ralph-cli/src/parallel_runner.rs`

- 网址: 本地仓库文件
- 要点:
  - 并行 job 启动时会把 `RALPH_HAT_INSTANCE_ID` 和 `RALPH_HAT_ID` 注入子进程环境.
  - 这意味着如果想做“只对 ralph 生效”的外层包装器,环境信息是够的.

### 来源5: `crates/ralph-cli/src/codex_app_server_session.rs`

- 网址: 本地仓库文件
- 要点:
  - app-server 解析并转发 `-c/--config` 和 `-p/--profile`.
  - 说明只要 backend 路径真的拿到这些参数,就能对 Codex 会话级配置做定向覆写.

## 综合发现

### 现象

- 用户希望实现的是: Ralph coordinator 使用 `-c features.hooks=false`,其他 hats 仍正常带 hooks.

### 候选结论

- 当前代码里,普通 hats 已经支持 per-hat backend/args,所以这件事对“非 ralph hats”不是问题.
- 但是 `ralph#1` 是 fallback 特例,当前不走 `hats.ralph.backend`,因此“只给 Ralph 禁 hooks”还不能靠现有 `hats` 配置直接表达.
- 如果不改代码,最轻的外部手段是用一个感知 `RALPH_HAT_ID` 的 wrapper command,在 `ralph` 时附加 `-c features.hooks=false`,其他实例保持原样.

### 风险

- 如果直接把 `-c features.hooks=false` 写进全局 `cli.args`,会影响所有 hats,违背用户目标.
- 如果试图在配置里新增 `hats.ralph.backend`,当前 fallback 特例仍可能绕过它,会造成误以为已生效.

## [2026-05-18 23:03:00] [Session ID: omx-1779004640353-blcixq] 笔记: coordinator-only role_args 落地验证

## 来源

### 来源1: 当前代码与配置

- 文件:
  - `crates/ralph-core/src/config.rs`
  - `crates/ralph-adapters/src/cli_backend.rs`
  - `crates/ralph-cli/src/parallel_runner.rs`
  - `crates/ralph-cli/src/loop_runner.rs`
  - `crates/ralph-cli/src/capability.rs`
  - `crates/ralph-cli/src/autopilot.rs`
  - `crates/ralph-core/src/lib.rs`
  - `ralph.yml`
- 要点:
  - 新增 `RoleArgsConfig`,并挂到 `CliConfig.role_args`。
  - `CliBackend::apply_role_args` 只按角色追加 argv,不解释参数语义。
  - parallel executor 用 `job.hat_id == "ralph"` 识别 coordinator,其他 jobs 是 worker。
  - serial loop 用 `display_hat == "ralph"` 识别 coordinator。
  - `hat:*` direct capability path 明确按 worker 角色执行。
  - `autopilot` analysis child config 会保留 `cli.role_args`,避免子配置丢失 coordinator-only 约束。

### 来源2: 验证命令

- 命令:
  - `cargo test -p ralph-core cli_role_args -- --nocapture`
  - `cargo test -p ralph-adapters role_args -- --nocapture`
  - `cargo test -p ralph-cli parallel_runner::tests::parallel_role_backend_overlays_apply_coordinator_hooks_only -- --exact --nocapture`
  - `cargo test -p ralph-cli autopilot::tests::analysis_config_preserves_cli_role_args -- --exact --nocapture`
  - `cargo fmt`
  - `git diff --check`
  - `cargo test --quiet`
- 结果:
  - focused tests 通过。
  - `git diff --check` 输出 `PASS git diff --check`。
  - `cargo test --quiet` exit code 0,全量测试通过。

## 综合发现

### 行为结论

- `ralph.yml` 现在可以写成:
  - `cli.reasoning_effort.coordinator = "medium"`
  - `cli.reasoning_effort.worker = "high"`
  - `cli.role_args.coordinator = ["-c", "features.hooks=false"]`
  - `cli.role_args.worker = []`
- coordinator 的 Codex 进程会获得 hooks disabled 参数。
- worker hats 不会继承 `features.hooks=false`,因此仍按正常 Codex hooks 行为运行。

### 风险边界

- 当前 coordinator 判定依赖 Ralph 内部约定: `hat_id == "ralph"` 或 `display_hat == "ralph"`。
- 未来如果支持多个 coordinator id 或更复杂 coordinator 拓扑,应升级为显式 role metadata,不要继续散落字符串判断。


## [2026-05-19 09:11:12] [Session ID: omx-1779004640353-blcixq] 笔记: 当前 parallel_rec 未生成三个 instance 的证据闭环

## 来源

### 来源1: `parallel_rec.jsonl`

- 文件: `/Users/cuiluming/local_doc/l_dev/my/rust/ralph-orchestrator/parallel_rec.jsonl`
- 命令: `ralph record summary parallel_rec.jsonl`
- 要点:
  - parent run argv 是 `ralph run --record-session parallel_rec.jsonl`。
  - parent UX mode 是 `parallel-tui`。
  - parent record-session 只统计到 `human.message: 1`。
  - `capability.request` 在 parent record 中是 `ux.terminal.write` 文本,不是 parent record 里的 `bus.publish` topic。

### 来源2: `.ralph/events.jsonl`

- 命令: `nl -ba .ralph/events.jsonl | sed -n '638,662p'`
- 要点:
  - line 644: `ralph#1` 确实发布了 `capability.request`。
  - line 645: runtime 发布 `capability.invoke`, capability id 为 `workflow:default-parallel`, mode 为 `isolated_child_run`,并且 `parent_topology_unchanged=true`。
  - line 646-649: child run topology 只创建了 `confessor#1`, `confession_handler#1`, `builder#1`, `ralph#1`。
  - line 657-659: child coordinator 发布 `build.task`,最终 delivery 到 `builder#1`,mode 为 `fanout`。

### 来源3: `.ralph/capability-invocations/cap-1779152487480`

- 文件:
  - `.ralph/capability-invocations/cap-1779152487480/invoke.json`
  - `.ralph/capability-invocations/cap-1779152487480/resolved-config.yml`
  - `.ralph/capability-invocations/cap-1779152487480/child-record-session.jsonl`
- 要点:
  - `invoke.json` 明确 `invocation_mode=isolated_child_run` 与 `parent_topology_unchanged=true`。
  - `resolved-config.yml` line 174 显示 `builder.instances: 1`。
  - `resolved-config.yml` line 187 显示 `parallel.topic_contracts: {}`。
  - child record line 250 的 `bus.publish` 包含 `audience_override.instances = [builder#功能补充, builder#功能完善, builder#review]` 和 `require_delivery=true`,但这些只是 event audience override,不是动态建实例协议。

### 来源4: 源码静态证据

- `crates/ralph-core/src/capability.rs:1-5`:
  - capability v1 是隔离执行,不能热改父 run topology。
- `crates/ralph-core/src/capability.rs:36-42`:
  - parent capability catalog 明确 runtime 会以 isolated child 或 micro-run 执行,并要求不要 mutate parent topology。
- `crates/ralph-cli/src/capability.rs:740-780`:
  - isolated invocation 写入 `invoke.json`,并将 `parent_topology_unchanged` 固定为 true。
- `crates/ralph-core/src/event_parser.rs:197-214`:
  - `audience_instances` 解析为 `AudienceOverride`,只是 routing audience override。
- `crates/ralph-core/src/event_parser.rs:232-239`:
  - `spawn_instance` 只接受 true/1/yes,不是数量或实例列表。
- `crates/ralph-core/src/parallel/supervisor/routing.rs:220-292`:
  - 真正显式动态实例创建路径要求 `spawn_instance=true` 并有 `target=<hat_id>`。
- `crates/ralph-core/src/parallel/supervisor/routing.rs:509-592`:
  - 当没有 TopicContract 时走 trigger fallback;明确 target=builder 时只收敛到 builder hat,然后从已有实例中选一个实例,当前就是 `builder#1`。
- `crates/ralph-core/src/parallel/supervisor/routing_tests.rs:1560-1624`:
  - 回归测试证明合法动态创建是 `with_target(writer)` + `with_spawn_instance(true)`,结果创建 `writer#2`。

## 综合发现

### 现象

- 用户看到 `capability.request` 已经输出,但父级 TUI 没有新增三个 instance。
- `.ralph/agents.json` 当前只有 `builder#1`, `confession_handler#1`, `confessor#1`, `ralph#1`,且都是 `is_dynamic=false`。
- `builder#1` 的 last_input 中包含三个“视角”的 JSON payload,说明三个视角被塞进了一个 builder 任务,而不是 runtime materialized 为三个 HatInstance。

### 当前已验证结论

- `workflow:default-parallel` 已被 runtime 消费,不是没收到。
- 该 capability 的执行方式是 isolated child run,父 topology 明确不变。
- child run 的 resolved config 只定义 `builder.instances: 1`。
- child 输出的 `audience_instances` 被解析成 audience override,但它不是 create/spawn 实例协议。
- 因为 child config 没有 `parallel.topic_contracts`,`build.task` 走 trigger fallback 路由;fallback 路由当前不会根据 `audience_override` 物化新实例,而是对 target hat 选择已有实例,最终交给 `builder#1`。

### 主假设与备选解释状态

- 原主假设“capability 没被消费”已被 `.ralph/events.jsonl:644-645` 推翻。
- 新结论是: capability 被消费了,但当前 contract 是隔离 child workflow,不是 parent-visible 动态创建三个实例。
- 最强备选解释“已有 dynamic spawn 机制但这条事件没触发”也被源码证实: dynamic spawn 机制存在,但必须用 `spawn_instance=true` + `target=<hat_id>`,本次事件没有这个字段。

### 风险

- prompt 只列出 `audience_instances` 和 `spawn_instance`,但没有足够清楚地区分:
  - `audience_instances`: 限制投递受众。
  - `spawn_instance`: 显式新建一个动态实例。
  - task-derived 三角色: 当前还不是正式协议。
- 这会诱导 coordinator 生成看起来符合人类预期、但 runtime 不会 materialize 的事件。

## [2026-05-19 10:12:52] [Session ID: omx-1779004640353-blcixq] 更正: 上一条方案笔记中有一部分被 shell 命令替换污染

### 更正内容
- 上一条新增笔记里,本来应该明确写出的关键句是:
  - capability.request 当前契约明确是 isolated child/micro-run,不能 mutate parent topology。
  - 真实动态实例创建路径存在,但它只作用于普通 event routing,不是 capability.request。
  - ParallelTuiState 当前只有真实 instances / gates / evidence_paths,没有 child run observable state。
- 由于追加时的 heredoc 问题,这几句被破坏了,这里补回正确意思。

### 结论不变
- 这次分析的结论仍然是:要把问题拆成两条线。
  1. parent-visible dynamic spawn: 需要新的明确协议或 capability mode,专门负责真的创建 hat instance。
  2. parent-observable child run: 需要新的 UI 投影状态,只负责展示 child run 的运行态与证据,不能伪装成真实 instance。

## [2026-05-19 10:30:31] [Session ID: omx-1779004640353-blcixq] 笔记: 已生成 spec 与 Ralph implementation plan

### 来源
- 
- 

### 要点
- spec 把三条语义明确分开了:
  - parent-visible dynamic spawn.
  - parent-observable child run.
  - 现有  单事件显式 spawn.
- spec 明确要求  是 runtime-shaped payload,不是静态配置。
- plan 把实现拆成 7 个步骤,覆盖:
  - core protocol.
  - Supervisor 真实 spawn.
  - agents snapshot 角色元数据.
  - TUI child-run state.
  - footer/instances/output status.
  - coordinator prompt guardrails.
  - recorded-session 验证.

### 验证
-  已成功验证 spec 中 2 个 Mermaid block。
-  通过。

### 结论
- 后续实现可以直接以这两个文件为单一入口。


## [2026-05-19 10:31:57] [Session ID: omx-1779004640353-blcixq] 更正: spec / Ralph plan 记录中的 shell substitution 污染

### 更正原因
- 上一条 notes/worklog 追加时再次误用了未加引号 heredoc。
- 反引号内容被 shell 当作命令执行,导致文件路径和命令名在记录中被吃掉。
- 这不影响已经创建好的 spec 与 plan 文件本身,只影响上下文记录的可读性。

### 正确产物
- Spec: `specs/parent-visible-topology-spawn-observability.spec.md`
- Ralph implementation plan: `docs/plans/2026-05-19-parent-visible-topology-spawn-and-child-run-observability.md`

### 正确验证结论
- `beautiful-mermaid-rs --ascii` 成功验证 spec 中 2 个 Mermaid block。
- `git diff --check` 通过。

### 正确设计要点
- `topology.spawn_group` 是 parent-visible dynamic spawn 的 runtime-shaped payload。
- `capability.request` / `workflow:*` 继续保持 isolated child run,但要 parent-observable。
- `spawn_instance=true + target=<hat_id>` 继续表示现有单事件显式 spawn。


## [2026-05-19 10:42:39] [Session ID: omx-1779158263949-kticiv] 笔记: 用户确认 open questions 的产品决策

### 用户决策
- `topology.spawn_group` 不需要原子成功,允许部分成功,但失败必须显式结构化。
- child-run 状态最好也显示在 `ralph agents` 中,但仍不能伪装成真实实例。
- 临时角色默认不写入 `.ralph/agents.json` 作为一等字段;如果 LLM coordinator 判断可以作为固定角色,并显式标记,则可以持久化为固定角色 metadata。

### 已同步文件
- `specs/parent-visible-topology-spawn-observability.spec.md`
- `docs/plans/2026-05-19-parent-visible-topology-spawn-and-child-run-observability.md`

## [2026-05-19 12:43:00] [Session ID: omx-1779158263949-kticiv] 笔记: parent-visible spawn 与 child-run projection 实现结论

## 来源

### 来源1: 本次实现与 focused tests

- 文件:
  - `crates/ralph-core/src/topology_spawn.rs`
  - `crates/ralph-core/src/parallel/supervisor/topology_runtime.rs`
  - `crates/ralph-core/src/parallel/supervisor/capability_runtime.rs`
  - `crates/ralph-tui/src/state/parallel.rs`
  - `crates/ralph-tui/src/widgets/footer.rs`
  - `crates/ralph-tui/src/widgets/instances.rs`
  - `crates/ralph-tui/src/widgets/parallel_output.rs`
  - `crates/ralph-cli/src/parallel_runner.rs`
  - `crates/ralph-cli/src/display.rs`
- 要点:
  - `topology.spawn_group` 是真实 parent topology mutation,会走 `spawn_dynamic_instance` 和 direct delivery。
  - `capability.request` 仍然保持 parent topology unchanged,但会生成 child-run projection。
  - 临时 role label 只进入 TUI live state;只有 `fixed_role=true` 才进入 agents snapshot 的 fixed-role metadata。
  - `ralph agents` 显示 child-run summary,但 child-run 不进入 instance rows。

## 综合发现

### 协议边界

- 真实父级实例创建和 isolated child-run 必须是两条不同协议。
- `parent_topology_unchanged` 只能作为结果证据,不能当作“改配置让 capability 变成真实例”的开关。
- 如果用户明确要“父级 TUI 新增 hat instance”,coordinator 应发 `topology.spawn_group`,不是 `workflow:*` capability。

### 验证证据

- Focused tests 覆盖了:
  - 三实例 dynamic spawn。
  - `request_id` 幂等。
  - 非 coordinator 请求拒绝。
  - child-run running/done/failed projection。
  - TUI footer/output/instances 展示。
  - `ralph agents` child-run summary。
  - coordinator prompt guardrails。
- `cargo test --quiet` 全量通过。

## [2026-05-20 00:04:53] [Session ID: omx-1779158263949-kticiv] 笔记: topology.spawn.result 重复派发的静态证据

## 来源

### 来源1: `crates/ralph-core/src/parallel/supervisor.rs`

- 代码位置: `build_ralph_coordinator_instructions()` 的 `## WHAT TO DO` 段。
- 已观察到的事实:
  - 当前 coordinator prompt 只显式处理 `task.start` 和 completion candidate。
  - 其他事件统一落入 `### If you receive any other event`。
  - 该兜底规则要求 coordinator "Decide which hat should handle it next and emit ONE event to delegate"。

## 综合发现

### 现象

- live dogfood 中 `topology.spawn_group` 已经创建 3 个动态实例,并完成 direct delivery。
- coordinator 收到 `topology.spawn.result` 后仍二次发 `analysis.task`,导致配置实例也参与工作。

### 候选假设

- `topology.spawn.result` 缺少专门语义,被 `any other event` 规则误导为需要继续 delegate 的普通事件。

### 最强备选解释

- `audience_instances` fallback 路由会放大影响,但它不是最早触发重复派发的原因。
- 如果 coordinator 不二次发 `analysis.task`,该 fallback 路由就不会参与这次失败路径。

### 修复边界

- 先补 coordinator-only prompt guardrail。
- 不改 runtime delivery 路径,因为已有 durable evidence 证明首次 direct delivery 正常。

## [2026-05-20 00:22:10] [Session ID: omx-1779158263949-kticiv] 笔记: topology.spawn.result guardrail 二次 dogfood 证据

## 来源

### 来源1: focused tests

- 命令:
  - `cargo test -p ralph-core --lib event_emission_protocol::tests::topology_spawn_prompt_documents_parent_visible_group_spawn_contract -- --exact --nocapture`
  - `cargo test -p ralph-core --lib capability::tests::parent_capability_catalog_renderer_includes_request_contract_and_bounded_metadata -- --exact --nocapture`
  - `cargo test -p ralph-core --lib parallel::supervisor::routing_tests::runtime_capability_catalog_is_injected_only_into_ralph_prompt -- --exact --nocapture`
  - `cargo test -p ralph-core --lib parallel::supervisor::routing_tests::topology_spawn_group_creates_three_dynamic_instances_and_delivers_direct -- --exact --nocapture`
  - topology spawn typed / idempotent / non-ralph exact tests。
- 结果: 全部通过。

### 来源2: full regression gate

- 命令: `git diff --check && cargo test --quiet`。
- 结果: 通过。

### 来源3: live dogfood record-session

- 命令: `target/debug/ralph run -c /tmp/ralph-topology-dogfood-guardrail.yml --no-tui --record-session /tmp/ralph-topology-dogfood-guardrail-record.jsonl -p <prompt>`。
- record-session: `/tmp/ralph-topology-dogfood-guardrail-record.jsonl`。
- stdout: `/tmp/ralph-topology-dogfood-guardrail.stdout`。
- stderr: `/tmp/ralph-topology-dogfood-guardrail.stderr`。
- `ralph record summary`:
  - termination: `MaxRuntime`。
  - topics: `analysis.task: 3`, `task.start: 1`, `topology.spawn.result: 1`, `topology.spawn_group: 1`。
- 手工脚本解析 bus.publish 顺序后确认:
  - `topology.spawn.result` 之后 `analysis_task_after_spawn_result=0`。

## 综合发现

### 已验证结论

- `topology.spawn.result` 现在会让 coordinator 用普通文本说明“已收到结果,等待 worker”,没有继续发 `analysis.task`。
- 本轮没有复现上一轮“spawn result 后二次 fanout 到配置实例”的问题。

### 独立问题

- 本轮 run 仍是 `MaxRuntime`,原因是 worker 侧没有稳定完成 `analysis.done`。
- 这属于 dogfood worker 收敛/失败处理问题,不是本轮重复派发修复的反证。
