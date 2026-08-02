# EXPERIENCE.md

本文件保存面向智能体协作的项目级经验。它不是运行时 scoped experience 设计里的项目根 `experience.md`,也不替代 `.agent/memories.md`。这里记录的是经过持续学习整理后,未来维护 Ralph 仓库时应该优先回看的判断口径。

### exp-20260430-rust-utf8-safe-truncation
> 只要代码语义是“字符预算”或“保留 N 个字符”,实现就不能直接拿预算值或 `String::len()` 结果做字符串切片边界。必须先通过 `char_indices`、`is_char_boundary` 或统一 helper 转换为安全 byte index,并用中文或 emoji 覆盖回归测试。
<!-- scope: project | source_topics: memory_boundary_fix,continuous_learning | source_hats: codex | status: active | confidence: high | created_at: 2026-04-30T09:22:00+08:00 | updated_at: 2026-04-30T09:22:00+08:00 | supersedes:  -->

- 触发条件:
  - Rust panic 文本包含 `byte index ... is not a char boundary`。
  - 被截断内容包含中文、emoji 或其他多字节 UTF-8 字符。
  - 代码里同时出现 token/char budget 和字符串切片。
- 已验证路径:
  - `crates/ralph-core/src/text.rs` 提供安全边界 helper。
  - `memory_store` 和 scratchpad tail truncation 复用同一 helper。
  - 中文回归测试、`ralph-core smoke_runner` 和根目录 `cargo test` 曾通过。
- 未来动作:
  - 再遇到同类问题,优先使用 `.codex/skills/self-learning.rust-utf8-safe-string-truncation/SKILL.md`。

### exp-20260430-tui-mode-before-render-bug
> TUI 某块“不见了”时,先确认当前 run mode。Chat / Gates 是 parallel Supervisor TUI 的控制面,serial TUI 看不到它不等于渲染回归。
<!-- scope: project | source_topics: tui_chat_missing,continuous_learning | source_hats: codex | status: active | confidence: high | created_at: 2026-04-30T09:22:00+08:00 | updated_at: 2026-04-30T09:22:00+08:00 | supersedes:  -->

- 触发条件:
  - 用户报告 Ralph TUI chat 窗口消失。
  - 当前配置可能没有 `parallel.enabled=true`。
- 已验证事实:
  - 根目录 `ralph.yml` 走 serial loop 时不会渲染 Chat / Gates。
  - 临时 parallel idle config 在 tmux 100x30、100x16、100x14、100x12 下都能捕获到 Chat / Gates。
- 未来动作:
  - 先跑 `ralph run --dry-run --no-tui` 或读 config 分支,确认是否进入 parallel。
  - 如果产品目标是默认根配置也有 chat,那是配置/产品决策,不是直接修渲染层。

### exp-20260430-runtime-graph-boundaries
> `ralph hats graph`、V1 Rerun live runtime graph、V2 durable replay graph 是三层不同能力。不能把 live `.rrd` 产物宣传成完整 replay truth,也不能让 Rerun runtime graph 替代静态 topology graph。
<!-- scope: project | source_topics: memory_axes,continuous_learning | source_hats: codex | status: active | confidence: high | created_at: 2026-04-30T09:22:00+08:00 | updated_at: 2026-04-30T09:22:00+08:00 | supersedes:  -->

- 触发条件:
  - 继续 `rerun-runtime-graphs`。
  - 讨论 runtime graph、Rerun、Mermaid、hat topology、replay graph 的边界。
- 当前事实:
  - `openspec/changes/rerun-runtime-graphs/tasks.md` 显示 11/15 完成。
  - 剩余 3.1 到 3.4 都属于 V2 durable replay graph。
  - V1 live graph 已依赖 live delivery observer,不能用旧 durable log 盲猜 recipient 边。
- 未来动作:
  - 继续实现前先读 `openspec/changes/rerun-runtime-graphs/design.md` 和 tasks。
  - 下一步直接做 V2 durable records / replay reconstruction,不要回头重做 V1 入口。

### exp-20260430-continuous-learning-branch-archive
> 持续学习整理支线六文件时,先按后缀分组、再按最后标准时间戳判定活跃度。当天活跃或明确仍推进的支线留在根目录; 已完成或非当天且无活跃证据的旧支线,总结后整组移入 `archive/branch_contexts/<topic>/`。
<!-- scope: project | source_topics: continuous_learning | source_hats: codex | status: active | confidence: high | created_at: 2026-04-30T09:22:00+08:00 | updated_at: 2026-04-30T09:22:00+08:00 | supersedes:  -->

- 触发条件:
  - 根目录出现 `task_plan__topic.md`, `notes__topic.md`, `WORKLOG__topic.md` 等支线文件。
  - 用户触发 `$continuous-learning` 或需要清理六文件上下文。
- 本轮已验证:
  - `serial_tui_issues` 是当天活跃支线,保留根目录。
  - `memory_axes`, `memory_boundary_fix`, `tui_chat_missing` 已总结并归档到 `archive/branch_contexts/`。
  - 归档说明写入 `archive/manifests/ARCHIVE_MANIFEST__continuous_learning_2026-04-30_0918.md`。
- 未来动作:
  - 不要只凭文件名把所有 `__suffix` 文件都当活跃。
  - 归档前必须先写六文件摘要,否则 archive 只会变成噪音仓库。

### exp-20260511-guidance-contract-governance
> 从 `oh-my-codex` 借鉴 agent 治理能力时,优先落地 guidance schema、prompt contract、manifest 和 verifier。不要先搬完整 team/tmux runtime,否则会把运行时复杂度前置成新平台。
<!-- scope: project | source_topics: oh_my_codex_learning,guidance_contract_governance | source_hats: codex | status: active | confidence: high | created_at: 2026-05-11T17:20:00+08:00 | updated_at: 2026-05-11T17:20:00+08:00 | supersedes:  -->

- 触发条件:
  - 继续从 `specs/oh-my-codex-learning-analysis.md` 落地建议。
  - 讨论 prompt / skill / AGENTS / hats 的漂移治理。
  - 想把 agent 行为从“口头约定”变成可校验资产。
- 建议顺序:
  1. 先写 `docs/agent-guidance-schema.md`,固定指导文档的必需章节。
  2. 再写 `docs/prompt-contract.md`,固定 prompt / skill / hat 的行为输出契约。
  3. 再建 agent assets manifest,让资产路径、类型和验证规则成为单一真相源。
  4. 最后接入 verifier 到 `cargo test` 或专门脚本。
- 明确不要先做:
  - 不要一开始搬完整 team/tmux runtime。
  - 不要先做 plugin/setup 双模式或 native hooks 全矩阵。
  - 不要靠 YAML 注释作为 runtime metadata contract; 机器可读信息必须进结构化字段。
- 未来动作:
  - 继续此方向时,优先读 `specs/oh-my-codex-learning-analysis.md` 第 4 节。
  - 如果要进入代码实现,先走 OpenSpec change,避免直接把治理规则散落在 docs 和 tests 里。


### exp-20260513-runtime-evidence-index-kernel-boundary
> Phase 1A runtime evidence 只能先做 minimal evidence index kernel: artifact link、correlation lookup、status marker、parent-child link。不要提前把 `ralph evidence summary`、`ralph evidence inspect`、`ralph doctor evidence` 或诊断 taxonomy 塞进 kernel。
<!-- scope: project | source_topics: runtime_evidence_index_kernel,ralph_evolution_roadmap | source_hats: codex | status: active | confidence: high | created_at: 2026-05-13T19:37:49+08:00 | updated_at: 2026-05-13T19:37:49+08:00 | supersedes:  -->

- 触发条件:
  - 继续 `.omx/plans/ralph-evolution-roadmap-consensus-draft.md` 的 Phase 1A。
  - 讨论 record-session、runtime delivery / lifecycle、reply、capability invocation artifacts 的统一 evidence contract。
- 已验证事实:
  - OpenSpec change `runtime-evidence-index-kernel` 已创建并通过 `openspec validate runtime-evidence-index-kernel --type change`。
  - `openspec validate --all --strict` 返回 25 passed,0 failed。
  - `openspec show runtime-evidence-index-kernel --json --deltas-only` 解析出 5 个 ADDED requirements。
- 关键边界:
  - index 是最小 artifact lookup kernel,不是 CLI UX、doctor、runtime graph 或 orchestration platform。
  - live runtime graph / Rerun layout 不能作为 durable truth source。
  - parent-child link 使用 correlation id,不能要求热改 parent topology。
- 未来动作:
  - 实现前先读 `openspec/changes/runtime-evidence-index-kernel/design.md` 与 `test-plan.md`。
  - 先补 schema / writer-reader / missing marker / parent-child contract tests,再接现有 record-session、event logger、capability artifacts。

### exp-20260514-request-reply-answer-evidence-boundary
> Answer-return evidence 的正确入口是显式 `reply.hat.message` requester-return 分支。不要把普通 workflow event 的 `reply` 属性、human-visible reply、request broker、CLI evidence UX 或 live topology 热改混进 Phase 2 边界。
<!-- scope: project | source_topics: request_reply_answer_evidence,task_plan_rollover_continuous_learning | source_hats: codex | status: active | confidence: high | created_at: 2026-05-14T13:55:00+08:00 | updated_at: 2026-05-14T13:55:00+08:00 | supersedes:  -->

- 触发条件:
  - 继续 `request-reply-answer-evidence`、live runtime answer evidence dogfood,或 Phase 3 capability invocation / child run evidence 串联。
  - 调试 `reply.hat.message` answer 是否可回到 requester,以及 evidence index 是否能按 request id / answer id 查到 durable artifact。
- 已验证事实:
  - `reply.hat.message` success 分支写入 request id 与 answer event id 两类 evidence index entry。
  - fail-closed 分支写入 failure evidence;missing/timeout 通过显式 marker API 写入 missing evidence。
  - evidence entry 指向 `.ralph/events.jsonl` 等 durable artifact;event log 仍是真相源。
  - 普通 workflow event 即使带 `reply` 属性,也不能被归类成 answer-return evidence。
  - 内部 `reply.hat.message` 不能自动合成 `reply.human.message`;面向人的最终答案仍必须是显式 workflow/event 决策。
- 关键边界:
  - `EvidenceIndexEntry.producer` 表示写入者身份,不要塞 failure reason。失败原因应留在原始 JSONL artifact payload 里。
  - 不要为了最小 evidence 闭环新增 request broker 或热改 live topology。
  - OpenSpec archive 后检查稳定 spec 的 `Purpose TBD`;Phase 1A 和 Phase 2 都出现过这个归档收尾点。
- 验证锚点:
  - `cargo test --package ralph-core --lib parallel::supervisor::routing_tests`
  - `cargo test --package ralph-core --lib evidence_index::tests`
  - `cargo test -p ralph-core smoke_runner`
  - `openspec validate --all --strict`

### exp-20260515-live-capability-invocation-boundary
> Runtime capability invocation 的正确 Phase 4 边界是: parent `ralph#1` 发结构化 `capability.request`, supervisor 通过 adapter 触发 isolated child/micro-run,再把 `capability.result` / `capability.failed` 回传 parent。不要把被调用 capability 热注入 live parent topology。
<!-- scope: project | source_topics: live_runtime_capability_invocation,capability_evidence_inspect_ux | source_hats: codex | status: active | confidence: high | created_at: 2026-05-15T11:52:00+08:00 | updated_at: 2026-05-15T11:52:00+08:00 | supersedes:  -->

- 触发条件:
  - 继续 Phase 3/4 capability invocation、child run evidence、parent-side capability policy 或 catalog selection。
  - 调试 parent run 为什么没有收到 child/micro-run result/failure。
  - dogfood 时需要用 `ralph tools capability inspect <invocation_id> --json` 查证据链。
- 已验证事实:
  - `crates/ralph-core/src/parallel/supervisor/capability_runtime.rs` 是 parent runtime hook 的主要实现点。
  - `crates/ralph-cli/src/capability.rs` 仍是 isolated invocation adapter 和 artifact 写入点。
  - parent topology 必须保持 immutable; invocation evidence 落在 `.ralph/capability-invocations/<id>/...` 与 `.ralph/evidence-index.jsonl`。
  - `capability.result` 有 child lifecycle result 与 parent-return result 两类语义;查 parent 回传必须用 `request_id` 区分。
- 关键边界:
  - core 负责协议与 hook surface,CLI adapter 负责执行 child/micro-run。
  - failure 也必须 parent-visible,不能只留在 child artifact 里。
  - 如果继续 Phase 4.1 parent-side selection UX,优先把可选能力作为 catalog/metadata 暴露给 `ralph#1`,而不是改 live `HatRegistry`。
- 验证锚点:
  - `cargo test -p ralph-cli --test integration_live_capability`
  - `cargo test -p ralph-cli --test integration_capability`
  - `cargo test -p ralph-core capability_request`
  - `cargo test -p ralph-core smoke_runner`
  - `openspec validate --all --strict`

### exp-20260515-parent-capability-selection-catalog
> Parent-side capability selection 的正确 Phase 4.1 边界是: 把 bounded `CapabilityMetadata` catalog 注入给 `ralph#1` coordinator,让它选择并发 `capability.request`。不要把 catalog 当作热改 parent topology 或普通 worker prompt 的理由。
<!-- scope: project | source_topics: parent_capability_selection_ux,live_runtime_capability_invocation | source_hats: codex | status: active | confidence: high | created_at: 2026-05-15T23:25:52+08:00 | updated_at: 2026-05-15T23:25:52+08:00 | supersedes:  -->

- 触发条件:
  - 继续 Phase 4.1 parent-side capability policy / selection UX。
  - 调试 `ralph#1` 为什么不知道有哪些 capability 可调用。
  - 设计 capability catalog、metadata、selection policy 或 future chooser。
- 已验证事实:
  - `render_parent_capability_catalog()` 是 parent-visible catalog 的稳定 renderer。
  - `PARENT_CAPABILITY_CATALOG_HEADING` 是测试和 dogfood 使用的稳定 marker。
  - `ParallelSupervisor::with_runtime_capability_catalog(...)` 必须在 `spawn_instances()` 前调用,否则 coordinator prompt 已经定型。
  - CLI 侧用 `capability_catalog()` 传入 supervisor;core 不反向依赖 CLI catalog builder。
  - live dogfood 已验证 custom backend 只有在 `ralph#1` stdin prompt 中看到 catalog marker、`capability.request` contract 和 `hat:focused-reviewer` 后才发 request。
- 关键边界:
  - catalog 来源必须是 structured `CapabilityMetadata`,不要读 YAML 注释或完整 prompt body。
  - catalog 只注入 Ralph coordinator instructions,不要污染普通 hats。
  - 选择 capability 后仍走 existing isolated child/micro-run invocation path。
  - parent `ralph.yml` / live `HatRegistry` 必须保持不变。
- 验证锚点:
  - `cargo test -p ralph-core runtime_capability_catalog_is_injected_only_into_ralph_prompt`
  - `cargo test -p ralph-core parent_capability_catalog_renderer`
  - `cargo test -p ralph-cli --test integration_live_capability`
  - `openspec validate --all --strict`

### exp-20260516-capability-failure-class-branching
> Parent-side capability failure branching 的稳定输入是 `capability.failed.failure_class`,不是自由文本 `error`。继续 richer fallback policy 时,先扩结构化 class 和 gate,不要先做 retry engine、planner 或 live topology mutation。
<!-- scope: project | source_topics: capability_failure_class_branching_policy,live_runtime_capability_invocation | source_hats: codex | status: active | confidence: high | created_at: 2026-05-16T23:31:00+08:00 | updated_at: 2026-05-16T23:31:00+08:00 | supersedes:  -->

- 触发条件:
  - 继续 capability invocation failure / fallback / parent policy 这条线。
  - 调试 parent 为什么根据失败原因选择了错误 fallback。
  - 想新增 `capability.failed` 的失败类型或 richer branching policy。
- 已验证事实:
  - `CapabilityFailureClass` 是 runtime capability failure 的结构化分类,序列化为 snake_case。
  - parent-visible `CapabilityParentFailedRecord` 与 child/micro-run `CapabilityFailedRecord` 都带 `failure_class`。
  - invalid capability id 会变成 `invalid_capability_id`,并且 live parent fallback gate 已证明 parent 后续 prompt 能看到这个 class。
  - child/micro-run 启动后失败写为 `child_run_failed`,用于区分“选择前失败”和“执行后失败”。
- 关键边界:
  - `error` 可以保留做人类诊断,但不能成为 parent policy 的唯一稳定信号。
  - fallback success 和 final `reply.human.message` 仍然要作为独立事件审计。
  - 不要因为引入 failure class 就顺手加通用 retry engine、planner 或 parent topology 热改。
- 验证锚点:
  - `cargo test -p ralph-cli --test integration_live_capability parallel_parent_run_can_fallback_after_capability_failed_before_final_human_reply`
  - `cargo test -p ralph-cli capability::tests`
  - `cargo test -p ralph-core capability::tests`
  - `cargo test -p ralph-core smoke_runner`
  - `openspec validate --all --strict`

### exp-20260517-capability-failure-branching-matrix
> Richer parent capability failure policy 应优先做 class-specific dogfood matrix,不是做 retry engine。当前已验证 `invalid_capability_id -> fallback request` 与 `malformed_request -> diagnostic human reply without retry` 两条分支。
<!-- scope: project | source_topics: capability_failure_branching_matrix,live_runtime_capability_invocation | source_hats: codex | status: active | confidence: high | created_at: 2026-05-17T00:05:00+08:00 | updated_at: 2026-05-17T00:05:00+08:00 | supersedes:  -->

- 触发条件:
  - 继续 B.4/B.5 parent-side failure branching policy。
  - 讨论是否需要 retry engine、planner 或 fallback matrix。
  - 调试 malformed capability request 为什么没有进入人类可见诊断回复。
- 已验证事实:
  - `malformed_request` 会成为 parent-visible `capability.failed`,并能进入后续 parent turn prompt。
  - parent 可以在看到 `malformed_request` 后显式发 `reply.human.message`,而不发 fallback `capability.request`。
  - malformed branch 不创建 `.ralph/capability-invocations/<id>` artifact,因为它在 invocation 前失败。
  - 既有 `invalid_capability_id` branch 仍可 fallback 到有效 capability request。
- 关键边界:
  - 不要把 `capability.failed` 自动转换成人类最终答案; human reply 仍必须显式发 `reply.human.message`。
  - 不要为 malformed request 做盲目 retry。先 diagnostic,再由 parent/human 决定是否重新发结构正确的新 request。
  - 不要为了测试 `child_run_failed` live branch 引入测试专用 runtime failure switch;应单独设计真实 child failure dogfood。
- 验证锚点:
  - `cargo test -p ralph-cli --test integration_live_capability parallel_parent_run_can_emit_diagnostic_reply_for_malformed_capability_request_without_retry`
  - `cargo test -p ralph-cli --test integration_live_capability parallel_parent_run_can_fallback_after_capability_failed_before_final_human_reply`
  - `cargo test -p ralph-cli --test integration_live_capability`
  - `openspec validate --all --strict`


### exp-20260517-parallel-tui-status-summary
> 并行 TUI 信息缺失类问题先分清 runtime truth 与 display aggregation。优先复用 `ParallelTuiState` / `InstanceViewState` / `TuiState.last_event` 做状态摘要,不要为了显示“当前在做什么”再建第二套状态源。
<!-- scope: project | source_topics: tui_status_summary,default_notes_rollover | source_hats: codex | status: active | confidence: high | created_at: 2026-05-17T16:55:40+08:00 | updated_at: 2026-05-17T16:55:40+08:00 | supersedes:  -->

- 触发条件:
  - 用户反馈 TUI 和 Codex/CLI 直接输出差异很大,怀疑信息没有显示。
  - 并行 TUI 需要让用户快速知道 selected instance、current job、last event、Rendered/Plain 模式。
- 已验证事实:
  - CLI/log-mode 偏审计流,TUI 偏操作面;差异不等于 runtime 丢信息。
  - `should_forward_event_to_tui` 会过滤无 source/source_instance 的普通业务事件。
  - `Output` 标题已经能显示 selected instance state/job;Instances 和 Footer 更适合补状态摘要。
  - Footer 80 列空间有限,verbose label 会挤掉关键 event topic;紧凑格式 `writer#1 j1/1 m:P e:reply.human.message` 已通过测试。
- 未来动作:
  - 状态摘要优先用现有 state helper,例如 `InstanceViewState::current_job_summary()`。
  - 如果要显示 stderr visible/hidden,先把 runner 的 `show_stderr` 明确传入 TUI state,不要在 widget 里猜。
  - raw/audit 视图应作为下一层能力,不要把默认 TUI 退化成 stdout 全量镜像。

### exp-20260518-role-aware-cli-args-for-coordinator-hooks
> Coordinator-only backend 行为不要放进全局 `cli.args`。当 Ralph coordinator 与 worker hats 需要不同 CLI 参数时,优先使用 role-aware overlay,顺序保持 `role_args -> custom_args -> reasoning_effort defaults`。
<!-- scope: project | source_topics: parallel_rec_analysis,coordinator_only_codex_hooks | source_hats: codex | status: active | confidence: high | created_at: 2026-05-18T23:03:00+08:00 | updated_at: 2026-05-18T23:03:00+08:00 | supersedes:  -->

- 触发条件:
  - 需要让 `ralph#1` / coordinator 使用某个 Codex CLI override,但普通 hats 不能继承。
  - 讨论 `features.hooks=false`、reasoning effort、backend argv 隔离或 clean backend profile。
- 已验证事实:
  - `cli.role_args.coordinator` 可以承载 `-c features.hooks=false`。
  - `cli.role_args.worker` 保持空数组时,worker hats 不会收到 coordinator-only hooks override。
  - parallel path 当前用 `job.hat_id == "ralph"` 判定 coordinator;serial path 用 `display_hat == "ralph"` 判定 coordinator。
  - `hat:*` capability direct backend path 应按 worker role 执行。
- 关键边界:
  - 不要把 coordinator-only 参数塞进全局 `cli.args`。
  - 不要用独立 `CODEX_HOME` 作为默认方案;它能隔离,但复杂度过高。
  - 如果未来出现多个 coordinator id,应升级为显式 role metadata,不要继续扩散字符串判断。
- 验证锚点:
  - `cargo test -p ralph-core cli_role_args -- --nocapture`
  - `cargo test -p ralph-adapters role_args -- --nocapture`
  - `cargo test -p ralph-cli parallel_runner::tests::parallel_role_backend_overlays_apply_coordinator_hooks_only -- --exact --nocapture`
  - `cargo test -p ralph-cli autopilot::tests::analysis_config_preserves_cli_role_args -- --exact --nocapture`
  - `cargo test --quiet`

### exp-20260519-parallel-output-status-strip-viewport
> 并行 TUI Output 底部如果加入 `evidence:` / `act:` status strip,正文滚动、选择、复制和 autoscroll 必须统一使用 content viewport,不能再用完整 `output_inner` 高度。
<!-- scope: project | source_topics: display_info_evidence,parallel_output_status_strip | source_hats: codex | status: active | confidence: high | created_at: 2026-05-19T07:58:00+08:00 | updated_at: 2026-05-19T07:58:00+08:00 | supersedes:  -->

- 触发条件:
  - 修改 `crates/ralph-tui/src/widgets/parallel_output.rs` 的 Output status strip。
  - 调试用户反馈“Output 底部状态遮挡输出 / 最后几行看不到 / act 状态压住正文”。
  - 改动并行 TUI 的 output selection、copy、autoscroll 或测试 harness。
- 已验证事实:
  - status strip 是 display-only 区域,不属于 stdout/stderr 正文 viewport。
  - `split_parallel_output_areas(inner)` 是正文区与 status 区的单一几何入口。
  - autoscroll 预计算必须使用 `content_area.height`,否则 `scroll_offset` 会少滚 status strip 的行数。
  - 鼠标选择、拖拽、复制和键盘扩展选择也必须使用 `output_content_area`。
- 关键边界:
  - 不要让 `output_inner.height` 直接进入正文滚动计算。
  - 点击 status area 可以聚焦 Output,但不能创建正文 selection anchor。
  - 测试 harness 必须复用同一 split helper,不要复制一套独立 status height 公式。
- 验证锚点:
  - `cargo test -p ralph-tui --lib split_parallel_output_areas_reserves_bottom_status_rows -- --nocapture`
  - `cargo test -p ralph-tui --lib app::tests::mouse_click_output_status_area_focuses_output_without_starting_selection -- --exact --nocapture`
  - `cargo test -p ralph-tui --quiet`
  - `cargo test --quiet`

### exp-20260520-topology-spawn-result-ack-guardrail
> `topology.spawn.result` 是 parent-visible group spawn 的 acknowledgement,不是再次 delegate 的触发器。收到 result 后不要重发 `delivery_topic`,也不要把 `audience_instances` 当 replay 机制。
<!-- scope: project | source_topics: parallel_rec_analysis,parent_visible_topology_spawn | source_hats: codex | status: active | confidence: high | created_at: 2026-05-20T07:55:00+08:00 | updated_at: 2026-05-20T07:55:00+08:00 | supersedes:  -->

- 触发条件:
  - 调试 "event 已发出,父级 TUI 里没有按预期出现/运行实例"。
  - 修改 `topology.spawn_group`、`topology.spawn.result`、coordinator prompt 或 dynamic instance delivery。
  - 看到 coordinator 在 spawn 成功后又发出同一个 `delivery_topic`。
- 已验证事实:
  - `topology.spawn_group` 会创建真实 parent-visible dynamic instances,并对每个 spawned instance 做 direct delivery。
  - `topology.spawn.result` 回到 `ralph#1` 时,spawned instances 已经收到 `delivery_topic`。
  - live dogfood `/tmp/ralph-topology-dogfood-guardrail-record.jsonl` 验证: `analysis.task` 总数为 3,且 `topology.spawn.result` 之后 `analysis_task_after_spawn_result=0`。
  - `capability.request` 仍是 isolated child/micro-run,不能用来表达父级可见新实例。
- 关键边界:
  - `topology.spawn.result` 只能触发等待 worker results 或处理 `failed` 成员,不能 replay 原始任务。
  - `audience_instances` 不是实例创建机制,也不是 spawn result 后的重放机制。
  - 如果 `topology.spawn.failed` 出现,应报告或修正失败,不能伪造实例存在。
  - dogfood worker `MaxRuntime` 是 worker 收敛问题,不要误判成 topology spawn redelivery 回归。
- 验证锚点:
  - `cargo test -p ralph-core --lib parallel::supervisor::routing_tests::runtime_capability_catalog_is_injected_only_into_ralph_prompt -- --exact --nocapture`
  - `cargo test -p ralph-core --lib parallel::supervisor::routing_tests::topology_spawn_group_creates_three_dynamic_instances_and_delivers_direct -- --exact --nocapture`
  - `cargo test -p ralph-core topology_spawn_group -- --nocapture`
  - `git diff --check && cargo test --quiet`
  - `target/debug/ralph record summary <record-session.jsonl>` 并检查 `topology.spawn.result` 之后没有新增 `delivery_topic`。

### exp-20260520-multi-agent-collaboration-evidence-layers
> Ralph 的 multi-agent collaboration 证据要分层表达: core routing/fanout/queue/dynamic-spawn 单测证明机械协议,E2E scenario registration 证明入口存在,live Codex E2E 才能证明真实模型协作稳定性。
<!-- scope: project | source_topics: multi_agent_collab_evidence,parallel_hat_instances | source_hats: codex | status: active | confidence: high | created_at: 2026-05-20T07:55:00+08:00 | updated_at: 2026-05-20T07:55:00+08:00 | supersedes:  -->

- 触发条件:
  - 用户问 "多智能体协作到底怎么测试" 或 "有没有真实协作案例"。
  - 审查 `crates/ralph-core/src/parallel/*`、`crates/ralph-e2e/src/scenarios/parallel*` 或 `examples/parallel-*`。
- 已验证事实:
  - 当前仓库的真实协作 runtime 是 `parallel hat instances`,核心链路是 event topic -> routing -> instance delivery -> runtime delivery / agents snapshot。
  - core focused tests 能验证 fanout、queue、dynamic spawn 会写 runtime delivery。
  - `ralph-e2e -- --list` 能证明 parallel scenarios 已注册,但这不是 live model 稳定性证明。
- 关键边界:
  - 不要把协议/状态机测试说成 live LLM 协作已经稳定。
  - 如果要证明真实模型协作,需要单独跑 `ralph-e2e` live Codex scenario,并保留 record-session / report 证据。
  - 静态 evidence、focused tests、live E2E 的结论层级必须分开写。
- 验证锚点:
  - `cargo test --package ralph-core --lib parallel::supervisor::routing_tests::fanout_delivery_writes_one_runtime_delivery_record_per_recipient -- --exact`
  - `cargo test --package ralph-core --lib parallel::supervisor::routing_tests::queue_delivery_writes_runtime_delivery_record -- --exact`
  - `cargo test --package ralph-core --lib parallel::supervisor::routing_tests::spawn_instance_forces_new_dynamic_instance_and_delivers_direct -- --exact`
  - `cargo run -p ralph-e2e -- --list | rg 'parallel-hat-instances|parallel-trigger-routing|parallel-human-approval-gate|parallel-emit-spawn-instance'`

### exp-20260522-clean-live-dogfood-record-session-vs-agents-snapshot
> Clean live dogfood 要用专门临时 config 收窄拓扑,并以 record-session 的 Evidence Inspect 作为历史真相源; `.ralph/agents.json` 只是 current registry sidecar,动态实例被 TTL 回收后可能不再显示。
<!-- scope: project | source_topics: clean_live_dogfood,task_derived_role_contract,parent_visible_spawn,agents_snapshot_ttl | source_hats: codex | status: active | confidence: high | created_at: 2026-05-22T12:12:00+08:00 | updated_at: 2026-05-22T12:12:00+08:00 | supersedes:  -->

- 触发条件:
  - live dogfood 需要验证 parent-visible `topology.spawn_group`、task-derived dynamic role contract、worker result topic 和自然收敛。
  - 默认 `ralph.yml` 中存在 confessor / confession_handler 或目标 hat publishes 与 dogfood 期望 topic 不一致。
  - `record summary --agents-file .ralph/agents.json` 中 Agents Snapshot 与 Result Topics 看起来不一致。
- 已验证事实:
  - 临时 clean config 移除 confessor / confession_handler,并让 `builder.publishes` 包含 `analysis.done`,可把 3-worker dogfood 收敛到 49.620 秒。
  - coordinator 可通过 `cli.role_args.coordinator = ["-c", "features.hooks=false"]` 禁用 hooks,同时保持 `cli.role_args.worker = []` 让 worker 正常带 hooks。
  - `record-session` 的 Evidence Inspect 能证明 `topology.spawn_group: 1`, `topology.spawn.result: 1`, `parent_topology_unchanged=false`, `topology.spawn.failed: 0`, `analysis.done: 3 source_instances=builder#2,builder#3,builder#4`, `Termination.reason=CompletionPromise`。
  - `.ralph/agents.json` 来自当前 `self.instances` registry。动态实例进入 `Done` 后会被 `unregister_dynamic_instance()` 移除,所以最终 sidecar 可能缺少最早完成并已被 TTL 回收的 instance。
- 关键边界:
  - 不要把 `.ralph/agents.json` 少了某个 completed dynamic instance 直接解释成“实例没跑”。先看 record-session / Result Topics / topology.spawn.result。
  - Clean dogfood 不要修改长期 `ralph.yml`;优先用 `/tmp/*.yml` 和 `/tmp/*.prompt.md`。
  - 如果移除所有会 publish `workflow.complete` 的 hat,就不要保留 `event_loop.complete_publishes: workflow.complete`,否则 config validator 会正确 fail closed。
- 验证锚点:
  - `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-114604.jsonl`
  - `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-114604.summary.txt`
  - `./target/debug/ralph record summary /tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-114604.jsonl --agents-file .ralph/agents.json`

### exp-20260526-runtime-evidence-closure-and-dynamic-role-index
> Dynamic hats 的完成证据要形成 closure: protocol SSOT、role contract summary、record-session summary、agents snapshot/tombstone、evidence-index correlation 和 preserved dogfood artifacts 必须能互相指回,但 evidence-index 仍只能是导航索引,不能变成第二套事实源。
<!-- scope: project | source_topics: clean_current_runtime_evidence,dynamic_role_contract,evidence_index,topology_spawn_group | source_hats: codex | status: active | confidence: high | created_at: 2026-05-26T00:10:00+08:00 | updated_at: 2026-05-26T00:10:00+08:00 | supersedes: exp-20260513-runtime-evidence-index-kernel-boundary -->

- 触发条件:
  - 修改 `topology.spawn_group`、task-derived dynamic role contract、record summary / Evidence Inspect、agents snapshot 或 `.ralph/evidence-index.jsonl`。
  - 用户要验证“父级 TUI 里新增 hat instance 是否是真实实例”或“dynamic worker 结果是否真的闭环”。
  - OpenSpec 需要声明 runtime/evidence lane 的 release-fast gate。
- 已验证事实:
  - `topology.spawn_group` 真实运行路径应写入 request id -> child instance,role hash -> event log / agents snapshot + result topic 的 evidence-index links。
  - `EvidenceIndexEntry.result_topic` 只能保存 produced / expected topic 名称,不能保存 result payload 或完整 role contract。
  - `find_by_correlation` 需要能匹配 primary / parent / child / result_topic,否则 lookup by spawn request id 看不到 lineage 上的 missing marker。
  - `record summary --agents-file` 应区分 current registry、completed dynamic tombstones、record-session semantic termination 和 dynamic result coverage。
  - Preserved dogfood 证据位置: `/tmp/ralph-runtime-evidence-lane-dogfood-20260525-140000/{session.jsonl,.ralph/events.jsonl,.ralph/agents.json,.ralph/evidence-index.jsonl,summary.txt}`。
- 关键边界:
  - evidence-index 是 artifact correlation kernel,不是 record-session / events / agents snapshot 的替代真相源。
  - 不要从 `topology.spawn.result`、stdout tail、TUI display 或 wrapper exit status 推断 semantic completion;优先 record-session `_meta.termination`。
  - macOS 临时目录可能出现 `/var` 与 `/private/var` 文本差异,测试 artifact path 时优先断言稳定后缀或 canonicalized path。
  - `agent-cli-recoverable-failure-retry` 已在 2026-05-28 归档;不要继续沿用旧的 `no-delta change` 阻断判断。若 `openspec validate --all --strict` 再失败,应按当前输出重新定位具体 change/spec,不要把失败默认归因到 recoverable retry 主线。
- 验证锚点:
  - `cargo test -p ralph-core evidence_index --lib --quiet`
  - `cargo test -p ralph-cli --test integration_topology_spawn parallel_parent_visible_spawn_materializes_dynamic_agents_without_redelivery --quiet`
  - `cargo test -p ralph-cli --test integration_answer_evidence --quiet`
  - `cargo test -p ralph-core smoke_runner --quiet`
  - `openspec validate clean-current-runtime-evidence-and-dynamic-role-contract --type change --strict`
  - `ralph record summary <session.jsonl> --agents-file <agents.json>` 并检查 `topology.spawn_group: 1`,`topology.spawn.result: 1`,`topology.spawn.failed: 0`,`analysis.done: 3`,`reason: CompletionPromise`。


### exp-20260529-scoped-commit-in-mixed-worktree
> 在混杂工作区做 scoped commit 时,只把 staged index 当提交真相源。先证明 staged 里没有上下文/临时状态文件,再 commit,最后用空 index 证明提交边界。
<!-- scope: project | source_topics: recoverable_retry_scoped_commit,continuous_learning | source_hats: codex | status: active | confidence: high | created_at: 2026-05-29T18:00:00+08:00 | updated_at: 2026-05-29T18:00:00+08:00 | supersedes:  -->

- 触发条件:
  - 工作区有大量未暂存支线改动,但用户只允许提交当前主线 staged patch。
  - git status --short 出现大量未暂存/未跟踪项,且部分同文件存在 staged + unstaged 双层改动。
  - 要提交 recoverable/runtime 类主线,但六文件上下文、.omx/state 或其它支线不能混入。
- 已验证做法:
  - 提交前运行 git diff --cached --name-status 确认 intended files。
  - 运行 git diff --cached --check 确认 staged patch 基础质量。
  - 运行 staged forbidden context check,确保 task_plan / notes / WORKLOG / ERRORFIX / LATER_PLANS / EPIPHANY_LOG / .omx 不在 index。
  - 运行 git submodule status,确认没有意外 submodule 指针变化。
  - commit 后运行 git diff --cached --name-status,必须为空。
- 关键边界:
  - 不要为了 commit 方便执行整仓 git add .。
  - 不要因为同一个文件还有 unstaged 修改就否定 staged commit;Git 可以只提交 index 中那一层。
  - 如果 pre-commit hook 读取整个 working tree 而非 staged patch 并失败,先记录证据,不要立即绕过 hook。
- 验证锚点:
  - 本轮 commit: 8bf37643 feat: add recoverable agent cli retry lifecycle。
  - 提交前 staged forbidden context check 无输出。
  - 提交后 git diff --cached --name-status 为空。

### exp-20260529-spec-code-drift-needs-reconciliation-first
> OpenSpec tasks 的勾选状态不是实现事实。若 tasks 声称 TUI 富渲染已完成,但 Cargo 依赖和代码注释显示仍是纯文本模型,下一步必须先做 spec-code reconciliation。
<!-- scope: project | source_topics: evolution_analysis,tui_mdfried_viewer,spec_code_drift | source_hats: codex | status: active | confidence: medium | created_at: 2026-05-29T18:00:00+08:00 | updated_at: 2026-05-29T18:00:00+08:00 | supersedes:  -->

- 触发条件:
  - 继续 tui-mdfried-viewer 或任何 OpenSpec change 时,看到 tasks 已勾选但代码依赖/模块结构对不上。
  - 用户问某个功能是否已经实现,而证据来自 tasks 或计划文档,不是当前代码。
- 已观察现象:
  - tui-mdfried-viewer tasks 曾标记 ratatui-image、OutputBlock::{Text, Image} 和 Big Headers 已完成。
  - 当前 ralph-tui 依赖和 parallel output buffer 注释显示仍是纯文本行模型。
- 关键边界:
  - 不能把 tasks 勾选直接表述成已验证实现事实。
  - 先看当前代码、Cargo 依赖、测试和 record/screenshot evidence,再决定恢复实现、修正 tasks 状态,还是开 correction change。
- 验证锚点:
  - 支线归档位置: archive/branch_contexts/evolution_analysis/。
  - 后续入口: LATER_PLANS.md 中 tui-mdfried-viewer spec-code reconciliation 项。
