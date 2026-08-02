## [2026-05-17 23:18:00] [Session ID: omx-1779004640353-blcixq] 错误修复: 中文 prompt 触发 UTF-8 byte slice panic

### 现象
- `parallel_rec.jsonl` 中的 `capability.request` 对应 child run 没有产出 result,而是返回 `capability.failed`。
- `failed.json` 里的错误是 `byte index 60 is not a char boundary`。
- panic 位置在 `crates/ralph-cli/src/main.rs:1256:41`。

### 原因
- `main.rs` 的 dry-run prompt preview 使用了 `&inline[..60]` 这种按 byte 切片。
- 输入里包含中文字符,导致切片落在 UTF-8 字符中间,直接 panic。
- 同类问题还存在于 `loop_runner.rs` 的 debug preview 和 `memory.rs` 的预览/预算裁剪中。

### 修复
- 新增共享安全 helper:
  - `display::byte_index_after_chars`
  - `display::preview_one_line`
- `main.rs` dry-run prompt preview 改为 UTF-8 安全预览。
- `loop_runner.rs` 的 inline prompt debug preview 改为安全预览。
- `memory.rs` 的 content preview、`truncate_str`、`truncate_to_budget` 改为安全边界计算。
- 新增中文/emoji 回归测试,覆盖 preview 和 budget truncation 路径。

### 验证
- `cargo test -p ralph-cli --bin ralph display::tests::test_preview_one_line_is_utf8_safe_and_removes_newlines -- --exact` : passed
- `cargo test -p ralph-cli --bin ralph display::tests::test_byte_index_after_chars_returns_valid_utf8_boundary -- --exact` : passed
- `cargo test -p ralph-cli --bin ralph memory::tests::truncate_str_does_not_panic_on_multibyte_boundary -- --exact` : passed
- `cargo test -p ralph-cli --bin ralph memory::tests::truncate_to_budget_does_not_panic_on_multibyte_boundary -- --exact` : passed
- `cargo run -p ralph-cli --bin ralph -- run --config .ralph/capability-invocations/cap-1779029514701/resolved-config.yml --dry-run --no-tui --prompt <中文原文>` : passed,不再 panic
- `cargo fmt --all -- --check` : passed
- `cargo test -p ralph-cli --bin ralph` : passed
- `cargo test --quiet` : passed

### 结论
- 这类“持续思考但没有结果”的现象,在这次 record 里不是思考逻辑本身卡住,而是 child run 在输出预览阶段先崩了。
- 目前最关键的回归点已经补上。

## [2026-05-18 00:10:30] [Session ID: omx-1779004640353-blcixq] 错误修复: workflow capability resolved config 误用空 stub

### 现象
- `workflow:default-parallel` capability 暴露在 catalog 中。
- 但 invocation 写出的 `.ralph/capability-invocations/<id>/resolved-config.yml` 仍是 `hats: {}` / `parallel.enabled: false`。
- 因此即使 UTF-8 panic 已修,该 capability 也不会 materialize 出 default-parallel 的三个 hats。

### 原因
- `invoke_isolated_with_runner()` 写 artifact 前调用 `resolved_config_for_capability()`。
- 该函数无论 capability kind 是 workflow 还是 hat,都从 `RalphConfig::default()` 手搓一个 child dry-run stub。
- `capability_catalog()` 只把 startup resource 暴露成 metadata,但旧实现没有回到 startup resource 的 workflow YAML 内容。

### 修复
- `resolved_config_for_capability()` 改为返回 `Result<RalphConfig>`。
- workflow capability 分支调用 `startup_resources::resolve_workflow_capability_config()`。
- `resolve_workflow_capability_config()` 从 `startup_resources::embedded_catalog()` 找对应 workflow preset,解析真实 YAML,再把 parent 传入的 input 注入为 inline prompt。
- hat capability 分支继续走 `resolved_micro_run_config_for_capability()` stub,保持 micro-run contract 不漂移。

### 验证
- 新增回归测试 `tools_capability_invoke_materializes_default_parallel_workflow_config`。
- 修复前该测试失败,失败点为 `parallel.enabled=false`。
- 修复后该测试通过。
- `cargo test -p ralph-cli --test integration_capability` : passed。
- `cargo test -p ralph-cli --bin ralph startup_resources::tests` : passed。
- `cargo fmt --all -- --check` : passed。
- `git diff --check` : passed。
- `cargo test --quiet` : passed。

### 结论
- 已修复的是 workflow capability 的 resolved config materialization 层。
- 尚未修复的是 child capability 从 dry-run 变成真实 execute、以及 execute 后的递归 capability guard。

## [2026-05-18 11:30:57] [Session ID: omx-1779004640353-blcixq] 错误修复: 新增 reasoning 语义后未先导出 crate root 类型

### 现象
- 在给 `ralph-adapters` 和 `ralph-cli` 增加 reasoning role helper 后,首次跑 focused test 时编译失败。
- 报错是 `unresolved imports ralph_core::ReasoningEffort, ralph_core::RoleReasoningEffortConfig`。
- 随后在 `ralph-cli` 编译时又报 `unresolved import ralph_adapters::CliExecutionRole`。

### 原因
- 新增的配置/角色类型只定义在模块里,但没有从 crate root 重新导出。
- 下游 crate 按既有风格通过 `ralph_core::...` / `ralph_adapters::...` 引用,因此会直接编译失败。

### 修复
- 在 `crates/ralph-core/src/lib.rs` 重新导出 `ReasoningEffort` 和 `RoleReasoningEffortConfig`。
- 在 `crates/ralph-adapters/src/lib.rs` 重新导出 `CliExecutionRole`。

### 验证
- `cargo test -p ralph-adapters codex_reasoning_defaults -- --nocapture` 通过。
- `cargo test -p ralph-cli --bin ralph parallel_runner::tests::finalize_output_for_parsing_keeps_text_backend_stdout_only -- --exact` 通过。
- `cargo test -p ralph-cli --bin ralph codex_app_server_session::tests::parse_codex_app_server_options_maps_full_auto_and_model -- --exact` 通过。
- 最终 `cargo test --quiet` 全量通过。

## [2026-05-18 14:12:00] [Session ID: omx-1779004640353-blcixq] 错误修复: Ralph prompt 污染与首轮无 event 不可观测

### 问题
- `config/all_hat.md` 会注入所有 hat prompt,但原内容包含大量 coordinator-only 操作策略,例如 topology、并行度、`ralph emit`、task 分发等。
- worker 因此可能继承 Ralph coordinator 职责,导致简单任务也持续进入 heavy coordination / thinking,而不是第一轮就发 event。
- task-derived capability micro-run 和 runtime-autoscale dynamic instance 缺少明确 provenance,容易被 `is_dynamic` 混成一类。
- `ralph#1` 第一轮如果没有输出结构化 event,之前没有 durable diagnostic,只能从 UI 现象猜。

### 原因
- all-hat overlay 没有 shared-only surface 审计,成为 prompt 污染后门。
- 运行时 artifact 没有 `IdentitySource` / `RoleContract` 这种可审计语义字段。
- `JobCompleted` 没有 `job_id`,Supervisor 缺少写 first-turn no-event 诊断所需上下文。

### 修复
- 新增 prompt surface 薄语义层,集中定义 coordinator-only / worker-only / shared-protocol heading。
- `load_all_hat_prompt` 对 compiled / inline / file 内容做 shared-only audit,发现 forbidden heading 直接报错。
- 将 `config/all_hat.md` 收敛为 shared ontology、runtime identity、event envelope、reply semantics、control topic、文件上下文位置和 worker boundary。
- agents snapshot 增加 `identity_source`,runtime autoscale 标记为 `runtime-autoscale`。
- capability invocation 增加 `role_contract`,hat capability 标记为 `task-derived`。
- `JobCompleted` 增加 `job_id`,Supervisor 在 `ralph#1` 第一轮无 parsed event 时写 `coordinator.no_event_first_turn`。
- diagnostic payload 包含 `event_id`、`instance_id`、`job_id`、`input_topic`、`input_topic_missing`、`output_len`、`parsed_event_count`、`reason`。

### 验证
- focused prompt surface / overlay / prompt regression / provenance / diagnostic tests 均通过。
- `cargo fmt --all -- --check && git diff --check` passed。
- `cargo test --quiet` passed。

### 防复发提醒
- 以后新增全局 prompt overlay 内容时,必须先判断它是不是 shared protocol。
- 不要再把 “dynamic” 当成一个泛泛来源;task-derived role 和 runtime-autoscale instance 必须分开记录。
- 对 simple-task fast path,如果第一轮没有 event,必须留下 durable diagnostic,不能只靠 UI 状态或模型 thinking 文本判断。

## [2026-05-18 17:23:00] [Session ID: omx-1779004640353-blcixq] 错误修复: `hat:*` capability execute 嵌套 Ralph loop 导致失败和 prompt 污染

### 现象
- 用户选择方案 B 后,`hat:*` capability 默认 execute 的 focused integration 初次失败。
- `result.stdout_summary` 出现 `ITERATION 1 | ? ralph` 和 `I'm Ralph. Let's do this.`。
- child exit code 为 2,`capability.failed` 的 error 为空。
- 手工复现显示 backend 实际输出了 `focused reviewer executed real child path` 和 `LOOP_COMPLETE`,但 loop 仍因 `max_iterations` 失败。

### 原因
- 默认 execute 虽然不再显式 `--dry-run`,但 `hat:*` 仍通过 `ralph run --config ... --no-tui` 执行。
- 这会把 task-derived reviewer 变成新的 Ralph coordinator loop。
- child prompt 同时包含 task-derived capability prompt 和 Ralph coordinator prompt。
- 现有 event loop 需要两次 consecutive `LOOP_COMPLETE` 才算自然完成,而 micro-run config 设为 `max_iterations = 1`,所以 backend 第一轮已经完成也会被判成 iteration limit failure。

### 修复
- `CapabilityInvokeArgs` 增加显式 `--preview`。
- 默认 `hat:*` 和 `workflow:*` 都通过 `child_run_mode_for_capability()` 选择 `Execute`。
- `--preview` 才使用旧 `DryRun`。
- `hat:*` execute 改为直接调用 resolved config 中的底层 CLI backend:
  - 使用 task-derived prompt。
  - 设置 `RALPH_CAPABILITY_CHILD=1`。
  - 设置 `RALPH_CAPABILITY_MODE=execute`。
  - 应用 worker reasoning 默认值。
  - 不再嵌套 `ralph run`。
- `workflow:*` execute 保留 isolated child `ralph run`,因为它代表完整 workflow。
- 新增/调整测试覆盖:
  - 默认 `hat:*` execute 捕获真实 child backend stdout。
  - child prompt 包含 `Runtime hat capability invocation`。
  - child prompt 不包含 `You are Ralph.`。
  - 旧 artifact/inspect/materialization 测试显式传 `--preview`。

### 验证
- `cargo test -p ralph-cli --bin ralph hat_capability_defaults_to_execute_mode -- --nocapture`: passed。
- `cargo test -p ralph-cli --bin ralph resolved_micro_run_inherits_backend_and_disables_recursion -- --nocapture`: passed。
- `cargo test -p ralph-cli --test integration_capability tools_capability_invoke_hat_executes_by_default_and_preview_is_explicit -- --nocapture`: passed。
- `cargo test -p ralph-cli --test integration_capability -- --nocapture`: passed,7 tests。
- `cargo test -p ralph-cli --test integration_live_capability -- --nocapture`: passed,5 tests。
- `cargo test -p ralph-cli --bin ralph capability::tests -- --nocapture`: passed,10 tests。
- `cargo fmt --all -- --check`: passed。
- `git diff --check`: passed。
- `cargo test --quiet`: passed。

### 结论
- `hat:*` capability 的默认产品路径已经不再是 dry-run preview。
- `hat:*` execute 现在是 task-derived worker backend execution,不会再继承 Ralph coordinator prompt。
- 显式 inspect/debug 的旧 dry-run preview 仍通过 `--preview` 保留。


## [2026-05-18 17:39:46] [Session ID: omx-1779004640353-blcixq] 错误修复: `workflow:*` child run 完成但无 record-session termination

### 问题
- `workflow:default-parallel` child run 能输出 `build.task -> build.done -> confession.clean -> LOOP_COMPLETE`,但进程不自然退出。
- `ralph record summary child-session.jsonl` 显示 Termination 为 `<missing>`。
- `tools capability invoke --id workflow:default-parallel` 之前也没有把 child record-session 作为 invocation evidence 保留下来。

### 原因
- parallel supervisor 只把 `ralph` hat 的输出中的 `event_loop.completion_promise` 视为硬终止信号。
- 默认 workflow 让 `confession_handler` 直接输出 `LOOP_COMPLETE`,违反了“只有 `ralph#1` 终止 run”的单一真相源。
- 默认 workflow 没有 `complete_publishes`,导致完成候选不会回到 `ralph#1` 做收敛判断。
- workflow capability execute 没有自动添加 `--record-session`,所以 invocation artifact 缺少 child run 主证据流。

### 修复
- `ralph.yml` 增加 `event_loop.complete_publishes: "workflow.complete"`。
- `confession_handler` 的 `publishes` 增加 `workflow.complete`。
- `confession_handler` clean path 改为发布 `workflow.complete`,不再自行输出 completion promise。
- `workflow:*` execute child run 增加 `--record-session <invocation_dir>/child-record-session.jsonl`。
- 如果 child record-session 存在,capability evidence index 记录 `record_session_jsonl`。
- specs/docs 同步说明 workflow capability child record-session artifact。

### 验证
- `cargo test -p ralph-cli --bin ralph capability::tests -- --nocapture`: passed。
- `cargo test -p ralph-cli --test integration_capability -- --nocapture`: passed,8 tests。
- `cargo test -p ralph-cli --test integration_startup_resources -- --nocapture`: passed,3 tests。
- `cargo test -p ralph-cli --test integration_live_capability -- --nocapture`: passed,5 tests。
- `git diff --check`: passed。
- `cargo test --quiet`: passed。
- 手工 dogfood 路径 `/tmp/ralph-workflow-capability-record-dogfood-final`:
  - exit code 0。
  - record summary Termination 为 `CompletionPromise`。
  - evidence index 包含 `record_session_jsonl`。

### 防复发提醒
- `complete_publishes` 是 completion candidate,不是硬终止本身。
- 非 Ralph worker 不应该直接输出 completion promise 来结束 parallel run。
- `workflow:*` 这种完整 child run 必须保留 record-session 主证据流,否则“跑过”无法复验。
## [2026-05-18 18:13:22] [Session ID: omx-1779004640353-blcixq] 错误修复: 追加计划时误用未加引号 heredoc

### 现象
- 在追加支线 `task_plan__parallel_rec_analysis.md` 时,误用了未加引号 heredoc。
- shell 把正文里的反引号和通配内容当成命令替换,出现了 `no matches found`、`permission denied`、`command not found` 和 `parse error`。

### 原因
- 追加内容里包含反引号,但没有使用 `cat <<'EOF'`。
- shell 在 heredoc 过程中执行了命令替换,导致计划内容未按预期落盘。

### 修复
- 重新使用 `cat <<'EOF'` 追加计划、WORKLOG 和 ERRORFIX。
- 后续凡是往这些文件追加含反引号的 Markdown,都必须使用单引号 heredoc。

### 验证
- 重新追加后的计划、WORKLOG、ERRORFIX 内容均已落盘。
- 之后继续执行 focused test 和全量测试时没有再出现该类 shell substitution 错误。

## [2026-05-18 23:03:00] [Session ID: omx-1779004640353-blcixq] 错误修复: role_args 接线过程中的开发期错误

### 现象
- 过程里曾出现 focused test filter 写法不准确的问题。
- `parallel_runner` 中曾出现 `config.cli.role_args` move 后继续借用 `config` 的 Rust 所有权错误。

### 原因
- 测试过滤命令需要精确到 package / module path / `--exact`,不能把多个 filter 混成一个不稳定命令。
- executor 构造时需要保留后续对 `config` 的借用,因此不能直接 move 出 `config.cli.role_args`。

### 修复
- 使用 focused test 的准确命令重新验证。
- 在 parallel executor 构造路径中 clone `config.cli.role_args`,避免 move 后借用问题。

### 验证
- `cargo test -p ralph-cli parallel_runner::tests::parallel_role_backend_overlays_apply_coordinator_hooks_only -- --exact --nocapture`: 通过。
- `cargo test --quiet`: 通过,exit code 0。


## [2026-05-19 09:11:12] [Session ID: omx-1779004640353-blcixq] 错误修复: 追加结论时再次误用未加引号 heredoc

### 现象
- 追加 notes/worklog/later plan/epiphany/task_plan 时,误用了未加引号 heredoc。
- shell 将 Markdown 中的反引号内容当作命令替换,触发 `command not found`, `permission denied`, `parse error` 等错误。
- 本次失败没有把目标记录写入文件尾部。

### 原因
- 违反了支线上下文规则: 只要正文包含反引号,必须使用 `cat <<'EOF'` 或其他不会触发 shell substitution 的写入方式。

### 修复
- 改用 `python3` 直接按 UTF-8 追加字符串,避免 shell 解释 Markdown 正文。
- 重新检查目标文件尾部,确认失败前没有产生半截记录。

### 验证
- `tail` 检查显示失败前目标文件未追加半截内容。
- 随后使用 `python3` 追加完整记录。

## [2026-05-19 10:12:52] [Session ID: omx-1779004640353-blcixq] 错误修复: 追加 notes 时 heredoc 未加单引号导致命令替换

### 现象
- 我在追加支线 notes/task_plan 时,正文里包含反引号,但 heredoc 没有使用单引号。
- shell 把反引号中的内容当成命令执行,导致追加内容被破坏,并出现 command not found / no such file 之类的报错。

### 原因
- 违反了本项目的上下文写入规则: 追加 Markdown 且正文包含反引号时,必须使用单引号 heredoc。
- 这类写法会触发命令替换,把文档内容误当 shell 代码执行。

### 修复
- 后续追加上下文一律改用安全写法。
- 这次补写一条更正记录,把被破坏的方案说明重新写清楚。

### 验证
- 已重新阅读 task_plan 和 notes 尾部,确认需要补一条更正说明。
- 后续若再写入包含反引号的 Markdown,会改用单引号 heredoc 或等价的安全写法。


## [2026-05-19 10:31:57] [Session ID: omx-1779004640353-blcixq] 错误修复: 追加 spec/plan 交付记录时再次触发 heredoc 命令替换

### 现象
- 追加 notes/worklog 时使用了未加引号 heredoc。
- Markdown 中的反引号内容被 shell 执行,出现 `permission denied`, `command not found`, `Unknown backend: custom` 等输出。
- 其中 `ralph plan` 被误触发但未成功进入 planning session,因为当前 config backend 为 custom,命令以错误退出。

### 原因
- 再次违反上下文写入规则:含反引号 Markdown 必须使用单引号 heredoc 或其他不解释正文的写入方式。
- 这类错误容易污染 append-only 记录,而不能直接删除旧记录。

### 修复
- 保留被污染记录作为历史事实,不在中间删除。
- 使用 Python 追加更正记录,补回正确路径和验证结论。
- 后续上下文记录默认改用 Python append,避免 shell 解释正文。

### 验证
- 已检查尾部记录,确认污染只发生在 notes/worklog 文本中。
- 已确认 spec 与 plan 文件仍存在且内容正确。
- 已重新追加更正记录。

## [2026-05-19 12:43:00] [Session ID: omx-1779158263949-kticiv] 错误修复: capability.request 有 event 但父级 TUI 不出现新实例

### 现象
- 用户在 `parallel_rec.jsonl` 中看到 coordinator 发出了 `capability.request`。
- 请求语义看起来是在创建多个视角/实例,但父级 TUI 没有立即出现新的 instance。
- 原因不是 TUI 简单漏刷新,而是 `workflow:*` capability 本来就是 isolated child-run,不会修改 parent topology。

### 原因
- 现有协议把 `capability.request` 用作 isolated child/micro-run。
- Coordinator prompt 只告诉模型有 runtime capability catalog,但没有明确“父级可见新实例”应使用另一条 topology mutation 协议。
- TUI 和 `ralph agents` 也缺少 isolated child-run 的轻量可观测投影,导致 non-parent-visible child run 像是“没跑”。

### 修复
- 新增 `topology.spawn_group` 协议,用于 parent-visible group spawn。
- Supervisor 只接受 `ralph#*` coordinator 发出的 `topology.spawn_group`,并创建真实动态 HatInstance。
- `capability.request` 保持 isolated 语义,但在 core snapshot、TUI state、footer/output、`ralph agents` 中显示 child-run 状态。
- Coordinator prompt 和 capability catalog 明确三条路径:
  - `topology.spawn_group`: 父级可见多实例。
  - `capability.request`: isolated child/micro-run。
  - `spawn_instance="true" + target="<hat_id>"`: 父级可见单实例。

### 验证
- Focused tests、smoke_runner、全量 `cargo test --quiet` 均通过。
- `git diff --check` 通过。

### 防止复发
- 后续如果用户说“父级 TUI 里新增 hat instance”,不要让 coordinator 发 `workflow:*` capability。
- 如果看到 `capability.request`,判断标准应是 child-run projection 和 invocation artifacts,不是 parent instance list。

## [2026-05-20 00:22:10] [Session ID: omx-1779158263949-kticiv] 错误修复: `topology.spawn.result` 后 coordinator 重复派发 delivery topic

### 问题

- live dogfood 中 `topology.spawn_group` 已经创建 3 个动态 `analyst` 实例,并完成 direct delivery。
- 但 `ralph#1` 收到 `topology.spawn.result` 后,又二次发出 `analysis.task` with `audience_instances`,导致配置实例也参与任务。

### 原因

- coordinator prompt 的 `## WHAT TO DO` 只有 `task.start`、completion candidate 和 `any other event` 三类处理。
- `topology.spawn.result` 没有专门语义,会被 `any other event` 误导为需要继续 delegate 的普通事件。

### 修复

- 在 `build_ralph_coordinator_instructions()` 中新增 `topology.spawn.result` 处理规则:
  - spawned instances 已经通过 `delivery_topic` 收到 direct delivery。
  - 不要 re-emit delivery topic。
  - 不要把 `audience_instances` 当 replay 机制。
  - 等待 spawned workers 的 publish topics/results。
  - 如有 `failed`,只处理失败成员。
- 同时新增 `topology.spawn.failed` 处理规则:
  - 不伪造实例存在。
  - 不把原始 delivery topic 当 fallback 重发。
  - 改为发 corrective event 或 `reply.human.message`。
- 在 prompt focused test 中断言该规则只进入 coordinator prompt,不污染 worker prompt。

### 验证

- focused exact tests 全部通过。
- `git diff --check` 通过。
- `cargo test --quiet` 通过。
- 二次 live dogfood record-session `/tmp/ralph-topology-dogfood-guardrail-record.jsonl` 中:
  - `analysis.task` 总数为 3。
  - `topology.spawn.result` 之后 `analysis_task_after_spawn_result=0`。

### 后续注意

- 本轮 dogfood 仍因 worker 未稳定完成而 `MaxRuntime`,这是独立收敛问题,不要和 topology spawn 重复派发混为一谈。
