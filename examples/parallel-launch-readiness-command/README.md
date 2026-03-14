# 并行上线准备指挥台

这是一个"正式发布前多准备面并行收敛"的可运行范例。
它更接近真实上线指挥台场景。

这个范例展示的是:

- `ralph#1` 先把上线资料包扇出到 4 条处理线
- 测试、可观测性、回滚方案、沟通计划同时推进
- `launch_commander` 汇总统一上线指令
- `ralph#1` 最后输出上线摘要并结束

## 适合用来演示什么

- 发布前准备阶段的并行协作
- 多输入线收敛为单一上线决策
- fanout -> fanin -> launch commander 的收口流程

## 目录内容

- `ralph.yml`: 并行拓扑和协调协议
- `PROMPT.md`: 一份可替换的上线资料包

## 运行

在 example 目录中直接运行:

```bash
cd examples/parallel-launch-readiness-command
cargo run --bin ralph -- run --no-tui
```

如果你在仓库根目录运行:

```bash
cargo run --bin ralph -- run \
  -c examples/parallel-launch-readiness-command/ralph.yml \
  -P examples/parallel-launch-readiness-command/PROMPT.md \
  --no-tui
```

## 你应该看到什么

一次正常闭环至少包含这些 topic:

- `launch.qa.readiness`
- `launch.observability.review`
- `launch.rollback.review`
- `launch.comms.review`
- `launch.qa.ready`
- `launch.observability.ready`
- `launch.rollback.ready`
- `launch.comms.ready`
- `launch.command.request`
- `launch.command.ready`

最后由 `ralph#1` 输出 `LOOP_COMPLETE`。

## 如何替换成你自己的上线场景

只需要改 `PROMPT.md`。
保持这 4 个资料包章节结构不变:

- `## QA Packet`
- `## Observability Packet`
- `## Rollback Packet`
- `## Comms Packet`

这样 `ralph#1` 就能继续稳定扇出给对应处理线。
