# Sync Origin/Main Features — Q3 2026

## Problem Statement

`b3bbe91e` (本地 `main` HEAD) 与 `e88b7e3` (origin/main HEAD) 已经从 merge-base
`1d90c1e feat(prompt): add OBJECTIVE section to prevent goal drift (#103)` 分叉。
两边各自独立演进,合计 **1,818 文件 / 24 万行净变更**,**1,642 个核心文件被两边同时改动**。

- **本地 main 一边**(147 commits):把精力投入 **declarative e2e 重写**(`b9d909d`+
  一系列候选 5/6 commit)+ **`3ff4b47 refactor(core): EventLoop 收窄为 run() 窄入口
  + PromptExecutor port`**。本地独有的 crates 模块代码增量为 **0**。
- **origin/main 一边**(248 commits):押注 **CLI 增量(10 个 feat/cli + 5 个 fix/cli)、
  adapters 修复(5)、presets / loops / hats / web / backends 的新能力**,已演进到 v2.10.1。

结果:本地 main 缺失一批独立、低风险、高价值的上游工作(`e88b7e3 ralph clean --events`、
robot RPC、Forge CLI、musl Linux 目标、`ralph-docs` skill 等),而若直接 rebase 到
origin/main 又会跟本地 main 的架构重构(declarative e2e、PromptExecutor port、EventLoop 收窄)
发生 1,642 个文件的合并冲突。

用户需要一份**经过优先级排序、可执行、回退可控**的整合方案:既能拿到上游独立 feature
与 fix,又不破坏本地 main 的核心架构演进。

## Solution

按 **价值 / 风险比** 把所有可做的工作分成 5 组,按顺序落地:

1. **零风险 cherry-pick**(6 项,半天):`ralph clean --events`、artifact path 规范化、
   ACP drain、MCP schema root inline、musl Linux 目标、新增 `ralph-docs` skill。
2. **小风险 cherry-pick**(6 项,一天):continue 状态持久化、timeout 测试加固、
   event history fixture 隔离、per-hat scratchpad instructions、MCP tool schema 去重、
   pi-coding-agent 包名更新。
3. **中风险 cherry-pick**(5 项,二天):Claude stream result wait、`hats validate -i`、
   local hat imports in preflight、explicit completion after guidance、context window telemetry。
4. **重做而非 cherry-pick**(6 项,作为独立 sprint):robot RPC domain、Forge CLI、
   publish remote review branches、file-backed web mode、`-H <name>` preset unify、
   TUI 相关 fix/feat。**不在本 change 内做**,只登记。
5. **本地 main 补丁**(6 项,与 cherry-pick 并行):declarative e2e 逃生舱 deprecate、
   `PromptExecutor` port contract test、`ralph-e2e/src/runner.rs` 反向 diff 审计、
   `ralph-api/src/main.rs` 反向 22 行审计、`specs/` ↔ `.ralph/specs/` 规范化、
   完成后 bump release tag。

整组完成触发**版本 bump 到 v2.10.x+1**。

## User Stories

1. As a Ralph CLI operator, I want to run `ralph clean --events`, so that I can
   remove only the event run history (`events.jsonl`, `events-*.jsonl`,
   `current-events` marker) without touching the agent scratchpad, memories, or
   other `.ralph/` artifacts.
2. As a Ralph user with strict disk budgets, I want `ralph clean --events` to be
   mutually exclusive with `--diagnostics`, so that the command line stays
   unambiguous about its target.
3. As a Ralph user who only wants a dry-run preview, I want `ralph clean --events
   --dry-run` to list the files that would be deleted without removing them.
4. As a Linux installer on AL2 (glibc 2.26) or AL2023 (glibc 2.34), I want
   statically linked musl binaries for both x86_64 and aarch64, so that the
   glibc-version matrix stops blocking installs.
5. As a Linux user who needs memory allocator parity with glibc builds, I want
   mimalloc enabled on the musl target, so that throughput regressions are bounded.
6. As a Ralph agent (Codex/Claude/Codex app-server), I want the `ralph-docs`
   skill installed as a third public skill, so that I can route documentation
   questions through the canonical `llms.txt` instead of guessing from stale memory.
7. As a Ralph CLI operator, I want `ralph hats validate -i/--instructions`, so
   that I can sanity-check hat instruction text against topology without changing
   the default topology path.
8. As a Claude API consumer, I want the adapter to wait for Claude's stream
   `result` events, so that result chunks are not dropped mid-stream.
9. As an MCP-compatible tool caller running against strict validators, I want the
   API to inline MCP tool schema roots instead of emitting bare `$ref`, so that
   clients do not reject responses.
10. As an ACP backend user, I want terminal output drained before adapter exit,
    so that I do not lose trailing log lines that arrive after the main response.
11. As an event-loop user who needs to resume after a crash or interrupt, I want
    `continue` state persisted, so that I do not lose position after a restart.
12. As a Claude stream consumer running e2e tests, I want event-history payload
    fixtures isolated per scenario, so that scenarios do not leak state into each
    other.
13. As a hat operator, I want instruction generation to honor each hat's
    scratchpad path, so that hat prompts see the right per-hat context.
14. As an MCP API consumer, I want duplicated MCP tool schemas deduplicated, so
    that responses are smaller and less ambiguous.
15. As a Claude stream consumer, I want explicit completion required after a
    guidance event, so that I never hang on a partial guidance response.
16. As a local hat author, I want file-based hat imports supported in preflight,
    so that I can split large hat collections across multiple files.
17. As a telemetry operator, I want context-window utilization tracked, so that I
    can size token budgets based on real consumption.
18. As an automation runner / external controller, I want a robot RPC domain with
    a v1 schema, so that I can drive Ralph programmatically.
19. As a multi-backend user, I want Forge CLI backend support, so that I can use
    Forge as a Claude alternative.
20. As a parallel loops operator, I want remote review branches published, so
    that reviewers can fetch a loop without worktree access.
21. As a web dashboard user, I want file-backed web mode, so that the web
    dashboard can survive restarts and offline replays.
22. As a preset user, I want the preset mechanism unified under `-H <name>` for
    both YAML and TOML, so that I can switch formats without re-learning CLI.
23. As a local main developer, I want the declarative e2e framework's imperative
    escape hatch marked deprecated once declarative coverage reaches ≥ 90%, so
    that the codebase does not drift between two syntaxes.
24. As a local main developer, I want `PromptExecutor` port to have a round-trip
    contract test, so that future cherry-picks of upstream loop changes cannot
    silently break the port.
25. As a local main developer, I want the reverse diff on
    `ralph-e2e/src/runner.rs` audited, so that I confirm declarative covers the
    coverage that was dropped from the imperative path.
26. As a local main developer, I want the reverse 22-line diff on
    `ralph-api/src/main.rs` audited, so that I know whether I dropped unused code
    or a real capability.
27. As a spec maintainer, I want `.ralph/specs/` (origin/main side) and `specs/`
    (local main side) reconciled, so that the two directories do not drift on the
    same canonical specs.
28. As a release manager, I want a release tag bumped after the cherry-pick set
    lands, so that downstream consumers can pin to the integrated release.

## Implementation Decisions

### Group 1 — Zero-risk cherry-picks (apply directly)

These commits have **no overlap with local main's architectural refactor
(`3ff4b47 EventLoop 收窄 + PromptExecutor port`) or declarative e2e rewrite**, so
they apply cleanly via `git cherry-pick -x` and require only `cargo check` +
`cargo test` for verification.

- `e88b7e3` — `feat(cli): add ralph clean --events (#357)`. Adds `event_artifacts`
  + `clean_events` to `ralph-cli` library surface; adds `events: bool` field with
  `conflicts_with = "diagnostics"` to `CleanArgs`; adds dispatch branch in
  `clean_command`; updates `docs/guide/cli-reference.md`. Local main has zero
  edits to `ralph-cli/src/lib.rs` since merge-base, so this is a clean apply.
- `db48462` — `fix: canonicalize Ralph artifact paths`. Path normalization only.
- `192f5f9` — `fix(adapters): drain ACP terminal output before exit`. Single
  adapter change.
- `01dd250` — `fix(api): inline MCP tool schema roots instead of emitting bare
  $ref (#351)`. Pure schema emitter change.
- `86cfb1a` — `build: switch Linux dist targets to musl for AL2/AL2023
  compatibility`. Build target switch; enable mimalloc on musl.
- `6aacc6b` — `feat(skills): add ralph-docs skill for llms.txt-driven
  introspection (#318)`. Adds `skills/ralph-docs/` subtree.

### Group 2 — Small-risk cherry-picks (apply with `git cherry-pick -x` + manual smoke test)

- `0207c8b` — `fix(event-loop): persist continue state`. Touches
  `event_loop/state` region that `3ff4b47` also touched; expect localized
  conflict that resolves by keeping both states.
- `c9f2182` — `fix(cli_executor): harden timeout activity test`. Test-only.
- `cf0ec8d` — `test: isolate event history payload fixture`. Test-only.
- `7b673cc` — `fix(prompts): honor per-hat scratchpad in generated instructions`.
  Touches `instructions.rs`; resolve by re-asserting the `EventLoop::run`
  wrapper.
- `0b61a78` — `fix(api): deduplicate MCP tool schemas`. Cherry-pick together with
  `01dd250` to keep MCP path consistent.
- `4ba3d3a` — `docs(pi): update pi-coding-agent package name reference to
  @earendil-works/pi-coding-agent`. Doc-only.

### Group 3 — Medium-risk cherry-picks (manual review + adapt)

- `4a38b8d` — `fix(adapters): wait for Claude stream result events (#355)`.
  Touches `claude_stream.rs` + `cli_executor.rs`; declarative e2e test harness
  exercises both regions. Plan to first cherry-pick, then rerun
  `cargo run -p ralph-e2e -- codex --filter events,backpressure,hat-instances`.
- `ee9fa67` — `feat(cli): opt-in hats validate --instructions checks (#356)`.
  Local main never touched `hats`, so apply is clean; verify by running
  `ralph hats validate -i fixtures/hats/...`.
- `25afeb0` — `feat(hats): support local hat imports in preflight`. Large diff
  (`+627` in `preflight.rs`); apply only after preflight is re-baselined against
  local main's `3ff4b47` changes.
- `a4b6d45` — `fix(runtime): require explicit completion after guidance (#326)`.
  High overlap with `3ff4b47`. Cherry-pick, then rerun PromptExecutor contract
  test (added in patch P2).
- `d631ef7` — `feat(telemetry): track context window utilization`. Touches
  metrics; independent of declarative e2e path; expect clean apply.

### Group 4 — Rewrite instead of cherry-pick (out of scope for this change)

These six commits overlap too heavily with local main's parallel changes to make
`git cherry-pick` cheaper than rewriting. They are listed here only for tracking
and should become their own follow-up changes.

- `6972444` — `feat(api): add robot RPC domain`.
- `2cfe7c9` — `feat(backends): add Forge CLI support`.
- `93e170d` — `feat(loops): publish remote review branches`.
- `3f1e0c3` — `feat(robot): add file-backed web mode`.
- `246336f` — `feat(cli): unify preset mechanism under -H <name>`.
- TUI region (`317266f`, `3454c62`, etc.) — local main's TUI work is
  incompatible; track separately.

### Group 5 — Local main patches (run in parallel with cherry-picks)

- **P1 — Deprecate imperative e2e escape hatch.** When declarative coverage
  reaches ≥ 90% of imperative scenarios (measured by `cargo test -p ralph-e2e
  -- --list` ratio), annotate the imperative `TestScenario` impls with
  `#[deprecated]` and add a doc link to the declarative migration guide.
- **P2 — Add `PromptExecutor` port contract test.** Add a round-trip test that
  drives `PromptExecutor::run` with synthetic `RunHooks` and asserts the produced
  `PromptOutput` matches the recorded trace.
- **P3 — Audit reverse diff on `ralph-e2e/src/runner.rs`.** Produce a coverage
  matrix mapping each removed assertion to either a declarative replacement or a
  documented acceptance of the loss.
- **P4 — Audit reverse 22-line diff on `ralph-api/src/main.rs`.** Either land a
  no-op commit documenting the deletion is unused, or restore the lines if they
  are required for any cherry-picked commit.
- **P5 — Reconcile `.ralph/specs/` and `specs/`.** Pick one as canonical (proposed:
  `specs/` because it is versioned in git), and either symlink or migrate.
- **P6 — Bump release tag.** After Group 1-3 cherry-picks land, bump to
  v2.10.x+1 in line with origin/main's v2.10.1 baseline.

## Testing Decisions

### What makes a good test for this change

A test passes if it proves the cherry-picked behavior still works **after the
local main's architectural surface** (`3ff4b47`, declarative e2e, runner.rs
changes) is in place. Tests target external behavior (CLI exit codes, event
logs, schema correctness) — not the internal commit hash or branch layout.

### Modules tested

- `ralph-cli` lib — `clean_diagnostics` and `clean_events` unit tests (3 tests
  per function: empty, dry-run, real delete). The dry-run and delete tests use
  a `tempfile::TempDir` fixture that creates `.ralph/` with mixed content
  (events, non-events) and asserts only the event artifacts are removed.
- `ralph-cli` arg parsing — `clap::Command::try_get_matches_from` test asserting
  `--events` and `--diagnostics` conflict (this is what `conflicts_with`
  guarantees).
- `ralph-adapters` — existing ACP drain integration test rerun after cherry-pick.
- `ralph-api` MCP schema serialization — schema-emit unit test asserting no
  bare `$ref` survives in the output JSON.
- `ralph-e2e` — declarative runner contract test (`scenario.rs`) rerun against
  the three flavors: `events`, `backpressure`, `hat-instances`.
- `ralph-e2e` (post-patch P2) — `PromptExecutor` port round-trip contract test.

### Prior art in this codebase

- `crates/ralph-cli/src/lib.rs::tests` already establishes the dry-run + delete
  pattern for `clean_diagnostics`; `clean_events` follows the same template.
- `crates/ralph-e2e/src/declarative/scenario.rs` (`b9d909d` and later) sets the
  pattern for declarative scenarios with embedded YAML fixtures; Group 1
  cherry-picks should reuse this for any new declarative coverage.
- `crates/ralph-core/src/event_loop/tests.rs` already has the
  `EventLoop::run`-level test pattern; the new `PromptExecutor` contract test
  (P2) should reuse this harness shape, not invent a new one.

### Manual verification

After cherry-pick, the operator runs:

```bash
cargo check --workspace --all-features
cargo test --workspace
cargo run -p ralph-e2e -- codex --filter events,backpressure,hat-instances,clean
```

The last command exercises `clean --events` end-to-end inside a real Codex
run, which is the strictest cross-check we have.

## Out of Scope

- **Rebasing local main onto origin/main.** The 248-commit rebase would burn
  the entire local main delta (declarative e2e, `3ff4b47` refactor). This
  change takes the opposite approach: cherry-pick the independent origin/main
  work, leave the architectural divergence in place.
- **Cherry-picking the 1,642-file overlap directly.** Files like
  `crates/ralph-cli/src/main.rs` (+2695/+1615), `ralph-tui/src/widgets/*`,
  `ralph-adapters/src/*` have structural changes on both sides. These land via
  the rewrite path (Group 4) or stay unmerged.
- **Replacing declarative e2e with imperative.** Local main chose declarative;
  this change does not revisit that choice.
- **Deprecating `--diagnostics`.** The two flags stay mutually exclusive.
- **CI workflow changes.** The existing GitHub Actions + `claude-code-action`
  pipeline is unaffected by this change.
- **Robot RPC, Forge CLI, file-backed web mode, `-H` preset unify, TUI
  region.** All are tracked under Group 4 as future changes; they require
  rewrite, not cherry-pick, and are not part of this spec.

## Further Notes

- The conversation trace for this decision is in `notes__branch_diff_review.md`
  (152 lines). It contains the full commit inventory, scope distribution, and
  per-commit risk/value notes that drove the grouping above.
- The change name `sync-origin-main-features-q3-2026` will get a date prefix on
  archive (mirroring `2026-03-04-...` archive convention) once this change is
  merged and verified.
- The `ready-for-agent` triage state is implicit: every Group 1-3 cherry-pick
  is a bounded, mechanical change with a clear test gate; Group 4 and Group 5
  items require planning before an agent picks them up and are flagged
  accordingly.
- If any Group 1 commit lands and breaks `cargo test --workspace`, that is a
  signal to either (a) drop it from Group 1, or (b) escalate to Group 4 as a
  rewrite. Do not silently patch around the failure.

## Appendix A: 2026-08-12 cherry-pick dry-run results

This appendix documents a discrepancy between the original Group 1
assumption ("zero-risk cherry-pick, no overlap with local main's
architectural refactor") and what `git cherry-pick --no-commit` actually
finds. All five remaining Group 1 items (1.2-1.6) have at least one
conflict on local main; only 1.6 admits a partial port that is worth
landing in this change.

### A.1 Outcome summary

| # | Commit         | Scope                          | Dry-run result          | Decision |
|---|----------------|--------------------------------|-------------------------|----------|
| 1.1 | `e88b7e3`    | `feat(cli): add ralph clean --events (#357)` | already landed as manual port in commit `4624750` (verified: cargo check green, 6/6 unit tests, dry-run + delete + clap conflict) | **kept** |
| 1.2 | `db48462`    | `fix: canonicalize Ralph artifact paths` | CONFLICT on `.claude/`, `.gitignore`, `README.md`, multiple `presets/*.yml`, `crates/ralph-cli/presets/*.yml`, `crates/ralph-cli/src/presets.rs`, `crates/ralph-core/data/ralph-tools-tasks.md` (deleted locally), `docs/*` | **rewritten, not cherry-picked** — see A.2 |
| 1.3 | `192f5f9`    | `fix(adapters): drain ACP terminal output before exit` | CONFLICT (modify/delete) on `crates/ralph-adapters/src/acp_executor.rs` — local main deleted this file as part of the architecture migration | **drop, see A.3** |
| 1.4 | `01dd250`    | `fix(api): inline MCP tool schema roots instead of emitting bare $ref (#351)` | CONFLICT (modify/delete) on `crates/ralph-api/src/mcp.rs` — local main deleted this file | **drop, see A.3** |
| 1.5 | `86cfb1a`    | `build: switch Linux dist targets to musl for AL2/AL2023 compatibility` | CONFLICT (content) on `Cargo.toml`, `Cargo.lock`, `crates/ralph-cli/Cargo.toml`, `crates/ralph-cli/src/main.rs` — local main rewrote Cargo manifest and CLI surface | **rewritten, not cherry-picked**, see A.4 |
| 1.6 | `6aacc6b`    | `feat(skills): add ralph-docs skill for llms.txt-driven introspection (#318)` | CONFLICT (modify/delete) on `.claude-plugin/marketplace.json`, `skills/README.md` (both deleted locally); five new files under `skills/ralph-docs/` apply cleanly | **partially landed as `8b27556`** — see A.5 |

### A.2 1.2 / 1.5: paths and build targets already reconciled locally

Local main never had `.ralph/specs/` or `.ralph/tasks/` (they were
single-file spec repositories under `.ralph/specs/`, deprecated and
removed early in this branch in favor of top-level `specs/` and `tasks/`
for openspec-style governance). The same applies to a number of
upstream presets (`autoresearch`, `merge-loop`, `roo`, `amp`-family,
...) that local main has already deleted in favor of a curated
`presets/minimal/` set. `1.2 db48462` overlaps exactly with these
deletions.

`1.5 86cfb1a` overlaps because `crates/ralph-cli/Cargo.toml` and
`crates/ralph-cli/src/main.rs` were rewritten by the `1.1 manual port`
plus local e2e work; switching the build target now requires
recomputing the linux musl x86_64 + aarch64 entries against the local
manifest.

Both items become rewrite tasks; see Group 4 follow-ups §1 and §2.

### A.3 1.3 / 1.4: deleted files mean the upstream fix is moot

`crates/ralph-adapters/src/acp_executor.rs` was deleted by local main's
adapter refactor (which introduced `crates/ralph-adapters/src/job/*`
and the `PromptExecutor` port). The ACP drain fix in `1.3 192f5f9` is
not directly applicable to the new adapter surface — the equivalent
drain logic needs to be re-asserted against the new code path. Until
we audit that, the upstream fix is parked.

`crates/ralph-api/src/mcp.rs` was deleted by local main's API
restructuring (which routes MCP through
`crates/ralph-api/src/mcp_domain.rs`). The MCP schema `$ref`-inlining
fix in `1.4 01dd250` similarly needs to be re-applied to the new file.

Both items become small rewrite tasks; see Group 4 follow-ups §3 and §4.

### A.4 1.5 becomes a separate "build target" change

The musl decision is high-value but tied to a build-system rewrite. The
1.5 commit content (Cargo profiles, mimalloc toggle, cargo-dist
config) is preserved in notes; a new rewrite change will land it with
the local main `Cargo.toml` already in place. See Group 4 follow-up §2.

### A.5 1.6 lands as a partial cherry-pick

Commit `8b27556 feat(skills): add ralph-docs from upstream #318 (partial)`
brings across the five new files under `skills/ralph-docs/`:

- `SKILL.md` — skill definition, trigger conditions, 6-step workflow.
- `agents/openai.yaml` — Codex/OpenAI agent manifest.
- `references/llms-txt-map.md` — subsystem → page shortcut map.
- `references/common-questions.md` — FAQ recipes.
- `references/contributing.md` — crate map, conventions.

Skipped in the same commit (modify/delete conflicts):

- `.claude-plugin/marketplace.json` — local main has never adopted the
  plugin marketplace layout; restoring registration requires a separate
  decision about whether local main wants a plugin layout at all.
- `skills/README.md` — local main routes skills through
  `.claude/skills/` instead of `skills/`; the index file would have to
  be adapted, not copied.

The skill body is independent of plugin registration metadata. Agents
on this branch will discover ralph-docs via the `.claude/skills/`
convention already in place; a future change can re-introduce
`marketplace.json` once the local `.claude-plugin/` policy converges.

### A.6 Re-classification triggers

This appendix supersedes the in-line cherry-pick decisions written
earlier in the Implementation Decisions section. Effective after this
appendix lands:

- Group 1 contains **only 1.1 and 1.6** (both already landed).
- **1.2, 1.3, 1.4, 1.5** move out of Group 1 into Group 4 (rewrite).
- Group 2 keeps its original content (small-risk cherry-picks);
  the next Group 2 candidate should be re-evaluated against the same
  dry-run gate before being attempted.

## Appendix B: 2026-08-12 Group 2 dry-run results

All six Group 2 ("small-risk") items failed `git cherry-pick --no-commit`
on local main. None admits a partial port worth landing in this change.

### B.1 Outcome summary

| # | Commit         | Scope                                    | Dry-run result          | Decision |
|---|----------------|------------------------------------------|-------------------------|----------|
| 2.1 | `0207c8b`    | `fix(event-loop): persist continue state` | CONFLICT (content) on `crates/ralph-cli/src/loop_runner.rs`, `crates/ralph-core/src/event_loop/mod.rs`, `crates/ralph-core/src/event_loop/tests.rs` — local main's `3ff4b47` EventLoop refactor + manual `4624750` CLI port rewrote all three files | **rewrite, Group 4 §5** |
| 2.2 | `c9f2182`    | `fix(cli_executor): harden timeout activity test` | CONFLICT (content) on `crates/ralph-adapters/src/cli_executor.rs` — local main's adapter surface moved | **rewrite, Group 4 §6** |
| 2.3 | `cf0ec8d`    | `test: isolate event history payload fixture` | CONFLICT (content) on `crates/ralph-core/tests/event_loop_ralph.rs` — local main's test surface changed shape | **rewrite, Group 4 §7** |
| 2.4 | `7b673cc`    | `fix(prompts): honor per-hat scratchpad in generated instructions` | CONFLICT (content) on `event_loop/mod.rs`, `instructions.rs`; rename detect on `.ralph/tasks/issue-293-scratchpad.code-task.md` → `tasks/issue-293-scratchpad.code-task.md` (git detected local rename) | **no partial value, skip** |
| 2.5 | `0b61a78`    | `fix(api): deduplicate MCP tool schemas` | CONFLICT (modify/delete) on `crates/ralph-api/src/mcp.rs` — file deleted locally | **rewrite, Group 4 §4 (combine with 1.4)** |
| 2.6 | `4ba3d3a`    | `docs(pi): update pi-coding-agent package name reference` | CONFLICT (content) on `docs/guide/backends.md`; modify/delete on `specs/pi-agent-support/research/05-pi-cli-flags.md` + `specs/pi-agent-support/rough-idea.md` — local main tracked the package migration through openspec instead | **rewrite, Group 4 §8** |

### B.2 Why every Group 2 item conflicted

The pattern repeats Group 1's diagnosis: local main's
`3ff4b47 EventLoop 收窄` and parallel e2e/CLI work touch every file
that Group 2 listed as "small risk". Once an EventLoop/CLI surface
rewrite happens, no upstream test or feature depending on the old
shape can land cleanly. The "small risk" label in the original
proposal was a guess, not a measurement.

The `mcp.rs` situation also recurs (2.5 / Group 4 §4): the file is
deleted locally, so anything modifying it conflicts.

`2.4 7b673cc` is the most interesting — git's rename detection
correctly identified that local main moved `.ralph/tasks/` up to
`tasks/` (matching the spec migration). A pure-rename cherry-pick
of just the new task file would have been technically possible, but
the commit only makes sense in conjunction with the EventLoop and
`instructions.rs` changes that conflict on the same surface; without
those, the task file is orphaned.

### B.3 Re-classification triggers

This appendix supersedes the in-line Group 2 cherry-pick decisions
written earlier. Effective after this appendix lands:

- Group 2 contains **no cherry-pick candidates** — all six move to Group 4 (rewrite).
- Future change proposals that classify cherry-picks by "risk group"
  must include a dry-run gate before assigning a group label.

## Appendix C: 2026-08-12 P3 + P4 audit findings

This appendix consolidates the read-only Group 5 P3 + P4 audits performed
on `8b27556 feat(skills): add ralph-docs from upstream #318 (partial)`
(HEAD at audit time). The full report — including the verbatim −87 / +197
breakdown for `runner.rs` and the file inventory for the deleted
`ralph-api/` crate — lives at `audit-p3-p4.md` in this change's
directory. The findings below are the load-bearing ones that should
travel with this proposal.

### C.1 P3 — `ralph-e2e/src/runner.rs` (#87/#197 reverse diff)

- **Verdict — no functionality loss.** The local −87 lines in
  `runner.rs` are a module re-organisation: the `configure_mock_mode`
  body was rewritten in place, the early "skip + log" branch is replaced
  with a hard-fail "missing cassette = real FAIL" branch (fixing a
  documented false-green class), and a new `persist_e2e_artifacts`
  helper was added to preserve `.e2e/` logs past workspace cleanup.
- **Mock subsystem unchanged in shape.** `crates/ralph-e2e/src/mock.rs`
  holds `MockConfig`, `CassetteResolver`, `CassetteError`, and
  `build_mock_cli_args`. `RunConfig.mock_config` and `with_mock(...)`
  remain on the public surface.

### C.2 F1 — declarative e2e does not import `mock::*`

- **Finding.** `crates/ralph-e2e/src/declarative/scenario.rs` contains
  no `use crate::mock::*` and no `mock_config` field. Imperative
  scenarios can run under `--mock`; declarative (YAML) scenarios cannot.
- **Severity.** Low for now. Today's declarative scenarios are
  small and ride live backends, so this is not on the critical path.
- **Recommended action.** Open a **separate follow-up change**,
  `openspec/changes/declarative-e2e-mock-parity/`, with its own
  proposal and tasks. It is intentionally **not** in scope for this
  change, since the surface (YAML schema + runner wiring) is a
  different concern from the upstream cherry-pick synchronisation we
  are tracking here.

### C.3 P4 — `ralph-api/src/main.rs` (proposal said: 22-line audit)

- **Audit scope was too narrow.** Local main did not delete `main.rs`
  in isolation: it deleted the entire `crates/ralph-api/` crate —
  17 source files (`auth.rs`, `config.rs`, `event_watcher.rs`,
  `idempotency.rs`, `loop_domain.rs`, `mcp.rs`, `planning_domain.rs`,
  `preset_domain.rs`, `protocol.rs`, `robot_domain.rs`, `runtime.rs`,
  `task_domain.rs`, `transport.rs`, and the binary/library entry
  points) plus subtrees `collection_domain/`, `runtime/`,
  `stream_domain/`, `task_domain/`, the `data/rpc-v1-*.json`
  artifacts, and the `tests/` integration suite.
- **Verdict — no capability loss in this repo.** `git grep 'ralph_api::'`
  on local main returns zero matches. The CLI in this branch runs
  ralph in-process and has no consumer of an HTTP API.
- **Replacement architecture:**
  - `ralph-proto` holds the cross-crate protocols (event, bus, topic,
    hat, gate, routing, error).
  - `ralph-cli` consumes `ralph-proto` directly.
  - `ralph-e2e` runs `ralph` as a subprocess; never reaches for HTTP.

### C.4 F2 — drop Group 4 rewrites that targeted `ralph-api` files

Two Group 4 rewrite tasks are no longer meaningful because their
target files (inside `ralph-api/`) do not exist on local main:

- `4.4` — Rewrite `01dd250` (inline MCP schema roots) — **dropped**.
  The destination file was `mcp.rs`, which was deleted with the
  whole crate. Re-introducing ralph-api (or a successor HTTP API)
  is a separate decision that is out of scope for this change.
- `4.15` — Rewrite `0b61a78` (dedupe MCP tool schemas, combined
  with 1.4) — **dropped**, same reason.

Other Group 4 entries (4.1, 4.2, 4.3, 4.5–4.14) target files that
do exist locally and remain valid rewrite tasks.

### C.5 Implication for this change

After Appendix C:

- The cherry-pick-able surface in Group 1-3 is fully exhausted
  (Group 1: 2 of 6, Group 2: 0 of 6, Group 3: pending dry-run).
- The remaining work is **rewrites** (Group 4) and **local patches**
  (Group 5, with P3 + P4 now audited and P1 / P2 / P5 / P6 still open).
- The F1 follow-up (`declarative-e2e-mock-parity`) lives in its own
  change, not in `tasks.md` here.

This change can therefore close once:

- P5 (`.ralph/specs/` ↔ `specs/` reconcile) is done,
- Group 4 §4.5–4.10 + §4.14 are explicitly handed off to a follow-up,
- the release tag (P6) is bumped past the integrated Group 1 commits.
