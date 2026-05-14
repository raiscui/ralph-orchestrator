## 1. OpenSpec artifacts

- [x] 1.1 Create proposal for `runtime-evidence-index-kernel`.
- [x] 1.2 Create design that fixes the Phase 1A / Phase 1B boundary.
- [x] 1.3 Create delta spec for `runtime-evidence-index-kernel`.
- [x] 1.4 Create test plan for the minimal evidence index kernel.
- [x] 1.5 Validate the change with `openspec validate runtime-evidence-index-kernel --type change`.

## 2. Implementation preparation only

- [x] 2.1 Map the final implementation owner module after user approval.
- [x] 2.2 Decide whether the v1 index storage is JSONL or structured JSON after user approval.
- [x] 2.3 Define public Rust types and functions after user approval.

## 3. Contract tests to implement after approval

- [x] 3.1 Add schema serialization tests for minimal `EvidenceIndexEntry`.
- [x] 3.2 Add writer / reader tests for lookup by correlation id.
- [x] 3.3 Add missing artifact marker tests.
- [x] 3.4 Add parent-child link tests for isolated capability invocation artifacts.
- [x] 3.5 Add a fixture-backed test proving at least one existing record-session or event-log artifact can be indexed and resolved.

## 4. Guardrail tests to implement after approval

- [x] 4.1 Add a test proving Phase 1A index lookup does not require evidence CLI / doctor display fields.
- [x] 4.2 Add a test proving runtime graph output is not treated as the durable truth source for index lookup.
- [x] 4.3 Add regression coverage that keeps existing record-session and event logger truth sources readable without the index.

## 5. Verification to run after implementation approval

- [x] 5.1 Run the new focused evidence index tests.
- [x] 5.2 Run existing record-session / event logger focused tests.
- [x] 5.3 Run existing capability invocation artifact tests.
- [x] 5.4 Run relevant replay smoke tests if implementation touches replay or fixture parsing. Not required: implementation only adds standalone core index module and does not touch replay / fixture parsing.
- [x] 5.5 Run `openspec validate runtime-evidence-index-kernel --type change` again after code changes.
