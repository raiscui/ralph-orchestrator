# Group 3 dry-run log (2026-08-17)

> Follow-up to `tasks.md` Group 3, after minimax-full-auto-compat closed (e2977175 + 7eb270f2 + 6fa0075e)
> on `my/main` (HEAD `6fa0075e`). Each Group 3 commit was dry-runned with
> `git cherry-pick --no-commit <sha>` and reset with `git reset --hard HEAD`.

## Result: ALL 5 CONFLICT

| ID | SHA | Subject | Conflict files | Decision |
|---|---|---|---|---|
| 3.1 | `4a38b8d` | fix(adapters): wait for Claude stream result events (#355) | 2 (cli_executor.rs, event_loop/mod.rs) | Move to Group 4 §15 |
| 3.2 | `ee9fa67` | feat(cli): opt-in hats validate --instructions checks (#356) | 2 (hats.rs, cli-reference.md) | Already landed as manual port (commit `620411ce` parent) — DROP |
| 3.3 | `25afeb0` | feat(hats): support local hat imports in preflight | 3 (preflight.rs, integration_preflight.rs, `.ralph/` → `tasks/` rename) | Move to Group 4 §16 |
| 3.4 | `a4b6d45` | fix(runtime): require explicit completion after guidance (#326) | 5 (event_loop/loop_state.rs, event_loop/mod.rs, event_loop/tests.rs, hatless_ralph.rs, summary_writer.rs) | Move to Group 4 §17 |
| 3.5 | `d631ef7` | feat(telemetry): track context window utilization | 16 (massive — adapters/* x5, event_loop/* x3, summary_writer, config, loop_runner, json_rpc_handler, lib, frontend builder, specs, tasks) | Move to Group 4 §18 |

## Notes

- **3.2** (`ee9fa67`) was already manually ported as part of `fix/completion-via-event`
  branch work (commit `620411ce` / `620411c` parent `99ebe5dd`). The conflict here
  reflects that the manual port is already on `my/main`. **DROP** — no further work.

- **3.5** (`d631ef7`) is the most invasive: 16 files, includes front-end (React
  component), proto changes (json_rpc), and runtime summary changes. Per-case
  resolution is impractical — full rewrite needed against current main architecture.

- **3.1 / 3.3 / 3.4** are per-case resolvable but require audit. Same pattern as
  the Group 2 → Group 4 §5-§8 rewrites in the existing Q3 plan.

## Verification

- All dry-runs done on scratch branch `q3-grp3-dryrun-2026-08-17` (deleted after)
- `git reset --hard HEAD` after each dry-run confirmed working tree clean
- HEAD before/after: `6fa0075e` (unchanged)

## Decision summary

- Group 3 全部 → Group 4 rewrite
- Group 4 累计: §15 (3.1) + §16 (3.3) + §17 (3.4) + §18 (3.5) — 4 个新 rewrite task
- Group 3.2 (ee9fa67): DROP (already landed)

## Next steps

- Re-evaluate per-case resolution vs full rewrite in a separate change (out of scope here)
- P6 (release bump 5.6) remains pending — independent decision, see `tasks.md`
- Group 4 §15-§18 are blockers for Q3 plan closure
