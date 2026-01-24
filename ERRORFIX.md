# ERRORFIX

## 2026-01-23 16:02 (CST) - `ralph events` 非 ASCII payload panic
- 问题: `ralph events` 在输出表格时，遇到包含中文/emoji 的 payload 可能 panic。
- 影响: 无法稳定查看事件历史；一旦有非 ASCII 内容就会崩溃。
- 预期: 任何合法 UTF-8 字符串都不应导致 CLI 崩溃。

### 根因
- `crates/ralph-cli/src/display.rs` 在展示层做了按字节索引的字符串切片：
  - `truncate()` 里使用 `&s[..max_len - 1]`
  - `print_events_table()` 构造 `payload_preview` 时使用 `&record.payload[..40]`
- 当切片位置落在 UTF-8 多字节字符(中文/emoji)的中间时，Rust 会 panic：`byte index is not a char boundary`。

### 修复
- 引入私有辅助函数 `truncate_prefix_bytes()`：
  - 用 `char_indices()` 计算最后一个合法 UTF-8 边界，再进行切片。
- `truncate()` 改为调用该辅助函数，保持旧的截断行为(仍然追加 "...")，但不再 panic。
- `payload_preview` 先做换行替换，再用相同方式做 UTF-8 安全截断。

#### 补充：时间字段 `ts` 的 UTF-8 边界
- `print_events_table()` 解析 `ts` 的时间字段时，原先也存在 `&time_str[..8]` 这类按字节切片。
- 这在 `ts` 不是预期的 ISO 8601 ASCII 字符串时，同样可能因为 UTF-8 非边界切片而 panic。
- 已修复为：在取前 8 字节前先回退到合法的 `is_char_boundary()` 位置。

### 回归测试
- 新增 2 个单测：
  - `test_truncate_does_not_panic_on_multibyte_chars`
  - `test_print_events_table_does_not_panic_on_multibyte_payload`
- 新增 1 个单测：
  - `test_print_events_table_does_not_panic_on_multibyte_ts`

### 验证
- `cargo test -p ralph-cli`
- `cargo test`
- `cargo fmt --check`
- `cargo clippy -p ralph-cli`

## 2026-01-23 17:07 (CST) - `ralph-e2e` 截断在多字节字符下 panic 风险
- 问题: `crates/ralph-e2e/src/scenarios/*` 多处 `truncate()` 仍用 `&s[..N]` 按字节切片。
- 影响: 一旦 e2e 场景的 stdout/stderr/payload 包含中文/emoji，e2e 断言构造阶段就可能 panic，导致“测试失败原因被 panic 掩盖”。
- 预期: e2e 的错误报告必须稳健，展示层截断不应成为新的故障源。

### 根因
- 7 处重复实现使用 `format!("{}...", &s[..max_len])`，当 `max_len` 落在 UTF-8 多字节字符中间时会 panic。

### 修复
- 将所有 `truncate()` 改为：先用 `is_char_boundary()` 从 `max_len` 向前回退到合法 UTF-8 边界，再切片并追加 "..."。

### 回归测试
- 每个场景文件补充 1 个单测：构造 emoji 靠近边界的字符串，调用 `truncate()`，确保不 panic。

### 验证
- `cargo test -p ralph-e2e`
- `cargo fmt --check`
- `cargo clippy -p ralph-e2e`
