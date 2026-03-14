# 并行方案组装

这是一个"方案材料多输入线并行收敛"的可运行范例。
它更接近真实售前、投标或客户方案准备场景。

这个范例展示的是:

- `ralph#1` 先把方案资料包扇出到 4 条处理线
- 研究、定价、法务、管理层材料同时推进
- `proposal_editor` 汇总最终建议
- `ralph#1` 最后输出方案摘要并结束

## 适合用来演示什么

- 售前方案准备中的并行协作
- 多输入线收敛
- fanout -> 编辑汇总 -> 最终摘要 的收口方式

## 目录内容

- `ralph.yml`: 并行拓扑和协调协议
- `PROMPT.md`: 一份可替换的方案资料包

## 运行

在 example 目录中直接运行:

```bash
cd examples/parallel-proposal-assembly
cargo run --bin ralph -- run --no-tui
```

如果你在仓库根目录运行:

```bash
cargo run --bin ralph -- run \
  -c examples/parallel-proposal-assembly/ralph.yml \
  -P examples/parallel-proposal-assembly/PROMPT.md \
  --no-tui
```

## 你应该看到什么

一次正常闭环至少包含这些 topic:

- `proposal.research.task`
- `proposal.pricing.task`
- `proposal.legal.task`
- `proposal.exec.task`
- `research.done`
- `pricing.done`
- `legal.done`
- `exec.done`
- `proposal.merge.request`
- `proposal.ready`

最后由 `ralph#1` 输出 `LOOP_COMPLETE`。

## 如何替换成你自己的方案场景

只需要改 `PROMPT.md`。
保持这 4 个资料包章节结构不变:

- `## Research Packet`
- `## Pricing Packet`
- `## Legal Packet`
- `## Executive Packet`

这样 `ralph#1` 就能继续稳定扇出给对应处理线。
