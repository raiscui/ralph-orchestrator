# MIGRATION_REHEARSAL_PACKET

你正在主持一次数据库迁移 rehearsal。
这个 packet 只包含并行判断所需的结构化上下文。

## Migration Meta

- migration_id: MIG-2026-0415
- target_release: v5.2.0
- database: tenant_primary
- window: 2026-04-15 01:00 UTC

## Schema Packet

- focus: confirm schema diff and compatibility path
- expected_status: ready
- expected_key_change: add nullable checksum column before backfill
- evidence:
  - the new column is nullable during phase 1
  - application code already tolerates the missing field

## Backup Packet

- focus: verify backup and restore point
- expected_status: ready
- expected_snapshot: snap-2026-04-15T00-30Z
- evidence:
  - snapshot restore was validated in staging yesterday
  - retention window is 14 days

## Smoke Packet

- focus: confirm rehearsal validation on restored data
- expected_status: ready
- expected_verification: smoke suite green on restored staging copy
- evidence:
  - write path, read path and reconciliation job all passed
  - migration runtime stayed below the 10 minute budget

## Rollback Packet

- focus: confirm reversal procedure
- expected_status: ready
- expected_reversal: revert migration 2026_04_add_checksum
- evidence:
  - rollback playbook was dry-run twice
  - DB owner and release owner are both on-call in the planned window

## Expected Final Outcome

- go_no_go: GO
- rollout_plan: 15m_canary_then_full
