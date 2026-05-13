## [2026-04-30 09:25:00] [Session ID: 019ddbe6-a5ae-7950-8ba0-27b3b9f53219] 笔记: 默认 notes.md 续档

## 来源

### 来源1: 本轮 continuous-learning 检索

- 旧 `notes.md` 已超过 1000 行,最新主体内容停留在 2026-03-18 的资源 catalog / preset selector 探索。
- 本轮已经读取并摘要旧 `notes.md` 的关键结论,摘要位置:
  - `notes__continuous_learning.md`
- 旧文件已续档到:
  - `archive/default_history/notes_2026-04-30_0925.md`

## 综合发现

### 续档原则

- 新的默认 `notes.md` 只保留当前之后的默认组笔记。
- 如果要追溯 2026-03-18 之前的默认组探索,先读本轮摘要,再按需要打开 `archive/default_history/notes_2026-04-30_0925.md`。
- 当前活跃调试支线仍然是 `task_plan__serial_tui_issues.md`,不要把它的笔记写回默认 `notes.md`。

## [2026-05-11 23:06:00] [Session ID: omx-1778510695653-7pd7o2] 笔记: Ralph / OMX / Hermes 对比事实源

### 来源1: ralph-orchestrator 本地 README 和源码
- `README.md:11`: Ralph 是 hat-based orchestration framework, 持续循环直到任务完成。
- `README.md:45-56`: 两种模式是 traditional loop 和 hat-based event coordination。
- `README.md:69-79`: 特性包括多 backend、hat system、event-driven coordination、backpressure、TUI、memories/tasks、session recording。
- `README.md:590-625`: hat-based 模式是 pub/sub event system, Ralph 是常驻 coordinator/fallback。
- `crates/ralph-core/src/lib.rs:1-11`: core 提供 orchestration loop、config/state、message routing、terminal capture、workspace isolation。
- `crates/ralph-cli/src/main.rs:1-13`: CLI 包含 init/run/events/plan/code-task/task 等入口。

### 来源2: oh-my-codex 本地 README、package 和源码
- `README.md:20`: OMX 是 OpenAI Codex CLI 的 workflow layer。
- `README.md:28-33`: Codex 仍是执行引擎, OMX 增强启动、工作流、canonical skills 和 `.omx/` 状态。
- `README.md:70-82`: 核心用户面是 `$deep-interview`, `$ralplan`, `$ralph`, `$team`, `$ultragoal`。
- `README.md:169-179`: 明确说 OMX 不替代 Codex, 增加 task routing + workflow + runtime。
- `src/index.ts:1-10`: 包含 30+ role prompts、35+ skills、AGENTS orchestration brain、MCP servers、omx CLI 和 notification hooks。
- `package.json:1-9`: npm 包二进制是 `omx`。
- `Cargo.toml:1-8`: Rust crates 是辅助 runtime/explore/mux/sparkshell, 主项目仍是 TypeScript/npm 分发。
- `docs/adapt.md:19-29`: 当前适配目标包括 `openclaw` 和 `hermes`, Hermes 适配是 probe/status/envelope/init, 且只写 `.omx/adapters/hermes/...`, 不改 Hermes runtime 内部。

### 来源3: Hermes-Agent 公开资料
- GitHub README: Hermes-Agent 是 mobile and computer automation agent, supports Android/iOS/macOS through MCP, with FastAPI web server, CLI, Gradio, memory, skills, scheduler, voice mode, telemetry。
- Hermes docs architecture: orchestration layer 里有 system prompts、planning、memory management、tool calling、response generation。
- Hermes features docs: skills 是 procedural memory, memory system 提供 session persistence 和 tool-aware memory extraction。

### 综合判断
- Ralph 是“多 backend 的任务完成 orchestrator”: 重点是 loop、hats、events、backpressure 和 replay evidence。
- OMX 是“Codex CLI 的工作流/runtime 增强层”: 重点是 Codex 启动、技能/角色/AGENTS、hook、tmux/team、`.omx` 状态和 prompt/governance。
- Hermes-Agent 是“设备/电脑自动化 agent 应用栈”: 重点是 MCP 控制设备、web/CLI/API、多模态、记忆、技能和 automation tasks。
- 三者都使用 memory/skills/orchestration 这些词, 但抽象层级不同。不能把它们当同类替代品。

## [2026-05-12 17:31:00] [Session ID: omx-1778510695653-7pd7o2] 笔记: 重新审视 Ralph 演进优先级

## 来源

### 来源1: 当前工作区状态
- 命令: `git status --short`, `git diff --stat`
- 要点:
  - 当前工作区有大量已修改和新增文件,覆盖 guidance governance、state operation、scoped experience、runtime graph、TUI、docs 等多条线。
  - 这意味着当前风险不是“缺想法”,而是“多条线并行后需要收口和主线排序”。

### 来源2: OpenSpec 状态
- 命令: `openspec list`, `openspec validate --all --strict`
- 要点:
  - `openspec validate --all --strict` 结果为 21 passed, 0 failed。
  - 已完成但仍在 active changes 列表里的方向包括 `agent-guidance-contracts`、`agent-guidance-catalog-cli`、`prompt-contract-runtime-alignment`、`state-operation-layer`、`scoped-experience-system`、`event-id-and-reply`。
  - 未完成的战略方向主要是 `startup-resource-bootstrap` 和 `runtime-capability-invocation`。
  - `tui-mdfried-viewer` 剩余 2 项图片内联任务,但它偏 UX 增强,不是当前 runtime 语义主线。

### 来源3: 代码落点
- 路径:
  - `crates/ralph-core/src/agent_guidance_manifest.rs`
  - `crates/ralph-core/src/state_operations.rs`
  - `crates/ralph-core/src/experience*.rs`
  - `crates/ralph-cli/src/main.rs`
- 要点:
  - guidance manifest verifier 已有 core 实现和 CLI 入口。
  - state operation layer 已有 core 实现,并且 CLI 中已经出现 `ralph state` 命令。
  - scoped experience 已有 parser/store/injection/governance/promotion 等模块,不再只是文档概念。

### 来源4: 仍打开的 runtime task
- 路径: `.agent/tasks.jsonl`
- 要点:
  - 仍有 `Fix: pin docs build dependencies or explicitly document docs gate` 打开。
  - 由于当前也改了 `.github/workflows/docs.yml` / `mkdocs.yml` / docs 首页,docs gate 应被视为收口工作的一部分。

## 综合发现

### 当前最值得做的事
- 第一优先级不是继续扩 runtime 能力,而是把已经完成的 changes 收口、验证、归档,降低当前大工作区的漂移风险。
- 第二优先级是把 adapter contract 做成具体可测边界,尤其是 stdout/stderr、prompt transport、event envelope、termination/flush。
- 第三优先级是先做 `startup-resource-bootstrap` v1,因为 `runtime-capability-invocation` 依赖 catalog/metadata/resolved config 基座。
- `runtime-capability-invocation` 值得做,但不应先做完整 LLM chooser 或 live topology hot swap; v1 应该走隔离 child run / micro-run。

### 当前不值得优先做的事
- 不建议现在搬完整 OMX team/tmux runtime。
- 不建议现在做运行中热切换整套 topology。
- 不建议现在把 TUI 图片内联作为主线,除非用户明确把视觉体验排到最高。
- 不建议继续堆 example/preset,因为现有 governance/runtime 基座尚未完全收口。

### 需要注意的边界
- `state-operation-layer` proposal 写明本 change 不新增 CLI/MCP/runtime adapter,但当前 `main.rs` 已经出现 `ralph state` CLI。后续应检查这是否已经被新的 spec/tasks 覆盖,避免 proposal 和实现边界出现口径漂移。

## [2026-05-12 21:12:00] [Session ID: omx-1778510695653-7pd7o2] 笔记: adapter contract tests 实现落点

## 来源

### 来源1: OpenSpec `adapter-contract-tests`
- `proposal.md` 要求固定 stdout-only event parsing、prompt transport、event envelope、termination flush。
- `tasks.md` 当前 16 项中 4 项完成,剩余从 stream contract tests 开始。

### 来源2: parallel runner
- `CliHatJobExecutor::handle_output_line` 已经把 stdout/stderr 分开缓存。
- 已有测试 `parallel_output_for_event_parsing_is_stdout_only` 能证明 stderr 仍转发给 supervisor,但不进入 stdout_output。
- `finalize_output_for_parsing` 当前只接收 stdout_output,已有 stdout-only 相关单测。

### 来源3: custom backend prompt transport
- `CliBackend::build_command` 在 `PromptMode::Stdin` 下返回 `Some(prompt.to_string())` 且不追加 prompt argv。
- 当前已有 stdin execution 测试,但缺一个直接锁定 custom backend `prompt_mode=stdin` 的 argv/stdio 契约测试。

### 来源4: mock-cli replay
- `mock_cli::replay_terminal_write_records` 当前把所有 selected `TerminalWrite` 都写到单一 writer。
- 这意味着 cassette 中 `stdout=false` 的 stderr 记录会被 mock-cli 回放到 stdout,从而可能被 Ralph 的默认 event parser 当成语义输出消费。
- 这是当前最强的可证伪候选 bug,需要补测试并修成 stdout/stderr 分流。

### 来源5: event/session evidence
- `EventRecord::new` 已保留 `event.id` 和 `event.reply`,但缺显式测试。
- `TerminalWrite::with_instance_id` 已实现,但缺 JSON roundtrip 测试。
- `SessionRecorder` 已有 flush policy 测试,但缺完整 critical sequence strict parse 测试。

## 综合发现
- 主假设: adapter contract 的实际缺口主要在 mock-cli replay 分流和显式 contract tests 缺失。
- 最强备选解释: 现有实现已经基本正确,只需要补测试;如果 mock-cli 分流测试失败,才说明有真实 bug。
- 下一步最小可证伪实验: 先补 replay stdout/stderr separation 单测,它会直接证明 mock-cli 是否把 stderr 错投到 stdout。

## [2026-05-12 22:03:00] [Session ID: omx-1778510695653-7pd7o2] 笔记: startup-resource-bootstrap v1 实施边界

## 来源

### 来源1: OpenSpec `startup-resource-bootstrap`
- 目标是 startup-only 的资源选择,不是 runtime capability invocation。
- v1 要走纯规则 selector,先产出 resolved config artifact,再启动真实 run。
- 缺少 `ralph.yml` 与 `PROMPT.md` 时不应直接失败,而应走 startup resolution。

### 来源2: 当前 `run_command`
- 当前默认 `--config` 是 `ralph.yml`。
- 当 `ralph.yml` 不存在时,当前直接 `RalphConfig::default()`。
- 随后 `loop_runner::resolve_prompt_content()` 会读取默认 `PROMPT.md`;如果也缺失,串行 run 会失败。

### 来源3: 当前 presets
- `crates/ralph-cli/src/presets.rs` 已有 embedded workflow preset 列表,但它只服务 `ralph init` / builtin config。
- `crates/ralph-cli/presets/minimal/*` 已被同步进 crate 目录,适合作为 resource catalog 的 backend/minimal 资源来源。

## 综合发现
- v1 的最小正确切入点是在 `run_command` 加一层 startup resolution,只在默认 `ralph.yml` 缺失且没有 CLI prompt/config source 时运行。
- 选择策略先固定为规则 selector: workflow=`feature-minimal`, prompt_template=`bootstrap/default-task`。
- resolved artifact 写入 `.ralph/resolved-config.yml` 和 `.ralph/bootstrap-selection.json`,作为后续 doctor/replay/debug 的事实源。
- 用户资源目录 v1 使用 `RALPH_HOME/resources` 优先,否则 `$HOME/.ralph/resources`,并实现首次 materialize + 不覆盖已有文件。

## [2026-05-12 22:51:48] [Session ID: omx-1778510695653-7pd7o2] 笔记: startup-resource-bootstrap v1 实现核对

## 来源

### 来源1: `openspec/changes/startup-resource-bootstrap/design.md`

- 要点:
  - startup selector 必须发生在真实 `EventLoop` / `Supervisor` 初始化前。
  - 当前 change 不实现 runtime workflow / hat capability invocation。
  - selector 产物必须是可审计的 `.ralph/resolved-config.yml` 和 `.ralph/bootstrap-selection.json`。

### 来源2: `crates/ralph-cli/src/main.rs`

- 要点:
  - 已加入 `cli_config_was_explicit()` 原始 argv 检测,避免显式 `--config ralph.yml` 被当成默认缺失配置。
  - 已将 `write_bootstrap_artifacts()` 移到 CLI override、validate、backend auto-detect 之后,但仍在 dry-run/真实 loop 初始化之前。

### 来源3: `crates/ralph-cli/src/startup_resources.rs`

- 要点:
  - resource catalog 覆盖 workflow/backend/prompt/example 四类。
  - `example_bundle` 默认 `selector_eligible=false`。
  - 测试通过 `resolve_default_bootstrap_with_root()` 注入临时资源根,避免写入真实用户 `$HOME/.ralph/resources`。

## 综合发现

- 当前 v1 采用三段式资源根目录解析: `RALPH_HOME/resources` -> `$HOME/.ralph/resources` -> `.ralph/resources`。
- OpenSpec design 之前写过 ProjectDirs 类路径,但仓库未引入相关依赖。为保持 v1 小步闭环,文档应明确当前策略,把平台规范目录作为后续可演进项,不要把未实现行为写成已完成。
- 已有动态证据:
  - `cargo test -p ralph-cli --bin ralph startup_resources -- --nocapture`: 6 passed。
  - `cargo test -p ralph-cli --test integration_startup_resources -- --nocapture`: 2 passed。

## [2026-05-12 23:24:48] [Session ID: omx-1778510695653-7pd7o2] 笔记: runtime-capability-invocation v1 实现核对

## 来源

### 来源1: `openspec/changes/runtime-capability-invocation/design.md`

- 要点:
  - workflow capability 必须通过 isolated child run 执行。
  - hat capability v1 也通过 isolated micro-run 执行。
  - 不允许热改当前 live topology。

### 来源2: `crates/ralph-core/src/capability.rs`

- 要点:
  - 新增 `CapabilityMetadata`、`CapabilityChoice`、`CapabilityInvocationRecord`、`CapabilityResultRecord`、`CapabilityFailedRecord`。
  - 控制面 topic 固定为 `capability.invoke`、`capability.result`、`capability.failed`。

### 来源3: `crates/ralph-cli/src/capability.rs`

- 要点:
  - `ralph tools capability list|summaries|invoke` 是 agent-facing surface。
  - workflow preset 从 startup resource catalog 暴露为 lightweight workflow capability。
  - v1 增加 `hat:focused-reviewer` micro-run capability,用于一阶 hat capability 验证。
  - `invoke` 写 `.ralph/capability-invocations/<id>/invoke.json`、`resolved-config.yml`、`result.json` 或 `failed.json`,并追加 `.ralph/events.jsonl` 控制面事件。

## 综合发现

- v1 选择器是规则驱动:
  - 显式 `--id` 优先。
  - 没有显式 id 时,review/audit 输入优先选 hat capability,其他输入优先选 workflow capability。
- v1 隔离执行采用当前 `ralph` 二进制的 `run --dry-run --no-tui` 子执行,用 `custom true` 配置避免真实 backend 消耗。
- 父 topology 稳定性通过测试确认: invoke 后 parent `ralph.yml` 原样保留,只新增 `.ralph/capability-invocations` 与 `.ralph/events.jsonl` 证据。
- 已有验证:
  - `cargo test -p ralph-core capability -- --nocapture`: 1 passed。
  - `cargo test -p ralph-cli --bin ralph capability -- --nocapture`: 3 passed。
  - `cargo test -p ralph-cli --test integration_capability -- --nocapture`: 2 passed。

## [2026-05-13 13:01:39] [Session ID: omx-1778510695653-7pd7o2] 笔记: 提交前变更分组审查

## 来源

### 来源1: `git status --short` / `git diff --name-status`
- 当前工作区覆盖多条线,不是单一 change。
- 主要分组包括:
  - OpenSpec archive/spec sync: 29 个路径。
  - adapter/runtime evidence contracts: 6 个 tracked 文件。
  - startup-resource-bootstrap v1: 新增 `startup_resources.rs` 和 integration test。
  - runtime-capability-invocation v1: 新增 core/cli capability 模块和 integration test。
  - docs/site restructuring、agent guidance governance、runtime graph earlier line、state/experience/guidance earlier line、TUI earlier line、parallel example earlier line。

### 来源2: `git diff --check`
- 没有输出,表示当前 tracked diff 没有 whitespace error。

### 来源3: `git status --ignored -s`
- `site/`、`target/`、`.ralph/`、`.omx/`、`.venv/` 等均处于 ignored 状态。
- 这次 mkdocs build 产生的 `site/` 不会被普通提交带入。

### 来源4: dependency diff
- `Cargo.toml` 新增 `toml` 和 `rerun` workspace dependency。
- `toml` 对应 scoped experience / config-like parsing 线。
- `rerun` 对应 runtime graph earlier line,不是本轮 startup/capability 的直接依赖。

## 综合发现
- 当前不能安全地直接把整个 worktree 当成“本轮四步”提交。
- 如果要提交,建议按主题拆分,至少分为:
  1. adapter contract tests + evidence stream fix。
  2. startup-resource-bootstrap v1。
  3. runtime-capability-invocation v1。
  4. OpenSpec archive/spec sync 批次。
  5. 其他已存在支线: runtime graph、state/experience/guidance、docs site、TUI、parallel example、context logs。
- 若要做精确 commit,需要用 `git add -p` 或按文件/目录 staged,不能直接 `git add .`。
