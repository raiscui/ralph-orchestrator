## [2026-04-30 09:35:00] [Session ID: 019ddbe6-a5ae-7950-8ba0-27b3b9f53219] 任务名称: continuous-learning 持续学习整理

### 任务内容

- 执行用户触发的 `$continuous-learning`。
- 按默认六文件、支线六文件、旧支线分组读取上下文。
- 提取可复用经验,写入项目长期知识文件和项目级 skill。
- 归档已总结过的旧支线,保留今天活跃支线。

### 完成过程

- 创建本轮支线上下文:
  - `task_plan__continuous_learning.md`
  - `notes__continuous_learning.md`
  - `LATER_PLANS__continuous_learning.md`
  - `WORKLOG__continuous_learning.md`
- 读取并分组当前根目录六文件:
  - 默认组
  - `__memory_axes`
  - `__memory_boundary_fix`
  - `__tui_chat_missing`
  - `__serial_tui_issues`
  - `__continuous_learning`
- 判定活跃度:
  - 活跃保留: `__serial_tui_issues`, `__continuous_learning`
  - 总结后归档: `__memory_axes`, `__memory_boundary_fix`, `__tui_chat_missing`
- 归档旧支线:
  - `archive/branch_contexts/memory_axes/`
  - `archive/branch_contexts/memory_boundary_fix/`
  - `archive/branch_contexts/tui_chat_missing/`
- 续档默认 `notes.md`:
  - 旧文件超过 1000 行,已移动到 `archive/default_history/notes_2026-04-30_0925.md`
  - 新 `notes.md` 只保留续档说明和后续入口
- 新增归档 manifest:
  - `archive/manifests/ARCHIVE_MANIFEST__continuous_learning_2026-04-30_0918.md`
- 新增项目经验文件:
  - `EXPERIENCE.md`
- 更新索引:
  - `AGENTS.md`
- 新增项目级 self-learning skill:
  - `.codex/skills/self-learning.rust-utf8-safe-string-truncation/SKILL.md`
- 检查 docs / specs / OpenSpec:
  - `rerun-runtime-graphs` 的 V2 剩余项已经在 OpenSpec tasks 中准确存在,无需改正式 specs。
  - `docs/concepts/memories-and-tasks.md` 已说明 runtime lower-case `experience.md`; 本轮在 `AGENTS.md` 中说明 uppercase `EXPERIENCE.md` 是 agent-facing 经验文件。

### 验证

- `git diff --check -- AGENTS.md EXPERIENCE.md task_plan.md task_plan__continuous_learning.md notes.md notes__continuous_learning.md LATER_PLANS__continuous_learning.md archive/manifests/ARCHIVE_MANIFEST__continuous_learning_2026-04-30_0918.md .codex/skills/self-learning.rust-utf8-safe-string-truncation/SKILL.md` 通过。
- 归档路径检查通过:
  - `archive/branch_contexts/memory_axes/`
  - `archive/branch_contexts/memory_boundary_fix/`
  - `archive/branch_contexts/tui_chat_missing/`
  - `archive/default_history/notes_2026-04-30_0925.md`
- `cargo test --quiet` 通过。

### 总结感悟

- 这轮最有价值的收获不是“多写一个文件”,而是把几个容易丢的判断口径放到了更稳定的位置:
  - Rust UTF-8 字符预算不能混用 byte index。
  - TUI chat 是否存在先看 serial / parallel mode。
  - Rerun runtime graph 的 V1 live 与 V2 durable replay 必须分层。
  - 支线六文件要按活跃度判断,总结后整组归档。
- 旧 `archive/` 根层仍有早期平铺历史文件,但这不是本轮必须展开的工作; 已写入 `LATER_PLANS__continuous_learning.md`。
