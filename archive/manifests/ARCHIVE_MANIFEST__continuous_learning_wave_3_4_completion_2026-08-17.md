# Archive Manifest — Continuous Learning 归档 (Wave 3.4 + Round 6 收尾, 2026-08-17)

**触发原因**: `task_plan.md` 在 2026-08-13 ~ 2026-08-17 期间累计 44 个 section / 1551 行,
超过 1000 行阈值 (per `文件上下文工作模式`);显式调用 `$continuous-learning`
要求完整复盘与归档。

## 归档项

| 原路径 | 新路径 | 行数 | 说明 |
| --- | --- | --- | --- |
| `./task_plan.md` | `./archive/default_history/task_plan_2026-08-17_pre_continuous_learning_wave3_4_complete.md` | 1551 | 5 天累计 (Wave 1/2/3.4 + §16/§17/§18 + minimax + Round 6) |

新的 `task_plan.md`(43 行)是 active 任务索引跳点,不再含历史 section。

## 已消费但未归档的同级文件

| 文件 | 行数 | 处理 |
| --- | --- | --- |
| `notes.md` | 790 | < 1000, 不归档;继续记录 active 任务的研究笔记 |
| `WORKLOG.md` | 950 | < 1000, 不归档;本归档后追加 §19 Wave 6 + follow-ups 总收尾 entry |
| `LATER_PLANS.md` | 277 | < 1000, 不归档;§18 Claude session peak 等任务仍在留待办 |
| `ERRORFIX.md` | 303 | < 1000, 不归档;Round 6 没有引入新的错误修复 |
| `EPIPHANY_LOG.md` | 972 | < 1000, 不归档;5 天内没有新灾难点 |
| `EXPERIENCE.md` | 659 | < 1000, 不归档;Round 6 的新经验落到 docs/solutions/ 而非本收件箱 |
| `CONTEXT.md` | 125 | < 1000, 不归档;本归档所产出的新事实已 append 到 glossary 段 |
| `AGENTS.md` | 456 | < 1000, 不归档;索引无需变更(已有 docs/solutions/ 入口) |

## 七项成熟度门禁评估总结

候选经验:

| 经验 | 7-门禁 | 处置 |
| --- | --- | --- |
| Wave 3.4 + Round 6 物理删除 `#[deprecated]` + legacy struct 的 4 步 pattern | ✅ 7/7 (已验证、非琐碎、可复用、边界明确、单一主题、无重叠、可复跑) | **Capture → 合并入** `docs/solutions/documentation-gaps/declarative-scenario-migration.md` "Wave 3.4 收尾" 段 |
| `--full-auto` → `--sandbox danger-full-access` 在 cli_backend.rs 的 Rust code path 修复 (Round 5) | ✅ 7/7 | **Scoped Refresh** `docs/solutions/minimax-full-auto-compat/README.md` frontmatter + 兼容性段 + 后续段 |
| declarative-scenario-migration 中 Wave 2 状态从 "65% → 100% 目标中" 改为 "100% 已达成 + Wave 3.4 收尾" | ✅ 7/7 | **Scoped Refresh** 同一文件 frontmatter `last_updated` + 新增段 |
| Forge / Robot RPC 评估结论 (DEFER / DROP) | 边界明确 (driven by no-installation + ADR-0001 阻止), 但与 ralph-orchestrator 维护低关联 | **不 Capture**;已 append 到 `LATER_PLANS.md` 留作未来 review |

## 关联长期载体变更

### Scoped Refresh

1. `docs/solutions/minimax-full-auto-compat/README.md`:
   - frontmatter: 升级到 schema v3 合规(补 `component` / `module` / `severity` / `status` /
     `date` / `last_updated` / `tags` / `verified_by` 等必填字段, `problem_type` 改为
     `integration_issue` 枚举值)
   - 内容: 兼容性段强调"codex-cli ≥ 0.147.0 也 reject --full-auto", 加 Rust code path
     修复(commit `005d840d`)与 YAML path 修复(commit `e2977175`)的并列表述
   - 后续段: 把 5 个 legacy Rust file follow-up 标注为已落地 (Wave 3.4 cleanup commit
     `ca54fb3b`),加上"CI enforcement TODO"以防回归

2. `docs/solutions/documentation-gaps/declarative-scenario-migration.md`:
   - frontmatter: `last_updated` 2026-08-13 → 2026-08-17
   - `verified_by` 改为反映 Wave 3.4 后的实际 test count (325 passed)
   - 新增 `## Wave 3.4 收尾 (2026-08-17)` 段,描述 Round 6 4 步物理删除 pattern
   - 新增 `related_solutions` 指针到 minimax-full-auto-compat (因为同次 Q3 plan 第 3.6 项)

### 新增 solution

无(2 个候选都浓缩为 Scoped Refresh,均合并入已有 canonical 文档)。

### AGENTS.md 索引

无需变更:`docs/solutions/` 入口早在 committed `5b33bd67` 之前已存在并指明 README.md / README.md
/ README.md 三条路径,本 refresh 不改变路径或可发现性。

### 验证结果

- `validate-solution-frontmatter.py docs/solutions/minimax-full-auto-compat/README.md` → OK
- `validate-solution-claims.py     docs/solutions/minimax-full-auto-compat/README.md` → OK (0 flags)
- `validate-solution-frontmatter.py docs/solutions/documentation-gaps/declarative-scenario-migration.md` → OK
- `validate-solution-claims.py     docs/solutions/documentation-gaps/declarative-scenario-migration.md` → OK (0 flags)

### 未触动

- `docs/solutions/lazy-model-completion/README.md`: pre-existing drift (frontmatter `architecture_flaw` 不在 enum,
  缺 5 个必填字段)。不在 Round 6 范围,**留 follow-up** (LATER_PLANS 已记录)。

## 已消费

新 `task_plan.md` 不含 Wave 1/2/3.4 历史细节;若后续需要回看,通过下列路径:

- 归档主体: `archive/default_history/task_plan_2026-08-17_pre_continuous_learning_wave3_4_complete.md`
- 本 manifest: `archive/manifests/ARCHIVE_MANIFEST__continuous_learning_wave_3_4_completion_2026-08-17.md`
- 关联 evidence: 5 天内的 19 commits 通过 `git log my/main` 容易遍历
