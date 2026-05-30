# CLOSE_PACKET

你正在组织一次月结前的 finance close control room。
这个 packet 只包含并行判断所需的结构化上下文。

## Close Meta

- close_id: CLOSE-2026-03
- entity: Ralph Cloud Ltd
- period_end: 2026-03-31
- owner_team: finance-ops

## Revenue Packet

- focus: confirm billed revenue reconciles to booked revenue
- expected_status: ready
- expected_revenue_status: reconciled_without_gap
- evidence:
  - billed ARR ties to the booking ledger for the closing month
  - no unresolved credit memo is left open past the cut-off
  - deferred revenue roll-forward matches the subscription schedule

## Expense Packet

- focus: confirm accruals are captured
- expected_status: ready
- expected_expense_status: accruals_booked
- evidence:
  - vendor invoices missing from AP are covered by accrual entries
  - contractor hours through period end are included in the close estimate
  - hosting overage true-up is already staged for posting

## Cash Packet

- focus: confirm bank and treasury position
- expected_status: ready
- expected_cash_status: bank_position_confirmed
- evidence:
  - operating cash position matches treasury worksheet
  - no unreconciled transfer remains pending after the cut-off
  - payroll funding has already cleared for the close window

## Anomaly Packet

- focus: confirm watchlist items are immaterial
- expected_status: ready
- expected_anomaly_status: within_threshold
- evidence:
  - late invoice variance remains below the close materiality threshold
  - no manual journal entry requires controller escalation
  - audit trail exists for the flagged revenue exception

## Expected Final Outcome

- close_status: READY_TO_CLOSE
- materiality: WITHIN_THRESHOLD
- owner: finance-ops
