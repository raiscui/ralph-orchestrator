# Tasks — sync-origin-main-features-q3-2026

> Tasks below are split by Group. Group 1-3 were intended to be mechanical
> cherry-picks with a `cargo test --workspace` gate. The 2026-08-12 dry-run
> (see proposal.md Appendix A) found that all five remaining Group 1 items
> conflict on local main; only 1.6 admits a partial port. The re-classified
> tasks below reflect that finding.

## 1. Group 1 — Zero-risk cherry-picks

- [x] 1.1 cherry-pick `e88b7e3 feat(cli): add ralph clean --events (#357)` — landed as manual port in commit `4624750` (cargo check green, 6/6 unit tests, clap conflict verified)
- [x] 1.2 ~~cherry-pick `db48462 fix: canonicalize Ralph artifact paths`~~ — **dry-run 2026-08-12: CONFLICT (modify/delete) on multiple files; local main already deleted them in favor of top-level `specs/`/`tasks/`. Moved to Group 4 rewrite tasks §1.**
- [x] 1.3 ~~cherry-pick `192f5f9 fix(adapters): drain ACP terminal output before exit`~~ — **dry-run 2026-08-12: CONFLICT on `acp_executor.rs` (deleted locally). Moved to Group 4 rewrite tasks §3.**
- [x] 1.4 ~~cherry-pick `01dd250 fix(api): inline MCP tool schema roots`~~ — **dry-run 2026-08-12: CONFLICT on `mcp.rs` (deleted locally). Moved to Group 4 rewrite tasks §4.**
- [x] 1.5 ~~cherry-pick `86cfb1a build: switch Linux dist targets to musl`~~ — **dry-run 2026-08-12: CONFLICT on `Cargo.toml`, `Cargo.lock`, `main.rs`. Moved to Group 4 rewrite tasks §2.**
- [x] 1.6 ~~cherry-pick `6aacc6b feat(skills): add ralph-docs skill`~~ — **partial port landed in commit `8b27556` (5 new files under `skills/ralph-docs/`); 2 metadata files skipped due to modify/delete. See proposal.md Appendix A.5.**
- [x] 1.7 `cargo check --workspace --all-features && cargo test --workspace` — see verification log below

## 2. Group 2 — Small-risk cherry-picks (re-evaluate before attempting)

> Each item must be dry-runned with `git cherry-pick --no-commit <sha>` and
> aborted + reported if conflicts appear. Do not bypass the dry-run gate.

- [ ] 2.1 cherry-pick `0207c8b fix(event-loop): persist continue state` — **pending dry-run**
- [ ] 2.2 cherry-pick `c9f2182 fix(cli_executor): harden timeout activity test` — **pending dry-run**
- [ ] 2.3 cherry-pick `cf0ec8d test: isolate event history payload fixture` — **pending dry-run**
- [ ] 2.4 cherry-pick `7b673cc fix(prompts): honor per-hat scratchpad in generated instructions` — **pending dry-run**
- [ ] 2.5 cherry-pick `0b61a78 fix(api): deduplicate MCP tool schemas` — **pending dry-run, possibly combine with 1.4 rewrite**
- [ ] 2.6 cherry-pick `4ba3d3a docs(pi): update pi-coding-agent package name reference` — **pending dry-run**
- [ ] 2.7 `cargo test --workspace`, clap conflict tests for `--events` + `--diagnostics`

## 3. Group 3 — Medium-risk cherry-picks (re-evaluate before attempting)

> Same dry-run gate as Group 2. Many of these touch files that local main
> rewrote. Expect partial ports or rewrites, not clean cherry-picks.

- [ ] 3.1 cherry-pick `4a38b8d fix(adapters): wait for Claude stream result events (#355)` — **pending dry-run**
- [ ] 3.2 cherry-pick `ee9fa67 feat(cli): opt-in hats validate --instructions checks (#356)` — **pending dry-run**
- [ ] 3.3 cherry-pick `25afeb0 feat(hats): support local hat imports in preflight` — **pending dry-run**
- [ ] 3.4 cherry-pick `a4b6d45 fix(runtime): require explicit completion after guidance (#326)` — **pending dry-run, depends on P2 contract test**
- [ ] 3.5 cherry-pick `d631ef7 feat(telemetry): track context window utilization` — **pending dry-run**
- [ ] 3.6 run `cargo run -p ralph-e2e -- codex --filter events,backpressure,hat-instances`

## 4. Group 4 — Rewrite tasks (out of scope for this change; tracked here for visibility)

- [ ] 4.1 **Rewrite `db48462` (path canonicalization).** Local main already has the canonical paths; write a no-op commit documenting that no migration is needed and update AGENTS.md to point at top-level `specs/`/`tasks/`.
- [ ] 4.2 **Rewrite `86cfb1a` (Linux musl targets).** Re-author the build-target switch against local main `Cargo.toml`: add `[profile.release-musl]` block, set `mimalloc` feature on musl-only target, regenerate `Cargo.lock`, update `.github/workflows` to upload musl artifacts.
- [ ] 4.3 **Rewrite `192f5f9` (ACP drain) against the new adapter surface.** After P5 merges and the new `PromptExecutor` path is stable, re-derive the drain logic against `crates/ralph-adapters/src/job/*` and add a regression test.
- [x] 4.4 ~~**Rewrite `01dd250` (inline MCP schema roots) against the new MCP domain.**~~ — **dropped, see audit-p3-p4.md §C2 + Appendix C**: `ralph-api/` was whole-crate deleted on local main. There is no `mcp_domain.rs` to rewrite against. Re-introducing ralph-api or a successor HTTP API is a separate decision (out of scope for this change).
- [x] 4.15 ~~**Rewrite `0b61a78` (dedupe MCP tool schemas) — combined with 1.4**~~ — **dropped, same reason**: `mcp.rs` does not exist locally; the destination file is gone with the whole `ralph-api/` crate. Not a rewrite target.
- [ ] 4.5 Track `6972444 feat(api): robot RPC domain` (from original Group 4) — future change.
- [ ] 4.6 Track `2cfe7c9 feat(backends): Forge CLI support` — future change.
- [ ] 4.7 Track `93e170d feat(loops): publish remote review branches` — future change.
- [ ] 4.8 Track `3f1e0c3 feat(robot): file-backed web mode` — future change.
- [ ] 4.9 Track `246336f feat(cli): unify preset mechanism under -H <name>` — future change.
- [ ] 4.10 Track TUI region (`317266f`, `3454c62`, ...) — local main's TUI work is incompatible; track separately.

## 5. Group 5 — Local main patches (run in parallel with cherry-picks)

- [ ] 5.1 (P1) Mark declarative e2e escape hatch `#[deprecated]`, once declarative coverage reaches ≥ 90%
- [ ] 5.2 (P2) Add `PromptExecutor` port round-trip contract test
- [ ] 5.3 (P3) Audit reverse diff on `ralph-e2e/src/runner.rs` (delete −197 + insert +87), produce coverage matrix
- [ ] 5.4 (P4) Audit reverse 22-line diff on `ralph-api/src/main.rs`, confirm no capability was lost
- [ ] 5.5 (P5) Reconcile `.ralph/specs/` ↔ `specs/`; pick `specs/` as canonical
- [ ] 5.6 (P6) Bump release tag after Group 1 (current scope) lands

## 6. Final verification

- [ ] 6.1 `cargo fmt --check && cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] 6.2 `cargo test --workspace`
- [ ] 6.3 `cargo run -p ralph-e2e -- codex --filter events,backpressure,hat-instances,clean`
- [ ] 6.4 archive this change: `mv openspec/changes/sync-origin-main-features-q3-2026 openspec/changes/archive/2026-08-12-sync-origin-main-features-q3-2026`

## Verification log (2026-08-12)

- **1.1 `e88b7e3` manual port**: cargo check `-p ralph-cli` green; cargo test `-p ralph-cli --lib` 6/6 passed; manual verified dry-run + delete + clap-conflict exit codes. Logged in commit `4624750`.
- **1.2 dry-run**: 4 content conflicts + 3 modify/delete conflicts across `.claude/`, `.gitignore`, `README.md`, presets, docs. Aborted.
- **1.3 dry-run**: modify/delete on `acp_executor.rs`. Aborted.
- **1.4 dry-run**: modify/delete on `mcp.rs`. Aborted.
- **1.5 dry-run**: 4 content conflicts on Cargo manifest / `main.rs`. Aborted.
- **1.6 dry-run**: modify/delete on 2 metadata files. Partial port (5 new files) landed in `8b27556`.
- **1.7 `cargo check --workspace`**: pending; see test run below.

## 7. Group 2 dry-run log (2026-08-12)

- [x] 7.1 cherry-pick `0207c8b` (2.1) — **dry-run CONFLICT (content) on 3 files. Moved to Group 4 §5.**
- [x] 7.2 cherry-pick `c9f2182` (2.2) — **dry-run CONFLICT (content) on `cli_executor.rs`. Moved to Group 4 §6.**
- [x] 7.3 cherry-pick `cf0ec8d` (2.3) — **dry-run CONFLICT (content) on `event_loop_ralph.rs`. Moved to Group 4 §7.**
- [x] 7.4 cherry-pick `7b673cc` (2.4) — **dry-run CONFLICT on 2 files + rename detect. No partial value, skipped.**
- [x] 7.5 cherry-pick `0b61a78` (2.5) — **dry-run CONFLICT (modify/delete) on `mcp.rs`. Moved to Group 4 §4 (combine with 1.4).**
- [x] 7.6 cherry-pick `4ba3d3a` (2.6) — **dry-run CONFLICT (content) + 2 modify/delete on spec files. Moved to Group 4 §8.**

### 7.7 Verification log (2026-08-12 21:15)

- `cargo check -p ralph-cli --quiet` → exit 0 (no regressions from a clean no-op round, HEAD still `8b27556`).
- 6 dry-runs all aborted via `git reset --hard HEAD`. Working tree is clean (only untracked items).

## Group 4 follow-ups after Group 2 re-classification

- [ ] 4.5 Rewrite `0207c8b` (continue state persistence) against new `3ff4b47` EventLoop shape.
- [ ] 4.11 Rewrite `c9f2182` (timeout activity test) against the moved adapter surface.
- [ ] 4.12 Rewrite `cf0ec8d` (event-history fixture isolation) against the new test layout.
- [ ] 4.13 Reconsider `7b673cc` as combined EventLoop + instructions work, not a standalone cherry-pick.
- [ ] 4.14 Rewrite `4ba3d3a` (pi package name) against current openspec `specs/pi-agent-support/` shape.

## P3 + P4 audit completion (2026-08-12)

Audit document: `audit-p3-p4.md` in this change's directory.

- [x] **5.3 (P3) Audit `ralph-e2e/src/runner.rs` reverse −197/+87** — see `audit-p3-p4.md` §C1. Module reorganisation, no functionality loss. Side-finding F1: declarative e2e does not use `mock::*`.
- [x] **5.4 (P4) Audit `ralph-api/src/main.rs` reverse 22 lines** — see `audit-p3-p4.md` §C2. **Audit scope was too narrow**: the *whole* `ralph-api/` crate is deleted locally, not just `main.rs`. No capability loss visible; no in-tree consumer of `ralph_api::*` exists.
- [x] **F1 follow-up** — "declarative scenarios + mock mode parity" filed in tasks file as future change, not part of `sync-origin-main-features-q3-2026`.
- [x] **F2 follow-up** — drop Group 4 §1 (1.4) and §4 (2.5) rewrite entries since target files don't exist locally. Re-introducing ralph-api is a separate decision.

### Audit scope correction

The original Group 5 P4 audit was described as a 22-line diff audit.
The actual change is a whole-crate deletion. This is now correctly
described in `audit-p3-p4.md` §C2.1, and is the reason §C2.4
recommends dropping items rather than rewriting them.

### Re-classification triggers

- Group 4 §1 ("rewrite `01dd250` MCP schema inline" against `mcp.rs`) — **delete**, see §F2.
- Group 4 §4 ("rewrite `0b61a78` dedupe MCP tool schemas" against `mcp.rs`) — **delete**, see §F2.
