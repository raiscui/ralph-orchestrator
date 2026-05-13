## 1. Specification and Documentation

- [x] 1.1 Add `docs/agent-guidance-schema.md` describing guidance asset types, required sections, and boundaries.
- [x] 1.2 Add `docs/prompt-contract.md` describing prompt / skill / hat / final-response behavior expectations.
- [x] 1.3 Update `AGENTS.md` Project Knowledge Index to include the new long-term governance documents.

## 2. Manifest

- [x] 2.1 Add `agent-guidance-manifest.toml` with schema version and core guidance assets.
- [x] 2.2 Include at minimum `AGENTS.md`, `EXPERIENCE.md`, the new schema doc, the new prompt contract doc, and `specs/oh-my-codex-learning-analysis.md`.
- [x] 2.3 Keep manifest metadata structured; do not rely on comments as machine-readable metadata.

## 3. Verifier and Tests

- [x] 3.1 Add a Rust verifier for `agent-guidance-manifest.toml` covering schema version, unique ids, valid types/statuses, non-empty summaries, safe repository-relative paths, and active file existence.
- [x] 3.2 Verify `required_in_agents_index = true` entries appear in `AGENTS.md`.
- [x] 3.3 Add regression tests for valid manifest, duplicate ids, missing file, invalid enum, path escape, empty summary, and missing AGENTS index.

## 4. Dogfood and Gates

- [x] 4.1 Run `openspec validate agent-guidance-contracts --type change`.
- [x] 4.2 Run the focused verifier tests.
- [x] 4.3 Run `cargo test` before declaring completion.
- [x] 4.4 Record worklog/errorfix/later-plans as needed.
