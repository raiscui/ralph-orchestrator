# notes.md

## [2026-05-17 16:55:40] [Session ID: omx-1779004640353-blcixq] 笔记: notes 续档入口

## 来源

### 来源1: archived previous notes

- 归档文件: `archive/default_history/notes_2026-05-17_1655_tui_status_prev.md`
- 原行数: 1166
- 触发条件: 默认六文件中的 `notes.md` 超过 1000 行,按项目规则续档。

## 综合发现

### 当前任务摘要

- 本轮已完成 TUI 与 Codex/CLI 直接输出差异排查。
- 已落地最小 TUI 状态增强:
  - Instances 行显示 `job x/y`。
  - Footer 并行模式显示 selected instance、紧凑 job、render mode、last event。
- 验证通过:
  - `cargo fmt --all -- --check`
  - focused ralph-tui widget tests
  - `cargo test --package ralph-tui`
  - `cargo test`
  - `git diff --check`

### 可复用经验

- TUI 信息缺失类问题要先区分 runtime truth 是否存在,再判断是否只是展示层没有聚合。
- 并行 TUI 状态展示应复用 `ParallelTuiState` / `InstanceViewState` / `TuiState.last_event`,不要新增第二套状态真相源。
- Footer 是窄空间,要用紧凑标签;长 label 应放到 Output title、Instances、raw/audit 面板或详情视图。

### 后续仍未完成

- raw/audit 视图仍未落地。
- stderr visible/hidden 仍需从 runner flag 正式进入 TUI state。
- evidence/status 面板仍未落地。

## [2026-05-17 17:10:00] [Session ID: omx-1779004640353-blcixq] 笔记: Codex 原生状态行与 Ralph 并行 TUI

## 来源

### 来源1: `crates/ralph-cli/src/parallel_runner.rs`

- 要点:
  - 普通并行 backend 使用 `BufReader::lines()` 分别读取 stdout 和 stderr。
  - stderr 会作为 `HatJobOutputChunk` 发送给 Supervisor,但不会进入 event parsing。
  - TUI observer 默认发送 stderr chunk,只有 `--hide-stderr` 才隐藏显示。

### 来源2: `crates/ralph-cli/src/codex_app_server_session.rs`

- 要点:
  - app-server 路径不会直接显示 Codex 原生 TUI 的 status bar。
  - 当前把 prompt transcript、stderr、reasoning summary / agentMessage delta 映射成 Ralph 自己的 stdout/stderr chunk。
  - `codex/event/task_started` 当前用于 steer flush 门槛,没有映射成人类可见的 `Working...` 状态文案。

### 来源3: `crates/ralph-tui/src/state/parallel.rs` 与 `crates/ralph-tui/src/state/parallel/output.rs`

- 要点:
  - 并行 TUI 按 job 保存 raw_lines,再渲染为可见行。
  - stderr 默认灰色弱化,不加 `[stderr]` 前缀。
  - 显示层会过滤控制字符,因此 `\r` 这类 TTY 原地刷新控制符不会成为稳定可读状态行。

## 综合发现

- stderr 的普通文本行: 当前并行 TUI 默认会显示。
- Codex 原生交互 UI 的临时状态条,如 `Working... esc to interrupt`: 当前不会被 Ralph 稳定当作状态字段显示。
- 如果这类状态条以 newline 形式从 stderr/stdout 输出,可能被当普通输出行显示。
- 如果它是 TTY 原地刷新/ANSI 控制序列,当前 TUI 不会稳定保留成“当前动作”状态。

## 验证

- `cargo test --package ralph-cli --bin ralph tests::run_args_show_stderr_defaults_to_true -- --exact`: passed。
- `cargo test --package ralph-tui --lib state::parallel::tests::parallel_output_stderr_markdown_rendering_matches_renderer_output -- --exact`: passed。

## [2026-05-17 18:18:00] [Session ID: omx-1779004640353-blcixq] 笔记: Codex 风格 current activity 落地验证

## 来源

### 来源1: `crates/ralph-core/src/activity.rs`

- 要点:
  - 新增 activity 文本归一化 helper。
  - 只处理已经成为可见文本的状态行,不解析私有 TTY 控制序列。
  - 可以把 `• Working (11s • esc to interrupt)` 归一成 `Working`。
  - 可以识别 `Inspecting current code behavior` 这类 reasoning 状态文案。

### 来源2: `crates/ralph-cli/src/codex_app_server_session.rs`

- 要点:
  - `codex/event/task_started` 映射为 `OutputStream::Activity` 的 `Working`。
  - `item/reasoning/summaryTextDelta` 和 agent message delta 中可识别的状态文本会映射为 activity。
  - activity 只发给 UI/observer,不参与 stdout 正文组装和 event parser。

### 来源3: `crates/ralph-tui/src/state/parallel.rs`

- 要点:
  - `InstanceViewState` 新增 `current_activity` 和 `state_since`。
  - `OutputStream::Activity` 只更新当前状态,不追加到正文 output buffer。
  - 普通 stdout/stderr 中如果出现稳定可见的 `Working...` / `Inspecting...` 行,也会 best-effort 更新 activity。

### 来源4: `crates/ralph-tui/src/widgets/footer.rs` 和 `crates/ralph-tui/src/widgets/instances.rs`

- 要点:
  - Footer 在并行模式下优先显示 `Activity (Ns • Ctrl+C to interrupt)`。
  - Instances 行显示 `a:<activity elapsed>` 的短摘要。
  - Footer 继续显示 selected instance、state、job、render mode 和 last event。

## 综合发现

- 现在并行 TUI 会稳定显示 Codex 风格的“当前正在做什么”。
- 中断提示使用 Ralph TUI 的真实交互键 `Ctrl+C to interrupt`,不是 Codex 原生 `esc to interrupt`。
- Activity 是状态流,不进入正文输出,也不进入事件解析。
- `stderr` 普通行仍默认显示; `Activity` 与 stderr 是否隐藏是两件事。

## 验证

- `cargo test -p ralph-cli`: passed。
- `cargo fmt --all -- --check`: passed。
- `cargo test`: passed。
- `git diff --check`: passed。

## [2026-05-17 19:05:00] [Session ID: omx-1779004640353-blcixq] 笔记: 并行 TUI raw/audit 视图

## 来源

### 来源1: `specs/parallel-tui-raw-audit-view.md`

- 要点:
  - Output 视图三态: Rendered / Plain / Audit。
  - `v` 键循环切换。
  - Audit 复用 `JobViewState.raw_lines`,不新增第二套输出缓存。

### 来源2: `crates/ralph-tui/src/state/parallel.rs`

- 要点:
  - 新增 `ParallelOutputViewMode`。
  - Audit 渲染格式: `[instance:stream:job=n] line`。
  - Activity 在 Rendered/Plain 仍不进入正文,但在 Audit 中可见。

## 验证

- `cargo test --package ralph-tui --lib`: passed。
- `cargo test -p ralph-tui`: passed。
- `cargo test`: passed。
- `cargo fmt --all -- --check`: passed。
- `git diff --check`: passed。

## [2026-05-17 21:46:00] [Session ID: omx-1779004640353-blcixq] 笔记: evidence/status 面板与 Output 底栏收口

## 来源

### 来源1: `crates/ralph-tui/src/widgets/parallel_output.rs`

- 要点:
  - `ParallelOutputStatusPane` 直接挂在 Output 面板底部。
  - 高度够时拆成两行,不够时退化成单行,避免窄屏把信息挤没。

### 来源2: `crates/ralph-cli/src/parallel_runner.rs`

- 要点:
  - CLI 从 `.ralph/current-events`、`evidence-index`、`.ralph/agents.json` 和 `--record-session` 解析展示路径。
  - TUI 只接收已解析好的文本,不在 widget 里再推导第二套真相源。

### 来源3: 验证门禁

- 要点:
  - evidence path、activity bottom line、audit view、path injection 的 focused tests 都已通过。
  - `cargo test --quiet`、`cargo fmt --all -- --check` 和 `git diff --check` 都是干净的。

## 综合发现

### 展示层边界

- `ParallelEvidencePaths` 只负责展示,不参与调度或落盘。
- `act` 和 evidence path 适合靠近 Output,Footer 则保留全局状态和选择摘要。

## [2026-05-20 07:55:00] [Session ID: omx-1779158263949-kticiv] 笔记: continuous-learning 六文件摘要

## 来源

### 默认组

- `task_plan.md`: 最近主线从 runtime evidence、capability failure branching、startup bootstrap、parallel TUI evidence/status 延伸到当前 continuous-learning。
- `notes.md`: 记录 TUI display truth、raw/audit、evidence path、Output status 的 source-of-truth 边界。
- `WORKLOG.md`: 已接近 1000 行,集中记录 runtime evidence、capability、startup bootstrap、parallel TUI 状态增强。
- `LATER_PLANS.md`: 仍保留 TUI stderr visible/hidden、last input preview 等后续项。
- `ERRORFIX.md`: 最近重点是 TUI current activity 和 display/status 可见性。
- `EPIPHANY_LOG.md`: 重要规律是 Ralph 应保持薄调度层,不要把 worker 执行和 coordinator prompt 混在一起。

### 支线组 `display_info_evidence`

- 活跃度判定: 未轮转旧支线。最后记录为 2026-05-19,当前任务没有继续推进证据。
- 关键发现:
  - `.ralph/*`、record-session、runtime chunks 才是 durable evidence。
  - TUI header/footer/instances/output status 是 display surfaces。
  - Output status strip 必须和正文 viewport 分区,否则会遮挡输出。
- 已沉淀: `EXPERIENCE.md` 已有 `exp-20260519-parallel-output-status-strip-viewport`。

### 支线组 `multi_agent_collab_evidence`

- 活跃度判定: 未轮转旧支线。最后记录为 2026-05-18,当前任务没有继续推进证据。
- 关键发现:
  - 当前仓库真实 multi-agent collaboration 实现面是 `parallel hat instances`。
  - 证据链是 event topic -> routing -> instance delivery -> runtime delivery / agents snapshot。
  - focused tests 证明协议和状态机存在,但不等于 live LLM 协作稳定性证明。

### 支线组 `parallel_rec_analysis`

- 活跃度判定: 刚完成的支线,本次触发 continuous-learning 的直接对象。
- 触发条件: `notes__parallel_rec_analysis.md` 已超过 1000 行。
- 关键发现:
  - `capability.request` 是 isolated child/micro-run,不会改 parent topology。
  - 父级可见多实例要走 `topology.spawn_group`。
  - `topology.spawn.result` 是 ack,spawned instances 已经收到 direct delivery,coordinator 不应再次发 `delivery_topic`。
  - `audience_instances` 不能当成 replay 或创建实例机制。
  - child-run 要 parent-observable,但不能伪装成真实 HatInstance。
- 动态证据:
  - `/tmp/ralph-topology-dogfood-guardrail-record.jsonl` 中 `analysis.task` 总数为 3。
  - `topology.spawn.result` 之后 `analysis_task_after_spawn_result=0`。

## 综合发现

### 可复用经验候选

1. Parent-visible dynamic spawn 和 capability child-run 必须分协议、分 UI 真相源。
2. Topology mutation 的成功 result 事件也必须有后续行为定义,否则 coordinator 会把 ack 当普通 orphan event 再派发。
3. Display-only status strip 一旦进入 Output frame,所有滚动/选择/复制/测试 harness 都要复用同一 content viewport helper。
4. Multi-agent collaboration 的验证要明确拆分为机械协议/状态机验证和 live LLM 语义协作验证。

### 最适合沉淀位置

- `EXPERIENCE.md`: 增补 topology spawn result ack guardrail 和 multi-agent collaboration evidence 分层。
- `specs/parent-visible-topology-spawn-observability.spec.md`: 增补 `topology.spawn.result` 不触发 redelivery 的 requirement。
- `docs/plans/2026-05-19-parent-visible-topology-spawn-and-child-run-observability.md`: 增补 Step 6/Step 7 dogfood 后修正规则。
- `docs/runbook/runtime-capabilities.md`: 补充 capability isolation 与 parent-visible topology spawn 的边界。
- `archive/branch_contexts/*`: 归档已总结的旧支线上下文。

## [2026-05-20 16:34:00] [Session ID: omx-1779158263949-kticiv] 笔记: dogfood MaxRuntime 第一轮证据

## 来源

### 来源1: `/tmp/ralph-topology-dogfood-guardrail-record.jsonl`

- `_meta.termination`: `reason=MaxRuntime`, `elapsed_secs≈120`, `iterations=2`。
- `bus.publish` 只有 6 条:
  - `task.start`
  - `topology.spawn_group`
  - `analysis.task` -> `analyst#2`
  - `analysis.task` -> `analyst#3`
  - `analysis.task` -> `analyst#4`
  - `topology.spawn.result`
- 没有 `analysis.done`。

### 来源2: `/tmp/ralph-topology-dogfood-guardrail.stdout`

- final states 显示:
  - `analyst#2: done`
  - `analyst#3: failed`
  - `analyst#4: done`
  - `ralph#1: done`
- 但 stdout 内大量 event tag 是 prompt/examples/代码内容,不等于 bus.publish。

## 当前结论

- 已验证现象: `MaxRuntime` 不是因为 `analysis.done` 已发布但 runtime 没路由,而是 record-session 中根本没有 `analysis.done` bus.publish。
- 当前主假设: worker 没有按 prompt 输出可解析的 `<event topic="analysis.done">...</event>`。
- 最强备选解释: worker 的最终 event 被写入了 stdout 但 parser 没识别;下一步需要查 ux stream 原始末尾和 parsed bus.publish 差异。

## [2026-05-20 16:38:00] [Session ID: omx-1779158263949-kticiv] 笔记: dogfood worker 过度探索证据

## 来源

### 来源1: record-session `ux.terminal.write`

- `analyst#2`: 约 6586 条 stderr terminal writes,执行 15 条 shell 读取/搜索命令。
- `analyst#3`: 约 2689 条 stderr terminal writes,执行 15 条 shell 读取/搜索命令。
- `analyst#4`: 约 3086 条 stderr terminal writes,执行 16 条 shell 读取/搜索命令。
- 三者都没有 stdout final event。

### 来源2: worker command timeline

- `analyst#2` 读了 `README.md`, `docs/api/orchestrator.md`, `EXPERIENCE.md`, `Cargo.toml`,大量 `crates/**/*.rs`, `docs/runbook/runtime-capabilities.md`, `crates/ralph-core/src/evidence_index.rs` 等。
- `analyst#3` 读了 `README.md`, `Cargo.toml`, `EXPERIENCE.md`,多份 `openspec/specs/*/spec.md`。
- `analyst#4` 读了 topology/capability/TUI 相关大量代码与 spec。

## 综合发现

### 现象

- worker 明确知道要发布 `analysis.done`,但一直在做 repo-grounded research,没有在 120 秒内结束。
- record-session 没有任何 `analysis.done` bus.publish。

### 当前主假设

- 临时 analyst 任务太开放,且 prompt 继承全局 agent 习惯,导致 worker 选择了大范围读仓库而不是 bounded evidence scan。

### 最强备选解释

- hooks 输出本身增加噪音和 token/IO 成本,但不是根本原因;真正放大的是 worker 可自由使用工具且缺少硬性 read budget / event-only final contract。

## [2026-05-20 17:02:00] [Session ID: omx-1779158263949-kticiv] 笔记: default_publishes 后 bounded dogfood 新证据

## 来源

### 来源1: `/tmp/ralph-topology-dogfood-bounded-after-default-record.jsonl`

- `bus.publish` 已出现 3 条 `analysis.done`:
  - `analyst#2` -> `analysis.done`
  - `analyst#4` -> `analysis.done`
  - `analyst#3` -> `analysis.done`
- 终止仍是 `reason=MaxRuntime`, `elapsed_secs≈90`, `iterations=3`。
- `ralph#1` 在收到第一条 `analysis.done` 后发布了 `reply.human.message`,说明它把“部分结果”当成了可汇报中间态,但还没有完成整个 workflow。
- 最后 `ralph#1:job=4` 被 cancel,说明剩余问题已经从 worker 无结果转为 coordinator completion 来不及收敛。

## 综合发现

### 现象

- parent-visible 三个 analyst 实例真实创建并全部产出 `analysis.done`。
- record-session 中已经有 worker result durable evidence。
- 90 秒 runtime 预算下,coordinator 在全部结果到齐后没有及时输出 `LOOP_COMPLETE`。

### 当前假设

- 当前主假设: worker 结果链路已通,剩余 `MaxRuntime` 是 live Codex / coordinator completion 的时间预算与收敛 prompt 问题。
- 最强备选解释: coordinator 收到多条 completion candidate 时缺少明确的 group aggregation state,导致它可能按“单条 result”逐步处理,而不是在全部 spawned instance 都完成后一次性收敛。

### 下一步验证

- 只把临时配置 `max_runtime_seconds` 从 90 提到 180,不改 worker hooks 和任务输入。
- 如果 180 能 completion,说明 90 秒预算不足是主要因素。
- 如果 180 仍不能 completion,再考虑补 coordinator aggregation guardrail 或 runtime-level completion summary。

## [2026-05-20 17:14:00] [Session ID: omx-1779158263949-kticiv] 笔记: dogfood MaxRuntime 最终结论

## 来源

### 来源1: `/tmp/ralph-topology-dogfood-bounded-180-record.jsonl`

- `topology.spawn_group` 成功创建 3 个 parent-visible analyst instances。
- `analysis.task` 分别 direct delivery 到 `analyst#2`, `analyst#3`, `analyst#4`。
- 3 个 worker 全部发布 `analysis.done`。
- `ralph#1` 在收到第 3 个结果后输出 `LOOP_COMPLETE`。
- `_meta.termination.reason=CompletionPromise`, `elapsed_secs≈85.8`, `iterations=5`。

### 来源2: focused tests / gates

- `parallel_default_publishes_injects_when_worker_finishes_without_event`: passed。
- `cargo test -p ralph-core smoke_runner`: passed。
- `cargo test`: passed。

## 综合发现

### 已验证结论

- 有 event 但不跑实例的问题已经不在本轮复现: parent-visible `topology.spawn_group` 能真实创建并投递到 3 个 `analyst` 实例。
- 原始 `MaxRuntime` 不是单点 parser 故障,而是 worker 任务太开放 + runtime 预算不足 + parallel 缺少 `default_publishes` fallback 的组合问题。
- 180 秒 live dogfood 证明当前链路可以自然收敛到 `CompletionPromise`。

### 仍需记住的边界

- `stdout` 里出现 `<event ...>` 不等于 `bus.publish`;判断 runtime 语义必须看 record-session 的 `bus.publish` 或 `.ralph/events.jsonl`。
- 90/120 秒对 3 个 high-reasoning live Codex worker 做 repo-grounded 分析不稳定。
- 如果要把这个 dogfood 固化为 CI/E2E,应该用 mock/replay 或更硬的 bounded worker contract,不要依赖 live backend 时间。

## [2026-05-20 19:01:00] [Session ID: omx-1779158263949-kticiv] 笔记: 3-worker live dogfood 复跑结果整理

## 来源

### 来源1: record-session summary

- 文件: `/tmp/ralph-topology-dogfood-bounded-180-rerun-20260520-185717.jsonl`
- 命令: `./target/debug/ralph record summary /tmp/ralph-topology-dogfood-bounded-180-rerun-20260520-185717.jsonl`
- 要点:
  - `cwd=/Users/cuiluming/local_doc/l_dev/my/rust/ralph-orchestrator`
  - `argv_joined=target/debug/ralph run --no-tui --hide-stderr --color never -c /tmp/ralph-topology-dogfood-bounded-180.yml --record-session /tmp/ralph-topology-dogfood-bounded-180-rerun-20260520-185717.jsonl -P /tmp/ralph-topology-dogfood-bounded-prompt.txt`
  - `version=2.2.2`
  - `Termination reason=CompletionPromise`
  - Topics: `analysis.done=3`, `analysis.task=3`, `reply.human.message=2`, `task.start=1`, `topology.spawn.result=1`, `topology.spawn_group=1`

### 来源2: structured record extraction

- 关键 bus.publish:
  - line 307: `topology.spawn_group` from `ralph#1`
  - line 308: `analysis.task` to `analyst#2`
  - line 309: `analysis.task` to `analyst#3`
  - line 310: `analysis.task` to `analyst#4`
  - line 311: `topology.spawn.result` to `ralph#1`
  - line 3433: `analysis.done` from `analyst#4`
  - line 3452: `analysis.done` from `analyst#3`
  - line 3460: `analysis.done` from `analyst#2`
  - line 3490: `_meta.termination`, `elapsed_secs=85.605838208`, `iterations=4`, `reason=CompletionPromise`
- 结论:
  - 3 个 parent-visible analyst 实例都收到了直投任务。
  - `topology.spawn.result` 之后没有出现第二轮 `analysis.task`,这支持 acknowledgement guardrail 生效。
  - 本轮以 `CompletionPromise` 收敛,不是 MaxRuntime。

### 来源3: `.ralph/agents.json`

- 当前 snapshot 包含 5 个实例:
  - `analyst#1`: config-derived
  - `analyst#2`: dynamic, last_input topic=`analysis.task`,临时角色在 input preview 中体现
  - `analyst#3`: dynamic, last_input topic=`analysis.task`,临时角色在 input preview 中体现
  - `analyst#4`: dynamic, fixed_role_label=`review`, fixed_role_reason=`topology.spawn_group member marked fixed_role=true`
  - `ralph#1`: config-derived
- 结论:
  - 临时角色没有作为一等字段持久化,符合“临时角色不用;固定角色可写入”的口径。
  - `review` 因 `fixed_role=true` 作为固定角色元数据进入 snapshot。

## 三个 worker 输出摘要

### analyst#2 / 功能补充

- 现象:
  - 仓库已有 runtime capability 与 `topology.spawn_group` 两条路径。
  - 代码注释区分 child/micro-run 不改父拓扑,`spawn_group` 才创建父级可见实例。
  - parent-visible spawn 与 child-run observability 的计划 checklist 已全勾。
  - Claude adapter gap 文档显示 spec 已描述 stream-json,但当前实现仍偏 PTY raw output + XML event parser。
- 候选方向:
  - 补“用户可见运行图 / 实例生命周期查询”。
  - 为新证据面补 replay fixture、doc 示例和 `ralph agents` / TUI 验收用例。
  - 以 opt-in 方式补 adapter capability negotiation 和 Claude stream-json NDJSON parser。
- 风险:
  - capability child-run 与 topology spawn 展示混在一起,会让用户误判父拓扑是否真实变化。
  - stream-json 直接替换现有 PTY/XML 可能破坏交互模式、TUI raw output 和现有 fixture。
- 验证方式:
  - 用 record-session 同时触发 `capability.request` 与 `topology.spawn_group`,再用 `ralph record summary` 和 agents snapshot 对账。
  - stream-json 先加录制 fixture 和 parser 单测,再跑 smoke_runner 与 CLI integration tests。
- 评价:
  - 有用。它给出了一个新增功能方向,即“运行图 / 生命周期查询 / adapter capability negotiation”。
  - 但 stream-json 属于较大二期,不应混入当前 parent-visible spawn 收尾。

### analyst#3 / 功能完善

- 现象:
  - AGENTS.md 已把 Backpressure、record-session、diagnostics、TUI validation、E2E tiers 写成工作流,但入口分散。
  - EXPERIENCE.md 显示 runtime graph、reply.hat.message、capability invocation、parent catalog、failure_class 已有边界。
  - capability runtime 只让 `ParallelSupervisor` 识别 `ralph#1` 的 `capability.request`,并记录 evidence-index、parent_topology_unchanged、failure_class。
- 候选方向:
  - 把 record summary、diagnostics jq、tui-validate、E2E tier 选择收束成统一 evidence command/guide。
  - 围绕 evidence index 做 inspect UX 和缺失标记。
  - 优先打磨 failure_class、inspect 输出和 parent 可见 catalog,而不是新增通用 retry/planner。
- 风险:
  - 把功能完善理解成给 orchestrator 继续加调度智能,会违背 thin coordination layer 和 Backpressure Over Prescription。
  - 新增 prompt/hat/skill 不更新 manifest 和索引,会产生治理漂移。
- 验证方式:
  - 对功能打磨改动先用 replay smoke、record-session summary、evidence-index inspect 证明语义存在。
  - 再用 TUI/plain 视图验证显示契约。
  - capability 相关改动覆盖 ralph-core 和 ralph-cli integration tests。
- 评价:
  - 很有用。它更符合 Ralph tenets: 不堆复杂调度,而是加强 evidence inspect 和显示契约。
  - 适合转成近期任务。

### analyst#4 / review

- 现象:
  - 设计已经明确拆分三条路径: workflow capability 是 isolated child run 且 parent_topology_unchanged; `spawn_instance=true` 是单事件动态实例; `topology.spawn_group` 是父级真实创建一组 HatInstance。
  - child-run observability 设计为可见但不混入真实实例列表。
  - `topology.spawn.result` 是 acknowledgement,spawned instances 已通过 delivery_topic 收到直投。
- 风险:
  - coordinator 或提示面混用 `capability.request` 与 `topology.spawn_group`,会导致用户要求父级可见实例时只得到隔离 child run。
  - `topology.spawn.result` 如果被误当成 delegation 请求,会造成重复投递或假成功。
- 验证方式:
  - record-session 检查 `topology.spawn_group`、runtime lifecycle、runtime delivery、`.ralph/agents.json`。
  - 断言 `topology.spawn.result` 之后没有再次 bus.publish 原 delivery_topic。
- 评价:
  - 有用。它没有提出很多新功能,但很好地确认了边界和风险,适合作为 guardrail / regression test 口径。

## 综合判断

### 有价值的部分

1. “用户可见运行图 / 实例生命周期查询”有实际价值,因为它直接解决用户关心的“看没看到,怎么确认真跑了”。
2. “统一 evidence command/guide / inspect UX / 缺失标记”有实际价值,并且比继续加调度智能更符合项目理念。
3. “acknowledgement guardrail”价值很高,本轮 record 已证明 `topology.spawn.result` 后没有重复 `analysis.task`。
4. “固定角色才写入 agents snapshot,临时角色只留在任务输入”这个行为被本轮 `.ralph/agents.json` 证实,符合已决策口径。

### 泛化或暂缓的部分

1. Claude stream-json adapter capability negotiation 有价值,但它比当前 parent-visible spawn 主线大,建议作为单独 OpenSpec / adapter 任务。
2. “补更多 doc 示例”有用,但不能替代回放 fixture、record summary 和 agents snapshot 断言。
3. 三个 worker 都没有运行测试,它们的输出只能作为 repo-grounded 只读建议,不能当作已验证实现结论。

### 可转成后续任务

1. 给 `ralph record summary` 或新 inspect 命令补 topology/capability 对照摘要:
   - 展示真实 parent-visible instances。
   - 展示 child-run projection。
   - 标明 parent_topology_unchanged / dynamic / fixed_role。
2. 补一个 replay fixture 或 integration test:
   - 同一 run 里触发 `topology.spawn_group`。
   - 断言 `.ralph/agents.json` 出现动态实例。
   - 断言 `topology.spawn.result` 后没有重复 delivery topic。
3. TUI/plain 显示验收:
   - 父级实例列表显示 dynamic analyst 实例。
   - footer/status 或实例栏显示 child-run 数量/状态。
   - output frame 给 act 状态预留底部空间,避免遮挡输出。
4. Claude stream-json adapter capability negotiation 单独立项。

## [2026-05-21 07:25:47] [Session ID: omx-1779158263949-kticiv] 笔记: unified evidence inspect 实装验证

## 来源

### 来源1: 代码变更

- `crates/ralph-cli/src/record_cli.rs`
  - `ralph record summary` 新增 `--agents-file FILE`。
  - summary 输出 `Topics` 后新增 `Evidence Inspect`。
  - 自动候选 agents snapshot 路径: 显式 `--agents-file`、`workspace_root/.ralph/agents.json`、`cwd/.ralph/agents.json`、向上查找 `.ralph/agents.json`、record 文件同目录兜底。
- `crates/ralph-cli/src/record_session.rs`
  - 新增 `EvidenceInspectAggregate`。
  - 从 record-session bus events 提取 topology spawn、capability request/result/failed、result-like topics。
  - 新增 `render_evidence_inspect`。
- `crates/ralph-cli/tests/integration_record_session.rs`
  - 修复 `record_watch_auto_locates_latest_pointer_and_streams_lines` 的强杀竞态,改用 `--until-event` 自然退出。
- `specs/unified-evidence-inspect.spec.md`
  - 记录输出契约和验收方式。

### 来源2: 真实 dogfood record 验证

- 命令:
  - `./target/debug/ralph record summary /tmp/ralph-topology-dogfood-bounded-180-rerun-20260520-185717.jsonl --agents-file .ralph/agents.json`
- Evidence Inspect 关键输出:
  - `reason: CompletionPromise`
  - `topology.spawn_group: 1`
  - `topology.spawn.result: 1`
  - `parent_topology_unchanged=false`
  - `spawned=[analyst#2:功能补充, analyst#3:功能完善, analyst#4:review,fixed]`
  - `instances: 5`
  - `child_runs: 0`
  - `analysis.done: 3 source_instances=analyst#2,analyst#3,analyst#4`

## 综合结论

- 统一 Evidence Inspect 已经能回答用户最关心的问题:
  - 是否真实 parent-visible: 通过 `Topology` 和 `Agents Snapshot` 证明。
  - 是否只是 child-run: 通过 `Child Runs` 和 `Capability Events` 证明。
  - 结果有没有回来: 通过 `Result Topics` 证明。
  - run 是否收敛: 通过 `Termination` 证明。
- 当前没有新增 runtime 语义,只改良现有 `record summary` 证据入口。

## [2026-05-21 07:38:00] [Session ID: omx-1779158263949-kticiv] 笔记: TUI/plain 显示验收收敛

## 现象
- `Evidence Inspect` 已能离线证明 topology / agents / child-run / result / termination。
- TUI 层已有 footer / instances / output status strip / bottom reserved rows 测试。
- plain/no-tui 层缺少运行中可读的 topology/capability 控制面摘要,用户仍需要看 XML event 或另跑 record summary。

## 假设
- 候选主假设: no-tui 分支的 `event_observer` 只在有 `record-session` 时存在,且只写 recorder,所以 plain 终端不会显示 control-plane 事件摘要。
- 最强备选解释: TUI/plain 的显示问题可能在 TUI state/widgets,而不是 CLI observer。已有 TUI focused tests 证明 TUI 关键状态行、实例 role label 和 child-run artifact 已被覆盖,所以本轮优先补 plain/no-tui。

## 验证计划
- 给 `parallel_cli_event_summary` 补 focused tests,覆盖 topology/capability result/failed/request 和 noise topic。
- 把 no-tui event observer 改为始终存在,quiet 模式不输出,但有 recorder 时仍写 record-session。
- 重跑 TUI focused tests,避免 output status / footer / instances 回归。
- 跑 `cargo fmt --all -- --check`、`git diff --check`、`cargo test -p ralph-core smoke_runner`、`cargo test`。

## 已验证结论
- plain/no-tui 现在会输出 `[supervisor:event] ...` 控制面摘要。
- quiet 模式不会输出该摘要。
- TUI 的 child-run footer、instances role label、output status artifact、底部预留行测试均通过。
- 全量 `cargo test` 已通过。

## 过程提醒
- 一开始用短测试名执行 TUI tests 时返回 0 tests,不能作为证据。
- 已通过 `cargo test -p ralph-tui -- --list | rg ...` 找到完整路径后用 `--exact` 重跑并命中真实测试。

## [2026-05-21 07:43:00] [Session ID: omx-1779158263949-kticiv] 笔记: warning gate 修正

## 现象
- 优化 no-tui observer 后,`maybe_write_parallel_cli_event_summary` 只被 tests 调用。
- `cargo test` 输出 `dead_code` warning。

## 处理
- 没有忽略 warning。
- 将该 helper 标记为 `#[cfg(test)]`。
- 使用 `RUSTFLAGS="-Dwarnings" cargo test --quiet` 做最终验证,保证 warning 不会被隐藏。

## 结论
- warning 已清除。
- 全量测试在 deny warnings 下通过。

## [2026-05-21 08:10:00] [Session ID: omx-1779158263949-kticiv] 笔记: parent-visible spawn replay/integration guardrail

## 现象
- core 已有 `topology_spawn_group_creates_three_dynamic_instances_and_delivers_direct` 与 idempotent focused tests。
- 缺口在 CLI 层: 没有一个真实 `ralph run --no-tui --record-session` 同时证明 stdout、`.ralph/events.jsonl`、`.ralph/agents.json` 和 `record summary --agents-file` 的跨层证据链。

## 假设
- 主假设: 增加 CLI integration test 是这条线最小且最有价值的 guardrail,因为它能防止 runtime 真跑时又出现“有 event 但父级没有真实实例”或 “spawn result 后重复派发原任务”。
- 备选解释: 只补 core routing test 也能覆盖大部分行为。但 core tests 已存在,继续只补 core 会漏掉 binary/config/custom backend/record-session/agents sidecar 这几个真实运行边界。

## 已实现测试
- 新增 `crates/ralph-cli/tests/integration_topology_spawn.rs`。
- 测试脚本模拟:
  - `ralph#1` 第一轮输出 `topology.spawn_group`。
  - `builder#2/#3/#4` 分别收到 `build.task` 并输出 `analysis.done`。
  - `ralph#1` 第二轮等待后输出 `LOOP_COMPLETE`。
- 断言:
  - plain stdout 包含 `[supervisor:event] topology.spawn.result` 和 `parent_topology_unchanged=false`。
  - `.ralph/events.jsonl` 中只有 3 条 `build.task`,且都出现在 `topology.spawn.result` 之前。
  - runtime delivery recipients 正好是 `builder#2/#3/#4`,没有 `builder#1`。
  - runtime lifecycle 有 3 条 dynamic Spawn。
  - `.ralph/agents.json` 有 3 个 dynamic builder instances。
  - 临时角色不持久化为 fixed role,`review` 因 `fixed_role=true` 被持久化。
  - `analysis.done` 有 3 条。
  - `record summary --agents-file` 能回放 Evidence Inspect。

## 调试记录
- 初次失败是 `analysis.done` source 顺序不稳定。并发 worker 完成顺序不应作为断言,已改为排序后比较。
- 初次完整验证中 core focused tests 用短名加 `--exact` 命中 0 tests。已用 `cargo test -p ralph-core -- --list | rg ...` 找到完整路径并重跑。
- 初次格式检查失败在新测试文件格式,已运行 `cargo fmt --all` 修正。

## 已验证结论
- parent-visible spawn 的 CLI integration guardrail 已落地并通过。
- 这条测试现在能同时证明 parent-visible materialization、redelivery absence、agents sidecar、record replay summary 和 termination。

## [2026-05-21 19:03:10] [Session ID: omx-1779158263949-kticiv] 笔记: ralplan task-derived dynamic hat identity / role contract

## 来源

### Planner draft
- 文件: .omx/drafts/task-derived-dynamic-hat-identity-role-contract-draft.md
- 初始方向: 在 topology.spawn_group member 上增加 role contract,并串联 prompt / agents / record summary evidence。

### Architect / Critic 评审
- Architect 首轮: ITERATE。
  - 关键反馈: raw role_contract 不能直接成为权限真相源,需要 canonical EffectiveRoleContract。
  - 关键反馈: delivery_topic 与 output allowed topics 必须拆开。
  - 关键反馈: prompt isolation 要从 API/类型结构保证,不能只靠字符串排除。
- Critic 首轮: ITERATE。
  - 关键反馈: Option B 需要公平表达为 runtime-only canonical contract。
  - 关键反馈: 验收和验证必须补 negative tests 与 hash/source id evidence alignment。
- Architect rev2: APPROVE,但要求 objective 冲突策略钉死。
- Critic rev2: ITERATE。
  - 阻塞点: objective canonicalization 仍含糊。
- Architect rev3: APPROVE。
- Critic rev3: APPROVE。

## 最终共识

- 采用 raw spawn input + runtime canonical EffectiveRoleContract。
- raw spawn payload 只是 intent/hint。
- EffectiveRoleContract 是 downstream 唯一 contract truth source。
- EffectiveRoleContract.objective 永远取 member.task。
- raw role_contract.objective 只进入 structured warning/evidence,不进入 prompt / agents snapshot / record summary 的 canonical objective。
- delivery_topic 是 input event topic。
- allowed_result_topics / allowed_topics 是 output publish allowlist,不能混入 delivery_topic。
- agents snapshot 只写 summary/hash/source id/schema/persistence,完整证据留在 record-session。

## 最终产物

- .omx/plans/task-derived-dynamic-hat-identity-role-contract.md

## [2026-05-21 20:31:20] [Session ID: omx-1779158263949-kticiv] 笔记: live dogfood 接管中的新增现象

## 现象
- 正在运行的 task-derived role contract live dogfood 已产生 `builder#2/#3/#4` 动态实例输出。
- stdout/stderr 证据显示 worker prompt 中已经出现 `### ROLE CONTRACT`。
- live run 首轮曾出现 `topology.spawn.failed`,错误是 `instances[0]: field input must be a string when present`。
- 从输出看,LLM 首轮把 `role_contract` 错放进了 `input` object,后续 retry 才改成正确结构并成功 spawn。
- worker 虽然被要求不改代码,但仍写入 `ralph/log/builder#*/...` 与 `.agent/memories.md` 这类运行记录/记忆文件。

## 候选假设
- 主假设: coordinator prompt/event protocol 中对 `topology.spawn_group.instances[].role_contract` 的示例不足,没有明确它是 `instances[]` item 的兄弟字段,也没有强调 `input` 只能是 string。
- 备选解释: 即使 prompt 示例足够,模型也可能因为把 contract 视为 worker input 而嵌套错误;这需要通过更强 schema wording 和回归测试降低概率。

## 验证计划
- 等 live dogfood 完成后读取 record summary,确认 topology failure/success、agents snapshot、result topics、termination。
- 静态查找 coordinator event protocol prompt 里 `topology.spawn_group` 的 schema 描述。
- 补测试断言 prompt 明确包含 `role_contract` sibling field 和 `input must be string` / `do not put role_contract inside input` 语义。

## 当前边界
- 还没有把 prompt guidance 问题表述为最终根因。需要同时拿到静态 prompt 证据和 live dogfood 动态失败证据后才能定性。

## [2026-05-21 20:38:00] [Session ID: omx-1779158263949-kticiv] 笔记: task-derived role contract live dogfood 结果

## 验证命令

```bash
SESSION="/tmp/ralph-task-derived-role-contract-dogfood-20260521-202623.jsonl"
/opt/homebrew/bin/timeout 420 ./target/debug/ralph run -c ralph.yml --no-tui --record-session "$SESSION" -p '创建 3 个 task-derived dynamic hat instances: 功能补充, 功能完善, review。要求每个实例输出 analysis.done,并在 role_contract 中给出 objective/output boundary。不要改代码,只基于当前项目做 repo-grounded 演进分析,最后由 ralph#1 汇总结果。'
./target/debug/ralph record summary "$SESSION" --agents-file .ralph/agents.json
```

## 动态证据摘要

- `RUN_STATUS=124`。
- `Termination.reason=Interrupted`。
- `Termination.elapsed_secs=419.970`。
- `topology.spawn.failed=1`:
  - `request_id=spawn-task-derived-analysis-3`
  - `error=instances[0]: field input must be a string when present`
- `topology.spawn.result=1`:
  - `request_id=spawn-task-derived-analysis-3-retry`
  - `parent_topology_unchanged=false`
  - `spawned=[builder#2:功能补充, builder#3:功能完善, builder#4:review]`
  - 三个 spawned summary 都带 `identity_source=task-derived`, `persistence=fixed`, `contract_schema_version=1`, `role_contract_hash`, `source_spawn_request_id`。
- `Result Topics`:
  - `build.done: 1 source_instances=builder#4`
  - `reply.human.message: 1 source_instances=ralph#1`
  - `topology.spawn.failed: 1 source_instances=ralph#1`
  - `topology.spawn.result: 1 source_instances=ralph#1`

## 现象 -> 假设 -> 结论

### 现象1: 首轮 schema misnesting
- LLM 首轮输出把 `role_contract` 放在 `instances[].input.role_contract` 里。
- parser 动态拒绝,因为 `input` 必须是 string。

候选假设:
- coordinator event protocol prompt 对 `role_contract` 作为 `instances[]` sibling field 的示例不够清楚。

已验证结论边界:
- 动态证据已经确认错误路径发生。
- 仍需读取静态 prompt source 并补测试,才能把 prompt guidance 缺口作为已验证修复对象。

### 现象2: topic contract 保护生效
- 用户 prompt 要求 `analysis.done`,但当前 target `builder` publishes 不含 `analysis.done`。
- worker 最终改用 `build.done` payload 承载 analysis.done 语义。

结论:
- output allowlist 没有被用户 prompt 越权绕过。
- 这符合 runtime canonical contract 设计,但也说明 dogfood prompt 与 `ralph.yml` target publishes 不一致。

### 现象3: live dogfood 未自然收敛
- 420 秒 timeout 后 termination 是 Interrupted。
- 只有 builder#4 的 `build.done` 进入 result topics。
- 终端中可见 builder#2 输出了 `<event topic="build.done">`,但它出现在 stderr 标记流中,没有进入 result topics。
- builder#3 最终 failed。

候选假设:
- live worker 真实 repo-grounded 分析太重,加上 stderr/stdout/工具 hook 噪声,导致部分 result 未进入解析路径或超时。
- 备选解释: role contract prompt 仍不足以约束 worker 不写运行文件、不跑额外流程。

本轮边界:
- 这不是当前 role contract 主链路的阻塞验收,因为 focused/integration gates 已覆盖 runtime 主链路。
- 但它是后续 live dogfood 稳定性和 artifact 污染治理的明确后续问题。

## [2026-05-21 20:49:30] [Session ID: omx-1779158263949-kticiv] 笔记: ai-slop-cleaner changed-files-only pass

## Scope
- 本轮 task-derived role contract 相关文件。
- 重点关注本轮新增的 prompt guidance、role contract canonicalization、agents/record/TUI display、tests。

## Behavior Lock
- deslop 前已经通过:
  - topology spawn focused routing tests 9 passed。
  - dynamic worker prompt focused test passed。
  - CLI integration_topology_spawn passed。
  - smoke_runner passed。
  - `RUSTFLAGS="-Dwarnings" cargo test --quiet` passed。

## Cleanup Plan
- Pass 1: 查找本轮新增的长字符串/重复/死代码/掩盖式 fallback。
- Pass 2: 只做行为不变、可读性提升的小清理。
- Pass 3: 复跑 tests。

## Fallback Findings
- `rg` 扫描命中若干 `fallback`/`TODO`。
- 分类:
  - `parallel/supervisor/routing.rs` 的 wildcard fallback、primary/secondary fallback、default_publishes fallback 是既有运行时语义或已有测试覆盖的兼容/恢复路径,不是本轮新增 masking fallback。
  - `parallel/supervisor.rs` 的 TODO 是既有自然结束判断后续项,不是本轮新增。
  - 本轮新增 `event_emission_protocol.rs` 没有 masking fallback。

## Passes Completed
- 将 `topology.spawn_group` 的长转义 JSON example 提成局部 raw string,保持输出文本不变,提高可读性。

## Remaining Risk
- live dogfood 仍显示 worker artifact 写入和 timeout 稳定性问题。这不是本轮 deslop 范围内的安全小修,已作为后续问题记录。

## [2026-05-21 20:58:30] [Session ID: omx-1779158263949-kticiv] 笔记: architect-style 本地审计

## 审计对象
- `crates/ralph-core/src/prompt_surface.rs`
- `crates/ralph-core/src/topology_spawn.rs`
- `crates/ralph-core/src/parallel/supervisor/topology_runtime.rs`
- `crates/ralph-core/src/parallel/supervisor/routing.rs`
- `crates/ralph-core/src/parallel/supervisor.rs`
- `crates/ralph-cli/src/record_session.rs`
- `crates/ralph-cli/src/parallel_runner.rs`
- `crates/ralph-cli/src/display.rs`
- `crates/ralph-tui/src/state/parallel.rs`
- `crates/ralph-tui/src/widgets/instances.rs`
- `crates/ralph-core/src/event_emission_protocol.rs`
- `crates/ralph-core/tests/event_loop_ralph.rs`
- 相关 specs/tests。

## 关键审计问题

### 1. EffectiveRoleContract 是否是 downstream 唯一真相源?
- 静态证据:
  - `TopologySpawnMember.role_contract` 注释明确 raw hint。
  - `ParallelSupervisor.effective_role_contracts` 保存 per-instance canonical contract。
  - dynamic worker prompt 注入来自 `effective_role_contracts.get(instance_id).render_worker_section()`。
  - agents snapshot 写 `EffectiveRoleContract::summary()`。
- 动态/测试证据:
  - `dynamic_worker_prompt_contains_effective_role_contract` passed。
  - `agents_snapshot_stores_role_contract_summary_not_full_contract` 已在 topology focused suite 通过。
- 结论: PASS。

### 2. role_contract 是否只作为 raw hint?
- 静态证据:
  - parser 只解析结构。
  - `canonicalize_topology_role_contract` 执行 role_name / identity_source / input_contract / output_contract / allowlist 校验。
  - raw objective mismatch 只写 warning,canonical objective 来自 member.task。
- 动态/测试证据:
  - topology focused suite 9 passed,包含 conflict、objective canonical、non-task-derived、control-plane topic、empty intersection、delivery topic exclusion。
- 结论: PASS。

### 3. output allowlist 是否防越权?
- 静态证据:
  - allowlist 与 target hat publishes 取交集。
  - control-plane topic fail closed。
  - delivery_topic 显式剔除。
- 动态证据:
  - live dogfood 中用户要求 `analysis.done`,但当前 builder publishes 只有 `build.done/build.blocked`,worker 最终未成功越权发布 `analysis.done`。
- 结论: PASS,但 dogfood prompt 与 ralph.yml publishes 不一致需要后续改 dogfood 配置或 prompt。

### 4. event protocol guidance 是否覆盖 live misnesting?
- 静态证据:
  - `event_emission_protocol.rs` 现在明确 `role_contract` 是 sibling field。
  - 明确 `input` MUST be string。
  - 明确 Do NOT put `role_contract` inside `input`。
- 动态证据:
  - live dogfood 首轮失败就是 `input` object。
- 测试证据:
  - `event_emission_protocol::tests::topology_spawn_prompt_documents_parent_visible_group_spawn_contract` passed。
  - `runtime_capability_catalog_is_injected_only_into_ralph_prompt` passed 并断言 coordinator prompt 包含上述 guidance。
- 结论: PASS。

### 5. 测试环境是否足够稳定?
- 发现并修复:
  - `test_reads_actual_events_jsonl_with_object_payloads` 不再依赖工作区 `.ralph/events.jsonl`,改用 fixture。
- post-fix evidence:
  - `RUSTFLAGS="-Dwarnings" cargo test --quiet` passed。
- 结论: PASS。

## Architect Verdict
- PASS,没有发现阻塞性架构问题。
- 非阻塞后续问题:
  1. live dogfood 未自然收敛,termination=Interrupted。
  2. worker 在只读任务中写入 `ralph/log/...` 和 `.agent/memories.md`,需要后续约束 dogfood workspace/artifact policy。
  3. 如果要严格验证 `analysis.done`,应使用 target hat publishes 包含 `analysis.done` 的 dogfood config。

## [2026-05-22 12:09:14] [Session ID: omx-1779158263949-kticiv] 笔记: clean live dogfood 结果与 agents snapshot 差异

## 来源

### 来源1: live dogfood record summary

- record-session: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-114604.jsonl`
- summary: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-114604.summary.txt`
- stdout: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-114604.stdout.txt`
- stderr: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-114604.stderr.txt`

## 综合发现

### clean dogfood 已自然收敛

- `RUN_STATUS=0`。
- `Termination.reason=CompletionPromise`。
- `Evidence Inspect` 显示 `iterations=5`, `elapsed_secs=49.620`。
- `topology.spawn_group: 1`。
- `topology.spawn.result: 1`,且 `parent_topology_unchanged=false`, `failed=0`。
- `analysis.done: 3`,来源实例是 `builder#2,builder#3,builder#4`。
- `capability.request/result/failed` 都是 0,证明没有走 isolated child-run path。
- `reply.human.message: 3`,最后一条明确说 3/3 已全部收到。

### 本轮 clean config 达到了用户指定的隔离目标

- 未配置 `confessor` 与 `confession_handler`,因此没有 confession phase 继续拉长收敛链路。
- `builder.publishes` 包含 `analysis.done`,所以 worker 的 result topic 不再被当前 hat 发布权限阻止。
- coordinator role_args 保留 `-c features.hooks=false`。
- worker role_args 保持空数组,因此 worker 仍按正常 Codex hooks 运行。

### agents snapshot 少 builder#4 的原因判断

现象:
- record summary 的 `Result Topics` 有 `analysis.done: 3 source_instances=builder#2,builder#3,builder#4`。
- 但 `Agents Snapshot` 只列出 `builder#1,builder#2,builder#3,ralph#1`,缺少 `builder#4`。

静态证据:
- `build_agents_snapshot()` 从 `self.instances.keys()` 生成实例列表。
- `unregister_dynamic_instance()` 会从 `self.instances`、`dynamic_instances`、`agents_last_inputs`、`fixed_role_metadata`、`effective_role_contracts` 中移除动态实例。
- 主循环中动态实例进入 `Done` 后会调用 `unregister_dynamic_instance()` 并立即 `write_agents_snapshot_best_effort()`。

动态证据:
- stdout 中 `builder#4` 先于其他 worker 完成并进入 `done`。
- `.ralph/agents.json` 的 `generated_at` 是 run 结束附近,其中保留了还未被回收的 `builder#2` 和 `builder#3`,但 `builder#4` 已因 done/TTL 回收不在 registry 中。
- record-session durable stream 仍完整保留 `builder#4` 的 spawn result 与 `analysis.done`。

结论:
- 这不是 `builder#4` 没跑。
- 这是 agents sidecar 的语义限制: `.ralph/agents.json` 是当前 registry 快照,不是完整历史拓扑或完成实例账本。
- 若希望 `record summary --agents-file` 在 run 后也能展示所有 spawned dynamic instances,需要新增历史/tombstone 视图或 final snapshot policy,不能只依赖当前 registry。

## [2026-05-22 12:11:27] [Session ID: omx-1779158263949-kticiv] 笔记: task_plan 超限触发的 continuous-learning 摘要

## 六文件摘要（用于决定如何沉淀知识）
- 涉及的上下文集: 默认六文件。
- 任务目标: 给 task-derived role contract live dogfood 制作 clean config,关闭 confessor,对齐 `analysis.done`,验证 3-worker 自然收敛。
- 关键决定: 不修改长期 `ralph.yml`; 使用 `/tmp/ralph-clean-task-derived-dogfood-20260522.yml` 和 `/tmp/ralph-clean-task-derived-dogfood-20260522.prompt.md`。
- 关键发现: record-session 证明三 worker 成功,但 `.ralph/agents.json` 是 current registry snapshot,不是历史实例账本。
- 实际变更: 本轮只写文档上下文与 `EXPERIENCE.md`,没有改业务代码。
- 暂缓事项: agents snapshot 需要 tombstone/final historical view 或在 summary 中更明确标注 sidecar 语义。
- 错误与根因: 临时 config 移除 confession_handler 后仍保留 `complete_publishes: workflow.complete`,被 validator 正确拒绝;已改为只用 `LOOP_COMPLETE`。
- 重大风险 / 规律: 对 parent-visible dynamic spawn 的完成判断必须以 record-session durable stream 为主,sidecar 只能辅助观察当前状态。
- 可复用点候选: clean dogfood config 模式; role-aware coordinator hooks 隔离; record-session vs agents snapshot 证据分层。
- 最适合写到哪里: 已写入 `EXPERIENCE.md` 的 `exp-20260522-clean-live-dogfood-record-session-vs-agents-snapshot`。
- 是否提取/更新 skill: 否,这是 Ralph 项目特有经验,更适合项目级 `EXPERIENCE.md`。

## [2026-05-22 13:35:24] [Session ID: omx-1779158263949-kticiv] 笔记: completed dynamic instances 实现设计

## 现象
- Dynamic instance 完成后会进入 `Done`。
- Supervisor 看到 dynamic instance `Done` 后调用 `unregister_dynamic_instance()`。
- `unregister_dynamic_instance()` 会从 `instances`、`dynamic_instances`、`agents_last_inputs`、`fixed_role_metadata`、`effective_role_contracts` 删除该实例。
- `build_agents_snapshot()` 当前只从 `self.instances.keys()` 生成 `.ralph/agents.json` 的 `instances`。
- 因此已完成并被回收的 dynamic instance 会从 agents sidecar 消失,但 record-session 仍能证明它曾经 spawn 并发布 result。

## 假设
- 主假设: 在 `AgentsSnapshot` 增加 `completed_dynamic_instances` tombstone 列表,并在 unregister 前保留 summary-only snapshot,可以让 agents sidecar 和 Evidence Inspect 明确表达已完成 dynamic instances。
- 备选解释/方案: 只在 `record summary` 里从 `topology.spawn.result` 和 `Result Topics` 合成历史实例视图,但 `.ralph/agents.json` 和 `ralph agents` 仍缺少 completed 信息。

## 方案选择
- 采用主假设。
- 原因:
  - 用户明确要求同时让 Evidence Inspect / agents snapshot 表达 completed dynamic instances。
  - tombstone 不影响调度 registry,只增加观察面历史区。
  - 可以保持 `instances` 继续表示 current registry,避免把 completed instance 伪装成可投递实例。

## 验证计划
- Core focused test: dynamic instance unregister 后,`build_agents_snapshot()` 的 `instances` 不再包含它,但 `completed_dynamic_instances` 包含它,并保留 role contract summary / last input / final_state。
- CLI agents test: `ralph agents` 单独显示 Completed dynamic instances,不把它混进 active/current instance 表。
- Record summary test: Evidence Inspect 输出 `Completed Dynamic Instances` section。
- Integration topology spawn test: agents snapshot 仍能表达当前 dynamic instances,并兼容新的字段。

## [2026-05-22 14:25:19] [Session ID: omx-1779158263949-kticiv] 笔记: completed dynamic instances 最终验证结论

## 已验证结论
- `AgentsSnapshot.instances` 继续表示 current registry / 当前可投递实例。
- 新增 `AgentsSnapshot.completed_dynamic_instances` 表示已完成并从 registry unregister 的 dynamic instance tombstone。
- Supervisor 在 `unregister_dynamic_instance()` 删除 registry 数据前,保留 summary-only tombstone。
- Evidence Inspect 的 Agents Snapshot 区块现在明确区分:
  - `instances(current_registry)`。
  - `completed_dynamic_instances`。
- `ralph agents` 现在独立展示 `Completed dynamic instances`,不会把已完成实例混进 current instances 表。

## 验证证据
- `cargo test -p ralph-core parallel::supervisor::routing_tests::completed_dynamic_instance_remains_visible_as_agents_tombstone -- --exact --nocapture`: passed。
- `cargo test -p ralph-cli record_session::tests::evidence_inspect_renders_completed_dynamic_instances_from_agents_snapshot -- --exact --nocapture`: passed。
- `cargo test -p ralph-cli --test integration_agents test_agents_command_prints_completed_dynamic_instances_separately -- --exact --nocapture`: passed。
- `cargo test -p ralph-cli --test integration_agents -- --nocapture`: 8 passed。
- `cargo test -p ralph-cli --test integration_topology_spawn -- --nocapture`: 1 passed。
- `cargo test -p ralph-core topology_spawn_group -- --nocapture`: 15 passed。
- `cargo test -p ralph-cli record_session::tests::aggregate_collects_evidence_inspect -- --exact --nocapture`: passed。
- `cargo test -p ralph-cli --test integration_capability -- --nocapture`: 8 passed。
- `cargo fmt --all -- --check`: passed。
- `git diff --check`: passed。
- `cargo test -p ralph-core smoke_runner`: 12 passed。
- `RUSTFLAGS="-Dwarnings" cargo test --quiet`: passed。

## 关键边界
- completed tombstone 是观察面历史账本,不是调度 registry。
- tombstone 只保存 summary-only 字段,避免把完整 prompt / contract 复制进 sidecar。
- record-session 仍是 durable 历史真相源; `.ralph/agents.json` 现在同时提供 current registry 与 completed dynamic tombstone 的辅助快照。

## [2026-05-22 15:29:08] [Session ID: omx-1779158263949-kticiv] 笔记: clean 3-worker live dogfood completed_dynamic_instances 真实展示

## 运行文件
- record-session: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-152219.jsonl`
- stdout: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-152219.stdout.txt`
- stderr: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-152219.stderr.txt`
- summary: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-152219.summary.txt`
- agents display: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-152219.agents.txt`
- agents snapshot: `.ralph/agents.json`

## 动态证据
- RUN_STATUS=0。
- Termination.reason=CompletionPromise。
- iterations=5。
- elapsed_secs=155.208。
- topology.spawn_group=1。
- topology.spawn.result=1。
- topology.spawn.failed=0。
- topology.spawn.result 显示 parent_topology_unchanged=false。
- requested_instances=3。
- spawned dynamic instances:
  - builder#2: 功能补充, task-derived, temporary, role_contract_hash=erc-096c9f14。
  - builder#3: 功能完善, task-derived, temporary, role_contract_hash=erc-dfd3922b。
  - builder#4: review, task-derived, temporary, role_contract_hash=erc-6c5d8b99。
- Result Topics 显示 analysis.done=3, source_instances=builder#2,builder#3,builder#4。

##  与 Instance        | Hat     | State    | Dynamic | Source            | Fixed Role       | Role Contract        | Last Input
---------------|---------|----------|---------|-------------------|------------------|----------------------|----------------------------------------
builder#1      | builder | idle     | no      | config-derived    | -                | -                    | -
ralph#1        | ralph   | idle     | no      | config-derived    | -                | -                    | analysis.done: {"role":"review","suggestions":["把当前演... 展示
- current registry instances=2:
  - builder#1 static idle。
  - ralph#1 static idle。
- completed_dynamic_instances=3:
  - builder#2 final_state=done, last_input=build.task, retirement_reason=dynamic_instance_unregistered_after_done。
  - builder#3 final_state=done, last_input=build.task, retirement_reason=dynamic_instance_unregistered_after_done。
  - builder#4 final_state=done, last_input=build.task, retirement_reason=dynamic_instance_unregistered_after_done。
- Instance        | Hat     | State    | Dynamic | Source            | Fixed Role       | Role Contract        | Last Input
---------------|---------|----------|---------|-------------------|------------------|----------------------|----------------------------------------
builder#1      | builder | idle     | no      | config-derived    | -                | -                    | -
ralph#1        | ralph   | idle     | no      | config-derived    | -                | -                    | analysis.done: {"role":"review","suggestions":["把当前演... 已显示独立  表,证明 CLI 观察面可读。

## 观察到的附带问题
- coordinator 最终输出的  是多行 XML event。
- record-session 中能看到 stdout=true 的 ,但  的  没列出 。
- 当前结论: completed dynamic instances 验证通过;但最终人类摘要事件如果要求 durable bus.publish,应后续要求 single-line event 或增强 multi-line event parsing。
## [2026-05-22 15:30:56] [Session ID: omx-1779158263949-kticiv] 更正记录: clean 3-worker live dogfood completed_dynamic_instances 真实展示

> 更正说明: 上一条同主题上下文记录写入时误用了未 quoted heredoc,导致反引号内容被 shell 执行并污染部分正文。本条为完整更正版,以后引用本次 live dogfood 证据以本条为准。

### 运行文件
- record-session: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-152219.jsonl`
- stdout: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-152219.stdout.txt`
- stderr: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-152219.stderr.txt`
- summary: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-152219.summary.txt`
- agents display: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-152219.agents.txt`
- agents snapshot: `.ralph/agents.json`

### 动态证据
- RUN_STATUS=0。
- Termination.reason=CompletionPromise。
- iterations=5。
- elapsed_secs=155.208。
- topology.spawn_group=1。
- topology.spawn.result=1。
- topology.spawn.failed=0。
- topology.spawn.result 显示 parent_topology_unchanged=false。
- requested_instances=3。
- spawned dynamic instances:
  - builder#2: 功能补充, task-derived, temporary, role_contract_hash=erc-096c9f14。
  - builder#3: 功能完善, task-derived, temporary, role_contract_hash=erc-dfd3922b。
  - builder#4: review, task-derived, temporary, role_contract_hash=erc-6c5d8b99。
- Result Topics 显示 analysis.done=3, source_instances=builder#2,builder#3,builder#4。

###  与 Instance        | Hat     | State    | Dynamic | Source            | Fixed Role       | Role Contract        | Last Input
---------------|---------|----------|---------|-------------------|------------------|----------------------|----------------------------------------
builder#1      | builder | idle     | no      | config-derived    | -                | -                    | -
ralph#1        | ralph   | idle     | no      | config-derived    | -                | -                    | analysis.done: {"role":"review","suggestions":["把当前演... 展示
- current registry instances=2:
  - builder#1 static idle。
  - ralph#1 static idle。
- completed_dynamic_instances=3:
  - builder#2 final_state=done, last_input=build.task, retirement_reason=dynamic_instance_unregistered_after_done。
  - builder#3 final_state=done, last_input=build.task, retirement_reason=dynamic_instance_unregistered_after_done。
  - builder#4 final_state=done, last_input=build.task, retirement_reason=dynamic_instance_unregistered_after_done。
- Instance        | Hat     | State    | Dynamic | Source            | Fixed Role       | Role Contract        | Last Input
---------------|---------|----------|---------|-------------------|------------------|----------------------|----------------------------------------
builder#1      | builder | idle     | no      | config-derived    | -                | -                    | -
ralph#1        | ralph   | idle     | no      | config-derived    | -                | -                    | analysis.done: {"role":"review","suggestions":["把当前演... 已显示独立  表,证明 CLI 观察面可读。

### 附带观察
- coordinator 最终输出的  是多行 XML event。
- record-session 中能看到 stdout=true 的 ,但  的  未列出 。
- completed dynamic instances 验证已经通过;如果后续要求 final human summary 进入 durable bus.publish / Result Topics,建议单独处理 single-line event 或 multi-line parser。


## [2026-05-22 15:32:09] [Session ID: omx-1779158263949-kticiv] 最终更正记录: clean 3-worker live dogfood completed_dynamic_instances 真实展示

> 更正说明: 前两次上下文追加误用了未 quoted heredoc,导致反引号内容被 shell 执行。本条由 Python 直接追加,未经过 shell heredoc。以后引用本次 live dogfood 证据以本条为准。

### 运行文件
- record-session: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-152219.jsonl`
- stdout: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-152219.stdout.txt`
- stderr: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-152219.stderr.txt`
- summary: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-152219.summary.txt`
- agents display: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-152219.agents.txt`
- agents snapshot: `.ralph/agents.json`

### 动态证据
- RUN_STATUS=0。
- Termination.reason=CompletionPromise。
- iterations=5。
- elapsed_secs=155.208。
- topology.spawn_group=1。
- topology.spawn.result=1。
- topology.spawn.failed=0。
- topology.spawn.result 显示 parent_topology_unchanged=false。
- requested_instances=3。
- spawned dynamic instances:
  - builder#2: 功能补充, task-derived, temporary, role_contract_hash=erc-096c9f14。
  - builder#3: 功能完善, task-derived, temporary, role_contract_hash=erc-dfd3922b。
  - builder#4: review, task-derived, temporary, role_contract_hash=erc-6c5d8b99。
- Result Topics 显示 analysis.done=3, source_instances=builder#2,builder#3,builder#4。

### `.ralph/agents.json` 与 `ralph agents` 展示
- current registry instances=2:
  - builder#1 static idle。
  - ralph#1 static idle。
- completed_dynamic_instances=3:
  - builder#2 final_state=done, last_input=build.task, retirement_reason=dynamic_instance_unregistered_after_done。
  - builder#3 final_state=done, last_input=build.task, retirement_reason=dynamic_instance_unregistered_after_done。
  - builder#4 final_state=done, last_input=build.task, retirement_reason=dynamic_instance_unregistered_after_done。
- `ralph agents` 已显示独立 `Completed dynamic instances: 3` 表,证明 CLI 观察面可读。

### 附带观察
- coordinator 最终输出的 `reply.human.message` 是多行 XML event。
- record-session 中能看到 stdout=true 的 `ux.terminal.write`,但 `record summary` 的 `Result Topics` 未列出 `reply.human.message`。
- completed dynamic instances 验证已经通过;如果后续要求 final human summary 进入 durable bus.publish / Result Topics,建议单独处理 single-line event 或 multi-line parser。
