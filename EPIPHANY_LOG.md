## 2026-03-08 13:08 +0800 | 主题: 事件提取逻辑正在跨 crate 漂移

### 发现来源

- 整体项目代码 review 后,继续落修 `EventLoop`、`ralph-e2e analyzer`、`integration_agents`。
- 对照时发现:
  - `crates/ralph-cli/src/autopilot.rs` 的事件提取已经支持任意属性顺序
  - `crates/ralph-e2e/src/analyzer.rs` 之前仍停留在更窄的 regex

### 核心问题

- 同一协议(`<event ...>...</event>`)在不同 crate 内各自维护提取规则。
- 一处增强后,另一处很容易继续沿用旧假设,最后形成"主链路能吃,旁路吃不动"的隐性分叉。

### 为什么重要

- 这类问题平时很难靠肉眼发现。
- 一旦模型输出风格轻微变化,旁路工具链就会先坏:
  - autopilot 正常
  - analyzer 失败
  - review / report / diagnostics 口径开始不一致

### 未来风险

- 未来如果再出现:
  - 多行 opening tag
  - 属性顺序变化
  - 末尾保留多个同 topic 事件
- 很可能又会在某个辅助链路里重复踩到同类问题。

### 当前结论

- 这次已经把 `ralph-e2e` 的解析修到与主链路更一致的鲁棒性。
- 但"多个 crate 各写一套事件提取"这个结构性风险还在。

### 后续讨论入口

- 下次如果要继续收敛这类问题,建议先看:
  - `crates/ralph-cli/src/autopilot.rs`
  - `crates/ralph-e2e/src/analyzer.rs`
  - `crates/ralph-core/src/event_parser.rs`
- 优先讨论是否抽出共享 helper 或共享测试夹具,而不是继续各修各的 regex。

## 2026-03-08 13:31 +0800 | 主题: 事件提取漂移风险已部分收敛,但要继续盯 shared boundary

### 发现来源

- 针对上一条 epiphany 继续落地后,已把 `autopilot` 与 `ralph-e2e analyzer` 的 payload 提取收敛到 `ralph-core::EventParser`。

### 当前结论

- 这两条最直接的 analyze.complete 链路已经不再各自维护 regex。
- 风险从"两边一定会继续漂移"下降成了"未来新增旁路解析器时,要优先走 shared helper"。

### 后续讨论入口

- 如果后面还有别的 crate 需要按 topic 提取 `<event ...>` payload,优先先看:
  - `crates/ralph-core/src/event_parser.rs`
- 原则:
  - 先复用 shared helper
  - 再考虑是否需要扩 helper 语义
  - 最后才允许局部例外

## [2026-03-09 09:16:00] 主题: example 型 E2E 会污染控制面,还可能直接改写主仓库 HEAD

### 发现来源
- 运行 `cargo run -p ralph-e2e -- codex --filter example --keep-workspace --verbose`
- 结合 `.e2e-tests/report.md`、两个 scenario 的 `events.jsonl` 与主仓库 `git log` 观察得到

### 核心问题
- 两个 example 场景都混入了不属于当前拓扑的占位/示例型事件,持续触发 `routing.escalate`。
- `parallel-experimental-dev-engine-example` 甚至把集成提交落到了主仓库,当前 HEAD 被推进到 `2c4dd37 exp-002: e2e marker file`。

### 为什么重要
- 这说明 example E2E 当前不只是“回归失败”,还可能一边失败一边污染开发者真实工作区。
- 如果不先修隔离边界,后续任何 example 跑测都可能把主仓库状态和测试结论搅在一起,增加排查成本。

### 未来风险
- 误把 prompt 示例当运行时协议输出,会持续制造额外 job、迟到 job 和错路由,导致并行收敛指标失真。
- 集成阶段若继续写主仓库,可能把测试提交混入真实开发分支,让后续 diff、rebase、发布判断全部失真。

### 当前结论
- 已确认事实:
  - `parallel-trigger-routing-example` 因 job run count 异常与 `LOOP_COMPLETE` 后仍有新 job 失败
  - `parallel-experimental-dev-engine-example` 因 `routing.escalate` 过多失败
  - 主仓库 HEAD 当前为 `2c4dd37`
- 仍待确认:
  - 错路由究竟来自 prompt 示例泄漏、stderr 解析污染,还是例子配置本身仍引用旧拓扑
  - integration workspace 为什么会指向主仓库而不是隔离 example workspace

### 后续讨论入口
- 先读两个 example scenario 的场景定义与 prompt 生成逻辑
- 再核对 integrator 的 workspace_root / current_dir 决定链路

## 2026-03-09 13:42:00 +0800 | 主题: 并行模式可能先记录 CompletionPromise,再在退出期暴露 late JobCompleted race

### 发现来源
- 排查 `parallel-trigger-routing-example` 的 E2E 失败。
- 对照 `.ralph/session-20260309-1201.jsonl`、artifact stdout,以及 `parallel/instance.rs` / `parallel/supervisor.rs`。

### 核心问题
- 并行 run 在 record-session 里已经写出 `_meta.termination(reason=CompletionPromise)` 后,外层进程仍可能继续做 runtime cleanup。
- 此时若某个实例晚到一步才上报 `JobCompleted`,而 Supervisor receiver 生命周期已结束,就会打出 `Failed to send JobCompleted to supervisor`。

### 为什么重要
- 这会制造一种很误导的表象:
  - 业务已经闭环
  - record-session 已显示完成
  - 但 E2E/外层 CLI 最后仍可能 timeout/exit 130
- 如果不区分"语义完成"与"进程完全退出",后续很容易误判成 workflow 逻辑没跑通。

### 未来风险
- 任何依赖 app-server / continuation turn 的并行场景,都可能在 completion 之后继续暴露同类退出期竞态。
- 未来如果只盯 stdout 末尾 warning,可能会把真正问题误归因到事件路由或 completion 检测本身。

### 当前结论
- 当前最可疑的边界在:
  - `crates/ralph-core/src/parallel/instance.rs::HatInstanceActor::on_job_completed`
  - `crates/ralph-core/src/parallel/supervisor.rs::run_inner` / `drain_shutdown`
- 外围放大器在:
  - `crates/ralph-cli/src/parallel_runner.rs::run_parallel_loop_impl`
  - `crates/ralph-e2e/src/executor.rs` 的 timeout kill 逻辑

### 后续讨论入口
- 若后续要正式修,优先先验证 receiver 生命周期与 late completion 的先后顺序。
- 再决定是延长/重构 shutdown-drain,还是把 `JobCompleted` 发送失败在 completion 之后降级为非致命收尾信号。

## 2026-03-09 12:34:05 +0800 | 主题: 并行 log-mode 的 stdout 也是一种 durability contract

### 发现来源
- 修复 `parallel-trigger-routing-example` 时,对照:
  - `.e2e/stdout.txt`
  - `.ralph/session-*.jsonl`
  - `crates/ralph-cli/src/parallel_runner.rs`

### 核心问题
- 同一轮 run 里,session recorder 已经记录到完整实例输出,child stdout 却丢了尾部前缀行。
- 根因不是“业务没输出”,而是 log-mode stdout 在 pipe/E2E 场景下没有及时 flush。

### 为什么重要
- 这暴露了一个容易被忽略的事实:
  - stdout 不只是“给人看”
  - 在 E2E、wrapper、`tee`、CI pipe 里,stdout 本身就是证据面
- 如果不把 stdout 当成 durability contract,就会出现:
  - recorder 说完成了
  - stdout 看起来没完成
  - 报告/断言/人工观感三套口径分裂

### 未来风险
- 未来任何依赖并行 log-mode 前缀的场景,都可能再次被 stdout 缓冲放大成假失败。
- 如果后续继续做 stdout-based 断言,这类问题会反复出现。

### 当前结论
- 这轮已经先用“写入 + flush”把当前 E2E 收住了。
- 但从架构角度看,真正更稳的方向仍是:
  - 业务语义走结构化事件
  - job 运行次数走结构化 ledger
  - stdout 只承担展示与辅助诊断

### 后续讨论入口
- 优先看:
  - `crates/ralph-cli/src/parallel_runner.rs`
  - `crates/ralph-e2e/src/scenarios/parallel/job_run_counts.rs`
  - `crates/ralph-e2e/src/scenarios/parallel_trigger_routing_example.rs`

## 2026-03-09 16:35:00 +0800 | 主题: 真实并行 example 的 terminal topic 必须有明确 owner,外部 gate 等待窗口要按真实后端设计

### 发现来源

- 新增并验证 3 个真实并行 example:
  - `parallel-pr-review`
  - `parallel-release-checklist`
  - `parallel-human-approval-gate`
- 对照第一轮 live E2E 失败与修后通过结果得到。

### 核心问题

- 问题1:
  - `event_loop.complete_publishes` 只是 completion 目标的声明。
  - 如果没有某个 hat 真正 `publishes` 这个 terminal topic,闭环往往不够稳,甚至可能根本不会按预期收敛。
- 问题2:
  - 带外部事件注入的 human gate 场景,常常不是协议错,而是等待窗口比真实后端的收敛时延更短。

### 为什么重要

- 这两点都很容易在写 example 时被误当成“测试参数问题”。
- 但它们本质上是工作流设计问题:
  - terminal topic 没 owner,说明拓扑没有真正闭环
  - wait window 太短,说明测试把“时延预算”误写成了“正确性条件”

### 未来风险

- 未来再新增真实并行范例时,如果 terminal topic 仍然没有明确 owner:
  - 场景可能在文档上看起来合理
  - 但 live backend 下会表现成 flaky 或根本不完成
- 如果 human gate 的 injector 等待窗口继续卡太死:
  - `.ralph/events.jsonl` 已经出现正确信号
  - E2E 仍会以 timeout 形式假失败

### 当前结论

- 对真实并行 example 的设计,建议默认遵守两条:
  - 最终 completion topic 由明确的 synthesizer / finalizer hat 负责发布
  - 外部审批 / 人工 gate 场景的等待预算按真实后端收敛时延预留,不要用过紧窗口做 correctness gate

### 后续讨论入口

- 先看:
  - `examples/parallel-release-checklist/ralph.yml`
  - `examples/parallel-human-approval-gate/ralph.yml`
  - `crates/ralph-e2e/src/scenarios/parallel_release_checklist_example.rs`
  - `crates/ralph-e2e/src/scenarios/parallel_human_approval_gate_example.rs`

## 2026-03-09 23:19:50 +0800 | 主题: completion promise 在并行 example 里本质上是控制面 token,不能当普通 prose 提及

### 发现来源

- 排查第二批 example 的 `parallel-migration-rehearsal-example` live E2E 首轮失败。
- 对照:
  - `.e2e-tests/parallel-migration-rehearsal-example/.e2e/stdout.txt`
  - `.e2e-tests/parallel-migration-rehearsal-example/.ralph/events.jsonl`
  - `crates/ralph-core/src/event_parser.rs`
  - `crates/ralph-core/src/parallel/supervisor.rs`

### 核心问题

- 在当前 runtime 语义里,completion promise 不是单纯的“最终文案提示”。
- 只要 coordinator 在事件标签外的普通文本里提到它,Supervisor 就可能直接进入 completion drain。
- 这会形成一个很迷惑的表象:
  - 上游 lane 都完成了
  - 中间 fan-in 也可能已经发生
  - 但 finalizer job 永远起不来,因为后续路由已经被 completion drain 截断

### 为什么重要

- 这个坑不是 migration 场景独有。
- 任何 direct example / live backend 场景,只要允许 coordinator 在等待态输出自由 prose,都有机会误把 completion token 带出来。
- 一旦踩中,表面上很像:
  - finalizer prompt 不稳
  - runtime 没派发
  - worker 零输出
  但真正切断链路的是更上游的 completion 语义

### 未来风险

- 未来新增 example 时,如果只强调“不要太啰嗦”,而没有把等待态写成“静默或只发事件”:
  - 真实后端下仍可能复现同类假失败
- 如果后续测试只看 `events.jsonl` 的 `triggered` 字段,还容易误判成“finalizer 已经被真正调度”

### 当前结论

- 这轮已经用 prompt 级修复把 migration 场景收住:
  - 未收齐条件时静默
  - 不允许提 completion token
- 但架构层面的规律已经很清楚:
  - completion promise 是控制面 token
  - 不能把它当普通解释文字写进 example 的自由输出空间

### 后续讨论入口

- 优先看:
  - `examples/parallel-migration-rehearsal/ralph.yml`
  - `crates/ralph-core/src/event_parser.rs`
  - `crates/ralph-core/src/parallel/supervisor.rs`

## 2026-03-10 20:55:00 +0800 | 主题: E2E 真失败不一定在业务链路,也可能死在“证据层”自己的 UTF-8 处理

### 发现来源
- `parallel-postmortem-action-board-example` 第二轮 live E2E。
- 业务 topic 已经走到 `postmortem.board.ready`,但 `ralph-e2e` reporter 在写报告阶段 panic。

### 核心问题
- 测试框架如果在报告层按字节切字符串,会把多字节字符直接切坏。
- 这会制造一种很迷惑的假象:
  - 实际工作流已经成功
  - 但最终 verdict 仍是失败
  - 人会误以为业务逻辑还没收敛

### 为什么重要
- 这类问题不会出现在纯英文 fixture 上。
- 一旦真实后端开始稳定产出中文 payload,整个 E2E 体系都会暴露这个盲点。

### 当前结论
- 证据层必须和业务层一样遵守 Unicode 安全。
- 任何 report / TUI / CLI 的截断 helper 都应该统一走字符边界安全逻辑,不要各处手写 `[..N]`。

### 后续讨论入口
- 先看:
  - `crates/ralph-e2e/src/reporter.rs`
  - `crates/ralph-cli/src/display.rs`
  - `crates/ralph-adapters/src/stream_handler.rs`

## 2026-03-10 22:08:00 +0800 | 主题: 并行 example 的 event 协议不只要限制 topic,还要限制标签形态

### 发现来源
- 第四批真实并行 example 的 live E2E。
- `parallel-customer-renewal-desk-example` 首轮失败后,对照了 `.e2e/stdout.txt` 与 `.ralph/events.jsonl`。

### 核心问题
- 模型即使“看起来在发事件”,也可能发成 runtime 不接受的形态。
- 这次最典型的是自闭合 `<event .../>` 和 attribute-only payload。
- 从人眼看像事件,从 parser 看却不是同一个协议。

### 为什么重要
- 如果 example 只约束 topic 名和字段名,不约束事件标签形态:
  - stdout 会给人一种“已经发了”的错觉
  - `.ralph/events.jsonl` 却没有对应 topic
  - coordinator 就会一直等待缺失 ready,最后超时

### 未来风险
- 以后新增 direct example 时,如果仍然只写“从真实 event 开始”,但不禁止 self-closing:
  - 真实后端下仍可能复现同类假失败
  - 而且很容易被误判成 parser bug 或 runtime 路由 bug

### 当前结论
- 对 live backend example,事件形态本身就是协议的一部分。
- 推荐默认把以下 3 条写进 worker / finalizer prompt:
  - 禁止自闭合 `&lt;event .../&gt;`
  - 禁止把业务字段塞进 opening tag 属性
  - 强制 `<event ...>payload</event>`

### 后续讨论入口
- 先看:
  - `examples/parallel-customer-renewal-desk/ralph.yml`
  - `examples/parallel-audit-evidence-pack/ralph.yml`
  - `crates/ralph-core/src/event_parser.rs`

## 2026-03-12 00:52:00 +0800 | 主题: 并行 live E2E 的“最终 payload 证据”与“多行 event 形态”都需要单独护栏

### 发现来源
- batch-7 的 `parallel-revops-quote-desk-example` 连续两轮真实 Codex live E2E。
- 对照了:
  - `.e2e/stdout.txt`
  - `.ralph/events.jsonl`
  - `crates/ralph-e2e/src/scenarios/parallel_revops_quote_desk_example.rs`
  - `examples/parallel-revops-quote-desk/ralph.yml`

### 核心问题
- 问题不是单一层面的。
- 第一类是假失败:
  - 业务已经成功
  - 但断言读取的是截断版 payload
  - 于是长 summary 把关键固定字段挤出了证据窗口
- 第二类是真失败:
  - worker 确实“看起来发了 event”
  - 但 closing tag 少一个 `>`
  - parser 就不会认它是 event

### 为什么重要
- 如果只盯 `events.jsonl`,会把“证据层截断”误判成业务失败。
- 如果允许 worker 自由输出多行 line-style event,真实 backend 下 closing tag 漂移概率会明显上升。
- 这两类问题叠在一起时,非常容易把排障带偏:
  - 一会儿像 matcher 问题
  - 一会儿又像 routing 问题
  - 实际上是“证据层”和“协议层”各有一个坑

### 当前结论
- 对并行 live E2E:
  - 最终 payload 断言应优先从“剥掉 `[hat#n:out:job=m]` 前缀后的 stdout out 行”提取
  - 不要默认把 `.ralph/events.jsonl` 当成唯一真相,因为它可能是截断证据
- 对 worker event 设计:
  - 能用单行 JSON event 的,尽量不要给多行 YAML/列表自由度
  - 多行 event 只在确有必要时使用,并且必须把 closing tag 精确性写成硬约束

### 未来风险
- 后续 batch-8 或更长 payload 的商业场景,如果继续沿用“长 summary + 截断证据”组合:
  - 还会重复出现“业务已成功,断言误报失败”
- 如果新 worker prompt 继续允许多行自然语言式 payload:
  - closing tag 漂移仍可能再次复现

### 后续讨论入口
- 优先看:
  - `crates/ralph-e2e/src/scenarios/parallel/mod.rs`
  - `crates/ralph-e2e/src/scenarios/parallel_revops_quote_desk_example.rs`
  - `examples/parallel-revops-quote-desk/ralph.yml`

## 2026-03-12 13:49:00 +0800 | 主题: 对真实 backend 漂移较高的 worker lane,需要“唯一允许模板”,而不只是“单行 JSON 原则”

### 发现来源
- batch-8 的 `parallel-renewal-risk-calibration-example` 首轮真实 Codex live E2E。
- 对照了:
  - `.e2e/stdout.txt`
  - `.ralph/events.jsonl`
  - `examples/parallel-renewal-risk-calibration/ralph.yml`

### 核心问题
- 有些 worker lane 即使已经被约束为:
  - 单行真实事件
  - 紧凑 JSON payload
  - 精确 `</event>`
- 在真实 backend 下仍然可能不产出任何结果。
- 也就是说,“原则上够机械”不等于“运行时真的够机械”。

### 为什么重要
- 这类失败最容易误判成:
  - coordinator 没路由
  - parser 没识别
  - finalizer 没收口
- 但真实情况可能只是某个 lane 根本没吐出 event。
- 如果没有同时看:
  - lane 状态
  - `.e2e/stdout.txt`
  - `.ralph/events.jsonl`
  很容易把修复方向带偏。

### 当前结论
- 对真实 backend 漂移风险更高的 lane:
  - 不要只写“请输出单行 JSON event”
  - 要直接给“唯一允许模板”
  - 必要时把关键字段固定到示例值
- 这次 `success_plan_reviewer` 加上 literal 模板后,二次 live E2E 通过。

### 未来风险
- 后续 batch-9 以后如果继续扩经营类、评审类、摘要类场景:
  - 某些 lane 仍可能“看上去约束充分,实际运行仍沉默”
- 特别是名称比较抽象的 lane:
  - `success`
  - `planning`
  - `alignment`
  - `readiness`
  更容易让模型自由发挥

### 后续讨论入口
- 优先看:
  - `examples/parallel-renewal-risk-calibration/ralph.yml`
  - `crates/ralph-e2e/src/scenarios/parallel_renewal_risk_calibration_example.rs`
  - `notes.md` 里 batch-8 验证摘要

## [2026-03-12 14:37:00 +0800] 主题: `reply` 关联语义与“答案回给请求方”不是同一层协议

### 发现来源
- 在 `openspec-explore` 讨论“hat 最终回答是否应返回给创建者”时,回读了 `event-id-and-reply` change 与 parallel runtime.

### 核心问题
- 当前系统已经有 `Event.reply`,但它只回答“这条事件回复了谁”。
- 它没有回答“这条回复应该被送回给谁”。
- 如果团队把这两个问题混成一个,后面很容易在 prompt、topic 和 routing 上出现半协议、土协议和双通道噪音。

### 为什么重要
- 这会直接影响 future multi-hat ask/research/query 场景。
- explorer hat、资料搜集 hat、路径探测 hat 天然更像 request-reply,而不是纯 workflow handoff。

### 未来风险
- 如果把“所有 hat final answer 默认回传 creator”做成全局规则,会制造大量无用输出和潜在循环。
- 如果完全不定义 answer-return,各 hat 又会各自发明 `*.answer` / `*.result` / `reply.human.message` 式变体,协议会漂。

### 当前结论
- 现有 `event-id-and-reply` 足够支撑“关联”。
- 若要支撑“回答回流”,应作为一层显式、可选的 request-reply / answer-return 协议单独设计。

### 后续讨论入口
- 继续看:
  - `openspec/changes/event-id-and-reply/design.md`
  - `crates/ralph-core/src/parallel/instance.rs`
  - `crates/ralph-core/src/parallel/supervisor/routing.rs`
## [2026-03-18 18:02:00] [Session ID: 2d1fc46f-d36c-45b6-af3b-ab3318b8c122] 主题: preset 自主选择应放在 bootstrap 阶段,不要在正式 run 中途热切换拓扑

### 发现来源
- 探索“无 `PROMPT.md` / 无 `ralph.yml` 默认运行 + presets 内嵌 + Ralph 自选 preset/hat”时,回读了 `EventLoop`、`HatRegistry`、`parallel Supervisor`、config validate 与 preset embed 代码。

### 核心问题
- 当前 Ralph 的大量护栏都建立在“run 启动前已经有一份最终 `RalphConfig`”这个前提上。
- 如果在正式运行中途再切换整套 preset / hat 拓扑,就会把配置校验、路由约束、收敛语义、topic contract 全部推成动态重建问题。

### 为什么重要
- 这类需求很容易被表述成“让 Ralph 自己决定就好了”。
- 但真正危险的地方不在选择动作本身,而在“选择发生在哪个时刻”。
- 时机一旦错了,系统复杂度会指数上升。

### 未来风险
- 若直接支持运行中热切换 preset:
  - 可能破坏 `complete_publishes` 的发布者约束
  - 可能引入 trigger 冲突与 hat 污染
  - 可能让串行/并行模式的验证规则失去意义

### 当前结论
- “Ralph 自主选择 preset”是可行方向。
- 但应优先设计成 bootstrap selector,先产出 resolved config,再启动真实 run。
- 只有这样,现有 guardrails 和可验证性才能最大程度复用。

### 后续讨论入口
- 先讨论 resource catalog 的结构与 resolved config 产物格式。
- 再决定 selector 是规则优先、LLM 优先,还是二者结合。

## [2026-03-19 14:14:57] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 主题: YAML 注释不是稳定的 runtime metadata contract

### 发现来源
- 评审 `startup-resource-bootstrap` 时,用户提出希望把 `ralph.yml` 头部注释和 hat 描述一起注入给 `ralph#1`,用于 workflow / hat 选择。
- 对照 `crates/ralph-cli/src/presets.rs`、`crates/ralph-core/src/config.rs`、`crates/ralph-core/src/hat_registry.rs` 的当前实现得到。

### 核心问题
- `hat.description` 是正式结构化字段,能稳定进入 runtime。
- 但 workflow 文件头的 YAML 注释不会进入 `RalphConfig`,当前 preset 简介也是编译期手工常量,不是运行时动态解析注释。
- 如果把 selector / capability invoker 建在注释上,后面 catalog、materialize、用户编辑、child run 都会出现不一致。

### 为什么重要
- 这不是一个“实现细节”,而是 catalog 设计边界。
- 一旦边界错了,后续所有:
  - startup selector
  - runtime capability discovery
  - doctor / debug / replay
  都会建立在不稳定输入上。

### 未来风险
- 继续把 workflow 摘要写在注释头里,很容易出现:
  - 人类看到有说明
  - 机器运行时却根本拿不到
- 以后换一种 materialization / parse 路径时,还会重复冒出“为什么这次读不到描述”的隐性问题。

### 当前结论
- 注释可以保留,但只能作为人类可读说明。
- 真正给 `ralph#1`、selector、capability invoker 使用的信息,必须落在结构化 metadata 上。
- 这条规则已经分别写进:
  - `startup-resource-bootstrap`
  - `runtime-capability-invocation`

### 后续讨论入口
- 下次如果继续做 catalog / capability,优先先看:
  - `openspec/changes/startup-resource-bootstrap/design.md`
  - `openspec/changes/runtime-capability-invocation/design.md`
## [2026-05-18 07:01:08] [Session ID: omx-1779004640353-blcixq] 主题: live capability “只思考不出结果” 可能是多层故障叠加

### 发现来源
- 这次对 `parallel_rec.jsonl` 的复跑与回溯分析。
- 结合了 child run panic、workflow capability materialization 和 live capability 专项测试的复核结果。

### 核心问题
- 表面现象是 parent 侧一直在输出 reasoning / thinking,但没有最终结果。
- 实际上可能同时叠着两层甚至多层问题:
  - child 预览/截断阶段先 panic。
  - workflow capability resolved config 仍是 stub,导致根本没有正确 materialize 出预期的运行图。

### 为什么重要
- 只盯着 UI 或单一日志层,很容易把“没有结果”误判成同一个根因。
- 这种问题如果不拆层,后续还会继续把 child failure、parent result 回写、以及 runtime materialization 混成一团。

### 未来风险
- 以后任何“持续输出 thinking 但没有 result”的 case,都可能再次出现这种叠层故障。
- 如果先入为主只修一层,很容易留下表面已绿、实际语义契约仍坏的假修复。

### 当前结论
- 这次验证后,当前代码状态下 live capability 专项测试已经转绿。
- 但分析方法上必须保留“先看 `events.jsonl` / `capability.result` / `capability.failed`,再看 UI”的顺序。

### 后续讨论入口
- 下次再遇到类似问题,先从 record / events.jsonl / failed.json 三件套切层,不要直接从思考流量判断卡死。

## [2026-05-18 07:18:00] [Session ID: omx-1779004640353-blcixq] 主题: 简单问题也会拖长的真正原因是协调面太宽

### 发现来源
- 对 `parallel_rec.jsonl` 的二次统计和对 `parallel#1` prompt 注入逻辑的回看。

### 核心问题
- `ralph#1` 不只是“回答问题”的角色,它同时被要求做协调、记忆治理、文件上下文维护、事件发射、证据收集,这让它在简单问题上也会先陷入元流程。

### 为什么重要
- token 浪费并不总是因为模型不会答,而是因为它在过度遵守一整套复杂协作协议。
- 如果不拆分角色,后面同类简单问题还会继续被拖成长轮次思考。

### 未来风险
- 任何涉及 `AGENTS.md`、六文件上下文、skill、omx state、memory 的任务,都会有再次放大轮次的风险。
- 越是简单的问答,越容易被这套治理协议挤成“先想很多,后说很少”。

### 当前结论
- 代码失败是第一层,流程过宽和缺少 event-first 快路径是第二层。
- 当前现象最像“协调协议诱发的高 token 额外成本”,不是业务逻辑本身复杂。

### 后续讨论入口
- 后面如果要优化,建议先拆 `coordination` 和 `answering` 两个 prompt surface,再考虑给简单问题加 turn budget。
## [2026-05-18 10:34:11] [Session ID: omx-1779004640353-blcixq] 主题: Ralph 必须是调度者,非 Ralph hat 必须是执行者

### 发现来源
- 用户在分析 `parallel_rec.jsonl` 的 token 浪费问题后指出: Ralph 的任务应该是决定任务如何分发、安排和分配,而不是真正解决问题。
- 这和前面观察到的“简单问题也被协调/文件治理/记忆治理拖长”现象一致。

### 核心问题
- 当前 prompt surface 容易让 `ralph#1` 同时承担协调者、分析者、执行者、记录员等角色。
- 非 Ralph hat 如果也继承太多和 Ralph 相同的 prompt,就会导致职责边界模糊: worker 会思考调度,coordinator 会亲自解题。

### 为什么重要
- Ralph 的核心价值不是亲自回答,而是把任务切成清晰事件、选择合适 hat、管理依赖和收敛结果。
- Child hat 的核心价值是按 role contract 完成具体任务,不应该携带完整 coordinator prompt。

### 未来风险
- 如果继续让 Ralph 和 worker 共享过宽 prompt,简单问题会继续耗费大量 token 在元流程上。
- 后续 parallel runtime / capability runtime 的行为也会变得不可预测: 有的任务被 Ralph 自己做掉,有的任务被 worker 再次调度。

### 当前结论
- 应该拆成三层 prompt:
  - Coordinator prompt: 只给 Ralph,负责分发、安排、收敛、兜底。
  - Worker prompt: 只给普通 hat instance,负责完成被分配的具体任务。
  - Shared protocol prompt: 极小公共层,只包含 event envelope、reply 语义、必要 stop/输出协议。

### 后续讨论入口
- 后续如果实现 prompt 瘦身,优先从 `crates/ralph-core/src/parallel/supervisor.rs` 和 `crates/ralph-core/src/parallel/instance.rs` 的 prompt assembly 边界切入。
## [2026-05-18 10:39:32] [Session ID: omx-1779004640353-blcixq] 主题: Ralph 动态创建 hat 时应支持三类身份来源

### 发现来源
- 用户补充: Ralph 分配任务、创建 hat 实例时,可以基于项目模板,也可以基于 `ralph.yml` 中配置的 hats,还可以实时根据任务性质直接生成 hat 身份角色。

### 核心问题
- hat 身份不能被限制成“只能来自静态配置”。
- 但动态生成的 hat 也不能继承过多 Ralph coordinator prompt,否则它会同时承担调度和执行职责,重新制造职责混淆。

### 为什么重要
- 这给 Ralph 的分发能力打开了第三条路径: dynamic role synthesis。
- Ralph 的任务应该是选择合适身份来源,而不是把自己的一整套 prompt 复制给子实例。

### 未来风险
- 如果动态 hat 只是把 Ralph prompt 复制一份再加一个角色名,它还是会继续过度思考、过度治理、过度协调。
- 如果动态生成完全无边界,也可能造成 role 漂移、输出契约不稳定、测试难以断言。

### 当前结论
- 后续 prompt / runtime 设计应显式区分三种 hat 身份来源:
  - template-derived hat: 来自项目内模板或 preset。
  - config-derived hat: 来自 `ralph.yml` 的静态 hats。
  - task-derived dynamic hat: Ralph 根据当前任务性质即时生成的轻量身份。
- 无论哪种来源,非 Ralph hat 都应该只获得 worker prompt + role contract + shared protocol,不能继承完整 coordinator prompt。

### 后续讨论入口
- 继续设计时,需要给 dynamic hat synthesis 增加最小字段: role name, objective, input contract, output contract, allowed topics, stop rule。
## [2026-05-18 10:44:57] [Session ID: omx-1779004640353-blcixq] 主题: Ralph 只该负责调度,worker 只该负责执行,共享 prompt 必须极小化

### 发现来源
- 用户对 `parallel_rec.jsonl` 的 token 浪费问题做出的架构补充。
- 本轮新落盘的 `specs/ralph-prompt-role-layering.md`。

### 核心问题
- 如果 Ralph 和 worker 共享太多 prompt,就会把调度、治理、分析、执行这些职责混成一团。
- 这会让简单问题也被迫走完整的元流程,造成严重 token 浪费。

### 为什么重要
- Ralph 的价值在于分发与收敛,不是亲自解题。
- worker 的价值在于完成被分配任务,不是重新做全局调度。

### 未来风险
- 如果后续实现仍把 coordinator prompt 广播给所有 hat,职责边界会再次崩掉。
- dynamic hat 如果没有最小 role contract,很容易变成“换个名字的 Ralph”。

### 当前结论
- 需要明确三层 prompt:
  - coordinator prompt.
  - worker prompt.
  - shared protocol prompt.
- 三类 hat 身份来源也应明确: config-derived, template-derived, task-derived dynamic.

### 后续讨论入口
- 后续实现 prompt assembly 时,优先检查是否把 coordinator-only sections 错注入给 worker。

## [2026-05-21 21:02:00] [Session ID: omx-1779158263949-kticiv] 主题: 控制面协议必须给 schema-literate 示例,不能只依赖 Rust parser

### 发现来源
- task-derived role contract live dogfood。
- 首轮 `topology.spawn_group` 失败: `instances[0]: field input must be a string when present`。

### 核心问题
- Rust parser 和 runtime canonicalization 已经严格,但 LLM coordinator 首轮仍会把 `role_contract` 这种结构化 contract 错放进 `input` object。
- 如果 prompt 只说字段列表而不给 sibling field 示例,模型会倾向把 contract 当作 worker input 子对象。

### 为什么重要
- parent-visible topology spawn、capability request、reply.hat.message 这类控制面协议都依赖 LLM 正确发结构化事件。
- 只靠 parser fail closed 可以避免坏状态,但会制造 live run retry、超时和用户可见的不确定性。

### 未来风险
- 后续新增任何控制面字段时,如果 prompt 示例不同步,会再次出现“类型系统正确,模型输出错误”的断层。
- 这种问题不会在纯 Rust unit test 中自然暴露,需要 live dogfood 或 prompt regression test 才能抓到。

### 当前结论
- 已补强 `event_emission_protocol.rs`: `role_contract` 是 `instances[]` sibling field,`input` 必须是 string,禁止把 `role_contract` 放进 `input`。
- 已补 focused prompt test 锁住 guidance。

### 后续讨论入口
- 继续设计控制面协议时,先写 prompt schema 示例和负例,再写 parser。
- live dogfood 后应把 LLM 实际犯错形态转成 prompt guardrail test。

## [2026-05-28 12:26:41] [Session ID: omx-1779158263949-kticiv] 主题: recoverable retry 必须有 recovered 终态并能打断 completion freeze

### 发现来源
- `agent-cli-recoverable-failure-retry` 3.x parallel runtime retry lifecycle 实现。
- 将 recoverable failure 接入 `HatInstanceActor` 和 `ParallelSupervisor` 时发现 completion promise / freeze 与 scheduled retry 存在竞态。

### 核心问题
- 如果 retry 成功后不写入 `recovered` 终态,ledger replay 会长期停在 `retrying`,Supervisor / evidence inspect 无法证明 lifecycle 已解决。
- 如果 coordinator 先输出 completion promise,worker 随后才暴露 recoverable failure,旧 completion freeze 可能让 scheduled retry 永远起不来。

### 为什么重要
- retry lifecycle 不是纯 executor 行为,它会影响 Supervisor 的完成判定。
- completion promise 是软退出信号,不能覆盖仍然 pending 的 recoverable job lifecycle。

### 未来风险
- 后续做 manual `!continue` 或 agents snapshot 可视化时,如果只看 `Failed/Idle/Running` 这些粗状态,可能会把 retryable job 误显示为已结束或空闲。
- 如果新增 retry 状态但没有终态,`ralph agents` / record summary 会出现“永远 retrying”的假象。

### 当前结论
- recoverable lifecycle 至少需要 pending 类状态和 terminal 类状态两组语义。
- terminal 类状态当前包括 `recovered` 和 `exhausted`。
- pending recoverable transition 出现时,Supervisor 应撤销 completion drain/lockdown,让 retry lifecycle 先闭环。

### 后续讨论入口
- 继续 4.x manual continue 时,`continued_by_human` 应走同一套 scheduler path,并最终落到 `retrying -> recovered/exhausted`。
- 继续 5.x observability 时,agents snapshot 应能把 pending 和 terminal recoverable 状态区分展示。

## [2026-08-12 21:20:00] [Session ID: omx-1786419140441-df5ql8] 主题: cherry-pick 「risk group」标签必须 dry-run 实证,zero/small/medium risk 全是脆性假设

### 发现来源

Group 2 6 项全部 dry-run 失败,proposal 标为「small-risk」但全部冲突。

### 核心问题

proposal 给 cherry-pick 打的「zero/small/medium risk」分级是基于 commit message 与
scope 推测,不是 dry-run 实证。本地 main 的架构调整(EventLoop 收窄 / 大量
adapter 重写 / mcp.rs 删除)让这些 scope 假设全部失效。

### 为什么重要

直接后果:
- 6 项 Group 2「small-risk」全部冲突 -> 14 行 Group 1 + 2 14 行(假设可执行)
  -> 实际只有 2 项可落地(1.1 manual port + 1.6 partial)
- 影响 proposal 数据可信度:所有「risk group」标签都需要重新审查

规律:
- 「small-risk」类(test 改动、文档)冲突率 = 100%
- 「medium-risk」类(没测)预计类似高冲突率
- Group 3 的 5 项(中风险)也得先 dry-run 才能 verify
- Group 4 rewrite 任务从 6 个变 12+ 个

### 未来风险

- Group 3 同样会失稳,不验证就推进会浪费时间
- proposal 的「groups 1-3 cherry-pick, 4 rewrite, 5 patch」框架基本失效
- 唯一可信的 cherry-pick 门:`git cherry-pick --no-commit <sha>`,必跑
- 整个 change 名存实亡,可能 archive 时就只能附经验记录

### 当前结论
- 已验证规律:Group 2 「small-risk」实际全部冲突
- 已落实:Group 2.1-2.6 全部移 Group 4 follow-ups
- 必须推进:Group 3 也先 dry-run 才能继续

### 后续讨论入口
- 此规律已加到 self-learning.git-cherry-pick-preflight 候选
- 任何 cherry-pick 计划必须 explicit dry-run gate
- 「risk group」标签 should be deprecated 变成 dry-run 实证

## [2026-08-12 22:05:00] [Session ID: omx-1786419140441-df5ql8] 主题: audit 「22 lines」叙述低估了 whole-crate 删除,scope 必须先验证再引用

### 发现来源

P4 audit 揭示 proposal.md 写「22 lines reverse diff on `ralph-api/src/main.rs`」实际是「整个
ralph-api/ crate 删除」(17 src 文件 + 多个子目录 + data + tests)。

### 核心问题

proposal 在写 Group 5 P4 时只看到 surface (`main.rs` 22 行),没看整体(crate 1e88b7e3 的
完整 src 文件清单)。Auditor 不知道 rewrite target 是不是真的存在。

### 为什么重要

- 直接后果:Group 4 §1 / §4 rewrite tasks 的目标文件根本不存在 → rewrite 是无意义的
- 误判风险:rewrite 一通后发现工作全 white 白费;但「audit scope 不足」不是致命的,
  致命的是「audit 写完没人 read」所以 rewrite 任务保留很 negative
- 类似 P1、P2 是否也低估了 scope? 值得重新 verify

### 未来风险
- 任何 audit 描述用「22 lines」「minimal」之类都要警惕:
  - 「22 lines」可能背后是「whole module」
  - 「minimal」可能背后是「whole feature」
- audit 必须先 git ls-tree + git grep 验证 file inventory,
  而非只看 stat 数

### 当前结论
- 已落地:Proposal Appendix C 反映 audit 实际 scope
- 已落地:Group 4 §4 / §4.15 标 dropped
- 后续:类似 P4 的描述「22 lines」「+87/-197」要避免,改用 file inventory + capability grep

### 后续讨论入口
- 新建 declarative-e2e-mock-parity change 作为 F1 follow-up
- 任何 follow-up Change 不混在 sync-origin-main-features 里,单独 openspec/change
