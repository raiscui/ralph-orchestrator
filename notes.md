# 笔记：`ralph events` 非 ASCII payload 导致 panic

> 创建时间: 2026-01-23 16:02 (CST)

## 现象
- 运行 `ralph events` 时，如果某条事件的 payload 含中文/emoji 等非 ASCII 字符，可能触发 panic。

## 初步定位(代码点)
### `crates/ralph-cli/src/display.rs`
- `truncate(s, max_len)` 使用 `&s[..max_len - 1]` 按字节切片。
  - 当 `max_len - 1` 落在 UTF-8 多字节字符中间时，会 panic: "byte index is not a char boundary"。
- `print_events_table()` 里构造 `payload_preview` 时使用 `&record.payload[..40]`，同样会在非 ASCII payload 上 panic。
- `print_events_table()` 里解析时间字段时使用 `&time_str[..N]`（取前 8 字节），如果 `ts` 文本异常包含多字节字符，也可能 panic。

## 修复方案(已实现)
### `crates/ralph-cli/src/display.rs`
- 用 `char_indices()` 计算合法 UTF-8 边界，再进行 `&s[..boundary]` 切片。
- 将可复用逻辑抽成私有函数 `truncate_prefix_bytes()`，同时服务于：
  - `truncate()`(保持原先“max_len-1 + ...”的行为，但不再 panic)
  - `payload_preview`(保持原先“40 bytes + ...”的行为，但不再 panic)
- 补齐时间字段的 UTF-8 边界保护：在取前 8 字节前先回退到合法 `is_char_boundary()` 位置。
- 新增了 3 个回归测试，覆盖 payload/ts/截断边界附近的多字节字符场景。

## 相关线索(可复用实现)
### `crates/ralph-adapters/src/stream_handler.rs`
- 已存在一个“用 `char_indices()` 找 UTF-8 边界”的 `truncate` 实现，且有 emoji/箭头字符的测试用例。

## 风险评估
- 修复 `ralph-cli` 的截断逻辑属于纯展示层变更，风险低。
- 若只修 `truncate` 而不修 `payload_preview`，`ralph events` 仍可能 panic，所以两个点都要修。

## 延伸修复：`ralph-e2e` 同类风险
### `crates/ralph-e2e/src/scenarios/*`
- 多个场景文件各自定义了 `fn truncate(s: &str, max_len: usize) -> String`，旧实现同样是 `&s[..max_len]` 按字节切片。
- 这些 `truncate()` 主要用于“把 stdout/stderr/payload 缩短后拼进断言失败信息”，一旦输出包含中文/emoji 就可能在 e2e 里 panic，反而掩盖真正的失败原因。
- 我已把它们统一改为：先用 `is_char_boundary()` 回退到合法 UTF-8 边界，再切片并追加 "..."，并加了回归测试。

---

# 笔记：TUI 中文显示异常（排查中）

> 追加时间: 2026-01-24 15:50 (+0800)

## 现象（用户反馈）
- Ralph 默认启用的 TUI 在显示中文时“有问题”（具体表现待进一步明确）。

## 可能原因（按概率排序的假设列表）
### 假设A：PTY 流式读写导致 UTF-8 分片，解码方式不正确（高概率）
- PTY `read()` 返回的 byte chunk 可能在任意字节边界断开。
- UTF-8 的中文字符通常是 3 字节（emoji 常见 4 字节）。
- 如果 chunk 末尾恰好只包含“一个字符的一部分字节”，对这个 chunk 做 `from_utf8()` 会失败。

**关键代码证据**
- `crates/ralph-adapters/src/pty_executor.rs` 的 `run_observe_streaming()`：
  - 对每个 `OutputEvent::Data(data)` 都直接 `std::str::from_utf8(&data)`。
  - 只有 `Ok(text)` 才会继续解析/渲染；`Err(_)` 会导致该 chunk **完全被丢弃**（不会进入 handler，也不会进入 NDJSON 行缓冲）。

这会造成：
- 中文输出出现丢字/乱码（例如出现 `�` 或直接缺失）。
- 在 StreamJson 模式下，可能连 JSON 行解析都会受影响（因为 line_buffer 丢 chunk）。

### 假设B：宽字符显示宽度（CJK double-width / emoji）导致布局错位（中概率）
- ratatui 的布局按“终端列宽”计算。
- 如果我们在自己的代码里用 `.len()` 或 `chars().count()` 去做对齐/截断，会和真实显示宽度不一致。
- 这类问题一般表现为：边框/表格错位、内容挤压、意外换行，但字符本身通常不“乱码”。

### 假设C：ANSI/markdown 渲染链路对 Unicode 处理不当（中低概率）
- TUI 渲染链：`termimad`（markdown→ANSI）→ `ansi_to_tui`（ANSI→ratatui Text）。
- 如果 ANSI 序列与 Unicode 混排处理有 bug，也可能导致错位或丢样式。

### 假设D：终端环境/字体/locale（低概率，但需要排除）
- 字体不支持中文会显示方块。
- locale 非 UTF-8 时可能出现乱码（macOS 一般默认 UTF-8）。

## 下一步（验证路径）
- 先区分问题类型：是“乱码/丢字”（更像假设A）还是“对齐/换行错位”（更像假设B）。
- 设计一个可控的最小复现：用 `custom` backend 输出包含中文的超长行，让 PTY 更容易在字符中间分片。

## 结论（已验证并修复）
- 复现方式：使用 `custom` backend 打印一行 `a*4095 + 中 + <MARK> + b*20`，确保 `中` 落在 4096 bytes 边界上。
  - 旧实现会稳定出现 `中` 被吞掉（TUI 与 `--no-tui` 均可复现）。
- 根因确认：`run_observe_streaming()` 逐 chunk `from_utf8(&data)`，在 UTF-8 被拆分时解码失败并丢弃 chunk。
- 修复完成：引入 `Utf8StreamDecoder` 做 UTF-8 增量解码，并接入 `run_observe_streaming()`（主循环 + drain）。
- 回归测试：已添加针对“拆分多字节字符”的单测，保证以后不回退。

---

# 追加笔记：TUI 中文宽字符错位/吞首字母（已修复）

> 追加时间: 2026-01-25 01:12 (+0800)

## 新现象（用户反馈）
- 中文字符之间像被插入空格
- 中文后面的英文会缺首字母（例如 `search/notes` 变成 `earch/notes`）
- 典型输入：中文后面紧贴英文路径，例如 `新增示例合集目录：examples/.../README.md:1`（来自 `iced_emg/PROMPT.md`）

## 根因（高置信）
### `crates/ralph-tui/src/widgets/content.rs`
- `ContentPane::render()` 旧实现：
  - 逐 `chars()` 写入 cell
  - 每写一个字符都 `x += 1`

当遇到中文/CJK 或 emoji 这类“显示宽度为 2”的字符时：
- 该字符会占用两列
- 下一列是 continuation cell（终端渲染会跳过这一列）
- 旧逻辑仍会把紧随其后的 ASCII 首字母写进 continuation cell
- 实际渲染时这格会被跳过 → 就出现“吞首字母”和“看起来像插空格”的错位

## 复现证据
- 新增单测：`cjk_double_width_does_not_swallow_next_ascii_char`
  - 渲染 `"将search/notes"` 时，旧实现会让 `buf[(1,0)] == "s"`（把 s 写进 continuation cell），从而复现问题。

## 修复方案（已实现）
- 改为按 grapheme cluster（`unicode-segmentation`）迭代，而不是 `chars()`。
- 用 `unicode-width` 计算每个 grapheme 的显示宽度（列宽）。
- 写入时遵循 ratatui `Buffer::set_stringn` 的策略：
  - 宽 grapheme 写入首格
  - 后续被遮挡的格子 reset
  - 光标按显示宽度推进，避免写进 continuation cell
- 软换行时先清理当前行剩余格子，避免残影（artifact）

## 验证
- `cargo test -p ralph-tui`、`cargo clippy -p ralph-tui`、`cargo test` 全通过
