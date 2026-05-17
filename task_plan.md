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
