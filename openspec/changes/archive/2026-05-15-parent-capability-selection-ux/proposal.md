## Why

Phase 4 lets a live parent run invoke a capability when `ralph#1` emits a structured `capability.request`. That proves the execution and evidence chain. The remaining product gap is selection: `ralph#1` should not rely on hardcoded hidden knowledge of capability ids or request JSON.

Ralph needs a parent-visible capability catalog / selection surface that tells `ralph#1` which capabilities are callable, what each one is for, and exactly how to request one. This must preserve the Phase 4 invariant: selection metadata is startup/runtime context, not live topology mutation.

## What Changes

- Add a parent-side capability selection UX for `ralph#1`.
- Expose a bounded, structured capability catalog summary to the parent coordinator.
- The catalog MUST include capability id, kind, description/summary, invocation input guidance, and the structured `capability.request` event contract.
- The catalog MUST be generated from structured capability metadata, not YAML comments.
- The parent run MUST still execute selected capabilities through the existing isolated child/micro-run invocation path.
- The parent topology MUST remain unchanged; no invoked capability may be injected into the live parent `HatRegistry`.
- Dogfood validation MUST prove that a deterministic parent coordinator can read the catalog, choose a listed capability id, emit `capability.request`, receive `capability.result`, and inspect the invocation evidence.

## Capabilities

### New Capabilities

### Modified Capabilities
- `capability-invocation`: Add parent-side capability catalog / selection UX requirements for live parent runs.

## Impact

- Runtime prompt/context construction for `ralph#1` in parallel mode.
- Capability catalog / summary generation in CLI/core boundary code.
- Existing Phase 4 runtime invocation hook should be reused, not duplicated.
- Tests should remain deterministic and avoid external LLM dependency.
- No live topology mutation, no generic tool-calling framework, and no reliance on YAML comments as runtime metadata.
