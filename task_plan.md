# 任务计划: 方向B.3 - structured failure classes for parent branching policy

## [2026-05-16 23:14:00] [Session ID: omx-1778510695653-7pd7o2] 续档启动: capability B.3 收口

目标:
- 继续 capability invocation failure 演进线,把 `capability.failed` 从“只有错误文本”推进到“带结构化失败类型”。
- 证明 parent run 可以依据结构化 `failure_class` 做分支决策,而不是解析自由文本 `error`。
- 保持边界: child/micro-run 仍隔离执行, parent topology 不热改,最终 human-facing answer 仍必须显式发布 `reply.human.message`。

续档原因:
- 原 `task_plan.md` 已达到 1120 行,超过六文件规则的 1000 行阈值。
- 已将旧计划快照保存到 `archive/default_history/task_plan_2026-05-16_2314_capability_b3_prev.md`。
- 本文件只承接当前 B.3 收口,避免旧上下文继续污染注意力窗口。

六文件摘要:
- `task_plan.md` 旧档最新主线是 B.1/B.2/B.3: 从 answer evidence inspect UX,到 explicit human reply,到 multi-step capability orchestration,再到 failure fallback 和 structured failure class。
- `notes.md` 已记录 B.1/B.2 的边界: `capability.result` / `capability.failed` 是 parent-consumable runtime event, `reply.human.message` 才是对人的最终输出。
- `WORKLOG.md` 已记录 B.1/B.2 的完成证据和 OpenSpec archive 结果。
- `LATER_PLANS.md` 当前没有直接阻塞 B.3 的待办; 与 startup/bootstrap 相关的事项仍是后续线。
- `ERRORFIX.md` / `EPIPHANY_LOG.md` 没有新增会改变 B.3 当前实现方向的约束。

阶段计划:
- [x] 阶段1: 盘点现有 failure payload 与最适合结构化的失败分类
- [x] 阶段2: 写窄 OpenSpec change,定义 `failure_class` 与 parent branching policy 的最小边界
- [x] 阶段3: 实现结构化 failure class,并让现有 fallback gate 按 class 断言
- [x] 阶段4: 补 focused gate / unit tests / stable spec sync / archive
- [x] 阶段5: 收口验证、归档与本地提交

当前已知现场:
- 已新增 `CapabilityFailureClass` 与 `failure_class` 字段。
- 已补 core/cli 构造点和 focused/unit/integration 断言。
- OpenSpec change `capability-failure-class-branching-policy` 已存在且 `openspec validate --all --strict` 曾通过。
- 上一轮 `cargo test` 未看到最终完成输出; 当前进程检查没有 Ralph 仓库的 `cargo test` 在跑,所以必须重新跑 fresh gates。

即将执行:
- review 当前 diff,确认实现与 OpenSpec change 一致。
- 继续 OpenSpec archive,并复跑 focused gates、smoke 和 `cargo test`。
- 将 B.3 的 notes / WORKLOG / task_plan 收口后做本地 commit,不 push。

状态:
- **目前在阶段4** - 先 review diff 与 OpenSpec 任务状态,再 archive 和验证。

## [2026-05-16 23:16:00] [Session ID: omx-1778510695653-7pd7o2] 阶段动作: B.3 archive 前验证

即将执行:
- 先跑 change-level OpenSpec validate 与 focused tests,确认 `failure_class` 的静态 contract 和动态 gate 都成立。
- 再跑 `cargo fmt --all -- --check`、`git diff --check` 与 `cargo test`,作为 archive 前 repo-wide gate。

状态:
- **目前在阶段4** - 进入 archive 前验证。

## [2026-05-16 23:31:00] [Session ID: omx-1778510695653-7pd7o2] 阶段完成: B.3 archive 与验证已完成

已完成:
- `CapabilityFailureClass` 已进入 core runtime capability records。
- parent-visible `capability.failed` 与 child/micro-run `failed.json` 都带 `failure_class`。
- live fallback gate 已证明 parent 可以看见 `invalid_capability_id` 并继续 fallback。
- OpenSpec change 已 archive,稳定 spec 已同步。
- `notes.md`、`WORKLOG.md`、`EXPERIENCE.md` 已补记录。

验证证据:
- `openspec validate capability-failure-class-branching-policy --type change`
- `cargo test -p ralph-core capability::tests -- --nocapture`
- `cargo test -p ralph-cli capability::tests -- --nocapture`
- `cargo test -p ralph-cli --test integration_live_capability parallel_parent_run_can_fallback_after_capability_failed_before_final_human_reply`
- `cargo test -p ralph-cli --test integration_live_capability`
- `cargo test -p ralph-core smoke_runner`
- `cargo fmt --all -- --check`
- `git diff --check`
- `openspec validate --all --strict`
- `cargo test`
- archive 后复跑: `openspec validate --all --strict`, `git diff --check`, `cargo test -p ralph-cli --test integration_live_capability`, `cargo test -p ralph-core smoke_runner`, `cargo test`

待办更新:
- [x] 阶段1: 盘点现有 failure payload 与最适合结构化的失败分类
- [x] 阶段2: 写窄 OpenSpec change,定义 `failure_class` 与 parent branching policy 的最小边界
- [x] 阶段3: 实现结构化 failure class,并让现有 fallback gate 按 class 断言
- [x] 阶段4: 补 focused gate / unit tests / stable spec sync / archive
- [x] 阶段5: 收口验证、归档与本地提交

状态:
- **当前任务已完成实现与验证** - 下一步执行本地 commit,不 push。

## [2026-05-16 23:36:00] [Session ID: omx-1778510695653-7pd7o2] 新任务启动: B.4 - richer branching policy without retry engine

目标:
- 继续 B.3 的 `failure_class` 结构化分支输入。
- 证明 parent run 可以根据不同 `failure_class` 做不同后续策略,但不引入通用 retry engine、planner 或 live topology mutation。
- 保持边界: `capability.failed` 是 parent-consumable runtime event,最终 human-facing answer 仍必须显式发 `reply.human.message`。

阶段计划:
- [x] 阶段1: 盘点现有 failure classes、integration gate 与最小 branching matrix 缺口
- [x] 阶段2: 写窄 OpenSpec change,定义 B.4 richer branching policy 的最小边界和非目标
- [x] 阶段3: 设计 focused dogfood gate,优先复用 `integration_live_capability.rs`
- [x] 阶段4: 若实现面仍然窄,进入代码/测试实现
- [x] 阶段5: archive OpenSpec,跑 focused/smoke/full gates,本地提交

当前假设:
- B.4 最小价值不是实现一个 retry framework,而是固定 parent policy 可以区分至少两类失败并走不同策略。
- 候选矩阵:
  - `invalid_capability_id` -> fallback capability request
  - `child_run_failed` -> degraded explicit human reply,不再重复调用
- 备选解释:
  - 如果现有 prompt/event replay 不能稳定把 `child_run_failed` 暴露给 parent 后续 turn,则 B.4 应先修 evidence/prompt replay,不要硬写 branching gate。

推翻当前假设的证据:
- 如果 child/micro-run failure 没有进入 parent-visible `capability.failed`,或者后续 turn 看不到 `failure_class`,则当前 branching matrix 不成立。
- 如果需要新增通用 retry engine 才能证明 B.4,说明 scope 已经太大,应退回 OpenSpec 重新切分。

状态:
- **目前在阶段1** - 先做只读勘查,确认最小 branching matrix 是否可用现有 runtime 证明。

## [2026-05-16 23:42:00] [Session ID: omx-1778510695653-7pd7o2] 阶段推进: B.4 最小 branching matrix 已确定

勘查结论:
- `invalid_capability_id` 已有 live fallback gate,可以作为矩阵中的“可恢复 fallback”分支。
- `malformed_request` 在 core supervisor 中已经是 parent-visible `capability.failed`,可以用 live gate 证明 parent 看到 class 后选择 diagnostic `reply.human.message`,而不是重试。
- `child_run_failed` 当前有 artifact/unit 覆盖,但 live dry-run 子流程默认成功。为了避免引入测试专用 runtime 开关,B.4 暂不把它作为 live matrix 的必需分支。

决策:
- B.4 新增一条 `malformed_request -> diagnostic human reply, no retry` focused gate。
- B.4 不新增 retry engine、planner、child failure injection 开关或 topology mutation。

状态:
- **阶段1完成,进入阶段2** - 写窄 OpenSpec change。

## [2026-05-16 23:47:00] [Session ID: omx-1778510695653-7pd7o2] 阶段动作: B.4 focused gate 实现后验证

即将执行:
- 先跑新增的 malformed-request diagnostic live gate。
- 如果通过,再跑整组 `integration_live_capability` 和 OpenSpec 全量校验。

状态:
- **目前在阶段4** - 用最小动态证据验证 B.4 branching matrix。

## [2026-05-16 23:50:00] [Session ID: omx-1778510695653-7pd7o2] 遇到错误: B.4 fmt check 失败

现象:
- `cargo test -p ralph-cli --test integration_live_capability` 通过,5 passed。
- `openspec validate --all --strict` 通过,27 passed。
- `cargo fmt --all -- --check` 失败,指出新增 B.4 测试函数签名和长 assert 需要格式化。

处理:
- 运行 `cargo fmt --all`。
- 重新跑 focused/live/OpenSpec/fmt/diff check,确认格式化后无回归。

状态:
- **目前在阶段4** - 修复格式化门禁失败。

## [2026-05-16 23:55:00] [Session ID: omx-1778510695653-7pd7o2] 阶段推进: B.4 archive 前验证通过

已验证:
- 新增 malformed diagnostic gate 通过。
- 整组 `integration_live_capability` 通过,包含既有 invalid fallback 分支和新增 malformed no-retry diagnostic 分支。
- `openspec validate --all --strict` 通过。
- `cargo fmt --all -- --check` 通过。
- `git diff --check` 通过。
- `cargo test -p ralph-core smoke_runner` 通过。
- `cargo test` 通过。

状态:
- **阶段4完成,进入阶段5** - archive OpenSpec change 并做最终提交。

## [2026-05-17 00:05:00] [Session ID: omx-1778510695653-7pd7o2] 阶段完成: B.4 richer branching policy 已收口

已完成:
- OpenSpec change `capability-failure-branching-matrix` 已 archive。
- 稳定 spec 已同步 class-specific failure branching requirements。
- 新增 malformed-request diagnostic no-retry live gate。
- 既有 invalid capability id fallback gate 仍通过。
- `notes.md`、`WORKLOG.md`、`EXPERIENCE.md` 已补记录。

验证证据:
- `openspec validate capability-failure-branching-matrix --type change`
- `cargo test -p ralph-cli --test integration_live_capability parallel_parent_run_can_emit_diagnostic_reply_for_malformed_capability_request_without_retry -- --nocapture`
- `cargo test -p ralph-cli --test integration_live_capability parallel_parent_run_can_emit_diagnostic_reply_for_malformed_capability_request_without_retry`
- `cargo test -p ralph-cli --test integration_live_capability`
- `openspec validate --all --strict`
- `cargo fmt --all -- --check`
- `git diff --check`
- `cargo test -p ralph-core smoke_runner`
- `cargo test`
- archive 后复跑: `openspec validate --all --strict`, `cargo test -p ralph-cli --test integration_live_capability`, `cargo test -p ralph-core smoke_runner`, `cargo test`

待办更新:
- [x] 阶段1: 盘点现有 failure classes、integration gate 与最小 branching matrix 缺口
- [x] 阶段2: 写窄 OpenSpec change,定义 B.4 richer branching policy 的最小边界和非目标
- [x] 阶段3: 设计 focused dogfood gate,优先复用 `integration_live_capability.rs`
- [x] 阶段4: 若实现面仍然窄,进入代码/测试实现
- [x] 阶段5: archive OpenSpec,跑 focused/smoke/full gates,本地提交

状态:
- **当前任务已完成实现与验证** - 下一步执行本地 commit,不 push。

## [2026-05-17 00:37:29] [Session ID: omx-1778510695653-7pd7o2] 阶段修正: startup bootstrap 复核记录补正

说明:
- 上一条 `task_plan.md` 追加记录用了未加引号 heredoc,其中 Markdown 反引号被 shell 当作命令替换执行,导致文件路径文本被吞掉。
- 本条记录使用单引号 heredoc 补正,以后包含反引号的 Markdown 追加必须使用 `cat <<'EOF'`。

复核目标:
- 解释 `/Users/cuiluming/local_doc/l_dev/my/rust/rustdog/.ralph/resolved-config.yml` 为什么没有等同于 `/Users/cuiluming/local_doc/l_dev/my/rust/ralph-orchestrator/ralph.yml`。

fresh evidence:
- rustdog 根目录不存在 `ralph.yml`。
- rustdog `.ralph/bootstrap-selection.json` 的 reason 是 `missing default ralph.yml and no explicit prompt; selected v1 rule default`。
- rustdog `.ralph/bootstrap-selection.json` 的 selected resources 是 `workflow:feature-minimal` 和 `prompt:bootstrap-default-task`。
- `crates/ralph-cli/src/startup_resources.rs` 当前 `DEFAULT_BOOTSTRAP_WORKFLOW_ID` 是 `workflow:feature-minimal`。
- `crates/ralph-cli/presets/feature-minimal.yml` 的 `cli.backend` 是 `claude`。
- 项目根 `ralph.yml` 的 `cli.backend` 是 `custom`, `command` 是 `codex`, `args` 包含 `exec --sandbox danger-full-access`。

结论:
- 当前行为不是读取错了 rustdog 的配置,而是 startup bootstrap 的 canonical source 仍然是旧 `feature-minimal` preset。
- 如果产品契约要求无配置启动 artifact 等同项目根 `ralph.yml`,那当前实现存在默认资源源头漂移。

状态:
- **调查结论已形成** - 下一步可开 OpenSpec 修复默认 bootstrap resource contract。


## [2026-05-17 00:44:57] [Session ID: omx-1778510695653-7pd7o2] 阶段完成: startup bootstrap 默认配置漂移调查

补充规格证据:
- `openspec/specs/resource-bootstrap/spec.md` 当前要求 Ralph 在无 `ralph.yml` / 无 `PROMPT.md` 时进入 startup resource resolution。
- 同一 spec 当前要求 `.ralph/resolved-config.yml` 必须写出,并且必须包含 `parallel.enabled=true`。
- 现有 spec 没有要求 `.ralph/resolved-config.yml` 与项目根 `ralph.yml` 等同。

验证命令:
- `cargo test -p ralph-cli startup_resources -- --nocapture`
- `cargo test -p ralph-cli --test integration_startup_resources -- --nocapture`

验证结果:
- `startup_resources`: 8 passed。
- `integration_startup_resources`: 3 passed。

结论:
- 当前不一致是 spec 与实现共同造成的默认资源契约漂移,不是 rustdog 本地配置读取错误。
- 如果要实现用户期望,需要新 OpenSpec 明确 canonical default bootstrap config contract,并修改 selector 与测试断言。

状态:
- **调查与验证已完成** - 等待进入 OpenSpec 修复实现。

## [2026-05-17 10:27:57] [Session ID: omx-1778510695653-7pd7o2] 阶段动作: 开启 OpenSpec change canonical-default-bootstrap-config

目标:
- 把 no-config startup bootstrap 的默认 resolved config 对齐 canonical default config。
- 只做 OpenSpec change,先不改代码。

计划:
1. 创建 `openspec/changes/canonical-default-bootstrap-config/`。
2. 读取 change status 和 first ready artifact instructions。
3. 先产出 OpenSpec 和测试计划,不进入实现。

状态:
- **目前在阶段1** - 创建 change scaffold 并读取首个 artifact 模板。

## [2026-05-17 10:30:04] [Session ID: omx-1778510695653-7pd7o2] 阶段动作: 继续 canonical-default-bootstrap-config OpenSpec 草案

目标:
- 为 `canonical-default-bootstrap-config` 产出 proposal / design / specs / tasks 草案。
- 保持边界: 只写 OpenSpec 与测试计划,不改业务代码。

即将执行:
1. 查看 change scaffold 当前文件。
2. 起草 proposal,明确 capability 归属与影响面。
3. 继续写 design / spec delta / tasks,形成 apply-ready 草案。

状态:
- **目前在阶段2** - 编写 OpenSpec artifact 草案。

## [2026-05-17 10:32:05] [Session ID: omx-1778510695653-7pd7o2] 阶段完成: canonical-default-bootstrap-config OpenSpec 已 apply-ready

已完成:
- 创建 `openspec/changes/canonical-default-bootstrap-config/`。
- 写完 `proposal.md`、`design.md`、`specs/resource-bootstrap/spec.md`、`tasks.md`。
- change 级校验通过,artifact 状态为 4/4 complete。

验证证据:
- `openspec validate canonical-default-bootstrap-config --type change`
- `openspec status --change canonical-default-bootstrap-config`

状态:
- **当前 OpenSpec 草案已完成** - 下一步若继续,应进入 review 或实现前确认,而不是直接跳过 spec。

## [2026-05-17 10:39:06] [Session ID: omx-1778510695653-7pd7o2] 阶段动作: 按 review 收紧 canonical-default-bootstrap-config 草案

已修改:
- stable spec 从“值级硬编码”收紧为“字段级 contract”。
- 具体默认值下沉到 focused / live / drift gate。
- tasks 新增 repo-owned drift gate 任务。

即将执行:
- 重新跑 change 级 OpenSpec 校验。
- 检查 artifact 状态是否仍为 4/4 complete。

状态:
- **目前在阶段2** - review 后草案收口与复验。

## [2026-05-17 11:08:00] [Session ID: omx-1778510695653-7pd7o2] 阶段动作: 继续 canonical-default-bootstrap-config 实现收口

目标:
- 从已实现状态继续,先复核 OpenSpec apply 状态和当前 diff,再跑完整验证 gate。
- 保持边界: 不触碰 `.serena/project.yml` 这类非本任务改动,不 push。

即将执行:
1. 读取 OpenSpec status / apply instructions,确认 change 名与任务完成度。
2. review `startup_resources.rs`、`integration_startup_resources.rs` 和 OpenSpec artifact diff。
3. 运行完整 gate: `openspec validate --all --strict`、smoke、full cargo test 等。
4. 验证通过后补 WORKLOG / task_plan,再判断是否进入 archive。

状态:
- **目前在阶段3/4** - 实现已在工作区,正在做复核与验证收口。

## [2026-05-17 11:18:00] [Session ID: omx-1778510695653-7pd7o2] 阶段推进: canonical-default-bootstrap-config archive 前验证通过

已验证:
- `cargo fmt --all -- --check` 通过。
- `openspec validate --all --strict` 通过,27 passed。
- `git diff --check` 通过。
- `cargo test -p ralph-cli startup_resources -- --nocapture` 通过,8 passed。
- `cargo test -p ralph-cli --test integration_startup_resources -- --nocapture` 通过,3 passed。
- `cargo test -p ralph-core smoke_runner` 通过,12 passed。
- `cargo test` 通过。

即将执行:
- 归档 OpenSpec change `canonical-default-bootstrap-config`。
- 归档后复跑 `openspec validate --all --strict`、diff check、focused startup gate、smoke 和必要 full test。

状态:
- **阶段4完成,进入阶段5** - archive OpenSpec change 并做最终验证。

## [2026-05-17 15:12:00] [Session ID: omx-1778510695653-7pd7o2] 遇到错误: canonical-default-bootstrap-config archive 自动同步失败

现象:
- `openspec archive canonical-default-bootstrap-config --yes` 失败。
- 输出: `resource-bootstrap MODIFIED failed for header "### Requirement: Default startup bootstrap MUST resolve to canonical default parallel mode" - not found`。
- OpenSpec 明确提示 `Aborted. No files were changed.`。

当前假设:
- delta spec 使用了新的 Requirement 标题,但稳定 spec `openspec/specs/resource-bootstrap/spec.md` 里还没有同名 Requirement。
- OpenSpec archive 的 MODIFIED 合并需要标题已经存在,因此需要先同步稳定 spec 标题/内容,或调整 delta 到旧标题。

即将执行:
1. 读取稳定 `resource-bootstrap` spec,确认旧 Requirement 标题。
2. 用最小 spec 同步修复标题/内容不一致。
3. 重新运行 OpenSpec validate 和 archive。

状态:
- **阶段5阻塞于 OpenSpec archive 同步** - 正在修复 stable spec / delta spec 对齐问题。

## [2026-05-17 15:25:00] [Session ID: omx-1778510695653-7pd7o2] 阶段完成: canonical-default-bootstrap-config 已实现并归档

已完成:
- 默认 no-config bootstrap workflow 已从 `workflow:feature-minimal` 切换到 `workflow:default-parallel`。
- canonical default bootstrap config 使用仓库根 `ralph.yml` 编译期嵌入。
- legacy `feature-minimal` 保留可物化,但不再参与默认 selector。
- startup focused tests 和 integration live gate 已覆盖 canonical selector、resolved config、root/materialized/resolved drift gate。
- OpenSpec change 已 archive 到 `openspec/changes/archive/2026-05-17-canonical-default-bootstrap-config/`。
- 稳定 spec `openspec/specs/resource-bootstrap/spec.md` 已同步 canonical source contract。

验证证据:
- archive 前 `openspec validate --all --strict`: 27 passed。
- archive 前 `cargo test -p ralph-cli startup_resources -- --nocapture`: 8 passed。
- archive 前 `cargo test -p ralph-cli --test integration_startup_resources -- --nocapture`: 3 passed。
- archive 前 `cargo test -p ralph-core smoke_runner`: 12 passed。
- archive 前 `cargo test`: passed。
- archive 后 `openspec validate --all --strict`: 26 passed。
- archive 后 `cargo fmt --all -- --check`: passed。
- archive 后 `git diff --check`: passed。
- archive 后 `cargo test -p ralph-cli startup_resources -- --nocapture`: 8 passed。
- archive 后 `cargo test -p ralph-cli --test integration_startup_resources -- --nocapture`: 3 passed。
- archive 后 `cargo test -p ralph-core smoke_runner`: 12 passed。
- archive 后 `cargo test`: passed。

待办更新:
- [x] 阶段1: review OpenSpec 草案和稳定 spec 匹配关系
- [x] 阶段2: 落地 canonical default bootstrap resource 与 selector 切换
- [x] 阶段3: 强化 focused/live/drift gate
- [x] 阶段4: 跑 archive 前完整验证
- [x] 阶段5: archive OpenSpec change 并跑 archive 后验证

状态:
- **当前任务已完成实现、归档和验证** - 工作区仍有未提交改动,其中 `.serena/project.yml` 是既有非本任务改动,未触碰。

## [2026-05-17 15:34:00] [Session ID: omx-1778510695653-7pd7o2] 阶段动作: 移除 `.serena` 并创建本地 commit

目标:
- 按用户明确指令移除 `.serena`。
- 将 canonical default bootstrap config 的实现、OpenSpec archive、上下文记录和 `.serena` 删除一起做一次本地 commit。
- 不 push。

即将执行:
1. 检查 `.serena` 在 git 中是否 tracked,避免误判删除范围。
2. 删除 `.serena` 目录并检查 diff。
3. 运行 `git diff --check` 和必要状态检查。
4. 使用 Lore Commit Protocol 生成安全 commit message file,再执行本地 commit。

状态:
- **目前在提交收口阶段** - 先确认 `.serena` 的 tracked/untracked 状态。

## [2026-05-17 15:28:00] [Session ID: omx-1778510695653-7pd7o2] 遇到错误: 首次 commit 包含 archive design trailing whitespace

现象:
- `git diff --cached --check` 报出 archive design 的 3 处 trailing whitespace。
- 因提交命令未设置 `set -e`,commit `d7babbd1` 仍然成功生成。

处理:
- 立即修复 archive design 行尾空白。
- 补充 ERRORFIX 记录。
- 重新运行 diff check / OpenSpec validate。
- amend 刚才的本地 commit,避免保留脏提交。

状态:
- **提交收口阶段出现可修复门禁问题** - 正在 amend 本地 commit。

## [2026-05-17 15:58:00] [Session ID: omx-1779004640353-blcixq] 新任务启动: TUI 与 Codex 直接输出差异排查

目标:
- 对照当前 Ralph TUI 输出路径和 Codex 直接输出路径,找出哪些信息被隐藏、压缩、改写或没有展示。
- 给出“不遗漏信息”与“能看见当前正在做什么/状态是什么”的改良方案。
- 本轮先以只读调查为主,除非证据明确且用户确认,先不改 runtime 行为。

阶段计划:
- [ ] 阶段1: 读取项目长期经验、相关 skill 与当前 TUI/输出代码路径。
- [ ] 阶段2: 对照 Codex 直接输出信息种类与 Ralph TUI 当前展示模型。
- [ ] 阶段3: 整理遗漏清单、状态字段缺口和最小改良方案。
- [ ] 阶段4: 如需要,补一个可验证的设计/测试建议,但本轮不擅自落代码。

现象:
- 用户观察到 TUI 输出和 Codex 直接输出差别很大。
- 用户怀疑“一些东西没有显示”。
- 用户希望同时满足: 不遗漏信息,知道当前进行什么,状态是什么。

当前假设:
- 候选假设A: TUI 只展示经过 summary/filter 的 event 或 agent state,没有展示完整 stdout/stderr/token/tool call 流。
- 候选假设B: 直接 Codex 输出包含 reasoning/status/tool/progress/error 等流式片段,但 Ralph TUI 只映射了部分 topic 或只显示最后消息。
- 最强备选解释: 信息并非丢失,而是落在 record-session、diagnostics、events.jsonl 或 agents snapshot 中,只是 TUI 没有提供同屏可见入口。

验证计划:
- 先用代码路径和已有测试确认 TUI 数据源、渲染字段、过滤规则。
- 再查 record-session/diagnostics/events/agent snapshot 是否保留了 TUI 未展示的信息。
- 如果只有静态证据,结论标注为“候选缺口”; 如果找到测试或样例,再标注为“已验证缺口”。

状态:
- **目前在阶段1** - 开始只读排查相关代码和历史经验。

## [2026-05-17 16:16:00] [Session ID: omx-1779004640353-blcixq] 阶段推进: TUI 差异排查结论已形成

已完成:
- [x] 阶段1: 读取项目长期经验、相关 skill 与当前 TUI/输出代码路径。
- [x] 阶段2: 对照 Codex/CLI 直接输出信息种类与 Ralph TUI 当前展示模型。
- [x] 阶段3: 整理遗漏清单、状态字段缺口和最小改良方案。
- [ ] 阶段4: 如需要,补一个可验证的设计/测试建议,但本轮不擅自落代码。

关键结论:
- TUI 当前不是 stdout 等价镜像,而是一个筛选后的操作面。
- CLI/log-mode 的审计信息更全,包含 per-instance/job 标签、state 行、final states。
- TUI 目前最缺的是“当前进行什么”的 richer status: last_input、current job、last event、stderr 可见性、record-session/evidence 路径。
- `Rendered` 模式为了阅读会隐藏部分原始控制符/event wrapper; 排障时需要 Plain/raw 视图。

验证证据:
- `cargo test --package ralph-cli --bin ralph parallel_runner::tests::parallel_tui_event_forwarding_filters_noise_without_source_or_instance -- --exact`: 1 passed。
- `cargo test --package ralph-cli --bin ralph parallel_runner::tests::parallel_tui_event_forwarding_allows_events_with_source_instance -- --exact`: 1 passed。
- `cargo test --package ralph-cli --bin ralph parallel_runner::guardrail_tests::parallel_output_for_event_parsing_is_stdout_only -- --exact`: 1 passed。
- `cargo test --package ralph-adapters --lib stream_handler::tests::tui_stream_handler::markdown_rendered_mode_shows_reply_human_message_payload -- --exact`: 1 passed。
- `cargo test --package ralph-adapters --lib stream_handler::tests::tui_stream_handler::pretty_plain_mode_keeps_reply_human_message_event_text -- --exact`: 1 passed。

遇到错误:
- 曾误跑 `cargo test --package ralph-cli --lib ... -- --exact`,结果为 0 tests。已回滚证据口径,改用 `--bin ralph` 重跑。

状态:
- **目前在阶段4** - 本轮只交付排查结论和改良建议,不擅自改代码。

## [2026-05-17 16:18:10] [Session ID: omx-1779004640353-blcixq] 行动记录: 复核 TUI 输出差异证据并收口建议

目的:
- 复核上一轮关于 TUI 与 Codex/CLI 直接输出差异的静态证据和 focused test 证据。
- 形成可执行的设计建议,说明如何做到不遗漏信息,同时让用户看见当前进行什么、状态是什么。

即将执行:
- 读取关键代码片段,确认输出过滤、stderr 可见性、Rendered/Plain 展示层改写和 agents snapshot 状态字段。
- 运行上一轮列出的 focused tests,用当前会话重新建立动态证据。
- 若没有代码改动,不跑全量 cargo test,只交付排查结论和建议。

状态:
- **目前在阶段4** - 复核证据,准备最终交付。


## [2026-05-17 16:24:01] [Session ID: omx-1779004640353-blcixq] 阶段完成: TUI 差异排查收口

已完成:
- [x] 阶段1: 读取项目长期经验、相关 skill 与当前 TUI/输出代码路径。
- [x] 阶段2: 对照 Codex/CLI 直接输出信息种类与 Ralph TUI 当前展示模型。
- [x] 阶段3: 整理遗漏清单、状态字段缺口和最小改良方案。
- [x] 阶段4: 补充可验证的设计/测试建议,本轮未改 runtime 代码。

当前结论:
- TUI 与 Codex/CLI 直出的差异来自“展示模型不同”: 直出是审计流,TUI 是筛选后的操作面。
- 已通过 focused tests 复核事件过滤、stdout-only 事件解析和 Rendered/Plain 差异。
- 推荐下一步按“状态摘要 + raw/audit 视图”改良 TUI,而不是简单复制 stdout 全量文本。

状态:
- **阶段4已完成** - 可以向用户交付排查结论、证据和后续实现建议。


## [2026-05-17 16:26:24] [Session ID: omx-1779004640353-blcixq] 追加验证: hook 要求的新鲜证据

目的:
- 回应 stop hook 的要求,继续补一组新鲜验证证据后再结束。
- 用更贴近 TUI 状态面的测试,确认当前结论没有被新证据推翻。

即将执行:
- 复查 Instances / header / snapshot 相关测试,优先验证“当前进行什么”和“状态是什么”的展示边界。
- 如发现新缺口,再补充 notes 记录;如没有新缺口,直接收尾。

状态:
- **继续进行中** - 正在补新鲜验证证据。


## [2026-05-17 16:29:49] [Session ID: omx-1779004640353-blcixq] 追加验证完成: hook 后新证据已收集

已完成:
- [x] 复查 ultrawork 收尾规则。
- [x] 运行 agents 状态面集成测试。
- [x] 运行并行 TUI full layout snapshot smoke。
- [x] 将新证据写入 notes 和 WORKLOG。

验证结果:
- cargo test --package ralph-cli --test integration_agents test_agents_command_prints_table -- --exact: passed。
- cargo test --package ralph-tui --test integration_snapshots test_parallel_full_layout_renders_instances_output_and_gates -- --exact: passed。

状态:
- **追加验证已完成** - 可以收尾交付。


## [2026-05-17 16:32:58] [Session ID: omx-1779004640353-blcixq] 新阶段启动: TUI 状态增强最小实现

目标:
- 从当前 ultrawork planning 状态继续,把上一轮已经验证的缺口推进到一个最小代码改良。
- 优先让 TUI 主画面能回答“当前在做什么、状态是什么”,同时避免把 TUI 退化成全量 stdout 镜像。

阶段计划:
- [ ] 阶段1: 读取现有 TUI state/widget 结构,确认已有状态真相源。
- [ ] 阶段2: 设计最小展示改动,优先复用已有状态字段。
- [ ] 阶段3: 修改现有 widget/state,补充测试。
- [ ] 阶段4: 运行 focused tests 与必要 smoke,记录证据。

约束:
- 不改事件解析语义。
- 不新增第二套状态真相源。
- 不触碰用户已有的无关改动,当前已知 `PROMPT.md` 删除不是本轮产生。

遇到错误:
- 首次读取代码时误填工作目录 `/Users/cuiluming/local_doc/l_dev/my/rust/ralph/ralph-orchestrator`,命令因目录不存在失败。已纠正为当前仓库目录。

状态:
- **目前在阶段1** - 读取 TUI state/widget 结构。


## [2026-05-17 16:40:39] [Session ID: omx-1779004640353-blcixq] 阶段推进: TUI 状态增强最小实现 - 结构确认完成

已完成:
- [x] 阶段1: 读取现有 TUI state/widget 结构,确认已有状态真相源。

当前发现:
- `ParallelTuiState` 已有 `selected_instance`、`current_job`、`output_render_mode`、`last_event` 等可复用状态。
- `InstanceViewState` 已有 `state`、`last_output_at`、`jobs`、`current_job`,不需要新增第二套状态源。
- `Output` 标题已经显示 selected instance 的 state/job,但 Instances 列表和 Footer 还没有把 job / last event / render mode 讲清楚。

状态:
- **目前在阶段2** - 设计最小展示改动,优先复用已有状态字段。


## [2026-05-17 16:43:52] [Session ID: omx-1779004640353-blcixq] 阶段推进: TUI 状态增强代码已修改

已完成:
- [x] 阶段2: 设计最小展示改动,优先复用已有状态字段。
- [x] 阶段3: 修改现有 widget/state,补充测试。

代码变更:
- `InstanceViewState` 增加 `current_job_summary()` helper。
- `InstancesPane` 在实例行中显示 `job x/y`。
- `Footer` 在并行模式下显示 selected instance、state、job summary、last event、render mode。
- 新增 Instances 和 Footer 的 focused tests,锁定状态摘要可见性。

遇到错误:
- 查询状态结构时误把不存在的 `crates/ralph-tui/src/state/mod.rs` 放进 `rg` 参数,命令返回错误码 2。已改用真实文件 `state.rs`。
- 第一次 footer patch 的测试上下文不匹配,`apply_patch` 未应用。已重新读取 footer 真实片段并按真实位置补丁成功。

状态:
- **目前在阶段4** - 运行格式化和 focused tests。


## [2026-05-17 16:51:58] [Session ID: omx-1779004640353-blcixq] 阶段完成: TUI 状态增强最小实现

已完成:
- [x] 阶段1: 读取现有 TUI state/widget 结构,确认已有状态真相源。
- [x] 阶段2: 设计最小展示改动,优先复用已有状态字段。
- [x] 阶段3: 修改现有 widget/state,补充测试。
- [x] 阶段4: 运行 focused tests、package tests、全量 tests 与 diff check。

验证结果:
- `cargo fmt --all -- --check`: passed。
- `cargo test --package ralph-tui --lib widgets::instances::tests::instances_pane_shows_current_job_summary -- --exact`: passed。
- `cargo test --package ralph-tui --lib widgets::footer::tests::footer_shows_parallel_status_summary -- --exact`: passed。
- `cargo test --package ralph-tui --test integration_snapshots test_parallel_full_layout_renders_instances_output_and_gates -- --exact`: passed。
- `cargo test --package ralph-tui`: passed。
- `cargo test`: passed。
- `git diff --check`: passed。

遇到错误:
- `cargo fmt --all -- --check` 首次失败,因为 rustfmt 要压缩 `spans.extend` 数组写法。已运行 `cargo fmt --all` 修复。
- Footer 初版 verbose 摘要在 80 列下截断 last event。已改成紧凑摘要并通过测试。

状态:
- **TUI 状态增强最小实现已完成** - 可以收尾交付。


## [2026-05-17 16:55:40] [Session ID: omx-1779004640353-blcixq] 维护记录: notes.md 超限续档与持续学习

触发原因:
- `notes.md` 达到 1166 行,超过 1000 行续档阈值。

已完成:
- [x] 回读默认六文件最新段落。
- [x] 将旧 `notes.md` 移到 `archive/default_history/notes_2026-05-17_1655_tui_status_prev.md`。
- [x] 创建新的 `notes.md` 续档入口。
- [x] 将 TUI 状态摘要经验沉淀到 `EXPERIENCE.md`。
- [x] 创建 archive manifest: `archive/manifests/ARCHIVE_MANIFEST__default_notes_rollover_2026-05-17_1655.md`。

状态:
- **持续学习续档已完成** - 回到最终收尾检查。

## [2026-05-17 17:08:00] [Session ID: omx-1779004640353-blcixq] 快速核查: Codex 原生状态行与并行 TUI

目标:
- 回答用户关于 Codex 原生状态行 `Working...` / `Inspecting current code behavior...` 是否会在 `ralph run` 并行 TUI 中显示的问题。

已观察静态路径:
- 普通并行 backend 通过 `BufReader::lines()` 按换行读取 stdout/stderr。
- TUI 默认接收 stderr chunk,除非显式 `--hide-stderr`。
- Codex app-server 路径不显示 Codex 原生 TUI 状态条,而是把 app-server 事件映射为 stdout/stderr chunk。

验证结果:
- `cargo test --package ralph-cli --bin ralph tests::run_args_show_stderr_defaults_to_true -- --exact`: passed。
- `cargo test --package ralph-tui --lib state::parallel::tests::parallel_output_stderr_markdown_rendering_matches_renderer_output -- --exact`: passed。

遇到错误:
- 追加计划时第一次误用未加引号 heredoc,反引号内容被 shell 当命令执行。已确认目标关键字没有写入 `task_plan.md`,并改用 `cat <<'EOF'` 方式追加。
- 一次 `rg` 搜索把换行模式写进普通正则,返回用法错误。该输出不作为证据,已改用简单搜索。

状态:
- **快速核查完成** - 准备给出结论: stderr 行会显示,但 Codex 原生临时状态条不会稳定显示。

## [2026-05-17 17:18:00] [Session ID: omx-1779004640353-blcixq] 新阶段启动: Codex 风格 current activity 状态显示

目标:
- 在 `ralph run` 并行 TUI 中稳定显示类似 Codex 的当前动作状态,例如 `Working (11s • esc to interrupt)` 和 `Inspecting current code behavior (29s • esc to interrupt)`。
- 不依赖解析 Codex 私有 TTY 控制序列作为唯一真相源,而是复用 Ralph 已有并行状态与输出流。

可选方向:
1. 不惜代价最佳方案:
   - 在 runtime 层新增结构化 `current_activity` update/event,由 Codex app-server lifecycle/reasoning event 和普通 backend 输出共同驱动。
   - Footer/Instances/Output title 都从同一字段读取。
   - 优点是语义最稳,缺点是跨 core/cli/tui 改动范围更大。
2. 先能用,后面再优雅:
   - 在 TUI state 内从已存在的 `HatJobOutputChunk` 和 job/state 时间戳派生 activity。
   - 对 Codex app-server 的 reasoning stderr / task_started 生命周期映射成短 activity。
   - 优点是复用已有 TUI update 和 output chunk,影响面小;缺点是普通 backend 的 TTY `\r` 状态仍不能完全捕获。

当前决策:
- 采用折中方案: 先在 TUI state 增加正式 `current_activity` 字段,由现有 chunk/lifecycle 驱动,并把 Codex app-server 的稳定事件映射为 activity 文案。
- 暂不解析 `\r` 原地刷新控制序列作为状态真相源,避免版本漂移和误判。

阶段计划:
- [ ] 阶段1: 确认现有 TUI update / state / widget 路径。
- [ ] 阶段2: 先补 focused tests,锁定 activity 文案可见性。
- [ ] 阶段3: 实现 activity 更新与展示。
- [ ] 阶段4: 跑 focused tests、package tests、必要 full gates。
- [ ] 阶段5: 记录 notes / WORKLOG / 后续建议,收口交付。

状态:
- **目前在阶段1** - 正在读取 TUI update、parallel state 和 Codex app-server 输出路径。

## [2026-05-17 17:38:00] [Session ID: omx-1779004640353-blcixq] 阶段推进: current activity 初版实现完成

已完成:
- [x] 阶段1: 确认现有 TUI update / state / widget 路径。
- [x] 阶段2: 先补 focused tests,锁定 activity 文案可见性。
- [x] 阶段3: 实现 activity 更新与展示。

代码变更:
- `ralph-core` 新增 activity 文本归一化 helper。
- `OutputStream` 新增 `Activity` 变体,作为不参与正文解析的状态信号。
- Codex app-server 在 `task_started` 发出 `Working`,在 reasoning summary 中识别 `Inspecting...` 类状态并发出 activity chunk。
- TUI `InstanceViewState` 新增 `current_activity` 和 `state_since`。
- Footer 显示 `Activity (Ns • Ctrl+C to interrupt)`、实例 id、state、job、render mode、last event。
- Instances 行显示当前 activity 简短摘要。

即将执行:
- 运行 `cargo fmt --all`。
- 运行 core/tui/cli focused tests,修复编译器指出的漏分支或格式问题。

状态:
- **目前在阶段4** - 进入验证与修补。

## [2026-05-17 18:06:00] [Session ID: omx-1779004640353-blcixq] 接续验证: Codex 风格 current activity 状态显示

接手状态:
- 上一轮已经完成 activity 初版实现和 focused tests。
- 按交接摘要,`cargo test -p ralph-cli` 可能仍在运行或刚结束,需要先确认。

即将执行:
- 检查是否存在仍在运行的 `cargo test -p ralph-cli` 进程。
- 若没有可接续进程,重新运行必要验证命令,避免把不完整交接当作已验证结论。
- 通过后继续跑 `cargo fmt --all -- --check`、`cargo test`、`git diff --check`。
- 若失败,按错误文件/行号修复,并记录到 `ERRORFIX.md`。

状态:
- **目前在阶段4** - 接续验证,目标是拿到可复现的完整 gate 证据。

## [2026-05-17 18:18:00] [Session ID: omx-1779004640353-blcixq] 阶段完成: current activity 验证通过

已完成:
- [x] 阶段1: 确认现有 TUI update / state / widget 路径。
- [x] 阶段2: 先补 focused tests,锁定 activity 文案可见性。
- [x] 阶段3: 实现 activity 更新与展示。
- [x] 阶段4: 跑 focused tests、package tests、full gates。
- [x] 阶段5: 记录 notes / WORKLOG / ERRORFIX / LATER_PLANS,收口交付。

验证证据:
- `cargo test -p ralph-cli`: passed, 173 bin unit tests plus CLI integration tests passed。
- `cargo fmt --all -- --check`: passed。
- `cargo test`: passed, 包含 `ralph-tui` widget/state tests 与 `integration_snapshots`。
- `git diff --check`: passed。

状态:
- **阶段5完成** - 可以向用户交付实现说明和验证结果。

## [2026-05-17 18:32:00] [Session ID: omx-1779004640353-blcixq] 新阶段启动: 并行 TUI raw/audit 视图

目标:
- 给 `ralph run` 并行 TUI 增加 raw/audit 输出视图。
- 在不丢失现有可读视图的前提下,允许用户切到接近 CLI/log-mode 的完整审计流。
- 继续复用现有 `ParallelTuiState` / `JobViewState.raw_lines`,不另建第二套 runtime 真相源。

可选方向:
1. 不惜代价最佳方案:
   - 新增正式 `ParallelOutputViewMode::{Rendered, Plain, Audit}`。
   - 键盘操作在三态间循环,Footer/Output title 同步展示 mode。
   - Audit 视图从 `raw_lines` 重建 `[instance:stream:job=n] line`。
   - 优点: 状态模型清晰,以后可继续扩展 evidence/status 面板。
2. 先能用,后面再优雅:
   - 保留现有 Rendered/Plain,单独加一个临时 bool `audit_mode`。
   - 优点: 改动小;缺点: mode 状态容易和 `MarkdownRenderMode` 分裂。

当前决策:
- 采用方向1的收敛版: 引入并行 Output 视图三态,但只改 TUI 展示层和相关测试,不改 runtime/event 协议。

阶段计划:
- [ ] 阶段1: 读取并梳理现有并行 output render / input shortcut / footer mode 路径。
- [ ] 阶段2: 编写 spec 文档和图,用 `beautiful-mermaid-rs` 验证 Mermaid。
- [ ] 阶段3: 先补 focused tests 锁定 audit 视图和 mode 切换。
- [ ] 阶段4: 实现 raw/audit 视图。
- [ ] 阶段5: 跑 focused tests、`cargo test -p ralph-tui`、必要 full gates。
- [ ] 阶段6: 更新 notes / WORKLOG / LATER_PLANS / ERRORFIX,交付。

状态:
- **目前在阶段1** - 开始读取 TUI output、快捷键与 footer 路径。

## [2026-05-17 18:40:00] [Session ID: omx-1779004640353-blcixq] 阶段推进: raw/audit 规格完成

已完成:
- [x] 阶段1: 读取并梳理现有并行 output render / input shortcut / footer mode 路径。
- [x] 阶段2: 编写 spec 文档和图,用 `beautiful-mermaid-rs` 验证 Mermaid。

产物:
- `specs/parallel-tui-raw-audit-view.md`

关键决策:
- 新增三态输出视图: Rendered / Plain / Audit。
- `v` 键循环切换视图。
- Audit 视图复用 `raw_lines`,不新增第二套输出缓存。
- Activity 在 Rendered/Plain 中仍不进正文,但在 Audit 中可见。

状态:
- **目前在阶段3** - 开始补 focused tests,预期先失败再实现。

## [2026-05-17 19:05:00] [Session ID: omx-1779004640353-blcixq] 阶段完成: raw/audit 视图实现与验证通过

已完成:
- [x] 阶段3: 补 focused tests,覆盖 `v` 键、mode 循环、audit 输出、footer mode。
- [x] 阶段4: 实现 raw/audit 视图,复用 `raw_lines` 单一真相源。
- [x] 阶段5: 跑 focused tests、`cargo test -p ralph-tui`、全量 `cargo test`、格式和 diff 检查。
- [x] 阶段6: 更新 notes / WORKLOG / LATER_PLANS,准备交付。

验证证据:
- `cargo test --package ralph-tui --lib`: passed, 223 tests。
- `cargo test -p ralph-tui`: passed, 包含 integration snapshots。
- `cargo test`: passed。
- `cargo fmt --all -- --check`: passed。
- `git diff --check`: passed。

遇到错误:
- 一次 focused test 命令误把多个 test name 同时传给 `cargo test`,Cargo 返回用法错误。已改用 `cargo test --package ralph-tui --lib` 覆盖。
- `cargo fmt --all -- --check` 首次提示 app test 中两个 assert 需要换行。已运行 `cargo fmt --all` 修复并复验通过。

状态:
- **raw/audit 视图已完成** - 可以交付。

## [2026-05-17 19:41:00] [Session ID: omx-1779004640353-blcixq] 最终复核: raw/audit 视图当前会话验证

已复核:
- `git diff --check`: passed, 没有 whitespace/error 输出。
- `cargo test -p ralph-tui`: passed, 223 lib tests + 26 integration snapshot tests + 4 iteration boundary tests + doc-tests。
- `WORKLOG.md` 当前 987 行,本轮不再追加,避免触发六文件续档阈值。

状态:
- **raw/audit 视图可以交付** - 当前会话已有轻量验证证据。

## [2026-05-17 20:54:23] [Session ID: omx-1779004640353-blcixq] 新阶段启动: evidence/status 面板与 Output 内 activity 底栏

目标:
- 在并行 TUI 中增加 evidence/status 展示,把当前证据路径露出来,让用户知道 record-session / events / agents 等证据在哪里。
- 把 `act` 从 Footer 挪到 Output 窗口最下方,让 Codex 风格的 `Working` / `Inspecting ...` 更接近正在阅读的输出区域。
- 保持单一真相源: activity 仍来自已有 `current_activity`; evidence 路径优先来自已有 runtime/update/config,不在 widget 里推断第二套状态。

阶段计划:
- [ ] 阶段1: 读取现有并行 TUI state / app layout / footer / output renderer / external event writer 路径。
- [ ] 阶段2: 设计 evidence/status 数据模型和 Output 内 activity 底栏,先补 focused tests。
- [ ] 阶段3: 实现面板与 activity 位置调整。
- [ ] 阶段4: 运行 focused tests、`cargo test -p ralph-tui`、格式和 diff 检查。
- [ ] 阶段5: 更新 notes / WORKLOG / LATER_PLANS / ERRORFIX,注意 `WORKLOG.md` 接近 1000 行阈值。

状态:
- **目前在阶段1** - 先只读梳理现有实现,再改。
