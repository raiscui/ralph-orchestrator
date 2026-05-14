## [2026-04-30 09:19:23] [Session ID: 019dd984-e9a3-7660-8264-86f293870a2b] 任务名称: 修复 serial TUI 选择与 Codex 子进程环境泄漏

### 任务内容

- 查清非 parallel 模式 Codex `failed to record rollout items: thread ... not found` 的来源。
- 修复 serial TUI 输出文本无法选择的问题。
- 不回滚或覆盖工作区内其他已有改动。

### 完成过程

- 先读取支线计划和相关代码路径,确认非 parallel TUI 走 `execute_pty(...)` / `PtyExecutor::spawn_pty(...)`。
- 检查当前环境,确认存在父 Codex 会话变量 `CODEX_THREAD_ID`。
- 新增 Codex 子进程环境隔离 helper,并接入 PTY、CLI、parallel fallback、app-server、mcp-server、SOP runner。
- 对 serial TUI 增加 output selection state,并接入鼠标 Down/Drag/Up、`y` 复制、`Esc` 清空、`Shift+方向键` 扩展选择。
- 复用 `ContentPane::with_selection(...)` 渲染高亮,避免新增第二套输出渲染器。
- 更新 footer 状态提示和 help 文案。

### 验证结果

- `cargo fmt --all`: 通过。
- `cargo test -p ralph-adapters codex_env --quiet`: 通过。
- `cargo test -p ralph-tui serial_output_selection --quiet`: 通过。
- `cargo test -p ralph-tui --quiet`: 通过。
- `cargo test -p ralph-adapters --quiet`: 通过。
- `cargo test -p ralph-cli --quiet`: 通过。
- `cargo test -p ralph-core smoke_runner --quiet`: 通过。
- `cargo test --workspace --quiet`: 通过。

### 总结感悟

- 运行在 Codex 内部的工具再启动 Codex 子进程时,必须主动清理父会话私有环境变量。
- TUI 启用 mouse capture 后,不能依赖终端原生拖选;需要应用内 selection + clipboard 闭环。
- serial 与 parallel 的输出选择应共享渲染抽象,不要再复制一套输出 pane。
