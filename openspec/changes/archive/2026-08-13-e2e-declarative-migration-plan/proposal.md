# E2E Declarative Migration Plan — Q3 2026

## Problem Statement

The local `main` branch's e2e scenarios are split between two
representations:

- **Declarative** (`.yaml` files under `crates/ralph-e2e/scenarios/`)
  rendered through `ralph_e2e::declarative::from_yaml(id, yaml)`.
- **Imperative** (Rust `TestScenario` impls under
  `crates/ralph-e2e/src/scenarios/*.rs`) registered in
  `main.rs::get_all_scenarios()`.

The audit report `audit-p5-p1.md` (inside the archived
`sync-origin-main-features-q3-2026` change) measured coverage at
**63.93 % declarative** (39 / 61), which is below the 90 %
threshold that Group 5 5.1 set for retiring the imperative escape
hatch. The 22 remaining imperatives are listed in §A.2 below;
they span errors / memory / hats / tools / parallel-non-live /
one explicit-keep.

That audit closed with three recommendations:

1. Do **not** annotate imperative `TestScenario` impls with
   `#[deprecated]` yet — the 90 % gate has no mechanical checker
   and would be narrative-driven.
2. Move the migration out of the archived change and into its own
   plan with per-commit migrations and re-audits.
3. Encode the 90 % gate as a CI-enforced **declarative coverage
   gate test** that constructs scenarios the same way
   `get_all_scenarios()` does.

This change is the implementation of those three recommendations.

## Solution

Three orthogonal deliverables, each landing behind its own commit
and its own audit-regenerate step:

### 1. CI gate test (must land **before** any migration)

Add `crates/ralph-e2e/tests/declarative_coverage_gate.rs` which:

- Imports the future `ralph_e2e::all_scenarios()` (new public surface
  — see §3 below) and counts:
  - `declarative_count` — boxes returned by
    `ralph_e2e::declarative::from_yaml(...)`.
  - `imperative_count` — boxes of concrete `TestScenario` impls.
- Asserts `declarative_count / total_count >= 0.90`.
- Emits a precise drift log on failure (per-tier breakdown) so the
  next audit report can copy the table verbatim.
- Lives in `tests/`, so it ships as part of `cargo test -p ralph-e2e`
  in CI, no separate workflow.

This is the **mechanical back-pressure** that proposal §2 of this
change requires. It also gives the next migration commit an
automated before/after snapshot.

### 2. Refactor `get_all_scenarios()` into a public lib surface

The current registry function lives in
`crates/ralph-e2e/src/main.rs::get_all_scenarios()`. The CI gate
test needs access to it. Move / re-export it as:

```rust
// crates/ralph-e2e/src/lib.rs
pub fn all_scenarios() -> Vec<Box<dyn TestScenario>> { … }
```

The `main.rs::get_all_scenarios()` becomes a thin wrapper or is
deleted; the binary CLI list / run paths use the new lib surface.

This is a **light refactor** — same data, same ordering, just
relocated and made `pub`. No behaviour change at runtime.

### 3. Twenty-two migration commits

One commit per imperative in §A.2 of `audit-p5-p1.md`. Each commit:

- Adds a `.yaml` sibling under `crates/ralph-e2e/scenarios/`
  (or extends an existing YAML).
- Swaps one `Box::new(TypeNameScenario::new())` for
  `Box::new(ralph_e2e::declarative::from_yaml(...))` inside the
  registry, removing the imperative entry.
- Records in tasks.md which category it landed under.
- Re-runs the audit script and bumps `coverage %` in the cumulative
  archive readme.

Migrations are **independent** once the gate lands; the archived
change stays at "0 / 22 done", and this change increments one tick
per migration commit.

### 4. Explicit `experimental-dev-engine` exclusion

`ParallelExperimentalDevEngineExampleScenario` is annotated in the
registry as "保留命令式: 依赖复杂 git seed/commit 工作流, 不适合
声明化". It is **always** counted as imperative, i.e. the 90 %
denominator must exclude it. The CI gate test records this exclusion
explicitly so a future contributor cannot silently flip the
definition of "imperative" to game the ratio.

## A. Detailed migration schedule (audit-p5-p1.md cross-references)

The 22 imperatives are listed here in the same order as audit-p5-p1
§A.2 so cross-referencing is mechanical.

### A.1 Easy — pure-function assertions

| # | Imperative struct | Target YAML | Acceptance gate |
|---|--------------------|-------------|-----------------|
| 1 | `TimeoutScenario` | new `timeout.yaml` | declarative run reports `failure: timeout` for the same scenario config the imperative asserts on |
| 2 | `MaxIterationsScenario` | new `max-iterations.yaml` | declarative run reports `failure: max_iterations`, recorded iteration count matches imperative baseline |
| 3 | `BackendUnavailableScenario` | new `backend-unavailable.yaml` | declarative run fails fast with a backend-not-authenticated outcome |
| 4 | `AuthFailureScenario` | new `auth-failure.yaml` | declarative run fails fast with the auth token error |

### A.2 Medium — multi-step assertions that already have example YAMLs

| # | Imperative struct | Target YAML | Acceptance gate |
|---|--------------------|-------------|-----------------|
| 5 | `HatSingleScenario` | new `hat-single.yaml` | declarative run drives one hat with a single trigger chain |
| 6 | `HatInstructionsScenario` | new `hat-instructions.yaml` | declarative run respects per-hat prompt instructions |
| 7 | `HatEventRoutingScenario` | new `hat-event-routing.yaml` | declarative run asserts event emission ordering |
| 8 | `HatBackendOverrideScenario` | new `hat-backend-override.yaml` | declarative run applies hat-level backend config |
| 9 | `HatMultiWorkflowScenario` | new `hat-multi-workflow.yaml` | declarative run drives N hats with branching triggers |

### A.3 Medium-hard — MemorySystem

| # | Imperative struct | Target YAML | Acceptance gate |
|---|--------------------|-------------|-----------------|
| 10 | `MemoryAddScenario` | new `memory-add.yaml` | declarative run writes `/agent/memories.md` and asserts file contents |
| 11 | `MemorySearchScenario` | new `memory-search.yaml` | declarative run searches memories and asserts injection |
| 12 | `MemoryInjectionScenario` | new `memory-inject.yaml` | declarative run injects scoped experience into prompt |
| 13 | `MemoryPersistenceScenario` | new `memory-persist.yaml` | declarative run iterates twice and asserts memory persists |
| 14 | `MemoryCorruptedFileScenario` | new `memory-corrupted.yaml` | declarative run writes garbage to memory file, asserts graceful handling |
| 15 | `MemoryMissingFileScenario` | new `memory-missing.yaml` | declarative run deletes memory file, asserts graceful handling |
| 16 | `MemoryRapidWriteScenario` | new `memory-rapid-write.yaml` | declarative run writes N times in fast succession |
| 17 | `MemoryLargeContentScenario` | new `memory-large-content.yaml` | declarative run writes huge content, asserts truncation safety |

### A.4 Hard — schema extension needed

| # | Imperative struct | Target YAML | Acceptance gate |
|---|--------------------|-------------|-----------------|
| 18 | `ToolUseScenario` | new `tool-use.yaml` | declarative schema gains `expect.tool_invocations:` per-tool payload assertions |
| 19 | `StreamingScenario` | new `streaming.yaml` | declarative schema gains per-token pacing assertions |
| 20 | `ParallelAppServerIdleStartScenario` (non-live) | new `parallel-app-server-idle-start.yaml` | declarative schema gains a deterministic non-live backend harness |
| 21 | `ParallelAppServerSteerMultiTurnScenario` (non-live) | new `parallel-app-server-steer-multi-turn.yaml` | declarative schema gains a deterministic non-live steer harness |

### A.5 Explicit-keep (NOT a migration target)

| # | Imperative struct | Reason | Disposition |
|---|--------------------|--------|--------------|
| 22 | `ParallelExperimentalDevEngineExampleScenario` | relies on git seed/commit workflow outside declarative scope | counted as imperative forever; excluded from denominator |

The CI gate test subtracts entry 22 from the denominator before
computing the ratio, so the threshold is **21 of 21 needed for
100 %, 19 of 21 for 90 %**.

## B. Out of Scope

- Cherry-picking upstream work (`sync-origin-main-features-q3-2026`).
- Robot RPC, Forge CLI, MCP schema fixes (deleted `ralph-api`).
- Linux musl build target.
- TUI refactor.
- Declarative e2e + mock mode parity (`declarative-e2e-mock-parity`).
  This change ships YAML only; mock-mode wiring is a separate concern.

## C. Implementation Decisions

- **Gate test lives in `tests/`** (integration test, not unit test).
  This is intentional: the future `ralph_e2e::all_scenarios()` API
  is the public surface, and the test must use it the same way the
  CLI does. Putting it in `tests/` also makes `cargo test -p
  ralph-e2e` the single CI gate; no second workflow to maintain.
- **Migration commits stay small.** One imperative → one commit.
  Reverts are easy if a YAML does not faithfully reproduce the
  imperative's behaviour on `--mock` or in CI.
- **Audit regenerates per migration.** The
  `audit-p5-p1.md` numbers (63.93 %) become a per-migration
  delta:
  - 0 / 22 → 39 / 61 (now)
  - 1 / 22 → 40 / 61 (≈ 64.5 %)
  - …
  - 21 / 22 → 59 / 60 → 98.33 %
  - 21 / 22 + gate test passing = 5.1 ready to land.
- **The 22nd imperative (`experimental-dev-engine`) is excluded
  from the denominator** explicitly in the test's source code.
  This matches the registry's deliberate comment.

## D. User Stories

1. As a Ralph e2e contributor, I want `cargo test -p ralph-e2e` to
   fail loudly if the declarative coverage drops below 90 %, so
   that retro-migrations or accidental imperative re-introductions
   fail CI rather than go unnoticed.
2. As a Ralph e2e author, I want each migration to be one PR-sized
   commit with a clear YAML diff, so that review and rollback are
   trivial.
3. As a reviewer of the archive, I want the per-migration deltas in
   `audit-p5-p1.md` to be honest, so I can see how many migrations
   remain without re-running the audit script.
4. As the eventual 5.1 (escape hatch deprecate) owner, I want a
   single test to be the gating artefact, so the deprecation lands
   mechanically and not on a vibe.
5. As a maintainer reading the registry in 6 months, I want the
   registry to contain either a YAML pointer or an explicit-keep
   comment for every line, so the historical "why imperative?"
   answer is one commit away.

## E. Cross-references

- Originating audit: `audit-p5-p1.md` inside
  `openspec/changes/archive/2026-08-12-sync-origin-main-features-q3-2026/`
- Originating decision: `tasks.md` 5.1 NO-GO note inside the
  same archived change.
