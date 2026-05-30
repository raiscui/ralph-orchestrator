# MULTI_REGION_PIPELINE_PACKET

你正在准备一次全球 forecast call 之前的多区域 pipeline 校准资料包。
这个 packet 只承载并行扇出所需的结构化上下文,不包含任何模型执行指令。

## Sync Meta

- sync_id: MRP-2026-W15
- operating_motion: Global Forecast Call
- owner_team: revenue-operations
- target_week: FY26_W15

## Americas Pipeline Packet

- focus: 对齐 Americas 区域的 commit、coverage 与 deal risk
- expected_status: ready
- coverage_view:
  - 核心 enterprise deals 已完成经理复核
  - coverage gap 已映射到两条新增 SDR pod
  - deal desk 已确认本周无新的非标 blocker

## EMEA Pipeline Packet

- focus: 对齐 EMEA 区域的 commit posture 与 stage hygiene
- expected_status: ready
- coverage_view:
  - EMEA manager sync 已确认 top deals 的 next step owner
  - 两个跨国机会已补齐 MEDDPICC 证据
  - 区域 forecast review 已更新变更窗口

## APJ Pipeline Packet

- focus: 对齐 APJ 区域的 pipeline shape 与 expansion mix
- expected_status: ready
- coverage_view:
  - APJ team 已确认大客户扩张机会的 close plan
  - partner-assisted deals 已补齐联合 owner
  - 扩张与新客占比已更新到本周版本

## LATAM Pipeline Packet

- focus: 对齐 LATAM 区域的 risk map 与 inspection 节奏
- expected_status: ready
- coverage_view:
  - LATAM frontline manager 已确认 inspection 节奏
  - 两个高波动机会已给出明确的 rescue owner
  - pricing escalation 已完成本周校正

## Expected Final Outcome

- sync_status: READY_FOR_GLOBAL_FORECAST_CALL
- forecast_week: FY26_W15
- sync_owner: global-revenue-operations
- next_forum: global-forecast-call
