## Why

Phase 2 already writes durable answer-return evidence for explicit `reply.hat.message` requester-return flows, and the repository already has a live dogfood gate proving those artifacts are produced.

What is still missing is a stable CLI lookup surface. Today, a human or agent still has to open `.ralph/evidence-index.jsonl` manually to answer a simple question such as:

- what evidence exists for request id `req-123`?
- did answer id `ans-456` resolve successfully or fail closed?
- is this a success, a missing marker, or no entry at all?

That is the same ergonomics gap capability invocation had before `ralph tools capability inspect` landed.

## What Changes

- Add a focused inspect UX under `ralph tools answer inspect <correlation_id>`.
- Reuse the existing evidence index reader and `.ralph/evidence-index.jsonl` path.
- Support both human-readable output and `--json` output.
- Preserve the current answer-return boundary: this command only reports existing evidence-index state; it does not invent answers, does not read runtime graphs as truth, and does not turn ordinary workflow events into answer-return flows.
- Keep the command narrow. This is not a new generic `ralph evidence` subsystem.

## Impact

- CLI surface: `crates/ralph-cli/src/tools.rs` plus a new answer-evidence inspect module.
- Tests: extend `crates/ralph-cli/tests/integration_answer_evidence.rs` and add focused unit coverage for missing/no-entry mapping.
- Stable spec: `openspec/specs/request-reply-answer-evidence/spec.md` gains inspect UX requirements.
