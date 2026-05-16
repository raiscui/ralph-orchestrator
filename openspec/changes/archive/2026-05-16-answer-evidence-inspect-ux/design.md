# Design: answer-evidence-inspect-ux

## Context

The runtime already writes answer-return evidence for explicit `reply.hat.message` requester-return flows:

- request id -> reply event / runtime delivery evidence
- answer id -> event log evidence
- missing or fail-closed paths -> explicit missing/failure evidence

The current read path is only available through low-level code (`EvidenceIndexReader::find_by_correlation(...)`) or manual JSONL inspection.

Relevant existing surfaces:

- `crates/ralph-cli/src/tools.rs`
  - owns the `ralph tools ...` namespace
- `crates/ralph-cli/src/capability.rs`
  - already implements a small inspect UX over `EvidenceLookup`
- `crates/ralph-core/src/evidence_index.rs`
  - exposes `Entries`, `Missing`, and `NoEntry`

## Goals

- Add a minimal CLI UX for request/answer evidence lookup.
- Keep JSON output stable enough for tests and agents.
- Preserve the durable truth-source boundary: evidence index is only a lookup surface.
- Reuse existing evidence-index semantics instead of introducing a second answer store.

## Non-goals

- No generic `ralph evidence` top-level command.
- No new request broker or answer aggregation layer.
- No event-log parsing as the primary contract.
- No graph-dependent lookup path.
- No automatic `reply.human.message` synthesis.

## Command placement

Use:

`ralph tools answer inspect <correlation_id>`

Rationale:

- Keeps the UX close to runtime-facing tooling.
- Makes the scope obvious: this is specifically for answer-return evidence, not for every evidence use case.
- Avoids prematurely creating a broad evidence product surface.

## Lookup semantics

Input:
- one correlation id, which may be either a request id or an answer id

Behavior:
- Read `.ralph/evidence-index.jsonl`
- Call `EvidenceIndexReader::find_by_correlation(correlation_id)`
- Map results as:
  - `Entries` -> success output with status `entries`
  - `Missing` -> success output with status `missing`
  - `NoEntry` -> non-zero command failure

Rationale:
- `Missing` means the runtime did write durable evidence, so it is a successful lookup result.
- `NoEntry` means the operator asked for a correlation id that has no evidence and should fail visibly.

## Output contract

Human output should show:
- correlation id
- evidence index path
- status
- each entry's artifact kind, path, producer, and status

JSON output should include at least:
- `correlation_id`
- `index_path`
- `status`
- `entries`

Optionally include direct entry serialization from core to avoid schema drift.

## Risks

### Risk: this grows into a generic evidence subsystem too early
Mitigation:
- Keep the command under `tools answer`
- Do not add search/list/summary in this change

### Risk: human output and JSON output drift
Mitigation:
- Treat JSON as the stable automation contract
- Keep human output intentionally thin

### Risk: command implies event-log truth moved to the index
Mitigation:
- Word help text and tests so that the index is always described as a lookup surface pointing to durable artifacts
