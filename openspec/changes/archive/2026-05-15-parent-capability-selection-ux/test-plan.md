# Test Plan: parent-capability-selection-ux

## Acceptance Criteria

- `ralph#1` parent context includes a bounded runtime capability catalog.
- Catalog includes stable marker text, callable capability id, kind, concise summary, and input guidance.
- Catalog includes the exact `capability.request` event topic and required payload fields: `request_id`, `capability_id`, `input`.
- Catalog generation uses structured metadata and does not require YAML comments.
- Catalog does not inject full workflow/hat bodies into the parent context.
- A deterministic parent run can select a listed capability, emit `capability.request`, receive `capability.result`, and inspect evidence by invocation id.
- Parent topology and parent `ralph.yml` remain unchanged.

## Focused Test Cases

### Focused: parent capability catalog renderer

Setup:

- Build a small in-memory set of capability summaries.
- Render parent-visible catalog text/context.

Assertions:

- output contains stable catalog marker
- output contains `capability.request`
- output contains `request_id`, `capability_id`, and `input`
- output contains a sample capability id such as `hat:focused-reviewer`
- output contains capability kind and summary
- output does not contain a full synthetic long instruction body

### Integration: parent sees catalog and invokes listed capability

Setup:

- Workspace contains parent `ralph.yml` with deterministic custom backend.
- Parent backend records or echoes the startup prompt/context enough for the test to assert catalog presence.
- Backend emits a `capability.request` for a capability id that appears in the catalog.

Assertions:

- parent run exits successfully
- recorded parent context contains runtime capability catalog marker and listed capability id
- `.ralph/events.jsonl` contains request and parent-return result event
- parent-return result event includes `request_id` and `invocation_id`
- `.ralph/capability-invocations/<invocation_id>/` artifacts exist
- `.ralph/evidence-index.jsonl` contains entries for invocation id
- `ralph tools capability inspect <invocation_id> --json` returns `status = entries`
- parent `ralph.yml` remains unchanged

## Regression Gates

- `openspec validate parent-capability-selection-ux --type change`
- `openspec validate --all --strict`
- `cargo fmt --all -- --check`
- `cargo test -p ralph-cli --test integration_live_capability`
- `cargo test -p ralph-cli --test integration_capability`
- `cargo test -p ralph-cli capability::tests`
- `cargo test -p ralph-core smoke_runner`
- `cargo test`
- `git diff --check`
