# Design: internalize-event-emission-protocol

## Current Facts

- `EventParser` still parses XML-style event tags from stdout, for example `<event topic="impl.done">payload</event>`.
- The parser supports attributes including `id`, `reply`, `topic`, `target`, `target_instance`, `audience_instances`, `require_delivery`, `workspace_strategy`, `session_strategy`, `turn_action`, and `spawn_instance`.
- The parser supports multiple event blocks in one output and accepts multiline opening tags.
- `LOOP_COMPLETE` is a completion promise only when it appears outside event tags and occupies its own line.
- In parallel mode, job completion parsing uses stdout-only output, so event tags in stderr, tool transcript, or files are not normal workflow return events.
- `config/all_hat.md` is compiled into `ralph-core` and injected into all hat prompts by `prompt_overlay`.
- `prompt_overlay` currently escapes raw `<event>` examples from the all-hat overlay to reduce accidental event replay.
- `ParallelSupervisor::build_ralph_coordinator_instructions()` already includes coordinator-specific raw event emission examples.

## Decision

Create a built-in event emission protocol section as a first-class runtime prompt contract.

The contract should be injected by Ralph, not copied into each execution-directory `ralph.yml`.

It should describe:

- the raw event envelope: `<event topic="..."><payload></event>`
- that payload may be text or JSON / YAML-like text, depending on workflow contract
- supported stable attributes, especially `id`, `reply`, `target`, `target_instance`, `session_strategy`, `workspace_strategy`, `turn_action`, and `spawn_instance`
- stdout-only emission: final assistant output for the current job is the normal workflow event channel
- forbidden indirect emission: no shell `echo`, no file writes, no stderr/tool transcript as the normal return path
- completion promise boundary: `LOOP_COMPLETE` is event-outside text and should not be embedded in event payloads
- multiple event blocks may be emitted in one assistant output when the workflow requires fanout / batching

## What stays in workflow config

Workflow config should still define domain-specific behavior:

- topic names: `experiment.task`, `experiment.result`, `experiment.reviewed`, `integration.applied`, etc.
- payload field requirements: `run_id`, `experiment_id`, `verification_evidence`, `commit`, etc.
- role-specific backpressure rules
- workflow-specific convergence conditions
- selection criteria and integration policy

In other words: Ralph owns the event envelope and routing mechanics; workflow config owns the business protocol.

## Injection shape

Recommended implementation shape:

1. Add a small core renderer, for example `render_event_emission_protocol()`, with stable marker text such as `## RALPH EVENT EMISSION PROTOCOL`.
2. Inject it into parallel hat prompts near the existing `Incoming Events` guidance.
3. For `ralph#1`, either reuse the same renderer plus coordinator-specific additions, or ensure `build_ralph_coordinator_instructions()` uses the same source of truth.
4. Keep `config/all_hat.md` as human/system guidance, but do not rely on unescaped raw event examples there as the machine contract.
5. Add tests that assert presence of the stable marker and supported attributes in generated runtime prompts.

## Guardrails

- Do not make tests assert long prose paragraphs.
- Do not remove escaping of raw `<event>` examples in all-hat overlay unless tests prove no accidental replay risk.
- Do not change parser syntax as part of this cleanup.
- Do not move workflow-specific payload schemas into core.
- Do not rewrite all example configs in the same first implementation unless the built-in prompt contract is already covered by focused tests.

## Migration path

1. Land built-in event emission protocol renderer and prompt tests.
2. Dogfood with one example config by deleting redundant generic event-format blocks while keeping topic and payload fields.
3. Run the example smoke / E2E fixture that covers that workflow.
4. After one example is stable, clean other examples in smaller batches.
