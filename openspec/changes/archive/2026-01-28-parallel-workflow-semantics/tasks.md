## 1. Config / Schema

- [x] 1.1 Add `event_loop.complete_publishes: Option<String>` to `EventLoopConfig` with serde support and validation (must be non-empty when set)
- [x] 1.2 Update config documentation and API docs to include `event_loop.complete_publishes` and clarify `event_loop.starting_event` semantics (not “first event published”)
- [x] 1.3 Add unit tests for parsing/validation of `event_loop.complete_publishes`

## 2. Parallel Start Semantics

- [x] 2.1 Treat `task.start` / `task.resume` as control-plane topics in parallel mode and ensure they are routed to `ralph#1`
- [x] 2.2 Add routing tests to guarantee other hats cannot intercept `task.start` / `task.resume` via triggers (no prompt pollution)

## 3. Orphan / Fallback Routing (BREAKING)

- [x] 3.1 Update trigger-derived routing so `ralph#1` is only selected when there are no subscribers at all (true orphan)
- [x] 3.2 Add routing tests covering: (a) wildcard manager receives without ralph, (b) true orphan escalates to ralph#1
- [x] 3.3 Update any affected examples/fixtures that relied on the previous “ralph always included in fallback” behavior

## 4. Coordinator Prompt Semantics (Parallel)

- [x] 4.1 Align parallel ralph#1 instructions with HatlessRalph semantics: explain `starting_event` as workflow entry event and `complete_publishes` as completion candidate
- [x] 4.2 Ensure `starting_event` is reflected in coordinator behavior (fresh run publishes the configured entry topic when set)

## 5. Docs + Smoke Tests

- [x] 5.1 Fix docs that describe `starting_event` as “first event published” and update architecture diagrams accordingly
- [x] 5.2 Add/adjust replay smoke tests to cover: chain fallback boundary + completion candidate event → coordinator-controlled LOOP_COMPLETE

## 6. Examples + E2E (Chinese)

- [x] 6.1 Translate `examples/parallel-trigger-routing/ralph.yml` prompt/description/instructions to Chinese (keep topics + keys unchanged)
- [x] 6.2 Add a Chinese parallel E2E scenario (`parallel-hat-instances-zh`) covering the same routing + autoscale + strict-target assertions
- [x] 6.3 Run the parallel E2E scenario twice with prompt variants (variant1 + variant2) and record results (stability / robustness)
