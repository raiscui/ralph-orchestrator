# ESCALATION_PACKET

你正在组织一次高优先级客户 support escalation desk。
这个 packet 只包含并行判断所需的结构化上下文。

## Escalation Meta

- case_id: ESC-2049
- account_name: Summit Grid Systems
- service_tier: enterprise
- escalation_owner: support-director

## Case Triage Packet

- focus: confirm the active customer case is correctly triaged
- expected_status: ready
- expected_case_state: reproducible_with_clear_customer_impact
- evidence:
  - frontline support reproduced the failure on the latest supported workflow
  - the customer impact is isolated to a high-value production path
  - timeline and requested recovery target were confirmed with the customer

## Product Assessment Packet

- focus: confirm product engineering assessment for the defect path
- expected_status: ready
- expected_product_disposition: bug_backlog_promoted_for_hotfix
- evidence:
  - product support matched the behavior to a known regression window
  - engineering agreed the defect needs hotfix prioritization
  - workaround coverage is partial and does not remove the escalation need

## Account Risk Packet

- focus: confirm account risk and business exposure
- expected_status: ready
- expected_account_risk: renewal_exposure_managed_with_exec_visibility
- evidence:
  - the account team identified renewal sensitivity if recovery drifts
  - executive sponsor was informed of the current risk posture
  - success plan owners are aligned on daily status expectations

## Comms Packet

- focus: confirm the outbound communications plan
- expected_status: ready
- expected_comms_plan: daily_exec_update_with_customer_bridge
- evidence:
  - support committed to a customer bridge with named moderators
  - the next executive update window is already reserved
  - internal stakeholders agreed on one owner for customer-facing updates

## Expected Final Outcome

- escalation_status: READY_FOR_EXECUTION
- severity: SEV_2
- next_update_owner: support-director
