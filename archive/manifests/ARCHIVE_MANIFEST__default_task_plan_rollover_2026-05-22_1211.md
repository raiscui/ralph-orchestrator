# Archive Manifest: default task_plan rollover after clean live dogfood

## Batch

- Date: 2026-05-22 12:11:00 +0800
- Session ID: omx-1779158263949-kticiv
- Trigger: `task_plan.md` reached 1018 lines after clean live dogfood closure.
- Reason: Project six-file rule requires rollover after exceeding 1000 lines.

## Summary

This rollover keeps the current default six-file context active while moving the oversized `task_plan.md` to default history.
Reusable knowledge was extracted into `EXPERIENCE.md` before rollover.

## Files copied/moved to default history

- `task_plan.md` -> `archive/default_history/task_plan_2026-05-22_1211_prev_clean_live_dogfood.md`

## Current root context left active

- `task_plan.md` was recreated as a short current-entry file.
- `notes.md`, `WORKLOG.md`, `LATER_PLANS.md`, `ERRORFIX.md`, `EPIPHANY_LOG.md` remain active in root.

## Long-term knowledge updated

- `EXPERIENCE.md`: `exp-20260522-clean-live-dogfood-record-session-vs-agents-snapshot`

## Key retained guidance

- Clean live dogfood should use a temporary clean config, not long-term `ralph.yml`.
- `record-session` Evidence Inspect is the historical truth source.
- `.ralph/agents.json` is a current registry sidecar and may omit completed dynamic instances after TTL reaping.

## Verification

- `task_plan.md` line count after rollover is small.
- Archive manifest records the rollover path.
