# Design: parent-capability-selection-ux

## Context

Current state:

- Ralph can list and summarize runtime capabilities through CLI surfaces.
- Phase 3 records isolated capability invocation artifacts and evidence-index entries.
- Phase 3.1 adds `ralph tools capability inspect <invocation_id>`.
- Phase 4 lets a real parent run handle a structured `capability.request` from `ralph#1` and execute it through isolated child/micro-run invocation.

The missing piece is selection UX. In Phase 4 dogfood, the deterministic parent backend already knew the exact capability id and JSON shape. That is useful for proving the runtime hook, but not enough for a real coordinator. `ralph#1` needs a concise, machine-readable capability catalog in its parent-side context so it can choose a listed capability and emit the right request.

The key invariant remains: capability selection does not change live topology. The catalog is an instruction/resource surface, not a dynamic `HatRegistry` mutation.

## Goals / Non-Goals

**Goals:**

- Expose a bounded runtime capability catalog to `ralph#1`.
- Include enough metadata for selection: id, kind, summary, when-to-use guidance, input guidance, and request event contract.
- Generate the catalog from structured capability metadata.
- Keep parent topology immutable and reuse the existing isolated invocation hook.
- Dogfood the selection surface with deterministic integration tests and Phase 3.1 inspect.

**Non-Goals:**

- No natural-language tool-call parser.
- No generic function-calling protocol.
- No live `HatRegistry` mutation or parent config replacement.
- No external LLM E2E requirement.
- No multi-step autonomous planner for capability chaining.
- No dependence on YAML comments as runtime metadata.

## Decisions

### Decision 1: Inject a bounded capability catalog into parent coordinator context

Ralph should add a short capability section to the parent coordinator prompt/context when runtime capability invocation is available.

The section should include:

- a stable heading/marker that tests can assert;
- one entry per selected callable capability;
- capability id, kind, summary, and input guidance;
- exact `capability.request` event shape.

Rationale:

- `ralph#1` already reasons from prompt/context.
- This makes selection explicit without adding a new broker.
- Tests can verify the actual parent context contains the catalog.

Rejected alternative:

- Keep capability ids out-of-band and expect prompt authors to hardcode them. That makes Phase 4.1 non-productized and brittle.

### Decision 2: Catalog metadata is structured, not comment-derived

Catalog entries must come from data structures already used for capability listing / summaries, or from new explicit metadata fields if needed.

Rationale:

- YAML comments are not preserved as runtime metadata.
- Startup selector and capability invocation should share the same rule: human comments can explain, but machines need structured fields.

Rejected alternative:

- Parse workflow file comments at runtime. That would fail when comments are stripped, reformatted, or not loaded through the same path.

### Decision 3: Keep the request contract as `capability.request`

The selection UX should teach `ralph#1` to emit the same structured event introduced by Phase 4:

```xml
<event id="..." topic="capability.request">{"request_id":"...","capability_id":"hat:focused-reviewer","input":"..."}</event>
```

Rationale:

- The execution path already exists and is tested.
- Phase 4.1 should improve discovery/selection, not create a second invocation protocol.

### Decision 4: Start with concise summaries, not full capability bodies

The parent catalog should not load every full workflow or hat instruction body. It should provide enough to choose and call, then rely on isolated child/micro-run execution for the actual capability body.

Rationale:

- Keeps parent startup context bounded.
- Preserves fresh-context isolation for child invocations.

## Proposed UX Shape

Parent coordinator context gets an added section similar to:

```markdown
## Runtime Capability Catalog

You may invoke one of these capabilities by emitting a structured event:

<event id="capability-request-unique-id" topic="capability.request">{"request_id":"unique-request-id","capability_id":"<id>","input":"<task-specific input>"}</event>

Available capabilities:
- id: hat:focused-reviewer
  kind: hat
  summary: Review a focused slice and return findings.
  input: Describe the artifact or question to review.
```

Exact wording may differ, but tests should assert stable markers and the machine contract, not prose style.

## Risks / Trade-offs

- [Risk] Catalog bloat can pollute parent context. → Mitigation: bounded summaries, no full body injection.
- [Risk] The parent may emit invalid ids despite catalog. → Mitigation: existing Phase 4 failure path returns `capability.failed`.
- [Risk] Tests become brittle if they assert prose wording. → Mitigation: assert stable marker, ids, topics, and JSON fields.
- [Risk] Capability listing code may live in CLI while parent prompt assembly lives elsewhere. → Mitigation: expose a small internal builder instead of duplicating list logic.
