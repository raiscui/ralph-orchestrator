# 客户顾问委员会并行筹备

这是一个以客户顾问委员会(CAB)筹备为模型的并行 example，展示多条准备线路如何在 coordinated fan-in 后形成统一确认包。

这个范例聚焦的是：

- `ralph#1` 先把 CAB packet 扇出到四条 lane：客户群体、议程形状、高层主持准备与物流读数。
- 每条 lane 角色都必须输出真实的 `<event>` 事件（非转义文本），并在完成后发布 `*.ready`。
- `cab_program_lead` 在 fan-in 后收到 `cab.packet.request`，产出 `cab.packet.ready`，并由 `ralph#1` 输出收尾 summary 与 `LOOP_COMPLETE`。

## 适合演示的场景

- 客户顾问委员会召集前的多维输入并行核实。
- 多个 lane 同步推进后由专人打包确认，防止遗漏要素。
- 示范 CAB packet fanned-out -> ready -> fan-in -> finalizer 这一典型流程。

## 目录说明

- `ralph.yml`: 并行拓扑与 coordinator/worker/finalizer instructions。重点强调静默等待与事件格式。
- `PROMPT.md`: `CAB_PACKET`，包含 cohort/agenda/host/logistics 的结构化内容。
- `README.md`: 中文说明与运行/输出预期。

## 运行方式

在目录下直接运行：

```bash
cd examples/parallel-customer-advisory-board-prep
cargo run --bin ralph -- run --no-tui
```

在仓库根目录跑：

```bash
cargo run --bin ralph -- run \
  -c examples/parallel-customer-advisory-board-prep/ralph.yml \
  -P examples/parallel-customer-advisory-board-prep/PROMPT.md \
  --no-tui
```

## 你应该看到的 topic 链

- `cab.customer.cohort.review`
- `cab.agenda.shaping.review`
- `cab.exec.host.prep.review`
- `cab.logistics.readiness.review`
- `cohort.ready`, `agenda.ready`, `host.ready`, `logistics.ready`
- `cab.packet.request`
- `cab.packet.ready`
- 最后一行 `ralph#1` 输出 `LOOP_COMPLETE`

## 终态 payload 要求

`cab.packet.ready` 的 payload 必须包含：

- `cab_status: READY_TO_CONFIRM`
- `event_region: APJ`
- `next_owner: customer-marketing`
- `packet_focus`（用于描述本轮重点）
- `summary`（一句话概况）

## 自定义提示

只需替换 `PROMPT.md` 中的 4 个分区内容与 metadata，其余 fanout/fanin 协议保持不变。
