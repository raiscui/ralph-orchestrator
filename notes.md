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
