## 1. OpenSpec artifacts

- [x] 1.1 Create proposal for `adapter-contract-tests`.
- [x] 1.2 Create design with stream, prompt transport, envelope, and termination/flush boundaries.
- [x] 1.3 Create delta spec for adapter contract tests.
- [x] 1.4 Validate the change with `openspec validate adapter-contract-tests --type change`.

## 2. Stream contract tests

- [x] 2.1 Add or strengthen tests proving stderr terminal writes are recorded as diagnostics but do not feed default event parsing.
- [x] 2.2 Add or strengthen tests proving stdout terminal writes still feed event parsing / replay.

## 3. Prompt transport contract tests

- [x] 3.1 Add focused test proving `prompt_mode=stdin` does not append prompt as trailing argv for custom backend execution.
- [x] 3.2 Add focused test or fixture proving mock replay is compatible with stdin prompt mode.

## 4. Event envelope and replay attribution tests

- [x] 4.1 Add tests proving `EventRecord` preserves `id` and `reply`.
- [x] 4.2 Add tests proving parallel `TerminalWrite` evidence preserves `instance_id` for mock-cli filtering.
- [x] 4.3 Add tests proving runtime delivery/lifecycle replay-critical payloads are not truncated.

## 5. Termination / flush tests

- [x] 5.1 Add strict parse test for a record-session file containing `_meta.session_start`, `_meta.loop_start`, `ux.terminal.write`, `bus.publish`, and `_meta.termination`.
- [x] 5.2 Add or strengthen test proving critical records flush to disk before summary/watch reads them.

## 6. Verification

- [x] 6.1 Run focused tests for adapter contract areas.
- [x] 6.2 Run `cargo test -p ralph-core smoke_runner`.
- [x] 6.3 Run `openspec validate --all --strict`.
