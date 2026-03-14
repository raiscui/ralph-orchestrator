# 并行客户激活准备

这是一个"客户落地前多工作面并行收敛"的可运行范例。
它更接近真实客户启动前的激活准备场景。

这个范例展示的是:

- `ralph#1` 先把客户激活资料包扇出到 4 条处理线
- 集成准备、安全交接、赋能计划、成功计划同时推进
- `activation_manager` 汇总统一激活结论
- `ralph#1` 最后输出激活摘要并结束
- 每个处理线角色都必须只输出真实事件,不能输出 `&lt;event ...&gt;` 这种展示文本

## 适合用来演示什么

- 客户落地前的并行协作
- 多输入线收敛为单一激活结论
- fanout -> fanin -> activation manager 的收口流程

## 目录内容

- `ralph.yml`: 并行拓扑和协调协议
- `PROMPT.md`: 一份可替换的客户激活资料包

## 运行

在 example 目录中直接运行:

```bash
cd examples/parallel-customer-onboarding-activation
cargo run --bin ralph -- run --no-tui
```

如果你在仓库根目录运行:

```bash
cargo run --bin ralph -- run \
  -c examples/parallel-customer-onboarding-activation/ralph.yml \
  -P examples/parallel-customer-onboarding-activation/PROMPT.md \
  --no-tui
```

## 你应该看到什么

一次正常闭环至少包含这些 topic:

- `onboarding.integration.review`
- `onboarding.security.handoff`
- `onboarding.enablement.plan`
- `onboarding.success.plan.review`
- `integration.ready`
- `security.handoff.ready`
- `enablement.ready`
- `success.plan.ready`
- `onboarding.activation.request`
- `onboarding.activation.ready`

最后由 `ralph#1` 输出 `LOOP_COMPLETE`。

如果某个处理线只打印了转义的 `&lt;event ...&gt;`,那不算真正发布事件。
这时 `ralph#1` 会继续等待缺失的 ready topic,不会发布 `onboarding.activation.request`。

## 如何替换成你自己的客户激活场景

只需要改 `PROMPT.md`。
保持这 4 个资料包章节结构不变:

- `## Integration Packet`
- `## Security Handoff Packet`
- `## Enablement Packet`
- `## Success Plan Packet`

这样 `ralph#1` 就能继续稳定扇出给对应处理线。
