//! Runtime capability tools。
//!
//! 这是 `ralph#1` 可调用的 agent-facing surface。
//! v1 只做规则选择 + 隔离 child/micro-run,不会热改当前 live topology。

use anyhow::{Context, Result, anyhow, bail};
use chrono::Utc;
use clap::{Parser, Subcommand, ValueEnum};
use ralph_core::{
    CapabilityChoice, CapabilityFailedRecord, CapabilityInvocationMode, CapabilityInvocationRecord,
    CapabilityKind, CapabilityMetadata, CapabilityResultRecord, EventLogger, RalphConfig,
    TOPIC_CAPABILITY_FAILED, TOPIC_CAPABILITY_INVOKE, TOPIC_CAPABILITY_RESULT,
};
use ralph_proto::Event;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::startup_resources::{self, ResourceKind};

const CHOOSER_VERSION_V1: &str = "rules-v1";
const INVOCATION_ROOT: &str = ".ralph/capability-invocations";

/// `ralph tools capability` 参数。
#[derive(Parser, Debug)]
pub struct CapabilityArgs {
    #[command(subcommand)]
    pub command: CapabilityCommands,
}

/// Runtime capability tool 子命令。
#[derive(Subcommand, Debug)]
pub enum CapabilityCommands {
    /// 列出 capability metadata。
    List(CapabilityListArgs),
    /// 输出注入给 `ralph#1` 的轻量 capability 摘要。
    Summaries(CapabilitySummaryArgs),
    /// 调用一个 capability,生成隔离 child/micro-run artifact。
    Invoke(CapabilityInvokeArgs),
}

/// Capability 列表参数。
#[derive(Parser, Debug)]
pub struct CapabilityListArgs {
    /// 只显示某类 capability。
    #[arg(long, value_enum)]
    pub kind: Option<CapabilityKindArg>,

    /// 输出 JSON。
    #[arg(long)]
    pub json: bool,
}

/// Capability 摘要参数。
#[derive(Parser, Debug)]
pub struct CapabilitySummaryArgs {
    /// 输出 JSON。
    #[arg(long)]
    pub json: bool,
}

/// Capability 调用参数。
#[derive(Parser, Debug)]
pub struct CapabilityInvokeArgs {
    /// Capability id。省略时使用 v1 规则 chooser。
    #[arg(long)]
    pub id: Option<String>,

    /// 调用输入。
    #[arg(long, default_value = "")]
    pub input: String,

    /// 只选择 workflow 或 hat capability。
    #[arg(long, value_enum)]
    pub kind: Option<CapabilityKindArg>,

    /// 工作区根目录,默认当前目录。
    #[arg(long)]
    pub workspace: Option<PathBuf>,

    /// 输出 JSON。
    #[arg(long)]
    pub json: bool,
}

/// CLI 用 capability kind。
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "snake_case")]
pub enum CapabilityKindArg {
    Workflow,
    Hat,
}

impl CapabilityKindArg {
    fn matches(self, kind: CapabilityKind) -> bool {
        matches!(
            (self, kind),
            (Self::Workflow, CapabilityKind::WorkflowCapability)
                | (Self::Hat, CapabilityKind::HatCapability)
        )
    }
}

/// 执行 capability tool。
pub fn execute(args: CapabilityArgs, _use_colors: bool) -> Result<()> {
    match args.command {
        CapabilityCommands::List(args) => list_capabilities(args),
        CapabilityCommands::Summaries(args) => print_capability_summaries(args),
        CapabilityCommands::Invoke(args) => invoke_capability(args),
    }
}

/// 从 startup resource catalog 暴露 lightweight capability metadata。
///
/// 说明:
/// - workflow preset 暴露为 workflow capability。
/// - v1 另外提供一个内嵌 hat capability,用于验证 micro-run 模型。
/// - 不读取 YAML 注释,也不把完整 instructions 注入摘要。
pub(crate) fn capability_catalog() -> Vec<CapabilityMetadata> {
    let mut capabilities = startup_resources::embedded_catalog()
        .iter()
        .filter(|resource| resource.kind == ResourceKind::WorkflowPreset)
        .map(|resource| CapabilityMetadata {
            id: resource.id.to_string(),
            kind: CapabilityKind::WorkflowCapability,
            summary: resource.summary.to_string(),
            goal: resource.goal.to_string(),
            when_to_use: if resource.selector_eligible {
                "Use when the user asks for workflow-level execution and no more specific capability was selected."
            } else {
                "Use only when explicitly requested for baseline/control workflow behavior."
            }
            .to_string(),
            input_contract: "Natural-language task input. The child run receives it as an inline prompt."
                .to_string(),
            output_contract:
                "Structured result summary with stdout/stderr evidence and child run artifacts."
                    .to_string(),
            invocation_mode: CapabilityInvocationMode::IsolatedChildRun,
        })
        .collect::<Vec<_>>();

    capabilities.push(CapabilityMetadata {
        id: "hat:focused-reviewer".to_string(),
        kind: CapabilityKind::HatCapability,
        summary: "Focused reviewer micro-run".to_string(),
        goal: "Review a bounded input and return concise findings without joining the parent topology."
            .to_string(),
        when_to_use: "Use when ralph#1 needs a one-off review lens rather than a full workflow."
            .to_string(),
        input_contract: "A bounded text or task description to review.".to_string(),
        output_contract: "A short review summary suitable for parent-run consumption.".to_string(),
        invocation_mode: CapabilityInvocationMode::IsolatedMicroRun,
    });

    capabilities
}

fn list_capabilities(args: CapabilityListArgs) -> Result<()> {
    let capabilities = filtered_capabilities(args.kind);
    if args.json {
        println!("{}", serde_json::to_string_pretty(&capabilities)?);
        return Ok(());
    }

    for capability in capabilities {
        println!(
            "{}\t{}\t{}\t{}",
            capability.id, capability.kind, capability.invocation_mode, capability.summary
        );
    }
    Ok(())
}

fn print_capability_summaries(args: CapabilitySummaryArgs) -> Result<()> {
    let summaries = capability_summaries();
    if args.json {
        println!("{}", serde_json::to_string_pretty(&summaries)?);
        return Ok(());
    }

    println!("Runtime capabilities available to ralph#1:");
    for summary in summaries {
        println!("- {} [{}]: {}", summary.id, summary.kind, summary.summary);
        println!("  When: {}", summary.when_to_use);
        println!("  Input: {}", summary.input_contract);
        println!("  Output: {}", summary.output_contract);
    }
    Ok(())
}

fn invoke_capability(args: CapabilityInvokeArgs) -> Result<()> {
    let workspace = args
        .workspace
        .unwrap_or(std::env::current_dir().context("Failed to resolve current directory")?);
    let catalog = filtered_capabilities(args.kind);
    let choice = choose_capability(&catalog, args.id.as_deref(), &args.input)?;
    let capability = catalog
        .iter()
        .find(|candidate| candidate.id == choice.capability_id)
        .cloned()
        .ok_or_else(|| anyhow!("Selected capability disappeared: {}", choice.capability_id))?;

    let report = invoke_isolated(&workspace, capability, choice, &args.input)?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("Invocation: {}", report.invocation.invocation_id);
        println!("Capability: {}", report.invocation.capability.id);
        println!("Mode: {}", report.invocation.capability.invocation_mode);
        println!(
            "Resolved config: {}",
            report.invocation.resolved_config_path
        );
        println!("Result: {}", report.result.result_summary);
    }
    Ok(())
}

fn filtered_capabilities(kind: Option<CapabilityKindArg>) -> Vec<CapabilityMetadata> {
    capability_catalog()
        .into_iter()
        .filter(|capability| kind.is_none_or(|kind| kind.matches(capability.kind)))
        .collect()
}

#[derive(Debug, Clone, Serialize)]
struct CapabilitySummaryView {
    id: String,
    kind: CapabilityKind,
    summary: String,
    goal: String,
    when_to_use: String,
    input_contract: String,
    output_contract: String,
    invocation_mode: CapabilityInvocationMode,
}

fn capability_summaries() -> Vec<CapabilitySummaryView> {
    capability_catalog()
        .into_iter()
        .map(|capability| CapabilitySummaryView {
            id: capability.id,
            kind: capability.kind,
            summary: capability.summary,
            goal: capability.goal,
            when_to_use: capability.when_to_use,
            input_contract: capability.input_contract,
            output_contract: capability.output_contract,
            invocation_mode: capability.invocation_mode,
        })
        .collect()
}

fn choose_capability(
    catalog: &[CapabilityMetadata],
    explicit_id: Option<&str>,
    input: &str,
) -> Result<CapabilityChoice> {
    if let Some(id) = explicit_id {
        if catalog.iter().any(|candidate| candidate.id == id) {
            return Ok(CapabilityChoice {
                capability_id: id.to_string(),
                reason: "explicit capability id".to_string(),
                chooser_version: CHOOSER_VERSION_V1.to_string(),
            });
        }
        bail!("Unknown capability id: {id}");
    }

    let input_lower = input.to_ascii_lowercase();
    let preferred_kind = if input_lower.contains("review") || input_lower.contains("audit") {
        CapabilityKind::HatCapability
    } else {
        CapabilityKind::WorkflowCapability
    };

    let capability = catalog
        .iter()
        .find(|candidate| candidate.kind == preferred_kind)
        .or_else(|| catalog.first())
        .context("No runtime capabilities are available")?;

    Ok(CapabilityChoice {
        capability_id: capability.id.clone(),
        reason: format!(
            "rules-v1 selected {} from input keywords and catalog order",
            capability.kind
        ),
        chooser_version: CHOOSER_VERSION_V1.to_string(),
    })
}

#[derive(Debug, Clone, Serialize)]
struct CapabilityInvokeReport {
    invocation: CapabilityInvocationRecord,
    result: CapabilityResultRecord,
}

#[derive(Debug, Clone)]
struct ChildRunOutput {
    success: bool,
    exit_code: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl From<std::process::Output> for ChildRunOutput {
    fn from(output: std::process::Output) -> Self {
        Self {
            success: output.status.success(),
            exit_code: output.status.code(),
            stdout: output.stdout,
            stderr: output.stderr,
        }
    }
}

fn invoke_isolated(
    workspace: &Path,
    capability: CapabilityMetadata,
    choice: CapabilityChoice,
    input: &str,
) -> Result<CapabilityInvokeReport> {
    invoke_isolated_with_runner(workspace, capability, choice, input, run_child_dry_run)
}

fn invoke_isolated_with_runner(
    workspace: &Path,
    capability: CapabilityMetadata,
    choice: CapabilityChoice,
    input: &str,
    runner: impl FnOnce(&Path, &Path, &str) -> Result<ChildRunOutput>,
) -> Result<CapabilityInvokeReport> {
    fs::create_dir_all(workspace)
        .with_context(|| format!("Failed to create {}", workspace.display()))?;
    let invocation_id = format!("cap-{}", chrono::Utc::now().timestamp_millis());
    let invocation_dir = workspace.join(INVOCATION_ROOT).join(&invocation_id);
    fs::create_dir_all(&invocation_dir)
        .with_context(|| format!("Failed to create {}", invocation_dir.display()))?;

    let config = resolved_config_for_capability(&capability, input);
    let resolved_config_path = invocation_dir.join("resolved-config.yml");
    fs::write(&resolved_config_path, serde_yaml::to_string(&config)?)
        .with_context(|| format!("Failed to write {}", resolved_config_path.display()))?;

    let invocation = CapabilityInvocationRecord {
        invocation_id: invocation_id.clone(),
        ts: Utc::now(),
        capability: capability.clone(),
        choice,
        input: input.to_string(),
        input_contract: capability.input_contract.clone(),
        resolved_config_path: resolved_config_path.display().to_string(),
        parent_topology_unchanged: true,
    };
    write_json(&invocation_dir.join("invoke.json"), &invocation)?;

    log_capability_event(workspace, TOPIC_CAPABILITY_INVOKE, &invocation)?;

    let child_output = runner(workspace, &resolved_config_path, input)?;

    let result_summary = if child_output.success {
        format!(
            "{} completed as isolated {}",
            capability.id, capability.invocation_mode
        )
    } else {
        format!(
            "{} failed as isolated {}",
            capability.id, capability.invocation_mode
        )
    };
    let result = CapabilityResultRecord {
        invocation_id,
        ts: Utc::now(),
        capability_id: capability.id.clone(),
        result_summary,
        exit_code: child_output.exit_code,
        stdout_summary: summarize_output(&child_output.stdout),
        stderr_summary: summarize_output(&child_output.stderr),
        output_contract: capability.output_contract.clone(),
        parent_topology_unchanged: true,
    };

    if child_output.success {
        write_json(&invocation_dir.join("result.json"), &result)?;
        log_capability_event(workspace, TOPIC_CAPABILITY_RESULT, &result)?;
    } else {
        let failed = CapabilityFailedRecord {
            invocation_id: result.invocation_id.clone(),
            ts: Utc::now(),
            capability_id: capability.id.clone(),
            error: result.stderr_summary.clone(),
            parent_topology_unchanged: true,
        };
        write_json(&invocation_dir.join("failed.json"), &failed)?;
        log_capability_event(workspace, TOPIC_CAPABILITY_FAILED, &failed)?;
    }

    Ok(CapabilityInvokeReport { invocation, result })
}

fn resolved_config_for_capability(capability: &CapabilityMetadata, input: &str) -> RalphConfig {
    let mut config = RalphConfig::default();
    config.event_loop.prompt = Some(format!(
        "Runtime capability invocation: {}\n\nInput:\n{}",
        capability.id, input
    ));
    config.event_loop.prompt_file.clear();
    config.event_loop.max_iterations = 1;
    config.event_loop.max_runtime_seconds = 120;
    config.cli.backend = "custom".to_string();
    config.cli.command = Some("true".to_string());
    config.cli.prompt_mode = "stdin".to_string();
    config.core.workspace_root = PathBuf::from(".");
    config
}

fn run_child_dry_run(
    workspace: &Path,
    resolved_config_path: &Path,
    input: &str,
) -> Result<ChildRunOutput> {
    let current_exe = std::env::current_exe().context("Failed to resolve current executable")?;
    let output = Command::new(current_exe)
        .args([
            "run",
            "--config",
            resolved_config_path.to_string_lossy().as_ref(),
            "--dry-run",
            "--no-tui",
            "--prompt",
            input,
        ])
        .current_dir(workspace)
        .output()
        .context("Failed to execute isolated capability child dry-run")?;
    Ok(output.into())
}

fn log_capability_event(workspace: &Path, topic: &str, value: &impl Serialize) -> Result<()> {
    let payload = serde_json::to_string(value)?;
    let event = Event::new(topic, payload);
    let mut logger = EventLogger::new(workspace.join(".ralph/events.jsonl"));
    logger.log_event(0, "capability", &event, None)?;
    Ok(())
}

fn summarize_output(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes).replace('\n', " ");
    let trimmed = text.trim();
    if trimmed.chars().count() <= 500 {
        return trimmed.to_string();
    }
    let mut summary = trimmed.chars().take(500).collect::<String>();
    summary.push_str("... [truncated]");
    summary
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("Invalid artifact path: {}", path.display()))?;
    fs::create_dir_all(parent).with_context(|| format!("Failed to create {}", parent.display()))?;
    fs::write(path, serde_json::to_string_pretty(value)?)
        .with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn catalog_exposes_workflow_and_hat_capability_summaries() {
        let catalog = capability_catalog();

        assert!(
            catalog
                .iter()
                .any(|capability| capability.kind == CapabilityKind::WorkflowCapability)
        );
        assert!(
            catalog
                .iter()
                .any(|capability| capability.kind == CapabilityKind::HatCapability)
        );
        assert!(
            catalog
                .iter()
                .all(|capability| !capability.summary.is_empty())
        );
        assert!(
            catalog
                .iter()
                .all(|capability| !capability.input_contract.is_empty())
        );
    }

    #[test]
    fn rules_v1_chooser_prefers_hat_for_review_input() {
        let catalog = capability_catalog();
        let choice = choose_capability(&catalog, None, "review this patch").unwrap();
        let selected = catalog
            .iter()
            .find(|capability| capability.id == choice.capability_id)
            .unwrap();

        assert_eq!(selected.kind, CapabilityKind::HatCapability);
        assert_eq!(choice.chooser_version, CHOOSER_VERSION_V1);
    }

    #[test]
    fn isolated_invocation_writes_auditable_artifacts_without_parent_topology_mutation() {
        let temp = TempDir::new().unwrap();
        let parent_config = temp.path().join("ralph.yml");
        fs::write(&parent_config, "parent topology sentinel").unwrap();
        let before = fs::read_to_string(&parent_config).unwrap();
        let capability = capability_catalog()
            .into_iter()
            .find(|capability| capability.kind == CapabilityKind::HatCapability)
            .unwrap();
        let choice = CapabilityChoice {
            capability_id: capability.id.clone(),
            reason: "test".to_string(),
            chooser_version: CHOOSER_VERSION_V1.to_string(),
        };

        let report = invoke_isolated_with_runner(
            temp.path(),
            capability,
            choice,
            "review input",
            |_workspace, resolved_config, _input| {
                assert!(resolved_config.exists());
                Ok(ChildRunOutput {
                    success: true,
                    exit_code: Some(0),
                    stdout: b"child dry-run ok".to_vec(),
                    stderr: Vec::new(),
                })
            },
        )
        .unwrap();

        assert!(report.invocation.parent_topology_unchanged);
        assert!(report.result.parent_topology_unchanged);
        assert_eq!(fs::read_to_string(&parent_config).unwrap(), before);
        let invocation_dir = temp
            .path()
            .join(INVOCATION_ROOT)
            .join(&report.invocation.invocation_id);
        assert!(invocation_dir.join("invoke.json").exists());
        assert!(invocation_dir.join("result.json").exists());
        assert!(invocation_dir.join("resolved-config.yml").exists());

        let events = fs::read_to_string(temp.path().join(".ralph/events.jsonl")).unwrap();
        assert!(events.contains(TOPIC_CAPABILITY_INVOKE));
        assert!(events.contains(TOPIC_CAPABILITY_RESULT));
    }
}
