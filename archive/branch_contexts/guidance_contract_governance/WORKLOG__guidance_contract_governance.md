
## [2026-05-11 17:40:28] [Session ID: omx-1778475786175-ogndry] 任务名称: 落地 agent guidance contract governance 最小闭环

### 任务内容
- 按 `specs/oh-my-codex-learning-analysis.md` 第 4 节建议,优先落地契约治理,而不是复制完整 OMX runtime。
- 创建 OpenSpec change `agent-guidance-contracts`。
- 新增 agent guidance schema、prompt contract、manifest 和 Rust verifier。
- 将 manifest verifier 接入 `cargo test`。

### 完成过程
- 先处理默认 `task_plan.md` 超过 1000 行的问题,归档到 `archive/default_history/`,并把可复用经验写入 `EXPERIENCE.md`。
- 创建 `openspec/changes/agent-guidance-contracts/` 并写入 proposal/design/tasks/spec。
- 使用 `openspec validate agent-guidance-contracts --type change` 验证规格有效。
- 编写 `docs/agent-guidance-schema.md` 和 `docs/prompt-contract.md`,更新 `AGENTS.md` 索引。
- 新增 `agent-guidance-manifest.toml`,用结构化字段登记核心 guidance 资产。
- 新增 `crates/ralph-core/src/agent_guidance_manifest.rs`,验证 schema version、唯一 id、合法 type/status、summary、路径安全、文件存在和 AGENTS 索引引用。
- 先写 verifier 测试红灯,再补实现。

### 验证证据
- `openspec validate agent-guidance-contracts --type change`: 通过。
- `beautiful-mermaid-rs --ascii < /tmp/agent-guidance-contracts-mermaid/block-1.mmd`: 通过。
- `cargo test --package ralph-core --lib agent_guidance_manifest`: 8 passed,0 failed。
- `cargo test -p ralph-core smoke_runner`: 12 passed,0 failed。
- `cargo test -p ralph-core`: 通过。
- `cargo test`: exit code 0,全量通过。

### 总结感悟
- 这次正确的最小闭环是“docs + manifest + verifier”,不是先做 runtime feature。
- manifest 使用结构化 TOML 比解析 Markdown 注释稳,也和此前“YAML 注释不是 runtime metadata contract”的经验一致。
- verifier 暴露为 public module 后,既消除了 dead_code warning,也让这项能力不只是测试私有 helper。

### 追加验证
- `cargo fmt --check`: 通过。
- `git diff --check`: 通过。
- `openspec validate --all --strict`: 17 passed,0 failed。

## [2026-05-11 19:13:39] [Session ID: omx-1778475786175-ogndry] 任务名称: guidance governance 第二阶段 skill catalog + CLI verifier

### 任务内容
- 按上一阶段建议继续推进 agent guidance governance。
- 新建 OpenSpec change `agent-guidance-catalog-cli`。
- 将项目自有 skills 纳入 `agent-guidance-manifest.toml`。
- 将 verifier 从 cargo-test-only 扩展为独立 CLI 入口 `ralph verify agent-guidance`。

### 完成过程
- 创建并补齐 `openspec/changes/agent-guidance-catalog-cli/` 的 proposal、design、spec 和 tasks。
- 扩展 `crates/ralph-core/src/agent_guidance_manifest.rs`:
  - 新增 `GuidanceManifestReport`。
  - 新增 `verify_default_manifest_with_report` / `verify_manifest_at_with_report`。
  - 保留原有兼容 wrapper。
  - 增加 skill root 检查。
  - 增加 `SKILL.md` frontmatter `name` / `description` 检查。
  - 增加 active/draft skill name 去重。
- 扩展 `agent-guidance-manifest.toml`:
  - 登记 `.agents/skills/*/SKILL.md`。
  - 登记 `.codex/skills/*/SKILL.md`。
  - 对重复 OpenSpec skill,保留 `.agents/skills` 为 canonical active,把 `.codex/skills` 重复条目标为 `archived`。
- 扩展 `crates/ralph-cli/src/main.rs`:
  - 新增 `ralph verify` 命令组。
  - 新增 `ralph verify agent-guidance --manifest ...`。
- 新增 `crates/ralph-cli/tests/integration_verify.rs`,覆盖成功摘要和失败退出码。
- 同步 `docs/agent-guidance-schema.md`,写清 skill catalog 规则和 verifier CLI 命令。

### 验证证据
- `openspec validate agent-guidance-catalog-cli --type change`: 通过。
- `beautiful-mermaid-rs --ascii` 校验 design 中 2 个 Mermaid block: 通过。
- `cargo run -p ralph-cli -- verify agent-guidance --color never`: 通过,输出 `Assets checked: 52`, `Skills checked: 35`。
- `cargo test --package ralph-core --lib agent_guidance_manifest -- --nocapture`: 13 passed,0 failed。
- `cargo test -p ralph-cli verify_agent_guidance_command -- --nocapture`: 2 passed,0 failed。
- `cargo test -p ralph-core smoke_runner`: 12 passed,0 failed。
- `cargo test`: 全量通过。
- `cargo fmt --check`: 通过。
- `git diff --check`: 通过。
- `openspec validate --all --strict`: 18 passed,0 failed。

### 总结感悟
- 第二阶段正确边界是“catalog + CLI”,不是把 runtime state/question/team 一次性带进来。
- `.agents/skills` 和 `.codex/skills` 的 OpenSpec skills 有重复名称,manifest 必须保留一个 canonical active 真相源。
- 独立 CLI 和 cargo test 使用同一套 `ralph-core` verifier,避免长期出现两个校验口径。

## [2026-05-11 20:49:30] [Session ID: omx-1778475786175-ogndry] 任务名称: prompt contract runtime alignment

### 任务内容
- 按用户要求另开 OpenSpec change `prompt-contract-runtime-alignment`。
- 将 `docs/prompt-contract.md` 的 output contract 和 `InstructionBuilder` / hat prompt tests 对齐。
- 明确保留 state operation layer 为后续独立 change,不混入当前 guidance catalog/CLI change。

### 完成过程
- 创建 `openspec/changes/prompt-contract-runtime-alignment/`。
- 写入 proposal、design、spec 和 tasks。
- 用 `beautiful-mermaid-rs --ascii` 验证 design 中 2 个 Mermaid block。
- 先补 `InstructionBuilder` output anchor 断言制造红灯。
- 修改 `InstructionBuilder::build_custom_hat` 的 REPORT 段,加入 `outcome:`、`evidence:`、`changed files:`、`known gaps:`、`next suggestions:` 五个稳定输出锚点。
- 保留原有 evidence-before-completion 和 must-publish 规则。
- 在 `EventLoop::build_prompt` 相关测试里补充同样的 runtime prompt anchor 断言。
- 更新 `docs/prompt-contract.md`,说明这些字段名是 runtime prompt tests 的稳定锚点。
- 将新 OpenSpec change 登记到 `agent-guidance-manifest.toml`。

### 验证证据
- `openspec validate prompt-contract-runtime-alignment --type change`: 通过。
- `beautiful-mermaid-rs --ascii` 校验 design 中 2 个 Mermaid block: 通过。
- 红灯验证: `cargo test --package ralph-core --lib instructions::tests::test_custom_hat_with_rfc2119_patterns -- --exact --nocapture` 先失败于缺少 `outcome:`。
- `cargo run -p ralph-cli -- verify agent-guidance --color never`: 通过,输出 `Assets checked: 53`, `Skills checked: 35`。
- `cargo test --package ralph-core --lib instructions::tests -- --nocapture`: 5 passed,0 failed。
- `cargo test --package ralph-core --lib event_loop::tests::test_custom_hat_with_instructions_uses_build_custom_hat -- --exact --nocapture`: 1 passed,0 failed。
- `cargo test --package ralph-core --lib event_loop::tests::test_build_prompt_uses_ghuntley_style_for_all_hats -- --exact --nocapture`: 1 passed,0 failed。
- `cargo test -p ralph-core smoke_runner`: 12 passed,0 failed。
- `cargo test`: 全量通过。
- `cargo fmt --check`: 通过。
- `git diff --check`: 通过。
- `openspec validate --all --strict`: 19 passed,0 failed。

### 总结感悟
- 这次正确边界是 runtime prompt contract alignment,不是继续扩大到 state operation layer。
- output contract 不应只停留在文档里,需要被 `InstructionBuilder` 和 prompt 构造测试同时固定。
- 文档字段名作为测试锚点后,后续重写 prompt 文案时可以自由优化表达,但不能悄悄丢掉输出契约。

## [2026-05-11 22:54:00] [Session ID: omx-1778475786175-ogndry] 任务名称: state operation layer OpenSpec change

### 任务内容
- 按用户要求继续 oh-my-codex 借鉴路线中的 state operation layer。
- 独立创建 OpenSpec change `state-operation-layer`。
- 只定义 state operation layer 的规格和设计边界,不混入 guidance catalog/CLI 或 prompt contract alignment。

### 完成过程
- 读取 `specs/oh-my-codex-learning-analysis.md` 中 state operation 相关结论。
- 读取 oh-my-codex 的 `src/state/operations.ts` 和 `src/mcp/state-server.ts`,确认可借鉴点是统一 operation contract、原子写和 per-path write queue。
- 读取 Ralph 当前状态面,区分 `.agent/memories.md`、`.agent/tasks.jsonl`、`.ralph/events*.jsonl`、record-session 和未来 runtime workflow state。
- 创建 `openspec/changes/state-operation-layer/`。
- 写入 proposal、design、spec 和 tasks。
- 设计中明确 v1 使用 `.ralph/state/`,不直接复制 `.omx/state/`。
- 设计中明确 v1 不替代 memories/tasks/events/record-session/diagnostics。
- 将 `state-operation-layer-change` 登记到 `agent-guidance-manifest.toml`。

### 验证证据
- `openspec new change state-operation-layer`: 成功创建 spec-driven change。
- `beautiful-mermaid-rs --ascii < /tmp/state-operation-layer-mermaid/block-1.mmd`: 通过。
- `beautiful-mermaid-rs --ascii < /tmp/state-operation-layer-mermaid/block-2.mmd`: 通过。
- `openspec validate state-operation-layer --type change`: 通过。
- `cargo run -p ralph-cli -- verify agent-guidance --color never`: 通过,输出 `Assets checked: 54`, `Skills checked: 35`。
- `openspec validate --all --strict`: 20 passed,0 failed。
- `git diff --check`: 通过。

### 总结感悟
- state operation layer 的第一步不是写 runtime 代码,而是先保护已有真相源边界。
- Ralph 适合采用 `.ralph/state/` 作为 runtime workflow lifecycle state 根目录,避免和 `.agent` 的 agent-facing 文件职责混淆。
- 后续实现应先落 `ralph-core` operation 和单测,再考虑 CLI/MCP/runtime adapter。

## [2026-05-12 09:35:00] [Session ID: omx-1778475786175-ogndry] 任务名称: 实现 state-operation-layer core API

### 任务内容
- 继续 OpenSpec change `state-operation-layer` 的实现任务。
- 在 `ralph-core` 中实现 runtime workflow state operation layer 的 core API。
- 不新增 CLI/MCP adapter,不接入 runtime/question/capability。

### 完成过程
- 按 `openspec-apply-change` 读取 `state-operation-layer` 的 proposal、design、spec 和 tasks。
- 先写 `state_operations` 单元测试制造红灯。
- 红灯结果是 `StateOperationStore`、`StateMode`、`RunOutcome`、`LifecycleOutcome`、`StateWriteRequest`、`StateClearRequest` 等类型未实现。
- 新增 `crates/ralph-core/src/state_operations.rs`。
- 在 `crates/ralph-core/src/lib.rs` 公开 `state_operations` module 和核心类型。
- 实现:
  - `StateMode` 白名单: `ralph`、`ralplan`、`team`、`deep-interview`、`capability-invocation`。
  - `RunOutcome` / `LifecycleOutcome` 枚举和解析校验。
  - `.ralph/state/<mode>-state.json` 路径解析。
  - `.ralph/state/sessions/<session_id>/<mode>-state.json` 路径解析。
  - `StateOperationStore::state_read`。
  - `StateOperationStore::state_write`。
  - `StateOperationStore::state_clear`。
  - `StateOperationStore::state_list_active`。
  - `StateOperationStore::state_get_status`。
  - temp file + rename 原子写。
  - process 内同 path 写入锁。
  - malformed JSON structured error 和 status error。
  - custom `state` object 浅合并。
- 更新 OpenSpec artifacts,把原先设计阶段的“不实现 Rust module”改成“实现 core module,但不新增 CLI/MCP/runtime adapter”。
- 将 `tasks.md` 中 3.x 实现任务全部勾选。

### 验证证据
- 红灯: `cargo test --package ralph-core --lib state_operations -- --nocapture` 首次编译失败,缺少目标类型和函数。
- `cargo test --package ralph-core --lib state_operations -- --nocapture`: 9 passed,0 failed。
- `cargo fmt --check`: 通过。
- `cargo test -p ralph-core smoke_runner`: 12 passed,0 failed。
- `cargo test`: 全量通过。
- `git diff --check`: 通过。
- `openspec validate --all --strict`: 20 passed,0 failed。
- `cargo run -p ralph-cli -- verify agent-guidance --color never`: 54 assets / 35 skills。

### 总结感悟
- state operation layer 的正确第一步是 core-only API,不是先做 CLI 或 MCP 外壳。
- `.ralph/state` 适合作为 workflow lifecycle current view,而 `.ralph/events*.jsonl` / record-session 继续承担 evidence stream。
- 后续 adapter 必须只调用 `StateOperationStore`,不能绕过 core 层直接读写 JSON。

## [2026-05-12 10:34:00] [Session ID: omx-1778475786175-ogndry] 任务名称: 实现 ralph-state-cli-adapter

### 任务内容
- 基于已完成的 `ralph-core` state operation layer,新增 `ralph state ...` CLI adapter。
- 独立创建 OpenSpec change `ralph-state-cli-adapter`。
- 实现 status/read/clear 三个 inspection/cleanup 命令。
- 不新增 `state write`,不接 MCP/runtime/question/capability。

### 完成过程
- 创建 `openspec/changes/ralph-state-cli-adapter/`。
- 写入 proposal、design、spec 和 tasks。
- design 中补充 flowchart 和 sequenceDiagram,并用 `beautiful-mermaid-rs --ascii` 验证。
- 在 `agent-guidance-manifest.toml` 登记新的 OpenSpec change。
- 新增 `crates/ralph-cli/tests/integration_state.rs`。
- 先运行 focused tests 得到红灯: `unrecognized subcommand 'state'`。
- 修改 `crates/ralph-cli/src/main.rs`:
  - 新增 `Commands::State(StateArgs)`。
  - 新增 `StateCommands::{Status, Read, Clear}`。
  - 新增 `StateStatusArgs` / `StateReadArgs` / `StateClearArgs`。
  - 新增 `state_command` / `state_status_command` / `state_read_command` / `state_clear_command`。
  - handler 通过 `StateOperationStore` 调用 core API。
  - mode 解析通过 `StateMode`。
  - `clear` 使用 clap conflict 拒绝 `--session-id` 与 `--all-sessions` 同时出现。
- 补 help 测试,确认 v1 不暴露 `write`。

### 验证证据
- 红灯: `cargo test -p ralph-cli --test integration_state -- --nocapture` 初始失败,4 个测试均报 `unrecognized subcommand 'state'`。
- 实现后 focused: `cargo test -p ralph-cli --test integration_state -- --nocapture`: 5 passed,0 failed。
- `cargo fmt --check`: 通过。
- `cargo test -p ralph-core smoke_runner`: 12 passed,0 failed。
- `cargo test`: 全量通过。
- `git diff --check`: 通过。
- `openspec validate --all --strict`: 21 passed,0 failed。
- `cargo run -p ralph-cli -- verify agent-guidance --color never`: 55 assets / 35 skills。

### 总结感悟
- state CLI adapter 的正确职责是 inspection 和 cleanup,而不是手工 mutation surface。
- `StateOperationStore` 已经成为 state 文件语义的唯一入口,CLI 不应该复刻路径和 JSON 语义。
- 后续 MCP adapter、question obligation、capability invocation runtime state 都应复用同一 core API,并各自单独开 change。

## [2026-05-12 10:48:00] [Session ID: omx-1778475786175-ogndry] 任务名称: 归档 ralph-state-cli-adapter

### 任务内容
- 执行用户要求的 `openspec archive ralph-state-cli-adapter`。
- 处理非交互环境下 OpenSpec 归档确认提示被关闭的问题。
- 归档后修正 guidance manifest 与 main spec purpose。

### 完成过程
- 先读取 `openspec archive --help`,确认可使用 `--yes` 跳过确认提示。
- 执行 `openspec status --change ralph-state-cli-adapter --json`,确认 artifacts 全部 done。
- 执行 `openspec archive ralph-state-cli-adapter --yes`。
- OpenSpec 将 delta spec 同步到 `openspec/specs/state-cli-adapter/spec.md`。
- Change 已移动到 `openspec/changes/archive/2026-05-12-ralph-state-cli-adapter/`。
- 将 `agent-guidance-manifest.toml` 中 `ralph-state-cli-adapter-change` 的 path 更新为 archive 路径,并把 status 改为 `archived`。
- 将新建 main spec 的 Purpose 从 OpenSpec 默认 TBD 更新为实际说明。

### 验证证据
- `openspec archive ralph-state-cli-adapter --yes`: 成功,输出 `archived as '2026-05-12-ralph-state-cli-adapter'`。
- `openspec validate --all --strict`: 21 passed,0 failed。
- `cargo run -p ralph-cli -- verify agent-guidance --color never`: 55 assets / 35 skills。
- `git diff --check`: 通过。
- `openspec list --json`: active changes 中不再包含 `ralph-state-cli-adapter`。

### 总结感悟
- OpenSpec archive 在非交互环境中需要使用 `--yes`,否则确认提示会被关闭导致 exit 1。
- 归档后如果 manifest 登记了 change proposal,必须同步 path 到 archive 目录,否则 guidance verifier 会指向不存在的 active change。
