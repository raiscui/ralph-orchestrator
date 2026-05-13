## 1. OpenSpec artifacts

- [x] 1.1 Create proposal for `prompt-contract-runtime-alignment`.
- [x] 1.2 Create design with runtime prompt alignment boundaries.
- [x] 1.3 Create spec for output contract prompt anchors and tests.
- [x] 1.4 Validate the change with `openspec validate prompt-contract-runtime-alignment --type change`.

## 2. Runtime prompt implementation

- [x] 2.1 Update `InstructionBuilder::build_custom_hat` REPORT section to include output contract fields.
- [x] 2.2 Preserve existing evidence-before-completion wording.
- [x] 2.3 Preserve existing must-publish wording for hats with publishable events.

## 3. Tests

- [x] 3.1 Add `InstructionBuilder` unit assertions for outcome/evidence/changed files/known gaps/next suggestions.
- [x] 3.2 Add `EventLoop::build_prompt` integration assertions for the same output contract anchors.
- [x] 3.3 Run focused prompt tests.

## 4. Documentation and manifest

- [x] 4.1 Update `docs/prompt-contract.md` to identify output fields as runtime prompt test anchors.
- [x] 4.2 Register this OpenSpec change in `agent-guidance-manifest.toml`.
- [x] 4.3 Run `ralph verify agent-guidance`.

## 5. Verification

- [x] 5.1 Validate Mermaid blocks with `beautiful-mermaid-rs --ascii`.
- [x] 5.2 Run `openspec validate prompt-contract-runtime-alignment --type change`.
- [x] 5.3 Run focused `ralph-core` prompt tests.
- [x] 5.4 Run `cargo test -p ralph-core smoke_runner`.
- [x] 5.5 Run `cargo test`.
- [x] 5.6 Run `cargo fmt --check`.
- [x] 5.7 Run `git diff --check`.
- [x] 5.8 Run `openspec validate --all --strict`.

## 6. Records

- [x] 6.1 Update `task_plan__guidance_contract_governance.md` after each phase.
- [x] 6.2 Update `WORKLOG__guidance_contract_governance.md` with final evidence.
- [x] 6.3 Update `ERRORFIX__guidance_contract_governance.md` if implementation or validation errors occur.
- [x] 6.4 Keep state operation layer in `LATER_PLANS__guidance_contract_governance.md`, not in this change.
