# 并行发布检查清单

这是一个"发布前置检查并行执行"的可运行范例。
它适合用来演示发布就绪这类扇出 / 收口工作流。

## 它在做什么

- `ralph#1` 从 `RELEASE_PACKET` 中拆出 3 条检查任务
- `qa_checker`、`docs_checker`、`ops_checker` 并行工作
- 3 条就绪结果全部到齐后,`ralph#1` 发布 `release.summary.request`
- `release_synthesizer` 再产出 `release.ready`
- 最后由 `ralph#1` 输出总结和 `LOOP_COMPLETE`

## 运行

在 example 目录中直接运行:

```bash
cd examples/parallel-release-checklist
cargo run --bin ralph -- run --no-tui
```

或者在仓库根目录运行:

```bash
cargo run --bin ralph -- run \
  -c examples/parallel-release-checklist/ralph.yml \
  -P examples/parallel-release-checklist/PROMPT.md \
  --no-tui
```

## 你应该看到什么

一次正常闭环至少包含:

- `release.qa.check`
- `release.docs.check`
- `release.ops.check`
- `qa.ready`
- `docs.ready`
- `ops.ready`
- `release.summary.request`
- `release.ready`

最后由 `ralph#1` 输出 `LOOP_COMPLETE`。

## 什么时候适合参考这个范例

- 你想表达"多个前置条件必须全部就绪"
- 你想让不同检查角色各管一块
- 你希望最终决策由协调者统一给出
