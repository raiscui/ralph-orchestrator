# 并行一线赋能推广

这是一个"一线赋能推广前,4 条准备线并行收口"的可运行范例。
它展示的是内部一线团队的推广准备,不是单个客户启动。

这个范例重点演示:

- `ralph#1` 先把推广资料包扇出到 4 条赋能处理线
- 课程、演示环境、经理同步、认证计划同时推进
- `rollout_conductor` 汇总统一推广结论
- `ralph#1` 只有在终态 topic 到达后才输出摘要和 `LOOP_COMPLETE`
- 每个 worker 和 finalizer 都必须只输出真实事件,不能输出转义展示文本

## 适合演示什么

- 一线赋能方案的并行收口
- 多条输入线收敛为单一推广资料包
- fanout -> fanin -> finalizer 的真实并行拓扑

## 目录内容

- `ralph.yml`: 并行拓扑和协调协议
- `PROMPT.md`: 一份可替换的赋能推广资料包

## 运行

在 example 目录中直接运行:

```bash
cd examples/parallel-field-enablement-rollout
cargo run --bin ralph -- run --no-tui
```

如果你在仓库根目录运行:

```bash
cargo run --bin ralph -- run \
  -c examples/parallel-field-enablement-rollout/ralph.yml \
  -P examples/parallel-field-enablement-rollout/PROMPT.md \
  --no-tui
```

## 你应该看到什么

一次正常闭环至少包含这些 topic:

- `enablement.curriculum.review`
- `enablement.demo.environment.review`
- `enablement.manager.briefing.review`
- `enablement.certification.plan.review`
- `curriculum.ready`
- `demo.ready`
- `briefing.ready`
- `certification.ready`
- `enablement.rollout.packet.request`
- `enablement.rollout.ready`

最后由 `ralph#1` 输出 `LOOP_COMPLETE`。

如果某个处理线只打印了转义的 `&lt;event ...&gt;`,那不算真正发布事件。
这时 `ralph#1` 会继续等待缺失的 ready topic,不会发布 `enablement.rollout.packet.request`。

## 如何替换成你自己的推广场景

只需要改 `PROMPT.md`。
保持这 4 个资料包章节结构不变:

- `## Curriculum Packet`
- `## Demo Environment Packet`
- `## Manager Briefing Packet`
- `## Certification Plan Packet`

这样 `ralph#1` 就能继续稳定扇出给对应处理线。
