# Ralph Orchestrator

<section class="ralph-hero" markdown>

<div markdown>

Ralph is a Rust implementation of the Ralph Wiggum orchestration technique.
It keeps an AI agent in a loop until the task is actually done, then uses tests,
recorded evidence, and event routing to prevent false "done" claims.

[Start with Ralph](start/index.md){ .md-button .md-button--primary }
[Read the method](method/ralph-wiggum.md){ .md-button }

</div>

```text
PROMPT.md + specs
      |
      v
ralph run --record-session /tmp/session.jsonl
      |
      +--> agent iteration
      +--> files, git, memories, tasks
      +--> build.done only after evidence
```

</section>

## What This Site Covers

<div class="grid cards" markdown>

-   **Method**

    How Ralph adapts the original loop into fresh context, disk-backed state,
    and verification backpressure.

-   **System**

    The Cargo workspace, hats, events, parallel supervisor, and where each
    responsibility lives in the codebase.

-   **Runbook**

    The commands that matter: initialize, run, emit, inspect agents, record
    sessions, validate hats, and run tests.

-   **Sources**

    A source map that ties claims back to `AGENTS.md`, `README.md`, `specs/`,
    and crate source files.

</div>

## Current Shape

Ralph Orchestrator is a Cargo workspace with seven crates:

| Crate | Responsibility |
| --- | --- |
| `ralph-proto` | Protocol types such as events, hats, topics, and routing metadata |
| `ralph-core` | Configuration, event loop, event parsing, memory and task stores |
| `ralph-adapters` | Backend CLI execution for agent providers |
| `ralph-tui` | Terminal UI and live observation surfaces |
| `ralph-cli` | The `ralph` command line entry point |
| `ralph-e2e` | End-to-end scenarios against real or mock backends |
| `ralph-bench` | Benchmark and performance harnesses |

## Design Posture

The project is intentionally not a platform with every behavior hard-coded into
the orchestrator. The main job of Ralph is to route work, apply backpressure,
record evidence, and let capable agents do the implementation.

That stance shows up everywhere:

- specs live in `specs/`
- runtime work lives in `.agent/tasks.jsonl`
- persistent memories live in `.agent/memories.md`
- record-session JSONL is the evidence stream
- `build.done` is accepted only when the evidence is present

## Next

Start with [the first run guide](start/index.md), then read [the Ralph Wiggum method](method/ralph-wiggum.md).
