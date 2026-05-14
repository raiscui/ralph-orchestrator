# 任务计划: experience_promotion workaround 审计修复

## 目标

消除 Stop hook 指出的未落地 fallback/workaround 代码,让 `experience_promotion.rs` 中对应逻辑具备明确设计语义、测试或近旁依据。

## 阶段

- [x] 阶段1: 启动支线上下文与确认 hook 目标
- [ ] 阶段2: 只读调查代码路径、候选假设和测试覆盖
- [ ] 阶段3: 修改实现或补足明确 rationale
- [ ] 阶段4: 运行针对性验证
- [ ] 阶段5: 记录 worklog/errorfix 并交付

## 关键问题

1. `Temporary workaround for one topic only.` 所在逻辑是不是临时绕过?
2. 如果不是临时绕过,它实际保护的兼容/迁移/安全语义是什么?
3. 是否已有测试覆盖该语义?如果没有,需要补针对性测试还是改代码去掉 fallback?

## 做出的决定

- 先按 systematic-debugging 做证据调查,不直接“改注释过 hook”。
- 限定范围在 `experience_promotion.rs` 及其直接测试/调用点。

## 遇到错误

- Stop hook 阻断: 检测到未落地 fallback/workaround 文案。

## 状态

**目前在阶段2** - 准备读取源码、调用点和测试。

## [2026-05-11 13:58:20] [Session ID: omx-1778475786175-ogndry] [行动记录]: 开始只读调查 workaround 语义

- 当前要做:
  - 读取 `crates/ralph-core/src/experience_promotion.rs`。
  - 搜索 “Temporary workaround” 与 experience promotion 相关测试。
  - 对照 `EXPERIENCE.md` 和 scoped-experience 相关代码,判断该分支是不是正式兼容路径。
- 为什么这样做:
  - hook 要求不能保留无依据 workaround。
  - 直接删 fallback 可能破坏已有迁移/降级语义,所以必须先确认真实代码路径。


## [2026-05-11 14:00:31] [Session ID: omx-1778475786175-ogndry] [调查结论与行动记录]: workaround 命中点是测试 fixture 文案

### 现象
- hook 报告 `crates/ralph-core/src/experience_promotion.rs` 包含 `Temporary workaround for one topic only.`。
- 精确读取后,该字符串位于 `role_demotion_to_topic_keeps_topic_reference` 测试中,作为 `ExperienceEntry::new(...)` 的 summary fixture。

### 主假设
- 这不是运行时 fallback 分支,而是测试 fixture 使用了 “workaround” 字眼,触发了审计器对新增源码行的保守拦截。

### 备选解释
- 如果该 fixture 原本想表达临时兼容降级,则应在近旁写清 compatibility/fail-safe/tested rationale。

### 验证计划
- 将 fixture summary 改为明确的 topic-local 经验语义,不再使用 workaround/bypass/fallback 词汇。
- 运行单个精确测试验证 role -> topic demotion 行为不变。

### 状态
- 阶段2 完成,进入阶段3 修改。


## [2026-05-11 14:03:02] [Session ID: omx-1778475786175-ogndry] [错误记录]: 单测命令参数位置错误

- 错误:
  - `cargo test --package ralph-core --lib experience_promotion::tests::role_demotion_to_topic_keeps_topic_reference --exact` 被 Cargo 拒绝。
- 处理:
  - 已记录到 `ERRORFIX__experience_promotion_workaround.md`。
  - 接下来使用 `cargo test --package ralph-core --lib -- experience_promotion::tests::role_demotion_to_topic_keeps_topic_reference --exact` 重跑。
- 状态:
  - 阶段4 验证继续。


## [2026-05-11 14:05:03] [Session ID: omx-1778475786175-ogndry] [验证记录]: 模块测试通过并启动全量测试

- 已完成:
  - `rg -n "Temporary workaround|workaround|bypass|fallback" crates/ralph-core/src/experience_promotion.rs` 无命中。
  - `cargo test --package ralph-core --lib experience_promotion` 通过,8 个测试全部 ok。
- 当前要做:
  - 继续运行仓库要求的 `cargo test` 全量验证。
- 为什么这样做:
  - 本次虽然只改测试 fixture 文案,但仓库规则要求代码改动完成前运行全量测试。

## [2026-05-11 14:08:24] [Session ID: omx-1778475786175-ogndry] [行动记录]: 当前会话复核最终验证证据

- 当前要做:
  - 复查  中 workaround/fallback/bypass 审计词是否清空。
  - 运行精确测试、模块测试和仓库全量测试,用当前会话证据确认状态。
  - 补写 WORKLOG / ERRORFIX 收尾记录。
- 为什么这样做:
  - 上一轮模型交接说验证已通过,但最终回复前需要当前会话重新读取输出和退出码。
  - Stop hook 修复属于审计门禁问题,不能只凭记忆或摘要结论交付。


## [2026-05-11 14:09:34] [Session ID: omx-1778475786175-ogndry] [更正记录]: 上一条行动记录的反引号文本被 shell 吞掉

- 更正内容:
  - 上一条计划中的“复查  中 workaround/fallback/bypass 审计词是否清空”应为: 复查 `experience_promotion.rs` 中 workaround/fallback/bypass 审计词是否清空。
- 原因:
  - 追加 Markdown 时误用了未加引号 heredoc,导致反引号触发命令替换。
- 当前处理:
  - 已将该错误记录到 `ERRORFIX__experience_promotion_workaround.md`。
  - 后续所有含反引号的 Markdown 追加都改用 `cat <<'EOF'`。
- 状态:
  - 继续执行当前会话的验证复核。

## [2026-05-11 14:11:52] [Session ID: omx-1778475786175-ogndry] [验证记录]: 最终验证通过并进入交付

### 已完成验证
- 文案审计:
  - `rg -n 'Temporary workaround|workaround|bypass|fallback' crates/ralph-core/src/experience_promotion.rs || true`
  - 结果: 无命中。
- 精确测试:
  - `cargo test --package ralph-core --lib -- experience_promotion::tests::role_demotion_to_topic_keeps_topic_reference --exact`
  - 结果: 1 passed, 0 failed。
- 模块测试:
  - `cargo test --package ralph-core --lib experience_promotion`
  - 结果: 8 passed, 0 failed。
- 仓库全量测试:
  - `cargo test`
  - 结果: exit code 0。输出中各 crate/unit/integration/doctest 均为 ok,没有 error。

### 阶段状态
- [x] 阶段1: 启动支线上下文与确认 hook 目标
- [x] 阶段2: 只读调查代码路径、候选假设和测试覆盖
- [x] 阶段3: 修改实现或补足明确 rationale
- [x] 阶段4: 运行针对性验证
- [x] 阶段5: 记录 worklog/errorfix 并交付

### 最终结论
- Stop hook 指出的内容不是运行时 fallback/workaround 分支。
- 实际命中点是测试 fixture 的 summary 文案。
- 已替换为 topic-local scope 语义明确的文案,不改变业务逻辑。

### 状态
**目前在阶段5** - 收尾记录完成后交付。
