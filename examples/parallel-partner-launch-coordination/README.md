# 并行合作伙伴发布协调

这是一个"合作伙伴联合发布,多条输入线并行收口"的可运行范例。
它更接近真实的渠道伙伴联合发布协调场景。

这个范例展示的是:

- `ralph#1` 先把合作伙伴发布资料包扇出到 4 条处理线
- 方案使能、法务、市场、销售交接 4 条处理线同时推进
- `partner_launch_manager` 汇总统一的发布资料包
- `ralph#1` 只有在最终 ready 后才输出摘要并结束
- worker 和 finalizer 都必须只输出真实事件,不能混入解释文本

## 适合用来演示什么

- 合作伙伴联合发布前的跨团队并行协作
- 多条输入线收敛为单一发布资料包
- fanout -> fanin -> launch manager 的真实收口流程

## 目录内容

- `ralph.yml`: 并行拓扑和协调协议
- `PROMPT.md`: 一份可替换的合作伙伴发布资料包

## 运行

在 example 目录中直接运行:

```bash
cd examples/parallel-partner-launch-coordination
cargo run --bin ralph -- run --no-tui
```

如果你在仓库根目录运行:

```bash
cargo run --bin ralph -- run \
  -c examples/parallel-partner-launch-coordination/ralph.yml \
  -P examples/parallel-partner-launch-coordination/PROMPT.md \
  --no-tui
```

## 你应该看到什么

一次正常闭环至少包含这些 topic:

- `partner.solution.enablement.review`
- `partner.legal.terms.review`
- `partner.channel.marketing.review`
- `partner.sales.handoff.review`
- `solution.ready`
- `legal.ready`
- `marketing.ready`
- `sales.ready`
- `partner.launch.packet.request`
- `partner.launch.ready`

最后由 `ralph#1` 输出 `LOOP_COMPLETE`。

如果某个处理线只打印了转义的 `&lt;event ...&gt;`,那不算真正发布事件。
这时 `ralph#1` 会继续静默等待缺失的 ready topic,不会提前发布 `partner.launch.packet.request`。

## 如何替换成你自己的合作伙伴发布场景

只需要改 `PROMPT.md`。
保持这 4 个资料包章节结构不变:

- `## Solution Enablement Packet`
- `## Legal Terms Packet`
- `## Channel Marketing Packet`
- `## Sales Handoff Packet`

这样 `ralph#1` 就能继续稳定扇出给对应处理线。
