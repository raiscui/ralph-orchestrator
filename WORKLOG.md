# WORKLOG
#
# 说明：
# - 本文件用于追加记录“完成的工作与验证结果”（面向未来回看）。
# - 当文件超过 1000 行会按时间戳轮换为 `WORKLOG_YYYY-MM-DD_HHMM.md`，避免变成巨无霸难以检索。
#
# 上一个轮换文件：
# - `WORKLOG_2026-02-03_1655.md`

## 2026-02-03 16:55 +0800｜Hat Graph Radar：Running 高亮色 #a9dc76；并行模式边动画基于 source_instance 触发

### 你指出的关键点
- `examples/parallel-trigger-routing/.ralph/events.jsonl` 里 publisher 字段是 `hat`，不是 `source`。
- 因此并行模式不应强行把 `event.source` 自动填成当前实例的 hat_id。

### 我做了什么
- `crates/ralph-tui/src/theme.rs` / `crates/ralph-tui/src/app.rs`
  - Running hat box 高亮色固定为 `#a9dc76`（前景色）。
- `crates/ralph-cli/src/parallel_runner.rs`
  - 并行事件转发策略：`gate.*` / `human.message` / `source_instance` / `source` 事件进入 TUI，避免漏掉业务事件。
- `crates/ralph-tui/src/state.rs`
  - Radar 动画触发：优先用 `event.source`；否则用 `event.source_instance.split_hat_id()` 推导发布者 hat。
- `crates/ralph-core/src/parallel/instance.rs`
  - 回滚：不再自动补齐 `event.source`（保持协议语义更原样）。
  - 保留：继续补齐 `event.source_instance` 与 `event.id`（用于归因与回放）。

### 验证
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅

---

## 2026-02-04 20:13 +0800｜补充：新增 parallel-experimental-dev-engine example 的 Codex E2E 场景

### 交付物

- 新增专用 E2E scenario（直跑 example，E2E workspace 预填 EXPERIMENT_PLAN，断言 topic 链路 + patch + LOOP_COMPLETE）：
  - `crates/ralph-e2e/src/scenarios/parallel_experimental_dev_engine_example.rs`
- 注册并导出该 scenario（可被 `ralph-e2e --list` / runner 发现）：
  - `crates/ralph-e2e/src/scenarios/mod.rs`
  - `crates/ralph-e2e/src/lib.rs`
  - `crates/ralph-e2e/src/main.rs`
- OpenSpec 同步：补充“必须有专用 Codex E2E 场景”的 MUST 约束，并追加 tasks：
  - `openspec/changes/parallel-hat-solution-eval-example/specs/parallel-experimental-dev-engine/spec.md`
  - `openspec/changes/parallel-hat-solution-eval-example/tasks.md`

### 验证（Backpressure）

- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-e2e` ✅（确保新增 scenario 编译通过）
---

## 2026-02-03 23:41 +0800｜TUI：取消 Radar 高对比模式（移除 c 切换）

### 做了什么

- `crates/ralph-tui/src/state.rs`
  - 删除 `hat_graph_high_contrast`（不再保存/切换高对比偏好）
- `crates/ralph-tui/src/input.rs`
  - 删除 `ToggleHatGraphHighContrast` 与 `c` 键映射
- `crates/ralph-tui/src/app.rs`
  - 删除 reducer 分支与并行模式非 Chat 场景的 `c` 全局快捷键处理
  - `apply_hat_graph_radar_scan_head` 去掉 `high_contrast` 参数与配色分支（只保留默认 stop）
  - 删除/更新相关回归测试
- `crates/ralph-tui/src/widgets/help.rs`、`specs/terminal-ui.spec.md`
  - 移除 `c` 的 help/spec 文档描述，避免“幽灵功能”

### 验证

- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅

---

## 2026-02-03 23:40 +0800｜TUI：拖尾加长 + 扫描头提亮 + base 边变暗（让动效更聚焦）

### 做了什么

- 拖尾加长
  - `crates/ralph-tui/src/state.rs`
    - `HAT_GRAPH_EDGE_HEAD_LEN: 16`（扫头更长，拖尾更明显）
- 扫描头整体提亮（不再把拖尾压暗）
  - `crates/ralph-tui/src/app.rs`
    - 去掉 tail 的 `DIM` 依赖（避免“拖尾变暗”的观感）
    - normal 渐变改为 `blue -> lavender -> text`
    - BOLD 区间扩大（更亮的方向提示）
- base 高亮边变暗（降低抢眼程度）
  - `crates/ralph-tui/src/app.rs`
    - base 高亮色从 `sapphire` 改为 `overlay1`
- 测试更新
  - `crates/ralph-tui/src/app.rs`
    - 更新扫描头回归测试，锁死“bg 不改、拖尾有渐变、tip BOLD 更亮”的规则

### 验证

- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅

---

## 2026-02-03 23:10 +0800｜TUI：Radar 扫描头去掉 bg + truecolor 渐变拖尾 + 高对比模式（c）

### 你最新要求

- 去掉扫描头的轻微发光底色（bg）。
- 扫描头更高级：tip 更亮，tail 更长更柔和。
- 增加一档高对比模式（切换更醒目的扫描头配色）。

### 做了什么

- 扫描头渲染（去掉 bg + truecolor 渐变 + 更长拖尾）
  - `crates/ralph-tui/src/app.rs`
    - `apply_hat_graph_radar_scan_head` 移除 `bg` 上色。
    - 改为两段插值渐变（normal 冷色系 / high-contrast 暖色系）。
    - 通过 `DIM/BOLD` 做 tail/tip 层次，避免再靠 bg“发光”。
- 拖尾长度
  - `crates/ralph-tui/src/state.rs`
    - `HAT_GRAPH_EDGE_HEAD_LEN: 6 → 10`
- 高对比模式开关（纯 UI）
  - `crates/ralph-tui/src/state.rs`：新增 `hat_graph_high_contrast: bool`
  - `crates/ralph-tui/src/input.rs`：`c` 绑定为 `ToggleHatGraphHighContrast`
  - `crates/ralph-tui/src/app.rs`：
    - 串行：走 `map_key → dispatch_action`
    - 并行：非 Chat 输入场景下 `c` 切换（Chat 输入框里不抢字符）
    - Radar 标题显示 `c: std/HC`
- 帮助与规格同步
  - `crates/ralph-tui/src/widgets/help.rs`：help 增加 `c` 说明
  - `specs/terminal-ui.spec.md`：补充 `c` 的语义与“输入上下文不触发”约束

### 验证

- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅

---

## 2026-02-03 18:15 +0800｜Hat Graph Radar：事件线动画改为“跟随 Running 目标”的短动画

### 你最新确认的 UX 规则
- event 线路不需要持续很久。
- 如果线路指向的目标 box 不再 Running，则立刻取消该线路动画。
- 如果有新的 box 进入 Running，则新 box 染色，并同时显示“导致它 Running 的 event”线路动画。

### 我做的改动（只改 TUI 行为，不扩协议）
- `specs/terminal-ui.spec.md`
  - 删除“循环播放 + 60s 驻留”的旧要求。
  - 改为“按 Running 目标驱动”的短动画，并写清取消/触发规则。
- `crates/ralph-tui/src/state.rs`
  - 新增两层状态：
    - `hat_graph_recent_events`：记录最近业务事件（用于推断 cause event）。
    - `hat_graph_edge_animations`：按 `target_hat` 保存短动画（从非 Running → Running 时启动）。
  - 在 `ParallelInstanceState` 更新里捕捉 Running 跃迁：
    - 进入 Running：从 recent events + graph meta 推断 cause event，启动动画。
    - 退出 Running：若该 hat 已无 Running 实例，则立刻取消动画。
  - `tick_hat_graph_radar_animation` 改为“清理过期事件/动画 + 目标不 Running 即移除”。
- `crates/ralph-tui/src/app.rs`
  - 渲染侧不再依赖“全局最新 event”。
  - 改为遍历 `hat_graph_edge_animations`，只对 Running 目标绘制 progressive reveal，超时即消失。

### 验证
- `cargo fmt` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅

---

## 2026-02-03 17:08 +0800｜OpenSpec：创建新 change `parallel-hat-solution-eval-example`（并行 hat 多方案评估 example）

### 目标（你提出的需求）
- 新增一个 example + 并行 hats 框架，用于“同一改动存在多条可行实现路径，需要都实现并通过测试验证后再决策”的场景。

### 我做了什么（按 artifact 驱动流程）
- 创建 change：`openspec/changes/parallel-hat-solution-eval-example/`（默认 schema：`spec-driven`）
- 查看 artifacts 状态：`0/4`
  - `proposal`：ready
  - `design`：blocked by proposal
  - `specs`：blocked by proposal
  - `tasks`：blocked by design, specs
- 仅获取首个 artifact 指引（不创建 artifact）：
  - `openspec instructions proposal --change "parallel-hat-solution-eval-example"`
  - proposal 输出文件位置：`openspec/changes/parallel-hat-solution-eval-example/proposal.md`

### 下一步（等待你确认后继续）
- 起草并提交 `proposal.md` 后，才会解锁 `design` 与 `specs` 两个 artifacts。

---

## 2026-02-03 17:31 +0800｜OpenSpec：`parallel-hat-solution-eval-example` 创建 `proposal.md`

### 我做了什么
- 继续 change：`parallel-hat-solution-eval-example`
- 创建 artifact：`proposal`
  - 写入：`openspec/changes/parallel-hat-solution-eval-example/proposal.md`

### 结果
- Schema：`spec-driven`
- Progress：`1/4`
- 解锁情况：`design` 与 `specs` 已变为可创建；`tasks` 仍被 `design/specs` 阻塞

---

## 2026-02-03 19:03 +0800｜OpenSpec：`parallel-hat-solution-eval-example` 创建 `design.md`

### 我做了什么
- 创建 artifact：`design`
  - 写入：`openspec/changes/parallel-hat-solution-eval-example/design.md`

### 结果
- Schema：`spec-driven`
- Progress：`2/4`
- 解锁情况：
  - `specs` 仍可创建（下一步应创建 specs）
  - `tasks` 仍被 `specs` 阻塞（已不再依赖 design）

---

## 2026-02-03 17:55 +0800｜Hat Graph Radar：向内偏移再下移 + 线动画循环播放 + 60s 驻留 + event.source 补齐

### 你反馈的现象
- Radar 覆盖层仍会遮挡 Output 的边线，需要再向下偏移一点。
- 你看不到 event 线段动画（或只是一闪而过）。
- 你指出 `.ralph/events.jsonl` 明明有 `hat/source_instance`，不应该“拍脑袋填当前实例”。

### 我做了什么（核心改动）

- 布局偏移（避免遮挡边线）
  - `crates/ralph-tui/src/app.rs`：`HAT_GRAPH_RADAR_INSET_Y` 从 `3` 增加到 `4`。
- 线动画：从“播完就停”改为“循环播放”
  - `crates/ralph-tui/src/app.rs`：边动画改成“按步进取模”循环渲染，不再依赖 `elapsed <= total_ms`。
  - `crates/ralph-tui/src/app.rs`：在每帧 render tick 调用 `state.tick_hat_graph_radar_animation(now)`，
    让 `pending` 能在 **60 秒最小驻留**到期后切换生效。
- event source 归因（发布者 hat）
  - `crates/ralph-core/src/parallel/instance.rs`：在 `decorate_outgoing_event` 里补齐 `event.source=hat_id`（仅在缺失时补齐，不覆盖已有值），并继续保留 `source_instance`。
  - 这样 TUI/诊断可以直接用 `event.source`，同时 `source_instance` 仍用于实例级归因。

### 你问的“Radar 图是用 CLI 还是 crate？”
- 现在是 **直接集成 `beautiful-mermaid-rs` crate**（不是 shell 调 CLI）。
  - 入口：`crates/ralph-cli/src/hats.rs`：`render_mermaid_ascii_with_meta(...)` 生成 Unicode 图 + meta
  - TUI：`crates/ralph-tui/src/app.rs` 用 meta 做 cell 级高亮与动画（不拼 ANSI 字符串）

### 验证
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅

---

## 2026-02-03 19:47 +0800｜交付：并行实验开发永动机（Parallel Experimental Dev Engine）配置方案

### 产物（可直接复制使用）

- 配置：`examples/parallel-experimental-dev-engine/ralph.yml`
- 说明：`examples/parallel-experimental-dev-engine/README.md`
- 回放夹具：`crates/ralph-core/tests/fixtures/parallel_experimental_dev_engine.jsonl`
- 回归测试：`crates/ralph-core/tests/smoke_runner.rs`（新增该 fixture 的 exists + full replay flow 校验）

### 这份 ralph.yml 解决什么问题

- 面向“探索型开发任务”：
  - 同一目标需要多轮试验、多轮验证，且希望并行跑起来。
- 把“怎么做 / 怎么验证”的责任交还给用户：
  - 用户在 `EXPERIMENT_PLAN` 里写清楚实现步骤与验证命令。
  - Ralph 只负责并行化、结构化、强制产出验证证据，并收敛结束。

### 配置关键点（为什么这样配）

- 两类 hats 分工明确（减少特殊情况，提升确定性）：
  - `experiment_dispatcher`：只拆分与派发 `experiment.task`，不跑工具、不改文件。
  - `experiment_runner`：多实例并行执行，每个任务必须“实现 + 验证”，并发布 `experiment.result`。
- worktree 隔离 + 产物导出（避免 worktree 回收丢改动）：
  - runner 使用 `workspace.strategy=worktree`。
  - runner 必须在 result 里给出 `patch` 或 `commit`（至少一个），用于把改动带回主工作区。
- 可收敛的入口/完成语义（三件套）：
  - `event_loop.starting_event=experiment.start`
  - `event_loop.complete_publishes=experiment.complete`
  - `event_loop.completion_promise=LOOP_COMPLETE`
- 安全刹车（防止“永动机”卡死）：
  - `event_loop.max_iterations` / `event_loop.max_runtime_seconds`
  - `parallel.autoscale.max_running_jobs` / `dynamic_idle_ttl_secs`
  - runner `job_timeout_secs`

### 如何运行（仓库根目录）

```bash
cargo run --bin ralph -- run -c examples/parallel-experimental-dev-engine/ralph.yml --no-tui
```

### 验证

- `cargo test -p ralph-core --test smoke_runner` ✅

## 2026-02-03 21:25 +0800｜TUI：Hat Graph Radar “常亮边 + 跑动高亮头”扫描动效

### 做了什么
- `crates/ralph-tui/src/state.rs`：
  - 新增渲染计划纯函数 `plan_hat_graph_radar_edge_animation`（reveal → full → scan）
  - 新增扫描头参数：`HAT_GRAPH_EDGE_HEAD_STEP_MS`、`HAT_GRAPH_EDGE_HEAD_LEN`
  - 新增单测：`hat_graph_edge_render_plan_reveals_then_scans_until_cancelled_by_running_state`
- `crates/ralph-tui/src/app.rs`：
  - Radar 线路渲染改为“两层上色”：
    - base：全亮路径（`sapphire`）
    - head：短段扫描头（`sky` + `BOLD`），reveal 阶段贴前沿，reveal 后循环移动
- `specs/terminal-ui.spec.md`：补充扫描头行为描述
- `task_plan.md` 超 1000 行轮换：
  - `task_plan_2026-02-03_2105.md`（归档）
  - `task_plan.md`（新）

### 验证
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅

---

## 2026-02-03 20:45 +0800｜TUI：Hat Graph Radar 事件连线动画补点（修复“半截消失/断线观感”）

### 做了什么
- `crates/ralph-cli/src/hats.rs`：
  - 注入 radar meta 时，把 `edge.path` 从“关键点序列”补齐为“逐 cell 的连续路径”
  - 增加单测锁死补点规则（水平/垂直补齐 + 非正交兜底）
- `crates/ralph-tui/src/app.rs`、`crates/ralph-tui/src/state.rs`：
  - 同步修正 Radar tick/动画相关注释口径（避免继续误导为“60 秒驻留/短动画”）

### 验证
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅

---

## 2026-02-03 19:52 +0800｜纠错：该需求仅补充到 OpenSpec change（不落盘实现）

### 为什么要纠错

- 你澄清：你要的是把方案补充到 change：`parallel-hat-solution-eval-example`，而不是现在就进入 apply/实现阶段。
- 因此需要把误落到主仓库实现层的 example/fixture/smoke test 撤回，回到“artifact 驱动”流程。

### 我做了什么

- 回滚实现层落盘内容（不再在主仓库新增 example/fixture/tests）：
  - 删除：`examples/parallel-experimental-dev-engine/`
  - 删除：`crates/ralph-core/tests/fixtures/parallel_experimental_dev_engine.jsonl`
  - 同步移除 smoke_runner 中对应的 fixture 校验用例
- 把可复制的草案内容补充进 change artifacts（供你 review）：
  - `openspec/changes/parallel-hat-solution-eval-example/design.md` 新增 Appendix：
    - `ralph.yml` 草案
    - `README.md` 草案

### 验证

- `cargo test -p ralph-core --test smoke_runner` ✅

---

## 2026-02-03 21:40 +0800｜TUI：Hat Graph Radar 扫描头渐变/发光（质感 + 对比度增强）

### 做了什么

- `crates/ralph-tui/src/app.rs`
  - 扫描头从“单色 + BOLD”升级为“渐变 + 轻微发光底色（bg）”。
  - 仍保持“两层渲染”结构：
    - base：常亮全路径（用于表达“cause event 线路持续有效”）
    - head：短段循环扫描头（用于表达“仍在运行”）
- `crates/ralph-tui/src/app.rs`
  - 新增回归测试 `hat_graph_radar_scan_head_uses_gradient_and_glow_bg`，
    锁死 tip 必须“最亮 + BOLD + 更亮 bg”的对比度规则。

### 验证

- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅

---

## 2026-02-04 00:27 +0800｜补充：仅更新 OpenSpec change（`parallel-hat-solution-eval-example`），不落盘实现

### 你澄清的目标（我为什么要这样做）

- 你要的是“研究出一个适用于并行方案评估/多轮实验探索的 `ralph.yml` 配置范式”。
- 这一步应该先沉淀到 OpenSpec change（proposal/design/spec/tasks）。
- 你明确不要进入 apply/实现阶段（不在主仓库新增 example/fixture/tests）。

### 我做了什么（落在 change artifacts）

- `openspec/changes/parallel-hat-solution-eval-example/design.md`
  - 明确“窗口化派发（in-flight window）”：
    - 以 `experiment.reviewed(evidence_ok=true)` 作为释放 slot 的完成信号
    - 禁止洪水式一次性派发全部实验
  - 明确“自适应并行度（激进 + AIMD）”：
    - `P_max` 由 `ralph#1` 基于用户 plan/prompt 推断
    - 运行中 `P += 1` / `P = floor(P/2)` 动态调参
    - 强护栏：`P <= parallel.autoscale.max_running_jobs - 2`（预留 `ralph#1` + `auditor`）
  - 新增独立 `experiment_auditor`（硬门槛审计：证据不足必须拒绝收敛）
  - 新增独立 `experiment_integrator`（主工作区采纳/集成/最终验收：消费 `integration.task`，产出 `integration.applied`/`integration.rejected`）
  - 收紧证据/产物口径：runner 必须提供 `patch`（`commit` 仅可选补充信息；auditor/integrator 以 patch 作为最低审计载体）
  - 修复 README 草案里的 Mermaid 图表语法（label 含括号需加引号）
  - 同步“生产建议”权限口径：`worktree: ask`、`hooks: allow`，并提示可用 `parallel.gate.default_timeout_secs` 做超时策略
- `openspec/changes/parallel-hat-solution-eval-example/specs/parallel-experimental-dev-engine/spec.md`
  - 补充 gate timeout 的 guardrail 要求（`parallel.gate.default_timeout_secs`）
  - 补充 README 必须包含生产权限建议片段（`worktree: ask` / `hooks: allow`）
  - 补充 integrator 的 MUST 约束（主工作区 `workspace.strategy=shared`，并产出可审计的集成结果事件）

### 验证

- Mermaid：已用 `mermaid-validator` 校验并修复 parse error（括号 label 引号化）

---

## 2026-02-04 16:24 +0800｜apply：parallel-hat-solution-eval-example（落地并行实验开发永动机 example + 回放门禁）

### 交付物

- 新增示例配置与文档：
  - `examples/parallel-experimental-dev-engine/ralph.yml`
  - `examples/parallel-experimental-dev-engine/README.md`
- 补充仓库文档入口（parallel runnable examples）：
  - `README.md`
- 新增 replay fixture（用于 CI/本地确定性回放）：
  - `crates/ralph-core/tests/fixtures/parallel_experimental_dev_engine.jsonl`
- 新增 smoke tests（锁死关键语义：topic 链路 + 归因前缀 + patch + LOOP_COMPLETE）：
  - `crates/ralph-core/tests/smoke_runner.rs`

### 验证（Backpressure）

- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅
