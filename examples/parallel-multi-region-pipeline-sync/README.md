# 并行多区域 pipeline 校准

这是一个演示全球 forecast call 之前,多个区域如何把 pipeline 口径并行收口的范例。
它强调的是同一经营主题在多区域同时推进。
它不是单一区域周会,也不是单个客户 deal review。

这个范例展示的是:

- `ralph#1` 先把同一份 pipeline 校准资料包扇出到 4 个区域处理线。
- Americas、EMEA、APJ、LATAM 分别给出自己的 ready 结论。
- 4 条区域 ready 全部收齐后,`ralph#1` 只发布一次 `pipeline.sync.packet.request`。
- `global_pipeline_sync_lead` finalizer 统一生成 `pipeline.sync.ready`。
- 最终 payload 中的 `sync_status`、`forecast_week`、`sync_owner` 会被锁死,便于 E2E 验证。

## 适合用来演示什么

- 多区域 forecast 口径如何用 fanout + fanin 并行建模。
- 为什么 coordinator 在 ready 没齐前必须保持静默。
- 为什么 finalizer 明确 owner 和固定终态字段后,真实后端验证会更稳。

## 目录内容

- `ralph.yml`: coordinator / hat 协议与事件约束。
- `PROMPT.md`: 可替换的多区域 pipeline 资料包。

## 运行

在 example 目录中直接运行:

```bash
cd examples/parallel-multi-region-pipeline-sync
cargo run --bin ralph -- run --no-tui
```

如果在仓库根目录:

```bash
cargo run --bin ralph -- run \
  -c examples/parallel-multi-region-pipeline-sync/ralph.yml \
  -P examples/parallel-multi-region-pipeline-sync/PROMPT.md \
  --no-tui
```

## 你应该看到什么

一次完整闭环至少包含以下 topic:

- `pipeline.amer.review`
- `pipeline.emea.review`
- `pipeline.apj.review`
- `pipeline.latam.review`
- `amer.ready`
- `emea.ready`
- `apj.ready`
- `latam.ready`
- `pipeline.sync.packet.request`
- `pipeline.sync.ready`

最终 `pipeline.sync.ready` payload 必须包含:

- `sync_status: READY_FOR_GLOBAL_FORECAST_CALL`
- `forecast_week: FY26_W15`
- `sync_owner: global-revenue-operations`
- `sync_id`、`regional_summary` 与 `next_forum`

```text
<event topic="pipeline.sync.ready">
sync_id: ...
sync_status: READY_FOR_GLOBAL_FORECAST_CALL
forecast_week: FY26_W15
sync_owner: global-revenue-operations
next_forum: global-forecast-call
regional_summary: ...
</event>
```

如果任一区域 lane 只输出了转义的 `<event ...>` 展示文本,那不算 ready。
这时 `ralph#1` 会继续等待缺失的 ready,不会过早发出 `pipeline.sync.packet.request`。

## 如何替换成你自己的多区域 pipeline 场景

只要替换 `PROMPT.md`,保持 4 个区域 packet 段存在即可。
它们会被 `ralph#1` 识别并分别发送到对应区域 lane,从而保持 fanout / fanin 的节奏。
