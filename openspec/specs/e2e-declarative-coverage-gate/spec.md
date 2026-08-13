# e2e-declarative-coverage-gate Specification

## Purpose
TBD - created by archiving change e2e-declarative-migration-plan. Update Purpose after archive.
## Requirements
### Requirement: E2E test suite MUST fail loudly when declarative coverage drops below 90 %
The `ralph-e2e` test crate MUST expose an integration test
`declarative_coverage_gate` that constructs the same scenario list
the `ralph-e2e` binary uses, splits the list into declarative /
imperative kinds, excludes the explicitly-kept
`ParallelExperimentalDevEngineExampleScenario` from the imperative
denominator, computes
`declarative_count / (declarative_count + effective_imperative_count)`,
and asserts the ratio is at least `0.90`. The test MUST be wired
into CI so a coverage drop below the threshold is a hard build
failure, not a soft warning.

#### Scenario: Coverage above threshold keeps CI green
- **WHEN** the registry is in its current state (39 declarative, 21
  effective imperative after the explicit-keep is excluded)
- **THEN** the gate test computes `39 / 60 ≈ 65.0 %` and FAILS, which
  is the expected pre-migration state; CI is intentionally red
  until enough migrations land to cross the 90 % line

#### Scenario: Coverage drops below threshold after a regression
- **WHEN** a contributor accidentally re-introduces an imperative
  `Box::new(TypeNameScenario::new())` (or deletes a YAML)
- **THEN** the ratio decreases by the appropriate amount and the
  gate test fails CI with a per-tier breakdown so the regression is
  diagnosable from the failure output alone

#### Scenario: Explicit-keep is excluded from the denominator
- **WHEN** the registry contains `ParallelExperimentalDevEngineExampleScenario`
  with the existing "保留命令式" comment
- **THEN** the gate test MUST NOT count that scenario as part of the
  imperative denominator, and MUST record the exclusion explicitly
  in its drift log so future contributors cannot silently flip the
  definition of "imperative" to game the ratio

---

### Requirement: Scenario registry MUST expose each entry's kind (declarative vs imperative)
The `ralph-e2e` public surface MUST expose a function
`all_scenarios()` returning `Vec<(ScenarioKind, Box<dyn TestScenario>)>`
(or an equivalent typed pairing) so the CI gate test can compute
the declarative / imperative split mechanically. `ScenarioKind`
MUST have at least `Declarative` and `Imperative` variants, and the
declarative variant MUST be reachable from a YAML loader
(`ralph_e2e::declarative::from_yaml(...)`) without going through
private fields. The existing `crates/ralph-e2e/src/main.rs`
`get_all_scenarios()` function MAY be removed or kept as a thin
wrapper; the public lib surface is the new source of truth.

#### Scenario: CLI list path uses the public lib surface
- **WHEN** the user runs `cargo run -p ralph-e2e -- --list`
- **THEN** the binary iterates `ralph_e2e::all_scenarios()` and
  produces the same ordered list of scenario ids as before the
  refactor (no behavioural change for the user)

#### Scenario: Gate test imports the same public lib surface
- **WHEN** the CI gate test runs `cargo test -p ralph-e2e --test
  declarative_coverage_gate`
- **THEN** the test imports `ralph_e2e::all_scenarios` and
  `ralph_e2e::ScenarioKind`, asserts the same registry ordering,
  and computes the ratio without depending on private fields

