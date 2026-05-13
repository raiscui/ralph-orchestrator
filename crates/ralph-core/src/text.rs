//! Text utilities for the Ralph Orchestrator.
//!
//! This module provides common text manipulation functions used throughout
//! the codebase, including UTF-8 safe string truncation.

/// Truncates a string to a maximum number of characters, adding "..." if truncated.
///
/// This function is UTF-8 safe: it uses character boundaries, not byte boundaries,
/// so it will never split a multi-byte character (emoji, non-ASCII, etc.).
///
/// # Arguments
///
/// * `s` - The string to truncate
/// * `max_chars` - Maximum number of characters (not bytes) before truncation
///
/// # Returns
///
/// - The original string if its character count is <= `max_chars`
/// - A truncated string with "..." appended if longer
///
/// # Examples
///
/// ```
/// use ralph_core::truncate_with_ellipsis;
///
/// // Short strings pass through unchanged
/// assert_eq!(truncate_with_ellipsis("hello", 10), "hello");
///
/// // Long strings are truncated with ellipsis
/// assert_eq!(truncate_with_ellipsis("hello world", 5), "hello...");
///
/// // UTF-8 safe: emojis are not split
/// assert_eq!(truncate_with_ellipsis("🎉🎊🎁🎄", 2), "🎉🎊...");
/// ```
pub fn truncate_with_ellipsis(s: &str, max_chars: usize) -> String {
    if let Some(byte_idx) = byte_index_after_chars(s, max_chars) {
        format!("{}...", &s[..byte_idx])
    } else {
        s.to_string()
    }
}

/// 返回保留指定字符数之后的字节边界。
///
/// Rust 字符串只能在 UTF-8 字符边界切片。所有“字符预算”截断都应该先通过
/// 这个 helper 把字符数量转换为安全的 byte index,避免中文和 emoji 被切开。
pub(crate) fn byte_index_after_chars(s: &str, max_chars: usize) -> Option<usize> {
    s.char_indices().nth(max_chars).map(|(idx, _)| idx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_string_unchanged() {
        assert_eq!(truncate_with_ellipsis("short", 10), "short");
        assert_eq!(truncate_with_ellipsis("", 5), "");
        assert_eq!(truncate_with_ellipsis("exact", 5), "exact");
    }

    #[test]
    fn test_long_string_truncated() {
        assert_eq!(
            truncate_with_ellipsis("this is a long string", 10),
            "this is a ..."
        );
        assert_eq!(truncate_with_ellipsis("abcdef", 3), "abc...");
    }

    #[test]
    fn test_utf8_boundaries_arrows() {
        // Arrow characters are 3 bytes each in UTF-8
        let arrows = "→→→→→→→→";
        assert_eq!(truncate_with_ellipsis(arrows, 5), "→→→→→...");
    }

    #[test]
    fn test_utf8_boundaries_mixed() {
        let mixed = "a→b→c→d";
        assert_eq!(truncate_with_ellipsis(mixed, 5), "a→b→c...");
    }

    #[test]
    fn test_utf8_boundaries_emoji() {
        // Emojis are 4 bytes each in UTF-8
        let emoji = "🎉🎊🎁🎄";
        assert_eq!(truncate_with_ellipsis(emoji, 3), "🎉🎊🎁...");
    }

    #[test]
    fn test_utf8_complex_emoji() {
        // Rust crab emoji
        let s = "hi 🦀 there";
        // "hi 🦀" = 4 characters (h, i, space, 🦀)
        assert_eq!(truncate_with_ellipsis(s, 4), "hi 🦀...");
    }

    #[test]
    fn test_zero_max_chars() {
        assert_eq!(truncate_with_ellipsis("hello", 0), "...");
    }

    #[test]
    fn test_single_char_truncation() {
        assert_eq!(truncate_with_ellipsis("hello", 1), "h...");
        assert_eq!(truncate_with_ellipsis("🎉hello", 1), "🎉...");
    }

    #[test]
    fn test_byte_index_after_chars_uses_utf8_boundaries() {
        assert_eq!(byte_index_after_chars("设置", 0), Some(0));
        assert_eq!(byte_index_after_chars("设置", 1), Some("设".len()));
        assert_eq!(byte_index_after_chars("设置", 2), None);
    }
}
