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
