# WORKLOG

## 2026-01-23 16:02 (CST)
- 新建 `task_plan.md` / `notes.md` / `WORKLOG.md` / `ERRORFIX.md`，进入四文件上下文模式。

## 2026-01-23 16:10 (CST)
- 修复 `crates/ralph-cli/src/display.rs`：把 `truncate()` 和事件表格里的 `payload_preview` 改成 UTF-8 安全截断，避免 `&str` 在非字符边界切片导致 panic。
- 增加回归测试：覆盖多字节字符(emoji)在截断边界附近的场景，确保不再出现 "not a char boundary"。
- 验证：已运行 `cargo test -p ralph-cli`、`cargo test`、`cargo fmt --check`、`cargo clippy -p ralph-cli`，全部通过。

## 2026-01-23 17:07 (CST)
- 修复 `crates/ralph-e2e/src/scenarios/*.rs`：把 7 处重复 `truncate()` 从按字节切片改为 UTF-8 安全截断，避免 e2e 在输出含中文/emoji 时 panic。
- 增加回归测试：每个场景文件各补 1 个 “多字节字符靠近边界不 panic” 的测试用例。
- 验证：已运行 `cargo test -p ralph-e2e`、`cargo fmt --check`、`cargo clippy -p ralph-e2e`，全部通过。

## 2026-01-23 17:29 (CST)
- 加固 `crates/ralph-cli/src/display.rs`：修复时间字段 `ts` 的截断逻辑，避免异常 `ts` 含多字节字符时 `&time_str[..8]` 触发 panic。
- 增加回归测试：新增 `test_print_events_table_does_not_panic_on_multibyte_ts` 覆盖该场景。
- 验证：已运行 `cargo test -p ralph-cli`、`cargo test`、`cargo fmt --check`、`cargo clippy -p ralph-cli`，全部通过。

## 2026-01-24 15:50 (+0800)
- 排查并修复 “TUI/流式输出中文丢字” 问题：根因是 PTY 流式输出逐 chunk `from_utf8`，在多字节字符跨 chunk 时会解码失败并丢弃 chunk。
- 实现 `Utf8StreamDecoder` 做 UTF-8 增量解码，并接入 `crates/ralph-adapters/src/pty_executor.rs` 的 `run_observe_streaming()`（主循环 + drain）。
- 增加回归测试：覆盖“拆分中文字符”和“非法字节替换并继续”两类场景。
- 验证：已运行 `cargo test -p ralph-adapters`、`cargo test`；并用 `.ralph/tui_chinese_custom.yml` 人工验证 TUI 与 `--no-tui` 均能看到 `中<MARK>`。

## 2026-01-25 01:12 (+0800)
- 修复 “TUI 中文宽字符导致错位/吞英文首字母”：
  - 根因：`crates/ralph-tui/src/widgets/content.rs` 的 `ContentPane::render()` 按 `chars()` 写入并 `x += 1`，把 ASCII 写进 CJK/emoji 的 continuation cell，终端渲染会跳过该格。
  - 修复：改为按 grapheme cluster 渲染，并按显示宽度推进光标；软换行前清理本行剩余格子避免残影。
- 增加回归测试：`cjk_double_width_does_not_swallow_next_ascii_char`（覆盖 `"将search/notes"`）。
- 验证：已运行 `cargo fmt`、`cargo test -p ralph-tui`、`cargo clippy -p ralph-tui`、`cargo test -p ralph-core smoke_runner`、`cargo test -p ralph-core kiro`、`cargo test`，全部通过。

## 2026-01-25 23:21 (+0800)
- 将未提交变更中新增的中文代码注释翻译为英文（只改注释文本，不改动测试用的中文字符串）。
- 涉及文件：`crates/ralph-adapters/src/pty_executor.rs`、`crates/ralph-tui/src/widgets/content.rs`。
- 验证：已运行 `cargo fmt --check`、`cargo test`，全部通过。
