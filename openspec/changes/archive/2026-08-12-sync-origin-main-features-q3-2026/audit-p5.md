# Audit Report P5 — 2026-08-13

Author: agent session `omx-1786419140441-df5ql8`
Status: read-only; no code changes proposed.

## TL;DR

P5 ("Reconcile `.ralph/specs/` ↔ `specs/`") is **already complete** on
local main as of `472fd92 chore(openspec): archive two changes…`. No
follow-up code work is needed; this audit just records the
ground-truth and closes the task.

| Probe | Result |
|-------|--------|
| `ls .ralph/specs/` | absent — local main deleted it in the early merge-base window |
| `ls .ralph/tasks/` | absent — same path; tasks moved to top-level `tasks/` |
| `ls specs/` | 160 files, git-tracked canonical |
| `ls tasks/` | present, git-tracked, replaces `.ralph/tasks/` |
| `grep -rE '\.ralph/(specs\|tasks)' .` (excluding `.scratch/`) | only matches inside historical archive proposal/tasks notes and in the agent's own analysis notes |

## Static evidence

```
$ ls -la .ralph/ | grep -E 'specs|tasks'
(no output — both directories are gone)

$ ls specs/ | wc -l
160

$ ls tasks/ | head -5
add-copilot-backend.code-task.md
add-event-validation-backpressure.code-task.md
add-opencode-backend-adapter.code-task.md
add-ralph-emit-command.code-task.md
add-task-frontmatter-tracking.code-task.md
```

`.ralph/` therefore remains, but only as the **runtime scratchpad**
the project itself defines:

```
$ ls .ralph/
.DS_Store            events-20260517-075207.jsonl   events-20260523-020131.jsonl
agents.json          events-20260517-075535.jsonl   events.jsonl
capability-invocations/  events-20260517-110652.jsonl  evidence-index.jsonl
current-events       events-20260517-114307.jsonl   record-session.latest
events-20260218-085712.jsonl  events-20260517-141938.jsonl   tui_chinese_custom.yml
…
```

This is the correct division: `specs/` and `tasks/` are
git-versioned; `.ralph/` is the agent runtime state.

## Dynamic evidence — leftover references

```
$ grep -rE '\.ralph/specs|\.ralph/tasks' --include='*.md' --include='*.yml' \
       --include='*.toml' . 2>/dev/null | grep -v '\.scratch/'
./notes__branch_diff_review.md:        (three occurrences, all in 2026-08-12
                                        analysis noting that origin/main
                                        keeps `.ralph/specs/` while local
                                        main does not)
./notes__group1_dryrun.md:             (one historical reference inside
                                        a candidate-path bullet)
./openspec/changes/archive/2026-08-12-sync-origin-main-features-q3-2026/proposal.md:
                                       (one user-story + one P5 description,
                                        both describing what *was* true on
                                        origin/main)
./openspec/changes/archive/2026-08-12-sync-origin-main-features-q3-2026/tasks.md:
                                       (one task entry 5.5)
```

There is **no live** code or doc that points a reader at
`.ralph/specs/` or `.ralph/tasks/` for read or write. Every remaining
match is in historical artefacts describing the contrast between
origin/main and local main — those should stay in the archive.

## Verdict

**P5 is done by virtue of local main's pre-existing rewrite.** The
proposal asked us to "pick one as canonical"; local main picked
`specs/` (and `tasks/` for the sibling concern) early, with no
follow-up needed.

## Closing recommendation

- Mark `tasks.md` 5.5 as completed.
- This audit lives next to `audit-p3-p4.md` so a future reader of the
  archived change can see all five Group 5 audit / close-out entries
  in one place.
- Do **not** create a `.ralph/specs` symlink or back-port the runtime
  layer to use `specs/` paths — the runtime `.ralph/` exists for
  scratchpad reasons that the rest of the repo already accepts.
