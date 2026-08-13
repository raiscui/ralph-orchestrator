# LATER_PLANS

> 说明: 记录本次不落地,但值得后续跟进的事项. 仅追加到文件末尾.

## 2026-02-11 20:55 +0800 | Ralph Codex MCP 后续增强

- 主题: 跨进程恢复 `ralph#1/ralph#2` 的 Codex MCP `threadId`.
- 当前状态: 仅进程内常驻有效,进程重启后 thread 会重新建立.
- 后续建议:
  - 在 `.ralph/` 下落盘会话元信息(实例 -> threadId).
  - 恢复时做一次有效性探测,失效则自动重建.

## 2026-02-11 21:13 +0800 | parallel-hat-instances 场景稳定性收敛

- 背景: 真实 Codex 下场景存在行为漂移,`routing.escalate` 与 `LOOP_COMPLETE` 不稳定,导致 120s timeout.
- 建议后续动作:
  - 收敛 scenario 指令,减少对“模型必须输出 200 行/严格条件分支”的依赖.
  - 调整断言为“语义等价”而非固定实例计数(允许 `collector#2`/`writer#1` 接管).
  - 评估提升 `max_runtime_seconds` 或更早、确定性的 completion candidate.

## 2026-02-11 22:49 +0800 | 回写: parallel-hat-instances 场景稳定性收敛已完成

- 状态: 已完成并落地。
- 对应实现:
  - `crates/ralph-e2e/src/scenarios/parallel/hat_instances.rs` 已收敛为稳定事件链与语义断言。
  - `scripts/run-parallel-hat-instances-codex.sh` 已修复 release 构建与 workspace 清理基线。
- 验证: `parallel-hat-instances` + `parallel-hat-instances-zh` 均通过。

## 2026-02-11 23:20 +0800 | 编译期 all_hat 配置变更提示能力

- 背景:
  - `config/all_hat.md` 已改为编译期内嵌,修改后必须重新编译才能生效.
- 后续建议:
  - 可在 `ralph doctor` 或启动日志中增加提示:
    - 检测工作区 `config/all_hat.md` 修改时间晚于二进制构建时间时,提示用户重编译.

## 2026-02-13 01:12 +0800 | session_strategy 后续增强(可选)

- 显式降级/重置语义:
  - 目前采用方案1(只升级,不降级).
  - 若未来确实需要从 mcp 回到 exec,建议新增 control-plane topic 或显式 reset 事件,并配套 handoff summary.
- MCP 会话资源收敛:
  - 目前 mcp session 会按 instance 创建,并在进程退出时统一 shutdown.
  - 若 hat autoscale 很多动态实例,可考虑按 dynamic idle TTL 同步关闭对应 mcp session.

## 2026-02-14 18:01 +0800 | E2E: Codex 后端降噪/提速(可选)

- 现状:
  - 并行 E2E 在真实 Codex 下,stdout/stderr 里会出现较长的思考/解释输出,并且 completion 的第二轮有时会被 max_runtime 逼近.
  - 这会让 E2E 时长和波动变大.

- 后续建议:
  - 在 `crates/ralph-e2e` 的场景生成 `ralph.yml` 时,对 codex backend 使用 `HatBackend::NamedWithArgs` 追加参数:
    - 指定更快的 `--model`.
    - 或用 `-c` 覆盖 config(例如 `-c model=...`),并把 reasoning 相关选项调低.
  - 为并行 runner 增加一个仅 E2E 使用的开关,允许 ralph#1 不强制走 MCP(改走 exec),以降低第二轮收敛的时延不确定性.

## 2026-02-14 20:41 +0800 | 回写: E2E Codex 后端降噪/提速已完成(按建议落地)

- 状态: 已完成并落地(仅影响 E2E 生成的 `ralph.yml`,不改默认配置).
- 对应实现:
  - `crates/ralph-e2e/src/scenarios/parallel/hat_instances.rs`
  - `crates/ralph-e2e/src/scenarios/parallel/starting_event_inference.rs`
  - 生成的 `cli` 配置改为 `backend: custom + command: codex + args: ["exec","--full-auto","-c",...]`.
  - 默认注入:
    - `-c model_reasoning_effort="low"`
    - `-c model_reasoning_summary="none"`
    - `-c rmcp_client=false`
- 验证:
  - `cargo test -p ralph-e2e` ✅
  - `bash scripts/run-parallel-hat-instances-codex.sh` ✅
    - `parallel-hat-instances`: 77.9s
    - `parallel-hat-instances-zh`: 77.6s

## 2026-02-14 23:04 +0800 | 后续增强: event.id 端到端覆盖(可选)

- E2E 覆盖建议:
  - 目前 `crates/ralph-e2e` 的 `ExecutionResult.events` 只提取了 topic/payload/source_instance.
  - 可考虑在不大改现有断言的前提下,新增一条"文件级"断言:
    - 针对 `.ralph/events.jsonl`(debug log)逐行检查 `id` 字段存在且非空.
- 外部事件输入建议:
  - `crates/ralph-tui/src/external_event_writer.rs` 写入的外部事件 JSONL 目前不带 `id`.
  - 运行时会在 Supervisor 路由时补齐 `event.id` 并写入 debug log.
  - 如果你希望"外部输入文件"本身也可引用,可以为该 JSONL schema 增加可选 `id` 字段,默认填 `new_event_id()`.

## 2026-02-15 10:58 +0800 | 后续增强: parallel TUI 逐帧录制(可选)

- 需求动机:
  - 现在 `--record-session` 录的是 cassette(事件 + stdout),适合 replay/E2E.
  - 但它不包含 TUI 逐帧画面,不适合做 UI 动画回归或演示录像.
- 建议:
  - 在 `ralph-tui` 渲染 tick 里可选写入 `UxEvent::TuiFrame` 到 `SessionRecorder`.
  - 默认关闭,避免 JSONL 过大与额外 I/O 开销.
- 备注:
- `crates/ralph-core/src/session_recorder.rs` 已支持 `ux.tui.frame` 的 record 类型,属于 wiring 级补齐.

## 2026-02-15 12:26 +0800 | 后续可选: cassette `text` 字段的体积控制与旧数据补全

- 现状:
  - `ux.terminal.write` 现在会同时落盘 `bytes`(base64) + `text`(UTF-8 lossy).
  - cassette 可读性提升,但文件体积也会随之增大.
- 建议(可选):
  - 增加一个录制开关(例如 CLI flag 或 config),允许在“只做回放/CI”时关闭 `text` 以节省体积.
  - 提供一个小工具/子命令,把旧 cassette 批量补齐 `text` 字段,方便历史排障.

## 2026-02-16 22:16 +0800 | 后续可选增强: autopilot 的可配置性与证据稳健性

- 可配置 required/banned topics(可选):
  - 现状: hard verdict 固化了严格闭环 topic 集合.
  - 后续可选: 通过 CLI flag 或 config 提供覆盖/扩展(required/banned topic list).
- agent 分析解析稳健性(可选):
  - 现状: 从分析子进程 stdout 用 regex 抓取 `<event topic="analyze.complete">...`.
  - 后续可选: 给分析子进程也启用 `--record-session`,从 JSONL 的 `bus.publish` 解析 analyze.complete,避免 stdout 形态漂移.
- run 模式 preflight 失败时的 report(可选):
  - 现状: 少数 preflight 错误(例如 config 文件缺失/不可读)会直接返回 Err,不一定能落盘 report.
  - 后续可选: 只要 out-dir 可创建,就尽量落盘 report.json/report.md,便于 CI 收集与排障.

## 2026-02-18 10:21 +0800 | 后续可选: agent analysis 子进程 stdout/stderr 落盘(排障更友好)

- 现状:
  - autopilot agent analysis 目前只落盘 `analysis_prompt.md`/`analysis_ralph.yml`/`analysis_input.json`/`analysis_output.json`.
  - 若子进程退出码非 0(尤其是 exit code=2 的护栏退出),stdout/stderr 对排障很关键,但当前未落盘.
- 建议:
  - 在 `run_agent_analysis()` 内把子进程 stdout/stderr 始终写入 out_dir:
    - `analysis_stdout.txt`
    - `analysis_stderr.txt`
  - 这样 report.json 的 reason 不需要塞超长文本,同时也保留可审计证据.

## 2026-02-18 14:08 +0800 | autopilot 报告增强: 并行度指标/硬断言(可选)

- 动机:
  - 现在 hard verdict 主要验证 topic 闭环(`experiment.task/result/reviewed`, `integration.*`, `experiment.complete`).
  - 但它不直接验证“runner 是否真的并发”.
  - 对并行 engine 的回归来说,并发度是更关键的信号.

- 可选增强方向:
  1. report.json 里新增并行度指标(不改变默认 verdict):
     - unique_runner_instances_seen
     - runner_instances_entered_running
     - max_concurrent_running
     - 证据引用: out_dir/stdout.txt 里 `experiment_runner#*:state` 行.
  2. 增加可选 hard assertion(通过 flag 开启),例如:
     - `--require-max-concurrent-running >= 2`
     - 或 `--require-runner-instances >= 2`

- 价值:
  - CI 可以把“并行性”变成明确的 backpressure 信号.
  - parallel 示例也更接近“有效并行 smoke test”.

## 2026-02-18 15:06 +0800 | 备注: ralph event alias 已移除

- 外部事件注入命令已统一为 `ralph emit`.
- 因此这里的“emit/event 自动向上查找 marker”后续只需要考虑 `ralph emit`.

## 2026-02-18 18:14 +0800 | 回写: autopilot report.md 已补齐 Topic Counts(部分完成)

- 已完成:
  - report.md 增加 Topic Counts(关键 topic 次数),并在 stdout 可用时额外展示并行度指标.
- 仍待(可选,尚未实施):
  - 把并行度指标写入 report.json/analysis_input,并提供可选 hard assertion(例如 require max_concurrent_running>=2).
  - 幂等性加固: 降低重复 experiment.task / integration.task 的重投频率,或对同 experiment_id 做去重.
  - 降噪: rmcp UnexpectedContentType(None) 相关的工具启动错误,是否需要更友好的容错/降级日志.

## 2026-02-18 19:20 +0800 | autopilot 派生 child_ralph.yml 的敏感信息风险(可选)

- 现状:
  - `ralph autopilot run --child-parallel-max-running-jobs <N>` 会在 out_dir 写入 `child_ralph.yml`(完整派生配置),并用它启动子进程 `ralph run`.
  - 这对审计/复现实验很友好.
- 潜在风险:
  - 如果用户的 config 含 token/密钥,且 out_dir 作为 CI artifact 上传,可能泄露敏感信息.
- 可选改良方向:
  1) 默认把派生 config 写入临时目录(不进入 out_dir artifact),仅用于本次 child run.
  2) 或新增开关: `--keep-derived-config`,默认不落盘到 out_dir.
  3) 或在 README/skill 文档里加明确提示: out_dir 可能包含敏感配置,上传前需审计.

## 2026-02-18 21:15 +0800 | 后续可选增强: `ralph agents` 快照的定位与安全性

- 定位增强(可选):
  - 现状: `ralph agents` 默认读 `./.ralph/agents.json`.
  - 建议: 支持向上遍历父目录查找最近的 `.ralph/agents.json`,避免用户在子目录执行时误以为“没有快照”.
  - 或者复用 `.ralph/current-events` marker 的定位逻辑,从 marker 推导工作区根目录.

- 安全性增强(可选):
  - 现状: agents 快照只写 payload 的单行截断预览(160 字符),仍可能包含敏感信息.
  - 建议: 增加轻量 redaction(例如常见 token 前缀/"api_key" 键),或提供 `--redact` 开关.

- 交互增强(可选):
  - 快照里可选补充 `session_strategy`/`running_job_id`,让“它在做什么”更直观.

## 2026-02-18 21:26 +0800 | 完成记录: `ralph agents` 子目录自动定位已实现

- 已完成:
  - `ralph agents` 现在会向上遍历父目录,选择最近的 `.ralph/agents.json`。
  - `ralph agents --watch` 定时刷新已实现。
- 仍待(可选):
  - 快照内容脱敏/`--redact`。

## 2026-02-19 19:45 +0800 | 并行 example: integration.task.commit 非 hash 的漂移风险(可选)

- 现象:
  - 个别 run 中 `ralph#1` 可能发出 `integration.task.commit="exp-par-001"` 这类占位值(非 git hash).
  - integrator 会因此走 fallback(例如按提交信息 `git log --grep ...` 去定位真实 commit).
- 风险:
  - fallback 会强依赖 git 历史完整性与提交信息格式,更容易引入 flaky.
  - 在“临时 repo 不保留历史”的场景下,该 fallback 会直接 miss,导致闭环失败.
- 建议(可选):
  1) 在 example 的 `event_loop.ralph_prompt` 增强硬门槛: `integration.task.commit` 必须是 git hash(来自 experiment.result.commit).
  2) 在 integrator instructions 中明确: 非 hash 直接 `integration.blocked` 或自动解析后回写一个“修正过的 integration.task”(带 hash).
  3) autopilot hard verdict 增加一条断言: `integration.task` payload 的 `commit` 字段必须匹配 git hash 形态(避免假阳性闭环).

## 2026-02-24 13:30 +0800 | E2E runner filter: substring 匹配容易误跑 live 场景(消耗 token)

- 现象:
  - `cargo run -p ralph-e2e -- codex --filter parallel-app-server-steer-multi-turn`
    会同时匹配:
    - `parallel-app-server-steer-multi-turn`
    - `parallel-app-server-steer-multi-turn-live`
  - 这会在你只想跑 fake 场景时,意外把 live 场景也跑了(真实 token 成本 + 时长)。

- 后续建议(可选):
## 2026-03-09 23:19:50 +0800 | 并行 completion promise 仍然容易被普通 prose 误触发(可选)

- 现状:
  - `crates/ralph-core/src/event_parser.rs` 的 `contains_promise()` 只要在事件标签外看到 completion token 就会命中。
  - 这对严格 example 很有用,但也意味着 coordinator 如果在等待态的普通文本里提到 `LOOP_COMPLETE`,会直接改变控制面语义。

- 本轮已落地的短期处理:
  - 通过 example prompt 把等待态改成“静默优先”,先把 `parallel-migration-rehearsal-example` 收住了。

- 后续可选增强:
  - 评估是否把 completion promise 检测进一步收紧成:
    - 独立单行匹配
    - 或保留一个更强的保留字/sentinel 语义
    - 或在并行模式下给 completion promise 增加结构化包裹/显式 event 替代

- 价值:
  - 可以降低 example prompt 对“不要随口提 completion token”这类软约束的敏感度。
  - 也能减少真实后端在等待态自由发挥时带来的误触发风险。

## 2026-03-09 12:34:05 +0800 | 后续可选增强: 给并行 E2E 增加结构化 job ledger

- 现状:
  - `parallel-trigger-routing-example` 这轮虽然通过了,但 `Hat job run counts` 仍然依赖 stdout 前缀 `[instance:out|err:job=...]`
  - `result.events` 可以证明业务闭环,却不能证明 `spec_logger` 这类“不发事件的 hat”究竟跑了几次
- 风险:
  - 只要 future 再出现 stdout 形态变化/降噪策略变化,这类断言就容易再次漂移
- 建议:
  - 补一份结构化 job ledger,例如 `.ralph/jobs.jsonl`
  - 或把 `job_id` 正式打通到 session recorder / supervisor 持久化路径
  - 这样 example 场景就能把:
    - 业务正确性 -> `events.jsonl`
    - job 次数/实例运行 -> 结构化 ledger
  - 和 stdout 的展示 contract 解耦
  - 提供更精确的筛选能力,例如:
    - `--filter-exact <scenario_id>`
    - 或 `--filter-regex <re>`
    - 或 `--exclude-filter <pattern>`(例如排除 `-live`)

## 2026-02-26 10:08 +0800 | 后续可选: TUI chat 把 human.message 回复渲染成“聊天输出”(隐藏 <event ...>)

- 背景:
  - 本次已在 parallel Supervisor 路由层打断了 `human.message` 自我对话回路。
  - 但在 TUI Output 面板里,用户仍可能看到 `<event topic="human.message">...` 的包装,不够像“聊天回复”。
- 建议(可选,偏 UX):
  1) TUI 展示层做特殊渲染:
     - 识别 `human.message` 的 bus.publish,把 payload 作为“chat reply”展示在独立区域。
     - 同时在 Output 面板里折叠/隐藏对应的 `<event ...>` 标签行,降低噪音。
  2) 若必须走协议层区分(不推荐,会引入新概念):
     - 引入专用 topic(例如 `ui.message`/`chat.reply`)区分“输入 vs 输出”,避免 `human.message` 语义歧义。

## 2026-02-26 14:50 +0800 | 后续可选: app-server prompt transcript 输出节流(只首 turn 或 header-only)

- 背景:
  - 目前为了对齐 `codex exec` 的可观测性,app-server 路径默认会在每次 `turn/start` 前回显完整 prompt transcript。
  - 但在交互式 TUI chat 场景,这会非常“刷屏”,并且容易让人误解为“不是同一个会话,每次都在重新注入 prompt?”。
- 建议(可选):
  1) 提供一个可配置开关:
     - `full`(默认): 每次 turn 打印全量 transcript(最可审计)。
     - `header-only`: 只打印一行 header(含 chars/instance/job),不打印全文。
     - `first-turn-only`: 只在每个 instance 的第一个 turn 打印全量,后续只打印 header。
     - `off`: 关闭 transcript(仅保留必要错误与 stderr)。
  2) 或者在 TUI 侧把 transcript 做折叠/单独面板,避免和 agent 输出混在一起。

## 2026-02-27 12:10 +0800 | 借鉴 openclaw: doctor/wizard/guardrails/lanes(可选)

- 背景:
  - 我阅读了 `openclaw/openclaw` 的源码与文档,它在"个人 AI assistant/网关/多通道/多后端"这类系统上,
    做了很多可迁移的工程化手段(而不只是写约定)。
  - 相关笔记: `notes.md` 末尾的 "源码研究: openclaw..." 段落。

- 后续建议(按价值/投入排序):
  1) 并发 "lane + generation + draining"(较大投入,但能降低 flaky):
     - 借鉴 openclaw 的 `CommandLane`/`command-queue.ts`:
       - 将不同风险/交互面的任务分 lane(主 job/外部注入/cron/分析子进程等)。
       - restart/early-exit 时 bump generation,忽略旧 in-flight completion,避免 stale 状态把队列锁死。
       - draining 模式下拒绝新任务,让退出/重启更确定。

  2) backend runner 超时原因结构化 + per-backend serialize(可选,中等投入):
     - 把 "硬超时" 与 "无输出超时" 区分为一等概念,并在错误/报告里显式标注 reason(提升可审计性)。
     - per-backend serialize/replaceExistingScope 语义也值得系统化整理(减少 session/端口/锁的互相污染)。

- 已推进(不再作为后续计划项):
  - `ralph doctor`: 已写 spec,见 `specs/ralph-doctor.spec.md`(实施进度见 `task_plan.md`)。
  - guardrails: 已落地 stdout-only 事件解析边界 + scratchpad truncate + HatJobResult 输出语义拆分(见 `WORKLOG.md`)。
  - no-output watchdog: Codex app-server runtime 已对齐 `HatJob.timeout/output_stale_timeout`(cancel 立即退出 + 超时重启 session)(见 `WORKLOG.md`)。
  - context window guard: 已在 `ralph doctor` 落地(配置 `adapters.<backend>.context_window_tokens` 后可启用 warn/block)。

## 2026-02-28 15:41 +0800 | 已落地: 并发 lane + generation + draining(对齐 openclaw command-queue 思路)

- 已落地:
  - spec: `specs/parallel-command-lanes.spec.md`
  - 实现: `crates/ralph-core/src/parallel/command_queue.rs`
- 已接入:
  - `workspace.git` lane 串行化 `git worktree add/remove` 与 clone->main 的 `git fetch`。
  - HatInstance shutdown draining: 退出前 best-effort workspace cleanup,并跳过 hooks(保证退出可控)。
- 因此:
  - `2026-02-27 12:10` 里的后续建议 1) "lane + generation + draining" 可视为已完成。

## 2026-02-28 18:40 +0800 | 借鉴 zeroclaw: approval/audit/prompt-guard/doctor-json/docs hub(可选)

- 背景:
  - 我阅读了 https://github.com/zeroclaw-labs/zeroclaw 的源码与文档.
  - 其中一些机制和 ralph 的 "backpressure + gate + 可审计" 方向高度同构.
  - 相关笔记: `notes.md` 末尾的 "研究: zeroclaw-labs/zeroclaw..." 段落.

- 后续建议(可选,按 ROI/投入排序):
  1) docs IA: 在 docs 入口补 "10 秒决策树" 和 "按受众" 导航.
     - 目标: 降低新用户找文档的时间,减少 issue 噪音.
  2) gate/approval/audit: 把关键审批/咨询的证据落盘更明确.
     - 目标: 回放与排障时能回答 "谁触发,为什么,结果是什么".
  3) PromptGuard(轻量版): 先覆盖最常见注入特征,并把命中 patterns 写入诊断日志.
     - 目标: 把 "感觉被 prompt injection" 变成可观测信号.
  4) 供应链治理: 评估引入 cargo-deny(license/source allowlist)作为 CI 可选护栏.
     - 目标: 把依赖风险变成显式 backpressure 信号.

- 边界提醒(避免做成平台):
  - 不建议把 ralph 扩成 "runtime OS"(channel/gateway/硬件外设/多租户存储全家桶).
  - 优先做 "协议清晰 + 审计证据 + 可复现测试" 这类薄层改良.

## 2026-03-02 16:12 +0800 | 已完成回收: e2e 报告一致性

- 已完成项:
  - `2026-03-02 16:42 +0800 | e2e 报告一致性(可选优化)` 已落地。
- 完成结果:
  - 默认 `--report markdown` 现已同步刷新 `report.json`。
  - `report.md` 与 `report.json` 不再出现“新旧轮次混用”的歧义。

## 2026-03-04 14:05 +0800 | 可选: 面向编程智能体的"实时证据"工作流强化(record watch)

- 背景:
  - `ralph record watch`/`ralph record summary` 已经把 record-session 从“事后文件”升级为“可实时观测的证据流”。
  - 但为了避免再次出现“智能体说修好了,人工测仍失败”的假阳性,需要把它纳入默认验证闭环.

- 可选增强(偏流程/工具,按 ROI 排序):
  1) record-session 元信息更自描述:
     - `_meta.session_start` 增加 `current_exe`(实际二进制路径)与 `version`。
     - 价值: 一眼区分“跑的是不是同一个 ralph”,减少口径歧义。
  2) record watch 增强为可脚本化探针:
     - `--until-topic <topic>` / `--timeout <secs>` / `--grep <pattern>` 之类的参数.
     - 价值: 编程智能体可以用它做实时断言,而不是只能人工看输出.
  3) 文档与 DoD 固化:
     - 明确 "durability contract" 与 "display contract" 是两件事.
     - display 类修复默认做 2x2 验证矩阵(TUI/Pretty x Rendered/Plain).

## 2026-03-04 15:25 +0800 | 回溯: 2026-03-04 14:05 的"实时证据"增强已落地

- 已落地:
  - `_meta.session_start` 增加 `current_exe` 与 `version`.
  - `ralph record watch` 增加 `--until-event/--until-topic/--timeout-secs/--quiet` 并约定 timeout exit code=2.
  - 文档与规范已固化到:
    - `docs/advanced/testing.md`
    - `docs/guide/cli-reference.md`
    - 根 `AGENTS.md`

## 2026-03-06 10:42 +0800 | 并行 Output 滚动模型后续可选增强: 从"显示行预换行"进一步升级到"视觉行级滚动"

- 本次已修复:
  - reply 被长行包裹后顶出视口的问题
  - 自动到底/底部可见性重新可靠
- 仍可继续打磨(可选,不是本次 blocker):
  - 当前手动 `j/k` 滚动仍按 buffer 行推进
  - 由于 buffer 已是预换行后的显示行,大部分场景已足够
  - 但如果未来要支持更细粒度的"行内继续滚动 / 复制跨包裹行",可以把并行 Output 进一步统一成真正的视觉行级 viewport 模型
## 2026-03-07 16:34 +0800 | 跟进 `integration_agents` 里的 watch 测试稳定性

- 现象:
  - `cargo test -p ralph-cli` 中, `tests/integration_agents.rs::test_agents_command_watch_prints_output_at_least_once` 仍失败。
  - 失败断言是 `stdout.contains("Watching")`。
- 当前证据:
  - 用 shell 重定向与 Python `pipe + kill + wait` 最小复现时, `ralph agents --watch --watch-interval-ms 50` 能稳定输出 `Watching`、表头和 `writer#1` 行。
  - 因此问题不像主功能失效, 更像测试窗口太短、启动时序不稳, 或测试夹具与真实运行条件存在细微差异。
- 后续建议:
  - 单独为该测试做稳定性修正, 不与 Gemini backend 修复混做。
  - 优先方向:
    - 改为轮询 stdout 直到看到首轮输出, 而不是固定睡眠 `300ms` 后直接 kill。
    - 或者在测试里延长等待窗口, 并把失败时的 stdout/stderr 原样打印出来。

## 2026-03-08 13:08 +0800 | 可选后续: 收敛跨 crate 的 `<event ...>` 提取逻辑

- 背景:
  - 这次修 `ralph-e2e/src/analyzer.rs` 时,发现它和 `crates/ralph-cli/src/autopilot.rs` 对同一协议的提取规则已经出现细微分叉。
- 建议:
  - 评估把 `<event ...>` payload 提取收敛成共享 helper,或至少共享一组测试夹具。
- 价值:
  - 避免以后再次出现"主链路已兼容,旁路分析器仍卡在旧 regex"的隐性回归。

## 2026-03-08 13:31 +0800 | 回收: `<event ...>` 提取收敛已落地

- 已完成:
  - `ralph-core::EventParser::extract_last_payload_for_topic()`
  - `autopilot` 与 `ralph-e2e analyzer` 改用共享 helper
  - 相关回归测试已补齐并通过
- 因此,上面这条已不再是待办。

## [2026-03-09 09:16:00] 后续计划: 修复 example 场景的错路由与工作区副作用

- 排查 `parallel-trigger-routing-example` 与 `parallel-experimental-dev-engine-example` 的示例 prompt / scenario 构造,定位为什么运行时会持续产出 `build.task`、`human.message`、`reply.human.message` 的占位型事件,并触发 `build.task -> writer` 错路由。
- 修复 example 场景在 `LOOP_COMPLETE` 之后仍继续拉起新 job 的问题,重新锁定 deterministic job run count。
- 检查 `parallel-experimental-dev-engine-example` 的 integration workspace 选择逻辑,避免 example 测试直接在主仓库执行 `git cherry-pick` 并推进真实 HEAD。

## 2026-03-09 13:42 +0800 | 可选后续: 收敛 parallel completion 后的 late JobCompleted / receiver 生命周期竞态

- 先做一个最小验证:
  - 在 `ParallelSupervisor::drain_shutdown()` 与 `HatInstanceActor::on_job_completed()` 两侧补临时 tracing,确认 receiver drop 与 late JobCompleted 的先后顺序。
- 若验证成立,优先考虑两条方向:
  - 方向1: 让 Supervisor 在 completion 后继续持有 `instance_rx` 直到所有实例真正退出,再 drop receiver。
  - 方向2: completion/shutdown 期间若 `JobCompleted` 发送失败,视为收尾期非致命事件,避免把实例状态打成 failed。

## 2026-03-09 16:35 +0800 | 第二批真实并行 example 候选

- 背景:
  - 第一批 3 个高价值范例已经补齐并验证通过:
    - PR review
    - release checklist
    - human approval gate
- 后续可继续扩的实际场景:
  - incident response war-room
    - triage / log analysis / rollback plan / status comms 并行推进
  - migration rehearsal
    - schema diff / backup check / smoke test / go-no-go gate
  - proposal assembly
    - research / pricing / legal review / executive signoff 并行收敛
- 当前不立即继续的原因:
  - live E2E 成本较高。
  - 更适合等这一批 example 被实际使用一轮后,再按反馈补第二批。

## 2026-03-12 13:50 +0800 | batch-9 可选方向: 继续扩经营材料与预测对齐

- batch-8 已经补到:
  - 区域经营周会
  - 续费组合盘校准
  - 多区域 pipeline 同步
- 下一批如果继续扩,优先候选:
  - 董事会材料预演
  - 季度投资组合回顾
  - forecast commit 对齐会
  - 区域定价例外校准
- 继续约束:
  - 优先选能写出固定终态字段的场景
  - 对高漂移 lane 默认给 literal 单行模板
  - live E2E 仍优先使用 stdout out 行提取 final payload

## [2026-03-18 18:02:00] [Session ID: 2d1fc46f-d36c-45b6-af3b-ab3318b8c122] 默认资源仓 / selector preset 系统(探索结论待落地)

- 建议分阶段推进:
  1. 统一 `config source` 与 `prompt source` 的来源解析优先级,先补齐“无 `PROMPT.md` / 无 `ralph.yml`”启动闭环。
  2. 引入 resource catalog,把 builtin presets、minimal presets、prompt templates、example bundles 做成结构化索引。
  3. 引入用户级资源目录(实现层建议走 `ProjectDirs`,如有需要再给 `~/.ralph` 别名入口)。
  4. 若要让 Ralph 自选 preset,优先实现 bootstrap selector -> resolved config -> real run 的两阶段模式。
  5. 多 preset 混编只在定义清楚 merge 规则后开放,尤其要先处理 `cli` / `event_loop` / `hats` / `parallel` / `events` 的冲突策略。

- 特别提醒:
  - 不建议直接做“正式 run 中途热切换整套 `ralph.yml`”。
  - `examples/` 更像 bundle/template,未必都适合作为默认 selector catalog 的候选 workflow,最好单独分类。

## [2026-05-17 16:16:00] [Session ID: omx-1779004640353-blcixq] 后续建议: 并行 TUI 剩余状态增强

- 背景: 本轮排查发现 TUI 与 Codex/CLI 直接输出差异主要来自展示模型不同。TUI 当前偏操作面,CLI/log-mode 偏审计流。
- 建议后续落地:
  1. Instances 行补 `last_input.preview` / input topic / current job。
  2. Header 或 Footer 补 selected instance、state、job、last event、stderr visible/hidden。
- 验证建议:
  - 给 TUI Rendered / TUI Plain / Pretty Rendered / Pretty Plain 加 2x2 regression。
  - 用现有 `tui-validate` 或 ratatui TestBackend snapshot 验证 status 字段实际可见。


## [2026-05-17 16:51:58] [Session ID: omx-1779004640353-blcixq] 后续计划更新: TUI 状态摘要已部分落地,剩余 stderr/last-input 视图

- 已完成:
  - Instances 行显示 `job x/y`。
  - Footer 并行模式显示 selected instance、紧凑 job、render mode、last event。
- 仍未完成,后续建议保留:
  1. 明确显示 stderr visible/hidden,需要先把 `show_stderr` 从 runner 配置传入 TUI state。
  2. 如果要显示 `last_input.preview`,应优先复用 `.ralph/agents.json` 或把 last input 作为正式 TUI update,不要在 widget 中另行推断。

## [2026-05-17 18:18:00] [Session ID: omx-1779004640353-blcixq] 后续建议: TUI stderr 可见性与 last-input 视图仍可继续增强

- 已完成:
  - Codex 风格 `current_activity` 状态字段已经落地。
  - Footer / Instances 已能显示当前正在做什么和持续时间。
- 仍建议后续做:
  1. 明确显示 stderr visible/hidden,需要先把 `show_stderr` 从 runner 配置传入 TUI state。
  2. 如果要显示 `last_input.preview`,应优先复用 `.ralph/agents.json` 或把 last input 作为正式 TUI update。

## [2026-05-20 08:05:00] [Session ID: omx-1779158263949-kticiv] 后续建议: parent-visible spawn dogfood worker 收敛

- 背景: `topology.spawn.result` 后重复派发已经通过 record-session dogfood 验证修复,但 no-TUI dogfood 仍出现 `MaxRuntime`。
- 建议后续单独处理:
  1. 分析 analyst worker 为什么没有稳定产出 `analysis.done`。
  2. 检查 worker prompt、gate timeout、read-only tool noise 和失败状态回写。
  3. 不要把该问题和 topology spawn redelivery 混为一谈。
- 完成条件:
  - parent-visible 三实例 dogfood 能自然收敛到 completion candidate 或明确的失败诊断。
  - record-session 中保留 `topology.spawn_group`、3 条 direct delivery、3 条 worker result 或结构化 failed evidence。

## [2026-05-20 08:05:00] [Session ID: omx-1779158263949-kticiv] 后续建议: task-derived dynamic hat identity / role contract

- 背景: 当前 `topology.spawn_group` 的 `role` 是运行时标签,目标 hat 仍来自已有配置。临时角色默认不写入 `.ralph/agents.json` 一等字段。
- 建议后续继续设计:
  1. task-derived dynamic hat identity。
  2. role contract schema。
  3. prompt isolation 和 agents snapshot provenance。
  4. 真实 E2E dogfood,证明 LLM 运行时创建的角色不会污染 worker prompt。
- 完成条件:
  - 用户要求新角色时,record/evidence 能直接说明它是固定 role、临时 role,还是无法物化的请求。

## [2026-05-20 08:05:00] [Session ID: omx-1779158263949-kticiv] 后续建议: live Codex multi-agent collaboration E2E

- 背景: `multi_agent_collab_evidence` 支线只证明了 parallel hat instances 的 runtime 入口、focused tests 和 E2E scenario registration。
- 建议后续如要证明真实模型协作稳定性,单独跑 live Codex E2E:
  - `cargo run -p ralph-e2e -- codex --filter parallel-hat-instances --keep-workspace --verbose`
  - 或针对 trigger routing / human approval / spawn instance 的具体场景分别跑。
- 完成条件:
  - `.e2e-tests/report.md` / `report.json` 和 record-session 能证明真实 backend 下的 topic、delivery、completion 都收敛。

## [2026-05-20 19:03:00] [Session ID: omx-1779158263949-kticiv] 后续计划: 3-worker live dogfood 产出的候选任务

### 来源
- `/tmp/ralph-topology-dogfood-bounded-180-rerun-20260520-185717.jsonl`
- 3 条 `analysis.done`:
  - `analyst#2`: 功能补充
  - `analyst#3`: 功能完善
  - `analyst#4`: review

### 候选任务
1. 补 topology/capability evidence inspect 能力:
   - 在 `ralph record summary` 或新 inspect 命令中展示 parent-visible dynamic instances、child-run projection、parent_topology_unchanged、fixed_role 和 last_input。
   - 目标是让用户不用手写 jq 也能确认“真实例是否跑起来”。
2. 补 parent-visible spawn replay/integration guardrail:
   - 断言 `topology.spawn_group` 创建 `.ralph/agents.json` dynamic instances。
   - 断言 `topology.spawn.result` 后不再重复 publish 原 delivery topic。
3. 补 TUI/plain 显示验收:
   - 父级实例列表显示 dynamic hat instance。
   - child-run 状态以 footer/status 或实例栏 children count 形式可观测。
   - output frame 给 act 状态预留底部空间,避免遮挡输出。
4. 单独评估 Claude stream-json adapter capability negotiation:
   - 不混入当前 parent-visible spawn 收尾。
   - 需要独立 fixture、parser 单测、smoke_runner 和 CLI integration gates。

### 暂不执行原因
- 当前用户要求是先看 live dogfood 结果有没有用。
- 这些内容需要进一步拆 spec 或 code task 后再进入实现。

## [2026-05-21 07:25:47] [Session ID: omx-1779158263949-kticiv] 后续计划更新: topology/capability evidence inspect 已完成

### 已完成
- `ralph record summary` 已新增 `Evidence Inspect` section。
- 覆盖 topology、agents snapshot、child-runs、capability events、result topics、termination。
- 真实 dogfood record 已验证能看到 `analyst#2/#3/#4` dynamic parent-visible instances、`analysis.done=3` 和 `CompletionPromise`。

### 仍保留的后续项
- parent-visible spawn replay/integration guardrail 仍值得做。
- TUI/plain 显示验收仍值得做。
- Claude stream-json adapter capability negotiation 仍应单独立项。

## [2026-05-21 07:38:00] [Session ID: omx-1779158263949-kticiv] 后续计划更新: TUI/plain 显示验收已完成

### 已完成
- `parallel no-tui/plain` 已能显示 topology/capability 控制面事件摘要。
- TUI 现有 footer / instances / output status / bottom reserved rows focused tests 已重新验证通过。
- `specs/unified-evidence-inspect.spec.md` 已补充 display guardrails。

### 仍保留的后续项
- parent-visible spawn replay/integration guardrail 仍值得做。
- Claude stream-json adapter capability negotiation 仍应单独立项。

## [2026-05-21 08:10:00] [Session ID: omx-1779158263949-kticiv] 后续计划更新: parent-visible spawn replay/integration guardrail 已完成

### 已完成
- 新增 CLI integration guardrail,真实运行 `ralph run --no-tui --record-session`。
- 已证明 `topology.spawn_group` 会创建 parent-visible dynamic builder instances。
- 已证明 `topology.spawn.result` 后不会追加 redeliver 原始 `build.task`。
- 已证明 `.ralph/agents.json`、`.ralph/events.jsonl` 和 `record summary --agents-file` 能组成完整证据链。

### 仍保留的后续项
- Claude stream-json adapter capability negotiation 仍应单独立项。
- task-derived dynamic hat identity / role contract 仍是后续架构设计项。

## [2026-05-21 19:03:10] [Session ID: omx-1779158263949-kticiv] 后续计划: 实现 task-derived dynamic hat identity / role contract

### 后续事项
- 按 .omx/plans/task-derived-dynamic-hat-identity-role-contract.md 实现代码。
- 建议优先使用 `` 或 `` 执行,因为该任务跨 runtime、prompt、evidence 和测试。

### 关键验收提醒
- 旧 topology.spawn_group payload 必须兼容。
- EffectiveRoleContract 必须是 downstream 唯一 contract truth source。
- worker prompt 不得继承 coordinator-only prompt。
- agents snapshot 只写 summary/hash/source id,不写完整 contract/prompt。
- 最终需要 live dogfood + `ralph record summary --agents-file .ralph/agents.json` 证明。

## [2026-05-21 21:02:00] [Session ID: omx-1779158263949-kticiv] 后续计划更新: task-derived role contract 已完成,保留 live dogfood 稳定性问题

### 已完成
- task-derived dynamic hat identity / role contract 已落地。
- `topology.spawn_group.instances[].role_contract` 已作为 raw hint 接入。
- runtime canonical `EffectiveRoleContract` 已成为 downstream prompt / agents snapshot / record summary / TUI/plain display 的唯一真相源。
- 旧 payload 兼容、conflict fail-closed、output allowlist、prompt isolation、summary-only agents snapshot 等 guardrails 已有 focused/integration tests。

### 仍保留的后续项
1. live 3-worker dogfood 稳定收敛:
   - 本轮 420 秒 run 是 `Interrupted`,不是自然 completion。
   - 需要单独分析 worker 长耗时、stderr/stdout event parsing、confessor 干预和结果汇总时机。
2. dogfood workspace/artifact policy:
   - worker 在“不要改代码”任务中仍写入 `ralph/log/builder#*/...` 和 `.agent/memories.md`。
   - 需要明确 read-only dogfood 是否允许写 evidence artifact,以及这些 artifact 是否应隔离到临时 workspace。
3. `analysis.done` strict dogfood:
   - 当前 `ralph.yml` builder publishes 是 `build.done/build.blocked`,所以 runtime 正确阻止了 `analysis.done` 越权。
   - 若要严格验证 `analysis.done`,应使用专门 dogfood config 或目标 hat publishes 包含 `analysis.done`。

### 可删除/视为完成的旧项
- `task-derived dynamic hat identity / role contract` 作为实现任务已完成。
- 后续只保留 live dogfood 稳定性和 artifact policy 这两个新问题。

## [2026-05-22 12:09:14] [Session ID: omx-1779158263949-kticiv] 后续计划: agents snapshot 应区分当前 registry 与历史 spawned dynamic instances

### 发现
- clean live dogfood 已自然收敛,record-session 显示 `builder#2/#3/#4` 均发布 `analysis.done`。
- 但 `.ralph/agents.json` 最终只展示当前 registry 中尚未被回收的实例,最早完成并被 dynamic idle TTL 回收的 `builder#4` 不在 snapshot 中。

### 为什么后续值得做
- 用户希望 parent-visible / parent-observable 的实例状态容易确认。
- 如果 summary 里的 Agents Snapshot 少了已完成但已回收的动态实例,用户容易误判为实例没跑起来。

### 建议方向
1. 在 record summary 的 Evidence Inspect 中,把 topology.spawn.result 和 Result Topics 作为历史真相源,Agents Snapshot 标注为 current registry sidecar。
2. 或在 `.ralph/agents.json` 增加 completed/tombstone dynamic instances,至少保留 `instance_id`, `role`, `role_contract_summary`, `final_state`, `last_input`, `completed_at`。
3. 或在 run shutdown 前写一次 final agents snapshot,但要明确是否包含已 unregister 的 dynamic instances。若不保留 tombstone,final snapshot 仍不能表达完整历史。

### 当前不做原因
- 本轮用户要求是给 live dogfood 一个 clean config 并验证自然收敛。
- 该问题属于观测面语义增强,需要单独 spec / guardrail,不应混入本轮临时 dogfood 配置。

## [2026-05-22 12:11:27] [Session ID: omx-1779158263949-kticiv] 后续计划更新: live 3-worker dogfood 稳定收敛已完成

### 已完成
- 已用 clean dogfood config 验证 3-worker 自然收敛。
- 已证明 `analysis.done: 3 source_instances=builder#2,builder#3,builder#4`。
- 已证明 `Termination.reason=CompletionPromise`。
- 已证明本轮没有走 `capability.request` isolated child-run path。

### 仍保留
- dogfood workspace/artifact policy 仍值得单独处理。
- agents snapshot 对已回收 dynamic instances 的历史表达仍值得单独处理。

## [2026-05-22 14:25:19] [Session ID: omx-1779158263949-kticiv] 后续计划更新: agents snapshot completed dynamic tombstone 已完成

### 已完成
- agents snapshot 现在区分 current registry 与 `completed_dynamic_instances` tombstone。
- Evidence Inspect 和 `ralph agents` 均能显示 completed dynamic instances。
- 原先关于“已回收 dynamic instance 从 `.ralph/agents.json` 消失导致误判”的后续项已经落地。

### 仍保留
- dogfood workspace/artifact policy 仍可单独立项。
- live dogfood 稳定性如果继续加强,可基于 clean config 模式另起任务。

## [2026-05-22 15:29:08] [Session ID: omx-1779158263949-kticiv] 后续计划: reply.human.message 多行 event 未进入 Result Topics

### 发现
- clean 3-worker live dogfood 中,coordinator 最终输出了多行 `reply.human.message` XML event。
- record-session 中可以看到 stdout 写入,但 `record summary` 的 `Result Topics` 未列出 `reply.human.message`。

### 后续建议
- 如果产品契约要求最终人类摘要也进入 durable bus.publish / Result Topics,应做其中一种:
  1. Prompt 约束 final `reply.human.message` 必须单行 XML event。
  2. Runtime parser 支持多行 XML event 聚合解析。
  3. Evidence Inspect 把 stdout-only reply 与 bus.publish reply 区分展示。


## [2026-05-22 15:32:09] [Session ID: omx-1779158263949-kticiv] 后续计划确认: reply.human.message 多行 event 未进入 Result Topics

### 发现
- clean 3-worker live dogfood 中,coordinator 最终输出了多行 `reply.human.message` XML event。
- record-session 中可以看到 stdout 写入,但 `record summary` 的 `Result Topics` 未列出 `reply.human.message`。

### 后续建议
- 如果产品契约要求最终人类摘要也进入 durable bus.publish / Result Topics,应做其中一种:
  1. Prompt 约束 final `reply.human.message` 必须单行 XML event。
  2. Runtime parser 支持多行 XML event 聚合解析。
  3. Evidence Inspect 把 stdout-only reply 与 bus.publish reply 区分展示。


## [2026-05-22 19:23:13] [Session ID: omx-1779158263949-kticiv] 后续计划: 六文件 notes / ERRORFIX 续档 continuous-learning

### 发现
- 本轮收尾时发现 `notes.md` 已超过 1000 行。
- `ERRORFIX.md` 正好 1000 行,本次又是 bug fix,继续追加会触发续档要求。

### 为什么暂不在本轮执行
- 当前主任务是 runtime multi-line event parsing 修复,代码与验证已经闭环。
- 六文件归档必须先执行 continuous-learning 摘要与索引判断,不能未经学习就把文件移入 archive。
- 当前系统约束不允许随意开启后台子智能体,所以先记录为后续清理任务。

### 建议后续动作
- 用 continuous-learning 对默认六文件做一次续档整理。
- 重点处理 `notes.md` 与 `ERRORFIX.md`。
- 归档前先提炼可复用经验,再决定是否同步到 `EXPERIENCE.md` 或项目索引。


## [2026-05-22 20:26:54] [Session ID: omx-1779158263949-kticiv] 后续计划更新: reply.human.message 多行 event durable 记录已验证完成

### 已完成
- 本次 clean 3-worker live dogfood 已确认 multi-line `reply.human.message` 进入 `record summary Result Topics`。
- summary 显示 `reply.human.message: 1 source_instances=ralph#1`。
- 原始 record-session `bus.publish` 统计也显示 `reply.human.message: 1 sources=['ralph#1']`。

### 可视为解决的旧后续项
- `reply.human.message 多行 event 未进入 Result Topics` 已通过 runtime observer-only drain + live dogfood 验证闭环。

### 仍可继续做但不阻塞本项
- 将 clean 3-worker dogfood 固化成 repo-local fixture / script,减少未来依赖 `/tmp` 临时配置。
- dogfood workspace/artifact policy 仍值得单独立项。

## [2026-05-23 16:45:00] [Session ID: omx-1779158263949-kticiv] 后续计划: dynamic hats dogfood 推荐主线

### 来源
- 用户自然语言 prompt live dogfood: `/tmp/ralph-dynamic-evolution-angle-dogfood-20260523-151612.jsonl`。
- 最终 reply 推荐主线: `clean-current-runtime-evidence-and-dynamic-role-contract`。

### 建议后续动作
- 优先生成/推进一个 OpenSpec 主线,范围包括:
  - runtime protocol SSOT。
  - dynamic role contract evidence。
  - topology.spawn_group partial/tombstone 语义。
  - record-session/evidence inspect correlation。
  - parallel runtime release gate。
- 延期但保留:
  - `tui-mdfried-viewer` 作为 UX 线。
  - `agent-cli-recoverable-failure-retry` 作为可靠性线。
  - manifest schema v2 作为治理增强线。

### 注意
- 本计划来自 dogfood 分析结果,尚未开始实现。

## [2026-05-23 17:24:00] [Session ID: omx-1779158263949-kticiv] 后续计划更新: dynamic hats dogfood 推荐主线已转为 OpenSpec

### 已完成
- 已创建并验证 `openspec/changes/clean-current-runtime-evidence-and-dynamic-role-contract/`。
- `proposal`, `design`, `specs`, `tasks` 均已完成。
- 单项验证 `openspec validate clean-current-runtime-evidence-and-dynamic-role-contract --type change --strict` 已通过。

### 后续仍待
- 该 OpenSpec 还没有实现代码。
- 下一步若继续,应按 `tasks.md` 从 runtime protocol SSOT 与 prompt boundary tests 开始。
- 原延期线仍保留: `tui-mdfried-viewer`, `agent-cli-recoverable-failure-retry`, manifest schema v2。

## [2026-05-25 10:47:10] [Session ID: omx-1779158263949-kticiv] 后续计划: 全量 OpenSpec strict 被无 delta 的 retry change 阻断

### 来源
- 本轮 3.x 验证时额外运行 `openspec validate --all --strict`。

### 现象
- 当前 change `clean-current-runtime-evidence-and-dynamic-role-contract` 单项 strict 校验通过。
- 全量 strict 失败在 unrelated active change: `agent-cli-recoverable-failure-retry`。
- 失败原因是该 change 目前没有 specs delta: `Change must have at least one delta. No deltas found.`

### 为什么本轮不处理
- 用户已明确本轮不扩 retry / recoverable CLI failure 分支。
- 直接修 `agent-cli-recoverable-failure-retry` 会污染当前 runtime evidence 主线。

### 后续建议
- 等用户重新切到 retry 可靠性线时,补齐 `openspec/changes/agent-cli-recoverable-failure-retry/specs/**/spec.md` delta。
- 在 retry change 补齐前,当前 change 的有效 gate 以单项 `openspec validate clean-current-runtime-evidence-and-dynamic-role-contract --type change --strict` 为准。

## [2026-05-25 13:05:20] [Session ID: omx-1779158263949-kticiv] 后续计划: record_session.rs 可拆分 aggregate 与 renderer

### 来源
- 本轮 4.x record summary/evidence correlation 修改集中在 `crates/ralph-cli/src/record_session.rs`。

### 现象
- `record_session.rs` 已超过 1000 行,同时包含 strict parse、aggregate、Evidence Inspect render、record-session pointer helper 和 tests。
- 本轮为了不扩散主线,只做了局部改良,没有拆文件。

### 后续建议
- 在后续专门的整理任务中,可以将该文件拆成:
  - `record_session/aggregate.rs`
  - `record_session/evidence_render.rs`
  - `record_session/pointer.rs`
- 拆分时保持 public API 不变,优先用现有 tests 做回归护栏。

## [2026-05-26 00:28:00] [Session ID: omx-1779158263949-kticiv] 后续计划更新: dynamic hats runtime evidence 主线已完成并归档

### 已完成
- `clean-current-runtime-evidence-and-dynamic-role-contract` 已完成实现、验证、主规格同步和 OpenSpec 归档。
- 归档位置: `openspec/changes/archive/2026-05-26-clean-current-runtime-evidence-and-dynamic-role-contract/`。
- 这覆盖了此前 `dynamic hats dogfood 推荐主线` 中的 runtime protocol SSOT、dynamic role contract evidence、topology.spawn_group partial/tombstone、record-session/evidence inspect correlation、parallel runtime release gate。

### 仍保留的延期线
- `agent-cli-recoverable-failure-retry`: 当前仍是 active change,但没有 delta,会阻断 `openspec validate --all --strict`。
- `tui-mdfried-viewer`: 仍是独立 UX 线,未在本次处理。
- manifest schema v2: 仍是治理增强线,未在本次处理。

## [2026-05-28 16:57:37] [Session ID: omx-1779954714247-oab9zc] 后续计划更新: agent-cli-recoverable-failure-retry 已完成,不再阻断全量 OpenSpec strict

### 已完成
- `agent-cli-recoverable-failure-retry` 已完成实现,`tasks.md` 为 34/34 complete。
- `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate agent-cli-recoverable-failure-retry --type change --strict` 已通过。
- `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate --all --strict` 已通过,28 passed,0 failed。

### 可视为解决的旧后续项
- 2026-05-25 记录的“全量 OpenSpec strict 被无 delta 的 retry change 阻断”已经失效。
- 2026-05-26 记录的“`agent-cli-recoverable-failure-retry` 当前仍是 active change,但没有 delta,会阻断 `openspec validate --all --strict`”已经失效。

### 仍保留
- `tui-mdfried-viewer` 仍是独立 UX 线。
- manifest schema v2 仍是治理增强线。
- `record_session.rs` 拆分 aggregate / renderer 仍可作为后续整理任务。

## [2026-05-29 17:06:44] [Session ID: native-codex-20260529] 后续计划: example PROMPT.md fixture 应进入 Git 真相源

### 来源
- recoverable retry staged-only 全量 `cargo test --quiet` 验证。

### 现象
- `integration_examples` 要求多个 `examples/parallel-*/PROMPT.md` 存在。
- 当前主工作区确实有这些文件,但 `git ls-files examples` 显示其中 24 个 `PROMPT.md` 未被 Git 跟踪。
- 因此从 HEAD 创建的干净 worktree 会缺 fixture,导致 full cargo test 失败。

### 后续建议
- 单独开一个 fixture/governance 小任务,决定这些 `PROMPT.md` 应该正式 tracked,还是测试应改为只扫描 tracked/runnable examples。
- 不建议混入 recoverable retry commit,否则会扩大提交边界。

## [2026-05-29 17:32:21] [Session ID: native-codex-20260529] 后续计划: 默认 task_plan.md 已超过 1000 行,需单独续档

### 来源
- 第六次 hook 预检显示 `task_plan.md` 为 1548 行。

### 现象
- 项目规则要求六文件超过 1000 行后做续档,并配合 continuous-learning。
- 当前 recoverable retry staged patch 已经收敛,不适合把上下文续档混入同一个 scoped commit。

### 后续建议
- 在 recoverable retry scoped commit 之后,单独执行上下文续档/continuous-learning 任务。
- 续档时只处理六文件上下文,不要混入 runtime/recoverable 代码改动。


## [2026-05-29 18:02:02] [Session ID: omx-1779004640353-blcixq] 后续计划更新: continuous-learning 续档与 evolution_analysis 承接

### 已完成
- `task_plan.md` 超过 1000 行的延期项已在本轮 continuous-learning 中处理: 已生成摘要,并准备续档到 `archive/default_history/`。
- `evolution_analysis` 支线已完成总结,准备归档到 `archive/branch_contexts/evolution_analysis/`。

### 仍应保留的后续项
- example `PROMPT.md` fixture 真相源仍未处理: 需要决定未跟踪 prompt fixtures 是正式 tracked,还是测试只扫描 tracked/runnable examples。
- 大文件拆分仍值得做,但应单独开任务并保持 public API 不变:
  - `crates/ralph-cli/src/record_session.rs`: aggregate / evidence_render / pointer。
  - `crates/ralph-core/src/parallel/instance.rs`: retry runtime / workspace lifecycle / prompt build / result handling。
  - `crates/ralph-core/src/parallel/supervisor.rs`: agents snapshot / completion gate / recoverable map / topology/capability runtime。
  - `crates/ralph-tui/src/app.rs`: layout / hit-test / clipboard / action dispatch / run loop。
- `tui-mdfried-viewer` 需要先做 spec-code reconciliation,不要直接假设 tasks 中已勾选的 Big Headers / `ratatui-image` 已经存在。
- 旧 docs tree 的 legacy/archived 边界和搜索污染仍值得治理。
- runtime/evidence release-fast gate 仍值得固化成脚本或 task runner。

### 当前不立即实施原因
- 本轮目标是持续学习、续档和经验沉淀。
- 当前 worktree 仍有大量其它支线改动,不适合把后续工程混入同一轮上下文整理。


## [2026-05-29 18:57:29] [Session ID: omx-1779004640353-blcixq] 后续计划更新: example PROMPT.md fixture 真相源已处理

### 已完成
- 已确认 runnable parallel examples 的 `PROMPT.md` 应作为 committed templates 纳入 Git 真相源。
- 已更新 `.gitignore` 为 `!examples/parallel-*/PROMPT.md`。
- 已 staged 24 个 `examples/parallel-*/PROMPT.md`。
- staged-only clean worktree full `cargo test --quiet` 已通过。

### 后续仍可考虑
- 如果未来新增新的非 parallel example 且也需要 committed `PROMPT.md`,应显式扩展 `.gitignore` 例外,不要依赖 `git add -f`。
- 当前不需要修改 `integration_examples.rs`,因为它的 self-contained example 契约是正确的。

## [2026-08-01 11:40:00] [Session ID: omx-1785579233065-awidzo] 备忘: 架构报告其余候选 + flaky 测试

- 候选2(Strong): CLI 运行时(46k 行)收进 core — 杠杆最大,建议单独立项: codex_app_server_session/parallel_runner/autopilot 的运行时外壳推进 core,CLI 只剩命令面。
- 候选3(Worth exploring): TUI 领域切片 — app.rs 3968 行 + TuiState 50+ pub 方法,按 radar/output/task/chat 切片。
- 候选4(Worth exploring): Evidence 深模块 — JSONL 知识横跨 8+ 文件,find_file_in_parents 3 处。
- 候选5(Worth exploring): EventLoop interface 收窄(25+ pub 方法,驱动知识劈成 core/cli 两半)。
- 候选6(Speculative): e2e 场景声明化(35k 行 harness)。
- flaky 测试: `integration_record_session::sigint_leaves_record_session_parseable_and_writes_termination_and_pointer` 在并发跑时偶发失败(单独跑通过),SIGINT 时序竞态,值得后续加固。

## [2026-08-04 01:30:00] [Session ID: omx-1785634382266-fz89ur] 候选6 收尾完成

- [x] 已用 minimax profile 补验: 24/24 example 场景 live 通过(见 task_plan 2026-08-03 23:55)
- [x] 核心 parallel 场景 minimax 补验: emit-spawn / starting ×2 / steer-live ×2 / idle-start-live / hat-instances-zh 全过
- [x] fake shim ×2 评估结论: 300 行 python 嵌入 YAML 无收益(可读性/维护性差),
      live 版已声明化, fake 版保留命令式作确定性回归
- [x] {profile_args} 缩进 bug 修复(见 task_plan 2026-08-04 01:30)
- [x] 候选6 全部完成, 本条目关闭

## [2026-08-13 15:05:00] [Session ID: omx-1786600320381-z290x9] Schema 扩展:backend-unavailable 类型 scenarios 需要 3 个新字段

### 来源
- Wave 2 任务 2.1.3 迁移调研发现 `BackendUnavailableScenario`(OpenSpec §2.1.3 列为 Easy,
  但 audit-p5-p1.md:73 明确指出需要 schema 扩展,OpenSpec tasks.md 分类与 audit 不一致)。

### 阻塞点
- 命令式 3 条断言在 declarative schema 中没有对应字段:
  1. `execution_failed` = `exit_code != Some(0)` —— schema 只有 `exit_code_success_or_limit: bool`
     (matches!(Some(0|2))),语义相反,反设会破坏等价。
  2. `error_mentions_backend` = stderr + stdout 低位化后含任一关键词
     (not found / command not found / no such file / cannot find / nonexistent / backend / cli)
     —— schema 只有 `output_contains`(stdout only)和 `output_contains_any`(stdout only)。
  3. `failed_fast` = `duration < 20s` —— schema 无 `duration_within_secs` 字段。
- 此外,命令式 setup 设 `cli.command: nonexistent-cli-...` 试图触发 backend spawn 失败,
  但 `crates/ralph-core/src/config.rs:795-803` 显示 `cli.command` 只在 `cli.backend == "custom"`
  时生效;当 `backend: claude|kiro|opencode` 时,ralph 用 `self.adapters.<backend>` 跑真 CLI,
  `command` 字段被静默忽略。这意味着命令式 test 即便跑 live scenario 也不一定真触发
  "backend unavailable" —— 这本身就是个待澄清的语义问题。
- 真正的修法(audit 建议):schema 加 `require_backend: <wrong>` 字段,显式声明"该 scenario
  期望 ralph 报 backend 不可用",由 declarative runner 构造一个不可能成功的 backend 路径,
  而不是依赖 `cli.command` 这个被忽略的字段。

### 后续动作(本轮不做)
- 在 OpenSpec 新增 delta spec(例如 `e2e-declarative-schema-extension-v1`)或在
  `e2e-declarative-coverage-gate` 里追加 §2.6 "schema extension" 段,显式列出 3 个新字段:
  - `expect.stderr_contains: Vec<String>`
  - `expect.stderr_contains_any: Vec<Vec<String>>`
  - `expect.exit_code_nonzero: bool`
  - `expect.failed_within_secs: u64`
- schema 实现 + 单元测试 + registry 重新接受 2.1.3 + 跑 live 验证
- 完成后回头迁移 §2.1.3(估计需要重写命令式 setup,确认 audit 建议的 `require_backend` 语义)

### 临时决策
- 本轮跳过 2.1.3(不迁移、不动 lib.rs),直接进 §2.1.4 AuthFailureScenario 迁移。
- OpenSpec tasks.md 暂不改(避免触动已绿 29/29 validate),等 schema 扩展 spec 起时一并改。

## [2026-08-13 15:28:00] [Session ID: omx-1786600320381-z290x9] ~~Wave 2 收官后:执行 continuous-learning 流程~~ ✅ DONE (commit a7daa79)

- 完成时间: 2026-08-13 17:15
- 完成内容:
  - 2 个新 self-learning skills (yaml-schema-or-vs-and-semantics, yaml-duplicate-field-bug)
  - 4 个新 EXPERIENCE.md 条目 (exp-20260813-yaml-schema-or-vs-and-semantics,
    yaml-duplicate-field-detection, schema-cost-vs-assertion-value,
    audit-classification-reality-check)
  - docs/solutions/declarative-scenario-migration.md (160 行综合迁移指南)
  - AGENTS.md Project Knowledge Index 增加 3 个新条目
  - WORKLOG__2026-08-13_pre_section_2_2_4_migrations.md 归档到
    archive/branch_contexts/wave2_e2e_declarative_migration/
- 验证: cargo test -p ralph-e2e --lib 536 passed; gate test Coverage 100.00% PASS
- 未做: 命令式 cli.command 静默忽略 (2.1.3 命令式本身的语义问题) 仍待后续修复,
  见下一条目

## [2026-08-13 17:25:00] [Session ID: omx-1786600320381-z290x9] ~~$continuous-learning 完整复盘 (root 上下文整理)~~ ✅ DONE (本轮 chore commit)

- 完成时间: 2026-08-13 17:25
- 完成内容:
  - 6 文件活跃度分析 (task_plan/WORKLOG/EPIPHANY/LATER_PLANS = current; ERRORFIX/notes = 历史)
  - 0 新 Compound Capture (距上次 CL 仅 LATER_PLANS 标记更新)
  - Scoped Refresh: docs/solutions/declarative-scenario-migration.md 重构
    (移到 documentation-gaps/ 子目录 + 加 11 必填 frontmatter 字段)
  - AGENTS.md 索引路径同步
  - 验证: validate-solution-frontmatter.py OK + validate-solution-claims.py OK
    (0 flags) + cargo test 536 passed + gate 100% PASS
  - 0 归档 (无文件达 1000 行, 当前 session 仍 active)
- 未做: 命令式 cli.command 静默忽略 (2.1.3 命令式本身的语义问题) 仍待后续修复
- 详情: 见 task_plan.md / WORKLOG.md 最新 entries

## [2026-08-13 17:40:00] [Session ID: omx-1786600320381-z290x9] ~~整理清理根目录分支上下文文件~~ ✅ DONE (本轮 chore commit)

- 完成时间: 2026-08-13 17:40
- 完成内容:
  - 4 个 notes__*.md (branch_diff_review / clean_events_review / e2e_conv / group1_dryrun)
    移到 archive/branch_contexts/<suffix>/, 0 引用 + 异 Session + 不同主题归档
  - archive/manifests/ARCHIVE_MANIFEST__sync_origin_main_2026-08-13.md (103 行)
  - EXPERIENCE.md 加 exp-20260813-e2e-live-convergence-issue (inbox 路线)
- 验证: cargo test 536 passed; gate 100% PASS
- 详情: 见 task_plan.md / WORKLOG.md 最新 entries

## [2026-08-13 17:40:00] [Session ID: omx-1786600320381-z290x9] e2e-live-convergence 诊断 — 解 exp-20260813-e2e-live-convergence-issue

### 来源
- EXP-20260813-e2e-live-convergence-issue (EXPERIENCE.md): 3 个 live 场景失败模式
  (termination_reason=None, 事件流完整但无 loop.terminate); 根因未知, 留 Wave 3 期间
  诊断

### 待执行
- 抓 `human-log.md` 看协调者最后输出 (是什么阻止 LOOP_COMPLETE)
- 抓 `agents.json` 看 ralph#1 状态转换 (是否进入 Running 后未回到 Idle)
- 减少 max_iterations 看是否 early termination (排除 max_runtime 提前收掉)
- 对比 declarative 版本 vs live 版本行为差异 (declarative 跑通, live 仍 fail)
- 定位后:
  1. 修命令式或 ralph app-server runtime
  2. 把 exp-20260813-e2e-live-convergence-issue 升级到 docs/solutions/ formal capture
     (problem_type: runtime_error 或新增 live_convergence 类型)
  3. 加 Wave 3 验证 checklist

### 触发条件
- Wave 3 closure (#[deprecated] + OpenSpec archive) 期间执行
- 或用户显式调用 "$continuous-learning 解 e2e-live-convergence-issue" 时执行
