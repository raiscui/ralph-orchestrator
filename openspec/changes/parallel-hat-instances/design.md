## Context

### 背景

本 change 的动机来自现有 Event Loop 的 KISS 约束与实现现状：

> - "Sequential hats"
> - "No parallel delegation"
> - "Single executor"

这导致即使配置了多个 hat，本质上也仍是“单执行者串行调度”，并不会出现 reviewer/tester/decider 这类帽子真正并行跑的效果。

### 当前状态（现状约束）

- **执行者固定**：内置 `ralph` hat 是 catch-all coordinator，现状 multi-hat 更像“拓扑/指令注入”，而不是独立执行器。
- **并行不可落地**：没有多执行器/多实例的生命周期与资源隔离模型，无法把“并行”做成一等能力。
- **可靠性护栏必须保留**：Ralph 的核心价值在于可回放（replay）、可观测（events/diagnostics）、以及用 backpressure 拒绝低质量工作流。

### 约束与干系人

- 约束：
  - orchestrator 仍保持“薄协调层”，不演化成平台。
  - 并行 hats 全部是 **headless**（并行的本质是多个外部 CLI agent 子进程并行跑）。
  - 关键决策必须落盘到事件日志，确保 replay 不重算。
- 干系人：
  - CLI/TUI 用户（希望吞吐提升、状态可见、可控）。
  - hat 设计者（需要声明能力/权限/工作区策略，并能被验证与回放）。
  - CI/测试（需要确定性与可回放的 smoke fixtures）。

## Goals / Non-Goals

**Goals:**

- 真并行：不同 hat 可并行、同一 hat 可多实例并行，并允许实例独立结束。
- 可回放：并行下的路由/选择/审批等决策必须事件化，回放只读 events 复现结果。
- 可控：引入 human gate（等待/可选超时），让人类能在 async loop 中持续施加约束。
- 可用：Supervisor 能汇总实例状态与输出，提供可追踪的运行期视图（TUI/日志均可）。
- 可扩展：workspace 隔离策略与权限能力可配置，避免并行写冲突与“无意改坏仓库”。

**Non-Goals:**

- 不把 LLM SDK 直接接进 Rust orchestrator（仍以外部 CLI invocation 为执行单元）。
- 不做多 TUI/多 PTY 并行展示（并行执行 headless，展示由 Supervisor 汇总）。
- 不追求与旧的“假并行”行为完全兼容（该 change 允许 **BREAKING** 行为调整）。

## Decisions

### 1) 并行执行的基本单位：HatInstance Actor + HatJob

**Decision：**每个 HatInstance 作为一个 tokio actor（拥有状态机与 inbox/outbox），每个 job = 一次 headless CLI invocation。

**Rationale：**

- actor 模型天然适合“独立生命周期 + 可并发 + 可观测”的实例管理。
- job 作为外部进程，让后端（codex/claude code/…）保持可插拔，避免 orchestrator 绑定具体 LLM SDK。

**Alternatives：**

- 线程池/任务队列直接并发执行：实现简单，但实例生命周期与状态聚合会散落，难以做统一 backpressure 与 replay。
- 在 Rust 内集成 LLM SDK：可减少进程开销，但会把 orchestrator 变重，并引入凭据/网络/供应商耦合风险。

### 2) 事件路由语义：显式 queue/fanout + 受众可约束

**Decision：**Topic 投递语义必须显式声明 `queue | fanout`，并支持实例级受众限制（例如 `audience_override.instances=[...]`）。

**Rationale：**

- 显式语义能避免“到底谁会收到消息”的隐式规则，便于调试与回放。
- 受众约束能让 human/decider 精确地把事件送到目标实例，减少跨实例噪音。

**Alternatives：**

- 全部 broadcast：最简单，但并行规模一大就不可用（噪音与成本爆炸），且难以保证 deterministic replay。

### 3) Replay 硬门槛：决策必须落盘

**Decision：**所有影响路由与执行路径的决策都必须写入事件日志（例如 queue 选择结果、gate 超时自决结果、missing instance 的处理分支）。

**Rationale：**

- 这是“Fresh Context Is Reliability”的工程落点：回放不依赖 LLM 再次判断，避免漂移。

**Alternatives：**

- 回放时重算（重新问 LLM/重新做选择）：成本高且不确定，会破坏 smoke tests 的确定性。

### 4) Human gate：事件化的请求/解析 + 可选超时

**Decision：**human gate 用事件协议表达（request/resolve/timeout），默认不阻塞其他 HatInstance；允许设置超时后由决策型 job 继续推进并落盘。

**Rationale：**

- 并行系统里“等待人类”必须是可组合的，不应把整个系统卡死。
- 超时自决是成本/速度的刹车，但必须透明（落盘）且可追溯。

**Alternatives：**

- 纯同步阻塞式审批：实现简单，但会把并行退化为串行，破坏吞吐与体验。

### 5) Workspace 策略：共享优先 + 需要时 worktree 隔离

**Decision：**默认共享工作区（小改动走 patch/必要时文件锁），当 hat 具备能力且预判为“大改动/高风险”时，按 job 创建临时 worktree 隔离。

**Rationale：**

- worktree + submodules 可能很慢且受网络影响，不应成为默认路径。
- 共享优先能保持轻量；worktree 作为“需要时的隔离”，由能力白名单与决策共同控制。

**Alternatives：**

- 永远 worktree：隔离强，但性能与复杂度开销大，且对 submodules/钩子要求更高。

### 6) Backpressure：用验证门槛拒绝坏结果

**Decision：**保持“以验证为导向”的 gate：并行带来的不确定性通过 tests/typecheck/smoke replay fixtures 来兜底。

**Rationale：**

- 让系统“拒绝不可靠”，而不是试图把每种情况都写成复杂重试逻辑。

## Risks / Trade-offs

- [并行导致回放不确定] → 关键决策事件化落盘；回放只读事件，不依赖运行期调度顺序。
- [TUI/日志输出混杂难读] → 每实例独立输出缓冲与标记（instance id / seq），Supervisor 负责聚合与过滤。
- [并行写冲突/脏工作区] → workspace 策略分层（共享/锁/临时 worktree）+ capabilities/permissions 约束。
- [worktree + submodules 过慢] → 默认不启用 worktree；用 hooks 作为可选能力；失败时 bounded retry + escalate。
- [系统复杂度上升] → 先以并发度=1 落地框架，再逐步打开并发；用 smoke fixtures 固化行为。

## Migration Plan

1. **框架先落地**：引入 HatInstance/Job/事件路由框架，但并发度默认=1，行为接近现状。
2. **并发逐步打开**：先允许两个不同 hat 并行，再允许同 hat 多实例。
3. **补齐护栏**：补齐 replay-based smoke fixtures，确保并行行为可回放、可验证。
4. **默认策略收敛**：确定 workspace 默认策略与 gate 默认超时策略，减少“配置陷阱”。

Rollback 思路：将并发度降回 1 或关闭并行开关（如果实现为配置项），即可快速回退到串行执行路径。

## Open Questions

- `tasks` 生成前：`specs` 工件将落在 `openspec/specs/`，与仓库现有 `specs/parallel-hat-instances.spec.md` 的关系如何收敛（复制/拆分/以谁为准）？
- worktree 任务完成后的合并与校验：是否需要专门的 Integrator/Verifier 角色，还是由 Supervisor 机械执行 + 决策型 job 仲裁？
- TUI 呈现：实例列表/详情/输出的交互模型最终选型（tab vs split vs filter）与最小可用范围。
