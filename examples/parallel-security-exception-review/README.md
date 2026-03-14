# 并行安全例外审查

这是一个"内部安全例外多审查面并行收敛"的可运行范例。
它更接近真实安全例外评审会的场景。

这个范例展示的是:

- `ralph#1` 先把例外资料包扇出到 4 条处理线
- 威胁评估、控制措施、数据范围、到期策略同时推进
- `exception_decider` 汇总统一例外审批结论
- `ralph#1` 最后输出审批摘要并结束
- 每个处理线角色都必须只输出真实事件,不能输出 `&lt;event ...&gt;` 这种展示文本

## 适合用来演示什么

- 内部安全例外场景下的并行协作
- 多输入线收敛为单一例外审批结论
- fanout -> fanin -> exception decider 的收口流程

## 目录内容

- `ralph.yml`: 并行拓扑和协调协议
- `PROMPT.md`: 一份可替换的例外资料包

## 运行

在 example 目录中直接运行:

```bash
cd examples/parallel-security-exception-review
cargo run --bin ralph -- run --no-tui
```

如果你在仓库根目录运行:

```bash
cargo run --bin ralph -- run \
  -c examples/parallel-security-exception-review/ralph.yml \
  -P examples/parallel-security-exception-review/PROMPT.md \
  --no-tui
```

## 你应该看到什么

一次正常闭环至少包含这些 topic:

- `exception.threat.review`
- `exception.controls.review`
- `exception.data.scope.review`
- `exception.expiry.review`
- `threat.reviewed`
- `controls.reviewed`
- `data.scope.ready`
- `expiry.ready`
- `exception.decision.request`
- `exception.ready`

最后由 `ralph#1` 输出 `LOOP_COMPLETE`。

如果某个处理线只打印了转义的 `&lt;event ...&gt;`,那不算真正发布事件。
这时 `ralph#1` 会继续等待缺失的 ready topic,不会发布 `exception.decision.request`。

## 如何替换成你自己的例外场景

只需要改 `PROMPT.md`。
保持这 4 个资料包章节结构不变:

- `## Threat Packet`
- `## Controls Packet`
- `## Data Scope Packet`
- `## Expiry Packet`

这样 `ralph#1` 就能继续稳定扇出给对应处理线。
