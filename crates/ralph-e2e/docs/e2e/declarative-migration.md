# ralph-e2e Declarative Scenario Migration

## TL;DR

> **新场景请用 YAML (declarative), 不要再写 Rust `TestScenario` impl.**

Wave 2 (Q3 2026) 把 21 个 imperative scenarios 全部迁移为 declarative YAML, 累计 Coverage 从 65% 推到 100%, gate test `cargo test -p ralph-e2e --test declarative_coverage_gate` PASS。

历史命令式 impl 保留, 已加 `#[deprecated(since = "2.3.0", ...)]`, **1 release cycle 后物理删除**。不要新增 imperative scenarios。

## 添加新 declarative scenario 的步骤

```bash
# 1. 创建 YAML 文件
$EDITOR crates/ralph-e2e/scenarios/<your-scenario>.yaml

# 2. 在 registry 中加入 declarative entry
$EDITOR crates/ralph-e2e/src/lib.rs
#    找到 "Declarative" 块, 加:
#    (
#        ScenarioKind::Declarative,
#        "<your-scenario-id>",
#        Box::new(crate::declarative::from_yaml(
#            "<your-scenario-id>",
#            include_str!("../scenarios/<your-scenario>.yaml"),
#        )),
#    ),

# 3. 跑 4 个验证
cargo check -p ralph-e2e --quiet
cargo test -p ralph-e2e --lib
cargo run -p ralph-e2e -- --list | grep <your-scenario-id>   # 必跑, 验证 YAML 反序列化
cargo test -p ralph-e2e --test declarative_coverage_gate -- --nocapture
```

## YAML schema 速查 (常用字段)

完整 schema 定义: `crates/ralph-e2e/src/declarative/scenario.rs::DeclarativeExpect`。

| 字段 | 用途 | 例 |
|---|---|---|
| `id` | scenario id (与 `scenario.id()` 一致) | `timeout-handling` |
| `description` | 显示在 `--list` | `Verifies timeout termination (declarative)` |
| `tier` | tier 分类 | `Tier 7: Error Handling` |
| `backends` | 显式 backend 列表 (空 = 全 backend) | `[claude, kiro, opencode]` |
| `setup.config` | ralph.yml 内容 (含 `{backend}` 占位符) | `cli:\n  backend: {backend}` |
| `setup.prompt` | 内联 prompt | `You are testing ...` |
| `setup.max_iterations` | max_iterations 覆盖 | `1` |
| `setup.timeout_secs` | 超时秒数 (None = backend.default_timeout) | `5` |
| `setup.extra_args` | 额外 CLI 参数 | `["--no-tui", "--idle-start"]` |
| `setup.write_files` | 写入额外文件 (e.g. fake shim, 预填充) | `[{path: ..., content: ...}]` |
| `setup.inject` | 注入时序 (Wait/Sleep/Assert/Emit) | `[{type: wait, ...}]` |
| `expect.response_received` | agent 有响应 | `true` |
| `expect.exit_code_success_or_limit` | exit code 0 或 2 (limit) | `true` |
| `expect.no_timeout` | 没被 timeout 收掉 | `true` |
| `expect.failed_within_secs` | duration < N | `120` |
| `expect.duration_at_least_secs` | duration >= N | `10` |
| `expect.termination` | termination_reason 严格相等 | `"LOOP_COMPLETE"` |
| `expect.output_contains` | stdout 严格包含 (case-sensitive) | `["LOOP_COMPLETE"]` |
| `expect.output_contains_any` | 任一 needle 命中 (per group) | `[["a", "b"], ["c"]]` |
| `expect.events` | events.jsonl topic count | `[{topic: build.done, min_count: 1}]` |
| `expect.event_payload_contains` | events payload 子串 | `[{topic: ..., contains: ...}]` |
| `expect.artifacts` | 文件存在 | `[".agent/memories.md"]` |
| `expect.failed` | 失败语义 (exit != 0 OR stderr 非空) | `true` |
| `expect.stderr_contains_any` | stderr 任一命中 (per group) | `[["panic", "fatal"]]` |
| `expect.event_absent_prefixes` | 无 topic 以前缀出现 | `["gate."]` |
| `expect.min_total_events` | events 总数下限 | `3` |

## 常见陷阱 (来自 Wave 2 经验)

### 1. `output_contains_any` 是 `Vec<Vec<String>>` — 不能在顶层写多个同名 key

```yaml
# 反例: duplicate field
expect:
  output_contains_any:
    - ["a", "b"]
  output_contains_any:    # ← serde_yaml duplicate field
    - ["c", "d"]

# 正确: 合并到 1 个字段, nested list 区分 group
expect:
  output_contains_any:
    - ["a", "b"]
    - ["c", "d"]           # ← AND 语义 (各 group 都需命中)
```

**写完每个 YAML 必须跑 `cargo run -p ralph-e2e -- --list`**, 不要只信 `cargo test --lib` (单测不 catch YAML 反序列化错误)。

详细 skill: `.codex/skills/self-learning.yaml-duplicate-field-bug/`

### 2. 命令式 OR 断言 (`A || B`) 不能拆 2 个独立 schema 字段

runner 的 `assertions.iter().all(...)` 是 AND 语义, 拆 OR 的两臂到 2 字段会让 OR 退化成 AND (更严格)。两种处理:

```yaml
# (a) 保留 OR: 单字段承载
expect:
  failed: true  # exit_code != 0 || !stderr.is_empty()

# (b) OR 折 AND (更严格, 适合 "指令强制要求全部产物" 的场景):
expect:
  output_contains: ["approved"]
  events:
    - topic: review.done
      min_count: 1
# 命令式 verdict_provided (verdict 文本 || events review.*) 拆 AND,
# 因为 hat instructions 强制要求两者都有
```

详细 skill: `.codex/skills/self-learning.yaml-schema-or-vs-and-semantics/`

### 3. `case_insensitive` stdout 关键词 → 用 N case 变体

`output_contains` / `output_contains_any` 是 case-sensitive。命令式用 `.to_lowercase()` 后 contains 时, 用 N 个 case 变体覆盖 (如 `["Builder", "builder", "Implement", "implement"]`)。

### 4. `events topic starts_with("X.")` → 用 N 条精确 topic

`event_count_at_least` 只支持精确 topic 匹配, 不支持 starts_with。若 scenario 已知只 emit 2 个 topic (如 `build.task` + `build.done`), 用 2 条精确匹配等价于 starts_with; 若未来增加, 需要追加新精确 topic。

### 5. file content / NEGATED checks → 多数 dropped

schema 无 `file_content` / `output_absent` 字段。File existence 用 `artifacts` 覆盖; "NOT contains" 类检查 dropped (schema-cost > value)。详见 repo-level guide。

## 历史命令式 impl (Wave 2 已 deprecated)

21 个 imperative `TestScenario` impl 仍保留在 `crates/ralph-e2e/src/scenarios/`, 已加 `#[deprecated(since = "2.3.0", ...)]`。**不要复用这些 impl**, 直接写 YAML。

| 类别 | scenarios (5 个文件) |
|---|---|
| §2.1 errors | TimeoutScenario / MaxIterationsScenario / BackendUnavailableScenario / AuthFailureScenario |
| §2.2 hats | HatSingleScenario / HatMultiWorkflowScenario / HatInstructionsScenario / HatEventRoutingScenario / HatBackendOverrideScenario |
| §2.3 memory | MemoryAddScenario / MemorySearchScenario / MemoryInjectionScenario / MemoryPersistenceScenario / MemoryCorruptedFileScenario / MemoryMissingFileScenario / MemoryRapidWriteScenario / MemoryLargeContentScenario |
| §2.4 capabilities | ToolUseScenario / StreamingScenario |
| §2.4 parallel | ParallelAppServerIdleStartScenario / ParallelAppServerSteerMultiTurnScenario |

Explicit-keep (NOT deprecated, 永久 imperative): `ParallelExperimentalDevEngineExampleScenario` (per OpenSpec tasks.md §2.5.0)。

## 仓库级深度指南

- `docs/solutions/documentation-gaps/declarative-scenario-migration.md`: Wave 2 完整迁移套路综合指南 (160 行), 含 6 个 schema 字段映射 patterns + dropped 决策 rubric + audit 反预期 workflow。
- `.codex/skills/self-learning.yaml-schema-or-vs-and-semantics/`: OR vs AND 语义详细 skill。
- `.codex/skills/self-learning.yaml-duplicate-field-bug/`: duplicate field 详细 skill。
- `openspec/changes/e2e-declarative-migration-plan/tasks.md`: 完整 Wave 2/3 任务规划 (含 3.1-3.4 closure 状态)。
- `EXPERIENCE.md`: 5 个 `exp-20260813-*` entries 记录 schema 扩展 + OR 语义 + duplicate field + schema-cost 决策 + audit 现实检验。
