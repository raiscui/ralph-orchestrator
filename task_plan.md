# 任务计划：彻底回退 mdfried 相关功能 + 许可证回退 + 移除 stderr 前缀列

## 目标

- 执行“彻底回退”（你选择了方案 A）：
  - 移除 Big Headers / 图片块等 `mdfried` 相关渲染特性
  - Output 面板回到纯文本渲染（继续使用 `termimad`）
- 许可证回退到“项目原本”许可（计划回到 MIT，并同步更新仓库元数据与文档）
- 并行 Output 面板取消左侧红色 `E` 前缀列
  - stderr 用“灰色弱化”区分即可
- 保证 `cargo fmt --check`、`cargo clippy -- -D warnings`、`cargo test`、replay smoke tests 全部通过

## 方案（至少二选一）

### 方案 A：彻底回退（你已选择）

- 移除 Big Headers / 图片块等 `mdfried` 相关渲染特性
- Output 面板完全使用 `termimad` 的文本渲染结果

### 方案 B：最小回退（先恢复 termimad，保留其他 UI 改良）

- 仅把“Markdown→ANSI/Line”渲染器切回 `termimad`
- 其余与渲染器无关的 TUI 结构改良（例如前缀列/并行面板布局）保持不动
- 后续如需再“彻底回退”，再做第二步

> 你已明确选择：方案 A（彻底回退）。

## 阶段

- [x] 阶段1：盘点需要撤回的功能点
- [x] 阶段2：移除 Big Headers / 图片渲染与相关依赖
- [x] 阶段3：移除 Output 左侧红色 `E` 前缀列
- [x] 阶段4：许可证回退到 MIT（含 docs/README）
- [x] 阶段5：全量验证与四文件记录

## 关键问题（默认先做最小回退）

1. 许可证目标：我按“回到 MIT”执行（与 docs 里 Historical Notice 一致）。
   - 若你要 `MIT OR Apache-2.0` 之类的双许可，需要你再明确一下。

## 遇到错误

- [记录] `cargo fmt --check` 发现末尾空行差异，已用 `cargo fmt` 修复并通过复检。

## 状态

**已完成**：
已彻底移除 Big Headers/图片渲染与左侧 `E` 前缀列，并将许可证回退到 MIT。
全量验证已通过（fmt/clippy/test/smoke）。

## 日志

### 2026-01-30 12:49 +0800

- [确认] 你要求执行：
  - 方案 A：彻底回退 Big Headers/图片块
  - 许可证回退（预期回到 MIT）
  - 取消 Output 面板左侧红色 `E` 前缀列

### 2026-01-30 13:34 +0800

- [完成] Big Headers/图片渲染与相关依赖已移除；Output 前缀列已移除（不再显示红色 `E`）。
- [完成] 许可证已回退到 MIT，并同步更新 `Cargo.toml` / `LICENSE` / README / docs。
- [验证] `cargo fmt --check` / `cargo clippy -- -D warnings` / `cargo test` / `cargo test -p ralph-core smoke_runner` / `cargo test -p ralph-core kiro` 全部通过。

---

# 任务计划：termimad 渲染的 H1 从居中改为左对齐

## 目标

- `termimad` 渲染 Markdown 时，H1（`# Title`）不再居中，改为靠左对齐。
- 行为在两条渲染路径保持一致：
  - stdout（Pretty 输出）
  - TUI（转为 `ratatui::Line`）
- 保证 `cargo fmt --check`、`cargo clippy --all-targets --all-features -- -D warnings`、`cargo test` 全部通过。

## 阶段

- [x] 阶段1：定位 termimad 的 H1 对齐配置点
- [x] 阶段2：实现自定义 `MadSkin`（只改 H1 对齐）
- [x] 阶段3：补回归测试（验证 H1 不再被填充左侧空格）
- [x] 阶段4：验证并提交变更

## 状态

**已完成**：
已把 termimad 渲染的 H1 对齐从居中改为左对齐，并保证 stdout/TUI 两条渲染路径一致生效；同时补充了回归测试并完成全量验证。

## 日志

### 2026-01-30 22:17 +0800

- [启动] 新需求：termimad 渲染的 H1 从“居中”改为“靠左对齐”。
- [计划] 先调研 `MadSkin` / header style 的对齐 API，再统一封装一个 skin builder，最后补测试并提交。

### 2026-01-30 22:22 +0800

- [完成] 在 `default_markdown_skin()` 中把 `headers[0].align` 设置为 `Alignment::Left`，H1 不再居中。
- [测试] 新增回归测试：`markdown_h1_is_left_aligned_in_rendered_mode`。
- [验证] `cargo fmt --check` / `cargo clippy --all-targets --all-features -- -D warnings` / `cargo test` 全部通过。

---

# 任务计划：continuous-learning（持续学习）+ 清理未跟踪文件残留

## 目标

- 产出“四文件摘要”，明确本次可复用点与是否需要提取/更新 skill。
- 清理工作区：把本次会话产生的 `task_plan_*.md` / `notes_*.md` / `WORKLOG_*.md` 等历史版本归档到 `archive/`。
- 删除明显无用的未跟踪残留（例如重复的 example 目录），让 `git status` 结果干净可控。
- 必要时创建一次“chore/cleanup”提交，把归档动作固定在 git 历史里，避免下次继续积累噪音。

## 阶段

- [x] 阶段1：盘点 `git status`（确认只剩未跟踪文件）
- [x] 阶段2：阅读四文件与历史版本（排除 `archive/**`）
- [x] 阶段3：输出“四文件摘要”并决定是否提取/更新 skill
- [x] 阶段4：归档/清理未跟踪文件（移动到 `archive/` / 删除多余目录）
- [x] 阶段5：复核 `git status` 并提交（如需要）

## 状态

**已完成**：
已产出“四文件摘要”，并新增一个可复用的 `self-learning.*` 技能；同时把历史版本文件归档到 `archive/`，并清理掉重复的 example 目录；最后已创建一次归档提交并确认 `git status` 干净。

## 日志

### 2026-01-30 22:54 +0800

- [启动] 进入 `continuous-learning`：先全量检索/阅读四文件与历史版本，再做摘要与归档，避免“凭记忆提交/误删”。

### 2026-01-30 23:03 +0800

- [摘要] 已把“四文件摘要（用于决定是否提取 skill）”写入 `notes.md`，并确认需要新增一条 termimad 相关 skill。
- [归档] 已将本次会话产生的 `notes_*.md` / `task_plan_*.md` 归档到 `archive/`；并把历史 `WORKLOG_2026-01-29_*.md` 也移动到 `archive/`，降低根目录噪音。
- [清理] 已删除重复的未跟踪目录：`examples/parallel-trigger-routing2/`。
- [提交] 已创建归档提交：`f4de8c5`（`chore: archive session notes and plans`），并确认当前工作区无未提交修改。

---

# 任务计划：理性整合一组上游提交（backend args / hats topology / presets / events / scratchpad）+ Mermaid ASCII 改用 beautiful-mermaid-rs

## 目标

- 将指定提交中的“可复用价值点”整合进当前代码线，避免把上游实现细节/副作用原样搬进来。
- 特别要求：合入 hats 拓扑/图表相关能力时，Mermaid ASCII 渲染必须改用本机仓库 `/Users/cuiluming/local_doc/l_dev/my/rust/beautiful-mermaid-rs`。
- 保证回归验证通过：
  - `cargo fmt --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
  - replay smoke tests（`ralph-core` 的 `smoke_runner` / `kiro`）

## 输入（待整合提交）

1. 理性提取：`988541883f328b897b034cbb0f8dbc8bc6046a9c`
2. 整合（但 Mermaid ASCII 用 `beautiful-mermaid-rs`）：`26f2364566fbe1d35880d889b836e5b55d343301`
3. 理性整合：`ec58e14bb6f95aa8b705f478881a9d754315219e`
4. 理性整合：`887ea9972c9877f72e20f3e60a821d32b5a249c7` / `70f224b4f61bfa6e6862236ce5ccb7b006765886` / `eb1f7e0e4ea585bbefd895b70c2a0959bcc0c02d` / `413dae5675a91fa7b3cdf5479accc9f747480c75`
5. 评估是否采用：`0fc152cf6a8ec53e4f0f25d3259905ae36d94d29`
6. 整合：`e1727dcb39c4f389d2137bb11694665a6487aaac`

## 方案（至少二选一）

### 方案 A：最佳方案（偏“长期可维护”）

- 逐提交审阅（commit message + diff），把“需求点/行为点”抽象成可测的 acceptance criteria。
- 对“用户入口/配置/日志/事件存储”类改动尽量保持向后兼容（但不为兼容牺牲结构）。
- 对 hats 图表能力：
  - 把 Mermaid 渲染做成独立模块/trait（编译期可替换），避免 CLI 逻辑里散落渲染细节。
  - 默认走 `beautiful-mermaid-rs`（本机路径依赖），并提供清晰错误信息/降级策略。
- 补/修回归测试：覆盖 per-hat backend args、生效优先级、事件写入原子性、scratchpad 注入/清理等关键行为。

### 方案 B：先能用（偏“快速把功能带过来”）

- 直接 `cherry-pick -n` 这些提交的主体改动，快速解决冲突后跑全量测试。
- Mermaid 渲染部分只做最小替换：把原实现替换成对 `beautiful-mermaid-rs` 的调用。
- 测试只补“最容易回归”的 1-2 条（例如 backend args 的优先级、事件写入不会产生破损 JSONL）。

> 我会默认按 **方案 A** 执行：因为你明确强调“理性提取/融合”，这本质是在追求“价值整合而非代码搬运”。

## 阶段

- [x] 阶段1：逐个提交盘点差异（输出到 notes.md）
- [x] 阶段2：确定合并策略与依赖顺序（尤其是 hats/配置/事件）
- [x] 阶段3：落地实现（含 Mermaid ASCII 替换为 beautiful-mermaid-rs）
- [x] 阶段4：补测试 + 全量验证（fmt/clippy/test/smoke）
- [x] 阶段5：四文件记录与后续建议（WORKLOG/ERRORFIX/notes/task_plan）

## 关键问题

1. `beautiful-mermaid-rs` 作为本机路径依赖：是否允许只在你本机可编译？（我会先按“允许”执行；如果你希望 CI/他人也能编译，我们需要把它发布/子模块化/或在本仓库内置。）
2. 这些提交里可能引入的行为变更：例如 scratchpad 注入、事件写入原子性、per-hat backend args 的优先级。最终以“可测试的行为”作为裁决标准。

## 遇到错误

- [记录] `zsh: no matches found`：我误把 hash 写成了带 `?` 的通配符，导致命令失败；已改为精确 hash 处理。
- [记录] `cargo clippy` / `cargo test` 失败：`crates/ralph-cli/src/hats.rs` 仍残留 “AI 生成图表” 相关测试（函数已删除）；已删除这些测试，改为确定性渲染链路的测试。
- [记录] `cargo test` 失败：`fresh run` 清理 scratchpad 时直接 `remove_file`，导致 `run` 后 scratchpad 不存在，`run --continue` 直接在 CLI 层报错退出；已改为“清空内容（truncate）而非删除文件”，仍能达到“fresh run 清理旧状态”的目的。
- [记录] `cargo test` 失败：`HatlessRalph` 在 active hat 场景下改为输出 `## ACTIVE HAT`（省略 `## HATS` 拓扑），导致旧断言不匹配；已更新对应测试断言。

---

# 任务计划：starting_event 的 LLM 推测正确性（E2E）

## 目标

- 新增一个 **E2E 场景**，专门覆盖：
  - `event_loop.starting_event` **未设置** 时，`ralph#1`（LLM 协调者）必须能基于 hats 拓扑 **推测并选择正确的工作流入口事件**。
  - 该入口事件必须能触发正确的 hat 链路，最终可靠收敛到 `LOOP_COMPLETE`。
- 该场景优先覆盖 **parallel runtime**，因为它的 `ralph#1` prompt 内已经提供“候选入口 topic 列表”，可把模型选择空间压缩到“拓扑合法值”，更适合做“稳定性回归”。
- 验证门槛：
  - `cargo fmt --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test -p ralph-e2e`

## 方案（至少二选一）

### 方案 A：最佳方案（更贴近“真实 LLM 推测”，同时可做确定性回归）

- 实现新的 `ralph-e2e` scenario（live backend，Codex）。
- 额外 **录制 cassette** 到 `cassettes/e2e/`，让 `ralph-e2e --mock` 也能覆盖该场景：
  - mock-mode 负责“确定性回归”（CI 友好）
  - live-mode 负责“真实模型漂移”（发布/大改前）

### 方案 B：先能用（仅 live E2E，不录制 cassette）

- 只实现 scenario + 断言，先让你可以在本地用真实后端跑起来。
- 后续再补录 cassette（如果你希望 CI/零成本回归也覆盖）。

> 我会默认按 **方案 A** 推进；如果因为本机/环境缺少后端认证导致无法录制 cassette，则会降级为方案 B，但 scenario 本身仍会完成并可运行。

## 阶段

- [x] 阶段1：规格与设计（spec + 断言口径）
- [x] 阶段2：实现 scenario（parallel：starting_event 未配置）
- [x] 阶段3：本地验证（fmt/clippy/test）
- [x] 阶段4：录制 cassette（可行则做；不可行则记录原因）
- [x] 阶段5：四文件记录 + 后续建议

## 关键问题

1. “推测正确性”的可测定义：
   - 我会把“正确”定义为：`task.start` 之后，`ralph#1` 发布的 **第一个 workflow entry 事件** 必须是拓扑里“外部入口候选”（derived candidates）中的那个（此场景会让候选集退化为单元素，从而稳定可断言）。
2. 为了降低模型漂移导致的 flaky：
   - 场景内会把 hats 链路设计成“纯路由信号”，不做复杂实现任务；
   - 通过 `event_loop.complete_publishes` 固化收敛条件（观测到某事件后输出 `LOOP_COMPLETE`）。

## 状态

**已完成**：
已新增并验证 `parallel-starting-event-inference` E2E 场景（Codex），覆盖 starting_event 未设置时 `ralph#1` 的入口推测；
同时录制 cassette 并打通 mock-mode（为 parallel 多 job 增加分段回放能力）。

## 日志

### 2026-01-31 12:20 +0800

- [完成] 新增 spec：`specs/e2e-starting-event-inference.spec.md`。
- [完成] 新增 E2E scenario：`parallel-starting-event-inference`（Tier 8，Codex only）。
- [完成] 本地验证：
  - `cargo fmt --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
  - `cargo test -p ralph-core smoke_runner`
  - `cargo test -p ralph-core kiro`
- [完成] live E2E（Codex）通过：
  - `cargo run -p ralph-e2e -- codex --filter parallel-starting-event-inference --skip-analysis --verbose --keep-workspace`
- [完成] 录制 cassette：`cassettes/e2e/parallel-starting-event-inference-codex.jsonl`。
- [修复] mock-mode 回放：新增“按调用次数分段回放”（避免 parallel 下 `ralph#1` 多 job 造成 `LOOP_COMPLETE` 提前回放）。
- [完成] mock E2E 通过：
  - `cargo run -p ralph-e2e -- --mock --filter parallel-starting-event-inference --verbose`

---

## 状态

**已完成**：指定提交的价值点已完成整合，并已通过全量验证；四文件总结已更新。

## 日志

### 2026-01-31 01:30 +0800

- [启动] 收到任务：按你给的 9 个 commit hash，做“理性提取/整合/评估是否采用”，并要求 Mermaid ASCII 改用 `beautiful-mermaid-rs`。

### 2026-01-31 01:40 +0800

- [完成] 已逐个查看 9 个提交的 diff/改动范围，并把“价值点/风险/整合策略”写入 `notes.md`。
- [下一步] 进入阶段2：确定合并顺序，并开始落地实现（阶段3）。

### 2026-01-31 02:20 +0800

- [进展] 已完成主体代码整合（backend args / hats graph / presets 镜像 / starting_event / scratchpad 注入与清理 / active hat 省略拓扑等）。
- [待办] 现在开始做收口：
  - 修复编译链路遗漏（优先处理 `task_cli::execute` 的调用点）
  - 补齐 events JSONL 原子写（避免中断时产生半行 JSON）
  - 跑全量验证（fmt/clippy/test/smoke），再进入阶段5 写四文件总结

### 2026-01-31 02:35 +0800

- [完成] 已补齐遗漏点并通过全量验证：
  - `cargo fmt --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
  - `cargo test -p ralph-core smoke_runner`
  - `cargo test -p ralph-core kiro`
- [同步] README 已更新：`ralph hats graph` 不再需要 `--backend`，并补充 `ralph run -- <BACKEND_ARGS...>` 的说明。

### 2026-01-31 02:38 +0800

- [收尾] 已更新四文件：
  - `task_plan.md`：阶段状态、遇到错误与验证结果
  - `notes.md`：最终整合结果与已知风险
  - `WORKLOG.md`：变更摘要与验证证据
  - `ERRORFIX.md`：收口过程的失败→根因→修复→验证

---

# 任务计划：回退 starting_event 的“被当作初始化事件”行为 + 强化 ralph#1 的决策提示

## 目标

- `event_loop.starting_event` **不应作为初始化事件**：
  - fresh run 的初始化事件始终是 `task.start`
  - `starting_event` 仅作为“协调后工作流入口事件”的提示（由 ralph#1 决策/发布）
- 当 `starting_event` 未设置时：
  - 明确由 `ralph#1` 自行决定下一步要发布的入口事件
  - 通过 prompt 文案把“如何决定入口事件”说清楚，减少歧义
- 同步 README 中对 `starting_event` 的错误描述（当前 README 把它写成了“First event published”，与设计相悖）
- 保证全量验证通过（fmt/clippy/test/smoke）

## 方案（至少二选一）

### 方案 A：按设计回退（你已选择）

- `EventLoop::initialize()` 固定发布 `task.start`（fresh run）。
- `starting_event` 不再影响初始化 topic，而是仅在 prompt 中生效。
- prompt 增强：
  - `starting_event` 未设置：提示 ralph#1 “必须自行从拓扑中选择一个入口事件并发布”
  - `starting_event` 已设置：提示 ralph#1 “协调后应优先发布该入口事件”
- README 对 `starting_event` 的说明改为“协调后入口事件（不是 first event）”。

### 方案 B：维持现状但打补丁（不推荐）

- 继续让 `starting_event` 影响初始化 topic（当前实现），但在 prompt/README 强行解释它的双重含义。
- 缺点：概念混乱，后续维护/用户理解成本高。

## 阶段

- [x] 阶段1：回退初始化事件语义（task.start 固定）
- [x] 阶段2：增强 ralph#1 prompt（starting_event 未设置时“由你决定”）
- [x] 阶段3：同步 README（starting_event 不是 first event）
- [x] 阶段4：全量验证（fmt/clippy/test/smoke）
- [x] 阶段5：四文件追加记录（WORKLOG/ERRORFIX/notes/task_plan）

## 状态

**已完成**：starting_event 语义已按你认可的设计回退完成；prompt 与 README 已同步；验证通过；四文件已更新。

## 日志

### 2026-01-31 03:02 +0800

- [完成] starting_event 语义回退：
  - fresh run 初始化事件固定为 `task.start`（starting_event 不再影响初始化 topic）
  - `starting_event` 仅作为“协调后工作流入口事件”提示，由 ralph#1 决策/发布
- [完成] prompt 增强：
  - 当 `starting_event` 未配置时，明确提示 ralph#1 必须自行决定入口事件，并给出启发式候选列表
  - 当 `starting_event` 已配置时，提示 ralph#1 优先遵循该入口事件
- [完成] README 同步：纠正 `starting_event` 的含义（不是 first event）
- [验证] 通过：
  - `cargo fmt --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
  - `cargo test -p ralph-core smoke_runner`
  - `cargo test -p ralph-core kiro`

---

# 任务计划：为 starting_event 推测正确性新增“变体”E2E 场景（多入口候选）

## 目标

- 在已有 `parallel-starting-event-inference` 的基础上，再补一个“更贴近真实”的变体：
  - `event_loop.starting_event` **仍然不设置**
  - hats 拓扑里存在 **多个** “derived entry candidates”（例如 `spec.start` + `docs.start`）
  - prompt 明确要求 “Planner 先跑，再跑 Builder”，因此 `ralph#1` 必须选择能触发 Planner 的入口事件
- 为该变体场景补齐 mock-mode cassette（确保确定性回归）
- 保证验证门槛全部通过（fmt/clippy/test + ralph-core replay smoke tests + ralph-e2e）

## 阶段

- [x] 阶段1：更新 spec（补充变体场景的验收口径）
- [x] 阶段2：实现变体 E2E 场景并注册到 runner
- [x] 阶段3：录制 cassette 并验证 mock-mode 可跑通
- [x] 阶段4：全量验证（fmt/clippy/test/smoke + e2e）
- [x] 阶段5：四文件追加记录 + continuous-learning 复盘

## 关键问题

1. 变体的“稳定断言口径”是什么？
   - 我将优先断言：`task.start` 后 `ralph#1` 的第一个 workflow entry event 仍应为 `spec.start`（符合“Planner 先跑”的要求）。
   - 同时补一个弱断言：`spec.start → build.task → build.done` 链路必须发生，且最终 `LOOP_COMPLETE`。

## 做出的决定

- [决定] 变体采用“多入口候选 + 明确 workflow 顺序”的设计，而不是“多入口候选 + 主观最佳选择”。
  - [理由] 后者太容易 flaky；前者的正确性可被 prompt 的显式约束锁定，可做强断言。

## 状态

**已完成**：已补齐变体场景 + cassette + 全量验证，并把关键结论追加到四文件；continuous-learning 复盘后无需新增 skill（已在 notes/README 固化必要提醒）。

## 日志

### 2026-01-31 12:45 +0800

- [启动] 你要求：再加一个 starting_event 推测的变体 E2E 场景。
- [计划] 我先更新 spec（把“多入口候选”的验收口径写清楚），然后再落地场景代码与 cassette。
- [完成] 已更新 `specs/e2e-starting-event-inference.spec.md`：补充“多入口候选下的可判定选择”变体需求与 cassette 约定。
- [完成] 已实现并注册变体场景：
  - scenario id：`parallel-starting-event-inference-multi-candidate`
  - 设计：新增 `docs` 干扰 hat（`docs.start → docs.done`），但 prompt 仍要求 Planner 先跑，因此入口事件应选择 `spec.start`。
- [完成] 已录制并验证变体 cassette：
  - `cassettes/e2e/parallel-starting-event-inference-multi-candidate-codex.jsonl`
  - mock-mode 验证：`cargo run -p ralph-e2e -- --mock --filter parallel-starting-event-inference-multi-candidate --verbose`
- [验证] 全量验证已通过：
  - `cargo fmt --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
  - `cargo test -p ralph-core smoke_runner`
  - `cargo test -p ralph-core kiro`
  - live E2E：`cargo run -p ralph-e2e -- codex --filter parallel-starting-event-inference-multi-candidate --skip-analysis --keep-workspace --verbose`
- [完成] 四文件与文档更新：
  - `WORKLOG.md` / `notes.md` / `task_plan.md`：追加变体场景落地、cassette 录制流程与验证证据
  - `crates/ralph-e2e/README.md`：补充 `--filter` 示例（子串匹配提醒）
  - `cassettes/e2e/README.md`：登记新 cassette

---

# 任务计划：拆分 `ralph-e2e` Tier8 并行场景文件（parallel.rs），回到 <1000 行

## 目标

- 将 `crates/ralph-e2e/src/scenarios/parallel.rs` 拆分为更小的模块文件：
  - 控制单文件行数（回到 <1000 行，最好每个文件 <800 行）
  - 降低后续继续新增并行场景时的维护成本（减少冲突/减少复制粘贴）
  - 保持对外 API 不变（`ParallelHatInstancesScenario` / `ParallelStartingEventInferenceScenario` 仍从 `scenarios::parallel` re-export）
- 不改变任何场景语义与断言口径（纯重构）
- 保证全量验证通过（fmt/clippy/test/smoke）

## 方案（至少二选一）

### 方案 A：将 `parallel` 变成模块目录（推荐，你已同意）

- 把 `scenarios/parallel.rs` 迁移为 `scenarios/parallel/mod.rs`
- 在 `scenarios/parallel/` 下拆分子模块：
  - `hat_instances.rs`
  - `starting_event_inference.rs`
  - `job_run_counts.rs`（共享解析/统计工具）
- `mod.rs` 负责 `pub use` 对外导出，避免上层 import 改动扩散

### 方案 B：保留单文件，只抽 helper（不推荐）

- 仍保留 `parallel.rs`，只把 helper 抽到 `parallel_helpers.rs`
- 优点：改动更小
- 缺点：单文件依旧会继续膨胀，不解决根本问题

## 阶段

- [x] 阶段1：确认是否存在子目录 `AGENTS.md` 约束 + 盘点现有引用点
- [x] 阶段2：创建 `scenarios/parallel/` 目录并完成文件拆分
- [x] 阶段3：修复编译与 `cargo fmt`
- [x] 阶段4：跑 clippy/test/smoke 验证无回归
- [x] 阶段5：四文件追加记录 + continuous-learning 复盘

## 做出的决定

- [决定] 采用方案 A（模块目录拆分）。
  - [理由] 这是“改良胜过新增”的维护性改良：降低冲突、降低认知负担，并把未来扩展点放在正确的结构里。

## 状态

**已完成**：已将并行 Tier8 场景拆分为模块目录，并通过全量验证；四文件已追加记录。

## 日志

### 2026-01-31 13:35 +0800

- [确认] 你同意我把 `crates/ralph-e2e/src/scenarios/parallel.rs` 拆分成多个模块文件（降低维护成本）。
- [完成] 已将 `parallel.rs` 拆分为目录模块：
  - `crates/ralph-e2e/src/scenarios/parallel/mod.rs`
  - `crates/ralph-e2e/src/scenarios/parallel/hat_instances.rs`
  - `crates/ralph-e2e/src/scenarios/parallel/starting_event_inference.rs`
  - `crates/ralph-e2e/src/scenarios/parallel/job_run_counts.rs`
- [验证] 通过：
  - `cargo fmt --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
  - `cargo test -p ralph-core smoke_runner`
  - `cargo test -p ralph-core kiro`
  - mock E2E：`cargo run -p ralph-e2e -- --mock --filter parallel-starting-event-inference --verbose`

---

# 任务计划：starting_event 语义对齐 + parallel instance=failed 含义说明

## 目标

- 对齐并固化 `event_loop.starting_event` 的语义（你强调的点）：
  - 如果 **starting_event 未设置**：由 `ralph#1` 基于目标与 hats 拓扑 **自行决定** workflow entry event
  - 如果 **starting_event 已设置**：`ralph#1` **必须优先遵循该 topic** 作为 workflow entry event（不应当“忽略/改口”）
- 解释你在 `examples/parallel-trigger-routing` 里看到的：instance 状态显示 `failed` 的含义，并给出最短排查路径
- 保证回归验证通过：
  - `cargo fmt --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
  - 必要时补跑相关 `ralph-e2e` 场景

## 阶段

- [x] 阶段1：梳理当前语义（代码/文档/测试）与用户期望的差异点
- [x] 阶段2：改良注释与 prompt 文案（优先不改变核心行为）
- [x] 阶段3：补/改测试锁定语义（避免回归）
- [x] 阶段4：全量验证（fmt/clippy/test + 关键 E2E）
- [x] 阶段5：四文件追加记录 + continuous-learning 复盘

## 关键问题

1. starting_event 是否要从“prompt 约束”升级为“系统自动发布”？
   - 默认先不升级（避免引入隐式控制面行为），优先通过更清晰的 prompt/注释 + 回归测试来保证语义稳定。

## 状态

**已完成**：
已对齐 `starting_event` 的“可选语义”（有配置必须遵循、无配置由 ralph#1 决定），并在 parallel 协调者 prompt 里把该语义写得更明确；同时澄清 instance 状态 `failed` 的代码口径，并完成全量验证与四文件记录。

## 日志

### 2026-01-31 15:40 +0800

- [启动] 你反馈：`starting_event` 被忽略（指向 `crates/ralph-core/src/event_loop/mod.rs:270`），并询问 parallel 模式里 instance 显示 `failed` 的含义。
- [计划] 先定位 `starting_event` 在串行/并行两条链路的“官方语义锚点”与 prompt 注入点，再给出最小改良（以测试做背压）。
- [完成] 注释语义对齐（不改变核心行为）：
  - `crates/ralph-core/src/event_loop/mod.rs`：明确 starting_event “有/无配置”的分支语义（避免误读成“总是由 ralph#1 决定”）
  - `crates/ralph-cli/src/loop_runner.rs`：同步修正同类注释，避免 CLI 层出现相反口径
- [完成] parallel 协调者 prompt 改良：
  - `crates/ralph-core/src/parallel/supervisor.rs`：在 `KEY SEMANTICS` 中补充 starting_event 的 MUST 规则（set/unset 两种情况）
- [复现] 运行 demo（确认不是 starting_event 语义导致必现 failed）：
  - `examples/parallel-trigger-routing`：`../../target/release/ralph run -c ralph.yml --no-tui --plain --verbose`
- [验证] 通过：
  - `cargo fmt --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
  - `cargo test -p ralph-core smoke_runner`
  - `cargo test -p ralph-core kiro`
  - mock E2E：`cargo run -p ralph-e2e -- --mock --filter parallel-starting-event-inference --verbose`

---

# 任务计划：并行 TUI 输出缓冲上限改为 10000 + ralph.yml 可配置

## 目标

- 将并行 Supervisor TUI 的 `max_buffer_lines` 默认值从 5000 提升到 10000。
- 把 `max_buffer_lines` 暴露为 `ralph.yml` 的配置项：`tui.max_buffer_lines`。
- 保持向后兼容：未配置该字段时，行为与默认值一致（仅默认从 5000 变为 10000）。
- 保证验证通过：`cargo fmt --check`、`cargo test`、`cargo test -p ralph-core smoke_runner`。

## 方案（至少二选一）

### 方案 A：放在 `tui:` 下（推荐，我将采用）

- `ralph-core::TuiConfig` 新增 `max_buffer_lines`，默认 10000。
- `ralph-cli` 在创建并行 TUI 时，将 `config.tui.max_buffer_lines` 注入到 `ralph-tui` 的 state。
- 这与“这个参数本质上是 UI 内存/回看上限”的语义更一致。

### 方案 B：放在 `parallel:` 下（不采用）

- 缺点：`parallel` 更像“运行时调度/资源限制”，而 `max_buffer_lines` 是“UI 回看策略”。
- 容易让用户误以为它影响 job 执行或事件回放，而实际只影响 TUI 内存窗口。

## 阶段

- [x] 阶段1：更新 `task_plan.md` 并确认落点（tui vs parallel）
- [x] 阶段2：实现配置字段 + 默认值调整（core + tui + cli）
- [x] 阶段3：更新 `ralph init` 生成模板（补充示例配置）
- [x] 阶段4：验证（fmt/test/smoke）
- [x] 阶段5：四文件追加记录（notes/WORKLOG）

## 做出的决定

- [决定] 采用方案 A：`tui.max_buffer_lines`。
  - [理由] 语义清晰：这是 UI 缓冲策略，不是并行运行时的调度策略。

## 状态

**已完成**：
默认上限已提升到 10000 行，并支持在 `ralph.yml` 里通过 `tui.max_buffer_lines` 自定义；验证已通过，四文件已追加记录。

## 日志

### 2026-01-31 17:08 +0800

- [启动] 你希望：并行 instance/job 输出不要太快丢历史，把默认上限从 5000 改为 10000，并且做成 `ralph.yml` 配置项。
- [计划] 先扩展 `ralph-core::TuiConfig`，再把配置注入到 `ralph-tui` 的并行 state，最后跑 fmt/test/smoke 做背压验证。
- [完成] 默认值调整：并行 TUI 的 `max_buffer_lines` 从 5000 → 10000。
- [完成] 配置项落地：新增 `tui.max_buffer_lines`（默认 10000）。
- [完成] CLI 注入：创建并行 TUI 时读取 `config.tui.max_buffer_lines` 并写入 state。
- [完成] `ralph init` 模板补充：在注释区新增该字段示例。
- [验证] 通过：
  - `cargo fmt --check`
  - `cargo test`
  - `cargo test -p ralph-core smoke_runner`
  - `cargo test -p ralph-core kiro`

---

# 任务计划：合并 `for_marge` 分支

## 目标

- 将 `for_marge` 分支合并到当前分支（当前为 `main`）。
- 合并后保证工作区干净，并完成背压验证（fmt/clippy/test + replay smoke tests）。

## 方案（至少二选一）

### 方案 A：标准 merge（推荐，我将采用）

- 直接在 `main` 上执行 `git merge for_marge`（避免改写历史）。
- 如果需要产生 merge commit，则接受它（可追溯、协作成本最低）。

### 方案 B：rebase 后快进（更“线性”，但默认不采用）

- 把 `for_marge` rebase 到 `main`，再快进 `main`。
- 这会改写 commit SHA；如果分支已共享/已推送，通常不划算。

## 阶段

- [x] 阶段1：合并前检查（工作区/分支存在/差异预览）
- [x] 阶段2：执行合并（含冲突处理）
- [x] 阶段3：验证（fmt/clippy/test/smoke）
- [x] 阶段4：四文件记录（notes/WORKLOG/ERRORFIX）

## 状态

**已完成**：
已完成 `for_marge` → `main` 的合并、冲突解决、验证（fmt/clippy/test/smoke）以及四文件记录。

## 日志

### 2026-01-31 22:18 +0800

- [启动] 你要求：合并 `for_marge` 分支。
- [确认] 当前在 `main`，工作区干净；`for_marge` 分支存在（且在另一个 worktree 中检出）。
- [预览] `for_marge` 相对 `merge-base` 有 2 个提交：
  - `68ccc0d`：`ui 调整`
  - `3ccf9eb`：`fix(tui): Alacritty 边框高亮 + 入场动画错峰`
- [风险] 两边在 `task_plan.md / notes.md / WORKLOG.md / ERRORFIX.md` 等文件上都有改动，合并时可能需要手工解决冲突并保证记录可读。

### 2026-01-31 22:38 +0800

- [执行] 为避免未提交的 `notes.md/task_plan.md` 阻塞 merge，我先做了 stash，再执行 `git merge for_marge --no-edit`，冲突解决后完成 merge commit。
- [完成] merge commit：`5f8f58c`（`Merge branch 'for_marge'`）。
- [处理] 关键冲突解决策略：
  - 会话记录类文件（`notes.md/task_plan.md/WORKLOG.md/ERRORFIX.md`）优先保留 `main` 的版本，避免把两边日志搅在一起；同时保留 `for_marge` 新增的历史文件（如 `*_2026-01-30_*.md`）。
  - TUI 相关：采纳 `for_marge` 的 `TuiTheme` + exabind border 方案，并修复/对齐并行输出仍使用 `ParallelOutputPane`（因为 `main` 的并行输出 buffer 类型不同）。
- [下一步] 进入验证阶段（fmt/clippy/test/smoke），用测试做背压保证合并质量。

### 2026-01-31 22:45 +0800

- [验证] 已通过：
  - `cargo fmt --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
  - `cargo test -p ralph-core smoke_runner`
  - `cargo test -p ralph-core kiro`
- [收尾] 已完成四文件记录：更新 `notes.md` / `WORKLOG.md` / `task_plan.md`（本节）。
