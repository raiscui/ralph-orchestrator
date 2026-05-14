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
