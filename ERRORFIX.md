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
