# Test Plan: live-runtime-capability-invocation

## Acceptance Criteria

- A deterministic parent run can emit a structured capability request.
- Ralph handles the request exactly once per request id.
- The requested capability runs through the existing isolated child/micro-run path.
- Parent-visible result/failure event includes request id and invocation id.
- `.ralph/capability-invocations/<invocation_id>/` artifacts are produced.
- `.ralph/evidence-index.jsonl` contains entries for the invocation id.
- `ralph tools capability inspect <invocation_id> --json` can locate the evidence chain.
- Parent `ralph.yml` remains unchanged.

## Focused Test Cases

### Integration: parent run triggers a hat micro-run capability

Setup:

- Workspace contains parent `ralph.yml` with a custom deterministic backend.
- Backend emits a structured `capability.request` event with:
  - `request_id = cap-req-dogfood-1`
  - `capability_id = hat:focused-reviewer`
  - `input = review this patch`

Assertions:

- parent run exits successfully
- `.ralph/events.jsonl` contains the request event and a result event
- result event includes `request_id = cap-req-dogfood-1`
- result event includes an `invocation_id`
- `.ralph/capability-invocations/<invocation_id>/invoke.json` exists
- `.ralph/capability-invocations/<invocation_id>/result.json` exists
- `.ralph/evidence-index.jsonl` contains entries for invocation id
- `ralph tools capability inspect <invocation_id> --json` returns `status = entries`
- parent `ralph.yml` remains unchanged

### Focused unit: duplicate request id is handled once

Feed the runtime capability request handler the same request id twice.

Assertions:

- first request starts one isolated invocation
- second request does not start another invocation
- duplicate handling is explicit in the result/status

### Focused unit: malformed request does not invoke capability

Feed a request event missing capability id or input.

Assertions:

- no isolated invocation starts
- parent-visible failure/error is produced or returned
- failure is explicit enough for debugging

## Regression Gates

- `openspec validate live-runtime-capability-invocation --type change`
- `openspec validate --all --strict`
- `cargo fmt --all -- --check`
- `cargo test -p ralph-cli --test integration_capability`
- `cargo test -p ralph-cli capability::tests`
- `cargo test -p ralph-core smoke_runner`
- `cargo test`
- `git diff --check`
