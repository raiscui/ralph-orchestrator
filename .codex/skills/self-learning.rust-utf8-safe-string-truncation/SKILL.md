---
name: self-learning.rust-utf8-safe-string-truncation
description: |
  Use when Rust panics with "byte index ... is not a char boundary", or when code truncates UTF-8 strings by a token/character budget.
  Solves the bug pattern where a char budget is reused as a byte index, especially with Chinese, emoji, or other multi-byte text.
  Covers safe use of char_indices/is_char_boundary, shared helper placement, and regression tests with non-ASCII input.
author: Codex
version: 1.0.0
date: 2026-04-30
---

# Rust UTF-8 Safe String Truncation

## 问题

Rust 的 `str` 是 UTF-8。字符串切片边界必须落在合法字符边界上。

如果代码把“保留 N 个字符”误写成 `&s[..n]`,当文本里有中文或 emoji 时,`n` 很可能落在某个字符的中间字节。结果就是运行时 panic:

```text
byte index ... is not a char boundary
```

## 上下文 / 触发条件

- panic 文本包含 `byte index ... is not a char boundary`。
- 报错字符是中文、emoji 或其他非 ASCII 字符。
- 代码里出现 token budget、char budget、preview length、tail/head truncation 之类逻辑。
- 实现中直接用了 `String::len()` 或预算数值当切片边界。

## 解决方案

1. 先确认语义:
   - 如果预算是 byte budget,可以使用 byte 长度,但仍要在切片前校正到字符边界。
   - 如果预算是 char budget,不要碰裸 byte index。

2. 把“字符数 -> 安全 byte index”收口成一个 helper:

```rust
pub fn byte_index_after_chars(content: &str, max_chars: usize) -> usize {
    content
        .char_indices()
        .nth(max_chars)
        .map(|(index, _)| index)
        .unwrap_or(content.len())
}
```

3. head truncation 使用 helper:

```rust
let end = byte_index_after_chars(content, max_chars);
let preview = &content[..end];
```

4. tail truncation 不要用 `content.len() - n` 直接倒推。
   先按字符数找到保留尾部的起点,再切片。

5. 如果必须从 byte index 开始,先用 `is_char_boundary` 校验。
   校验失败时,向前或向后走到最近合法边界。

6. 把 helper 放在唯一共享文本工具模块里。
   不要在 memory、scratchpad、TUI、日志预览里各写一份类似逻辑。

## 验证

- 单测必须包含中文或 emoji,不能只用 ASCII。
- 至少覆盖:
  - helper 返回合法 byte index。
  - head truncation 不 panic。
  - tail truncation 不 panic。
  - 截断后内容仍是合法 UTF-8。

示例断言:

```rust
#[test]
fn truncation_uses_utf8_boundaries() {
    let content = "设置设置设置";
    let index = byte_index_after_chars(content, 3);

    assert!(content.is_char_boundary(index));
    assert_eq!(&content[..index], "设置设");
}
```

## 示例

Ralph 曾在 `memory_store` 中把 token budget 乘以 4 得到“字符预算”,但随后直接把这个预算当 byte index 切片。

中文上下文触发了:

```text
byte index 1200 is not a char boundary
```

修复方式是:

- 在共享 text helper 中提供安全 byte index 转换。
- `memory_store` 的 head budget 截断复用 helper。
- scratchpad tail budget 截断也复用 helper。
- 用中文内容补回归测试。

## 备注

- `String::len()` 返回 byte 数,不是字符数。
- `.chars().count()` 返回 Unicode scalar value 数,不等于用户眼里的字素簇数量。
- 对预览、预算、日志裁剪这类场景,通常 scalar value 已经足够; 如果要按用户可见字符处理组合字符,需要额外考虑 grapheme segmentation。
- Rust 的字符串 panic 是好事: 它阻止了无效 UTF-8 悄悄进入系统。

## 参考资料

- Rust Book, Storing UTF-8 Encoded Text with Strings: https://doc.rust-lang.org/book/ch08-02-strings.html
- Rust `str::is_char_boundary`: https://doc.rust-lang.org/std/primitive.str.html#method.is_char_boundary
- Rust `str::char_indices`: https://doc.rust-lang.org/std/primitive.str.html#method.char_indices
