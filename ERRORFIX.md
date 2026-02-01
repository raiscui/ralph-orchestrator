# ERRORFIX

> 本次是设计/brainstorming 任务，非 bug 修复。
> 若后续演进为 bug fix，会在这里追加：问题、原因、修复、验证。

## 2026-01-27 03:06:52 +0800｜收尾验证中发现的编译错误（clippy 阶段）

### 现象
- 执行 `cargo clippy` 时失败，报错：E0308 mismatched types
- 位置：`crates/ralph-core/src/parallel/instance.rs`
  - `release_worktree(...)` 期望 `Option<&String>`
  - 实际传入了 `&Option<String>`

### 根因
- `RunningWorkspace.on_release_hook` 的类型是 `Option<String>`。
- `release_worktree` 为了避免不必要 clone，参数设计为 `Option<&String>`。
- 调用侧把 `Option<String>` 按引用传入，导致类型不匹配。

### 修复
- 将调用参数从 `&ws.on_release_hook` 改为 `ws.on_release_hook.as_ref()`。
- 同时顺手消除几处 clippy warnings（保证 8.2“全量检查”更干净）：
  - `clippy::ref_option`：把 `&Option<String>` 改为 `Option<&String>`
  - `clippy::bool_to_int_with_if`：用 `u8::from(bool)` 替代 `if {1}else{0}`
  - `clippy::collapsible_if`：折叠嵌套 `if`，使用条件链写法

### 验证
- `cargo fmt --check` ✅
- `cargo clippy` ✅（无 warnings）
- `cargo test` ✅

## 2026-01-27 04:23:16 +0800｜Mermaid 图表语法错误（graph 节点 label）

### 现象
- `mermaid-validator` 校验 `graph TD` 失败：Parse error
- 出错点：节点 label 使用了包含括号等特殊字符的写法（例如 `Ralph[ralph run (parallel)]`）

### 根因
- Mermaid flowchart/graph 的部分 label 字符在特定括号组合下会触发解析歧义。

### 修复
- 将节点 label 改为带引号的安全写法，例如：
  - `Ralph["ralph run (parallel)"]`
  - `Report["report.md / report.json"]`

### 验证
- `mermaid-validator` 对 `graph` 与 `sequenceDiagram` 均返回 `isValid=true`

## 2026-01-27 11:12:00 +0800｜E2E harness 在并行模式下的编译/退出问题（source_instance + hard kill）

### 现象
- `cargo clippy --all-targets --all-features -- -D warnings` 失败：
  - `crates/ralph-e2e`：`EventRecord` 新增字段后，多个单测/场景里结构体初始化缺字段导致编译失败。
  - `crates/ralph-cli`：`display.rs` 的单测里构造 `ralph_core::EventRecord` 同样缺字段导致编译失败。
  - `crates/ralph-e2e/src/executor.rs`：事件解析里触发 `clippy::implicit-clone`（`s.to_string()`）。
- 同时，为了“timeout 时强杀进程组”，最初方案使用 `pre_exec`，但仓库 lint 禁止 `unsafe`，导致编译直接报错。

### 根因
- 并行 HatInstance 模型引入 `source_instance` 后：
  - `EventRecord` 结构体新增 `source_instance: Option<_>`（用于事件归因）。
  - 但调用侧（尤其是测试里手写的结构体字面量）没有同步补齐字段。
- E2E 的“硬退出”需要可靠 kill 进程组：
  - `pre_exec` 方案依赖 `unsafe`，与仓库的 `forbid(unsafe_code)` 冲突。

### 修复
- `crates/ralph-e2e`：
  - `EventRecord` 全量补齐 `source_instance: None`（测试/场景）。
  - `RalphExecutor` 解析 JSONL 的 `source_instance` 字段写入 `EventRecord.source_instance`。
  - timeout 强杀：用安全 API `cmd.process_group(0)` 创建独立进程组；kill 时用 `getpgid(pid)` 获取真实 pgid，再 `SIGTERM -> grace -> SIGKILL`。
  - `clippy::implicit-clone`：`s.to_string()` 改为 `s.clone()`。
- `crates/ralph-cli/src/display.rs`：
  - 单测构造 `EventRecord` 补齐 `source_instance: None`。
- `crates/ralph-core/src/parallel/supervisor/routing_tests.rs`：
  - `make_supervisor` 去掉无意义的 `async`，修复 `clippy::unused_async`。

### 验证
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core --test smoke_runner` ✅

## 2026-01-27 13:01:41 +0800｜parallel-hat-instances E2E：无完成事件/超时卡死（以及性能基线测试 flaky）

### 现象
- `scripts/run-parallel-hat-instances-codex.sh` 早期会跑很久，需要手动 kill。
- 即便能结束，`.e2e-tests/parallel-hat-instances/.ralph/events.jsonl` 中只有 `build.task`，没有 `build.done/test.done`，导致 E2E 断言失败。
- 额外发现：`cargo test -p ralph-core` 偶发失败在 `bench_get_for_topic_baseline`（debug 下性能门槛过严，受负载影响）。

### 根因
1) 并行 Supervisor 只靠 `completion_promise` 退出，没有对齐串行 event_loop 的 `max_runtime/max_iterations` 护栏。
2) custom hat 的 prompt 被 InstructionBuilder “重型模板”包裹：
   - 模板包含 `### 2. VERIFY`，强制要求跑 tests/验证。
   - E2E 的 writer/tester 原本只需立刻发事件，但模型会优先去 `ls` / 找 Cargo.toml / 尝试 `cargo test`，导致超时前都没发 `build.done/test.done`。
3) 性能基线测试在 debug profile 下用固定 10_000 ns/op 作为硬门槛，容易在 CI/本机负载波动时变成 flaky。

### 修复
- `crates/ralph-core/src/parallel/supervisor.rs`：
  - 增加 `max_runtime_seconds` 与 `max_iterations`（以 ralph#1 job 完成次数近似）硬退出护栏。
- `crates/ralph-core/src/parallel/instance.rs`：
  - 仅 ralph#1 注入顶层 prompt（避免角色污染）。
  - custom hat 如果已显式提供 `instructions`，直接使用原文，不再套 InstructionBuilder 模板（避免 VERIFY 把 E2E 带偏）。
- `crates/ralph-e2e/src/scenarios/parallel.rs`：
  - 在生成的 `ralph.yml` 里设置 `event_loop.max_runtime_seconds: 240`。
  - 强化 writer/tester instructions：明确禁止跑 tests/命令/改文件，要求立即输出事件。
- `scripts/run-parallel-hat-instances-codex.sh`：
  - 运行前清理 `.ralph/events.jsonl`，避免旧数据污染断言。
- `crates/ralph-core/src/hat_registry.rs`：
  - `bench_get_for_topic_baseline`：debug 放宽阈值、release 保持严格阈值，避免 flaky。

### 验证
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `bash scripts/run-parallel-hat-instances-codex.sh` ✅（E2E: parallel-hat-instances 通过；events.jsonl 含 `build.done/test.done` 且带 `source_instance`）

## 2026-01-27 14:25:14 +0800｜并行 completion promise 早退导致丢事件（E2E flaky）

### 现象
- `bash scripts/run-parallel-hat-instances-codex.sh` 偶发失败：
  - `.e2e-tests/parallel-hat-instances/.ralph/events.jsonl` 只有 `build.task`
  - 没有 `build.done/test.done`

### 根因
1) Supervisor 在收到 ralph#1 的 completion promise 时曾经“立刻 break”：
   - 会导致同一轮输出里解析到的事件还没来得及 `route_event(...)`，下游实例就永远收不到触发事件。
2) completion drain 窗口过短（15s）：
   - 真实后端冷启动/慢响应时，下游 job 可能超过 15s 才能产出事件
   - drain 太短会提前 cancel，下游事件仍会丢失

### 修复
- `crates/ralph-core/src/parallel/supervisor.rs`
  - completion promise 改为“软退出信号”：
    - 先路由事件
    - 再进入 drain（min 0.5s / max 60s）
- `crates/ralph-e2e/src/scenarios/parallel.rs`
  - ralph prompt 改为更机械：发 `build.task` 后直接输出 `LOOP_COMPLETE`
  - `event_loop.max_runtime_seconds` 调整为 120（避免 240s 过慢，同时不给卡死机会）

### 验证
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `bash scripts/run-parallel-hat-instances-codex.sh` ✅（E2E: parallel-hat-instances 通过）

## 2026-01-27 15:59:35 +0800｜adapters.*.timeout：硬超时语义不一致 + ralph-adapters 编译失败

### 现象
- 你要求 `adapters.*.timeout` / `job_timeout_secs` 采用“检测超时（到点先看输出是否停滞）”语义，但 `ralph-adapters::CliExecutor` 仍是硬超时实现（到点就 SIGTERM）。
- workspace 处于编译失败状态：
  - `crates/ralph-adapters/src/cli_executor.rs` 的 `execute_capture_with_timeout` 调用 `execute(...)` 缺少 `output_stale_timeout` 参数，导致 `cargo check` 报错。

### 根因
1) `CliExecutor::execute` 仍使用 `tokio::time::timeout(duration, stream_result)`：
   - 这会把 `timeout` 解释为“最大运行时长”（硬超时），与“检测窗口 + stale 判定 + reset”的目标语义冲突。
2) 在为 `CliExecutor::execute` 增加 `output_stale_timeout` 参数后，部分调用点未同步更新，导致编译失败。

### 修复
- `crates/ralph-adapters/src/cli_executor.rs`
  - 将实现改为：stdout/stderr 并发读 + watchdog 检测窗口到期时按 `output_stale_timeout` 判定停滞 + 通过则 reset 检测窗口。
  - 修复 `execute_capture_with_timeout` 缺参：capture 模式默认退化为“硬超时”（`output_stale_timeout=None`），保持可预测行为。
  - 增补单测：停滞触发超时；持续输出不触发超时。
- `crates/ralph-cli/src/loop_runner.rs` / `crates/ralph-bench/src/main.rs`
  - 补齐 `CliExecutor::execute` 的 `output_stale_timeout` 传参（来自 `adapters.*.output_stale_timeout_secs`）。
- `crates/ralph-cli/src/parallel_runner.rs`
  - 清理无意义赋值，消除 warnings（语义不变）。

### 验证
- `cargo fmt` ✅
- `cargo check` ✅
- `cargo test` ✅

## 2026-01-27 16:50:06 +0800｜custom+codex：adapters.codex 的 timeout 不生效（回退 claude）

### 现象
- 用户配置：
  - `cli.backend: "custom"`
  - `cli.command: "codex"`
  - （可选）`cli.args: [...]`
- 期望：timeout 读取 `adapters.codex.timeout` / `adapters.codex.output_stale_timeout_secs`
- 实际：timeout 回退使用 `adapters.claude.*`（写了 codex 也不生效）

### 根因
- `RalphConfig::adapter_settings(backend)` 不认识 `"custom"`，直接走 default fallback 到 `adapters.claude`。
- 并行模式下 `HatBackend::Custom` 也只被标记为 `"custom"`，同样会触发 fallback。

### 修复
- `crates/ralph-core/src/config.rs`
  - `adapter_settings("custom")` 增加最小映射：当 `cli.command == "codex"` 时返回 `adapters.codex`
- `crates/ralph-core/src/parallel/supervisor.rs`
  - `HatBackend::Custom { command: "codex", .. }` 推导为 `"codex"`，让并行 job timeout/stale timeout 也走 codex 配置
- 新增测试覆盖映射语义（config + parallel job timeout）

### 验证
- `cargo test` ✅

## 2026-01-27 22:28:33 +0800｜parallel-trigger-routing：文档 Mermaid 解析失败 + clippy 生命周期错误

### 现象 1：Mermaid flowchart `Parse error`
- `specs/parallel-hat-instances/e2e.md` 的 flowchart 在 mermaid-validator 下报错：
  - edge label 里包含括号（例如 `emit build.task (target=writer)`）导致解析失败

### 修复 1
- 将 edge label 改为不含括号的写法（例如 `emit build.task target=writer`），并重新通过 mermaid-validator 校验。

### 现象 2：`cargo clippy` 报 `lifetime may not live long enough`
- 在 `crates/ralph-core/src/parallel/supervisor/routing.rs` 里使用：
  - `sort_by_key(|id| id.as_str())`
- 触发 `slice::sort_by_key` 的 key 缓存机制需要“可持有”的 key，导致引用生命周期不满足。

### 修复 2
- 改为不分配、无引用缓存的排序方式：
  - `sort_by(|a, b| a.as_str().cmp(b.as_str()))`

### 验证
- Mermaid：`mermaid-validator` ✅
- Rust：`cargo fmt --check` ✅、`cargo clippy --workspace --all-targets` ✅、`cargo test` ✅

## 2026-01-27 23:14:56 +0800｜E2E 首次运行失败：ralph-e2e 使用了过期的 `ralph` binary

### 现象
- 运行：
  - `cargo run -p ralph-e2e -- codex --filter parallel-trigger-routing --keep-workspace --verbose`
- `parallel-hat-instances` 场景断言失败：
  - 期望 `writer#2` 输出前缀存在（autoscale 扩容）
  - 实际 `writer#2` 不存在（两个 build.task 都落在 writer#1）

### 根因
- `ralph-e2e` 运行时会解析并直接使用本地的 `target/debug/ralph` 作为被测二进制。
- 我先改了 `ralph-core`（并行 batch 内 inflight 状态）但没有重新构建 `ralph`，导致 E2E 仍在跑旧逻辑。

### 修复
- 在跑 E2E 前先构建最新被测二进制：
  - `cargo build --bin ralph`
- 然后再次执行同一条 E2E 命令，场景通过。

### 验证
- `cargo run -p ralph-e2e -- codex --filter parallel-trigger-routing --keep-workspace --verbose` ✅

## 2026-01-28 20:50:27 +0800｜parallel-workflow-semantics：并行启动/收敛语义不一致导致“看起来靠 prompt.md 才能闭环”

### 现象
- 团队使用 parallel 模式时，`starting_event` / `task.start` / “什么时候结束” / orphan 兜底边界在实现、文档、示例里存在语义分裂。
- 直观表现是：示例里出现 `examples/parallel-trigger-routing/prompt.md` 后，容易让人误以为“没有 prompt.md 并行就无法正常闭环”。

### 根因
- 并行模式下 `ralph#1`（协调者）的默认指令过弱：没有把 `starting_event` / `complete_publishes` 的官方语义写死，导致 demo prompt 看起来像“必需品”。
- triggers 默认路由的 fallback/orphan 语义边界不够“链式”：在存在 wildcard 订阅者时仍会额外打扰 `ralph#1`（老板兜底语义不成立）。

### 修复
- 引入/固化 `event_loop.complete_publishes`（唯一、可选）作为 workflow completion candidate topic，并做非空校验与单测。
- 并行启动：`task.start/task.resume` 作为控制面 topic，强制 `target_instance=ralph#1`（避免 prompt pollution）。
- 路由语义：收敛为 specific > wildcard > 真 orphan→`ralph#1`，并补充单测。
- prompt 语义：在并行模式为 `ralph#1` 注入更强约束的协调指令（含 hats 拓扑表与动作约束），让“闭环”更多依赖 config+官方语义，而不是 demo prompt。
- 示例：`examples/parallel-trigger-routing` 将目标 prompt 内联到 `event_loop.prompt`，并用 `starting_event/complete_publishes` 表达 entry/exit（避免示例依赖额外 `prompt.md` 文件）。
- 文档：修正把 `starting_event` 当作第一条事件的 Mermaid 图；新增 replay smoke fixture 覆盖“completion candidate → coordinator 输出 LOOP_COMPLETE”。

### 验证
- `cargo test -p ralph-core` ✅
- `cargo test` ✅

## 2026-01-28 21:56:16 +0800｜E2E 回归：parallel-hat-instances 事件统计异常（stderr 回显被误判为事件）

### 现象
- 运行 `bash scripts/run-parallel-hat-instances-codex.sh` 时，`Parallel events recorded` 断言曾出现失败：
  - 期望：`events.jsonl contains >=2 build.task, >=2 build.done, >=1 test.done`
  - 实际（失败现场之一）：`build.task: 13, build.done: 0, test.done: 0`

### 根因
- `crates/ralph-cli/src/parallel_runner.rs` 的 `CliHatJobExecutor::handle_output_line` 会把 **stderr** 也拼进 `HatJobResult.output`。
- 但在 Codex CLI 下，stderr 往往包含“后端回显的 user prompt / 内部日志”。
  - 这些回显文本里可能出现 `<event ...>`（来自 prompt 示例或指令片段），从而被 EventParser 误判为“已发出的真实事件”。
- 结果就是：
  - `build.task` 等 topic 被重复解析/重复落盘（数量异常偏大）
  - `build.done/test.done` 的统计被污染，导致 E2E 断言失败或波动

### 修复
- 只让 stdout 进入 `HatJobResult.output`（用于 EventParser）。
- stderr 仍然流式输出给 Supervisor（可观测性保留），但不再参与事件解析。
  - 修改位置：`crates/ralph-cli/src/parallel_runner.rs`

### 验证
- `bash scripts/run-parallel-hat-instances-codex.sh` ✅
  - `Parallel events recorded`：`build.task: 3, build.done: 2, test.done: 1`
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test` ✅

## 2026-01-29 01:58 +0800｜E2E：workspace 复跑污染导致断言误判（尤其在 --keep-workspace 后）

### 现象
- 先运行一次 `ralph-e2e ... --keep-workspace` 保留工作区后，再次运行同一个 scenario：
  - `.e2e-tests/<scenario-id>/.ralph/events.jsonl` 会把两次运行的事件都堆在一起。
  - 结果是：事件计数类断言可能“虚假通过/虚假失败”，排查时也会被历史输出污染。

### 根因
- `WorkspaceManager::create_workspace()` 只做 `create_dir_all`，不会清理已存在的 workspace 目录。
- 当 workspace 目录已存在时（例如上次用了 `--keep-workspace`），新一轮测试会在旧目录上继续写入 `.ralph/events.jsonl`。

### 修复
- `crates/ralph-e2e/src/workspace.rs`：
  - 在 `create_workspace()` 里，如果 workspace 目录已存在，则先 `remove_dir_all` 再重新创建。
  - 语义：E2E 每次运行都从“干净工作区”开始，保证隔离与可重复。

### 验证
- `cargo test -p ralph-e2e workspace` ✅
- `cargo test` ✅

## 2026-01-29 02:15 +0800｜OpenSpec：archive 校验失败（Requirement 首句缺少 MUST/SHALL）

### 现象
- 执行 `openspec archive -y parallel-workflow-semantics` 时失败：
  - `Validation errors in change delta specs:`
  - `✗ ADDED "task.start and task.resume are control-plane topics routed to ralph#1" must contain SHALL or MUST`

### 根因
- OpenSpec validator 对 `### Requirement:` 段落的校验比较“强约束”：
  - 要求该 Requirement 的首句（紧跟标题的第一句话）必须包含 `MUST` 或 `SHALL`。
- 该 Requirement 的第一句原本是描述性陈述（没有 MUST/SHALL），因此触发校验错误。

### 修复
- 文件：`openspec/changes/parallel-workflow-semantics/specs/parallel-hat-instances/spec.md`
- 将第一句改为：
  - `In parallel mode, task.start and task.resume MUST be treated as control-plane topics.`

### 验证
- 再次执行 `openspec archive -y parallel-workflow-semantics`：归档成功，并完成 spec update ✅

## 2026-01-29 14:38 +0800｜并行模式收尾 warning：Failed to send StateChanged to supervisor

### 现象
- 运行 `examples/parallel-trigger-routing`（或并行模式相关场景）自然结束后，stderr 出现多条 warning：
  - `HatInstance actor exited with error ... Failed to send StateChanged to supervisor`
- 同时 `--no-tui` 的 `[supervisor] final states` 可能残留 `running/idle`，看起来“不像真正收尾完成”。

### 根因
- `ParallelSupervisor::run` 在决定退出后会立刻 return，从而 drop 掉 `instance_rx`（Supervisor 接收端）。
- HatInstance actor 在收尾阶段会 `set_state(Done)` 并发送 `StateChanged` 给 Supervisor：
  - receiver 已被 drop ⇒ `send(StateChanged)` 失败 ⇒ `set_state` 返回 Err
  - 外层把该 Err 记录为 warning：`HatInstance actor exited with error ...`

### 修复
- `crates/ralph-core/src/parallel/supervisor.rs`：
  - shutdown（cancel + shutdown）后，增加一个短暂的 shutdown-drain 窗口：
    - 继续 drain `StateChanged`，等待实例进入终态（Done/Failed）或超时
    - 避免 Supervisor 先退出导致实例收尾发送失败
    - 同时让 `final states` 更可信（尽量收敛到终态）
  - 另外：外部事件文件 `.ralph/current-events` 的读取路径改为相对 `workspace_root` 解析，避免测试/隔离环境被 repo 根目录 `.ralph/` 污染。
- `crates/ralph-core/src/parallel/supervisor/routing_tests.rs`：
  - 新增回归测试：`supervisor_run_waits_for_instances_to_reach_terminal_state_on_shutdown`

### 验证
- `cargo test --package ralph-core --lib parallel::supervisor::routing_tests::supervisor_run_waits_for_instances_to_reach_terminal_state_on_shutdown -- --exact` ✅
- `cargo test` ✅

## 2026-01-29 16:03 +0800｜ralph-tui：测试仍引用 chat_input 导致编译失败 + warning 收敛

### 现象
- `cargo test` 编译失败（发生在 ralph-tui 的 integration snapshot 测试编译阶段）：
  - `crates/ralph-tui/tests/common/mod.rs` 仍引用 `state.parallel.chat_input`，但实际 state 已迁移为 `chat_editor`。
- 同时编译输出包含 warning（影响 CI 干净度）：
  - `unused_assignments`（`grapheme_col_to_byte_idx` 里无意义赋值）
  - `dead_code`（`ParallelLayoutSnapshot` 存在未读取字段）

### 根因
- 并行 TUI 的 Chat 输入模型从“单行字符串”升级为“多行编辑器状态”（`ChatEditorState`）。
- 测试用的快照渲染器（`TuiTestHarness`）没有同步升级，仍按旧布局（1 行 input）与旧字段渲染。

### 修复
- `crates/ralph-tui/tests/common/mod.rs`：
  - 使用 `state.parallel.chat_editor` 渲染输入区（3 行输入 + `>`/`|` 提示符）。
  - bottom panel 高度对齐真实渲染（`9`）。
- `crates/ralph-tui/src/state/parallel.rs`：
  - 精简 `grapheme_col_to_byte_idx`，去掉无意义的 `idx` 初始赋值，消除 `unused_assignments`。
- `crates/ralph-tui/src/app.rs`：
  - 精简 `ParallelLayoutSnapshot` 字段，只保留 hit-test 需要的数据，消除 `dead_code`。

### 验证
- `cargo test -p ralph-tui` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅

## 2026-01-29 16:15 +0800｜record-session：parallel 忽略 + UX 记录格式不一致导致 cassette 不可回放

### 现象
- `ralph run --record-session xxx.jsonl`：
  - 串行模式：基本只有 `bus.publish` / `_meta.loop_start`，缺少 `ux.terminal.write`，cassette 无法用于 mock-cli 回放。
  - 并行模式：直接 warn “ignores --record-session (not wired yet)”。
- 录制出来的 UX 记录存在结构异常（double-wrapped）：
  - `data` 里又嵌套了一层 `{ event: "ux.terminal.write", data: {...} }`，与 fixtures/SessionPlayer 的解析假设不一致。

### 根因
1) `SessionRecorder::from_ux_event` 把整个 tagged `UxEvent` 再嵌套写进 `Record.data`。
   - 但 `SessionPlayer::parse_ux_event` 假设 `Record.data` 只包含 payload（TerminalWrite/Resize/...），它会用
     `{ event: record.event, data: record.data }` 重新组装 tagged 结构做反序列化。
   - 两者不一致 → 回放/解析会失败或行为不稳定。

2) `ralph-cli` 的 parallel runner 没有把 record-session 贯穿到 Supervisor 的输出/事件流。

### 修复
- `crates/ralph-core/src/session_recorder.rs`：
  - `Record::from_ux_event` 改为只写 payload（TerminalWrite/Resize/...），不再嵌套 tagged UxEvent。
- `crates/ralph-cli/src/loop_runner.rs`：
  - 每轮把“用于 event parsing 的输出文本”写为 `ux.terminal.write`（stdout-only），并补写 `_meta.termination`（best-effort）。
- `crates/ralph-cli/src/parallel_runner.rs`：
  - 接线 record-session：
    - stdout chunk → `ux.terminal.write`
    - supervisor event → `bus.publish`
  - 并行执行时注入 `RALPH_HAT_INSTANCE_ID` / `RALPH_HAT_ID`（用于回放分流）。
- `crates/ralph-proto/src/ux_event.rs`：
  - `TerminalWrite` 增加可选 `instance_id` 字段（并行录制归因）。

### 验证
- `cargo test` ✅
- 手工最小回归（无需真实后端）：
  - 并行模式下 `--record-session` 生成的 JSONL 包含：
    - `_meta.loop_start`
    - `bus.publish`
    - `ux.terminal.write`（含 `instance_id: "ralph#1"`）

## 2026-01-29 20:22 +0800｜并行 TUI：chat `Shift+Enter` 换行不稳定 + 单行输入贴顶 + 默认 stderr 策略调整

### 现象
1) 并行 Supervisor TUI 的 chat 输入框里，`Shift+Enter` 期望换行，但在部分终端环境里无效：
   - 用户实际看到的效果是“仍触发提交/发送”。

2) chat 输入框为多行高度，但当只输入一行（例如 `@writer#1 hello`）时，文本贴着输入框上沿，视觉上“太靠上”。

3) 并行模式流式输出默认隐藏 stderr，不利于调试（用户希望默认显示 stderr）。

### 根因
1) `Shift+Enter` 的可识别性依赖终端是否上报 `KeyModifiers::SHIFT`：
   - crossterm 在 API 层面支持 `KeyModifiers::SHIFT/ALT/CONTROL`；
   - 但“终端是否区分 Enter + Shift”是终端实现相关，部分环境下 `Shift+Enter` 与 `Enter` 无法区分。

2) chat 输入框渲染逻辑默认从顶部开始绘制；当行数不足输入框高度时，上方没有 padding → 单行内容贴顶。

3) stderr 的默认展示策略在 CLI 层被设定为默认隐藏（`show_stderr=false`），并在 observer 侧直接过滤掉了 stderr chunk。

### 修复
- `crates/ralph-tui/src/app.rs`：
  - 换行：保留 `Shift+Enter`，并增加更稳定的 fallback：
    - `Alt+Enter` 换行
    - `Ctrl+J` 换行
  - 视觉：当总行数不足输入框高度时，在顶部补空行做“底部对齐”，让输入内容整体下移。
  - 一致性：`hit_test_chat_editor` 同步扣除顶部 padding，避免鼠标点击定位与渲染不一致。

- `crates/ralph-tui/src/widgets/help.rs`：
  - help overlay 补充 `Alt+Enter` / `Ctrl+J` 的说明，降低学习成本。

- `crates/ralph-cli/src/main.rs`：
  - 并行模式默认 `show_stderr=true`；
  - 提供 `--hide-stderr`（SetFalse）用于显式隐藏 stderr（降噪）。
  - 增加单测覆盖默认值与开关行为（run/resume）。

- `crates/ralph-cli/src/parallel_runner.rs`：
  - 更新注释与提示文案，避免仍写“默认隐藏”造成误导。

### 验证
- `cargo fmt` ✅
- `cargo test` ✅（包含 replay smoke tests）

## 2026-02-01 00:34 +0800｜termimad Monokai Pro 配色：crossterm 版本冲突 + `NO_COLOR=1` 导致测试误判

### 现象
- `cargo clippy` / `cargo test` 编译失败（E0308）：
  - `expected ratatui::crossterm::style::Color, found crossterm::style::Color`
- 单元测试里 termimad 渲染出的 ANSI 只有 `\x1b[m`，没有 `38;2;...` / `48;2;...`：
  - 导致“解析后的 span fg/bg 应为 RGB”的断言失败。

### 根因
1) **依赖图里同时存在两套 crossterm：**
   - workspace 直接依赖 `crossterm 0.28`
   - `termimad 0.34.1` 依赖 `crossterm 0.29`
   - `MadSkin/LineStyle/CompoundStyle` 的方法参数使用的是 termimad 依赖的 `Color` 类型，所以直接传 workspace 的 `crossterm::style::Color` 会类型不匹配。

2) **环境变量 `NO_COLOR=1`：**
   - crossterm 会抑制彩色输出（`SetForegroundColor/SetBackgroundColor` 的 ANSI 参数变为空），于是渲染结果只剩 `\x1b[m`。
   - 这会让“基于渲染后 ANSI → 再解析”的颜色测试变得不稳定或恒失败。

### 修复
- `crates/ralph-adapters/src/stream_handler.rs`：
  - Monokai Pro palette 常量改用 `termimad::crossterm::style::Color`，避免 crossterm 版本冲突。
  - 回归测试改为直接断言 `default_markdown_skin()` 的 fg/bg 配置（不再依赖 ANSI 实际输出）。

### 验证
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test -p ralph-core kiro` ✅

---

## 2026-01-31 22:45 +0800｜合并 `for_marge` 后的 clippy/编译收口

### 现象
- 合并 `for_marge` 后运行 `cargo clippy --all-targets --all-features -- -D warnings`：
  - `ralph-tui` 出现编译错误（`MUTED_FG` 不存在、`ContentPane::new` 参数不匹配）。
  - `clippy` 触发多处 deny 警告（`unreadable_literal` / `new_without_default` / `double_ended_iterator_last` / `unchecked_time_subtraction`）。

### 根因
- `for_marge` 引入新的主题体系（`TuiTheme`）并替换了原本的 `MUTED_FG` 常量，但仍有少量旧调用点未迁移完成。
- `ContentPane` 构造函数升级为显式接收 `TuiTheme`，而复制提取逻辑仍在用旧签名。
- `clippy -D warnings` 对“可读性/性能/时间安全性”类 lint 也会强制失败，需要逐个修正。

### 修复
- `crates/ralph-tui/src/theme.rs`：
  - 补 `pub const MUTED_FG` 作为兼容常量（并行 state/help 等旧调用点先不炸）。
  - 颜色字面量加分隔符（`0xF5_E0_DC` 形式），通过 `unreadable_literal`。
  - `CatppuccinMocha` 补 `Default` 实现，通过 `new_without_default`。
  - `rows()/columns()` 的末尾元素读取改用 `next_back()`，通过 `double_ended_iterator_last`。
- `crates/ralph-tui/src/app.rs`：
  - 复制提取路径改为 `ContentPane::new(buffer, TuiTheme::default())`，通过参数检查。
  - 测试里 `Instant` 的减法改用 `checked_sub(...).unwrap()`，通过 `unchecked_time_subtraction`。

### 验证
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test -p ralph-core kiro` ✅

## 2026-01-31 12:20 +0800｜E2E mock-mode：parallel 下 ralph#1 多 job 导致回放提前终止

### 现象
- 新增的并行 E2E 场景在 live 后端（Codex）通过，但在 mock-mode 下失败：
  - 断言里缺少 `build.done`（workflow 被提前打断）
  - 实际表现为：`ralph#1` 在第一轮就“回放”出了 `LOOP_COMPLETE`，导致 supervisor 提前收敛

### 根因
- mock-mode 的 backend 是 `ralph-e2e mock-cli`。
- 旧实现只做了 `instance_id` 过滤，但**每次调用都会回放该 instance 的全部输出**：
  - parallel 下 `ralph#1` 往往会有多个 job（例如：job1 发入口事件，job2 才输出 `LOOP_COMPLETE`）
  - 于是 job1 的 mock 回放里就混入了 job2 的 `LOOP_COMPLETE`，导致 workflow 未跑完就退出

### 修复
- `crates/ralph-e2e/src/mock_cli.rs`：
  - 引入“按调用次数分段回放”机制（让一个 cassette 支撑多次 backend spawn）：
    - 顺序模式：按 `_meta.iteration` 分段（每次调用≈一轮 iteration）
    - 并行模式：按 `bus.publish.source_instance==instance` 的经验边界分段（每次调用≈一个 job）
  - 用 workspace 内的 `.ralph/mock-cli/*.count` 记录各 instance 的调用次数（0-based），每次调用消费下一段。

### 验证
- `cargo test -p ralph-e2e` ✅
- `cargo run -p ralph-e2e -- --mock --filter parallel-starting-event-inference --verbose` ✅

## 2026-01-31 02:35 +0800｜理性整合提交收口：修复测试失败（scratchpad 清理策略 + ACTIVE HAT prompt 断言）

### 现象
1) `cargo test` 失败：`crates/ralph-cli/tests/integration_resume.rs::test_continue_vs_run_event_difference`
   - 期望 `ralph run` 之后再 `ralph run --continue`，`.ralph/events.jsonl` 里能新增 `task.resume`。
   - 实际没有 `task.resume`，导致断言失败。

2) `cargo test` 失败：`crates/ralph-core/src/event_loop/tests.rs` 中两条断言仍然要求 `## HATS`。
   - 但我们已采纳 “active hat 时输出 `## ACTIVE HAT` + Event Publishing Guide、跳过全量 topology” 的新行为。

3) `cargo clippy` 失败：`crates/ralph-cli/src/hats.rs` 中残留 “AI 生成图表” 相关测试。
   - 函数已被删除（改为 `beautiful-mermaid-rs` 确定性渲染），测试仍在调用旧函数导致编译失败。

### 根因
1) fresh run 清理 scratchpad 时使用了 `remove_file`：
   - 在测试环境中 backend 是 `command: "true"`，不会生成新的 scratchpad。
   - 因此 `ralph run` 之后 scratchpad 不存在，`ralph run --continue` 在 CLI 层直接报错退出，
     自然也不会写入 `task.resume` 到 `.ralph/events.jsonl`。

2) prompt 结构变更后，旧测试仍按 “总是输出 `## HATS`” 的预期断言。

3) hats 图表从 “AI 生成” 改为 “Mermaid → 渲染器” 后，旧测试没有同步清理。

### 修复
1) scratchpad 清理策略改良（保持目标，但更稳健）：
   - 将 fresh run 的清理从“删除文件”改为“truncate 为空”（仍是清理旧状态，但保留文件存在性）。
   - 串行与并行 runner 都同步改了这一点：
     - `crates/ralph-cli/src/loop_runner.rs`
     - `crates/ralph-cli/src/parallel_runner.rs`

2) 同步测试到新 prompt 结构：
   - `crates/ralph-core/src/event_loop/tests.rs`：
     - active hat 场景改断言 `## ACTIVE HAT` + `### Event Publishing Guide`。

3) 清理 hats.rs 中已失效的旧测试：
   - 删除对 `build_diagram_prompt` / `extract_diagram` / `resolve_backend` 等已删除函数的引用测试。

### 验证
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test -p ralph-core kiro` ✅

## 2026-01-31 03:02 +0800｜语义修复：starting_event 不应作为初始化事件（未设置时由 ralph#1 决策）

### 现象
- `EventLoop::initialize()` 在 fresh run 时把 `event_loop.starting_event` 当作“初始化事件 topic”发布。
- 这与你的约定冲突：`starting_event` 不设置时，应该由 `ralph#1` 自行决定入口事件，而不是代码替你默认成 `task.start` 或直接改变初始化 topic。

### 根因
- 我把 `starting_event` 误解成了“第一个事件（initial event）”，并将它接入了初始化发布逻辑。
- 但项目的配置注释/文档语义实际是：
  - `task.start` 永远是 fresh run 的初始化事件（承载 top-level prompt）
  - `starting_event` 是“协调后工作流入口事件提示”（可选；未设置时由 ralph#1 决策）

### 修复
- `crates/ralph-core/src/event_loop/mod.rs`：
  - `initialize()` 固定发布 `task.start`，不再读取 `starting_event` 作为 topic。
- `crates/ralph-cli/src/loop_runner.rs`：
  - debug event logger 的“初始事件记录”同步修正为固定 `task.start`（fresh run）。
- `crates/ralph-core/src/hatless_ralph.rs`：
  - prompt 增强：
    - `starting_event` 未配置：明确提示 ralph#1 必须自行决定入口事件，并给出启发式候选列表。
    - `starting_event` 已配置：提示 ralph#1 协调后优先发布该入口事件。
- `README.md`：
  - 修正 `starting_event` 的说明（不是 first event），并修正示例配置。

### 验证
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test -p ralph-core kiro` ✅

## 2026-01-30 13:34 +0800｜并行 Output：移除左侧红色 E + 撤回 Big Headers/图片渲染 + 许可证回退 MIT

### 现象
1) 你希望“彻底回退” Big Headers/图片渲染等 `mdfried` 相关特性，恢复为纯文本 Output。

2) 你反馈并行 Output 面板左侧的红色 `E`（stderr 标识）不正常：
   - 你认为 stderr 用灰色弱化即可区分，无需额外的前缀列。

3) 你要求把仓库许可证从 `GPL-3.0-or-later` 回退到项目原本的许可（MIT）。

### 根因
- Big Headers/图片渲染与左侧前缀列属于同一轮“对齐 mdfried 视觉层级”的实现，会引入：
  - 额外的数据结构（Image block/row）
  - 额外依赖（`ratatui-image`/`cosmic-text`/`image`）
  - 输出布局复杂度（前缀列占宽、copy 需要特殊处理）
- 许可证当时为兼容 `mdfrier`（GPL）而切到 GPL；当决定取消 `mdfrier` 后，GPL 也不再是必须条件。

### 修复
- 移除 Big Headers/图片渲染：
  - 删除 Image 相关中间表示与渲染逻辑，输出 buffer 回到“纯文本行”模型。
  - 移除 `ratatui-image` / `cosmic-text` / `image` 依赖，并删除对应代码。
  - 移除 `tui.images.*` 配置项与 CLI/TUI 传递链路。
- 移除左侧红色 `E`：
  - `ParallelOutputPane` 不再渲染任何左侧前缀列。
  - stderr 仅通过 `MUTED_FG`（灰色）弱化呈现来区分。
- 许可证回退：
  - `Cargo.toml`：`workspace.package.license = "MIT"`
  - 根目录 `LICENSE`：替换为 MIT License 文本
  - README/docs：许可证 badge 与说明同步改为 MIT

### 验证
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅（包含 replay smoke tests）
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test -p ralph-core kiro` ✅

## 2026-01-30 12:47 +0800｜回退 Markdown 渲染器：mdfrier → termimad

### 现象
- 需求变更：你决定取消 `mdfried/mdfrier` 渲染 Markdown，并要求恢复项目原本的 `termimad` 渲染方式。

### 根因
- 之前为了对齐 `mdfried` 的渲染风格与语义换行能力，引入了 `mdfrier` 并替换了渲染链路。
- 现在需求反转，需要把渲染器与依赖完整回退，避免维护两套行为分叉。

### 修复
- 渲染回退：
  - `crates/ralph-adapters/src/stream_handler.rs`：
    - `Rendered` 模式改用 `termimad::MadSkin` 渲染 Markdown。
    - stdout 直接输出 termimad ANSI；TUI 用 `ansi-to-tui` 解析回 `ratatui::Line`。
- 依赖回退：
  - `Cargo.toml`：移除 `mdfrier`，新增 `termimad = "0.34.1"`。
  - `crates/ralph-adapters/Cargo.toml`：依赖切回 `termimad.workspace`。
  - `Cargo.lock`：更新后不再包含 `mdfrier`。

### 验证
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test -p ralph-core kiro` ✅

## 2026-01-30 03:56 +0800｜并行 Output 面板重构后的编译/Clippy 修复记录

### 问题
- `cargo check -p ralph-tui` 失败：
  - `parallel_output.rs` 中软换行渲染函数引用了不存在的变量 `area`
  - `ratatui_image::protocol::Protocol` 不实现 `Debug`，导致 `#[derive(Debug)]` 编译失败
  - `ParallelTuiState` 在遍历 `instances.values_mut()` 时又调用需要 `&mut self` 的渲染方法，触发 E0499（可变借用冲突）
- `cargo clippy -- -D warnings` 失败：
  - `clippy::collapsible_if`（两处嵌套 if 可折叠）

### 根因
- 重构把并行 Output 从“纯文本行”升级为“Text + Image”后：
  - 原有测试/示例里仍用 `ContentPane` 渲染并行 buffer，类型不匹配
  - 渲染逻辑放在 `ParallelTuiState` 的 `&mut self` 方法里，在持有 `instances` 的可变借用时无法重入调用
  - 图片协议对象 `Protocol` 是“不希望被 Debug 打印”的外部类型

### 修复
- `crates/ralph-tui/src/widgets/parallel_output.rs`
  - 修复软换行渲染函数变量名（`area` → `widget_area`）
  - 按 clippy 建议折叠嵌套 if
- `crates/ralph-tui/src/state/parallel/output.rs` + `crates/ralph-tui/src/state/parallel.rs`
  - 为包含 `Protocol` 的结构改为手写 `Debug`（只打印 `alt/area`，省略 protocol 细节）
  - 抽出 `ParallelOutputRenderer`，避免 `&mut self` 重入借用导致 E0499
- `crates/ralph-tui/examples/validate_widgets.rs` + `crates/ralph-tui/tests/common/mod.rs`
  - 并行 Output 统一使用 `ParallelOutputPane` 渲染（不再错误复用 `ContentPane`）
- `crates/ralph-tui/src/app.rs`
  - 单测里调用 `extract_output_selection_text` 时按新签名传 `CurrentOutputBuffer::Serial(...)`

### 验证
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core --test smoke_runner` ✅

## 2026-01-30 01:16 +0800｜并行 Supervisor：`LOOP_COMPLETE` 后仍派工（parallel-trigger-routing）+ Tier8 E2E 回归

### 现象
1) 在 `examples/parallel-trigger-routing` 的并行 demo 中：
   - `ralph#1` 已输出 `LOOP_COMPLETE`，
   - 但其他实例仍持续创建/运行新的 job，表现为“已经完成但仍在忙”。

2) 你还观察到 `spec_writer` 在 `LOOP_COMPLETE` 前跑了 3 次（理论上该 demo 应为 2 次）。

### 根因
1) completion promise（`LOOP_COMPLETE`）属于“软退出信号”：
   - 旧逻辑在看到 completion promise 后，仍继续路由其它实例产出的事件/Published 事件，
   - 从而在 completion 之后仍可能派生新 job（出现不收敛）。

2) 关于 “spec_writer 跑 3 次”：
   - `examples/parallel-trigger-routing/.ralph/events.jsonl` 是 append-only 历史日志，
   - 多次运行 demo 会把多次 run 的事件叠加在同一个文件里，容易把“多次 run”误判成“单次 run 重复触发”。

### 修复
- `crates/ralph-core/src/parallel/supervisor.rs`：
  - completion promise 出现后进入“收敛态”：
    - 允许已在跑的 job 自然结束（drain）；
    - **禁止再路由/派发任何新 job**（包括 Published / external / gate.timeout 等）。
  - 额外增加一个短 drain 窗口，避免 ralph 输出 completion 的同轮事件还没来得及触发下游就被立刻打断。

- `crates/ralph-e2e/src/scenarios/parallel_trigger_routing_example.rs`：
  - 新增 Tier8 场景：直接拷贝 `examples/parallel-trigger-routing/ralph.yml` 到 E2E workspace 跑。
  - 断言按 job_id 去重并按 hat 聚合的 `job_runs`：
    - `spec_writer == 2`
    - `spec_reviewer == 2`
    - `spec_logger == 3`
  - 额外断言：`LOOP_COMPLETE` 后不得出现新的 job_id（防止回归）。

- `crates/ralph-e2e/src/executor.rs`：
  - 新增 `PromptSource::Config`：让 E2E 场景可以不传 `-p`，直接使用 `ralph.yml` 内置的 `event_loop.prompt`（避免 E2E 提示词“改写 example 语义”）。

### 验证
- `cargo fmt --check` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test` ✅
- `cargo run -p ralph-e2e -- codex --filter parallel-trigger-routing-example --keep-workspace --verbose --skip-analysis` ✅

## 2026-01-29 23:59 +0800｜并行 E2E：example 覆盖 + spec_writer 次数断言（防回归）

### 现象
- 你观察到 `examples/parallel-trigger-routing` 在一次闭环中可能出现 **3 次** `spec_writer`（预期应为 2 次）。
- 且在 `ralph#1` 输出 `LOOP_COMPLETE` 后，仍可能出现其它进程继续创建/运行的“假活跃”。

### 根因
- E2E 之前没有“直接覆盖 example 配置”的场景，也缺少“按 job_id 去重统计 hat 运行次数”的硬断言。
  - 这会导致回归发生时，只能靠人工看日志发现，反馈周期长且不稳定。

### 修复
- 新增 Tier8 场景：`ParallelTriggerRoutingExampleScenario`
  - 直接拷贝 `examples/parallel-trigger-routing/ralph.yml` 到 E2E workspace 运行。
  - 解析并行 stdout 的 `job_id` 前缀并聚合到 hat 名（跨 instance 汇总）。
  - 断言（强调是 job 次数，不是 instance 数）：`spec_writer job_runs=2`、`spec_reviewer job_runs=2`、`spec_logger job_runs=3`，并额外要求 `LOOP_COMPLETE` 后不应出现新 job_id。

### 验证
- `cargo fmt --check` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅

## 2026-01-29 23:01 +0800｜并行运行时：`LOOP_COMPLETE` 之后仍派生新 job

### 现象
- 并行模式下，`ralph#1` 已输出 `LOOP_COMPLETE`（completion promise）后：
  - Supervisor 仍可能继续路由其它实例产出的事件；
  - 进而触发新的 hat job（例如 `writer` 结束后发出 `build.done`，继续触发 `collector`），表现为“已收敛但仍在跑/仍在创建新 job”。

### 根因
- `ParallelSupervisor::run` 在检测到 completion promise 后，只设置 `termination=CompletionPromise` 并进入 drain 窗口：
  - 但 drain 期间仍会继续调用 `route_events_batch(...)` / `route_event(...)` 去路由后续事件。
  - 结果是 completion 之后依然可能产生新的投递与新 job（这会放大并行场景的噪音与不确定性）。

### 修复
- `crates/ralph-core/src/parallel/supervisor.rs`：
  - completion promise 之后进入“收敛态”：
    - 仍允许正在运行的 job 自然结束（保留 drain 行为）。
    - 但不再路由/派发任何新事件（含 JobCompleted/Published/external/gate.timeout）。
- `crates/ralph-core/src/parallel/supervisor/routing_tests.rs`：
  - 新增回归测试 `supervisor_does_not_route_new_events_after_completion_promise`：
    - 构造 “completion 先发生、build.done 后发生” 的最小链路；
    - 断言 completion 后不应再触发下游 `collector`（修复前失败，修复后通过）。

### 额外加固（E2E）
- `crates/ralph-e2e/src/scenarios/parallel.rs`：
  - 新增按 stdout `job_id` 统计 instance job 次数的断言；
  - 新增 `LOOP_COMPLETE` 后不应出现新 job_id 的断言；
  - 调整场景，让 `collector` 在 `build.done(task_id=2)` 时触发严格投递失败，确保收敛发生在闭环末尾，便于稳定计数。

### 验证
- `cargo test` ✅（workspace 全量）

## 2026-01-29 22:26 +0800｜tui-markdown-rendering：并行/串行 TUI 默认渲染 Markdown，`--plain` 可关闭

### 现象
- AI code agent 的 CLI 输出通常是 Markdown（h1/h2、引用、代码块等）。
- 之前并行 Supervisor TUI 的输出视图只是 `Line::from(raw)`，导致：
  - Markdown 控制符（`##`、`>`、`````）原样堆在屏幕上，可读性差；
  - CLI 侧即使新增了 `--plain` 开关，也无法真正影响并行输出的渲染路径（TUI 侧缺少贯通的 render mode）。

### 根因
1) 渲染逻辑分叉：
   - 串行 TUI 已经有 `termimad` 渲染能力；
   - 并行 Supervisor TUI 只做“按行追加”，没有 Markdown 渲染与模式开关。

2) 配置未贯通：
   - `ralph-cli` 需要把 `--plain` 传给 TUI；
   - `ralph-tui` 需要在 state 中保存渲染模式，并在追加输出时使用该模式重新渲染。

### 修复
- 统一渲染入口（复用既有依赖，避免引入 GPL 风险）：
  - `crates/ralph-adapters/src/stream_handler.rs`：
    - 新增 `MarkdownRenderMode`（Rendered/Plain）
    - 新增 `render_text_to_lines(text, mode)`：Rendered best-effort Markdown；Plain 保留控制符；两者都保留 ANSI 解析

- 并行 Supervisor TUI 贯通：
  - `crates/ralph-tui/src/state/parallel.rs`：
    - 增加 `output_render_mode: MarkdownRenderMode`
    - job 侧保存 raw 输出行并“全量重渲染”（支持跨行 fenced code block）
    - stderr 始终 Plain 并弱化前景色（但仍解析 ANSI）
  - `crates/ralph-tui/src/lib.rs`：
    - `with_parallel_output_render_mode(...)` / `with_parallel_markdown_rendering(...)` 实际写入 state（不再 no-op）

- CLI 参数：
  - `crates/ralph-cli/src/main.rs`：`ralph run` / `ralph resume` 新增 `--plain`

### 验证
- `cargo test` ✅（workspace 全量，包含 replay smoke tests）

## 2026-01-29 22:20 +0800｜`cargo test` 编译失败：`Tui` 缺少 Markdown 渲染开关接口

### 现象
- 运行 `cargo test` 时，`ralph-cli` 编译失败：
  - `crates/ralph-cli/src/parallel_runner.rs` 调用 `Tui::with_parallel_markdown_rendering(...)`，
    但 `ralph-tui::Tui` 未实现该方法（以及另一处 render-mode 相关接口）。

### 根因
- CLI 侧已开始为 `--plain` 等参数预留“并行 TUI 输出渲染模式”的接线点；
- TUI 侧缺少对应的 builder-style 方法，导致链接阶段前的编译直接失败。

### 修复
- `crates/ralph-tui/src/lib.rs`：
  - 补齐兼容接口（最小实现）：
    - `with_parallel_markdown_rendering(...)`
    - `with_parallel_output_render_mode(...)`
  - 当前实现为 no-op（先保证编译与测试闭环），并在注释中标明后续应该把 render mode 写入 state 并接入渲染管线。

### 验证
- `cargo fmt` ✅
- `cargo test` ✅

## 2026-01-29 22:10 +0800｜并行 TUI：Shift+Enter 无法换行 + 灰色过亮/过暗

### 现象
1) `Shift+Enter` 在并行 TUI 的 chat 输入框里仍然无法触发换行（表现为普通 Enter 的提交行为），但 `Ctrl+J` 可以换行。

2) 灰色样式在不同终端主题下不稳定：
   - `Color::DarkGray` 太暗；
   - `Color::Gray`（在你的主题下）接近白色，太亮。

### 根因
1) `Shift+Enter` 无法区分：
   - 不是我们 chat editor 的逻辑问题，而是“终端输入上报模式”的问题。
   - 很多终端在默认模式下不会把 `Shift+Enter` 作为“带 SHIFT 的 Enter”上报，导致应用层拿不到 `KeyModifiers::SHIFT`。

2) 灰色不稳定：
   - `Color::DarkGray`/`Color::Gray` 都属于 ANSI 16 色语义色，具体亮度由终端主题调色板决定；
   - 因此在不同主题下可能出现“过暗/过亮”的极端表现。

### 修复
- `Shift+Enter`：
  - `crates/ralph-tui/src/app.rs` 启动时 best-effort 启用 crossterm 的 kitty keyboard protocol：
    - `PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)`
  - 退出时配对：
    - `PopKeyboardEnhancementFlags`
  - 让支持该协议的终端可以上报 Enter 的修饰键，从而触发我们已有的 `Shift+Enter=换行` 分支。

- 灰色折中：
  - 新增 `crates/ralph-tui/src/theme.rs`，集中定义：
    - `MUTED_FG = Color::Indexed(245)`（256 色灰阶中灰）
  - 将提示/标签/空态文本统一改用 `MUTED_FG`，避免 `Gray≈白色` 或 `DarkGray≈看不清`。

### 验证
- `cargo fmt` ✅
- `cargo test` ✅（包含 replay smoke tests）

## 2026-01-29 21:45 +0800｜并行 TUI：Chat 输入贴 Targets + 灰色过暗（可读性差）

### 现象
1) Chat 输入框只输入一行时，文本/光标会贴着输入框底线显示，紧挨着下方 `Targets:` 行，视觉上“太挤”。

2) TUI 内多处灰色提示/标签使用 `Color::DarkGray`，在部分终端主题下对比度偏低，阅读困难。

### 根因
1) 之前为了解决“太靠上”，输入框做了“底部对齐”（上方 padding）。
   - 但当输入框高度固定为 3 行且内容只有 1 行时，正文会落在最后一行，导致与下方区域没有“呼吸间距”。

2) `Color::DarkGray` 在某些主题里接近背景色，导致灰色信息“看不清”。

### 修复
- `crates/ralph-tui/src/app.rs`：
  - 抽出 `chat_editor_pad_top()`，统一管理 chat 输入框的垂直对齐策略：
    - 内容不足高度时“下移”；
    - 同时保留 1 行底部留白，让输入内容不贴着 `Targets:` 行。
  - `hit_test_chat_editor` 同步使用同一策略，保证点击定位与渲染一致。
  - 更新回归测试 `hit_test_chat_editor_accounts_for_bottom_aligned_padding`，覆盖“顶部 padding + 底部留白”的布局。

- 灰色提亮（可读性改良）：
  - `crates/ralph-tui/src/app.rs` / `crates/ralph-tui/src/state/parallel.rs` / `crates/ralph-tui/src/widgets/*`：
    - 将 `Color::DarkGray` 统一替换为 `Color::Gray`。

### 验证
- `cargo fmt` ✅
- `cargo test` ✅（包含 replay smoke tests）

## 2026-01-29 20:50 +0800｜并行 TUI：框选后 Cmd+C/Cmd+V 无法复制粘贴

### 现象
- 用户在 TUI 输出面板里用鼠标框选（蓝色高亮）后：
  - `Command+C` 没有把选中文本复制到系统剪贴板；
  - 随后 `Command+V` 也无法粘贴出预期内容（因为剪贴板里没有被选中的文本）。

### 根因
1) TUI 使用 raw mode 并启用了 mouse capture：
   - 终端模拟器的“原生文本选择”通常会被关闭或需要额外按键绕过；
   - 应用内的蓝色高亮只是 UI 状态，不会自动进入系统剪贴板。

2) `Command+C` / `Command+V` 是终端模拟器快捷键：
   - `Cmd+C` 通常不会以 key event 的形式交给应用（crossterm 也拿不到 Command 修饰键）；
   - 所以如果应用不主动写剪贴板，就无法形成复制闭环。

3) 粘贴侧：部分终端会用 bracketed paste 上报 `Event::Paste(text)`：
   - 若应用忽略该事件，用户会感知为“Cmd+V 没反应”。

### 修复
- `crates/ralph-tui/src/app.rs`：
  - MouseUp（结束输出框选）时自动复制选中文本到剪贴板（best-effort）。
  - 增加 `y` 显式复制键（并行模式非 Chat 焦点下可用）。
  - 支持 `Event::Paste(text)`：
    - search mode：追加到搜索输入（压平换行）
    - chat focus：插入到 chat editor（保留换行）
  - 选中文本提取复用 `ContentPane` 渲染，保证“所见即所得”（含 soft wrap / scroll）。
  - 剪贴板后端：
    - macOS 优先 `pbcopy`
    - 兜底使用 OSC52（终端剪贴板）

- `crates/ralph-tui/src/widgets/help.rs`：
  - 补充 `Drag`（自动复制）与 `y`（复制）提示。

- `crates/ralph-tui/Cargo.toml`：
  - 增加 `base64` 依赖（用于 OSC52）。

- 规格：
  - `specs/tui-selection-clipboard.spec.md` 补齐验收标准。

### 验证
- `cargo fmt` ✅
- `cargo test` ✅（包含 replay smoke tests）
