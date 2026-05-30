# POSTMORTEM_PACKET

你正在整理一次 Sev-1 事故后的 action board。
这个 packet 只包含并行判断所需的结构化上下文。

## Postmortem Meta

- postmortem_id: PM-2026-0307
- incident_id: INC-431
- incident_name: parallel completion drained too early
- owner_team: runtime-platform
- review_meeting: 2026-03-12 10:00 UTC

## Timeline Packet

- focus: build the minimal operator-facing timeline
- expected_status: ready
- expected_anchor_event: coordinator mentioned LOOP_COMPLETE before finalizer ran
- evidence:
  - 09:41 UTC migration lanes all published ready events
  - 09:42 UTC coordinator prose mentioned LOOP_COMPLETE in a continuation
  - 09:42 UTC supervisor entered completion drain before finalizer job started

## Root Cause Packet

- focus: summarize the actual control-plane failure
- expected_status: ready
- expected_root_cause: completion promise was treated as prose substring instead of control token
- evidence:
  - runtime accepted LOOP_COMPLETE via substring match outside event tags
  - e2e termination detection used the same weak substring rule
  - finalizer route was recorded but no worker job actually started

## Action Mapping Packet

- focus: define the highest-value remediation item
- expected_status: ready
- expected_top_action: add_completion_promise_guardrail
- expected_owner: runtime-platform
- expected_due_date: 2026-05-15
- evidence:
  - parser should only accept event-external exact-line completion
  - e2e termination detection should reuse the same parser semantics
  - new examples should inherit the stricter rule

## Customer Recap Packet

- focus: draft the external recap stance
- expected_status: ready
- expected_message: no customer data risk, but control-plane completion semantics needed a hardening fix
- evidence:
  - the issue was isolated to orchestration completion detection
  - rollback guidance and action items were already available
  - no data loss occurred during the failed rehearsal

## Expected Final Outcome

- status: READY_FOR_REVIEW
- top_action: add_completion_promise_guardrail
- owner: runtime-platform
- due_date: 2026-05-15
