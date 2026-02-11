//! Prompt overlay helpers.
//!
//! 说明:
//! - 用于加载并注入“所有 hat 通用补充提示”.
//! - 当前约定来源文件: `config/all_hat.md`.

use std::path::Path;

const ALL_HAT_PROMPT_RELATIVE_PATH: &str = "config/all_hat.md";

/// 从工作区加载所有 hat 通用补充提示.
///
/// 规则:
/// - 文件不存在: 返回 `None`(不注入)
/// - 文件存在但为空白: 返回 `None`(不注入)
/// - 其余: 返回去掉首尾空白后的正文
pub(crate) fn load_all_hat_prompt(workspace_root: &Path) -> Option<String> {
    let path = workspace_root.join(ALL_HAT_PROMPT_RELATIVE_PATH);
    let content = std::fs::read_to_string(path).ok()?;
    let trimmed = content.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// 把 all-hat 补充提示注入到 prompt 顶部.
pub(crate) fn inject_all_hat_prompt(prompt: String, all_hat_prompt: Option<&str>) -> String {
    let Some(extra) = all_hat_prompt.map(str::trim).filter(|s| !s.is_empty()) else {
        return prompt;
    };

    format!("## ALL HAT PROMPT (config/all_hat.md)\n\n{extra}\n\n{prompt}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inject_all_hat_prompt_skips_none_or_blank() {
        let prompt = "base prompt".to_string();
        assert_eq!(inject_all_hat_prompt(prompt.clone(), None), prompt);
        assert_eq!(
            inject_all_hat_prompt(prompt.clone(), Some("   \n\t")),
            prompt
        );
    }

    #[test]
    fn inject_all_hat_prompt_adds_overlay_header() {
        let prompt = "base prompt".to_string();
        let merged = inject_all_hat_prompt(prompt, Some("shared guidance"));
        assert!(merged.contains("## ALL HAT PROMPT (config/all_hat.md)"));
        assert!(merged.contains("shared guidance"));
        assert!(merged.contains("base prompt"));
    }
}
