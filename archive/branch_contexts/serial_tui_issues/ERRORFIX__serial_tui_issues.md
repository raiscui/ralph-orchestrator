## [2026-04-30 09:19:23] [Session ID: 019dd984-e9a3-7660-8264-86f293870a2b] 错误修复: 非 parallel Codex rollout 报错与 serial TUI 输出无法选择

### 问题

- 非 parallel / serial 模式下,用户看到 Codex stderr:
  - `codex_core::session: failed to record rollout items: thread ... not found`
- serial TUI 中无法选择输出文本。

### 原因

- Codex 报错:
  - Ralph 可能运行在父 Codex 会话内。
  - 当前环境存在 `CODEX_THREAD_ID`。
  - serial PTY / CLI / parallel fallback / app-server / mcp-server 等启动 Codex 子进程时没有清理父 Codex 私有 thread/turn 环境变量。
  - 子 Codex 在 shutdown 时可能把 rollout 写向父会话 thread,导致 `thread not found`。
- serial TUI 输出无法选择:
  - TUI 启用了 mouse capture,终端原生选择会被应用接管。
  - parallel TUI 已实现 output selection + copy。
  - serial TUI 只渲染 `ContentPane`,没有 serial output selection state,也没有 Mouse Down/Drag/Up 或 `y` 复制分支。

### 修复

- 新增 `crates/ralph-adapters/src/codex_env.rs`:
  - 识别 `codex` 命令 basename。
  - 对 `std::process::Command`、`tokio::process::Command`、PTY `CommandBuilder` 清理 `CODEX_THREAD_ID` / `CODEX_TURN_ID`。
- 接入所有 Ralph 启动 Codex 子进程的主要路径:
  - serial PTY: `crates/ralph-adapters/src/pty_executor.rs`
  - non-PTY CLI: `crates/ralph-adapters/src/cli_executor.rs`
  - parallel fallback job: `crates/ralph-cli/src/parallel_runner.rs`
  - Codex app-server: `crates/ralph-cli/src/codex_app_server_session.rs`
  - Codex mcp-server: `crates/ralph-cli/src/codex_mcp_session.rs`
  - SOP interactive runner: `crates/ralph-cli/src/sop_runner.rs`
- serial TUI 增加输出选择能力:
  - `TuiState` 增加 serial output cursor / selection / selecting / status。
  - Mouse Down/Drag/Up 可在 serial content area 中选择并自动复制。
  - `y` 可复制当前 serial output selection。
  - `Esc` 清空 serial output selection。
  - `Shift+方向键` 可扩展 serial output selection。
  - serial `ContentPane` 渲染 selection 高亮。
  - Help / Footer 同步显示 copy 入口和结果。

### 验证

- `cargo fmt --all`: 通过。
- `cargo test -p ralph-adapters codex_env --quiet`: 通过,3 tests。
- `cargo test -p ralph-tui serial_output_selection --quiet`: 通过。
- `cargo test -p ralph-tui --quiet`: 通过,212 + 26 + 4 tests。
- `cargo test -p ralph-adapters --quiet`: 通过,168 tests。
- `cargo test -p ralph-cli --quiet`: 通过。
- `cargo test -p ralph-core smoke_runner --quiet`: 通过,12 smoke tests。
- `cargo test --workspace --quiet`: 通过。
- `tui-validate` 截图验证: 未执行,因为本机缺少 `freeze`; 已记录到 `notes__serial_tui_issues.md`。
