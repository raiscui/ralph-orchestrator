# Test Plan: bootstrap-live-dogfood-gate

## Goal

Prove that the default startup bootstrap path and the built-in event emission protocol remain connected in a repeatable repository-owned runtime flow, not only in separate unit tests or manual `/tmp` evidence.

## Scope

In scope:
- No-config/no-prompt workspace startup bootstrap.
- Startup artifact generation.
- Resolved `parallel.enabled=true` behavior.
- Live `ralph#1` prompt capture via custom backend.
- `record-session` convergence facts.
- A two-step test flow made of two real `ralph run` invocations.

Out of scope:
- Capability invocation.
- Multi-hat business workflows.
- TUI rendering.
- External backend behavior.

## Target gate

Primary gate:
- `cargo test -p ralph-cli --test integration_startup_resources <new_live_gate_test>`

Companion regression coverage:
- existing dry-run bootstrap integration test
- existing prompt contract / event protocol focused tests

## Assertions

The live gate MUST assert all of the following from one repeatable test flow:

1. `.ralph/bootstrap-selection.json` exists and records startup bootstrap selection.
2. `.ralph/resolved-config.yml` exists and includes:
   - the startup bootstrap coordinator prompt
   - `parallel.enabled=true`
3. The captured live `ralph#1` prompt includes:
   - `Act as Ralph's startup bootstrap coordinator`
   - `## RALPH EVENT EMISSION PROTOCOL`
   - `reply.human.message`
4. `record-session` evidence proves:
   - `ux_mode: parallel-cli`
   - termination reason `CompletionPromise`
5. The run succeeds without requiring workspace `ralph.yml` or `PROMPT.md`.

## Execution shape

The gate is expected to use two real runtime steps:

1. A no-config/no-prompt bootstrap run that produces `.ralph/bootstrap-selection.json` and `.ralph/resolved-config.yml`.
2. A second run that reuses the resolved config artifact after swapping only the backend execution surface to a controlled custom backend for prompt capture.

This keeps the gate aligned with the current product boundary: startup bootstrap chooses workflow and prompt resources first, and the live prompt inspection step must not hot-edit workflow topology.

## Failure diagnostics

On failure, the test should print enough context to identify which contract drifted:
- run stdout
- run stderr
- captured prompt text when available
- resolved config text when available
- record-session summary or raw record-session contents when available
