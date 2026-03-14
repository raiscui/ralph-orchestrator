# 并行区域经营周会收口

这是一个演示区域经营周会如何在多条经营输入线并行收口后生成统一周会结论包的范例。
它强调的不是发布准备,也不是单客户续约。
它更像真实的区域经营节奏: 销售 pipeline、交付产能、支持信号、人才计划四条线同时到位,最后再由 finalizer 发出统一 review packet。

这个范例展示的是:

- `ralph#1` 先把区域周会资料包扇出到 4 条 review lane。
- 每个 lane 只输出真实事件 `<event ...>payload</event>`，不允许命令、解释或 `LOOP_COMPLETE`。
- 四条 ready 全部落盘后,`ralph#1` 才会发布一次 `regional.operating.packet.request`。
- `regional_operating_lead` finalizer 以 `regional.review.ready` 汇总最终 payload,并把终态字段锁死。
- finalizer 输出后,`ralph#1` 再输出一段周会摘要并以 `LOOP_COMPLETE` 结束。

## 适合用来演示什么

- 区域经营周会为什么适合 fanout + fanin 的并行收口。
- coordinator 静默等待所有 ready 的必要性。
- 通过固定的 `regional.review.ready` 字段,让真实后端验证更稳定。

## 目录内容

- `ralph.yml`: coordinator / hat 协议与事件约束。
- `PROMPT.md`: 可替换的区域周会资料包。

## 运行

在 example 目录中直接运行:

```bash
cd examples/parallel-regional-operating-review
cargo run --bin ralph -- run --no-tui
```

如果在仓库根目录:

```bash
cargo run --bin ralph -- run \
  -c examples/parallel-regional-operating-review/ralph.yml \
  -P examples/parallel-regional-operating-review/PROMPT.md \
  --no-tui
```

## 你应该看到什么

一次完整闭环至少包含以下 topic:

- `regional.pipeline.health.review`
- `regional.delivery.capacity.review`
- `regional.support.signal.review`
- `regional.talent.plan.review`
- `pipeline.ready`
- `delivery.ready`
- `support.ready`
- `talent.ready`
- `regional.operating.packet.request`
- `regional.review.ready`

最终 `regional.review.ready` payload 必须包含:

- `review_status: READY_FOR_REGION_WEEKLY`
- `region_code: APAC_ENTERPRISE`
- `operating_owner: regional-chief-of-staff`
- `review_id`、`packet_summary` 与 `next_action_owner`

```text
<event topic="regional.review.ready">
review_id: ...
review_status: READY_FOR_REGION_WEEKLY
region_code: APAC_ENTERPRISE
operating_owner: regional-chief-of-staff
packet_summary: ...
next_action_owner: regional-chief-of-staff
</event>
```

如果任一 lane 只输出了转义的 `<event ...>` 展示文本,那不算真正的 ready。
这时 `ralph#1` 会继续等待缺失的 ready topic,不会提前发起 `regional.operating.packet.request`。

## 如何替换成你自己的区域经营周会

只要替换 `PROMPT.md`,同时保持 4 个 packet 段存在即可。
`ralph#1` 会继续按四条 lane 扇出,从而保持相同的 fanout / fanin 节奏。
