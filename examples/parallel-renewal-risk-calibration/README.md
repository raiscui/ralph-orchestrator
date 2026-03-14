# 并行续费风险预测校准

这是一个"续费组合盘 forecast 校准"的可运行范例。
它不是单个客户的续约处理台。
它更像续费经营周会上,把多条风险输入线并行收口以后,再形成统一预测结论的场景。

这个范例展示的是:

- `ralph#1` 先把校准资料包扇出到 4 条处理线
- 使用信号、赞助覆盖、商业阻塞、成功计划同时推进
- `renewal_calibration_lead` 汇总统一的 forecast commit 结论
- `ralph#1` 只有在 final topic 出现后才输出摘要并结束
- 每个 worker 和 finalizer 都必须只输出真实事件,不能输出转义展示文本

## 适合用来演示什么

- 续费组合盘预测为什么适合并行编排
- 多条输入线如何收敛为单一 forecast 校准包
- fanout -> fanin -> finalizer 的真实并行拓扑

## 目录内容

- `ralph.yml`: 并行拓扑和协调协议
- `PROMPT.md`: 一份可替换的续费校准资料包

## 运行

在 example 目录中直接运行:

```bash
cd examples/parallel-renewal-risk-calibration
cargo run --bin ralph -- run --no-tui
```

如果你在仓库根目录运行:

```bash
cargo run --bin ralph -- run \
  -c examples/parallel-renewal-risk-calibration/ralph.yml \
  -P examples/parallel-renewal-risk-calibration/PROMPT.md \
  --no-tui
```

## 你应该看到什么

一次正常闭环至少包含这些 topic:

- `renewal.usage.signal.review`
- `renewal.sponsor.coverage.review`
- `renewal.commercial.blocker.review`
- `renewal.success.plan.review`
- `usage.ready`
- `sponsor.ready`
- `blocker.ready`
- `success.ready`
- `renewal.calibration.packet.request`
- `renewal.calibration.ready`

最终 `renewal.calibration.ready` payload 必须包含:

- `calibration_status: READY_FOR_FORECAST_COMMIT`
- `forecast_window: Q3_RENEWAL_CALIBRATION`
- `forecast_owner: retention-ops`
- `calibration_summary`

最后由 `ralph#1` 输出 `LOOP_COMPLETE`。

如果某个处理线只打印了转义的 `&lt;event ...&gt;`,那不算真正发布事件。
这时 `ralph#1` 会继续等待缺失的 ready topic,不会发布 `renewal.calibration.packet.request`。

## 如何替换成你自己的校准场景

只需要改 `PROMPT.md`。
保持这 4 个资料包章节结构不变:

- `## Usage Signal Packet`
- `## Sponsor Coverage Packet`
- `## Commercial Blocker Packet`
- `## Success Plan Packet`

这样 `ralph#1` 就能继续稳定扇出给对应处理线。
