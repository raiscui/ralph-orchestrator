# Test Plan: capability-child-run-evidence

## Acceptance Criteria

- A successful `ralph tools capability invoke` writes `.ralph/evidence-index.jsonl`.
- Lookup by invocation id finds durable evidence for:
  - `invoke.json`
  - `result.json`
  - `resolved-config.yml`
  - `.ralph/events.jsonl`
- A failed isolated invocation writes `failed.json` evidence with failure status.
- Parent `ralph.yml` remains unchanged.
- Existing capability list and invocation behavior remains compatible.

## Focused Test Cases

### CLI integration: successful micro-run evidence

Command shape:

```bash
ralph tools capability invoke --id hat:focused-reviewer --input "review this patch" --json
```

Assertions:

- command exits successfully
- JSON report includes `invocation.invocation_id`
- `.ralph/capability-invocations/<id>/invoke.json` exists
- `.ralph/capability-invocations/<id>/result.json` exists
- `.ralph/capability-invocations/<id>/resolved-config.yml` exists
- `.ralph/events.jsonl` contains `capability.invoke` and `capability.result`
- `.ralph/evidence-index.jsonl` contains entries for the invocation id with artifact kinds:
  - `capability_invoke_json`
  - `capability_result_json`
  - `resolved_config`
  - `event_log_jsonl`

### Focused unit: failed child-run evidence

Use `invoke_isolated_with_runner()` with a fake runner returning `success=false`.

Assertions:

- `failed.json` exists
- `.ralph/evidence-index.jsonl` contains `capability_failed_json`
- failure entry status is `failure`
- invocation/result report keeps `parent_topology_unchanged=true`

## Regression Gates

- `cargo test -p ralph-cli --test integration_capability`
- `cargo test -p ralph-core smoke_runner`
- `cargo test`
- `openspec validate --all --strict`
- `git diff --check`
