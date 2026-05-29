## 1. Core recoverable failure model and policy

- [x] 1.1 Add recoverable failure domain types for failure kind, lifecycle status, transition entry, retry policy, and replayed snapshot state.
- [x] 1.2 Add a single SSOT path resolver for `.ralph/recoverable-failures.jsonl`, using the configured workspace root rather than ad-hoc relative paths.
- [x] 1.3 Implement a deterministic classifier that receives HatJobResult-like fields and returns recoverable classification only for the explicit initial patterns.
- [x] 1.4 Add classifier tests for `429 Too Many Requests`, `exceeded retry limit` with temporary last status, ordinary command failures, timeout-only failures, and cancellation-only failures.
- [x] 1.5 Add retry policy config with defaults for enabled flag, max attempts, initial delay, backoff multiplier, and max delay.
- [x] 1.6 Add config parsing/default tests covering omitted config, disabled retry, custom bounded retry values, and invalid policy values.

## 2. Append-only ledger and replay snapshot

- [x] 2.1 Implement append-only JSONL writing for recoverable failure transitions with stable `failure_id`, job/instance/hat correlation, process outcome, retry timing, and bounded stderr excerpt.
- [x] 2.2 Implement ledger replay that derives the latest recoverable failure snapshot by replaying transitions in order.
- [x] 2.3 Add ledger tests for append ordering, multiple transitions for one `failure_id`, malformed line handling, and bounded stderr excerpt behavior.
- [x] 2.4 Ensure the ledger stores compact metadata only and does not duplicate full job prompts or raw event streams.

## 3. Parallel runtime retry lifecycle

- [x] 3.1 Wire parallel job failure handling to classify failed HatJobResult values after execution and before ordinary terminal failure handling.
- [x] 3.2 Introduce retry-aware job/instance states for `retry_scheduled`, `paused_recoverable`, `retrying`, `exhausted`, and `continued_by_human` where they are visible to the supervisor lifecycle.
- [x] 3.3 Implement retry scheduling with bounded backoff and an injectable clock or deterministic test clock.
- [x] 3.4 Re-enqueue retry attempts using the runtime-held job context, while keeping the ledger as metadata/correlation evidence only.
- [x] 3.5 Prevent coordinator-controlled completion while recoverable failures remain pending, paused, scheduled, or retrying.
- [x] 3.6 Convert exhausted recoverable failures into terminal job failures with a ledger evidence pointer.
- [x] 3.7 Preserve the stdout-only workflow event parsing invariant: stderr may be classified for retryability but MUST NOT enter `output_for_parsing`.

## 4. Manual continue control path

- [x] 4.1 Extend Supervisor chat parsing with explicit `!continue` and `!continue <failure_id>` control intent.
- [x] 4.2 Add parser tests proving `!continue` is a control action and plain text such as `继续分析这个问题` remains ordinary chat.
- [x] 4.3 Resolve explicit `failure_id` continue requests against the recoverable failure snapshot and reject unknown or terminal failures with a visible/auditable error.
- [x] 4.4 Resolve bare `!continue` only when the selected instance or selected recoverable failure is unambiguous; otherwise surface ambiguity instead of retrying silently.
- [x] 4.5 Append `continued_by_human` before enqueueing a manual retry, and route manual retry through the same scheduler path as delayed retry.

## 5. Human-facing evidence and observability

- [x] 5.1 Surface recoverable failure summary fields in runtime observability without making `.ralph/recoverable-failures.jsonl` a prompt or event-store duplicate.
- [x] 5.2 Ensure `ralph agents` or the agents snapshot can explain retry-aware / exhausted state with `failure_id`, affected instance, attempt, and next retry timing where available.
- [x] 5.3 Ensure record-session summaries or evidence inspection can point to recoverable failure ledger evidence for scheduled, continued, and exhausted retries.
- [x] 5.4 Add tests or fixtures proving recoverable retry state is observable after a failed attempt, after manual continue, and after exhaustion.

## 6. Integration guardrails and final validation

- [x] 6.1 Add a fake/custom backend or executor fixture that fails first with `ERROR: exceeded retry limit, last status: 429 Too Many Requests` and then succeeds after retry.
- [x] 6.2 Add an integration guardrail for automatic delayed retry with a small deterministic delay or injected clock.
- [x] 6.3 Add an integration guardrail for manual `!continue` retry.
- [x] 6.4 Add an integration guardrail for exhausted recoverable failures becoming terminal with ledger evidence.
- [x] 6.5 Run `OPENSPEC_TELEMETRY=0 DO_NOT_TRACK=1 openspec validate agent-cli-recoverable-failure-retry --type change --strict`.
- [x] 6.6 Run focused Rust tests for classifier, ledger, scheduler, parallel lifecycle, and Supervisor continue parsing.
- [x] 6.7 Run replay smoke tests relevant to parallel runtime and event parsing.
- [x] 6.8 Run full `cargo test` before declaring implementation complete.
