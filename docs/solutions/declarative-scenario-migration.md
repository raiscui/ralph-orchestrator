# E2E Declarative Scenario Migration Guide

## 背景

`crates/ralph-e2e/src/scenarios/` 下原本有两类场景:
- **Declarative**: `.yaml` 文件 + `crate::declarative::from_yaml(id, yaml)` 渲染。
- **Imperative**: Rust `TestScenario` impls + registry in `lib.rs::all_scenarios()`。

Q3 2026 Wave 2 完成了 21 个 imperative → declarative 迁移, 累计 Coverage 从 65% 推到 100% (除 `parallel-experimental-dev-engine-example` 永久保留 imperative)。本指南总结迁移套路,供后续增量迁移或类似 schema 迁移任务复用。

## Schema 字段映射 patterns

### Pattern 1: 1:1 平移 (最常见)
命令式断言直接映射到现有 schema 字段:
```yaml
expect:
  response_received: true
  exit_code_success_or_limit: true
  no_timeout: true
```

### Pattern 2: OR 折 AND (合理 stricter check)
命令式 OR (`A || B`) 拆 2 字段, runner AND 强制两者都通过 — 仅当命令式是 defensive 路径 (实际几乎总是某臂为 true) 时合理:
```yaml
# 命令式: verdict_provided (verdict 文本 || events review.*)
#   hat instructions 强制要求两者, AND 是更严格的"指令遵循"检查
expect:
  output_contains_any:
    - ["Approved", "approved", "NEEDS_CHANGES", "needs_changes"]
  events:
    - topic: review.done
      min_count: 1
```
详见 `.codex/skills/self-learning.yaml-schema-or-vs-and-semantics/SKILL.md`。

### Pattern 3: 保留 OR (单字段)
```yaml
# 命令式: execution_failed (exit_code != 0 || !stderr.is_empty())
expect:
  failed: true  # 单字段保留 OR 语义
```

### Pattern 4: case-insensitive via N case variants
```yaml
# 命令式: stdout_lowercase 含任一关键词 (builder/implement/build)
# schema output_contains case-sensitive, 用 N 个 case 变体覆盖
expect:
  output_contains_any:
    - ["Builder", "builder", "Implement", "implement", "Build", "build"]
```

### Pattern 5: starts_with via N 精确 topic
```yaml
# 命令式: events 任一 topic starts_with "build."
# schema event_count_at_least 只支持精确 topic, 用 N 条精确匹配
expect:
  events:
    - topic: build.task
      min_count: 1
    - topic: build.done
      min_count: 1
```

### Pattern 6: file content check → dropped
schema 无 `file_content` 字段, 但 `artifacts: [path]` 覆盖 file existence:
```yaml
# 命令式: file_exists AND content 非空含关键词
# dropped content 检查, artifacts 覆盖 existence
expect:
  artifacts:
    - .agent/memories.md
```

## Dropped assertions 决策 rubric

判断顺序:
1. 该 dropped 会让测试失真 (即无法捕获命令式意图的核心失败)?
   - 是 → schema 扩展 (1 字段 + builder + 测试, 镜像已有字段是 trivial)
   - 否 → 继续评估
2. 该 dropped 的命令式检查是 defensive 路径 (几乎永远通过)?
   - 是 → dropped 是合理选择
   - 否 → 考虑 schema 扩展
3. 多个类似 dropped 在不同 migrations 重复?
   - 是 → schema 扩展 (避免 N 次局部决策漂移)
   - 否 → dropped + YAML 注释说明 rationale

Wave 2 dropped 累计 ~15 条, 主要模式:
- file content 检查 (~6 条): dropped, artifacts 覆盖
- NEGATED stdout/stderr NOT contains (~5 条): dropped, 正向字段已 catch
- 冗余 defensive check (~4 条): dropped, response_received + exit_code_success_or_limit 已覆盖

## Setup 字段映射 patterns

### write_files (multi-purpose)
- 写入 fake codex shim Python 脚本 (`executable: true`, 用于 §2.4 parallel-app-server scenarios)。
- 预填充测试数据 (`memories.md` for memory scenarios)。
- 写入 test-data.txt (tool-use scenario)。

### path_prefix
- PATH 前部注入 workspace 相对目录 (`.e2e/bin` for fake codex shim)。

### inject (declarative runner 原生支持)
- `Wait { instance, state, timeout_secs }` — 等待实例达到状态 (`idle` / `running` / `running_then_idle`)。
- `Sleep { secs }` — 等待固定时长。
- `Assert { instance, state }` — 断言实例处于某状态 (不等待)。
- `Emit { topic, payload, target_instance, session_strategy, turn_action }` — 执行 `ralph emit`。
- `WaitEvent { topic, timeout_secs }` — 轮询 events.jsonl 等待事件。

## Audit 反预期 workflow

OpenSpec tasks.md 的 `§2.x Easy/Medium/Hard` 分类与命令式实际实现可能偏差 (Wave 2 累计 4 次反预期)。迁移前:

1. **读命令式 setup + run + 3-5 个 assertions 完整代码** (不要盲信 audit)。
2. **列 schema 已有字段** vs **命令式 assertion 语义**。
3. **diff**: 哪些命令式断言 → 1:1 schema 字段? 哪些 dropped? 哪些需 schema 扩展?
4. **schema 扩展决策**: 见上 "Dropped assertions 决策 rubric"。

详见 `.codex/skills/` 对应 skills。

## 验证 checklist

每个 migration commit 跑:

```bash
cargo check -p ralph-e2e --quiet                              # 编过
cargo test -p ralph-e2e --lib --quiet                         # 全 lib 测试
cargo run -p ralph-e2e --quiet -- --list | grep <scenario-id>  # YAML 反序列化
cargo test -p ralph-e2e --test declarative_coverage_gate -- --nocapture  # gate drift log
```

预期:
- `cargo check` 0 error
- `cargo test --lib` 526 → 536+ passed (无 regression)
- `--list` 显示 `<id>  <description> (declarative)`
- gate test `drift log` 显示 declarative count +1, imperative count -1, coverage +1.67%

## 已知 schema 缺口 (留待后续扩展)

- `file_content`: schema 无字段, file existence 由 `artifacts` 覆盖, file content 多 substring 检查 dropped。
- `output_absent` / `stderr_absent`: NEGATED "NOT contains" 检查 dropped。
- `duration_within_secs` (vs `<=`, 区别于 `failed_within_secs` 的严格 `<`): 已用 `failed_within_secs` 近似覆盖 (off-by-one 边界)。
- `require_backend: <wrong>` for backend-unavailable (命令式本身的语义问题, 详见 `LATER_PLANS.md`)。

## 后续增量迁移 (Wave 3+)

如果未来需要继续迁移其他 imperative scenarios (例如并行 runtime live variants), 流程:

1. 读 `LATER_PLANS.md` 看 deferred schema extensions。
2. 必要时先做 schema 扩展 (1 commit per field, +2 unit tests, +commit message 说明 rationale)。
3. 写 YAML + 改 registry (1 commit)。
4. 跑 4 个验证 (见上)。
5. chore commit 记录 decision + drift delta + 决定 + 下一 menu。

## 相关资源

- 详细 skills: `.codex/skills/self-learning.*`
- 项目级经验: `EXPERIENCE.md` (搜索 `exp-20260813-*`)
- 当前规划: `task_plan.md` (最末 entry 通常是 Wave 进度)
- 长期待办: `LATER_PLANS.md`
- OpenSpec change: `openspec/changes/e2e-declarative-migration-plan/`
