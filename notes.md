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

## [2026-05-14 13:53:00] [Session ID: codex-20260514-archive-learning] 笔记: task_plan 续档触发的 continuous-learning 六文件摘要

## 来源

### 来源1: 默认六文件当前版本

- `task_plan.md`: 当前任务是 archive `request-reply-answer-evidence`, 做续档 continuous-learning, 再选择下一条演进线。
- `WORKLOG.md`: 最新有效交付包括 Phase 1A evidence index kernel、Phase 2 answer-return evidence runtime wiring。
- `LATER_PLANS.md`: 明确记录了 `task_plan.md` 超过 1000 行后需要在安全点做 continuous-learning。
- `ERRORFIX.md`: 近期反复出现的可复用错误是 quoted heredoc、OpenSpec delta section、cargo test filter 位置、completion audit JSON contract。
- `EPIPHANY_LOG.md`: 已有长期风险包括 `reply` 关联语义不等于答案回给请求方、bootstrap selector 不能热改 live topology、YAML 注释不是 runtime metadata contract。

### 来源2: 默认历史版本

- `task_plan_2026-05-14_phase1a_phase2_prev.md`:
  - 覆盖 Phase 1A OpenSpec -> 实现 -> audit -> archive -> Phase 2 OpenSpec 草案。
  - 关键边界是 staged diff 启动前必须为空,Phase 1A 不做 evidence CLI/doctor,Phase 2 不做 request broker 或 topology 热改。
  - 该文件是本次续档触发对象,已被本次阅读分析覆盖,后续应移入 `archive/default_history/`。
- `WORKLOG_2026-05-13_1937_prev.md`:
  - 覆盖 3 月到 5 月多条 runtime graph、hat request/reply、OpenSpec、startup/capability/evidence 演进线。
  - 文件已超过 1000 行且此前只是日期化保存,本次已被标题级扫读覆盖,应移入 `archive/default_history/`。

### 来源3: 旧支线上下文组

- `continuous_learning`:
  - 已完成 2026-04-30 的分组、归档、长期知识写入。
  - 后续仍留有迁移 archive 根层旧平铺文件的备忘。
- `serial_tui_issues`:
  - 已完成非 parallel Codex rollout 报错与 serial TUI 输出选择问题。
- `rerun_runtime_graph_v2`:
  - 已完成 V2 durable replay graph 代码、测试、OpenSpec archive 的多轮验证记录。
- `oh_my_codex_learning`:
  - 已完成 `specs/oh-my-codex-learning-analysis.md` 报告与指导治理后续建议。
- `guidance_contract_governance`:
  - 已完成 guidance schema、prompt contract、manifest、state operation、state CLI adapter 等多阶段治理线。
- `experience_promotion_workaround`:
  - 已完成 workaround fixture 文案审计修复。

## 综合发现

### 默认组

- Phase 1A 的长期经验已经写入 `EXPERIENCE.md` 的 `exp-20260513-runtime-evidence-index-kernel-boundary`。
- Phase 2 新增长期经验还没有沉淀: answer-return evidence 的正确边界是显式 `reply.hat.message` requester-return 分支,不是 CLI UX、request broker 或 human reply 自动合成。
- OpenSpec archive 后要检查稳定 spec 的 `Purpose TBD`,这一点在 Phase 1A 和 Phase 2 都重复出现,适合沉淀成后续 archive 习惯。

### 支线组活跃度判定

- 当前默认六文件仍活跃,保留在根目录。
- `task_plan_2026-05-14_phase1a_phase2_prev.md` 和 `WORKLOG_2026-05-13_1937_prev.md` 是默认历史版本,应归档到 `archive/default_history/`。
- 本次列出的 `__continuous_learning`, `__serial_tui_issues`, `__rerun_runtime_graph_v2`, `__oh_my_codex_learning`, `__guidance_contract_governance`, `__experience_promotion_workaround` 均不是当天活跃支线,且有完成记录或非当前任务状态,应按主题归档到 `archive/branch_contexts/<topic>/`。

## 可复用点候选

1. Phase 2 answer-return evidence 的 producer 字段必须保持写入者身份,失败原因留在原始 JSONL artifact。
2. `reply.hat.message` 是唯一显式 requester-return answer channel;普通 workflow event 里有 `reply` 属性不等于 answer-return。
3. OpenSpec archive 自动生成稳定 spec 后,必须检查并修正 `Purpose TBD`。
4. continuous-learning 触发于续档时,要先分组摘要再移动文件,不能只搬文件。

## 最适合沉淀的位置

- `EXPERIENCE.md`: 新增 Phase 2 answer-return evidence 边界经验。
- `AGENTS.md`: Project Knowledge Index 已指向 `EXPERIENCE.md` 与 `archive/manifests/`,无需新增长期文件索引;但需要在 manifest 里记录本轮归档批次。
- `archive/manifests/`: 新增本轮 archive manifest。
- 不提取新 skill: 本轮经验是 Ralph repo 的项目级演进口径,已有 `EXPERIENCE.md` 更合适。

## [2026-05-14 15:10:00] [Session ID: omx-1778510695653-7pd7o2] 笔记: Phase 3 capability invocation / child run evidence 现状

## 来源

### 来源1: `openspec/specs/capability-invocation/spec.md`
- 要点:
  - 现有稳定 spec 已要求 workflow capability 使用 isolated child execution。
  - hat capability 使用 isolated transient execution / micro-run。
  - 已要求记录 auditable invocation artifacts,但没有明确要求写入 evidence index。
  - `Purpose` 仍是 archive 自动生成的 TBD,Phase 3 可顺手修正。

### 来源2: `crates/ralph-cli/src/capability.rs`
- 要点:
  - `ralph tools capability invoke` 当前入口是 `invoke_capability()` -> `invoke_isolated()` -> `invoke_isolated_with_runner()`。
  - 现有 artifact:
    - `.ralph/capability-invocations/<id>/resolved-config.yml`
    - `.ralph/capability-invocations/<id>/invoke.json`
    - `.ralph/capability-invocations/<id>/result.json` 或 `failed.json`
  - 现有 event:
    - `.ralph/events.jsonl` 中的 `capability.invoke`
    - `.ralph/events.jsonl` 中的 `capability.result` 或 `capability.failed`
  - 缺口:
    - 没有写 `.ralph/evidence-index.jsonl`。
    - integration test 只断言 artifact 和 event,不查 evidence index。

### 来源3: `crates/ralph-core/src/evidence_index.rs`
- 要点:
  - 已有 artifact kind:
    - `CapabilityInvokeJson`
    - `CapabilityResultJson`
    - `CapabilityFailedJson`
    - `ResolvedConfig`
    - `EventLogJsonl`
  - writer 默认路径为 `.ralph/evidence-index.jsonl`。
  - 适合直接复用,不需要新增 evidence index kernel。

## 综合发现

### Phase 3 最小契约
- 主假设:
  - 在现有 isolated invocation artifact 写入点旁边补 evidence index registration,即可完成 Phase 3 最小真实串联。
- 备选解释:
  - 如果用户期待真正运行非 dry-run child run,那会扩大到 execution semantics;但当前项目既有 OpenSpec 和测试明确 v1 是 isolated child/micro-run artifact,且 runner 抽象已用于 deterministic 测试。
- 推翻主假设的证据:
  - 如果现有 spec 或测试要求 `capability invoke` 必须完成真实 LLM/backend run,而不是 child dry-run / micro-run artifact,则需要先扩 spec。
- 当前结论:
  - 先做 evidence wiring,不热改 topology,不新增 broker。

## [2026-05-14 16:54:00] [Session ID: omx-1778510695653-7pd7o2] 笔记: Phase 3.1 evidence UX 入口探查

## 来源

### 来源1: `crates/ralph-cli/src/main.rs`
- 当前顶层 CLI 没有 `ralph evidence` 子命令。
- 顶层已有 `Tools(tools::ToolsArgs)`,并分发到 `tools::execute(...)`。

### 来源2: `crates/ralph-cli/src/tools.rs`
- `ralph tools` 当前包含 `memory`、`task`、`capability`。
- `capability` 已经是 runtime capability 的稳定工具入口。

### 来源3: `crates/ralph-cli/src/capability.rs`
- `CapabilityCommands` 当前有 `list`、`summaries`、`invoke`。
- `invoke` 已写 `.ralph/capability-invocations/<id>/...` 和 `.ralph/evidence-index.jsonl`。
- 可以在同一模块内增加 `inspect`,复用 `EvidenceIndexReader::find_by_correlation`。

### 来源4: `crates/ralph-core/src/evidence_index.rs`
- Reader API 是 `read_all()` 和 `find_by_correlation(...)`。
- `EvidenceLookup` 已区分 `Entries`、`Missing`、`NoEntry`。

## 综合发现

### 推荐 Phase 3.1 最小 UX
- 做 `ralph tools capability inspect <invocation_id>`。
- 默认从当前工作目录读取 `.ralph/evidence-index.jsonl`。
- `--json` 输出机器可读对象,包含 invocation id、lookup status、entries、artifact paths。
- human 输出展示 status、artifact kind、artifact path、producer/status/reason。
- 找不到 id 时应返回非零,不要伪造空成功。

### 暂不做的方向
- 暂不新增泛化 `ralph evidence lookup` 顶层命令。
- 原因: Phase 1A 已明确不要把 kernel 提前扩成完整 evidence UX/doctor;当前最小产品价值来自 capability invocation 调试闭环。

### Phase 4 依赖关系
- Phase 4 live runtime capability invocation 应依赖 Phase 3.1 inspect UX。
- 否则 live 调用失败时只有 artifacts,没有稳定人为/agent 查询入口。

## [2026-05-15 12:14:00] [Session ID: omx-1778510695653-7pd7o2] 笔记: Phase 4.1 注入点探查

## 来源

### 来源1: `crates/ralph-cli/src/capability.rs`
- 已有 `capability_catalog() -> Vec<CapabilityMetadata>`。
- catalog 来源是 structured startup resource metadata + 内嵌 `hat:focused-reviewer`。
- 注释明确不读取 YAML 注释,也不把完整 instructions 注入摘要。

### 来源2: `crates/ralph-core/src/parallel/supervisor.rs`
- `build_ralph_coordinator_instructions()` 是 `ralph#1` / `ralph#2` coordinator 指令来源。
- 这里已有 `event_loop.ralph_prompt` 的 Ralph-only 注入,说明它是合适的 coordinator-only context 注入点。

### 来源3: `crates/ralph-core/src/parallel/instance.rs`
- `prompt_prelude` 只给 `hat.id == "ralph"` 注入。
- 但直接把 catalog 拼到 `prompt_prelude` 会把用户 objective 和 capability catalog 混在一起,不如作为 coordinator instructions 的独立 section 更清晰。

## 综合发现

### 推荐实现
- 在 core `ParallelSupervisor` 增加 `runtime_capability_catalog: Vec<CapabilityMetadata>` 字段和 builder 方法。
- 在 core `capability.rs` 增加 parent-visible catalog renderer,负责稳定 marker、`capability.request` contract、bounded metadata。
- 在 `build_ralph_coordinator_instructions()` 中注入 renderer 输出,只给 `ralph#1` / `ralph#2`。
- 在 CLI `parallel_runner` 用现有 `crate::capability::capability_catalog()` 注入 supervisor。

### 不采用的方案
- 不把 catalog 拼进 `prompt_prelude`,避免把 capability selection surface 与 top-level objective 混在一起。
- 不让 core 直接调用 CLI catalog builder,避免反向依赖。

## [2026-05-16 12:24:00] [Session ID: omx-1778510695653-7pd7o2] 笔记: ralph-example 事件发送格式内置化调查

## 来源

### 来源1: `/Users/cuiluming/local_doc/l_dev/my/rust/ralph-example/ralph.yml`
- `ralph_prompt`、`experiment_runner`、`experiment_auditor`、`experiment_integrator` 都重复写了 `发事件必须使用如下格式`。
- 重复格式是 `<event topic="..."><payload></event>`。
- 文件中也保留了 workflow-specific 规则,例如 `experiment.result` 必须带 `verification_evidence` 和 `commit`,这些仍应保留在 workflow 配置中。

### 来源2: `crates/ralph-core/src/event_parser.rs`
- 当前 parser 仍以 XML-style `<event ...>...</event>` 作为 in-band event envelope。
- 支持属性包括 `id`、`reply`、`topic`、`target`、`target_instance`、`audience_instances`、`require_delivery`、`workspace_strategy`、`session_strategy`、`turn_action`、`spawn_instance`。
- 支持同一输出里多个 event block,支持多行 opening tag,也支持 `<\/event>` 形式的 escaped close tag。
- `LOOP_COMPLETE` 必须在 event 外独占一行才算 completion promise;event payload 里的 `LOOP_COMPLETE` 不应触发完成。

### 来源3: `crates/ralph-core/src/parallel/instance.rs`
- 并行 job 完成时使用 `EventParser::new().parse(&result.output_for_parsing)`。
- `output_for_parsing` 是 stdout-only 链路,避免 stderr / tool transcript 造成假事件。
- `build_prompt` 会把 Incoming Events 渲染成纯文本 `id=... topic=... payload=...`,避免把输入事件原样渲染成 `<event>` 再被模型复述。

### 来源4: `config/all_hat.md` 与 `crates/ralph-core/src/prompt_overlay.rs`
- `config/all_hat.md` 已经作为编译期内嵌 overlay 注入所有 hat prompt。
- 这里已经包含“正常 workflow event 发射”的通用规则。
- `prompt_overlay` 会把 overlay 中 raw `<event>` 示例转义为 `&lt;event`,避免共享 overlay 里的示例被模型复制成真实事件。

## 综合发现

### 当前格式是否正确
- 是,`ralph-example/ralph.yml` 中 `<event topic="..."><payload></event>` 仍然是正确 event envelope。
- 但它是不完整的最新协议说明,因为当前 runtime 还支持更多属性和 stdout-only / completion promise 等边界。

### 是否应该继续写在执行目录 `ralph.yml`
- 不应该把通用 envelope 教程继续散落在执行目录配置里。
- 这些属于 runtime prompt contract,应该由 Ralph 内置 renderer / compiled prompt overlay 注入。
- 执行目录 `ralph.yml` 应只写 workflow-specific 内容: topic、payload 字段、backpressure、收敛规则。

### 推荐方案
- 已创建 OpenSpec change `internalize-event-emission-protocol`。
- 先实现 built-in event emission protocol renderer,再注入 publishing hat prompts 和 `ralph#1` coordinator prompt。
- 通过测试后,再瘦身 `ralph-example/ralph.yml`,删除重复的 generic event-format blocks,保留业务 payload schema。

### 不建议方案
- 不建议直接手动删 `ralph-example/ralph.yml` 里的格式说明。
- 如果内置 prompt contract 还没有测试锁住,直接删会让 example 依赖隐式知识,风险更高。

## [2026-05-16 13:34:00] [Session ID: omx-1778510695653-7pd7o2] 笔记: 内置事件发送协议实现证据

## 来源

### 来源1: `crates/ralph-core/src/event_emission_protocol.rs`
- 新增 `EVENT_EMISSION_PROTOCOL_HEADING` 稳定 marker。
- `render_event_emission_protocol(...)` 负责渲染 canonical envelope、stdout-only 规则、禁止 shell/file/stderr/tool transcript 作为 normal workflow event channel、`LOOP_COMPLETE` 边界、supported attributes 与当前角色可发布 topic 列表。

### 来源2: `crates/ralph-core/src/parallel/instance.rs`
- `build_prompt(...)` 现在基于 `hat.publishes` 渲染内置事件协议,再追加 Incoming Events。
- publishing hat 的 workflow-specific payload 字段仍来自 hat instructions,不是 core renderer 推断。

### 来源3: `crates/ralph-core/src/parallel/supervisor.rs`
- `build_ralph_coordinator_instructions(...)` 改为复用同一个 renderer。
- coordinator 额外保留 `## OUT-OF-BAND EVENT INJECTION`,只说明 `ralph emit` 是可执行命令通道,不再复制旧的 divergent in-band envelope 教程。

### 来源4: example dogfood
- repo 内 `examples/parallel-experimental-dev-engine/ralph.yml` 已移除 generic `发事件格式` / `<event topic=...>` 教程块。
- 外部 `/Users/cuiluming/local_doc/l_dev/my/rust/ralph-example/ralph.yml` 同步瘦身。
- 两者仍保留 `experiment.result` / `integration.applied` / `experiment.complete` 的 payload 字段要求,例如 `verification_evidence` 与 `commit`。

## 已跑的 focused 验证
- `cargo test -p ralph-core event_emission_protocol`: 2 passed。
- `cargo test -p ralph-core ralph_coordinator_event_protocol`: 1 passed。
- `cargo test -p ralph-cli --test integration_examples test_example_parallel_experimental_dev_engine_uses_builtin_event_protocol`: 1 passed。

## [2026-05-16 14:48:00] [Session ID: omx-1778510695653-7pd7o2] 笔记: startup bootstrap + live run dogfood 证据

## 来源

### 来源1: `/tmp/ralph-live-dogfood-73rELo/workspace/.ralph/bootstrap-selection.json`
- 无 `ralph.yml` 且无 `PROMPT.md` 的空工作区启动后,bootstrap selector 选择了:
  - `workflow:feature-minimal`
  - `prompt:bootstrap-default-task`
- `startup_only` 为 `true`,说明这是 startup bootstrap 阶段产物,不是执行目录静态配置文件。

### 来源2: `/tmp/ralph-live-dogfood-73rELo/workspace/.ralph/resolved-config.yml`
- resolved config 内联 prompt 含 `Act as Ralph's startup bootstrap coordinator`。
- `parallel.enabled: true` 已落盘到 startup 产物,说明“默认并行模式”不是口头约定,而是可执行配置事实。

### 来源3: `/tmp/ralph-live-dogfood-73rELo/workspace/.ralph/dogfood/ralph#1.prompt.txt`
- live run 抓到的 `ralph#1` prompt 同时包含:
  - `Act as Ralph's startup bootstrap coordinator`
  - `## RALPH EVENT EMISSION PROTOCOL`
  - `reply.human.message`
- 这说明 startup bootstrap 产出的 coordinator prompt 已真实接上内置 event emission protocol,不是只在单元测试里成立。

### 来源4: `target/debug/ralph record summary /tmp/ralph-live-dogfood-73rELo/live-session.jsonl`
- `ux_mode: parallel-cli`
- `Termination: CompletionPromise`
- `current_exe: /Users/cuiluming/local_doc/l_dev/my/rust/ralph-orchestrator/target/debug/ralph`
- stdout tail 为:
  - `Startup bootstrap summary from live dogfood.`
  - `LOOP_COMPLETE`

## 综合发现

### 已验证事实
- 运行目录中没有 `ralph.yml`、也没有 `PROMPT.md` 时,`ralph run` 现在可以走 startup bootstrap 闭环。
- 这条闭环默认启用并行模式,并且由 startup 产出的 resolved config 承载,不是要求用户在执行目录手写默认配置。
- 内置 `## RALPH EVENT EMISSION PROTOCOL` 已真实进入 live `ralph#1` prompt,说明“事件协议内置化”不只是静态实现,已经接到了默认启动链路上。

### 边界确认
- 这次 live dogfood 只替换了 resolved config 的 backend 执行器,没有热改 workflow、hat topology、prompt contract 或业务 payload schema。
- 因此这条证据能证明的,是 startup bootstrap + 内置 prompt contract 的真实串联,而不是某个临时 example 特判。

## [2026-05-16 15:08:00] [Session ID: omx-1778510695653-7pd7o2] 笔记: bootstrap live gate 实现落点选择

## 综合发现

### 推荐落点
- 新 gate 最适合落在 `crates/ralph-cli/tests/integration_startup_resources.rs`。
- 原因不是它最省事,而是语义最对:
  - 这条线的主真相源是 startup bootstrap
  - live prompt capture 只是用来证明 bootstrap 产出的真实 coordinator prompt 已接上内置事件协议
- 因此不应把它塞进 capability integration,避免测试语义再次漂移。

### 推荐实现形态
- 复用 `integration_live_capability` 的 custom backend 技法,但 backend 行为更窄:
  - `ralph#1` 首轮把 stdin prompt 落盘
  - 断言 prompt 中存在 startup bootstrap coordinator marker 与 event protocol marker
  - 然后输出 `LOOP_COMPLETE`
- 普通实例直接 `LOOP_COMPLETE`,避免演变成多 hat workflow。

### 推荐断言集合
- `.ralph/bootstrap-selection.json`
- `.ralph/resolved-config.yml`
- `.ralph/dogfood/ralph#1.prompt.txt`
- `record-session` summary 或原始 JSONL 中的 `CompletionPromise`

## [2026-05-16 17:18:00] [Session ID: omx-1778510695653-7pd7o2] 笔记: answer evidence inspect UX 实现策略

## 综合发现

### 命令落点
- 采用 `ralph tools answer inspect <correlation_id>`。
- 理由: answer-return evidence 已经有稳定 runtime 语义,现在缺的是最小 lookup surface,而不是新 evidence 子系统。

### 实现策略
- 新增 `crates/ralph-cli/src/answer.rs`,直接复用 `EvidenceIndexReader::find_by_correlation(...)`。
- `Entries` 与 `Missing` 都视为成功查询结果。
- `NoEntry` 视为命令失败。
- JSON 作为稳定 automation contract,人类输出只保留简短摘要。

### 测试策略
- 扩 `integration_answer_evidence.rs`,直接在现有 live dogfood 后调用 `ralph tools answer inspect`。
- 这样能证明 inspect UX 和真实 answer-return runtime evidence 用的是同一条 durable artifact 链。

## [2026-05-16 18:12:00] [Session ID: omx-1778510695653-7pd7o2] 笔记: 方向B.1 的现状盘点与最小边界

## 来源

### 来源1: `EXPERIENCE.md`
- `exp-20260514-request-reply-answer-evidence-boundary`
- 要点:
  - `reply.hat.message` 是显式 requester-return answer channel。
  - 内部 answer return 不能自动合成 `reply.human.message`。
  - human-visible answer 必须保持显式 workflow / event 决策。

### 来源2: `openspec/specs/request-reply-answer-evidence/spec.md`
- 要点:
  - 稳定 spec 已经锁定 `reply.hat.message` 与 `reply.human.message` 的职责边界。
  - 现有稳定 spec 覆盖 answer-return evidence 和 answer inspect UX,但还没有一个“human-facing answer return dogfood”条目。

### 来源3: `crates/ralph-core/src/parallel/supervisor/routing.rs`
- 要点:
  - `reply.human.message` 到达 supervisor 后只走 observer,不会再参与 hat 路由。
  - `reply.hat.message` 则会按 `reply=<request_event_id>` 找回 requester instance 并定向投递。

### 来源4: `crates/ralph-core/src/parallel/supervisor.rs`
- 要点:
  - live prompt 已明确要求: 观察到外部 `human.message` 时,必须显式发 `reply.human.message`。
  - 同一段协议也明确要求: hat-to-hat answer return 必须走 `reply.hat.message`。

### 来源5: 现有测试与场景
- `crates/ralph-cli/tests/integration_answer_evidence.rs`
  - 已证明内部 requester-return evidence 可闭环。
  - 但没有覆盖最终 human-visible reply。
- `crates/ralph-core/src/parallel/supervisor/routing_tests.rs`
  - 已有 guardrail test 证明 `reply.hat.message` 不自动 synthesize `reply.human.message`。
- `crates/ralph-e2e/src/scenarios/parallel/app_server_steer_live_reply_multi_turn.rs`
  - 已存在 live “可见 answer” 场景,但它是 E2E 级、依赖真实 codex app-server,不适合作为 repo-native 最小 gate。

## 综合发现

### 现象
- 仓库已经有 internal answer return 的 durable evidence contract。
- 仓库也已经有 human-facing reply 的 prompt/display/record contract。
- 但缺一条 repo-native、无外部模型依赖的最小动态证据,把这两层 contract 放进同一条 runtime run 里证明。

### 候选假设
- 当前方向B.1最值得做的不是新增 routing 功能。
- 更像是新增一个 focused dogfood gate + 配套 OpenSpec 条目:
  - 外部 `human.message` -> `ralph#1`
  - `ralph#1` 发内部 request 给 worker
  - worker 用 `reply.hat.message` 回答给 `ralph#1`
  - `ralph#1` 再显式发 `reply.human.message`
  - 断言 stdout / record-session / events.jsonl 都有可验证证据

### 为什么这条边界最稳
- 不会改 live topology。
- 不会引入 request broker。
- 不会破坏“human-visible answer 必须显式发布”的既有产品契约。
- 可以直接复用前一条 `integration_answer_evidence` 的 test harness 风格。

### 当前尚缺的证据
- 还没看到现有 CLI integration 已经覆盖这条完整闭环。
- 还需要创建新 change,把 dogfood 的断言点和非目标写清楚。

## [2026-05-16 18:52:00] [Session ID: omx-1778510695653-7pd7o2] 笔记: capability.result 到 explicit human reply 的最小闭环

## 来源

### 来源1: `crates/ralph-cli/tests/integration_live_capability.rs`
- 现有 gate 已证明:
  - parent `ralph#1` 可发 `capability.request`
  - runtime 会执行 isolated invocation
  - parent event log 会收到 `capability.result`
  - inspect UX 能按 invocation id 查 artifact 链
- 现有缺口:
  - 还没有证明 parent 如何把 `capability.result` 继续转成 human-visible answer。

### 来源2: `openspec/specs/capability-invocation/spec.md`
- 稳定 spec 已经覆盖 capability request / result / inspect / parent topology unchanged。
- 但之前没有 requirement 明确: `capability.result` 本身不是 human-facing answer,真正面向人的输出仍需显式 `reply.human.message`。

### 来源3: 已完成的 B.1 产物
- `reply.human.message` 的显式边界已经被 repo-native gate 固定。
- 因此本阶段最稳的推进不是再发明新通道,而是把 capability result 接到这条显式人类回复 contract 上。

## 综合发现

### 现象
- 现有 live capability runtime 已经能把 child/micro-run result 回到 parent run。
- 现有 human-facing answer contract 已经能在 repo-native gate 里单独成立。
- 缺的是把两条链路放进同一条 run 的动态证据。

### 结论
- 最小可行推进是新增 focused live capability integration gate:
  - turn1: `capability.request`
  - turn2: 观察到 `capability.result`
  - same parent run 显式发 `reply.human.message`
- 实际实现时不需要新增 runtime 功能; 只是在现有测试 harness 上补一条更完整的产品链 gate。
## [2026-05-16 20:05:00] [Session ID: omx-1778510695653-7pd7o2] 笔记: multi-step orchestration over capability results 的最小产品闭环

## 来源

### 来源1: `crates/ralph-cli/tests/integration_live_capability.rs`
- 新增 `write_multi_step_backend_script(...)`。
- turn1 由 parent `ralph#1` 发 `capability.request(step1)`。
- turn2 只有在 prompt 已看到 `capability.result(step1)` 后,才继续发 `capability.request(step2)`。
- turn3 只有在 prompt 已看到 `capability.result(step2)` 后,才显式发 `reply.human.message` 并 `LOOP_COMPLETE`。

### 来源2: `openspec/specs/capability-invocation/spec.md`
- 稳定 spec 这次新增了两条 multi-step requirement。
- 第一条要求 parent run 可以跨多个 turn 发多个 `capability.request`,并基于前一步 `capability.result` 决定后一步。
- 第二条要求最终 human-facing answer 仍必须显式 `reply.human.message`,中间 capability results 不能被误当成人类回复。

### 来源3: archived change `openspec/changes/archive/2026-05-16-multi-step-capability-result-orchestration/`
- archived change 已完整包含 proposal / design / delta spec / tasks / test-plan。
- 说明这次不是临时补测试,而是已经完成 spec -> gate -> archive 的闭环。

## 综合发现

### 现象
- 当前 runtime 已经自然支持 parent run 在多个 turn 内连续发多个不同 `request_id` 的 `capability.request`。
- 每一步 capability invocation 都能回到 parent-visible `capability.result`,并保留各自独立 `invocation_id`。
- 最终 human-facing answer 仍然只在最后一步通过显式 `reply.human.message` 暴露给用户。

### 边界判断
- 这不是 broker、planner engine 或 topology mutation。
- 真正的 orchestrator 仍然是 parent `ralph#1` 自己。
- capability execution path 仍然是 isolated child / micro-run,没有改变 parent topology。

### 结论
- 方向B已经从“单步 capability result -> human reply”演进到“多步 capability results -> final human reply”。
- 这条链路现在有 repo-native focused gate,也有 OpenSpec requirement,后续可以在这个基础上继续做更丰富的 parent policy 或 failure branching。
## [2026-05-16 20:24:00] [Session ID: omx-1778510695653-7pd7o2] 笔记: capability.failed 后的 parent fallback orchestration 最小边界

## 来源

### 来源1: `crates/ralph-cli/src/capability.rs`
- `invoke_parent_request(...)` 有三条返回分支:
  - child success -> `capability.result`
  - child run 自身失败 -> `parent_invocation_failed_event(...)`
  - capability id 无效等直接失败 -> `parent_failed_event(...)`
- 如果是直接失败,返回的 `CapabilityParentFailedRecord` 会包含:
  - `status = failed`
  - `request_id`
  - `capability_id`
  - `invocation_id = None`
  - `artifacts = None`
  - `parent_topology_unchanged = true`

### 来源2: `crates/ralph-core/src/parallel/supervisor/capability_runtime.rs`
- `ParallelSupervisor` 只认 `ralph#1` 发出的 `capability.request`。
- `capability.failed` 会像 `capability.result` 一样被回送到 parent run,并继续进入 parent event log / prompt 上下文。
- 这说明 B.2 的关键问题不是 runtime 是否能回 failure,而是 parent 是否能基于 failure 继续编排。

### 来源3: `openspec/specs/capability-invocation/spec.md`
- 稳定 spec 已经要求 failed parent-triggered invocation 返回结构化 failure event。
- 但还没有 requirement 明确: parent 在看到 `capability.failed` 后可以继续发 fallback capability request,并在后续显式对人回答。

## 综合发现

### 现象
- `capability.failed` 的结构化 payload 已存在。
- 失败事件回 parent run 的 runtime contract 已存在。
- 现在缺的不是 failure transport,而是 failure 后 parent-side orchestration 的 repo-native 产品证明。

### 当前主假设
- 最小产品闭环可以用“无效 capability id -> parent-visible `capability.failed` -> fallback 有效 capability -> explicit `reply.human.message`”来证明。
- 这样不需要新增 runtime 机制,也能验证 failure context 确实进入了 parent 后续 turn。

### 最强备选解释
- 也可能 prompt 中虽然有 `capability.failed`,但信息不够稳定,不足以让 parent 做 deterministic fallback。
- 如果后续 focused gate 失败,优先检查 parent continuation prompt 是否确实包含 `capability.failed`、`request_id`、`capability_id`,而不是先改 runtime。

### 结论
- B.2 最适合继续走“窄 OpenSpec + focused integration gate”路线。
- 当前先不扩成 retry engine、planner policy 或 failure broker。

## [2026-05-16 23:14:00] [Session ID: omx-1778510695653-7pd7o2] 笔记: task_plan 超过 1000 行后的轻量持续学习续档

## 来源

### 来源1: 默认六文件当前状态

- `task_plan.md`: 1120 行,最新主线是 B.3 `failure_class` 收口。
- `notes.md`: 770 行,已记录 B.1/B.2 的 answer/capability 边界。
- `WORKLOG.md`: 674 行,已记录 B.1/B.2 的实现、OpenSpec archive 与验证证据。
- `LATER_PLANS.md`: 474 行,没有直接阻塞 B.3 的活跃待办。
- `ERRORFIX.md`: 606 行,没有需要立即改变 B.3 的错误修复结论。
- `EPIPHANY_LOG.md`: 520 行,已有 bootstrap 不热切换拓扑等架构警戒,与 B.3 的“不热改 parent topology”一致。

## 综合发现

- 方向B这条线已经形成稳定节奏: 窄 OpenSpec change -> focused integration gate -> archive -> stable spec -> smoke/full test -> 本地 commit。
- B.3 当前不应扩成 retry engine、planner 或 runtime topology mutation。
- B.3 的复用知识还在形成中。等 `failure_class` archive 与验证完成后,再决定是否补 `EXPERIENCE.md`。

## 续档动作

- 原 `task_plan.md` 已复制到 `archive/default_history/task_plan_2026-05-16_2314_capability_b3_prev.md`。
- 新 `task_plan.md` 只保留当前 B.3 收口上下文,避免继续在超过 1000 行的文件上追加。

## [2026-05-16 23:31:00] [Session ID: omx-1778510695653-7pd7o2] 笔记: B.3 structured capability failure class 收口结论

## 来源

### 来源1: runtime protocol and records

- `crates/ralph-core/src/capability.rs` 新增 `CapabilityFailureClass`。
- `CapabilityParentFailedRecord` 与 `CapabilityFailedRecord` 都新增 `failure_class`。
- `serde(rename_all = "snake_case")` 让 event payload / artifact JSON 使用稳定小写下划线值。

### 来源2: parent runtime hook

- `crates/ralph-core/src/parallel/supervisor/capability_runtime.rs` 现在给 malformed request、invoker unavailable、generic invoker error 分别写入结构化 class。
- malformed request focused test 已断言返回 payload 包含 `malformed_request`。

### 来源3: CLI isolated invocation adapter

- `crates/ralph-cli/src/capability.rs` 现在给 invalid capability id 分类为 `invalid_capability_id`。
- 已启动 child/micro-run 后失败的路径写入 `child_run_failed`,并进入 `failed.json` artifact。

### 来源4: live parent fallback gate

- `crates/ralph-cli/tests/integration_live_capability.rs` 的 failure fallback backend 第二轮 prompt 必须能 grep 到 `invalid_capability_id` 才会继续发 fallback request。
- 集成测试同时断言 `.ralph/events.jsonl` 中 `capability.failed` payload 的 `failure_class == "invalid_capability_id"`。

## 综合发现

- B.3 的核心价值不是新增 retry engine,而是把 parent branching policy 的输入从自由文本错误提升为结构化字段。
- `error` 字段仍然保留,但用途是人类诊断,不是 policy 的唯一稳定信号。
- 当前最小真实分支是 invalid capability id -> parent sees `invalid_capability_id` -> emits fallback capability request -> fallback success -> explicit `reply.human.message`。
- child/micro-run 执行失败被单独分类为 `child_run_failed`,避免把“选择前失败”和“执行后失败”混在一起。

## [2026-05-17 00:05:00] [Session ID: omx-1778510695653-7pd7o2] 笔记: B.4 richer branching policy without retry engine 收口结论

## 来源

### 来源1: B.4 OpenSpec

- Change: `capability-failure-branching-matrix`
- Archive: `openspec/changes/archive/2026-05-16-capability-failure-branching-matrix/`
- Stable spec: `openspec/specs/capability-invocation/spec.md`

### 来源2: live integration gate

- 新增脚本 helper: `write_malformed_request_diagnostic_backend_script(...)`
- 新增测试: `parallel_parent_run_can_emit_diagnostic_reply_for_malformed_capability_request_without_retry()`

## 综合发现

- B.4 不需要 runtime 代码改动。现有 supervisor 已经能把 malformed `capability.request` 变成 parent-visible `capability.failed`。
- 这次固定了两类 class-specific policy:
  - `invalid_capability_id` 可以走 explicit fallback capability request。
  - `malformed_request` 可以走 explicit diagnostic `reply.human.message`,并且不要求 fallback `capability.result`。
- 这证明 `failure_class` 已经开始服务真实 parent branching policy,而不是只作为日志标签。
- `child_run_failed` 仍适合后续单独做,但不应该为了 B.4 引入测试专用 child failure injection 开关。

## 验证结论

- 新 gate 会先发缺少 `capability_id` 的 malformed request。
- 第二轮 parent prompt 必须包含 `capability.failed`、`malformed_request` 和原 request id,否则脚本失败。
- 事件日志必须同时包含 malformed failure 和 diagnostic human reply,并且不能包含 fallback `capability.result`。

## [2026-05-17 00:37:29] [Session ID: omx-1778510695653-7pd7o2] 笔记: startup bootstrap resolved config 与项目根 ralph.yml 不一致

## 来源

### 来源1: rustdog runtime artifacts

- 路径: `/Users/cuiluming/local_doc/l_dev/my/rust/rustdog/.ralph/bootstrap-selection.json`
- 要点:
  - `reason` 是 `missing default ralph.yml and no explicit prompt; selected v1 rule default`。
  - `selected_resources` 是 `workflow:feature-minimal` 和 `prompt:bootstrap-default-task`。

- 路径: `/Users/cuiluming/local_doc/l_dev/my/rust/rustdog/.ralph/resolved-config.yml`
- 要点:
  - `cli.backend` 是 `claude`。
  - `cli.command` 是 `null`。
  - `parallel.enabled` 是 `true`。

### 来源2: Ralph startup bootstrap 源码

- 路径: `crates/ralph-cli/src/startup_resources.rs`
- 要点:
  - `DEFAULT_CONFIG_FILE` 是 `ralph.yml`。
  - `DEFAULT_BOOTSTRAP_WORKFLOW_ID` 是 `workflow:feature-minimal`。
  - `resolve_default_bootstrap()` 会选择该 workflow 与 `prompt:bootstrap-default-task`。
  - `resolve_workflow_with_prompt_template()` 从 workflow content 解析 `RalphConfig`,再注入 prompt,最后只强制 `config.parallel.enabled = true`。

### 来源3: 当前 default preset 与项目根配置

- 路径: `crates/ralph-cli/presets/feature-minimal.yml`
- 要点:
  - `cli.backend` 是 `claude`。

- 路径: `ralph.yml`
- 要点:
  - `cli.backend` 是 `custom`。
  - `cli.command` 是 `codex`。
  - `cli.args` 是 `exec --sandbox danger-full-access`。
  - `parallel.enabled` 是 `true`,但 `autoscale.max_running_jobs` 是 `7`,不同于 generated artifact 的默认 `4`。

## 综合发现

- 当前 no-config startup bootstrap 的真相源不是项目根 `ralph.yml`。
- 它的真相源是 embedded catalog 中的 `workflow:feature-minimal`。
- 因为 `feature-minimal.yml` 本身只配置了 `cli.backend: claude`,所以 generated artifact 保留了 Claude backend,并从结构体默认值补齐 `command: null`、`prompt_mode: arg` 等字段。
- 如果当前产品契约要求 no-config artifact 等同项目维护的 canonical `ralph.yml`,则要把 canonical default config 纳入 embedded resource 或改 selector 指向新的 default parallel/codex preset。

## [2026-05-17 10:36:48] [Session ID: omx-1778510695653-7pd7o2] 笔记: canonical-default-bootstrap-config OpenSpec review

## 综合发现

### 优点
- proposal 把问题本体收得比较准: 不是 rustdog 目录读取错误,而是 default bootstrap canonical source 漂移。
- design 的大方向是对的: canonical source 应该是 embedded startup resource,而不是运行时直接依赖仓库根 `ralph.yml`。
- spec 已经把“不是字节级一致,而是核心运行语义一致”说清楚,这能避免后面被 YAML 展开字段卡死。

### 需要收紧的点
- spec 现在把 `cli.args=["exec","--sandbox","danger-full-access"]` 和 `parallel.autoscale.max_running_jobs=7` 写死了,这有把 repo 当前默认值直接抬成长期协议的风险。
- 如果这些值未来出于平台、sandbox 或产品策略调整,每次都要改 stable spec,会让 `resource-bootstrap` 规格过度绑定当前实现细节。
- 更稳的写法应该区分:
  - 哪些是“必须对齐 canonical source 的字段集合”
  - 哪些只是“当前 canonical source 的实例值”,更适合由 focused/live gate 锁住而不是写进长期 spec。

### 建议调整方向
- stable spec 保持在字段级 contract:
  - resolved config MUST match canonical startup resource for user-visible bootstrap runtime fields
  - 至少覆盖 `cli.backend`、`cli.command`、`cli.prompt_mode`、`cli.args`、`parallel.enabled`
- 对 `max_running_jobs=7` 这类更像 repo 当前默认策略的值,建议下沉到 change design / tasks / focused test,而不是长期 stable spec 常量。
- 再补一个更明确的 drift gate 语义:
  - repo-maintained `ralph.yml` 和 canonical embedded resource 的关键字段必须通过同一 helper / 同一 fixture / 同一 assertion surface 比较
  - 避免只是口头说“要同步”而没有机械 gate。

### review 结论
- 这组草案方向正确,可以继续。
- 但我建议在进入实现前先把 spec 从“值级硬编码”收窄成“字段级 contract + gate-level concrete values”,这样更稳。

## [2026-05-17 15:25:00] [Session ID: omx-1778510695653-7pd7o2] 笔记: canonical-default-bootstrap-config 实现与归档收口

## 来源

### 来源1: startup resource 实现

- 路径: `crates/ralph-cli/src/startup_resources.rs`
- 要点:
  - 默认 workflow ID 已从 `workflow:feature-minimal` 切到 `workflow:default-parallel`。
  - canonical bootstrap resource 使用 `include_str!("../../../ralph.yml")` 编译期嵌入仓库根默认配置。
  - 旧 `workflow:feature-minimal` 仍保留可物化,但 `selector_eligible=false`,避免继续参与隐式默认选择。

### 来源2: integration drift gate

- 路径: `crates/ralph-cli/tests/integration_startup_resources.rs`
- 要点:
  - dry-run bootstrap gate 断言 selector 选择 `workflow:default-parallel`。
  - gate 比较 root `ralph.yml`、materialized `resources/workflows/default-parallel.yml` 和 `.ralph/resolved-config.yml`。
  - 比较字段覆盖 `cli.backend`、`cli.command`、`cli.prompt_mode`、`cli.args`、`parallel.enabled`、`parallel.autoscale.max_running_jobs`。
  - live gate 也断言真实 run 前的 resolved config 与 repository default config 对齐。

### 来源3: OpenSpec archive

- Archive: `openspec/changes/archive/2026-05-17-canonical-default-bootstrap-config/`
- Stable spec: `openspec/specs/resource-bootstrap/spec.md`
- 要点:
  - `Default startup bootstrap MUST resolve to parallel mode` 已扩展为 canonical source 字段级 contract。
  - 新增 `Startup bootstrap MUST keep one canonical source for default resource semantics` requirement。

## 综合发现

- 这次修复没有把运行时补丁散落在 resolver 中,而是把默认 selector 的源头切到 canonical startup resource。
- 因为 canonical resource 直接编译期嵌入 root `ralph.yml`,仓库维护入口和二进制内置资源之间不再需要手工复制 YAML。
- resolved config 不追求字节级相等,但用户可见的 backend/command/prompt_mode/args/parallel 字段已经由 focused/live gate 锁住。
- archive 过程中暴露了一个 OpenSpec delta 写法问题: 修改旧 requirement 时标题必须匹配 stable spec; 新 requirement 应放在 `ADDED Requirements`。
