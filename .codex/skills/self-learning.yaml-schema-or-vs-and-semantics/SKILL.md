---
name: self-learning.yaml-schema-or-vs-and-semantics
description: |
  修复/规避 YAML schema 字段设计时把 "OR 语义" 命令式断言拆成多个独立字段导致的语义失真。
  适用场景：(1) 命令式断言是 `A || B`(任一通过即通过), 你打算映射到 2 个独立 schema 字段(让 runner 各产生一条 assertion); (2) runner 的 `assertions.iter().all(...)` 是 AND 语义; (3) 你担心 1:1 schema 字段会让 OR 退化成 AND,导致测试比命令式更严格。
  方案: 设计单一字段保留 OR 语义(不拆字段); 若必须拆, 显式记录 "OR 折 AND = 更严格" 的影响并在 YAML 注释里说明。
author: Codex CLI
version: 1.0.0
date: 2026-08-13
---

# YAML schema 设计: OR vs AND 语义保留

## 问题
命令式断言常用 OR 表达 "任一通过即通过":
```rust
let ok = has_marker || (has_plan && has_build_done);
let assertions = vec![Assertions::A, self.B(condition)];  // runner 全部 AND
```

如果你把命令式 OR 拆成 2 个独立 schema 字段:
```yaml
field_a: true   # 产生 1 条 assertion
field_b: true   # 产生 1 条 assertion
# runner: assertions.iter().all(|a| a.passed) → AND 全部通过
```

结果是: 命令式 OR(任一通过即通过) 退化为 declarative AND(全部通过才通过), 测试比命令式严格。

## 触发条件
满足任一条, 应该用这个 skill:

1. 你在做 schema 字段迁移(命令式 → YAML/JSON/etc.), 命令式断言用 `||` 表达 OR。
2. 你打算把 OR 的两臂拆成多个独立 schema 字段, 每个字段产生 1 条 assertion。
3. 你担心 1:1 字段映射会让测试失真。

## 解决方案

### 1) 优先保留 OR 语义的字段设计
对于 "OR across N predicates" 的命令式断言, 设计单一字段承载所有 OR 臂。例如 `Vec<T>` 接受 N 个 needle, 任一命中即通过; 或设计一个 combined flag 字段, 仅当命令式 "任一臂为 true" 时为 true。

例:
```rust
// 命令式
let has_evidence = stderr.contains("not found") 
    || stderr.contains("command not found");
```
映射成:
```yaml
expect:
  stderr_contains_any:
    - ["not found", "command not found"]
```
单字段, OR 语义保留。

### 2) 必要时 "OR 折 AND" 是更严格方向
如果命令式 OR 是 defensive 路径 (实际几乎总是某臂为 true), OR 折 AND 实际更接近 "指令遵循" 的真实期望, 比命令式更严格。这是 [ponytail strict-check] 的合理应用。

例: `BackendUnavailableScenario::execution_failed` (exit_code != 0 OR stderr 非空) → `failed: true` (单字段保留 OR); 但 `HatInstructionsScenario::verdict_provided` (verdict 文本 OR events review.*) → `output_contains_any` + `events` (拆 2 字段 AND), 因为 hat instructions 强制要求两者都有, AND 正确。

### 3) 在 YAML 注释里显式记录
无论保留 OR 还是 OR 折 AND, 都在 YAML 顶部用注释说明:
```yaml
# 命令式 execution_failed (exit_code != 0 || !stderr.is_empty())
#   → expect.failed: true    # 保留 OR 语义
# 命令式 verdict_provided (verdict 文本 || events review.*)
#   → output_contains_any + events (OR 折 AND, hat instructions 强制两者)
```

后续 reviewer / 维护者读 YAML 注释能立刻看到 "为什么是这条 schema 路径"。

## 验证
- 跑 `cargo test -p ralph-e2e --lib` 全过(无回归)。
- 跑 schema 字段对应的 YAML 反序列化, 确认字段结构正确。
- 若有 live scenario, 跑该 scenario 确认 OR 语义在 live run 下保持。

## 反例
不要做这种 OR 拆字段设计:
```yaml
# 反例: 拆 OR 的两臂到 2 个独立 boolean 字段
expect:
  has_evidence_a: true   # 第一臂: stderr 含 "not found"
  has_evidence_b: true   # 第二臂: stderr 含 "command not found"
```
这是反例因为:
- runner 全部 AND, 比命令式 OR 严格
- 单字段 `evidence: any` 是更清晰的设计
- 用户读 YAML 难以理解 "为什么是 2 个 boolean"
