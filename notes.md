# notes.md

## [2026-05-17 16:55:40] [Session ID: omx-1779004640353-blcixq] 笔记: notes 续档入口

## 来源

### 来源1: archived previous notes

- 归档文件: `archive/default_history/notes_2026-05-17_1655_tui_status_prev.md`
- 原行数: 1166
- 触发条件: 默认六文件中的 `notes.md` 超过 1000 行,按项目规则续档。

## 综合发现

### 当前任务摘要

- 本轮已完成 TUI 与 Codex/CLI 直接输出差异排查。
- 已落地最小 TUI 状态增强:
  - Instances 行显示 `job x/y`。
  - Footer 并行模式显示 selected instance、紧凑 job、render mode、last event。
- 验证通过:
  - `cargo fmt --all -- --check`
  - focused ralph-tui widget tests
  - `cargo test --package ralph-tui`
  - `cargo test`
  - `git diff --check`

### 可复用经验

- TUI 信息缺失类问题要先区分 runtime truth 是否存在,再判断是否只是展示层没有聚合。
- 并行 TUI 状态展示应复用 `ParallelTuiState` / `InstanceViewState` / `TuiState.last_event`,不要新增第二套状态真相源。
- Footer 是窄空间,要用紧凑标签;长 label 应放到 Output title、Instances、raw/audit 面板或详情视图。

### 后续仍未完成

- raw/audit 视图仍未落地。
- stderr visible/hidden 仍需从 runner flag 正式进入 TUI state。
- evidence/status 面板仍未落地。

## [2026-05-17 17:10:00] [Session ID: omx-1779004640353-blcixq] 笔记: Codex 原生状态行与 Ralph 并行 TUI

## 来源

### 来源1: `crates/ralph-cli/src/parallel_runner.rs`

- 要点:
  - 普通并行 backend 使用 `BufReader::lines()` 分别读取 stdout 和 stderr。
  - stderr 会作为 `HatJobOutputChunk` 发送给 Supervisor,但不会进入 event parsing。
  - TUI observer 默认发送 stderr chunk,只有 `--hide-stderr` 才隐藏显示。

### 来源2: `crates/ralph-cli/src/codex_app_server_session.rs`

- 要点:
  - app-server 路径不会直接显示 Codex 原生 TUI 的 status bar。
  - 当前把 prompt transcript、stderr、reasoning summary / agentMessage delta 映射成 Ralph 自己的 stdout/stderr chunk。
  - `codex/event/task_started` 当前用于 steer flush 门槛,没有映射成人类可见的 `Working...` 状态文案。

### 来源3: `crates/ralph-tui/src/state/parallel.rs` 与 `crates/ralph-tui/src/state/parallel/output.rs`

- 要点:
  - 并行 TUI 按 job 保存 raw_lines,再渲染为可见行。
  - stderr 默认灰色弱化,不加 `[stderr]` 前缀。
  - 显示层会过滤控制字符,因此 `\r` 这类 TTY 原地刷新控制符不会成为稳定可读状态行。

## 综合发现

- stderr 的普通文本行: 当前并行 TUI 默认会显示。
- Codex 原生交互 UI 的临时状态条,如 `Working... esc to interrupt`: 当前不会被 Ralph 稳定当作状态字段显示。
- 如果这类状态条以 newline 形式从 stderr/stdout 输出,可能被当普通输出行显示。
- 如果它是 TTY 原地刷新/ANSI 控制序列,当前 TUI 不会稳定保留成“当前动作”状态。

## 验证

- `cargo test --package ralph-cli --bin ralph tests::run_args_show_stderr_defaults_to_true -- --exact`: passed。
- `cargo test --package ralph-tui --lib state::parallel::tests::parallel_output_stderr_markdown_rendering_matches_renderer_output -- --exact`: passed。

## [2026-05-17 18:18:00] [Session ID: omx-1779004640353-blcixq] 笔记: Codex 风格 current activity 落地验证

## 来源

### 来源1: `crates/ralph-core/src/activity.rs`

- 要点:
  - 新增 activity 文本归一化 helper。
  - 只处理已经成为可见文本的状态行,不解析私有 TTY 控制序列。
  - 可以把 `• Working (11s • esc to interrupt)` 归一成 `Working`。
  - 可以识别 `Inspecting current code behavior` 这类 reasoning 状态文案。

### 来源2: `crates/ralph-cli/src/codex_app_server_session.rs`

- 要点:
  - `codex/event/task_started` 映射为 `OutputStream::Activity` 的 `Working`。
  - `item/reasoning/summaryTextDelta` 和 agent message delta 中可识别的状态文本会映射为 activity。
  - activity 只发给 UI/observer,不参与 stdout 正文组装和 event parser。

### 来源3: `crates/ralph-tui/src/state/parallel.rs`

- 要点:
  - `InstanceViewState` 新增 `current_activity` 和 `state_since`。
  - `OutputStream::Activity` 只更新当前状态,不追加到正文 output buffer。
  - 普通 stdout/stderr 中如果出现稳定可见的 `Working...` / `Inspecting...` 行,也会 best-effort 更新 activity。

### 来源4: `crates/ralph-tui/src/widgets/footer.rs` 和 `crates/ralph-tui/src/widgets/instances.rs`

- 要点:
  - Footer 在并行模式下优先显示 `Activity (Ns • Ctrl+C to interrupt)`。
  - Instances 行显示 `a:<activity elapsed>` 的短摘要。
  - Footer 继续显示 selected instance、state、job、render mode 和 last event。

## 综合发现

- 现在并行 TUI 会稳定显示 Codex 风格的“当前正在做什么”。
- 中断提示使用 Ralph TUI 的真实交互键 `Ctrl+C to interrupt`,不是 Codex 原生 `esc to interrupt`。
- Activity 是状态流,不进入正文输出,也不进入事件解析。
- `stderr` 普通行仍默认显示; `Activity` 与 stderr 是否隐藏是两件事。

## 验证

- `cargo test -p ralph-cli`: passed。
- `cargo fmt --all -- --check`: passed。
- `cargo test`: passed。
- `git diff --check`: passed。

## [2026-05-17 19:05:00] [Session ID: omx-1779004640353-blcixq] 笔记: 并行 TUI raw/audit 视图

## 来源

### 来源1: `specs/parallel-tui-raw-audit-view.md`

- 要点:
  - Output 视图三态: Rendered / Plain / Audit。
  - `v` 键循环切换。
  - Audit 复用 `JobViewState.raw_lines`,不新增第二套输出缓存。

### 来源2: `crates/ralph-tui/src/state/parallel.rs`

- 要点:
  - 新增 `ParallelOutputViewMode`。
  - Audit 渲染格式: `[instance:stream:job=n] line`。
  - Activity 在 Rendered/Plain 仍不进入正文,但在 Audit 中可见。

## 验证

- `cargo test --package ralph-tui --lib`: passed。
- `cargo test -p ralph-tui`: passed。
- `cargo test`: passed。
- `cargo fmt --all -- --check`: passed。
- `git diff --check`: passed。
