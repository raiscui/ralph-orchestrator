## 1. OpenSpec artifacts

- [x] 1.1 Create proposal for `agent-guidance-catalog-cli`.
- [x] 1.2 Create design with CLI, verifier, skill root, and frontmatter rules.
- [x] 1.3 Create spec for skill catalog and standalone verifier CLI behavior.
- [x] 1.4 Validate the change with `openspec validate agent-guidance-catalog-cli --type change`.

## 2. Verifier implementation

- [x] 2.1 Extend `ralph-core::agent_guidance_manifest` with `GuidanceManifestReport`.
- [x] 2.2 Keep existing `verify_default_manifest` and `verify_manifest_at` compatibility wrappers.
- [x] 2.3 Add skill root validation for `.agents/skills/*/SKILL.md` and `.codex/skills/*/SKILL.md`.
- [x] 2.4 Add frontmatter parsing for `skill` assets and require non-empty `name` and `description`.
- [x] 2.5 Reject duplicate active/draft skill names.
- [x] 2.6 Add focused unit tests for valid skills, invalid root, missing frontmatter fields, and duplicate skill names.

## 3. Manifest catalog

- [x] 3.1 Add project-owned `.agents/skills/*/SKILL.md` entries to `agent-guidance-manifest.toml`.
- [x] 3.2 Add project-owned `.codex/skills/*/SKILL.md` entries to `agent-guidance-manifest.toml`.
- [x] 3.3 Run repository manifest dogfood to confirm all registered skills pass.

## 4. CLI implementation

- [x] 4.1 Add `ralph verify` command group.
- [x] 4.2 Add `ralph verify agent-guidance` subcommand with optional `--manifest`.
- [x] 4.3 Print success summary with manifest path, asset count, and skill count.
- [x] 4.4 Return non-zero failure with verifier error context.
- [x] 4.5 Add CLI-level test or focused command test where practical.

## 5. Verification

- [x] 5.1 Run `openspec validate agent-guidance-catalog-cli --type change`.
- [x] 5.2 Validate Mermaid blocks in design with `beautiful-mermaid-rs --ascii`.
- [x] 5.3 Run `cargo test --package ralph-core --lib agent_guidance_manifest`.
- [x] 5.4 Run focused `ralph-cli` tests for the verify command if present.
- [x] 5.5 Run `cargo test -p ralph-core smoke_runner`.
- [x] 5.6 Run `cargo test`.
- [x] 5.7 Run `cargo fmt --check`.
- [x] 5.8 Run `git diff --check`.
- [x] 5.9 Run `openspec validate --all --strict`.

## 6. Records

- [x] 6.1 Update `task_plan__guidance_contract_governance.md` after each phase.
- [x] 6.2 Update `WORKLOG__guidance_contract_governance.md` with final evidence.
- [x] 6.3 Update `ERRORFIX__guidance_contract_governance.md` if implementation or validation errors occur.
- [x] 6.4 Review `LATER_PLANS__guidance_contract_governance.md` and `EPIPHANY_LOG__guidance_contract_governance.md` before final response.
