## 1. OpenSpec artifacts

- [x] 1.1 Create proposal for `state-operation-layer`.
- [x] 1.2 Create design with state operation boundaries and non-goals.
- [x] 1.3 Create spec for unified state operations, atomic writes, scopes, fields, and existing truth-source boundaries.
- [x] 1.4 Validate the change with `openspec validate state-operation-layer --type change`.

## 2. Design validation

- [x] 2.1 Validate Mermaid flowchart with `beautiful-mermaid-rs --ascii`.
- [x] 2.2 Validate Mermaid sequence diagram with `beautiful-mermaid-rs --ascii`.
- [x] 2.3 Confirm design keeps state operation layer separate from guidance catalog/CLI and prompt contract changes.

## 3. Implementation tasks

- [x] 3.1 Add `ralph-core` state operation data types and mode validation.
- [x] 3.2 Add path resolver for `.ralph/state/<mode>-state.json` and `.ralph/state/sessions/<session_id>/<mode>-state.json`.
- [x] 3.3 Add atomic write helper and per-path write serialization.
- [x] 3.4 Add `state_read`, `state_write`, `state_clear`, `state_list_active`, and `state_get_status` core functions.
- [x] 3.5 Add unit tests for unsupported modes, malformed JSON, merge semantics, scope precedence, clear semantics, and concurrent writes.
- [x] 3.6 Only after core tests pass, decide whether to add CLI or MCP adapters in a follow-up implementation phase.

## 4. Records

- [x] 4.1 Update `task_plan__guidance_contract_governance.md` after each phase.
- [x] 4.2 Update `WORKLOG__guidance_contract_governance.md` with final OpenSpec evidence.
- [x] 4.3 Update `LATER_PLANS__guidance_contract_governance.md` so state operation layer is no longer listed as merely pending analysis.
- [x] 4.4 Do not create `ERRORFIX__guidance_contract_governance.md` entries unless validation fails.
