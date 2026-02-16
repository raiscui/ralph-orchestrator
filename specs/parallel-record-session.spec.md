# Spec: Parallel `--record-session` (Cassette) Wiring

## 背景 / 问题

`ralph run --record-session <FILE>` 的目标是生成一个可回放的 JSONL “cassette”，用于：

- replay smoke tests（确定性回放）
- `ralph-e2e --mock`（用 `ralph-e2e mock-cli` 回放 cassette，零成本 E2E）

当前问题：

1) 串行模式只记录了 `bus.publish`，没有 `ux.terminal.write`，导致 cassette 不可用于回放。

2) 并行模式（parallel runtime）直接忽略 `--record-session`（not wired yet）。

3) mock-mode 运行时，custom backend 默认用 `prompt_mode=arg` 传 prompt，`ralph-e2e mock-cli` 会收到“额外的末尾 prompt 参数”，clap 解析可能直接失败。

---

## 术语

- **cassette**：用 JSONL 记录一次运行的“可回放输出 + 关键事件”。本项目里对应 `SessionRecorder` 的输出格式（`ts/event/data`）。

---

## 目标（Goals）

### G1: 串行模式录制可回放

`ralph run --record-session <FILE>` 在串行模式下 **必须** 写入：

- `_meta.loop_start`
- 至少 1 条 `ux.terminal.write`（内容为“用于 event parsing 的文本输出”，而不是 stderr 日志）
- （可选但推荐）`_meta.termination`
- `bus.publish`（每条业务事件一条）

### G2: 并行模式不再忽略录制

`ralph run --record-session <FILE>` 在并行模式下 **必须**：

- 不再 warning “ignores --record-session”
- 写入 `_meta.loop_start`
- 写入 `bus.publish`（每条业务事件一条）
- 写入 `ux.terminal.write`（stdout+stderr,用 `data.stdout` 区分; 事件解析/回放默认只使用 stdout）

### G3: 并行回放可分流（避免多实例重复回放）

并行模式录制的 `ux.terminal.write` **应该**携带输出归因：

- `instance_id`（例如 `writer#1`）

这样 `mock-cli` 在回放时可以按 `instance_id` 过滤输出，避免：

- 同一份 cassette 被多个 hat instance 重复回放
- 导致事件倍增、路由漂移、E2E 不稳定

### G4: mock-mode custom backend 可用

`ralph-e2e --mock` 写入 workspace 的 `ralph.yml` 时 **必须**：

- 将 `cli.prompt_mode` 设置为 `stdin`，让 `ralph run` 不把 prompt 作为末尾 argv 传给 `mock-cli`
- `mock-cli` 忽略 stdin 内容（仅按 cassette 回放）

---

## 非目标（Non-Goals）

- 不要求录制 TUI frame（`ux.tui.frame`）或 terminal resize/color-mode（可后续补齐）。
- 不追求“逐字节等同于真实终端渲染”；只要求回放可驱动事件解析与 E2E 断言。

---

## 设计要点

1) **stdout-only(event parsing)**：
   - cassette 允许录制 stdout+stderr(便于诊断),但事件解析/ReplayBackend 必须只看 stdout.
   - 目的: 避免 stderr 的 `<event ...>` 假事件污染,导致路由漂移或 completion 假阳性。

2) **低侵入**：尽量用现有 `SessionRecorder` / `Record` / `TerminalWrite`，避免引入新格式。

3) **可回放**：录制文件需要能被 `SessionPlayer` 读取，并被 `ralph-e2e mock-cli` 输出。

4) **可读性(诊断)**：`ux.terminal.write` 的 payload 除了 `bytes`(base64) 之外，还会写入 `text`(UTF-8 lossy)。
   - `text` 仅用于人类直接阅读 JSONL 排障.
   - 回放与事件解析仍以 `bytes` 为准.

---

## 验收标准（Acceptance Criteria）

- `cargo test` 全通过。
- `ralph run --record-session /tmp/x.jsonl` 在串行模式下生成的 JSONL 包含：
  - `_meta.loop_start`
  - `ux.terminal.write`（至少一条）
- `ralph run --record-session /tmp/y.jsonl` 在并行模式下不再输出“忽略 record-session”的 warning，且 JSONL 包含：
  - `_meta.loop_start`
  - `ux.terminal.write`（至少一条）
- `ralph-e2e --mock --filter parallel-hat-instances --backend codex` 能正常启动并执行（不因 mock-cli 参数解析失败而提前退出）。
