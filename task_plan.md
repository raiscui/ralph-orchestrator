# 任务计划: ralph-orchestrator 当前主线上下文指针

> 创建于 2026-08-17 `$continuous-learning` round。
> 上一代 task_plan (44 个 section, 2026-08-13 ~ 2026-08-17 Wave 1/2/3.4 + §16/§17/§18 + minimax + Forge/RPC 评估 + Round 6 follow-ups) 已归档到
> `archive/default_history/task_plan_2026-08-17_pre_continuous_learning_wave3_4_complete.md`。
> 配套 manifest: `archive/manifests/ARCHIVE_MANIFEST__continuous_learning_wave_3_4_completion_2026-08-17.md`。

## 当前主线状态

- **HEAD**: `971e5710 docs(e2e): remove stale struct-type references from README + 28 scenario YAML headers` (latest, pushed to my/main)
- **`my/main` 上 5 天连续 19 commits** —— Wave 3.4 + §16/§17/§18 + minimax 修复 + Round 6 全部完成。
- **declarative coverage gate = 100% PASS** (60 Declarative + 0 Imperative + 1 ImperativeExplicitKeep, 阈值 90%)
- **编译 deprecation warning: 0** (Round 6 净效应,从 297+ → 0)
- **E2E lib tests: 325 passed; 7 doctests pass**

## 索引跳点

按活跃度排序,后续任务应按这个顺序读:

1. **`docs/solutions/documentation-gaps/declarative-scenario-migration.md`** —
   Wave 2 schema patterns + Wave 3.4 收尾 (Round 6 物理删除 ~11K 行)。所有
   future declarative migration / cleanup 任务的复用入口。
2. **`docs/solutions/minimax-full-auto-compat/README.md`** —
   `--full-auto` → `--sandbox danger-full-access` 双 path 修复
   (declarative YAML + cli_backend.rs Round 5),含 minimax live 4/4 PASSED 凭据。
3. **`CONTEXT.md`** Wave 3.4 段 — `declarative coverage = 100%` 实际状态 +
   `#[deprecated] = dead code` 判定规则。
4. **`LATER_PLANS.md`** — 当前待办(§18 Claude session peak extraction,
   backpressure flake, lazy-model-completion drift fix)。
5. **`EXPERIENCE.md`** — 历史捕获的项目级经验收件箱。

## 短期可做 (按 ROI)

| Rank | 任务 | ROI | 备注 |
|---|---|---|---|
| 1 | `docs/solutions/lazy-model-completion/README.md` 前置字段修复 (frontmatter + problem_type enum) | 低成本 / 信息准确 | 不在 Round 6 范围但 pre-existing drift |
| 2 | §18 Claude session peak extraction PR | 高(打通完整 telemetry 通路) | 单开 OpenSpec change, ~452 行 origin 重写 + 本地 job/ adapter 适配 |
| 3 | backpressure flake (Codex 0.147.0) 观察 | 取决于 Codex 升级 | 无代码动作,等下次 live 跑 |
| 4 | 5 个删除 Wave 3.4 续候选项 | 低(major cleanup 已 done) | 0 file left |

## 状态

**Active task 大集已 close。** 后续任务在 `LATER_PLANS.md` 与本文件索引跳点处。
