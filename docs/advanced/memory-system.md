# Memory System

!!! note "Documentation In Progress"
    This page is under development. Check back soon for comprehensive memory system documentation.

## Overview

Ralph's memory system now has two concurrent realities:

- the legacy compatibility layer still reads `.agent/memories.md`
- the scoped experience layer now reads project, role, topic summary, and instance summary context

The broader scoped experience design now distinguishes five layers:

- runtime work graph in `.agent/tasks.jsonl`
- instance context in `.ralph/log/<instance_id>/...`
- topic shared context in topic-scoped planning and notes files
- role experience in `.ralph/roles/<hat_id>/experience.md`
- project experience in project-root `experience.md`

Today the implemented baseline is:

- `.agent/memories.md` remains a compatibility layer
- project experience injection from `experience.md` is active
- role experience injection from `.ralph/roles/<hat_id>/experience.md` is active
- topic and instance context are injected as summary-first context
- canonical writer ownership is tracked under `.ralph/canonical-writers/`

The migration is therefore no longer "legacy only". It is now a hybrid system that must keep backward compatibility while newer scoped experience flows become the preferred path.

## Memory Types

- **Codebase Patterns** - Discovered conventions and patterns
- **Architectural Decisions** - Design choices and rationale
- **Recurring Solutions** - Common problem-solving approaches
- **Project Context** - Domain-specific knowledge

## Configuration

```yaml
memories:
  enabled: true  # Default
  path: .agent/memories.md
```

Important: `memories.path` is already documented here as the intended direction, but the current implementation baseline still reads `.agent/memories.md` directly. Treat the legacy path as the real runtime behavior until the scoped experience migration lands.

## Scoped Experience Model

| Scope | Storage | Purpose |
|------|---------|---------|
| Runtime | `.agent/tasks.jsonl` | Active work graph |
| Instance | `.ralph/log/<instance_id>/...` | Raw execution trail |
| Topic | `task_plan__topic.md`, `notes__topic.md`, `WORKLOG__topic.md` | Shared topic conclusion |
| Role | `.ralph/roles/<hat_id>/experience.md` | Reusable guidance for one role |
| Project | `experience.md` | Reusable guidance across the project |

The scoped model exists to keep:

- raw execution traces out of long-term knowledge
- role-specific heuristics out of project-wide defaults
- project-wide defaults tight enough for `ralph#1` to use during workflow selection

## Writer Governance

Shared scoped knowledge is not multi-writer by default.

- Topic shared files must have exactly one canonical writer at a time
- Role experience must have one canonical writer per role
- Project experience defaults to `ralph#1`

Runtime ownership metadata is stored under:

```text
.ralph/canonical-writers/
├── project.json
├── roles/<hat_id>.json
└── topics/<suffix>.json
```

Handoff summaries are append-only:

- topic handoffs append to `WORKLOG__topic.md`
- role handoffs append to `.ralph/roles/<hat_id>/handoff.md`

This split is intentional. Role handoffs use a sidecar file so normal `experience.md` rewrites do not silently erase transfer context.

## See Also

- [Memories & Tasks](../concepts/memories-and-tasks.md) - Core concepts
- [Task System](task-system.md) - Runtime task tracking
- [Configuration](../guide/configuration.md) - Full configuration reference
