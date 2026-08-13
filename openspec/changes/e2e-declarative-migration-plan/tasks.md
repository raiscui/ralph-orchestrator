# Tasks — e2e-declarative-migration-plan

> Tasks land in three waves: ① the CI gate first (so subsequent migrations
> can verify their own deltas), ② the 22 migrations one-commit-each,
> ③ the 5.1 closure (deprecate imperative escape hatch only after gate
> is green).

## 1. Wave 1 — CI gate infrastructure

- [ ] 1.1 Refactor `get_all_scenarios()` from `main.rs` into
      `ralph_e2e::all_scenarios()` in `crates/ralph-e2e/src/lib.rs`,
      re-exporting every concrete scenario struct that the CLI
      currently uses. `main.rs` becomes a thin caller (or removes
      the function entirely; same ordered data, just relocated).
- [ ] 1.2 Add `pub enum ScenarioKind { Declarative, Imperative }` to
      `ralph_e2e::scenarios`. Wire each `Box<dyn TestScenario>` in
      `all_scenarios()` to carry its kind (e.g. via a parallel
      `Vec<(ScenarioKind, &str)>` or a `TestScenario::kind()` default
      method). Imperative kinds are the default; declarative comes
      from a tag the YAML loader returns.
- [ ] 1.3 Add `crates/ralph-e2e/tests/declarative_coverage_gate.rs`
      that constructs the same list as the binary, counts:
      - `declarative_count`: number of `Declarative`-kind entries
      - `imperative_count`: number of `Imperative`-kind entries,
        **excluding** `ParallelExperimentalDevEngineExampleScenario`
      - asserts `declarative_count / (declarative_count + effective_imperative_count) >= 0.90`
- [ ] 1.4 Wire the gate test into CI:
      - `cargo test -p ralph-e2e --test declarative_coverage_gate`
        must succeed for the build to pass.
- [ ] 1.5 Emit a per-tier drift log from the gate on failure so that
      the next commit can copy-paste the failure message into a PR
      description.

## 2. Wave 2 — 22 migrations (one commit per imperative)

Each row below maps 1:1 to `audit-p5-p1.md` §A.2 ordering. Land as
one commit per row; each commit must include the YAML source and the
single registry-line swap.

### 2.1 Easy (4 commits)

- [ ] 2.1.1 Migrate `TimeoutScenario` → `scenarios/timeout.yaml`
- [ ] 2.1.2 Migrate `MaxIterationsScenario` → `scenarios/max-iterations.yaml`
- [ ] 2.1.3 Migrate `BackendUnavailableScenario` → `scenarios/backend-unavailable.yaml`
- [ ] 2.1.4 Migrate `AuthFailureScenario` → `scenarios/auth-failure.yaml`

### 2.2 Medium (5 commits)

- [ ] 2.2.1 Migrate `HatSingleScenario` → `scenarios/hat-single.yaml`
- [ ] 2.2.2 Migrate `HatInstructionsScenario` → `scenarios/hat-instructions.yaml`
- [ ] 2.2.3 Migrate `HatEventRoutingScenario` → `scenarios/hat-event-routing.yaml`
- [ ] 2.2.4 Migrate `HatBackendOverrideScenario` → `scenarios/hat-backend-override.yaml`
- [ ] 2.2.5 Migrate `HatMultiWorkflowScenario` → `scenarios/hat-multi-workflow.yaml`

### 2.3 MemorySystem, medium-hard (8 commits)

- [ ] 2.3.1 Migrate `MemoryAddScenario` → `scenarios/memory-add.yaml`
- [ ] 2.3.2 Migrate `MemorySearchScenario` → `scenarios/memory-search.yaml`
- [ ] 2.3.3 Migrate `MemoryInjectionScenario` → `scenarios/memory-inject.yaml`
- [ ] 2.3.4 Migrate `MemoryPersistenceScenario` → `scenarios/memory-persist.yaml`
- [ ] 2.3.5 Migrate `MemoryCorruptedFileScenario` → `scenarios/memory-corrupted.yaml`
- [ ] 2.3.6 Migrate `MemoryMissingFileScenario` → `scenarios/memory-missing.yaml`
- [ ] 2.3.7 Migrate `MemoryRapidWriteScenario` → `scenarios/memory-rapid-write.yaml`
- [ ] 2.3.8 Migrate `MemoryLargeContentScenario` → `scenarios/memory-large-content.yaml`

### 2.4 Hard, schema extension needed (4 commits)

- [ ] 2.4.1 Migrate `ToolUseScenario` → schema adds `expect.tool_invocations:` + `scenarios/tool-use.yaml`
- [ ] 2.4.2 Migrate `StreamingScenario` → schema adds per-token pacing + `scenarios/streaming.yaml`
- [ ] 2.4.3 Migrate `ParallelAppServerIdleStartScenario` → non-live harness + `scenarios/parallel-app-server-idle-start.yaml`
- [ ] 2.4.4 Migrate `ParallelAppServerSteerMultiTurnScenario` → non-live harness + `scenarios/parallel-app-server-steer-multi-turn.yaml`

### 2.5 Explicit-keep (NOT a migration target)

- [x] 2.5.0 `ParallelExperimentalDevEngineExampleScenario` is registered with the registry comment "保留命令式" and stays in the imperative list **forever**. The CI gate test explicitly subtracts this entry from the denominator.

## 3. Wave 3 — 5.1 closure (after gate is green + ≥ 19 of 21 migrations landed)

- [ ] 3.1 Confirm `cargo test -p ralph-e2e --test declarative_coverage_gate` is green.
- [ ] 3.2 Annotate every remaining imperative `TestScenario` impl with `#[deprecated(since = "…", note = "use the declarative YAML under scenarios/<name>.yaml")]`.
- [ ] 3.3 Add a `docs/e2e/declarative-migration.md` pointer under
      `crates/ralph-e2e/README.md` so new contributors discover the
      declarative path first.
- [ ] 3.4 Open a follow-up issue / change tracker for eventual
      physical removal of the imperative structs after one release
      cycle. **Do not delete them in this change** — deprecated code
      stays compile-able.

## 4. Verification

- [ ] 4.1 After each Wave 2 commit, re-run the audit script; the
      resulting percentage is the per-commit delta. The PR
      description must include the table:
  ```
  $ cargo test -p ralph-e2e --test declarative_coverage_gate -- --nocapture
  Declarative: N
  Imperative: M (effective, after subtracting the explicit-keep)
  Coverage:    X.XX %
  Threshold:   90.00 %
  Pass / Fail: PASS/FAIL
  ```
- [ ] 4.2 CI green throughout (no test regressions in ralph-e2e or
      ralph-cli).
- [ ] 4.3 `cargo run -p ralph-e2e -- --list` shows the migrated
      scenarios with a `yaml:` chip (or equivalent marker) so the
      CLI makes the new authoring path visible.

## 5. Final

- [ ] 5.1 Archive this change on completion:
      `mv openspec/changes/e2e-declarative-migration-plan openspec/changes/archive/<date>-e2e-declarative-migration-plan`
      where `<date>` matches the convention `YYYY-MM-DD-<short-slug>`
      already used in this repo's archive directory.
