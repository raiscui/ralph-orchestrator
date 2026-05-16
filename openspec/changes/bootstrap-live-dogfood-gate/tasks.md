## 1. Specification
- [x] 1.1 Add a resource-bootstrap delta requiring a repeatable live startup bootstrap gate.
- [x] 1.2 Add a prompt-contract runtime-alignment delta requiring the live gate to verify the built-in event emission protocol in the real coordinator prompt.
- [x] 1.3 Write a focused test plan describing the runtime artifacts and assertions.

## 2. Implementation
- [x] 2.1 Add or extend a CLI integration test that runs `ralph run` from a no-config/no-prompt workspace using a controlled custom backend.
- [x] 2.2 Assert bootstrap artifacts, resolved parallel config, live `ralph#1` prompt markers, and record-session termination facts.
- [x] 2.3 Keep the test narrow and deterministic without introducing a new heavyweight E2E harness.

## 3. Verification
- [x] 3.1 Run `openspec validate bootstrap-live-dogfood-gate --type change`.
- [x] 3.2 Run focused CLI integration tests for startup bootstrap and the new live gate.
- [x] 3.3 Run relevant smoke/full validation before finalizing.
