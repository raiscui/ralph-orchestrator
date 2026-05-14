## Why

Phase 3 made isolated capability invocations write durable artifacts and evidence-index entries, but humans and agents still have to inspect `.ralph/evidence-index.jsonl` manually to answer a simple question: "what did this invocation produce?"

Phase 3.1 adds a small, purpose-built inspect UX so capability invocation evidence can be verified from the CLI before Phase 4 wires live runtime invocation into parent runs.

## What Changes

- Add a capability-focused inspect command, scoped to invocation evidence lookup.
- The command MUST read the existing `.ralph/evidence-index.jsonl` file instead of creating a second evidence store.
- The command MUST look up entries by capability invocation id / correlation id.
- The command MUST support both human-readable output and `--json` output.
- The command MUST fail clearly when the invocation id has no evidence entry.
- The command MUST preserve the durable truth-source boundary: artifacts and event logs remain the truth source; the inspect command is only a lookup/reporting surface.
- Phase 4 live runtime capability invocation is intentionally not part of this change.

## Capabilities

### New Capabilities

### Modified Capabilities
- `capability-invocation`: Add a CLI inspect UX requirement for locating capability invocation evidence entries and their durable artifact paths.

## Impact

- CLI surface: `crates/ralph-cli/src/capability.rs`, likely under `ralph tools capability inspect <invocation_id>`.
- Tests: `crates/ralph-cli/tests/integration_capability.rs` and focused unit tests around evidence lookup formatting/error behavior.
- Specs: `openspec/specs/capability-invocation/spec.md` will receive the archived requirement delta after completion.
- No new runtime topology mutation, no new child-run broker, and no new external dependency.
