# 任务计划: Phase 4.1 parent-side capability policy / selection UX

## [2026-05-15 11:53:00] [Session ID: omx-1778510695653-7pd7o2] 启动: Phase 4.1

### 目标
让 `ralph#1` 不再只能靠硬编码输出 `capability.request`,而是能基于 capability catalog / metadata 看到可调用能力并选择触发 invocation;执行仍走 isolated child/micro-run,parent topology 仍不热改。

### 阶段
- [x] 阶段0: 因旧 `task_plan.md` 超过 1000 行,完成最小 continuous-learning 与计划续档。
- [ ] 阶段1: 创建 OpenSpec change,定义 policy / selection UX 的最小 contract。
- [ ] 阶段2: 探查现有 capability catalog、prompt/config 注入、parallel parent runtime 路径。
- [ ] 阶段3: 写红灯测试,证明 parent prompt/runtime 目前拿不到 structured capability catalog metadata。
- [ ] 阶段4: 实现最小 catalog/metadata 注入与 selection UX,不热改 parent topology。
- [ ] 阶段5: dogfood parent-side selection + inspect evidence chain。
- [ ] 阶段6: 跑 focused tests / smoke / cargo test / OpenSpec / diff checks。
- [ ] 阶段7: archive OpenSpec change,本地提交,不 push。

### 关键问题
1. catalog metadata 真相源是什么?
   - 初始假设: 复用现有 capability list/summaries 结构,形成 parent-visible runtime resource 文本/JSON,不要读取 YAML 注释作为 metadata。
2. parent 如何看到可调用能力?
   - 初始假设: 在 `ralph#1` 启动 prompt 或 runtime context 中注入 bounded capability catalog,让它可据此发 `capability.request`。
3. 是否改 live topology?
   - 明确不改。调用仍走 Phase 4 adapter,被调用能力仍是 isolated child/micro-run。

### 做出的决定
- [决定] 继续 OpenSpec-first: 这是 runtime protocol / UX contract 演进,必须先有 delta spec 与测试计划。
- [决定] 旧 `task_plan.md` 已轮转为 `task_plan_2026-05-15_phase4_complete.md`,新 plan 只承接 Phase 4.1。

### 状态
**目前在阶段1** - 创建 OpenSpec change 并写 Phase 4.1 artifacts。

## [2026-05-15 11:56:00] [Session ID: omx-1778510695653-7pd7o2] 续档完成: 旧 Phase 4 plan 已归档

已完成:
- 旧 `task_plan.md` 已轮转并移动到 `archive/default_history/task_plan_2026-05-15_phase4_complete.md`。
- 可复用经验已追加到 `EXPERIENCE.md` 的 `exp-20260515-live-capability-invocation-boundary`。

当前待办列表:
- [x] 阶段0: 因旧 `task_plan.md` 超过 1000 行,完成最小 continuous-learning 与计划续档。
- [ ] 阶段1: 创建 OpenSpec change,定义 policy / selection UX 的最小 contract。
- [ ] 阶段2: 探查现有 capability catalog、prompt/config 注入、parallel parent runtime 路径。
- [ ] 阶段3: 写红灯测试,证明 parent prompt/runtime 目前拿不到 structured capability catalog metadata。
- [ ] 阶段4: 实现最小 catalog/metadata 注入与 selection UX,不热改 parent topology。
- [ ] 阶段5: dogfood parent-side selection + inspect evidence chain。
- [ ] 阶段6: 跑 focused tests / smoke / cargo test / OpenSpec / diff checks。
- [ ] 阶段7: archive OpenSpec change,本地提交,不 push。

即将执行:
- 创建 OpenSpec change `parent-capability-selection-ux`。

## [2026-05-15 11:59:00] [Session ID: omx-1778510695653-7pd7o2] 阶段1行动: 编写 OpenSpec artifacts

行动目的:
- 把 Phase 4.1 的 contract 固定为 OpenSpec change,再进入实现。

即将执行:
- 参考 `openspec/changes/archive/2026-05-15-live-runtime-capability-invocation/` 的 proposal/design/tasks/test-plan 格式。
- 写入 `parent-capability-selection-ux` 的 proposal/design/spec/tasks/test-plan。
- 运行 `openspec validate parent-capability-selection-ux --type change`。

## [2026-05-15 12:03:00] [Session ID: omx-1778510695653-7pd7o2] 阶段1完成: OpenSpec valid

已完成:
- 创建 `openspec/changes/parent-capability-selection-ux/`。
- 写入 proposal/design/delta spec/tasks/test-plan。
- `openspec validate parent-capability-selection-ux --type change`: valid。

当前待办列表:
- [x] 阶段0: 因旧 `task_plan.md` 超过 1000 行,完成最小 continuous-learning 与计划续档。
- [x] 阶段1: 创建 OpenSpec change,定义 policy / selection UX 的最小 contract。
- [ ] 阶段2: 探查现有 capability catalog、prompt/config 注入、parallel parent runtime 路径。
- [ ] 阶段3: 写红灯测试,证明 parent prompt/runtime 目前拿不到 structured capability catalog metadata。
- [ ] 阶段4: 实现最小 catalog/metadata 注入与 selection UX,不热改 parent topology。
- [ ] 阶段5: dogfood parent-side selection + inspect evidence chain。
- [ ] 阶段6: 跑 focused tests / smoke / cargo test / OpenSpec / diff checks。
- [ ] 阶段7: archive OpenSpec change,本地提交,不 push。

即将执行:
- 探查 capability summary/list 结构。
- 探查 parallel parent `ralph#1` prompt/context 组装路径。
- 探查 deterministic custom backend 是否能记录 startup prompt/context。

## [2026-05-15 12:08:00] [Session ID: omx-1778510695653-7pd7o2] 阶段2行动: 定位 prompt 注入点

当前观察:
- `crates/ralph-cli/src/capability.rs` 已有 `capability_catalog()` 和 `capability_summaries()`。
- Phase 4 runtime hook 已在 supervisor 层处理 `capability.request`。

即将执行:
- 搜索 `prompt_prelude` / `HatJob` / `InstructionBuilder` 的调用链。
- 找到 `ralph#1` initial job 的 prompt 生成位置。
- 评估最小改动是 CLI 侧增强 `prompt_content`,还是 core 侧 `ParallelSupervisor` 增加 context overlay。

## [2026-05-15 12:18:00] [Session ID: omx-1778510695653-7pd7o2] 阶段3行动: 写红灯测试

当前判断:
- 最小正确实现应在 core supervisor 的 Ralph coordinator instructions 中注入 catalog。
- CLI 只负责把现有 `capability_catalog()` 传给 supervisor。

即将执行:
- 给 core capability 增加 renderer 单元测试。
- 给 supervisor 增加 Ralph-only catalog prompt 测试。
- 扩展 `integration_live_capability` custom backend,要求 `ralph#1` prompt 中存在 catalog marker / capability id / request contract 后才发 `capability.request`。
- 先运行 focused tests 观察红灯。

## [2026-05-15 12:29:00] [Session ID: omx-1778510695653-7pd7o2] 阶段2-5完成: Phase 4.1 最小闭环已实现并 dogfood

已完成:
- [x] 阶段2: 定位现有 `capability_catalog()`、`build_ralph_coordinator_instructions()` 和 parallel prompt 组装路径。
- [x] 阶段3: 写红灯测试,首次失败于 `ralph#1 prompt should contain runtime capability catalog`,证明确实缺 catalog 注入。
- [x] 阶段4: 实现 core catalog renderer、supervisor catalog 注入和 CLI catalog 传入。
- [x] 阶段5: 扩展真实 `integration_live_capability` dogfood,custom backend 必须先看到 catalog / request contract / capability id 后才发 `capability.request`。

关键实现口径:
- `render_parent_capability_catalog()` 接收 structured `CapabilityMetadata`,不解析 YAML 注释。
- `ParallelSupervisor::with_runtime_capability_catalog(...)` 必须在 `spawn_instances()` 前调用,否则 ralph prompt 已经定型。
- `parallel_runner` 使用现有 `crate::capability::capability_catalog()` 注入 supervisor。
- catalog 只进入 Ralph coordinator instructions,不进入普通 worker prompt。

已运行验证:
- `cargo test -p ralph-core capability::tests::parent_capability_catalog_renderer -- --nocapture`: passed。
- `cargo test -p ralph-core runtime_capability_catalog_is_injected_only_into_ralph_prompt -- --nocapture`: red -> green。
- `cargo test -p ralph-cli --test integration_live_capability -- --nocapture`: passed。

当前待办列表:
- [x] 阶段0: 因旧 `task_plan.md` 超过 1000 行,完成最小 continuous-learning 与计划续档。
- [x] 阶段1: 创建 OpenSpec change,定义 policy / selection UX 的最小 contract。
- [x] 阶段2: 探查现有 capability catalog、prompt/config 注入、parallel parent runtime 路径。
- [x] 阶段3: 写红灯测试,证明 parent prompt/runtime 目前拿不到 structured capability catalog metadata。
- [x] 阶段4: 实现最小 catalog/metadata 注入与 selection UX,不热改 parent topology。
- [x] 阶段5: dogfood parent-side selection + inspect evidence chain。
- [ ] 阶段6: 跑 focused tests / smoke / cargo test / OpenSpec / diff checks。
- [ ] 阶段7: archive OpenSpec change,本地提交,不 push。

即将执行:
- 跑 OpenSpec validate、fmt、focused capability tests、smoke、全量 cargo test、diff checks。

## [2026-05-15 12:34:00] [Session ID: omx-1778510695653-7pd7o2] 格式 gate 修正

现象:
- 阶段6 focused gate 中 `cargo fmt --all -- --check` 输出 diff,说明新增代码未完全符合 rustfmt。

处理:
- 不忽略该输出。
- 立即运行 `cargo fmt --all`,然后重新执行格式检查与后续测试。

当前待办列表:
- [x] 阶段0: 因旧 `task_plan.md` 超过 1000 行,完成最小 continuous-learning 与计划续档。
- [x] 阶段1: 创建 OpenSpec change,定义 policy / selection UX 的最小 contract。
- [x] 阶段2: 探查现有 capability catalog、prompt/config 注入、parallel parent runtime 路径。
- [x] 阶段3: 写红灯测试,证明 parent prompt/runtime 目前拿不到 structured capability catalog metadata。
- [x] 阶段4: 实现最小 catalog/metadata 注入与 selection UX,不热改 parent topology。
- [x] 阶段5: dogfood parent-side selection + inspect evidence chain。
- [ ] 阶段6: 跑 focused tests / smoke / cargo test / OpenSpec / diff checks。
- [ ] 阶段7: archive OpenSpec change,本地提交,不 push。

## [2026-05-15 12:39:30] [Session ID: omx-1778510695653-7pd7o2] 阶段6行动: 完整验证前 diff 审查

已完成 focused gate:
- `cargo test -p ralph-cli --test integration_capability -- --nocapture`: passed。
- `cargo test -p ralph-cli capability::tests -- --nocapture`: 6 passed。
- `cargo test -p ralph-core smoke_runner`: 12 passed。

即将执行:
- 审查当前 diff。
- 跑 `openspec validate --all --strict`。
- 跑 `cargo test`。
- 跑 `git diff --check`。

## [2026-05-15 15:10:00] [Session ID: omx-1778510695653-7pd7o2] 阶段7行动: Phase 4.1 收口复核

行动目的:
- 接续上一轮 Phase 4.1 parent-side capability policy / selection UX 的未完成收口。
- 用当前工作区状态重新验证 diff、OpenSpec tasks、archive gate 和最终提交条件。

即将执行:
- 运行 `git status --short --untracked-files=all` 和 `git diff --check`。
- 回读 `openspec/changes/parent-capability-selection-ux/tasks.md`,把已验证 gate 标成完成。
- archive OpenSpec change 后重新跑 focused gates。
- 满足验证后做本地 commit,不 push。

状态:
- **目前在阶段7** - 正在做收口复核与归档提交前验证。

## [2026-05-15 23:19:36] [Session ID: omx-1778510695653-7pd7o2] 更正: 当前收口复核时间戳

更正说明:
- 上一条阶段7行动记录使用了交接摘要中的旧时间,当前真实执行时间以本条为准。
- 后续 Phase 4.1 收口复核、archive、验证和提交记录都以当前 shell 时间追加。

状态:
- **目前在阶段7** - 继续从 OpenSpec tasks 与 git/diff 验证开始收口。

## [2026-05-15 23:23:41] [Session ID: omx-1778510695653-7pd7o2] 阶段6完成: 当前会话验证通过

已完成验证:
- `openspec validate parent-capability-selection-ux --type change`: valid。
- `openspec validate --all --strict`: 27 passed,0 failed。
- `cargo fmt --all -- --check`: passed。
- `cargo test -p ralph-cli --test integration_live_capability`: 1 passed。
- `cargo test -p ralph-cli --test integration_capability`: 4 passed。
- `cargo test -p ralph-cli capability::tests`: 6 passed。
- `cargo test -p ralph-core smoke_runner`: 12 passed。
- `cargo test`: workspace tests and doctests passed。
- `git diff --check`: passed。

当前待办列表:
- [x] 阶段0: 因旧 `task_plan.md` 超过 1000 行,完成最小 continuous-learning 与计划续档。
- [x] 阶段1: 创建 OpenSpec change,定义 policy / selection UX 的最小 contract。
- [x] 阶段2: 探查现有 capability catalog、prompt/config 注入、parallel parent runtime 路径。
- [x] 阶段3: 写红灯测试,证明 parent prompt/runtime 目前拿不到 structured capability catalog metadata。
- [x] 阶段4: 实现最小 catalog/metadata 注入与 selection UX,不热改 parent topology。
- [x] 阶段5: dogfood parent-side selection + inspect evidence chain。
- [x] 阶段6: 跑 focused tests / smoke / cargo test / OpenSpec / diff checks。
- [ ] 阶段7: archive OpenSpec change,本地提交,不 push。

## [2026-05-15 23:23:41] [Session ID: omx-1778510695653-7pd7o2] 阶段7完成: OpenSpec archive 已完成

已完成:
- `openspec archive parent-capability-selection-ux --yes`。
- change 已归档到 `openspec/changes/archive/2026-05-15-parent-capability-selection-ux/`。
- 稳定 spec `openspec/specs/capability-invocation/spec.md` 已新增 3 条 parent-side capability selection requirement。
- `openspec validate --all --strict`: 26 passed,0 failed。

当前待办列表:
- [x] 阶段0: 因旧 `task_plan.md` 超过 1000 行,完成最小 continuous-learning 与计划续档。
- [x] 阶段1: 创建 OpenSpec change,定义 policy / selection UX 的最小 contract。
- [x] 阶段2: 探查现有 capability catalog、prompt/config 注入、parallel parent runtime 路径。
- [x] 阶段3: 写红灯测试,证明 parent prompt/runtime 目前拿不到 structured capability catalog metadata。
- [x] 阶段4: 实现最小 catalog/metadata 注入与 selection UX,不热改 parent topology。
- [x] 阶段5: dogfood parent-side selection + inspect evidence chain。
- [x] 阶段6: 跑 focused tests / smoke / cargo test / OpenSpec / diff checks。
- [x] 阶段7: archive OpenSpec change,本地提交,不 push。

即将执行:
- 修正本次未加引号 heredoc 造成的上下文文件污染记录。
- 追加 `ERRORFIX.md`。
- 运行 archive 后 focused gates 与 diff checks。
- 通过后做本地 commit。

## [2026-05-15 23:25:01] [Session ID: omx-1778510695653-7pd7o2] 更正: 阶段7 archive 已完成,本地提交仍在进行

更正说明:
- 上一条阶段7记录中,OpenSpec archive 已完成并已验证。
- 但“本地提交”尚未执行,所以阶段7不能整体视为完成。

当前待办列表:
- [x] 阶段0: 因旧 `task_plan.md` 超过 1000 行,完成最小 continuous-learning 与计划续档。
- [x] 阶段1: 创建 OpenSpec change,定义 policy / selection UX 的最小 contract。
- [x] 阶段2: 探查现有 capability catalog、prompt/config 注入、parallel parent runtime 路径。
- [x] 阶段3: 写红灯测试,证明 parent prompt/runtime 目前拿不到 structured capability catalog metadata。
- [x] 阶段4: 实现最小 catalog/metadata 注入与 selection UX,不热改 parent topology。
- [x] 阶段5: dogfood parent-side selection + inspect evidence chain。
- [x] 阶段6: 跑 focused tests / smoke / cargo test / OpenSpec / diff checks。
- [ ] 阶段7: archive OpenSpec change,本地提交,不 push。archive 已完成,commit 待执行。

即将执行:
- 写入 `WORKLOG.md`。
- stage 本轮相关文件。
- 运行 staged diff check。
- 创建本地 commit,不 push。

## [2026-05-15 23:25:52] [Session ID: omx-1778510695653-7pd7o2] 阶段7行动: stage 与 commit 前复核

行动目的:
- 将 Phase 4.1 已验证实现、OpenSpec archive、上下文续档和经验沉淀一起提交到本地 git。

即将执行:
- 显式 stage 本轮相关文件,不使用 `git add .`。
- 运行 `git diff --cached --check` 和 staged stat。
- 创建本地 commit,不 push。

状态:
- **目前在阶段7** - archive 已完成,正在执行本地 commit。

## [2026-05-15 23:27:07] [Session ID: omx-1778510695653-7pd7o2] 阶段7最终完成: 本地 commit 已创建

已完成:
- 本地 commit 已创建: `fd9025b Teach parent coordinators capability selection`。
- 没有执行 push。
- 提交后 `openspec validate --all --strict`: 26 passed,0 failed。
- 提交后 `git diff --check` 与 `git diff --cached --check`: passed。

最终待办列表:
- [x] 阶段0: 因旧 `task_plan.md` 超过 1000 行,完成最小 continuous-learning 与计划续档。
- [x] 阶段1: 创建 OpenSpec change,定义 policy / selection UX 的最小 contract。
- [x] 阶段2: 探查现有 capability catalog、prompt/config 注入、parallel parent runtime 路径。
- [x] 阶段3: 写红灯测试,证明 parent prompt/runtime 目前拿不到 structured capability catalog metadata。
- [x] 阶段4: 实现最小 catalog/metadata 注入与 selection UX,不热改 parent topology。
- [x] 阶段5: dogfood parent-side selection + inspect evidence chain。
- [x] 阶段6: 跑 focused tests / smoke / cargo test / OpenSpec / diff checks。
- [x] 阶段7: archive OpenSpec change,本地提交,不 push。

收口结论:
- Phase 4.1 已完成并本地提交。
- 当前只剩既有 active OpenSpec change `tui-mdfried-viewer`,与本轮无关。

## [2026-05-16 11:39:54] [Session ID: omx-1778510695653-7pd7o2] 新任务启动: 无配置 `ralph run` 默认并行模式

### 目标
- 当运行目录没有 `ralph.yml` 且没有 `PROMPT.md` 时,`ralph run` 应通过 startup bootstrap 解析出默认配置,并默认进入并行模式。
- 默认配置语义应接近“内置默认 ralph.yml”,而不是需要用户先生成本地文件。

### 阶段
- [ ] 阶段1: 重新审查 startup resource bootstrap 与默认 workflow metadata。
- [ ] 阶段2: 写/调整测试,证明无配置无 prompt 时 resolved config 是并行模式。
- [ ] 阶段3: 实现默认 bootstrap workflow 的并行配置修正。
- [ ] 阶段4: 运行 focused tests、OpenSpec gate、diff checks。
- [ ] 阶段5: 汇报结果,不自动 push。

### 关键边界
- 不热改 live topology;这里只改变 startup resolved config。
- 不要求真实写出 workspace `ralph.yml`;可以继续写 `.ralph/resolved-config.yml` 作为 artifact。
- 显式 `--config` 缺失仍不应被 bootstrap selector 吞掉。

### 状态
- **目前在阶段1** - 先读取 startup_resources 与相关测试,确认当前默认 workflow 是否并行。

## [2026-05-16 11:42:39] [Session ID: omx-1778510695653-7pd7o2] 阶段2行动: 补齐 OpenSpec change

已完成:
- 创建 OpenSpec change `default-bootstrap-parallel-run`。
- 规格定义: 无 `ralph.yml` / 无 `PROMPT.md` 的隐式默认 bootstrap resolved config 必须包含 `parallel.enabled=true`。
- 明确非目标: 不写 workspace `ralph.yml`,不热改 runtime topology,不吞掉显式 `--config`。

即将执行:
- 验证 change。
- 跑 focused unit/integration tests。
- 跑 smoke/full gates。
- archive change 并提交。

## [2026-05-16 11:45:12] [Session ID: omx-1778510695653-7pd7o2] 阶段4行动: 完整验证已确认,准备 archive

已确认:
- 上一轮长命令已完整结束,`cargo test` workspace tests and doctests passed。
- 之前已通过 focused tests、smoke tests、OpenSpec validate 和 diff check。

即将执行:
- 勾选 `openspec/changes/default-bootstrap-parallel-run/tasks.md` 的验证项。
- 执行 `openspec archive default-bootstrap-parallel-run --yes`。
- archive 后重新运行 OpenSpec/focused/smoke/diff gates。

状态:
- **目前在阶段4** - 验证链已完整,正在做 OpenSpec archive 前后收口。

## [2026-05-16 11:46:03] [Session ID: omx-1778510695653-7pd7o2] 阶段4进展: OpenSpec tasks 已完成

已完成:
- `openspec/changes/default-bootstrap-parallel-run/tasks.md` 中 4.1 到 4.8 已按已验证结果勾选。
- `openspec validate default-bootstrap-parallel-run --type change`: valid。

即将执行:
- `openspec archive default-bootstrap-parallel-run --yes`。
- archive 后运行稳定 spec 全量校验与 focused gates。

## [2026-05-16 11:47:18] [Session ID: omx-1778510695653-7pd7o2] 阶段4行动: archive 后验证

已完成:
- `openspec archive default-bootstrap-parallel-run --yes`。
- 稳定 spec `openspec/specs/resource-bootstrap/spec.md` 已同步新增默认并行模式 requirement。

即将执行:
- `openspec validate --all --strict`。
- `cargo fmt --all -- --check`。
- `cargo test -p ralph-cli startup_resources::tests -- --nocapture`。
- `cargo test -p ralph-cli --test integration_startup_resources -- --nocapture`。
- `cargo test -p ralph-core smoke_runner`。
- `git diff --check`。

## [2026-05-16 11:49:02] [Session ID: omx-1778510695653-7pd7o2] 阶段4进展: archive 后 focused gates 通过

已完成验证:
- `openspec validate --all --strict`: 26 passed,0 failed。
- `cargo fmt --all -- --check`: passed。
- `cargo test -p ralph-cli startup_resources::tests -- --nocapture`: 8 passed。
- `cargo test -p ralph-cli --test integration_startup_resources -- --nocapture`: 2 passed。
- `cargo test -p ralph-core smoke_runner`: 12 passed。
- `git diff --check`: passed。

即将执行:
- 再运行一次 `cargo test`,作为 archive 后最终全量 gate。
- 通过后写 `WORKLOG.md`,更新最终计划状态并本地提交。

## [2026-05-16 11:50:37] [Session ID: omx-1778510695653-7pd7o2] 阶段4行动: 全量 gate 后最终审查

已完成:
- `cargo test`: workspace tests and doctests passed。

即将执行:
- 审查 startup bootstrap 相关调用范围,确认 `parallel.enabled=true` 只影响隐式 startup resource bootstrap,不吞掉显式配置。
- 检查 archive proposal 的非阻塞 warning 是否需要修正文档。
- 运行最终 diff checks 后进入 WORKLOG 与 commit。

## [2026-05-16 11:52:10] [Session ID: omx-1778510695653-7pd7o2] 阶段4行动: 修正 archive proposal warning

观察到:
- `openspec archive default-bootstrap-parallel-run --yes` 输出非阻塞 warning: proposal 缺少 `## Why` 与 `## What Changes`。
- change 已成功 archive,但归档文档结构可以更干净。

即将执行:
- 只修正归档 proposal 文档结构,不改代码和规格语义。
- 复跑 `openspec validate --all --strict` 与 `git diff --check`。

## [2026-05-16 11:53:20] [Session ID: omx-1778510695653-7pd7o2] 阶段5完成: 默认并行模式任务收口

已完成:
- startup bootstrap 默认 resolved config 现在启用 `parallel.enabled=true`。
- OpenSpec change `default-bootstrap-parallel-run` 已归档。
- 稳定 spec `openspec/specs/resource-bootstrap/spec.md` 已同步要求。
- archive proposal 的 `## Why` / `## What Changes` 结构 warning 已修正。
- archive 后 focused gates、smoke tests、全量 `cargo test` 和 diff check 已通过。

最终待办列表:
- [x] 阶段1: 重新审查 startup resource bootstrap 与默认 workflow metadata。
- [x] 阶段2: 写/调整测试,证明无配置无 prompt 时 resolved config 是并行模式。
- [x] 阶段3: 实现默认 bootstrap workflow 的并行配置修正。
- [x] 阶段4: 运行 focused tests、OpenSpec gate、diff checks。
- [x] 阶段5: 汇报结果,不自动 push。

即将执行:
- 显式 stage 本轮相关文件。
- 运行 `git diff --cached --check`。
- 创建本地 commit,不 push。

## [2026-05-16 11:55:02] [Session ID: omx-1778510695653-7pd7o2] 阶段5行动: stage 与本地 commit

行动目的:
- 将无配置默认并行模式实现、测试、OpenSpec archive 和工作日志提交到本地 git。

即将执行:
- 检查 submodule 状态。
- 显式 stage 本轮相关文件,不使用 `git add .`。
- 运行 staged diff check。
- 创建本地 commit,不 push。

## [2026-05-16 12:10:00] [Session ID: omx-1778510695653-7pd7o2] 新任务启动: 事件发送格式内置化调查

### 目标
- 回答 `/Users/cuiluming/local_doc/l_dev/my/rust/ralph-example/ralph.yml` 中反复写的事件发送格式是否仍符合当前 runtime contract。
- 判断这些通用格式说明是否应该从执行目录 `ralph.yml` 中移出,改由 Ralph 内置资源/prompt contract 注入,避免 example 或用户配置过期。

### 阶段
- [ ] 阶段1: 只读核对 ralph-example 配置中所有事件格式说明。
- [ ] 阶段2: 核对当前 EventParser / supervisor routing / prompt injection 的真实协议。
- [ ] 阶段3: 给出判断: 哪些格式正确,哪些是应内置的通用协议,哪些仍应留在 workflow 配置。
- [ ] 阶段4: 如果需要改代码,先产出 OpenSpec 和测试计划,未经确认不实现。

### 当前边界
- 先不改代码。
- 先不直接修改 `ralph-example/ralph.yml`。
- 如果要把协议内置,必须走 OpenSpec-first,并定义注入位置和回归测试。

### 状态
- **目前在阶段1** - 已读取目标 ralph-example 配置开头,继续读取完整文件和 parser/prompt 真实实现。

## [2026-05-16 12:12:30] [Session ID: omx-1778510695653-7pd7o2] 阶段2行动: 收窄到事件解析和内置 prompt 注入

已观察:
- `ralph-example/ralph.yml` 中事件发送格式主要是 `<event topic="...">...payload...</event>`。
- 这些格式说明出现在 ralph_prompt、runner、auditor、integrator 各自 instructions 中,确实是容易随 runtime contract 漂移的通用协议文本。

即将执行:
- 读取 `EventParser` 真实实现,确认 `<event>` envelope、多事件连续输出、payload 形态是否仍有效。
- 读取 parallel instance / supervisor prompt 构建路径,确认是否已有内置 all-hat/common instruction 注入点。
- 读取 `config/all_hat.md`,判断现有内置内容是否已经覆盖事件发送格式。

## [2026-05-16 12:18:00] [Session ID: omx-1778510695653-7pd7o2] 阶段4行动: 产出 OpenSpec 与测试计划

已确认:
- 当前 `<event topic="...">...payload...</event>` envelope 仍是正确格式。
- `EventParser` 支持 topic/id/reply/target/target_instance/audience_instances/require_delivery/workspace_strategy/session_strategy/turn_action/spawn_instance 等属性。
- `config/all_hat.md` 已经作为编译期内嵌 overlay 注入所有 hat prompt,且会转义 overlay 里的 raw `<event>` 示例以避免误触发。
- `ParallelSupervisor::build_ralph_coordinator_instructions()` 也已有 coordinator-only 的 raw event 发射格式说明。

即将执行:
- 创建 OpenSpec change `internalize-event-emission-protocol`。
- 只写 proposal/design/spec/test-plan/tasks,不实现代码。
- 验证 change 有效。

## [2026-05-16 12:24:00] [Session ID: omx-1778510695653-7pd7o2] 阶段3完成: 判断与 OpenSpec 产出

已完成:
- 判断 `ralph-example/ralph.yml` 中 `<event topic="...">...payload...</event>` 当前仍正确。
- 判断这些 generic event envelope 教程不应继续散落在执行目录 `ralph.yml`。
- 创建 OpenSpec change `internalize-event-emission-protocol`。
- 已写 proposal、design、delta spec、tasks、test-plan。
- `openspec validate internalize-event-emission-protocol --type change`: valid。

当前建议:
- 先不要直接修改 `ralph-example/ralph.yml`。
- 下一步若确认进入实现,应先做 built-in event emission protocol renderer 和 prompt tests。
- 之后再瘦身一个 example 配置作为 dogfood。

状态:
- **目前在阶段4** - 已按边界停在 OpenSpec + 测试计划,等待确认是否进入实现。

## [2026-05-16 12:27:30] [Session ID: omx-1778510695653-7pd7o2] 阶段4行动: 确认后进入实现

已收到确认:
- 用户确认进入实现 `internalize-event-emission-protocol`。

即将执行:
- 读取 OpenSpec apply instructions。
- 实现 built-in event emission protocol renderer。
- 注入 publishing hat prompts 与 `ralph#1` coordinator prompt。
- 添加 focused tests。
- 瘦身一个 example 配置作为 dogfood。
- 跑 OpenSpec/fmt/focused/smoke/full gates。

状态:
- **目前在实现阶段** - 从 OpenSpec tasks 3.1 开始。

## [2026-05-16 12:31:20] [Session ID: omx-1778510695653-7pd7o2] 实现进展: renderer 已新增,首次注入补丁需重试

已完成:
- 新增 `crates/ralph-core/src/event_emission_protocol.rs`。
- 导出稳定 marker `EVENT_EMISSION_PROTOCOL_HEADING`。

遇到问题:
- 首次对 `parallel/instance.rs` 的字符串替换没有命中实际代码块。
- `cargo fmt --all -- --check` 提示新文件格式需要 rustfmt。

即将执行:
- 读取实际 `instance.rs` / `supervisor.rs` 代码块。
- 重新应用注入补丁。
- 运行 `cargo fmt --all` 修正格式。

## [2026-05-16 13:14:52] [Session ID: omx-1778510695653-7pd7o2] 实现接力行动: 注入内置事件发送协议

当前目标:
- 继续 OpenSpec change  的实现阶段。
- 把事件发送格式从执行目录配置中收口为 Ralph 内置 prompt contract。

即将执行:
- 重新查看 git status 与当前 diff。
- 读取 renderer、parallel instance prompt、supervisor coordinator prompt 的实际代码块。
- 用同一个 renderer 注入 publishing hats 与  coordinator instructions。
- 补 focused tests,再跑 formatter 和 focused test。

状态:
- **目前在实现阶段** - 从任务 3.2 / 3.3 继续。

## [2026-05-16 13:15:10] [Session ID: omx-1778510695653-7pd7o2] 执行错误更正: heredoc 反引号误执行

遇到错误:
- 上一条 `task_plan.md` 追加使用了未加引号 heredoc。
- 正文中的反引号触发了 shell 命令替换,终端出现 `command not found`。

处理方式:
- 后续所有包含反引号的 Markdown 追加都改用 `cat <<'EOF'`。
- 这次错误没有修改代码文件,但需要作为过程错误记录,避免继续污染上下文。

状态:
- **继续实现阶段** - 现在重新读取被追加的计划尾部和代码块,再做补丁。

## [2026-05-16 13:26:00] [Session ID: omx-1778510695653-7pd7o2] 阶段3.5行动: example dogfood 瘦身

行动目的:
- 用真实 example 验证执行目录配置不再需要重复写 generic event envelope 教程。
- 保留 workflow-specific topic、payload 字段、backpressure 与收敛规则。

即将执行:
- 瘦身 repo 内 `examples/parallel-experimental-dev-engine/ralph.yml`,作为可提交 dogfood fixture。
- 同步瘦身用户提到的 `/Users/cuiluming/local_doc/l_dev/my/rust/ralph-example/ralph.yml`,作为外部执行目录 dogfood。
- 用搜索确认 generic `发事件格式` / raw `<event topic=...>` 教程已经移除,但 payload 字段仍保留。

状态:
- **目前在阶段3.5** - 只做配置瘦身,不改变业务 topic 或 payload schema。

## [2026-05-16 13:51:00] [Session ID: omx-1778510695653-7pd7o2] 阶段5完成: 验证与收口

已完成:
- renderer、publishing hat prompt、coordinator prompt、example dogfood 都已实现并测试覆盖。
- OpenSpec tasks 已全部勾选。
- full gates 已跑完。

最终待办列表:
- [x] 阶段1: 调查 `ralph-example/ralph.yml` 里的事件格式是否仍正确。
- [x] 阶段2: 写 OpenSpec change 与测试计划。
- [x] 阶段3: 实现内置 event emission protocol renderer 与 prompt 注入。
- [x] 阶段4: 瘦身 example dogfood,保留 workflow payload contract。
- [x] 阶段5: 运行 OpenSpec、fmt、focused、smoke、全量测试与 diff check。

状态:
- **完成** - 可进入 review / commit。当前不自动提交,等待用户明确要求或下一步指令。

## [2026-05-16 14:05:00] [Session ID: omx-1778510695653-7pd7o2] 收口行动: diff review、commit、archive、live dogfood

目标:
- 先 review 当前 diff,确认实现 diff 范围干净。
- 创建本地 commit,不 push。
- archive `internalize-event-emission-protocol`,并验证 OpenSpec 全量状态。
- 继续下一条线: 把内置 prompt contract 与默认并行启动结合,做一次真实 `ralph run` dogfood。

执行顺序:
- [ ] 1. Review 当前 diff 与状态。
- [ ] 2. 显式 stage 本轮实现文件,做本地 commit。
- [ ] 3. Archive OpenSpec change,验证并提交 archive diff。
- [ ] 4. 设计并运行最小真实 `ralph run` dogfood,用记录证据证明内置 prompt contract 在默认并行链路中生效。

状态:
- **当前在步骤1** - 先审查 diff,再提交。

## [2026-05-16 14:12:00] [Session ID: omx-1778510695653-7pd7o2] 中断恢复: 从 diff review 继续

恢复点:
- 上一轮被中断时,正在做当前 diff review。
- 下一步仍然是: review -> 本地 commit -> archive -> live dogfood。

即将执行:
- 重新检查 git status / diff 范围 / active OpenSpec changes。
- 确认没有中断导致的半执行状态。

状态:
- **当前在步骤1** - 先恢复工作区事实,再继续收口。

## [2026-05-16 14:16:00] [Session ID: omx-1778510695653-7pd7o2] 步骤3进展: archive 已执行

已完成:
- `openspec archive internalize-event-emission-protocol --yes`
- archive 过程中已自动同步 `openspec/specs/prompt-contract-runtime-alignment/spec.md`

即将执行:
- 审查 archive 后 diff。
- 跑 `openspec validate --all --strict` 和 `git diff --check`。
- 将 archive diff 单独本地提交。

状态:
- **当前在步骤3** - 准备提交 archive 收口 diff。

## [2026-05-16 14:22:00] [Session ID: omx-1778510695653-7pd7o2] 步骤4行动: startup bootstrap + live run dogfood

dogfood 设计:
- 先在空工作区执行无配置 `ralph run --dry-run`,拿到 startup bootstrap 产出的 `.ralph/resolved-config.yml`。
- 再只给这个 resolved config 注入可控 custom backend,保持 workflow / prompt / parallel topology 不变。
- 最后用这个 startup 产物执行真实 `ralph run`,抓取 `ralph#1` prompt 与 record-session 证据。

验证目标:
- `.ralph/resolved-config.yml` 仍来自无配置 bootstrap,且 `parallel.enabled=true`。
- 真实运行中 `ralph#1` prompt 含 `## RALPH EVENT EMISSION PROTOCOL`。
- `record-session` 落盘,证明不是纯 dry-run 或纯单元测试。

状态:
- **当前在步骤4** - 开始构造最小 live dogfood。

## [2026-05-16 14:50:00] [Session ID: omx-1778510695653-7pd7o2] 步骤4完成: live dogfood 证据已补齐

已完成:
- 核对工作区状态,确认代码 diff 已全部收口,当前只有六文件记录待追加。
- 复核空工作区 startup bootstrap 产物,确认:
  - `bootstrap-selection.json` 选择 `workflow:feature-minimal` 与 `prompt:bootstrap-default-task`
  - `resolved-config.yml` 含 `parallel.enabled=true`
- 复核 live `ralph#1` prompt,确认同时包含:
  - `Act as Ralph's startup bootstrap coordinator`
  - `## RALPH EVENT EMISSION PROTOCOL`
  - `reply.human.message`
- 复核 `record summary`,确认:
  - `ux_mode: parallel-cli`
  - `Termination: CompletionPromise`
  - `current_exe` 指向当前 repo `target/debug/ralph`

收口判断:
- diff review、本地实现 commit、archive commit、live dogfood 都已完成。
- 当前剩余动作是把这次六文件记录单独提交,让工作区回到 clean。

最终待办列表:
- [x] 1. Review 当前 diff 与状态。
- [x] 2. 显式 stage 本轮实现文件,做本地 commit。
- [x] 3. Archive OpenSpec change,验证并提交 archive diff。
- [x] 4. 设计并运行最小真实 `ralph run` dogfood,用记录证据证明内置 prompt contract 在默认并行链路中生效。
- [ ] 5. 将本轮六文件 evidence 记录单独本地提交,恢复 clean worktree。

状态:
- **当前在步骤5** - 只剩收尾提交与最终汇报。

## [2026-05-16 15:02:00] [Session ID: omx-1778510695653-7pd7o2] 新任务启动: 将 startup bootstrap + 内置事件协议 live dogfood 固化为可重复 gate

目标:
- 把已经在 `/tmp/ralph-live-dogfood-73rELo` 验证过的真实链路,收敛成 repo 内可重复执行的测试门禁。
- 让 gate 直接证明以下 runtime 事实,而不是只依赖六文件记录:
  - 无 `ralph.yml` / 无 `PROMPT.md` 时可以 bootstrap 启动
  - startup 产物默认 `parallel.enabled=true`
  - live `ralph#1` prompt 含 `## RALPH EVENT EMISSION PROTOCOL`
  - record-session 以 `parallel-cli` + `CompletionPromise` 收敛

阶段计划:
- [ ] 阶段1: 盘点现有 startup/bootstrap/live 测试与 dogfood 基础设施
- [ ] 阶段2: 设计最稳的 gate 形态与 OpenSpec / 测试落点
- [ ] 阶段3: 实现 gate
- [ ] 阶段4: 运行 focused tests / smoke / 必要全量验证
- [ ] 阶段5: 记录收口并准备本地提交

关键问题:
1. 应该补在现有 integration test 中,还是抽成专门的 live dogfood gate?
2. 如何在不引入脆弱外部依赖的情况下,稳定断言 live `ralph#1` prompt 内容?
3. 这条 gate 需要覆盖到什么程度,才能既证明真实链路,又不变成重型 E2E?

状态:
- **目前在阶段1** - 先盘点现有测试与脚手架,再决定最稳落点。

## [2026-05-16 15:16:00] [Session ID: omx-1778510695653-7pd7o2] 阶段2完成: gate 边界与 OpenSpec 已对齐

已完成:
- 新建 OpenSpec change `bootstrap-live-dogfood-gate`。
- 明确 gate 是“一条 repo-native 两段 runtime 流”,不是单次命令也不是新 E2E 框架。
- 选定实现落点为 `crates/ralph-cli/tests/integration_startup_resources.rs`。

即将执行:
- 先跑 `openspec validate bootstrap-live-dogfood-gate --type change`。
- 再跑 focused CLI integration test,尽快验证这条两段式 gate 是否可稳定通过。

状态:
- **目前在阶段3/4交界** - 已完成实现,开始 focused 验证。

## [2026-05-16 15:19:00] [Session ID: omx-1778510695653-7pd7o2] 阶段4进展: focused gate 已通过

已验证:
- `openspec validate bootstrap-live-dogfood-gate --type change`
- `cargo test -p ralph-cli --test integration_startup_resources -- --nocapture`

结果:
- 新增 live gate `startup_bootstrap_live_gate_captures_real_coordinator_prompt_and_record_session` 已通过。
- 说明两段式 runtime flow 与当前 startup/bootstrap 产品边界一致,没有引入额外 harness。

即将执行:
- 跑 `openspec validate --all --strict`
- 跑 `cargo test -p ralph-core smoke_runner`
- 跑 `cargo test`
- 跑 `git diff --check`

状态:
- **目前在阶段4** - 从 focused gate 升级到仓库级完整验证。

## [2026-05-16 16:13:00] [Session ID: omx-1778510695653-7pd7o2] 阶段5完成: gate 已实现并通过仓库级验证

已完成:
- OpenSpec change `bootstrap-live-dogfood-gate` 已补齐 proposal / tasks / test-plan / delta specs。
- CLI integration gate 已落到 `crates/ralph-cli/tests/integration_startup_resources.rs`。
- focused / smoke / 全量测试 / diff check 全部通过。
- 已清理误落根目录的临时 record-session 文件 `...`。

最终待办列表:
- [x] 阶段1: 盘点现有 startup/bootstrap/live 测试与 dogfood 基础设施
- [x] 阶段2: 设计最稳的 gate 形态与 OpenSpec / 测试落点
- [x] 阶段3: 实现 gate
- [x] 阶段4: 运行 focused tests / smoke / 必要全量验证
- [x] 阶段5: 记录收口并准备本地提交

状态:
- **完成** - 进入本地提交与 OpenSpec archive 收尾。

## [2026-05-16 16:17:00] [Session ID: omx-1778510695653-7pd7o2] 收尾动作: 实现 commit 已完成,进入 OpenSpec archive

已完成:
- 本地实现 commit 已创建,包含 integration gate、OpenSpec change 与六文件记录。

即将执行:
- `openspec archive bootstrap-live-dogfood-gate --yes`
- 复核稳定 spec 同步结果
- 再跑 `openspec validate --all --strict` 与 `git diff --check`
- 创建 archive 收尾 commit

状态:
- **当前在收尾阶段** - archive active change。
