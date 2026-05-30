# VENDOR_PACKET

你正在组织一个新供应商引入前的并行审查。
这个 packet 只包含并行判断所需的结构化上下文。

## Vendor Meta

- vendor_id: VND-77
- vendor_name: ScoutGraph AI
- use_case: internal incident triage copilots
- target_start: 2026-05-20

## Security Packet

- focus: confirm baseline security controls
- expected_status: ready
- expected_required_controls: sso_scim_audit_logs
- evidence:
  - vendor supports SSO and SCIM provisioning
  - audit logs are available via admin export
  - support engineers do not get default production access

## Privacy Packet

- focus: confirm privacy boundary
- expected_status: ready
- expected_privacy_scope: no_training_on_customer_pii
- evidence:
  - vendor contract forbids model training on customer payloads
  - retention window can be reduced for support exports

## Procurement Packet

- focus: confirm purchasing path
- expected_status: ready
- expected_procurement_path: msa_plus_security_addendum
- evidence:
  - finance approved pilot budget line item
  - procurement requested a short-form addendum instead of a new master contract

## Legal Packet

- focus: confirm legal signing path
- expected_status: ready
- expected_term_path: standard_dpa_plus_security_addendum
- evidence:
  - vendor accepts standard DPA with security addendum
  - no data residency exception is pending

## Expected Final Outcome

- decision: APPROVE_PILOT
- required_controls: sso_scim_audit_logs
- procurement_path: msa_plus_security_addendum
