## [2026-04-30 09:07:16] [Session ID: 019dd984-e9a3-7660-8264-86f293870a2b] 笔记: 非 parallel Codex 报错与 serial TUI 选择缺失

## 来源

### 来源1: 用户报错

- 报错文本:
  - `codex_core::session: failed to record rollout items: thread ... not found`
- 初步判断:
  - 这是 Codex 子进程在 shutdown / rollout record 阶段写入某个 thread 失败,不是 Ralph core 自己抛出的错误。

### 来源2: 当前环境变量

- `env | rg "SESSION|CODEX|OMX|RALPH|PWD"` 显示:
  - `CODEX_THREAD_ID=019dd984-e9a3-7660-8264-86f293870a2b`
  - `CODEX_CI=1`
  - `CODEX_MANAGED_BY_NPM=1`
- 综合判断:
  - Ralph 在 Codex 会话内运行时,子后端 `codex exec` 会继承父 Codex 的 `CODEX_THREAD_ID`。
  - 这会把子 Codex 的会话记录逻辑指向父线程或已不存在线程,符合用户看到的 thread not found 形态。

### 来源3: serial 执行路径

- `crates/ralph-cli/src/loop_runner.rs`:
  - 非 parallel TUI 走 `execute_pty(...)`。
  - TUI 模式下执行 `exec.run_observe_streaming(...)`,输出进入 `TuiStreamHandler`。
- `crates/ralph-adapters/src/pty_executor.rs`:
  - `spawn_pty(...)` 通过 `CommandBuilder::new(&cmd)` 启动 backend。
  - 当前只设置了 `TERM=xterm-256color`,没有清理 `CODEX_THREAD_ID` / `CODEX_TURN_ID`。

### 来源4: TUI 选择实现

- `crates/ralph-tui/src/state/parallel.rs`:
  - parallel 有 `output_cursor`、`output_selection`、`output_selecting`。
  - 有 `start_output_selection`、`update_output_selection_cursor`、`finish_output_selection`。
- `crates/ralph-tui/src/app.rs`:
  - parallel Mouse Down/Drag/Up 分支会更新 output selection,Mouse Up 后自动 copy。
  - serial 分支只处理滚动和键盘 action,没有 mouse selection。
- `crates/ralph-tui/src/widgets/content.rs`:
  - `ContentPane` 已支持 `.with_selection(...)`。
  - 因此 serial TUI 不是渲染能力缺失,而是状态和输入分发没有接上。

## 综合发现

- Codex 报错更像父 Codex 会话环境变量泄漏到子 `codex exec`。
- serial TUI 无法选择文本是实现缺口:
  - TUI 启用了 mouse capture,终端原生拖选通常被应用接管。
  - parallel 模式已有应用内选择与复制。
  - serial 模式缺少等价的选择状态和事件处理。

## [2026-04-30 09:22:53] [Session ID: 019dd984-e9a3-7660-8264-86f293870a2b] 笔记: TUI 截图验证条件

## 来源

### 来源1: `tui-validate` skill

- 该 skill 要求先确认 `freeze` 是否安装。
- 本机 `tmux -V` 可用,返回 `tmux 3.6a`。
- 本机 `command -v freeze` 无输出,`/opt/homebrew/bin/freeze`、`/usr/local/bin/freeze`、`~/.cargo/bin/freeze` 均未找到。

## 综合发现

- 本次不能用 `freeze` 进行截图级 TUI 验证。
- 已改用代码级验证:
  - serial mouse down selection 单元测试。
  - serial keyboard selection 单元测试。
  - `ContentPane` selection extraction 既有 serial 测试。
  - `cargo test --workspace --quiet` 完整通过。
