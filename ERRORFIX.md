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

## 2026-01-24 15:50 (+0800) - TUI/流式输出在中文场景下丢字
- 问题: Ralph 默认启用的 TUI 在显示中文时出现“丢字”（复现用例中，字符 `中` 会被吞掉）。
- 影响:
  - TUI 观察模式下，助手输出/工具输出包含中文时可能缺字，影响可读性与调试。
  - 该问题不只影响 TUI：只要走 `run_observe_streaming()` 的流式输出链路（即使 `--no-tui`），同样可能丢字。

### 根因
- `crates/ralph-adapters/src/pty_executor.rs` 的 `run_observe_streaming()` 把 PTY 读到的 bytes 当成“每个 chunk 都是完整 UTF-8”来处理：
  - 逐 chunk `std::str::from_utf8(&data)`。
  - 一旦 chunk 末尾落在中文/emoji 多字节字符中间，`from_utf8` 会失败。
  - 旧逻辑在失败时直接跳过该 chunk，导致合法输出被丢弃（中文丢字最明显）。

### 修复
- 新增 `Utf8StreamDecoder`（UTF-8 增量解码器）：
  - 对“不完整的 UTF-8 尾部”做缓存，等下一个 chunk 拼接补齐后再输出。
  - 对“确实非法的字节序列”输出 U+FFFD 并跳过，避免卡死。
- `run_observe_streaming()` 的两处输出处理点（主循环 + child exit drain）都改为通过该解码器输出文本片段。
- 在 EOF / drain 结束处增加 `flush_lossy()`，避免极端情况下尾部 bytes 永远留在缓冲区。

### 回归测试
- `test_utf8_stream_decoder_handles_split_multibyte_char`
- `test_utf8_stream_decoder_replaces_invalid_bytes_and_continues`

### 验证
- `cargo test -p ralph-adapters`
- `cargo test`
- 人工复现/验证（自定义 backend）：
  - 输出构造：`'a'*4095 + '中' + '<MARK>' + 'b'*20`（确保 `中` 被放在 4096 bytes 边界上）
  - 观察：TUI 与 `--no-tui` 均能稳定看到 `中<MARK>`，不再丢字

## 2026-01-25 01:12 (+0800) - TUI 中文宽字符导致错位/吞英文首字母
- 问题: TUI 内容区在“中文 + 紧随其后的英文路径/单词”场景下显示错位：
  - 中文看起来像被插入空格
  - 英文首字母被吞（例如 `search` 显示成 `earch`）
- 影响: 实际内容里常见中文后直接跟路径（例如 `新增示例合集目录：examples/.../README.md:1`），会导致关键信息缺失。

### 根因
- `crates/ralph-tui/src/widgets/content.rs` 的 `ContentPane::render()` 旧实现按 `chars()` 逐个写入，并且每次 `x += 1`。
- 但中文/CJK、emoji 等字符在终端里通常是“双宽”（占两列）：
  - 写入第 1 列后，第 2 列是 continuation cell，终端渲染会跳过
  - 旧实现仍把紧随其后的 ASCII 写进 continuation cell
  - 结果：ASCII 首字母在终端渲染时被跳过 → “吞首字母/错位”

### 修复
- 改为按 grapheme cluster 渲染（`unicode-segmentation`），避免把组合 emoji 等拆开。
- 用 `unicode-width` 计算每个 grapheme 的显示宽度，并按宽度推进光标。
- 写入宽 grapheme 后 reset 被遮挡的 cell，保持与 ratatui `Buffer::set_stringn` 的语义一致。
- 软换行前先清理本行剩余格子，避免上一帧残影（artifact）。

### 回归测试
- 新增: `cjk_double_width_does_not_swallow_next_ascii_char`
  - 用 `"将search/notes"` 断言 `buf[(1,0)]` 为 `" "` 且 `buf[(2,0)]` 为 `"s"`，确保不会再把 ASCII 写进 continuation cell。

### 验证
- `cargo fmt`
- `cargo test -p ralph-tui`
- `cargo clippy -p ralph-tui`
- `cargo test -p ralph-core smoke_runner`
- `cargo test -p ralph-core kiro`
- `cargo test`
