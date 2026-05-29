# agent-cli-recoverable-failure-retry Specification

## Purpose
TBD - created by archiving change agent-cli-recoverable-failure-retry. Update Purpose after archive.
## Requirements
### Requirement: Recoverable failure ledger
The system MUST record recoverable agent CLI failure state transitions in a single append-only JSONL ledger at `.ralph/recoverable-failures.jsonl`.

The ledger MUST store compact machine-readable evidence and retry metadata, not full prompts or a duplicate event stream. Each transition entry MUST include enough correlation data to identify the affected job and instance, including a stable `failure_id`, `job_id`, `instance_id`, `hat_id`, `backend_kind`, `failure_kind`, `status`, `attempt`, `max_attempts`, retry timing fields, process outcome fields, and a bounded stderr excerpt.

#### Scenario: Recoverable failure is appended to the ledger
- **WHEN** an agent CLI job exits non-zero with stderr containing a classified recoverable failure such as `ERROR: exceeded retry limit, last status: 429 Too Many Requests`
- **THEN** the system MUST append a JSONL transition to `.ralph/recoverable-failures.jsonl`
- **THEN** the transition MUST include `failure_kind`, `status`, `attempt`, `max_attempts`, `retry_after_ms` or `next_retry_at`, and the affected `job_id` and `instance_id`

#### Scenario: Ledger replay derives the latest failure snapshot
- **WHEN** multiple transitions exist for the same `failure_id`
- **THEN** the system MUST derive the current recoverable-failure state by replaying the append-only ledger in order

### Requirement: Deterministic recoverable failure classification
The system MUST classify recoverable agent CLI failures using a deterministic and narrow classifier based on structured job-result fields and bounded stderr evidence.

The initial classifier MUST treat `429 Too Many Requests` as `rate_limited`, and MUST treat `exceeded retry limit` as recoverable only when the associated last status indicates a temporary class such as 429. The system MUST NOT ask an LLM to classify retryability, and MUST NOT classify every non-zero exit as recoverable.

#### Scenario: Known 429 retry-limit stderr is recoverable
- **WHEN** a job result has `success=false` and stderr contains `exceeded retry limit` plus `429 Too Many Requests`
- **THEN** the classifier MUST return a recoverable failure classification with kind `retry_limit_exceeded` or `rate_limited`

#### Scenario: Ordinary command failure remains terminal
- **WHEN** a job result has `success=false` and stderr contains an ordinary command or build error that does not match an explicit recoverable pattern
- **THEN** the classifier MUST return no recoverable classification

### Requirement: Retry scheduling policy
The system MUST schedule retries for classified recoverable failures through an explicit bounded retry policy.

The policy MUST include enablement, maximum attempts, initial delay, backoff multiplier, and maximum delay. For each scheduled retry, the system MUST record the computed attempt number and next retry timing in the recoverable failure ledger.

#### Scenario: Retry is scheduled with bounded backoff
- **GIVEN** recoverable failure retry is enabled with `max_attempts=3`
- **WHEN** the first classified recoverable failure is recorded for a job
- **THEN** the system MUST append a transition with status `retry_scheduled`
- **THEN** the transition MUST include `attempt=1`, `max_attempts=3`, and a bounded `next_retry_at` or `retry_after_ms`

#### Scenario: Retry success closes the recoverable lifecycle
- **GIVEN** a recoverable failure has a scheduled retry attempt
- **WHEN** the retry attempt succeeds
- **THEN** the system MUST append a transition with status `recovered`
- **THEN** ledger replay MUST treat that lifecycle as resolved rather than pending

#### Scenario: Exhausted attempts become terminal recoverable evidence
- **GIVEN** a recoverable failure has reached its configured `max_attempts`
- **WHEN** another matching recoverable failure occurs for the same retry lifecycle
- **THEN** the system MUST append a transition with status `exhausted`
- **THEN** the affected job MUST be allowed to become terminal with evidence pointing to the ledger entry

### Requirement: Manual continue retry control
The system MUST allow a paused recoverable failure to be retried through an explicit human continue control action.

The continue action MUST target either a specific `failure_id` or the currently selected paused recoverable failure. A manual continue MUST append an auditable ledger transition before enqueueing the retry through the same retry path used by scheduled retry.

#### Scenario: Human continues a specific recoverable failure
- **GIVEN** a recoverable failure ledger contains a paused or scheduled failure with `failure_id="failure-123"`
- **WHEN** the human submits an explicit continue control action targeting `failure-123`
- **THEN** the system MUST append a transition with status `continued_by_human`
- **THEN** the system MUST enqueue a retry for the original job context associated with `failure-123`

#### Scenario: Continue does not reconstruct the job manually
- **GIVEN** a recoverable failure references an original job context by correlation ids
- **WHEN** a manual continue retry is accepted
- **THEN** the retry MUST reuse the runtime-held job context for that job lifecycle
- **THEN** the recoverable failure ledger MUST NOT store a duplicate copy of the full prompt or event stream

### Requirement: Terminal failure boundary
The system MUST keep terminal failures terminal unless they match an explicit recoverable classifier and retry policy allows retry.

Timeouts, cancellations, application errors, and non-zero process exits MUST NOT become retryable solely because they failed. Stderr MUST remain observable failure evidence and MUST NOT be parsed as workflow event output.

#### Scenario: Unclassified stderr is not parsed as events
- **WHEN** a failed agent CLI process writes text resembling workflow markup to stderr but does not match a recoverable failure classifier
- **THEN** the system MUST NOT parse that stderr as workflow events
- **THEN** the system MUST treat the job as an ordinary terminal failure

#### Scenario: Retry disabled keeps recoverable evidence terminal
- **GIVEN** recoverable failure retry policy is disabled
- **WHEN** a job fails with a classified recoverable failure
- **THEN** the system MUST append recoverable failure evidence if ledger recording is enabled
- **THEN** the system MUST NOT schedule or enqueue a retry
