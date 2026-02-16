//! Tier 8: Parallel Runtime (experimental) test scenarios.
//!
//! 说明：
//! - 这些场景用于验证 **parallel hat instances** 在“真实后端”上的端到端行为。
//! - 与 replay smoke tests 的差异：
//!   - E2E 会覆盖真实 CLI、真实认证、真实网络与真实模型漂移带来的风险
//!   - 代价更高、速度更慢，因此场景应尽量“短、稳、可排障”

mod hat_instances;
mod job_run_counts;
mod starting_event_inference;

pub use hat_instances::ParallelHatInstancesScenario;
pub use starting_event_inference::ParallelStartingEventInferenceScenario;

// 说明：
// - 这些 helper 目前会被 `parallel_trigger_routing_example` 复用。
// - 可见性限制在 `crate::scenarios`，避免扩散到整个 crate。
pub(in crate::scenarios) use job_run_counts::{JobRunCounts, parse_parallel_job_line};

pub(in crate::scenarios) fn replace_top_level_yaml_block(
    content: &str,
    block_key_line: &str,
    replacement_block: &str,
) -> Result<String, String> {
    // ---------------------------------------------------------------------
    // 说明：
    // - 我们只替换顶层某个 block(例如 `cli:`)，避免重新序列化 YAML 导致 `|` 字面量块格式变化。
    // - 这在并行 E2E 里很重要：prompt 文本要尽量保持 example 原样，减少模型漂移风险。
    // ---------------------------------------------------------------------
    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
    let start = lines
        .iter()
        .position(|l| l.trim_end() == block_key_line)
        .ok_or_else(|| format!("missing top-level block key: {block_key_line}"))?;

    // 寻找下一个顶层 key(不缩进,以 ":" 结尾,且不是注释/空行)作为 block 结束。
    let mut end = lines.len();
    for (i, line) in lines.iter().enumerate().skip(start + 1) {
        let line = line.as_str();
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        if !line.starts_with(' ') && !line.starts_with('\t') && line.trim_end().ends_with(':') {
            end = i;
            break;
        }
    }

    let replacement_lines = replacement_block
        .lines()
        .map(|l| l.to_string())
        .collect::<Vec<_>>();
    lines.splice(start..end, replacement_lines);

    let mut merged = lines.join("\n");
    merged.push('\n');
    Ok(merged)
}
