# Start

This page is the shortest reliable path from a clone to a Ralph run.
It is grounded in the current README, CLI source, and repository build notes.

## Prerequisites

Ralph is a Rust workspace using edition 2024. Use a Rust toolchain capable of
building the workspace, and install at least one supported agent CLI before you
run real orchestration.

Supported backend names are documented in the source and README:

- `claude`
- `kiro`
- `gemini`
- `codex`
- `amp`
- `copilot`
- `opencode`
- `custom`

## Build From Source

```bash
cargo build
```

For release-style local usage:

```bash
cargo build --release
./target/release/ralph --help
```

## Initialize A Project

Traditional mode creates the lightest config:

```bash
ralph init --backend claude
```

Preset mode creates a hat-based workflow:

```bash
ralph init --preset tdd-red-green
```

List embedded presets:

```bash
ralph init --list-presets
```

## Empty Workspace Bootstrap

If the current directory has no `ralph.yml` and no `PROMPT.md`, `ralph run` can
enter startup resource bootstrap instead of failing on the missing files.
Use dry-run to inspect the resolved startup config without launching a backend:

```bash
RALPH_HOME=/tmp/ralph-home ralph run --dry-run --no-tui
```

Ralph writes `.ralph/bootstrap-selection.json` and `.ralph/resolved-config.yml`
before the real loop starts. See the
[startup resource bootstrap runbook](../runbook/startup-resource-bootstrap.md)
for the exact selector boundary.

## Run A Task

Inline prompt:

```bash
ralph run -p "Add validation to the user import command"
```

Prompt file:

```bash
ralph run --prompt-file PROMPT.md
```

Record the session while debugging or validating:

```bash
ralph run --record-session /tmp/ralph-session.jsonl -p "Fix the failing parser test"
```

## Runtime Capability Tools

During a run, `ralph#1` can use agent-facing tools to inspect and invoke runtime
capabilities without hot-switching the parent topology:

```bash
ralph tools capability list --json
ralph tools capability invoke --id hat:focused-reviewer --input "review this patch" --json
```

See [Runtime Capabilities](../runbook/runtime-capabilities.md) for the artifact
and topology boundary.

## Inspect A Run

Use the record tools to avoid guessing what happened:

```bash
ralph record summary /tmp/ralph-session.jsonl
```

For a live run that wrote `.ralph/record-session.latest`:

```bash
ralph record watch --until-event _meta.termination --timeout-secs 120 --quiet
```

## Parallel Mode Tools

When a parallel supervisor is active, inspect instances:

```bash
ralph agents
ralph agents --format json
```

Inject an external event into the current run:

```bash
ralph emit human.message "continue, but keep the window small" --target-instance ralph#1
```

Use `ralph emit` for external injection. Use in-band `<event ...>` output for
normal hat job results when the current job stdout is being parsed.

## Local Evidence Checklist

Before you call a task done:

- run the docs or code build that proves the change
- run the focused tests that cover the modified behavior
- inspect command output, especially warnings promoted to errors
- keep record-session evidence for orchestration behavior
- close runtime tasks only after the evidence exists
