# Archive Manifest: continuous learning parallel_rec cleanup 2026-05-20 07:48

## 触发条件

- 用户显式触发 `$continuous-learning`。
- `notes__parallel_rec_analysis.md` 超过 1000 行。
- 默认 `WORKLOG.md` 已 997 行,本轮完成记录会触发超过 1000 行。

## 本次已检索/总结的上下文集

### 默认组

- `task_plan.md`
- `notes.md`
- `WORKLOG.md`
- `LATER_PLANS.md`
- `ERRORFIX.md`
- `EPIPHANY_LOG.md`

默认组没有整体归档。仅对接近 1000 行的 `WORKLOG.md` 做续档。

### `display_info_evidence`

活跃度判定: 未轮转旧支线。

归档文件:

- `task_plan__display_info_evidence.md`
- `notes__display_info_evidence.md`
- `WORKLOG__display_info_evidence.md`
- `ERRORFIX__display_info_evidence.md`

可复用结论:

- TUI/CLI display surfaces 不是 durable truth。
- Output status strip 必须和正文 viewport 共享同一几何 helper。

长期沉淀:

- `EXPERIENCE.md#exp-20260519-parallel-output-status-strip-viewport`

### `multi_agent_collab_evidence`

活跃度判定: 未轮转旧支线。

归档文件:

- `task_plan__multi_agent_collab_evidence.md`
- `notes__multi_agent_collab_evidence.md`
- `WORKLOG__multi_agent_collab_evidence.md`

可复用结论:

- 当前仓库的真实协作 runtime 是 parallel hat instances。
- focused tests / E2E scenario registration / live Codex E2E 是三层不同证据。

长期沉淀:

- `EXPERIENCE.md#exp-20260520-multi-agent-collaboration-evidence-layers`
- 根 `LATER_PLANS.md` 中的 live Codex E2E 后续建议。

### `parallel_rec_analysis`

活跃度判定: 刚完成并触发续档的支线。

归档文件进入 `archive/branch_contexts/parallel_rec_analysis/snapshots/2026-05-20_0748/`:

- `task_plan__parallel_rec_analysis.md`
- `notes__parallel_rec_analysis.md`
- `WORKLOG__parallel_rec_analysis.md`
- `LATER_PLANS__parallel_rec_analysis.md`
- `ERRORFIX__parallel_rec_analysis.md`
- `EPIPHANY_LOG__parallel_rec_analysis.md`

可复用结论:

- `capability.request` 是 isolated child/micro-run,不会改 parent topology。
- parent-visible 多实例要走 `topology.spawn_group`。
- `topology.spawn.result` 是 acknowledgement,不能触发重新投递原始 `delivery_topic`。
- `audience_instances` 不是 replay 或实例创建机制。

长期沉淀:

- `EXPERIENCE.md#exp-20260520-topology-spawn-result-ack-guardrail`
- `specs/parent-visible-topology-spawn-observability.spec.md`
- `docs/plans/2026-05-19-parent-visible-topology-spawn-and-child-run-observability.md`
- `docs/runbook/runtime-capabilities.md`
- 根 `LATER_PLANS.md` 中的 dogfood worker 收敛与 task-derived role contract 后续建议。

## 验证

- Mermaid blocks in `specs/parent-visible-topology-spawn-observability.spec.md` rendered successfully with `beautiful-mermaid-rs --ascii`。
- 归档后应运行 `git diff --check`。
