# Architecture

Ralph Orchestrator is a Rust workspace. The orchestrator stays thin: it loads
configuration, assembles instructions, runs backend CLIs, routes events, records
evidence, and applies backpressure.

## Workspace Map

| Crate | Evidence path | Responsibility |
| --- | --- | --- |
| `ralph-proto` | `crates/ralph-proto/` | Shared protocol types: event, topic, hat, audience, session strategy |
| `ralph-core` | `crates/ralph-core/` | Event loop, config model, parser, stores, instruction assembly |
| `ralph-adapters` | `crates/ralph-adapters/` | Backend CLI process execution and stream handling |
| `ralph-tui` | `crates/ralph-tui/` | Terminal observation UI |
| `ralph-cli` | `crates/ralph-cli/` | User-facing commands and runtime glue |
| `ralph-e2e` | `crates/ralph-e2e/` | End-to-end scenario runner |
| `ralph-bench` | `crates/ralph-bench/` | Benchmark tasks and measurements |

## Main Execution Layers

```text
ralph-cli
  parses commands, loads config, starts run modes

ralph-core
  owns event loop semantics, config, event parsing, memory/task stores

ralph-adapters
  runs external agent CLIs and streams output back

ralph-tui
  observes runtime state without becoming the source of truth
```

## State Boundaries

| State | Storage | Notes |
| --- | --- | --- |
| Runtime tasks | `.agent/tasks.jsonl` | Open work graph; tasks close only after evidence. |
| Persistent memories | `.agent/memories.md` | Compatibility baseline for cross-session learning. |
| Session recordings | JSONL file from `--record-session` | Primary evidence stream for orchestration debugging. |
| Parallel runtime | `.ralph/` | Current events marker, agents snapshot, diagnostics, runtime files. |
| Specs | `specs/` and `openspec/` | Requirements and design source material. |

The repository currently also has scoped experience work in progress.
Docs that describe scoped project/role/topic experience must distinguish
implemented baseline from migration direction.

## Configuration Model

`RalphConfig` supports both flat v1-style fields and nested v2-style fields.
The current top-level sections include:

- `event_loop`
- `cli`
- `core`
- `hats`
- `events`
- `parallel`
- `tui`
- `memories`
- `tasks`

This compatibility layer matters for migration. A user can still bring older
flat config fields, while newer docs should prefer the nested shape.

## Parallel Runtime

The parallel supervisor models `HatInstance` workers. Each instance owns a
pending queue and runs at most one job at a time. Global permits enforce the
overall running job cap.

Important implementation points:

- the supervisor uses topic contracts when configured
- missing contracts fall back to trigger-based routing
- request/reply origins are tracked for `reply.hat.message`
- external events are read from the file referenced by `.ralph/current-events`
- TUI pause semantics keep the run alive after `LOOP_COMPLETE` in interactive mode

## Design Rule

If a feature can be handled by the agent with better specs, tasks, memories, or
tests, prefer that over adding orchestration logic. Add runtime code when the
runtime must enforce a boundary that an agent cannot reliably enforce itself.
