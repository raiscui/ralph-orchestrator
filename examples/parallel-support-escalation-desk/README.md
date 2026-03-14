# 并行支持升级处理台

这是一个"高优先级客户升级处理台,多条输入线并行收口"的可运行范例。
它更接近真实的支持升级协同,而不是单人单线的问题跟进。

这个范例展示的是:

- `ralph#1` 先把升级资料包扇出到 4 条处理线
- 案例分诊、产品判断、客户经营、沟通方案 4 条处理线同时推进
- `escalation_director` 汇总统一的升级执行计划
- `ralph#1` 最后输出升级摘要并结束
- 每个处理线角色都必须只输出真实事件,不能输出 `&lt;event ...&gt;` 这种展示文本

## 适合用来演示什么

- 支持升级场景下的并行协作
- 多条输入线收敛为单一升级执行计划
- fanout -> fanin -> escalation director 的真实收口流程

## 目录内容

- `ralph.yml`: 并行拓扑和协调协议
- `PROMPT.md`: 一份可替换的升级资料包

## 运行

在 example 目录中直接运行:

```bash
cd examples/parallel-support-escalation-desk
cargo run --bin ralph -- run --no-tui
```

如果你在仓库根目录运行:

```bash
cargo run --bin ralph -- run \
  -c examples/parallel-support-escalation-desk/ralph.yml \
  -P examples/parallel-support-escalation-desk/PROMPT.md \
  --no-tui
```

## 你应该看到什么

一次正常闭环至少包含这些 topic:

- `support.case.triage.review`
- `support.product.assessment.review`
- `support.account.context.review`
- `support.comms.plan.review`
- `case.ready`
- `product.ready`
- `account.ready`
- `comms.ready`
- `support.escalation.plan.request`
- `escalation.plan.ready`

最后由 `ralph#1` 输出 `LOOP_COMPLETE`。

如果某个处理线只打印了转义的 `&lt;event ...&gt;`,那不算真正发布事件。
这时 `ralph#1` 会继续等待缺失的 ready topic,不会发布 `support.escalation.plan.request`。

## 如何替换成你自己的 escalation 场景

只需要改 `PROMPT.md`。
保持这 4 个资料包章节结构不变:

- `## Case Triage Packet`
- `## Product Assessment Packet`
- `## Account Risk Packet`
- `## Comms Packet`

这样 `ralph#1` 就能继续稳定扇出给对应处理线。
