# 任务计划: 修复 Ralph TUI chat 窗口缺失

## 目标

确认 Ralph TUI 中 chat 窗口消失的真实原因,修复根因,并用 TUI 级测试或捕获证据锁住行为。

## 阶段

- [x] 阶段1: 建立支线上下文并确认调查边界
- [x] 阶段2: 复现或捕获当前 TUI 输出,判断 chat 是未渲染、被挤掉,还是状态未启用
- [x] 阶段3: 阅读 chat 渲染、布局、输入和事件转发路径,形成单一根因假设
- [x] 阶段4: 编写回归测试或最小捕获用例,再做修复
- [x] 阶段5: 运行 TUI/CLI 相关测试和必要验证,记录交付结果

## 关键问题

1. chat 窗口是所有 TUI 都消失,还是仅并行 runtime TUI 消失?
2. 当前运行命令是否进入了并行 Supervisor TUI,还是普通 observation-only TUI?
3. chat pane 的显示是否被高度阈值、focus 状态、delivery mode 或 terminal size 影响?

## 方案方向

- 不惜代价,最佳方案:
  - 用 tmux 捕获真实 TUI 画面,结合渲染单元测试定位缺失层级。
  - 若是布局回归,补布局快照或组件测试。
  - 若是事件/输入回归,补 chat submission / external event writer / parallel TUI state 测试。
- 先能用,后面再优雅:
  - 先找到 chat pane 被隐藏的最小条件,恢复默认显示。
  - 再补一个聚焦测试防止完全消失。

## 做出的决定

- 决定: 使用 systematic-debugging 流程。
  理由: 这是用户看到的 TUI 行为回归,不能先猜是哪个组件坏了。
- 决定: 优先使用 tmux 文本捕获。
  理由: 当前环境有 tmux,但没有检测到 freeze,所以先用可用工具获得动态证据。

## 遇到错误

- 暂无。

## 状态

**全部完成** - 已确认 chat pane 没有从并行 TUI 消失; 用户当前根配置走的是非并行 TUI,所以看不到 chat。

## [2026-04-29 22:44:00] [Session ID: 019dd984-e9a3-7660-8264-86f293870a2b] [完成]: chat 窗口缺失调查结论

- [x] 阶段1: 建立支线上下文并确认调查边界
- [x] 阶段2: 复现或捕获当前 TUI 输出,判断 chat 是未渲染、被挤掉,还是状态未启用
- [x] 阶段3: 阅读 chat 渲染、布局、输入和事件转发路径,形成单一根因假设
- [x] 阶段4: 编写回归测试或最小捕获用例,再做修复
- [x] 阶段5: 运行 TUI/CLI 相关测试和必要验证,记录交付结果

- 结论:
  - 当前根目录 ralph.yml 没有 parallel.enabled=true。
  - `ralph run` 在 config.parallel.enabled=false 时进入 serial loop_runner。
  - chat pane 只在 parallel TUI 中渲染,标题为 Chat / Gates。
- 动态证据:
  - 用临时 parallel idle config 在 tmux 100x30 捕获到 Chat / Gates。
  - 用 100x16、100x14、100x12 也捕获到 Chat / Gates。
  - 所以这不是 chat pane 被小窗口挤掉,也不是当前源码渲染完全坏掉。
- 当前状态:
  - 无需修源码。
  - 如果期望运行根目录默认 `ralph` 时也出现 chat,需要把默认配置切到 parallel workflow,这是配置/产品行为决策,不是本轮 bugfix。
