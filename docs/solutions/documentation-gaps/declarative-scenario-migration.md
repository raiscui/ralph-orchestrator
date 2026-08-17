---
title: E2E Declarative Scenario Migration Guide
date: 2026-08-13
last_updated: 2026-08-17
module: crates/ralph-e2e/src/scenarios/
component: declarative migration
problem_type: documentation_gap
severity: medium
status: active
tags:
  - declarative-yaml
  - schema-design
  - rust-e2e
  - migration-guide
  - wave2-q3-2026
verified_by:
  - "cargo test -p ralph-e2e --lib (325 passed after Wave 3.4 cleanup; 396 before, deletion removed 71 unit tests)"
  - "cargo test -p ralph-e2e --test declarative_coverage_gate (Coverage 100.00% PASS)"
  - "cargo run -p ralph-e2e --quiet -- --list (61 scenarios, 60 Declarative + 1 ImperativeExplicitKeep)"
related_solutions:
  - ../EXPERIENCE.md (4 exp-20260813 entries)
  - ../minimax-full-auto-compat/README.md (Q3 plan 3.6 cli_backend.rs fix + minimax live E2E)
related_skills:
  - self-learning.yaml-schema-or-vs-and-semantics
  - self-learning.yaml-duplicate-field-bug
---

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



## Wave 3.4 收尾 (2026-08-17) — post-migration dead code 物理删除

**关键判断 (供未来同类任务复用)**:
- 当 `all_scenarios()` 完全迁移到 declarative 路径之后,任何**未被注册**的
  Rust `TestScenario` impl 都是 dead code。
- 即使它们仍 `pub use` 暴露在外(`lib.rs` re-export 还在),也只是
  公开 API surface 上的死皮。
- 物理删除它们的判定信号:`#[deprecated(since = "2.3.0", ...)]` 是 Wave 2
  时打的"下个 release 删"标记,Wave 3.4 + Round 6 直接提前清零。

### Wave 3.4 cleanup (commit `ca54fb3b`): 4 个 parallel code-defined scenarios
物理删除 `parallel/{emit_spawn_instance, hat_instances, starting_event_inference}.rs`
+ `parallel_trigger_routing_example.rs` 共 ~2470 行。这些是 Wave 2 期间以
code-defined 形式存在的最后一批 imperative parallel 场景,YAML 替代版
已经稳定运行。

### Round 6 (commit `ee73fcf8` + `03fab390` + `e1edf762`): 22 + 12 = 34 个 legacy struct

- `ee73fcf8`: 删除 `capabilities.rs` (ToolUseScenario + StreamingScenario, 698 行)。
- `03fab390`: 删除 5 个 deprecated `.rs` 文件 (`errors.rs` / `hats.rs` /
  `memory.rs` 以及 parallel/ 子目录下两个 app_server 场景代码文件 (3 个 Round 6 物理删除, 此处为历史引用),共 19 个 `#[deprecated]` struct
  + 7057 行。
- `e1edf762`: 删除 5 个非 deprecated 但同样的 dead code `.rs` 文件
  (`connectivity.rs` / `events.rs` / `orchestration.rs` / `incremental.rs` /
  `tasks.rs`),共 12 个 struct + 3865 行。

### 关键 pattern (复用机会)

**当 `all_scenarios()` 是注册真相源,`pub use` 只是 re-export 时,删除流程是机械的 4 步**:

1. `grep -rn '\b<StructName>\b' crates/ralph-e2e/src/ --include='*.rs' | rg -v 'lib.rs\|scenarios/mod.rs\|<StructName 定义所在文件>'`
   - 期望: 0 命中(除了自身文件 + lib.rs/scena rios/mod.rs 的 `pub use`)
   - 若非零: 先看是不是 doctest 引用 (`runner.rs`, `crates/ralph-e2e/src/scenarios/mod.rs` 顶部),删之前修改 doctest
2. `scenarios/<file>.rs`: `mod` 声明 + `#[allow(deprecated)] pub use <file>::{...};` 删除
3. `lib.rs`: 类型 re-export + 空 Tier 注释删除
4. `rm scenarios/<file>.rs` 后跑:
   - `cargo build -p ralph-e2e`(应该 0 error,deprecation warning 减 N)
   - `cargo test -p ralph-e2e --test declarative_coverage_gate`(gate 仍 100% PASS)
   - `cargo test -p ralph-e2e --lib`(regression)
   - `cargo test -p ralph-e2e --doc`(doctest 编译通过)
   - `cargo run -p ralph-e2e --quiet -- --list`(61 scenarios 不变)

### Drift log 净效应

```text
Round 6 起点:  60 Declarative + 0 Imperative + 1 ExplicitKeep = 61
Round 6 终点:  60 Declarative + 0 Imperative + 1 ExplicitKeep = 61
```

注册表**没变**(删除的都是未注册的 dead struct),但场景实现层
清理掉了 ~11K 行业务/技术债。

## 相关资源

- 详细 skills: `.codex/skills/self-learning.*`
- 项目级经验: `EXPERIENCE.md` (搜索 `exp-20260813-*`)
- 当前规划: `task_plan.md` (最末 entry 通常是 Wave 进度)
- 长期待办: `LATER_PLANS.md`
- OpenSpec change: `openspec/changes/archive/2026-08-13-e2e-declarative-migration-plan/` (已归档)
