# spec: local hat imports in preflight (port of origin #209 Phase 1)

## Goal

Allow a `hats:` entry in `ralph.yml` (or any local file-source hat config) to
import a base hat from a relative file path, with imported fields as the base
and local fields overriding at field level. This unblocks hat reuse across
presets without copy-paste.

## Source

- origin commit: `25afeb084c8a67797c3d9c676b3a90d62da4d5df feat(hats): support local hat imports in preflight` (#209, Phase 1)
- origin task description: `.ralph/tasks/issue-209-hat-imports.code-task.md`
- Origin PR: https://github.com/mikeyobrien/ralph-orchestrator/pull/360 (verify before land)

## Schema (user-facing)

In any local file source's `hats:` block, a hat may include an `imports:` key:

```yaml
hats:
  my_hat:
    imports: ../shared/base-hat.yml
    description: local override
    instructions: |
      Extra instructions on top of base.
    publishes: [local.done]    # replaces base.publishes
    triggers: [build.task]     # replaces base.triggers
```

Resolution rules:

- `imports` MUST be a single string (relative path).
- The import is resolved relative to the importing file's directory.
- Imported hat fields form the base; local fields override at field level.
- Other file sources (builtin / remote / override) MUST NOT contain `imports:`.
- Reject transitive imports (A imports B which imports C).
- Reject non-string `imports:` values.
- Reject missing import files.
- Reject invalid YAML in imported file.
- Reject `events:` field inside an imported hat (no event metadata import).

## Architecture

Local main does NOT have `preflight.rs` (preflight merged into `autopilot.rs`).
The hats YAML is loaded via `serde_yaml::from_str(content)` directly into
`RalphConfig`. To port the feature with minimal invasion:

### Approach: pre-processing step

1. Parse YAML content to `serde_yaml::Mapping` (no `RalphConfig` binding yet)
2. Walk the `hats:` sub-mapping and resolve any `imports:` keys
3. Serialize Mapping back to YAML string
4. Continue with existing `serde_yaml::from_str(content)` → `RalphConfig`

This keeps `HatConfig` unchanged (per origin spec) and localizes all new code
to a single module: `crates/ralph-core/src/hat_imports.rs`.

### New module

```
crates/ralph-core/src/hat_imports.rs   (~300 LOC + tests)
  pub fn resolve_hat_imports_in_mapping(
      mapping: &mut Mapping,
      base_dir: &Path,
      source_label: &str,
  ) -> Result<()>

  pub fn reject_hat_imports_in_mapping(
      mapping: &Mapping,
      source_label: &str,
      reason: UnsupportedImportSource,
  ) -> Result<()>

  pub enum UnsupportedImportSource {
      Builtin,
      Remote,
  }
```

### Wire-in point

Modify `crates/ralph-core/src/config.rs::RalphConfig::from_yaml` (or
equivalent entry point) so it:

1. Parses content to `Mapping` first
2. Calls `resolve_hat_imports_in_mapping` for local file sources
3. Calls `reject_hat_imports_in_mapping` for builtin/remote sources
4. Then parses Mapping → `RalphConfig`

Or, in the case where `RalphConfig` already accepts the file via a function
wrapper, intercept before the typed parse.

## Out of scope (Phase 2+)

- Multi-file single-hat config (origin's Phase 2)
- Directory-based hat collection (`hats/*.yml` glob)
- HTTP / git-based imports
- Import-cycle detection beyond direct transitive (only direct reject)

## Acceptance criteria

1. Local file source: `imports: ../shared/base.yml` resolves and merges fields
2. Builtin source: any `imports:` key → typed error
3. Remote source: any `imports:` key → typed error
4. Transitive: A imports B which has `imports:` → typed error
5. Missing file: `imports: missing.yml` → typed error with source label
6. Invalid YAML in imported file → typed error
7. Non-string `imports:` → typed error
8. Imported `events:` field → typed error
9. `HatConfig` Rust struct unchanged
10. Existing hats / presets without `imports:` keep working (no behavior change)

## Verification

- Unit tests in `hat_imports.rs` (8 acceptance cases × edge cases)
- Integration test in `crates/ralph-cli/tests/integration_hat_imports.rs`:
  - Write a small ralph.yml + base hat file, parse via `RalphConfig::from_yaml`
  - Assert the merged hat has base + override fields
- `cargo test -p ralph-core` green
- `cargo test -p ralph-cli` green
- Live smoke: create `examples/hat-import-demo/`, `cargo run -p ralph-cli -- preflight` (if preflight subcommand exists; otherwise skip)

## Risk

- YAML parsing order change: if anywhere in the code parses YAML content
  bypassing `RalphConfig::from_yaml`, hat imports won't be resolved there.
- Resolution happens once at load. If the same file is parsed multiple times
  (e.g., per-iteration), resolution cost is repeated. Acceptable for Phase 1.
- Imported file content is read at parse time. No hot-reload. Acceptable.

## Migration

Existing hats / presets: zero impact. No `imports:` usage locally means
nothing changes for current users.

When users want to share a hat, they add `imports: ../shared/foo.yml` to
their local override.
