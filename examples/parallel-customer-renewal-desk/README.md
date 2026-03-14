# 并行客户续约处理台

这是一个"高价值客户续约前多经营面并行收敛"的可运行范例。
它更接近真实续约推进台的保续约场景。

这个范例展示的是:

- `ralph#1` 先把续约资料包扇出到 4 条处理线
- 采用情况、支持健康度、商业条件、赞助人关系同时推进
- `renewal_strategist` 汇总统一续约动作计划
- `ralph#1` 最后输出续约摘要并结束
- 每个处理线角色都必须只输出真实事件,不能输出 `&lt;event ...&gt;` 这种展示文本

## 适合用来演示什么

- 客户续约保卫战中的并行协作
- 多输入线收敛为单一续约动作计划
- fanout -> fanin -> renewal strategist 的收口流程

## 目录内容

- `ralph.yml`: 并行拓扑和协调协议
- `PROMPT.md`: 一份可替换的续约资料包

## 运行

在 example 目录中直接运行:

```bash
cd examples/parallel-customer-renewal-desk
cargo run --bin ralph -- run --no-tui
```

如果你在仓库根目录运行:

```bash
cargo run --bin ralph -- run \
  -c examples/parallel-customer-renewal-desk/ralph.yml \
  -P examples/parallel-customer-renewal-desk/PROMPT.md \
  --no-tui
```

## 你应该看到什么

一次正常闭环至少包含这些 topic:

- `renewal.adoption.review`
- `renewal.support.health`
- `renewal.commercial.review`
- `renewal.sponsor.map`
- `adoption.ready`
- `support.ready`
- `commercial.ready`
- `sponsor.ready`
- `renewal.plan.request`
- `renewal.plan.ready`

最后由 `ralph#1` 输出 `LOOP_COMPLETE`。

如果某个处理线只打印了转义的 `&lt;event ...&gt;`,那不算真正发布事件。
这时 `ralph#1` 会继续等待缺失的 ready topic,不会发布 `renewal.plan.request`。

## 如何替换成你自己的续约场景

只需要改 `PROMPT.md`。
保持这 4 个资料包章节结构不变:

- `## Adoption Packet`
- `## Support Packet`
- `## Commercial Packet`
- `## Sponsor Packet`

这样 `ralph#1` 就能继续稳定扇出给对应处理线。
