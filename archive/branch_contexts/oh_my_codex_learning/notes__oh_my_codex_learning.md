
## [2026-05-11 13:24:08] [Session ID: omx-1778475786175-ogndry] 笔记: oh-my-codex 第一轮全局结构画像

## 来源

### 来源1: 仓库入口与包配置

- 路径:
  - `/Users/cuiluming/local_doc/l_dev/my/rust/oh-my-codex/README.md`
  - `/Users/cuiluming/local_doc/l_dev/my/rust/oh-my-codex/package.json`
  - `/Users/cuiluming/local_doc/l_dev/my/rust/oh-my-codex/Cargo.toml`
- 要点:
  - 项目定位是 Codex CLI 的 workflow layer,不是替换 Codex 的执行引擎。
  - 推荐路径是 `omx --madmax --high`,并围绕 `$deep-interview`、`$ralplan`、`$team`、`$ralph` 建立工作流。
  - TypeScript 是主 CLI/runtime 实现,同时包含 Rust workspace: `omx-explore`、`omx-mux`、`omx-runtime-core`、`omx-runtime`、`omx-sparkshell`。
  - package 脚本强调 build、plugin mirror、native agents verify、node tests、coverage gates、packed install smoke。

### 来源2: 目录结构

- 路径:
  - `src/cli/*`
  - `src/hooks/*`
  - `src/team/*`
  - `src/state/*`
  - `src/mcp/*`
  - `src/question/*`
  - `src/ralplan/*`
  - `src/ralph/*`
  - `skills/*/SKILL.md`
  - `prompts/*.md`
  - `templates/AGENTS.md`
  - `templates/catalog-manifest.json`
- 要点:
  - CLI 面很宽: setup、doctor、team、ralph、ultragoal、performance-goal、autoresearch、question、explore、sparkshell、hud、state、mcp-serve、session、agents、adapt 等。
  - skill 是用户可调用 workflow/utility 表面, prompt 是角色/专家执行表面, catalog manifest 是治理这些表面的结构化索引。
  - AGENTS 模板是顶层操作契约,包含 autonomy、delegation、keyword detection、state management、verification、runtime overlay markers。

### 来源3: hook / keyword / triage

- 路径:
  - `src/hooks/keyword-detector.ts`
  - `src/hooks/keyword-registry.ts`
  - `src/hooks/triage-heuristic.ts`
- 要点:
  - keyword registry 是确定性路由面,显式 `$name` 触发 workflow skill。
  - triage heuristic 是 advisory-only,不会直接激活 workflow,只给 explore/executor/designer/researcher/autopilot 这类建议目的地。
  - deep-interview 有 input lock 和 question obligation,避免用户用 `yes/continue` 之类自动批准绕过澄清流程。
  - workflow state 会写入 `.omx/state`,并区分 active skill 和 tracked workflow mode。

### 来源4: state / MCP / runtime persistence

- 路径:
  - `src/state/operations.ts`
  - `src/mcp/state-server.ts`
  - `src/ralph/persistence.ts`
  - `src/ralplan/runtime.ts`
- 要点:
  - 状态读写抽成统一 operation: `state_read`、`state_write`、`state_clear`、`state_list_active`、`state_get_status`。
  - state 写入带 path-level promise queue 和 atomic temp rename,避免并发写破坏 JSON。
  - MCP state server 只是 state operation 的工具化外壳,并且对旧 team MCP mutation 明确 hard-deprecated,提示走 CLI interop。
  - Ralph persistence 会迁移旧 `.omx/prd.json` / `.omx/progress.txt` 到 canonical `.omx/plans` 与 `ralph-progress.json`,强调状态可恢复和迁移路径。
  - Ralplan runtime 明确 draft -> architect-review -> critic-review -> complete/failed 的状态机,并把 planning artifacts 作为 completion gate。

### 来源5: team / tmux / worktree

- 路径:
  - `src/team/runtime.ts`
  - `src/team/orchestrator.ts`
  - `src/team/worktree.ts`
- 要点:
  - team runtime 是高开销协调面,不是普通并行 map。
  - team pipeline 明确: `team-plan -> team-prd -> team-exec -> team-verify -> team-fix`,终态是 `complete`、`failed`、`cancelled`。
  - worker 通过 tmux pane、inbox、mailbox、heartbeat、task claim、dispatch request 等文件状态协同。
  - worktree 模块要求 worker worktree 前 leader workspace 干净,否则直接报错,避免把用户未提交改动复制成多路冲突。

### 来源6: question / deep-interview

- 路径:
  - `src/question/deep-interview.ts`
  - `src/question/renderer.ts`
- 要点:
  - deep-interview 把
## [2026-05-11 13:24:32] [Session ID: omx-1778475786175-ogndry] 笔记补全: 第一轮全局结构画像续写

### 补充来源6: question / deep-interview

- 路径:
  - `src/question/deep-interview.ts`
  - `src/question/renderer.ts`
- 要点:
  - deep-interview 把

## [2026-05-11 13:24:32] [Session ID: omx-1778475786175-ogndry] 笔记补全: 第一轮全局结构画像完整补充

### 补充来源6: question / deep-interview

- 路径:
  - `src/question/deep-interview.ts`
  - `src/question/renderer.ts`
- 要点:
  - deep-interview 把“必须问用户”的动作状态化为 question obligation。
  - 如果 obligation pending,状态会写成 `lifecycle_outcome: askuserQuestion` 和 `run_outcome: blocked_on_user`。
  - renderer 会根据 tmux、显式 pane、Windows psmux、inline TTY 等条件选择问题渲染策略。
  - 这说明 OMX 不是只靠 prompt 说“问用户”,而是把问答动作变成可追踪的 runtime contract。

### 当前全局判断

- oh-my-codex 的核心价值不在某一个命令,而在几个层次叠加:
  1. 顶层 AGENTS 操作契约。
  2. hook/keyword 的确定性激活面。
  3. advisory triage 的低风险建议面。
  4. skills 和 prompts 的结构化能力目录。
  5. `.omx/state` 的持久状态面。
  6. tmux/team/worktree 的并行执行面。
  7. package scripts、coverage、plugin mirror、native asset verify 的验证面。
- 可借鉴价值要按“能否脱离 OMX 运行时复用”来分层:
  - 可直接照搬: 文档契约模板、catalog manifest 思路、state JSON contract、hook-owned marker 区块、验证脚本命名习惯。
  - 需要改造后借鉴: team/tmux runtime、question renderer、plugin/setup 双模式、native agent mirror。
  - 只适合参考理念: 全量 workflow surface、复杂 runtime 自动路由、跨平台 team runtime。


## [2026-05-11 13:55:03] [Session ID: omx-1778475786175-ogndry] 笔记: 正式报告完成后的综合结论

## 来源

### 来源1: `specs/oh-my-codex-learning-analysis.md`

- 报告已按用户要求分成两步:
  - 第一部分: 综合全面分析目标仓库定位、架构、运行机制、文档治理、状态和验证链路。
  - 第二部分: 逐个价值点深度挖掘,说明可照搬程度、迁移建议和风险。
- 报告核心结论:
  - `oh-my-codex` 最值得学的是“契约化治理方式”,不是命令数量。
  - 高价值内容集中在 AGENTS 顶层契约、guidance schema、prompt contract、state operation、question obligation、team phase/worktree gate、plugin mirror SSOT 和验证链路。
  - 不建议直接照搬完整 team/tmux runtime、plugin/setup 双轨、native hook 全矩阵和全量 workflow keyword。

## 综合发现

### 可直接进入后续 Ralph 设计讨论的点

- 建议优先从三件低风险事情开始:
  - `docs/agent-guidance-schema.md`
  - `docs/prompt-contract.md`
  - prompt/skill/hat manifest + verifier
- 状态层建议先抽内部 operation,再考虑 MCP 外壳。
- 并行 runtime 建议先学 phase table 和 clean worktree gate,不要先学 tmux runtime。

### 验证

- 已抽取报告中的 Mermaid 图到 `/tmp/oh_my_codex_learning_mermaid/diagram-1.mmd`。
- 已运行 `beautiful-mermaid-rs --ascii < /tmp/oh_my_codex_learning_mermaid/diagram-1.mmd`,命令退出码为 0,说明图表语法可渲染。
