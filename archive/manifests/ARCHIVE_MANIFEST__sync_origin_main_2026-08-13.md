# Archive Manifest: sync-origin-main-features-q3-2026 分支上下文整理 `2026-08-13`

## 触发条件

- 用户显式执行 `$continuous-learning` + "整理清理所有根目录 分支上下文文件"。
- 当前仓库根目录存在 4 个 `notes__*.md` 支线文件, 来自上次 sync-origin-main
  cherry-pick 调查工作 (Session omx-1786419140441-df5ql8, 2026-08-11/12)。
- sync-origin-main-features-q3-2026 整体工作已 commit `c623abb` 完成(2026-08-12)并
  已归档到 `openspec/changes/archive/2026-08-12-sync-origin-main-features-q3-2026/`,
  这 4 个调查 notes 是 sync-origin-main 工作的过程产物, 不再被当前 Session 引用。

## 六文件活跃度判定

- 当前 Session ID: `omx-1786600320381-z290x9` (Wave 2 declarative migration, 14:10 起)。
- 当前 Session 工作主题: e2e declarative scenario migration (Coverage 65%→100%)。
- 4 个 `notes__*.md` Session ID:
  - `notes__branch_diff_review.md` → omx-1786419140441-df5ql8, 2026-08-11
  - `notes__clean_events_review.md` → omx-1786419140441-df5ql8, 2026-08-11
  - `notes__e2e_conv.md` → 无 Session ID, 2026-08-02 (最早, 无标准时间戳)
  - `notes__group1_dryrun.md` → omx-1786419140441-df5ql8, 2026-08-12
- 当前 Session 6 文件对 4 个 notes__* 的引用次数:
  - task_plan.md / WORKLOG.md / EPIPHANY_LOG.md / LATER_PLANS.md / ERRORFIX.md / notes.md: 全 0 引用
- 结论: 0 引用 + 异 Session + 不同主题 = **未轮转旧支线**, 按 continuous-learning
  规则归档到 `archive/branch_contexts/`。

## 本轮归档映射

### `__branch_diff_review`

- `notes__branch_diff_review.md` (root, 191 行)
  → `archive/branch_contexts/branch_diff_review/notes__branch_diff_review.md`

### `__clean_events_review`

- `notes__clean_events_review.md` (root, 126 行)
  → `archive/branch_contexts/clean_events_review/notes__clean_events_review.md`

### `__e2e_conv`

- `notes__e2e_conv.md` (root, 6 行)
  → `archive/branch_contexts/e2e_conv/notes__e2e_conv.md`

### `__group1_dryrun`

- `notes__group1_dryrun.md` (root, 80 行)
  → `archive/branch_contexts/group1_dryrun/notes__group1_dryrun.md`

## Compound Capture / Refresh 结果

### 已捕获的成熟经验 (本轮)

- **EXP-20260813-e2e-live-convergence-issue** (`EXPERIENCE.md`):
  e2e live 场景失败模式: termination_reason=None, 事件流完整但无 loop.terminate。
  来源: `notes__e2e_conv.md` + Wave 2 declarative migration 期间观察验证。
  状态: active, confidence: medium (根因未知, 证据缺口明确)。
  未来捕获条件: 根因定位 + 修复方案后升级到 docs/solutions/ formal capture
  (problem_type: runtime_error 或 live_convergence)。

### 未捕获的研究材料 (保留在 archive)

- `notes__branch_diff_review.md`: sync-origin-main 分支差异分析 (1818 文件, 24 万行净变更,
  两边独有文件镜像关系分析)。是 sync-origin-main 工作的过程产物, 不是 reusable 知识。
- `notes__clean_events_review.md`: commit e88b7e3 (ralph clean --events) 移植价值分析。
  是具体 commit 移植决策的调查笔记, 不通用化。
- `notes__group1_dryrun.md`: Group 1 cherry-pick dry-run 结果 (5/6 失败, 根因是本地 main
  主动删了文件)。是 sync-origin-main 工作的具体执行记录。

### Scoped Refresh

- 现有 captures (上轮 a7daa79 + 上上轮 fe71186):
  - 2 self-learning skills (yaml-schema-or-vs-and-semantics, yaml-duplicate-field-bug) 仍 active。
  - 4 exp-20260813-* entries 仍 active。
  - docs/solutions/documentation-gaps/declarative-scenario-migration.md frontmatter OK
    (上次 fe71186 修复)。
- 无漂移, 无需 Refresh 任何已有 captures。

### skill / glossary / AGENTS 同步

- 无新 skill 需建 (本轮无新的可执行流程模式)。
- 无新 glossary 术语需写。
- AGENTS.md Project Knowledge Index 无需更新 (上轮已 sync 4 个 Wave 2 相关条目)。

## 验证

- `cargo test -p ralph-e2e --lib`: 536 passed / 0 failed / 24 ignored (无回归)
- `cargo test -p ralph-e2e --test declarative_coverage_gate`: Coverage 100.00% PASS
- `git ls-files notes__*.md`: 0 个 (全部已 archive)
- `git status --short`: 仅新增 archive 目录文件 + EXPERIENCE.md 改动, 无未跟踪分支文件

## 保留在 `EXPERIENCE.md` 的候选

- `exp-20260813-e2e-live-convergence-issue` — 证据缺口明确(根因未知), 待诊断后
  升级 docs/solutions/。

## 未完成风险与后续建议

- sync-origin-main 工作的整体过程产物(包括 cherry-pick candidates, conflict analysis
  等) 已 commit `c623abb`; 本轮 archive 的 4 个 notes 是 commit 后产生的细粒度调查笔记,
  不影响 sync-origin-main 主工作流。
- 未来若 sync-origin-main 工作重启, 可从 `openspec/changes/archive/2026-08-12-.../`
  + `archive/branch_contexts/*` 完整还原历史。
- `notes__e2e_conv.md` 内容是真正未解的 LIVE 路径问题; Wave 3 (#[deprecated] + docs +
  follow-up issue) 期间应排入诊断工作。
