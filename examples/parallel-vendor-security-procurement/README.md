# 并行供应商安全与采购审查

这是一个"供应商引入前多审查面并行收敛"的可运行范例。
它更接近真实供应商接入评审场景。

这个范例展示的是:

- `ralph#1` 先把供应商资料包扇出到 4 条处理线
- security / privacy / procurement / legal 同时推进
- `vendor_decider` 汇总统一准入结论
- `ralph#1` 最后输出审批摘要并结束

## 适合用来演示什么

- 供应商接入场景下的并行协作
- 多输入线收敛为单一准入决策
- fanout -> fanin -> vendor decider 的收口流程

## 目录内容

- `ralph.yml`: 并行拓扑和协调协议
- `PROMPT.md`: 一份可替换的供应商资料包

## 运行

在 example 目录中直接运行:

```bash
cd examples/parallel-vendor-security-procurement
cargo run --bin ralph -- run --no-tui
```

如果你在仓库根目录运行:

```bash
cargo run --bin ralph -- run \
  -c examples/parallel-vendor-security-procurement/ralph.yml \
  -P examples/parallel-vendor-security-procurement/PROMPT.md \
  --no-tui
```

## 你应该看到什么

一次正常闭环至少包含这些 topic:

- `vendor.security.assess`
- `vendor.privacy.review`
- `vendor.procurement.check`
- `vendor.legal.review`
- `security.assessed`
- `privacy.ready`
- `procurement.ready`
- `legal.ready`
- `vendor.decision.request`
- `vendor.ready`

最后由 `ralph#1` 输出 `LOOP_COMPLETE`。

## 如何替换成你自己的供应商场景

只需要改 `PROMPT.md`。
保持这 4 个资料包章节结构不变:

- `## Security Packet`
- `## Privacy Packet`
- `## Procurement Packet`
- `## Legal Packet`

这样 `ralph#1` 就能继续稳定扇出给对应处理线。
