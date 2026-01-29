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

## 补充：TUI 视觉回归工具（freeze）的安装坑（2026-01-28）

- `docs/advanced/testing.md` 与 `.claude/skills/tui-validate/SKILL.md` 推荐 `brew install charmbracelet/tap/freeze`。
- 但如果你的 shell 环境里设置了 `http_proxy/all_proxy` 指向本机代理（例如 `127.0.0.1:7897`），并且当前代理未启动，brew 的 `git clone` 会直接失败。
- 一个可行的替代方案是用 Go 安装（更容易走 Go proxy）：
  - `GOPROXY=https://goproxy.cn,direct GOSUMDB=sum.golang.google.cn go install github.com/charmbracelet/freeze@latest`

## 补充：delta specs 已同步到主规格（2026-01-28）

- 之前 `openspec/specs/` 为空，会导致“实现已经落地，但主规格缺失”的长期文档漂移风险。
- 现在已把相关 changes 的 delta specs 同步到主规格目录（后续新增 scenario 可做增量 merge）：
  - `openspec/specs/parallel-hat-instances/spec.md`
  - `openspec/specs/parallel-trigger-routing/spec.md`
  - `openspec/specs/parallel-supervisor-tui/spec.md`
  - `openspec/specs/supervisor-human-chat-gate/spec.md`

## 补充：spec 之间的语义对齐（2026-01-28）

### 为什么需要对齐
- `parallel-trigger-routing` 是“默认路由语义”，而 `parallel-hat-instances` 引入了更通用的 `TopicContract/queue_selection` 模型。
- 如果把两份 spec 当成“同一层级的强约束”，会出现表述冲突（例如是否必须配置 TopicContract、实例选择是否只能 deterministic）。

### 对齐后的约定（主规格）
- 系统 **总是 resolve 一个 TopicContract**，只是来源不同：
  - 配置命中（`parallel.topic_contracts`）→ 用配置的
  - 配置未命中 → 用 triggers 派生的默认 TopicContract（hat-level fanout + instance-level queue）
- 实例选择规则：
  - deterministic（默认）→ idle-first + deterministic tie-break
  - llm（可选）→ 允许非确定选择，但必须落盘候选集+选择结果以支持 replay

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

### /opsx:new 起步检查（2026-01-26）
- 我已确认本机 `openspec` CLI 可用：`/Users/cuiluming/n/bin/openspec`。
- 当前仓库 `openspec/changes/` 目录只有 `archive/`，尚无活跃 change。
- 后续创建 change 时，以本仓库的实际子命令为准（预计是 `openspec change new "<name>"`），避免照抄旧文档导致命令不匹配。

### /opsx:new：spec-driven schema 的工件序列（2026-01-26）
- 本仓库 `openspec/config.yaml` 的默认 schema：`spec-driven`
- `openspec status --change "parallel-hat-instances"` 显示的工件依赖关系：
  - `proposal` → 解锁 `design` 与 `specs`
  - `design` + `specs` → 解锁 `tasks`
- 当前首个可写工件是 `proposal`，其模板由 `openspec instructions proposal --change "parallel-hat-instances"` 输出。

### /opsx:continue：design 工件模板要点（2026-01-26）
- `design` 工件（`openspec instructions design --json`）要求的核心段落：
  - Context / Goals / Non-Goals / Decisions / Risks
  - 并建议补充 Migration Plan 与 Open Questions（有助于后续实现前对齐）
- `design` 的依赖：`proposal.md`（已完成，必须先读再写）

### /opsx:ff：apply-ready 的定义与落点（2026-01-26）
- 本次 schema 的 `applyRequires=["tasks"]`，因此只要 `tasks` 工件 done，就可以进入实现阶段。
- 当前 change `parallel-hat-instances` 已满足：
  - `proposal.md`、`design.md`、`specs/**/*.md`、`tasks.md` 全部 done
- 进入实现时，应优先把 `spec.md` 的每条 Requirement/Scenario 映射为可验证的测试或 smoke fixture。

## 2026-01-27 03:10:00 +0800｜E2E 测试流程调研：ralph-e2e + parallel 模式

### 现有 E2E Harness（`crates/ralph-e2e`）
- 入口命令：
  - `cargo run -p ralph-e2e -- --list`：列出所有场景（按 Tier 分组）
  - `cargo run -p ralph-e2e -- claude|kiro|opencode`：跑某个后端全套场景
  - `cargo run -p ralph-e2e -- claude --filter <pattern>`：只跑匹配 pattern 的场景（适合回归某个功能点）
  - `--keep-workspace`：保留 `.e2e-tests/<scenario>/` 方便排障
  - `--skip-analysis`：跳过 meta-Ralph 分析，加速
- `RalphExecutor` 默认会：
  - 强制开启 `RALPH_DIAGNOSTICS=1`，让每次 E2E 都自动产出诊断日志
  - 设置 `RALPH_WORKSPACE_ROOT=<workspace>`，避免路径解析错误
  - 目前还会设置 `CLAUDE_MODEL=haiku`（降低成本/加速）

### parallel 模式关键点（影响 E2E 设计）
- 开关：`parallel.enabled: true`
- 硬门槛：`parallel.topic_contracts` 必须显式配置，且必须能解析 `task.start`/`task.resume`（Supervisor 启动时会校验）
- 并行 runner 目前是“日志输出模式”（无 parallel TUI）：
  - 启动会打印 `[supervisor] instances ...`
  - 每行输出会带归因：`[writer#1:out] ...` / `[writer#1:err] ...`
  - 状态变更会打印：`[writer#1:state] running|idle|failed|done`

### E2E 场景设计原则（针对并行）
- 不走 human gate：E2E 无法交互回答 gate，因此场景配置要避免触发 `gate.request`
- 通过 topic_contracts 控制“谁收到什么事件”：
  - `task.*` 建议只投递给 `ralph#1`，避免所有实例在启动时被无意义唤醒
  - `build.task` 用 `fanout + audience.hats` 同时投递给 `writer`/`tester`（以及 writer 的多实例）
  - `build.done` / `test.done` 回送给 `ralph#1`，让 `ralph#1` 观察结果后输出 `LOOP_COMPLETE`

## 2026-01-27 11:08:00 +0800｜并行 E2E 流程收敛：只跑 Codex + 强制退出 + 事件归因

### 关键结论
- `ParallelHatInstancesScenario` 目前 **只支持 Codex**：文档里用 `-- claude` 会导致场景被过滤掉（跑空）。
- E2E harness 需要解析 `.ralph/events*.jsonl` 的 `source_instance`：
  - 并行模式下这是“事件归因”的关键字段（谁发的 build.done/test.done）。
  - 没有它，report/断言只能做 topic 级别判断，很难定位卡在哪个实例。
- E2E 超时必须能“杀干净”：
  - `ralph run` 会 spawn 多个 backend 子进程。
  - 只 kill 父进程会留下残留，影响下一次 E2E（端口/认证/进程数/资源）。
  - 因此 timeout 时要 kill **进程组**，并且 pgid 必须以 OS 查询为准（`getpgid`），避免误杀/漏杀。

### 已做的落地（对应代码点）
- `crates/ralph-e2e/src/executor.rs`：
  - 解析事件时把 `source_instance` 写入 `EventRecord`（可选字段）。
  - 超时强杀：先查 `getpgid(pid)`，再 `SIGTERM -> grace -> SIGKILL` 杀整组。
- `crates/ralph-cli/src/display.rs`：
  - 更新单测里构造的 `EventRecord`，补齐 `source_instance: None`，避免编译失败。
- 文档：
  - `specs/parallel-hat-instances/e2e.md`、`docs/advanced/testing.md`：并行场景命令改为 `-- codex`，并补充 `source_instance` 作为排障信号。

## 2026-01-27 20:40:00 +0800｜四文件摘要（用于决定是否提取 skill）
- 任务目标（task_plan.md）：
  - 为 `parallel-hat-instances` 补齐可执行的 E2E 测试流程与可靠性护栏。
  - 收尾包含：只跑 Codex、timeout 强制退出、events 解析 `source_instance` 等。
- 关键决定（task_plan.md）：
  - E2E 超时必须“杀干净”，避免后台残留子进程影响下一轮。
  - E2E 需要解析 `source_instance`，把“实例归因”从日志前缀升级为结构化字段。
- 关键发现（notes.md）：
  - `ParallelHatInstancesScenario` 当前只支持 `codex`（用 `-- claude` 会跑空场景）。
  - 并行模式下排障核心信号之一是 `events.jsonl` 的 `source_instance`。
- 实际变更（WORKLOG.md）：
  - 新增并行 Tier 8 E2E 场景与配套文档。
  - 强化 `RalphExecutor`：timeout 时按进程组强杀，并解析 `source_instance`。
- 错误与根因（ERRORFIX.md，如有）：
  - 进程组强杀最初尝试 `pre_exec`，但仓库 `forbid(unsafe_code)` 导致不可用，需改用安全 API。
  - Mermaid `graph` label 因括号等字符触发 Parse error，需要改为带引号的 label。
- 可复用点候选（1-3 条）：
  1. Rust E2E/集成测试中，为防止 timeout 后残留子进程：用“独立进程组 + SIGTERM→SIGKILL”实现硬退出（避免 `unsafe pre_exec`）。
  2. Mermaid flowchart 节点 label 触发 Parse error 时，优先改为带引号的安全写法：`Node["text (x)"]`。
- 是否提取/更新 skill：是（理由：两条都属于不明显且已验证的踩坑点，能直接复用到后续类似工作）。

## 2026-01-27 13:01:41 +0800｜parallel-hat-instances E2E（Codex）二次排障补记

### 现象
- E2E 能看到实例启动与输出归因前缀（writer#1/writer#2/tester#1）。
- 但 `.ralph/events.jsonl` 长时间只有 `build.task`，没有 `build.done/test.done`，导致场景断言失败。

### 根因定位（关键证据）
- 在并行模式下，custom hat 的 prompt 之前会被 InstructionBuilder 的“重型模板”包裹：
  - 模板包含 `### 2. VERIFY`：强制要求“跑 tests/验证”。
  - writer/tester 在收到 `build.task` 后，会优先去 `ls`、找 Cargo.toml、尝试 `cargo test`，导致迟迟不发 `<event topic=\"build.done\">`/`<event topic=\"test.done\">`。
- 这对 E2E 来说是“反向 backpressure”：
  - 我们只想验证“事件解析 + fanout + 归因 + 落盘”，不应该让模型去做昂贵且不确定的测试工作。

### 修复要点
- 并行模式 prompt 组装：
  - 当 hat 在配置里显式提供了 `instructions` 时，优先直接使用原文，不再套 InstructionBuilder 模板。
  - 这样 writer/tester 会按 E2E 指令立刻发事件，避免被 VERIFY 段落带偏。
- E2E 场景指令强化：
  - writer/tester instructions 增补 “IMPORTANT (E2E harness)：禁止跑 tests/命令/改文件，必须立即输出事件”。
- 最终验证：
  - `bash scripts/run-parallel-hat-instances-codex.sh` 通过，`events.jsonl` 中可见 `build.done/test.done` 且带 `source_instance`。

## 2026-01-27 14:25:14 +0800｜job-level timeout + completion drain（并行模式补记）

### 新增能力：job-level timeout（支持 per-hat override）
- 配置入口：`hats.<hat>.job_timeout_secs`
  - 未设置：继承 `adapters.<backend>.timeout`
  - `0`：禁用 timeout（None）
  - `>0`：使用该秒数
- 解析与注入位置：
  - `ParallelSupervisor` 在 spawn 实例时计算每个 hat 的 timeout，并注入到 HatInstance actor
  - HatInstance 在构造 `HatJob` 时写入 `HatJob.timeout`
  - CLI executor 使用 `HatJob.timeout` 做 tokio 级别的超时终止

### 新增护栏：completion promise 改为“软退出信号”
- 关键点：不能在检测到 completion promise 时立刻 break
  - 否则同一轮输出里解析到的事件可能还没来得及路由，下游实例就永远收不到
- 当前策略：
  - 先路由事件，再进入 drain 窗口（min 0.5s / max 60s）
  - drain 期间等待并行实例把最后一波事件产出并落盘

### E2E 经验：尽量不要把“等待/观察”交给模型
- 现象：让 ralph#1 “等到看到 build.done/test.done 再 LOOP_COMPLETE”在真实后端上容易漂移，导致跑到 max_runtime
- 调整方向：
  - ralph prompt 只做两件事：发 `build.task` + 输出 `LOOP_COMPLETE`
  - 由 Supervisor 的 completion drain 负责收尾，让 E2E 更机械、更稳定

## 2026-01-27 16:05:00 +0800｜topic_contracts（TopicContract）路由逻辑速记（对照代码）

### 1) topic pattern 的匹配规则（glob，但很“窄”）
- 入口实现：`crates/ralph-proto/src/topic.rs` 的 `Topic::matches_str`
- 规则要点：
  - `"*"` 是全局通配：匹配所有 topic（无视段数）
  - `*` 只匹配“一个 segment”（用 `.` 分段），不支持子串通配
    - ✅ `"task.*"` 匹配 `"task.start"`
    - ❌ `"task.*"` 不匹配 `"task.start.now"`（段数不一致）

### 2) 一个 topic 会命中哪个 contract（“最具体的”优先）
- 构建与排序：`crates/ralph-core/src/parallel/router.rs` 的 `TopicContractStore::new`
- 排序规则（更具体 = 更靠前）：
  1. 非 `"*"` 优先于 `"*"`（全局兜底永远最后）
  2. `*` 越少越具体（例如 `"task.*"` 比 `"*.*"` 更具体）
  3. 同样 `*` 数量下，pattern 越长越具体
- 解析：`TopicContractStore::resolve` 会按排序后的顺序，找第一个 `pattern.matches_str(topic)` 的 contract

### 3) TopicContract 自己定义了什么
- 定义在 `crates/ralph-proto/src/routing.rs`：
  - `delivery`: `queue`（选一个） / `fanout`（投递给所有）
  - `audience`: 这个 topic 的“基准受众”（instances / instance_prefixes / hats）
  - `queue_selection`: queue 多候选时如何选（`llm` / `deterministic`）
  - `missing_instance_policy`: 指向的实例不存在时怎么处理（spawn / queue / escalate / drop）

### 4) 并行 Supervisor 里，topic_contracts 真正怎么参与路由
- 核心入口：`crates/ralph-core/src/parallel/supervisor/routing.rs` 的 `ParallelSupervisor::route_event`
- 关键流程（按执行顺序）：
  1. **特殊 topic 不走业务路由**
     - `dispatch.decision`：只用于记录 queue 派发决策（replay/观测），不会投递给业务 hat
     - `gate.request`/`gate.resolve`：先更新本地 gate 状态机，`gate.resolve` 可能会改写成 `target_instance` 直达回送
  2. **`target_instance` 的优先级最高**
     - 如果实例存在：直接投递，不受 TopicContract 影响
     - 如果实例不存在：按 `missing_instance_policy=spawn` 可先创建再投递，否则走 escalation
  3. **解析 contract**
     - `self.resolve_contract(event.topic)` -> `TopicContractStore::resolve`
  4. **计算 base audience（来自 TopicContract.audience）**
     - `instances`：显式列出
     - `hats`：展开成当前已创建的实例集合（`instances_by_hat`）
     - `instance_prefixes`：只匹配“当前已存在实例”（注意：不会凭空生成未来实例）
     - 如果 base audience 为空：直接 `bail!`（避免“隐式 broadcast/隐式 none”）
  5. **应用 event 级别的收缩/覆盖**
     - `event.target`：进一步把 base audience 过滤成“只投递给某个 hat 的实例”
     - `event.audience_override.instances`：计算交集 `TopicContract.audience ∩ override.instances`
       - override 不能“扩权”，指向不在 contract audience 的实例会被记为 `missing_outside_base`
       - `require_delivery=true` 时，任何 missing 都会触发 escalation（不允许静默 reroute）
  6. **缺失实例策略（missing_instance_policy）**
     - `spawn`：尝试创建缺失实例，再投递
     - `queue`：如果 override 指向的实例缺失且最终收件人为空，会回退到 base_existing（best-effort）
     - `escalate`：直接 escalation，并停止投递
     - `drop`：忽略缺失实例（best-effort 下继续投递已有 recipients）
  7. **delivery 语义**
     - `fanout`：给每个 recipient 投递一份（clone）
     - `queue`：必须选一个实例
       - `llm`：起一个 `ralph#decider-*` job 让模型选；失败则 fallback 到 deterministic
        - `deterministic`：优先选 Idle/Created（least-busy），同忙闲等级内 round-robin
        - 无论哪种，都强制写入 `dispatch.decision` 事件，保证 replay 不重算

## 2026-01-27 19:00:33 +0800｜Explore：parallel-trigger-routing（默认 triggers fanout + per-hat queue + autoscale + workspace override）

### 现状冲突点（直接引用）
- README 明确写了并行模式必须显式 contracts：
  - "Parallel mode requires **explicit topic routing contracts** (`parallel.topic_contracts`). There is no implicit broadcast."
  - 位置：`README.md` 的 "Experimental: Parallel Hat Instances (Headless)"
- 并行 Supervisor 也把它做成硬校验：
  - "parallel.enabled=true 但 parallel.topic_contracts 为空：并行模式要求每个 topic 都能解析到显式 TopicContract..."
  - 位置：`crates/ralph-core/src/parallel/supervisor.rs`

### 我们拍板的新语义（用于新 change）
- 默认路由语义（并行）：
  - topic → hats：按 `hats.*.triggers` 订阅关系 **fanout 到所有订阅 hats**
  - hat → instance：每个 hat **只选 1 个实例执行**（instance-level queue），不对该 hat 的所有实例 fanout
- TopicContract 定位：可选 override
  - 有匹配 TopicContract：按 contract 路由
  - 无匹配/为空：回退到 triggers 默认路由
- 自动扩缩容（默认开）：
  - 空闲实例优先；全忙则动态创建实例
  - 全局并发上限默认 4（安全刹车）
  - 动态实例 idle 30s 自动回收
  - 实例 key 单调递增且永不复用（方案 A）
- 严格校验：
  - `event.target` / `event.target_instance` 必须订阅该 topic，否则 warn + escalate，禁止投递
  - 允许少数控制面 topic 做特例（避免打断 gate/控制信号）
- workspace override：
  - 走 Event 显式字段
  - 合并多个 events 成同一 job 时：`worktree > patch > shared`

### OpenSpec 工件输出
- 新 change：`openspec/changes/parallel-trigger-routing/`
  - `proposal.md`：WHY / WHAT / BREAKING / impact
  - `design.md`：两层路由语义 + autoscale/cap/idle reaper + workspace override 的设计
  - `specs/parallel-trigger-routing/spec.md`：可测试的 MUST + scenarios
  - `tasks.md`：实现拆解清单（含 docs + tests + E2E/fixtures）

## 2026-01-28 02:18:46 +0800｜新 OpenSpec：parallel-supervisor-tui（并行 Supervisor TUI + chat/gate）

### 触发原因（用户需求）
- 用户明确要求：把并行模式的 Supervisor TUI 真正做出来，并且 **连同 human async chat + gate 面板一起做**。
- 现状：并行 runner 仍提示 “no TUI”，只能看 stdout 日志；与 `specs/parallel-hat-instances.spec.md` 8.x 的草案不一致。

### 产物位置
- change 目录：`openspec/changes/parallel-supervisor-tui/`
  - `proposal.md`：WHY/WHAT/Capabilities/Impact
  - `design.md`：复用 `ralph-tui` 的并行模式设计（instance→jobs→buffer + observer→channel→reducer + 外部事件落盘）
  - specs：
    - `openspec/changes/parallel-supervisor-tui/specs/parallel-supervisor-tui/spec.md`
    - `openspec/changes/parallel-supervisor-tui/specs/supervisor-human-chat-gate/spec.md`
  - `tasks.md`：实现拆解清单（CLI 接入、TUI state、实例列表/详情、job 分段、chat、gate、验证）

### 关键决策（摘要）
- 不新建 TUI crate：在 `crates/ralph-tui` 内新增并行模式（避免两套 TUI 分裂）。
- 并行内容维度：instance→jobs→buffer（对齐 spec 8.1 的心智）。
- human 输入落盘：直接追加到 `.ralph/current-events` 指向的 JSONL（等价 `ralph emit`，避免 spawn 子进程）。
- chat topic 固化为 `human.message`，定向消息用 `target_instance`，payload 为原始文本。

## 2026-01-28 02:11:10 +0800｜核对：Supervisor TUI 是否已落地（parallel-hat-instances）

### 规格里写了什么
- `specs/parallel-hat-instances.spec.md:632`：`## 8. Supervisor TUI（高层交互草案）`
  - 布局：左侧 HatInstance 列表、右侧实例输出、下方 human async chat（含 gate）。
- `specs/parallel-hat-instances.spec.md:662`：8.1 给了明确的“复用现有 TUI”落点
  - 把 `IterationBuffer` 抽象为 `TextBuffer`
  - 把 `iterations: Vec<IterationBuffer>` 替换为 `instances: HashMap<HatInstanceId, InstanceViewState>`，并在实例内维护 `jobs: Vec<TextBuffer>`

### 代码里实际是什么
- 并行模式入口直接声明“当前无 TUI”：
  - `crates/ralph-cli/src/parallel_runner.rs:311`：`warn!("Parallel mode currently runs without TUI (log output only)")`
- 当前 `ralph-tui` 仍是“按 iteration”管理 buffer（未按 spec 8.1 改成 instance/job 维度）：
  - `crates/ralph-tui/src/state.rs:129`：`pub iterations: Vec<IterationBuffer>`
- 并行模式目前的最小可用“展示”是 stdout 日志：
  - `crates/ralph-cli/src/parallel_runner.rs:351`：打印 `[supervisor] instances (initial=created):`
  - `crates/ralph-cli/src/parallel_runner.rs:380`：逐行输出带实例归因前缀（例如 `[writer#1:out] ...`）

### 结论
- Supervisor TUI 在 spec 里是“草案”，目前代码尚未落地；并行模式暂时只有日志输出（无列表/详情/chat 的 TUI）。

## 2026-01-28 11:34:09 +0800｜更新：Supervisor TUI 已可启动（骨架完成），chat/gate 仍未闭环

### 背景
- 02:11 的结论是“并行模式只有日志输出，没有 TUI”。
- 随着 `parallel-supervisor-tui` 的实现推进，这个结论已经 **过期**，需要被更新。

### 代码现状（可定位）
- 并行 runner 已启动并行 TUI：
  - `crates/ralph-cli/src/parallel_runner.rs`：`Tui::new_parallel()`，并通过 `update_sender()` 推送 `TuiUpdate`
- `ralph-tui` 已引入并行模式 state：
  - `crates/ralph-tui/src/state.rs`：`TuiMode::Parallel` + `ParallelTuiState` + `apply_update(...)`
  - `crates/ralph-tui/src/state/parallel.rs`：instance→jobs→buffer 的骨架（按 `job_id` 分段）
- UI 渲染已具备三 pane 骨架：
  - `crates/ralph-tui/src/app.rs`：左 instances 列表、右输出、下 chat/gates 占位
  - 并行模式输入目前只保留最小闭环：`q` 退出、`?` help（Tab/滚动/搜索/编辑框尚未补齐）

### OpenSpec（把 chat + gate 一起做的依据）
- `openspec/changes/parallel-supervisor-tui/specs/supervisor-human-chat-gate/spec.md`
  - 规定 `human.message` 写入外部事件流（可带 `target_instance`）
  - 规定 gate 面板展示 `gate.request`，并能在 UI 内写入 `gate.resolve`

### 仍未完成的关键点（对应 tasks）
- 3.x：三 pane 焦点/导航 + 搜索/滚动复用
- 5.x：chat 输入框编辑 + ExternalEventWriter（写 `.ralph/current-events`）
- 6.x：gate reducer + 倒计时 + `!approve/!deny/!resolve` 落盘
- 7.x：单测 / replay fixture / `/tui-validate` / 全量 `cargo fmt/clippy/test`

## 2026-01-28 20:50:27 +0800｜parallel-workflow-semantics：关键语义锚点（并行模式）

### 语义锚点（给团队统一口径）
- runtime 第一条事件永远是 `task.start`（fresh）/`task.resume`（resume），并且在并行模式下强制只投递给 `ralph#1`（避免顶层 prompt 污染其他 hats）。
- `event_loop.starting_event`：可选的 workflow entry topic（协调后发出），不是 runtime 的第一条事件。
- `event_loop.complete_publishes`：该 workflow 的“完成候选事件 topic”（唯一、可选）。
  - 当 `ralph#1` 观察到该 topic 时，决定是否输出 `event_loop.completion_promise`（例如 `LOOP_COMPLETE`）结束；也可以选择继续派发下一轮事件。
- orphan 定义：只有“在 hats 配置里完全找不到任何订阅者（specific + wildcard 都没有）”才算真 orphan，才升级给 `ralph#1`。

### 对应实现落点（便于快速定位）
- 配置字段：`crates/ralph-core/src/config.rs`（`EventLoopConfig.complete_publishes` + validate）
- 并行启动控制面：`crates/ralph-core/src/parallel/supervisor.rs`（`task.start/task.resume` 强制 target_instance=ralph#1）
- 并行路由链式 fallback：`crates/ralph-core/src/parallel/supervisor/routing.rs`
- 并行 ralph#1 prompt 注入：`crates/ralph-core/src/parallel/supervisor.rs`（生成 instructions）+ `crates/ralph-core/src/parallel/instance.rs`（优先使用注入指令）
- replay smoke fixture：`crates/ralph-core/tests/fixtures/parallel_workflow_semantics.jsonl`

## 2026-01-28 21:56:16 +0800｜E2E：并行事件解析的关键约束（只 parse stdout）

### 现象（为什么需要这个约束）
- 在 Codex CLI 下，stderr 往往会回显 user prompt / 后端日志。
- 这些回显文本里可能包含 `<event ...>`（来自 prompt 示例或指令片段），如果被 EventParser 当成输出解析，会导致重复路由/假阳性事件，进而让 E2E 事件统计波动。

### 实现落点（可定位）
- `crates/ralph-cli/src/parallel_runner.rs`
  - `CliHatJobExecutor::handle_output_line`：stdout 进入 `HatJobResult.output`（用于 EventParser），stderr 仅用于流式可观测输出，不参与解析。

## 2026-01-28 23:24:49 +0800｜E2E：parallel-hat-instances prompt 变体两次回归（鲁棒性）

### 目的
- 你要求“多跑两次 E2E，并且让 prompt 内容稍微变化”，用来验证：
  - 即使 prompt 里出现“伪 event 示例块”或 fenced code block，系统也不会把它误当成真实输出事件。
  - 并行路由/事件解析/归因输出在真实后端（Codex）下依然稳定。

### 执行方式
- 使用 `crates/ralph-e2e/src/scenarios/parallel.rs` 里内置的环境变量开关：
  - `RALPH_E2E_PARALLEL_PROMPT_VARIANT=variant1`
  - `RALPH_E2E_PARALLEL_PROMPT_VARIANT=variant2`

### 结果
- variant1 ✅（耗时约 114.8s）：`parallel-hat-instances` 通过
- variant2 ✅（耗时约 91.7s）：`parallel-hat-instances` 通过

### 命令（复现）
```bash
RALPH_E2E_PARALLEL_PROMPT_VARIANT=variant1 cargo run -p ralph-e2e -- codex --filter parallel-hat-instances --verbose
RALPH_E2E_PARALLEL_PROMPT_VARIANT=variant2 cargo run -p ralph-e2e -- codex --filter parallel-hat-instances --verbose
```

## 2026-01-28 23:42:01 +0800｜示例对齐：parallel-trigger-routing 不再靠 prompt 写死闭环

### 改动思路
- “prompt 只承载目标”，不要把“控制面 entry/exit 语义”藏在 prompt 里。
- 示例应当优先演示官方语义：`starting_event`（协调后入口）与 `complete_publishes`（完成候选）。

### 落点
- `examples/parallel-trigger-routing/ralph.yml`：
  - 加入 `starting_event=spec.start`，`complete_publishes=spec.approved`
  - 加入 `event_loop.prompt: | ...`（内联示例目标 prompt，避免依赖额外 prompt 文件）
- `examples/parallel-trigger-routing/README.md`：更新 Run/Notes（不再要求 `-P prompt.md`）

## 2026-01-29 00:17 +0800｜文档：parallel-trigger-routing README 中文化

### 处理原则
- 仅翻译说明文字。
- 所有命令、topic 名称、配置字段 key（例如 `parallel.enabled`、`parallel.topic_contracts`、`hats.*.triggers`）保持不变。

### 验证
- `cargo test -q` ✅

## 2026-01-29 02:30 +0800｜四文件摘要（用于决定是否提取 skill）

### 任务目标（task_plan.md）
- 让 `examples/parallel-trigger-routing` 不再“靠 prompt 写死闭环”，而是用 `event_loop.starting_event` / `event_loop.complete_publishes` 固化官方语义。
- 增加中文 parallel E2E 场景，并做两次 prompt 变体回归，验证稳定性与鲁棒性。

### 关键决定（task_plan.md）
- 示例入口/出口语义写在 config：`starting_event`（协调后 workflow entry）+ `complete_publishes`（completion candidate）。
- 目标 prompt 内联到 `event_loop.prompt`（避免依赖额外 prompt 文件导致的“是不是必须靠 prompt 才能闭环”的困惑）。
- 新增独立中文 E2E 场景（保留英文场景），并用 prompt 变体验证“伪 `<event>`/代码块”不应被误解析。

### 关键发现（notes.md）
- 并行模式下的“官方语义锚点”需要被写死在主规格与协调者指令里：
  - runtime 第一条事件永远是 `task.start/task.resume`，并强制只投递给 `ralph#1`（避免 prompt pollution）。
  - `starting_event` 是“协调后 workflow entry event”，不是 runtime 第一条事件。
  - `complete_publishes` 是“workflow completion candidate topic”，由 `ralph#1` 决定是否输出 `completion_promise`。
  - orphan 边界：只有“完全无人订阅（specific+widlcard 都无）”才升级给 `ralph#1`。
- E2E 稳定性关键约束：只从 stdout 解析 `<event ...>`，stderr 仅用于可观测输出，避免 Codex/后端回显导致假事件。

### 实际变更（WORKLOG.md）
- 示例：`examples/parallel-trigger-routing/ralph.yml` 与 `README.md` 已对齐“entry/exit 靠 config”。
- E2E：新增中文场景 `parallel-hat-instances-zh`，并做两次变体回归。
- 稳定性修复：E2E workspace 在复跑时先清理旧目录，避免 `.ralph/events.jsonl` 累积污染断言。
- OpenSpec：`parallel-workflow-semantics` 已归档并同步 delta specs → `openspec/specs/`。

### 错误与根因（ERRORFIX.md，如有）
- 并行 E2E 事件统计波动：stderr 回显包含 `<event ...>`，被误判为真实事件。
- E2E 复跑污染：`--keep-workspace` 后复跑复用旧目录，导致 `.ralph/events.jsonl` 累积。
- OpenSpec `archive` 校验失败：Requirement 首句缺少 MUST/SHALL（validator 的硬规则）。

### 可复用点候选（1-3 条）
1. 事件解析：不要从 stderr 解析 `<event>`（只 parse stdout），否则会出现“伪事件/重复路由/计数波动”。
2. E2E 复跑隔离：workspace 若可能复用（尤其 `--keep-workspace`），必须在新 run 前清理旧目录，避免事件日志叠加污染。
3. OpenSpec 写作/归档：`### Requirement:` 的首句必须带 MUST/SHALL，否则 `openspec archive` 会被 validator 卡住。

### 是否需要固化到 docs/specs
- 是（已固化）：
  - 主规格：`openspec/specs/parallel-hat-instances/spec.md`、`openspec/specs/parallel-trigger-routing/spec.md`
  - 用户文档：`docs/` 下已补充 `starting_event/complete_publishes` 的解释与示例

### 是否提取/更新 skill
- 是：建议新增 2-3 个项目级 `self-learning.*` skill（放在 `.codex/skills/`），覆盖：
  - OpenSpec validator 的 MUST/SHALL 首句规则
  - 并行事件解析只 parse stdout（Codex/CLI stderr 回显导致假事件）
  - E2E workspace 复跑污染（keep-workspace/旧目录残留导致断言误判）

---

## 2026-01-29 合并 `7a346bd`：preset 配置更新（价值评估）

### 来源
- `git show 7a346bd425cf2d7a45d086875eba413a21111744`
  - 标题：Update preset configurations (#108)
  - 影响文件：`presets/*`（6 个）+ `tools/preset-test-tasks.yml`

### 变更摘要（按文件）
- `presets/api-design.yml`
  - 在 `critic` 阶段新增“务实/收敛”约束：只对关键问题要求 refinement。
  - 明确 1 次 refinement 后应“带注释通过”，避免无限循环。
- `presets/documentation-first.yml`
  - 在 `reviewer` 阶段新增“务实/收敛”约束：只对根本性问题 reject。
  - 明确 1 次 reject 后应“带注释通过”，避免无限循环。
- `presets/spec-driven.yml`
  - 在 `spec_reviewer` 阶段新增“务实/收敛”约束：只对根本性歧义/缺失关键需求 reject。
  - 明确 1 次 reject 后应“带注释通过”，避免无限循环。
- `presets/code-archaeology.yml`
  - 更明确的结束条件：完成时用 `LOOP_COMPLETE` 退出。
  - 对“只做理解/文档、不改代码”的情况给出明确落盘行为（写到指定输出文件后退出）。
- `presets/mob-programming.yml`
  - `navigator` 不再发布 `mob.complete`（只保留 `direction.set`）。
  - 补充 completion criteria，并改为“任务完成就直接输出 `LOOP_COMPLETE`”。
  - 这能降低“靠事件 topic 才能停机”的误用风险。
- `presets/socratic-learning.yml`
  - `questioner` 不再发布 `understanding.verified`（只保留 `question.asked`）。
  - 将“完成条件”改为：2-3 轮问答后，整理最终理解到输出文件，然后输出 `LOOP_COMPLETE`。
  - 删除 `answerer` 中“等 understanding.verified 再 LOOP_COMPLETE”的不可达指令（因为它并不会被该 topic 触发）。
- `tools/preset-test-tasks.yml`
  - 调整复杂度分级（例如 `socratic-learning`、`mob-programming`、`code-archaeology` 提升等级）。
  - 调整 timeout（simple/medium/complex 变为 450/900/1200），并补充基于观测的解释。

### 初步判断：哪些是“有价值内容”
- “收敛性”改良：对 review 类 preset 加入“只拦关键问题 + 1 次后带注释通过”，能显著减少无意义迭代。
- “可停机性”改良：将某些 preset 从“发布一个完成事件”改成“直接 LOOP_COMPLETE”，更贴近 Ralph 的控制面语义。
- “时间预算”改良：timeout/复杂度更贴近实际迭代时长，减少误判超时。

### 主要风险点（待用测试背压验证）
- 事件 topic 调整是否影响下游 hat 触发链（例如移除 `mob.complete` / `understanding.verified`）。
  - 初看：这些 topic 没有任何 hat 订阅，属于“名义上存在但实际上无用”的事件，移除更合理。
- timeout 与复杂度等级调整，是否会影响某些基于固定值的断言/基准测试（需要跑 `cargo test` 验证）。

---

## 2026-01-29 合并 `685526d`：避免 npx 进程组下 TUI 卡死（价值评估）

### 来源
- `git show 685526d8b901a19f73774e7f2c80bb22494dd1c2`
  - 标题：fix(cli): avoid TUI hang under npx process group (#114)
  - 影响文件：`crates/ralph-cli/src/main.rs`

### 变更摘要
- 目标问题：在某些 wrapper（典型是 `npx`）环境里，CLI 如果在启动时强行 `setpgid(pid, pid)`，
  可能把自己移出“前台 TTY 进程组”，导致 TUI 键盘输入不再到达，从而表现为“界面卡死/无响应”。
- 修复思路：初始化进程组时先判断：
  - 如果已经是进程组 leader（`getpgrp() == pid`）：直接返回；
  - 如果当前进程组就是前台 TTY 的进程组：跳过 `setpgid`，保持交互可用；
  - 否则再尝试 `setpgid(pid, pid)`。

### 本次落地的“有价值内容”
- 这是一个“硬故障修复”：优先保证交互（TUI 输入）不挂死。
- 改动范围小，只影响 Unix 下的进程组初始化逻辑，风险集中、易验证。

### 主要 trade-off / 风险点
- 在 wrapper 场景下我们可能不再强制成为 process group leader，
  进而降低“通过 kill 整个进程组清理子进程”的确定性。
  - 但相比“直接卡死不可用”，这个 trade-off 可接受（并且更符合“先能用”的工程优先级）。
