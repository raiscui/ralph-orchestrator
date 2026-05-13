# Archive Manifest: continuous-learning 批次 `2026-04-30_0918`

## 六文件摘要

- 本轮触发:
  - 用户显式执行 `$continuous-learning`。
  - 当前仓库根目录存在默认六文件、多套支线六文件和旧支线残留,需要按持续学习流程先总结再归档。
- 本轮事实来源:
  - `notes__continuous_learning.md`
  - `task_plan__continuous_learning.md`
  - 旧支线 `__memory_axes`, `__memory_boundary_fix`, `__tui_chat_missing`
  - 活跃支线 `__serial_tui_issues`

## 活跃度判定

- 保持根目录活跃:
  - `task_plan__serial_tui_issues.md`
  - `task_plan__continuous_learning.md`
  - `notes__continuous_learning.md`
- 本轮归档:
  - `__memory_axes`
  - `__memory_boundary_fix`
  - `__tui_chat_missing`
- 不归档默认六文件:
  - 默认组是项目当前主上下文,本轮只读取和总结。
  - 例外: 旧 `notes.md` 已超过 1000 行,且本轮已完成摘要,因此按六文件续档规则移动到 `archive/default_history/notes_2026-04-30_0925.md`,并重建新的 `notes.md`。

## 归档映射

### `__memory_axes`

- `task_plan__memory_axes.md` -> `archive/branch_contexts/memory_axes/task_plan__memory_axes.md`
- `notes__memory_axes.md` -> `archive/branch_contexts/memory_axes/notes__memory_axes.md`
- `WORKLOG__memory_axes.md` -> `archive/branch_contexts/memory_axes/WORKLOG__memory_axes.md`
- `LATER_PLANS__memory_axes.md` -> `archive/branch_contexts/memory_axes/LATER_PLANS__memory_axes.md`
- `ERRORFIX__memory_axes.md` -> `archive/branch_contexts/memory_axes/ERRORFIX__memory_axes.md`
- `EPIPHANY_LOG__memory_axes.md` -> `archive/branch_contexts/memory_axes/EPIPHANY_LOG__memory_axes.md`

### `__memory_boundary_fix`

- `task_plan__memory_boundary_fix.md` -> `archive/branch_contexts/memory_boundary_fix/task_plan__memory_boundary_fix.md`
- `notes__memory_boundary_fix.md` -> `archive/branch_contexts/memory_boundary_fix/notes__memory_boundary_fix.md`
- `WORKLOG__memory_boundary_fix.md` -> `archive/branch_contexts/memory_boundary_fix/WORKLOG__memory_boundary_fix.md`
- `ERRORFIX__memory_boundary_fix.md` -> `archive/branch_contexts/memory_boundary_fix/ERRORFIX__memory_boundary_fix.md`

### `__tui_chat_missing`

- `task_plan__tui_chat_missing.md` -> `archive/branch_contexts/tui_chat_missing/task_plan__tui_chat_missing.md`
- `notes__tui_chat_missing.md` -> `archive/branch_contexts/tui_chat_missing/notes__tui_chat_missing.md`
- `WORKLOG__tui_chat_missing.md` -> `archive/branch_contexts/tui_chat_missing/WORKLOG__tui_chat_missing.md`

### 默认组续档

- `notes.md` -> `archive/default_history/notes_2026-04-30_0925.md`
- 新 `notes.md` 已重建,只保留续档说明和后续入口。

## 沉淀去向

- `EXPERIENCE.md`:
  - 项目级经验沉淀,保存本轮提取的 TUI mode 判断、runtime graph 分层、持续学习归档口径和 Rust UTF-8 截断经验。
- `.codex/skills/self-learning.rust-utf8-safe-string-truncation/SKILL.md`:
  - 项目级 self-learning skill,用于未来遇到 Rust `byte index ... is not a char boundary` 或字符预算截断时召回。
- `AGENTS.md`:
  - 增加长期知识索引,避免新文件落地后失联。

## 后续入口

- 如果继续 `serial_tui_issues`:
  - 先读根目录的 `task_plan__serial_tui_issues.md`。
- 如果继续 Rerun runtime graph:
  - 先读 `openspec/changes/rerun-runtime-graphs/tasks.md`,从 V2 durable replay 的 `3.1` 到 `3.4` 开始。
- 如果追溯本轮归档:
  - 先读本 manifest,再按主题进入 `archive/branch_contexts/<topic>/`。
