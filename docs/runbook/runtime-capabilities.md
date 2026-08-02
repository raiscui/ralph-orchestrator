# Runtime Capabilities

Runtime capability invocation lets `ralph#1` call a workflow or hat-shaped helper
without replacing the active parent topology.

This is intentionally different from startup resource bootstrap:

- startup bootstrap resolves the first config before the run starts
- runtime capability invocation runs after the parent run exists
- v1 always uses an isolated child run or micro-run

## Capability Metadata

Capabilities use structured metadata. Runtime selection does not depend on YAML
comments.

Required metadata fields:

| Field | Meaning |
| --- | --- |
| `id` | Stable capability id, for example `workflow:feature-minimal`. |
| `kind` | `workflow_capability` or `hat_capability`. |
| `summary` | Short description shown to `ralph#1`. |
| `goal` | What the capability tries to accomplish. |
| `when_to_use` | Selection hint for the coordinator. |
| `input_contract` | What input the isolated run expects. |
| `output_contract` | What result the parent run can consume. |
| `invocation_mode` | `isolated_child_run` or `isolated_micro_run`. |

The lightweight summary is safe to show early. Full workflow config or hat
instructions are only used when the capability is invoked.

## Agent-Facing Commands

List capabilities:

```bash
ralph tools capability list --json
```

Print coordinator-friendly summaries:

```bash
ralph tools capability summaries
```

Invoke explicitly. This performs a real isolated capability execution by default:

```bash
ralph tools capability invoke \
  --id hat:focused-reviewer \
  --input "review this patch" \
  --json
```

Preview the resolved child configuration without executing it:

```bash
ralph tools capability invoke \
  --id hat:focused-reviewer \
  --input "review this patch" \
  --preview \
  --json
```

Let rules-v1 choose:

```bash
ralph tools capability invoke --input "review this patch" --json
```

In v1, review/audit-shaped input prefers a hat capability. Other input prefers a
workflow capability unless the caller passes `--id`.

## Artifacts

Each invocation writes an audit folder:

```text
.ralph/capability-invocations/<invocation_id>/
  invoke.json
  resolved-config.yml
  child-record-session.jsonl  # workflow child run evidence, when available
  result.json      # on success
  failed.json      # on failure
```

It also appends control-plane evidence to `.ralph/events.jsonl` using these
topics:

- `capability.invoke`
- `capability.result`
- `capability.failed`

For workflow capabilities, the isolated child run is started with
`--record-session`. The resulting `child-record-session.jsonl` is linked from
`.ralph/evidence-index.jsonl` with artifact kind `record_session_jsonl`.

## Topology Boundary

Runtime capability invocation must not mutate the parent `HatRegistry`, parent
`EventLoop`, or active parallel `Supervisor` topology.

The implementation proves this by creating isolated artifacts and running the
child in a separate process. Parent config files are not rewritten.

For inspect/debug workflows, `ralph tools capability invoke --preview` keeps the
old dry-run behavior visible and explicit.

If a human asks for new instances to appear in the parent TUI, do not use
`capability.request`. Use the topology mutation protocol instead:

- `topology.spawn_group` creates real parent-visible dynamic `HatInstance`
  entries.
- spawned instances receive direct delivery through the request's
  `delivery_topic`.
- `topology.spawn.result` is only an acknowledgement; it must not cause the
  coordinator to re-emit the original `delivery_topic`.
- `audience_instances` is not a replay or instance-creation mechanism.

This keeps the two evidence lanes separate:

- capability lane: isolated child/micro-run artifacts under
  `.ralph/capability-invocations/<invocation_id>/`
- topology lane: parent runtime lifecycle/delivery evidence plus
  `.ralph/agents.json` dynamic instances

## v1 / v2 Route

v1:

- rules-driven chooser
- one capability per invocation
- workflow capability = isolated child run
- hat capability = isolated micro-run

v2:

- rules first, LLM fallback chooser when rules do not converge
- multi-capability plans within catalog boundaries
- still no hot-switching of the parent live topology
