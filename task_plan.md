# 任务计划: 【多 Hat 并行运行 + 独立结束】(brainstorming)

## 目标
让 Ralph 的 hat 可以**同时运行多个**，并且：

- 可以分别启动，也可以一起启动
- 允许不同 hat 在不同时间结束（不会互相阻塞）
- 运行期的状态/输出/事件可追踪，且尽量不破坏现有架构

## 阶段
- [x] 阶段1: 建立上下文（读代码/文档）
- [x] 阶段2: 澄清需求（明确“同时运行”的语义 + UI/人机协作）
- [x] 阶段3: 方案发散（2-3 个可选架构）
- [x] 阶段4: 收敛成推荐设计（数据流/生命周期/退出策略）
- [x] 阶段5: 验证路径（如何测试 & 如何渐进迁移）
- [x] 阶段6: LLM 决策边界确认（哪些交给 LLM、哪些保持机械规则/人类 gate）
- [x] 阶段7: 生成 OpenSpec 草案（requirements/design/tasks）
- [ ] 阶段8: 准备进入实现（拆 code-tasks + 验证清单）

## 关键问题（待补充答案）
1. “多个 hat 同时运行”指的是：并行执行多个 agent loop？还是并行执行多个 tool/子任务？或两者？
2. 多个 hat 的输出要如何呈现：同一 TUI 里分栏/分 tab/混排？还是只做日志/事件层面的并行？
3. 当某个 hat 结束后，其他 hat 是否继续保持可交互（仍能接收事件、继续跑 loop）？
4. 并行 hat 的“工作区策略”如何配置：共享 / git worktree / 复制？以及读写权限如何声明与约束？
5. “Supervisor”是固定角色还是能力？由 Rust orchestrator 承担还是由 LLM(hat) 承担？
6. Topic 的投递语义：queue / fanout 是否必须显式声明？fanout 的“范围”是 per-hat 还是 per-instance？
7. fanout 事件的“受众限制”想怎么表达：按 hat 类型白名单？按实例标签？按 topic pattern？
8. “可否由 LLM 决策”的边界是什么：LLM 只提议还是能直接触发动作？哪些必须走 human gate？

## 做出的决定
- [决定測略]: 选择 **方向1（HatInstance Actor 模型）** 做“真并行”核心架构。
- [决定语义]: 事件/Topic 的投递语义 **必须显式声明**（queue / fanout），并支持 **实例级（HatInstanceId）受众限制**。
- [决定派发]: `queue` 语义下“投递到哪个具体实例”由 **LLM 决策**（你选 B），并把候选集+选择结果写入事件日志保证 replay。
- [决定实例点名语义]: `audience_override.instances=[...]` 默认 **best-effort**（你选 A），实例不存在时按 `missing_instance_policy` 处理；如需强制送达用 `audience_override.require_delivery=true`。
- [决定权限]: 权限条目 1/2/3/4/5 全部存在，但初期默认都 `allow`（后续可切换为 `ask/deny`）。
- [决定执行模型]: 每个 job = 一次 headless CLI invocation（codex/claude code/…），HatInstance actor 负责调度与状态机。
- [决定 worktree 策略]: **临时 worktree（每 job 一次）**仅在“hat 具备能力 + LLM 预判需要”时才创建；避免默认总开 worktree（考虑 submodules/网络/性能）。
- [决定 capabilities 形态]: hat 的能力用**字符串白名单**表达，例如 `capabilities: ["workspace.worktree", "git.merge", "verify.tests"]`。
- [决定 hooks]: 对于允许 worktree 的 hat，支持 `pre_script` / `post_script`（由 hat 设计者配置与维护），用于 submodules 初始化等“避不开”的坑。
- [决定 gate]: human gate 支持两种模式：普通 gate（等待 human）/ 超时 gate（默认 60s，超时后由 LLM 自行决策）。
- [决定 human 异步输入]: human 可随时异步发送“调整需求”，通过事件日志落盘；并为每个 HatInstance 维护轻量 inbox 文件，方便 LLM 高频读取，不阻塞并行执行（默认不打断；`urgent` 才允许 cancel+重启）。
- [决定 human chat 路由]: human async chat 默认以 `ThreadId` 作为路由主键；`@writer#2` 仅作为 UI 便捷别名（实例消亡时 thread 不丢）。
- [决定 LLM 决策 executor]: 第一版不新增 `decider` hat，默认用内置 `ralph` hat 承担“决策类 HatJob”（派发决策、gate 超时自决等）。
- [待定职责]: worktree 任务完成后的“合并与校验”是否由专门 hat 承担（Integrator/Verifier），还是由 orchestrator 机械执行 + LLM 决策。
- [决定 hooks 失败策略]: hooks 失败后先落盘 `workspace.hook_failed`，再由 `ralph(decision)` 决策自愈（retry/repair/escalate/abort），默认 bounded 重试（建议 max_attempts=3）。

## 遇到的错误
- 暂无

## 状态
**目前已完成阶段7（OpenSpec 草案已生成，等待进入实现）**
- 我已经把并行 HatInstance 的关键决策都固化到 spec，并进一步拆成 OpenSpec 三件套。
- 我这样做是为了让后续实现能按 tasks 切分推进，同时保持“可回放 + 可控 + human async loop”的约束不走样。

### 阶段6结论（已固化到 spec）
- `queue` 派发由 LLM 决策（候选集+选择结果落盘，replay 不重算）
- human gate 支持普通/超时两种模式（超时默认 60s，触发后由 LLM 自行决策并写入事件日志）
- human 异步调整需求：默认不中断当前 job，只在安全点应用；`urgent` 才允许 cancel+重启
- human chat：以 `ThreadId` 为长期路由主键，实例仅作为可变 owner；`@instance` 只是 UI alias
- LLM 决策层落地：不在 Rust 内接 LLM SDK，而是把决策实现为“决策类 HatJob”，通过 headless CLI invocation 输出 `<event ...>` 并落盘
- `audience_override.instances=[...]` 默认 best-effort，缺失实例不算失败（按 `missing_instance_policy`）

### 阶段8建议的下一步（可选）
- 方向A（推荐）：把 `specs/parallel-hat-instances/tasks.md` 拆成 `tasks/*.code-task.md`，然后按“硬门槛优先”（cwd + 真流式输出）进入实现。
- 方向B：你先 review spec / OpenSpec，我再根据你的反馈做二次收敛（尤其是事件字段命名与配置结构）。

### 已交付物
- `specs/parallel-hat-instances.spec.md`（包含 graph + sequenceDiagram，已通过 mermaid-validator 校验）
  - 已补充：`7.3 可否由 LLM 决策？（推荐：LLM 提议 + Supervisor 执行）`
  - 已补充：`7.4 LLM 决策层怎么落地？（默认用内置 ralph hat 作为决策 executor）`
- OpenSpec 草案：
  - `specs/parallel-hat-instances/requirements.md`
  - `specs/parallel-hat-instances/design.md`（含 2 张 Mermaid 图，已通过 mermaid-validator 校验）
  - `specs/parallel-hat-instances/tasks.md`

### 阶段1进展（已确认的事实）
- 规格层面：`specs/event-loop/design/detailed-design.md` 明确写了 **Sequential hats / No parallel delegation / Single executor**。
- 代码层面：`EventLoop::next_hat()` 在 multi-hat 配置存在时，会“总是返回 ralph”，把自定义 hats 当作拓扑/指令注入，而不是独立执行器。
- CLI 层面：`crates/ralph-cli/src/loop_runner.rs` 是单循环串行，并且当前只从 `config.cli` 创建了一个 `CliBackend` 来执行，不会按 hat 切换 backend。

### 接下来
- 如果你要继续推进实现，我建议按 `specs/parallel-hat-instances/tasks.md` 的任务拆分进入阶段8。

### 新增需求已确认（来自用户）
- 你选择 **C（混合）**：同一种 hat 既可能常驻 worker，也可能以一次性 job 方式运行。
- 你希望 **并行 hats 全部 headless**，Supervisor 负责汇总输出并提供更高层 UI。
- 你希望 **workspace 策略能在 hat 设定里配置**（共享 / git worktree / 其他隔离方式），并且允许“只读 hat 共享工作区”。
- 你更倾向“看情况切换”：
  - 小改动：走 patch / 必要时文件锁
  - 大改动 + 反复迭代：走 git worktree 隔离
- Supervisor/示例是“用户故事”，不是固定帽子角色；你希望评估现有 `ralph` 是否能承担 Supervisor 职责。
