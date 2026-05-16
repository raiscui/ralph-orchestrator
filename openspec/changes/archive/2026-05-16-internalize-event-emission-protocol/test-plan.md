# Test Plan: internalize-event-emission-protocol

## Acceptance Criteria

- Event parser syntax remains unchanged.
- Runtime prompts for publishing parallel hats include a stable built-in event emission protocol marker.
- The built-in section includes the event envelope, `topic`, stdout-only rule, and stable attributes.
- A workflow config can omit repeated generic event-format tutorial text while generated runtime prompts still teach event emission.
- Workflow-specific payload fields remain in workflow config / prompt, not in generic core protocol.
- Shared all-hat overlay examples remain escaped / non-emittable to avoid accidental event replay.

## Focused Tests

### Focused: event emission protocol renderer

Command:

```bash
cargo test -p ralph-core event_emission_protocol
```

Assertions:

- rendered text contains stable marker `RALPH EVENT EMISSION PROTOCOL`
- rendered text contains `<event`
- rendered text contains `topic`
- rendered text mentions stdout-only / final assistant output rule
- rendered text mentions `id`, `reply`, `target`, `target_instance`, `session_strategy`, `workspace_strategy`, `turn_action`, and `spawn_instance`
- rendered text mentions that `LOOP_COMPLETE` belongs outside event payloads

### Focused: parallel hat prompt includes built-in protocol

Command:

```bash
cargo test -p ralph-core parallel_prompt_includes_event_emission_protocol
```

Assertions:

- generated prompt for a hat with `publishes` contains the stable protocol marker
- generated prompt contains the hat's workflow-specific publish topic from config
- generated prompt still includes the workflow-specific instructions / payload requirements
- test config does not include duplicated generic event-format tutorial text

### Focused: coordinator prompt uses same source of truth

Command:

```bash
cargo test -p ralph-core ralph_coordinator_event_protocol
```

Assertions:

- `ralph#1` coordinator instructions include the stable protocol marker or use the same renderer
- coordinator-specific reply/human-message guidance remains present
- duplicate, divergent event envelope prose is not introduced

### Regression: all-hat overlay examples stay escaped

Command:

```bash
cargo test -p ralph-core prompt_overlay
```

Assertions:

- injected `config/all_hat.md` examples do not leave raw `<event topic="...">` examples from the shared overlay
- runtime id remains the first prompt line

## Example Dogfood After Implementation

Use `/Users/cuiluming/local_doc/l_dev/my/rust/ralph-example/ralph.yml` or a copied fixture:

1. Remove generic `发事件必须使用如下格式` blocks from the local config.
2. Keep workflow-specific topic and payload requirements.
3. Run a dry-run or deterministic backend integration that captures the generated prompt.
4. Assert generated prompts still contain built-in event emission protocol.
5. Assert the local config no longer needs to duplicate the generic envelope tutorial.

## Regression Gates

```bash
openspec validate internalize-event-emission-protocol --type change
openspec validate --all --strict
cargo fmt --all -- --check
cargo test -p ralph-core event_parser::tests
cargo test -p ralph-core prompt_overlay
cargo test -p ralph-core smoke_runner
cargo test
git diff --check
```
