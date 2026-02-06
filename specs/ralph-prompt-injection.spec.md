---
status: draft
related:
  - VERIFICATION_PROMPT_PRECEDENCE.md
  - specs/parallel-hat-instances.spec.md
---

# `event_loop.ralph_prompt` 配置项规范

## 背景

目前 Ralph 的“顶层 prompt”来源遵循固定优先级（`-p/-P` > `event_loop.prompt` > `event_loop.prompt_file` > 默认 `PROMPT.md`）。
这段 prompt 的语义本质上是“用户 objective/任务描述”，最终会作为 `task.start/task.resume` 的 payload 进入事件上下文。

但在实践中，常常还需要一段“只给协调者 Ralph 看的额外语义锚点/行为约束”：

- 并行模式下尤其重要（避免 prompt pollution：不希望其它 hats 也收到这段文本）
- 需要和 objective 解耦（不能依赖是否存在 `PROMPT.md`，也不能被 `event_loop.prompt` 覆盖掉）

因此需要一个专门的配置项：`event_loop.ralph_prompt`。

## 目标

1. `event_loop.ralph_prompt` **必须（MUST）** 始终注入到 Ralph（协调者）的 prompt 中。
2. 该注入行为 **不受** `PROMPT.md` 是否存在、以及是否配置了 `event_loop.prompt` 的影响。
3. `event_loop.ralph_prompt` **必须（MUST）** 只影响 Ralph，不应注入到非 Ralph hats 的 prompt（保持并行模式的 prompt pollution 防线）。

## 非目标

- 不新增 CLI flags（例如 `--ralph-prompt`）。
- 不改变现有 prompt precedence 规则。
- 不把 `event_loop.ralph_prompt` 写进事件 payload（避免污染事件流与回放数据）。

## 配置 Schema

新增字段（可选）：

```yaml
event_loop:
  # 只注入给 ralph（协调者）的额外 prompt（可多行）
  ralph_prompt: |
    这里写“只给 Ralph 看的固定约束/语义锚点”
```

- 类型：`Option<String>`（未配置或为空白则视为禁用）

## 行为语义

### 非并行（EventLoop / HatlessRalph）

- Ralph prompt 的组装逻辑 **必须（MUST）** 在固定位置包含 `event_loop.ralph_prompt` 内容（若非空）。
- 该内容只进入 Ralph 的 prompt，不进入其它 hat 的 prompt。

### 并行（ParallelSupervisor / ralph#1）

- ralph#1（并行协调者）的 coordinator instructions **必须（MUST）** 包含 `event_loop.ralph_prompt`（若非空）。
- 其它 hat 的 prompt **不得（MUST NOT）** 因 `event_loop.ralph_prompt` 发生变化。

### 空白处理

- 如果 `event_loop.ralph_prompt` 仅包含空白字符（`trim().is_empty()`），则视为未配置，不注入任何额外段落（避免产生空标题）。

## 验收标准（Acceptance Criteria）

1. `serde_yaml` 能正确解析 `event_loop.ralph_prompt`。
2. 单元/集成测试覆盖：
   - 非并行：`EventLoop::build_ralph_prompt()` 生成的 prompt 包含 `ralph_prompt`。
   - 并行：ralph#1 的 prompt（或 coordinator instructions）包含 `ralph_prompt`。
3. `cargo fmt --check`、`cargo clippy --all-targets --all-features -- -D warnings`、`cargo test` 全部通过。
