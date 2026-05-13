# Source Map

This page is the anti-hallucination map for the documentation site.
If a claim is not grounded here, treat it as a candidate for follow-up rather
than as confirmed current behavior.

## Project Method

| Claim | Primary source |
| --- | --- |
| Ralph tenets define the operating philosophy | `AGENTS.md` |
| Specs should be read before implementation | `AGENTS.md`, `specs/README.md` |
| Backpressure should reject bad work | `AGENTS.md`, `crates/ralph-core/src/event_loop/mod.rs` |
| Record sessions are evidence | `AGENTS.md`, `crates/ralph-cli/src/record_cli.rs` |

## Workspace Structure

| Claim | Primary source |
| --- | --- |
| Workspace has seven crates | `Cargo.toml` |
| CLI command definitions live in `ralph-cli` | `crates/ralph-cli/src/main.rs` |
| Configuration model lives in `ralph-core` | `crates/ralph-core/src/config.rs` |
| Event parsing lives in `ralph-core` | `crates/ralph-core/src/event_parser.rs` |
| Parallel supervisor lives in `ralph-core` | `crates/ralph-core/src/parallel/supervisor.rs` |

## Specs Used For This Site

| Topic | Source |
| --- | --- |
| In-band vs out-of-band event channels | `specs/parallel-event-channels.spec.md` |
| Hat instances and parallel runtime | `specs/parallel-hat-instances.spec.md` |
| Record-session wiring | `specs/parallel-record-session.spec.md` |
| Completion promise semantics | `specs/completion-promise-guardrail.spec.md` |
| v1 to v2 simplification posture | `specs/v1-v2-feature-parity.spec.md` |
| Memories requirements | `specs/ralph-memories/requirements.md` |
| Diagnostics design | `specs/diagnostics/summary.md` |
| E2E harness | `specs/e2e-testing/summary.md` |

## External Research

| Topic | Source |
| --- | --- |
| Original Ralph Wiggum technique | <https://ghuntley.com/ralph/> |
| GitHub Pages custom workflows | <https://docs.github.com/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages> |
| MkDocs deployment model | <https://www.mkdocs.org/user-guide/deploying-your-docs/> |
| Material for MkDocs publishing | <https://squidfunk.github.io/mkdocs-material/publishing-your-site/> |

## Current Caveats

- Some older docs files remain in `docs/` but are excluded from this fresh site
  build. They are preserved to avoid deleting unrelated local work.
- The memory documentation is in a migration period. `.agent/memories.md` is the
  compatibility baseline; scoped experience files are an active direction in
  the current worktree and should not be described as universally complete
  without checking the latest code.
- README installation text and badges may diverge on exact minimum Rust version.
  This site avoids pinning a Rust version beyond requiring a toolchain capable
  of building the edition 2024 workspace.
