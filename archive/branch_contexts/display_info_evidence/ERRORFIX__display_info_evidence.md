## [2026-05-19 07:57:18] [Session ID: omx-1779004640353-blcixq] 错误修复: Output act/status 遮挡输出

### 现象
- 并行 TUI 的 Output frame 底部新增 `act:` 状态后,用户看到底部输出被遮挡或没有空出状态条空间。

### 原因
- 渲染路径把 Output inner 拆成正文区和 status 区。
- 但 autoscroll 预计算仍使用完整 `output_inner.height`。
- 鼠标选择、复制、键盘扩展选择也使用完整 `output_inner` 高度。
- 这导致“运行时认为可见的正文高度”比实际正文区域高,最后几行容易落到 status strip 的占位之外。

### 修复
- 新增 `split_parallel_output_areas(inner)` 作为 Output 正文区与 status 区的单一几何入口。
- autoscroll 使用 `content_area.height`。
- 渲染、选择、复制、测试 harness 统一使用同一 split 结果。
- status area 点击只聚焦 Output,不创建正文选择。

### 验证
- `cargo test -p ralph-tui --lib split_parallel_output_areas_reserves_bottom_status_rows -- --nocapture`: 通过。
- `cargo test -p ralph-tui --lib widgets::parallel_output::tests::split_parallel_output_areas_reserves_status_rows_outside_content -- --exact --nocapture`: 通过。
- `cargo test -p ralph-tui --lib app::tests::mouse_click_output_status_area_focuses_output_without_starting_selection -- --exact --nocapture`: 通过。
- `cargo test -p ralph-tui --quiet`: 通过。
- `cargo test --quiet`: 通过。

### 过程错误
- 曾运行错误命令: `cargo test -p ralph-tui --lib widgets::parallel_output::tests::split_parallel_output_areas_reserves_status_rows_outside_content app::tests::mouse_click_output_status_area_focuses_output_without_starting_selection -- --nocapture`。
- 原因: `cargo test` 只接受一个 TESTNAME filter。
- 处理: 拆成两条 `--exact` focused test 后通过。
