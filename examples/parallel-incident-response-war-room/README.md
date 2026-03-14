# 并行事故响应指挥台

这是一个"事故处置多工作面并行推进"的可运行范例。
它更接近真实值班响应时的指挥台场景。

这个范例展示的是:

- `ralph#1` 先把事故资料包扇出到 4 条处理线
- 分诊、日志分析、回滚方案、状态同步同时推进
- `incident_commander` 汇总统一行动方案
- `ralph#1` 最后输出事故指挥摘要并结束

## 适合用来演示什么

- 事故响应场景下的并行协作
- 多个工作面同时推进
- fanout -> fanin -> commander 的收口流程

## 目录内容

- `ralph.yml`: 并行拓扑和协调协议
- `PROMPT.md`: 一份可替换的事故资料包

## 运行

在 example 目录中直接运行:

```bash
cd examples/parallel-incident-response-war-room
cargo run --bin ralph -- run --no-tui
```

如果你在仓库根目录运行:

```bash
cargo run --bin ralph -- run \
  -c examples/parallel-incident-response-war-room/ralph.yml \
  -P examples/parallel-incident-response-war-room/PROMPT.md \
  --no-tui
```

## 你应该看到什么

一次正常闭环至少包含这些 topic:

- `incident.triage`
- `incident.logs.analyze`
- `incident.rollback.plan`
- `incident.status.prepare`
- `triage.done`
- `logs.done`
- `rollback.done`
- `status.draft.done`
- `incident.command.request`
- `incident.command.ready`

最后由 `ralph#1` 输出 `LOOP_COMPLETE`。

## 如何替换成你自己的事故场景

只需要改 `PROMPT.md`。
保持这 4 个资料包章节结构不变:

- `## Triage Packet`
- `## Logs Packet`
- `## Rollback Packet`
- `## Status Packet`

这样 `ralph#1` 就能继续稳定扇出给对应处理线。
