## [2026-05-18 19:16:59] [Session ID: display-info#1] 任务名称: display information 真实证据只读调查

### 任务内容
- 只基于当前仓库 docs 和 code,调查 display information 的真实证据。
- 聚焦 `crates/ralph-tui/`、`crates/ralph-cli/`、`docs/`、`specs/` 中 evidence/status/activity/view 相关路径。

### 完成过程
- 静态读取 CLI path 注入、TUI state、Output status pane、Footer、Instances、Header、docs/runbook 和 raw/audit spec。
- 运行 focused tests 验证 evidence path、activity status、audit attribution、footer summary、完整布局 smoke。
- 识别并排除一次无效验证命令: `ralph-cli --lib` 没有覆盖 bin 内测试。

### 总结感悟
- display information 的关键边界是: TUI/CLI 可以显示证据路径和状态摘要,但 `.ralph/*` 与 record-session 才是 durable evidence。
- Audit 视图仍然是展示层,不是新的 source of truth;它复用 `raw_lines` 重建归因。

## [2026-05-19 07:57:18] [Session ID: omx-1779004640353-blcixq] 任务名称: 修复 Output 底部 act/status 遮挡输出

### 任务内容
- 修复并行 TUI Output frame 底部 `act:` 状态条占位不一致导致的输出遮挡/缺尾问题。
- 涉及文件:
  - `crates/ralph-tui/src/widgets/parallel_output.rs`
  - `crates/ralph-tui/src/app.rs`
  - `crates/ralph-tui/tests/common/mod.rs`

### 完成过程
- 新增 `split_parallel_output_areas` 作为正文区与 status 区的单一几何真相源。
- App 渲染路径改用该 helper。
- autoscroll 预计算改用 content viewport 高度,不再使用完整 output inner 高度。
- 鼠标选择、拖拽、复制、键盘选择扩展都改用 `output_content_area`。
- status area 点击现在只聚焦 Output,不会创建正文选择。
- 测试 harness 改为复用同一 helper,减少测试布局漂移。

### 验证
- `cargo test -p ralph-tui --lib split_parallel_output_areas_reserves_bottom_status_rows -- --nocapture`: 通过。
- `cargo test -p ralph-tui --lib widgets::parallel_output::tests::split_parallel_output_areas_reserves_status_rows_outside_content -- --exact --nocapture`: 通过。
- `cargo test -p ralph-tui --lib app::tests::mouse_click_output_status_area_focuses_output_without_starting_selection -- --exact --nocapture`: 通过。
- `cargo test -p ralph-tui --test integration_snapshots test_parallel_full_layout_renders_instances_output_and_gates -- --exact --nocapture`: 通过。
- `cargo test -p ralph-tui --quiet`: 通过。
- `cargo test --quiet`: 通过。

### 总结感悟
- Output pane 里一旦加入 display-only status strip,所有 viewport 相关逻辑都必须以 content area 为准。
- 渲染、滚动、选择、复制和测试 harness 不能各自计算区域,否则用户看到的最后几行就会和 runtime buffer 的底部不一致。
