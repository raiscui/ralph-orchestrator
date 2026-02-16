//! Prompt overlay helpers.
//!
//! 说明:
//! - 用于注入“所有 hat 通用补充提示”.
//! - 来源文件 `config/all_hat.md` 在编译期静态内嵌.

/// 统一注入段落标题，便于测试和日志定位。
const ALL_HAT_PROMPT_HEADER: &str = "## ALL HAT PROMPT (config/all_hat.md)";

/// `config/all_hat.md` 的编译期内嵌内容。
///
/// 路径说明:
/// - `CARGO_MANIFEST_DIR` 指向 `crates/ralph-core`
/// - `../../config/all_hat.md` 回到仓库根目录下的配置文件
const COMPILED_ALL_HAT_PROMPT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../config/all_hat.md"
));

/// 加载所有 hat 通用补充提示（编译期内嵌版本）。
///
/// 规则:
/// - 内嵌内容为空白: 返回 `None`(不注入)
/// - 其余: 返回去掉首尾空白后的正文
pub(crate) fn load_all_hat_prompt() -> Option<String> {
    let trimmed = COMPILED_ALL_HAT_PROMPT.trim();
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

    // ---------------------------------------------------------------------
    // 重要: `ralph_hat_instance_id` 必须置顶
    //
    // 背景:
    // - 我们会在每个 hat prompt 的顶部注入一行:
    //   `ralph_hat_instance_id:"<hat_or_instance_id>"`
    // - 同时又会把 `config/all_hat.md` 作为 overlay "整体前置" 到 prompt 顶部.
    //
    // 问题:
    // - 如果 overlay 里包含 `<event ...>` 的示例块,
    //   那么 `ralph_hat_instance_id` 出现在示例块之后时,
    //   模型更容易把它误解为"示例的一部分",造成行为漂移.
    //
    // 方案:
    // - 当 prompt 已经以 `ralph_hat_instance_id:"..."` 开头时,
    //   把 overlay 插入到该行之后,确保 `ralph_hat_instance_id` 永远是第一行.
    // - 其余情况(没有注入该行): 仍保持 overlay 置顶.
    // ---------------------------------------------------------------------
    const RUNTIME_ID_PREFIX: &str = "ralph_hat_instance_id:\"";

    if prompt.starts_with(RUNTIME_ID_PREFIX) {
        // 取第一行(到 '\n' 之前)作为 runtime id 行.
        let line_end = prompt.find('\n').unwrap_or(prompt.len());
        let first_line = &prompt[..line_end];

        // 额外校验: 只在形如 `ralph_hat_instance_id:"..."`
        // 的情况下才做"插入到第一行之后"的重排,避免误匹配.
        if first_line.starts_with(RUNTIME_ID_PREFIX) && first_line.ends_with('"') {
            // 剩余正文: 去掉第一行以及其后所有空行(主要是 `\n\n`),
            // 由我们统一用 `\n\n` 重新分隔,避免出现多余空白.
            let rest = prompt[line_end..].trim_start_matches(|c| c == '\r' || c == '\n');
            return format!("{first_line}\n\n{ALL_HAT_PROMPT_HEADER}\n\n{extra}\n\n{rest}");
        }
    }

    format!("{ALL_HAT_PROMPT_HEADER}\n\n{extra}\n\n{prompt}")
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
        assert!(merged.contains(ALL_HAT_PROMPT_HEADER));
        assert!(merged.contains("shared guidance"));
        assert!(merged.contains("base prompt"));
    }

    #[test]
    fn inject_all_hat_prompt_keeps_runtime_id_as_first_line() {
        // 该测试锁死一个关键约束:
        // - `ralph_hat_instance_id:"..."` 永远必须是 prompt 第一行.
        //
        // 否则当 overlay 里有 `<event ...>` 示例时,
        // 模型容易把紧随其后的 runtime id 误当成示例续行.
        let prompt = "ralph_hat_instance_id:\"writer#1\"\n\nbase prompt".to_string();
        let merged = inject_all_hat_prompt(prompt, Some("shared guidance"));

        assert!(
            merged.starts_with("ralph_hat_instance_id:\"writer#1\"\n\n"),
            "merged prompt should start with runtime id line"
        );
        assert!(
            merged.contains(ALL_HAT_PROMPT_HEADER),
            "merged prompt should still include overlay header"
        );
        let id_pos = merged
            .find("ralph_hat_instance_id:\"writer#1\"")
            .expect("merged prompt must contain runtime id");
        let header_pos = merged
            .find(ALL_HAT_PROMPT_HEADER)
            .expect("merged prompt must contain overlay header");
        assert!(
            id_pos < header_pos,
            "runtime id must appear before overlay header"
        );
    }

    #[test]
    fn load_all_hat_prompt_reads_compiled_overlay() {
        let overlay = load_all_hat_prompt().expect("compiled all-hat overlay should not be empty");
        assert!(
            overlay.contains("文件上下文位置特殊情况转移"),
            "compiled overlay should contain content from config/all_hat.md"
        );
    }
}
