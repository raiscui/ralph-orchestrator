# Tasks: default-bootstrap-parallel-run

## 1. Spec and design

- [x] 1.1 Define resource-bootstrap requirement for no-config/no-prompt default parallel mode.
- [x] 1.2 Document implementation decision and guardrails.

## 2. Implementation

- [x] 2.1 Ensure bootstrap-resolved config sets `parallel.enabled=true`.
- [x] 2.2 Preserve explicit config bypass behavior.

## 3. Tests

- [x] 3.1 Add unit assertion for default bootstrap resolution parallel mode.
- [x] 3.2 Add integration assertion for `.ralph/resolved-config.yml` parallel mode.
- [x] 3.3 Keep explicit missing config bypass test passing.

## 4. Validation

- [x] 4.1 `openspec validate default-bootstrap-parallel-run --type change`
- [x] 4.2 `cargo test -p ralph-cli startup_resources::tests -- --nocapture`
- [x] 4.3 `cargo test -p ralph-cli --test integration_startup_resources -- --nocapture`
- [x] 4.4 `cargo fmt --all -- --check`
- [x] 4.5 `openspec validate --all --strict`
- [x] 4.6 `cargo test -p ralph-core smoke_runner`
- [x] 4.7 `cargo test`
- [x] 4.8 `git diff --check`
