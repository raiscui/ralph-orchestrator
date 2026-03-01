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
