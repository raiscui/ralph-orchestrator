## 2026-03-09 11:54:58 +0800 | WORKLOG 续档

### 任务内容

- 因 `WORKLOG.md` 超过 1000 行,按项目规则执行续档。
- 同步为 `task_plan.md`、`notes.md`、`ERRORFIX.md` 一并重开新档,避免本轮 example E2E 证据继续堆在旧文件中。

### 完成过程

- 已将旧文件重命名为带时间戳版本。
- 已回读当前四文件与最近一批归档文件尾部,完成持续学习摘要。
- 新档已建立,当前任务将继续在新档中记录。

### 总结感悟

- 续档不是机械换文件名。
- 真正有价值的是把最近几轮形成的稳定规律提炼出来,再继续执行。

## 2026-03-09 13:42:00 +0800 任务名称: 静态排查 parallel-trigger-routing-example completion 后的 JobCompleted 发送失败

### 任务内容
- 只做代码阅读与运行证据比对,不改代码。
- 排查 `HatInstance actor exited with error instance=ralph#1 error=Failed to send JobCompleted to supervisor` 的最可能触发路径与收尾时序。

### 完成过程
- 回读了仓库六文件上下文,确认这个问题此前已被记录为 example 场景的高风险项。
- 定位到 warning 只会从 `crates/ralph-core/src/parallel/instance.rs` 的 `JobCompleted` 发送路径冒出。
- 对照 `parallel/supervisor.rs` 的 completion drain / shutdown drain 与 `.ralph/session-20260309-1201.jsonl` 的 `_meta.termination` 证据,确认这是 completion 后的退出期竞态。
- 同时核对 `crates/ralph-e2e/src/executor.rs`,确认 E2E timeout 会对进程组发终止,能解释最终 `exit 130`。

### 总结感悟
- 这次现象不是“业务没完成”,而是“业务已完成但进程没彻底收干净”。
- record-session 与 stdout artifact 必须分开看: 前者证明语义完成,后者暴露退出期 race。

## 2026-03-09 12:34:05 +0800 任务名称: 收敛 example 覆盖型 E2E 的 stdout 丢尾与 cleanup 卡死

### 任务内容
- 修复 `parallel-trigger-routing-example` 在真实 Codex E2E 下的双重失败:
  - log-mode stdout 尾部输出丢失,导致 job 计数与 `LOOP_COMPLETE` 断言假失败
  - completion 已发生后,并行 runtime cleanup 无界等待,导致最终 `exit 130`
- 覆盖验证:
  - `parallel-trigger-routing-example`
  - `example`
  - `cargo test -p ralph-core smoke_runner`
  - `cargo test`

### 完成过程
- 在 `crates/ralph-cli/src/parallel_runner.rs` 增加了 `write_parallel_cli_line()`:
  - 统一把 log-mode stdout 的“写一行”升级成“写入 + flush”
  - 让 E2E 父进程、pipe、`tee` 能及时拿到完整带前缀的实例输出
- 在同文件增加 `shutdown_parallel_runtime_with_timeout()` 与 `shutdown_parallel_runtimes()`:
  - 给 `codex_mcp_runtime.shutdown_all()`
  - 给 `codex_app_server_runtime.shutdown_all()`
  - 都加上有界等待,避免 `_meta.termination` 已写出后还无限卡在 cleanup
- 新增了 3 条回归测试:
  - stdout 写入会立刻 flush
  - runtime cleanup timeout 会被检测为 timeout
  - runtime cleanup 正常完成时会返回 success
- 动态验证结果:
  - `parallel-trigger-routing-example` ✅ `86.2s`
  - `example` ✅ `463.7s`
    - `parallel-trigger-routing-example` `79.7s`
    - `parallel-experimental-dev-engine-example` `382.1s`
  - `cargo test -p ralph-core smoke_runner` ✅
  - `cargo test` ✅

### 总结感悟
- 并行 E2E 不能只看“record-session 里有没有事件”,也不能只看“stdout 表面有没有前缀行”。
- 真正可靠的判断需要把三层拆开看:
  - 语义完成
  - 可见性/耐久性
  - 进程退出

## 2026-03-09 16:35:00 +0800 任务名称: 扩充 3 个真实并行 example 与对应 live E2E

### 任务内容
- 新增 3 个更贴近真实工作流的并行范例:
  - `examples/parallel-pr-review`
  - `examples/parallel-release-checklist`
  - `examples/parallel-human-approval-gate`
- 为每个范例补齐 `ralph-e2e` scenario,让 example 目录不只是展示,还能自动回归验证。
- 同步补齐 spec、README、E2E README 与 example 自包含性测试。

### 完成过程
- 先写了 `specs/parallel-real-world-examples.spec.md`,把 3 条协议链路、terminal topic 与验证目标先钉清楚。
- 再分别实现 3 个 example 的 `ralph.yml`、`PROMPT.md`、`README.md`,保持都能独立阅读和运行。
- 在 `crates/ralph-e2e/src/scenarios/` 新增 3 个 direct example scenario,并补到:
  - `crates/ralph-e2e/src/scenarios/mod.rs`
  - `crates/ralph-e2e/src/lib.rs`
  - `crates/ralph-e2e/src/main.rs`
- 在 `crates/ralph-e2e/src/scenarios/parallel/mod.rs` 提出共享 helper `patch_example_config_for_codex_e2e`,避免 3 份 Codex 降噪 patch 重复漂移。
- 真实验证时先后修正了两个协议边界:
  - completion topic 需要由具体 hat 发布,不能只写在 event loop 完成条件里
  - 人类审批注入场景的等待窗口不能过短
- 最终验证:
  - live E2E:
    - `parallel-pr-review-example` ✅ `179.7s`
    - `parallel-release-checklist-example` ✅ `127.5s`
    - `parallel-human-approval-gate-example` ✅ `201.9s`
  - fresh 仓库级验证:
    - `cargo test` ✅

### 总结感悟
- 真正有价值的 example,不是“看起来像 demo”,而是能被真实后端跑通并留下自动化证据。
- 对并行工作流来说,terminal topic 的归属与外部事件等待预算,都是一等设计问题,不是测试细节。

## 2026-03-09 23:19:50 +0800 任务名称: 扩充第二批真实并行 example 并修复 migration rehearsal 的 premature completion

### 任务内容
- 新增第二批 3 个真实并行 example:
  - `examples/parallel-incident-response-war-room`
  - `examples/parallel-migration-rehearsal`
  - `examples/parallel-proposal-assembly`
- 为每个 example 补齐 direct example E2E scenario、注册点、README 与自包含性验证。
- 在验证过程中修复 `parallel-migration-rehearsal-example` 的 premature completion 问题。

### 完成过程
- 先实现第二批 spec、example 三件套与 direct example scenario。
- 再补到:
  - `crates/ralph-e2e/src/scenarios/mod.rs`
  - `crates/ralph-e2e/src/lib.rs`
  - `crates/ralph-e2e/src/main.rs`
- 共享接线层做了一个小改良:
  - 在 `crates/ralph-e2e/src/scenarios/parallel/mod.rs` 新增 `setup_prompt_file_example_workspace(...)`
  - 让 direct example 的 workspace setup 不再在 6 个 scenario 里重复漂移
- live E2E 过程中发现 migration 场景首轮失败。
  - 通过 `.ralph/events.jsonl`、`.e2e/stdout.txt` 与 `EventParser::contains_promise()` 的代码路径对照,确认是 coordinator 普通文本误提 `LOOP_COMPLETE` 导致提前 completion drain。
  - 随后只强化 migration example 的 coordinator 静默等待规则与 finalizer 触发约束,并补充 guard 测试。

### 验证结果
- live E2E:
  - `parallel-incident-response-war-room-example` ✅ `104.7s`
  - `parallel-migration-rehearsal-example` ✅ `88.5s`
  - `parallel-proposal-assembly-example` ✅ `151.1s`
- targeted regression:
  - `cargo test --package ralph-e2e --lib scenarios::parallel_migration_rehearsal_example::tests::example_config_requires_silent_wait_before_all_ready_lanes -- --exact` ✅
- fresh 仓库级验证:
  - `cargo test` ✅

### 总结感悟
- completion promise 不只是“最后一行文案”,它在当前 runtime 里其实是控制面信号。
- 只要 example 允许 coordinator 在等待态输出自由文本,就有机会把 terminal token 意外带进普通 prose,从而提前切断后续路由。
- 对这类 direct example,等待态最好机械化:
  - 要么发事件
  - 要么静默
  - 不要写“半解释、半自言自语”的过渡文本

## 2026-03-10 20:55:00 +0800 任务名称: 修复 postmortem 并行 example 收敛与 reporter UTF-8 panic

### 任务内容
- 收敛 `examples/parallel-postmortem-action-board` 的 live E2E 失败。
- 修复 `crates/ralph-e2e/src/reporter.rs` 因中文多字节字符被按字节截断而 panic 的问题。
- 补齐该场景的自包含测试与 payload 语义断言,并完成 live + 仓库级验证。

### 完成过程
- 先复跑 `parallel-postmortem-action-board-example` live E2E,对照 `.ralph/events.jsonl` 与 `stdout.txt` 做最小可证伪验证。
- 动态证据确认第一轮阻塞不是 runtime 丢事件,而是 `action_owner_mapper` 漂移成输出 `&lt;event ...&gt;` 展示文本,没有真正发布 `actions.ready`。
- 随后收紧了 `examples/parallel-postmortem-action-board/ralph.yml`:
  - worker / facilitator 必须从真实 event 开始标签直接输出
  - 禁止 `&lt;event`、代码块、前后 prose、后续建议
  - README 也同步补了“转义展示文本不算真实事件”的说明
- 第二轮 live E2E 把业务 topic 已打通到 `postmortem.board.ready`,但 reporter 在 `crates/ralph-e2e/src/reporter.rs` 因 `&s[..max_len]` 截断中文 JSON payload 而 panic。
- 我把 reporter 的截断逻辑统一改成按字符边界安全截断,并补了中文回归测试。
- 最后把 `parallel_postmortem_action_board_example` 的最终 payload 断言改成语义匹配,兼容 JSON 与 `key: value` 两种合法 payload 形态。

### 总结感悟
- 对 live backend 的 example,最脆弱的往往不是“主逻辑有没有写”,而是 prompt 是否足够机械到能稳定产出真实事件。
- E2E 框架自己的报告层也必须做 Unicode 安全处理,否则会出现“业务已通过,测试框架自己先炸掉”的假失败。

## 2026-03-10 22:08:00 +0800 任务名称: 扩充第四批真实并行 example 并修复 renewal 的 self-closing event 漂移

### 任务内容
- 新增第四批 3 个真实并行 example:
  - `examples/parallel-security-exception-review`
  - `examples/parallel-customer-renewal-desk`
  - `examples/parallel-audit-evidence-pack`
- 为每个 example 补齐 direct example E2E scenario、注册点、README 与自包含测试。
- 在 live 验证过程中修复 `parallel-customer-renewal-desk-example` 的 self-closing event 漂移问题。

### 完成过程
- 先根据 `specs/parallel-real-world-examples-batch-4.spec.md` 和前三批已通过的 prompt-file 骨架,落了 3 个 example 的 `ralph.yml`、`PROMPT.md`、`README.md`。
- 再新增 3 个 direct example scenario,并补到:
  - `crates/ralph-e2e/src/scenarios/mod.rs`
  - `crates/ralph-e2e/src/lib.rs`
  - `crates/ralph-e2e/src/main.rs`
  - `crates/ralph-cli/tests/integration_examples.rs`
- 文档入口也一并补到:
  - `README.md`
  - `crates/ralph-e2e/README.md`
- live E2E 首轮中,`parallel-customer-renewal-desk-example` 失败。
  - 通过 `.ralph/events.jsonl` 与 `.e2e/stdout.txt` 对照,确认不是 worker 没产出,而是 `commercial_owner` / `sponsor_mapper` 输出了自闭合 `<event .../>`,导致 parser 不记账。
  - 随后只收紧 renewal 与 audit example 的 event 形态约束:
    - 禁止自闭合 `&lt;event .../&gt;`
    - 禁止把业务字段塞进 opening tag 属性
    - 强制开始标签 + payload 正文 + 结束标签
  - 同步补 guard 测试后复跑 live,renewal 即通过。

### 验证结果
- `cargo fmt --all --check` ✅
- 定向测试:
  - `cargo test -p ralph-e2e parallel_security_exception_review_example` ✅
  - `cargo test -p ralph-e2e parallel_customer_renewal_desk_example` ✅
  - `cargo test -p ralph-e2e parallel_audit_evidence_pack_example` ✅
  - `cargo test -p ralph-cli --test integration_examples` ✅
- live E2E:
  - `cargo run -p ralph-e2e -- codex --filter parallel-security-exception-review-example --skip-analysis --keep-workspace --verbose` ✅ `130.5s`
  - `cargo run -p ralph-e2e -- codex --filter parallel-customer-renewal-desk-example --skip-analysis --keep-workspace --verbose` ✅ `131.1s`
  - `cargo run -p ralph-e2e -- codex --filter parallel-audit-evidence-pack-example --skip-analysis --keep-workspace --verbose` ✅ `113.2s`
- 仓库级验证:
  - `cargo test` ✅

### 总结感悟
- 对 direct example 来说,只要求“从真实 event 开始”还不够,还要把 event 形态写死,否则模型会偷懒输出自闭合标签。
- live E2E 最有价值的地方,就是能把这种“stdout 看起来像事件,但 runtime 实际没收到”的假闭环抓出来。

## 2026-03-11 18:12:53 +0800 任务名称: 扩充第五批真实并行 example 到 finance / hiring / onboarding

### 任务内容
- 新增第五批 3 个真实并行 example:
  - `examples/parallel-finance-close-control-room`

## 2026-03-12 16:28:00 +0800 任务名称: OpenSpec `hat-request-reply-channel` 实现闭环

### 任务内容

- 按 OpenSpec change `hat-request-reply-channel` 把 hat-to-hat 答案回流协议真正落到并行运行时。
- 完成协议常量、路由 special-case、resume 恢复索引、可观测性记录、prompt / 文档说明和测试闭环。

### 完成过程

- 在 `crates/ralph-proto` 增加了:
  - `reply.hat.message`
  - `routing.requester_return`
- 在 `ParallelSupervisor` 路由层增加 requester-return 分支:
  - 先记录 `event_id -> source_instance`
  - 再对 `reply.hat.message` 按 `reply=<request_event_id>` 解析原请求方
  - 成功时定向回送给 requester
  - 失败时 fail-closed,并写 requester-return 诊断日志
  - resume 时从历史 `events.jsonl` 恢复该薄索引
- 在并行 prompt 和 `config/all_hat.md` 中补齐 `reply.hat.message` 的中文说明,明确它不是普通 workflow topic。
- 在 `routing_tests.rs` 中补了 4 条覆盖:
  - requester 成功收回答案
  - unknown reply id fail-closed
  - missing `source_instance` fail-closed
  - answer-return 与 workflow event 同批共存

### 验证结果

- `cargo test --package ralph-core --lib busy_ralph_secondary_includes_coordinator_instructions_and_config_prompt` ✅
- `cargo test --package ralph-core --lib parallel::supervisor::routing_tests` ✅
- `cargo fmt --all` ✅
- `cargo fmt --all --check` ✅
- `cargo test -p ralph-core` ✅
- `cargo test` ✅

### 总结感悟

- 这次最关键的点,不是“多一个 topic”,而是把“答案回流”和“流程推进”彻底拆开了。
- 运行时只用最薄的一层 `event_id -> source_instance` 索引就能完成 requester-return,这是对现有结构侵入较小、但语义提升很大的改良。

## 2026-03-13 00:16:00 +0800 任务名称: 归档 OpenSpec `hat-request-reply-channel`

### 任务内容

- 对已完成实现的 `hat-request-reply-channel` 做主 specs sync 与 archive 收尾。
- 保证归档后主 specs 中仍然保留这项能力定义,而不是只留在 change archive 里。

### 完成过程

- 先检查了:
  - `openspec status --change "hat-request-reply-channel" --json`
  - `openspec/changes/hat-request-reply-channel/tasks.md`
- 确认 artifacts 全部完成,任务 9/9 已完成。
- 因为主 specs 中还没有对应 capability,所以先新建:
  - `openspec/specs/hat-request-reply-channel/spec.md`
- 同步完成后执行:
  - `openspec validate hat-request-reply-channel --type change`
- 然后将 change 目录移动到:
  - `openspec/changes/archive/2026-03-13-hat-request-reply-channel/`
- 最后用 `openspec list --json` 确认 active changes 已不再包含它。

### 总结感悟

- OpenSpec archive 真正重要的不是“把目录移走”,而是确保主 specs 已经接住这次 change 的长期语义。
- 对这种新增 capability 的 change,归档前同步主 spec 是非常值得坚持的动作,否则 archive 后主线知识会断层。
  - `examples/parallel-hiring-debrief-panel`
  - `examples/parallel-customer-onboarding-activation`
- 为每个 example 补齐 direct example E2E scenario、注册点、README 与自包含测试。
- 完成 mermaid 校验、定向测试、3 条 live E2E 与仓库级验证。

### 完成过程
- 先回读六文件、batch-4 成功模板和 `specs/parallel-real-world-examples-batch-5.spec.md`,确认继续沿用:
  - `prompt_file: "PROMPT.md"`
  - 4 lane + 1 fan-in request + 1 final topic
  - coordinator 未收齐 ready 前静默
  - worker / finalizer event-only
- 用 `beautiful-mermaid-rs --ascii` 校验 batch-5 spec 的 2 个 mermaid block,确保图表语法无误。
- 新增 3 个 direct example scenario:
  - `crates/ralph-e2e/src/scenarios/parallel_finance_close_control_room_example.rs`
  - `crates/ralph-e2e/src/scenarios/parallel_hiring_debrief_panel_example.rs`
  - `crates/ralph-e2e/src/scenarios/parallel_customer_onboarding_activation_example.rs`
- 同步接线到:
  - `crates/ralph-e2e/src/scenarios/mod.rs`
  - `crates/ralph-e2e/src/lib.rs`
  - `crates/ralph-e2e/src/main.rs`
  - `crates/ralph-cli/tests/integration_examples.rs`
- README 入口同步到:
  - `README.md`
  - `crates/ralph-e2e/README.md`
- 中途碰到的唯一小插曲是:
  - `rustfmt` 默认按旧 edition 解释新文件
  - 改成 `rustfmt --edition 2024` 后即恢复正常

### 验证结果
- `cargo fmt --all --check` ✅
- `cargo test -p ralph-e2e parallel_finance_close_control_room_example` ✅
- `cargo test -p ralph-e2e parallel_hiring_debrief_panel_example` ✅
- `cargo test -p ralph-e2e parallel_customer_onboarding_activation_example` ✅
- `cargo test -p ralph-cli --test integration_examples` ✅
- live E2E:
  - `parallel-finance-close-control-room-example` ✅ `89.1s`
  - `parallel-hiring-debrief-panel-example` ✅ `81.4s`
  - `parallel-customer-onboarding-activation-example` ✅ `87.1s`
- 仓库级验证:
  - `cargo test` ✅

### 总结感悟
- 这批场景继续证明了一个很有价值的点:
  - 并行 example 不是只能服务工程和治理
  - 也能稳定表达 finance、people、post-sales activation 这类真实运营工作流
- 当前 direct example 方法已经形成比较稳的建模套路:
  - coordinator 静默等待
  - terminal topic 明确 owner
  - event 形态写死为 `<event ...>payload</event>`
  - payload 断言兼容 JSON / `key: value`

## 2026-03-11 21:17:47 +0800 任务名称: 扩充第六批真实并行 example 到 support / partner / field enablement

### 任务内容
- 新增第六批 3 个真实并行 example:
  - `examples/parallel-support-escalation-desk`
  - `examples/parallel-partner-launch-coordination`
  - `examples/parallel-field-enablement-rollout`
- 为每个 example 补齐 direct example E2E scenario、注册点、README 与测试入口。
- 完成 mermaid 校验、定向测试、3 条 live E2E、仓库级验证,以及六文件续档收尾。

### 完成过程
- 先回读六文件与 batch-5 结果,确认继续沿用已经验证过的 direct example 骨架:
  - `prompt_file: "PROMPT.md"`
  - `4 lane + 1 fan-in request + 1 final topic`
  - coordinator 未收齐 ready 前静默
  - worker / finalizer 继续 event-only
- 为了避免题材重复,本轮刻意避开:
  - incident
  - launch
  - vendor procurement
  - onboarding
  - approval
- 最终选择了 3 个更偏实际运营协同的场景:
  - support escalation desk
  - partner launch coordination
  - field enablement rollout
- 用 `beautiful-mermaid-rs --ascii` 校验 `specs/parallel-real-world-examples-batch-6.spec.md` 中的 2 个 mermaid block。
- 新增 3 个 direct example scenario:
  - `crates/ralph-e2e/src/scenarios/parallel_support_escalation_desk_example.rs`
  - `crates/ralph-e2e/src/scenarios/parallel_partner_launch_coordination_example.rs`
  - `crates/ralph-e2e/src/scenarios/parallel_field_enablement_rollout_example.rs`
- 同步接线到:
  - `crates/ralph-e2e/src/scenarios/mod.rs`
  - `crates/ralph-e2e/src/lib.rs`
  - `crates/ralph-e2e/src/main.rs`
  - `crates/ralph-cli/tests/integration_examples.rs`
- README 入口同步到:
  - `README.md`
  - `crates/ralph-e2e/README.md`
- 收尾时按超过 1000 行规则续档了:
  - `task_plan.md` -> `archive/task_plan_20260311-211747.md`
  - `notes.md` -> `archive/notes_20260311-211747.md`

### 验证结果
- `cargo fmt --all --check` ✅
- `cargo test -p ralph-e2e parallel_support_escalation_desk_example` ✅
- `cargo test -p ralph-e2e parallel_partner_launch_coordination_example` ✅
- `cargo test -p ralph-e2e parallel_field_enablement_rollout_example` ✅
- `cargo test -p ralph-cli --test integration_examples` ✅
- live E2E:
  - `parallel-support-escalation-desk-example` ✅ `96.1s`
  - `parallel-partner-launch-coordination-example` ✅ `130.0s`
  - `parallel-field-enablement-rollout-example` ✅ `98.8s`
- 仓库级验证:
  - `cargo test` ✅

### 总结感悟
- 这轮最有价值的点不是“又多了 3 个 demo”,而是继续证明了 direct example 方法的横向泛化能力。
- 当前真实并行 example 已经稳定扩到 support、ecosystem、enablement,这会让用户更容易把 Ralph 理解成通用协作编排器,而不是只偏工程链路。
- 本轮没有新增架构级 epiphany,说明我们前几批收敛出的协议骨架已经比较稳:
  - coordinator 静默等待
  - finalizer 明确 owner
  - event 形态强约束
  - 固定终态字段利于 live E2E 锁定

## 2026-03-11 22:52:08 +0800 任务名称: 把真实并行 example 的方案和描述收敛成中文

### 任务内容
- 为真实并行 example 新增一份集中式中文总览。
- 把根 README、`crates/ralph-e2e/README.md` 以及 batch-6 的 3 个 example README 里的相关描述改成中文。
- 保留路径名、topic、类名这些技术标识不变,只调整面向人的说明文本。

### 完成过程
- 先回读现有文档入口,确认英文描述主要分布在:
  - `README.md`
  - `crates/ralph-e2e/README.md`
  - batch-6 三个 example README
- 新增 `docs/examples/parallel-real-world-examples.zh-CN.md`,把以下内容集中收敛:
  - 共用方案骨架
  - 按题材分组的快速选型
  - 全量范例矩阵
  - batch-6 三个场景的中文解释
  - 后续扩批建议
- 同步把根 README 的 parallel example 列表描述切换成中文,并挂上中文总览入口。
- 同步把 `crates/ralph-e2e/README.md` 中 parallel example scenario 的说明切换成中文。
- 继续打磨 batch-6 三个 example README,把标题、用途说明和运行期说明都改得更像自然中文。

### 验证结果
- `git diff --check -- README.md crates/ralph-e2e/README.md docs/examples/parallel-real-world-examples.zh-CN.md examples/parallel-support-escalation-desk/README.md examples/parallel-partner-launch-coordination/README.md examples/parallel-field-enablement-rollout/README.md` ✅

### 总结感悟
- 这轮没有改任何并行 runtime 逻辑,但对外说明层的价值很高。
- 当 example 数量越来越多时,如果没有中文总览和按题材分组的入口,用户会更容易“看到目录,但不知道先看哪个”。
- 现在这套 example 已经不只是跑得通,也更接近“可教、可选、可复用”的范例库了。

## 2026-03-11 23:08:21 +0800 任务名称: 统一 batch-1 到 batch-5 的中文 README,并接入 docs 入口

### 任务内容
- 把 batch-1 到 batch-5 的真实并行 example README 统一成中文风格。
- 把中文总览接入 `docs/examples/index.md`,让 docs 入口也能直接找到中文方案页。

### 完成过程
- 先用 `rg` 扫描 batch-1 到 batch-5 的 README,把残留的说明性英文摸出来。
- 统一重写了 15 个 README 的标题、开场说明、用途说明和替换说明。
- 统一保留:
  - 路径名
  - topic 名
  - 类名 / hat 名
  - prompt section 标题
- 这样既保证中文可读性,也不破坏和代码、prompt 的对齐关系。
- 继续把 `docs/examples/index.md` 改成更偏中文的入口页,并把 `parallel-real-world-examples.zh-CN.md` 正式挂进去。

### 验证结果
- `git diff --check -- docs/examples/index.md docs/examples/parallel-real-world-examples.zh-CN.md examples/parallel-pr-review/README.md examples/parallel-release-checklist/README.md examples/parallel-human-approval-gate/README.md examples/parallel-incident-response-war-room/README.md examples/parallel-security-exception-review/README.md examples/parallel-customer-renewal-desk/README.md examples/parallel-audit-evidence-pack/README.md examples/parallel-finance-close-control-room/README.md examples/parallel-hiring-debrief-panel/README.md examples/parallel-customer-onboarding-activation/README.md examples/parallel-launch-readiness-command/README.md examples/parallel-migration-rehearsal/README.md examples/parallel-postmortem-action-board/README.md examples/parallel-proposal-assembly/README.md examples/parallel-vendor-security-procurement/README.md task_plan.md notes.md WORKLOG.md` ✅

### 总结感悟
- 这轮最重要的成果不是“翻译了 15 个 README”,而是把整条 example 阅读路径补齐了。
- 现在用户可以:
  - 先从 docs 入口看到中文总览
  - 再按题材选具体范例
  - 进入单个 example 后继续看到统一风格的中文说明
- 这会让后续继续扩 batch-7 时,新范例也有现成的中文文档模板可复用。

## 2026-03-12 00:52:00 +0800 任务名称: batch-7 真实并行范例落地与 live E2E 验收

### 任务内容
- 新增第七批 3 个真实并行 example:
  - `examples/parallel-revops-quote-desk`
  - `examples/parallel-executive-business-review-prep`
  - `examples/parallel-customer-advisory-board-prep`
- 为每个 example 补齐 direct example E2E scenario、共享注册点、中文文档入口和测试入口。
- 完成 mermaid 校验、定向测试、3 条真实 Codex live E2E、仓库级 `cargo test` 与六文件收尾。

### 完成过程
- 先回读六文件和 batch-6 结果,确认 batch-7 继续沿用已验证稳定的 direct example 骨架:
  - `prompt_file: "PROMPT.md"`
  - 4 lane fanout
  - 1 次 fan-in request
  - 1 个 final topic
  - coordinator 未收齐 ready 前保持静默
- 新增并接线:
  - `crates/ralph-e2e/src/scenarios/parallel_revops_quote_desk_example.rs`
  - `crates/ralph-e2e/src/scenarios/parallel_executive_business_review_prep_example.rs`
  - `crates/ralph-e2e/src/scenarios/parallel_customer_advisory_board_prep_example.rs`
  - `crates/ralph-e2e/src/scenarios/mod.rs`
  - `crates/ralph-e2e/src/lib.rs`
  - `crates/ralph-e2e/src/main.rs`
  - `crates/ralph-cli/tests/integration_examples.rs`
- 中文入口同步到:
  - `README.md`
  - `crates/ralph-e2e/README.md`
  - `docs/examples/parallel-real-world-examples.zh-CN.md`
- 过程中修掉了 2 个 live E2E 才会暴露的问题:
  - batch-7 scenario 直接依赖截断版 `events.jsonl` payload,导致 quote 首轮出现假失败
  - `billing_setup_reviewer` 在真实 backend 下把多行 event 的 closing tag 写坏,导致 quote 第二轮真超时
- 为此补了两个稳定器:
  - 并行 stdout payload 提取 helper,优先从去前缀后的 `:out:job=` 行提取最终 event payload
  - quote billing lane 改成“单行 JSON + 精确 `&lt;/event&gt;`”的硬约束

### 验证结果
- `beautiful-mermaid-rs --ascii` 校验 `specs/parallel-real-world-examples-batch-7.spec.md` 两个 mermaid 图 ✅
- `cargo test -p ralph-e2e parallel_revops_quote_desk_example` ✅
- `cargo test -p ralph-e2e parallel_executive_business_review_prep_example` ✅
- `cargo test -p ralph-e2e parallel_customer_advisory_board_prep_example` ✅
- `cargo test -p ralph-cli --test integration_examples` ✅
- live E2E:
  - `cargo run -p ralph-e2e -- codex --filter parallel-revops-quote-desk-example --skip-analysis --keep-workspace --verbose` ✅ `184.5s`
  - `cargo run -p ralph-e2e -- codex --filter parallel-executive-business-review-prep-example --skip-analysis --keep-workspace --verbose` ✅ `146.6s`
  - `cargo run -p ralph-e2e -- codex --filter parallel-customer-advisory-board-prep-example --skip-analysis --keep-workspace --verbose` ✅ `102.4s`
- `cargo test` ✅
- `cargo fmt --all --check` ✅
- `git diff --check` ✅

### 总结感悟
- 这轮最有价值的收获不是“多了 3 个范例”,而是又把 direct example 方法往商业协同场景扩了一层:
  - 报价台
  - 管理层业务回顾
  - 客户顾问委员会筹备
- 另一条很重要的规律是:
  - live backend 下,scenario 断言不能盲信截断版 `events.jsonl`
  - worker 如果允许多行 line-style event,就更容易把 closing tag 写坏
- 现在 batch-7 不只是静态文件齐了,而是真的在真实 Codex backend 下闭环通过了。

## 2026-03-12 13:47:00 +0800 任务名称: batch-8 真实并行范例落地与 live E2E 验收

### 任务内容
- 新增第八批 3 个真实并行 example:
  - `examples/parallel-regional-operating-review`
  - `examples/parallel-renewal-risk-calibration`
  - `examples/parallel-multi-region-pipeline-sync`
- 为每个 example 补齐 direct example E2E scenario、共享注册点、中文文档入口和测试入口。
- 完成 mermaid 校验、定向测试、3 条真实 Codex live E2E、仓库级 `cargo test` 与六文件收尾。

### 完成过程
- 先回读六文件和 batch-7 的验证规律,确认 batch-8 继续沿用已证明稳定的骨架:
  - `prompt_file: "PROMPT.md"`
  - 4 lane fanout
  - 1 次 fan-in request
  - 1 个 final topic
  - coordinator 未收齐 ready 前保持静默
  - 最终 payload 断言优先从去掉 `[hat#n:out:job=m]` 前缀后的 stdout out 行提取
- 新增并接线:
  - `crates/ralph-e2e/src/scenarios/parallel_regional_operating_review_example.rs`
  - `crates/ralph-e2e/src/scenarios/parallel_renewal_risk_calibration_example.rs`
  - `crates/ralph-e2e/src/scenarios/parallel_multi_region_pipeline_sync_example.rs`
  - `crates/ralph-e2e/src/scenarios/mod.rs`
  - `crates/ralph-e2e/src/lib.rs`
  - `crates/ralph-e2e/src/main.rs`
  - `crates/ralph-cli/tests/integration_examples.rs`
- 中文入口同步到:
  - `README.md`
  - `crates/ralph-e2e/README.md`
  - `docs/examples/parallel-real-world-examples.zh-CN.md`
- 过程中修掉了 1 个真实 live E2E 才会暴露的问题:
  - `renewal-risk-calibration` 的 `success_plan_reviewer` 首轮没有产出 `success.ready`,导致整个 fan-in 永远不触发
- 为此补了 1 个稳定器:
  - 把 `success_plan_reviewer` 从“单行 JSON 原则约束”进一步收紧成“唯一允许模板 + 固定值 `risk_playbooks_assigned`”

### 验证结果
- mermaid 校验:
  - `sed -n '73,83p' specs/parallel-real-world-examples-batch-8.spec.md | beautiful-mermaid-rs --ascii` ✅
  - `sed -n '89,107p' specs/parallel-real-world-examples-batch-8.spec.md | beautiful-mermaid-rs --ascii` ✅
- 定向测试:
  - `cargo test -p ralph-e2e parallel_regional_operating_review_example` ✅
  - `cargo test -p ralph-e2e parallel_renewal_risk_calibration_example` ✅
  - `cargo test -p ralph-e2e parallel_multi_region_pipeline_sync_example` ✅
  - `cargo test -p ralph-cli --test integration_examples` ✅
- live E2E:
  - `cargo run -p ralph-e2e -- codex --filter parallel-regional-operating-review-example --skip-analysis --keep-workspace --verbose` ✅ `130.9s`
  - `cargo run -p ralph-e2e -- codex --filter parallel-renewal-risk-calibration-example --skip-analysis --keep-workspace --verbose` 首轮 ❌ `240.1s`
  - `cargo run -p ralph-e2e -- codex --filter parallel-renewal-risk-calibration-example --skip-analysis --keep-workspace --verbose` 修复后复跑 ✅ `172.9s`
  - `cargo run -p ralph-e2e -- codex --filter parallel-multi-region-pipeline-sync-example --skip-analysis --keep-workspace --verbose` ✅ `216.9s`
- `cargo fmt --all --check` ✅
- `git diff --check -- <batch-8相关文件>` ✅
- `cargo test` ✅

### 总结感悟
- batch-8 把真实并行 example 继续从“商业协同”推进到了“经营节奏与预测校准”。
- 这轮最重要的新规律是:
  - 有些看起来已经足够严格的 worker lane,在真实 backend 下仍然不够机械
  - 对漂移风险更高的 lane,要直接给唯一允许模板,不要只给原则说明
- 到 batch-8 为止,这套范例已经能同时覆盖:
  - 单一区域经营周会
  - 续费组合盘预测校准
  - 多区域 pipeline 同步

## [2026-03-12 14:47:00 +0800] 任务名称: 为“hat 回答回到请求方”协议拆出独立 OpenSpec change

### 任务内容
- 将“hat 的答案回流给请求方”从 `event-id-and-reply` 中独立出来
- 新建独立 change 骨架,避免把事件关联语义和回答回传语义混在一起

### 完成过程
- 回读了现有 OpenSpec change 与 parallel runtime,确认 `reply` 目前只表达事件关联,不表达回送目标
- 根据用户选择,执行 `openspec new change "hat-request-reply-channel"`
- 确认新 change 使用 `spec-driven` workflow
- 读取 `openspec status` 与 `openspec instructions proposal`,确认首个 artifact 是 `proposal`

### 总结感悟
- 把“事件关联”和“答案回流”拆成两个 change,后续协议边界会更稳定
- explorer / researcher 类 hat 更适合显式 request-reply 语义,不适合被塞进“所有 hat 默认 final 回传”的全局规则

## [2026-03-12 15:10:00 +0800] 任务名称: fast-forward 完成 `hat-request-reply-channel` 的 OpenSpec artifacts

### 任务内容
- 为 `hat-request-reply-channel` 一次性生成 `proposal`、`design`、`specs`、`tasks`
- 将“hat 的答案回流给请求方”沉淀为独立 capability,达到 apply-ready 状态

### 完成过程
- 读取 `openspec status --change ... --json` 与每个 artifact 的 `openspec instructions ... --json`
- 先落 proposal,确认 capability 采用新增 `hat-request-reply-channel`
- 再落 design,把协议收敛为显式 `reply.hat.message` + 基于 `reply` 查原请求 `source_instance` 的 requester-return
- 再落 spec,把显式 opt-in、回送目标、双通道并存、fail-closed、关联保留写成 MUST requirements
- 最后落 tasks,拆成协议与路由、失败收口与可观测性、验证与示例 3 组实现任务
- 使用 `beautiful-mermaid-rs` 校验 design 中的 flowchart 与 sequenceDiagram
- 使用 `openspec validate hat-request-reply-channel --type change` 确认 change 有效

### 总结感悟
- 把“事件关联”与“答案回流”拆成两个 change 后,协议边界明显更稳了
- `reply.hat.message` 这种显式 topic 比“让所有 reply 自动回送”更可控,也更适合未来做测试和诊断
