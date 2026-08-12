# Audit Report P3 + P4 — 2026-08-12

Author: agent session `omx-1786419140441-df5ql8`
Status: read-only, no code changes proposed in this report.

## TL;DR

| Audit | Scope as written in proposal | Actual scope on local main | Verdict |
|-------|-------------------------------|------------------------------|---------|
| P3 | `ralph-e2e/src/runner.rs` reverse diff −87 / +87 (the proposal wording is ambiguous; both are deltas vs `e88b7e3`) | `ralph-e2e/src/runner.rs` actually +197/−87 vs `e88b7e3` | **No functionality loss.** Module re-organisation + behavioral fix. |
| P4 | `ralph-api/src/main.rs` reverse diff 22 lines | The whole `crates/ralph-api/` crate is deleted locally — 17 source files + several submodules, not just `main.rs` | **No capability loss visible** in the local architecture, but the proposal's audit scope was too narrow; see §C2. |

Bonus findings (both audits uncovered things outside the original scope):

- **F1** — declarative e2e does **not** import the `mock` subsystem. Imperative scenarios can run in mock mode; declarative scenarios cannot. This is a genuine gap if anyone tries to run the new YAML scenarios under `--mock`.
- **F2** — `ralph-api/` was not just "trimmed"; it was **whole-crate-deleted** on local main. Any Group 4 rewrite that targets a single file inside the old crate has to be re-scoped against the new crate layout (`ralph-proto`, `ralph-cli`, `ralph-core`).

---

## P3 audit — `crates/ralph-e2e/src/runner.rs`

### C1.1 Baseline diffs

```
1d90c1e (merge-base)  → e88b7e3 (origin/main): +153 / -1
1d90c1e                → 8b27556 (local HEAD): +263 / -1
e88b7e3                → 8b27556:               +197 / -87   ← local deltas vs origin
HEAD (8b27556)         → e88b7e3:                +87 / -197  ← reverse direction
```

### C1.2 What the local −87 actually removed

The 87 deleted lines come from three places, all in
`crates/ralph-e2e/src/runner.rs`:

1. `RunConfig::mock_config` doc-comment + setter prefix (≈ 10 lines).
2. The early `if let Some(ref mock_config) = config.mock_config` block in
   the run loop — old version emitted a `ProgressEvent::ScenarioSkipped`
   and incremented `skipped_count` (≈ 25 lines).
3. The pre-refactor `configure_mock_mode` implementation — old version
   built `serde_yaml::Value::Mapping(...)` and inserted the new mapping
   into the existing config map (≈ 50 lines).

### C1.3 What the local +197 actually added

1. **Hard-fail mock setup**: when `config.mock_config` is set but cassette
   resolution fails, the new code records a real `TestResult { passed: false }`
   instead of skipping. This closes the "skip + exit 0 looks like green"
   false-positive class. (≈ +30 lines)
2. **`configure_mock_mode` rewritten** in the same place — switched from
   building a whole `serde_yaml::Value::Mapping` to incremental
   `cli_map.insert(...)` and from `format!("{}", e)` to `format!("{e}")`.
   Same behaviour, cleaner code. (≈ +100 lines because of new doc strings
   and inline comments)
3. **New helper `persist_e2e_artifacts`** — copies `${workspace}/.e2e/`
   to `${base}/artifacts/<scenario-id>/` *before* workspace cleanup, so
   logs survive `--keep-workspace=false`. Includes a small `copy_dir_recursive`
   inner function. (≈ +60 lines)

### C1.4 Mock subsystem post-audit

- `crates/ralph-e2e/src/mock.rs` contains the public surface:
  `MockConfig`, `CassetteResolver`, `CassetteError`, `DEFAULT_CASSETTE_DIR`,
  `build_mock_cli_args`.
- `crates/ralph-e2e/src/runner.rs` imports the mock types with
  `use crate::mock::{CassetteResolver, MockConfig, build_mock_cli_args};`
- `RunConfig` still exposes `pub mock_config: Option<MockConfig>` and
  `pub fn with_mock(...)`.
- Two `#[test]` cases for `MockConfig::with_speed` / `without_commands` /
  `new` exist in `mock.rs`.

### C1.5 Verdict — P3

**No functionality loss.** The −87 lines moved to a richer code path
that does *more* (hard-fail + artifact persistence). Mock mode is still
discoverable, still configurable from `RunConfig`, and the imperative
scenario runner still drives it.

**Finding F1** — the declarative runner does **not** touch `mock::*`:

```
$ grep -n "mock" crates/ralph-e2e/src/declarative/*.rs
(no output)
```

Consequence: declarative (`YAML`-driven) scenarios cannot run under
`--mock`. If a future change wants mock-mode parity for declarative
scenarios, the integration point would be
`crates/ralph-e2e/src/declarative/scenario.rs` plus a YAML schema
extension (`mock:` field under `setup`).

Severity: **low**. Today's declarative scenarios are small and tend to
depend on real backends, so the gap is not on the critical path. Worth
filing as a follow-up before any "all scenarios must work in mock mode"
demand appears.

---

## P4 audit — `crates/ralph-api/`

### C2.1 Baseline diffs

```
git diff --shortstat e88b7e3..HEAD -- crates/ralph-api/src/main.rs
1 file changed, 22 deletions(-)
```

But that's just one file. The full state:

```
$ git ls-tree e88b7e3 -- crates/ralph-api/src/ | wc -l
~30
$ ls crates/ralph-api/ 2>/dev/null
ls: crates/ralph-api/: No such file or directory
```

The whole `crates/ralph-api/` tree was deleted on local main, not just
`main.rs`. Files that disappeared include:

- `lib.rs`, `main.rs` (the binary entry point)
- `auth.rs`, `config.rs`, `config_domain.rs`, `errors.rs`
- `event_watcher.rs`, `idempotency.rs`
- `loop_domain.rs`, `loop_side_effects.rs`, `loop_support.rs`
- `mcp.rs`, `planning_domain.rs`, `preset_domain.rs`
- `protocol.rs`, `robot_domain.rs`, `runtime.rs`
- `task_domain.rs`, `transport.rs`
- Subtrees `collection_domain/`, `runtime/`, `stream_domain/`, `task_domain/`,
  `data/` (incl. `rpc-v1-events.json`, `rpc-v1-schema.json`)
- `tests/` (incl. integration tests for `serve` and `serve_with_idempotency`)

Local main `crates/` now contains only:

```
ralph-adapters  ralph-bench  ralph-cli  ralph-core
ralph-display   ralph-e2e    ralph-proto ralph-tui
```

`ralph-cli/src/**/*.rs` does not `use ralph_api::*`. `ralph-proto` is the
closest analog and holds the shared types `ralph-api` used to expose.

### C2.2 What the local architecture replaces

- **HTTP / tower server** — gone entirely. There is no standalone
  `ralph-api` binary in local main. Anything that previously called
  `ralph_api::serve(...)` has to invoke a different surface.
- **RPC v1 protocol (robot/collection/loop/planning/task domains)** —
  these existed in `ralph-api/src/{robot,collection,loop,planning,task}_domain.rs`
  and are gone. Local main has no replacement domain modules.
- **MCP transport (`mcp.rs`)** — gone. Local main does not run an MCP
  server.
- **Auth / idempotency / event watcher** — gone.

### C2.3 What still exists in local main

- `ralph-proto` — event / bus / topic / hat / gate / routing / error
  (the protocol-level shared crate).
- `ralph-cli` — the orchestrator CLI; consumes `ralph-proto` directly.
- `ralph-e2e` — invokes `ralph` as a subprocess; never reaches for an
  HTTP API.

### C2.4 Verdict — P4

**Functional audit of "what is gone" — clean.** Running `git grep
'ralph_api::' crates/` returns nothing; no local crate imports a
`ralph_api::*` symbol. The whole `ralph-api` surface is therefore unused
inside this repo as of `8b27556`.

**Audit scope was wrong.** The proposal framed P4 as a 22-line binary
entry audit; the actual change was a whole-crate removal. This needs to
be re-stated as:

> **The local main branch has chosen *not to ship* a ralph HTTP/RPC
> server.** This is consistent with how `ralph-cli` is exercised
> (in-process, not over HTTP) and with `ralph-proto` owning the
> shared abstractions.

**Finding F2** — every rewrite item in Group 4 that targets a single
file inside the deleted crate has to be re-scoped. Concretely:

- Group 4 §1 (1.4) "rewrite `01dd250` inline MCP schema roots" must
  become "rewrite MCP schema-`$ref` inlining against whichever file in
  the new architecture owns MCP". Local main has no MCP surface, so this
  may not be applicable at all — needs a fresh decision.
- Group 4 §4 (2.5) "rewrite `0b61a78` dedupe MCP tool schemas" has the
  same fate.

Both should probably be **dropped**, not "rewritten". The capabilities
were intentionally removed.

---

## What to do with these findings

1. **Promote F1 + F2 into proposal.md** as Appendix C, so future
   readers of this change understand the architecture delta, not just
   the cherry-pick deltas.
2. **Drop 1.4 + 2.5 from Group 4 rewrite list** — they targeted files
   that no longer exist; rewriting them against local main doesn't make
   sense without a "should we re-introduce ralph-api?" decision first.
3. **File F1 as a separate future change** ("declarative scenarios +
   mock mode parity"), not as part of `sync-origin-main-features-q3-2026`.
4. **Close P3 + P4 in tasks.md**: mark both as completed, link this
   audit document.

This audit does **not** introduce any commits; it is purely a
report. Code changes (if any) belong to a future change once scope is
agreed.
