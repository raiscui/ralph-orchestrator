## Why

Ralph 已经有多条 evidence 流: record-session JSONL、`.ralph/events.jsonl`、runtime delivery / lifecycle durable records、capability invocation artifacts、request / reply correlation。问题是这些证据现在只能按各自文件读取,后续 request/reply 和 capability v2 很难用同一个最小 contract 找到相关 artifact。

Phase 1A 需要先建立一个克制的 evidence index kernel。它只登记 artifact link 和 correlation 关系,不提前实现 evidence CLI / doctor UX,避免把诊断展示层做成新的平台。

## What Changes

- 新增 `runtime-evidence-index-kernel` 能力,定义最小 evidence index schema 和读写 contract。
- 规定 index 只覆盖后续 Phase 2 / Phase 3 必需的 artifact 连接能力:
  - session id / run id
  - artifact kind
  - artifact path
  - producer
  - correlation id
  - success / failure marker
  - parent-child link
- 规定 record-session、runtime delivery、reply、capability invocation 这些已有 evidence 流可以登记 artifact link。
- 规定测试和后续 runtime code 可以按 correlation id 找回相关 artifact。
- 明确本阶段不实现完整 `ralph evidence summary`、`ralph evidence inspect`、`ralph doctor evidence` UX。
- 明确本阶段不把 live runtime graph 或 Rerun graph 当作 durable evidence 真相源。

## Capabilities

### New Capabilities

- `runtime-evidence-index-kernel`: 定义 Ralph 最小 evidence index kernel,用于把 session、runtime delivery、reply 和 capability invocation artifacts 通过 stable correlation 连接起来。

### Modified Capabilities

- None.

## Impact

- 受影响规格区域:
  - `openspec/specs/record-session-contract-and-watch/spec.md`: 作为可索引 artifact source,但不改变其既有 requirement。
  - `openspec/specs/adapter-contract-tests/spec.md`: 作为 event / stream attribution 证据来源,但不改变其既有 requirement。
  - `openspec/specs/runtime-graph-observability/spec.md`: 继续保持 runtime graph 与 durable evidence 的边界,不把 graph 展示层作为 index 真相源。
  - `openspec/specs/capability-invocation/spec.md`: capability invocation artifacts 将成为可索引 artifact source,但本 change 不改变 capability execution 语义。
  - `openspec/specs/hat-request-reply-channel/spec.md`: reply correlation 可被 index 记录,但本 change 不改变 reply routing 语义。
- 预计后续实现代码触点:
  - `crates/ralph-core/src/event_logger.rs`
  - `crates/ralph-core/src/capability.rs`
  - `crates/ralph-cli/src/capability.rs`
  - record-session writer / summary / fixture tests
- 不做事项:
  - 不新增 CLI evidence UX。
  - 不新增 doctor evidence 分类。
  - 不改变 live topology。
  - 不实现 Phase 1B 的展示/诊断模型。
