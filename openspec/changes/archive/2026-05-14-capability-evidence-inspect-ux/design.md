# Design: capability-evidence-inspect-ux

## Context

Phase 3 connected isolated capability invocation artifacts to `.ralph/evidence-index.jsonl`.
The write path now produces durable artifacts, event log entries, and evidence-index links.

The missing piece is a small read path. Today, an operator has to manually open JSONL and correlate entries by invocation id. That is bad ergonomics for humans, and brittle for agents that need a stable inspection surface before Phase 4 starts invoking capabilities from live parent runs.

Current relevant APIs:

- `crates/ralph-cli/src/capability.rs`
  - owns `ralph tools capability list|summaries|invoke`
  - already knows capability invocation terminology and artifact layout
- `crates/ralph-core/src/evidence_index.rs`
  - owns `EvidenceIndexReader::find_by_correlation(...)`
  - returns `Entries`, `Missing`, or `NoEntry`

## Goals / Non-Goals

**Goals:**

- Add a focused CLI UX for inspecting capability invocation evidence by invocation id.
- Reuse the existing evidence index reader and `.ralph/evidence-index.jsonl` path.
- Provide both human-readable and JSON output.
- Fail visibly when the invocation id has no entry.
- Keep artifact/event logs as the durable truth source.

**Non-Goals:**

- Do not add a broad `ralph evidence` subsystem in this phase.
- Do not implement Phase 4 live runtime invocation here.
- Do not mutate parent topology or child artifacts.
- Do not add a database, cache, or second index.
- Do not validate artifact contents deeply; Phase 3.1 only locates and reports linked artifacts.

## Decisions

### Decision 1: Add `ralph tools capability inspect <invocation_id>`

Use the existing `tools capability` command group.

Rationale:

- The UX is specifically for capability invocation evidence.
- It avoids prematurely turning the minimal evidence kernel into a broad diagnostic platform.
- It keeps command discovery close to `list`, `summaries`, and `invoke`.

Rejected alternative:

- `ralph evidence lookup <id>` was deferred. It may become useful later, but it would broaden Phase 3.1 beyond capability invocation and repeat the overgrowth risk called out in the Phase 1A evidence-kernel boundary.

### Decision 2: Treat missing invocation evidence as a command failure

`NoEntry` should return a non-zero exit code with a clear message.

Rationale:

- A lookup command is normally used as a verification gate.
- Returning success with an empty report would make automation falsely pass.

Missing marker records are different: if the reader returns `EvidenceLookup::Missing`, the command should report that explicit missing evidence with status `missing`, because the index did contain a durable missing marker.

### Decision 3: JSON output is the automation contract

Human output should be concise and readable. JSON output should be stable enough for tests and agents.

The JSON shape should include at least:

- `invocation_id`
- `status`: `entries` or `missing`
- `entries`: evidence index entries as serialized core records
- optionally `index_path` for debugging path confusion

Rationale:

- Reusing core entry serialization avoids a second schema.
- Including the index path reduces confusion when commands run from the wrong workspace.

### Decision 4: Artifact path existence checks are optional and non-authoritative

The command may show artifact paths. It should not claim artifact content validity unless it actually opens and validates those files.

Rationale:

- The evidence index is a lookup surface.
- The artifacts and event logs remain the truth source.
- Deep artifact validation belongs in a later doctor/audit command, not Phase 3.1.

## Risks / Trade-offs

- [Risk] A focused `tools capability inspect` command may later need to share behavior with a generic evidence lookup command. → Mitigation: keep formatting helpers small and local, but do not hide core reader semantics behind a new abstraction yet.
- [Risk] Human output can drift from JSON output. → Mitigation: integration tests should use JSON output as the contract and only lightly assert human output if needed.
- [Risk] Users may pass a capability id instead of an invocation id. → Mitigation: error wording should say "invocation id / correlation id" and mention `.ralph/evidence-index.jsonl`.
