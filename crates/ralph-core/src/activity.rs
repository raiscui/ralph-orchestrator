//! 活动状态文本的轻量判定与归一化。
//!
//! 说明：
//! - 这里不试图解析任何私有 TTY 控制序列。
//! - 只针对“已经是可见文本”的状态行做 best-effort 归一化。
//! - 目标是让 TUI 能显示类似 `Working` / `Inspecting ...` 这类 activity。

const STATUS_PREFIXES: [&str; 17] = [
    "working",
    "inspecting",
    "thinking",
    "reviewing",
    "analyzing",
    "searching",
    "checking",
    "reading",
    "writing",
    "implementing",
    "planning",
    "preparing",
    "running",
    "building",
    "debugging",
    "investigating",
    "exploring",
];

/// 判断一段可见文本是否像“活动状态”。
///
/// 这类文本通常是：
/// - `Working...`
/// - `Inspecting current code behavior`
/// - `• Working (11s • esc to interrupt)`
pub fn normalize_activity_label(raw: &str) -> Option<String> {
    let Some(first_line) = raw.lines().map(str::trim).find(|line| !line.is_empty()) else {
        return None;
    };

    let cleaned = clean_activity_label(first_line);
    if cleaned.is_empty() {
        return None;
    }

    if looks_like_activity(&cleaned) {
        Some(cleaned.to_string())
    } else {
        None
    }
}

/// 归一化已知的 activity 文本。
///
/// 规则：
/// - 去掉前导 bullet / 空白。
/// - 去掉常见的 `(... • ... to interrupt)` 后缀。
/// - 去掉末尾省略号。
pub fn clean_activity_label(raw: &str) -> String {
    let mut text = raw.trim();
    text =
        text.trim_start_matches(|c: char| c.is_whitespace() || matches!(c, '•' | '*' | '-' | '>'));
    text = text.trim();

    if let Some((prefix, suffix)) = text.rsplit_once(" (")
        && suffix.ends_with(')')
        && suffix.contains("to interrupt")
    {
        text = prefix.trim_end();
    }

    let text = text.trim_end_matches('.');
    text.trim().to_string()
}

fn looks_like_activity(text: &str) -> bool {
    let lower = text.trim_start().to_ascii_lowercase();
    STATUS_PREFIXES
        .iter()
        .any(|prefix| lower.starts_with(prefix))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_codex_style_status_line() {
        let label = normalize_activity_label("• Working (11s • esc to interrupt)")
            .expect("status line should be recognized");

        assert_eq!(label, "Working");
    }

    #[test]
    fn normalizes_reasoning_activity_text() {
        let label = normalize_activity_label("Inspecting current code behavior")
            .expect("reasoning activity should be recognized");

        assert_eq!(label, "Inspecting current code behavior");
    }

    #[test]
    fn ignores_non_status_text() {
        assert!(normalize_activity_label("[codex-app-server] turn input").is_none());
        assert!(normalize_activity_label("hello world").is_none());
    }
}
