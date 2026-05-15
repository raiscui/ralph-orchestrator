# Design: live-runtime-capability-invocation

## Context

Current state:

- Phase 3 writes isolated capability invocation artifacts and evidence index entries.
- Phase 3.1 adds `ralph tools capability inspect <invocation_id>` for evidence lookup.
- The remaining gap is parent-run integration: a live run can produce ordinary events, but there is not yet a stable runtime action that takes a capability request from `ralph#1`, runs the isolated invocation, and returns a parent-consumable result.

The important invariant stays unchanged: the parent run topology is fixed after startup. Capability invocation is an isolated side run, not a topology mutation.

## Goals / Non-Goals

**Goals:**

- Let a parent run request a capability invocation through a structured runtime signal.
- Execute the selected capability through the existing isolated child/micro-run path.
- Emit a parent-visible result/failure event with invocation id and artifact references.
- Keep `.ralph/evidence-index.jsonl` and child artifacts as durable audit surfaces.
- Dogfood the result with Phase 3.1 `inspect`.

**Non-Goals:**

- No hot mutation of parent `HatRegistry`, parent config, or event-loop topology.
- No full asynchronous job scheduler.
- No external LLM E2E requirement.
- No generic function-calling system.
- No broad `ralph evidence` command beyond the existing capability inspect UX.

## Decisions

### Decision 1: Use a structured event as the parent-run request surface

Introduce a narrow request event, for example `capability.request`, whose JSON payload includes:

- `request_id`
- `capability_id`
- `input`

Rationale:

- Ralph already treats events as orchestration signals.
- A structured event is easy to test with deterministic custom backends.
- It avoids scraping prose for tool calls.

Rejected alternative:

- Parse arbitrary natural-language text from `ralph#1`. That would make the trigger ambiguous and hard to test.

### Decision 2: Reuse the existing isolated invocation implementation

The request handler should call the same underlying isolated invocation path as `ralph tools capability invoke`.

Rationale:

- Artifact writing, evidence recording, failure behavior, and parent topology flags already exist.
- Reuse prevents a second child-run broker.

Implementation note:

- If current functions are private to `capability.rs`, expose a small internal API rather than duplicating invocation logic.

### Decision 3: Return result/failure to the parent run as structured events

On success, emit a parent-visible event such as `capability.result` containing:

- `request_id`
- `invocation_id`
- `capability_id`
- result summary
- artifact paths
- `parent_topology_unchanged=true`

On failure, emit `capability.failed` with equivalent correlation and artifact references.

Rationale:

- Existing topics already exist for capability lifecycle artifacts.
- Parent-run consumers need an event-level contract, not only files on disk.

### Decision 4: Synchronous v1 handling is acceptable

Phase 4 can handle the request synchronously inside the parent run output processing path.

Rationale:

- This keeps v1 deterministic and easy to test.
- A full async scheduler is premature.

Trade-off:

- The parent run waits while the isolated invocation runs. This is acceptable for v1 and deterministic dry-run dogfood.

## Risks / Trade-offs

- [Risk] Event handling may accidentally reprocess the same request on replay or repeated parsing. → Mitigation: track handled request ids in the parent run process and only invoke once per request id.
- [Risk] Reusing existing `capability.result` topic for both child lifecycle and parent-return semantics could confuse tests. → Mitigation: include `request_id` when the result is parent-returned, and assert invocation id + artifact paths.
- [Risk] Failed invocations may have partial artifacts. → Mitigation: return failure event with invocation id when available and rely on evidence index / inspect UX for audit.
- [Risk] The first implementation may expose a CLI-only helper awkwardly. → Mitigation: keep the API internal to `ralph-cli` first, then promote to core only if multiple crates need it.
