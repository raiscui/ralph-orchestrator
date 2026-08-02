# record-session-contract-and-watch Specification

## Purpose
TBD - created by archiving change record-session-contract-and-watch. Update Purpose after archive.
## Requirements
### Requirement: record-session 必须包含基本会话元信息
The `ralph run --record-session <FILE>` output MUST include a `_meta.session_start` record so a single JSONL file is self-describing for offline debugging and replay attribution.

#### Scenario: `_meta.session_start` 字段最小集
- **WHEN** a developer runs `ralph run --record-session /tmp/session.jsonl`
- **THEN** the JSONL MUST include a record whose `event` is `_meta.session_start`
- **AND** its `data` MUST include:
  - `cwd`
  - `workspace_root`
  - `argv`
  - `argv_joined`
  - `pid`

---

### Requirement: 可控中断下 JSONL 必须保持可解析并写入终止原因
On SIGINT/SIGTERM, `ralph run --record-session <FILE>` MUST exit cleanly and MUST leave a JSONL file that is line-by-line parseable, including a `_meta.termination` record with reason `Interrupted`.

#### Scenario: Ctrl+C(SIGINT) 后 JSONL 仍可逐行解析
- **WHEN** a developer starts `ralph run --record-session /tmp/session.jsonl` and then sends SIGINT
- **THEN** every complete line in `/tmp/session.jsonl` MUST be valid JSON
- **AND** the file MUST contain `_meta.session_start` and `_meta.loop_start`
- **AND** the file MUST contain a `_meta.termination` record whose `data.reason` is `Interrupted`

---

### Requirement: CLI 必须提供 `ralph record` 命令族
The `ralph` CLI MUST provide a `record` command group that can summarize and watch a record-session JSONL file for fast human debugging.

#### Scenario: record 子命令可发现
- **WHEN** a developer runs `ralph --help`
- **THEN** the help output MUST mention a `record` command

#### Scenario: record 提供 summary 与 watch
- **WHEN** a developer runs `ralph record --help`
- **THEN** the help output MUST mention `summary` and `watch`

---

### Requirement: `record summary` 必须输出稳定的摘要信息
`ralph record summary <FILE>` MUST print a human-readable summary that includes session meta, termination state, bus topic counts, and a stdout tail window.

#### Scenario: summary 输出包含核心字段
- **WHEN** a developer runs `ralph record summary /tmp/session.jsonl`
- **THEN** the output MUST include:
  - `cwd` (if present in `_meta.session_start`)
  - `argv_joined` (if present in `_meta.session_start`)
  - `termination.reason` (if `_meta.termination` exists)
  - at least one line derived from `bus.publish` topic counting (if any exist)

---

### Requirement: `record watch` 必须容忍半行 JSON 并持续输出摘要
`ralph record watch [FILE]` MUST follow an append-only record-session JSONL file and MUST stream newly appended complete JSONL lines to stdout without reformatting, while tolerating a trailing partial line.

#### Scenario: 末尾 incomplete line 不得导致崩溃,且不得被提前输出
- **GIVEN** a record-session file whose last line does not end with `\n` yet (incomplete write)
- **WHEN** a developer runs `ralph record watch <FILE>`
- **THEN** the command MUST NOT crash
- **AND** it MUST continue watching until the line becomes complete or the user terminates the watcher
- **AND** it MUST NOT print the incomplete line before it becomes complete

---

### Requirement: `record watch` 在缺省 FILE 时必须可自动定位最近一次录制
When `ralph run --record-session <FILE>` is used, the CLI MUST write a `.ralph/record-session.latest` pointer under the workspace root so `ralph record watch` can be invoked without providing `<FILE>`.

#### Scenario: `record-session.latest` 指针可用于无参 watch
- **WHEN** a developer runs `ralph run --record-session /tmp/session.jsonl`
- **THEN** the workspace root MUST contain `.ralph/record-session.latest`
- **AND** it MUST resolve to `/tmp/session.jsonl`---

### Requirement: Record summary MUST expose dynamic spawn correlation
`ralph record summary` MUST expose dynamic spawn correlation when a record-session contains parent-visible dynamic spawn events.

The summary MUST include `topology.spawn_group` count, `topology.spawn.result` count, `topology.spawn.failed` count, spawned instance ids, source instances for result topics, and final termination state.

#### Scenario: summary shows spawned instances and result coverage
- **WHEN** a record-session contains `topology.spawn.result` for `builder#2` through `builder#6` and matching `analysis.done` events
- **THEN** `ralph record summary` MUST show the spawned instance ids or enough source-instance coverage to verify the dynamic run
- **AND** it MUST show `analysis.done` source instances without requiring manual JSONL scanning

#### Scenario: summary distinguishes spawn success from workflow completion
- **WHEN** a record-session has `topology.spawn.result` but lacks `_meta.termination`
- **THEN** `ralph record summary` MUST make the missing termination visible
- **AND** it MUST NOT imply that spawn success alone means the workflow completed

### Requirement: Record summary MUST distinguish semantic completion from wrapper exit status
`ralph record summary` MUST treat record-session `_meta.termination` as the primary semantic completion signal for a recorded run.

Wrapper shell status, stdout tails, and TUI display state MAY be useful diagnostics, but they MUST NOT override a parseable record-session termination reason.

#### Scenario: wrapper script fails after record-session completion
- **WHEN** an outer shell wrapper fails after the Ralph run writes `_meta.termination.reason = CompletionPromise`
- **THEN** the summary MUST still report the semantic termination reason from the record-session
- **AND** a reviewer MUST be able to separate wrapper failure from runtime semantic failure

### Requirement: Record summary with agents file MUST distinguish current registry and completed dynamics
`ralph record summary --agents-file` MUST distinguish currently registered instances from completed dynamic tombstones when the agents sidecar contains both.

The summary MUST not present current registry snapshots as the complete history of dynamic instances unless completed tombstones or record-session spawn/result evidence are also consulted.

#### Scenario: completed dynamic instances are displayed separately
- **WHEN** dynamic instances have completed and been reaped before summary time
- **THEN** the summary MUST show completed dynamic instances separately from current registry instances
- **AND** the summary MUST still allow source-instance result coverage to be verified

