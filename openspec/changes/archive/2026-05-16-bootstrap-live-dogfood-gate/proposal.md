## Why

The current startup bootstrap and built-in event emission protocol work is verified by unit tests, focused integration tests, and one manually recorded live dogfood run. That is enough to prove the behavior once, but not enough to keep it stable over time.

We need one repeatable repository gate that exercises the real startup path end-to-end enough to prove four runtime facts together:

1. A workspace with no `ralph.yml` and no `PROMPT.md` still bootstraps successfully.
2. The startup bootstrap artifact resolves to `parallel.enabled=true`.
3. The live `ralph#1` coordinator prompt includes the built-in `## RALPH EVENT EMISSION PROTOCOL` contract.
4. The run terminates through record-session evidence as `parallel-cli` + `CompletionPromise`.

Because the current default bootstrap workflow resolves to a builtin backend preset, the repeatable gate should follow the same two-step runtime chain that a human used for dogfood:

1. Run real no-config startup bootstrap to produce durable startup artifacts.
2. Reuse the generated resolved config in a second real run with a controlled custom backend to inspect the live `ralph#1` prompt and record-session evidence.

Without that combined gate, the behavior is split across separate tests and ad hoc `/tmp` evidence, which makes drift easier to miss.

## What Changes

- Add a repeatable CLI integration gate for the startup bootstrap + built-in event protocol live path.
- Reuse the existing custom-backend prompt-capture technique so the test can inspect the real `ralph#1` prompt without introducing a new harness.
- Keep the gate narrow: it validates bootstrap resolution, prompt contract presence, and record-session convergence, but it does not expand into capability invocation or full multi-hat workflow E2E.
- Model the gate as one repository test flow with two real `ralph run` invocations, not as a new runtime feature or a heavyweight E2E framework.

## Impact

- Tightens regression coverage around the current default startup path.
- Turns the previously manual live dogfood chain into repo-native evidence.
- Keeps the truth source in existing runtime artifacts: `.ralph/bootstrap-selection.json`, `.ralph/resolved-config.yml`, captured `ralph#1` prompt, and `record-session` JSONL.
