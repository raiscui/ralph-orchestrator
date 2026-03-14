## 1. 基础改造（状态模型与输入分发）

- [x] 1.1 为并行 TUI 增加“可点击区域”的 hit-test 支持（instances/output/chat），并在 `MouseEventKind::Down/Drag/Up` 中做分发
- [x] 1.2 扩展 `ParallelTuiState`：增加 chat 编辑器状态（多行/光标/选择）并保留现有 `chat_status` 行为
- [x] 1.3 扩展并行输出视图状态：增加输出选择（selection）状态（起点/终点/模式），并定义清晰的清空规则（如 Esc）

## 2. 鼠标点选实例 + 焦点切换

- [x] 2.1 实现“鼠标点击实例列表行 → 选中实例”并同步刷新 Output 面板（与键盘上下选择一致）
- [x] 2.2 实现“鼠标点击 Chat 区域 → 聚焦 Chat 输入框”，并在聚焦时显示光标与提示符高亮
- [x] 2.3 更新 TUI help/提示文案：说明鼠标可点选实例、可点击进入 chat

## 3. 输出视图文本选择（多行/框选）

- [x] 3.1 定义输出选择的最小坐标系（以 Output inner area 的屏幕坐标为准），并实现鼠标拖拽框选（Down→Drag→Up）
- [x] 3.2 实现键盘选择（最小集：Shift+方向键扩展/收缩选择），并保证与滚动/搜索可组合
- [x] 3.3 修改/扩展 `ContentPane` 渲染：对落在选择范围内的 cell 应用高亮样式（例如反色或背景色）
- [x] 3.4 为输出选择增加单元测试（使用 `ratatui::backend::TestBackend` 断言高亮区域）

## 4. Chat 多行输入（提示符 + 光标移动 + 选择/框选）

- [x] 4.1 实现 chat 编辑器的基础编辑操作：插入字符、Backspace/Delete、Enter 提交、Shift+Enter 换行
- [x] 4.2 实现 chat 光标移动：键盘左右/上下；并处理行首/行尾的边界行为
- [x] 4.3 实现 chat 文本选择：Shift+方向键选择；鼠标拖拽框选；输入替换选中内容
- [x] 4.4 调整底部 Chat 面板布局：支持展示多行输入（不再限制 input_area=1 行），并确保 gates 列表仍可见
- [x] 4.5 更新 `parse_chat_submit`：支持多行 payload；对 `@instance` 前缀仅解析第一行前缀，其余行保留在 payload
- [x] 4.6 为 chat 编辑器与提交行为补充单元测试（包含 `Shift+Enter` 换行与多行 payload 发送）

## 5. 退出语义：退出 TUI 时清理所有 worker CLI 子进程

- [x] 5.1 修改并行 TUI 的 `q` 行为：从“仅退出 UI”改为“触发全局 interrupt/shutdown”（复用 `interrupt_tx` 路径）
- [x] 5.2 确保 runner 在退出路径中终止并回收所有 HatJob 子进程（graceful → timeout → force），并且不会留下孤儿进程
- [x] 5.3 增加回归测试/验证脚本：覆盖“按 q 退出时 worker 子进程被终止”的场景（尽量避免 flake）

## 6. 文档与验证

- [x] 6.1 更新 TUI 内置帮助与 README（如有）：补充鼠标交互、`Shift+Enter`、退出会终止 worker 的说明
- [x] 6.2 运行并通过：`cargo test`（包含 replay smoke tests），并记录验证结果到 `WORKLOG.md`

## 7. Chat 目标选择（Targets chips）与默认定向发送

- [x] 7.1 在 Chat 面板渲染 Targets chips（展示全部实例，包含 `ralph#1`），并高亮当前 `selected_instance`
- [x] 7.2 支持鼠标点击某个 target chip：切换 `selected_instance`，并同步刷新 Output 面板（等价于点击 instances 列表）
- [x] 7.3 Chat 提交 `human.message` 时：若用户未显式写 `@<HatInstanceId>`，则默认写入 `target_instance=<selected_instance>`

## 8. Gate 选中与快捷操作（chips）

- [x] 8.1 支持鼠标点击 gate 列表行：设为 `selected_gate`，并自动切换 `selected_instance = gate.requested_by`
- [x] 8.2 在 Chat 面板展示当前 gate 的关键信息（gate_id/kind/requested_by/prompt）
- [x] 8.3 渲染 Gate actions chips：`!approve` / `!deny` / `!resolve`；点击后预填输入框（不自动发送）

## 9. 并行运行时：默认订阅 human.message

- [x] 9.1 当 `parallel.enabled: true` 时，自动为所有 hats 补齐 `human.message` 订阅（不要求用户在 triggers 里显式写）
- [x] 9.2 补充单元测试：覆盖“默认消息定向到 selected_instance / gate 点击联动 / action chips 预填 / 自动订阅”关键路径
- [x] 9.3 更新 TUI help/提示文案：说明 Targets chips、gate 点击选中与 actions chips 的用法
- [x] 9.4 重新运行并通过：`cargo test`（包含 replay smoke tests），并记录验证结果到 `WORKLOG.md`
