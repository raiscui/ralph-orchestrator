## 1. OpenSpec artifacts

- [x] 1.1 Create proposal for `ralph-state-cli-adapter`.
- [x] 1.2 Create design with CLI command shape, scope boundaries, JSON output, and non-goals.
- [x] 1.3 Create spec for `state-cli-adapter` requirements.
- [x] 1.4 Validate Mermaid flowchart with `beautiful-mermaid-rs --ascii`.
- [x] 1.5 Validate Mermaid sequence diagram with `beautiful-mermaid-rs --ascii`.
- [x] 1.6 Validate the change with `openspec validate ralph-state-cli-adapter --type change`.

## 2. CLI tests

- [x] 2.1 Add integration test for `ralph state status --json` using state written by `StateOperationStore`.
- [x] 2.2 Add integration test for `ralph state read <mode> --json` missing state behavior.
- [x] 2.3 Add integration test for `ralph state clear <mode>` deleting core-written state.
- [x] 2.4 Add integration test for invalid mode or malformed state error behavior.

## 3. CLI implementation

- [x] 3.1 Add `State` command group to `crates/ralph-cli/src/main.rs`.
- [x] 3.2 Add `status`, `read`, and `clear` subcommands.
- [x] 3.3 Parse mode through `StateMode` rather than a duplicate enum.
- [x] 3.4 Implement handlers using `StateOperationStore` only.
- [x] 3.5 Add human-readable and `--json` output.
- [x] 3.6 Reject `--session-id` plus `--all-sessions` for `clear`.

## 4. Verification

- [x] 4.1 Run focused `ralph-cli` state tests.
- [x] 4.2 Run `cargo fmt --check`.
- [x] 4.3 Run `cargo test -p ralph-core smoke_runner`.
- [x] 4.4 Run `cargo test`.
- [x] 4.5 Run `git diff --check`.
- [x] 4.6 Run `openspec validate --all --strict`.
- [x] 4.7 Run `cargo run -p ralph-cli -- verify agent-guidance --color never`.

## 5. Records

- [x] 5.1 Update `task_plan__guidance_contract_governance.md` after each phase.
- [x] 5.2 Update `WORKLOG__guidance_contract_governance.md` with final evidence.
- [x] 5.3 Update `ERRORFIX__guidance_contract_governance.md` if implementation or validation errors occur.
- [x] 5.4 Review `LATER_PLANS__guidance_contract_governance.md` and `EPIPHANY_LOG__guidance_contract_governance.md` before final response.
