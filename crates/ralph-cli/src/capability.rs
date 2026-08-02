//! Runtime capability tools。
//!
//! 这是 `ralph#1` 可调用的 agent-facing surface。
//! v1 只做规则选择 + 隔离 child/micro-run,不会热改当前 live topology。

use anyhow::{Context, Result, anyhow, bail};
use async_trait::async_trait;
use chrono::Utc;
use clap::{Parser, Subcommand, ValueEnum};
use ralph_adapters::{
    CliBackend, CliExecutionRole, scrub_codex_parent_session_env, scrub_ralph_parent_worker_env,
};
use ralph_core::{
    CapabilityChoice, CapabilityFailedRecord, CapabilityFailureClass, CapabilityInvocationMode,
    CapabilityInvocationRecord, CapabilityKind, CapabilityMetadata, CapabilityParentArtifactPaths,
    CapabilityParentFailedRecord, CapabilityParentResultRecord, CapabilityRequestRecord,
    CapabilityResultRecord, EventLogger, EvidenceArtifactKind, EvidenceIndexEntry,
    EvidenceIndexReader, EvidenceIndexWriter, EvidenceLookup, EvidenceStatus, IdentitySource,
    RalphConfig, RoleContract, RuntimeCapabilityInvoker, TOPIC_CAPABILITY_FAILED,
    TOPIC_CAPABILITY_INVOKE, TOPIC_CAPABILITY_RESULT,
};
use ralph_proto::Event;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;

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
    /// 按 invocation id 查看 capability evidence index 链接。
    Inspect(CapabilityInspectArgs),
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

    /// 只预览 resolved child config,不执行真实 child run。
    #[arg(long)]
    pub preview: bool,
}

/// Capability evidence inspect 参数。
#[derive(Parser, Debug)]
pub struct CapabilityInspectArgs {
    /// Capability invocation id / correlation id。
    pub invocation_id: String,

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
        CapabilityCommands::Inspect(args) => inspect_capability_evidence(args),
    }
}

fn load_capability_base_config(workspace: &Path) -> Result<RalphConfig> {
    let config_path = workspace.join("ralph.yml");
    let mut config = if config_path.exists() {
        RalphConfig::from_file(&config_path)
            .with_context(|| format!("Failed to load config from {}", config_path.display()))?
    } else {
        RalphConfig::default()
    };

    // ---------------------------------------------------------------------
    // capability child config base
    //
    // 说明:
    // - `tools capability invoke` 没有全局 `--config`,所以默认读取 workspace
    //   里的 `ralph.yml`。
    // - live parent run 会直接把已归一化的 parent config 注入 invoker。
    // - 这里也做 normalize / workspace_root 对齐,避免 v1/v2 字段在 child
    //   config 中出现语义漂移。
    // ---------------------------------------------------------------------
    config.normalize();
    config.core.workspace_root = workspace.to_path_buf();
    Ok(config)
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
    let base_config = load_capability_base_config(&workspace)?;
    let catalog = filtered_capabilities(args.kind);
    let choice = choose_capability(&catalog, args.id.as_deref(), &args.input)?;
    let capability = catalog
        .iter()
        .find(|candidate| candidate.id == choice.capability_id)
        .cloned()
        .ok_or_else(|| anyhow!("Selected capability disappeared: {}", choice.capability_id))?;

    let child_run_mode = if args.preview {
        CapabilityChildRunMode::DryRun
    } else {
        child_run_mode_for_capability(&capability)
    };
    let report = invoke_isolated(
        &workspace,
        capability,
        choice,
        &args.input,
        &base_config,
        child_run_mode,
    )?;
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

/// 创建 CLI runtime capability invoker。
///
/// 说明:
/// - 这是 parallel parent run 注入给 core supervisor 的 adapter。
/// - adapter 只复用现有 isolated invocation path,不修改 parent config/topology。
pub(crate) fn runtime_capability_invoker(
    workspace: PathBuf,
    base_config: RalphConfig,
) -> Arc<dyn RuntimeCapabilityInvoker> {
    Arc::new(CliRuntimeCapabilityInvoker {
        workspace,
        base_config,
    })
}

struct CliRuntimeCapabilityInvoker {
    workspace: PathBuf,
    base_config: RalphConfig,
}

#[async_trait]
impl RuntimeCapabilityInvoker for CliRuntimeCapabilityInvoker {
    async fn invoke(&self, request: CapabilityRequestRecord) -> Result<Event> {
        let workspace = self.workspace.clone();
        let base_config = self.base_config.clone();
        tokio::task::spawn_blocking(move || {
            invoke_parent_request(&workspace, request, &base_config)
        })
        .await
        .context("runtime capability invocation task panicked")?
    }
}

fn invoke_parent_request(
    workspace: &Path,
    request: CapabilityRequestRecord,
    base_config: &RalphConfig,
) -> Result<Event> {
    match invoke_capability_by_id(
        workspace,
        &request.capability_id,
        &request.input,
        base_config,
    ) {
        Ok(report) if report.child_success => {
            Ok(parent_result_event(workspace, &request, &report)?)
        }
        Ok(report) => Ok(parent_invocation_failed_event(
            workspace, &request, &report,
        )?),
        Err(error) => Ok(parent_failed_event(workspace, &request, error)),
    }
}

fn invoke_capability_by_id(
    workspace: &Path,
    capability_id: &str,
    input: &str,
    base_config: &RalphConfig,
) -> Result<CapabilityInvokeReport> {
    let catalog = capability_catalog();
    let choice = choose_capability(&catalog, Some(capability_id), input)?;
    let capability = catalog
        .iter()
        .find(|candidate| candidate.id == choice.capability_id)
        .cloned()
        .ok_or_else(|| anyhow!("Selected capability disappeared: {}", choice.capability_id))?;

    let child_run_mode = child_run_mode_for_capability(&capability);
    invoke_isolated(
        workspace,
        capability,
        choice,
        input,
        base_config,
        child_run_mode,
    )
}

fn classify_capability_resolution_error(error: &anyhow::Error) -> CapabilityFailureClass {
    let error_text = error.to_string();
    if error_text.contains("Unknown capability id:") {
        return CapabilityFailureClass::InvalidCapabilityId;
    }

    CapabilityFailureClass::Other
}

fn parent_result_event(
    workspace: &Path,
    request: &CapabilityRequestRecord,
    report: &CapabilityInvokeReport,
) -> Result<Event> {
    let result = CapabilityParentResultRecord {
        status: "result".to_string(),
        request_id: request.request_id.clone(),
        invocation_id: report.invocation.invocation_id.clone(),
        capability_id: report.invocation.capability.id.clone(),
        result_summary: report.result.result_summary.clone(),
        artifacts: parent_artifact_paths(workspace, &report.invocation.invocation_id, true),
        parent_topology_unchanged: report.invocation.parent_topology_unchanged
            && report.result.parent_topology_unchanged,
    };

    Ok(Event::new(
        TOPIC_CAPABILITY_RESULT,
        serde_json::to_string(&result)?,
    ))
}

fn parent_failed_event(
    _workspace: &Path,
    request: &CapabilityRequestRecord,
    error: anyhow::Error,
) -> Event {
    let failed = CapabilityParentFailedRecord {
        status: "failed".to_string(),
        failure_class: classify_capability_resolution_error(&error),
        request_id: Some(request.request_id.clone()),
        invocation_id: None,
        capability_id: Some(request.capability_id.clone()),
        error: format!("{error:#}"),
        artifacts: None,
        parent_topology_unchanged: true,
    };

    Event::new(
        TOPIC_CAPABILITY_FAILED,
        serde_json::to_string(&failed).expect("CapabilityParentFailedRecord serializes"),
    )
}

fn parent_invocation_failed_event(
    workspace: &Path,
    request: &CapabilityRequestRecord,
    report: &CapabilityInvokeReport,
) -> Result<Event> {
    let failed = CapabilityParentFailedRecord {
        status: "failed".to_string(),
        failure_class: CapabilityFailureClass::ChildRunFailed,
        request_id: Some(request.request_id.clone()),
        invocation_id: Some(report.invocation.invocation_id.clone()),
        capability_id: Some(report.invocation.capability.id.clone()),
        error: report.result.stderr_summary.clone(),
        artifacts: Some(parent_artifact_paths(
            workspace,
            &report.invocation.invocation_id,
            false,
        )),
        parent_topology_unchanged: report.invocation.parent_topology_unchanged
            && report.result.parent_topology_unchanged,
    };

    Ok(Event::new(
        TOPIC_CAPABILITY_FAILED,
        serde_json::to_string(&failed)?,
    ))
}

fn parent_artifact_paths(
    workspace: &Path,
    invocation_id: &str,
    success: bool,
) -> CapabilityParentArtifactPaths {
    let invocation_dir = workspace.join(INVOCATION_ROOT).join(invocation_id);

    CapabilityParentArtifactPaths {
        invoke_json: invocation_dir.join("invoke.json").display().to_string(),
        result_json: success.then(|| invocation_dir.join("result.json").display().to_string()),
        failed_json: (!success).then(|| invocation_dir.join("failed.json").display().to_string()),
        resolved_config: invocation_dir
            .join("resolved-config.yml")
            .display()
            .to_string(),
        events_jsonl: workspace.join(".ralph/events.jsonl").display().to_string(),
        evidence_index: workspace
            .join(EvidenceIndexWriter::DEFAULT_PATH)
            .display()
            .to_string(),
    }
}

fn inspect_capability_evidence(args: CapabilityInspectArgs) -> Result<()> {
    let workspace = args
        .workspace
        .unwrap_or(std::env::current_dir().context("Failed to resolve current directory")?);
    let report = inspect_capability_evidence_report(&workspace, &args.invocation_id)?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    print_capability_inspect_report(&report);
    Ok(())
}

fn inspect_capability_evidence_report(
    workspace: &Path,
    invocation_id: &str,
) -> Result<CapabilityInspectReport> {
    let index_path = workspace.join(EvidenceIndexWriter::DEFAULT_PATH);
    let lookup = EvidenceIndexReader::new(&index_path)
        .find_by_correlation(invocation_id)
        .with_context(|| format!("Failed to read evidence index at {}", index_path.display()))?;

    let status = match lookup {
        EvidenceLookup::Entries(entries) => CapabilityInspectStatus::Entries { entries },
        EvidenceLookup::Missing(entries) => CapabilityInspectStatus::Missing { entries },
        EvidenceLookup::NoEntry => bail!(
            "No evidence entries for capability invocation id `{}` in {}",
            invocation_id,
            index_path.display()
        ),
    };

    Ok(CapabilityInspectReport {
        invocation_id: invocation_id.to_string(),
        index_path: index_path.display().to_string(),
        status,
    })
}

fn print_capability_inspect_report(report: &CapabilityInspectReport) {
    println!("Invocation: {}", report.invocation_id);
    println!("Evidence index: {}", report.index_path);
    println!("Status: {}", report.status.as_str());

    for entry in report.entries() {
        let artifact_kind = serde_json::to_value(entry.artifact_kind)
            .ok()
            .and_then(|value| value.as_str().map(ToOwned::to_owned))
            .unwrap_or_else(|| format!("{:?}", entry.artifact_kind));
        let status = serde_json::to_value(entry.status)
            .ok()
            .and_then(|value| value.as_str().map(ToOwned::to_owned))
            .unwrap_or_else(|| format!("{:?}", entry.status));

        println!("- {}", artifact_kind);
        println!("  Path: {}", entry.artifact_path);
        println!("  Producer: {}", entry.producer);
        println!("  Status: {}", status);
    }
}

/// `inspect` 的 JSON 契约。
///
/// 说明:
/// - 这里复用 core 的 `EvidenceIndexEntry` 序列化,避免 CLI 再定义一套 artifact schema。
/// - `status` 表达 lookup 分类,不是覆盖每条 entry 自己的 success/failure/missing 状态。
#[derive(Debug, Clone, Serialize)]
struct CapabilityInspectReport {
    invocation_id: String,
    index_path: String,
    #[serde(flatten)]
    status: CapabilityInspectStatus,
}

impl CapabilityInspectReport {
    fn entries(&self) -> &[EvidenceIndexEntry] {
        match &self.status {
            CapabilityInspectStatus::Entries { entries }
            | CapabilityInspectStatus::Missing { entries } => entries,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum CapabilityInspectStatus {
    Entries { entries: Vec<EvidenceIndexEntry> },
    Missing { entries: Vec<EvidenceIndexEntry> },
}

impl CapabilityInspectStatus {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Entries { .. } => "entries",
            Self::Missing { .. } => "missing",
        }
    }
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
    #[serde(skip_serializing)]
    child_success: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CapabilityChildRunMode {
    DryRun,
    Execute,
}

fn child_run_mode_for_capability(capability: &CapabilityMetadata) -> CapabilityChildRunMode {
    match capability.kind {
        CapabilityKind::WorkflowCapability => CapabilityChildRunMode::Execute,
        CapabilityKind::HatCapability => CapabilityChildRunMode::Execute,
    }
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
    base_config: &RalphConfig,
    child_run_mode: CapabilityChildRunMode,
) -> Result<CapabilityInvokeReport> {
    let runner = match (child_run_mode, capability.kind) {
        (CapabilityChildRunMode::DryRun, _) => run_child_dry_run,
        (CapabilityChildRunMode::Execute, CapabilityKind::HatCapability) => {
            run_hat_capability_execute
        }
        (CapabilityChildRunMode::Execute, CapabilityKind::WorkflowCapability) => run_child_execute,
    };
    invoke_isolated_with_runner(workspace, capability, choice, input, base_config, runner)
}

fn role_contract_for_capability(capability: &CapabilityMetadata) -> Option<RoleContract> {
    if capability.kind != CapabilityKind::HatCapability {
        return None;
    }

    // ---------------------------------------------------------------------
    // task-derived micro-run provenance
    //
    // 说明:
    // - `hat:*` capability 是 Ralph 根据当前任务临时调用的 bounded reviewer/worker。
    // - 它不会加入 parent topology,也不是 autoscale 出来的 runtime instance。
    // - 因此这里把身份来源显式写成 TaskDerived,让 artifact 能证明两者没有混用。
    // ---------------------------------------------------------------------
    Some(RoleContract::new(
        capability.id.clone(),
        capability.goal.clone(),
        capability.input_contract.clone(),
        capability.output_contract.clone(),
        vec![
            TOPIC_CAPABILITY_RESULT.to_string(),
            TOPIC_CAPABILITY_FAILED.to_string(),
        ],
        vec![
            "Do not mutate the parent topology.".to_string(),
            "Do not act as the Ralph coordinator.".to_string(),
            "Do not create additional hats unless explicitly delegated by the parent run."
                .to_string(),
        ],
        vec![
            "Return a bounded result summary suitable for the parent run.".to_string(),
            "Preserve parent_topology_unchanged=true.".to_string(),
        ],
        IdentitySource::TaskDerived,
    ))
}

fn invoke_isolated_with_runner(
    workspace: &Path,
    capability: CapabilityMetadata,
    choice: CapabilityChoice,
    input: &str,
    base_config: &RalphConfig,
    runner: impl FnOnce(&Path, &Path, &str, &Path) -> Result<ChildRunOutput>,
) -> Result<CapabilityInvokeReport> {
    fs::create_dir_all(workspace)
        .with_context(|| format!("Failed to create {}", workspace.display()))?;
    let invocation_id = format!("cap-{}", chrono::Utc::now().timestamp_millis());
    let invocation_dir = workspace.join(INVOCATION_ROOT).join(&invocation_id);
    fs::create_dir_all(&invocation_dir)
        .with_context(|| format!("Failed to create {}", invocation_dir.display()))?;
    let events_path = workspace.join(".ralph/events.jsonl");
    let evidence_path = workspace.join(EvidenceIndexWriter::DEFAULT_PATH);
    let mut evidence_writer = EvidenceIndexWriter::new(&evidence_path);

    let config = resolved_config_for_capability(&capability, input, base_config)?;
    let resolved_config_path = invocation_dir.join("resolved-config.yml");
    fs::write(&resolved_config_path, serde_yaml::to_string(&config)?)
        .with_context(|| format!("Failed to write {}", resolved_config_path.display()))?;
    record_capability_evidence(
        &mut evidence_writer,
        &invocation_id,
        EvidenceArtifactKind::ResolvedConfig,
        &resolved_config_path,
        EvidenceStatus::Success,
    )?;

    let invocation = CapabilityInvocationRecord {
        invocation_id: invocation_id.clone(),
        ts: Utc::now(),
        capability: capability.clone(),
        choice,
        input: input.to_string(),
        input_contract: capability.input_contract.clone(),
        resolved_config_path: resolved_config_path.display().to_string(),
        role_contract: role_contract_for_capability(&capability),
        parent_topology_unchanged: true,
    };
    let invoke_path = invocation_dir.join("invoke.json");
    write_json(&invoke_path, &invocation)?;
    record_capability_evidence(
        &mut evidence_writer,
        &invocation_id,
        EvidenceArtifactKind::CapabilityInvokeJson,
        &invoke_path,
        EvidenceStatus::Success,
    )?;

    log_capability_event(workspace, TOPIC_CAPABILITY_INVOKE, &invocation)?;
    record_capability_evidence(
        &mut evidence_writer,
        &invocation_id,
        EvidenceArtifactKind::EventLogJsonl,
        &events_path,
        EvidenceStatus::Success,
    )?;

    let child_record_session_path = invocation_dir.join("child-record-session.jsonl");
    let child_output = runner(
        workspace,
        &resolved_config_path,
        input,
        &child_record_session_path,
    )?;

    if child_record_session_path.exists() {
        record_capability_evidence(
            &mut evidence_writer,
            &invocation_id,
            EvidenceArtifactKind::RecordSessionJsonl,
            &child_record_session_path,
            if child_output.success {
                EvidenceStatus::Success
            } else {
                EvidenceStatus::Failure
            },
        )?;
    }

    let stdout_summary = summarize_output(&child_output.stdout);
    let stderr_summary = summarize_output(&child_output.stderr);
    let result_summary = child_result_summary(
        &capability,
        child_output.success,
        &stdout_summary,
        &stderr_summary,
        child_record_session_path
            .exists()
            .then_some(child_record_session_path.as_path()),
    );
    let result = CapabilityResultRecord {
        invocation_id,
        ts: Utc::now(),
        capability_id: capability.id.clone(),
        result_summary,
        exit_code: child_output.exit_code,
        stdout_summary,
        stderr_summary,
        output_contract: capability.output_contract.clone(),
        parent_topology_unchanged: true,
    };

    let child_success = child_output.success;

    if child_success {
        let result_path = invocation_dir.join("result.json");
        write_json(&result_path, &result)?;
        record_capability_evidence(
            &mut evidence_writer,
            &result.invocation_id,
            EvidenceArtifactKind::CapabilityResultJson,
            &result_path,
            EvidenceStatus::Success,
        )?;
        log_capability_event(workspace, TOPIC_CAPABILITY_RESULT, &result)?;
    } else {
        let failed = CapabilityFailedRecord {
            invocation_id: result.invocation_id.clone(),
            ts: Utc::now(),
            capability_id: capability.id.clone(),
            failure_class: CapabilityFailureClass::ChildRunFailed,
            error: result.stderr_summary.clone(),
            parent_topology_unchanged: true,
        };
        let failed_path = invocation_dir.join("failed.json");
        write_json(&failed_path, &failed)?;
        record_capability_evidence(
            &mut evidence_writer,
            &failed.invocation_id,
            EvidenceArtifactKind::CapabilityFailedJson,
            &failed_path,
            EvidenceStatus::Failure,
        )?;
        log_capability_event(workspace, TOPIC_CAPABILITY_FAILED, &failed)?;
    }

    Ok(CapabilityInvokeReport {
        invocation,
        result,
        child_success,
    })
}

fn resolved_config_for_capability(
    capability: &CapabilityMetadata,
    input: &str,
    base_config: &RalphConfig,
) -> Result<RalphConfig> {
    if capability.kind == CapabilityKind::WorkflowCapability {
        return startup_resources::resolve_workflow_capability_config(&capability.id, input);
    }

    Ok(resolved_micro_run_config_for_capability(
        capability,
        input,
        base_config,
    ))
}

fn resolved_micro_run_config_for_capability(
    capability: &CapabilityMetadata,
    input: &str,
    base_config: &RalphConfig,
) -> RalphConfig {
    let mut config = base_config.clone();
    config.event_loop.prompt = Some(format!(
        "Runtime hat capability invocation: {}\n\nRole goal:\n{}\n\nInput:\n{}\n\nReturn a concise result for the parent run. Do not act as the Ralph coordinator. Do not create additional hats.",
        capability.id, capability.goal, input
    ));
    config.event_loop.ralph_prompt = None;
    config.event_loop.prompt_file.clear();
    config.event_loop.max_iterations = 1;
    config.event_loop.max_runtime_seconds = 120;
    config.event_loop.starting_hat = None;
    config.event_loop.starting_event = None;
    config.event_loop.complete_publishes = None;
    config.parallel.enabled = false;
    config.hats = HashMap::new();
    config.events = HashMap::new();
    config.core.runtime_capabilities_enabled = false;
    config
}

fn run_child_dry_run(
    workspace: &Path,
    resolved_config_path: &Path,
    input: &str,
    _child_record_session_path: &Path,
) -> Result<ChildRunOutput> {
    run_child(
        workspace,
        resolved_config_path,
        input,
        CapabilityChildRunMode::DryRun,
        None,
    )
}

fn run_child_execute(
    workspace: &Path,
    resolved_config_path: &Path,
    input: &str,
    child_record_session_path: &Path,
) -> Result<ChildRunOutput> {
    run_child(
        workspace,
        resolved_config_path,
        input,
        CapabilityChildRunMode::Execute,
        Some(child_record_session_path),
    )
}

fn run_hat_capability_execute(
    workspace: &Path,
    resolved_config_path: &Path,
    input: &str,
    _child_record_session_path: &Path,
) -> Result<ChildRunOutput> {
    let config = RalphConfig::from_file(resolved_config_path).with_context(|| {
        format!(
            "Failed to load hat capability child config from {}",
            resolved_config_path.display()
        )
    })?;
    let prompt = config
        .event_loop
        .prompt
        .clone()
        .filter(|prompt| !prompt.trim().is_empty())
        .unwrap_or_else(|| input.to_string());

    // ---------------------------------------------------------------------
    // hat capability execute
    //
    // 说明:
    // - `hat:*` capability 是 task-derived transient worker,不是新的 Ralph
    //   coordinator。
    // - 因此这里直接调用底层 backend,不再嵌套 `ralph run`。
    // - 这样可以避免注入 coordinator prompt,也避免 loop 的双确认机制让
    //   简单 review 多耗一轮。
    // ---------------------------------------------------------------------
    let mut backend = CliBackend::from_config(&config.cli).map_err(anyhow::Error::new)?;
    backend.apply_role_args(&config.cli.role_args, CliExecutionRole::Worker);
    backend.apply_role_reasoning_effort_defaults(
        &config.cli.reasoning_effort,
        CliExecutionRole::Worker,
    );
    let (command_name, args, stdin_input, _temp_file) = backend.build_command(&prompt, false);
    let mut command = Command::new(&command_name);
    command.args(args);
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    command.current_dir(workspace);
    scrub_ralph_parent_worker_env(&mut command);
    command.env("RALPH_CAPABILITY_CHILD", "1");
    command.env("RALPH_CAPABILITY_MODE", "execute");
    scrub_codex_parent_session_env(&mut command, &command_name);

    if stdin_input.is_some() {
        command.stdin(Stdio::piped());
    }

    let mut child = command.spawn().with_context(|| {
        format!(
            "Failed to execute hat capability backend command `{}`",
            command_name
        )
    })?;

    if let Some(input) = stdin_input
        && let Some(mut stdin) = child.stdin.take()
    {
        stdin
            .write_all(input.as_bytes())
            .context("Failed to write hat capability prompt to backend stdin")?;
    }

    let output = child
        .wait_with_output()
        .context("Failed to wait for hat capability backend output")?;
    Ok(output.into())
}

fn run_child(
    workspace: &Path,
    resolved_config_path: &Path,
    input: &str,
    child_run_mode: CapabilityChildRunMode,
    child_record_session_path: Option<&Path>,
) -> Result<ChildRunOutput> {
    let current_exe = std::env::current_exe().context("Failed to resolve current executable")?;
    let mut command = Command::new(current_exe);
    command.args(child_run_args(
        resolved_config_path,
        input,
        child_run_mode,
        child_record_session_path,
    ));
    scrub_ralph_parent_worker_env(&mut command);
    command.env("RALPH_CAPABILITY_CHILD", "1");
    command.env(
        "RALPH_CAPABILITY_MODE",
        match child_run_mode {
            CapabilityChildRunMode::DryRun => "preview",
            CapabilityChildRunMode::Execute => "execute",
        },
    );
    command.current_dir(workspace);
    let output = command.output().with_context(|| match child_run_mode {
        CapabilityChildRunMode::DryRun => {
            "Failed to execute isolated capability child dry-run".to_string()
        }
        CapabilityChildRunMode::Execute => {
            "Failed to execute isolated capability child run".to_string()
        }
    })?;
    Ok(output.into())
}

fn child_run_args(
    resolved_config_path: &Path,
    input: &str,
    child_run_mode: CapabilityChildRunMode,
    child_record_session_path: Option<&Path>,
) -> Vec<String> {
    let mut args = vec![
        "run".to_string(),
        "--config".to_string(),
        resolved_config_path.display().to_string(),
        "--no-tui".to_string(),
    ];

    if matches!(child_run_mode, CapabilityChildRunMode::DryRun) {
        args.push("--dry-run".to_string());
        args.push("--prompt".to_string());
        args.push(input.to_string());
    }

    if matches!(child_run_mode, CapabilityChildRunMode::Execute)
        && let Some(record_session_path) = child_record_session_path
    {
        args.push("--record-session".to_string());
        args.push(record_session_path.display().to_string());
    }

    args
}

fn log_capability_event(workspace: &Path, topic: &str, value: &impl Serialize) -> Result<()> {
    let payload = serde_json::to_string(value)?;
    let event = Event::new(topic, payload);
    let mut logger = EventLogger::new(workspace.join(".ralph/events.jsonl"));
    logger.log_event(0, "capability", &event, None)?;
    Ok(())
}

fn record_capability_evidence(
    writer: &mut EvidenceIndexWriter,
    invocation_id: &str,
    artifact_kind: EvidenceArtifactKind,
    artifact_path: &Path,
    status: EvidenceStatus,
) -> Result<()> {
    // ─────────────────────────────────────────────────────────────────────
    // Phase 3 的关键边界:
    // artifact 本身仍是真相源,evidence index 只保存可查询的路径链接。
    // 因此调用方必须先写 artifact,再调用这个 helper 注册 evidence。
    // ─────────────────────────────────────────────────────────────────────
    let entry = EvidenceIndexEntry::new(
        invocation_id,
        artifact_kind,
        artifact_path.display().to_string(),
        "capability",
        status,
    );
    writer
        .record(&entry)
        .with_context(|| format!("Failed to record evidence for {invocation_id}"))
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

fn child_result_summary(
    capability: &CapabilityMetadata,
    success: bool,
    stdout_summary: &str,
    stderr_summary: &str,
    child_record_session_path: Option<&Path>,
) -> String {
    if capability.kind == CapabilityKind::WorkflowCapability
        && let Some(path) = child_record_session_path
        && let Ok(summary) = workflow_child_record_summary(capability, success, path)
    {
        return summary;
    }

    let status = if success { "completed" } else { "failed" };
    let evidence = if success {
        stdout_summary
    } else {
        stderr_summary
    };
    if evidence.is_empty() {
        return format!(
            "{} {} as isolated {}",
            capability.id, status, capability.invocation_mode
        );
    }

    format!(
        "{} {} as isolated {}: {}",
        capability.id, status, capability.invocation_mode, evidence
    )
}

fn workflow_child_record_summary(
    capability: &CapabilityMetadata,
    success: bool,
    child_record_session_path: &Path,
) -> Result<String> {
    let aggregate = ralph_core::aggregate_session(child_record_session_path)?;
    let status = if success { "completed" } else { "failed" };
    let termination = aggregate
        .termination
        .as_ref()
        .and_then(|termination| termination.reason.as_deref())
        .unwrap_or("<missing>");
    let topics = concise_topic_timeline(&aggregate.topic_timeline, 8);
    let topics_summary = if topics.is_empty() {
        "<none>".to_string()
    } else {
        topics.join(" -> ")
    };
    let record_file = child_record_session_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("child-record-session.jsonl");

    Ok(format!(
        "{} {} as isolated {}: termination={}; topics={}; record_session={}",
        capability.id, status, capability.invocation_mode, termination, topics_summary, record_file
    ))
}

fn concise_topic_timeline(timeline: &[String], max_topics: usize) -> Vec<String> {
    let mut out = Vec::new();
    for topic in timeline {
        if out.last() == Some(topic) {
            continue;
        }
        out.push(topic.clone());
        if out.len() >= max_topics {
            break;
        }
    }
    out
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
    use ralph_core::{EvidenceIndexReader, EvidenceLookup};
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
        fs::write(&parent_config, "core: {}\n# parent topology sentinel\n").unwrap();
        let before = fs::read_to_string(&parent_config).unwrap();
        let base_config = RalphConfig::default();
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
            &base_config,
            |_workspace, resolved_config, _input, child_record_session_path| {
                assert!(resolved_config.exists());
                fs::write(
                    child_record_session_path,
                    "{\"event\":\"_meta.termination\"}\n",
                )
                .unwrap();
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

        let evidence_lookup =
            EvidenceIndexReader::new(temp.path().join(EvidenceIndexWriter::DEFAULT_PATH))
                .find_by_correlation(&report.invocation.invocation_id)
                .unwrap();
        let evidence_entries = evidence_lookup.entries();
        assert!(matches!(evidence_lookup, EvidenceLookup::Entries(_)));
        assert!(evidence_entries.iter().any(|entry| {
            entry.artifact_kind == EvidenceArtifactKind::CapabilityInvokeJson
                && entry.status == EvidenceStatus::Success
        }));
        assert!(evidence_entries.iter().any(|entry| {
            entry.artifact_kind == EvidenceArtifactKind::CapabilityResultJson
                && entry.status == EvidenceStatus::Success
        }));
        assert!(evidence_entries.iter().any(|entry| {
            entry.artifact_kind == EvidenceArtifactKind::ResolvedConfig
                && entry.status == EvidenceStatus::Success
        }));
        assert!(evidence_entries.iter().any(|entry| {
            entry.artifact_kind == EvidenceArtifactKind::EventLogJsonl
                && entry.status == EvidenceStatus::Success
        }));
        assert!(evidence_entries.iter().any(|entry| {
            entry.artifact_kind == EvidenceArtifactKind::RecordSessionJsonl
                && entry.status == EvidenceStatus::Success
                && entry.artifact_path.ends_with("child-record-session.jsonl")
        }));
    }

    #[test]
    fn isolated_invocation_failure_writes_failed_artifact_for_parent_audit() {
        let temp = TempDir::new().unwrap();
        let base_config = RalphConfig::default();
        let capability = capability_catalog()
            .into_iter()
            .find(|capability| capability.kind == CapabilityKind::HatCapability)
            .unwrap();
        let choice = CapabilityChoice {
            capability_id: capability.id.clone(),
            reason: "test failure".to_string(),
            chooser_version: CHOOSER_VERSION_V1.to_string(),
        };

        let report = invoke_isolated_with_runner(
            temp.path(),
            capability,
            choice,
            "review input",
            &base_config,
            |_workspace, resolved_config, _input, _child_record_session_path| {
                assert!(resolved_config.exists());
                Ok(ChildRunOutput {
                    success: false,
                    exit_code: Some(77),
                    stdout: Vec::new(),
                    stderr: b"child failed".to_vec(),
                })
            },
        )
        .unwrap();

        assert!(report.invocation.parent_topology_unchanged);
        assert!(report.result.parent_topology_unchanged);

        let invocation_dir = temp
            .path()
            .join(INVOCATION_ROOT)
            .join(&report.invocation.invocation_id);
        assert!(invocation_dir.join("invoke.json").exists());
        assert!(invocation_dir.join("failed.json").exists());
        assert!(!invocation_dir.join("result.json").exists());
        let failed_json: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(invocation_dir.join("failed.json")).unwrap())
                .unwrap();
        assert_eq!(failed_json["failure_class"], "child_run_failed");

        let events = fs::read_to_string(temp.path().join(".ralph/events.jsonl")).unwrap();
        assert!(events.contains(TOPIC_CAPABILITY_INVOKE));
        assert!(events.contains(TOPIC_CAPABILITY_FAILED));

        let evidence_lookup =
            EvidenceIndexReader::new(temp.path().join(EvidenceIndexWriter::DEFAULT_PATH))
                .find_by_correlation(&report.invocation.invocation_id)
                .unwrap();
        let evidence_entries = evidence_lookup.entries();
        assert!(matches!(evidence_lookup, EvidenceLookup::Entries(_)));
        assert!(evidence_entries.iter().any(|entry| {
            entry.artifact_kind == EvidenceArtifactKind::CapabilityFailedJson
                && entry.status == EvidenceStatus::Failure
                && entry.artifact_path.ends_with("failed.json")
        }));
        assert!(evidence_entries.iter().any(|entry| {
            entry.artifact_kind == EvidenceArtifactKind::ResolvedConfig
                && entry.status == EvidenceStatus::Success
        }));
    }

    #[test]
    fn isolated_invocation_fails_when_evidence_index_cannot_be_recorded() {
        let temp = TempDir::new().unwrap();
        let blocked_evidence_path = temp.path().join(EvidenceIndexWriter::DEFAULT_PATH);
        fs::create_dir_all(&blocked_evidence_path).unwrap();
        let base_config = RalphConfig::default();
        let capability = capability_catalog()
            .into_iter()
            .find(|capability| capability.kind == CapabilityKind::HatCapability)
            .unwrap();
        let choice = CapabilityChoice {
            capability_id: capability.id.clone(),
            reason: "test evidence failure".to_string(),
            chooser_version: CHOOSER_VERSION_V1.to_string(),
        };

        let error = invoke_isolated_with_runner(
            temp.path(),
            capability,
            choice,
            "review input",
            &base_config,
            |_workspace, _resolved_config, _input, _child_record_session_path| {
                panic!("runner should not start after evidence index recording fails");
            },
        )
        .unwrap_err();

        assert!(
            error.to_string().contains("Failed to record evidence for"),
            "unexpected error: {error:#}"
        );
    }

    #[test]
    fn resolution_error_is_classified_as_invalid_capability_id() {
        let error = anyhow!("Unknown capability id: hat:missing-reviewer");
        assert_eq!(
            classify_capability_resolution_error(&error),
            CapabilityFailureClass::InvalidCapabilityId
        );
    }

    #[test]
    fn child_run_args_include_prompt_only_in_dry_run_mode() {
        let config_path = PathBuf::from(".ralph/capability-invocations/cap-1/resolved-config.yml");
        let input = "中文输入";

        let record_path =
            PathBuf::from(".ralph/capability-invocations/cap-1/child-record-session.jsonl");
        let dry_run_args = child_run_args(
            &config_path,
            input,
            CapabilityChildRunMode::DryRun,
            Some(&record_path),
        );
        assert!(dry_run_args.contains(&"--dry-run".to_string()));
        assert!(dry_run_args.contains(&"--prompt".to_string()));
        assert!(dry_run_args.contains(&input.to_string()));
        assert!(!dry_run_args.contains(&"--record-session".to_string()));

        let execute_args = child_run_args(
            &config_path,
            input,
            CapabilityChildRunMode::Execute,
            Some(&record_path),
        );
        assert!(!execute_args.contains(&"--dry-run".to_string()));
        assert!(!execute_args.contains(&"--prompt".to_string()));
        assert!(!execute_args.contains(&input.to_string()));
        assert!(execute_args.contains(&"--no-tui".to_string()));
        assert!(execute_args.contains(&"--record-session".to_string()));
        assert!(execute_args.contains(&record_path.display().to_string()));
    }

    #[test]
    fn hat_capability_defaults_to_execute_mode() {
        let capability = capability_catalog()
            .into_iter()
            .find(|capability| capability.kind == CapabilityKind::HatCapability)
            .unwrap();

        assert_eq!(
            child_run_mode_for_capability(&capability),
            CapabilityChildRunMode::Execute
        );
    }

    #[test]
    fn resolved_micro_run_inherits_backend_and_disables_recursion() {
        let capability = capability_catalog()
            .into_iter()
            .find(|capability| capability.kind == CapabilityKind::HatCapability)
            .unwrap();
        let mut base_config = RalphConfig::default();
        base_config.cli.backend = "custom".to_string();
        base_config.cli.command = Some("/tmp/real-reviewer".to_string());
        base_config.cli.prompt_mode = "stdin".to_string();
        base_config.core.runtime_capabilities_enabled = true;
        base_config.parallel.enabled = true;

        let resolved =
            resolved_micro_run_config_for_capability(&capability, "review input", &base_config);

        assert_eq!(resolved.cli.backend, "custom");
        assert_eq!(resolved.cli.command.as_deref(), Some("/tmp/real-reviewer"));
        assert_ne!(resolved.cli.command.as_deref(), Some("true"));
        assert!(!resolved.parallel.enabled);
        assert!(resolved.hats.is_empty());
        assert!(!resolved.core.runtime_capabilities_enabled);
        assert!(
            resolved
                .event_loop
                .prompt
                .as_deref()
                .is_some_and(|prompt| prompt.contains("review input"))
        );
    }

    #[test]
    fn inspect_report_preserves_explicit_missing_evidence_markers() {
        let temp = TempDir::new().unwrap();
        let index_path = temp.path().join(EvidenceIndexWriter::DEFAULT_PATH);
        let mut writer = EvidenceIndexWriter::new(&index_path);
        writer
            .record(&EvidenceIndexEntry::missing(
                "cap-missing-1",
                EvidenceArtifactKind::CapabilityResultJson,
                ".ralph/capability-invocations/cap-missing-1/result.json",
                "capability",
            ))
            .unwrap();

        let report = inspect_capability_evidence_report(temp.path(), "cap-missing-1").unwrap();

        assert!(matches!(
            report.status,
            CapabilityInspectStatus::Missing { .. }
        ));
        assert_eq!(report.entries()[0].status, EvidenceStatus::Missing);
    }
}
