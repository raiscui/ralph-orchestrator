## Context

### 背景（现状）

并行运行时（`parallel.enabled=true`）已经落地：

- `ralph-core` 负责并行调度（Supervisor + HatInstance actors）。
- `ralph-cli` 的并行入口目前仅提供 **日志模式**：
  - `--tui` 会提示 “Parallel mode currently runs without TUI (log output only)”。
  - 输出以 `[instance_id:out] line` 的形式落到 stdout，便于基本排障，但交互性不足。

与此同时，仓库已经存在成熟的串行 TUI：

- `crates/ralph-tui` 提供 buffer/滚动/搜索等体验。
- `crates/ralph-cli/src/loop_runner.rs` 已经实现 “启用 TUI → 启动 UI task → 通过 observer 增量更新 state” 的成熟模式。

而 `specs/parallel-hat-instances.spec.md` 的 8.x 已经把 Supervisor TUI（实例列表/实例详情/human chat + gate）写成了明确草案。
现在要做的是把这个草案变成工程现实，并且与并行运行时的“可回放/可观测/背压”约束一致。

### 关键约束

- **改良胜过新增**：优先复用/扩展现有 `ralph-tui`，避免另起炉灶导致两套 UI 心智分裂。
- **并行仍是 headless**：TUI 只是 Supervisor 的“观测与输入面板”，不引入多 PTY 并发交互。
- **可回放不退化**：TUI 产生的人类输入必须落盘为事件（写入 `.ralph/current-events` 指向的 JSONL），以便复现与排障。
- **不把 orchestrator 变平台**：UI 只消费“已存在的信号”（输出 chunk、实例状态、gate 事件），不在 UI 内引入复杂调度逻辑。

## Goals / Non-Goals

**Goals:**

- 并行模式真正启用 Supervisor TUI（TTY 环境），替代当前“只能日志看并发”。
- UI 结构满足三块核心面板：
  1) HatInstance 列表（状态/最后输出时间等）
  2) 实例输出详情（滚动/搜索/按 job 历史切换）
  3) human async chat + gate 面板（展示 gate、approve/deny/resolve、倒计时）
- UI 输入与 gate 操作以事件形式落盘，不阻塞并行实例运行。

**Non-Goals:**

- 不实现多路交互式 PTY（多实例同时可输入）。
- 不引入新的外部依赖（除非确实缺少能力且收益明显）。
- 不改变并行调度语义（路由/扩缩容/权限等仍由 Supervisor 负责）。

## Decisions

### 1) UI 架构：在 `ralph-tui` 内新增“并行模式”

**Decision：**在 `crates/ralph-tui` 内引入并行模式的 state/widgets/input，而不是新建一个独立 TUI crate。

**Rationale：**

- 复用现有的 buffer/搜索/滚动代码，避免重复实现。
- 让用户在串行/并行之间切换时，核心交互（`/` 搜索、滚动、history）保持一致。

**Alternatives：**

- 新建 `ralph-supervisor-tui`：短期改动小，但长期必然出现“两个 TUI 分叉”，维护成本更高。

### 2) 并行 TUI 的数据模型：instance → jobs → buffer

**Decision：**按 spec 8.1 的建议，将并行 TUI 的内容组织为：

- `instances: HashMap<HatInstanceId, InstanceViewState>`
- `InstanceViewState.jobs: Vec<JobBuffer>`（每个 HatJob 一段输出）

同时保留串行模式的 iteration 结构，避免破坏既有路径。

**Rationale：**

- 并行语义里，“全局 iteration”不再是最自然的浏览维度。
- instance/job 是用户排障与理解并发行为的最短路径。

**Alternatives：**

- 先只做 instance 维度的单 buffer（不分 job）：能更快落地，但很快会遇到“输出混在一起难回看”的体验瓶颈，需要二次重构。

### 3) UI 与并行运行时的连接方式：observer → channel → state reducer

**Decision：**延续串行模式的成熟模式：

- `ralph-cli` 在并行 runner 中启动 TUI task。
- 通过 supervisor 的 observer（输出 chunk、实例状态、事件）把更新送入一个 channel。
- `ralph-tui` 在 UI 线程里消费 update stream，并用 reducer 更新 state（避免跨线程直接改 UI state）。

**Rationale：**

- 并行输出高频，必须用“批量/节流 + 单写者 state”避免锁竞争与渲染抖动。
- 复用既有 `Tui::run()` 的生命周期管理与 Ctrl-C 退出路径。

**Alternatives：**

- UI 直接 tail `.ralph/events*.jsonl`：实现表面简单，但会引入去重/乱序/延迟问题，且 gate 需要额外状态机才能做到“当前待处理 gate 列表”。

### 4) human 输入落盘：直接追加到 `.ralph/current-events` 指向的 JSONL

**Decision：**TUI 内部实现一个轻量的 “ExternalEventWriter”：

- 读取 `.ralph/current-events` 的 marker 得到目标 JSONL 路径（与 Supervisor 读取逻辑一致）。
- 以“追加写 + flush”方式写入一行事件 JSON（等价于 `ralph emit` 的效果）。

**Rationale：**

- 这是并行 Supervisor 已经明确支持的扩展点（human / tools 通过 JSONL 注入事件）。
- 不需要额外 IPC，也不需要 spawn 子进程去调用 `ralph emit`。

**Alternatives：**

- TUI 里 shell out 调用 `ralph emit`：实现慢、依赖 PATH、错误处理复杂，还会污染输出。

### 5) Gate 面板的数据来源：消费 gate.* 事件并在 UI 内维护视图态

**Decision：**并行 TUI 通过 “event observer” 接收 `gate.request/gate.timeout/gate.resolve`，并在 UI state 内维护：

- `open_gates: Vec<GateView>`
- `resolved_gates`（可选，供历史查看）

**Rationale：**

- gate 的真相来源是事件流；UI 只需做视图层聚合。
- 保持“人类输入不阻塞实例”的核心约束：UI 只是写 `gate.resolve`，Supervisor 再把 resolve 路由回实例。

**Alternatives：**

- UI 直接读 Supervisor 内部 gate 状态：耦合更强，不利于将来把 UI/运行时解耦。

## Risks / Trade-offs

- [串行/并行 TUI state 分裂] → 用 `TuiMode::{Serial, Parallel}` 明确分支；公共组件（搜索/滚动/文本 buffer）抽象复用。
- [高频输出导致渲染卡顿] → update channel 合并/节流（例如 16ms tick 批量 apply），并对 buffer 做长度上限（ring buffer）。
- [事件文件并发写读] → 采用追加写（append）并 flush；写失败必须在 UI 中可见（提示用户事件未送达）。
- [job 边界不清晰] → 在并行运行时补齐 job 生命周期信号（job_start/job_end 或在 chunk 中携带 job_id），以最小字段改动实现 UI 分段。

## Migration Plan

1. 在并行 runner 中接入 TUI 生命周期（先能启动并显示实例列表）。
2. 接入输出 chunk 与实例状态更新（完成列表 + 详情输出可用）。
3. 增加 chat 输入框，先实现 `human.message`（含 `@instance` 定向）。
4. 增加 gate 面板（展示 gate + 发送 gate.resolve）。
5. 最后补齐 job 分段与 job 历史切换、以及 `/tui-validate` 的回归验证。

## Open Questions

- job 边界信号的最小字段方案：是给 `HatJobOutputChunk` 增加 `job_id`，还是新增 `HatJobStarted/Ended` 更新事件？
- `human.message` 的消费方：短期只作为“可投递/可落盘”信号；未来是否需要标准化 payload 结构（thread_id 等）？
