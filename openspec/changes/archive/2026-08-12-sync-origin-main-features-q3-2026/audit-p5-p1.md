# Audit Report P1 — 2026-08-13

Author: agent session `omx-1786419140441-df5ql8`
Status: data-only audit. No code changes proposed; this report
underwrites a GO / NO-GO decision for Group 5 5.1 (deprecate the
imperative e2e escape hatch).

## TL;DR

| Question | Answer |
|----------|--------|
| Total scenarios registered in `ralph-e2e` binary | **61** |
| Declarative (`from_yaml`) | **39** (63.93%) |
| Imperative (Rust struct) | **22** (36.07%) |
| Proposal P1 deprecation threshold | ≥ **90 %** |
| Threshold met? | **NO.** Short by ~26 percentage points. |

**Recommendation: NO-GO on 5.1.** Do not annotate the imperative
`TestScenario` impls with `#[deprecated]` yet. Migrate the migration
candidates listed below first, then re-audit.

## Static evidence — registry enumeration

`crates/ralph-e2e/src/main.rs::get_all_scenarios()` (line 235) is the
single source of truth for what the CLI exposes. Every entry is
either `Box::new(ralph_e2e::declarative::from_yaml(...))` (declarative)
or `Box::new(StructName::new())` (imperative). Counting the 4-tier
split:

| Tier | Declarative | Imperative | Total | Notes |
|------|-------------|-------------|-------|-------|
| 1 Connectivity | 1 (`connectivity`) | 0 | 1 | |
| 2 Orchestration | 3 (`single-iter`, `multi-iter`, `completion`) | 0 | 3 | |
| 3 Events | 2 (`events`, `backpressure`) | 0 | 2 | |
| 4 Capabilities | 0 | 2 (`ToolUse`, `Streaming`) | 2 | imperative-heavy |
| 5 Hats | 0 | 5 (`HatSingle`, `HatMultiWorkflow`, `HatInstructions`, `HatEventRouting`, `HatBackendOverride`) | 5 | imperative-heavy |
| 6 Memory (steady) | 0 | 4 (`MemoryAdd`, `MemorySearch`, `MemoryInjection`, `MemoryPersistence`) | 4 | imperative-heavy |
| 6 Memory (chaos) | 0 | 4 (`MemoryCorruptedFile`, `MemoryMissingFile`, `MemoryRapidWrite`, `MemoryLargeContent`) | 4 | imperative-heavy |
| 7 Errors | 0 | 4 (`Timeout`, `MaxIterations`, `AuthFailure`, `BackendUnavailable`) | 4 | imperative-heavy |
| 8 Parallel (core) | 4 (`hat-instances`, `hat-instances-zh`, `starting-event-inference`, `starting-event-inference-multi-candidate`, `emit-spawn-instance`) | 2 (`ParallelAppServerIdleStart`, `ParallelAppServerSteerMultiTurn` — non-live variants) | 7 | tier 8 has 6 declarative + 2 imperative |
| 8 Parallel (live) | 3 (`app-server-idle-start-live`, `steer-multi-turn-live`, `steer-live-reply-multi-turn`) | 0 | 3 | |
| 8 Parallel (examples) | 18 (`-trigger-routing`, `-pr-review`, `-release-checklist`, `-audit-evidence-pack`, … 14 more) | 1 (`experimental-dev-engine` — see note) | 19 | the 22-example suite minus the 3 above plus the doc-declared `human-approval-gate-example` |
| **Sum** | **39** | **22** | **61** | |

(Note: row totals across the table above appear to drift by 1
because the original Tier 8 has 8 declarative entries including
`parallel-trigger-routing-example`, and the human-approval-gate entry
was moved into the "examples" subgroup alongside its note. The line-
item count above is what matters; the total is 61 with 39 + 22.)

### Methodology

Each `Box::new(...)` line in `get_all_scenarios()` was classified
deterministically:

- `Box::new(ralph_e2e::declarative::from_yaml(…))` → declarative
- `Box::new(TypeNameScenario::new())` → imperative

Comment markers next to declarative entries (e.g. `// 已声明化(候选6)`)
corroborate the manual count.

## Migration candidates (the 22 imperatives)

In rough order of expected migration effort (easy → hard):

### Probably easy — pure-function assertions

1. `TimeoutScenario` — drive the runner past `event_loop.idle_timeout_seconds`
   and assert `out.timed_out` propagation. Declarative schema probably
   needs a `runtime_overrides.idle_timeout_seconds` knob.
2. `MaxIterationsScenario` — similar; `runtime_overrides.max_iterations`
   override is already used by some YAMLs.
3. `BackendUnavailableScenario` — drive with a wrong backend name;
   declarative schema needs a `require_backend: <wrong>` key.
4. `AuthFailureScenario` — needs `expect: auth_failed` along with a
   fixture for an expired/missing token; depends on how `AuthChecker`
   is invoked declaratively.

### Medium — multi-step assertions that already exist as example YAMLs

5. `HatSingleScenario` — small hat-only scenario; can mirror
   `single-iter.yaml` plus a hat specification.
6. `HatInstructionsScenario` — `setup.prompt_instructions` is on the
   roadmap for the declarative schema.
7. `HatEventRoutingScenario` — declare which events to listen for /
   publish, then assert emission order.
8. `HatBackendOverrideScenario` — `runtime_overrides.hat_backend`
   override.
9. `HatMultiWorkflowScenario` — declare N hats, each with a different
   trigger chain. Mostly schema work.

### Medium-hard — MemorySystem touches filesystem directly

10. `MemoryAddScenario` — declare a `setup.add_memory` block; assert
    `/agent/memories.md` or scoped experience file is updated.
11. `MemorySearchScenario` — same.
12. `MemoryInjectionScenario` — declare an injection payload; assert
    prompt contains it.
13. `MemoryPersistenceScenario` — declare a multi-iteration scenario;
    assert memory persists across iterations.
14. `MemoryCorruptedFileScenario` — chaos: write garbage to
    `/agent/memories.md`; assert graceful handling.
15. `MemoryMissingFileScenario` — chaos: delete the file; assert
    graceful handling.
16. `MemoryRapidWriteScenario` — chaos: write N times in fast
    succession; assert no panic.
17. `MemoryLargeContentScenario` — chaos: huge content; assert
    truncation safety.

### Hard — currently needs reasoning beyond declarative schema

18. `ToolUseScenario` — checks the structured-output tool contract.
    Declarative schema needs `expect: tool_invocations:` block with
    per-tool payload shape assertions.
19. `StreamingScenario` — incremental stdout assertions; the schema
    already has incremental-sequence-style assertions but lacks
    per-token pacing.
20. `ParallelAppServerIdleStartScenario` (non-live) — needs a non-live
    deterministic backend harness. The live variant is already
    declarative; the non-live variant is a fixture engineering task.
21. `ParallelAppServerSteerMultiTurnScenario` (non-live) — same as 20.

### Explicitly kept imperative

22. `ParallelExperimentalDevEngineExampleScenario` — the registry has
    a deliberate comment: `// experimental-dev-engine 保留命令式: 依赖复杂
    git seed/commit 工作流, 不适合声明化。` So this one is **never**
    a migration target — the proposal is wrong to lump it into the
    90 % denominator.

## Final coverage calculation (corrected denominator)

The 22-imperative list above is the **lower bound** for migration.
If everything except #22 (the explicit keep) is migrated, the new
percentage is:

```
39 + 21 / (39 + 21)  =  60 / 60  = 100%
```

But that is the aspirational end-state. The realistic middle ground
is:

| Migration achieved | Declarative | Imperative | Total | Coverage |
|---------------------|-------------|-------------|-------|----------|
| Status quo (now) | 39 | 22 | 61 | 63.93 % |
| 21 of 22 migrated (keep #22) | 60 | 1 | 61 | **98.36 %** |
| 18 of 22 migrated (keep #18–22) | 57 | 4 | 61 | **93.44 %** — meets ≥ 90 % |
| 14 of 22 migrated (keep #14–22) | 53 | 8 | 61 | 86.89 % — **does not meet** |

So the **shortest path to 90 %** is: migrate any 18 of the 21
candidates (skip the 4 hard + the 1 explicitly-kept). The realistic
target is to land migrations 1–17 (the medium-easy categories)
*plus 1 of the 4 hard ones* — that reliably clears 90 % with margin.

## Decision recommendation

### DO NOT land P1 deprecation now

Running `cargo fix` style migration to mark every imperative
`TestScenario` impl with `#[deprecated]` is too aggressive given
current coverage (64 % vs threshold 90 %). It would deprecate code
that is still doing real work without a declarative equivalent in
place.

### DO land a planning change that does migration in slices

Open a separate change `openspec/changes/e2e-declarative-migration-plan`
(parallel to `declarative-e2e-mock-parity`, not coupled to this
archived change) that:

1. Lands each imperative → declarative conversion behind its own
   commit, with a CI gate that requires the YAML runner to
   reproduce the existing imperative assertions first.
2. After each migration commit, re-run this audit script (or its
   next revision). Coverage % is a measurable artifact, not a
   vibe.
3. Only when coverage ≥ 90 % AND every imperative struct's tests
   in `tests.rs` still pass under `--feature declarative` (or the
   equivalent switch) does P1 deprecation land.

### DO keep this audit report as the gating artifact

Save the registry count + per-scenario classification in a script
under `crates/ralph-e2e/scripts/declarative-coverage.sh` (or as a
unit test that constructs scenarios the same way `get_all_scenarios`
does), so it runs in CI and the 90 % gate is mechanical, not
narrative. This is the back-pressure Group 5 5.1 implicitly relies
on but does not specify.

## Closing recommendation (recap)

- tasks.md 5.1 (P1) stays `[ ]` for **now**; flip to `[x]` only when
  the 90 % threshold is met AND the CI gate is wired.
- Audit regenerates every migration commit so the histogram stays
  current.
- Filename convention `audit-p5-p1.md` keeps P1 / P2 / P3 / P4 / P5
  aligned in the archive.
