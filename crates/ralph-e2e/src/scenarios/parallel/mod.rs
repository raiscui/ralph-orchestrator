//! Tier 8: Parallel Runtime (experimental) test scenarios.
//!
//! 说明：
//! - 这些场景用于验证 **parallel hat instances** 在“真实后端”上的端到端行为。
//! - 与 replay smoke tests 的差异：
//!   - E2E 会覆盖真实 CLI、真实认证、真实网络与真实模型漂移带来的风险
//!   - 代价更高、速度更慢，因此场景应尽量“短、稳、可排障”

mod app_server_idle_start;
mod app_server_idle_start_live;
mod app_server_steer_live_reply_multi_turn;
mod app_server_steer_multi_turn;
mod app_server_steer_multi_turn_live;
mod emit_spawn_instance;
mod hat_instances;
mod job_run_counts;
mod starting_event_inference;

pub use app_server_idle_start::ParallelAppServerIdleStartScenario;
pub use app_server_idle_start_live::ParallelAppServerIdleStartLiveScenario;
pub use app_server_steer_live_reply_multi_turn::ParallelAppServerSteerLiveReplyMultiTurnScenario;
pub use app_server_steer_multi_turn::ParallelAppServerSteerMultiTurnScenario;
pub use app_server_steer_multi_turn_live::ParallelAppServerSteerMultiTurnLiveScenario;
pub use emit_spawn_instance::ParallelEmitSpawnInstanceScenario;
pub use hat_instances::ParallelHatInstancesScenario;
pub use starting_event_inference::ParallelStartingEventInferenceScenario;

use crate::Backend;
use crate::executor::{PromptSource, ScenarioConfig};
use crate::scenarios::ScenarioError;
use ralph_core::{AgentsSnapshot, EventParser};
use std::path::Path;
use std::time::Duration;

// 说明：
// - 这些 helper 目前会被 `parallel_trigger_routing_example` 复用。
// - 可见性限制在 `crate::scenarios`，避免扩散到整个 crate。
pub(in crate::scenarios) use job_run_counts::{JobRunCounts, parse_parallel_job_line};

/// 读取并解析并行运行态的 agent 快照：`.ralph/agents.json`。
///
/// 说明：
/// - 该文件由并行 Supervisor 维护,用于 `ralph agents` 命令做可观测性展示。
/// - E2E 里读取它的目的:
///   - 验证“最近新增的可观测性能力”没有回归(至少能落盘且 JSON 可解析)。
pub(in crate::scenarios) fn read_agents_snapshot(
    workspace: &Path,
) -> Result<AgentsSnapshot, String> {
    let path = workspace.join(".ralph/agents.json");
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("failed to read {}: {e}", path.display()))?;

    serde_json::from_str::<AgentsSnapshot>(&content)
        .map_err(|e| format!("invalid agents snapshot JSON {}: {e}", path.display()))
}

/// 从并行 stdout 的 `:out:job=` 行中提取最后一个指定 topic 的完整 payload。
///
/// 说明:
/// - 并行模式会给每一行 stdout 加上 `[hat#n:out:job=m] ` 前缀。
/// - 直接把整份 stdout 丢给 `EventParser`，可能拿不到纯净 event。
/// - 这里先剥掉所有 out 行前缀，再复用共享 parser。
/// - 若 out 行中没有命中，再回退到整份 stdout，兼容少数非标准输出形态。
pub(in crate::scenarios) fn extract_last_parallel_out_payload_for_topic(
    stdout: &str,
    topic: &str,
) -> Option<String> {
    let mut normalized = String::new();

    for line in stdout.lines() {
        if line.contains(":out:job=")
            && let Some((_prefix, payload)) = line.split_once("] ")
        {
            normalized.push_str(payload);
            normalized.push('\n');
        }
    }

    EventParser::extract_last_payload_for_topic(&normalized, topic)
        .or_else(|| EventParser::extract_last_payload_for_topic(stdout, topic))
}

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

pub(in crate::scenarios) fn patch_example_config_for_codex_e2e(
    config_content: &str,
    backend: Backend,
) -> Result<String, String> {
    // ---------------------------------------------------------------------
    // 说明:
    // - 这批 example 场景都只是验证并行协议与 topic 收敛。
    // - 它们不需要高强度推理,也不依赖 MCP/tool runtime。
    // - 因此在 E2E workspace 里统一把 Codex CLI 覆写成更安静、更快的参数组合。
    // ---------------------------------------------------------------------
    if backend != Backend::Codex {
        return Ok(config_content.to_string());
    }

    let cli_block = r#"cli:
  # E2E: 覆写 Codex 参数,降噪/提速(不影响仓库 example 原文件).
  backend: custom
  command: codex
  args:
    - exec
    - -m
    - gpt-5-codex
    - --full-auto
    - -c
    - 'model_reasoning_effort="low"'
    - -c
    - 'model_reasoning_summary="none"'
    - -c
    - 'rmcp_client=false'

"#;

    replace_top_level_yaml_block(config_content, "cli:", cli_block)
}

/// 为带 `prompt_file: "PROMPT.md"` 的 direct example scenario 准备 E2E workspace。
///
/// 说明:
/// - 这批真实 example 都强调“目录自包含”。
/// - 但当前 `prompt_file` 仍相对 workspace root 解析。
/// - 因此 E2E 既要把 example 原目录拷进 workspace,也要在 workspace 根补一份 `PROMPT.md`。
pub(in crate::scenarios) fn setup_prompt_file_example_workspace(
    workspace: &Path,
    backend: Backend,
    example_name: &str,
    max_iterations: u32,
) -> Result<ScenarioConfig, ScenarioError> {
    let root = crate::executor::find_workspace_root().ok_or_else(|| {
        ScenarioError::SetupError("failed to find workspace root (Cargo.toml)".to_string())
    })?;

    std::fs::create_dir_all(workspace.join(".agent")).map_err(|error| {
        ScenarioError::SetupError(format!("failed to create .agent directory: {error}"))
    })?;

    let example_dir = root.join(format!("examples/{example_name}"));
    let config_path = example_dir.join("ralph.yml");
    let prompt_path = example_dir.join("PROMPT.md");

    let config_content = std::fs::read_to_string(&config_path).map_err(|error| {
        ScenarioError::SetupError(format!(
            "failed to read example config {}: {error}",
            config_path.display()
        ))
    })?;
    let prompt_content = std::fs::read_to_string(&prompt_path).map_err(|error| {
        ScenarioError::SetupError(format!(
            "failed to read example prompt {}: {error}",
            prompt_path.display()
        ))
    })?;

    let dest_dir = workspace.join(format!("examples/{example_name}"));
    std::fs::create_dir_all(&dest_dir).map_err(|error| {
        ScenarioError::SetupError(format!(
            "failed to create workspace example dir {}: {error}",
            dest_dir.display()
        ))
    })?;

    let patched = patch_example_config_for_codex_e2e(&config_content, backend)
        .map_err(ScenarioError::SetupError)?;
    std::fs::write(dest_dir.join("ralph.yml"), patched).map_err(|error| {
        ScenarioError::SetupError(format!("failed to write workspace ralph.yml: {error}"))
    })?;
    std::fs::write(dest_dir.join("PROMPT.md"), prompt_content.as_str()).map_err(|error| {
        ScenarioError::SetupError(format!(
            "failed to write workspace example PROMPT.md: {error}"
        ))
    })?;
    std::fs::write(workspace.join("PROMPT.md"), prompt_content.as_str()).map_err(|error| {
        ScenarioError::SetupError(format!("failed to write workspace root PROMPT.md: {error}"))
    })?;

    Ok(ScenarioConfig {
        config_file: format!("examples/{example_name}/ralph.yml").into(),
        prompt: PromptSource::Config,
        max_iterations,
        timeout: std::cmp::min(backend.default_timeout(), Duration::from_secs(300)),
        extra_args: vec!["--no-tui".to_string()],
    })
}
