# resource-bootstrap Specification

## Purpose
TBD - created by archiving change startup-resource-bootstrap. Update Purpose after archive.
## Requirements
### Requirement: Startup resource metadata MUST be available as structured machine-readable data
Ralph MUST represent startup resource summaries and selection hints as structured machine-readable metadata instead of depending on YAML comments at runtime.

Human-oriented header comments MAY remain in embedded files or materialized resources, but startup resolution MUST still work when those comments are absent or not preserved.

#### Scenario: Selector reads structured metadata from catalog
- **WHEN** the bootstrap selector enumerates workflow candidates
- **THEN** it MUST be able to read each candidate's summary, kind, and selection flags from structured catalog metadata

#### Scenario: Missing header comments do not break startup resolution
- **WHEN** a workflow resource is available but its YAML header comments are absent, stripped, or ignored by the parser
- **THEN** startup resolution MUST still behave correctly using structured metadata

---

### Requirement: Ralph MUST resolve startup resources without requiring workspace `ralph.yml` or `PROMPT.md`
Ralph MUST support a startup resolution flow that can produce a valid run configuration even when the current workspace does not contain `ralph.yml` and does not contain `PROMPT.md`.

This startup resolution MUST treat configuration sources and prompt sources as layered resources rather than assuming the default task input always lives in a workspace file.

#### Scenario: Missing `ralph.yml` falls back to startup resource resolution
- **WHEN** the user runs `ralph run` in a workspace that has no `ralph.yml`
- **THEN** Ralph MUST enter startup resource resolution instead of failing on the missing config file

#### Scenario: Missing `PROMPT.md` uses a resolved prompt source instead of hard failure
- **WHEN** the user runs `ralph run` without CLI prompt input and the workspace has no `PROMPT.md`
- **THEN** Ralph MUST resolve a prompt source from catalog resources or idle bootstrap strategy instead of immediately failing on the missing file

---

### Requirement: Ralph MUST synchronize embedded startup resources into a user resource root
Ralph MUST provide a user-editable resource root that can be initialized from embedded startup resources distributed with the binary.

The synchronization flow MUST support first-use materialization and MUST NOT silently overwrite user-modified resource files during later runs.

#### Scenario: First use materializes embedded resources
- **WHEN** Ralph needs startup resources and the user resource root has not been initialized yet
- **THEN** Ralph MUST materialize the required embedded resources into the user resource root before continuing startup resolution

#### Scenario: User-modified resources are preserved
- **WHEN** a resource file in the user resource root has been modified by the user
- **THEN** a later startup sync MUST NOT silently replace that file with the embedded version

---

### Requirement: Ralph MUST finish preset selection before starting the real orchestration loop
Ralph MUST perform preset or resource selection in a bootstrap phase that completes before `EventLoop` or parallel `Supervisor` initialization.

The bootstrap phase MUST produce one resolved configuration artifact that is then used to start the real run.

#### Scenario: Explicit config source bypasses bootstrap selector
- **WHEN** the user provides an explicit config source such as `-c path/to/file.yml` or `-c builtin:feature`
- **THEN** Ralph MUST skip bootstrap preset selection and use the explicit source directly

#### Scenario: Bootstrap selector emits a resolved configuration before real run
- **WHEN** the user runs Ralph without an explicit config source
- **THEN** Ralph MUST select resources, produce a resolved configuration artifact, and only then initialize the real orchestration loop

---

### Requirement: Startup composition MUST use deterministic structured merge rules
The startup selector MUST compose resources using deterministic structured roles rather than arbitrary YAML text concatenation.

At minimum, the composition model MUST distinguish workflow presets, backend presets, overlays, prompt templates, and example bundles.

#### Scenario: Workflow and backend resources merge in a fixed order
- **WHEN** the selector composes one workflow preset with one backend preset
- **THEN** the final configuration MUST be produced using a deterministic merge order documented by the system

#### Scenario: Conflicting structured keys do not silently drift
- **WHEN** two selected resources define incompatible values for the same structured key
- **THEN** the startup composition flow MUST apply an explicit conflict rule or fail loudly rather than silently producing an ambiguous result

---

### Requirement: Example bundles MUST NOT be auto-selected as normal startup workflows
The system MUST treat example bundles as a separate resource kind from normal workflow presets.

Example bundles MAY be materialized or explicitly invoked, but they MUST NOT participate in default startup selection unless the user explicitly chooses them.

#### Scenario: Default selector excludes examples
- **WHEN** the bootstrap selector is choosing a default workflow for a normal `ralph run`
- **THEN** selector-ineligible example bundles MUST NOT be considered workflow candidates

#### Scenario: Explicit example request still works
- **WHEN** the user explicitly requests an example bundle or materializes one through a dedicated command flow
- **THEN** the example bundle MUST remain available for that explicit use case

---

### Requirement: Ralph MUST NOT hot-switch the full startup topology after the real run begins
Once the real orchestration loop has started from a resolved configuration, the system MUST NOT replace the full startup topology with a different preset set during that same run.

This guardrail preserves existing validation guarantees for routing, completion semantics, and parallel topic contracts.

#### Scenario: Selection completes before `EventLoop` initialization
- **WHEN** Ralph starts a run through bootstrap selection
- **THEN** all preset selection and topology composition MUST be complete before `EventLoop` initialization begins

#### Scenario: Runtime topology replacement is rejected
- **WHEN** a later runtime action attempts to replace the active workflow with a different full preset set after the real run has started
- **THEN** the system MUST reject that action rather than hot-switching the live topology

