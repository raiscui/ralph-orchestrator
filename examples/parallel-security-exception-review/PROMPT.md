# EXCEPTION_PACKET

你正在组织一个内部安全例外申请的并行评审。
这个 packet 只包含并行判断所需的结构化上下文。

## Exception Meta

- exception_id: EXC-2026-17
- service: edge-admin-console
- requested_by: growth-platform
- target_expiry: 2026-06-30

## Threat Packet

- focus: confirm residual threat after the exception
- expected_status: ready
- expected_residual_risk: medium
- evidence:
  - admin console must stay reachable from partner office ranges during launch week
  - privileged paths remain behind SSO and device trust
  - threat model shows internet-wide exposure is still blocked

## Controls Packet

- focus: confirm compensating controls
- expected_status: ready
- expected_required_controls: waf_rate_limit_plus_audit
- evidence:
  - WAF rule pack can enforce partner-range allowlist plus rate limiting
  - audit logs already capture admin path access
  - alerting can page on repeated failed admin requests

## Data Scope Packet

- focus: confirm data boundary under the exception
- expected_status: ready
- expected_data_scope: masked_admin_metadata_only
- evidence:
  - no customer payload body is exposed through this path
  - only masked admin metadata is visible to support operators
  - exports remain disabled for the temporary access path

## Expiry Packet

- focus: confirm expiry and owner accountability
- expected_status: ready
- expected_expiry_date: 2026-06-30
- evidence:
  - business owner accepted a fixed sunset date
  - renewal requires a fresh exception review
  - rollback plan exists if the controls fail review

## Expected Final Outcome

- decision: APPROVE_WITH_COMPENSATING_CONTROLS
- required_controls: waf_rate_limit_plus_audit
- expiry_date: 2026-06-30
