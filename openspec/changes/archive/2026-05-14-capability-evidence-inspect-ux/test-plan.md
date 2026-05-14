# Test Plan: capability-evidence-inspect-ux

## Acceptance Criteria

- `ralph tools capability inspect <invocation_id> --json` reads `.ralph/evidence-index.jsonl` and returns entries for a real capability invocation.
- JSON output includes the invocation id, lookup status, and evidence entries.
- Human output prints a concise summary with artifact kind, artifact path, producer, and status.
- Unknown invocation ids fail with a non-zero exit code and a clear message.
- Existing `ralph tools capability invoke` behavior remains unchanged.

## Focused Test Cases

### CLI integration: inspect successful invocation evidence as JSON

Setup:

```bash
ralph tools capability invoke --id hat:focused-reviewer --input "review this patch" --json
```

Command:

```bash
ralph tools capability inspect <invocation_id> --json
```

Assertions:

- command exits successfully
- stdout is valid JSON
- JSON `invocation_id` equals the requested invocation id
- JSON `status` is `entries`
- JSON `entries` contains artifact kinds:
  - `capability_invoke_json`
  - `capability_result_json`
  - `resolved_config`
  - `event_log_jsonl`
- JSON entries include artifact paths pointing at existing Phase 3 artifacts

### CLI integration: inspect successful invocation evidence as human text

Command:

```bash
ralph tools capability inspect <invocation_id>
```

Assertions:

- command exits successfully
- stdout includes the invocation id
- stdout includes `capability_invoke_json`
- stdout includes `.ralph/capability-invocations`
- stdout includes `.ralph/events.jsonl`

### CLI integration: unknown invocation id fails

Command:

```bash
ralph tools capability inspect missing-invocation-id --json
```

Assertions:

- command exits non-zero
- stderr includes `missing-invocation-id`
- stderr includes `.ralph/evidence-index.jsonl`
- stdout does not contain a successful empty report

## Regression Gates

- `openspec validate capability-evidence-inspect-ux --type change`
- `openspec validate --all --strict`
- `cargo fmt --all -- --check`
- `cargo test -p ralph-cli --test integration_capability`
- `cargo test -p ralph-cli capability::tests`
- `cargo test -p ralph-core smoke_runner`
- `cargo test`
- `git diff --check`
