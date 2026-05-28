---
name: self-learning.ralph-agent-cli-recoverable-failure-retry
description: |
  修复/排查 Ralph agent CLI 可恢复失败重试链路。适用于 Codex/agent CLI 返回 429、`exceeded retry limit`、recoverable failure ledger、manual continue、agents snapshot、record summary 或 Evidence Inspect 三类状态(scheduled/continued/exhausted)异常时。
  核心经验: 以 `.ralph/recoverable-failures.jsonl` 为可恢复失败生命周期真相源,用窄分类器和有界 retry policy,不要把 stderr 当 workflow event 解析,也不要沿用旧的 no-delta OpenSpec 阻断说法。
author: Ralph contributors
version: 1.0.0
date: 2026-05-28
---

# Ralph agent CLI recoverable failure retry

## 适用场景

当你看到以下任一现象时,先使用这个 skill:

- agent CLI 失败里出现 `429 Too Many Requests`。
- stderr 包含 `ERROR: exceeded retry limit, last status: 429 Too Many Requests`。
- `.ralph/recoverable-failures.jsonl`、`ralph agents`、`ralph record summary` 或 Evidence Inspect 的 recoverable failure 状态不一致。
- 需要验证 `retry_scheduled`、`continued_by_human`、`exhausted`、`recovered` 这些生命周期状态。
- 继续或归档 `agent-cli-recoverable-failure-retry` 相关 OpenSpec / spec / docs 时,需要避免使用过期的 `no-delta change` 阻断口径。

## 核心原则

1. **ledger 是生命周期真相源**
   - `.ralph/recoverable-failures.jsonl` 记录 append-only transition。
   - 通过 replay ledger 得到当前 recoverable failure snapshot。
   - 不要在 ledger 里复制完整 prompt 或完整 event stream。

2. **分类必须窄且确定**
   - `429 Too Many Requests` 可以被归类为可恢复。
   - `exceeded retry limit` 只有在 last status 指向临时类错误(例如 429)时才可恢复。
   - 不要用 LLM 判断 retryability。
   - 不要把所有 non-zero exit 都升级成 recoverable。

3. **stderr 只可观测,不参与 workflow event 解析**
   - stderr 可以进 bounded excerpt,用于诊断。
   - stderr 不能进入 `<event ...>` parser。
   - 这条边界和 `self-learning.ralph-parallel-event-parser-stdout-only` 一致。

4. **manual continue 必须复用同一 retry path**
   - human continue 要先写 `continued_by_human` transition。
   - retry 必须复用 runtime-held job context。
   - 不要手工重构 job prompt 或事件流。

5. **Agents Snapshot 是摘要面,不是第二份 ledger**
   - `AgentInstanceSnapshot.recoverable_failures` 只保留 summary metadata。
   - 完整 `failure_id`、`next_retry_at`、ledger path 等证据应能从 JSON / ledger 指回。
   - 紧凑表格断言不要过强,避免把展示列宽当协议。

## 推荐排查顺序

### 1. 先看 ledger

```bash
jq '.' .ralph/recoverable-failures.jsonl
```

确认:

- `failure_id` 是否稳定。
- `attempt` / `max_attempts` 是否符合 policy。
- 是否出现 `retry_scheduled`、`continued_by_human`、`recovered` 或 `exhausted`。
- 每条 transition 是否包含 job / instance / hat / backend 相关 correlation id。

### 2. 再看 agents snapshot

```bash
ralph agents
ralph agents --format json
```

确认:

- 紧凑表格只需要展示 recoverable 状态和 attempt 摘要。
- JSON 输出必须保留可回指 ledger 的关键 metadata。
- 测试断言应检查语义字段,不要依赖完整表格文本。

### 3. 再看 record summary / Evidence Inspect

```bash
ralph record summary <session.jsonl> --agents-file .ralph/agents.json
```

确认 Evidence Inspect 能区分:

- scheduled: 已计划自动 retry。
- continued: human 显式 continue 触发同一路径 retry。
- exhausted: 达到上限后变成 terminal evidence,并指向 ledger。

### 4. 最后看 OpenSpec 状态

`agent-cli-recoverable-failure-retry` 已归档后,不要再沿用“no-delta change 阻断 validate --all”的旧判断。

如果 `openspec validate --all --strict` 失败,按当前输出重新定位具体失败 spec/change。

## 验证锚点

优先跑 focused gates,再跑 package/workspace gates:

```bash
cargo test -p ralph-core --lib recoverable --quiet
cargo test -p ralph-core --lib parallel::instance::tests::recoverable_failure_schedules_retry_and_preserves_stdout_only_parsing -- --exact --nocapture
cargo test -p ralph-core --lib parallel::instance::tests::manual_continue_appends_transition_and_uses_scheduled_retry_path -- --exact --nocapture
cargo test -p ralph-core --lib parallel::instance::tests::recoverable_failure_exhaustion_becomes_terminal_with_ledger_pointer -- --exact --nocapture
cargo test -p ralph-cli --test integration_agents test_agents_command_prints_recoverable_summary -- --exact --nocapture
cargo test -p ralph-cli --bin ralph record_session::tests::aggregate_collects_evidence_inspect -- --exact --nocapture
cargo test -p ralph-core smoke_runner --quiet
cargo test --quiet
OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate --all --strict
```

## 常见误判

- 不要把 `ralph agents` 表格裁剪当成 ledger 缺字段。
- 不要把 `exceeded retry limit` 本身当成所有场景都可恢复;必须结合 temporary status。
- 不要把 retry 后成功的 lifecycle 留成 pending;成功后应写 `recovered`。
- 不要把 exhausted 伪装成成功;它是 terminal failure,但必须保留 ledger evidence pointer。
- 不要把 archive 后的 `Purpose TBD` 或其它 spec 校验问题误记成旧 `no-delta` 阻断。
