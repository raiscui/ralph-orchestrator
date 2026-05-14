# Design: capability-child-run-evidence

## Current State

`ralph tools capability invoke` currently follows this path:

```text
invoke_capability()
  -> choose_capability()
  -> invoke_isolated()
  -> invoke_isolated_with_runner()
```

`invoke_isolated_with_runner()` already creates:

- `.ralph/capability-invocations/<invocation_id>/resolved-config.yml`
- `.ralph/capability-invocations/<invocation_id>/invoke.json`
- `.ralph/capability-invocations/<invocation_id>/result.json` on success
- `.ralph/capability-invocations/<invocation_id>/failed.json` on failure
- `.ralph/events.jsonl` records for lifecycle topics

The parent `ralph.yml` is intentionally left unchanged.

## Proposed Design

Add evidence-index recording directly inside the existing invocation path.

### Evidence entries

For every invocation:

1. `CapabilityInvokeJson`
   - `correlation_id`: invocation id
   - `artifact_path`: `invoke.json`
   - `status`: `success`
   - `producer`: `capability`

2. `ResolvedConfig`
   - `correlation_id`: invocation id
   - `artifact_path`: `resolved-config.yml`
   - `status`: `success`
   - `producer`: `capability`

3. `EventLogJsonl`
   - `correlation_id`: invocation id
   - `artifact_path`: `.ralph/events.jsonl`
   - `status`: `success`
   - `producer`: `capability`

On successful child/micro-run:

4. `CapabilityResultJson`
   - `correlation_id`: invocation id
   - `artifact_path`: `result.json`
   - `status`: `success`
   - `producer`: `capability`

On failed child/micro-run:

4. `CapabilityFailedJson`
   - `correlation_id`: invocation id
   - `artifact_path`: `failed.json`
   - `status`: `failure`
   - `producer`: `capability`

### Ordering

The implementation should write the actual artifact first, then register the evidence entry. This preserves the invariant that an index entry points to an existing durable artifact.

### Failure behavior

If evidence-index recording fails after the child artifact is written, the CLI invocation should fail rather than silently claiming an auditable invocation. Evidence durability is part of this feature's contract.

## Non-Goals

- No live topology mutation.
- No new child-run broker.
- No external backend requirement.
- No UI or CLI evidence-inspection command in this phase.
- No attempt to deduplicate all runtime artifact writers into one framework.

## Risks

- Duplicating small evidence-writing calls in CLI code can grow noisy. The mitigation is a small local helper in `capability.rs`, not a new cross-runtime abstraction.
- Timestamp-based invocation ids may collide in very fast tests. Existing code already uses millisecond timestamps; Phase 3 does not change that risk.
