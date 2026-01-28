# Parallel Trigger Routing (App Example)

This is a runnable, end-to-end example for **parallel-trigger-routing**.

It demonstrates the default routing semantics when `parallel.enabled: true` and **no** `parallel.topic_contracts` are configured:

- `topic -> hats`: fanout to **all** hats that subscribe to the topic (`hats.*.triggers`)
- `hat -> instance`: for each hat, queue to **exactly one** instance (idle-first, round-robin)

## What you should see

This example intentionally produces **two** `spec.ready` events:

1. `spec_writer` emits `spec.ready` with `version: 1`
2. `spec_reviewer` rejects it (`spec.rejected`)
3. `spec_writer` revises and emits `spec.ready` with `version: 2`
4. `spec_reviewer` approves (`spec.approved`)
5. Ralph receives `spec.approved` and outputs `LOOP_COMPLETE`

`spec.ready` is subscribed by **two hats** (`spec_reviewer` and `spec_logger`), so it should trigger both.

`spec_logger` is configured with `instances: 2`, so the two `spec.ready` events should typically be handled by:
- `spec_logger#1` (first `spec.ready`)
- `spec_logger#2` (second `spec.ready`)

## Run

From the repo root:

```bash
# Use config + prompt files directly
cargo run --bin ralph -- run \
  -c examples/parallel-trigger-routing/ralph.yml \
  -P examples/parallel-trigger-routing/prompt.md \
  --no-tui
```

Optional: override backend on the CLI (recommended if your default is not configured):

```bash
cargo run --bin ralph -- run \
  -c examples/parallel-trigger-routing/ralph.yml \
  -P examples/parallel-trigger-routing/prompt.md \
  -b codex \
  --no-tui
```

## Notes

- This example is intentionally **trigger-driven** and does not use `parallel.topic_contracts`.
- If you need explicit delivery/audience rules, add topic contracts and they will take precedence over triggers.
