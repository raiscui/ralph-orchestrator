# Cherry-pick upstream sync instead of merge

## Status

Accepted 2026-08-15. Triggered by sync/origin-v2.10.1.

## Context

The fork (`my/main`, 209 commits ahead of `origin/main`) has diverged symmetrically: 596 source files modified on both sides, ~109K lines added/removed on each. A naive `git merge origin/main` would create a single merge commit resolving ~596 conflicts at once, with no per-PR validation surface and no way to selectively accept upstream changes. Re-basing would rewrite already-published history.

## Decision

For each upstream release (e.g., v2.10.1), open a feature branch `sync/origin-vX.Y.Z`, cherry-pick the chosen PRs one at a time (squash each into a single commit whose body indexes the upstream `#PR` number), resolve conflicts per-case with no blanket "local wins" or "upstream wins" rule, run `cargo test --lib -j 2` per cherry-pick and `cargo test --workspace` per wave, push the feature branch to `my/main` once per wave (4-5 pushes per release), then squash-merge into `main` and delete the feature branch.

## Considered Options

- **Direct merge of `origin/main` into `my/main`.** Rejected: 596 conflicts in one shot, no per-PR validation surface, merge commit pollutes `my/main`'s linear history.
- **Rebase `my/main` onto `origin/main`.** Rejected: rewrites already-pushed history, violates the "no rewriting pushed commits" rule.
- **Stop following upstream entirely; cherry-pick only individual fixes as needed.** Rejected: long-term drift, our fork loses the compatibility window with upstream releases.

## Consequences

- Upstream-only features (Forge CLI backend, robot RPC domain, telemetry context-window tracking, hat imports, remote review branches) are intentionally excluded from each sync. Each feature needs its own grill when/if adopted.
- Each cherry-pick is independently revertable. A bad PR can be reverted without unwinding the whole sync.
- The 14 cherry-picks per release become 4-5 push events, easier to review and reason about than 14 separate small pushes.
- Conflict resolution is per-case and documented in the squash commit body when non-obvious.

## Verified outcomes (2026-08-15 sync/origin-v2.10.1)

After dry-running all candidate cherry-picks on the v2.10.1 baseline:

- **Three upstream PRs confirmed un-cherry-pickable** because the destination files no longer exist on local main, replaced by the architectural migration captured in `3ff4b47 EventLoop 收窄 + PromptExecutor port`:
  - `01dd250` / `0b61a78` (MCP schema `$ref` inline + dedup) — `crates/ralph-api/src/mcp.rs` deleted locally as part of the `ralph-api/` whole-crate deletion; equivalent functionality needs to be re-asserted against the new `ralph-proto` crate.
  - `25afeb0` (local hat imports in preflight) — `crates/ralph-cli/src/preflight.rs` deleted; equivalent needs to land in the new preflight surface.
  - `d631ef7` (context-window telemetry) — `crates/ralph-adapters/src/{acp_executor,claude_stream,json_rpc_handler,pi_stream,stream_handler}.rs` all deleted; telemetry needs to be re-derived against the `job/{app_server,headless,mcp}.rs` adapter surface.
- **One upstream PR (#357 ralph clean --events)** already landed via manual port commit `4624750` with explicit reasoning recorded in that commit's body. This validated the strategy decision: manual port is preferred over cherry-pick when upstream's refactor preconditions (here: `load_config_with_overrides` and `RalphConfig.core.scratchpad` as struct) are not yet on local main.
- **Group 2 (small-risk) and Group 3 (medium-risk)** items remain per-case candidates; each touched files local main has rewritten (especially `event_loop/*`), so per-PR resolution cost is high. Re-evaluate each cherry-pick's value-vs-cost before pursuing.
- **Group 4 §15 (4a38b8d Claude stream wait)** dropped 2026-08-17: origin uses `StreamEvent` enum + `line_signals_event_emitted` / `post_event_deadline` logic that local `(StreamKind, line)` tuple architecture does not have. Porting = inventing the bug + adding 60+ lines for Claude stream JSON, which local main does not exercise. Per "改良胜过新增", DROP. Documents the strategy decision: per-PR cherry-pick value-vs-cost must include a local-architecture-fit check, not just conflict resolution feasibility.
