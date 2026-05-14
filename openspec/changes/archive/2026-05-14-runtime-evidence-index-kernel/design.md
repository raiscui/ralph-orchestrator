## Context

当前 Ralph evidence 已经分散在多条稳定或半稳定的 artifact 流里:

- record-session JSONL: `_meta.session_start`、`ux.terminal.write`、`bus.publish`、`_meta.termination` 等。
- `.ralph/events.jsonl`: `EventLogger` 持久化业务事件、runtime delivery、runtime lifecycle。
- capability invocation artifact: `.ralph/capability-invocations/<invocation_id>/invoke.json`、`result.json`、`failed.json`、`resolved-config.yml`。
- request / reply correlation: event id、reply id、source instance、reply topic。
- runtime graph durable replay: 已明确 replay graph 必须依赖 durable delivery / lifecycle 证据,不能把 live graph 当真相源。

这些证据现在各自可用,但缺少一个最小索引层来回答:

- 这次 run 的相关 artifact 在哪里?
- 某个 request / reply / invocation / delivery 的 correlation id 能关联到哪些 artifact?
- 某个 artifact 是成功、失败,还是缺失?
- child run / micro-run artifact 如何挂到 parent run 下?

Phase 1A 的目标不是给人看的 evidence 产品,而是给 Phase 2 / Phase 3 和测试使用的最小 kernel contract。

## Goals

- 定义最小 evidence index schema。
- 定义 artifact registration contract。
- 定义 correlation lookup contract。
- 定义 missing artifact marker,让失败证据可审计。
- 定义 parent-child link,支持 capability isolated child run / micro-run。
- 固定 Phase 1A 与 Phase 1B 的边界。

## Non-Goals

- 不实现 `ralph evidence summary`。
- 不实现 `ralph evidence inspect`。
- 不实现 `ralph doctor evidence`。
- 不设计完整诊断分类 taxonomy。
- 不把 Rerun graph、TUI graph、live observer snapshot 当作 durable evidence 真相源。
- 不改变 request/reply routing 或 capability invocation execution 语义。
- 不要求所有历史 fixture 立刻迁移到 index。

## Data model

最小 index entry 建议包含以下字段。实现阶段可以选择 JSONL 或 structured JSON,但 contract 不应依赖 CLI 展示模型。

```text
EvidenceIndexEntry
- schema_version
- session_id
- run_id
- correlation_id
- artifact_kind
- artifact_path
- producer
- status
- parent_correlation_id
- child_correlation_id
- created_at
```

字段边界:

- `schema_version`: 允许后续演进,但 Phase 1A 只承诺 v1。
- `session_id` / `run_id`: 定位 run 级 scope。
- `correlation_id`: request id、reply id、invocation id、event id、delivery id 中的稳定关联键。
- `artifact_kind`: enum-like 字符串,只覆盖 Phase 1A 必需集合。
- `artifact_path`: repo/workspace 相对路径优先,用于可移植 fixture。
- `producer`: 记录写入者,例如 `record-session`、`event-logger`、`capability-invocation`。
- `status`: `success`、`failure`、`missing`、`unknown` 中的最小集合。
- `parent_correlation_id` / `child_correlation_id`: 表达 parent-child link,不表达复杂 graph。

## Minimal artifact kinds

Phase 1A 只建议固定以下 artifact kinds:

- `record_session_jsonl`
- `event_log_jsonl`
- `runtime_delivery_record`
- `runtime_lifecycle_record`
- `reply_event`
- `capability_invoke_json`
- `capability_result_json`
- `capability_failed_json`
- `resolved_config`
- `missing_artifact`

如果实现阶段需要新增 kind,必须先证明它服务于 Phase 2 / Phase 3 的最小闭环,而不是 Phase 1B 展示 UX。

## Write contract

写入方只负责登记 artifact link,不负责解释整条运行链路。

- record-session 写入或 flush 后,可以登记 JSONL artifact link。
- event logger 写入 runtime delivery / lifecycle durable record 后,可以登记对应 event log artifact link 和 correlation id。
- reply / answer return 后续实现可以登记 reply event artifact link。
- capability invocation 写入 `invoke.json`、`result.json`、`failed.json`、`resolved-config.yml` 后,可以登记 invocation 相关 artifact link。
- 如果预期 artifact 不存在,必须能登记 `missing_artifact` marker,而不是静默忽略。

## Read contract

读取方最小能力只有一个: 按 correlation id 找到相关 artifact entries。

读取结果必须能区分:

- 找到了成功 artifact。
- 找到了 failure artifact。
- 找到了 missing marker。
- 没有任何 index entry。

Phase 1A 不要求提供人类友好的 summary 文案。

## Relationship to existing specs

- `record-session-contract-and-watch`: 保持 JSONL 自描述和 flush/termination contract; index 只是登记它的位置。
- `adapter-contract-tests`: 保持 stdout/stderr、event envelope 和 stream attribution contract; index 可引用这些 artifact。
- `runtime-graph-observability`: 保持 durable replay graph 必须依赖 durable records 的边界; index 不把 graph layout 当真相源。
- `capability-invocation`: 保持 isolated child run / micro-run 和 invocation artifacts; index 只登记 artifacts 与 parent-child link。
- `hat-request-reply-channel`: 保持 reply routing contract; index 只登记 reply correlation artifact。

## Implementation guidance for later phase

后续实现时优先考虑一个小的 core module,例如 `ralph-core::evidence_index`,而不是把 index 逻辑塞进 CLI 展示层。

推荐接口形态:

```text
EvidenceIndexWriter::record(entry)
EvidenceIndexReader::find_by_correlation(correlation_id)
EvidenceIndexEntry::missing(...)
```

这只是 implementation guidance,不是本轮必须实现的代码。

## Risks

### Risk: schema 过大

如果为了未来 CLI 展示加入 detailed diagnosis taxonomy,Phase 1A 会变成 Phase 1B。缓解方式: schema 只保留 artifact link、correlation、status、parent-child link。

### Risk: live graph 被误当 durable source

runtime graph 可以展示 evidence,但 durable replay 和 index 必须依赖 JSONL / artifact 文件。缓解方式: spec 明确禁止把 graph layout 当真相源。

### Risk: index 成为新的 orchestration platform

Evidence index 只回答 artifact lookup,不负责调度、不负责 routing、不负责 repair。缓解方式: write/read contract 保持单一。

## Test strategy summary

详细测试计划见 `test-plan.md`。核心门禁是 contract tests,不依赖完整 CLI / doctor UX。
