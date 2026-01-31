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

---

## 2026-01-30 16:00 +0800｜TUI：启动进场动画“先全显示再动画”导致闪烁

### 现象
- 刚打开 TUI 时，会先看到完整的 panel/block 都出现了。
- 随后进场动画才开始扫入，视觉上像“闪一下/抖一下”，很不自然。

### 根因
- tachyonfx 的默认 Shader 流程是：`timer.process(delta)` → `execute()`。
- 首帧如果被输入事件拖慢，`fx_delta` 会偏大：
  - 启动动画第一次执行时就已经处在“中途进度”
  - 于是看起来像“先把完整 UI 画出来，再扫一遍动画”

### 修复
- `crates/ralph-tui/src/app.rs`
  - 启动动画被添加的那一帧，强制 priming：把 `fx_delta` 归零，并重置 `last_effect_tick`。
  - 保证启动动画首帧一定从“全隐藏起步态”开始，下一帧再正常推进时间轴。
- `crates/ralph-tui/src/app.rs`
  - 新增回归测试：`startup_animation_first_frame_priming_prevents_full_ui_flash`

### 验证
- `cargo test -p ralph-tui` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅

---

## 2026-01-30 20:51 +0800｜Warp：窗口 padding 外圈发灰（放弃 OSC 11/111，改为终端默认背景）

### 现象
- Warp 中 TUI 之外的 padding（圆角外圈/边缘留白）出现一圈偏灰背景。
- 用户反馈该问题是引入 exabind 风格/主题背景后出现的，之前 Warp 半透明背景是统一的。

### 根因
- 该灰色区域本身不属于 ratatui 的 cell，无法直接通过 widget `bg` 绘制去改变它。
- 但我们“间接制造”了对比：TUI 内容区被我们大量刷成显式 `bg`（crust/base）后变成不透明纯色，
  而 Warp 的 padding 仍然是半透明窗口背景，二者并列时就产生“外圈灰了一圈”的强烈对比。
- `OSC 11/111` 属于 best-effort，实测/用户反馈在 Warp padding 上不稳定或不生效，因此不能作为可靠修复。

### 修复
- `crates/ralph-tui/src/theme.rs`
  - 新增 `use_terminal_default_bg` 背景模式，并提供 `app_bg_color()` / `panel_bg_color()`：
    - Warp 模式：`Color::Reset`（使用终端默认背景）
    - 默认模式：`crust` / `base`（显式主题背景）
- `crates/ralph-tui/src/app.rs`
  - 当 `stdout` 为 TTY 且检测到 Warp（`TERM_PROGRAM` 包含 `warp`）时，启用 `with_terminal_default_bg()`。
- `crates/ralph-tui/src/widgets/header.rs`、`crates/ralph-tui/src/widgets/footer.rs`、`crates/ralph-tui/src/animation.rs`
  - 背景统一改为 `theme.app_bg_color()`，确保 Warp 下不再强制刷不透明纯色背景。

### 验证
- `cargo test -p ralph-tui` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅

---

## 2026-01-30 21:47 +0800｜Warp：Output 重启动画触发“全屏背景变动”

### 现象
- Warp 透明背景模式（`bg=Reset`）下，切换实例触发 Output “重启动画”时，肉眼可见全屏背景在变暗/变色。

### 根因
- tachyonfx 的 `sweep_in/out` 会对 fg/bg 做颜色插值。
- 在 tachyonfx 内部：
  - `Color::Reset` 的 RGB 表达是 (0,0,0)（黑色）
  - `sweep_in/out` 的中间帧还会把 `cell.bg==Reset` 临时视作 Black 参与 lerp
- 因此当 Output pane 面积很大时，动画遮罩会把大面积区域短暂插值成“黑底”，观感上就像全屏背景在动。

### 修复
- `crates/ralph-tui/src/animation.rs`
  - 当 `theme.app_bg_color()==Color::Reset`（Warp 透明背景模式）时：
    - `output_reopen_effect` 改用 `dissolve_to + coalesce_from`（带 `SweepPattern::up_to_down`）
    - 避免颜色插值，彻底规避 Reset→Black 的副作用
  - 增加回归测试：`output_reopen_effect_terminal_default_bg_does_not_paint_black_background`

### 验证
- `cargo test -p ralph-tui` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test` ✅


---

## 2026-01-31 12:00 ｜TUI：ContentPane 清空时抹掉 pane 底色（导致透明发灰/动画眩光）

### 现象
- 你希望 “block 内部有底色”，但外圈保持 Warp 的半透明（`bg=Reset`）。
- 实际体验上会出现：
  - pane 内部底色不稳定（看起来发灰、像透出来了 Warp 背景纹理）；
  - Output 动画更容易出现刺眼的白条/强对比变化（甚至引发背景跟随的错觉）。

### 根因
- `panel_block(...).style(theme.panel_bg())` 先把 pane area 铺成 `base` 底色没问题。
- 但 `ContentPane` 在逐格写入/清空时，如果使用 `Cell::reset()` 或写入 `Style::default()`：
  - 很容易把 cell 的 `bg` 还原为 `Reset`，把外层铺好的 `base` 底色抹掉。
- 一旦 pane 内部大量 cell 回到 `bg=Reset`，就会放大 Warp 透明模式下的视觉问题与动画副作用。

### 修复
- `crates/ralph-tui/src/widgets/content.rs`
  - 读取当前区域左上角 cell 的 `bg` 作为 `base_bg`。
  - 构造 `base_style = theme.text().bg(base_bg)` 并先铺满区域（清空残影，同时保留底色）。
  - 渲染内容时用 `base_style.patch(span.style)` 合并样式，避免把 bg 写回 `Reset`。
  - 宽字符 continuation cell 写入 `symbol==\"\"`，避免对齐异常。
  - selection 改为末尾统一 overlay，保证空白处也能正确高亮。

### 验证
- `cargo test -p ralph-tui` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test` ✅


---

## 2026-01-30 23:15 ｜TUI：Output 动画期间“最外圈”被染上 pane 底色（Warp 透明模式）

### 现象
- 目标体验：
  - pane/block 内部允许有底色（Catppuccin `base`），方便阅读。
  - 最外圈（终端默认背景 / Warp 半透明区域）保持透明（`bg=Reset`）。
- 实际表现：
  - Instances pane 看起来正常：周围仍透明。
  - Output pane 触发动画时，用户能看到“最外圈”也跟着被染上同样的底色（像外圈不再透明）。

### 根因
- Ralph 的动画是后处理（shader-like）：
  - 我们先渲染 widgets（含 exabind 边框补丁 `patch_exabind_panel_border_bg`）。
  - 然后才由 `EffectManager::process_effects` 对 buffer 施加动画。
- 一旦某个动画 effect 在某一帧覆盖到了边框 cell（尤其是 `bg=Reset` 的外圈），
  它就会把我们“刚刷回去的 Reset 背景”覆盖掉，导致用户感知为“外圈被染色”。

### 修复
- `crates/ralph-tui/src/app.rs`
  - 在 `EffectManager::process_effects(...)` 执行后（同一帧内），
    对 Instances/Output/Bottom 三个 pane 再执行一次 `patch_exabind_panel_border_bg`。
  - 仅在 Warp 透明背景模式（`theme.app_bg_color()==Color::Reset`）下执行，避免影响非 Warp 的动画观感。
- `crates/ralph-tui/src/theme.rs`
  - 新增回归测试 `patch_exabind_panel_border_bg_restores_border_after_bg_mutating_effect`：
    - 模拟一个会改 `bg` 的 sweep effect 覆盖边框；
    - 断言 re-patch 后边框 bg 恢复为 `Reset`，防止“外圈被染色”回归。

### 验证
- `cargo test -p ralph-tui` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test` ✅


---

## 2026-01-31 03:05 ｜TUI：切换 Instances 时 Output 先显示后消失（闪烁）

### 现象
- 在并行模式（Supervisor TUI）切换 Instances 选中项时：
  - Output 会先显示“新实例输出”一帧
  - 再消失、再做入场动画
- 观感是明显闪烁。

### 根因
- 之前 Output 重启动画是 `sweep_out + sweep_in`：
  - `sweep_out` 的首帧是“完全可见态”（timer reversed → `alpha=1`）。
- 同时我们在这一帧里已经用“新选中的实例”渲染了 Output 内容：
  - 结果就是：新内容先露出一帧，然后才被 sweep_out 盖掉。

### 修复
- `crates/ralph-tui/src/animation.rs`
  - Output 重启动画改为只做 `sweep_in + fade_from_fg`（从隐藏态揭开），避免 `sweep_out` 的“首帧可见”特性。
- `crates/ralph-tui/src/app.rs`
  - 在添加 Output 重启动画的那一帧执行 priming：`fx_delta=0`，确保动画从初始态起步。
  - 该 priming 逻辑同时适用于启动入场动画与 Output 重启动画（统一处理“首帧大 delta”问题）。

### 验证
- `cargo fmt --check` ✅
- `cargo test -p ralph-tui` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test` ✅

---

## 2026-01-30 21:55 +0800｜Warp：启动进场动画“先显示全部再逐块出场”

### 现象
- Warp 透明背景模式（`bg=Reset`）下，启动时会先看到完整 UI，再逐块出场动画。
- 期望是：动画开始前应为空屏（所有 block 不可见），然后逐块显示出来。

### 根因
- 原实现使用 `sweep_in + fade_from_fg` 作为启动遮罩：
  - 该组合只改 fg/bg，不改 symbol；
  - 在 `bg=Reset` 时，fg/bg=Reset 仍会显示终端默认前景色，导致“遮罩无效”，于是先看到完整 UI。

### 修复
- `crates/ralph-tui/src/animation.rs`
  - `startup_open_effect`：当 `app_bg==Reset` 时改用 `slide_in`，通过把未揭开的 cell 变成空格实现真正的“空屏起步态”。
  - `startup_open_effect_parallel`：当 `app_bg==Reset` 时改用 `slide_in + prolong_start` 编排：
    - Instances(frame) → Instances(items) → Output → Chat/Gates 严格串行
    - Instances(items) 延迟启动，确保“先框后字”

### 验证
- `cargo test -p ralph-tui` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test` ✅

## 2026-01-30 16:30 +0800｜Warp：窗口 padding（UI 范围外）发灰，但会“跟随动画变色”

### 现象
- Warp 终端里，TUI 之外（窗口 padding / 圆角外圈）仍然发灰。
- 用户观察到在启动/Output 动画时，这个外圈也会跟随变色。

### 根因（推断 + 边界）
- 该区域不属于 ratatui 的字符栅格（cell），无法通过“改 widget 的 bg”直接绘制。
- 但 Warp 的 padding 背景可能与“终端默认背景色”（或透明/blur/vibrancy 的合成结果）有关，因此会在 TUI 动画改变整体观感时显得“跟随变色”。

### 修复（best-effort）
- `crates/ralph-tui/src/app.rs`
  - 仅在 stdout 为 TTY 且检测到 Warp（`TERM_PROGRAM` 包含 `warp`）时：
    - 进入 alternate screen 后发送 `OSC 11` 设置终端默认背景色为主题 `crust`
    - 退出时发送 `OSC 111` 恢复终端主题默认背景色
  - 增加单测校验转义序列格式：`osc_set_background_sequence_*` / `osc_reset_background_sequence_*`

### 验证
- `cargo test -p ralph-tui` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅

## 2026-01-30 16:26 +0800｜TUI：用户要求 panel 外圈“整圈深色”（补齐顶边）

### 现象
- 用户进一步明确：不仅左右侧竖边，panel 的“最外圈整圈”都要是统一的深色背景。

### 根因
- exabind 的 `▔`（顶边横线）同样属于块元素：字形只占 cell 的一部分，空白会用 cell 的 `bg` 填充。
- 之前只修了底边/竖边，没有把顶边整行刷回外侧背景，因此顶边仍会显得偏灰。

### 修复
- `crates/ralph-tui/src/theme.rs`
  - 扩展 `patch_exabind_panel_border_bg`：增加“顶边整行” `bg=crust` 的补丁。
  - 更新单元测试：增加顶边整行断言，确保 border ring（四边）背景一致。

### 验证
- `cargo test -p ralph-tui` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅

## 2026-01-30 16:23 +0800｜TUI：exabind 竖边背景偏灰（与深色分隔不一致）

### 现象
- TUI 左右两侧（竖边/分割线）背景偏灰。
- 与 chat / output 之间的深色分隔区域观感不一致，期望统一为更深的背景（“半透明”在终端里用同一深色近似）。

### 根因
- exabind 边框集使用 `▏` / `▕` 这类“细竖条”块元素：
  - 字形只占 cell 的一小部分，其余空白会用 **cell 的背景色** 填充。
- panel 边框 cell 默认继承 panel 内部背景 `base`：
  - 相比外侧/分隔区域的 `crust` 更亮 → 视觉上就像一条“发灰”的竖边。

### 修复
- `crates/ralph-tui/src/theme.rs`
  - 扩展 `patch_exabind_panel_border_bg`：
    - 除了“左上角 + 底边整行”，再把 **左右边框列** 的 `bg` 也刷回 `crust`。
  - 更新单元测试：覆盖左右边框列 `bg` 为 `crust` 的断言，且内部区域不受影响。

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

---

## 2026-01-30 14:00 +0800｜TUI：exabind 边框左上角“斜切角”在本地被背景糊住（观感像锯齿）

### 现象
- `ralph-tui` 使用 exabind 风格边框（`▟▜▔▏▕`）后：
  - 左上角的“斜切角”不够干净，看起来像锯齿/缺口被糊住。
  - 与 exabind 网页 demo（JetBrains Mono + beamterm/ratzilla）观感有差异。

### 根因
- `▟` / `▔` 这类 Unicode 块元素字形内部存在空白区域，空白区域会使用 **cell 的背景色** 填充。
- 我们的 panel 内部背景使用 `base`（略亮），而 panel 外侧背景是 `crust`（更暗）：
  - 若不做额外处理，`▟` 的空白象限与 `▔` 的下方空白区域会被 `base` 填满，导致斜切角与底边“贴不住”。

### 修复
- `crates/ralph-tui/src/theme.rs`
  - 新增 `patch_exabind_panel_border_bg`：渲染后把左上角 cell 与底边整行的 `bg` 刷回外侧背景（`crust`），对齐 exabind 的做法。
  - 补充单元测试确保不回归。
- `crates/ralph-tui/src/widgets/instances.rs`、`crates/ralph-tui/src/app.rs`
  - 在 Instances / Output / Chat-Gates 面板渲染后调用 patch。

### 验证
- `cargo test -p ralph-tui` ✅
- `cargo test` ✅


---

## 2026-01-30 22:28 ｜TUI：并行启动入场动画首帧必须为空屏（避免先全显示再动画）

### 现象
- Warp（透明背景）下启动 Supervisor TUI：
  - 会先看到完整 UI（header/footer + Instances/Output/Chat/Gates）。
  - 随后才开始逐块入场动画。
- 观感像“闪了一下”，很不自然。

### 根因
- 并行模式的启动动画只覆盖了 content panes（Instances/Output/Bottom）。
- header/footer 不在动画遮罩范围内：
  - 首帧会被直接渲染出来。
  - 用户就会感知到“先全显示一帧”。

### 修复
- `crates/ralph-tui/src/animation.rs`
  - `startup_open_effect_parallel(...)` 增加 `header_area` / `footer_area` 参数。
  - 在 `bg=Reset`（Warp 半透明）分支里，把 header/footer 也纳入 Stage 1 的 `slide_in` symbol 遮罩。
- `crates/ralph-tui/src/app.rs`
  - 调用 `startup_open_effect_parallel` 时传入 `chunks[0]` / `chunks[2]`。
- 新增回归测试：
  - `startup_open_effect_parallel_terminal_default_bg_starts_from_blank_screen`
  - 断言：duration=0 的首帧 buffer 全部为 ``（空屏）。

### 验证
- `cargo test -p ralph-tui` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test` ✅


---

## 2026-01-31 01:43 ｜TUI：Warp(bg=Reset) 下避免背景闪烁的同时，恢复更好看的“白条扫入”

### 现象
- 修复了 Output 动画引起的整屏背景变动后（避开 `sweep_in/out`），
  当前动画的“白条扫入”观感变差：更像噪点溶解，不如以前干净。

### 根因
- `bg=Reset` 下不能使用 tachyonfx 的 `sweep_in/out`（会把 Reset 当作黑色插值）。
- 之前选用的 `dissolve/coalesce` 虽然安全，但它天生是“随机噪点”风格，不是连续白条。

### 修复
- `crates/ralph-tui/src/animation.rs`
  - `bg=Reset` 时 Output 重启动画：改为 `slide_out + slide_in`（symbol 遮罩，连续白条感更强）。
  - 引入 `SYMBOL_SWEEP_GRADIENT_MAX=10`，收窄渐变带，避免白条过厚/过糊。

### 验证
- `cargo test -p ralph-tui` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test` ✅


---

## 2026-01-31 02:08 ｜TUI：恢复 sweep 渐变观感（并降低 Warp 透明模式的白条眩光）

### 现象
- 为了避免 Output 动画带动整屏背景闪烁（Warp 透明模式），我们曾把动画换成更安全的方案。
- 代价是：动画不再像原来的 `sweep` 渐变；并且出现了刺眼的“白条”观感。

### 根因
- `bg=Reset` + tachyonfx `sweep_in/out`：会触发库内部把 Reset 当作 Black/White 参与插值，导致背景变动与强对比白条。
- 用户允许 pane 内部有底色，因此我们不必强行让 pane 背景也保持 Reset。

### 修复
- `crates/ralph-tui/src/theme.rs`
  - Warp 模式：app bg=`Reset`，pane bg=`base`（可读性更好、动画更柔和）。
- `crates/ralph-tui/src/animation.rs`
  - Warp 模式下 Output 重启动画恢复 `sweep_out/sweep_in`（faded_color=base），更接近原始渐变观感。
  - 动画只作用于 output inner area，避免边框 bg=Reset 参与插值。
- `crates/ralph-tui/src/app.rs`
  - 触发动画时传入 inner area。

### 验证
- `cargo test -p ralph-tui` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test` ✅
