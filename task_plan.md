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

---

## 2026-02-04 00:45 +0800｜调整 example：parallel-experimental-dev-engine（PROMPT.md 驱动 + 允许放弃不理想实验）

### 目标

- 让 `examples/parallel-experimental-dev-engine` 的“实验计划/内容”写在 `PROMPT.md`，而不是 `ralph.yml`：
  - 开发者日常使用时只需要改 `PROMPT.md`（填 EXPERIMENT_PLAN），不需要动 `ralph.yml`。
- 明确“实验就是实验”的语义：
  - runner 产出的结果可能不理想，这是正常情况；
  - 允许 `experiment_auditor` 明确标记并放弃（reject/abandon）不理想结果；
  - workflow 不应因为“有失败/不理想实验”而卡住或强行等到所有实验都 OK。

### 阶段

- [ ] 阶段1：设计与决策（prompt 放置方式 + 放弃语义）
- [ ] 阶段2：改造 example（PROMPT.md + README + ralph.yml）
- [ ] 阶段3：同步 E2E scenario（预填计划位置迁移 + 路径修正）
- [ ] 阶段4：验证（fmt/clippy/test + 相关 smoke/e2e）
- [ ] 阶段5：四文件追加记录（notes/WORKLOG）

### 方案方向（两条路）

- 方案 A（更优雅，更符合“开发者不改 config”）：
  - 将 `event_loop.prompt` 全量迁移到 `examples/parallel-experimental-dev-engine/PROMPT.md`
  - `examples/parallel-experimental-dev-engine/ralph.yml` 固定为 `event_loop.prompt_file: examples/parallel-experimental-dev-engine/PROMPT.md`
  - 在 prompt 里引入明确的 `verdict: approved|rejected|needs_more_evidence`，并允许 rejected 释放 slot、允许提前进入 integration
  - 代价：需要同步更新 `ralph-e2e` 的预填逻辑（不再从 ralph.yml 里找 EXPERIMENT_PLAN）
- 方案 B（先能用，改动更小）：
  - 保留 `event_loop.prompt` 在 ralph.yml，但 README 改为“只编辑 prompt 片段”；同时在 prompt 文案里放宽“必须全部 evidence_ok=true”
  - 代价：开发者仍需要编辑 ralph.yml，不满足“理论上不需要动 ralph.yml”的目标

### 做出的决定

- 选择：方案 A。
- 理由：它把“prompt 内容”从“配置”里剥离出来，后续维护成本更低；也更符合并行模式下 prompt 的真实角色（顶层目标/计划，而不是路由控制面）。

### 当前状态

**目前在阶段1**：我正在梳理现有 example 与 E2E 的耦合点（EXPERIMENT_PLAN 的定位方式、prompt_file 路径、auditor 语义），然后按方案 A 落地修改。

---

## 2026-02-05 00:22 +0800｜补充修正：complete_publishes 必须有“明确发布者”（example + spec 对齐）

### 你指出的关键规则

- 若 `event_loop.complete_publishes = C`，则最好至少有一个 Hat 的 `publishes` 包含 `C`。
  - 否则 completion candidate 缺少明确生产者，容易写成“永远等不到”的收敛条件。
  - 同时 `ralph hats graph --view logical` 会出现 `Complete` 节点但无入边，表达上断链。

### 我做的对齐（已完成）

- 已把规则落盘到 spec：`specs/hats-graph-logical-view.spec.md`（G5 备注区）。
- 已让 `examples/parallel-experimental-dev-engine` 自身符合该规则：
  - `experiment_integrator.publishes` 增加 `experiment.complete`
  - integrator 成功时必须额外发布 `experiment.complete`
  - `PROMPT.md` 的收敛条件改为“观察到 experiment.complete 后输出 LOOP_COMPLETE”（并保留兜底补发）
  - README 的 flowchart 与叙事同步更新

### 当前状态

**已完成**：spec 与 example 已对齐该规则；Mermaid 图已用 `mermaid-validator` 校验通过。

---

## 2026-02-05 00:25 +0800｜systematic-debugging：`parallel-experimental-dev-engine` 下 `ralph hats graph` 拓扑“断开/不直观”

### 现象

- 在 `examples/parallel-experimental-dev-engine/` 下运行 `ralph hats graph`：
  - 图里只有 `experiment_runner -> experiment_auditor` 这条边；
  - `experiment_integrator` 与 `complete` 变成孤岛（看起来像“图坏了/不正常”）。

### 目标

- 找到“图看起来不正常”的根因（是工具 bug、配置问题，还是语义预期差异）。
- 给出可落地的解决方案：
  - 既能解释清楚“为什么现在会这样”，也能让用户在需要时得到“完整工作流视图”。

### 阶段

- [x] 阶段1：复现 + 采集证据（unicode + mermaid）
- [x] 阶段2：对照 spec/历史修复记录，确认当前语义（logical view：隐藏 Ralph）
- [x] 阶段3：给出两条可选方案，并选择一条落地
- [x] 阶段4：实现/修改（含回归测试）
- [x] 阶段5：验证（fmt/clippy/test + smoke）
- [x] 阶段6：四文件追加记录（notes/WORKLOG/ERRORFIX）

### 方案方向（两条路）

- 方案 A（不惜代价，最佳方案｜更符合直觉）：
  - 为 `ralph hats graph` 增加 `view` 概念：
    - `logical`（默认）：保持现状，隐藏调度员，只画 Hat→Hat 逻辑边
    - `physical`（可选）：显式画出 `ralph#1`（coordinator）节点与 Ralph↔Hat 的边，让 coordinator-driven workflow 不再“断开”
  - 代价：需要改 CLI + Mermaid 生成 + 补测试与文档
- 方案 B（先能用，后面再优雅｜改动更小但会改变 example 心智模型）：
  - 把 coordinator 行为“显式化”为一个 hat（例如 `experiment_coordinator`）：
    - 订阅 `experiment.reviewed` / `integration.applied`，发布 `experiment.task` / `integration.task` / `experiment.complete`
  - 代价：example 不再是“ralph#1 主导协调”，而是“一个 hat 主导协调”

### 做出的决定

- 倾向选择：方案 A。
- 理由：不改变现有 example 的工作流语义；只是给图提供一个“看全貌”的视角开关。

### 当前状态

**已完成**：已为 `ralph hats graph` 增加 `--view physical`，并同步文档与回归测试；同时修复 physical view 在 unicode/ascii 渲染下的 QuickJS exception（折叠 Ralph 相关多边）；logical view 默认行为保持不变。

---

## 2026-02-05 00:50 +0800｜调整需求：physical view 设为默认（取消必须写 `--view physical`），Radar 也默认 physical

### 新目标

- `ralph hats graph` 默认输出 physical view：
  - 用户不再需要写 `--view physical`（它变成默认值）。
- TUI 右上角 Hats Graph Radar 默认也使用 physical view（与 CLI 默认对齐）。

### 阶段

- [ ] 阶段1：改 CLI 默认值（view 默认=physical）
- [ ] 阶段2：改 Radar 默认值（render_hat_graph_radar_ascii 默认=physical）
- [ ] 阶段3：补齐 Radar 的 label 匹配逻辑（处理 physical view 折叠边的多 topic label）
- [ ] 阶段4：验证（fmt/clippy/test + 手动跑一个 example 的 hats graph）
- [ ] 阶段5：四文件追加记录（notes/WORKLOG/ERRORFIX）

### 关键风险点（提前写下，避免“改了但没意识到副作用”）

- physical view 为了规避 `beautiful-mermaid-rs --ascii` 的不稳定点，会把 Ralph 相关的多条边折叠成一条：
  - label 形如：`integration.applied / integration.blocked / integration.rejected`
  - Radar 之前用的是 `edge.label == topic` 的完全匹配；需要升级为“包含匹配”。

### 当前状态

**目前在阶段1**：我将先把 CLI/Radar 的默认 view 切到 physical，然后把 Radar 的边匹配逻辑补齐，最后跑全量门禁验证。

---

## 2026-02-05 09:00 +0800｜hats graph：让 Unicode/ASCII 图里 `ralph#1` 尽量靠左/靠上

### 目标

- 当用户运行 `ralph hats graph`（尤其是 `--format unicode/ascii/compact`）时：
  - 图中如果存在 `ralph#1`（coordinator）节点，则它在布局上应尽量成为“起点”：
    - 在 `flowchart LR`（左→右）布局里尽量靠最左
    - 在 `flowchart TB`（上→下）布局里尽量靠最上

### 阶段

- [x] 阶段1：定位 Mermaid 生成与渲染链路（CLI/TUI 共用路径）
- [x] 阶段2：给出两条实现方案并选择落地
- [x] 阶段3：实现（尽量小改动，优先“改良胜过新增”）
- [x] 阶段4：回归测试与门禁验证（fmt/clippy/test + smoke）
- [x] 阶段5：四文件追加记录（notes/WORKLOG）

### 方案方向（两条路）

- 方案 A（更稳，代价略高）：增加“隐形布局锚点”
  - 在 Mermaid 文本开头加一个不可见的 anchor 节点与一条不可见边，强制把 `ralph#1` 拉到最左/最上
  - 优点：对复杂拓扑更稳定
  - 风险：ASCII/Unicode 渲染器（`beautiful-mermaid-rs`）对“透明节点/透明边”的兼容性需要验证
- 方案 B（先能用，改动更小）：仅调整输出顺序
  - Mermaid 节点声明顺序/边输出顺序优先 `ralph#1`，再按稳定排序输出其它节点与边
  - 优点：实现简单、兼容性风险更小；也更符合“确定性输出”
  - 风险：对某些复杂 DAG 的布局影响有限（只能做到“尽量”）

### 当前状态

**已完成**：已将 physical view 的 Mermaid 文本调整为“优先声明 `Hat_ralph` 节点”，让 Unicode/ASCII 图中 `ralph#1` 更稳定地靠左/靠上；补了回归测试并通过 fmt/clippy/test/smoke 门禁。

---

## 2026-02-05 09:07 +0800｜继续：解释 `parallel-experimental-dev-engine` 下 hats graph “不正常”，并把 physical view 设为默认（含 Radar）

### 我正在做什么 & 为什么

- 我正在把你在 `examples/parallel-experimental-dev-engine/` 里看到的 `ralph hats graph` “断开/不直观”现象，按 systematic-debugging 流程做一次闭环：
  - 先复现与采集证据（mermaid + unicode/ascii）。
  - 再定位根因（是渲染器、配置，还是视图语义）。
  - 最后落地“默认 physical view + Radar 也 physical”的改动，并用测试门禁验证。
- 我这样做的原因是：这个 example 属于 coordinator-driven workflow（`ralph#1` 是路由中心）。
  - 如果默认视图隐藏 coordinator，图就天然会“看起来断开”，这不是渲染坏了。
  - 把 physical view 设为默认更符合此 example 的心智模型，也能避免你每次都要手写 `--view physical`。

### 下一步行动（按顺序执行）

1. 复现：分别跑 `ralph hats graph` 的默认输出、`--view logical`、`--view physical`，对比差异并确认“异常感”的来源。
2. 代码核对：确认 CLI 默认 view=physical；TUI Radar 默认也使用 physical；且 Radar 能用单个 topic 匹配 physical view 折叠后的 `"a / b / c"` label。
3. 验证：`cargo fmt --check` / `cargo clippy` / `cargo test`（含 smoke）并确保无 error/warn 遗留。
4. 四文件收尾：把结论与验证结果追加到 `notes.md`、`WORKLOG.md`，如涉及 bug fix 也追加到 `ERRORFIX.md`，并在本任务段落中记录完成状态。

### 当前状态

**目前在阶段1**：我将先用命令行复现并抓取输出证据（优先 mermaid，其次 unicode/ascii）。

---

## 2026-02-05 09:45 +0800｜完成：physical view 设为默认 + Radar 默认 physical（systematic-debugging 闭环）

### 根因结论（对应“为什么图不正常”）

- `parallel-experimental-dev-engine` 是典型的 coordinator-driven workflow：
  - 大量关键 topic（`experiment.task` / `integration.task` / `experiment.reviewed` 等）由 `ralph#1` 发布/消费。
- 当 `ralph hats graph` 使用 logical view（隐藏 `ralph#1`）时：
  - 这些边会按语义被裁掉；
  - 图就会天然“看起来断开/不直观”，容易被误判为渲染或配置问题。

### 完成情况（对照阶段清单）

- [x] 阶段1：改 CLI 默认值（view 默认=physical）
- [x] 阶段2：改 Radar 默认值（render_hat_graph_radar_ascii 默认=physical）
- [x] 阶段3：补齐 Radar 的 label 匹配逻辑（支持 `"a / b / c"` 折叠 label）
- [x] 阶段4：验证（fmt/clippy/test + 手动跑 example hats graph）
- [x] 阶段5：四文件追加记录（notes/WORKLOG/ERRORFIX）

### 复现/证据（命令行）

- physical（默认）：
  - `cargo run --bin ralph -- -c examples/parallel-experimental-dev-engine/ralph.yml hats graph --format mermaid`
- logical（更干净，但 coordinator-driven 会天然“断开”）：
  - `cargo run --bin ralph -- -c examples/parallel-experimental-dev-engine/ralph.yml hats graph --view logical`

### 门禁验证（Backpressure）

- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test -p ralph-core kiro` ✅

### 当前状态

**已完成**：默认 physical view + Radar physical 已落地；并通过 fmt/clippy/test/smoke 门禁验证。

---

## 2026-02-05 10:20 +0800｜调整 example：`parallel-experimental-dev-engine` 由 `PROMPT.md` 驱动 + `complete_publishes` 硬门禁

### 我正在做什么 & 为什么

- 我正在按你的要求改 example `examples/parallel-experimental-dev-engine/`：
  - 实验计划写在 `PROMPT.md`，而不是 `ralph.yml`；
  - `starting_event` 不写死，让 `ralph#1` 自己决定入口；
  - `experiment_auditor` 允许对不理想结果 `rejected` 放弃；
  - 对 `event_loop.complete_publishes` 增加硬门禁：必须有某个 hat 在 `publishes` 声明该 topic。
- 我这样做的原因是：
  - 配置更稳定：日常改计划不需要改 YAML；
  - completion candidate 必须有“明确生产者”，否则收敛逻辑会变成隐式约定，容易跑偏且难排查。

### 阶段

- [x] 阶段1：补齐计划文件（`PROMPT.md` 模板 + example README）
- [x] 阶段2：实现 hard gate（config validate）
- [x] 阶段3：修正受影响的 E2E 场景（让 `complete_publishes` 有明确 hat publisher）
- [x] 阶段4：全量门禁验证（fmt/clippy/test + smoke）
- [x] 阶段5：四文件收尾（notes/WORKLOG/ERRORFIX）

### 方案方向（两条路）

- 方案 A（更“硬”、更一致）：严格要求 `complete_publishes` 必须出现在某个 hat 的 `publishes` 中
  - 优点：语义自洽；拓扑图不会出现“completion candidate 没生产者”的悬空节点；减少隐式约定
  - 代价：需要把 `complete_publishes=routing.escalate` 这类“supervisor 直投给 `ralph#1` 的事件”改写成由 hat 发布的 completion topic
- 方案 B（更兼容历史配置）：对少数 supervisor/control-plane topic 做 allowlist 例外（如 `routing.escalate`）
  - 优点：不需要改现有 E2E 场景
  - 风险：规则变“半硬”，后续容易继续出现隐式 completion candidate，降低一致性

### 决定

- 选择方案 A：
  - 在 `hats` 非空时，`complete_publishes` 必须有明确 hat publisher（硬门禁）
  - 同步改 E2E：把 completion candidate 换成由 hat 发布的新 topic，`routing.escalate` 继续保留为“必出现的观测事件”

### 当前状态

**已完成**：example 已迁移到 `PROMPT.md` 驱动，`complete_publishes` 的“明确发布者”已做硬门禁，并通过 fmt/clippy/test/smoke 验证。

---

## 2026-02-05 10:44 +0800｜新增配置：`event_loop.ralph_prompt`（始终注入 Ralph prompt）

### 我正在做什么 & 为什么

- 我正在按你的需求新增一个配置项：`event_loop.ralph_prompt`。
- 这个配置项的目标很明确：**它只影响 Ralph（协调者）的 prompt**，并且**无论**你是否使用：
  - `PROMPT.md`（默认 prompt_file），或
  - `event_loop.prompt`（inline prompt）
  它都要**始终被注入**到 Ralph 的 prompt 里。
- 我这样做的原因是：目前 prompt 来源（`event_loop.prompt` / `prompt_file` / `PROMPT.md`）本质上是“顶层 objective”。
  但很多场景需要一段“只给协调者看的固定语义锚点/行为约束”（尤其并行模式），而不希望污染其他 hats 的输入。

### 阶段

- [x] 阶段1：补齐 spec（定义语义、注入位置、并行/非并行一致性、验收标准）
- [x] 阶段2：实现配置解析（YAML/serde）+ 代码注入点（HatlessRalph + ParallelSupervisor）
- [x] 阶段3：补充回归测试（确保只注入 Ralph、不影响 prompt precedence、并行/非并行都覆盖）
- [x] 阶段4：同步文档/示例（说明配置项用途与与 `event_loop.prompt` 的区别）
- [x] 阶段5：验证（fmt/clippy/test + replay smoke tests）
- [x] 阶段6：四文件收尾（notes/WORKLOG）

### 方案方向（两条路）

- 方案 A（不惜代价，最佳语义/最干净的边界）：
  - 新增 `event_loop.ralph_prompt`，只在“构造 Ralph prompt”时注入：
    - 非并行（HatlessRalph）：注入到 Ralph 的 core prompt（不改事件 payload）
    - 并行（ralph#1）：注入到 Supervisor 生成的协调者 instructions（不改其他 hat 的输入）
  - 优点：不会污染事件流；不会影响其它 hats；语义清晰且可测试。
  - 代价：需要同时改两条 prompt 组装路径（非并行/并行）。

- 方案 B（先能用，后面再优雅）：
  - 把 `ralph_prompt` 直接拼进顶层 prompt（等价于改写 `task.start` payload 或 prompt_prelude）。
  - 优点：改动点少。
  - 风险：会把“只给 Ralph 的语义”泄漏到事件与其它 hats，破坏并行模式里的“prompt pollution”防线。

### 决定

- 选择方案 A：用一个**明确的、只作用于 Ralph 的注入点**来实现 `event_loop.ralph_prompt`。

### 当前状态

**已完成**：`event_loop.ralph_prompt` 已落地（非并行 + 并行均注入且不污染其它 hats），并通过 fmt/clippy/test + smoke_runner/kiro 验证；四文件记录已追加。

---

## 2026-02-05 11:02 +0800｜调整 example：`parallel-experimental-dev-engine` 固定协议迁移到 `event_loop.ralph_prompt`

### 我正在做什么 & 为什么

- 我正在按你的要求调整 example `examples/parallel-experimental-dev-engine/`：
  - 把 `examples/parallel-experimental-dev-engine/PROMPT.md` 中“开发者不需要改的固定协议/安排”迁移到 `examples/parallel-experimental-dev-engine/ralph.yml` 的 `event_loop.ralph_prompt`；
  - 让 `PROMPT.md` 只保留“演示型范例/模板”，告诉开发者应该写什么（主要就是 `EXPERIMENT_PLAN` YAML）。
- 我这样做的原因是：
  - `event_loop.ralph_prompt` 是 **Ralph-only 的追加注入**，天然适合承载“稳定的协调语义锚点”；
  - `PROMPT.md` 只保留可变的实验计划，可以把日常使用的改动面压到最低，减少误改协议导致 workflow 跑偏。

### 阶段

- [x] 阶段1：回读 example 现状（PROMPT.md / ralph.yml / README），确认哪些是“固定协议”哪些是“开发者需改”
- [x] 阶段2：把固定协议搬到 `event_loop.ralph_prompt`（并校对与并行 coordinator 语义不冲突）
- [x] 阶段3：精简 PROMPT.md（只保留 EXPERIMENT_PLAN 模板 + 最小说明）
- [x] 阶段4：同步 example README（说明 fixed vs variable 的分工）
- [x] 阶段5：验证（至少跑 cargo test，确保不破坏 E2E 场景对 marker 的依赖）
- [x] 阶段6：四文件收尾（notes/WORKLOG）

### 方案方向（两条路）

- 方案 A（最佳一致性，推荐）：
  - `ralph.yml`：把协议/门槛/窗口/AIMD 等固定语义全部放到 `event_loop.ralph_prompt`；
  - `PROMPT.md`：只包含 `EXPERIMENT_PLAN`（可复制/可填空/可预填），其余不放说明性长文。
  - 优点：日常只改计划；协议稳定；并行 prompt pollution 风险最低。
  - 代价：需要在 `ralph_prompt` 里把“如何解释 task.start payload 是一个计划 YAML”讲清楚。

- 方案 B（先能用，兼容旧阅读体验）：
  - `PROMPT.md` 仍保留少量“短说明”，例如一屏以内的使用说明；
  - 其余固定协议仍迁移到 `ralph_prompt`。
  - 优点：开发者打开 PROMPT.md 就能马上理解在填什么；
  - 风险：说明可能又慢慢膨胀回长文，且容易出现“PROMPT.md 与 ralph_prompt 重复/冲突”的漂移。

### 决定

- 选择方案 A（PROMPT.md 极简 + ralph_prompt 承载固定协议）。
  - 已完成：固定协议已迁移到 `event_loop.ralph_prompt`；`PROMPT.md` 已精简为计划模板；`README.md` 已同步；并通过 `cargo test` 验证。

---

## 2026-02-05 11:15 +0800｜调整 example：用 `commit` 代替 `patch` 作为实验产物载体

### 我正在做什么 & 为什么

- 目前 `parallel-experimental-dev-engine` 这个 example 约定：`experiment.result` 必须携带 `patch`（`git diff` 的 unified diff 文本）。
- 但在真实改动里，patch 很容易变成几千行：
  - event payload 会膨胀；
  - 模型输出容易被截断；
  - auditor / integrator 的“可搬运产物”反而变得不可搬运。
- 因此按你的决定，把“最低可搬运产物”改为 `commit`（git hash）：
  - runner 负责在 worktree 里把改动提交成一个 commit；
  - integrator 在主工作区用 `git cherry-pick <hash>` 集成，并做最终验收。

### 阶段

- [x] 阶段1：更新 spec（产物从 patch→commit）
- [x] 阶段2：更新 example（`ralph.yml` / `PROMPT.md` / `README.md`）
- [x] 阶段3：更新测试与 fixture（smoke + e2e 断言）
- [x] 阶段4：门禁验证（fmt/clippy/test + smoke）
- [x] 阶段5：四文件收尾（notes/WORKLOG/ERRORFIX）

### 方案方向（两条路）

- 方案 A（commit-only，按你的决定）：
  - `experiment.result` **必须**包含 `commit`，不再要求 `patch`
  - 优点：payload 很小；避免长 diff 截断；更贴近真实开发流（review / cherry-pick）
  - 风险：需要 git 身份（`user.name`/`user.email`）；commit hash 依赖共享 object DB（但 worktree 共享 `.git`，通常没问题）
- 方案 B（兼容兜底）：
  - `commit` 为主，但允许 `patch` 作为可选备份
  - 优点：commit 因环境失败时仍有搬运手段
  - 缺点：模型可能继续输出超大 patch，无法从根上解决“payload 膨胀/截断”问题

### 决定

- 选择方案 A：commit-only（以 `commit` 作为 runner→auditor→integrator 的唯一交换载体）。

### 当前状态

**已完成**：协议已切换为 commit-only，并已同步 example / OpenSpec / fixture / tests；已通过 `cargo fmt --check`、`cargo clippy --all-targets --all-features -- -D warnings`、`cargo test` 验证；四文件记录已追加。

---

## 2026-02-05 15:04 +0800｜调整 example：`parallel-experimental-dev-engine` 的 `PROMPT.md` 变成纯 YAML（无说明/无 marker）

### 我正在做什么 & 为什么

- 你明确要求：`examples/parallel-experimental-dev-engine/PROMPT.md` 不要说明文字，也不要 `<!-- ... -->` 这种 marker。
- 我将把 `PROMPT.md` 改成“纯 YAML 模板文件”：
  - 文件内容只有 `EXPERIMENT_PLAN` YAML（用 TODO 字段引导开发者填写）。
  - 没有任何额外 Markdown 标题、说明段落、HTML 注释 marker。
- 同时需要同步调整：
  - `ralph.yml` 内 `event_loop.ralph_prompt` 的措辞（去掉“不要拷贝 marker 行”的旧描述）。
  - `ralph-e2e` 的示例场景：不再依赖 marker 来预填计划，而是直接覆写 workspace 里的 PROMPT.md 为确定性 YAML。
- 我这样做的原因是：PROMPT.md 既然要变成纯 YAML，那么任何“靠 marker 截取/替换”的逻辑都会失效，必须同步修正，否则 E2E/示例会漂移。

### 阶段

- [x] 阶段1：移除 PROMPT.md 中所有说明与 marker，仅保留 YAML 模板
- [x] 阶段2：同步 ralph.yml 的 ralph_prompt（去 marker 相关描述）
- [x] 阶段3：同步 example README（不再提“编辑 marker 区间”）
- [x] 阶段4：修正 ralph-e2e 示例场景（预填逻辑不依赖 marker）
- [x] 阶段5：验证（cargo test）
- [x] 阶段6：四文件收尾（notes/WORKLOG）

### 当前状态

**已完成**：`PROMPT.md` 已变成纯 YAML；`ralph_prompt`/`README`/E2E 预填逻辑已同步；已通过 `cargo fmt --check` 与 `cargo test`；四文件记录已追加。

---

## 2026-02-05 15:38 +0800｜修正 example：PROMPT.md / event_loop.ralph_prompt 统一为 Markdown prompt（不再把 PROMPT.md 当 YAML）

### 我正在做什么 & 为什么

- 你明确澄清：`PROMPT.md` 和 `event_loop.ralph_prompt` 都是“给 agent 的 prompt”，因此它们应该是 Markdown 文本，用来讲述任务需求，而不是把 `PROMPT.md` 当成 YAML 配置文件。
- 之前我把 `examples/parallel-experimental-dev-engine/PROMPT.md` 改成了“纯 YAML 模板”，这会让文件语义变形，也会让后续读者误以为 Ralph 会解析 YAML 配置。
- 因此我将：
  - 把 `examples/parallel-experimental-dev-engine/PROMPT.md` 改回 Markdown 的实验计划模板（结构化但仍是 prompt）；
  - 同步 `examples/parallel-experimental-dev-engine/ralph.yml` 的 `event_loop.ralph_prompt` 文案，去掉“PROMPT.md 是 YAML”的描述；
  - 同步 example README + ralph-e2e 预填逻辑，保证示例与测试一致。

### 阶段

- [x] 阶段1：补充 task_plan 记录与目标对齐
- [x] 阶段2：重写 PROMPT.md 为 Markdown 模板（保留演示范例 + TODO 占位）
- [x] 阶段3：同步 ralph.yml / README 文案（去掉 YAML 假设）
- [x] 阶段4：同步 ralph-e2e 预填逻辑输出 Markdown
- [x] 阶段5：门禁验证（fmt/clippy/test）
- [x] 阶段6：四文件收尾（notes/WORKLOG/ERRORFIX）

### 方案方向（两条路）

- 方案 A（严格按你的澄清，推荐）：
  - PROMPT.md：纯 Markdown 的“实验计划 prompt”，用标题/列表表达结构，不使用 YAML 作为主要载体。
  - 优点：语义最一致；不会误导为“配置文件”；易读。
  - 风险：结构性略弱，需要靠模板约束字段位置。

- 方案 B（Markdown 外壳 + 内嵌 YAML code block）：
  - PROMPT.md：Markdown，主要内容是一段 ```yaml``` 的计划块。
  - 优点：结构强，易复制到 event payload。
  - 风险：仍会让读者误以为“PROMPT.md 是 YAML”，与“不是 yaml”这条要求冲突。

### 决定

- 选择方案 A：PROMPT.md 只用 Markdown 结构表达计划。

### 当前状态

**已完成**：PROMPT.md/ralph_prompt/README/E2E 预填已统一为 Markdown prompt；已通过 `cargo fmt --check`、`cargo clippy --all-targets --all-features -- -D warnings`、`cargo test`、`cargo test -p ralph-core smoke_runner`、`cargo test -p ralph-core kiro` 验证。

---

## 2026-02-05 16:35 +0800｜增强：Experiments 为空时由 ralph#1 自动生成实验方案并派发 runner

### 我正在做什么 & 为什么

- 你希望：当 `PROMPT.md` 里不写“实验列表（Experiments）”时，`ralph#1` 需要自己：
  1) 根据目标与约束分析项目；
  2) 生成多条实验性方案（多条路径）；
  3) 再按并行窗口把这些实验派发给 runner 去跑。
- 我将把这条语义固化在 `examples/parallel-experimental-dev-engine/ralph.yml` 的 `event_loop.ralph_prompt` 里：
  - 这样不需要改 orchestrator 代码；
  - 也不会引入新 hat（遵循“改良胜过新增”）；
  - 并且对示例使用者来说是“开箱即用”的默认能力。

### 阶段

- [x] 阶段1：更新 `event_loop.ralph_prompt`（定义 auto-plan 触发条件与生成规则）
- [x] 阶段2：更新 `PROMPT.md` 模板（明确 Experiments 可选，以及建议补充的信息）
- [x] 阶段3：更新 README（同步文案）
- [x] 阶段4：验证（最少跑一次 `cargo test -p ralph-core smoke_runner`）
- [x] 阶段5：四文件收尾（notes/WORKLOG）

### 关键约束（要锁死）

- “强 backpressure”不变：runner 仍必须产出 `commit` + `verification_evidence`，auditor 仍必须审计。
- auto-plan 只影响 `ralph#1` 的行为：它生成的实验最终仍以 `experiment.task` 事件的形式进入 pipeline（runner/auditor/integrator 无需知道“是不是用户写的”）。

### 当前状态

**已完成**：Experiments 为空/占位时，`ralph#1` 会按 ralph_prompt 先做只读扫描、自动生成 2~5 个实验，再按窗口派发 `experiment.task`；并已通过 `cargo test -p ralph-core smoke_runner` 验证不破坏现有 smoke。

---

## 2026-02-05 16:58 +0800｜调整：移除“实验列表（Experiments）硬条目”概念，默认 PROMPT.md 不写实验任务

### 我正在做什么 & 为什么

- 你希望进一步把 `PROMPT.md` 的默认心智模型做成：
  - 默认不写任何实验任务条目；
  - `ralph#1` 看到没有实验任务，就自己先分析项目、生成多条实验方案，再按窗口派发给 runner；
  - 如果用户显式写了实验任务条目，则优先按条目执行（而不是强制 auto-plan）。
- 这样做的好处是：
  - 更符合“探索型工作流”的默认使用方式：先给目标/约束，让系统提出多种路径；
  - 避免 PROMPT.md 里出现 `exp-001/exp-002` 这种“看似必填”的硬条目，导致用户以为必须先拆实验才能跑。

### 阶段

- [x] 阶段1：调整 `PROMPT.md` 模板（默认不含任何实验条目）
- [x] 阶段2：调整 `event_loop.ralph_prompt` 的解析约定（条目可选；无条目则 auto-plan）
- [x] 阶段3：同步 README 使用说明
- [x] 阶段4：最小验证（至少 `cargo test -p ralph-core smoke_runner`）
- [x] 阶段5：四文件收尾（notes/WORKLOG）

### 当前状态

**已完成**：默认 PROMPT.md 不再包含任何实验任务条目；`ralph#1` 会优先按用户条目派发，缺失时才 Auto-Plan；并已通过 `cargo test -p ralph-core smoke_runner` 验证。

---

## 2026-02-06 12:00 +0800｜修正：`ralph hats graph --format mermaid` 节点 label 含 `()` 时需加引号（避免 Mermaid Parse error）

### 我正在做什么 & 为什么

- 你指出 `ralph hats graph` 的输出里出现了类似 `Hat_ralph[ralph#1 (coordinator)]` 的 Mermaid 节点。
- 这个 label 在 `[]` 内带有 `()`，在 Mermaid 解析器里很容易触发歧义/解析错误（尤其是 flowchart/graph 的节点形状语法本身就使用 `()`）。
- 因此我将把 hat graph 的节点 label 生成逻辑改为：
  - **遇到 `(` / `)` 就用引号包裹 label**（输出为 `["..."]` 并做必要转义）；
  - 并补充回归测试，避免以后又回退。

### 阶段

- [ ] 阶段1：定位 `hats graph` 的 Mermaid 输出生成点
- [x] 阶段1：定位 `hats graph` 的 Mermaid 输出生成点
- [x] 阶段2：调整 label 格式（括号自动加引号，输出标准 Mermaid）
- [x] 阶段3：补充回归测试（锁死括号 label 必须加引号）
- [x] 阶段4：验证（`cargo test` + smoke tests）
- [x] 阶段5：四文件收尾（notes/WORKLOG/ERRORFIX）

### 方案方向（两条路）

- 方案 A（保留括号，但用 Mermaid 引号包裹 label）：
  - 输出形态：`Hat_ralph["ralph#1 (coordinator)"]`
  - 优点：信息最完整；更通用（还能规避 `[]`/`()`/`/` 等字符歧义）。
  - 风险：部分终端渲染器会把引号也当作内容画出来，所以我们只在 `--format mermaid`（Strict）下启用该规则。

- 方案 B（备用：直接去掉括号，用分隔符表达角色）：
  - 输出形态：`Hat_ralph["ralph#1 - coordinator"]`（或 `ralph#1 coordinator`）
  - 优点：满足“label 不含 `()`”的硬约束；对 Mermaid 解析最稳；可读性也更好。
  - 风险：视觉上少了括号，但语义保留。

### 决定

- 选择方案 A：保留原信息，但当 label 含 `(` / `)` 时改为 `["..."]`，保证标准 Mermaid 可解析。

### 当前状态

**已完成**：`--format mermaid` 下，含括号的节点 label 会自动输出为 `["..."]`；已通过 `cargo fmt` 与 `cargo test` 验证；四文件记录已追加。

### 计划修正（基于 Mermaid Validator 证据）

- 我用 `mermaid-cli`（通过 `mermaid-validator` MCP）验证了最小样例：
  - `Hat_ralph[ralph#1 (coordinator)]` 会直接 Parse error（不被标准 Mermaid 语法接受）。
  - `Hat_ralph["ralph#1 (coordinator)"]` 可以通过校验。
- 因此我将优先采用“**遇到括号就用引号包裹 label**”的做法：
  - 保留原信息（仍显示 `(coordinator)`）；
  - 同时保证 `--format mermaid` 输出是标准 Mermaid 可渲染的文本。

---

## 2026-02-06 22:16 +0800｜调整：`ralph hats graph --format mermaid` 改用 `flowchart TD`（更符合流程图阅读方向）

### 我正在做什么 & 为什么

- 你希望：`ralph hats graph` 的 Mermaid 输出使用 `flowchart TD`（上→下），而不是 `flowchart LR`（左→右）。
- 这样做的收益是：在 Markdown/网页里阅读时更像“流程图”，也更适合从 `Start` 往下走到 `Complete` 的心智模型。
- 但终端 ASCII/Unicode（含 TUI Radar）目前对 `flowchart LR` 做过“布局稳定性”的优化经验总结.
  - 我会尽量避免让这次改动影响终端图的布局,先把改动范围收敛在 `--format mermaid` 上。

### 阶段

- [x] 阶段1：定位 Mermaid 首行生成位置
- [x] 阶段2：仅 `--format mermaid`（Strict）输出改为 `flowchart TD`
- [x] 阶段3：更新回归测试（锁死首行方向）
- [x] 阶段4：验证（`cargo fmt --check`、`cargo clippy`、`cargo test`、smoke tests）
- [x] 阶段5：四文件收尾（WORKLOG）

### 方案方向（两条路）

- 方案 A（不惜代价,最佳方案）：增加 `--direction (td/lr)` 参数.
  - 优点：用户可自由选择方向,兼容终端布局与 Mermaid 阅读偏好。
  - 风险：需要扩展 CLI 参数与文档,改动面更大。
- 方案 B（先能用,后面再优雅）：仅把 `--format mermaid` 的首行改为 `flowchart TD`,其余格式保持 `flowchart LR`。
  - 优点：改动面小,不影响现有终端渲染经验与 TUI Radar 观感。
  - 风险：如果你也希望 ASCII/Unicode 变成 TD,需要下一步再扩展。

### 决定

- 选择方案 B：你当前明确提的是 Mermaid 输出方向,我先把影响面控制在 `--format mermaid`。

### 当前状态

**已完成**：`--format mermaid` 输出已切换为 `flowchart TD`,终端 ASCII/Unicode/Radar 保持 `flowchart LR`; 已通过 fmt/clippy/test/smoke 验证,WORKLOG 已追加。

---

## 2026-02-06 22:25 +0800｜调整：ASCII/Unicode/Radar 也统一使用 `flowchart TD`

### 我正在做什么 & 为什么

- 你进一步明确要求:ASCII/Unicode/Radar 的方向也必须按 `flowchart TD`。
- 这意味着我们不能只改 `--format mermaid` 的首行,还要把终端渲染链路(ASCII/Unicode/Compact)与 TUI Radar 使用的 Mermaid 源也切到 TD。
- 我会先把改动收敛在 Mermaid 源生成函数里,避免在多个渲染入口重复加逻辑,并用测试锁死行为。

### 阶段

- [x] 阶段1：调整 Mermaid 生成函数:TerminalPretty 也输出 `flowchart TD`
- [x] 阶段2：补充/更新回归测试（锁死 TerminalPretty 方向）
- [x] 阶段3：验证（`cargo fmt --check`、`cargo clippy`、`cargo test`、smoke tests）
- [x] 阶段4：四文件收尾（WORKLOG）

### 决定

- 直接统一为 `flowchart TD`:
  - Strict(mermaid) 与 TerminalPretty(ASCII/Unicode/Radar) 都输出 TD,消除方向分叉。

### 当前状态

**已完成**：ASCII/Unicode/Compact/Radar 现在也统一基于 `flowchart TD`; 同时为 TD + 回边(backlink) 的 compact 渲染把 `padding_y` 调整到 `1` 规避 QuickJS exception; 已通过 fmt/clippy/test/smoke 验证,WORKLOG 已追加。
