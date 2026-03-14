# 并行迁移演练

这是一个"迁移演练多检查面并行收敛"的可运行范例。
它更接近真实发布前的数据库或关键数据迁移准备。

这个范例展示的是:

- `ralph#1` 先把迁移演练资料包扇出到 4 条处理线
- schema / backup / smoke / rollback 同时推进
- `migration_conductor` 汇总通过 / 暂缓结论
- `ralph#1` 最后输出迁移摘要并结束

## 适合用来演示什么

- 迁移演练场景下的并行协作
- 多条就绪处理线收敛
- 通过 / 暂缓汇总器模式

## 目录内容

- `ralph.yml`: 并行拓扑和协调协议
- `PROMPT.md`: 一份可替换的迁移演练资料包

## 运行

在 example 目录中直接运行:

```bash
cd examples/parallel-migration-rehearsal
cargo run --bin ralph -- run --no-tui
```

如果你在仓库根目录运行:

```bash
cargo run --bin ralph -- run \
  -c examples/parallel-migration-rehearsal/ralph.yml \
  -P examples/parallel-migration-rehearsal/PROMPT.md \
  --no-tui
```

## 你应该看到什么

一次正常闭环至少包含这些 topic:

- `migration.schema.review`
- `migration.backup.verify`
- `migration.smoke.run`
- `migration.rollback.audit`
- `schema.ready`
- `backup.ready`
- `smoke.ready`
- `rollback.ready`
- `migration.go_no_go.request`
- `migration.ready`

最后由 `ralph#1` 输出 `LOOP_COMPLETE`。

## 如何替换成你自己的迁移演练场景

只需要改 `PROMPT.md`。
保持这 4 个资料包章节结构不变:

- `## Schema Packet`
- `## Backup Packet`
- `## Smoke Packet`
- `## Rollback Packet`

这样 `ralph#1` 就能继续稳定扇出给对应处理线。
