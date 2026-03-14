# 并行复盘行动板

这是一个"事故复盘多输入线并行收敛"的可运行范例。
它更接近真实事故复盘会前的材料准备场景。

这个范例展示的是:

- `ralph#1` 先把复盘资料包扇出到 4 条处理线
- timeline / root cause / actions / customer recap 同时推进
- `board_facilitator` 汇总统一行动板
- `ralph#1` 最后输出行动板摘要并结束
- 每个处理线角色都必须只输出真实事件,不能输出 `&lt;event ...&gt;` 这种展示文本

## 适合用来演示什么

- 复盘材料并行整理
- 多输入线收敛为单一行动板
- fanout -> fanin -> board facilitator 的收口流程

## 目录内容

- `ralph.yml`: 并行拓扑和协调协议
- `PROMPT.md`: 一份可替换的复盘资料包

## 运行

在 example 目录中直接运行:

```bash
cd examples/parallel-postmortem-action-board
cargo run --bin ralph -- run --no-tui
```

如果你在仓库根目录运行:

```bash
cargo run --bin ralph -- run \
  -c examples/parallel-postmortem-action-board/ralph.yml \
  -P examples/parallel-postmortem-action-board/PROMPT.md \
  --no-tui
```

## 你应该看到什么

一次正常闭环至少包含这些 topic:

- `pm.timeline.build`
- `pm.root_cause.review`
- `pm.action.map`
- `pm.customer.recap`
- `timeline.ready`
- `root_cause.ready`
- `actions.ready`
- `customer.recap.ready`
- `pm.board.request`
- `postmortem.board.ready`

最后由 `ralph#1` 输出 `LOOP_COMPLETE`。

如果某个处理线只打印了转义的 `&lt;event ...&gt;`,那不算真正发布事件。
这时 `ralph#1` 会继续等待缺失的 ready topic,不会发布 `pm.board.request`。

## 如何替换成你自己的复盘案例

只需要改 `PROMPT.md`。
保持这 4 个资料包章节结构不变:

- `## Timeline Packet`
- `## Root Cause Packet`
- `## Action Mapping Packet`
- `## Customer Recap Packet`

这样 `ralph#1` 就能继续稳定扇出给对应处理线。
