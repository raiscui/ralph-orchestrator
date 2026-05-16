# Proposal: internalize-event-emission-protocol

## Why

`/Users/cuiluming/local_doc/l_dev/my/rust/ralph-example/ralph.yml` repeats generic Ralph event-emission instructions in the execution-directory config, including examples such as:

```text
<event topic="experiment.result">
...payload...
</event>
```

Those examples are still broadly correct today, but they are runtime protocol knowledge. Keeping them in every execution-directory `ralph.yml` makes examples and user configs drift-prone as Ralph evolves.

Ralph already has a compiled all-hat prompt overlay (`config/all_hat.md`) and coordinator-specific instruction builders. The stable event envelope, stdout-only rule, reply attributes, completion-promise boundary, and supported routing attributes should live in those built-in prompt surfaces, not in every local workflow config.

## What Changes

- Define a built-in runtime event-emission protocol contract for generated / injected parallel-mode prompts.
- Require the built-in contract to cover the stable `<event ...>...</event>` envelope and supported attributes.
- Require execution-directory `ralph.yml` files to only describe workflow-specific topics, payload fields, and business backpressure rules.
- Keep workflow-specific payload schemas in config, because those are domain-specific and not known to the runtime.
- Preserve the existing parser contract: raw event tags emitted in the current hat stdout are routed; tool transcript / stderr / files are not normal workflow event output.
- Avoid relying on raw `<event>` examples in the compiled all-hat overlay unless they are escaped or rendered as non-emittable documentation.

## Non-goals

- Do not change the event parser syntax in this change.
- Do not remove workflow-specific topic names such as `experiment.result`, `experiment.reviewed`, or `integration.applied` from configs.
- Do not infer payload schemas from `hats.*.publishes`; payload requirements remain workflow-specific.
- Do not hot-switch runtime topology.
- Do not edit `ralph-example/ralph.yml` before the spec and test plan are accepted.

## Impact

- Runtime prompt construction for parallel hat instances and `ralph#1` coordinator.
- `config/all_hat.md` / prompt overlay wording and escaping rules.
- Example config hygiene: local configs should become thinner and less protocol-version-sensitive.
- Regression tests should assert stable markers / contract fields rather than full prose.
