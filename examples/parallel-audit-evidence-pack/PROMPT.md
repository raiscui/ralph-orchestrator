# AUDIT_PACKET

你正在组织一次审计前的并行证据收集。
这个 packet 只包含并行判断所需的结构化上下文。

## Audit Meta

- audit_id: AUD-2026-Q2
- framework: SOC2
- control_owner: compliance-ops
- auditor_window: 2026-04-08

## Access Evidence Packet

- focus: confirm access review export is ready
- expected_status: ready
- expected_access_export_window: last_90_days
- evidence:
  - privileged access review export completed for the last quarter
  - break-glass access entries are tagged and traceable
  - access review owners already signed the quarterly review

## Change Log Packet

- focus: confirm change evidence is packaged
- expected_status: ready
- expected_change_log_window: last_90_days
- evidence:
  - deployment log keeps approver identity and timestamp
  - emergency changes are separately flagged in the log
  - release calendar can map changes back to the approval record

## Backup Verification Packet

- focus: confirm backup restore evidence is current
- expected_status: ready
- expected_backup_restore_status: verified_last_30_days
- evidence:
  - restore drill completed during the last 30 days
  - backup retention policy covers the audit window
  - evidence includes both backup job status and restore verification

## Incident History Packet

- focus: confirm incident evidence is collected
- expected_status: ready
- expected_incident_window: last_12_months
- evidence:
  - incident register includes all sev1 and sev2 events from the last year
  - each incident entry links to review notes or closure records
  - no open sev1 incident is waiting for audit explanation

## Expected Final Outcome

- audit_status: READY_FOR_AUDITOR
- control_set: soc2_cc7_cc8
- owner: compliance-ops
