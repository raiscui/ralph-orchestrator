# Tasks — declarative-e2e-mock-parity

## 1. Schema + integration

- [ ] 1.1 Extend `DeclarativeScenario` schema (`crates/ralph-e2e/src/declarative/scenario.rs`)
      to accept an optional `mock:` block under `setup:`. Fields:
      `cassette_dir` (default `cassettes/e2e`), `speed` (default 1.0),
      `allow_commands` (default empty).
- [ ] 1.2 When `mock:` is present, have `DeclarativeScenarioRunner::execute`
      resolve the cassette via `CassetteResolver::resolve(scenario_id, backend)`
      before handing off to the executor.
- [ ] 1.3 If the executor path needs the workspace `ralph.yml` rewritten
      with a mock custom backend, **call into the existing imperative
      `TestRunner::configure_mock_mode`** rather than reimplementing it.

## 2. Failure-mode parity

- [ ] 2.1 If `mock:` is set but the cassette cannot be resolved, record
      a hard-fail `TestResult { passed: false }` with a "Mock cassette"
      assertion matching the imperative fix.
- [ ] 2.2 Do **not** log this as a "scenario skipped" event; the
      declarative path must not reintroduce the false-green class that
      the imperative fix removed.

## 3. Test coverage

- [ ] 3.1 New YAML scenario `crates/ralph-e2e/scenarios/declarative-mock-happy.yaml`
      with a backend-specific cassette. Live-validate it through
      `cargo run -p ralph-e2e -- codex --filter declarative-mock-happy`.
- [ ] 3.2 New YAML scenario `crates/ralph-e2e/scenarios/declarative-mock-fallback.yaml`
      using a generic `scenario-id.jsonl` cassette. Validate.
- [ ] 3.3 New YAML scenario `crates/ralph-e2e/scenarios/declarative-mock-missing.yaml`
      asserting **FAIL** when the cassette is absent under explicit `mock:`.
- [ ] 3.4 New YAML scenario with `speed: 10.0`; time the run end-to-end
      and assert it is materially faster than the unspeeded variant.
- [ ] 3.5 Unit test: `DeclarativeScenarioRunner` propagates
      `MockConfig::with_speed` / `without_commands` through to the
      executor's argv.

## 4. Roll-out

- [ ] 4.1 Gate the wire-up behind `RALPH_DECLARATIVE_MOCK=allow`
      env toggle for one release.
- [ ] 4.2 Document the new schema in
      `crates/ralph-e2e/scenarios/README.md` (or equivalent) so
      authors discover the option.

## 5. Verification

- [ ] 5.1 `cargo test -p ralph-e2e` green.
- [ ] 5.2 `cargo run -p ralph-e2e -- codex --filter declarative-mock-happy,declarative-mock-fallback` PASS.
- [ ] 5.3 `cargo run -p ralph-e2e -- codex --filter declarative-mock-missing` exits non-zero (real FAIL, not skip).
- [ ] 5.4 With `RALPH_DECLARATIVE_MOCK=unset`, the existing imperative
      scenarios still pass via `cargo run -p ralph-e2e -- codex --filter events,backpressure,hat-instances`.

## 6. Final

- [ ] 6.1 Archive this change on completion:
      `mv openspec/changes/declarative-e2e-mock-parity openspec/changes/archive/2026-08-12-declarative-e2e-mock-parity`.
