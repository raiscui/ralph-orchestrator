# 笔记：多 Hat 并行运行设计调研

## 来源

### 来源1：现有规格（Hatless Ralph）
- 路径：`specs/event-loop/design/detailed-design.md`
- 关键摘录：
  - "Sequential hats" / "No parallel delegation"
  - "Single executor"
- 要点：
  - 现有 Hatless Ralph 设计明确以 **KISS** 为目标，假设“串行执行帽子”，不做并行。
  - 如果要“多个 hat 同时运行”，属于**突破既有约束**，需要重新定义：并行的边界、资源隔离、TUI/PTY 支持范围。

### 来源2：当前实现（EventLoop 选帽与构建 prompt）
- 路径：`crates/ralph-core/src/event_loop/mod.rs`
- 关键摘录：
  - `next_hat()` 注释：multi-hat 模式 "Always returns \"ralph\" if ANY hat has pending events"
  - `build_prompt()` 注释：multi-hat 模式 "Ralph is the sole executor, custom hats define topology only"
- 要点：
  - 当前实现把“multi-hat”做成了**拓扑/指令注入**：收集所有 pending events，算出 active hats，然后只构建 Ralph 的 prompt。
  - 代码里保留了“非 ralph hat 的 prompt 构建”分支，但注释明确这条路径在 multi-hat 下“不应该发生”。

### 来源3：当前实现（CLI 主循环）
- 路径：`crates/ralph-cli/src/loop_runner.rs`
- 要点：
  - 运行时是典型的**单循环串行**：每轮 `event_loop.next_hat()` → `build_prompt()` → `execute()` → `process_output()`。
  - 目前只构造了**一个** `CliBackend`（来自 `config.cli`），执行阶段不会按 hat 切换 backend。

### 来源4：测试场景（multi_hat.yml）
- 路径：`crates/ralph-core/tests/scenarios/multi_hat.yml`
- 要点：
  - 场景表达了 builder/reviewer 的订阅链路，但它并不等价于“真正并行执行多个 hat”。

## 综合发现（待填）

### 现状总结
- 现在的“multi-hat”更接近：**单执行器 + 多角色拓扑**，不是“多执行器并行”。
- 想实现“多个 hat 同时跑、可独立结束”，需要先澄清：
  - 你说的“同时”是**真并行**，还是**快速轮转/并发语义**？
  - 是否要求在 TUI/PTY 模式下也支持并行？
  - 全局完成条件如何定义（哪个事件表示整场 run 结束）？

### 用户新增需求（2026-01-25）
- 选择方案：**A 真并行**。
- 并行的 hats 全部是 **headless**（不依赖 PTY/交互式子进程）。
- 需要一个更高级的“上层界面”（Supervisor UI）：
  - 能列出当前运行中的 hats（以及它们的状态）
  - 能切换查看某个 hat 的界面（你希望复用/对齐“现在的 TUI”体验）
  - 上层界面需要新增一个 “agent chat” 输入/输出区域，用于更高层级的 human-in-loop

### 用户补充的并行协作场景（2026-01-25）
- **Writer/Reviewer/Tester 并行**：
  - 一个 hat 写代码的同时，另一个 hat 做检查/Review。
  - 写完后，另一个 hat 去跑测试，而写代码的 hat 继续往下做下一步（并行推进）。
- **并行探索路线**：
  - 多个写代码 hat 同时实现不同方案/不同优化路线。
  - 最后一起进入“测试/评估”阶段，再决定采用哪条路线。
- **Human in async loop（异步人类介入）**：
  - 检查者 hat 发现潜在更优思路，但需要跑测试/基准才知道是否更好。
  - 检查者可以发起“询问 human 是否尝试”的异步 chat，不阻塞它继续检查。
  - 当 human 回复“可以尝试”后，Ralph 可以再启动一个并行写代码 hat 做探索性实现。

### 用户对工作区策略的取向（2026-01-25）
- 不是固定选一个模式，而是“看情况”：
  - 改动很少时：倾向 **patch**（或必要时用文件锁）在共享工作区快速落地。
  - 改动较大、反复迭代时：倾向 **git worktree** 做隔离开发。
- 期望该策略可以在 **hat 设定**里配置（同时允许运行时按情况升级/切换）。

### 术语对齐：Supervisor vs Ralph（2026-01-25）
- 需要区分两件事：
  - **Orchestrator 进程**：就是 `ralph` 这个 Rust CLI 程序本身，会在一次 `ralph run` 期间持续运行，负责事件循环、状态、TUI 等。
  - **LLM/Agent 调用**：每个 iteration 会通过 backend（claude/kiro/等）执行一次 prompt；按现有设计是 “one iteration = one invocation”，并不是一个一直常驻的“ralph agent 进程”。
- 因此，“Supervisor”这类职责更适合落在 **Orchestrator 进程**（Rust）上：
  - 管理并行 headless hats（调度/取消/超时）
  - 汇总各 hat 的输出/事件，驱动上层 TUI
  - 维护 human async chat 的收发与关联
- 但也可以让 **Ralph（LLM 角色）**在“决策层面”承担 Supervisor 的一部分：
  - 例如它提出“需要开一个探索性 writer#2”、“需要升级到 worktree”、“需要 human 批准”
  - 真正的并发与隔离仍由 Orchestrator 执行（避免把并发复杂度塞给 LLM）

### 事件投递语义的决定（2026-01-25）
- 你选择：**C（必须显式声明）** —— topic/事件必须明确是 `queue` 还是 `fanout`。
- 你补充：对 fanout（广播）语义，希望能“针对某一事件，指定/限制哪些 hat 可以拿走”。
  - 这暗示我们需要一个“受众过滤器（audience filter）”机制：
    - 可能是 hat 类型白名单（reviewer/tester 才能消费）
    - 也可能是更细的 selector（按实例标签/能力/优先级）
  - 这一点会直接影响 Event 数据结构、EventBus 路由算法，以及 replay 测试的确定性。

### 架构方案选择（2026-01-25）
- 你选择：**方案1（HatInstance Actor 模型）**。
  - 并行不仅支持“同一 hat 多实例”，也支持“不同 hat 并行”。
  - 并发基本单位 = `HatInstance`（每个实例是一个独立可调度主体），实例内部串行执行 job，实例之间并行。

### 受众限制粒度（2026-01-25）
- 你选择：受众限制做到 **2（按实例）**。
  - 即：fanout / queue 的目标选择不仅能写 `reviewer`，也能写 `reviewer#2` 这种实例级 ID。

### CLI 执行模型的决定（2026-01-25）
- 你选择：**A（一份 job = 一次 CLI invocation）**。
  - `HatInstance` 作为 tokio task/actor 只负责调度与状态机。
  - 真正干活的是被 spawn 出来的 CLI 进程（codex/claude code/…）。
  - 这样并行的本质就是“多个 CLI 子进程并行运行”，更符合 headless 并发的可控性与可回放性。

### 可否由 LLM 决策？（2026-01-26）

你问“可否由 LLM 决策”。
我认为可以，而且应该让 LLM 参与决策。

但在并行系统里，更稳的落地方式是：

- **LLM 只负责提议**（策略/路线/是否升级成本）
- **Supervisor 负责校验与执行**（capabilities/permissions/human gate/机械执行/全局仲裁）

这样做能同时满足：

- 你要的“LLM 驱动为主”
- 并行系统必须具备的“可控、可回放、可追责（决策落盘）”

并且能自然对齐你已确认的权限模型：

- LLM 可以提议触发权限条目 1-5
- 但不能绕过 Permission Gate（默认 allow，但可随时切 ask/deny）

关键工程约束（为 replay 服务）：

- LLM 的决策必须显式编码成事件（`decision.request` / `gate.request` / `decision.result` / `gate.resolve`）
- 这些事件必须写入 `events.jsonl`，replay 时直接回放，不重新问 LLM

### queue 派发由 LLM 决策（2026-01-26）
- 你选择：`queue` 语义下由 LLM 决定“投递到哪个具体实例”（而不是 round-robin/least-busy）。
- 关键约束：派发决策必须落盘（候选集 + 选择结果 + 可选原因），replay 时不重新决策。
- 允许兜底：当 LLM 不可用/超时/成本受限时，可回退 deterministic 算法，但同样要落盘。

### human gate 支持超时（2026-01-26）
- 你希望：LLM 可以通过 human gate 寻求决策，并可选择两种 gate：
  - 普通 gate：等待 human 回复
  - 超时 gate：等待最多 60s，超时后由 LLM 自行决策
- 工程约束：必须把最终 `gate.resolve` 写入事件日志，replay 不等待、不重问。

### human 可随时异步调整需求（2026-01-26）
- 你希望：human 可以随时 async 发送“调整需求/新约束”，不阻断并行 hats 的运行。
- 你倾向用文件系统事件/日志做通道，并希望 LLM 能经常读取。
- 推荐落地：
  - `events.jsonl` 作为唯一真相（可回放）
  - 每个 HatInstance 维护轻量 inbox 文件（例如 `.agent/inbox/{instance}.jsonl`），方便 LLM 高频读取
  - 默认不打断（你已确认）：`priority=normal` 只在安全点应用；`priority=urgent` 才允许 cancel 并重启

### LLM 决策层怎么落地（2026-01-26）
- 你发现“orchestrator 不调用 LLM 做评审”，这在现状下是对的：multi-hat 只是拓扑注入。
  - `EventLoop::next_hat()` 在 multi-hat 时会“总是返回 ralph”，所以 reviewer/tester 并不会真的执行。
- 目标态（方向1 HatInstance Actor）会推翻这个限制：每个 HatInstance 都能真正执行一次 headless CLI invocation。
- LLM 决策层不建议在 Rust 内接 SDK，而是复用现有模型：**LLM = 外部 CLI agent**。
  - 把“派发决策 / gate 超时自决”等变成“决策类 HatJob”
  - 通过 `CliExecutor` spawn CLI 进程，要求输出结构化 `<event ...>`，由 `EventParser` 解析并落盘，replay 时直接回放。

### `ralph` 这个 hat 从哪来？（2026-01-26）
- **是内置的、默认永远存在的。**不是你在 YAML 里声明出来的那种 hat。
- 代码里在 EventLoop 初始化时“无条件注册”：
  - `crates/ralph-core/src/event_loop/mod.rs:145`：`ralph_proto::Hat::new("ralph", "Ralph").subscribe("*")`
  - 并且注释明确写了 “Hatless Ralph is constant — Cannot be replaced, overwritten, or configured away”
- 现状下，即使你配置了自定义 hats，multi-hat 也仍然会把执行者固定为 `ralph`（拓扑注入，不是真并行）。

### LLM 决策层默认用 `ralph` hat（2026-01-26）
- 你已确认：第一版不新增 `decider` hat，决策类 job（queue 派发、gate 超时自决等）默认以 `hat_id="ralph"` 执行。
- 这不等于“Rust orchestrator 内置调用 LLM SDK”，仍然是通过 headless CLI agent invocation 完成。
- prompt 需要区分：
  - `ralph(work)`：现有 Hatless Ralph 协调/规划
  - `ralph(decision)`：决策专用 prompt（只读、强结构化输出 `<event ...>`）

### human async chat 使用 `ThreadId` 路由（2026-01-26）
- 你确认：human async chat 的路由主键使用 `ThreadId`（长生命周期），而不是直接绑定实例 ID。
- `@writer#2` 之类只作为 UI 层便捷别名，实际会解析到某个 thread owner（或提示选择/创建 thread）。
- 好处：
  - 实例消亡/重启/owner 迁移不会导致对话丢失
  - human async loop 更贴近“工单/会话”而不是“进程”

### `audience_override.instances` 默认 best-effort（2026-01-26）
- 你确认：点名实例（例如 `audience_override.instances=["writer#2"]`）默认是 **best-effort**。
- 指定实例不存在时，不视为失败：
  - 按 `missing_instance_policy` 处理（spawn/queue/escalate/drop）
- 如果某次需要“必须送达”，事件可显式声明 `audience_override.require_delivery=true`，送不到就 `escalate`。

### hooks 失败后的自愈策略（2026-01-26）
- 你确认：`on_acquire/on_release` hook 失败后，默认让 LLM 先判断并尽量自我修复，而不是立刻失败。
- 推荐机制：
  - 先发布 `workspace.hook_failed`（带阶段/attempt/退出码/输出）
  - 再启动 `ralph(decision)` 做恢复决策（retry/repair_then_retry/escalate/abort），Supervisor 机械执行
  - bounded 重试：默认建议 `max_attempts=3`（含首次），超过就 abort 当前 job（不阻断其他 hats）

### workspace 决策的关键矛盾与解法（2026-01-25）
- 矛盾：hat 在执行前不知道 CLI 会改哪些文件、改多少，无法先验决定“patch vs worktree”。
- 解法方向（更新后更贴合你的偏好）：**LLM 预判为主 + orchestrator 观测兜底**
  - 任务前：LLM 做 preflight 难度评估，决定本次 job 是 `patch/shared` 还是 `worktree`
  - 执行后：orchestrator 用 `git diff / git status` 做客观观测
    - 用于生成 patch/commit 工件、汇总 UI、以及驱动合并/校验流程
    - 不把“submodules 要不要 init”内置进 orchestrator（交给 worktree hooks）

### 用户对 worktree 性能/子模块的担忧与策略（2026-01-25）
- 担忧：`git worktree` 创建如果伴随 `submodules` 初始化，可能非常慢，且可能受网络问题影响。
- 你的偏好：
  - 任务前让 **LLM 预判任务难度**，决定是否需要 worktree。
  - 选择 **B：每个 job 一个“临时 worktree”**（用完合并/校验后清理）。
  - 不是任何任务都能创建 worktree：需要在 **hat 设计/配置**里授予该 hat “可创建 worktree”的能力；然后由 LLM 决定本次是否启用。
  - worktree 任务完成后的 **合并与校验** 可以定义专门的 hat 来负责（例如 Integrator/Verifier）。

### “引用不存在实例”的处理思路（2026-01-26）
- 你指出的现象：并行 + 实例可结束 + human async loop 下，事件可能指向“不存在的实例”（例如 `writer#2`）。
- 推荐解法（已写入 spec）：
  - 不用一个全局 A/B/C 写死。
  - 把“目标引用”拆成两类：
    - 短生命周期：`HatInstanceId`（适合 cancel/kill 等控制类）
    - 长生命周期：`ThreadId/WorkItemId`（适合 human async chat 与跨 job 讨论）
  - 对缺失实例的处理，按消息类型给默认策略，并把路由决策写入事件日志，保证 replay 可复现。
