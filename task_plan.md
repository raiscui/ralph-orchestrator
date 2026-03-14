# 任务计划: 六文件续档与 batch-6 收尾

## 目标

完成 batch-6 的上下文续档与最终收尾记录,保证验证证据、关键决定和后续状态都留在活跃六文件中。

## 阶段

- [x] 阶段1: 回读六文件尾部,确认 batch-6 已完成的实现与验证证据
- [x] 阶段2: 按超过 1000 行规则续档 `task_plan.md` 与 `notes.md`
- [x] 阶段3: 在新的 `task_plan.md` / `notes.md` 中补 batch-6 完成记录与持续学习摘要
- [x] 阶段4: 追加 `WORKLOG.md`,并检查 `LATER_PLANS.md` / `EPIPHANY_LOG.md` 是否需要更新

## 关键问题

1. `notes.md` 已超过 1000 行,必须先续档,不能直接继续追加。
2. `task_plan.md` 仅剩 1 行空间,一旦补写完成状态就会越界,因此一起续档最稳妥。
3. batch-6 的代码与验证已经完成,本轮主要任务是把事实和结论收尾到位。

## 做出的决定

- 先做一次六文件摘要,再把超长的 `task_plan.md` 与 `notes.md` 续档到 `archive/`。
- 不新增 `EPIPHANY_LOG.md` 记录,因为 batch-6 没暴露新的架构级根因,只是继续验证既有方法论。
- 不新增 `LATER_PLANS.md` 项,因为这轮没有形成必须延后的新事项。

## 遇到错误

- 无新的代码错误或测试错误。本轮主要是上下文文件维护与交付收尾。

## 2026-03-11 21:17:47 +0800 | batch-6 收尾完成

- [x] 阶段1: 回读六文件尾部,确认 batch-6 已完成的实现与验证证据
- [x] 阶段2: 按超过 1000 行规则续档 `task_plan.md` 与 `notes.md`
- [x] 阶段3: 在新的 `task_plan.md` / `notes.md` 中补 batch-6 完成记录与持续学习摘要
- [x] 阶段4: 追加 `WORKLOG.md`,并检查 `LATER_PLANS.md` / `EPIPHANY_LOG.md` 是否需要更新

- 当前事实:
  - batch-6 已新增 3 个真实并行 example:
    - `parallel-support-escalation-desk`
    - `parallel-partner-launch-coordination`
    - `parallel-field-enablement-rollout`
  - 代码、定向测试、live E2E 与 `cargo test` 已在上一轮全部通过。
  - 本轮已把旧 `task_plan.md` / `notes.md` 续档到:
    - `archive/task_plan_20260311-211747.md`
    - `archive/notes_20260311-211747.md`

- 当前状态:
  - **全部完成**: batch-6 的实现、验证、文档入口与六文件收尾都已闭环。

## 2026-03-11 21:28:00 +0800 | 继续推进: 把并行 example 的方案说明收敛成中文

- [ ] 阶段1: 回读并定位目前仍是英文描述的并行 example 文档入口
- [ ] 阶段2: 新增一份中文总览,把真实并行 example 的方法、分批扩展和选型建议讲清楚
- [ ] 阶段3: 把 README 与 `crates/ralph-e2e/README.md` 中相关描述改成中文
- [ ] 阶段4: 把 batch-6 三个 example README 的标题和方案描述进一步中文化,再补六文件收尾

- 当前目标:
  - 用户希望后续继续补的是“中文的方案和描述”,而不是继续新增英文介绍。
  - 这轮不改 runtime 和测试逻辑,只收敛说明层。

- 当前决定:
  - 新增中文总览文档,避免中文说明散落在多个 README 里。
  - 保持路径名、topic 名、类名不变,只把面向人的说明文字改成中文。

- 当前状态:
  - **目前在阶段1**: 已定位到根 README、`crates/ralph-e2e/README.md` 和 batch-6 三个 example README 里的英文描述片段。

## 2026-03-11 22:52:08 +0800 | 中文方案与描述收尾完成

- [x] 阶段1: 回读并定位目前仍是英文描述的并行 example 文档入口
- [x] 阶段2: 新增一份中文总览,把真实并行 example 的方法、分批扩展和选型建议讲清楚
- [x] 阶段3: 把 README 与 `crates/ralph-e2e/README.md` 中相关描述改成中文
- [x] 阶段4: 把 batch-6 三个 example README 的标题和方案描述进一步中文化,再补六文件收尾

- 已完成内容:
  - 新增中文总览:
    - `docs/examples/parallel-real-world-examples.zh-CN.md`
  - 已同步中文描述到:
    - `README.md`
    - `crates/ralph-e2e/README.md`
    - `examples/parallel-support-escalation-desk/README.md`
    - `examples/parallel-partner-launch-coordination/README.md`
    - `examples/parallel-field-enablement-rollout/README.md`
  - 已补充按题材分组的中文选型方案、范例矩阵和 batch-6 中文说明

- 验证结果:
  - `git diff --check -- README.md crates/ralph-e2e/README.md docs/examples/parallel-real-world-examples.zh-CN.md examples/parallel-support-escalation-desk/README.md examples/parallel-partner-launch-coordination/README.md examples/parallel-field-enablement-rollout/README.md` ✅

- 当前结论:
  - 并行 example 的“对外解释层”已经从零散英文描述收敛成可直接阅读的中文方案。
  - 这轮没有改 runtime 或测试逻辑,因此没有重复跑 Rust 测试链路。

- 当前状态:
  - **全部完成**: 中文方案、中文描述、中文总览与六文件收尾都已闭环。

## 2026-03-11 23:01:00 +0800 | 继续推进: 统一 batch-1 到 batch-5 的中文 README,并挂入 docs 入口

- [ ] 阶段1: 扫描 batch-1 到 batch-5 的 example README,确认残留的英文说明模式
- [ ] 阶段2: 统一改写 batch-1 到 batch-5 的 README 标题、用途说明和运行说明
- [ ] 阶段3: 更新 `docs/examples/index.md`,把中文总览接入文档入口
- [ ] 阶段4: 做文档自检并补六文件收尾

- 当前目标:
  - 把 batch-1 到 batch-5 的真实并行范例说明也统一到中文风格。
  - 让中文总览不只存在于文件路径里,也能从 docs 目录入口找到。

- 当前决定:
  - 这轮继续只动说明文档,不动 runtime 与测试。
  - 继续保留路径名、topic 名、类名、section 标题这些和代码或 prompt 对齐的技术标识。

- 当前状态:
  - **目前在阶段1**: 已完成 batch-1 到 batch-5 README 的英文残留摸底。

## 2026-03-11 23:08:21 +0800 | batch-1 到 batch-5 中文 README 与 docs 入口收尾完成

- [x] 阶段1: 扫描 batch-1 到 batch-5 的 example README,确认残留的英文说明模式
- [x] 阶段2: 统一改写 batch-1 到 batch-5 的 README 标题、用途说明和运行说明
- [x] 阶段3: 更新 `docs/examples/index.md`,把中文总览接入文档入口
- [x] 阶段4: 做文档自检并补六文件收尾

- 已完成内容:
  - 已统一中文风格的 README:
    - `examples/parallel-pr-review/README.md`
    - `examples/parallel-release-checklist/README.md`
    - `examples/parallel-human-approval-gate/README.md`
    - `examples/parallel-incident-response-war-room/README.md`
    - `examples/parallel-security-exception-review/README.md`
    - `examples/parallel-customer-renewal-desk/README.md`
    - `examples/parallel-audit-evidence-pack/README.md`
    - `examples/parallel-finance-close-control-room/README.md`
    - `examples/parallel-hiring-debrief-panel/README.md`
    - `examples/parallel-customer-onboarding-activation/README.md`
    - `examples/parallel-launch-readiness-command/README.md`
    - `examples/parallel-migration-rehearsal/README.md`
    - `examples/parallel-postmortem-action-board/README.md`
    - `examples/parallel-proposal-assembly/README.md`
    - `examples/parallel-vendor-security-procurement/README.md`
  - 已把中文入口挂到:
    - `docs/examples/index.md`

- 验证结果:
  - `git diff --check -- docs/examples/index.md docs/examples/parallel-real-world-examples.zh-CN.md examples/parallel-pr-review/README.md examples/parallel-release-checklist/README.md examples/parallel-human-approval-gate/README.md examples/parallel-incident-response-war-room/README.md examples/parallel-security-exception-review/README.md examples/parallel-customer-renewal-desk/README.md examples/parallel-audit-evidence-pack/README.md examples/parallel-finance-close-control-room/README.md examples/parallel-hiring-debrief-panel/README.md examples/parallel-customer-onboarding-activation/README.md examples/parallel-launch-readiness-command/README.md examples/parallel-migration-rehearsal/README.md examples/parallel-postmortem-action-board/README.md examples/parallel-proposal-assembly/README.md examples/parallel-vendor-security-procurement/README.md task_plan.md notes.md WORKLOG.md` ✅

- 当前结论:
  - 现在真实并行 example 从 docs 入口到具体场景 README,已经形成了一条连续的中文阅读路径。
  - 这轮仍然没有改 runtime 或测试逻辑,因此没有重复跑 Rust 测试链路。

- 当前状态:
  - **全部完成**: batch-1 到 batch-6 的中文说明主线已经连起来了。

## 2026-03-11 23:16:00 +0800 | 直接启动 batch-7: 扩到营收运营 / 高层业务回顾 / 客户顾问委员会

- [ ] 阶段1: 收敛 batch-7 题材,明确与前六批的去重边界、终态字段和 topic 协议
- [ ] 阶段2: 落盘 `specs/parallel-real-world-examples-batch-7.spec.md`,并为 3 个 example 写 `ralph.yml`、`PROMPT.md`、`README.md`
- [ ] 阶段3: 实现 3 个 direct example scenario 与注册点,同步 README 入口
- [ ] 阶段4: 运行 mermaid 校验、定向测试、必要的 live E2E 与仓库级验证,再做六文件收尾

- 方向对比:
  - 方向A: 最佳方案
    - 继续拉向商业与经营协同
    - 候选:
      - `parallel-revops-quote-desk`
      - `parallel-executive-business-review-prep`
      - `parallel-customer-advisory-board-prep`
    - 优点:
      - 和前六批重叠最低
      - 更能证明并行编排不只服务工程和运营支持
      - 终态字段也比较容易固定
    - 风险:
      - 要刻意避开和 `proposal-assembly`、`customer-renewal-desk` 的语义重叠
  - 方向B: 先快做可用方案
    - 继续沿着内部运营节奏推进
    - 候选:
      - `parallel-regional-operating-review`
      - `parallel-quarterly-planning-handoff`
      - `parallel-forecast-risk-sync`
    - 优点:
      - 更偏内部流程,资料结构更容易稳定
    - 风险:
      - 三个场景彼此可能太像,展示面不如方向A 拉得开

- 当前决定:
  - 用户已经明确说“直接开始 batch-7”,本轮默认采用方向A。
  - 第七批暂定 3 个场景:
    1. `parallel-revops-quote-desk`
    2. `parallel-executive-business-review-prep`
    3. `parallel-customer-advisory-board-prep`

- 设计约束:
  - 继续沿用 direct prompt-file example 模式
  - 继续要求 coordinator 在未收齐所有 ready 前保持静默
  - final topic 由明确 finalizer 发布
  - worker 继续禁止输出 `LOOP_COMPLETE`
  - 事件形态继续锁定为 `<event ...>payload</event>`

- 当前状态:
  - **目前在阶段1**: 开始收敛 batch-7 的题材边界、终态字段和 direct example 协议。

## 2026-03-11 23:40:00 +0800 | batch-7: CAB 场景实现推进

- [x] 阶段1: 确认批次语义、固定字段与 fan-out/fan-in 约束
- [ ] 阶段2: `specs/parallel-real-world-examples-batch-7.spec.md` 的最终化（留给主线程完成注册/文档同步）
- [x] 阶段3: 假定 spec 方向、实现 `examples/parallel-customer-advisory-board-prep` 的 `ralph.yml`、`PROMPT.md`、`README.md` 及 `ParallelCustomerAdvisoryBoardPrep` Rust scenario
- [ ] 阶段4: 验证/测试（待主线程或并行 worker 跟进）

- 当前状态: 阶段3 正在推进中，已经完成具体 example 与 scenario 的草稿，等待后续协调补充 spec/注册与验证。

## 2026-03-12 00:02:00 +0800 | batch-7 主线程接管: 做共享注册、文档同步与验证收尾

- [x] 阶段1: 收敛 batch-7 题材,明确与前六批的去重边界、终态字段和 topic 协议
- [x] 阶段2: 复核 3 个新 example / scenario 与 spec 是否对齐,补齐共享注册点
- [x] 阶段3: 同步 README / docs / 场景入口,并完成 mermaid 校验
- [x] 阶段4: 跑定向测试、必要 live E2E 与仓库级验证,完成六文件收尾

- 当前动作:
  - 已完成 3 个新场景的共享注册、中文文档入口、mermaid 校验和定向测试。
  - 已完成 3 条真实 Codex live E2E 与 `cargo test` 仓库级验证。

- 当前状态:
  - **全部完成**: batch-7 已完成接线、文档、live E2E、仓库级验证与六文件收尾。

## 2026-03-12 16:10:00 +0800 | OpenSpec apply: hat-request-reply-channel 实现验证与收尾

- [ ] 阶段1: 回读 change 的 OpenSpec 状态、apply 指令与上下文文件,确认当前实现范围
- [ ] 阶段2: 运行针对 `reply.hat.message` 的关键测试链路,确认 prompt、路由、fail-closed 与共存路径都通过
- [ ] 阶段3: 补跑格式与更大范围测试,确认没有把现有并行监督器行为带坏
- [ ] 阶段4: 勾选 OpenSpec tasks,补 `notes.md` / `WORKLOG.md`,完成本轮收尾

- 当前目标:
  - 把 `hat-request-reply-channel` 从“已经写完主要代码”推进到“有验证证据、任务已勾选、上下文已收尾”的完成态。

- 当前已知事实:
  - `reply.hat.message` 的协议常量、路由 special-case、resume 恢复索引、prompt 与文档说明、针对性单测已经落地。
  - 还需要继续完成更完整的验证链路,然后才能安全勾选 OpenSpec tasks。

- 当前决定:
  - 先跑最关键的 prompt 测试和 routing 测试集。
  - 如果通过,继续跑 `cargo fmt --all --check` 与 `cargo test -p ralph-core`。
  - 只有验证通过后才勾选 `openspec/changes/hat-request-reply-channel/tasks.md`。

- 当前状态:
  - **目前在阶段1**: 正在重新确认 OpenSpec 状态、apply 指令和上下文文件,准备进入验证阶段。

## 2026-03-12 16:16:00 +0800 | OpenSpec apply: 进入验证阶段并发现格式漂移

- [x] 阶段1: 回读 change 的 OpenSpec 状态、apply 指令与上下文文件,确认当前实现范围
- [ ] 阶段2: 运行针对 `reply.hat.message` 的关键测试链路,确认 prompt、路由、fail-closed 与共存路径都通过
- [ ] 阶段3: 补跑格式与更大范围测试,确认没有把现有并行监督器行为带坏
- [ ] 阶段4: 勾选 OpenSpec tasks,补 `notes.md` / `WORKLOG.md`,完成本轮收尾

- 已验证事实:
  - `busy_ralph_secondary_includes_coordinator_instructions_and_config_prompt` 已通过。
  - `parallel::supervisor::routing_tests` 38 条测试已全部通过,包括新增的 `reply.hat.message` 成功、fail-closed 与双通道共存路径。

- 新发现:
  - `cargo fmt --all --check` 失败,目前看到的是 `routing_tests.rs` 与 `ralph-proto/src/lib.rs` 的格式漂移。
  - 现象层结论仅限于“格式未对齐”,还没有新的逻辑失败证据。

- 当前决定:
  - 先执行 `cargo fmt --all` 对齐格式。
  - 再继续跑 `cargo test -p ralph-core`,确认格式修正后仍然稳定通过。

- 当前状态:
  - **目前在阶段3**: 关键 routing 测试已通过,正在处理格式漂移并准备进入包级验证。

## 2026-03-12 16:20:00 +0800 | OpenSpec apply: 包级验证通过,转入仓库级收口

- [x] 阶段1: 回读 change 的 OpenSpec 状态、apply 指令与上下文文件,确认当前实现范围
- [x] 阶段2: 运行针对 `reply.hat.message` 的关键测试链路,确认 prompt、路由、fail-closed 与共存路径都通过
- [ ] 阶段3: 补跑格式与更大范围测试,确认没有把现有并行监督器行为带坏
- [ ] 阶段4: 勾选 OpenSpec tasks,补 `notes.md` / `WORKLOG.md`,完成本轮收尾

- 已验证事实:
  - `cargo fmt --all` 已执行完成,格式漂移已对齐。
  - `cargo test -p ralph-core` 已全部通过:
    - unit tests 456 passed
    - diagnostics e2e 3 passed
    - event_loop_ralph 11 passed
    - scenarios 5 passed
    - smoke_runner 40 passed
    - doctests 9 passed

- 当前决定:
  - 再跑一次 `cargo fmt --all --check`,确认工作区回到格式干净状态。
  - 随后执行仓库级 `cargo test`,把“改动局部通过”推进到“仓库主验证链通过”。

- 当前状态:
  - **目前在阶段3**: 已完成核心包验证,正在进入最终仓库级收口验证。

## 2026-03-12 16:28:00 +0800 | OpenSpec apply: hat-request-reply-channel 收尾完成

- [x] 阶段1: 回读 change 的 OpenSpec 状态、apply 指令与上下文文件,确认当前实现范围
- [x] 阶段2: 运行针对 `reply.hat.message` 的关键测试链路,确认 prompt、路由、fail-closed 与共存路径都通过
- [x] 阶段3: 补跑格式与更大范围测试,确认没有把现有并行监督器行为带坏
- [x] 阶段4: 勾选 OpenSpec tasks,补 `notes.md` / `WORKLOG.md`,完成本轮收尾

- 最终验证结果:
  - `cargo test --package ralph-core --lib busy_ralph_secondary_includes_coordinator_instructions_and_config_prompt` ✅
  - `cargo test --package ralph-core --lib parallel::supervisor::routing_tests` ✅
  - `cargo fmt --all` ✅
  - `cargo fmt --all --check` ✅
  - `cargo test -p ralph-core` ✅
  - `cargo test` ✅

- OpenSpec 结论:
  - `openspec/changes/hat-request-reply-channel/tasks.md` 已全部勾选完成。

- EPIPHANY / LATER 结论:
  - 本轮没有新增需要写入 `EPIPHANY_LOG.md` 的架构级风险。
  - 本轮没有新增必须延期到 `LATER_PLANS.md` 的后续事项。

- 当前状态:
  - **全部完成**: `hat-request-reply-channel` 已完成实现、验证、任务勾选与六文件收尾。

## 2026-03-13 00:05:00 +0800 | OpenSpec archive: 等待用户明确选择要归档的 change

- [ ] 阶段1: 列出当前活跃 change,由用户明确选择归档目标
- [ ] 阶段2: 检查所选 change 的 artifact、tasks 与 delta spec sync 状态
- [ ] 阶段3: 如需要,先执行 sync 或记录用户选择跳过 sync
- [ ] 阶段4: 执行 archive,补六文件收尾

- 当前事实:
  - `openspec list --json` 当前显示的活跃 change 有:
    - `hat-request-reply-channel` (`complete`, 9/9)
    - `event-id-and-reply` (`no-tasks`)
    - `tui-mdfried-viewer` (`in-progress`, 13/15)
  - 归档 skill 明确要求: 未显式提供 change 名时,不能自动猜测,必须先由用户选择。

- 当前状态:
  - **目前在阶段1**: 已列出活跃 change,正在等待用户明确指定归档目标。

## 2026-03-13 00:08:00 +0800 | OpenSpec archive: 已完成归档前检查,等待 sync 决策

- [x] 阶段1: 列出当前活跃 change,由用户明确选择归档目标
- [x] 阶段2: 检查所选 change 的 artifact、tasks 与 delta spec sync 状态
- [ ] 阶段3: 如需要,先执行 sync 或记录用户选择跳过 sync
- [ ] 阶段4: 执行 archive,补六文件收尾

- 已确认事实:
  - 用户已选择 `hat-request-reply-channel`。
  - `openspec status --change "hat-request-reply-channel" --json` 显示所有 artifacts 都是 `done`。
  - `openspec/changes/hat-request-reply-channel/tasks.md` 9/9 已完成。
  - 该 change 含 1 个 delta spec:
    - `openspec/changes/hat-request-reply-channel/specs/hat-request-reply-channel/spec.md`
  - 主 specs 中当前不存在:
    - `openspec/specs/hat-request-reply-channel/spec.md`
  - 因此如果执行 sync,本次会把这个 capability 作为新增主 spec 同步进去。

- 当前状态:
  - **目前在阶段3**: 已完成检查,正在等待用户决定“先 sync 再 archive”还是“直接 archive 不 sync”。

## 2026-03-13 00:11:00 +0800 | OpenSpec archive: 用户已同意先 sync 主 specs

- [x] 阶段1: 列出当前活跃 change,由用户明确选择归档目标
- [x] 阶段2: 检查所选 change 的 artifact、tasks 与 delta spec sync 状态
- [ ] 阶段3: 如需要,先执行 sync 或记录用户选择跳过 sync
- [ ] 阶段4: 执行 archive,补六文件收尾

- 当前决定:
  - 用户已确认对 `hat-request-reply-channel` 先做 spec sync,再执行 archive。
  - 由于主 specs 中还没有 `openspec/specs/hat-request-reply-channel/spec.md`,本轮 sync 会创建新的主规范文件。

- 当前状态:
  - **目前在阶段3**: 正在把 delta spec 同步到主 specs,随后进入 archive。

## 2026-03-13 00:14:00 +0800 | OpenSpec archive: sync 已完成,准备执行归档移动

- [x] 阶段1: 列出当前活跃 change,由用户明确选择归档目标
- [x] 阶段2: 检查所选 change 的 artifact、tasks 与 delta spec sync 状态
- [x] 阶段3: 如需要,先执行 sync 或记录用户选择跳过 sync
- [ ] 阶段4: 执行 archive,补六文件收尾

- 已完成:
  - 已创建主 spec:
    - `openspec/specs/hat-request-reply-channel/spec.md`
  - 已执行:
    - `git diff --check -- openspec/specs/hat-request-reply-channel/spec.md task_plan.md` ✅
    - `openspec validate hat-request-reply-channel --type change` ✅

- 当前状态:
  - **目前在阶段4**: 正在确认 archive 目标路径未占用,随后移动 change 目录并完成收尾。

## 2026-03-13 00:16:00 +0800 | OpenSpec archive: `hat-request-reply-channel` 已完成 sync 与归档

- [x] 阶段1: 列出当前活跃 change,由用户明确选择归档目标
- [x] 阶段2: 检查所选 change 的 artifact、tasks 与 delta spec sync 状态
- [x] 阶段3: 如需要,先执行 sync 或记录用户选择跳过 sync
- [x] 阶段4: 执行 archive,补六文件收尾

- 已完成:
  - 已新增主 spec:
    - `openspec/specs/hat-request-reply-channel/spec.md`
  - 已归档到:
    - `openspec/changes/archive/2026-03-13-hat-request-reply-channel/`
  - 已确认 active changes 列表中不再包含 `hat-request-reply-channel`

- 最终状态:
  - schema: `spec-driven`
  - artifacts: 全部完成
  - tasks: 9/9 完成
  - specs: 已同步到主 specs
  - archive: 已完成

- 当前状态:
  - **全部完成**: `hat-request-reply-channel` 已完成 sync、archive 与六文件收尾。

## 2026-03-12 10:24:00 +0800 | 直接启动 batch-8: 扩到区域经营收口 / 续费风险校准 / 多区域 pipeline 校准

- [ ] 阶段1: 收敛 batch-8 题材,明确与 batch-1 到 batch-7 的去重边界、终态字段和 topic 协议
- [ ] 阶段2: 落盘 `specs/parallel-real-world-examples-batch-8.spec.md`,并为 3 个 example 写 `ralph.yml`、`PROMPT.md`、`README.md`
- [ ] 阶段3: 实现 3 个 direct example scenario 与注册点,同步 README 入口
- [ ] 阶段4: 运行 mermaid 校验、定向测试、必要的 live E2E 与仓库级验证,再做六文件收尾

- 方向对比:
  - 方向A: 最佳方案
    - 继续沿着经营节奏和商业运营往前推
    - 候选:
      - `parallel-regional-operating-review`
      - `parallel-renewal-risk-calibration`
      - `parallel-multi-region-pipeline-sync`
    - 优点:
      - 和 batch-7 的商业协同方向自然衔接
      - 和前面已落地的工程、支持、伙伴、客户单案题材重复最低
      - 都能写出比较硬的固定终态字段
    - 风险:
      - 需要刻意避开与 `parallel-customer-renewal-desk` 的“单客户续费收口”重叠
      - 需要避免 3 个场景都写成“经营例会”导致同质化
  - 方向B: 先快做可用方案
    - 改走董事会 / 高层材料预演
    - 候选:
      - `parallel-board-pack-rehearsal`
      - `parallel-quarterly-portfolio-review`
      - `parallel-forecast-commit-alignment`
    - 优点:
      - final packet 的结构更容易固定
      - topic 语义更集中,写起来更快
    - 风险:
      - 和 batch-7 的 EBR 题材距离太近
      - 展示面没有方向A 拉得开

- 当前决定:
  - 用户已经明确说“直接开始 batch-8”,本轮默认采用方向A。
  - 第八批暂定 3 个场景:
    1. `parallel-regional-operating-review`
    2. `parallel-renewal-risk-calibration`
    3. `parallel-multi-region-pipeline-sync`

- 设计约束:
  - 继续沿用 direct prompt-file example 模式
  - 继续要求 coordinator 在未收齐所有 ready 前保持静默
  - final topic 由明确的 finalizer hat 发布
  - worker 继续禁止输出 `LOOP_COMPLETE`
  - 事件形态继续锁定为 `<event ...>payload</event>`
  - 优先让 worker / finalizer 输出单行 JSON event,减少真实 backend 下 closing tag 漂移
  - final payload 断言继续优先使用“去掉 `[hat#n:out:job=m]` 前缀后的 stdout out 行”作为主证据

- 当前状态:
  - **目前在阶段1**: 开始收敛 batch-8 的题材边界、终态字段和 direct example 协议。

## 2026-03-12 10:37:00 +0800 | batch-8 主线程推进: spec 已落盘,等待 3 个 example 草稿回收

- [x] 阶段1: 收敛 batch-8 题材,明确与 batch-1 到 batch-7 的去重边界、终态字段和 topic 协议
- [ ] 阶段2: 落盘 `specs/parallel-real-world-examples-batch-8.spec.md`,并为 3 个 example 写 `ralph.yml`、`PROMPT.md`、`README.md`
- [ ] 阶段3: 实现 3 个 direct example scenario 与注册点,同步 README 入口
- [ ] 阶段4: 运行 mermaid 校验、定向测试、必要的 live E2E 与仓库级验证,再做六文件收尾

- 当前动作:
  - `specs/parallel-real-world-examples-batch-8.spec.md` 已新增。
  - 主线程正在等待 3 个场景各自的 example/scenario 草稿回收,然后统一做共享注册与文档入口接线。

- 当前状态:
  - **目前在阶段2**: spec 已完成,正在汇总 3 个 example 本体与 scenario 实现。

## 2026-03-12 10:52:00 +0800 | batch-8 进入验证前状态刷新: 3 个场景、共享注册与文档入口都已接线

- [x] 阶段1: 收敛 batch-8 题材,明确与 batch-1 到 batch-7 的去重边界、终态字段和 topic 协议
- [x] 阶段2: 落盘 `specs/parallel-real-world-examples-batch-8.spec.md`,并为 3 个 example 写 `ralph.yml`、`PROMPT.md`、`README.md`
- [x] 阶段3: 实现 3 个 direct example scenario 与注册点,同步 README 入口
- [ ] 阶段4: 运行 mermaid 校验、定向测试、必要的 live E2E 与仓库级验证,再做六文件收尾

- 当前动作:
  - 已完成 3 个 batch-8 example:
    1. `parallel-regional-operating-review`
    2. `parallel-renewal-risk-calibration`
    3. `parallel-multi-region-pipeline-sync`
  - 已完成共享接线:
    - `crates/ralph-e2e/src/scenarios/mod.rs`
    - `crates/ralph-e2e/src/lib.rs`
    - `crates/ralph-e2e/src/main.rs`
    - `crates/ralph-cli/tests/integration_examples.rs`
    - `README.md`
    - `crates/ralph-e2e/README.md`
    - `docs/examples/parallel-real-world-examples.zh-CN.md`
  - 一个 worker 在主线程收尾阶段被中断,但目标文件已提前落盘并被主线程回读确认。

- 当前状态:
  - **目前在阶段4**: 开始执行 mermaid 校验、定向测试、live E2E 与仓库级验证。

## 2026-03-12 13:52:00 +0800 | batch-8 全部完成: live E2E、仓库级测试与六文件收尾已闭环

- [x] 阶段1: 收敛 batch-8 题材,明确与 batch-1 到 batch-7 的去重边界、终态字段和 topic 协议
- [x] 阶段2: 落盘 `specs/parallel-real-world-examples-batch-8.spec.md`,并为 3 个 example 写 `ralph.yml`、`PROMPT.md`、`README.md`
- [x] 阶段3: 实现 3 个 direct example scenario 与注册点,同步 README 入口
- [x] 阶段4: 运行 mermaid 校验、定向测试、必要的 live E2E 与仓库级验证,再做六文件收尾

- 最终结果:
  - batch-8 的 3 个真实并行场景已全部落地。
  - 3 条真实 Codex live E2E 已全部通过。
  - 其中 `parallel-renewal-risk-calibration-example` 首轮失败后已完成最小修复并复跑通过。
  - `cargo fmt --all --check`、`git diff --check -- <batch-8相关文件>`、`cargo test` 全部通过。

- 当前状态:
  - **全部完成**: batch-8 已完成实现、修复、验证与六文件收尾。

## 2026-03-12 14:18:00 +0800 | openspec-explore: 讨论“hat 回复返回给创建者”的协议语义

- [ ] 阶段1: 读取现有 OpenSpec change 与事件模型,确认系统现在已支持到哪一层
- [ ] 阶段2: 区分“reply 关联”与“creator 回传”是否是同一个问题
- [ ] 阶段3: 探索可选协议方向、风险和推荐路径

- 当前目标:
  - 用户在探索: hat 除了发布 workflow event 到下个环节,是否还应该把“最终答案”回给创建者。
  - 重点场景:
    - 一个 hat 调另一个 explorer hat 查资料
    - 发起方不想要一堆 workflow 细节,只想要结论

- 当前状态:
  - **目前在阶段1**: 已开始读取 `event-id-and-reply` change 与相关运行时代码。

## 2026-03-12 14:29:00 +0800 | openspec-explore 继续: 收敛 reply 关联 与 requester 回答回传 的协议边界

- [x] 阶段1: 读取现有 OpenSpec change 与事件模型,确认系统现在已支持到哪一层
- [ ] 阶段2: 区分“reply 关联”与“creator 回传”是否是同一个问题
- [ ] 阶段3: 探索可选协议方向、风险和推荐路径

- 当前动作:
  - 基于已读到的 `event-id-and-reply` change 与 runtime 代码,继续核对 `reply`、`reply.human.message`、`source_instance`、routing 行为。
  - 目标不是实现,而是形成一份可讨论的协议判断: 到底应该是“默认回传”,还是“显式 request-reply 通道”。

- 当前状态:
  - **目前在阶段2**: 正在把“事件关联”与“答案返回给请求方”拆开讨论,避免协议语义混淆。

## 2026-03-12 14:38:00 +0800 | openspec-explore 阶段收敛: 已形成协议判断与推荐方向

- [x] 阶段1: 读取现有 OpenSpec change 与事件模型,确认系统现在已支持到哪一层
- [x] 阶段2: 区分“reply 关联”与“creator 回传”是否是同一个问题
- [x] 阶段3: 探索可选协议方向、风险和推荐路径

- 当前结论:
  - 现有系统已经支持 `Event.reply`,但它只表达事件关联,不表达回送目标。
  - 用户提出的是一层新问题: 是否要为 hat-to-hat 提问场景提供“回答回到请求方”的显式通道。
  - 当前更推荐把它设计成可选 request-reply / answer-return 协议,而不是默认让所有 hat 回传 final answer。

- 当前状态:
  - **本轮探索已完成**: 可以继续选择是否把这个判断 capture 到现有 `event-id-and-reply` change,或者拆成新的 change。

## 2026-03-12 14:43:00 +0800 | 根据用户选择单独开新 change: `hat-request-reply-channel`

- [ ] 阶段1: 创建新的 OpenSpec change 骨架并确认 workflow
- [ ] 阶段2: 查看 change 状态与首个 artifact 指引
- [ ] 阶段3: 根据用户是否继续,再决定是否起草 proposal/design/spec

- 当前决定:
  - 用户已明确选择“拆成新的 change”。
  - 新 change 暂定命名为 `hat-request-reply-channel`,用于承载“请求方收到被调用 hat 的答案回流”协议,不再混入 `event-id-and-reply`。

- 当前状态:
  - **目前在阶段1**: 准备创建独立 change 骨架并检查 OpenSpec workflow 指引。

## 2026-03-12 14:46:00 +0800 | 新 change 骨架已创建,等待是否起草 proposal

- [x] 阶段1: 创建新的 OpenSpec change 骨架并确认 workflow
- [x] 阶段2: 查看 change 状态与首个 artifact 指引
- [ ] 阶段3: 根据用户是否继续,再决定是否起草 proposal/design/spec

- 已完成:
  - 已执行 `openspec new change "hat-request-reply-channel"`
  - 默认 schema 为 `spec-driven`
  - `openspec status --change "hat-request-reply-channel"` 显示当前进度 `0/4`
  - 首个可写 artifact 为 `proposal`

- 当前状态:
  - **目前在阶段3**: 已完成 change scaffold 与 proposal 指引读取,等待是否继续起草 artifact。

## 2026-03-12 14:55:00 +0800 | openspec-ff-change: 为 `hat-request-reply-channel` 一次性补齐到 apply-ready

- [ ] 阶段1: 读取 change 的 JSON 状态、artifact 顺序与每个 artifact 的 instructions
- [ ] 阶段2: 依赖顺序起草并落盘 proposal / design / specs
- [ ] 阶段3: 起草 tasks,运行 OpenSpec 状态校验,确认 change 已可进入 apply

- 当前动机:
  - 用户已明确切换到 `openspec-ff-change`,希望不要一步步停顿,而是直接把实现前需要的 artifact 全部补齐。
  - 本轮仍然只做 OpenSpec capture,不写运行时代码。

- 当前状态:
  - **目前在阶段1**: 正在读取 `hat-request-reply-channel` 的 JSON 状态和 artifact 指南,准备顺序生成全部文档。

## 2026-03-12 15:11:00 +0800 | openspec-ff-change 完成: `hat-request-reply-channel` 已到 apply-ready

- [x] 阶段1: 读取 change 的 JSON 状态、artifact 顺序与每个 artifact 的 instructions
- [x] 阶段2: 依赖顺序起草并落盘 proposal / design / specs
- [x] 阶段3: 起草 tasks,运行 OpenSpec 状态校验,确认 change 已可进入 apply

- 已完成:
  - 已生成 4 个 artifact:
    - `proposal.md`
    - `design.md`
    - `specs/hat-request-reply-channel/spec.md`
    - `tasks.md`
  - 已校验 design 内 2 个 mermaid 图通过 `beautiful-mermaid-rs --ascii`
  - 已执行 `openspec status --change "hat-request-reply-channel"` -> `4/4 artifacts complete`
  - 已执行 `openspec validate hat-request-reply-channel --type change` -> `valid`

- 当前状态:
  - **全部完成**: `hat-request-reply-channel` 已完成 fast-forward artifact 创建,可以进入 apply / implementation。

## 2026-03-12 15:18:00 +0800 | openspec-apply-change: 开始实现 `hat-request-reply-channel`

- [ ] 阶段1: 读取 apply 指引、OpenSpec 上下文与现有实现位置,确认实现顺序
- [ ] 阶段2: 实现 `reply.hat.message` 路由、fail-closed 与诊断/提示文案
- [ ] 阶段3: 补单元/集成测试与文档,跑验证并同步勾选 OpenSpec tasks

- 当前动机:
  - 用户已切换到 `openspec-apply-change`,本轮从已完成的 OpenSpec artifacts 进入实现。
  - 目标是让 `hat-request-reply-channel` 从 spec 进入可运行代码,并用测试锁住协议行为。

- 当前状态:
  - **目前在阶段1**: 正在读取 apply 指引、上下文文件与相关 runtime 模块,准备开始实现。

## 2026-03-12 15:36:00 +0800 | 进入代码编辑: 先落 request-reply 主干,再补测试

- [x] 阶段1: 读取 apply 指引、OpenSpec 上下文与现有实现位置,确认实现顺序
- [ ] 阶段2: 实现 `reply.hat.message` 路由、fail-closed 与诊断/提示文案
- [ ] 阶段3: 补单元/集成测试与文档,跑验证并同步勾选 OpenSpec tasks

- 当前动作:
  - 在 `ParallelSupervisor` 增加 event id -> requester source_instance 的最小索引与 history load。
  - 在 `routing.rs` 加入 `reply.hat.message` 特殊路由分支,并在 unresolved 时 fail-closed + 记录诊断。
  - 在 `config/all_hat.md` 与 coordinator prompt 补足 topic 边界说明。

- 当前状态:
  - **目前在阶段2**: 开始实现 request-reply 主干。
