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
