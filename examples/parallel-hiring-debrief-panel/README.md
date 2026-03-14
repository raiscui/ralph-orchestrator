# 并行招聘复盘面板

这是一个"招聘复盘前多评估面并行收敛"的可运行范例。
它更接近真实最终复盘会前的候选人材料汇总场景。

这个范例展示的是:

- `ralph#1` 先把招聘资料包扇出到 4 条处理线
- 编码能力、系统设计、协作表现、背调反馈同时推进
- `hiring_facilitator` 汇总统一招聘结论
- `ralph#1` 最后输出招聘摘要并结束
- 每个处理线角色都必须只输出真实事件,不能输出 `&lt;event ...&gt;` 这种展示文本

## 适合用来演示什么

- 招聘复盘场景下的并行协作
- 多输入线收敛为单一招聘建议
- fanout -> fanin -> hiring facilitator 的收口流程

## 目录内容

- `ralph.yml`: 并行拓扑和协调协议
- `PROMPT.md`: 一份可替换的招聘资料包

## 运行

在 example 目录中直接运行:

```bash
cd examples/parallel-hiring-debrief-panel
cargo run --bin ralph -- run --no-tui
```

如果你在仓库根目录运行:

```bash
cargo run --bin ralph -- run \
  -c examples/parallel-hiring-debrief-panel/ralph.yml \
  -P examples/parallel-hiring-debrief-panel/PROMPT.md \
  --no-tui
```

## 你应该看到什么

一次正常闭环至少包含这些 topic:

- `hiring.coding.debrief`
- `hiring.system.debrief`
- `hiring.collaboration.debrief`
- `hiring.reference.debrief`
- `coding.ready`
- `system.ready`
- `collaboration.ready`
- `reference.ready`
- `hiring.packet.request`
- `hiring.packet.ready`

最后由 `ralph#1` 输出 `LOOP_COMPLETE`。

如果某个处理线只打印了转义的 `&lt;event ...&gt;`,那不算真正发布事件。
这时 `ralph#1` 会继续等待缺失的 ready topic,不会发布 `hiring.packet.request`。

## 如何替换成你自己的招聘场景

只需要改 `PROMPT.md`。
保持这 4 个资料包章节结构不变:

- `## Coding Packet`
- `## System Design Packet`
- `## Collaboration Packet`
- `## Reference Packet`

这样 `ralph#1` 就能继续稳定扇出给对应处理线。
