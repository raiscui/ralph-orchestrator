## 1. Runtime protocol SSOT and prompt boundaries

- [x] 1.1 Define reserved/control/result/observer-only topic classification in a single runtime protocol helper or equivalent single source.
- [x] 1.2 Add validator or config/runtime tests that reject ordinary hat use of reserved control-plane topics unless explicitly allowed.
- [x] 1.3 Update coordinator and worker prompt generation so `task.start`, `task.resume`, `starting_event`, `reply.*`, and stdout-only output guidance use the same protocol wording.
- [x] 1.4 Add dynamic worker prompt tests proving coordinator-only `topology.spawn_group` instructions do not leak into ordinary task-derived workers.

## 2. Dynamic role contract evidence

- [x] 2.1 Audit current `EffectiveRoleContract` / role contract summary fields and define the minimal persisted summary shape.
- [x] 2.2 Ensure `topology.spawn.result` includes per-instance role contract summaries with role name, identity source, source spawn request id, allowed result topics, and role contract hash.
- [x] 2.3 Ensure `.ralph/agents.json` current instances and completed dynamic tombstones preserve role contract summaries after dynamic idle reaping.
- [x] 2.4 Add focused tests for raw role contract normalization, allowed topic clipping, forbidden control-plane topics, and readable warnings.

## 3. Spawn group partial and tombstone lifecycle

- [x] 3.1 Design and implement explicit partial outcome representation for spawn member validation failure, delivery failure, timeout, missing result, and cleanup/reaping.
- [x] 3.2 Add tests proving non-atomic spawn groups allow successful members to continue while failed members remain auditable.
- [x] 3.3 Add agents snapshot tests for failed-after-spawn or failed tombstone states.
- [x] 3.4 Add record-session or events-log evidence for partial failure phases with request id, role, instance id when available, and recovery hint.

## 4. Record summary and evidence correlation

- [x] 4.1 Extend `ralph record summary` Evidence Inspect to show dynamic spawn request id, spawned instances, role contract hashes, and result source coverage.
- [x] 4.2 Ensure summary distinguishes record-session semantic completion from wrapper exit status, stdout tail, or display state.
- [x] 4.3 Extend `record summary --agents-file` to present current registry and completed dynamic tombstones as separate evidence sections.
- [x] 4.4 Add tests for missing termination, missing dynamic result coverage, and completed dynamic instance display.

## 5. Evidence index correlation

- [x] 5.1 Add or extend evidence-index entry support for dynamic role contract hash and spawn request id correlation without duplicating full source artifacts.
- [x] 5.2 Add lookup tests for role contract hash, spawn request id, and missing dynamic result markers.
- [x] 5.3 Verify display/summary changes do not break evidence-index artifact path and correlation semantics.

## 6. Release-fast gate and dogfood evidence

- [x] 6.1 Create or document a focused release-fast command set for this runtime/evidence lane.
- [x] 6.2 Add a replay or integration guardrail for natural-language dynamic spawn or equivalent parent-visible multi-role dynamic spawn.
- [x] 6.3 Run `openspec validate --all --strict` after implementation and fix any spec validation drift.
- [x] 6.4 Run focused Rust tests for touched modules and `cargo test -p ralph-core smoke_runner`.
- [x] 6.5 Run a focused runtime dogfood or E2E, preserve record-session and agents snapshot artifacts, and record the evidence in `WORKLOG.md`.
