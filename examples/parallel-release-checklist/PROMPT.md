# RELEASE_PACKET

你正在准备 `v2.4.0` 的发布窗口。
下面不是完整发布单。
它是一份给并行 checker 使用的结构化 packet。

## Release Meta

- version: v2.4.0
- target_window: 2026-03-10 10:00 UTC
- owner: platform-release

## QA Packet

- checklist_lane: qa
- expected_status: ready
- evidence:
  - smoke suite: pass
  - migration dry-run: pass
  - rollback smoke: pass

## Docs Packet

- checklist_lane: docs
- expected_status: ready
- evidence:
  - release notes drafted
  - upgrade guide linked
  - known issues section updated

## Ops Packet

- checklist_lane: ops
- expected_status: ready
- evidence:
  - rollback owner assigned
  - canary window booked
  - on-call handoff confirmed

## Expected Final Outcome

- final_status: release.ready
- summary_should_include:
  - version
  - smoke suite status
  - rollback readiness
