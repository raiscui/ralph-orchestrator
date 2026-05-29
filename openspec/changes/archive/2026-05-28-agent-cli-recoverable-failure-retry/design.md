## Context

Ralph parallel mode executes hat jobs through external agent CLI processes.
A job currently returns a `HatJobResult` with stdout-only parsing output, captured stderr, exit status, timeout and cancellation flags.
This is enough to decide generic success or failure, but it is not enough to distinguish terminal failures from temporary CLI backend failures such as rate limits:

```text
ERROR: exceeded retry limit, last status: 429 Too Many Requests
```

When this happens today, the runtime can show a failed job, but it does not persist a machine-readable retry state.
The user also cannot say "继续" in a way that resumes the paused recoverable job without manually reconstructing the original task.

This change must preserve two existing boundaries:

- stderr is observable evidence, but it MUST NOT be parsed as workflow events.
- retry state is runtime evidence and scheduling metadata, not a second copy of the full job prompt or event stream.

## Goals / Non-Goals

**Goals:**

- Persist recoverable agent CLI failures under a single append-only `.ralph/` data file.
- Classify a narrow deterministic set of recoverable failures from structured job result fields and stderr excerpts.
- Record retry attempts, retry delay, next retry time, current state, and manual-continue transitions.
- Allow delayed retry and explicit human continue without losing the original job context.
- Keep terminal failures terminal unless they match an explicit recoverable classifier.
- Provide focused tests for classification, persistence, scheduler decisions, and manual continue routing.

**Non-Goals:**

- Do not build a general-purpose distributed retry platform.
- Do not retry every non-zero exit.
- Do not parse stderr as workflow event output.
- Do not introduce UI-heavy retry management in the first implementation.
- Do not require live backend calls to test classification and scheduling.

## Decisions

### Decision 1: Use one append-only recoverable failure ledger

Write a single JSONL sidecar at:

```text
.ralph/recoverable-failures.jsonl
```

Each line records one state transition rather than rewriting a mutable state file.
A derived snapshot can be computed by replaying the ledger.

Minimum transition fields:

- `schema_version`
- `failure_id`
- `job_id`
- `instance_id`
- `hat_id`
- `backend_kind`
- `failure_kind`
- `status`
- `attempt`
- `max_attempts`
- `retry_after_ms`
- `next_retry_at`
- `exit_code`
- `timed_out`
- `canceled`
- `stderr_excerpt`
- `created_at`
- `source_event_ids` or equivalent job input correlation ids when available

Rationale:

- Append-only JSONL matches existing evidence patterns such as events and record-session.
- It preserves auditability across automatic retry and manual continue.
- It avoids a second source of truth for prompts or event payloads.

Alternatives considered:

- Mutable JSON snapshot only: rejected because it loses transition history.
- Reusing `.ralph/events.jsonl` only: rejected because retry scheduling needs a compact machine-readable ledger that can be inspected without replaying every event.

### Decision 2: Classifier is deterministic and narrow

Recoverable classification should be a pure helper that receives `HatJobResult`-like fields and returns an optional `RecoverableFailureKind`.

Initial recoverable kinds:

- `rate_limited`: stderr or structured error contains `429 Too Many Requests`.
- `retry_limit_exceeded`: stderr or structured error contains `exceeded retry limit` and the last status indicates a temporary class such as 429.
- `transient_network`: only if the stderr matches a deliberately curated network-transient pattern.

Everything else remains terminal.
Timeout and cancellation are not automatically recoverable unless a later spec explicitly adds that behavior.

Rationale:

- A narrow classifier prevents retry loops for real code failures.
- It makes tests deterministic and cheap.
- It keeps failure semantics visible to the user.

Alternatives considered:

- Let the LLM classify failures: rejected because retry decisions must be deterministic.
- Match any stderr containing `retry`: rejected because it would retry terminal application errors with misleading messages.

### Decision 3: Retry scheduling is explicit policy, not hidden executor behavior

Add a small runtime policy with defaults such as:

```yaml
agent_cli_recoverable_failures:
  enabled: true
  max_attempts: 3
  initial_delay_ms: 30000
  backoff_multiplier: 2.0
  max_delay_ms: 300000
```

The scheduler computes `next_retry_at` and records it to the ledger.
A retry attempt reuses the existing job context held by the parallel instance/supervisor runtime, but the ledger only stores correlation ids and compact metadata.

Rationale:

- Operators can understand when a retry is expected.
- Tests can run with very small delays or an injected clock.
- The executor stays focused on running a single process; retry belongs to the supervisor/instance lifecycle.

Alternatives considered:

- Retry inside the CLI executor: rejected because the supervisor and TUI would not see paused/retryable state transitions.
- Always require manual continue: rejected because rate limits often become recoverable after a fixed delay.

### Decision 4: Recoverable failure introduces a paused/retryable lifecycle state

A recoverable failure should not immediately collapse the instance/job into ordinary terminal failure.
The runtime should expose a state such as:

- `retry_scheduled`
- `paused_recoverable`
- `retrying`
- `recovered`
- `exhausted`
- `continued_by_human`

When attempts are exhausted, the failure becomes terminal with evidence pointing to the retry ledger.
When a retry attempt succeeds, the lifecycle appends `recovered` so ledger replay and completion gating can prove that the retryable condition has resolved.

Rationale:

- The user can see the difference between "job is dead" and "job is waiting to retry".
- Human continue has a well-defined target.
- Existing completion logic can avoid declaring done while retryable jobs remain pending.

### Decision 5: Manual continue is a control action, not ordinary chat

Supervisor human input should support an explicit continue action that targets a recoverable failure id or selected paused instance.
It should append a ledger transition and enqueue the retry through the same scheduler path.

Possible command surface:

```text
!continue
!continue <failure_id>
```

Rationale:

- Ordinary chat text should not accidentally retry a failed CLI process.
- The command is easy to trigger when the user says "继续" in the TUI or control path.
- It keeps user intent auditable.

Alternatives considered:

- Treat any human message containing "continue" as retry: rejected because it is ambiguous and language-dependent.

## Risks / Trade-offs

- [Risk] Retrying can amplify rate limits.
  Mitigation: bounded attempts, backoff, and explicit `next_retry_at` evidence.

- [Risk] Stderr text matching can become brittle.
  Mitigation: keep patterns narrow, fixture the exact known examples, and prefer structured backend fields if available.

- [Risk] Retrying stale job context may duplicate side effects.
  Mitigation: first implementation should retry only jobs that failed before producing parseable workflow result events, and record attempt transitions.

- [Risk] Manual continue could be confused with ordinary chat.
  Mitigation: use explicit command/control action and record `continued_by_human` in the ledger.

- [Risk] The ledger could become a second prompt store.
  Mitigation: store compact metadata and correlation ids only; keep prompts/events in existing artifacts.

## Migration Plan

1. Add recoverable failure data types, classifier, and ledger writer/reader.
2. Add config defaults and tests for disabled/enabled behavior.
3. Wire parallel job failure handling to classify failures and record ledger transitions.
4. Add retry scheduler behavior with injected clock or small test delay.
5. Add manual continue command/control path.
6. Add focused integration guardrail using a custom backend that fails with a 429-like stderr on the first attempt and succeeds after retry/continue.
7. Run OpenSpec validation, focused Rust tests, smoke tests, and a preserved evidence run if runtime behavior is touched.

## Open Questions

- Should the first implementation include automatic delayed retry, manual continue, or both in the same patch?
- Should retry policy live under `parallel`, `cli`, or a top-level `agent_cli_recoverable_failures` config namespace?
- Should exhausted recoverable failures be surfaced in `ralph agents` as a separate state or only in the ledger and TUI status line?
