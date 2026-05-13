# 任务计划: 非 parallel 模式 Codex 线程报错与 serial TUI 选择缺失

## 目标

查清非 parallel 模式下 `failed to record rollout items: thread ... not found` 的来源,同时确认 serial TUI 输出文本无法选择是实现缺口还是回归,并在有根因证据后修复。

## 阶段

- [x] 阶段1: 建立支线上下文并读取相关规则
- [ ] 阶段2: 定位 Codex app/server/session 调用路径和报错来源
- [ ] 阶段3: 定位 serial TUI 输出选择路径和 parallel TUI 的差异
- [ ] 阶段4: 形成根因假设,补最小回归测试
- [ ] 阶段5: 实施修复并运行聚焦测试 / TUI 捕获验证
- [ ] 阶段6: 记录 WORKLOG / ERRORFIX,给出交付结论

## 关键问题

1. 这个 Codex error 是 Ralph 调用 Codex 的参数/环境导致,还是 Codex 自身记录 rollout 的非致命 stderr?
2. 非 parallel 模式是 serial TUI,当前是否没有实现鼠标拖选/复制输出?
3. parallel TUI 已有 output/chat selection 能否复用到 serial output pane,避免新增一套独立机制?

## 方案方向

- 不惜代价,最佳方案:
  - 复现非 parallel TUI + Codex backend 的 stderr,用 record-session 或 tmux 捕获证据。
  - 对比 serial TUI 和 parallel TUI selection 的状态模型,抽出共同选择能力或把 serial output 接到同一套选择模型。
  - 补 serial output selection 的单元测试和 TUI 文本捕获验证。
- 先能用,后面再优雅:
  - 先确认 Codex error 是否非致命,若是后端噪音则做文档/显示层降噪。
  - serial TUI 先补最小鼠标拖选输出复制能力,后续再与 parallel selection 统一。

## 做出的决定

- 决定: 使用 systematic-debugging。
  理由: 当前有两个表面症状,必须先分层定位,不能直接 patch。
- 决定: 使用 tmux 文本捕获作为 TUI 动态证据。
  理由: 当前本机有 tmux,freeze 不一定可用,文本捕获足以验证布局和选择路径。

## 遇到错误

- 暂无。

## 状态

**目前在阶段2** - 准备搜索 `failed to record rollout items`、Codex app server/session 调用,以及 serial/parallel TUI selection 差异。

## [2026-04-30 09:01:24] [Session ID: 019dd984-e9a3-7660-8264-86f293870a2b] [行动记录]: 继续定位两条问题链路

- 当前继续沿支线上下文推进,不切回默认六文件。
- 已确认计划仍处于阶段2,接下来会:
  - 阅读 serial 执行路径,确认非 parallel 是否走 `codex exec` 而不是 app-server。
  - 阅读 serial/parallel TUI 的输出选择实现差异,判断是缺口还是回归。
  - 记录证据到 notes 文件,再决定是否修改代码。

## [2026-04-30 09:07:16] [Session ID: 019dd984-e9a3-7660-8264-86f293870a2b] [状态更新]: 已形成初步根因

- 阶段2 证据:
  - serial/non-parallel 通过 `execute_pty(...)` -> `PtyExecutor::spawn_pty(...)` 启动 backend。
  - `PtyExecutor::spawn_pty(...)` 只设置 `TERM`,没有清理父 Codex 会话环境。
  - 当前 agent 环境存在 `CODEX_THREAD_ID`,用户报错来自 `codex_core::session` 的 rollout 记录线程查找失败。
- 阶段3 证据:
  - parallel TUI 已有 `output_selection` / `output_selecting` / 鼠标 Drag/Up copy。
  - serial TUI 只渲染 `ContentPane`,没有 serial output selection 状态,也没有 Mouse Down/Drag/Up 分支。
- 下一步:
  - 在后端 spawn 前清理 Codex 父会话 thread/turn 环境。
  - 给 serial TUI 补输出选择状态和鼠标选择/复制路径。

## [2026-04-30 09:19:23] [Session ID: 019dd984-e9a3-7660-8264-86f293870a2b] [状态更新]: 修复与验证完成

- [x] 阶段2: 已定位 Codex 报错来源为父 Codex 会话环境变量透传到子 Codex 进程的风险。
- [x] 阶段3: 已定位 serial TUI 输出选择缺口,`ContentPane` 支持选择,但 serial state/input/render 没接入。
- [x] 阶段4: 已补回归测试:
  - `codex_env` 环境清理单元测试。
  - serial output mouse/keyboard selection 单元测试。
- [x] 阶段5: 已实施修复并通过验证:
  - `cargo fmt --all`
  - `cargo test -p ralph-adapters codex_env --quiet`
  - `cargo test -p ralph-tui serial_output_selection --quiet`
  - `cargo test -p ralph-tui --quiet`
  - `cargo test -p ralph-adapters --quiet`
  - `cargo test -p ralph-cli --quiet`
  - `cargo test -p ralph-core smoke_runner --quiet`
  - `cargo test --workspace --quiet`
- [ ] 阶段6: 记录 WORKLOG / ERRORFIX,给出交付结论。

## [2026-04-30 09:21:02] [Session ID: 019dd984-e9a3-7660-8264-86f293870a2b] [完成记录]: 支线任务完成

- [x] 阶段6: 已记录 `WORKLOG__serial_tui_issues.md` 和 `ERRORFIX__serial_tui_issues.md`。
- 当前结论:
  - 非 parallel Codex 报错已从 Ralph 子进程环境隔离层修复。
  - serial TUI 输出选择已补齐应用内选择和复制路径。
  - workspace 级测试通过。
- 后续备忘:
  - 已记录 `LATER_PLANS__serial_tui_issues.md`,后续可统一 serial/parallel 的输出选择 helper。
- 验证备注:
  - 已尝试按 `tui-validate` 检查截图依赖,但本机缺少 `freeze`; 当前以单元测试和 workspace 测试作为完成证据。
