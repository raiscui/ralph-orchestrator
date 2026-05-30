# INCIDENT_PACKET

你正在主持一次 webhook 事故 war-room。
这个 packet 是给并行 lane 使用的结构化上下文。

## Incident Meta

- incident_id: INC-2026-0310
- severity: SEV-1
- service: api-gateway
- customer_impact: 12% webhook deliveries delayed
- started_at: 2026-03-10 09:12 UTC

## Triage Packet

- focus: confirm severity and commander
- expected_priority: SEV-1
- expected_owner: oncall-platform
- expected_status: ready
- evidence:
  - all failing requests route through the new signature cache layer
  - customer impact exceeds the SEV-2 threshold

## Logs Packet

- focus: identify strongest signal
- expected_signal: signature cache stampede after hot reload
- expected_status: ready
- evidence:
  - p95 latency spiked immediately after build 2026.03.09-rc3
  - cache misses and signature verification retries increased together

## Rollback Packet

- focus: choose immediate mitigation
- expected_action: rollback api-gateway to build 2026.03.09-rc2
- expected_status: ready
- evidence:
  - rc2 is the last known good build
  - rollback does not require database changes

## Status Packet

- focus: prepare customer update
- expected_message: degraded webhook delivery, mitigation in progress
- expected_status: ready
- evidence:
  - support already has three active customer tickets
  - status page can be updated without legal review

## Expected Final Outcome

- final_decision: EXECUTE_ROLLBACK
- customer_update: SEND_STATUS_PAGE_UPDATE
