# Memories & Tasks

Ralph currently uses two active persistence systems by default: memories for cross-session learning, and tasks for runtime work tracking.

The implementation baseline is still `.agent/memories.md`, but the scoped experience design now distinguishes five layers of state:

- Runtime work graph: `.agent/tasks.jsonl`
- Instance context: `.ralph/log/<instance_id>/...`
- Topic shared context: `task_plan__topic.md`, `notes__topic.md`, `WORKLOG__topic.md`
- Role experience: `.ralph/roles/<hat_id>/experience.md`
- Project experience: project-root `experience.md`

Today, only the legacy memories layer is fully implemented as the default long-term store. The newer role/project experience layers are the forward design direction and must remain compatible with `.agent/memories.md` during migration.

## Overview

| System | Storage | Purpose |
|--------|---------|---------|
| **Memories** | `.agent/memories.md` | Current compatibility baseline for accumulated wisdom across sessions |
| **Tasks** | `.agent/tasks.jsonl` | Runtime work items |

Both are enabled by default and work together to replace the legacy scratchpad.

## Memories

Memories persist learning across sessions. They capture patterns, decisions, fixes, and context that Ralph should remember.

### Scoped Experience Roadmap

The scoped experience system extends the current memory model rather than replacing it overnight:

| Scope | Storage | Role |
|------|---------|------|
| Runtime | `.agent/tasks.jsonl` | Open work graph |
| Instance | `.ralph/log/<instance_id>/...` | Raw execution trail |
| Topic | `task_plan__topic.md`, `notes__topic.md`, `WORKLOG__topic.md` | Shared topic conclusion |
| Role | `.ralph/roles/<hat_id>/experience.md` | Reusable guidance for one role |
| Project | `experience.md` | Reusable guidance across roles and workflows |

Until the scoped experience migration is complete, `.agent/memories.md` remains the compatibility entry point for persistent learning.

### Memory Types

| Type | Use For |
|------|---------|
| `pattern` | Codebase conventions discovered |
| `decision` | Architectural choices and rationale |
| `fix` | Solutions to recurring problems |
| `context` | Project-specific knowledge |

### Creating Memories

```bash
# Pattern: discovered convention
ralph tools memory add "All API handlers return Result<Json<T>, AppError>" \
  -t pattern --tags api,error-handling

# Decision: architectural choice
ralph tools memory add "Chose JSONL over SQLite: simpler, git-friendly" \
  -t decision --tags storage,architecture

# Fix: recurring problem solution
ralph tools memory add "cargo test hangs: kill orphan postgres" \
  -t fix --tags testing,postgres

# Context: project knowledge
ralph tools memory add "The /legacy folder is deprecated, use /v2" \
  -t context --tags api,migration
```

### Searching Memories

```bash
# Broad search
ralph tools memory search "api"

# Filter by type
ralph tools memory search -t fix "error"

# Filter by tags
ralph tools memory search --tags api,auth

# List all memories
ralph tools memory list

# List recent fixes
ralph tools memory list -t fix --last 10
```

### Memory Injection

Memories are automatically injected at the start of each iteration:

```yaml
memories:
  enabled: true
  inject: auto      # auto, manual, or none
  budget: 2000      # Max tokens to inject
  filter:
    types: []       # Filter by type (empty = all)
    tags: []        # Filter by tags (empty = all)
    recent: 0       # Days limit (0 = no limit)
```

This block documents the current implementation baseline. Even though some docs mention `memories.path`, the current core implementation still resolves the compatibility path as `.agent/memories.md`.

### Memory Best Practices

1. **Be specific** — "Uses barrel exports" not "Has good patterns"
2. **Include why** — "Chose X because Y" not just "Uses X"
3. **One concept per memory** — Split complex learnings
4. **Tag consistently** — Reuse existing tags

## Tasks

Tasks track runtime work items during orchestration.

### Creating Tasks

```bash
# Basic task
ralph tools task add "Implement user authentication"

# With priority (1-5, 1 = highest)
ralph tools task add "Fix critical bug" -p 1

# With dependency
ralph tools task add "Deploy to production" --blocked-by setup-infra
```

### Managing Tasks

```bash
# List all tasks
ralph tools task list

# List unblocked tasks only
ralph tools task ready

# Close a completed task
ralph tools task close task-123
```

### Task Workflow

1. Ralph creates tasks from the prompt/plan
2. Tasks are worked in priority order
3. Dependencies are respected (blocked tasks wait)
4. Completed tasks are closed
5. Loop ends when no tasks remain

### Task Closure Rules

Tasks must only be closed when:

1. Implementation is actually complete
2. Tests pass
3. Build succeeds (if applicable)
4. Evidence of completion exists

```bash
# Good: Close with evidence
cargo test  # passes
ralph tools task close task-123

# Bad: Close without verification
ralph tools task close task-123  # No tests run!
```

## Memories vs Tasks

| Aspect | Memories | Tasks |
|--------|----------|-------|
| **Persistence** | Cross-session | Single session |
| **Purpose** | Learning | Work tracking |
| **When created** | When something is learned | When work is identified |
| **When removed** | Rarely | When completed |

## Legacy Scratchpad Mode

To disable memories and tasks (legacy mode):

```yaml
memories:
  enabled: false
tasks:
  enabled: false
```

In this mode, `.agent/scratchpad.md` is used for all state.

## File Formats

### memories.md

```markdown
# Memories

## Patterns

### mem-1737372000-a1b2
> All API handlers return Result<Json<T>, AppError>
<!-- tags: api, error-handling | created: 2024-01-20 -->

## Decisions

### mem-1737372100-c3d4
> Chose JSONL over SQLite for simplicity
<!-- tags: storage | created: 2024-01-20 -->
```

### tasks.jsonl

```json
{"id":"task-001","title":"Implement auth","priority":2,"status":"open","created":"2024-01-20T10:00:00Z"}
{"id":"task-002","title":"Add tests","priority":3,"status":"open","blocked_by":["task-001"],"created":"2024-01-20T10:01:00Z"}
```

### scoped role/project experience (design direction)

```markdown
# Experience

### exp-1737372000-a1b2
> Only the canonical writer may update shared topic files.
<!-- scope: project | source_topics: memory-axes | source_hats: ralph#1 | status: active | confidence: high | created_at: 2026-03-21T00:00:00Z | updated_at: 2026-03-21T00:10:00Z | supersedes:  -->
```

Role and project experience intentionally share one entry shape. Their difference comes from file location, not from using two different protocols.

## Integration with Hats

Hats can use memories and tasks:

```yaml
hats:
  builder:
    triggers: ["task.start"]
    instructions: |
      1. Check memories for relevant patterns
      2. Pick a task from `ralph tools task ready`
      3. Implement the task
      4. Record learnings as memories
      5. Close the task with `ralph tools task close <id>`
```

## Next Steps

- Learn about [Backpressure](backpressure.md) for quality gates
- See [Configuration](../guide/configuration.md) for full options
- Explore the [Memory System](../advanced/memory-system.md) in depth
