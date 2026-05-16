# Test Plan: default-bootstrap-parallel-run

## Focused unit test

Command:

```bash
cargo test -p ralph-cli startup_resources::tests -- --nocapture
```

Covers:

- default bootstrap resolution has inline prompt
- default bootstrap resolution clears `PROMPT.md`
- default bootstrap resolution sets `parallel.enabled=true`
- explicit bootstrap gate remains narrow

## Integration test

Command:

```bash
cargo test -p ralph-cli --test integration_startup_resources -- --nocapture
```

Covers:

- empty workspace without `ralph.yml` / `PROMPT.md` can run dry-run
- `.ralph/bootstrap-selection.json` is written
- `.ralph/resolved-config.yml` is written
- resolved config includes `parallel.enabled=true`
- explicit `--config ralph.yml` bypasses bootstrap artifact writing

## Regression gates

```bash
cargo fmt --all -- --check
openspec validate --all --strict
cargo test -p ralph-core smoke_runner
cargo test
git diff --check
```
