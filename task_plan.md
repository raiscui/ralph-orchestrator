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
- [x] 阶段8: 进入实现并验证（Supervisor TUI + chat/gate）

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
**目前已完成阶段8（Supervisor TUI + chat/gate 已实现并验证）**
- 我已经把并行模式的 Supervisor TUI 接入 `ralph-tui`，并补齐 “human async chat + gate 面板” 的闭环交互。
- 我这样做是为了让并行运行不再只能看日志，而是具备“可观测 + 可输入 + 可回放”的上层控制面。

- OpenSpec 状态补充：
  - `openspec/changes/parallel-supervisor-tui/` 已完成并归档到 `openspec/changes/archive/2026-01-28-parallel-supervisor-tui/`

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

## 2026-01-26 20:04:23 +0800｜/opsx:new：启动新的 OpenSpec change（仅 scaffold）

### 我正在做什么 & 为什么
- 我正在按实验性的“工件驱动”流程启动一个新 change。
- 这样做是为了让后续的 requirements/design/tasks 等工件，都有明确依赖与完成状态。
- 你可以用 `openspec status` 一眼看到“下一步该写哪个工件”。

### 两个可选输入方式（你选其一就行）
- 方向A（你给名称）：直接给我一个 kebab-case 的 change 名称，例如 `parallel-hat-instances`
- 方向B（你给描述）：你用一句话描述要做什么，我来帮你提炼成 kebab-case 名称

### 下一步动作（等你确认后我立刻执行）
1. 创建 change 目录：运行 `openspec change new "<name>"`（以本仓库实际 CLI 为准）
2. 查看工件状态：运行 `openspec status --change "<name>"`
3. 输出首个可写工件模板：运行 `openspec instructions <first-artifact-id> --change "<name>"`
4. 停止，不创建任何工件内容，等待你决定是否继续

### 现状快照
- 当前 `openspec/changes/` 下只有 `archive/`，还没有活跃的 change（2026-01-26 20:04:23 +0800）

## 2026-01-26 20:19:37 +0800｜/opsx:new：已确认 change 名称，准备创建 scaffold

### 你刚确认的输入
- change 名称：`parallel-hat-instances`（kebab-case 合法）
- workflow/schema：你未指定，因此我会使用默认 schema（不传 `--schema`）

### 我接下来要做什么 & 为什么
- 我会先检查 `openspec change` 的实际子命令用法。
- 这样做是为了避免把旧文档命令照抄到当前版本，导致创建失败或生成在错误位置。
- 然后我会按顺序执行：
  1. 创建 change scaffold（仅目录与空模板，不写任何工件内容）
  2. 查看 `openspec status`，找出第一个状态为 “ready” 的工件
  3. 输出该工件的 `openspec instructions` 模板与上下文
  4. 立刻停止，等待你决定是否继续写第一个工件

### 执行结果（2026-01-26 20:22:10 +0800）
- 已创建 change scaffold：
  - `openspec/changes/parallel-hat-instances/`
  - schema：`spec-driven`（默认）
- 当前工件状态（`openspec status --change "parallel-hat-instances"`）：
  - `proposal`：ready
  - `design`：blocked by `proposal`
  - `specs`：blocked by `proposal`
  - `tasks`：blocked by `design, specs`
- 已拉取首个可写工件 `proposal` 的模板与说明：
  - `openspec instructions proposal --change "parallel-hat-instances"`
  - 目标输出文件：`openspec/changes/parallel-hat-instances/proposal.md`

## 2026-01-27 02:59:30 +0800｜/opsx:continue：准备继续生成下一个工件（proposal）

### 我正在做什么 & 为什么
- 我正在接手并继续推进 `openspec/changes/parallel-hat-instances` 这个 change。
- 当前 change 还停留在“只有 scaffold，没有任何工件内容”的阶段。
- 我这样做是为了把 `proposal → specs → design → tasks` 这条链真正补齐。
- 后面才能进入 `/opsx:apply` 的实现环节，而不是停留在文档草案。

### 我接下来要做什么（按 OpenSpec 工作流）
1. 先用 `openspec status --change "parallel-hat-instances" --json` 确认 schema 与每个工件状态。
2. 找到第一个 `status="ready"` 的工件（预期还是 `proposal`）。
3. 用 `openspec instructions proposal --change "parallel-hat-instances" --json` 拉取模板与输出路径。
4. 按模板生成并写入 `openspec/changes/parallel-hat-instances/proposal.md`（只创建 1 个工件）。
5. 再跑一次 `openspec status --change "parallel-hat-instances"`，确认后续工件解锁情况。

### 注意事项（对自己）
- `context` / `rules` 只作为约束，不允许原样拷贝进工件文件。
- 只生成一个工件，生成完立刻停下，等待下一轮 `/opsx:continue`。

## 2026-01-27 03:02:40 +0800｜/opsx:apply：进入收尾验证（完成 tasks 8.2）

### 我正在做什么 & 为什么
- 我正在根据 OpenSpec 的 `apply` 指令，收尾完成最后一个未完成任务：8.2 全量检查。
- 这个任务的本质是“用硬门槛确认变更质量”，避免把潜在格式/静态检查/测试问题带进主分支。

### 我接下来要做什么（验证顺序）
1. `cargo fmt --check`：先保证格式一致，减少后续 diff 噪音。
2. `cargo clippy`：用静态检查把潜在 bug/坏味道拦在实现之前。
3. `cargo test`：跑全量测试（含 replay smoke tests），确保行为回归可控。
4. 若出现 error/warning：当场修复，再重复上述命令直到全部通过。

## 2026-01-27 03:10:00 +0800｜新增需求：为并行 HatInstance 变更制作 E2E 测试流程

### 我正在做什么 & 为什么
- 我正在为 `parallel-hat-instances` 这个变更补上“可执行的 E2E 测试流程”。
- 这样做是为了把并行模式从“单元测试 + replay smoke”升级到“真实后端 E2E”。
- 这能覆盖：并行 headless 进程调度、真实 LLM 行为漂移、真实认证/网络失败等非确定性风险。

### 两个可选方向（我会先按 A 推进，B 作为降级方案）
- 方向A（不惜代价，最佳方案）：补齐 **E2E 场景 + 文档流程**
  - 新增一条 `ralph-e2e` scenario（可用 `--filter` 单独跑）
  - 通过 `parallel.enabled=true` 的真实运行，验证：
    - 多实例（例如 `writer#1`/`writer#2`）确实并行启动
    - 输出归因（`[writer#1:out]`）可观测
    - 事件路由契约（topic_contracts）能驱动 end-to-end 流转
  - 同时写一份“跑法 + 排障 + 诊断路径”的文档，并用 Mermaid 图表达流程
- 方向B（先能用，后面再优雅）：只写“手工 E2E 跑法/清单”，不新增自动化场景
  - 优点：改动小、见效快
  - 缺点：不可自动回归，容易在版本迭代中漂移

### 执行计划（接下来几步）
1. 盘点现有 `ralph-e2e` harness 的能力（filter/keep-workspace/diagnostics）与现有 Tier 场景。
2. 设计并行模式的最小 E2E 场景：一条 `build.task` fanout 到 `writer`/`tester` 多实例，done 事件回送 `ralph#1`，最后 `LOOP_COMPLETE`。
3. 落地：
   - 新增/注册 scenario
   - 补齐 docs/specs 文档（含 Mermaid graph + sequenceDiagram，并用 mermaid-validator 校验）
4. 本地验证（能跑的就跑）：至少 `cargo test -p ralph-e2e` + `cargo run -p ralph-e2e -- --list` 确认可发现。

## 2026-01-27 10:38:39 +0800｜E2E 流程改造：只跑 Codex + 硬退出 + 解析 source_instance

### 我正在做什么 & 为什么
- 我正在把并行模式的 E2E 流程从“只跑 Claude”切换为“只跑 Codex”。
- 同时我会补齐两条可靠性硬门槛：
  1. E2E timeout 时要 **强制杀掉 ralph 进程组**，避免后台残留子进程拖慢后续测试。
  2. 让 E2E harness 解析 `events.jsonl` 里的 `source_instance`，把“实例归因”从 stdout 前缀升级为结构化断言能力。

### 我接下来要做什么（分步可验收）
1. 为 `ralph-e2e` 增加 `Codex` backend（CLI 选项 + AuthChecker + 文档）。
2. 将并行 E2E 场景 `parallel-hat-instances` 的 supported_backends 改为 **只跑 Codex**。
3. 改造 `RalphExecutor`：
   - timeout 时强制终止 ralph 进程组（SIGTERM → grace → SIGKILL）
   - 读 events.jsonl 时优先读取 `.ralph/events.jsonl`，并可合并 marker 指向的外部 events 文件
4. 改造 events 记录与解析：
   - `ralph-core` 的 event logger 增补 `source_instance` 字段（可选）
   - `ralph-e2e` 解析 `source_instance` 并在并行场景里加一条断言
5. `cargo fmt/clippy/test` 全量跑一遍，确保无回归。

### 现在我会停在这里
- 我不会替你写任何工件内容。
- 下一步如果你要继续：你可以直接说“继续”，或运行 `/opsx:continue`，我会帮你起草 `proposal.md`。

## 2026-01-26 20:31:13 +0800｜/opsx:continue：已创建 proposal 工件（只创建一个）

### 我正在做什么 & 为什么
- 我正在严格按 OpenSpec 的 `status → instructions → 写入工件 → 复查 status` 流程推进。
- 这样做能把依赖关系锁死，减少“跳步写错工件”导致的返工。
- 同时遵守“一次 invocation 只创建一个工件”，让每一步都可回滚、可 review。

### 已完成动作（摘要）
- 选择 change：当前只有一个活跃 change，因此继续 `parallel-hat-instances`
- 读取状态（JSON）：确认首个 ready 工件为 `proposal`
- 读取 instructions（JSON）：拿到模板、输出路径与 unlocks
- 写入工件文件：`openspec/changes/parallel-hat-instances/proposal.md`
- 复查状态：进度已从 `0/4` 变为 `1/4`

### 当前状态（给你一眼扫清）
- schema：`spec-driven`

## 2026-01-27 02:02:33 +0800｜继续实现 parallel-hat-instances：推进 5.x Workspace/权限 + 7.x Fixtures

### 我正在做什么 & 为什么
- 我正在接着上一次的实现进度往前推进。
- 目前 OpenSpec 的 tasks 里，1.x~4.x 已经完成。
- 但 5.x~8.x 还没完成。
- 并且 5.x 的实现已经开始动到 `crates/ralph-core/src/parallel/instance.rs`。
- 这类改动很容易造成“调用点没同步”的编译断裂。
- 所以我会先修复编译，再继续把 5.x 的能力闭环做完。

### 本轮计划（按优先级）
- [ ] 先跑 `cargo check`，用编译错误定位未对齐的接口/调用点
- [ ] 对齐 `HatInstanceHandle::spawn(...)` 新签名的所有调用处（Supervisor + Router）
- [ ] 打通 permission 与 human gate 的最小闭环（Ask/Allow/Deny + capabilities）
- [ ] 实现 worktree acquire/release + hooks（on_acquire/on_release）与失败自愈回路（bounded retry + escalate）
- [ ] 增补 replay-based smoke fixtures，锁死并行行为（含 queue 决策落盘与 replay 不重算）
- [ ] 全量验证：`cargo fmt --check`、`cargo clippy`、`cargo test`，并把关键点写入 `WORKLOG.md`（必要时写入 `ERRORFIX.md`）

### 需要特别留意的潜在坑
- 目前外部事件读取走 `.ralph/current-events` marker，但 `EventLogger` 默认写 `.ralph/events.jsonl`。
- 这会造成同一轮 run 的事件分散在两个文件里。
- 我会在做 5.x/7.x 的验证时顺带确认它是否影响 replay/可观测性，并决定是否需要统一路径（尽量收敛改动，不做无谓新增）。

### 进展小结（到这里为止）
- [x] 已修复并行 runtime 的编译断裂：对齐 `HatInstanceHandle::spawn` 新签名调用点，并补齐 Supervisor 对 `HatInstanceEvent::Published` 的处理。
- [x] 已落地 5.1/5.2 的“策略+权限”骨架：
  - 支持从 hat config 读取 `workspace.strategy`（shared/patch/worktree）
  - 支持 capabilities 白名单（`workspace.worktree` / `workspace.hooks`）
  - 支持 permission allow/ask/deny，并在 ask 时发 `gate.request`（回包 gate.resolve 后继续启动 job）
- [x] 已落地 5.3/5.4 的“worktree+hooks”闭环：
  - worktree 目录位于 `parallel.workspace.worktree_base_dir`（默认 `.ralph/worktrees`）
  - on_acquire/on_release hooks 支持 bounded retry（默认 3 次）
  - hooks 失败会发布 `workspace.hook_failed`（投递到 `ralph#1`，并由 Supervisor 落盘），方便自愈/排查
- [x] 已补齐 6.x 的“日志模式最小展示”：
  - CLI 启动时打印实例列表（initial=created）
  - 运行中打印实例状态变更（`[writer#1:state] running` 这类行）
  - 输出流支持 `--instance writer#1` 过滤，且每行输出都带 `HatInstanceId`
  - `ralph emit --target-instance writer#1 ...` 可定向投递外部事件（用于 human async chat 最小闭环）
- [x] 已补齐 7.x 的“确定性护栏测试”（不依赖真实后端）：
  - 新增 `crates/ralph-core/src/parallel/supervisor/routing_tests.rs`：覆盖并行、missing、require_delivery escalate、queue 决策 replay
  - 已跑通：`cargo test -p ralph-core smoke_runner`
- [x] 已补齐 8.1 文档最小更新：
  - `README.md` 增加 “Experimental: Parallel Hat Instances (Headless)” 配置示例与 BREAKING 提示
- progress：`1/4 artifacts complete`
- 已完成：`proposal`
- 已解锁（ready）：`design`、`specs`
- 仍阻塞：`tasks`（blocked by `design, specs`）

## 2026-01-26 20:39:43 +0800｜/opsx:continue：准备创建 design 工件（只创建一个）

### 我正在做什么 & 为什么
- 我正在继续同一个 change：`parallel-hat-instances`。
- 当前 `design` 与 `specs` 都处于 ready。
- 我本次只会创建 **第一个 ready** 的工件：`design`。
- 这样做能保持工件顺序与依赖关系可控，也符合“一次 invocation 只创建一个工件”的规则。

### 下一步动作（马上执行）
1. 读取 `openspec instructions design --json`，拿到模板与输出路径
2. 读取依赖工件 `proposal.md` 作为上下文
3. 写入 `openspec/changes/parallel-hat-instances/design.md`
4. `openspec status` 复查进度，并停止

### 执行结果（2026-01-26 20:41:09 +0800）
- 已创建工件：`design`
  - 文件：`openspec/changes/parallel-hat-instances/design.md`
- 复查状态：`2/4 artifacts complete`
  - 已完成：`proposal`、`design`
  - 待完成：`specs`
  - `tasks`：blocked by `specs`
- 本次严格只创建一个工件，按流程停止在这里。

## 2026-01-26 23:23:21 +0800｜/opsx:ff：已快进补齐全部工件（apply-ready）

### 我正在做什么 & 为什么
- 我正在把 change 推进到“可开始实现”的状态（apply-ready）。
- 这样做的意义是：requirements/specs/design/tasks 都齐了，后续实现可以严格按 tasks 逐项推进，并用测试做 backpressure。

### 已完成动作（按依赖顺序）
- 生成 `specs` 工件：
  - 新增：`openspec/changes/parallel-hat-instances/specs/parallel-hat-instances/spec.md`
  - 以 proposal 里的 capability 为唯一来源：`parallel-hat-instances`
- 生成 `tasks` 工件：
  - 新增：`openspec/changes/parallel-hat-instances/tasks.md`

### 当前状态（最终）
- schema：`spec-driven`
- progress：`4/4 artifacts complete`（All artifacts complete）
- applyRequires：`tasks`（已完成）

### 下一步建议
- 如果你要进入实现：我建议直接运行 `/opsx:apply`，从 `tasks.md` 的 1.1 开始按依赖推进。

### 校验结果
- `openspec validate parallel-hat-instances --type change` 已通过（2026-01-26 23:25:29 +0800）

## 2026-01-27 00:16:00 +0800｜/opsx:apply：进入实现阶段（开始逐项完成 tasks.md）

### 我正在做什么 & 为什么
- 我正在把 `parallel-hat-instances` 从“工件齐全（apply-ready）”推进到“代码与测试齐全（apply-done）”。
- 我会严格按 `openspec/changes/parallel-hat-instances/tasks.md` 的顺序推进。
- 每完成一项，就立刻把对应的 `- [ ]` 改成 `- [x]`，避免进度漂移。

### 本轮优先级（先打地基，再做并行）
1. 先完成 1.x：补齐基础类型、把“路由决策记录”真正落盘，为后续 replay 与并行调度打底。
2. 再进入 2.x：实现 HatInstance actor + HatJob + headless runner（并行的最小闭环）。
3. 最后补齐 3.x~8.x：路由语义、human gate、workspace 权限、Supervisor 展示、fixtures 与全量验证。

### 风险提醒（提前锁死，避免返工）
- 事件日志当前会截断 payload（`EventRecord::MAX_PAYLOAD_LEN=500`），但“路由决策记录”必须可解析、可回放。
  - 所以我会为决策类事件提供“不截断落盘”的路径，避免 JSON 被截断导致 replay 失败。

## 2026-01-27 01:05:30 +0800｜/opsx:apply：继续实现（优先补齐 3.x 路由语义）

### 我正在做什么 & 为什么
- 我正在把并行运行时从“能跑通”推进到“语义可依赖、可回放、可验证”。
- 现阶段最缺的是 3.x：`queue / fanout` 的路由规则还不完整。
- 如果不先把路由语义锁死，后续 gate/workspace/fixtures 都会建立在不稳定的行为上，返工成本会很高。

### 两个可选推进方向（本轮我会先走 B，保证闭环可测）
- 方向 A（不惜代价，最佳方案）：一次性把 3.x 全部做完（显式 TopicContract + 受众交集 + missing 策略 + LLM queue 决策 + 决策落盘 + fanout），然后立刻补 7.x fixtures 把行为钉死。
- 方向 B（先能用，后面再优雅）：先把 3.1~3.4 + 3.6 的“规则与落盘”补齐，让路由行为确定；3.5 先用 deterministic（round-robin/least-busy）跑通，再补 LLM 决策型 job 的完整链路。

### 下一步动作（马上执行）
1. 读取并对齐当前实现：`crates/ralph-core/src/parallel/supervisor.rs` 的 `route_event()` 与 `crates/ralph-proto/src/routing.rs` 的协议约束
2. 完成 3.1~3.4：强制显式 TopicContract + recipients 交集 + missing 分支（best-effort / require_delivery）
3. 完成 3.5~3.7：queue 选择（deterministic + llm）+ 强制 `dispatch.decision` 落盘 + fanout 投递可观测
4. 更新 `openspec/changes/parallel-hat-instances/tasks.md` 的 3.x 勾选
5. 运行 `cargo test -p ralph-core` / `cargo test -p ralph-cli`，并在本仓库 smoke fixtures 阶段前确保不引入回归

## 2026-01-27 10:54:53 +0800｜补齐 Parallel Hat Instances 的 E2E 测试流程（Codex + 强制退出 + source_instance）

### 我正在做什么 & 为什么
- 我正在把 `parallel-hat-instances` 这项改动的 **E2E 测试链路**做成“可重复跑、可排障、可硬退出”的流程。
- 这样做是为了让并行 runtime 这种“更容易卡住/漂移”的能力，在真实后端下也有确定性的护栏。
- 同时我会让 E2E harness 在解析 `.ralph/events.jsonl` 时读取 `source_instance`，这样 report/断言才能看见“事件是谁发的”。

### 你要的三个改动（本次范围）
1. **只跑 Codex**：把并行 E2E 场景的推荐命令从 `-- claude` 改为 `-- codex`（避免跑空场景）。
2. **硬退出条件增加**：E2E 超时/卡死时要能强杀 `ralph run` 及其后端子进程，避免残留影响下一次。
3. **解析 source_instance**：E2E harness 读取 events.jsonl 时，把 `source_instance` 写进结构体，供断言/报告使用。

### 两个可选实现方向（我会先走 B，保证尽快可用）
- 方向 A（不惜代价，最佳方案）：
  - E2E executor 做“实时 stdout/事件监控”，满足条件就提前终止；
  - 增加“无输出/无事件”的 idle watchdog；
  - report 里按 `source_instance` 聚合事件，直接定位卡在哪个实例。
- 方向 B（先能用，后面再优雅，推荐）：
  - 先把 **现有强杀逻辑做对**（确保 ralph 子进程处于独立进程组，timeout 时 SIGTERM→SIGKILL 杀整组）；
  - 补齐 `EventRecord { source_instance }` 的字段与单测；
  - 并行场景文档命令改为 `-- codex`，并补充最小排障步骤。

### 下一步动作（马上执行）
1. 修复 `crates/ralph-e2e` 内所有 `EventRecord { ... }` 构造，补齐 `source_instance`
2. 强化 `RalphExecutor` 的“进程组强杀”实现（保证 timeout 能杀干净）
3. 更新 `specs/parallel-hat-instances/e2e.md`：只跑 codex + 失败排障路径
4. 跑 `cargo fmt --check` + `cargo test`（至少覆盖 `-p ralph-e2e` 与 `-p ralph-core smoke_runner`）

### 执行结果（已完成）
- 文档命令已统一为 `-- codex`（避免跑空场景）
- E2E harness 已解析 `source_instance` 并写入 `ExecutionResult.events`
- timeout 强杀已改为“独立进程组 + getpgid + SIGTERM→SIGKILL”
- 全量验证已通过：
  - `cargo fmt --check` ✅
  - `cargo clippy --all-targets --all-features -- -D warnings` ✅
  - `cargo test` ✅
  - `cargo test -p ralph-core --test smoke_runner` ✅

## 2026-01-27 11:16:30 +0800｜制作全自动运行 parallel-hat-instances 环境并执行一次

### 我正在做什么 & 为什么
- 我正在把“并行 E2E 场景”包装成一个 **一键可跑** 的脚本/流程，并在本机实际跑一遍。
- 这样做是为了让你以后不需要记复杂命令。
- 同时也能把“环境缺失/认证缺失”这种问题，用明确错误快速暴露出来，避免误以为测试已覆盖。

### 两个可选方向（我会先走 B，尽快落地）
- 方向 A（不惜代价，最佳方案）：
  - 做一个独立 workspace 生成器（生成 ralph.yml + prompt + diagnostics 配置），并支持多种后端切换。
  - 在脚本中做更多自检（codex 安装/认证/网络），并给出可读的失败原因。
- 方向 B（先能用，后面再优雅，推荐）：
  - 直接复用现有 `ralph-e2e` harness（它已经能生成 workspace、写 report、收集 diagnostics）。
  - 新增一个脚本把“只跑并行场景 + codex + keep-workspace + verbose”固定下来。

### 下一步动作（马上执行）
1. 新增脚本：`scripts/run-parallel-hat-instances-codex.sh`（一键运行并行 E2E）
2. 运行脚本并收集结果（成功则给出 report/workspace 路径；失败则给出缺失项）

### 实际运行结果（本轮失败，已定位根因）
- 现象：`parallel-hat-instances` E2E 立即失败（0.5s），stdout 为空，exit code=1
- 根因：`RalphConfig::validate()` 仍按“串行 hats”规则校验，禁止多个 hat 共享同一 trigger
  - 报错：`Ambiguous routing: trigger 'build.task' is claimed by both 'writer' and 'tester'`
  - 但在 **parallel mode** 下，`build.task` 的 fanout 语义本来就需要被多个 hat 同时消费

### 下一步动作（马上执行）
1. 放宽配置校验：仅在 `parallel.enabled=false` 时才强制“trigger 唯一”
2. 重新运行 `scripts/run-parallel-hat-instances-codex.sh`，确保并行 E2E 能真正跑起来

## 2026-01-27 12:28:33 +0800｜继续：让 parallel-hat-instances E2E 真正“全自动跑完并可判定”

### 我正在做什么 & 为什么
- 我正在继续解决：`parallel-hat-instances` 的 E2E 会跑很久，需要手动 kill，且 `events.jsonl` 里没有 `build.done/test.done`。
- 这会让“并行 runtime 的 E2E 护栏”失去意义。
- 我需要把它修成：一键脚本能稳定结束（成功或明确的 limit 退出），并且 report 能看到完成事件与 `source_instance`。

### 现场复盘（基于现有 `.e2e-tests/parallel-hat-instances/`）
- `events.jsonl` 里只有 `build.task`，没有 `build.done/test.done`。
- 但 stdout 断言里能看到 `writer#1/writer#2/tester#1` 的 state/out 前缀，说明 fanout/实例启动路径在跑。
- 并行 Supervisor 当前只在检测到 `LOOP_COMPLETE` 时退出，没有实现 `max_iterations/max_runtime` 的硬退出护栏。
  - 这会导致：只要 ralph#1 不输出 `LOOP_COMPLETE`，进程就可能无限运行，E2E 必然卡住。
- 更关键的是：并行实例的 prompt 组装把“顶层 prompt（-p）”无差别注入到所有 hat。
  - 在该场景里，顶层 prompt 是“协调者 ralph#1 要做的事”。
  - writer/tester 也看到了这段 prompt，容易被污染，不按它们的 hat instructions 发 `build.done/test.done`。

### 两个可选修复方向（我会先走 B，尽快跑通 E2E 闭环）
- 方向 A（不惜代价，最佳方案）：
  - 重构并行 prompt 语义：把“顶层 prompt”只作为 `task.start` payload，通过 TopicContract 决定谁能看到；
  - 让并行 Supervisor 的“迭代/退出”语义与串行 event_loop 完全对齐；
  - 再补充更强的 idle watchdog（无事件/无输出 N 秒即退出）。
- 方向 B（先能用，后面再优雅，推荐）：
  - 先阻断 prompt 污染：只有 `ralph` 实例才注入顶层 prompt，其他 hat 只看自己的 instructions + incoming events；
  - 给 ParallelSupervisor 加上硬退出护栏：`max_runtime_seconds`（以及一个保守的 `max_iterations` 计数）；
  - E2E 场景把 `max_runtime_seconds` 设成较小值，确保“没有 LOOP_COMPLETE 也能自动结束”，避免卡死。

### 下一步动作（马上执行）
1. 修改并行 prompt 组装逻辑，避免 writer/tester 看到 ralph#1 的顶层 prompt
2. 在 `ParallelSupervisor::run` 增加 `max_runtime_seconds/max_iterations` 退出护栏
3. 调整 E2E 场景的 `ralph.yml`（加入较小 `max_runtime_seconds`），并重新跑脚本验证

### 执行结果（2026-01-27 13:01:41 +0800）
- ✅ 并行模式不再需要手动 kill：加入了硬退出护栏（max_runtime/max_iterations），并在 E2E 场景设置了 `max_runtime_seconds: 240`。
- ✅ 修复了“writer/tester 不产出完成事件”的根因：
  - 并行模式下，custom hat 如果已经提供了 `instructions`，就不再套用 InstructionBuilder 的重型模板（避免强制跑 tests 导致超时）。
  - E2E 场景的 writer/tester instructions 明确声明：禁止跑测试/禁止跑命令，必须立即输出事件。
- ✅ `source_instance` 已在 `.ralph/events.jsonl` 中可见，report/断言可以按实例归因。

### 验证（本机已跑）
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `bash scripts/run-parallel-hat-instances-codex.sh` ✅（E2E: parallel-hat-instances 通过）

## 2026-01-27 13:44:59 +0800｜新增：parallel job-level timeout + per-hat override

### 我正在做什么 & 为什么
- 我正在把并行 runtime 的“硬退出”从 loop 级（max_runtime/max_iterations）进一步补齐到 **job 级 timeout**。
- 我这样做是为了避免某个 hat 的单次 headless job 卡死后，拖到全局 max_runtime 才止损。
- 同时你认可需要这个能力，并要求加入 **per-hat override**，让不同 hat 能按职责设定不同的 job 超时时间。

### 两个可选实现方向（我会走 B：改动最小，语义清晰）
- 方向 A（不惜代价，最佳方案）：
  - 做“全链路 timeout”统一：串行 event_loop / 并行 supervisor / e2e executor 全部用同一套 timeout 口径；
  - 支持更细粒度 override（per-hat/per-instance/per-topic），并加入 no-progress watchdog（N 秒无输出/无事件则 abort）。
  - 优点：能力最完整；缺点：改动面大，容易引入新耦合。
- 方向 B（先能用，后面再优雅，推荐）：
  - 复用现有 `adapters.*.timeout` 作为 **默认 job timeout**（按 hat backend 或默认 cli backend 推导）；
  - 在 `hats.<hat>.job_timeout_secs` 提供 override：
    - 未设置：继承 adapters 的默认值
    - 设为 `0`：显式禁用 timeout（None）
    - 设为 `>0`：使用该秒数
  - 优点：改动小、可预测；缺点：对 `auto/custom` backend 的推导先按现有 fallback（后续再增强）。

### 下一步动作（马上执行）
1. 在 `HatConfig` 增加 `job_timeout_secs` 字段，并补齐解析/默认语义
2. 在 `ParallelSupervisor::spawn_instances` 计算每个 hat 的 job timeout，并注入到 HatInstance actor
3. 在 HatInstance 里把 `HatJob.timeout` 从 None 改为使用计算结果
4. 增加单测覆盖：默认继承 + per-hat override + 0 禁用
5. 跑 `cargo fmt/clippy/test` + `bash scripts/run-parallel-hat-instances-codex.sh` 验证

### 遇到的错误 & 调整（2026-01-27 14:25:14 +0800）
- [错误] `parallel-hat-instances` E2E 断言偶发失败：只有 `build.task`，没有 `build.done/test.done`。
  - [原因] 并行 Supervisor 之前在检测到 completion promise 时“立刻 break”，会导致同一轮输出里解析出的事件还没来得及路由，进而下游实例没机会产出事件。
  - [原因] completion drain 窗口太短（15s）时，真实后端冷启动/慢响应会被提前 cancel，仍可能丢事件。
  - [决议] completion promise 改为“软退出信号”：先路由事件，再进入 drain（min 0.5s / max 60s），给下游实例收尾时间。

### 执行结果（2026-01-27 14:25:14 +0800）
- ✅ 并行 job-level timeout 已接入（并支持 per-hat override）：
  - 新增配置：`hats.<hat>.job_timeout_secs`
    - 未设置：继承 `adapters.<backend>.timeout`
    - `0`：禁用 timeout
    - `>0`：使用该秒数
  - timeout 解析位置：Supervisor spawn 时计算并注入到 HatInstance；执行器按 `HatJob.timeout` 强制 kill。
- ✅ completion promise 行为更稳：
  - 不再“立刻 break”，而是先路由同轮解析出的事件，并做短暂 drain，避免丢最后一波事件。
- ✅ E2E 场景调整为更“机械化”：
  - ralph prompt：只负责发 `build.task` + 输出 `LOOP_COMPLETE`（避免把等待/观察交给模型）
  - `max_runtime_seconds` 调整为 120（避免 240s 过慢，又不会卡死）

### 验证（本机已跑）
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `bash scripts/run-parallel-hat-instances-codex.sh` ✅（E2E: parallel-hat-instances 通过，约 96s）

## 2026-01-27 15:34:41 +0800｜改造：adapters/job_timeout 由“硬超时”改为“检测超时（看输出是否停滞）”

### 你新增的需求（我已理解）
- 你希望把 `adapters.*.timeout` 改为“检测超时”，而不是“一到时间就 kill”：
  - 默认检测窗口：1 小时
  - 当检测窗口到期时：
    - 如果 stdout/stderr 输出内容 **超过 N 分钟没有任何变化**（默认 30 分钟），才判定超时（kill）
    - 否则判定“检测通过”，并把检测计时 **重新从此刻开始计算**（继续跑）
- `job_timeout_secs` 也要沿用同样的语义（不能一到时间就终止 job）

### 我正在做什么 & 为什么
- 我正在把“超时”的语义从“最大运行时长”升级成“输出停滞检测（stall watchdog）”。
- 我这样做是为了：
  - 允许长任务（例如大改动/大量输出）持续运行，只要它确实在产出进展信号；
  - 同时仍能在 job 卡死/无输出的情况下可靠止损，避免无人值守挂死。

### 两个可选实现方向（我会走 B：最贴近你描述的语义）
- 方向 A（不惜代价，最佳方案）：
  - 抽象出统一的 `TimeoutPolicy`（hard_timeout + stale_timeout + check_interval），串行/并行/bench/e2e 全部共用；
  - 支持 per-hat / per-instance / per-topic 更细粒度覆盖；
  - 代价：改动面大，需要更多迁移与回归测试。
- 方向 B（先能用，后面再优雅，推荐）：
  - `adapters.*.timeout` 直接改成“检测窗口（check interval）”，默认 3600s；
  - 新增 `adapters.*.output_stale_timeout_secs`（默认 1800s）作为“输出停滞阈值”；
  - job 执行逻辑：
    - 每当检测窗口到期：如果距离最近一次输出更新已超过 stale 阈值 -> timed_out
    - 否则重置检测窗口（重新计时），继续执行
  - `job_timeout_secs` 仍作为“检测窗口覆盖值”，但停滞阈值沿用该 hat backend 对应 adapter 的 `output_stale_timeout_secs`

### 下一步动作（马上执行）
1. 扩展 `AdapterSettings`：默认 timeout=3600（1h），新增 output_stale_timeout_secs=1800（30m）
2. 改造 `CliExecutor` 与 `CliHatJobExecutor`：把 hard timeout 改为检测超时（stall watchdog）
3. 并行 runtime：HatJob 传递 output_stale_timeout，并在 Supervisor spawn 时按 backend 推导
4. 更新/补齐单测：覆盖“到点检查+重置计时”与“停滞超时”两条路径
5. 跑 `cargo fmt/clippy/test` + `bash scripts/run-parallel-hat-instances-codex.sh` 验证

## 2026-01-27 16:02:00 +0800｜解释：parallel.topic_contracts（TopicContract 路由契约）到底做了什么

### 我正在做什么 & 为什么
- 我正在从代码里把 `topic_contracts` 的“匹配规则 + 投递规则 + 兜底/报错规则”完整串起来。
- 我这样做是为了避免只讲概念。
  - 你会感觉“好像懂了”，但遇到 `audience_override/target_instance` 就又迷糊。
  - 直接对照代码梳理，会更确定，也更容易排查配置问题。

### 阶段
- [x] 阶段1: 定位入口（TopicContractStore / route_event）
- [x] 阶段2: 梳理规则（pattern 匹配/排序、audience 计算、queue/fanout、缺失实例策略）
- [x] 阶段3: 用示例解释，并同步到 notes/WORKLOG

### 状态
**已完成阶段3**
- 我已经把路由流程按代码执行顺序梳理清楚，并补齐了关键细节（target_instance / audience_override / missing policy）。
- 我已把速记追加到 `notes.md`，并把本次交付记录写入 `WORKLOG.md`，方便未来回看与排障。

## 2026-01-27 15:53:32 +0800｜继续：落地 adapters.*.timeout 的“检测超时”实现（修复当前编译错误）

### 我正在做什么 & 为什么
- 我正在把 `ralph-adapters::CliExecutor` 的 timeout 行为，从“硬超时（tokio::time::timeout 到点就 kill）”改成“检测超时（stall watchdog）”。
- 我这样做是因为：
  - 你明确要求：`adapters.*.timeout` 到点后要先看 stdout/stderr 是否长期无变化，只有停滞超过阈值才算超时；否则应当视为检测通过并重置计时。
  - 当前代码虽然已经在配置层（`AdapterSettings`）写了该语义的注释与默认值，但 `CliExecutor` 仍是硬超时实现，导致语义不一致。
  - 另外目前 workspace 处于 **编译失败** 状态（`execute_capture_with_timeout` 调用参数缺失），需要先修到可编译，才能继续做语义验证。

### 本轮计划（只做必要改动，优先闭环）
1. 修复 `CliExecutor` 的编译错误（补齐 `output_stale_timeout` 参数传递）
2. 在 `CliExecutor::execute` 实现“检测超时”主循环（参考 `ralph-cli/src/parallel_runner.rs` 的实现）
3. 更新 ralph-cli / ralph-bench 的调用点：传入 `output_stale_timeout`（来自 `adapters.*.output_stale_timeout_secs`）
4. 更新 `ralph-adapters` 单测：用小时间窗口验证“停滞才超时 / 有输出则不超时”
5. 跑 `cargo test` 验证（含 smoke tests）

### 状态
**目前在阶段：执行/构建**
- 我现在先修编译错误，然后实现检测超时循环，再补测试与全量验证。

### 执行结果（2026-01-27 15:59:35 +0800）
- ✅ `ralph-adapters::CliExecutor` 已从“硬超时”改为“检测超时（stall watchdog）”：
  - 检测窗口到期时，只在 stdout/stderr 输出停滞超过阈值才判定超时并终止；
  - 若输出仍有变化，则视为检测通过，并从当前时刻重新计时下一轮检测窗口。
- ✅ ralph-cli / ralph-bench 的 `CliExecutor::execute` 调用点已补齐 `output_stale_timeout` 传入（来自 `adapters.*.output_stale_timeout_secs`）。
- ✅ 修复了当前 workspace 的编译错误：`execute_capture_with_timeout` 缺参导致无法通过 `cargo check`。

### 验证（本机已跑）
- `cargo fmt` ✅
- `cargo check` ✅（无 warnings）
- `cargo test` ✅

## 2026-01-27 16:50:06 +0800｜增强：backend=custom 且 command=codex 时，timeout 配置自动映射到 adapters.codex

### 你新增的需求（我已理解）
- 你希望在如下配置下：
  - `cli.backend: "custom"`
  - `cli.command: "codex"`
  - `cli.args: [...]`（可选）
- timeout 的来源不要再回退到 `adapters.claude`，而是应当自动使用 `adapters.codex`（包含 `timeout` 与 `output_stale_timeout_secs`）。

### 我正在做什么 & 为什么
- 我正在把“custom backend 的 timeout 选择”从“按 backend 字符串匹配（custom -> fallback claude）”升级为“按实际 command 推导（command=codex -> 用 codex settings）”。
- 我这样做是为了让配置更直觉：
  - 运行时确实是 `codex` 在执行；
  - timeout 也应当跟着 `codex` 这组 adapter 配置走，避免用户误以为 `adapters.codex` 生效但实际上没用。

### 计划（改动最小，覆盖串行+并行）
1. 在 `RalphConfig::adapter_settings()` 增加映射：当 `backend == "custom"` 且 `cli.command == "codex"` 时返回 `adapters.codex`
2. 并行模式下 hat backend 是 custom 时，也用同样的“按 command 推导”规则（避免 per-hat custom=codex 时仍走 fallback）
3. 增加单测覆盖该映射语义
4. 跑 `cargo test` 验证

## 2026-01-27 18:43:46 +0800｜Explore：parallel-trigger-routing（并行默认 triggers fanout + 自动扩缩容 + workspace override）

### 我正在做什么 & 为什么
- 我正在把“并行模式的路由/实例/工作区语义”重新定义成：默认按 hats.triggers 做 fanout（到 hat 级别），每个 hat 再 queue 到单个实例执行。
- 我这样做是因为当前实现/README 强制要求 `parallel.topic_contracts` 显式配置且无隐式 broadcast。
- 这与我们想要的产品语义冲突：
  - 不写 topic_contracts 也能靠 triggers 跑起来。
  - 同一 topic 的多个订阅者 hat 会真正并发启动。

### 已达成的关键决策（本轮已拍板）
- 默认投递：topic → 所有订阅该 topic 的 hats（fanout），但不 fanout 到一个 hat 的所有实例；每个 hat 仅选 1 个实例执行。
- `event.target` / `event.target_instance`：启用严格校验（目标必须订阅该 topic），但允许对少数控制面事件做特例。
- 自动扩缩容：
  - 空闲实例优先；全忙则动态创建实例。
  - 全局并发上限默认 4（安全刹车）。
  - 动态实例 idle 超过 30s 自动回收。
  - 实例 key 单调递增且永不复用（方案 A）。
- workspace override：
  - 走 Event 显式字段/属性（而不是编码进 topic 字符串）。
  - 多事件合并成 job 时采用“最强隔离优先”合并策略：worktree > patch > shared。

### 下一步（探索产物）
1. 创建新的 OpenSpec change：`parallel-trigger-routing`
2. 在 proposal/design/specs/tasks 中把上述语义固化，并明确 BREAKING 点（README/旧 spec 需要更新）
3. 给出两套实现路径供选择：
   - A：并行模式默认按 triggers 路由，TopicContract 变为可选覆盖
   - B：启动时从 triggers/publishes 自动生成 TopicContracts（用户不写也能跑）

### 状态
**已完成：OpenSpec 工件创建**
- ✅ 已创建 change：`parallel-trigger-routing`
- ✅ 已写入工件：proposal / design / specs / tasks
- ✅ `design.md` 内的 Mermaid 图已通过 `mermaid-validator` 校验
- ✅ `openspec validate parallel-trigger-routing --type change` 已通过（2026-01-27 19:04:20 +0800）

## 2026-01-27 19:54:45 +0800｜/opsx:apply：开始实现 parallel-trigger-routing

### 我正在做什么 & 为什么
- 我正在把 `parallel-trigger-routing` 这个 change 的 tasks 真正落到代码里（不再停留在 spec）。
- 我这样做是为了让 `parallel.enabled=true` 的默认体验变得直觉：只写 `hats.*.triggers` 也能并发跑起来。
- 同时我会把 autoscale（默认 cap=4 / idle=30s）与 workspace_strategy override（worktree > patch > shared）一起落地，避免“语义分裂”（代码/README/spec 三方不一致）。

### 两条实现路线（我会先按 A 推进，B 作为降级备选）
- 方向 A（推荐，符合已拍板语义）：TopicContract 作为可选 override；未命中时按 triggers 默认 fanout（到 hat），再 queue 到单实例。
- 方向 B（先能用，兼容旧心智）：启动时从 triggers/publishes 自动生成 TopicContracts（用户不写也能跑，但本质仍是隐式生成规则）。

### 我接下来要做什么（按 tasks.md 顺序推进，逐条打勾）
1. 先落地配置/协议（autoscale 配置项、Event.workspace_strategy、控制面 topic 特例）。
2. 再改并行路由：移除 topic_contracts 硬依赖 + triggers 默认路由 + strict target 校验 + hat-level fanout。
3. 最后补齐实例调度（单调递增实例 id、idle-first、autoscale、全局 cap、idle reaper）。
4. 跑单测 / replay smoke / E2E，最后同步 README 与 specs，确保整个仓库一致。

### 状态
**目前：进入实现阶段（开始处理 tasks 1.x / 2.x）**

## 2026-01-27 22:28:33 +0800｜/opsx:apply：parallel-trigger-routing 已实现并验证通过

### 我刚完成了什么 & 为什么
- 我已经把 `parallel-trigger-routing` 的设计语义真正落到了代码里，并把 docs/tests/E2E/smoke fixture 同步到同一套语义上。
- 我这样做是为了避免“代码已经变了，但 README/spec 还在讲旧语义”的分裂，让团队成员靠文档就能推理并行行为。

### 完成情况（对应 tasks.md）
- `openspec/changes/parallel-trigger-routing/tasks.md` 已全部勾选完成（1.x ~ 7.x）。

### 验证结果（硬门槛）
- `cargo fmt --check` ✅
- `cargo clippy --workspace --all-targets` ✅
- `cargo test` ✅

## 2026-01-27 23:14:56 +0800｜E2E：parallel-trigger-routing 场景已跑通（Codex）

### 我正在做什么 & 为什么
- 我在补并真实执行 `parallel-trigger-routing` 的 E2E。
- 这样做是为了用“真实后端 + 真并行调度”验证：triggers 默认路由、strict target、autoscale 等承诺不会只停留在单测层面。

### 结果
- 首次 E2E 失败：原因是 `ralph-e2e` 使用了旧的 `target/debug/ralph`，我改了 core 但没重新 build。
- 修复后通过：先 `cargo build --bin ralph`，再运行 E2E 命令，场景通过。

## 2026-01-28 01:51:18 +0800｜示例：创建 parallel-trigger-routing 应用范例（examples/）

### 我正在做什么 & 为什么
- 我正在为 `parallel-trigger-routing` 增加一个“可直接跑起来”的应用范例，放在仓库根目录 `examples/` 下。
- 我这样做是为了让团队成员不用读代码也能直观理解并行模式的默认路由语义：
  - `topic -> hats`：按 `hats.*.triggers` 对所有订阅者 fanout
  - `hat -> instance`：每个 hat 只队列到一个实例（idle-first / RR）
- 同时，这个范例会刻意“不写 `parallel.topic_contracts`”，用来强调它现在是可选覆盖层。

### 我接下来要做什么
- [x] 创建 `examples/parallel-trigger-routing/`（README.md / ralph.yml / prompt.md）
- [x] 用 `ralph run --dry-run` 做一次配置解析自检（不调用外部模型）
- [x] 把产出记录追加到 `WORKLOG.md`

### 状态
**已完成：examples/parallel-trigger-routing 示例已创建并自检通过（2026-01-28 01:53:45 +0800）**

## 2026-01-28 02:11:10 +0800｜答复：parallel-hat-instances 的 Supervisor TUI 是否已实现？

### 我正在做什么 & 为什么
- 我在对照 `specs/parallel-hat-instances.spec.md` 的 8.x（Supervisor TUI 草案）和代码里的并行模式入口实现。
- 这样做是为了分清楚：这是“规格里想要的 UI”，还是“已经在代码里落地的 UI”，避免凭感觉误判。

### 结论（可直接回答用户）
- 目前 **没有实现 Supervisor TUI**；并行模式仍然是 **纯日志输出**（不会启动 TUI）。

### 证据点（可定位）
- 规格把 UI 明确标注为“草案”：`specs/parallel-hat-instances.spec.md` 的 `## 8. Supervisor TUI（高层交互草案）`。
- 代码在并行模式下显式提示“没有 TUI”：`crates/ralph-cli/src/parallel_runner.rs` 在 `enable_tui=true` 时会 `warn!("Parallel mode currently runs without TUI (log output only)")`。

### 下一步建议（如果要继续把 TUI 做出来）
- 先按 spec 8.1 的思路把 `ralph-tui` 的数据模型从“按 iteration”迁移到“按 instance/job”（例如把 `iterations: Vec<IterationBuffer>` 抽象为 `instances -> jobs -> buffer`）。
- 再把并行 runner 的输出观察者接入 TUI state（替换当前 stdout 日志模式），最后补一轮 TUI 验证（`/tui-validate`）。

## 2026-01-28 02:18:46 +0800｜新需求：实现 Supervisor TUI（含 human async chat + gate 面板），并先制定新的 OpenSpec

### 我正在做什么 & 为什么
- 我正在把“并行模式的日志输出”升级为真正的 Supervisor TUI，并把 human async chat + gate 面板一起纳入同一个实现闭环。
- 这样做是为了把 `specs/parallel-hat-instances.spec.md` 的 8.x 从“草案”推进到“可交互、可回放、可验证”的工程落地。

### 两条路线（先写进 OpenSpec，让后续实现不走偏）
- 方向 A（不惜代价，最佳方案 / 推荐）：
  - 直接在 `crates/ralph-tui` 内做“并行模式”，复用现有 buffer/搜索/滚动体验。
  - 关键重构：从 iteration 维度迁移到 instance/job 维度（和 spec 8.1 对齐），并把 chat/gate 作为第三个 pane。
  - 好处：长期一致性最好，用户心智迁移最低。
  - 代价：需要一次结构性重构（state/widgets/input 都要动）。
- 方向 B（先能用，后面再优雅 / 降级方案）：
  - 先做一个“并行专用 TUI”（仍放在 `crates/ralph-tui` 里，但尽量少改现有串行路径）。
  - 用“实例列表 + 当前实例输出 + chat/gate”最小闭环先跑通；instance/job 历史与更深的 buffer 抽象延后。
  - 好处：落地更快，改动面更可控。
  - 代价：后续可能需要二次重构把两套 state 合并，避免分裂。

### 我接下来要做什么（马上执行）
1. 创建新的 OpenSpec change（建议命名：`parallel-supervisor-tui`）。
2. 生成 proposal / specs / design / tasks（到 apply-ready）。
3. 跑 `openspec validate`，确保工件链路完整可用。

## 2026-01-28 02:25:30 +0800｜开始实现 parallel-supervisor-tui（方向A：完整重构并复用现有 TUI）

### 我正在做什么 & 为什么
- 我正在按 `openspec/changes/parallel-supervisor-tui/tasks.md` 开始落地实现，并严格采用你选择的 **方向 A**。
- 这样做是为了把并行模式从“只能看 stdout 日志”升级到“可交互的 Supervisor TUI”，并把 human async chat + gate 面板一次性做进闭环。

### 本轮实现的执行顺序（按依赖从小到大）
1. 先让并行 runner 真正启动 TUI（替换当前 `warn!(no TUI)` 的日志模式）。
2. 在 `crates/ralph-tui` 内引入 `TuiMode::Parallel` 与并行 state（instance→jobs→buffer），同时保持串行路径不破坏。
3. 做三 pane UI：实例列表 + 实例输出 + chat/gate（含焦点与导航）。
4. 补齐 job_id 分段、chat 事件落盘、gate 展示与 resolve。
5. 最后补齐 tests + replay fixture + `cargo fmt/clippy/test` 全量验证。

## 2026-01-28 11:34:09 +0800｜现状更新：parallel-hat-instances 的 Supervisor TUI 做到哪了？

### 我正在做什么 & 为什么
- 我在重新对照 `specs/parallel-hat-instances.spec.md` 的 8.x（Supervisor TUI 草案）与当前代码实现现状。
- 这样做是为了回答你问的“到底有没有实现”，并且把结论落到可定位的文件与 tasks 勾选状态上。

### 结论（直接回答）
- `specs/parallel-hat-instances.spec.md` 里的 Supervisor TUI **在规格里是“草案”**（8.x 明确写的是草案 + 复用策略）。
- 代码层面：Supervisor TUI **已经有骨架并能启动**，但 **human async chat + gate 面板还没有做完**（目前是占位 UI）。

### 证据点（可快速定位）
- 规格明确标注为草案：
  - `specs/parallel-hat-instances.spec.md`：`## 8. Supervisor TUI（高层交互草案）`
- 并行 runner 已改为启动并行 TUI（不再只 warn “no TUI”）：
  - `crates/ralph-cli/src/parallel_runner.rs`：`Tui::new_parallel()`
- 并行 TUI 的当前形态：
  - `crates/ralph-tui/src/app.rs`：并行模式渲染三块区域（instances / output / Chat-Gates 占位）
  - `crates/ralph-tui/src/app.rs`：并行模式输入目前只保留最小闭环（`q` 退出、`?` help）

### 你要求的“human async chat + gate 面板”目前状态
- 已经进入同一个 OpenSpec 变更里做，并且 tasks 已拆好：
  - change：`openspec/changes/parallel-supervisor-tui/`
  - chat + gate spec：`openspec/changes/parallel-supervisor-tui/specs/supervisor-human-chat-gate/spec.md`
  - 进度：`openspec/changes/parallel-supervisor-tui/tasks.md` 里 1.x/2.x/4.1 已完成；3.x/5.x/6.x/7.x 待完成

### 下一步（按 tasks.md 的“硬门槛顺序”继续）
1. 先补齐 3.x：Tab 焦点切换 + 实例选择 + 复用滚动/搜索（让右侧输出真正可用）。
2. 再补 5.x：chat 输入框编辑/提交，并落盘 `human.message` 到 `.ralph/current-events`。
3. 再补 6.x：gate 列表 + 倒计时 + `!approve/!deny/!resolve` 写入 `gate.resolve`。
4. 最后 7.x：单测 + replay fixture + `/tui-validate` + `cargo fmt/clippy/test` 全量验证。

## 2026-01-28 12:38:04 +0800｜进展：human async chat + gate 面板已进入可用闭环（事件可落盘）

### 我正在做什么 & 为什么
- 我在把并行 Supervisor TUI 的底部面板从“占位”升级为可交互的 human async chat + gate 面板。
- 这样做是为了让 human 的输入不再需要开另一个终端跑 `ralph emit`，而是直接在 TUI 里完成“发消息 / resolve gate”，并且通过 JSONL 落盘保证可回放。

### 本轮完成内容（对应 OpenSpec tasks）
- ✅ 5.x：chat 输入框（编辑/提交/取消）已实现
  - `@writer#2 hello` → 写入 `human.message`，并设置 `target_instance=writer#2`
  - `hello` → 写入默认 `human.message`
- ✅ 6.x：gate 面板（展示 + 倒计时 + resolve）已实现
  - 展示 `gate.request` 列表，超时 gate 显示 `T-xxs`，resolve/timeout 状态可见
  - 支持 `!approve <gate_id>` / `!deny <gate_id>` / `!resolve <gate_id> <text>`
  - 生成 `gate.resolve` payload（JSON，可反序列化为 `GateResolve`）并落盘
- ✅ 事件落盘路径遵循 Supervisor 约定：
  - 优先读取 `.ralph/current-events` 指向的 JSONL
  - 不存在则回退 `.ralph/events.jsonl`

### 验证
- `cargo test -p ralph-tui` ✅（包含 chat 解析单测）

### 下一步
- 进入 7.x：补齐 gate reducer/写入器的单测 + replay fixture + `/tui-validate` + 全量 `cargo fmt/clippy/test`。

## 2026-01-28 14:54:01 +0800｜/opsx:sync：同步 OpenSpec delta specs → main specs

### 我正在做什么 & 为什么
- 我正在准备把已完成 change 里的 delta specs，同步到仓库的“主规格”目录里（`openspec/specs/`）。
- 这样做是为了避免规格长期漂移：实现已经落地，但主规格为空会让新同学/未来自己找不到权威描述。

### 我发现的可同步对象（都存在 `specs/*/spec.md`）
- active changes（`openspec list --json` 可见）：
  - `parallel-hat-instances`
  - `parallel-trigger-routing`
- 已归档 change（仍有 delta specs）：
  - `archive/2026-01-28-parallel-supervisor-tui`

### 下一步（需要你选一个或多个）
- 我会在你确认要 sync 哪个 change 后：
  1) 读取对应的 delta spec 文件（`specs/*/spec.md`）
  2) 在 `openspec/specs/<capability>/spec.md` 中创建/更新主规格（当前该目录还是空的）
  3) 最后给出一份“新增/修改/移除”的同步摘要（可重复执行且幂等）
