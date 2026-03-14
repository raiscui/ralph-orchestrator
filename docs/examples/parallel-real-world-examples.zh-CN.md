# 真实并行范例中文总览

这份文档专门给人看。
它不重复解释并行运行时的底层细节。
它主要回答 3 个问题:

1. 现在仓库里已经有哪些真实并行范例
2. 每个范例适合拿来演示什么
3. 如果要继续扩批,下一批应该怎么选题

## 这组范例的共用方案

当前这批真实并行范例,基本都沿用同一套稳定骨架:

- `prompt_file: "PROMPT.md"`
- `ralph#1` 负责首轮扇出和最终收口
- 多条并行处理线各自只做一类输入的整理
- 所有处理线都收齐后,再触发一次汇总请求
- 由明确的 finalizer 发布最终 topic
- coordinator 在条件未收齐前保持静默
- worker 和 finalizer 只允许输出真实事件

这套骨架的价值在于,它不是只适合工程任务。
它已经被证明可以稳定迁移到运营、治理、支持、商务协作这些场景里。

## 快速选型

如果你要演示这些方向,可以优先选下面的范例:

- 代码协作与工程分工:
  - `parallel-pr-review`
  - `parallel-release-checklist`
  - `parallel-launch-readiness-command`
  - `parallel-migration-rehearsal`
- 人工批准与治理关口:
  - `parallel-human-approval-gate`
  - `parallel-security-exception-review`
  - `parallel-vendor-security-procurement`
  - `parallel-audit-evidence-pack`
- 事故响应与复盘:
  - `parallel-incident-response-war-room`
  - `parallel-postmortem-action-board`
- 商务协作、经营准备与客户经营:
  - `parallel-proposal-assembly`
  - `parallel-customer-renewal-desk`
  - `parallel-customer-onboarding-activation`
  - `parallel-revops-quote-desk`
  - `parallel-executive-business-review-prep`
  - `parallel-customer-advisory-board-prep`
- 区域经营与预测校准:
  - `parallel-regional-operating-review`
  - `parallel-renewal-risk-calibration`
  - `parallel-multi-region-pipeline-sync`
- 财务、招聘与内部运营:
  - `parallel-finance-close-control-room`
  - `parallel-hiring-debrief-panel`
- 支持、伙伴与赋能:
  - `parallel-support-escalation-desk`
  - `parallel-partner-launch-coordination`
  - `parallel-field-enablement-rollout`

## 范例矩阵

| 范例目录 | 中文说明 | 适合演示 | 最终 topic |
| --- | --- | --- | --- |
| `examples/parallel-pr-review/` | 多名评审角色并行审查后统一给出结论 | 工程评审、多视角审查 | `review.complete` |
| `examples/parallel-release-checklist/` | 测试、文档、运维并行检查后统一放行 | 发布前准备 | `release.ready` |
| `examples/parallel-human-approval-gate/` | 自动化准备完成后等待人工批准 | 上线闸口、人工确认 | `deployment.ready` |
| `examples/parallel-incident-response-war-room/` | 事故现场多条处理线并行收口 | 事故响应、指挥协同 | `incident.command.ready` |
| `examples/parallel-security-exception-review/` | 安全例外申请的多维审查汇总 | 风险治理、例外审批 | `exception.ready` |
| `examples/parallel-customer-renewal-desk/` | 续约前的客户状态与商业动作汇总 | 客户续约、经营收口 | `renewal.plan.ready` |
| `examples/parallel-audit-evidence-pack/` | 审计证据从多条来源并行收集 | 审计、合规交付 | `audit.packet.ready` |
| `examples/parallel-finance-close-control-room/` | 财务关账前多条核对线并行收敛 | 财务运营、月结关账 | `close.packet.ready` |
| `examples/parallel-hiring-debrief-panel/` | 面试多维反馈并行汇总 | 招聘复盘、录用决策 | `hiring.packet.ready` |
| `examples/parallel-customer-onboarding-activation/` | 客户激活前的多条准备线并行收口 | 客户启动、交付激活 | `onboarding.activation.ready` |
| `examples/parallel-support-escalation-desk/` | 高优先级支持升级的跨团队收口 | 支持升级、升级指挥 | `escalation.plan.ready` |
| `examples/parallel-partner-launch-coordination/` | 合作伙伴联合发布前的多方协调 | 合作伙伴发布、渠道协作 | `partner.launch.ready` |
| `examples/parallel-field-enablement-rollout/` | 一线赋能推广前的准备项并行汇总 | 一线赋能、内部推广 | `enablement.rollout.ready` |
| `examples/parallel-revops-quote-desk/` | 营收运营报价台的多条 review lane 收口 | 商业报价、营收运营 | `quote.packet.ready` |
| `examples/parallel-executive-business-review-prep/` | 高层业务回顾材料的四路输入并行汇总 | 管理层回顾、经营准备 | `ebr.packet.ready` |
| `examples/parallel-customer-advisory-board-prep/` | 客户顾问委员会筹备的多方准备线收敛 | 高价值客户活动、共创筹备 | `cab.packet.ready` |
| `examples/parallel-regional-operating-review/` | 单一区域周会前,销售、交付、支持、人才四条输入线并行收口 | 区域经营、周会收口 | `regional.review.ready` |
| `examples/parallel-renewal-risk-calibration/` | 续费组合盘 forecast 校准的四条风险输入线收敛 | 续费预测、风险校准 | `renewal.calibration.ready` |
| `examples/parallel-multi-region-pipeline-sync/` | 多区域 pipeline 口径在全球 forecast call 前并行同步 | 区域同步、pipeline 校准 | `pipeline.sync.ready` |
| `examples/parallel-launch-readiness-command/` | 上线前测试、观测、回滚、沟通并行确认 | 上线准备、发布指挥 | `launch.command.ready` |
| `examples/parallel-migration-rehearsal/` | 迁移演练的数据模式、备份、烟测、回滚并行确认 | 迁移演练、变更保障 | `migration.ready` |
| `examples/parallel-postmortem-action-board/` | 复盘材料、根因、行动项、客户回顾并行汇总 | 复盘行动板、事故后续 | `postmortem.board.ready` |
| `examples/parallel-proposal-assembly/` | 方案、定价、法务、管理层材料并行组装 | 商务方案、售前协同 | `proposal.ready` |
| `examples/parallel-vendor-security-procurement/` | 供应商引入前的安全、隐私、采购、法务并行审查 | 供应商接入、治理协作 | `vendor.ready` |

## 第六批新增范例

### `parallel-support-escalation-desk`

这个范例更像真实的一线升级处理台。
它不是单纯做故障排查。
它更强调支持、产品、客户经营、沟通四条线同时把信息补齐,最后形成统一升级执行方案。

适合演示:

- 高优先级客户问题如何跨团队收口
- 为什么 coordinator 在 ready 没齐之前必须保持静默
- 为什么 finalizer 明确负责人会更稳

### `parallel-partner-launch-coordination`

这个范例强调的是合作伙伴联合发布。
它和普通产品发布不一样。
这里要同时处理方案使能、条款确认、渠道营销、销售交接。

适合演示:

- 渠道伙伴协作为什么天然适合并行编排
- 多条准备线如何汇总成统一发布资料包
- 如何把跨职能准备状态收敛到一个明确的就绪 topic

### `parallel-field-enablement-rollout`

这个范例展示的是内部一线赋能推广。
它不是面向单个客户。
它面向的是销售、经理、演示环境和认证计划这几条内部准备线。

适合演示:

- 内部赋能工作流也适合用真实并行方式建模
- 推广前的课程、演示环境、经理同步、认证计划如何并行推进
- 为什么固定终态字段特别利于真实后端 E2E 验证

## 第七批新增范例

### `parallel-revops-quote-desk`

这个范例把“报价能不能顺利交给销售”拆成了四条并行输入线。
它不去做售前提案排版。
它更像真正的营收运营报价台,强调结构、定价、账单、条款同时收齐以后,才允许 finalizer 发布统一 quote packet。

适合演示:

- 商业报价为什么适合用 fanout + fanin 建模
- coordinator 静默等待如何避免过早发起 quote request
- 固定的 `quote_status`、`deal_motion`、`pricing_owner` 如何让协议更稳定

### `parallel-executive-business-review-prep`

这个范例更偏经营节奏。
它把高层业务回顾材料拆成营收叙事、产品采纳、风险展望、管理层诉求四条线。
最后再由 `ebr_chief_of_staff` 统一打包成一份 EBR packet。

适合演示:

- 管理层材料准备为什么也能复用同一套并行骨架
- 如何把 narrative、adoption、risk、asks 四种视角收敛成一个 final topic
- 为什么把固定终态字段直接写进协议,会让 live E2E 更稳

### `parallel-customer-advisory-board-prep`

这个范例偏高价值客户活动筹备。
它不是在做普通市场活动。
它更强调客户群体、议程塑形、高层主持准备、物流 readiness 四条线同时收口,最后形成统一确认包。

适合演示:

- 多方准备线如何在一个 `cab.packet.ready` 下收敛
- 高层客户活动为什么需要明确的 final owner
- 固定 region / owner / focus 字段如何帮助回放和验证

## 第八批新增范例

### `parallel-regional-operating-review`

这个范例强调的是单一区域周会前的经营收口。
它不是只看销售 forecast。
它把 pipeline、交付、支持、人才 4 条输入线一起收齐,最后再形成统一周会结论。

适合演示:

- 区域经营周会为什么适合用 fanout + fanin 建模
- 为什么 coordinator 在四条 ready 没齐前必须静默
- 固定的 `review_status`、`region_code`、`operating_owner` 如何让协议更稳

### `parallel-renewal-risk-calibration`

这个范例不是单个客户续约战情室。
它更像续费经营周会上的组合盘 forecast 校准。
这里更强调使用信号、赞助覆盖、商业阻塞、成功计划 4 条风险输入线同时收口。

适合演示:

- 续费组合盘预测为什么也适合复用真实并行骨架
- 如何把四种风险视角收敛到一个 `renewal.calibration.ready`
- 为什么固定 `calibration_status`、`forecast_window`、`forecast_owner` 后,live E2E 更稳

### `parallel-multi-region-pipeline-sync`

这个范例强调的是同一经营主题在多区域同时推进。
它不是单一区域周会。
它更像全球 forecast call 之前的区域口径统一动作。

适合演示:

- 多区域 pipeline 同步为什么天然适合并行编排
- Americas、EMEA、APJ、LATAM 四条 lane 如何汇总成一个 final topic
- 为什么把最脆弱的区域 lane 锁成单行 JSON event,会更利于真实 backend 稳定

## 为什么这些范例值得继续扩

到 batch-8 为止,这组范例已经覆盖:

- 工程协作
- 发布与迁移
- 人工批准
- 风险治理与合规
- 事故响应与复盘
- 财务与招聘
- 客户续约与客户激活
- 支持升级、伙伴协作、一线赋能
- 商业报价、高层经营准备、客户顾问委员会筹备
- 区域经营周会、续费组合盘校准、多区域 pipeline 同步

这说明 Ralph 的并行 example 已经不只是“工程演示”。
它已经可以被看作一套通用协作编排模板库。

## 后续扩批建议

如果后面还要继续扩批,建议继续遵守这 3 条:

1. 不要回到已经高度重复的题材
2. 优先选能写出明确终态字段的真实场景
3. 继续让 finalizer 负责发布最终 topic

当前比较值得考虑的后续方向有:

- 董事会材料预演
- 季度投资组合回顾
- forecast commit 对齐会
- 区域定价例外校准

## 相关文件

- 总体第一批方案: `specs/parallel-real-world-examples.spec.md`
- 第六批方案: `specs/parallel-real-world-examples-batch-6.spec.md`
- 第七批方案: `specs/parallel-real-world-examples-batch-7.spec.md`
- 第八批方案: `specs/parallel-real-world-examples-batch-8.spec.md`
- E2E 入口: `crates/ralph-e2e/src/scenarios/`
- 运行入口: `README.md`
