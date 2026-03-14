## ADDED Requirements

### Requirement: 新增`ralph autopilot`命令族(就地运行 + JSONL判定)
The `ralph` CLI MUST provide an `autopilot` subcommand that supports both `run` and `analyze`, so a Git repo can be exercised in-place and the resulting `--record-session` JSONL can be judged without human intervention.

#### Scenario: autopilot 子命令可发现
- **WHEN** a developer runs `ralph --help`
- **THEN** the help output MUST mention an `autopilot` command

#### Scenario: autopilot 支持 run 与 analyze
- **WHEN** a developer runs `ralph autopilot --help`
- **THEN** the help output MUST mention `run` and `analyze` subcommands

---

### Requirement: `autopilot run`必须在已初始化Git仓库内就地执行`ralph run --record-session`
`ralph autopilot run` MUST execute a real `ralph run` process inside a specified, already-initialized Git repository directory (worktree-compatible), and MUST force session recording via `--record-session` while running headlessly (`--no-tui`).

#### Scenario: 以repo-dir为工作目录启动真实的`ralph run`
- **WHEN** a developer runs `ralph autopilot run --repo-dir <git-repo> --record-session <file>.jsonl`
- **THEN** the child `ralph run` process MUST run with its current working directory set to `<git-repo>`
- **AND** the child process MUST receive `--record-session <file>.jsonl`
- **AND** the child process MUST receive `--no-tui`

#### Scenario: 非Git目录必须快速失败
- **WHEN** `--repo-dir` points to a directory that is not a Git repository
- **THEN** `ralph autopilot run` MUST fail before starting the child `ralph run` process
- **AND** it MUST explain that a Git-initialized repo is required for worktree-based workflows

#### Scenario: record-session 目标路径不可写必须快速失败
- **WHEN** the parent directory of `--record-session <file>.jsonl` is not writable
- **THEN** `ralph autopilot run` MUST fail before starting the child `ralph run` process

---

### Requirement: Autopilot 的硬判定必须以record-session JSONL为主证据源
Autopilot MUST treat the `--record-session` JSONL file as the primary and sufficient evidence source for automated verdicts, and MUST compute a deterministic hard verdict by parsing `bus.publish` records from that JSONL (not by scraping stdout/stderr).

#### Scenario: 严格闭环topic链路必须满足
- **WHEN** `ralph autopilot analyze --record-session <file>.jsonl` evaluates a completed run
- **THEN** it MUST observe `bus.publish` events for the required topics:
  - `experiment.task`
  - `experiment.result` (payload MUST contain `commit`)
  - `experiment.reviewed` (payload MUST indicate `evidence_ok=true`)
  - `integration.task`
  - `integration.applied`
  - `experiment.complete`

#### Scenario: 禁止出现需要人工介入或路由异常的topic
- **WHEN** `ralph autopilot analyze --record-session <file>.jsonl` evaluates a run
- **THEN** it MUST FAIL the hard verdict if it observes any of these topics:
  - `gate.request`
  - `gate.resolve`
  - `gate.timeout`
  - `routing.escalate`

#### Scenario: 必须以CompletionPromise结束
- **WHEN** `ralph autopilot analyze --record-session <file>.jsonl` evaluates a run
- **THEN** it MUST find a `_meta.termination` record whose `reason` is `CompletionPromise`

---

### Requirement: `autopilot analyze`必须支持离线判定与报告生成
`ralph autopilot analyze` MUST analyze an existing `--record-session` JSONL without re-running the workflow, and MUST produce a machine-readable report and a stable exit code suitable for unattended automation.

#### Scenario: analyze 不触发任何新运行
- **WHEN** a developer runs `ralph autopilot analyze --record-session <file>.jsonl`
- **THEN** it MUST NOT spawn a new `ralph run` process for the target repo workflow

#### Scenario: analyze 生成report.json与report.md
- **WHEN** `ralph autopilot analyze --record-session <file>.jsonl` completes
- **THEN** it MUST write `report.json` and `report.md` under an output directory
- **AND** `report.json` MUST include the absolute (or repo-resolved) path to the analyzed JSONL

---

### Requirement: 必须提供agent分析(基于证据包),用于判断是否满足程序设计要求
After the hard verdict passes, Autopilot MUST perform an agent analysis step that consumes a bounded "evidence pack" derived from the record-session JSONL and MUST output structured JSON verdicts about whether the run meets the intended program design requirements.

#### Scenario: 证据包必须可审计且有体积预算
- **WHEN** Autopilot prepares input for agent analysis
- **THEN** it MUST write an `analysis_input.json` (evidence pack) to disk
- **AND** it MUST include at minimum:
  - required/banned topic findings
  - experiment commit list (from `experiment.result`)
  - termination reason (`_meta.termination`)
  - a tail window of terminal output context (from `ux.terminal.write`)
- **AND** it MUST enforce a size budget so the analysis input is bounded

#### Scenario: agent分析输出必须是结构化JSON
- **WHEN** agent analysis completes
- **THEN** Autopilot MUST write `analysis_output.json` containing:
  - `verdict`: `pass` or `fail`
  - `quality_score`: `optimal|good|acceptable|suboptimal`
  - `requirements_met`: per-check pass/fail with evidence references
  - `risks` and `suggested_fixes`

#### Scenario: agent分析失败必须可区分退出码
- **WHEN** hard verdict passes but agent analysis fails to run or parse
- **THEN** Autopilot MUST exit with a distinct non-zero exit code reserved for "analysis error"

---

### Requirement: 退出码必须稳定表达无人值守判定结果
Autopilot MUST return stable exit codes so schedulers/CI can judge pass/fail without inspecting logs.

#### Scenario: 退出码语义
- **WHEN** Autopilot finishes a run or analysis
- **THEN** it MUST use these exit codes:
  - `0`: hard verdict PASS and agent verdict PASS (or agent analysis explicitly skipped)
  - `1`: hard verdict FAIL
  - `2`: hard verdict PASS but agent verdict FAIL or `quality_score=suboptimal`
  - `3`: agent analysis required but failed (timeout/parse/backend unavailable)

---

### Requirement: 必须提供本地skill以固化运行与复盘流程
The repository MUST include a local skill that documents how to run Autopilot and how to inspect its artifacts, so the workflow is reproducible and maintainable.

#### Scenario: skill文件存在且包含最小操作流
- **WHEN** a developer checks the repository tree
- **THEN** a skill directory under `.codex/skills/` MUST exist for this autopilot workflow
- **AND** its `SKILL.md` MUST include:
  - how to run `ralph autopilot run`
  - how to run `ralph autopilot analyze`
  - which artifact files to inspect on failure (`report.json`, `analysis_input.json`, `analysis_output.json`, stdout/stderr captures)
