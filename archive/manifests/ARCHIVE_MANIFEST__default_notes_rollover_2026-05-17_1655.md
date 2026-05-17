# 默认 notes 续档清单

- 时间: 2026-05-17 16:55:40 +0800
- Session ID: omx-1779004640353-blcixq
- 原因: 根目录 `notes.md` 已超过 1000 行,按六文件规则必须续档。
- 当前主线: TUI 与 Codex/CLI 直出差异排查,以及并行 TUI 状态摘要最小实现。

## 六文件摘要

- 涉及的上下文集: 默认六文件。
- 任务目标: 找出 TUI 与 Codex/CLI 直出差异,并最小增强 TUI 当前状态可见性。
- 关键决定: 不把 TUI 改成 stdout 全量镜像;先补状态摘要,raw/audit 视图后续再做。
- 关键发现: runtime truth 存在于 output/event/agents/cassette 等位置,TUI 主画面聚合不足。
- 实际变更: `ralph-tui` Instances 显示 `job x/y`,Footer 显示 selected instance、job、render mode、last event。
- 暂缓事项: raw/audit 视图、stderr visible/hidden、evidence/status 面板、last_input.preview 正式接入。
- 错误与根因: Footer verbose label 在 80 列下截断 last event;已改为紧凑摘要。
- 可复用点: 并行 TUI 状态增强应复用现有 `ParallelTuiState` / `InstanceViewState` / `TuiState.last_event`。
- 沉淀位置: 已追加 `EXPERIENCE.md` 条目 `exp-20260517-parallel-tui-status-summary`。

## 归档文件

- `notes.md` -> `archive/default_history/notes_2026-05-17_1655_tui_status_prev.md`

## 当前 root context left active

- `task_plan.md`
- `notes.md` (new rollover entry)
- `WORKLOG.md`
- `LATER_PLANS.md`
- `ERRORFIX.md`
- `EPIPHANY_LOG.md`

## 后续当前任务

- 完成最终 diff/status 复核。
- 将 ultrawork state 标记完成。
- 向用户交付变更、验证和剩余建议。
