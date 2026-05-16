# Tasks: internalize-event-emission-protocol

## 1. OpenSpec artifacts

- [x] 1.1 Write proposal for moving generic event emission protocol out of execution-directory `ralph.yml`.
- [x] 1.2 Write design covering parser facts, prompt injection source of truth, overlay escaping, and migration path.
- [x] 1.3 Write delta spec for runtime prompt contract alignment.
- [x] 1.4 Write test plan.

## 2. Discovery

- [x] 2.1 Inspect `/Users/cuiluming/local_doc/l_dev/my/rust/ralph-example/ralph.yml` event-format duplication.
- [x] 2.2 Inspect `EventParser` to confirm current envelope and attributes.
- [x] 2.3 Inspect parallel prompt construction and `config/all_hat.md` injection.
- [x] 2.4 Confirm shared overlay examples are escaped to avoid accidental event replay.

## 3. Implementation (not started)

- [x] 3.1 Add or extract a built-in event emission protocol renderer with a stable marker.
- [x] 3.2 Inject the protocol into publishing parallel hat prompts.
- [x] 3.3 Ensure `ralph#1` coordinator instructions use the same protocol source of truth.
- [x] 3.4 Keep all-hat overlay examples non-emittable while allowing intentional role-specific output contract examples.
- [x] 3.5 Dogfood one example config by removing duplicated generic event-format tutorial blocks while keeping workflow-specific payload fields.

## 4. Tests (not started)

- [x] 4.1 Add focused renderer test for event envelope, stdout-only rule, completion promise boundary, and supported attributes.
- [x] 4.2 Add prompt-construction test proving publishing hats receive the built-in protocol without config-local tutorial text.
- [x] 4.3 Add coordinator prompt test proving `ralph#1` uses the same source of truth.
- [x] 4.4 Keep prompt overlay escaping regression green.
- [x] 4.5 Add example dogfood / integration fixture for a slimmed config.

## 5. Validation (not started)

- [x] 5.1 `openspec validate internalize-event-emission-protocol --type change`
- [x] 5.2 `openspec validate --all --strict`
- [x] 5.3 `cargo fmt --all -- --check`
- [x] 5.4 `cargo test -p ralph-core event_parser::tests`
- [x] 5.5 `cargo test -p ralph-core prompt_overlay`
- [x] 5.6 `cargo test -p ralph-core smoke_runner`
- [x] 5.7 `cargo test`
- [x] 5.8 `git diff --check`
