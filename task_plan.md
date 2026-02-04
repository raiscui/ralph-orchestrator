# 任务计划：TUI Hat Graph Radar 扫描动效（常亮边 + 跑动高亮头）

## 目标

- Radar 中的 “cause event” 连线在 reveal 完成后保持全亮；
- 同时显示一个“高亮头”（短线段）沿路径循环移动；
- 当目标 hat 退出 `Running`（进入 `Idle/Done/Failed`）时，立刻取消该连线的所有高亮与动效。

## 阶段

- [x] 阶段1：实现渲染计划（可测）+ 渲染 overlay
- [x] 阶段2：补充回归测试（锁死扫描规则）
- [x] 阶段3：验证（fmt/clippy/test + replay smoke tests）
- [x] 阶段4：四文件追加记录（notes/WORKLOG/ERRORFIX）

## 状态

**已完成**：扫描头动效已落地，并通过 fmt/clippy/test/smoke 门禁验证；相关记录已追加到 notes/WORKLOG。

---

## 2026-02-03 21:35 +0800｜增强：扫描头更有“质感”（渐变/对比度增强）

### 目标

- 扫描头从“单一颜色 + 加粗”升级为“渐变 + 轻微发光底色”，让动效更立体；
- 明显拉开与 base 线路（sapphire）的对比度，让用户更容易聚焦到“流动方向/活跃状态”。

### 阶段

- [x] 阶段1：实现扫描头渐变样式（不改状态机，仅改渲染）
- [x] 阶段2：补测试（至少锁死 tip 的样式与对比度）
- [x] 阶段3：验证（fmt/clippy/test + replay smoke tests）
- [x] 阶段4：四文件追加记录（notes/WORKLOG）

### 当前状态

**已完成**：扫描头渐变/对比度增强已落地；已通过 fmt/clippy/test/smoke 门禁；记录已追加到 notes/WORKLOG。

---

## 2026-02-03 23:05 +0800｜调整：去掉扫描头 bg 发光 + 更亮 tip/更长柔和 tail + 高对比模式

### 目标

- 去掉扫描头的“发光底色（bg）”，只用前景色与字形修饰做质感。
- 扫描头更“高级”：
  - tip 更亮、更醒目
  - tail 更长、更柔和（更像一条拖尾，而不是一小截硬高亮）
- 增加一档“高对比模式”，用于在不同终端观感下快速切换更醒目的扫描头配色。

### 方案方向（两条路）

- 方案 A（不惜代价，最佳质感）：
  - 扫描头用 truecolor 做线性渐变（多 stop），并用 `DIM/BOLD` 做强弱层次；
  - 高对比模式提供第二套 stop（暖色系）；
  - 代价：实现稍复杂，但逻辑集中在渲染函数，风险可控。
- 方案 B（先能用，后面再优雅）：
  - 扫描头依旧用离散色阶（几段固定颜色）+ `DIM/BOLD`；
  - 高对比模式只替换固定色阶；
  - 代价：实现简单，但渐变“柔和度”不如 A。

### 做出的决定

- 选择：方案 A。
- 理由：你明确要“更高级 + 更柔和 tail”，离散色阶很容易看起来像“几个硬切换块”，不够顺滑。

### 阶段

- [x] 阶段1：移除 bg 上色 + 扫描头渐变改为 truecolor 插值（normal/high-contrast 两套 stop）
- [x] 阶段2：加入/调整回归测试（无 bg、tip 更亮、tail 更长）
- [x] 阶段3：加入高对比模式开关（按键）+ help/spec 同步
- [x] 阶段4：验证（fmt/clippy/test + replay smoke tests）
- [x] 阶段5：四文件追加记录（notes/WORKLOG）

### 当前状态

**已完成**：已移除扫描头 bg；truecolor 渐变拖尾与高对比模式已落地；已通过 fmt/clippy/test/smoke；记录已追加到 notes/WORKLOG。

---

## 2026-02-03 23:34 +0800｜调优：拖尾加长 + 扫描头整体提亮 + base 线路压暗

### 目标

- 拖尾太短：把扫描头 tail 进一步加长（更像“流动的能量带”）。
- 整体提亮：扫描头整体更亮、更醒目（让用户第一眼看到“方向”和“仍在跑”）。
- 线段原色变暗：base 高亮边（常亮线路）变暗，降低它对注意力的抢夺。
  - 重点：不是让拖尾变暗，而是让 base 更暗，扫描头承担主要亮度层次。

### 阶段

- [x] 阶段1：调整 base/highlight 配色策略（base 变暗、head 变亮）
- [x] 阶段2：加长拖尾（head_len）并取消 tail 的 DIM 依赖
- [x] 阶段3：更新回归测试（锁死“base 更暗、head 更亮、拖尾更长”）
- [x] 阶段4：验证（fmt/clippy/test + replay smoke tests）
- [x] 阶段5：四文件追加记录（notes/WORKLOG）

### 当前状态

**已完成**：拖尾已加长，扫描头整体已提亮，base 高亮边已压暗；已通过 fmt/clippy/test/smoke；记录已追加到 notes/WORKLOG。

---

## 2026-02-03 23:41 +0800｜调整：取消 Radar 扫描头“高对比模式”（c 切换）

### 目标

- 取消高对比模式（不再提供 `c` 切换）。
- Radar 扫描头只保留一套默认配色与动效（更简单、更确定）。
- 清理相关文档与 help 提示，避免 UI 产生“幽灵功能”。

### 阶段

- [x] 阶段1：移除 state/action/keybinding（`hat_graph_high_contrast` / `ToggleHatGraphHighContrast` / `c`）
- [x] 阶段2：简化扫描头渲染函数签名与配色分支
- [x] 阶段3：更新/删除回归测试（不再覆盖高对比分支）
- [x] 阶段4：同步 help/spec 文档
- [x] 阶段5：验证（fmt/clippy/test + replay smoke tests）
- [x] 阶段6：四文件追加记录（notes/WORKLOG）

### 当前状态

**已完成**：已移除高对比模式（`c`）；扫描头渲染已简化；测试与文档已同步；已通过 fmt/clippy/test/smoke；记录已追加到 notes/WORKLOG。

---

## 2026-02-04 00:27 +0800｜探索：parallel-hat-solution-eval-example（并行实验开发永动机 ralph.yml 配置范式）

### 目标

- 把“多方案并行实现 + 批量验证 + 多轮探索试验”的工作方式，固化成一份可复用的 `ralph.yml` 配置范式。
- 该范式必须在并行 hats 下稳定推进，并且具备强 backpressure（证据不足就不允许收敛）。
- 本轮只补充到 OpenSpec change：`parallel-hat-solution-eval-example`，不进入 apply/实现阶段。

### 阶段

- [x] 阶段1：回读并行 runtime 现有语义（尤其 gate timeout / routing / autoscale cap）
- [x] 阶段2：把关键决策补充进 change artifacts（proposal/design/spec/tasks）
- [x] 阶段3：校验文档图表（Mermaid）语法正确性并修复
- [x] 阶段4：四文件追加记录（notes/WORKLOG）

### 关键问题

1. `permissions: ask` 是否有“超时”机制？超时后谁来决定继续/终止？
2. “实验产物”用 `patch` 还是 `commit` 更合适？auditor 如何做硬门槛审计？
3. 并行度应该写死还是动态调参？如何避免“一开并行就更慢”？

### 方案方向（两条路）

- 方案 A（不惜代价，最强安全/最强确定性）：
  - `parallel.permissions.worktree: ask`、`parallel.permissions.hooks: ask`
  - runner 强制输出 `patch`（不接受 commit）
  - 更强的审计：证据字段更严格，缺一项就拒绝
  - 代价：更容易被 gate 打断；体验更重
- 方案 B（先能跑顺，后续再加强）：
  - example 默认 `allow`，确保“一条命令跑通”
  - 生产建议：`worktree: ask`，`hooks: allow`（避免 hooks 频繁打断）
  - runner 必须输出 `patch`（`commit` 仅可选补充信息），由独立 integrator 在主工作区采纳/验收
  - 代价：安全性靠“文档约定 + 审计门禁”兜底

### 做出的决定（你已确认）

- completion promise：继续使用 `LOOP_COMPLETE`。
- 控制面预留：并行窗口必须给 `ralph#1` + `auditor` 留 slot（避免控制面被饿死）。
- 审计：新增独立 `experiment_auditor` hat，硬门槛（证据不足必须拒绝收敛）。
- 采纳/集成：新增独立 `experiment_integrator` hat，在主工作区 apply patch 并跑最终验收；runner 不做“合并/验收”。
- 并行度：由 `ralph#1` 根据用户 plan/prompt 自动推断 `P_max`；运行中 AIMD 动态调参（激进）。

### 当前状态

**已完成**：关键决策已补充进 `parallel-hat-solution-eval-example` 的 change artifacts；并已把要点同步追加到 `notes.md` / `WORKLOG.md`（仅方案沉淀，不落盘实现）。

### 本轮继续（新增对齐点）

- 你进一步对齐：采纳/合并/应用 patch 不应该由 runner 做，而应该有独立的 integrator hat 来负责。
- 因此本轮把“integrator（主工作区单写者）”也补充进 change，并把 runner 的产物要求收紧为：
  - `patch` MUST
  - `commit` MAY（只能作为补充信息，不能替代 patch）

---

## 2026-02-04 16:07 +0800｜apply：parallel-hat-solution-eval-example（落地 example + 并行回放门禁）

### 目标

- 把 `parallel-hat-solution-eval-example` 这份 change 里的“并行实验开发永动机”范式，真正落盘到仓库中。
- 交付一个可运行的 example：包含 `ralph.yml` + 使用说明。
- 同时交付 replay fixture + smoke test，用硬门禁锁死关键语义（runner/auditor/integrator 分工、patch 采纳链路、LOOP_COMPLETE 结束条件）。

### 阶段

- [x] 阶段1：回读 change 上下文与 tasks，确认每条任务的验收口径
- [x] 阶段2：新增 example（`ralph.yml` + `README.md`）
- [x] 阶段3：新增 replay fixture，并把它接入 `smoke_runner`
- [x] 阶段4：验证（`cargo fmt --check` / `cargo clippy` / `cargo test` / replay smoke）
- [x] 阶段5：逐条勾选 tasks + 四文件追加记录（`notes.md` / `WORKLOG.md`）

### 当前状态

**已完成**：

- 示例配置与文档已落盘：`examples/parallel-experimental-dev-engine/`
- 仓库 README 已补充 runnable example 入口链接
- replay fixture + smoke tests 已落盘（并覆盖关键 topic/归因前缀/patch/LOOP_COMPLETE）
- 已通过 `cargo fmt --check` / `cargo clippy` / `cargo test`

---

## 2026-02-04 20:06 +0800｜补充：为 parallel-experimental-dev-engine example 增加专用 E2E 场景（Codex）

### 目标

- 增加一个 **专门覆盖** `examples/parallel-experimental-dev-engine/` 的 `ralph-e2e` 场景。
- 用 **Codex** 真后端跑一次端到端闭环。
- 断言要“比较硬”：
  - 必须出现关键 topic 链路（experiment → review → integration → complete）
  - 必须看到 `patch` 作为可搬运产物
  - 必须收敛到 `LOOP_COMPLETE`

### 方案方向（两条路）

- 方案 A（更硬、更贴近真实使用，优先选）：
  - E2E 直接跑该 example，并在 E2E workspace 里把 `EXPERIMENT_PLAN` 预填为一组“可确定成功”的轻量实验（只写入小文件 + rg 验证）。
  - 断言主要基于 `.ralph/events.jsonl`（比 stdout 更稳），并额外要求不出现 `routing.escalate` / `gate.request` 等异常信号。
  - 代价：仍属于“真后端 E2E”，可能受模型行为波动影响（但已经尽量压缩不确定性）。
- 方案 B（更稳、更便宜，但不够“真”）：
  - 不跑真后端，只扩展 replay fixture / smoke tests 的覆盖面（例如增加 `needs_more_evidence` 或 `integration.rejected` 分支 fixture）。
  - 代价：它验证的是“回放语义”，不是“Codex 真实端到端”。

### 做出的决定

- 采用方案 A 先做“硬 E2E”；如果 flakey，再回退到方案 B 或调软断言。

### 当前状态

**已完成**：

- 已新增专用 E2E scenario（Codex）：`crates/ralph-e2e/src/scenarios/parallel_experimental_dev_engine_example.rs`
  - 直跑 example，并在 E2E workspace 预填 `EXPERIMENT_PLAN`（轻量实验：写文件 + rg 验证）
  - 断言基于 `.ralph/events.jsonl`：关键 topic 链路 + patch + LOOP_COMPLETE
- 已注册/导出 scenario（可被 `ralph-e2e --list` 发现）：
  - `crates/ralph-e2e/src/scenarios/mod.rs`
  - `crates/ralph-e2e/src/lib.rs`
  - `crates/ralph-e2e/src/main.rs`
- 已通过 backpressure：
  - `cargo fmt --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
  - `cargo test -p ralph-e2e`
