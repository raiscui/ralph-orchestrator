# 任务计划: display information 真实证据只读调查

## [2026-05-18 19:12:37] [Session ID: display-info#1] [记录类型]: 新任务启动

# # 目标

只基于当前仓库 docs 和 code,说明哪些 UI/TUI/日志/状态栏信息是展示层,哪些才是 source of truth,并给出 fresh verification evidence。

# # 阶段

- [x] 阶段1: 读取项目上下文与历史风险,确认只读边界
- [ ] 阶段2: 收集 TUI/CLI/docs/specs 静态证据
- [ ] 阶段3: 运行 focused verification 命令
- [ ] 阶段4: 整理 findings、risks、next-step recommendations

# # 关键问题

1. 哪些信息显示在 UI/TUI/日志/面板/状态栏里?
2. 这些信息是 display-only,还是 runtime / record / state 的 source of truth?
3. 当前仓库的 focused tests 是否能验证这些 display 信息仍然存在?

# # 做出的决定

- 采用只读调查,不改代码。
- 只聚焦 display information,不扩散到 multi-agent 或 coordinator 设计。
- 验证优先使用现有 focused tests 和文本检索,避免启动 live backend。

# # 遇到错误

- 暂无。

# # 状态

**目前在阶段2** - 开始收集当前仓库的静态证据。

## [2026-05-18 19:16:01] [Session ID: display-info#1] [记录类型]: 阶段推进

已完成:
- [x] 阶段2: 收集 TUI/CLI/docs/specs 静态证据。
- [x] 阶段3: 运行 focused verification 命令。

静态证据摘要:
- `crates/ralph-cli/src/parallel_runner.rs` 负责把 `.ralph/current-events`、`.ralph/evidence-index.jsonl`、`.ralph/agents.json`、`--record-session` 路径转换成 TUI 展示用路径。
- `crates/ralph-tui/src/state/parallel.rs` 明确 `ParallelEvidencePaths` 是展示字段,不参与调度、解析或落盘。
- `crates/ralph-tui/src/widgets/parallel_output.rs` 的 `ParallelOutputStatusPane` 在 Output 底部显示 `evidence:` 与 `act:`。
- `crates/ralph-tui/src/state/parallel.rs` 的 `raw_lines` 是 Output 三态视图的单一显示输入,Audit 从这里重建 stdout/stderr/activity 归因。
- docs/runbook 将 durability 与 display 明确拆开: JSONL 是否存在是一层,UI 是否渲染是另一层。

验证命令:
- `cargo test -p ralph-tui --lib widgets::parallel_output::tests::output_status_pane_shows_evidence_paths -- --exact`: 1 passed。
- `cargo test -p ralph-tui --lib widgets::parallel_output::tests::output_status_pane_places_activity_on_bottom_line -- --exact`: 1 passed。
- `cargo test -p ralph-tui --lib state::parallel::tests::parallel_output_audit_shows_stream_job_and_activity_lines -- --exact`: 1 passed。
- `cargo test -p ralph-tui --lib state::parallel::tests::parallel_output_view_mode_switch_rerenders_existing_raw_lines -- --exact`: 1 passed。
- `cargo test -p ralph-tui --lib widgets::footer::tests::footer_shows_parallel_status_summary -- --exact`: 1 passed。
- `cargo test -p ralph-cli --bin ralph parallel_runner::tests::parallel_evidence_paths_for_tui_use_current_events_marker -- --exact`: 1 passed。
- `cargo test -p ralph-tui --test integration_snapshots test_parallel_full_layout_renders_instances_output_and_gates -- --exact`: 1 passed。

遇到错误:
- `cargo test -p ralph-cli --lib parallel_runner::tests::parallel_evidence_paths_for_tui_use_current_events_marker -- --exact` 返回 0 tests,不能作为有效验证。已改用 `--bin ralph` 重跑并通过。

状态:
**目前在阶段4** - 整理 findings、risks、next-step recommendations。

## [2026-05-18 19:16:59] [Session ID: display-info#1] [记录类型]: 任务收口

已完成:
- [x] 阶段4: 整理 findings、risks、next-step recommendations。

交付边界:
- 未修改代码。
- 只新增/更新支线调查上下文文件和默认 `task_plan.md` 的支线索引。
- 当前仓库已有大量非本 Session 改动,本任务不触碰、不回滚。

状态:
**任务完成** - 可以输出简短调查结论。

## [2026-05-19 08:58:00] [Session ID: omx-1779004640353-blcixq] [记录类型]: 新缺陷修复启动

用户反馈:
- Output frame 最下面因为新增 `act:` 状态遮挡输出,输出需要为空出状态条空间。

现象:
- 渲染阶段确实把 Output inner area 分成 `output_content_area` 与 `output_status_area`。
- 但自动滚动预计算阶段仍使用完整 `output_inner.height` 作为 viewport height。

当前假设:
- 由于 autoscroll 认为可见高度比实际内容区高 1-2 行,跟随底部时 `scroll_offset` 少滚了 status 高度,最后输出被底部 status 区隐藏。

最强备选解释:
- 不是 autoscroll 计算错误,而是 status strip 缺少视觉分隔行,导致用户感知像遮挡。

验证计划:
- [ ] 阶段1: 把 Output content/status split 抽成单一 helper,让预计算和渲染复用。
- [ ] 阶段2: 增加回归测试,断言并行 Output autoscroll 使用扣除 status strip 后的内容高度。
- [ ] 阶段3: 运行 focused TUI tests。
- [ ] 阶段4: 更新 notes / WORKLOG / ERRORFIX 并交付。

状态:
- **正在阶段1** - 先做最小正确修复,不改变 status strip 是否显示。

## [2026-05-19 07:57:18] [Session ID: omx-1779004640353-blcixq] [记录类型]: Output status 遮挡修复完成

已完成:
- [x] 阶段1: 把 Output content/status split 抽成单一 helper。
- [x] 阶段2: 让 autoscroll、渲染、鼠标选择、复制、测试 harness 统一使用 content viewport。
- [x] 阶段3: 增加回归测试,覆盖 status strip 预留高度和 status 区点击不创建正文选择。
- [x] 阶段4: 运行 focused tests、`ralph-tui` crate tests、workspace `cargo test --quiet`。

验证结果:
- `cargo test -p ralph-tui --lib split_parallel_output_areas_reserves_bottom_status_rows -- --nocapture`: 通过。
- `cargo test -p ralph-tui --lib widgets::parallel_output::tests::split_parallel_output_areas_reserves_status_rows_outside_content -- --exact --nocapture`: 通过。
- `cargo test -p ralph-tui --lib app::tests::mouse_click_output_status_area_focuses_output_without_starting_selection -- --exact --nocapture`: 通过。
- `cargo test -p ralph-tui --lib widgets::parallel_output::tests::output_status_pane_places_activity_on_bottom_line -- --exact --nocapture`: 通过。
- `cargo test -p ralph-tui --test integration_snapshots test_parallel_full_layout_renders_instances_output_and_gates -- --exact --nocapture`: 通过。
- `cargo test -p ralph-tui --quiet`: 通过。
- `cargo test --quiet`: 通过。
- `git diff --check` 针对本轮相关文件: 通过。

遇到错误:
- 曾错误把两个 libtest filter 写进同一个 `cargo test` 命令,libtest 只接受一个 TESTNAME。已拆成两条 focused test 重跑并通过。

状态:
- **Output 底部 status strip 遮挡输出的问题已修复并验证通过**。

## [2026-05-19 07:58:00] [Session ID: omx-1779004640353-blcixq] [记录类型]: 时间戳修正说明

修正说明:
- 上一条“新缺陷修复启动”记录标题中的 `2026-05-19 08:58:00` 是手写时间笔误。
- 本轮真实顺序以文件尾部完成记录和验证命令为准:实现、测试、收尾均在 `2026-05-19 07:57` 前后完成。

状态:
- **仅修正记录解释,代码和验证结论不变**。
