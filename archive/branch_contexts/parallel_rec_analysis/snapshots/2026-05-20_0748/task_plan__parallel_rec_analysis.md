# 任务计划: parallel_rec 分析与 coordinator-only Codex hooks 配置落地

## [2026-05-18 23:03:00] [Session ID: omx-1779004640353-blcixq] 续档入口: role_args 落地收口

续档原因:
- 旧计划文件超过 1000 行,按文件上下文规则续档。
- 旧文件保留为 `task_plan__parallel_rec_analysis_2026-05-18_2257_over1000.md`。
- 当前支线继续使用 `task_plan__parallel_rec_analysis.md` 记录最终验收状态。

目标:
- 落地 `cli.role_args.coordinator / worker`,让 Ralph coordinator 可以单独追加 `-c features.hooks=false`,普通 worker hats 不继承这个参数。

阶段状态:
- [x] 阶段1: 阅读现有 `CliConfig`、role reasoning、backend 与 executor 角色判断路径。
- [x] 阶段2: 补配置解析、adapter、parallel executor、autopilot 子配置保留测试。
- [x] 阶段3: 实现 `RoleArgsConfig` 与 `cli.role_args`。
- [x] 阶段4: 接入 serial loop、parallel executor、hat capability direct backend。
- [x] 阶段5: 更新 `ralph.yml`,让 coordinator 使用 `-c features.hooks=false`,worker 保持空数组。
- [x] 阶段6: 运行 focused tests、格式化、`git diff --check` 和全量 `cargo test --quiet`。

关键决定:
- 不使用独立 `CODEX_HOME`,因为用户明确认为过于复杂。
- 不把 `features.hooks=false` 放进全局 `cli.args`,避免误伤 worker hats。
- 采用与 `reasoning_effort.coordinator / worker` 类似的 role-aware 配置层。
- 参数叠加顺序保持为 `role_args -> custom_args -> reasoning_effort defaults`,让一次性 custom args 和显式 reasoning override 保持优先权。

验证结果:
- `git diff --check` 已通过。
- `cargo test --quiet` 已通过,全量测试进程 exit code 0。
- focused tests 覆盖 YAML 解析、coordinator/worker argv 差异、parallel executor 分流、autopilot 子配置保留。

状态:
- **本轮实现已完成并验证通过** - 等待最终交付说明。

## [2026-05-18 23:05:00] [Session ID: omx-1779004640353-blcixq] 归档索引: 超限计划快照已归档

归档动作:
- 已将旧计划快照移动到 `archive/branch_contexts/parallel_rec_analysis/task_plan__parallel_rec_analysis_2026-05-18_2257_over1000.md`。
- 已创建归档 manifest: `archive/manifests/ARCHIVE_MANIFEST__parallel_rec_analysis_2026-05-18_2305.md`。
- 已将可复用经验写入 `EXPERIENCE.md#exp-20260518-role-aware-cli-args-for-coordinator-hooks`。

状态:
- **上下文续档已完成** - 当前支线继续使用新的 `task_plan__parallel_rec_analysis.md`。

## [2026-05-19 08:06:00] [Session ID: omx-1779004640353-blcixq] 调查计划: capability.request 未生成新 instance

用户反馈:
- 当前 `ralph run` 输出了 `capability.request`。
- 请求内容是 `capability_id = "workflow:default-parallel"`,想创建 3 个 hat 实例并行分析。
- 但用户没有看到新的 instance 立即出现。
- 需要具体查看正在跑的 `parallel_rec.jsonl`。

现象记录:
- 已观察到用户贴出的 `<event topic="capability.request" ...>`。
- 尚未确认 runtime 是否消费该事件、是否写入 `capability.failed/result`、是否触发 `spawn_instance` 或 instance state 变化。

当前主假设:
- `workflow:default-parallel` 可能只是 workflow/bootstrap selector 或 catalog id,不是当前 runtime capability invocation 支持的可执行 capability,因此 parent 发出 request 后没有实际 materialize 新 hat instances。

最强备选解释:
- capability request 被 runtime 拦截,但失败/结果事件没有显示在 TUI,或者被写入 record-session / `.ralph/events` 后没有刷新到当前 UI。

验证计划:
- [ ] 阶段1: 只读解析 `parallel_rec.jsonl`,统计 topics、instance state、capability request/result/failure、spawn/new-instance 相关记录。
- [ ] 阶段2: 对照 `.ralph/events*` / `.ralph/agents.json` 如有必要确认 durable runtime truth。
- [ ] 阶段3: 查源码中 capability runtime 对 `workflow:*` 与 `hat:*` 的处理边界。
- [ ] 阶段4: 给出已验证结论和最小落地方向,先不改代码。

状态:
- **正在阶段1** - 先解析 record-session,不根据模型输出猜结论。

## [2026-05-19 08:26:00] [Session ID: omx-1779004640353-blcixq] 行动记录: 补源码证据

行动目的:
- 当前已经有 record-session / `.ralph/events.jsonl` 动态证据,但还需要源码侧静态证据确认协议边界。

即将执行:
- 查 `workflow:default-parallel` capability 的执行路径。
- 查 `audience_instances` / `spawn_instance` / dynamic instance materialization 的路由逻辑。
- 对照正在跑的 `parallel_rec.jsonl` 与 capability invocation artifact,说明为什么父级 TUI 没出现 3 个新 instance。

状态:
- **正在阶段3** - 补源码证据,不改代码。


## [2026-05-19 09:11:12] [Session ID: omx-1779004640353-blcixq] 状态更新: parallel_rec 未生成三个 instance 分析完成

阶段状态:
- [x] 阶段1: 只读解析 `parallel_rec.jsonl`,统计 parent record topic 与 stdout tail。
- [x] 阶段2: 对照 `.ralph/events.jsonl`, `.ralph/agents.json` 和 capability invocation artifacts。
- [x] 阶段3: 查源码中 capability runtime, event parser, routing, dynamic spawn 的处理边界。
- [x] 阶段4: 形成结论: 当前是 isolated child run + child static topology + fallback routing 到 `builder#1`,不是父 TUI 漏刷新。

状态:
- **本轮只读分析已完成** - 未改代码,已记录 notes/worklog/later plan/epiphany。

## [2026-05-19 10:11:38] [Session ID: omx-1779004640353-blcixq] 方案阶段: parent-visible 与 parent-observable 双线修复设计

用户目标:
- 父级 TUI 里能真实新增 hat instance,用于人类运行中动态提出的多个视角/角色。
- 即使能力走 isolated child run,也不能完全不可见,需要 parent-observable 的状态投影。
- 不把运行中才知道的 "功能补充/功能完善/review" 写死进静态配置。

已补充核验:
-  当前契约明确是 isolated child/micro-run,并写明 "do not mutate parent topology"。
-  已有真实动态实例创建路径,但只作用于普通 event routing,不是 。
- TUI 现有  只有真实 instances / gates / evidence_paths,没有 child run observable state。
- Output 状态条已经和正文区分离,适合继续承载 child-run summary,但不要把 child run 伪装成 instance。

阶段状态:
- [x] 只读核验 runtime capability 与 spawn 路径。
- [x] 只读核验 TUI state/footer/instances/output-status 可扩展点。
- [ ] 等待用户确认后进入 spec-first 落地。

状态:
- **当前交付方案,不直接改代码** - 下一步如果落地,先补 OpenSpec 或 specs 设计文档。

## [2026-05-19 10:22:13] [Session ID: omx-1779004640353-blcixq] 行动记录: 补 parent-visible / parent-observable spec 与 Ralph plan

行动目的:
- 用户要求先补一个 spec,并生成 Ralph plan。
- 本轮只创建/更新规格与计划文档,不实现 runtime/TUI 代码。

即将执行:
- 读取现有 `specs/` 与 `ralph plan` 相关约定。
- 创建一个聚焦 `parent-visible dynamic spawn` 与 `parent-observable child run` 的 spec。
- 创建一份可供后续实现执行的 Ralph plan / implementation plan。
- 用 `beautiful-mermaid-rs` 验证 spec 中 Mermaid 图。
- 更新 notes/worklog/context,不改产品代码。

阶段状态:
- [ ] 阶段1: 读取现有 spec/plan 格式。
- [ ] 阶段2: 编写 spec 文档。
- [ ] 阶段3: 编写 Ralph implementation plan。
- [ ] 阶段4: 验证 Mermaid 与基础格式。
- [ ] 阶段5: 记录交付结果。

状态:
- **正在阶段1** - 先对齐现有文档格式,再落文档。

## [2026-05-19 11:03:00] [Session ID: omx-1779004640353-blcixq] 状态更新: spec 与 Ralph plan 已生成

阶段状态:
- [x] 阶段1: 读取现有 spec/plan 格式。
- [x] 阶段2: 编写 spec 文档。
- [x] 阶段3: 编写 Ralph implementation plan。
- [x] 阶段4: 验证 Mermaid 与基础格式。
- [x] 阶段5: 记录交付结果。

验证结果:
- `beautiful-mermaid-rs --ascii` 成功验证 spec 中 2 个 Mermaid block。
- `git diff --check` 通过。
- 新增文档:
  - `specs/parent-visible-topology-spawn-observability.spec.md`
  - `docs/plans/2026-05-19-parent-visible-topology-spawn-and-child-run-observability.md`

状态:
- **本轮文档产物已完成** - 下一步若继续,就从 plan 第 1 步开始实现。
## 2026-05-19 10:44:29 [Session ID: omx-1779158263949-kticiv] 状态确认: 用户已确认 topology.spawn_group 的 3 个开放问题

已确认的设计决策:
- `topology.spawn_group` 不要求原子成功,允许部分成功,但失败项必须结构化可见。
- child-run 状态最好也显示在 `ralph agents` 里,但仍然不能伪装成真实 HatInstance。
- 角色标签默认不写入 `agents.json` 作为一等字段;只有当 LLM coordinator 判断某个角色是固定角色,并显式标记时,才允许持久化为 fixed-role metadata。

对后续实现的影响:
- 继续保留“父级可见动态 spawn”和“parent-observable child run”两条线,不要把两者混成同一种实例模型。
- 角色名输入保持运行时动态化,不把“功能补充 / 功能完善 / review”这类临时角色硬编码进配置或静态 schema。

当前状态:
- 设计决策已收敛,可以进入 implementation 阶段。
## [2026-05-19 10:45:45] [Session ID: omx-1779158263949-kticiv] 实现阶段: 执行 plan 第一批任务

执行方式:
- 使用 `executing-plans` skill,按 plan 的第一批任务推进。
- 本批范围限定为 Step 1 到 Step 3: typed topology spawn protocol, Supervisor `topology.spawn_group` handling, fixed-role metadata support。

当前约束:
- 不改写 isolated capability child-run 的拓扑隔离语义。
- 不把临时角色默认写进 `agents.json` 一等字段。
- 每一步都要加 focused tests 并运行对应验证命令。

阶段状态:
- [ ] Step 1: Add typed topology spawn protocol records。
- [ ] Step 2: Implement Supervisor `topology.spawn_group` handling。
- [ ] Step 3: Add fixed-role metadata support。

状态:
- **正在 Step 1** - 先新增协议类型和单测。
## [2026-05-19 10:54:13] [Session ID: omx-1779158263949-kticiv] 状态更新: Step 1 已完成,进入 Step 2

已完成内容:
- 新增 `crates/ralph-core/src/topology_spawn.rs`。
- 在 `crates/ralph-core/src/lib.rs` 导出 topology spawn 协议类型与 topic 常量。
- 为 `TopologySpawnGroupRequest` / `TopologySpawnGroupResult` / `TopologySpawnGroupFailed` 补了单测。

验证结果:
- `cargo test -p ralph-core topology_spawn -- --nocapture` 通过。
- 4 个 topology_spawn 相关单测全部通过。

阶段状态:
- [x] Step 1: Add typed topology spawn protocol records。
- [ ] Step 2: Implement Supervisor `topology.spawn_group` handling。
- [ ] Step 3: Add fixed-role metadata support。

状态:
- **正在 Step 2** - 开始接 Supervisor 的 group spawn 路由和实例创建。
## [2026-05-19 11:23:40] [Session ID: omx-1779158263949-kticiv] 状态更新: Step 2 和 Step 3 已完成

已完成内容:
- Step 2: Supervisor 已能处理 `topology.spawn_group`。
  - 只接受来自 `ralph#*` 的 coordinator 事件。
  - 复用现有动态实例创建路径,生成真实动态 HatInstance。
  - 为每个 spawned member 写入 `runtime.lifecycle kind=spawn` 和 direct `runtime.delivery`。
  - 返回结构化 `topology.spawn.result` / `topology.spawn.failed`。
  - `request_id` 做幂等,重复请求不会再创建第二批实例。
- Step 3: `agents.json` 支持 fixed-role metadata。
  - 临时角色默认不进入一等字段。
  - 只有 `fixed_role=true` 的 member 才写入 `fixed_role_label` / `fixed_role_reason`。
  - `ralph agents` 表格新增 `Fixed Role` 列。

验证结果:
- `cargo test -p ralph-core topology_spawn_group -- --nocapture` 通过,7 个相关测试通过。
- 第一次 `cargo test -p ralph-cli integration_agents -- --nocapture` 只完成编译但过滤后 0 tests,已纠正验证方式。
- `cargo test -p ralph-cli --test integration_agents -- --nocapture` 通过,5 个 integration_agents 测试通过。
- `cargo fmt --all` 已执行。
- `git diff --check` 通过。

阶段状态:
- [x] Step 1: Add typed topology spawn protocol records。
- [x] Step 2: Implement Supervisor `topology.spawn_group` handling。
- [x] Step 3: Add fixed-role metadata support。
- [ ] Step 4: Add parent-observable child-run state, event forwarding, and agents summary。
- [ ] Step 5: Render child-run and spawned-role information in TUI。
- [ ] Step 6: Update coordinator prompt / capability contract guardrails。
- [ ] Step 7: Add recorded evidence validation and run gates。

状态:
- **第一批实现完成** - 按 `executing-plans` skill,这里是一个 review checkpoint。
## [2026-05-19 11:25:06] [Session ID: omx-1779158263949-kticiv] 实现阶段: 进入 Step 4 child-run observability

行动目的:
- 解决 isolated child run 可观测性不足的问题。
- 让 `capability.invoke/result/failed` 和 `topology.*` 控制面事件进入父级 TUI 的 reducer。
- 增加独立 child-run 状态,不混入真实 HatInstance 列表。

即将执行:
- 阅读 `ralph-cli` 的 TUI event forwarding 规则。
- 阅读 `ralph-tui` 的 `ParallelTuiState::apply_event` 与 footer/output/instances 状态结构。
- 增加 child-run 状态模型和 focused tests。

状态:
- **正在 Step 4** - 先只读定位 TUI reducer 和事件 payload 格式。

## [2026-05-19 11:36:00] [Session ID: omx-1779158263949-kticiv] 继续执行: Step 4 child-run observability 收敛

行动目的:
- 继续上轮未完成的 Step 4,把 parent-observable child-run 状态、event forwarding、`ralph agents` summary 收敛到可编译和可验证。
- 严格区分真实 parent-visible HatInstance 与 parent-observable child-run projection,不把 child run 写成假的 instance。

即将执行:
- 先运行 focused test/编译,读取失败输出。
- 修正 `ralph-tui` reducer、CLI event forwarding、`ralph agents` summary 中未完成的接线。
- 添加/修正 focused tests,再执行 `cargo fmt --all` 和 `git diff --check`。

阶段状态:
- [x] Step 1: Add typed topology spawn protocol records。
- [x] Step 2: Implement Supervisor `topology.spawn_group` handling。
- [x] Step 3: Add fixed-role metadata support。
- [ ] Step 4: Add parent-observable child-run state, event forwarding, and agents summary。
- [ ] Step 5: Render child-run and spawned-role information in TUI。
- [ ] Step 6: Update coordinator prompt / capability contract guardrails。
- [ ] Step 7: Add recorded evidence validation and run gates。

状态:
- **正在 Step 4** - 先让已有半成品改动通过 focused tests,再继续 Step 5。

## [2026-05-19 11:54:00] [Session ID: omx-1779158263949-kticiv] 状态更新: Step 4 child-run observability 已完成

已完成内容:
- CLI parallel runner 会转发 `capability.*` 与 `topology.*` 控制面事件到 TUI,即使事件没有 `source` 或 `source_instance`。
- `ParallelTuiState` 已维护独立 child-run projection,消费 `capability.invoke` / `capability.result` / `capability.failed`。
- child-run 不会写入 `instance_order`,不会伪装成真实 HatInstance。
- `.ralph/agents.json` snapshot 已包含 `child_runs`,并且 `ralph agents` 会展示轻量 summary。

验证结果:
- `cargo test -p ralph-tui child_run -- --nocapture` 通过,3 个 child-run reducer tests 通过。
- `cargo test -p ralph-cli parallel_tui_event_forwarding -- --nocapture` 通过,7 个 event forwarding tests 通过。
- `cargo test -p ralph-cli --test integration_agents -- --nocapture` 通过,6 个 agents integration tests 通过。
- `cargo test -p ralph-core capability_runtime -- --nocapture` 通过,4 个 capability runtime tests 通过。

阶段状态:
- [x] Step 1: Add typed topology spawn protocol records。
- [x] Step 2: Implement Supervisor `topology.spawn_group` handling。
- [x] Step 3: Add fixed-role metadata support。
- [x] Step 4: Add parent-observable child-run state, event forwarding, and agents summary。
- [ ] Step 5: Render child-run and spawned-role information in TUI。
- [ ] Step 6: Update coordinator prompt / capability contract guardrails。
- [ ] Step 7: Add recorded evidence validation and run gates。

状态:
- **正在进入 Step 5** - 下一步把 child-run summary 和 spawned role 信息渲染到实际 TUI widget。

## [2026-05-19 12:13:00] [Session ID: omx-1779158263949-kticiv] 状态更新: Step 5 TUI 渲染已完成

已完成内容:
- Footer 已显示 compact child-run summary,例如 `child: 1 running / 0 done / 0 failed`。
- Output status pane 已在 evidence line 中显示 latest child-run identity 和 artifact path。
- Instances pane 已显示 `topology.spawn.result` 提供的临时 role 标签,例如 `role:功能补充`。
- 临时 role 标签只存在于 TUI live state,不写入 `agents.json` 一等字段。
- Output 正文区与底部状态区仍然分离,避免底部 act/status 遮挡正文。

验证结果:
- `cargo test -p ralph-tui footer -- --nocapture` 通过,包含新增 footer child-run summary 断言。
- `cargo test -p ralph-tui instances -- --nocapture` 初次因中文宽字符 buffer 断言失败,已改为宽字符安全分段断言后通过。
- `cargo test -p ralph-tui parallel_output -- --nocapture` 通过,包含 child-run artifact 和 status rows split 断言。

阶段状态:
- [x] Step 1: Add typed topology spawn protocol records。
- [x] Step 2: Implement Supervisor `topology.spawn_group` handling。
- [x] Step 3: Add fixed-role metadata support。
- [x] Step 4: Add parent-observable child-run state, event forwarding, and agents summary。
- [x] Step 5: Render child-run and spawned-role information in TUI。
- [ ] Step 6: Update coordinator prompt / capability contract guardrails。
- [ ] Step 7: Add recorded evidence validation and run gates。

状态:
- **正在进入 Step 6** - 下一步更新 prompt/protocol/capability catalog,降低 coordinator 误选 `workflow:*` 的概率。

## [2026-05-19 12:25:00] [Session ID: omx-1779158263949-kticiv] 状态更新: Step 6 prompt/protocol guardrails 已完成

已完成内容:
- Coordinator 可发布 topic 列表已加入 `topology.spawn_group`。
- Event emission protocol 在可发布 `topology.spawn_group` 时会说明 parent-visible group spawn 的 JSON 契约。
- Runtime capability catalog 明确 `capability.request` 只用于 isolated child/micro-run,不能用于父级 TUI 新增可见实例。
- Coordinator prompt focused test 已断言 `topology.spawn_group` 只进入 ralph prompt,不污染 worker prompt。

验证结果:
- `cargo test -p ralph-core topology_spawn_prompt -- --nocapture` 通过,1 个 protocol prompt test 通过。
- `cargo test -p ralph-core capability_catalog -- --nocapture` 通过,3 个 capability catalog / coordinator injection tests 通过。

阶段状态:
- [x] Step 1: Add typed topology spawn protocol records。
- [x] Step 2: Implement Supervisor `topology.spawn_group` handling。
- [x] Step 3: Add fixed-role metadata support。
- [x] Step 4: Add parent-observable child-run state, event forwarding, and agents summary。
- [x] Step 5: Render child-run and spawned-role information in TUI。
- [x] Step 6: Update coordinator prompt / capability contract guardrails。
- [ ] Step 7: Add recorded evidence validation and run gates。

状态:
- **正在进入 Step 7** - 下一步跑格式、diff、focused gates、smoke 和全量测试。

## [2026-05-19 12:43:00] [Session ID: omx-1779158263949-kticiv] 完成状态: parent-visible spawn 与 child-run observability 已闭环

最终完成内容:
- `topology.spawn_group` 已成为 parent-visible 多实例创建协议,会创建真实动态 HatInstance。
- `capability.request` 保持 isolated child/micro-run 语义,但现在通过 child-run projection 在 TUI 和 `ralph agents` 中可观测。
- TUI 已区分真实 instance row 与 child-run summary,不会用假 instance 表达 isolated child run。
- Coordinator prompt / capability catalog 已明确三条路径:
  - parent-visible group spawn: `topology.spawn_group`。
  - isolated child/micro-run: `capability.request`。
  - parent-visible single instance: `target="<hat_id>" spawn_instance="true"`。

最终验证:
- `cargo fmt --all && git diff --check` 通过。
- Focused gates 全部通过:
  - `cargo test -p ralph-core topology_spawn_group -- --nocapture`
  - `cargo test -p ralph-core capability_runtime -- --nocapture`
  - `cargo test -p ralph-tui child_run -- --nocapture`
  - `cargo test -p ralph-tui footer -- --nocapture`
  - `cargo test -p ralph-tui instances -- --nocapture`
  - `cargo test -p ralph-tui parallel_output -- --nocapture`
  - `cargo test -p ralph-cli parallel_tui_event_forwarding -- --nocapture`
  - `cargo test -p ralph-cli --test integration_agents -- --nocapture`
  - `cargo test -p ralph-core topology_spawn_prompt -- --nocapture`
  - `cargo test -p ralph-core capability_catalog -- --nocapture`
- Replay smoke test `cargo test -p ralph-core smoke_runner` 通过,12 个 smoke_runner tests 通过。
- 全量 `cargo test --quiet` 通过。
- 最终 `git diff --check` 通过。

阶段状态:
- [x] Step 1: Add typed topology spawn protocol records。
- [x] Step 2: Implement Supervisor `topology.spawn_group` handling。
- [x] Step 3: Add fixed-role metadata support。
- [x] Step 4: Add parent-observable child-run state, event forwarding, and agents summary。
- [x] Step 5: Render child-run and spawned-role information in TUI。
- [x] Step 6: Update coordinator prompt / capability contract guardrails。
- [x] Step 7: Add recorded evidence validation and run gates。

状态:
- **实现与验证已完成** - 等待最终交付说明。

## [2026-05-19 12:55:00] [Session ID: omx-1779158263949-kticiv] 继续验证: live dogfood parent-visible 三实例

行动目的:
- 按最终建议继续,用真实 `ralph run --record-session` 做 dogfood。
- 验证用户原始“三个视角/三个实例”提示是否会触发 `topology.spawn_group`,并在父级 runtime 中创建真实动态 HatInstance。
- 本轮优先收集 record-session、`.ralph/events.jsonl`、`.ralph/agents.json` 等 durable evidence;如 TUI 捕获可行,再补 tmux capture。

验证计划:
- [ ] 阶段1: 检查当前 CLI 命令形态和配置,准备隔离 record-session 路径。
- [ ] 阶段2: 运行一次 live `ralph run --record-session`,提示明确要求父级可见创建 3 个实例。
- [ ] 阶段3: 用 `ralph record summary`、events、agents snapshot 分析是否真的有 `topology.spawn_group`、`runtime.lifecycle kind=spawn`、3 个 dynamic instances。
- [ ] 阶段4: 若未成功,按“现象 -> 假设 -> 验证计划 -> 结论”说明失败原因,不把猜测当结论。
- [ ] 阶段5: 记录 dogfood 结果到 notes/worklog/errorfix 或 later plans。

状态:
- **正在阶段1** - 先确认本地命令和配置,避免用错二进制或误读旧 record。

## [2026-05-19 13:04:00] [Session ID: omx-1779158263949-kticiv] 行动记录: 启动 live no-TUI dogfood

行动目的:
- 启动真实 `ralph run --record-session`。
- 使用临时 read-only analyst 配置,避免 dogfood worker 修改当前仓库。
- 先验证 durable runtime evidence,不让 TUI ANSI 干扰记录解析。

即将执行:
- `target/debug/ralph run -c /tmp/ralph-topology-dogfood.yml --no-tui --record-session /tmp/ralph-topology-dogfood-record.jsonl -p <prompt>`。
- 超时上限 300 秒。
- stdout/stderr 写入 `/tmp/ralph-topology-dogfood.stdout` 和 `/tmp/ralph-topology-dogfood.stderr`。

状态:
- **正在阶段2** - 等待 live dogfood 输出。

## [2026-05-19 13:18:00] [Session ID: omx-1779158263949-kticiv] dogfood 发现: topology.spawn.result 后重复派发

现象:
- live dogfood 生成了 `topology.spawn_group`。
- runtime 创建了 `analyst#2`, `analyst#3`, `analyst#4` 三个动态实例,并对三者 direct delivery `analysis.task`。
- `topology.spawn.result` 回到 `ralph#1` 后,coordinator 又额外发出一条 `analysis.task` with `audience_instances="analyst#2,analyst#3,analyst#4"`。
- 这条额外事件最终 fanout 到配置实例 `analyst#1`,造成重复/偏离预期的第四个分析任务。

当前假设:
- coordinator prompt 的 `If you receive any other event` 泛化规则把 `topology.spawn.result` 当成需要继续 delegate 的普通事件。
- 缺少明确规则说明: spawned instances 已经收到 direct delivery,收到 spawn result 后不要重复派发 delivery topic。

备选解释:
- 事件路由层对 `audience_instances` fallback 的行为也会加剧混淆,但这不是重复派发的最早原因;最早原因是 coordinator 第二轮多发了 `analysis.task`。

即将执行:
- 在 coordinator prompt 的 WHAT TO DO 段落补 `topology.spawn.result` / `topology.spawn.failed` 处理规则。
- 增加 prompt focused test,防止回归。
- 跑 `cargo test -p ralph-core capability_catalog topology_spawn_prompt -- --nocapture` 或相关 prompt tests。

状态:
- **正在修复 dogfood 暴露的 prompt gap** - 这是 live evidence 推导出的后续修正。

## [2026-05-20 00:04:53] [Session ID: omx-1779158263949-kticiv] 行动记录: 继续修复 topology.spawn.result 重复派发

行动目的:
- 接续上一轮 live dogfood 暴露的问题: `topology.spawn_group` 已成功创建 3 个动态实例,但 coordinator 收到 `topology.spawn.result` 后又二次发出 `analysis.task`。
- 本轮只做小范围 prompt guardrail 与 focused test,避免扩散到 runtime 路由大改。

现象:
- `.ralph/events.jsonl` 中已有 durable evidence 证明 spawn 成功。
- 重复派发来自 coordinator 后续输出,不是首次 spawn delivery 失败。

当前假设:
- `build_ralph_coordinator_instructions()` 的 `If you receive any other event` 规则过宽,缺少 `topology.spawn.result` 的明确处理方式。

即将执行:
- 阅读 `crates/ralph-core/src/parallel/supervisor.rs` prompt 构造段和已有 routing prompt tests。
- 补充 `topology.spawn.result` / `topology.spawn.failed` 的 coordinator-only 指令。
- 增加 focused test 防止 worker prompt 被污染。
- 运行 focused cargo tests 和 `git diff --check`。

状态:
- **正在修复 prompt gap** - 先查代码路径,再写最小可验证补丁。

## [2026-05-20 00:07:30] [Session ID: omx-1779158263949-kticiv] 状态变更: 已补 prompt guardrail,准备验证

已完成:
- 在 `build_ralph_coordinator_instructions()` 中新增 `topology.spawn.result` 专门处理规则。
- 明确 spawned instances 已经通过 `delivery_topic` 收到 direct delivery。
- 明确禁止 re-emit delivery topic。
- 明确禁止把 `audience_instances` 当 replay 机制。
- 补充 `topology.spawn.failed` 失败处理规则。
- 在 prompt focused test 中断言规则只进入 coordinator prompt,不进入 worker prompt。

即将执行:
- `cargo fmt --all`
- `cargo test -p ralph-core topology_spawn_prompt capability_catalog -- --nocapture`
- `cargo test -p ralph-core topology_spawn_group -- --nocapture`
- `git diff --check`

状态:
- **正在验证** - 先跑 focused tests,根据输出决定是否需要继续修正。

## [2026-05-20 00:08:18] [Session ID: omx-1779158263949-kticiv] 验证错误: cargo test 过滤参数写法错误

现象:
- 命令 `cargo test -p ralph-core topology_spawn_prompt capability_catalog -- --nocapture` 失败。
- Cargo 输出: `error: unexpected argument 'capability_catalog' found`。

原因:
- `cargo test` 只接受一个 TESTNAME filter,不能在同一条命令中并列两个 filter。

修正:
- 分开运行 `topology_spawn_prompt` 和 `capability_catalog` 两组 focused tests。

状态:
- **继续验证** - 该错误属于验证命令用法错误,不是代码编译失败。

## [2026-05-20 00:12:30] [Session ID: omx-1779158263949-kticiv] 状态变更: 静态验证通过,准备二次 live dogfood

已验证:
- exact focused tests 全部通过。
- `git diff --check` 通过。
- 全量 `cargo test --quiet` 通过。

仍需验证:
- 这次改动是 prompt guardrail,需要尽量补一轮动态证据。
- 重点不是再次证明 spawn 能成功,而是观察 `topology.spawn.result` 后是否还会二次发 `analysis.task`。

即将执行:
- 使用 `/tmp/ralph-topology-dogfood.yml` 和 `/tmp/ralph-topology-dogfood-prompt.txt` 准备短时 no-TUI dogfood。
- 如配置 runtime 太长,复制成新的短时配置,避免无意义等待。
- 输出写入新的 `/tmp/ralph-topology-dogfood-guardrail-*` 文件。

状态:
- **正在动态验证** - 用 record-session/events 做证据,不凭肉眼 TUI 判断。

## [2026-05-20 00:15:40] [Session ID: omx-1779158263949-kticiv] 验证错误: zsh 只读变量 status

现象:
- live dogfood 命令运行后,包装脚本报 `zsh:11: read-only variable: status`。
- 原因是 zsh 将 `status` 作为只读特殊变量,不能用于 `status=$?`。

影响:
- `ralph run` 已产生 `/tmp/ralph-topology-dogfood-guardrail-record.jsonl` 和 stdout/stderr。
- 自动执行的 `ralph record summary` 被包装脚本错误中断,需要手工补读。

修正:
- 后续 shell 变量改用 `run_status`。
- 当前先读取已生成 record-session 和 `.ralph/events.jsonl` 作为动态证据。

状态:
- **继续动态验证** - 不忽略包装脚本错误,但保留并解析本轮已产出的 evidence。

## [2026-05-20 00:22:10] [Session ID: omx-1779158263949-kticiv] 验证结论: prompt guardrail 静态与动态证据

静态验证:
- `cargo fmt --all` 已执行。
- exact focused tests 已通过:
  - `cargo test -p ralph-core --lib event_emission_protocol::tests::topology_spawn_prompt_documents_parent_visible_group_spawn_contract -- --exact --nocapture`
  - `cargo test -p ralph-core --lib capability::tests::parent_capability_catalog_renderer_includes_request_contract_and_bounded_metadata -- --exact --nocapture`
  - `cargo test -p ralph-core --lib parallel::supervisor::routing_tests::runtime_capability_catalog_is_injected_only_into_ralph_prompt -- --exact --nocapture`
  - `cargo test -p ralph-core --lib parallel::supervisor::routing_tests::topology_spawn_group_creates_three_dynamic_instances_and_delivers_direct -- --exact --nocapture`
  - 其余 topology spawn typed / idempotent / non-ralph exact tests 也通过。
- `git diff --check` 通过。
- `cargo test --quiet` 通过。

动态 dogfood:
- record-session: `/tmp/ralph-topology-dogfood-guardrail-record.jsonl`。
- `ralph record summary` 显示 `Termination reason: MaxRuntime`。
- Topics 显示 `analysis.task: 3`, `topology.spawn_group: 1`, `topology.spawn.result: 1`。
- 脚本解析 bus.publish 顺序:
  - `task.start`
  - `topology.spawn_group`
  - `analysis.task` -> `analyst#2`
  - `analysis.task` -> `analyst#3`
  - `analysis.task` -> `analyst#4`
  - `topology.spawn.result`
- `topology.spawn.result` 之后 `analysis_task_after_spawn_result=0`。

结论:
- 本轮 prompt guardrail 修复了上一轮 dogfood 中 `topology.spawn.result` 后二次发 `analysis.task` 的行为。
- 本轮 dogfood 仍以 `MaxRuntime` 结束,这是 worker 未稳定产出 `analysis.done` 的独立问题,不等同于 topology spawn 重复派发回归。

状态:
- **prompt guardrail 修复完成并已验证** - 接下来记录 worklog/errorfix/later plans。

## [2026-05-20 00:25:00] [Session ID: omx-1779158263949-kticiv] 完成状态: topology.spawn.result guardrail 修复闭环

阶段完成情况:
- [x] 阶段1: 复核 live dogfood 现象与静态 prompt 缺口。
- [x] 阶段2: 修改 coordinator prompt,新增 `topology.spawn.result` / `topology.spawn.failed` 处理规则。
- [x] 阶段3: 增加 prompt focused test,防止 worker prompt 污染。
- [x] 阶段4: 运行 focused tests、全量测试和 diff check。
- [x] 阶段5: 运行二次 no-TUI live dogfood,验证 spawn result 后没有重复 `analysis.task`。
- [x] 阶段6: 记录 notes/worklog/errorfix/later plans。

最终状态:
- **本轮修复完成** - 重复派发问题已通过静态测试和 record-session 动态证据验证。
- **独立遗留** - dogfood worker 收敛仍会 `MaxRuntime`,已登记到 `LATER_PLANS__parallel_rec_analysis.md`。
