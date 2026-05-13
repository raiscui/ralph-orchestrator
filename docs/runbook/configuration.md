# Configuration Runbook

Ralph configuration is YAML. The runtime accepts older flat fields and newer
nested sections, but new docs and examples should prefer the nested shape.

## Minimal Config

```yaml
cli:
  backend: claude

event_loop:
  max_iterations: 10
  completion_promise: LOOP_COMPLETE
```

Run it:

```bash
ralph run -p "Add a regression test for the parser edge case"
```

## Hat-Based Config Shape

```yaml
hats:
  builder:
    triggers:
      - build.task
    publishes:
      - build.done
      - build.blocked
    instructions: |
      Implement the task, run checks, then publish evidence.
```

The exact accepted fields are defined by `RalphConfig` and `HatConfig` in
`crates/ralph-core/src/config.rs`.

## Parallel Safety Knobs

```yaml
parallel:
  enabled: true
  autoscale:
    max_running_jobs: 4
  workspace:
    worktree_base_dir: .ralph/worktrees
```

The global cap prevents a burst of events from starting unbounded backend CLI
jobs. Each hat instance still processes one job at a time.

## Session Strategy

Event attributes can request a session strategy:

| Strategy | Use case |
| --- | --- |
| `exec` | Stateless one-shot job. |
| `mcp` | Persistent session when continuity matters. |
| `app_server` | Persistent Codex app-server turn control and steer support. |

Do not downgrade a long-lived instance from a stronger session strategy to a
weaker one. If you must move from `mcp` to `app_server`, include a handoff
summary in the next prompt because those are different persistent session
implementations.

## Startup Bootstrap Artifacts

When the default `ralph.yml` is absent and there is no explicit prompt source,
Ralph can resolve a startup config from embedded resources. The audit files are:

- `.ralph/bootstrap-selection.json`
- `.ralph/resolved-config.yml`

Resource files are synchronized under `RALPH_HOME/resources`, then
`$HOME/.ralph/resources`, then `.ralph/resources` if no home directory is
available. See [Startup Resource Bootstrap](startup-resource-bootstrap.md).

## Useful CLI Commands

```bash
ralph init --list-presets
ralph hats validate
ralph hats graph --format mermaid
ralph agents --format json
ralph events --last 20
ralph tools memory list
ralph tools task ready
ralph doctor --format json
```

## Runtime Capability Tools

`ralph tools capability` exposes lightweight workflow / hat capability metadata
and can invoke one capability through an isolated child or micro-run.

```bash
ralph tools capability summaries
ralph tools capability invoke --input "review this patch" --json
```

Invocation evidence is written under `.ralph/capability-invocations/` and to
`.ralph/events.jsonl`. See [Runtime Capabilities](runtime-capabilities.md).

## Custom Backend Arguments

`ralph run` accepts backend-specific arguments after `--`:

```bash
ralph run -b codex -p "Explain the failing test" -- --model gpt-5.5
```

Keep those arguments in the command or config that owns the runtime behavior.
Do not hide important backend flags inside docs prose.

## Configuration Evidence

Primary source paths:

- `crates/ralph-cli/src/main.rs`
- `crates/ralph-core/src/config.rs`
- `presets/`
- `crates/ralph-cli/presets/`
- `specs/parallel-hat-instances.spec.md`
