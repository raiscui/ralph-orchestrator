# Declarative E2E + Mock Mode Parity

## Problem Statement

`crates/ralph-e2e/src/declarative/scenario.rs` (introduced by `b9d909d feat(e2e): 声明式场景试点`)
provides a YAML-driven scenario runner that lives alongside the imperative
`crates/ralph-e2e/src/runner.rs`. The imperative runner can run a scenario
under mock mode (cassette replay, `--mock`) through the following chain:

  `RunConfig::with_mock(MockConfig)`
  → `runner.rs::configure_mock_mode` (writes a `custom` backend into `ralph.yml`)
  → `ralph run` spawns `ralph-e2e mock-cli --cassette <path>`
  → cassette JSONL events replay as if they came from a real Claude/Kiro backend.

The declarative runner does **not** currently re-use any of that machinery:

```
$ grep -n 'mock' crates/ralph-e2e/src/declarative/*.rs
(no output)
```

Consequence: a scenario expressed as YAML cannot be exercised under
`--mock`, even when a cassette exists for it. As the declarative surface
becomes the dominant authoring format (per the 候选6 / declarative e2e
rewrite on local main), this gap will get in the way of:

- **Cost-free CI** for declarative scenarios (mock-mode runs do not
  call paid APIs).
- **Deterministic regression tests** for declarative scenarios
  (cassettes pin the output stream).
- **Local rehearsal** of long-running declarative scenarios
  (`--mock-speed 10.0` already exists in `mock.rs`; it just is not
  wired into the declarative path).

The audit report `openspec/changes/sync-origin-main-features-q3-2026/audit-p3-p4.md`
§C2 documents the absence; this change proposes the smallest viable
fix.

## Solution

Wire the existing `mock` subsystem into the declarative runner:

1. **YAML schema** — accept an optional `mock:` block under `setup:`
   mirroring the imperative `MockConfig` shape (`cassette_dir`,
   `speed`, `allow_commands`).
2. **`scenario.rs` integration** — when a scenario declares `mock:`,
   resolve the cassette the same way the imperative runner does
   (`CassetteResolver::resolve(scenario_id, backend)`), then either:
   - **option A** (preferred): call into the imperative runner's
     `configure_mock_mode` so the workspace `ralph.yml` is rewritten,
     and let the existing executor path take over.
   - **option B**: build the `ralph-e2e mock-cli` invocation directly
     from `DeclarativeScenarioRunner`. Only if option A surfaces a
     tight coupling that we want to break.
3. **Failure mode** — keep the **hard-fail** semantics introduced by
   local main's imperative fix (audit-p3-p4.md §C1.3): a missing
   cassette for an explicitly-requested mock scenario is a real
   `TestResult { passed: false }`, never a skip.
4. **Test coverage** — declarative YAML cases for:
   - happy path (cassette present + backend-specific)
   - fallback (cassette present + generic `scenario-id.jsonl`)
   - hard-fail (cassette missing under explicit `mock:`)
   - speed override (`--mock-speed`) replay timing

This change intentionally **does not** touch:

- The imperative path (`runner.rs::configure_mock_mode`).
- The `mock.rs` module's public surface — only calls into it.
- Any upstream cherry-pick work (`sync-origin-main-features-q3-2026`).
- The deleted `ralph-api/` crate. None of this needs an HTTP API.

## User Stories

1. As a declarative scenario author, I want to mark a YAML scenario as
   `mock: required`, so that CI runs it under cassette replay rather
   than burning live API credits.
2. As a regression-test author, I want the declarative mock-mode run to
   pin the output stream via cassette, so that a flaky or deterministic
   failure reproduces bit-for-bit.
3. As a local developer rehearsing a long scenario, I want
   `--mock-speed 10.0` to work on YAML scenarios, so that the rehearsal
   finishes in seconds instead of minutes.
4. As a CI maintainer, I want missing cassettes to produce a real FAIL
   (not a silent skip), so that a broken mock fixture does not give a
   green CI badge.
5. As a downstream maintainer, I want all of the above to reuse the
   existing `mock.rs` surface, so that the imperative and declarative
   paths cannot drift.

## Implementation Decisions

- **Wire through the imperative runner, not around it** (option A). The
  imperative runner already does the YAML rewriting, the executor spawn,
  and the failure-mode accounting. Forcing declarative to duplicate
  that logic invites drift (see F1 in `audit-p3-p4.md` §C2 — that audit
  flagged exactly this kind of split as a risk).
- **No new build artefacts.** All work is in `crates/ralph-e2e/` plus
  one new YAML scenario file under `crates/ralph-e2e/scenarios/`.
- **No new dependencies.** The `mock` types, `CassetteResolver`,
  `RunConfig.with_mock`, `build_mock_cli_args` are already in this
  repo as of `8b27556`.
- **Differential roll-out** — drop the wire-up behind a temporary
  `RALPH_DECLARATIVE_MOCK=allow` env toggle for one release so that
  imperative users are not surprised by any behaviour change in the
  declarative path.

## Out of Scope

- Re-introducing an HTTP/RPC API (drops the deleted `ralph-api/`
  crate back into scope; covered separately if at all).
- Cherry-picking upstream `01dd250` / `0b61a78` MCP fixes — their
  target files do not exist locally (see audit-p3-p4.md §C2.4, F2).
- Refactoring `mock.rs` to support a per-step replay cursor — that
  is a separate "deterministic replay" change if it is ever needed.

## Further Notes

- Originating audit:
  `openspec/changes/sync-origin-main-features-q3-2026/audit-p3-p4.md`
  §C2 (Finding F1).
- Originating proposal:
  `openspec/changes/sync-origin-main-features-q3-2026/proposal.md`
  Appendix C §C.2.
- This change has no upstream cherry-pick dependency; it can be worked
  on in parallel with the rewrite tasks in the parent change's Group 4.
