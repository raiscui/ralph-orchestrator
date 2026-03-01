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
