## Context

The recent natural-language dynamic-hats dogfood produced a useful runtime trace:

- record-session: `/tmp/ralph-dynamic-evolution-angle-dogfood-20260523-151612.jsonl`
- termination: `CompletionPromise`
- `topology.spawn_group = 1`
- `topology.spawn.result = 1`
- `analysis.done = 6`, including `builder#2..builder#6`
- dynamic roles: `protocol_architect`, `evidence_auditor`, `ux_reviewer`, `governance_reviewer`, `e2e_gatekeeper`

The dogfood showed the parent-visible dynamic spawn path can work, but it also exposed the next architecture need: runtime protocol semantics, dynamic role identity, record-session evidence, agents snapshot history, and release gates need to be treated as one evidence closure instead of separate UX conveniences.

Current relevant truth sources:

- record-session JSONL: semantic run and bus evidence.
- `.ralph/events.jsonl`: debug/replay event log.
- `.ralph/agents.json`: current registry plus completed dynamic tombstones.
- evidence index: correlation link kernel, not a replacement truth source.
- TUI/stdout/record summary: display surfaces over durable evidence.

## Goals / Non-Goals

**Goals:**

- Define a runtime protocol SSOT for reserved topics, workflow entry hints, reply semantics, and stdout-only parsing.
- Make task-derived dynamic role contracts auditable after the prompt is gone.
- Define `topology.spawn_group` partial/tombstone behavior.
- Let record summary / evidence inspect correlate request ids, event ids, reply ids, instance ids, role contract hashes, and result topics.
- Define a release-fast gate for this runtime/evidence lane.

**Non-Goals:**

- Do not implement `agent-cli-recoverable-failure-retry` in this change.
- Do not implement `tui-mdfried-viewer` or image rendering in this change.
- Do not replace record-session, events log, agents snapshot, or evidence index with a new monolithic truth source.
- Do not broaden this change into a full diagnostics taxonomy or generic retry engine.
- Do not require every backend parity problem to be solved before this focused runtime/evidence contract lands.

## Decisions

### Decision 1: Keep record-session as semantic completion truth

Use record-session `_meta.termination` and `bus.publish` records as the primary semantic evidence for runtime completion and result coverage.

Alternatives considered:

- Use wrapper exit status: rejected because outer shell wrappers can fail after Ralph already wrote semantic completion.
- Use stdout tail or TUI display: rejected because display can hide, truncate, re-render, or buffer output.

### Decision 2: Treat evidence index as correlation, not replacement

The evidence index should link artifacts by stable ids, not duplicate full record-session contents or become a second source of truth.

Rationale:

- Existing specs already require record-session and event logs to remain readable independently.
- Correlation lookup is useful for request id / role contract hash / invocation id navigation, but source artifacts must remain authoritative.

### Decision 3: Canonical role contract is the downstream authority

Raw `topology.spawn_group` payload is an intent. Runtime normalization must produce the canonical role contract used for prompts, snapshots, and evidence.

Consequences:

- Raw `allowed_topics` is not blindly trusted.
- Effective allowed result topics are clipped against target hat publishes and reserved control-plane exclusions.
- Evidence stores summaries and hashes by default; full canonical contract can be retained if needed by implementation.

### Decision 4: Partial spawn outcomes are first-class evidence

`topology.spawn_group` is non-atomic by product choice. Therefore partial success must be represented, not hidden.

Expected phases:

1. request parsed
2. member validated
3. instance registered
4. delivery attempted
5. job completed or failed
6. instance reaped or tombstoned

Each phase can produce a partial failure that must name request id, instance id when available, role, phase, and recovery hint.

### Decision 5: Release-fast gate is a focused gate, not full release certification

The runtime/evidence lane needs a small but meaningful gate:

- `openspec validate --all --strict`
- focused Rust tests for touched modules
- `cargo test -p ralph-core smoke_runner`
- focused dynamic spawn integration/dogfood with record-session and agents snapshot artifacts
- optional live Codex E2E when the change affects live app-server/parallel behavior

This is not the full release matrix. It is the minimum gate before claiming this runtime/evidence contract is implemented.

## Risks / Trade-offs

- [Risk] Over-centralizing evidence into a new file could create another truth source.  
  Mitigation: specs explicitly require links and summaries to point back to original artifacts.

- [Risk] Role contract evidence could leak too much prompt content or sensitive context.  
  Mitigation: start with summary + hash + warnings; only persist full canonical contract if a later design explicitly justifies it.

- [Risk] Partial spawn outcomes add state complexity.  
  Mitigation: model partial state around existing job lifecycle phases and tombstones rather than introducing a second scheduler.

- [Risk] Release-fast gate may become too expensive if it always runs live E2E.  
  Mitigation: keep replay/focused tests mandatory and make live E2E conditional on touched runtime surfaces.

- [Risk] Prompt-surface matrix may drift from implementation.  
  Mitigation: include prompt alignment tests for configured hats and task-derived dynamic workers.

## Migration Plan

1. Add spec-backed tests for runtime protocol reserved topic classification and prompt surface boundaries.
2. Add or update role contract evidence structures in the spawn result / agents snapshot path.
3. Add partial/tombstone lifecycle evidence for `topology.spawn_group` member failures.
4. Extend record summary / evidence inspect output with dynamic spawn correlation.
5. Add focused dynamic spawn replay/integration guardrails.
6. Run the release-fast gate and preserve artifacts in task/worklog evidence.

Rollback strategy:

- The change should be incremental and additive.
- If a display enhancement regresses, keep record-session/event-log semantics and roll back only the display layer.
- If role contract evidence shape needs adjustment, preserve backwards-compatible parsing for existing summary fields until archived evidence is no longer needed.

## Open Questions

- Should the full canonical role contract be persisted, or is summary + hash sufficient for the first implementation?
- Should partial spawn failures be emitted as a new topic, encoded in `topology.spawn.result`, or recorded only in agents/evidence sidecars?
- Which live dogfood script should become the canonical release-fast runtime/evidence gate?
- Should dynamic role result coverage require exact source instance set, or allow equivalent failed terminal topics such as `build.blocked`?
