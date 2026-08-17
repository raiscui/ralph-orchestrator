---
status: completed
created: 2026-08-17
started: 2026-08-17
completed: 2026-08-17
---
# Task: Wave 3.4 legacy code-defined scenario cleanup (subset)

## Description
Physical removal of 4 code-defined parallel scenarios that have been
moved to declarative YAML (the YAML versions are the source of truth
in `crates/ralph-e2e/scenarios/*.yaml`):

- `crates/ralph-e2e/src/scenarios/parallel/emit_spawn_instance.rs`
  → declarative: `crates/ralph-e2e/scenarios/emit-spawn-instance.yaml`
- `crates/ralph-e2e/src/scenarios/parallel/hat_instances.rs`
  → declarative: `crates/ralph-e2e/scenarios/hat-instances.yaml`
- `crates/ralph-e2e/src/scenarios/parallel/starting_event_inference.rs`
  → declarative: `crates/ralph-e2e/scenarios/starting-event-inference.yaml`
- `crates/ralph-e2e/src/scenarios/parallel_trigger_routing_example.rs`
  → declarative: `crates/ralph-e2e/scenarios/parallel-trigger-routing-example.yaml`

Plus mechanical fix in `parallel/mod.rs:204` `patch_example_config_for_codex_e2e`:
replace `--full-auto` → `--sandbox danger-full-access` (helper used by
30+ example scenarios, e.g. `parallel_audit_evidence_pack_example`).

## Background
- Q3 plan Group 4 task 4.5–4.14: rewrite the 14 cherry-pick rejects as
  Group 4 rewrite tasks; many of these are the same files as the 4
  parallel scenarios above.
- Q3 plan LATER_PLANS Wave 3.4 (target 2.3.0): physical removal of 21
  deprecated imperative structs. The 4 we touch here are NOT in the
  21-list (those are errors.rs / hats.rs / memory.rs / capabilities.rs
  / parallel/app_server_idle_start.rs / parallel/app_server_steer_multi_turn.rs),
  but they are equally dead code (replaced by YAML).
- LATER_PLANS `minimax-full-auto-compat` (2026-08-16) added
  `--sandbox danger-full-access` as the replacement for
  `--full-auto` in declarative YAML. The Rust code-defined scenarios
  still carry the old flag, which is rejected by codex-cli 0.147.0
  and minimax provider (the same compat issue we already closed in
  YAML).
- The 4 scenarios appear in `all_scenarios()` only via the
  declarative YAML; the Rust definitions compile into the binary
  but are never selected by the runner.

## Reference Documentation
- `openspec/changes/archive/2026-08-12-sync-origin-main-features-q3-2026/group3-dryrun-log-2026-08-17.md` §16 (DROPPED → port, then superseded by declarative)
- `docs/solutions/minimax-full-auto-compat/README.md`
- `CONTEXT.md` (Q3 plan 整合状态, 2026-08-17)

## Scope

### In scope (this change)
1. Delete 4 .rs files
2. Remove `mod` and `pub use` declarations in `parallel/mod.rs` and `scenarios/mod.rs`
3. Remove entries from `lib.rs` `pub use crate::scenarios::{...}` list
4. Replace `--full-auto` with `--sandbox danger-full-access` in
   `parallel/mod.rs` `patch_example_config_for_codex_e2e` (mechanical)
5. Verify: `cargo check -p ralph-e2e`, `cargo test -p ralph-e2e`, no
   `--full-auto` left in the repo (`rg '\\-\\-full-auto' crates/`)
6. Run minimax live `parallel-hat-instances*` for regression (uses
   the declarative YAML path)

### Out of scope
- The 21 deprecated imperative structs in errors/hats/memory/capabilities/app_server
  (separate Wave 3.4 work, blocked on declarative coverage ≥ 90%)
- Mechanical fix of `mcp.rs` / `app_server.rs` (in ralph-adapters, separate)
- Switching example scenarios (parallel_audit_evidence_pack etc.) to
  declarative YAML (out of scope; they are still live)

## Verification
- `cargo check -p ralph-e2e` 0 errors
- `cargo test -p ralph-e2e` (currently 0 tests in this crate, but should
  not fail to build)
- `rg '\\-\\-full-auto' crates/ ralph-cli/src/ ralph-core/src/ ralph-adapters/src/ ralph-proto/src/` returns 0 results
- minimax live: `parallel-hat-instances*` 2/2 still PASS (YAML path
  unchanged)

## Risk
- Low: 4 files are dead code (YAML versions are the source of truth)
- The `patch_example_config_for_codex_e2e` fix is mechanical and
  applies the same pattern that already passed minimax live 4/4

## Decision
Per `改良胜过新增` + `不保留向后兼容，除非是生产环境项目`:
dead code + deprecated flag → physical removal is the right call.
This is Wave 3.4 work done early per the established Q3 plan pattern.
