## 1. CLI 接入并行 TUI（替换当前“仅日志”）

- [x] 1.1 在 `crates/ralph-cli` 的并行入口启用真实 TUI（TTY 环境下），移除/替换 “no TUI” 警告路径
- [x] 1.2 为并行模式建立 TUI update 通道：输出 chunk + 实例状态 + gate 事件（observer → channel）
- [x] 1.3 打通 TUI 退出与并行 Supervisor 的生命周期（quit/Ctrl-C/interrupt 不留残态）

## 2. `ralph-tui` 并行模式 State（instance → jobs → buffer）

- [x] 2.1 引入 `TuiMode::Parallel`（或等价结构），确保串行 TUI 路径不被破坏
- [x] 2.2 新增并行 state：`instances: HashMap<HatInstanceId, InstanceViewState>`，并提供 reducer（apply_update）接口
- [x] 2.3 为输出 buffer 增加容量上限（ring buffer），避免长跑场景内存无限增长

## 3. UI：实例列表 + 实例输出详情（滚动/搜索）

- [x] 3.1 新增/扩展 widgets：左侧实例列表（id/state/时间线索），右侧实例输出视图（复用现有 content 渲染器）
- [x] 3.2 实现三 pane 的焦点与导航：Tab 切换、上下选择实例、Enter 进入详情/返回
- [x] 3.3 复用现有搜索与滚动能力：`/` 搜索、滚动查看历史输出

## 4. HatJob 分段与历史切换

- [x] 4.1 在并行运行时补齐 job 边界信号（最小改动：让 output chunk 携带 `job_id`，或新增 job started/ended update）
- [x] 4.2 TUI 按 `job_id` 分段维护 `jobs: Vec<JobBuffer>`，并支持在 job 历史间切换
- [x] 4.3 在 UI 中显示当前 job 索引/总数与运行态（帮助快速定位“当前在跑哪一段”）

## 5. Human async chat（写入 human.message）

- [x] 5.1 增加 chat 输入框与输入模式（底部 pane），支持编辑/提交/取消
- [x] 5.2 支持 `@<HatInstanceId>` 定向：解析前缀并写入 `human.message` 事件（设置 `target_instance`，payload 为消息文本）
- [x] 5.3 实现 ExternalEventWriter：向 `.ralph/current-events` 指向的 JSONL 追加事件；写入失败时在 UI 中可见提示

## 6. Gate 面板（展示 + 交互 resolve）

- [x] 6.1 TUI state 维护 open gates：消费 `gate.request/gate.timeout/gate.resolve` 更新并同步 UI
- [x] 6.2 UI 展示 gate 列表与倒计时（timeout_seconds），并对已 resolve/timeout 的 gate 做状态更新
- [x] 6.3 支持 `!approve/!deny/!resolve` 输入：生成 `gate.resolve`（payload 可反序列化为 `GateResolve`）并落盘

## 7. 验证与回归（背压门槛）

- [x] 7.1 单元测试：chat 输入解析（`@instance` / 非定向）与 ExternalEventWriter 的事件格式
- [x] 7.2 单元测试：gate reducer（request → open，resolve/timeout → closed）与倒计时展示逻辑
- [x] 7.3 replay fixture：补充并行 fixture 覆盖 `human.message` 与 gate 交互链路（保证可回放）
- [x] 7.4 TUI 验证：使用 `/tui-validate` 对并行 Supervisor TUI 的布局与 gate 面板做回归验证
- [x] 7.5 全量验证：`cargo fmt --check`、`cargo clippy --workspace --all-targets`、`cargo test`
