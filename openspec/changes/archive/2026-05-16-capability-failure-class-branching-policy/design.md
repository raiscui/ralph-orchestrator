# Design: capability-failure-class-branching-policy

## Context

Current failure handling already provides:

- parent-visible `capability.failed`
- request id, capability id, optional invocation id, optional artifact links
- a repo-native proof that the parent can continue with a fallback capability step

The remaining instability is semantic: the parent must still infer *what kind* of failure happened unless the runtime provides a structured class.

## Goal

Provide a bounded v1 structured failure classifier for parent-visible `capability.failed`, then prove parent fallback policy can branch on that class.

## Chosen surface

Add `failure_class` to:

- `CapabilityParentFailedRecord`
- `CapabilityFailedRecord`

### Initial v1 classes

- `invalid_capability_id`
- `malformed_request`
- `child_run_failed`
- `invoker_unavailable`
- `other`

## Branching policy boundary

v1 does **not** introduce a generic branching engine.

Instead, it establishes a stable parent-policy input:
- the runtime classifies failures
- the parent reads `failure_class`
- the parent chooses the next step explicitly

## Dogfood shape

Reuse the existing failure-fallback gate, but strengthen it:

1. turn 1 emits an invalid capability id
2. runtime returns `capability.failed` with `failure_class=invalid_capability_id`
3. turn 2 prompt must contain that structured class
4. parent emits fallback capability request
5. final human-facing reply remains explicit

## Non-goals

- No generalized retry planner
- No probabilistic error parsing in the parent
- No automatic fallback selection engine
- No taxonomy expansion beyond the bounded v1 classes

## Risks and mitigations

1. **Failure class drift**
   - Risk: different runtime paths classify the same failure inconsistently
   - Mitigation: centralize obvious mappings and add focused tests for known classes

2. **Class field without policy usage**
   - Risk: the field exists but parent flows still rely on error text
   - Mitigation: strengthen the live gate to require `invalid_capability_id` in the parent prompt before fallback

3. **Overdesigning taxonomy too early**
   - Risk: too many classes before product need is clear
   - Mitigation: keep v1 small and explicit
