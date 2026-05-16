## 1. Spec and design
- [x] 1.1 Create proposal for `capability-failure-branching-matrix`.
- [x] 1.2 Create design for class-specific parent branching without retry engine.
- [x] 1.3 Add delta spec for richer branching matrix.
- [x] 1.4 Validate the change with `openspec validate capability-failure-branching-matrix --type change`.

## 2. Implementation
- [x] 2.1 Add deterministic backend script for malformed-request diagnostic branch.
- [x] 2.2 Add live integration gate for `malformed_request -> reply.human.message`.
- [x] 2.3 Assert malformed branch does not require fallback `capability.result`.

## 3. Verification and archive
- [x] 3.1 Run focused live capability integration tests.
- [x] 3.2 Run smoke tests.
- [x] 3.3 Run full test suite.
- [x] 3.4 Archive the change and confirm stable spec sync.
- [x] 3.5 Record notes / worklog / task plan completion.
- [x] 3.6 Create a local commit.
