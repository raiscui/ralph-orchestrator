# Parent-visible Topology Spawn and Child-run Observability Implementation Plan

> **For Ralph/Codex:** REQUIRED SKILL: use `executing-plans` to implement this plan task-by-task.

**Goal:** 让父级 runtime 真正创建多个可见 hat instance,同时把 isolated child run 做成独立的可观测状态,不再把两者混成一件事。

**Architecture:** 复用现有动态实例 spawn 和 `RuntimeLifecycleKind::Spawn` 证据链,新增显式的 `topology.spawn_group` 运行时事件。真实实例继续走 Supervisor runtime 和 `.ralph/agents.json`; child run 继续走 capability invocation,但在 TUI 中独立投影为 child-run 状态。

**Spec:** `specs/parent-visible-topology-spawn-observability.spec.md`

**Tech Stack:** Rust, Tokio, serde/serde_json, existing `ParallelSupervisor`, `ralph_proto`, ratatui, existing event logger / agents snapshot / TUI reducer.

---

## Checklist

- [x] Step 1: Add typed topology spawn protocol records.
- [x] Step 2: Implement Supervisor `topology.spawn_group` handling.
- [x] Step 3: Add fixed-role metadata support to agents snapshot while keeping temporary roles transient.
- [x] Step 4: Add parent-observable child-run state, event forwarding, and `ralph agents` summary.
- [x] Step 5: Render child-run and spawned-role information in TUI.
- [x] Step 6: Update coordinator prompt / capability contract guardrails.
- [x] Step 7: Add recorded evidence validation and run gates.

---

## Step 1: Add typed topology spawn protocol records

**Objective:** Give `topology.spawn_group` a typed request/result/failure shape before wiring runtime behavior.

**Files:**
- Create: `crates/ralph-core/src/topology_spawn.rs`
- Modify: `crates/ralph-core/src/lib.rs`

**Implementation guidance:**
- Add constants:
  - `TOPIC_TOPOLOGY_SPAWN_GROUP`
  - `TOPIC_TOPOLOGY_SPAWN_RESULT`
  - `TOPIC_TOPOLOGY_SPAWN_FAILED`
- Add serde types:
  - `TopologySpawnGroupRequest`
  - `TopologySpawnMember`
  - `TopologySpawnedInstance`
  - `TopologySpawnGroupResult`
  - `TopologySpawnGroupFailed`
- Parse defensively:
  - `request_id`, `hat`, `delivery_topic`, `instances` are required.
  - `instances` must not be empty.
  - `role` and `task` should be non-empty after trim.

**Tests:**
- Unit test valid sample JSON from the spec.
- Unit test missing `request_id` fails.
- Unit test empty `instances` fails.

**Command:**
```bash
cargo test -p ralph-core topology_spawn -- --nocapture
```

**Demo after step:**
- A sample payload can round-trip through serde.
- Invalid payloads fail with clear field-level errors.

---

## Step 2: Implement Supervisor `topology.spawn_group` handling

**Objective:** Make parent coordinator output create real parent-visible dynamic instances, instead of silently collapsing into an existing instance.

**Files:**
- Create: `crates/ralph-core/src/parallel/supervisor/topology_runtime.rs`
- Modify: `crates/ralph-core/src/parallel/supervisor.rs`
- Modify if needed: `crates/ralph-core/src/parallel/supervisor/routing.rs`
- Test: `crates/ralph-core/src/parallel/supervisor/routing_tests.rs`

**Implementation guidance:**
- Add `handled_topology_spawn_request_ids: HashSet<String>` to `ParallelSupervisor`.
- Only accept `topology.spawn_group` from coordinator hats (`hat_id == "ralph"`).
- For each request member:
  - call the existing dynamic spawn path.
  - use existing `RuntimeLifecycleKind::Spawn` evidence.
  - deliver a direct `delivery_topic` event to the new `HatInstanceId`.
- Emit `topology.spawn.result` with the spawned instance ids.
- Emit `topology.spawn.failed` for parse/runtime errors.
- Do not silently fallback to `builder#1` or any existing instance when explicit group spawn was requested.

**Tests:**
- `topology_spawn_group_creates_three_dynamic_instances_and_delivers_direct`
- `topology_spawn_group_is_idempotent_by_request_id`
- `topology_spawn_group_from_non_ralph_hat_is_ignored_or_failed`
- `topology_spawn_group_invalid_payload_returns_failed_event`

**Command:**
```bash
cargo test -p ralph-core topology_spawn_group -- --nocapture
```

**Demo after step:**
- `.ralph/events.jsonl` contains three `runtime.lifecycle` records with `kind="spawn"`.
- `.ralph/events.jsonl` contains direct `runtime.delivery` records to the spawned instances.

---

## Step 3: Add fixed-role metadata support without persisting temporary roles

**Objective:** Let temporary spawned roles remain transient by default, while allowing coordinator-promoted fixed roles to become first-class `agents.json` metadata.

**Files:**
- Modify: `crates/ralph-core/src/agents_snapshot.rs`
- Modify: `crates/ralph-core/src/parallel/supervisor.rs`
- Modify: `crates/ralph-cli/src/display.rs`
- Test: `crates/ralph-cli/tests/integration_agents.rs`

**Implementation guidance:**
- Add optional fixed-role metadata to `AgentInstanceSnapshot`, for example:
  - `fixed_role_label: Option<String>`
  - `fixed_role_reason: Option<String>`
- Do not persist transient `role` labels from `topology.spawn_group` by default.
- Let TUI derive temporary labels from the live spawn request / last input preview.
- Only write fixed-role metadata when the request explicitly marks a member as fixed / promotable.
- Display fixed roles in `ralph agents` table without breaking JSON output.

**Tests:**
- Agents JSON with fixed-role metadata still deserializes.
- `ralph agents` table shows fixed roles when present.
- Temporary roles do not become first-class fields unless explicitly marked fixed.
- Existing agents tests still pass with old snapshots missing the new optional fields.

**Command:**
```bash
cargo test -p ralph-cli integration_agents -- --nocapture
```

**Demo after step:**
```text
builder#2 | builder | running | yes | runtime_autoscale | fixed_role=<only when promoted> ...
```

---

## Step 4: Add parent-observable child-run state, event forwarding, and agents summary

**Objective:** Keep capability child runs isolated, but make them visible as child-run projections in TUI and lightweight `ralph agents` output.

**Files:**
- Modify: `crates/ralph-cli/src/parallel_runner.rs`
- Modify: `crates/ralph-tui/src/state.rs`
- Modify: `crates/ralph-tui/src/state/parallel.rs`

**Implementation guidance:**
- Extend `should_forward_event_to_tui` to include:
  - `topic.starts_with("capability.")`
  - `topic.starts_with("topology.")`
- Add `ChildRunViewState` / `ChildRunStatus` under `ParallelTuiState`.
- Add lightweight child-run summary to the agents snapshot or adjacent agents status output.
- Update `ParallelTuiState::apply_event` to consume:
  - `capability.invoke`
  - `capability.result`
  - `capability.failed`
- Do not push child runs into `instance_order`.
- Do not serialize child runs as real `AgentInstanceSnapshot` entries.

**Tests:**
- Capability invoke creates a running child-run state.
- Capability result marks the child-run done/result.
- Capability failed marks the child-run failed.
- Child-run state does not create fake instances.

**Command:**
```bash
cargo test -p ralph-tui child_run -- --nocapture
```

**Demo after step:**
- TUI state can answer: `child runs: 1 running, 0 done, 0 failed`.
- `ralph agents` can show a lightweight child-run summary.
- `ParallelTuiState.instance_order` remains unchanged for child runs.

---

## Step 5: Render child-run and spawned-role information in TUI

**Objective:** Make both visible/non-visible paths observable in the actual parent TUI.

**Files:**
- Modify: `crates/ralph-tui/src/widgets/footer.rs`
- Modify: `crates/ralph-tui/src/widgets/instances.rs`
- Modify: `crates/ralph-tui/src/widgets/parallel_output.rs`
- Modify if needed: `crates/ralph-tui/src/app.rs`

**Implementation guidance:**
- Footer should show compact child-run summary:
  - `child: 1 running / 2 done / 0 failed`
- Instances pane should show dynamic role labels:
  - `builder#2 running role:功能补充`
- Output status pane should show current child-run evidence path when available.
- Do not let the status area cover output body rows.

**Tests:**
- Footer widget includes child-run counts.
- Instances widget includes spawn role labels.
- Output status widget shows capability artifact summary.
- Existing output status tests still prove status area is separate from body area.

**Command:**
```bash
cargo test -p ralph-tui footer instances parallel_output -- --nocapture
```

**Demo after step:**
- Parent TUI shows real spawned instances on the left.
- Parent TUI also shows isolated child-run status without adding fake instances.

---

## Step 6: Update coordinator prompt / capability contract guardrails

**Objective:** Prevent `ralph#1` from choosing `workflow:default-parallel` when the user explicitly asks for parent-visible instances.

**Files:**
- Modify: `crates/ralph-core/src/event_emission_protocol.rs`
- Modify: `crates/ralph-core/src/parallel/supervisor.rs`
- Modify: `crates/ralph-core/src/capability.rs`
- Modify if needed: `crates/ralph-cli/src/capability.rs`
- Test: `crates/ralph-core/src/parallel/supervisor/routing_tests.rs`

**Implementation guidance:**
- Document `topology.spawn_group` as the parent-visible dynamic topology mutation protocol.
- Keep `workflow:*` capability catalog language explicit: isolated child/micro-run, no parent topology mutation.
- Tell the coordinator:
  - use `topology.spawn_group` for parent-visible dynamic instances.
  - use `capability.request` for isolated child workflows.
  - use `spawn_instance=true + target=<hat_id>` only for one-off single instance delivery.
- Tell the coordinator how to handle topology result events:
  - `topology.spawn.result` is an acknowledgement that spawned instances already received direct delivery.
  - do not re-emit the original `delivery_topic`.
  - do not use `audience_instances` as a replay mechanism.
  - `topology.spawn.failed` should report or correct failed members without pretending instances exist.

**Tests:**
- Coordinator prompt contains `topology.spawn_group`.
- Runtime capability catalog still contains isolation warning.
- Event emission protocol still documents existing attributes.
- Coordinator prompt contains `topology.spawn.result` / `topology.spawn.failed` handling.
- Worker prompts do not inherit topology mutation guardrails.

**Command:**
```bash
cargo test -p ralph-core topology_spawn_prompt capability_catalog -- --nocapture
```

**Demo after step:**
- Prompt surface makes the three routes impossible to confuse:
  - parent-visible group spawn
  - isolated child run
  - single-event spawn
- Prompt surface also makes `topology.spawn.result` impossible to confuse with a second delegation request.

---

## Step 7: Add recorded evidence validation and run gates

**Objective:** Prove the original failure mode is fixed with durable evidence, not just unit tests.

**Files:**
- Add or update focused integration tests under existing `crates/ralph-cli/tests/` or `crates/ralph-e2e/` if needed.
- Add fixture only if it is small, deterministic, and not ephemeral.

**Implementation guidance:**
- Add a scenario equivalent to the user request:
  - role A: `功能补充`
  - role B: `功能完善`
  - role C: `review`
- Assert durable evidence:
  - three spawned dynamic instances.
  - three direct deliveries.
  - `.ralph/agents.json` exposes dynamic instances.
  - capability child run remains observable but not a fake instance.

**Commands:**
```bash
git diff --check
cargo test -p ralph-core topology_spawn_group -- --nocapture
cargo test -p ralph-tui child_run footer instances parallel_output -- --nocapture
cargo test -p ralph-cli integration_agents -- --nocapture
cargo test --quiet
```

**Recorded-session validation after implementation:**
```bash
cargo run --bin ralph -- run \
  -c ralph.yml \
  --record-session /tmp/parent-visible-spawn.jsonl \
  -p '创建三个 hat 实例,分别从 "功能补充"、"功能完善"、"review" 三个方面进行项目演进。'
```

Then inspect:
```bash
ralph record summary /tmp/parent-visible-spawn.jsonl
rg 'topology.spawn_group|runtime.lifecycle|runtime.delivery|builder#2|builder#3|builder#4' .ralph/events.jsonl
jq '.instances[] | select(.instance_id | test("builder#"))' .ralph/agents.json
```

Also inspect the event timeline and assert that no original `delivery_topic`
is published after `topology.spawn.result`:

```bash
python3 - <<'PY'
import json
from pathlib import Path

events = []
for line_no, line in enumerate(Path("/tmp/parent-visible-spawn.jsonl").read_text().splitlines(), 1):
    obj = json.loads(line)
    if obj.get("event") == "bus.publish":
        data = obj.get("data") or {}
        events.append((line_no, data.get("topic"), data.get("source_instance"), data.get("target_instance")))

first_result = next((idx for idx, (_, topic, _, _) in enumerate(events) if topic == "topology.spawn.result"), None)
assert first_result is not None, "missing topology.spawn.result"
assert not any(topic == "build.task" for _, topic, _, _ in events[first_result + 1:]), "delivery_topic was redelivered after spawn result"
PY
```

**Demo after step:**
- The parent TUI immediately shows the spawned instances.
- Record-session and `.ralph/events.jsonl` prove they were real runtime instances.
- A capability child run, if used, is shown as child-run status only.
- `topology.spawn.result` does not cause a fourth/fallback delivery to the configured instance.

---

## Decisions confirmed before implementation

1. `topology.spawn_group` does not need atomic success. Partial success is allowed, but failures must be explicit and structured.
2. Child-run status should also surface in `ralph agents` as a lightweight summary, while TUI keeps the richer view.
3. Temporary role labels should not become first-class `.ralph/agents.json` fields by default. If the LLM coordinator judges a role to be fixed / promotable and marks it as such, then it may be stored as fixed-role metadata.
