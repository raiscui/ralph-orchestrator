# RENEWAL_CALIBRATION_PACKET

你正在准备一场"续费组合盘预测校准"。
这不是单个客户的续约战情室。
这里的目标是把使用信号、赞助人覆盖、商业阻塞和成功计划 4 条输入线并行收齐,再形成统一 forecast commit 结论。

## Calibration Meta

- portfolio_id: RNL-Q3-PORTFOLIO-17
- portfolio_segment: enterprise-renewals
- forecast_window: Q3_RENEWAL_CALIBRATION
- review_owner: retention-ops

## Usage Signal Packet

- focus: 确认组合盘的真实使用趋势是否支撑续费预测
- expected_status: ready
- expected_usage_signal: broad_adoption_stable
- evidence:
  - 重点账户群的周活仍高于续费红线
  - 核心功能渗透率连续 6 周保持稳定
  - 最近一次产品培训后,新增团队开始进入常规使用

## Sponsor Coverage Packet

- focus: 确认高层赞助覆盖是否足以支撑 forecast
- expected_status: ready
- expected_sponsor_coverage: executive_paths_mapped
- evidence:
  - Top 15 续费账户都已标记 executive sponsor owner
  - 其中 9 个账户已经安排季度经营回顾
  - 尚未落位的 sponsor 缺口都有负责人和时间表

## Commercial Blocker Packet

- focus: 确认商业阻塞是否可控
- expected_status: ready
- expected_blocker_posture: blockers_within_plan
- evidence:
  - 价格例外都已进入 deal desk 清单
  - 法务风险主要集中在已知合同条款,没有新增红线
  - 采购延迟账户均已给出可接受的内部恢复方案

## Success Plan Packet

- focus: 确认 CS 成功计划是否覆盖高风险账户
- expected_status: ready
- expected_success_plan: risk_playbooks_assigned
- evidence:
  - 高风险续费账户都已绑定 success owner
  - 风险 playbook 已拆到周粒度动作
  - 未来 30 天的客户沟通节点已经排期

## Expected Final Outcome

- calibration_status: READY_FOR_FORECAST_COMMIT
- forecast_window: Q3_RENEWAL_CALIBRATION
- forecast_owner: retention-ops
- calibration_summary: usage、sponsor、commercial、success 四线都可支撑本轮 commit
