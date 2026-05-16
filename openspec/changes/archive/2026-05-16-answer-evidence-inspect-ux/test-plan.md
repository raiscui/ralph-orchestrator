# Test Plan: answer-evidence-inspect-ux

## Acceptance criteria

- `ralph tools answer inspect <correlation_id> --json` returns valid JSON for a real answer-evidence request id.
- `ralph tools answer inspect <correlation_id>` prints concise human-readable output for a real answer-evidence answer id.
- `Missing` evidence markers remain visible as successful lookup results with status `missing`.
- Unknown correlation ids fail with a clear non-zero error.
- Existing answer-evidence runtime behavior remains unchanged.

## Focused test cases

### CLI integration: inspect request-id answer evidence as JSON

Setup:
- run the existing live answer-evidence dogfood flow

Command:
- `ralph tools answer inspect req-dogfood-1 --json`

Assertions:
- exits successfully
- stdout is valid JSON
- `correlation_id == "req-dogfood-1"`
- `status == "entries"`
- entries include `reply_event` and `runtime_delivery_record`

### CLI integration: inspect answer-id evidence as human output

Command:
- `ralph tools answer inspect ans-dogfood-1`

Assertions:
- exits successfully
- stdout includes `ans-dogfood-1`
- stdout includes `event_log_jsonl`
- stdout includes `.ralph/events.jsonl`

### CLI integration: unknown correlation id fails

Command:
- `ralph tools answer inspect unknown-answer-id --json`

Assertions:
- exits non-zero
- stderr includes `unknown-answer-id`
- stderr includes `.ralph/evidence-index.jsonl`

### Focused unit: explicit missing marker remains visible

Setup:
- write a minimal evidence-index JSONL with one `missing` answer marker

Command path:
- call the inspect report helper directly

Assertions:
- report status is `missing`
- report entries are preserved

## Regression gates

- `openspec validate answer-evidence-inspect-ux --type change`
- `openspec validate --all --strict`
- `cargo fmt --all -- --check`
- `cargo test -p ralph-cli --test integration_answer_evidence`
- `cargo test -p ralph-cli answer`
- `cargo test -p ralph-core smoke_runner`
- `cargo test`
- `git diff --check`
