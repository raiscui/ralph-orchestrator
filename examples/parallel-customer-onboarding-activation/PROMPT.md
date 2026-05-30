# ONBOARDING_PACKET

你正在组织一次客户正式 kickoff 前的并行 onboarding activation 评审。
这个 packet 只包含并行判断所需的结构化上下文。

## Onboarding Meta

- onboarding_id: ONB-2026-0421
- account_name: Helix Retail Ops
- kickoff_date: 2026-04-21
- owner_team: post-sales-activation

## Integration Packet

- focus: confirm integration path is ready
- expected_status: ready
- expected_integration_state: sandbox_ready
- evidence:
  - customer API sandbox credentials are already provisioned
  - sample payload mapping has been validated with the implementation lead
  - no blocker remains on the connector dependency list

## Security Handoff Packet

- focus: confirm security and access handoff
- expected_status: ready
- expected_security_state: access_model_confirmed
- evidence:
  - customer SSO owner is identified for the kickoff week
  - admin access roles were mapped to the deployment model
  - security questionnaire follow-up is already closed

## Enablement Packet

- focus: confirm enablement plan is ready
- expected_status: ready
- expected_enablement_state: training_tracks_scheduled
- evidence:
  - admin training and end-user enablement tracks are booked
  - the first workshop agenda is already drafted
  - customer champions accepted the rollout calendar

## Success Plan Packet

- focus: confirm first success milestone
- expected_status: ready
- expected_success_state: milestone_locked
- evidence:
  - first 30-day success metric is agreed with the customer sponsor
  - kickoff scorecard owners are already assigned
  - implementation dependencies are sequenced for week one

## Expected Final Outcome

- onboarding_status: READY_FOR_KICKOFF
- primary_risk: LOW
- first_milestone: api_sandbox_enablement
