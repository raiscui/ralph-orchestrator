## [2026-05-18 19:16:01] [Session ID: display-info#1] 笔记: display information 证据分层

## 来源

### 来源1: `crates/ralph-cli/src/parallel_runner.rs`

- `current_events_path_for_tui()` 从 `.ralph/current-events` marker 读取当前 events JSONL,缺失时回退 `EventLogger::DEFAULT_PATH`。
- `parallel_evidence_paths_for_tui()` 组装 events、evidence-index、agents snapshot、record-session 四类路径,再注入 TUI。
- `run_parallel_loop_impl()` 只有在 TTY 条件满足时启用 TUI,并通过 `with_parallel_evidence_paths(...)` 把路径传入 UI。
- 日志模式直接输出 `[instance:stream:job=n] line`,其中 stream tag 是 `out` / `err` / `act`。

### 来源2: `crates/ralph-tui/src/state/parallel.rs`

- `ParallelEvidencePaths` 注释明确写着: 这些字段只负责显示 runtime 已经选择好的证据路径,不参与调度、解析、落盘,也不替代 `.ralph/*` 文件本身。
- `InstanceViewState.current_activity` 是实例当前活动状态,用于显示 "Working" / "Inspecting current code behavior" 等标签。
- `JobViewState.raw_lines` 是 Output Rendered / Plain / Audit 三态视图的单一输入。
- Activity 会进入 `raw_lines` 供 Audit 核对,但 Rendered / Plain 正文会忽略 Activity,避免污染正文输出。

### 来源3: `crates/ralph-tui/src/widgets/*`

- `parallel_output.rs` 的 `ParallelOutputStatusPane` 在 Output 底部显示:
  - `evidence: events=... | index=... | agents=... | record=...`
  - `act: <activity> (<elapsed> • Ctrl+C to interrupt)`
- `footer.rs` 在并行模式显示 selected instance、state、job short summary、view mode、last event,但不再显示 activity。
- `instances.rs` 在实例列表显示 instance id、state、job summary、activity short summary 或 last output age。
- `header.rs` 显示 iteration、elapsed time、hat display、live/review mode、scroll/help hint;这是 TUI 视图状态,不是 runtime 真相源。

### 来源4: docs/specs

- `docs/runbook/testing-and-evidence.md` 把 durability 和 display 拆成两类问题: JSONL 中是否存在事件/回复,以及现有 evidence 是否被当前 UI 渲染。
- `specs/parallel-tui-raw-audit-view.md` 明确 raw/audit 视图只改 TUI 展示层,不改变 runtime、event parser、record-session 或调度语义。
- `specs/parallel-tui-raw-audit-view.md` 还要求 Audit 复用 `JobViewState.raw_lines`,不得新增第二套输出缓存。

## 综合发现

- 真实 source of truth 主要是 `.ralph/events*.jsonl`、`.ralph/evidence-index.jsonl`、`.ralph/agents.json`、record-session JSONL、以及 runtime 传来的 `HatJobOutputChunk` / `TuiUpdate`。
- TUI 中的 header/footer/instances/output status 都是 display surfaces。
- `ParallelEvidencePaths` 是典型 display-only state: 它显示路径,不拥有 evidence。
- Audit 视图不是新真相源,只是把 `raw_lines` 以更接近 CLI/log-mode 的形式重渲染出来。
- Display 风险主要不是"没有数据",而是 viewport、mode、宽度截断、stderr hidden、或 docs/API 文档滞后导致用户以为没显示。

## [2026-05-19 07:57:18] [Session ID: omx-1779004640353-blcixq] 笔记: Output act/status 遮挡输出修复

## 来源

### 来源1: `crates/ralph-tui/src/app.rs`

- 现象:
  - 渲染路径已经把 Output inner area 拆成正文区和底部 status 区。
  - 但 autoscroll 预计算曾使用完整 `output_inner.height`,没有扣掉底部 `evidence:` / `act:` status strip。
- 修复:
  - autoscroll 改为使用 `split_parallel_output_areas(output_inner).content_area.height`。
  - `ParallelLayoutSnapshot` 同时保留 full output inner、content area、status area。
  - 鼠标框选、拖拽、复制、键盘扩展选择都改用 `output_content_area` 的宽高。
  - 点击 status area 只聚焦 Output,不再创建 output selection anchor。

### 来源2: `crates/ralph-tui/src/widgets/parallel_output.rs`

- 新增 `ParallelOutputAreas` 与 `split_parallel_output_areas(inner)`。
- 该 helper 是 Output 正文区和底部 status 区的单一几何真相源。
- 正文区负责 stdout/stderr、选择、复制、滚动。
- status 区负责 `evidence:` / `act:` 展示,不参与正文 viewport。

### 来源3: `crates/ralph-tui/tests/common/mod.rs`

- 测试 harness 改为复用同一个 `split_parallel_output_areas` helper。
- 避免测试布局与真实 App 布局再次漂移。

## 综合发现

### 已验证结论

- 这次不是 runtime 输出缺失,而是 display viewport 高度口径不一致。
- status strip 本身可以继续保留在 Output 底部。
- 真正需要修的是正文 viewport / autoscroll / selection / copy 都必须扣掉 status strip。

### 验证命令

- `cargo test -p ralph-tui --quiet`: 通过,229 个 lib tests、26 个 integration tests、4 个附加 tests 均通过。
- `cargo test --quiet`: 通过,workspace 全量测试 exit code 0。
