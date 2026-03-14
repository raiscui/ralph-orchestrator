## Why

并行模式现在已经支持 `Event.id` 与 `Event.reply`,可以表达“这条事件回复了哪条旧事件”。但系统还不能表达“这条回复应该自动回到哪个请求方实例”,于是 explorer / researcher / lookup 这类 hat 只能依赖临时 topic、手写 `target_instance` 或 prompt 约定来把答案送回去,协议很容易漂。

现在需要把“工作流推进事件”和“问答式答案回流”拆成两条明确语义,这样 hat 在发起一次子研究、子探查、子问答时,可以只拿到需要的答案,而不是被迫围绕一堆 workflow topic 自创土协议。

## What Changes

- 新增一个可选的 hat request-reply / answer-return 协议:
  - 请求方 hat 可以发起一个“期待答案回流”的请求事件。
  - 被调用 hat 可以发布与该请求关联的 answer 事件。
  - 运行时负责把 answer 回送给原请求方实例。
- 保持 workflow event 与 answer-return event 的职责分离:
  - workflow event 继续负责推动下游流程。
  - answer-return event 负责把结论回给请求方。
- 约束该协议为显式、可选能力:
  - 不要求所有 hat 默认回传 final answer。
  - 不把现有 `reply` 的“关联语义”偷偷升级成“自动回送语义”。
- 为 prompt、路由、日志和测试增加对应约定,确保 request-reply 链路可诊断、可回放、可验证。

## Capabilities

### New Capabilities
- `hat-request-reply-channel`: 定义 hat 之间的可选 request-reply / answer-return 协议,让被调用 hat 的答案可以回到原请求方实例。

### Modified Capabilities

## Impact

- 受影响代码区域:
  - `crates/ralph-proto`: 事件协议字段与语义说明可能需要扩展。
  - `crates/ralph-core`: 并行模式的 prompt 注入、event 路由、reply 解析、日志与回放。
  - `crates/ralph-cli` / `crates/ralph-e2e`: 真实并行场景、fixture 与端到端验证。
- 受影响行为:
  - hat 之间将拥有一条显式的“答案回流”通道。
  - workflow topic 不再承担“把答案返回给发起方”的隐式职责。
- 风险:
  - 如果协议定义不清楚,容易与现有 `reply`、`target_instance`、`reply.human.message` 混淆。
  - 如果默认化为“所有 hat 都回传 final answer”,会制造噪音与循环风险,因此本 change 会明确禁止该默认行为。
