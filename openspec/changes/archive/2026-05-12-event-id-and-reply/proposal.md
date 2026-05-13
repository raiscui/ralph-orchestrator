## Why

并行模式下,hat 之间依赖 `<event ...>` 协作.
但目前协作链路缺少一个关键的"可引用锚点": incoming events 在 prompt 里没有显式的 event id.
这会导致:

- 其他 hat 无法在回复时准确指向"我在回复哪一条事件".
- ralph 也很难在并发场景里可靠地做请求-响应关联(尤其当同一 topic 同时出现多条事件时).

因此我们需要把 event 的"可引用 id"与"回复关系(reply)"做成协议一等字段,并让 hat 在 prompt 中可见.

## What Changes

- 保证每条被发布/路由的 event 都有可引用 id:
  - 复用既有字段 `Event.id`.
  - 当 agent 未显式提供 id 时,由运行时自动补齐.
- event 增加 `reply`(单值)语义,用于表达"我在回复哪个 event id":
  - `<event ... reply=\"<event_id>\">...</event>` 可被解析并落盘.
- prompt 注入改造:
  - 并行模式下的 incoming events 必须显式展示 `id`,让其他 hat 可以把它原样带入 `reply`.
- 文档与示例同步:
  - 增加 "reply 链路" 的最小示例与约定,避免各 hat 自创格式导致不可互操作.
- 测试:
  - 覆盖 event 解析(reply 属性)与 id 自动补齐.
  - 覆盖并行模式 incoming events 的 prompt 展示包含 id.

## Capabilities

### New Capabilities

（无）

### Modified Capabilities

- `parallel-hat-instances`: 增加事件可引用 id 与 reply 链路的协议要求(让并发协作可关联,可回放,可诊断).

## Impact

- 受影响代码区域:
  - `crates/ralph-proto`: `Event` 协议字段扩展.
  - `crates/ralph-core`: `<event ...>` 解析器扩展,以及并行模式 prompt 注入的 incoming events 展示.
  - `crates/ralph-core` / `crates/ralph-cli`: 并行路由与日志/回放相关测试可能需要更新.
- 受影响行为:
  - 并行模式下 hat 的输入 prompt 将包含 event id,这会改变模型的上下文信息结构(预期是正向影响).
  - `.ralph/events*.jsonl` 将开始出现 reply 关联信息(如果事件输出携带).
