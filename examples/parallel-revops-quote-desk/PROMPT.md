# QUOTE_PACKET

你正在准备一个营收运营报价台的资料包。
这个 packet 仅承载并行扇出所需的结构化上下文,不包含任何模型执行指令。

## Deal Structure Packet

- focus: 确认当前扩张/追加业务的 deal structure 合理、账户目标清楚
- expected_status: ready
- highlights:
  - 本次报价冲刺基于已有增长承诺,复用销售线的既有条款
  - 打算在下个交付窗口交付 PoC + 成熟方案
  - 负责 team 已确认资源与交付节奏

## Pricing Guardrail Packet

- focus: 确保报价遵循内部 pricing guardrail,没有超标折扣
- expected_status: ready
- guardrail_alignment: pricing owner 已审核、毛利保持合理
- notes:
  - 工程复盘指出定价 cap 为 42% GM
  - 销售复核文档记录了审批流程与签字人

## Billing Setup Packet

- focus: 验证账单设置、收款节点与客户财务对齐
- expected_status: ready
- billing_notes:
  - billing owner 与 finance 已确认 invoice frequency
  - 已铺设 revrec 路径,确认 WIP length

## Commercial Terms Packet

- focus: 核对商业条款、一致性承诺与合同约定
- expected_status: ready
- terms_snapshot:
  - 客户同意 annual commitment + performance SLA
  - 新的 payment milestone 由 customer success 担当
  - 交付条款里有定级 escalation clause

## Expected Quote Packet

- quote_status: READY_FOR_SELLER_HANDOFF
- deal_motion: EXPANSION_UPSELL
- pricing_owner: revops-desk
- pricing_approval: revops-brand-team
- quote_owner: revops-quote-desk
