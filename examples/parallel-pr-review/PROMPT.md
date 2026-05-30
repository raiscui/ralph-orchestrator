# PR_REVIEW_PACKET

你正在审查一个“支付请求签名校验”相关 PR。
这个 packet 不是完整 diff。
它是给并行 reviewer 使用的结构化上下文。

## PR Meta

- pr_id: PR-418
- title: add signed request verification to payout webhook
- changed_files:
  - `src/http/webhook_handler.rs`
  - `src/security/signature.rs`
  - `tests/webhook_handler_test.rs`

## Change Summary

- webhook handler 现在会校验签名,再写入 payout request
- 新增了请求编号 `request_id`
- 补了单元测试,但没有集成测试

## Correctness Packet

- focus: request_id 分配时机与重试行为
- expected_approval: conditional
- expected_issue: reserve request_id before response write to avoid duplicate retry window
- evidence:
  - handler 在响应成功后才递增 `request_id`
  - 若外部重试发生在响应边界,相同编号可能被再次消费

## Security Packet

- focus: 签名比较是否存在时间侧信道
- expected_approval: approved
- expected_issue: none
- evidence:
  - `signature.rs` 使用 constant-time helper
  - packet 中没有出现跳过验证的 fallback 路径

## Architecture Packet

- focus: handler 职责是否过重
- expected_approval: conditional
- expected_issue: move payload validation and audit persistence behind a service boundary
- evidence:
  - `webhook_handler.rs` 同时做了解析、校验、审计写入
  - 这些职责后续还会被其他入口复用

## Expected Final Outcome

- final_verdict: REQUEST_CHANGES
- required_fixes:
  - reserve request id earlier
  - split validation / persistence responsibilities
