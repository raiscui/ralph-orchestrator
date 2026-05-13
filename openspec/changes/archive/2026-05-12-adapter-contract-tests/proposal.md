## Why

Ralph 已经开始把运行证据和可回放 cassette 当成核心质量门禁,但 adapter 边界仍然分散在多个实现里:

- `ralph-adapters` 负责 CLI / PTY 执行与 stdout/stderr 采集。
- `ralph-cli` 的 parallel runner 会把 backend stdout/stderr 写入 TUI、record-session 和事件解析链路。
- `ralph-e2e mock-cli` 会按 cassette 回放 `ux.terminal.write`。
- `ralph-core::EventParser` 负责从文本流里提取 `<event ...>`。

如果这些边界不被 contract tests 固定,后续 startup resource bootstrap、runtime capability invocation、mock replay 和 runtime graph 都会建立在不稳定输入上。

当前最需要固定的 adapter contract 是:

1. stdout 才是默认事件解析输入,stderr 只能作为诊断证据。
2. prompt transport 模式必须明确,尤其是 `stdin` 模式不能再把 prompt 作为尾部 argv 传给 backend。
3. event envelope 必须保留 `id`、`reply`、`source_instance` / `instance_id` 等归因字段。
4. termination 和关键 record-session 证据必须及时落盘,不能只依赖正常结束时统一 flush。

## What Changes

- 新增 adapter contract specs,把 stdout/stderr、prompt transport、event envelope、termination/flush 固定为测试契约。
- 增加 focused tests,覆盖:
  - record-session / replay 默认只用 stdout 参与 event parsing。
  - custom backend `prompt_mode=stdin` 不把 prompt 追加到 argv。
  - `TerminalWrite` / `EventRecord` / runtime delivery 相关证据保留归因字段。
  - `_meta.termination` 和关键 JSONL 记录写出后可 strict parse。
- 如测试暴露现有实现漂移,做必要小修,但不引入新 backend runtime。

## Capabilities

### New Capabilities

- `adapter-contract-tests`: 固定 Ralph backend adapter 与 evidence/replay 层之间的最小互操作契约。

### Modified Capabilities

- None.

## Impact

- 受影响区域:
  - `crates/ralph-core`: event parsing / event logger / record data contract tests。
  - `crates/ralph-cli`: record-session、parallel runner、custom backend prompt mode tests。
  - `crates/ralph-e2e`: mock-cli replay contract tests。
  - `openspec/specs/adapter-contract-tests`: 新增主规格归档目标。
- 不做的事情:
  - 不新增 backend provider。
  - 不实现 startup resource bootstrap。
  - 不实现 runtime capability invocation。
  - 不改变 parallel topology 或 live HatRegistry。
