# 并行财务关账控制台

这是一个"财务结账前多控制面并行收敛"的可运行范例。
它更接近真实财务关账控制台的收口场景。

这个范例展示的是:

- `ralph#1` 先把关账资料包扇出到 4 条处理线
- 收入、费用、现金、异常核对同时推进
- `close_conductor` 汇总统一关账结论
- `ralph#1` 最后输出关账摘要并结束
- 每个处理线角色都必须只输出真实事件,不能输出 `&lt;event ...&gt;` 这种展示文本

## 适合用来演示什么

- 财务关账场景下的并行协作
- 多输入线收敛为单一关账结论
- fanout -> fanin -> close conductor 的收口流程

## 目录内容

- `ralph.yml`: 并行拓扑和协调协议
- `PROMPT.md`: 一份可替换的关账资料包

## 运行

在 example 目录中直接运行:

```bash
cd examples/parallel-finance-close-control-room
cargo run --bin ralph -- run --no-tui
```

如果你在仓库根目录运行:

```bash
cargo run --bin ralph -- run \
  -c examples/parallel-finance-close-control-room/ralph.yml \
  -P examples/parallel-finance-close-control-room/PROMPT.md \
  --no-tui
```

## 你应该看到什么

一次正常闭环至少包含这些 topic:

- `close.revenue.reconcile`
- `close.expense.accrual.review`
- `close.cash.position.check`
- `close.anomaly.watch.review`
- `revenue.ready`
- `expense.ready`
- `cash.ready`
- `anomaly.ready`
- `close.packet.request`
- `close.packet.ready`

最后由 `ralph#1` 输出 `LOOP_COMPLETE`。

如果某个处理线只打印了转义的 `&lt;event ...&gt;`,那不算真正发布事件。
这时 `ralph#1` 会继续等待缺失的 ready topic,不会发布 `close.packet.request`。

## 如何替换成你自己的关账场景

只需要改 `PROMPT.md`。
保持这 4 个资料包章节结构不变:

- `## Revenue Packet`
- `## Expense Packet`
- `## Cash Packet`
- `## Anomaly Packet`

这样 `ralph#1` 就能继续稳定扇出给对应处理线。
