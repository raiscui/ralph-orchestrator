# ROLLOUT_PACKET

你正在准备一次受控上线。
自动化检查都可以先做。
但最终执行必须等人批准。

## Rollout Meta

- rollout_id: rollout-2026-03-10-01
- target_window: 2026-03-10 10:00 UTC
- approval_owner: release-manager

## Deployment Packet

- checklist_lane: deployment
- expected_status: ready
- evidence:
  - rollout window reserved
  - canary host selected
  - execution owner assigned

## Rollback Packet

- checklist_lane: rollback
- expected_status: ready
- evidence:
  - rollback script path confirmed
  - rollback owner assigned
  - verification window reserved

## Comms Packet

- checklist_lane: comms
- expected_status: ready
- evidence:
  - status page draft ready
  - incident bridge channel prepared
  - support handoff confirmed

## Approval Rule

- after_all_ready: emit `approval.requested`
- finish_only_after: `approval.granted`

## Example External Approval

```bash
ralph emit approval.granted --json '{"approved_by":"release-manager","window":"2026-03-10 10:00 UTC"}' --target-instance ralph#1
```
