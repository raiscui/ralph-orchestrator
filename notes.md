# 笔记(续档)

- 说明: 旧 `notes.md` 已超过 1000 行,已续档为 `archive/notes_2026-02-28_1706.md`。
- 新笔记从这里开始记录。

## 2026-02-28 17:17 +0800 | continuous-learning: 四文件摘要(因 task_plan 续档触发)

## 四文件摘要(用于决定是否提取 skill)

- 任务目标(task_plan):
  - 续档 `task_plan.md`(超过 1000 行)并保持根目录“低噪音、可检索”。
  - 回答你关于 openclaw 启发与 macOS Xcode license 的疑问,避免把环境问题误归因到 `cargo test`。
- 关键决定(task_plan):
  - 续档: `task_plan.md` -> `archive/task_plan_2026-02-28_1717.md`,新建 `task_plan.md`。
  - 归档: 在完成四文件摘要后,把本次检索覆盖到的历史版本移动到 `archive/`(减少根目录噪音)。
- 关键发现(notes):
  - openclaw 的启发点已经被提炼并落地为多项工程机制:
    - guardrails(可执行护栏进入 CI)
    - doctor/wizard(把常见失败变成可修复路径)
    - lanes + generation + draining(并发与退出收尾的确定性)
    - no-output watchdog(把“卡住无输出”变成一等失败原因)
    - context window guard(把上下文窗不足变成可执行预检)
  - macOS Xcode license 的根因是 Apple toolchain(xcrun/xcode-select)的选择,不是 cargo 自身依赖。
- 实际变更(WORKLOG):
  - 已在更早的实现中补齐:
    - `.envrc` 默认优先使用 Command Line Tools(`DEVELOPER_DIR=/Library/Developer/CommandLineTools`)来绕过 Xcode.app license。
    - `DEVELOPMENT.md` 增加 macOS troubleshooting 小节,避免后续误判。
- 错误与根因(ERRORFIX,如有):
  - 历史上 `cargo test` 失败(exit 69)的根因是 `xcode-select -p` 指向未接受 license 的 Xcode.app,导致 `xcrun` 拒绝执行。
- 可复用点候选(1-3 条):
  1. 续档规则 + 归档流程要“强制化”(否则根目录噪音会持续增长)。
  2. macOS toolchain 选择(`DEVELOPER_DIR`)应当作为“可执行默认”,而不是只写在聊天里。
  3. 把 openclaw 的“机制”而非“口号”迁移到 Rust(guardrails/lanes/doctor)是高 ROI 路线。
- 是否需要固化到 docs/specs: 否(已固化到 `.envrc`/`DEVELOPMENT.md`/`specs/*`/项目级 skills)。
- 是否提取/更新 skill: 否(已存在对应的 `self-learning.*` skills,且本次没有新增更通用的新模式)。

## 2026-02-28 18:31 +0800 | 研究: zeroclaw-labs/zeroclaw 值得借鉴点(映射到 ralph)

## 来源(本地 clone)

- /tmp/zeroclaw-20260228-1832

## 我认为最有启发的设计点(带原文/代码证据)

1. Trait 驱动 + 把扩展点写死成契约
   - README.md 直接把定位写成: "runtime operating system".
   - README.md 用一句话把卖点钉死: "Trait-driven architecture · secure-by-default runtime · provider/channel/tool swappable · pluggable everything".
   - AGENTS.md 直接列出 "Key extension points"(Provider/Channel/Tool/Memory/Observer/RuntimeAdapter/...).
   - 启发:
     - 扩展点不要藏在脑子里.
     - 用 trait + factory 注册点,把贡献路径固定下来.

2. 工具系统: Tool trait + JSON schema + runtime 能力门控 + SecurityPolicy 注入
   - src/tools/mod.rs 写得很明确:
     - tool 需要 name/description/JSON schema/execute.
     - "Security policy enforcement is injected via SecurityPolicy at construction time".
   - src/runtime/traits.rs 用 has_shell_access/has_filesystem_access/supports_long_running 来声明能力.
   - 启发:
     - "能不能用"由 runtime 能力声明决定.
     - "允不允许用"由 security policy 决定.
     - 这两层分开后,很多 if/else 会自然消失.

3. ApprovalManager: supervised 工具调用审批,带 session allowlist + audit log + 非 CLI 渠道 pending approvals
   - src/approval/mod.rs 顶部注释: "session-scoped 'Always' allowlists and audit logging".
   - needs_approval() 的优先级清晰:
     - autonomy_level(full/read_only) -> always_ask -> auto_approve -> session_allowlist -> default supervised.
   - 启发:
     - "工具审批"要有可审计证据(审计日志).
     - "Always" 这种临时放权,要显式建模成 session allowlist.
     - 非 CLI 渠道(telegram/slack)不能阻塞式 prompt,就要做 pending request + 显式确认.

4. Doctor: 结构化诊断结果 + 人类可读报告,并且能做错误分类
   - src/doctor/mod.rs: "Structured diagnostic result for programmatic consumption (web dashboard, API)."
   - check_config_semantics() 把配置问题分为 ok/warn/error,并给出可执行的 message.
   - model probe 里把常见 401/403/429 归类为 auth/access,而不是一律 error.
   - 启发:
     - doctor 不只是"检查",而是"把常见失败变成可修复路径".
     - 结构化结果(Vec<DiagResult>)比纯 stdout 更利于后续 TUI/CI/网页复用.

5. 安全: PromptGuard(快速特征匹配) + E-stop(一等公民,并且 fail-closed)
   - src/security/prompt_guard.rs: Aho-Corasick 多模式匹配 + regex 分类,输出 Safe/Suspicious/Blocked.
   - src/security/estop.rs: state file 解析失败会进入 fail-closed(kill_all=true),并落盘.
   - 启发:
     - "注入检测"要做到可配置(action+sensitivity),并且输出可解释的 patterns.
     - "紧急刹车"要先保证安全(失败即关闭),再谈易用.

6. Sandboxing: Sandbox trait + backend 实现,并且文档明确区分 proposal vs current
   - docs/sandboxing.md 头部明确标注: "Status: Proposal / Roadmap".
   - src/security/firejail.rs 以 wrap_command 方式做最小侵入的命令封装,并用单测锁死 flags.
   - 启发:
     - 先从"命令包装"切入,比一开始做复杂容器编排更容易落地.
     - proposal 文档必须显式标识,避免读者把未来设想当成现状.

7. 供应链与合规: cargo-deny 的 deny.toml
   - deny.toml 里把 unknown-registry/unknown-git 设为 deny.
   - licenses 用 allowlist,并且 ignore 条目都写 reason.
   - 启发:
     - 依赖治理要可审计.
     - ignore 不是"忽略",而是"带理由的延期偿还".

8. 文档信息架构(IA): docs hub + 决策树 + 按受众分类
   - docs/README.md 用表格回答 "I want to...".
   - docs/security/README.md 把文档分成 "Current-Behavior" 和 "Proposal/Roadmap".
   - 启发:
     - 文档是产品的一部分.
     - 对新手: 决策树.
     - 对维护者: runbook.
     - 对安全: 现状和设想分层.

## 对 ralph 的可迁移清单(初稿,按 ROI 排序)

- 低成本,高收益(建议优先考虑):
  - 在 docs 入口增加 "10 秒决策树" 和 "按受众" 导航,降低新用户学习成本.
  - 对现有 doctor 输出补一条 "结构化 JSON" 模式(若我们现在还没有),便于 TUI/CI 复用.
  - 为重要 gate/审批/外部输入增加更明确的审计落盘(谁触发,为什么,结果是什么).

- 中成本,高收益(需要确认边界,避免做成平台):
  - 把 "工具审批" 从实现细节提升为一等协议,并和 gate.request/gate.resolve 统一起来.
  - 引入轻量 PromptGuard,至少先覆盖最常见的系统覆盖/工具注入特征,并把命中证据写入日志.

- 不建议直接照搬(会违背 ralph 的 thin-coordination 目标):
  - 把 orchestrator 扩成 "runtime OS"(channel/memory/gateway/硬件外设全家桶).
  - 过早引入大量 sandbox backend,优先把"允许什么"和"审计证据"做扎实.

## 2026-02-28 20:46 +0800 | 设计: `ralph doctor` JSON 输出 schema(面向 code agent)

### 为什么要做

- 现状: `ralph doctor` 只有文本输出,CI/TUI/code agent 想做自动分流时,只能解析 stdout.
- 目标: 提供稳定的机器可读 schema,同时不破坏原文本输出.

### Schema(最小稳定字段)

- 顶层:
  - `schema_version`: u32(从 1 开始)
  - `verdict`: "pass" | "fail_errors" | "fail_strict"
  - `counts`: { `errors`: usize, `warnings`: usize }
  - `args`: { `fix`: bool, `strict`: bool, `format`: "text"|"json" }
  - `checks`: `DoctorCheck[]`

- DoctorCheck:
  - `id`: 稳定 check_id(用于分类)
  - `category`: 稳定类别(例如 config/hats/backend/workspace/context_window/events_marker/binary)
  - `status`: "ok" | "warn" | "err" | "skipped"
  - `message`: 保留原文本 message(包含 Fix/Skipped/Fixed 等,便于人读和回放证据)
  - `fix`: Option<String>(从 message 里解析 "Fix:" 后半段,便于程序直接展示/提取)

### check_id 初版约定(够用即可,后续可扩)

- config:
  - config.load
  - config.validate

- hats:
  - hats.validation.skipped
  - hats.solo_mode
  - hats.starting_event
  - hats.orphan_event
  - hats.dead_end

- backend:
  - backend.skipped
  - backend.auto_detect
  - backend.custom.command_required
  - backend.custom.command_available
  - backend.custom.version_failed
  - backend.custom.command_not_runnable
  - backend.available

- context_window:
  - context_window.skipped_backend_failed
  - context_window.skipped_not_configured
  - context_window.size
  - context_window.prompt_fit
  - context_window.prompt_fit.skipped

- workspace/scratchpad:
  - workspace.scratchpad.skipped
  - workspace.scratchpad_dir
  - workspace.scratchpad_exists
  - workspace.scratchpad_writable
  - workspace.scratchpad_path_kind

- events marker:
  - events_marker.missing
  - events_marker.invalid
  - events_marker.events_file_writable

- binary freshness:
  - binary_freshness.skipped
  - binary_freshness.stale
  - binary_freshness.ok

### 与 spec 的关系

- `specs/ralph-doctor.spec.md` 目前把 `--json` 放在 "后续扩展".
- 本次实现会把它提升为正式能力,并补齐回归测试(保证 schema 稳定).

## 2026-03-01 17:38 +0800 | 设计探索: hats 用 `ralph emit` 通信,同时对 `turn_action` 做 fail-closed 边界

### 现状(代码证据)

- `ralph emit` 已存在,支持 `--target/--target-instance/--spawn-instance/--workspace-strategy/--session-strategy/--turn-action`:
  - `crates/ralph-cli/src/main.rs`(EmitArgs)
  - `crates/ralph-cli/src/main.rs`(emit_command 写 JSONL)
- 并行 Supervisor 会轮询 `.ralph/current-events` 指向的 JSONL,把外部事件映射为 `ralph_proto::Event` 并路由:
  - `crates/ralph-core/src/parallel/supervisor.rs`(外部事件读取与映射)
  - 映射时当前会把 `turn_action` 直接带入,没有权限校验.
- 并行 runner 在 spawn hat backend 进程时,已经注入了:
  - `RALPH_HAT_ID`
  - `RALPH_HAT_INSTANCE_ID`
  - `crates/ralph-cli/src/parallel_runner.rs`(Spawning headless job)

### 风险

- 只要 hat 能执行 shell,就能执行 `ralph emit ... --turn-action interrupt`,从而在运行时打断 ralph#1 的 in-flight turn.
- 这属于典型的"控制面信号被数据面误用"问题,和普通 topic 消息不是一个风险等级.

### 我建议的边界(满足: hats 可沟通,但 steer/interrupt 不乱飞)

- 数据面(允许所有 hats): topic + payload(+可选 target/target_instance,且仍受订阅拓扑 strict 校验约束).
- 控制面(默认拒绝,fail-closed): `turn_action=steer|interrupt`(以及通常需要 app_server 的那类 in-flight 控制).
- hats 如果确实需要"打断/插话",改为发 request:
  - `turn_action.request` 或 `control.request_interrupt` 之类 topic,交给 ralph#1 或 Supervisor 决策并执行真正的 steer/interrupt.

### 4.2 的"先能用"落地思路(不依赖 guard token)

- 在 `ralph emit` CLI 里:
  - 如果检测到环境变量 `RALPH_HAT_INSTANCE_ID` 存在(说明是在 hat job 环境里调用),
    就直接拒绝 `--turn-action steer|interrupt` 并输出可行动的错误信息.
  - 这样模型误触发时会当场失败,hat 能从 tool 输出里自我纠正.

### 4.1 的增强(后续)

- 对 `<event ...>` 协议引入 guard token(或干脆在并行模式禁用 `<event>` 解析),进一步降低"文本里举例导致误触发"。

## 2026-03-01 17:45 +0800 | 决策: hat-to-hat 采用 request/result(数据面),不使用 steer 回传结果

### 结论(你已确认)

- hats 之间沟通只走 data-plane: `ralph emit <topic> <payload>`。
- `turn_action=steer|interrupt` 仅用于 ExternalInput -> ralph#1 的 control-plane。

### 为什么“B 回传 A 的结论”不需要 steer

- 即使 A 正在 Running,Supervisor 仍会把 B 的结果事件路由到 A 的 instance。
- A instance 在 Running 时不会被“真 in-flight 注入”(因为没有 turn_action),而是把事件放进 pending 队列。
- 因此 A 会在下一次 job/turn 中消费该结果,实现稳定的“异步 await”。

### 推荐协议形态(最小字段,够用即可)

- A -> B: `subtask.request`(或更具体的 topic)
  - payload(JSON) 包含:
    - `request_id`: 稳定关联 id
    - `task`: B 要做的事(必须足够自包含,避免依赖 A 的上下文)
    - `reply_to`: `{ "topic": "subtask.result", "target_instance": "hatA#1" }`

- B -> A: `subtask.result`
  - payload(JSON) 包含:
    - `request_id`
    - `status`: `"ok"|"error"`
    - `final`: `true`(A 只在 final=true 时推进,避免中途消息污染)
    - `result` 或 `error`

## 2026-03-01 22:45 +0800 | OpenSpec: proposal 已完成(emit-control-plane-fail-closed)

- 产物:
  - `openspec/changes/emit-control-plane-fail-closed/proposal.md`
- proposal 锁定的边界:
  - data-plane: hats 间沟通只用普通 `ralph emit topic=... payload=...`。
  - control-plane: `turn_action=steer|interrupt` 仅允许 ExternalInput 对 `ralph#1` 使用。
- fail-closed 策略:
  - hat job 环境(`RALPH_HAT_INSTANCE_ID` 存在)禁止 `ralph emit --turn-action ...`。
  - 使用 `--turn-action` 时必须显式 `--target-instance ralph#1`,否则拒绝(避免误投递)。
- 状态:
  - `openspec status` 已解锁 `design/specs`(ready)。

## 2026-03-01 23:57 +0800 | OpenSpec: design 已完成(实现落点与风险收敛)

- 产物:
  - `openspec/changes/emit-control-plane-fail-closed/design.md`
- design 的关键决策(摘录):
  - CLI 侧快速自纠:
    - hat job 环境(检测 `RALPH_HAT_INSTANCE_ID`)硬拒绝 `ralph emit --turn-action steer|interrupt`。
  - Supervisor 侧最终裁判(防御纵深):
    - 任意外部事件只要携带 `turn_action=steer|interrupt`,就必须显式且仅能 `target_instance=ralph#1`,否则拒绝并告警.
  - TUI 侧本地预检:
    - `!steer/!interrupt` 仅允许作用于 `ralph#1`,避免“写入后被拒绝”的黑盒体验.
- 明确 trade-off:
  - 放弃对 worker hats 的 in-flight steer/interrupt,换取无人值守环境下的稳定性与可预期.
- 后续增强方向(未纳入 4.2):
  - guard token 或 source attribution,把“只有 ExternalInput 能 steer/interrupt”做成更强约束.

## 2026-03-02 00:31 +0800 | OpenSpec: specs + tasks 已完成(可进入 apply)

- delta specs(Modified Capabilities):
  - `openspec/changes/emit-control-plane-fail-closed/specs/parallel-hat-instances/spec.md`
  - `openspec/changes/emit-control-plane-fail-closed/specs/parallel-trigger-routing/spec.md`
- tasks:
  - `openspec/changes/emit-control-plane-fail-closed/tasks.md`
- 规格要点(再次强调边界,避免实现漂移):
  - data-plane: hats 之间/hat->ralph 只用普通 `topic/payload`。
  - control-plane: `turn_action=steer|interrupt` 必须显式 target 到 `ralph#1`,其余一律 fail-closed 拒绝。

## 2026-03-02 00:36 +0800 | 补充: hat-to-hat “不在中途 reply,只回最终结论”已写入 OpenSpec change

- 已补写到:
  - `openspec/changes/emit-control-plane-fail-closed/design.md` 新增 D5:
    - A 触发 B 的子任务后,B 仅在自身 job/turn 结束时回传一次最终结论(例如 `subtask.result`),不在中途回传半成品.
  - `openspec/changes/emit-control-plane-fail-closed/tasks.md` 新增 4.3:
    - 要求把该约定同步写入 `specs/parallel-event-channels.spec.md`(面向 code agent 的使用指南)。

## 2026-03-02 12:34 +0800 | 确认决策: `<event>` 暂不收口,但 external turn_action 拒绝需可见告警

- 你确认的范围:
  - 4.2 暂不处理 in-band `<event ...>` 产生的 `turn_action=steer|interrupt`。
  - 4.2 只收敛 out-of-band external JSONL 注入(`ralph emit`/TUI 写 JSONL)的 `turn_action`.
- 你确认的行为:
  - all_hat 文档示例允许改掉,避免 hats 学到“对 worker 做 steer”的用法.
  - Supervisor 拒绝 external control-plane 注入时,需要让 `ralph#1` 明确看到(不只写日志)。
- 已同步到 OpenSpec change:
  - `openspec/changes/emit-control-plane-fail-closed/design.md`: 明确 external-only 范围,并指定拒绝时复用 `routing.escalate` 给 `ralph#1` 发告警.
  - `openspec/changes/emit-control-plane-fail-closed/specs/parallel-hat-instances/spec.md`: requirement 文案改为 external-only.
  - `openspec/changes/emit-control-plane-fail-closed/specs/parallel-trigger-routing/spec.md`: rejection 场景要求 emit `routing.escalate`。
  - `openspec/changes/emit-control-plane-fail-closed/tasks.md`: 增加 2.4(告警)与 4.4(更新 `config/all_hat.md`)。
