# CONTEXT.md — ralph-orchestrator 领域词汇表

> 架构讨论使用的领域语言。新增/修改术语时更新这里。

## 核心概念

- **Hat**: 可被事件触发的智能体角色。订阅事件、发布事件。
- **事件总线 (EventBus)**: 事件 pub/sub 通道,Hat 与协调者之间唯一的通信机制。
- **协调者 (Ralph / HatlessRalph)**: 常驻协调者,不能被配置关闭,作为兜底路由。
- **事件循环 (EventLoop)**: 串行模式的编排循环;终止原因枚举 `TerminationReason` 是循环语义的一部分。
- **Supervisor**: 并行模式的调度/路由核心(在 ralph-core::parallel)。
- **Workspace**: Hat 运行的隔离目录。

## 展示域 (ralph-display,2026-08-01 建立)

- **StreamHandler**: "进程输出 → 展示" 的 seam。适配层(`ralph-adapters`)只依赖这个 trait,展示实现全部在 `ralph-display`。
- **DisplayTarget**: 调用者对展示的意图(控制台 / TUI),合法组合由类型保证;选择矩阵收在 `make_stream_handler` 工厂里,不再泄漏给调用者。
- **MarkdownRenderMode**: Rendered(隐藏控制符)/ Plain(控制符可见)。
- **DisplayVerbosity**: Quiet / Normal / Verbose,工厂据此选 QuietStreamHandler / Pretty / Console。

## 记录域

- **Record session**: 每次 run 的 JSONL 证据流;`_meta.termination` 是终止契约的一部分。
- **Evidence index**: 从 record session 聚合出的结构化证据视图。

## Job 执行域 (ralph-adapters::job,2026-08-01 建立)

- **HatJobExecutor**: core 定义的 port(编排 → 进程执行 seam);`HatJob`/`HatJobResult`/`HatJobOutputChunk`/`HatJobControl` 是契约类型。
- **CliHatJobExecutor**: 选择器,按 backend / session_strategy 路由到三种形态(app_server > mcp > headless)。
- **headless**: 一次性 CLI 进程执行;只消费 stdout 做事件解析,stderr 仅可观测。
- **app_server**: Codex App Server 常驻会话(turn/steer/interrupt)。
- **mcp**: Codex MCP 常驻会话(不支持 in-flight steer)。

## 记录域更新 (2026-08-02)

- **record_aggregate** (ralph-core): record-session 的 strict 解析 + 聚合;窄入口 `aggregate_session(path)`;与运行时写入域的 `evidence_index` 区分。
- **聚合 vs 渲染**: 聚合(结构化)在 core,渲染(Evidence Inspect 文本)在 ralph-cli。

## TUI 域更新 (2026-08-02)

- **TuiState 领域切片**: `state/{radar,output,task,search}.rs`,每片独立 struct + 自治方法;壳只做跨域协调与兼容委托。
  - RadarSlice: 可视化状态机(running_hats 由壳注入,不依赖 parallel 域)
  - OutputSlice: 串行输出缓冲/浏览/选择
  - TaskSlice / SearchSlice: 纯状态 + 纯算法
- 兼容委托: 原 82 个方法签名保留(一行委托),调用者渐进迁移。

## 上游同步域 (2026-08-15 建立,sync/origin-v2.10.1)

- **ADR**: 架构决策记录,落在 `docs/adr/`。本仓库首条 ADR-0001 = "Cherry-pick upstream sync instead of merge"。涉及 hard-to-reverse 或 surprising-without-context 的决策时新建。
- **sync/origin-vX.Y.Z 分支**: 跟 upstream 单次 release 整合的 feature branch,落 `my/main` 后删除;每 wave push 一次 (4-5 pushes/release)。
- **Completion gate** (#326): `event_loop` 在 guidance 之后**强制显式 completion** —— 不能再靠模型 "假装完成"。直接影响 ralph#1 的终止判定,补的是 LOOP_COMPLETE 漂移护栏。

## CLI surface 扩展 (2026-08-15)

- **`ralph clean --events`** (#357): 只清理事件文件,不删 record-session / 诊断日志。适合 replay 跑完后只想回收 .ralph/events.jsonl 的场景。
- **Per-hat scratchpad 注入** (#293-scratchpad fix): `instructions` 模块生成 hat 指令时 honor per-hat scratchpad 模板(不是 global scratchpad)。

## Provider 域 (2026-08-16 建立)

- **minimax provider**: OpenAI codex CLI 的子集 wrapper,只透传认识的 flag。完整矩阵见 `docs/solutions/minimax-full-auto-compat/README.md`。
- **`--sandbox danger-full-access`**: minimax 兼容的"无沙箱 + 全权"等价于 OpenAI CLI 的 `--full-auto` 组合 flag。
- **minimax live E2E 凭据**: `.e2e-tests/parallel-*-instances*/` workspace 目录4,events.jsonl + agents.json + ralph.yml 完整保留。

## Hat imports 域 (2026-08-16 建立)

- **`imports:` 键**: `hats:` block 内每个 hat 可声明的本地相对路径;值必须是 string,。详细 schema 见 `specs/hat-imports/design.md`。
- **`HatImportError`**: `crates/ralph-core/src/hat_imports.rs` 的错误类型;`Hat` (解析/读文件/YAML/merge 失败) + `UnsupportedSource` (builtin/remote source 不允许 imports)。
- **预解析模式**: `RalphConfig::from_file` 先把 YAML parse 到 `serde_yaml::Mapping`,resolve imports,序列化回 string,再 `parse_yaml` 到 `RalphConfig`。`parse_yaml(content: &str)` 自身不变。
- **限制**: 本地 file source only。Builtin / remote / override source 必须在它们自己的 source 入口调 `reject_hat_imports_in_mapping`。
- **不接受**: 传递性 imports (A → B → C)、imported `events:` 字段、非 string `imports` 值、缺失文件、YAML 错误。

## Event topic 扩展 (2026-08-17 建立)

- **`human.guidance`**: 操作员注入的中途指引;payload 推到 `LoopState::unacknowledged_guidance` 队列,阻止下一轮完成信号。
- **`human.guidance.ack`**: 清空队列,允许完成;无 payload 必填要求。
- **`unacknowledged_guidance`**: `LoopState` 新字段,默认空 `Vec<String>`。
- **completion guard**: 完成检测通过 verification 后, 若 `unacknowledged_guidance` 非空, reset `completion_confirmations = 0` 并 publish `task.resume`。`这跟本地 2-strike pattern + lazy-model-completion (complete_publishes) 正交`。
- **`default_publishes` collision guard**: 当 `default_publishes == completion_promise` 时, `check_default_publishes` 不沉默注入,改为 publish `task.resume` 提示显式 evidence。

## Q3 plan 整合状态 (2026-08-17)

- **Group 1**: 全部 DONE (1.1 manual port, 1.6 partial port, 1.2-1.5 dropped / rewritten to Group 4 §1-§4)。
- **Group 2**: 6/6 dry-run CONFLICT (2026-08-12),全部 moved to Group 4 rewrite §5-§8。
- **Group 3**: 5/5 dry-run CONFLICT (2026-08-17),3.2 (ee9fa67) DROP (已 manual port), 其余 → Group 4 §15-§18。
- **Group 4 §15** (4a38b8d Claude stream wait): **DROPPED 2026-08-17**。origin 用 `StreamEvent` enum + `line_signals_event_emitted` / `post_event_deadline` 逻辑,本地 `(StreamKind, line)` tuple 不存在这些。Porting = 发明需求 + 加 60+ 行条件逻辑给 Claude stream JSON (本地不跑)。Per 改良胜过新增,DROP。
- **Group 4 §16** (25afeb0 local hat imports): **DONE 2026-08-16** (commit `ef6d83e1`)。实现见 `crates/ralph-core/src/hat_imports.rs`,` `design.md` 文档。
- **Group 4 §17** (a4b6d45 explicit completion after guidance): **DONE 2026-08-17** (commit `7de0d939`)。实现见 `crates/ralph-core/src/event_loop/{loop_state,mod}.rs`,` `specs/human-guidance/design.md` 文档。
- **Group 4 §18** (d631ef7 context window telemetry): 16 文件 massive, 涉及 proto/adapters/event_loop + frontend React。建议开新 OpenSpec change,不在当前 change 内做。
- **Round 4 (本文档同步)**: 2026-08-17 完成。

## Architectural drift 提示

- **Provider 兼容矩阵的"非发明需求"原则**: origin 引入某 flag 不等于本地需要该 flag。本地 minmax 不跑 Claude stream JSON,所以 `--full-auto` 的 Claude result-event wait 不落地 (Group 4 §15)。通用规则:port 一个 fix 前确认本地有该 use case。
- **Hat imports 的 schema-first 约束**: `HatConfig` Rust struct 不变,所有新逻辑在 `serde_yaml::Mapping` 空间。这样未来 origin 升级 HatConfig 时不会冲突。
- **Completion 信号的多源组合**: 当前有 3 条独立 termination 路径, 必须正交协同:
  1. **`completion_promise` + 2-strike pattern** (主): 本地基本 termination 信号
  2. **`complete_publishes` (lazy-model-completion)**: supervisor 硬终止信号, lazy model 不写 LOOP_COMPLETE 时启用
  3. **`unacknowledged_guidance` guard**: 操作员中途指引未 ack 时拒绝完成
  任何一条单独都不够;运行时三者都参与判定,详见 `event_loop/mod.rs` `process_output`。

## Wave 3.4 收尾 + §19 Round 6 (2026-08-17, commit `ee73fcf8` + `03fab390`)

### declarative coverage gate:实际状态

- **当前真实值:100.00% (PASS)** —— 不是 handoff 写的 63.93% (那是 2026-08-13 快照,Wave 2 之前)。
- 注册表 `crates/ralph-e2e/src/lib.rs::all_scenarios()` 始终包含:
  - 60 个 `ScenarioKind::Declarative` (走 `declarative::from_yaml()`, 与 `scenarios/*.rs` 里的 Rust TestScenario impl 无关)
  - 0 个 `ScenarioKind::Imperative`
  - 1 个 `ScenarioKind::ImperativeExplicitKeep` (`parallel-experimental-dev-engine-example`)
- 阈值 `THRESHOLD = 0.90` 在 `crates/ralph-e2e/tests/declarative_coverage_gate.rs` 里硬编码 (无 env override 路径)。
- 验证命令: `cargo test -p ralph-e2e --test declarative_coverage_gate -- --nocapture` —— drift log 自动打印。

### `#[deprecated]` (= dead code) 判定规则

- **规则:Wave 3.4 之后, E2E crate 里出现的任何 `#[deprecated(since = "2.3.0")]` struct 都是 dead code**, 因为:
  1. `all_scenarios()` 只通过 `from_yaml()` 注册 declarative scenario,完全不走 Rust TestScenario impl;
  2. 22 个 `#[deprecated]` 标记 (capabilities/errors/hats/memory/parallel/app_server_*) Round 6 已经物理清零;
  3. future deprecation: 如果再出现一个 `#[deprecated]` 标记,默认假设它是 dead,先查 `all_scenarios()` 是否注册,再决定是否保留。
- 防止机制:
  - Gate test `explicit_keep_is_exactly_parallel_experimental_dev_engine_example` 保证 `ImperativeExplicitKeep` 永远只有 1 项,新增必须 `audit-p5-p1.md §A.5` justify。
  - 编译警告数 = 0 (Round 6 净效应),新增 `#[deprecated]` 会被立刻看到,而不是藏在 297+ 的噪声里。

### 5 个 "非 deprecated 但 legacy" .rs 文件(待决策,不阻塞)

`connectivity.rs` (360) / `events.rs` (722) / `orchestration.rs` (956) / `incremental.rs` (1016) / `tasks.rs` (811) 共 ~3865 行。这些 struct **不在 `#[deprecated]` 标记里**,但 `all_scenarios()` 也不引用 —— 同样的 dead code 模式,但因为 `lib.rs::pub use` 还在 export 它们,物理删除前需要决定是否 binary/main 还依赖它们。结论:binary main.rs 不引用 → 可清。Round 6 没动是因为决策未一拍即合,留 follow-up。
