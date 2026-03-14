# 并行代码评审

这是一个"多名评审角色并行审查同一份 PR 资料包"的可运行范例。
它更接近日常代码评审场景。

这个范例展示的是:

- `ralph#1` 先把同一份 PR 资料包扇出给 3 个评审角色
- 3 个评审角色并行完成各自职责
- `review_synthesizer` 汇总结果
- `ralph#1` 最后输出统一结论并结束

## 适合用来演示什么

- 多视角代码评审并行推进
- fanout -> fanin 的收口流程
- 评审角色与汇总角色的职责拆分

## 目录内容

- `ralph.yml`: 并行拓扑和协调协议
- `PROMPT.md`: 一份可替换的代码评审资料包

## 运行

在 example 目录中直接运行:

```bash
cd examples/parallel-pr-review
cargo run --bin ralph -- run --no-tui
```

如果你在仓库根目录运行:

```bash
cargo run --bin ralph -- run \
  -c examples/parallel-pr-review/ralph.yml \
  -P examples/parallel-pr-review/PROMPT.md \
  --no-tui
```

## 你应该看到什么

一次正常闭环至少包含这些 topic:

- `review.correctness`
- `review.security`
- `review.architecture`
- `correctness.done`
- `security.done`
- `architecture.done`
- `synthesis.request`
- `review.complete`

最后由 `ralph#1` 输出 `LOOP_COMPLETE`。

## 如何替换成你自己的 PR

只需要改 `PROMPT.md`。
保持 3 个资料包章节结构不变:

- `## Correctness Packet`
- `## Security Packet`
- `## Architecture Packet`

这样 `ralph#1` 就能继续稳定扇出给对应评审角色。
