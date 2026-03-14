# 并行营收运营报价台

这是一个演示营收运营报价台如何在多条 review lane 并行收口后生成统一 quote packet 的范例。
它的重点是把结构化的 `Deal Structure`、`Pricing Guardrail`、`Billing Setup`、`Commercial Terms` 四条 lane 同步收齐，再由 finalizer 填充最终报价字段。

这个范例展示的是:

- `ralph#1` 先把报价资料包扇出到 4 条 review lane。
- 每个 lane 只输出真实事件 `<event ...>payload</event>`，不允许命令、解释或 `LOOP_COMPLETE`。
- 四条 ready 事件全部落盘后，`ralph#1` 只发布一次 `revops.quote.packet.request`。
- `quote_desk_lead` finalizer 以 `quote.packet.ready` 汇总最终 payload，并且 payload 中的核心字段被锁死。
- 一旦 finalizer 输出后，`ralph#1` 输出总结并以 `LOOP_COMPLETE` 结束。

## 适合用来演示什么

- 多条营收 lane 并行推进的 fanout + fanin 流程。
- finalizer 如何在统一的 completion topic 下汇总 quote packet。
- 通过固定的 `quote.packet.ready` payload 字段（如 `quote_status`、`deal_motion`、`pricing_owner`）让验证更稳定。

## 目录内容

- `ralph.yml`: coordinator/hat 协议与事件约束。
- `PROMPT.md`: 可替换的 quote packet 数据。

## 运行

在 example 目录中直接运行:

```bash
cd examples/parallel-revops-quote-desk
cargo run --bin ralph -- run --no-tui
```

如果在仓库根目录:

```bash
cargo run --bin ralph -- run \
  -c examples/parallel-revops-quote-desk/ralph.yml \
  -P examples/parallel-revops-quote-desk/PROMPT.md \
  --no-tui
```

## 你应该看到什么

一次完整闭环至少包含以下 topic:

- `revops.deal.structure.review`
- `revops.pricing.guardrail.review`
- `revops.billing.setup.review`
- `revops.commercial.terms.review`
- `structure.ready`
- `pricing.ready`
- `billing.ready`
- `terms.ready`
- `revops.quote.packet.request`
- `quote.packet.ready`

最终 `quote.packet.ready` payload 必须包含:

- `quote_status: READY_FOR_SELLER_HANDOFF`
- `deal_motion: EXPANSION_UPSELL`
- `pricing_owner: revops-desk`
- `quote_id`、`quote_summary` 与 `pricing_approval`

```text
<event topic="quote.packet.ready">
quote_id: ...
quote_status: READY_FOR_SELLER_HANDOFF
deal_motion: EXPANSION_UPSELL
pricing_owner: revops-desk
pricing_approval: ...
quote_summary: ...
</event>
```

如果任一 lane 只输出了转义的 `<event ...>` 展示文本，那不算 ready；`ralph#1` 会继续等待最后一道 ready。

## 如何替换成你自己的 quote desk

只要替换 `PROMPT.md`，保持 4 个 packet 段存在即可。
它们会被 `ralph#1` 识别并分别发送到对应 lane，从而保持 fanout / fanin 的节奏。
