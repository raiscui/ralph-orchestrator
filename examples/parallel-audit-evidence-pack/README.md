# 并行审计证据包

这是一个"审计前多证据面并行收敛"的可运行范例。
它更接近真实审计就绪前的证据打包场景。

这个范例展示的是:

- `ralph#1` 先把审计资料包扇出到 4 条处理线
- 访问记录、变更日志、备份状态、事故历史同时推进
- `audit_packet_editor` 汇总统一的审计证据包
- `ralph#1` 最后输出审计摘要并结束
- 每个处理线角色都必须只输出真实事件,不能输出 `&lt;event ...&gt;` 这种展示文本

## 适合用来演示什么

- 审计证据收集的并行协作
- 多输入线收敛为单一审计证据包
- fanout -> fanin -> `audit_packet_editor` 的收口流程

## 目录内容

- `ralph.yml`: 并行拓扑和协调协议
- `PROMPT.md`: 一份可替换的审计资料包

## 运行

在 example 目录中直接运行:

```bash
cd examples/parallel-audit-evidence-pack
cargo run --bin ralph -- run --no-tui
```

如果你在仓库根目录运行:

```bash
cargo run --bin ralph -- run \
  -c examples/parallel-audit-evidence-pack/ralph.yml \
  -P examples/parallel-audit-evidence-pack/PROMPT.md \
  --no-tui
```

## 你应该看到什么

一次正常闭环至少包含这些 topic:

- `audit.access.export`
- `audit.change.log.collect`
- `audit.backup.verify`
- `audit.incident.history.collect`
- `access.evidence.ready`
- `change.evidence.ready`
- `backup.evidence.ready`
- `incident.evidence.ready`
- `audit.packet.request`
- `audit.packet.ready`

最后由 `ralph#1` 输出 `LOOP_COMPLETE`。

如果某个处理线只打印了转义的 `&lt;event ...&gt;`,那不算真正发布事件。
这时 `ralph#1` 会继续等待缺失的 ready topic,不会发布 `audit.packet.request`。

## 如何替换成你自己的审计场景

只需要改 `PROMPT.md`。
保持这 4 个资料包章节结构不变:

- `## Access Evidence Packet`
- `## Change Log Packet`
- `## Backup Verification Packet`
- `## Incident History Packet`

这样 `ralph#1` 就能继续稳定扇出给对应处理线。
