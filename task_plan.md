# 任务计划: parallel-trigger-routing 示例对齐语义 + 中文 parallel E2E

## 目标
1) 让 `examples/parallel-trigger-routing` 这个示例不再“靠 prompt 写死闭环逻辑”，而是用 `event_loop.starting_event` / `event_loop.complete_publishes` 体现官方语义；prompt 只表达目标（内联在 `event_loop.prompt`），不承担控制面语义。

2) 增加“中文版本”的 parallel E2E 场景，并多跑几次不同 prompt 变体，验证解析/路由在文本扰动下的稳定性与鲁棒性。

## 阶段
- [x] 阶段1: 现状核对与设计定稿
- [x] 阶段2: 调整 example 配置与文档
- [x] 阶段3: 轻量验证与记录归档
- [x] 阶段4: 去掉 prompt.md 依赖（内联 prompt）
- [x] 阶段5: README 中文化（examples/parallel-trigger-routing/README.md）
- [x] 阶段6: 增加中文 parallel E2E 场景
- [x] 阶段7: 多跑两次 E2E（prompt 变体）验证稳定性
- [x] 阶段8: 汇总记录（WORKLOG/notes）并准备归档

## 关键问题
1. 示例的入口/终点 topic 应该分别是什么？（预期：`starting_event=spec.start`，`complete_publishes=spec.approved`）
2. 示例的“目标 prompt”放哪里最合适？（预期：内联到 `event_loop.prompt`，避免依赖额外 prompt 文件）
3. 中文 E2E 要覆盖哪些风险？（预期：中文 prompt + 中文 hats 指令；并复用现有“prompt 内包含伪 `<event>` / fenced code block”的变体回归）

## 做出的决定
- [决定] 用 config 固化 entry/exit：在示例 `ralph.yml` 写入 `starting_event` 与 `complete_publishes`。
  - [理由] 这是 parallel-workflow-semantics 的“官方语义锚点”，示例应该带头使用。
- [决定] 目标 prompt 内联到 `event_loop.prompt`。
  - [理由] 在 macOS 上 `.gitignore` 的 `PROMPT.md` 规则会让 `prompt.md` 这类文件对外不可复现；内联能从根上消除“依赖 prompt 文件”的困惑。
- [决定] 新增一个独立的中文 parallel E2E scenario（而不是改写现有英文场景）。
  - [理由] 英文场景仍然有价值；中文场景用于覆盖“中文提示词 + 同一套语义约束”的稳定性回归，且便于定位问题归因（语言差异 vs 逻辑差异）。

## 遇到错误
- 暂无

## 状态
**已完成**：已补写 WORKLOG/ERRORFIX，并完成两次中文 E2E 变体回归；已归档 `parallel-workflow-semantics` change，并同步 delta specs 到 `openspec/specs/`。

## 日志
### 2026-01-29 00:15
- [计划] 将 README 内容翻译为中文，但保留所有代码/配置 key、命令、topic 名称不变，避免读者复制运行时出错。

### 2026-01-29 00:17
- [完成] README 中文化已完成，示例语义未变；`cargo test -q` 全部通过。

### 2026-01-29 00:30
- [计划] 增加中文 parallel E2E 场景，并用 prompt 变体（含伪 `<event>` 与 fenced code block）多跑两次，观察是否会出现误解析、误路由或提早/卡死等不稳定行为。

### 2026-01-29 01:50
- [完成] 新增中文并行 E2E 场景：`parallel-hat-instances-zh`（Codex）。
- [完成] E2E 稳定性验证（两次 prompt 变体）：
  - `variant1`：✅ 通过（约 98s）
  - `variant2`：✅ 通过（约 119s）
- [改良] E2E WorkspaceManager：每次创建 workspace 前先清理旧目录，避免 `--keep-workspace` 后再次运行导致的历史产物污染与误判。

### 2026-01-29 02:10
- [计划] 使用 `openspec archive parallel-workflow-semantics` 归档该 change，并让 OpenSpec 将 delta specs 合并到 `openspec/specs/` 主规格中。
- [计划] 归档后复核 `openspec/specs/parallel-hat-instances/spec.md` 与 `openspec/specs/parallel-trigger-routing/spec.md` 是否包含 starting_event / complete_publishes / orphan→ralph#1 等“官方语义锚点”。

---

# 任务计划: 理性合并 preset 配置更新（commit: 7a346bd）

## 目标
把 `7a346bd425cf2d7a45d086875eba413a21111744` 里的“preset 配置改良”合并到当前分支。
只保留对工作流收敛、可执行性、测试时长预算有帮助的改动。

## 阶段
- [x] 阶段1: 审阅 commit 变更点与影响范围
- [x] 阶段2: 应用改动（不创建新 commit，只落地文件差异）
- [x] 阶段3: 运行 `cargo test` 做回放/单测验证
- [x] 阶段4: 记录合并结论（notes/WORKLOG，必要时 ERRORFIX）

## 关键问题
1. 这次变更是否会影响事件语义（publish vs LOOP_COMPLETE）以及 preset 的“停机”条件？
2. `tools/preset-test-tasks.yml` 的复杂度与 timeout 调整，是否会影响现有测试基准与期望？
3. 是否存在与当前分支同文件改动的冲突，需要做“择优合并”而不是整块覆盖？

## 做出的决定
- [决定] 优先按“整 commit 差异”落地，再用测试做背压验证。
  - [理由] 该 commit 只改 YAML 配置，风险可控；且改动方向统一（更务实、更易停机、时间预算更贴近真实）。
- [决定] 不执行 `git cherry-pick` 产生新 commit，只应用变更到工作区。
  - [理由] 遵循当前协作约定：除非明确要求，否则不自动创建提交。

## 遇到错误
- 暂无

## 状态
**已完成**：已把 `7a346bd` 的 preset 改良落地到工作区，并通过 `cargo test` 验证无回归。

## 日志
### 2026-01-29 12:35
- [完成] 审阅 `7a346bd` 差异：确认主要价值点为“更务实的 review 收敛策略”和“更可靠的 LOOP_COMPLETE 停机语义”。
- [完成] 应用 7 个 YAML 文件差异（使用 `git show | git apply`，未创建新 commit）。
- [完成] `cargo test` 全通过。

---

# 任务计划: 理性合并 TUI hang 修复（commit: 685526d）

## 目标
把 `685526d8b901a19f73774e7f2c80bb22494dd1c2` 中“避免在 `npx` 进程组下 TUI 卡死”的修复合并到当前分支。
同时尽量不破坏“进程组用于清理子进程”的既有语义。

## 方案（给自己看的取舍）
1) 不惜代价，最佳方案：
- 保持“自己成为进程组 leader”的能力，但在 TTY 场景下，必要时将新进程组设置回前台（需要 `tcsetpgrp`，并处理权限/失败分支）。
2) 先能用，后面再优雅（本次选择）：
- 按 upstream commit 的做法：当当前进程组就是前台 TTY 进程组时，跳过 `setpgid`，避免 TUI 输入被“踢出前台组”导致挂死。
- 代价是：在某些 wrapper 场景下我们不再强制成为 group leader，但能换来“交互可用性”。

## 阶段
- [x] 阶段1: 审阅差异与现状（main.rs 的 process group 初始化逻辑）
- [x] 阶段2: 落地代码改动（含必要的风格/日志改良）
- [x] 阶段3: `cargo test` 全量验证
- [x] 阶段4: 提交（带来源说明）+ 记录结论到 notes/WORKLOG

## 关键问题
1. 什么时候会出现 “npx process group” 触发的 TUI 卡死？（预期：我们调用 `setpgid` 把自己移出前台 TTY 组，导致输入不再送达）
2. 跳过 `setpgid` 是否会导致 orphan 清理能力下降？（预期：是 trade-off，但在 wrapper 场景下优先保证交互）
3. 是否需要额外日志/调试信息帮助以后定位（比如打印 pgrp/fg pgrp）？（预期：用 `debug!`，避免默认噪音）

## 做出的决定
- [决定] 采用 upstream 的“前台 TTY 组检测 + 安全跳过 setpgid”修复，并做少量 Rust 风格改良（inlined_format_args）。
  - [理由] 这是最小化、可回归测试的修复；且能直接解决“交互挂死”这种硬故障。

## 遇到错误
- 暂无

## 状态
**已完成**：已将 `685526d` 的修复落地到 `crates/ralph-cli/src/main.rs`，并通过 `cargo test` 验证无回归。

## 日志
### 2026-01-29 13:10
- [完成] 审阅 `685526d` 差异：确认这是“避免 npx/wrapper 场景下 TUI 输入挂死”的关键修复。
- [完成] 落地 process group 初始化保护逻辑（检测前台 TTY 进程组，必要时跳过 `setpgid`），并按 Rust 风格改用 inlined_format_args。
- [完成] `cargo test` 全通过。
