# 高层业务回顾材料准备

这是一个可运行示例,展示如何把营收叙事、产品采纳、风险展望、管理层诉求四条输入线并行收敛,最终形成一份 EBR packet。

## 展示内容

- `ralph#1` 先并行扇出四条 review topic,分别由 revenue、adoption、risk、asks 四位 lane owner 处理。
- 每条 lane 必须只输出真实 `<event ...>...</event>` 事件,且不得自闭合、不得把字段塞入标签属性。
- `ebr_chief_of_staff` 作为 finalizer 只在收到 `ebr.packet.request` 后输出 `ebr.packet.ready`。
- `ralph#1` 在 `ebr.packet.ready` 之后才输出包含固定字段的 summary 并以 `LOOP_COMPLETE` 结尾。

## 适合演示

- 高层业务回顾材料准备的多路并行协同与收敛。
- 让管理层同时看到 narrative、adoption、risk 和 exec asks 四个视角。
- 演示 fanout 到四条 lane、fan-in 到 finalizer、由 coordinator 输出最终 summary 的协议。

## 目录结构

- `ralph.yml`: 并行拓扑、帽子定义与 coordinator 指令。
- `PROMPT.md`: 中文 EBR packet 资料。

## 运行方式

```bash
cd examples/parallel-executive-business-review-prep
cargo run --bin ralph -- run --no-tui
```

或在仓库根目录:

```bash
cargo run --bin ralph -- run \
  -c examples/parallel-executive-business-review-prep/ralph.yml \
  -P examples/parallel-executive-business-review-prep/PROMPT.md \
  --no-tui
```

## 预期输出

应当包含如下 topic 顺序:

- `ebr.revenue.narrative.review`
- `ebr.product.adoption.review`
- `ebr.risk.outlook.review`
- `ebr.exec.asks.review`
- `revenue.ready`
- `adoption.ready`
- `risk.ready`
- `asks.ready`
- `ebr.packet.request`
- `ebr.packet.ready`

最终 `EBR Chief of Staff` 输出 summary,其中 payload 包含 `ebr_status: READY_FOR_EXEC_REVIEW`, `meeting_tier: Q2_BUSINESS_REVIEW`, `narrative_owner: gm-office`, `packet_summary` 和 `next_action_owner: gm-office`, 然后 `ralph#1` 在最后一行单独输出 `LOOP_COMPLETE`。

如果某条 lane 只是展示 `<event ...>` 以示例形式,那不算真实 event, coordinator 会继续等待缺失的 ready topic。
