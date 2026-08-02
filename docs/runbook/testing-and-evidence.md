# Testing And Evidence

Ralph's completion claim must be backed by reproducible evidence.
This runbook lists the checks that matter for documentation, Rust code, and
orchestration behavior.

## Docs Site

Build the GitHub Pages artifact locally:

```bash
uvx --from mkdocs --with mkdocs-material --with mkdocs-minify-plugin --with mkdocs-material-extensions --with pymdown-extensions mkdocs build --strict
```

`--strict` is intentional. Any warning should be treated as a failed docs build.

## Rust Workspace

The project-level build and test commands are:

```bash
cargo build
cargo test
```

For focused Rust changes, prefer exact package/test filters when possible.
For release or orchestration changes, also run replay-based smoke tests.

## Replay Smoke Tests

The repository's AGENTS.md names these smoke tests:

```bash
cargo test -p ralph-core smoke_runner
cargo test -p ralph-core kiro
```

They use recorded fixtures instead of live API calls.

## Runtime Evidence Lane Release-Fast Gate

Use this focused command set when changing runtime protocol, dynamic hats,
record-session evidence, agents snapshots, or evidence-index correlation:

```bash
openspec validate clean-current-runtime-evidence-and-dynamic-role-contract --type change --strict
cargo test -p ralph-core evidence_index --lib --quiet
cargo test -p ralph-cli --test integration_topology_spawn parallel_parent_visible_spawn_materializes_dynamic_agents_without_redelivery --quiet
cargo test -p ralph-core smoke_runner --quiet
openspec validate --all --strict
```

If the change touches live backend behavior, also run the focused Codex E2E:

```bash
bash scripts/run-parallel-hat-instances-codex.sh
```

The integration topology-spawn guardrail is the replay dogfood for this lane.
It must retain durable evidence in:

- `.ralph/events.jsonl`
- `.ralph/agents.json`
- `.ralph/evidence-index.jsonl`
- the requested `--record-session` JSONL

Do not treat stdout or a terminal screenshot as enough proof if those durable
artifacts do not contain the expected spawn/result/termination evidence.

## Record-Session Evidence

Record sessions while debugging or validating orchestration behavior:

```bash
ralph run --record-session /tmp/session.jsonl -p "Fix the failing behavior"
ralph record summary /tmp/session.jsonl
```

For live probing:

```bash
ralph record watch --until-topic reply.human.message --timeout-secs 30 --quiet
ralph record watch --until-event _meta.termination --timeout-secs 120 --quiet
```

Use the summary to separate two contracts:

| Contract | Question |
| --- | --- |
| Durability | Did the event or reply exist in JSONL? |
| Display | Was the existing evidence rendered in the selected UI? |

## Backpressure Rule

Do not close a runtime task because the implementation "looks done".
Close it after:

- implementation is complete
- the relevant build passes
- focused tests pass
- docs build passes for docs changes
- record-session or command output proves the claim

## Known Validation Trap

Historical memory notes a recurring trap: full `cargo nextest run` can surface
unrelated configuration failures. For docs/help work, first run the docs build
and focused checks that prove the task. If broad tests expose unrelated failure,
record it separately instead of attributing it to the docs change.
