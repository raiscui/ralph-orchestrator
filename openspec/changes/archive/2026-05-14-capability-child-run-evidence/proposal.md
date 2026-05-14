# Change: capability-child-run-evidence

## Why

Phase 3 should connect runtime capability invocation to Ralph's durable evidence index.

`ralph tools capability invoke` already writes isolated child/micro-run artifacts:

- `invoke.json`
- `result.json` or `failed.json`
- `resolved-config.yml`
- `.ralph/events.jsonl` records for `capability.invoke`, `capability.result`, or `capability.failed`

The current gap is lookup. An operator can inspect those files manually, but cannot ask the evidence index for all durable artifacts associated with a capability invocation id.

This change closes the smallest useful loop: capability invocation remains isolated, parent topology stays stable, and every child/micro-run artifact becomes discoverable through `.ralph/evidence-index.jsonl`.

## What Changes

- Extend the `capability-invocation` contract so invocation, result/failure, resolved config, and event-log artifacts are registered in the evidence index.
- Add focused CLI tests that run `ralph tools capability invoke` and query `.ralph/evidence-index.jsonl` by invocation id.
- Preserve the existing isolated execution model:
  - no live topology mutation
  - no registry injection
  - no external backend E2E requirement for v1
- Keep `.ralph/events.jsonl` and child artifacts as truth sources; evidence index only links to them.

## Capabilities

### Modified Capabilities

- `capability-invocation`: Adds durable evidence-index linkage for isolated child/micro-run artifacts.

### Related Capabilities

- `runtime-evidence-index-kernel`: Reuses the existing evidence index writer and artifact kind vocabulary.

## Impact

Expected touchpoints:

- `crates/ralph-cli/src/capability.rs`
- `crates/ralph-cli/tests/integration_capability.rs`
- `openspec/specs/capability-invocation/spec.md` may need its archived `Purpose TBD` corrected when this change is later archived or synced.

This change must not:

- mutate parent runtime topology
- turn capability invocation into a general request broker
- treat evidence index entries as replacement artifacts
- require live external LLM backends
