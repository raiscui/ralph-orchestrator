# WORKLOG.md

> Previous WORKLOG exceeded 1000 lines and was rolled to `WORKLOG_2026-05-13_1937_prev.md` during the Ralph Phase 1A OpenSpec task.


## [2026-05-13 19:37:49] [Session ID: 019e20e7-7092-78c1-b5f1-b34db5f243b8] 任务名称: Ralph Phase 1A runtime evidence index OpenSpec

### 任务内容
- 按 `.omx/plans/ralph-evolution-roadmap-consensus-draft.md` 只推进 Phase 1A minimal evidence index kernel。
- 启动前重新验证 staged diff 为空。
- 产出 OpenSpec 和测试计划,不实现 Rust 代码。

### 完成过程
- 创建 Ralph context snapshot: `.omx/context/runtime-evidence-index-kernel-20260513T110024Z.md`。
- 创建 OpenSpec change: `openspec/changes/runtime-evidence-index-kernel/`。
- 写入 `proposal.md`, `design.md`, `specs/runtime-evidence-index-kernel/spec.md`, `tasks.md`, `test-plan.md`。
- 修复 spec delta 格式,加入 `## ADDED Requirements`。
- 对本轮文档做 anti-slop / boundary review,确认 Phase 1B CLI / doctor 只作为非目标和 guardrail 出现。
- 因旧 `WORKLOG.md` 超过 1000 行,追加经验到 `EXPERIENCE.md` 并续档旧 WORKLOG。

### 验证证据
- `git diff --cached --name-status`: 空。
- `openspec validate runtime-evidence-index-kernel --type change`: valid。
- `openspec validate --all --strict`: 25 passed,0 failed。
- `openspec show runtime-evidence-index-kernel --json --deltas-only`: deltaCount=5。
- `git diff --check`: 通过,无输出。

### 总结感悟
- Phase 1A 的价值在于把 artifact link 和 correlation contract 钉住,不要提前做 evidence CLI / doctor 平台化能力。
- OpenSpec change 的 spec 文件必须使用 delta section,例如 `## ADDED Requirements`,否则 validate 会报 `No delta sections found`。
- 写 Markdown 日志时必须继续使用 quoted heredoc 或安全写入方式,避免反引号触发 shell command substitution。


## [2026-05-13 22:10:12] [Session ID: 019e20e7-7092-78c1-b5f1-b34db5f243b8] 任务名称: runtime-evidence-index-kernel Phase 1A 实现

### 任务内容
- 按 OpenSpec change `runtime-evidence-index-kernel` 实现 Phase 1A minimal evidence index kernel。
- 保持边界: 不实现 evidence CLI / doctor UX,不改变 live topology。

### 完成过程
- 新增 `crates/ralph-core/src/evidence_index.rs`。
- 在 `crates/ralph-core/src/lib.rs` 导出 evidence index public API。
- 采用 JSONL 作为 v1 index storage,与 record-session / events evidence 流保持一致。
- 实现最小类型:
  - `EvidenceIndexEntry`
  - `EvidenceArtifactKind`
  - `EvidenceStatus`
  - `EvidenceIndexWriter`
  - `EvidenceIndexReader`
  - `EvidenceLookup`
  - `EvidenceIndexError`
- 添加 7 个 contract tests,覆盖 schema、writer/reader、missing marker、parent-child link、record-session artifact、event-log artifact、runtime graph 非真相源 guardrail。
- 更新 OpenSpec tasks,标记 implementation 和 verification 项完成。

### 验证证据
- `cargo test --package ralph-core --lib evidence_index::tests`: 7 passed。
- `cargo test --package ralph-core --lib event_logger::tests::test_runtime_durable_payloads_are_not_truncated -- --exact`: 1 passed。
- `cargo test --package ralph-core --lib session_recorder::tests::test_record_session_critical_sequence_strict_parseable_after_flush -- --exact`: 1 passed。
- `cargo test --package ralph-core --lib session_recorder::tests::test_critical_records_flush_to_file_before_recorder_drop -- --exact`: 1 passed。
- `cargo test --package ralph-cli --bin ralph capability::tests::isolated_invocation_writes_auditable_artifacts_without_parent_topology_mutation -- --exact`: 1 passed。
- `cargo test -p ralph-core smoke_runner`: 12 passed in smoke runner unit tests; related filtered integration/doc targets completed with 0 failures。
- `cargo test`: passed,包括 workspace unit tests 和 doctests。
- `cargo fmt --all -- --check`: passed。
- `git diff --check`: passed。
- `openspec validate runtime-evidence-index-kernel --type change`: valid。
- `openspec validate --all --strict`: 25 passed,0 failed。
- `cargo clippy --all-targets --all-features`: exit code 0; 输出既有 warning,未发现本轮新增 `evidence_index.rs` warning。

### 总结感悟
- Phase 1A 的实现应保持在 core module,而不是 CLI/doctor 层,这样才能避免 evidence 平台化。
- JSONL writer/reader 与现有 evidence 流一致,足够支撑后续 request/reply 和 capability v2 的 artifact lookup。
- 后续 Phase 2/3 接入时,应只调用 writer 登记 artifact link,不要让 index 接管原始证据内容。


## [2026-05-13 22:20:17] [Session ID: omx-1778510695653-7pd7o2] 任务名称: runtime-evidence-index-kernel post-audit 收尾

### 任务内容
- 接续上轮 Phase 1A 实现后的 completion audit gate。
- 复核审计文件、OpenSpec tasks 和核心实现,再重跑关键验证。

### 完成过程
- 读取 `.omx/audits/runtime-evidence-index-kernel-completion-audit.md`,确认审计覆盖 prompt、scope、artifact checklist、验证命令和 known gaps。
- 读取 `openspec/changes/runtime-evidence-index-kernel/tasks.md`,确认任务全部完成。
- 读取 `crates/ralph-core/src/evidence_index.rs` 与 `crates/ralph-core/src/lib.rs`,确认 public API 与 Phase 1A 边界一致。
- 重跑 focused tests、smoke tests、全量测试、OpenSpec validate、format 和 diff gate。

### 验证证据
- `cargo test --package ralph-core --lib evidence_index::tests`: 7 passed,0 failed。
- `openspec validate runtime-evidence-index-kernel --type change`: valid。
- `git diff --check`: exit 0。
- `cargo fmt --all -- --check`: exit 0。
- `cargo test -p ralph-core smoke_runner`: 12 passed,0 failed。
- `cargo test`: workspace unit tests 与 doctests 通过。
- `openspec validate --all --strict`: 25 passed,0 failed。

### 总结感悟
- completion audit 不是文档装饰,而是 Ralph 闭环的一等 evidence artifact。
- 未来类似实现收尾时,应先写审计,再重跑最小验证门,最后关闭 state。


## [2026-05-13 22:31:33] [Session ID: omx-1778510695653-7pd7o2] 任务名称: Ralph completion audit hook 兼容修正

### 任务内容
- 响应 Stop hook 的 `missing_completion_audit` 阻塞。
- 将人类可读 Markdown audit 补为 hook 可解析的结构化 JSON audit。

### 完成过程
- 读取 hook 实现 `/Users/cuiluming/n/lib/node_modules/oh-my-codex/dist/ralph/completion-audit.js`。
- 确认 hook 只接受 state 内联 audit object 或 repo-relative `.json` artifact。
- 创建 `.omx/audits/runtime-evidence-index-kernel-completion-audit.json`。
- 更新 Ralph state,将 completion audit path/evidence path 指向 JSON,并内联 `completion_audit` 对象。

### 验证证据
- `node --input-type=module ... evaluateRalphCompletionAuditEvidence(...)`: 返回 `complete=true`, `reason=completion_audit_passed`, `source=state`。
- `omx state read --input '{"mode":"ralph"}' --json`: `active=false`, `current_phase=complete`, `completion_audit_gate=passed`。
- `omx state list-active --json`: `active_modes=[]`。

### 总结感悟
- Ralph completion audit 给人看的 Markdown 不等于 hook 可解析 evidence。
- Stop hook 的 completion audit 证据必须写成 JSON artifact 或内联 state object。


## [2026-05-14 11:22:42] [Session ID: omx-1778510695653-7pd7o2] 任务名称: Phase 1A 提交归档与 Phase 2 OpenSpec 准备

### 任务内容
- 按用户指定顺序继续:
  - review 当前 diff。
  - 做本地提交。
  - archive OpenSpec change。
  - 准备下一条产品演进线 Phase 2 request/reply answer return 最小闭环。

### 完成过程
- Review 了 Phase 1A diff,确认 `.omx/` audit/state 被 ignore,不会进入提交。
- 做了本地提交 `cadefa8 Build evidence lookup before evidence UX`。
- 执行 `openspec archive runtime-evidence-index-kernel --yes`,并修正归档生成的主 spec `Purpose TBD`。
- 做了 archive 提交 `0e00eb7 Archive evidence index contract after kernel landing`。
- 创建 Phase 2 OpenSpec change `request-reply-answer-evidence`,只产出 proposal/design/spec/tasks/test-plan,未实现代码。
- 做了 Phase 2 规格提交 `e18536d Specify answer-return evidence before wiring runtime`。

### 验证证据
- Phase 1A 提交前:
  - `cargo fmt --all -- --check`: passed。
  - `git diff --check`: passed。
  - `openspec validate runtime-evidence-index-kernel --type change`: valid。
  - `openspec validate --all --strict`: 25 passed,0 failed。
  - `cargo test --package ralph-core --lib evidence_index::tests`: 7 passed,0 failed。
  - `cargo test -p ralph-core smoke_runner`: 12 passed,0 failed。
  - `cargo test`: workspace unit tests and doctests passed。
- Archive 后:
  - `openspec validate --all --strict`: 25 passed,0 failed。
  - `git diff --check`: passed。
- Phase 2 OpenSpec:
  - `openspec validate request-reply-answer-evidence --type change`: valid。
  - `openspec validate --all --strict`: 26 passed,0 failed。
  - `git diff --check`: passed。
- 最终状态:
  - `git status --short --untracked-files=all`: clean。
  - active OpenSpec changes: `request-reply-answer-evidence` 和既有无关 `tui-mdfried-viewer`。

### 总结感悟
- Phase 1A 已完成并归档,后续实现应从 `openspec/specs/runtime-evidence-index-kernel/spec.md` 读取稳定 contract。
- Phase 2 的正确切入点不是继续做 CLI UX,而是先把 `reply.hat.message` 的成功、失败、missing/timeout 证据写进 evidence index。
- OpenSpec CLI 的 PostHog flush 网络错误会出现在 stderr,但本轮相关命令退出码为 0,内容验证通过;不要把遥测 flush 噪声误当规格失败。

## [2026-05-14 13:04:00] [Session ID: codex-20260514-phase2] 任务名称: Phase 2 request/reply answer evidence runtime wiring

### 任务内容
- 实现 OpenSpec change `request-reply-answer-evidence` 的 Phase 2 最小 runtime 闭环。
- 将 `reply.hat.message` requester-return 的成功、失败、missing marker 证据写入 Phase 1A evidence index。
- 保持 routing 边界: 普通带 `reply` 的 workflow event 不被当成 answer-return evidence,内部 `reply.hat.message` 不自动生成 `reply.human.message`。

### 完成过程
- 续档超过 1000 行的 `task_plan.md`,旧文件保存为 `task_plan_2026-05-14_phase1a_phase2_prev.md`。
- 重读 Phase 2 OpenSpec、`runtime-evidence-index-kernel` 和 `hat-request-reply-channel` 稳定 spec。
- 在 `ParallelSupervisor` 内部增加 evidence index writer,默认路径为 `.ralph/evidence-index.jsonl`。
- 在现有 `reply.hat.message` success / fail-closed 路由分支中写入 evidence index entry。
- 暴露 `ParallelSupervisor::record_missing_answer_evidence()` 作为 missing/timeout marker 的最小显式入口。
- 补齐 focused tests 覆盖 success、unknown request id、missing source_instance、no reply、missing marker、ordinary workflow boundary、human-visible boundary。
- 更新 OpenSpec tasks 进度。

### 验证证据
- `cargo test --package ralph-core --lib parallel::supervisor::routing_tests`: 48 passed。
- `cargo test --package ralph-core --lib evidence_index::tests`: 7 passed。
- `cargo test -p ralph-core smoke_runner`: 12 passed,无 warning。
- `cargo test`: workspace tests and doctests passed。
- `cargo fmt --all -- --check`: passed。
- `git diff --check`: passed。
- `openspec validate request-reply-answer-evidence --type change`: valid。
- `openspec validate --all --strict`: 26 passed,0 failed。

### 总结感悟
- Phase 2 的关键不是新建 request broker,而是把已有 `reply.hat.message` requester-return 分支变成可查证的 evidence producer。
- Evidence index 的 `producer` 字段必须保持写入者身份,失败原因应留在原始 JSONL artifact 里,不要借字段塞语义。
- Missing/timeout 先用显式 marker API 收口,不要为了 Phase 2 最小闭环引入 broad lifecycle broker。

## [2026-05-14 14:38:00] [Session ID: omx-1778510695653-7pd7o2] 任务名称: Archive Phase 2 OpenSpec,收口 continuous-learning,并 dogfood answer evidence

### 任务内容
- 按用户指定顺序继续:
  - archive `request-reply-answer-evidence` OpenSpec change。
  - 执行 `task_plan` 续档触发的 continuous-learning 收口。
  - 选择下一条演进线中的 `live runtime answer evidence dogfood`,证明 Phase 2 evidence index 已服务真实 runtime 链路。
  - 本地提交,不 push。

### 完成过程
- 已将 `request-reply-answer-evidence` 从 active change 归档到 `openspec/changes/archive/2026-05-14-request-reply-answer-evidence/`。
- 已生成稳定规格 `openspec/specs/request-reply-answer-evidence/spec.md`,并把 archive 默认生成的 `Purpose TBD` 改成可读的正式 Purpose。
- 已执行 continuous-learning 收口:
  - 在 `notes.md` 写入六文件摘要。
  - 在 `EXPERIENCE.md` 新增 `exp-20260514-request-reply-answer-evidence-boundary`。
  - 将已覆盖的默认历史文件移动到 `archive/default_history/`。
  - 将已覆盖的旧支线六文件移动到 `archive/branch_contexts/<topic>/`。
  - 新增 `archive/manifests/ARCHIVE_MANIFEST__task_plan_rollover_2026-05-14_1358.md`。
  - 清理 `LATER_PLANS.md` 中已完成的 continuous-learning 待办。
- 已新增 `crates/ralph-cli/tests/integration_answer_evidence.rs`:
  - 通过真实 `ralph run --no-tui --record-session` 启动 parallel runtime。
  - custom backend 按 `RALPH_HAT_INSTANCE_ID` 分流 `ralph#1` 和 `researcher#1`。
  - 触发 `research.request` -> `reply.hat.message reply="req-dogfood-1"` -> `LOOP_COMPLETE`。
  - 断言 `.ralph/evidence-index.jsonl` 可按 request id 和 answer id 查到 evidence。
  - 断言 `.ralph/events.jsonl` 包含 delivered `routing.requester_return` 记录。
  - 断言 record-session 包含 `_meta.termination` / `CompletionPromise`。

### 验证证据
- `cargo test`: workspace tests and doctests passed,exit 0。
- `cargo fmt --all -- --check`: passed。
- `cargo test -p ralph-cli --test integration_answer_evidence`: 1 passed,0 failed。
- `cargo test -p ralph-core smoke_runner`: 12 passed,0 failed。
- `openspec validate --all --strict`: 26 passed,0 failed。
- `git diff --check`: passed。
- `git submodule status`: no submodules listed。

### 总结感悟
- Phase 2 的价值点已经从 standalone evidence kernel 前进到真实 runtime dogfood: request/reply answer return 不再只是 core 单测,而是通过 CLI 运行产出 `.ralph/evidence-index.jsonl`、`.ralph/events.jsonl` 和 record-session 证据。
- answer evidence 的单一真相源仍然是 durable JSONL event log; evidence index 是 lookup surface,不是替代事实源。
- 下一条自然演进线可以进入 Phase 3: capability invocation / child run evidence 真实串联,但应继续避免热改 live topology。

## [2026-05-14 15:52:00] [Session ID: omx-1778510695653-7pd7o2] 任务名称: Phase 3 capability child-run evidence 真实串联

### 任务内容
- 进入 Phase 3: capability invocation / child run evidence 真实串联。
- 目标是在现有 isolated child/micro-run 路径上补 evidence index linkage,而不是新增第二套 runtime broker 或热改 live topology。

### 完成过程
- 新建 OpenSpec change `capability-child-run-evidence`,包含 proposal/design/delta spec/tasks/test-plan。
- 阅读现有 capability invocation 实现后确认最小缺口:
  - 已有 `invoke.json` / `result.json` / `failed.json` / `resolved-config.yml` / `.ralph/events.jsonl`。
  - 缺少 `.ralph/evidence-index.jsonl` 中按 invocation id 可查的 durable linkage。
- 扩展 `crates/ralph-cli/tests/integration_capability.rs`,让真实 `ralph tools capability invoke` 查询 evidence index。
- 先跑红灯验证,确认测试失败在 evidence lookup 缺失。
- 在 `crates/ralph-cli/src/capability.rs` 中复用现有 `invoke_isolated_with_runner()` 路径写 evidence index:
  - `resolved-config.yml` -> `resolved_config`
  - `invoke.json` -> `capability_invoke_json`
  - `.ralph/events.jsonl` -> `event_log_jsonl`
  - `result.json` -> `capability_result_json`
  - `failed.json` -> `capability_failed_json` + failure status
- 扩展 capability 单元测试覆盖成功和失败 evidence entries。
- 更新 OpenSpec tasks 为完成状态。

### 验证证据
- 红灯:
  - `cargo test -p ralph-cli --test integration_capability -- --nocapture` 曾失败在 `matches!(evidence_lookup, EvidenceLookup::Entries(_))`。
- 绿灯:
  - `cargo test -p ralph-cli capability::tests -- --nocapture`: 4 passed。
  - `cargo test -p ralph-cli --test integration_capability -- --nocapture`: 2 passed。
  - `cargo fmt --all -- --check`: passed。
  - `cargo test -p ralph-cli capability::tests`: 4 passed。
  - `cargo test -p ralph-cli --test integration_capability`: 2 passed。
  - `cargo test -p ralph-core smoke_runner`: 12 passed。
  - `openspec validate capability-child-run-evidence --type change`: valid。
  - `openspec validate --all --strict`: 27 passed,0 failed。
  - `git diff --check`: passed。
  - `cargo test`: workspace tests and doctests passed。

### 总结感悟
- Phase 3 的正确落点是改良现有 invocation artifact writer,不是把 capability invocation 扩成新的 runtime 平台。
- evidence index 仍然只保存 artifact link 和 correlation id; child artifact 与 event log 仍是真相源。
- 失败路径同样要注册 evidence,否则 audit 会在最需要排查时断链。

## [2026-05-14 16:21:00] [Session ID: omx-1778510695653-7pd7o2] 任务名称: Phase 3 OpenSpec archive 与最终验证

### 任务内容
- Phase 3 实现完成后归档 `capability-child-run-evidence` OpenSpec change。
- 同步稳定 spec,修正 `capability-invocation` 历史遗留的 `Purpose TBD`。
- 重新跑完整验证,准备本地提交。

### 完成过程
- 执行 `openspec archive capability-child-run-evidence --yes`。
- archive 将 3 个 added requirements 合入 `openspec/specs/capability-invocation/spec.md`。
- 修正 `openspec/specs/capability-invocation/spec.md` 的 Purpose,明确 capability invocation 的隔离 child/micro-run 与 evidence-index linkage 语义。
- active OpenSpec changes 回到只有既有无关 `tui-mdfried-viewer`。

### 验证证据
- `cargo fmt --all -- --check`: passed。
- `cargo test -p ralph-cli capability::tests`: 5 passed。
- `cargo test -p ralph-cli --test integration_capability`: 2 passed。
- `cargo test -p ralph-core smoke_runner`: 12 passed。
- `cargo test`: workspace tests and doctests passed。
- `openspec validate --all --strict`: 26 passed,0 failed。
- `git diff --check`: passed。

### 总结感悟
- 对已经完成的 OpenSpec change,及时 archive 能避免 active changes 噪音。
- archive 后要检查稳定 spec 的 Purpose,否则历史 `TBD` 会继续污染长期规格。

## [2026-05-14 17:49:00] [Session ID: omx-1778510695653-7pd7o2] 任务名称: Phase 3.1 capability invocation evidence UX

### 任务内容
- 为 Phase 3 的 capability invocation evidence 增加可用查询入口。
- 选择 `ralph tools capability inspect <invocation_id>` 作为最小 UX,暂不扩成泛化 `ralph evidence lookup` 子系统。
- 将 Phase 4 live runtime capability invocation 登记为后续独立演进线。

### 完成过程
- 创建并完成 OpenSpec change `capability-evidence-inspect-ux`。
- 写入 proposal/design/delta spec/tasks/test-plan。
- 先补 integration 红灯,证明当前 CLI 不支持 `inspect`。
- 在 `crates/ralph-cli/src/capability.rs` 增加:
  - `CapabilityCommands::Inspect`
  - `CapabilityInspectArgs`
  - `inspect_capability_evidence_report(...)`
  - JSON/human 输出结构
  - `NoEntry` 非零错误
  - explicit missing marker 的 `missing` 状态保留
- 扩展 `crates/ralph-cli/tests/integration_capability.rs`:
  - inspect 真实 invocation id 的 JSON 输出
  - inspect human 输出
  - unknown invocation id failure
- 增加 focused unit test 覆盖 missing marker。
- Archive OpenSpec change 到 `openspec/changes/archive/2026-05-14-capability-evidence-inspect-ux/`。
- 稳定 spec `openspec/specs/capability-invocation/spec.md` 已同步 inspect UX requirement。

### 验证证据
- 红灯:
  - `cargo test -p ralph-cli --test integration_capability -- --nocapture` 曾失败于 `unrecognized subcommand 'inspect'`。
- 绿灯:
  - `cargo fmt --all -- --check`: passed。
  - `cargo test -p ralph-cli --test integration_capability`: 4 passed。
  - `cargo test -p ralph-cli capability::tests`: 6 passed。
  - `cargo test -p ralph-core smoke_runner`: 12 passed。
  - `cargo test`: workspace tests and doctests passed。
  - `openspec validate capability-evidence-inspect-ux --type change`: valid。
  - `openspec validate --all --strict`: 26 passed,0 failed after archive。
  - `git diff --check`: passed。

### 总结感悟
- Phase 3.1 的正确边界是给 capability invocation evidence 一个稳定 lookup UX,而不是把 evidence kernel 膨胀成新的 doctor/diagnostic 平台。
- `--json` 是 agent/automation contract,human 输出只是阅读层。
- Phase 4 进入 live runtime 调用前,先有 inspect UX 是值得的,否则 live path 失败时调试面会太散。

## [2026-05-15 11:23:44] [Session ID: omx-1778510695653-7pd7o2] 任务名称: Phase 4 live runtime capability invocation 实现与验证

### 任务内容
- 实现真实 parent run 中 `ralph#1` 通过 `capability.request` 触发 isolated capability invocation。
- result/failure 通过 parent-visible `capability.result` / `capability.failed` event 回传。
- 复用 Phase 3/3.1 capability invocation artifacts、evidence index 和 inspect UX。

### 完成过程
- 在 core 协议层新增 `capability.request` payload、parent result/failure payload 和 `RuntimeCapabilityInvoker` adapter trait。
- 在 parallel supervisor 中只处理 `ralph#1` 输出的 `capability.request`,并按 `request_id` 幂等去重。
- 在 CLI capability module 中注入 runtime invoker,复用现有 `invoke_isolated` child/micro-run path。
- 新增 `integration_live_capability` dogfood,真实运行 parallel `ralph#1`,抽取 invocation id,并用 `ralph tools capability inspect <id> --json` 查询证据链。

### 验证
- `openspec validate live-runtime-capability-invocation --type change`: valid。
- `openspec validate --all --strict`: 27 passed,0 failed。
- `cargo fmt --all -- --check`: passed。
- `cargo test -p ralph-cli --test integration_capability`: passed。
- `cargo test -p ralph-cli --test integration_live_capability`: passed。
- `cargo test -p ralph-cli capability::tests`: passed。
- `cargo test -p ralph-core smoke_runner`: passed。
- `cargo test`: passed。
- `git diff --check`: passed。

### 总结感悟
- Phase 4 最稳的边界是 core 只做 runtime action hook,CLI adapter 负责执行 child/micro-run。
- `capability.result` 同时承载 child lifecycle 和 parent-return 语义,测试必须用 `request_id` 区分 parent-return result。

## [2026-05-15 23:25:01] [Session ID: omx-1778510695653-7pd7o2] 任务名称: Phase 4.1 parent-side capability selection UX

### 任务内容
- 实现 Phase 4.1: parent-side capability policy / selection UX。
- 让 `ralph#1` 基于 structured capability catalog / metadata 选择可调用能力,而不是靠硬编码隐藏知识。
- 保持 Phase 4 不变量: parent topology 不热改,调用仍走 isolated child/micro-run。

### 完成过程
- 创建并完成 OpenSpec change `parent-capability-selection-ux`,随后归档到 `openspec/changes/archive/2026-05-15-parent-capability-selection-ux/`。
- 在 core 新增 parent-visible capability catalog renderer,输出稳定 marker、`capability.request` contract 与 bounded metadata。
- 在 `ParallelSupervisor` 增加 `with_runtime_capability_catalog(...)`,并把 catalog 注入到 Ralph coordinator instructions,不注入普通 worker prompt。
- 在 CLI parallel runner 中复用已有 `capability_catalog()` 传入 supervisor。
- 扩展 live capability dogfood: custom backend 必须先从 `ralph#1` stdin prompt 捕获到 catalog marker、request contract 和 `hat:focused-reviewer`,才发 `capability.request`。
- 稳定 spec `openspec/specs/capability-invocation/spec.md` 新增 parent-side selection catalog、structured bounded metadata、topology isolation 3 条要求。

### 验证证据
- `openspec validate parent-capability-selection-ux --type change`: valid。
- `openspec validate --all --strict`: archive 前 27 passed,0 failed;archive 后 26 passed,0 failed。
- `cargo fmt --all -- --check`: passed。
- `cargo test -p ralph-core runtime_capability_catalog_is_injected_only_into_ralph_prompt`: 1 passed。
- `cargo test -p ralph-core parent_capability_catalog_renderer`: 2 passed。
- `cargo test -p ralph-cli --test integration_live_capability`: 1 passed。
- `cargo test -p ralph-cli --test integration_capability`: 4 passed。
- `cargo test -p ralph-cli capability::tests`: 6 passed。
- `cargo test -p ralph-core smoke_runner`: 12 passed。
- `cargo test`: workspace tests and doctests passed。
- `git diff --check`: passed。
- `git diff --cached --check`: passed。

### 总结感悟
- Phase 4.1 的正确落点是 parent-side selection surface,不是新的 invocation protocol。
- catalog 的真相源必须是 `CapabilityMetadata` 这种结构化 metadata,不能依赖 YAML 注释或完整 prompt body。
- catalog 注入必须发生在 `spawn_instances()` 前,否则 `ralph#1` prompt 已经定型。
- prompt pollution 要继续严控: catalog 只给 coordinator,不进普通 hats。

## [2026-05-16 11:53:20] [Session ID: omx-1778510695653-7pd7o2] 任务名称: 无配置 `ralph run` 默认并行模式

### 任务内容
- 调整 startup resource bootstrap,让运行目录没有 `ralph.yml` 且没有 `PROMPT.md` 时,默认 resolved config 启用并行模式。
- 保持显式 `--config` 语义不变: 用户明确传了配置路径时,缺失文件仍不被 bootstrap selector 吞掉。
- 创建并归档 OpenSpec change `default-bootstrap-parallel-run`,同步稳定 spec `openspec/specs/resource-bootstrap/spec.md`。

### 完成过程
- 在 `resolve_workflow_with_prompt_template(...)` 的 startup-only 配置合成边界设置 `config.parallel.enabled = true`。
- 补充 unit test,断言默认 bootstrap resolution 带 inline prompt 且 `parallel.enabled=true`。
- 补充 integration test,在空 workspace dry-run 后读取 `.ralph/resolved-config.yml`,断言包含 `parallel.enabled=true`。
- 归档 OpenSpec change 到 `openspec/changes/archive/2026-05-16-default-bootstrap-parallel-run/`。
- 修正归档 proposal 的 OpenSpec 标准章节,避免留下 `## Why` / `## What Changes` warning。

### 验证证据
- `openspec validate default-bootstrap-parallel-run --type change`: valid。
- `cargo test -p ralph-cli startup_resources::tests -- --nocapture`: 8 passed。
- `cargo test -p ralph-cli --test integration_startup_resources -- --nocapture`: 2 passed。
- `cargo fmt --all -- --check`: passed。
- `openspec validate --all --strict`: 26 passed,0 failed。
- `cargo test -p ralph-core smoke_runner`: 12 passed。
- `cargo test`: workspace tests and doctests passed。
- `git diff --check`: passed。

### 总结感悟
- 这个需求的正确切入点不是生成物理 `ralph.yml`,而是让 startup bootstrap 的 resolved config 承载“默认 ralph.yml”语义。
- 并行模式应该是隐式无配置启动的默认运行形态,这样 `ralph#1` 能保持 coordinator 角色,并接上后续 capability catalog / runtime evidence 链路。
- 显式配置路径仍是用户意图,不能因为文件不存在就悄悄改成默认 bootstrap。

## [2026-05-16 13:50:00] [Session ID: omx-1778510695653-7pd7o2] 任务名称: internalize-event-emission-protocol

### 任务内容
- 将通用 `<event topic="...">payload</event>` 事件发送格式从执行目录配置收口为 Ralph 内置 prompt contract。
- 保留执行目录 `ralph.yml` 的 workflow-specific topic、payload 字段、backpressure 与收敛规则。
- 瘦身 repo 内 `examples/parallel-experimental-dev-engine/ralph.yml` 和外部 `/Users/cuiluming/local_doc/l_dev/my/rust/ralph-example/ralph.yml` 的 generic event-format 教程块。

### 完成过程
- 新增 `crates/ralph-core/src/event_emission_protocol.rs`,用 `EVENT_EMISSION_PROTOCOL_HEADING` 作为稳定 marker。
- `HatInstanceActor::build_prompt(...)` 现在按 `hat.publishes` 注入内置事件发送协议。
- `ParallelSupervisor::build_ralph_coordinator_instructions(...)` 复用同一个 renderer,并把 `ralph emit` 放入独立 `## OUT-OF-BAND EVENT INJECTION` 段落。
- 增加 focused tests: renderer、publishing hat prompt、ralph coordinator prompt、example dogfood。
- 保持 `prompt_overlay` 对 shared all-hat overlay 示例的转义回归测试通过。

### 验证证据
- `cargo test -p ralph-core event_emission_protocol`: 2 passed。
- `cargo test -p ralph-core ralph_coordinator_event_protocol`: 1 passed。
- `cargo test -p ralph-cli --test integration_examples test_example_parallel_experimental_dev_engine_uses_builtin_event_protocol`: 1 passed。
- `cargo fmt --all -- --check`: passed。
- `cargo test -p ralph-core event_parser::tests`: 35 passed。
- `cargo test -p ralph-core prompt_overlay`: 8 passed。
- `openspec validate internalize-event-emission-protocol --type change`: valid。
- `openspec validate --all --strict`: 27 passed,0 failed。
- `cargo test -p ralph-core smoke_runner`: 12 passed。
- `cargo test`: workspace tests and doctests passed。
- `git diff --check`: passed。

### 总结感悟
- 通用 runtime envelope 属于 Ralph 内置 prompt contract,不应该长期复制在执行目录 `ralph.yml`。
- workflow 配置仍然必须保留业务 payload 字段,否则只知道如何发事件,不知道事件里应该带什么。
- coordinator 的 out-of-band `ralph emit` 是特例通道,应与 in-band `<event>` 协议分段说明,避免两套 envelope 文案继续漂移。

## [2026-05-16 14:49:00] [Session ID: omx-1778510695653-7pd7o2] 任务名称: startup bootstrap 与内置事件协议 live dogfood 收口

### 任务内容
- 对 `default-bootstrap-parallel-run` 与 `internalize-event-emission-protocol` 做真实运行收口。
- 证明无配置启动时,默认并行模式与内置事件发送协议已经在 live `ralph run` 链路中真实生效。
- 补齐六文件中的动态证据记录,避免只剩代码和测试结论。

### 完成过程
- 在空工作区先执行无配置 dry-run,读取 `.ralph/bootstrap-selection.json` 与 `.ralph/resolved-config.yml`。
- 确认 startup selector 选择 `workflow:feature-minimal` 与 `prompt:bootstrap-default-task`,并且 resolved config 已落盘 `parallel.enabled=true`。
- 基于 startup 产物执行真实 live run,抓取 `ralph#1` prompt 与 `record-session` summary。
- 确认 live prompt 同时含 `Act as Ralph's startup bootstrap coordinator` 与 `## RALPH EVENT EMISSION PROTOCOL`。
- 确认 `record summary` 为 `ux_mode: parallel-cli` 且 `Termination: CompletionPromise`。

### 总结感悟
- “默认并行模式”最稳的载体是 startup 产出的 resolved config,而不是要求用户维护一份容易过期的执行目录 `ralph.yml`。
- “事件协议内置化”真正有价值的完成标准,不是 example 变短了,而是 live `ralph#1` prompt 在默认启动链路里已经带上同一份协议真相源。

## [2026-05-16 16:12:00] [Session ID: omx-1778510695653-7pd7o2] 任务名称: 将 startup bootstrap live dogfood 固化为可重复 gate

### 任务内容
- 把之前只存在于 `/tmp` 证据链里的 startup bootstrap + 内置事件协议 live dogfood,固化成 repo 内可重复执行的 CLI integration gate。
- 让 gate 直接证明默认无配置启动、默认并行模式、live `ralph#1` prompt 协议注入、record-session 收敛这 4 个 runtime 事实。
- 补齐对应 OpenSpec change `bootstrap-live-dogfood-gate`。

### 完成过程
- 新建 OpenSpec change,把边界明确成“一条 repo-native 两段 runtime 流”,而不是单次命令或新 E2E 框架。
- 在 `crates/ralph-cli/tests/integration_startup_resources.rs` 中新增 live gate:
  - 第一步执行真实 no-config/no-prompt bootstrap dry-run,生成 `.ralph/bootstrap-selection.json` 与 `.ralph/resolved-config.yml`
  - 第二步只替换 resolved config 的 backend 执行表面,用 custom stdin backend 抓取 live `ralph#1` prompt
- gate 断言了:
  - bootstrap selection 资源选择事实
  - resolved config 含 `parallel.enabled=true`
  - live prompt 含 `Act as Ralph's startup bootstrap coordinator`
  - live prompt 含 `## RALPH EVENT EMISSION PROTOCOL`
  - live prompt 含 `reply.human.message`
  - record-session 含 `parallel-cli` 与 `CompletionPromise`
- 清理了误落在仓库根目录的临时 record-session 文件 `...`,避免污染提交。

### 验证证据
- `openspec validate bootstrap-live-dogfood-gate --type change`: passed。
- `cargo test -p ralph-cli --test integration_startup_resources -- --nocapture`: 3 passed。
- `openspec validate --all --strict`: 27 passed, 0 failed。
- `cargo test -p ralph-core smoke_runner`: 12 passed。
- `cargo test`: passed。
- `git diff --check`: passed。

### 总结感悟
- 这条 gate 的正确形态不是“再造一个大 E2E”,而是把已有 bootstrap artifact 和 live prompt capture 两种证据拼成一条窄而真的 runtime 链。
- 当默认 bootstrap workflow 自带 builtin backend 时,测试要尊重产品边界: 先产出 resolved config,再切换执行表面,而不是硬逼单次命令完成全部证明。

## [2026-05-16 17:36:00] [Session ID: omx-1778510695653-7pd7o2] 任务名称: 方向B继续 - answer evidence inspect UX

### 任务内容
- 在已完成的 `reply.hat.message` answer-return runtime evidence 和 live dogfood 基础上,补一个最小 CLI 查询入口。
- 让 request id / answer id 不再只能靠手动翻 `.ralph/evidence-index.jsonl` 才能查证。
- 保持边界收敛: 不做泛化 evidence 子系统,不改 runtime routing 语义。

### 完成过程
- 新建 OpenSpec change `answer-evidence-inspect-ux`,明确命令落点为 `ralph tools answer inspect <correlation_id>`。
- 新增 `crates/ralph-cli/src/answer.rs`,复用 `EvidenceIndexReader::find_by_correlation(...)`。
- 在 `tools.rs` 挂接 `Answer` 子命令,在 `main.rs` 注册模块。
- 扩展 `integration_answer_evidence.rs`,让现有 live dogfood 在同一工作区内继续调用:
  - `ralph tools answer inspect req-dogfood-1 --json`
  - `ralph tools answer inspect ans-dogfood-1`
- 新增 focused unit test,覆盖 explicit missing answer marker 会被保留为 `missing` 而不是误判成失败或 no-entry。

### 验证证据
- `openspec validate answer-evidence-inspect-ux --type change`: passed。
- `cargo test -p ralph-cli --test integration_answer_evidence -- --nocapture`: 2 passed。
- `cargo test -p ralph-cli answer -- --nocapture`: passed。
- `openspec validate --all --strict`: 27 passed, 0 failed。
- `cargo test -p ralph-core smoke_runner`: 12 passed。
- `cargo test`: passed。
- `git diff --check`: passed。

### 总结感悟
- 方向B当前最缺的不是再补一条 runtime gate,而是给已存在的 answer-return evidence 一个最小可查询面。
- `Entries` / `Missing` / `NoEntry` 这三个 lookup 语义已经足够表达 answer evidence 的第一阶段产品面,没必要现在就扩成通用 evidence 平台。

## [2026-05-16 18:20:00] [Session ID: omx-1778510695653-7pd7o2] 任务名称: 方向B.1 - human-facing answer return 最小闭环 dogfood 规格收口

### 任务内容
- 继续方向B,把“internal answer return 如何显式变成 human-visible answer”收束成一个窄 OpenSpec change。
- 避免把 `reply.hat.message` 与 `reply.human.message` 混成同一个机制。
- 为后续 focused gate 实现先锁定产品边界和验证口径。

### 完成过程
- 盘点了稳定 spec、项目经验、现有 CLI integration、routing guardrail tests 和 live E2E 场景。
- 确认当前最小缺口不是新 routing 功能,而是一条 repo-native dogfood gate。
- 新建 `openspec/changes/human-facing-answer-return-dogfood/`,并完成:
  - `proposal.md`
  - `design.md`
  - `specs/request-reply-answer-evidence/spec.md`
  - `tasks.md`
  - `test-plan.md`
- 用 `openspec validate human-facing-answer-return-dogfood --type change` 验证通过。

### 总结感悟
- B.1 最自然的做法不是再发明一条新 reply 通道,而是证明现有两条通道能在同一条 run 里各守其职。
- 先把“显示层问题、耐久化问题、真正 workflow 问题”三类失败解释拆开,后面实现时就不容易误补丁。

## [2026-05-16 18:34:00] [Session ID: omx-1778510695653-7pd7o2] 任务名称: 方向B.1 - human-facing answer return 最小闭环实现与归档

### 任务内容
- 为方向B.1补一条 repo-native focused gate,证明 internal `reply.hat.message` 与 explicit `reply.human.message` 能在同一条 runtime run 里闭环。
- 保持边界不变: internal answer return 不自动 synthesize human reply。
- 将该边界同步进稳定 spec 并 archive change。

### 完成过程
- 在 `crates/ralph-cli/tests/integration_answer_evidence.rs` 新增:
  - `write_explicit_human_reply_backend_script(...)`
  - `parallel_run_dogfoods_explicit_human_facing_answer_after_internal_reply()`
- 新 gate 断言了:
  - CLI stdout 出现最终 human-facing payload
  - `.ralph/events.jsonl` 同时保留 `reply.hat.message` 与 `reply.human.message`
  - record-session 保留 `reply.human.message` 发布证据
  - answer inspect 仍可按 internal request id 查到内部 answer evidence
- 完成 OpenSpec change `human-facing-answer-return-dogfood` 的 proposal / design / delta spec / tasks / test-plan。
- archive 后把 requirement 同步进 `openspec/specs/request-reply-answer-evidence/spec.md`,并修掉 EOF 空白行格式问题。

### 总结感悟
- 这条线最值钱的不是“新增了一个 reply 机制”,而是证明现有两条 reply 通道已经能在同一条 run 里正确协作。
- 当方向B再往前走时,可以默认把这条 gate 当成 request/reply/human-visible answer 的基础回归门禁。

## [2026-05-16 18:52:00] [Session ID: omx-1778510695653-7pd7o2] 任务名称: capability result 到 explicit human reply 的产品闭环

### 任务内容
- 继续产品演进,把 live runtime capability invocation 与 human-visible answer contract 接成一条真实运行链。
- 保持边界不变:
  - `capability.result` 是 parent-consumable runtime event
  - `reply.human.message` 才是面向人的最终回复
- 通过 repo-native gate 固定这条闭环。

### 完成过程
- 新建 OpenSpec change `capability-result-human-reply-dogfood`,并完成 proposal / design / delta spec / tasks / test-plan,随后 archive。
- 在 `crates/ralph-cli/tests/integration_live_capability.rs` 新增:
  - `write_human_reply_backend_script(...)`
  - `parallel_capability_result_can_become_explicit_human_reply()`
- 新 gate 断言了:
  - parent event log 保留 `capability.request`、`capability.result`、`reply.human.message`
  - CLI stdout 出现最终 human-facing payload
  - record-session 保留 `reply.human.message` 发布证据
  - `ralph tools capability inspect <invocation_id> --json` 仍可查证据链
  - parent config 保持不变
- archive 后把 requirement 同步进 `openspec/specs/capability-invocation/spec.md`。

### 总结感悟
- 这条线证明 capability invocation 不只是“能调子流程”,而是已经能服务真实对人回答的产品链路。
- 下一阶段如果再演进,更适合继续做“parent policy / multi-step orchestration / richer human answer shaping”,而不是回头重造 reply 机制。
