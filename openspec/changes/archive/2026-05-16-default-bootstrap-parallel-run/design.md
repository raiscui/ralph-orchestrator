# Design: default-bootstrap-parallel-run

## Context

`startup_resources::resolve_default_bootstrap()` chooses the embedded `workflow:feature-minimal` workflow and `prompt:bootstrap-default-task` prompt when the default `ralph.yml` is missing and the user did not provide explicit prompt/config input.

Before this change, the selected workflow did not necessarily include `parallel.enabled=true`, so the resolved startup config could behave like a serial fallback rather than the desired modern default run shape.

## Decision

Set `config.parallel.enabled = true` inside `resolve_workflow_with_prompt_template()` for bootstrap-resolved configs.

Why this location:

- It is after the embedded workflow has been parsed into structured `RalphConfig`.
- It affects only startup bootstrap composition, not explicit config loading.
- It keeps the product rule near the startup-only merge boundary.
- It avoids copying or forking the embedded preset just to add one runtime-mode invariant.

## Guardrails

- `should_bootstrap_missing_default_config(...)` remains narrow.
- Explicit `--config ralph.yml` still bypasses startup bootstrap.
- The resolved config artifact remains `.ralph/resolved-config.yml`.
- Runtime topology is still immutable after the real run begins.

## Verification

- Unit test asserts default bootstrap resolution has inline prompt and `parallel.enabled=true`.
- Integration test runs `ralph run --dry-run --no-tui` in an empty workspace and asserts `.ralph/resolved-config.yml` includes `parallel.enabled=true`.
- Existing explicit missing config test asserts bootstrap artifacts are not written when config is explicit.
