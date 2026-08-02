//! 启动期资源 catalog 与 bootstrap selector。
//!
//! 这个模块只处理“真实 orchestration loop 启动前”的资源解析。
//! 它不做运行中 topology 热切换,也不执行 runtime capability invocation。

use anyhow::{Context, Result};
use ralph_core::{EventLoopConfig, RalphConfig};
use serde::Serialize;
use std::path::{Path, PathBuf};

/// 默认配置文件名。
pub(crate) const DEFAULT_CONFIG_FILE: &str = "ralph.yml";

/// v1 规则 selector 选用的默认 workflow。
///
/// 说明:
/// - 这是无配置启动的 canonical bootstrap resource。
/// - 内容来自仓库根 `ralph.yml` 的编译期嵌入,避免再复制一份 YAML。
pub(crate) const DEFAULT_BOOTSTRAP_WORKFLOW_ID: &str = "workflow:default-parallel";

/// 默认 bootstrap workflow 在用户资源目录中的物化位置。
const DEFAULT_BOOTSTRAP_WORKFLOW_RELATIVE_PATH: &str = "workflows/default-parallel.yml";

/// legacy 最小 feature workflow,保留为可物化资源,但不再作为默认 selector 输出。
const LEGACY_FEATURE_MINIMAL_WORKFLOW_ID: &str = "workflow:feature-minimal";

/// canonical bootstrap 配置内容。
const CANONICAL_DEFAULT_BOOTSTRAP_CONFIG: &str = include_str!("../../../ralph.yml");

/// v1 规则 selector 选用的默认 prompt template。
pub(crate) const DEFAULT_BOOTSTRAP_PROMPT_ID: &str = "prompt:bootstrap-default-task";

/// 无 prompt/config 启动时注入的默认任务输入。
///
/// 说明:
/// - 这是“启动闭环”的兜底 prompt,不是用户任务的替代品。
/// - 它让 Ralph 可以先启动、观察工作区、给出下一步建议,而不是因为缺少 `PROMPT.md` 直接失败。
const DEFAULT_BOOTSTRAP_PROMPT: &str = r"No ralph.yml or PROMPT.md was found in this workspace.

Act as Ralph's startup bootstrap coordinator:
1. Inspect the current workspace.
2. Summarize what can be safely inferred.
3. If there is no concrete task to execute, ask the user for the next task.
4. Emit LOOP_COMPLETE only after producing a useful startup summary or clear next-step request.";

/// 启动资源类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ResourceKind {
    /// 可作为主工作流的 preset。
    WorkflowPreset,
    /// 只提供 backend/CLI 设置的 preset。
    BackendPreset,
    /// 可作为任务输入的 prompt 模板。
    PromptTemplate,
    /// 示例项目/场景包,默认不参与 selector。
    ExampleBundle,
}

/// 资源在组合过程中的角色。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CompositionRole {
    /// 主 workflow,每次 resolved config 只允许一个。
    Workflow,
    /// backend 覆盖层,每次 resolved config 最多一个。
    Backend,
    /// prompt 来源,每次 resolved config 最多一个。
    Prompt,
    /// 示例/materialize 专用资源,不参与默认 selector。
    MaterializeOnly,
}

/// 资源对 prompt 输入的要求。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ResourcePromptMode {
    /// 资源自带完整 prompt。
    SelfContained,
    /// 资源需要外部任务输入。
    RequiresTaskInput,
    /// 资源允许无任务输入时进入待机/启动说明。
    IdleCapable,
}

/// 内嵌启动资源 catalog 条目。
#[derive(Debug, Clone, Serialize)]
pub(crate) struct StartupResource {
    /// 稳定资源 id。
    pub id: &'static str,
    /// 资源类型。
    pub kind: ResourceKind,
    /// 人类和 selector 都可读的短摘要。
    pub summary: &'static str,
    /// 资源目标。
    pub goal: &'static str,
    /// 是否允许默认 selector 自动选择。
    pub selector_eligible: bool,
    /// 首次同步时是否物化到用户资源目录。
    pub materialize_on_sync: bool,
    /// 组合角色。
    pub composition_role: CompositionRole,
    /// prompt 输入模式。
    pub prompt_mode: ResourcePromptMode,
    /// 用户资源目录中的相对路径。
    pub relative_path: &'static str,
    /// 可物化内容。示例 bundle v1 只登记 metadata,可以没有单文件内容。
    #[serde(skip_serializing)]
    pub content: Option<&'static str>,
}

/// 用户资源目录解析结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResourceRoot {
    /// 实际资源根目录。
    pub path: PathBuf,
    /// 解析来源,用于 artifact/debug。
    pub source: String,
}

/// 用户资源同步摘要。
#[derive(Debug, Clone, Default, Serialize)]
pub(crate) struct ResourceSyncSummary {
    /// 新写入的资源文件数。
    pub created: usize,
    /// 因为用户已有文件而保留的资源数。
    pub preserved: usize,
    /// 没有单文件内容、仅登记 metadata 的资源数。
    pub metadata_only: usize,
}

/// prompt source resolver 的结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum ResolvedPromptSource {
    /// config 或 CLI 已提供 inline prompt。
    InlineConfig,
    /// config 或 CLI 指向 prompt 文件。
    PromptFile { path: String },
    /// selector 选中的 prompt template。
    PromptTemplate { resource_id: String },
    /// 无 prompt 时使用 idle/bootstrap 兜底。
    IdleBootstrap { resource_id: String },
}

/// bootstrap selector 的选择摘要。
#[derive(Debug, Clone, Serialize)]
pub(crate) struct BootstrapSelection {
    /// 选择原因。
    pub reason: String,
    /// 用户资源根目录。
    pub resource_root: String,
    /// 用户资源根目录来源。
    pub resource_root_source: String,
    /// 同步摘要。
    pub sync: ResourceSyncSummary,
    /// 选中的资源 id。
    pub selected_resources: Vec<String>,
    /// 确定性 merge 顺序。
    pub merge_order: Vec<String>,
    /// prompt 来源。
    pub prompt_source: ResolvedPromptSource,
    /// 明确记录 startup-only 边界。
    pub startup_only: bool,
}

/// 完整 bootstrap resolution 结果。
pub(crate) struct BootstrapResolution {
    /// 真实 run 将使用的配置。
    pub config: RalphConfig,
    /// selector 证据。
    pub selection: BootstrapSelection,
}

/// 返回内嵌启动资源 catalog。
pub(crate) fn embedded_catalog() -> &'static [StartupResource] {
    &[
        StartupResource {
            id: DEFAULT_BOOTSTRAP_WORKFLOW_ID,
            kind: ResourceKind::WorkflowPreset,
            summary: "Canonical default parallel bootstrap workflow",
            goal: "Provide the repository default custom+codex+parallel runtime semantics when no workspace config exists",
            selector_eligible: true,
            materialize_on_sync: true,
            composition_role: CompositionRole::Workflow,
            prompt_mode: ResourcePromptMode::RequiresTaskInput,
            relative_path: DEFAULT_BOOTSTRAP_WORKFLOW_RELATIVE_PATH,
            content: Some(CANONICAL_DEFAULT_BOOTSTRAP_CONFIG),
        },
        StartupResource {
            id: LEGACY_FEATURE_MINIMAL_WORKFLOW_ID,
            kind: ResourceKind::WorkflowPreset,
            summary: "Minimal feature workflow with builder/reviewer hats",
            goal: "Remain available as an explicit materialized workflow, but no longer define the implicit default bootstrap runtime",
            selector_eligible: false,
            materialize_on_sync: true,
            composition_role: CompositionRole::Workflow,
            prompt_mode: ResourcePromptMode::RequiresTaskInput,
            relative_path: "workflows/feature-minimal.yml",
            content: Some(include_str!("../presets/feature-minimal.yml")),
        },
        StartupResource {
            id: "workflow:hatless-baseline",
            kind: ResourceKind::WorkflowPreset,
            summary: "Hatless baseline workflow",
            goal: "Provide a simple control workflow for explicit materialization",
            selector_eligible: false,
            materialize_on_sync: true,
            composition_role: CompositionRole::Workflow,
            prompt_mode: ResourcePromptMode::RequiresTaskInput,
            relative_path: "workflows/hatless-baseline.yml",
            content: Some(include_str!("../presets/hatless-baseline.yml")),
        },
        StartupResource {
            id: "backend:claude",
            kind: ResourceKind::BackendPreset,
            summary: "Claude CLI backend defaults",
            goal: "Materialize a backend preset users can copy or compose explicitly",
            selector_eligible: false,
            materialize_on_sync: true,
            composition_role: CompositionRole::Backend,
            prompt_mode: ResourcePromptMode::RequiresTaskInput,
            relative_path: "backends/claude.yml",
            content: Some(include_str!("../presets/minimal/claude.yml")),
        },
        StartupResource {
            id: "backend:codex",
            kind: ResourceKind::BackendPreset,
            summary: "Codex CLI backend defaults",
            goal: "Materialize a backend preset users can copy or compose explicitly",
            selector_eligible: false,
            materialize_on_sync: true,
            composition_role: CompositionRole::Backend,
            prompt_mode: ResourcePromptMode::RequiresTaskInput,
            relative_path: "backends/codex.yml",
            content: Some(include_str!("../presets/minimal/codex.yml")),
        },
        StartupResource {
            id: "backend:kiro",
            kind: ResourceKind::BackendPreset,
            summary: "Kiro CLI backend defaults",
            goal: "Materialize a backend preset users can copy or compose explicitly",
            selector_eligible: false,
            materialize_on_sync: true,
            composition_role: CompositionRole::Backend,
            prompt_mode: ResourcePromptMode::RequiresTaskInput,
            relative_path: "backends/kiro.yml",
            content: Some(include_str!("../presets/minimal/kiro.yml")),
        },
        StartupResource {
            id: DEFAULT_BOOTSTRAP_PROMPT_ID,
            kind: ResourceKind::PromptTemplate,
            summary: "Default startup bootstrap prompt",
            goal: "Let Ralph start safely when no PROMPT.md is present",
            selector_eligible: true,
            materialize_on_sync: true,
            composition_role: CompositionRole::Prompt,
            prompt_mode: ResourcePromptMode::IdleCapable,
            relative_path: "prompts/bootstrap-default-task.md",
            content: Some(DEFAULT_BOOTSTRAP_PROMPT),
        },
        StartupResource {
            id: "example:parallel-pr-review",
            kind: ResourceKind::ExampleBundle,
            summary: "Parallel PR review example bundle",
            goal: "Remain available for explicit materialization without default auto-selection",
            selector_eligible: false,
            materialize_on_sync: false,
            composition_role: CompositionRole::MaterializeOnly,
            prompt_mode: ResourcePromptMode::SelfContained,
            relative_path: "examples/parallel-pr-review",
            content: None,
        },
    ]
}

/// 缺失默认 config 的 bootstrap 判定输入。
pub(crate) struct MissingDefaultConfigBootstrapInput<'a> {
    pub(crate) config_path: &'a Path,
    pub(crate) config_was_explicit: bool,
    pub(crate) has_cli_prompt_text: bool,
    pub(crate) has_cli_prompt_file: bool,
    pub(crate) resume: bool,
}

/// 判断缺失的 config 是否应该进入 bootstrap selector。
pub(crate) fn should_bootstrap_missing_default_config(
    input: MissingDefaultConfigBootstrapInput<'_>,
) -> bool {
    !input.config_was_explicit
        && !input.resume
        && !input.has_cli_prompt_text
        && !input.has_cli_prompt_file
        && input.config_path == Path::new(DEFAULT_CONFIG_FILE)
        && !input.config_path.exists()
}

/// 解析用户资源根目录。
pub(crate) fn resolve_resource_root() -> ResourceRoot {
    if let Some(home) = std::env::var_os("RALPH_HOME").filter(|value| !value.is_empty()) {
        return ResourceRoot {
            path: PathBuf::from(home).join("resources"),
            source: "RALPH_HOME".to_string(),
        };
    }

    if let Some(home) = std::env::var_os("HOME").filter(|value| !value.is_empty()) {
        return ResourceRoot {
            path: PathBuf::from(home).join(".ralph").join("resources"),
            source: "HOME/.ralph".to_string(),
        };
    }

    ResourceRoot {
        path: PathBuf::from(".ralph").join("resources"),
        source: "workspace-fallback".to_string(),
    }
}

/// 首次同步内嵌资源到用户资源目录,且不覆盖已有文件。
pub(crate) fn sync_embedded_resources(
    resource_root: &Path,
    catalog: &[StartupResource],
) -> Result<ResourceSyncSummary> {
    let mut summary = ResourceSyncSummary::default();
    std::fs::create_dir_all(resource_root)
        .with_context(|| format!("Failed to create resource root {}", resource_root.display()))?;

    for resource in catalog.iter().filter(|entry| entry.materialize_on_sync) {
        let Some(content) = resource.content else {
            summary.metadata_only += 1;
            continue;
        };

        let target = resource_root.join(resource.relative_path);
        if target.exists() {
            summary.preserved += 1;
            continue;
        }

        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create resource directory {}", parent.display())
            })?;
        }
        std::fs::write(&target, content)
            .with_context(|| format!("Failed to materialize resource {}", target.display()))?;
        summary.created += 1;
    }

    write_catalog_manifest(resource_root, catalog, &summary)?;
    Ok(summary)
}

/// 按 v1 纯规则 selector 产出默认启动配置。
pub(crate) fn resolve_default_bootstrap() -> Result<BootstrapResolution> {
    resolve_default_bootstrap_with_root(resolve_resource_root())
}

/// 解析 workflow preset 的真实配置内容。
///
/// 说明:
/// - 这个 helper 只负责把嵌入式 workflow YAML 变成 `RalphConfig`。
/// - prompt 注入策略由上层决定,这样 bootstrap / capability 可以共享同一份解析入口。
fn parse_workflow_preset(workflow: &StartupResource) -> Result<RalphConfig> {
    let content = workflow
        .content
        .with_context(|| format!("Workflow preset {} has no embedded content", workflow.id))?;

    RalphConfig::parse_yaml(content)
        .with_context(|| format!("Failed to parse workflow preset {}", workflow.id))
}

/// 使用指定资源根目录解析默认启动配置。
///
/// 说明:
/// - 生产路径走 `resolve_default_bootstrap()` 并读取真实环境。
/// - 测试路径注入临时目录,避免写入用户真实 `$HOME/.ralph/resources`。
fn resolve_default_bootstrap_with_root(resource_root: ResourceRoot) -> Result<BootstrapResolution> {
    let catalog = embedded_catalog();
    let sync = sync_embedded_resources(&resource_root.path, catalog)?;

    let workflow = catalog
        .iter()
        .find(|entry| entry.id == DEFAULT_BOOTSTRAP_WORKFLOW_ID)
        .context("Default bootstrap workflow missing from startup resource catalog")?;
    let prompt = catalog
        .iter()
        .find(|entry| entry.id == DEFAULT_BOOTSTRAP_PROMPT_ID)
        .context("Default bootstrap prompt missing from startup resource catalog")?;

    let config = resolve_workflow_with_prompt_template(workflow, prompt)?;

    let selection = BootstrapSelection {
        reason: format!(
            "missing default {DEFAULT_CONFIG_FILE} and no explicit prompt; selected v1 rule default"
        ),
        resource_root: resource_root.path.display().to_string(),
        resource_root_source: resource_root.source,
        sync,
        selected_resources: vec![workflow.id.to_string(), prompt.id.to_string()],
        merge_order: vec![
            workflow.id.to_string(),
            "resolved_prompt_source".to_string(),
            "cli_overrides".to_string(),
        ],
        prompt_source: resolve_prompt_source(&config.event_loop, Some(prompt.id)),
        startup_only: true,
    };

    Ok(BootstrapResolution { config, selection })
}

/// 将 workflow preset 与 prompt template 组合为单份 resolved config。
///
/// 说明:
/// - v1 只做单 workflow + 单 prompt template 的确定性组合。
/// - 自带 inline prompt 的 workflow 保持自包含,不会被 idle/bootstrap prompt 覆盖。
/// - 需要外部 prompt 的 workflow 才会注入选中的 prompt template。
fn resolve_workflow_with_prompt_template(
    workflow: &StartupResource,
    prompt: &StartupResource,
) -> Result<RalphConfig> {
    let mut config = parse_workflow_preset(workflow)?;

    if config.event_loop.prompt.is_none() {
        config.event_loop.prompt = Some(
            prompt
                .content
                .context("Bootstrap prompt has no embedded content")?
                .to_string(),
        );
        config.event_loop.prompt_file.clear();
    } else if config.event_loop.prompt_file == "PROMPT.md" {
        // 自带 inline prompt 的 workflow 是 self-contained。
        // 清掉历史默认 prompt_file,避免 artifact 误导用户以为还会读取 `PROMPT.md`。
        config.event_loop.prompt_file.clear();
    }

    // ─────────────────────────────────────────────────────────────────────
    // 无配置启动的产品契约:
    // - 用户执行 `ralph run` 时,缺失 `ralph.yml` / `PROMPT.md` 不应退回串行旧默认。
    // - bootstrap 产物等价于一份 startup-only 的默认 `ralph.yml`。
    // - 默认运行模式必须是并行,这样 `ralph#1` 能作为协调者接收 catalog / hats 拓扑。
    // ─────────────────────────────────────────────────────────────────────
    config.parallel.enabled = true;

    Ok(config)
}

/// 将 workflow preset 物化为 capability 可执行的 resolved config。
///
/// 说明:
/// - capability invocation 需要保留 workflow preset 的真实 hats / parallel 结构。
/// - 这里只替换 task prompt 为 parent 传入的输入,不再回退到空配置 stub。
pub(crate) fn resolve_workflow_capability_config(
    workflow_id: &str,
    input: &str,
) -> Result<RalphConfig> {
    let workflow = embedded_catalog()
        .iter()
        .find(|entry| entry.id == workflow_id && entry.kind == ResourceKind::WorkflowPreset)
        .with_context(|| {
            format!("Workflow capability resource `{workflow_id}` missing from startup catalog")
        })?;

    let mut config = parse_workflow_preset(workflow)?;
    config.event_loop.prompt = Some(input.to_string());
    config.event_loop.prompt_file.clear();
    config.core.runtime_capabilities_enabled = false;
    Ok(config)
}

/// 解析最终 prompt 来源。
pub(crate) fn resolve_prompt_source(
    event_loop: &EventLoopConfig,
    selected_prompt_template: Option<&str>,
) -> ResolvedPromptSource {
    if event_loop.prompt.is_some() {
        if let Some(resource_id) = selected_prompt_template {
            return ResolvedPromptSource::PromptTemplate {
                resource_id: resource_id.to_string(),
            };
        }
        return ResolvedPromptSource::InlineConfig;
    }

    if !event_loop.prompt_file.is_empty() {
        return ResolvedPromptSource::PromptFile {
            path: event_loop.prompt_file.clone(),
        };
    }

    ResolvedPromptSource::IdleBootstrap {
        resource_id: selected_prompt_template
            .unwrap_or(DEFAULT_BOOTSTRAP_PROMPT_ID)
            .to_string(),
    }
}

/// 写出 resolved config 和 selector 证据。
pub(crate) fn write_bootstrap_artifacts(
    workspace_root: &Path,
    selection: &BootstrapSelection,
    config: &RalphConfig,
) -> Result<()> {
    let ralph_dir = workspace_root.join(".ralph");
    std::fs::create_dir_all(&ralph_dir)
        .with_context(|| format!("Failed to create {}", ralph_dir.display()))?;

    let selection_path = ralph_dir.join("bootstrap-selection.json");
    let selection_json = serde_json::to_string_pretty(selection)
        .context("Failed to serialize bootstrap selection artifact")?;
    std::fs::write(&selection_path, selection_json)
        .with_context(|| format!("Failed to write {}", selection_path.display()))?;

    let config_path = ralph_dir.join("resolved-config.yml");
    let config_yaml =
        serde_yaml::to_string(config).context("Failed to serialize resolved config artifact")?;
    std::fs::write(&config_path, config_yaml)
        .with_context(|| format!("Failed to write {}", config_path.display()))?;

    Ok(())
}

fn write_catalog_manifest(
    resource_root: &Path,
    catalog: &[StartupResource],
    sync: &ResourceSyncSummary,
) -> Result<()> {
    #[derive(Serialize)]
    struct CatalogManifest<'a> {
        version: u32,
        resources: &'a [StartupResource],
        last_sync: &'a ResourceSyncSummary,
    }

    let manifest = CatalogManifest {
        version: 1,
        resources: catalog,
        last_sync: sync,
    };
    let path = resource_root.join("catalog-manifest.json");
    let json = serde_json::to_string_pretty(&manifest)
        .context("Failed to serialize resource catalog manifest")?;
    std::fs::write(&path, json)
        .with_context(|| format!("Failed to write catalog manifest {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn embedded_catalog_covers_required_resource_kinds() {
        let catalog = embedded_catalog();

        assert!(
            catalog
                .iter()
                .any(|entry| entry.kind == ResourceKind::WorkflowPreset)
        );
        assert!(
            catalog
                .iter()
                .any(|entry| entry.kind == ResourceKind::BackendPreset)
        );
        assert!(
            catalog
                .iter()
                .any(|entry| entry.kind == ResourceKind::PromptTemplate)
        );
        assert!(
            catalog
                .iter()
                .any(|entry| entry.kind == ResourceKind::ExampleBundle)
        );
        assert!(
            catalog
                .iter()
                .any(|entry| entry.id == DEFAULT_BOOTSTRAP_WORKFLOW_ID && entry.selector_eligible)
        );
        assert!(catalog.iter().any(
            |entry| entry.id == LEGACY_FEATURE_MINIMAL_WORKFLOW_ID && !entry.selector_eligible
        ));
        assert!(
            catalog
                .iter()
                .any(|entry| entry.kind == ResourceKind::ExampleBundle && !entry.selector_eligible)
        );
    }

    #[test]
    fn sync_embedded_resources_materializes_without_overwriting_user_changes() {
        let temp = TempDir::new().unwrap();
        let catalog = embedded_catalog();

        let first = sync_embedded_resources(temp.path(), catalog).unwrap();
        assert!(first.created > 0);
        assert!(temp.path().join("catalog-manifest.json").exists());

        let workflow_path = temp.path().join(DEFAULT_BOOTSTRAP_WORKFLOW_RELATIVE_PATH);
        std::fs::write(&workflow_path, "user-edited").unwrap();

        let second = sync_embedded_resources(temp.path(), catalog).unwrap();
        assert!(second.preserved > 0);
        assert_eq!(
            std::fs::read_to_string(&workflow_path).unwrap(),
            "user-edited"
        );
    }

    fn repository_default_config() -> RalphConfig {
        RalphConfig::parse_yaml(CANONICAL_DEFAULT_BOOTSTRAP_CONFIG)
            .expect("repository default ralph.yml should parse")
    }

    fn assert_bootstrap_runtime_fields_match(actual: &RalphConfig, expected: &RalphConfig) {
        assert_eq!(actual.cli.backend, expected.cli.backend);
        assert_eq!(actual.cli.command, expected.cli.command);
        assert_eq!(actual.cli.prompt_mode, expected.cli.prompt_mode);
        assert_eq!(actual.cli.args, expected.cli.args);
        assert_eq!(actual.parallel.enabled, expected.parallel.enabled);
        assert_eq!(
            actual.parallel.autoscale.max_running_jobs,
            expected.parallel.autoscale.max_running_jobs
        );
    }

    #[test]
    fn default_bootstrap_resolution_has_inline_prompt_and_selection_artifact_data() {
        let temp = TempDir::new().unwrap();
        let resolution = resolve_default_bootstrap_with_root(ResourceRoot {
            path: temp.path().join("resources"),
            source: "test".to_string(),
        })
        .unwrap();

        assert!(resolution.config.event_loop.prompt.is_some());
        assert!(resolution.config.event_loop.prompt_file.is_empty());
        let repository_config = repository_default_config();
        assert_bootstrap_runtime_fields_match(&resolution.config, &repository_config);
        assert!(
            resolution.config.parallel.enabled,
            "无配置 bootstrap 应默认生成并行模式配置"
        );
        assert_eq!(
            resolution.config.parallel.autoscale.max_running_jobs, 7,
            "当前 canonical 默认并行安全刹车应和仓库根 ralph.yml 保持一致"
        );
        assert!(resolution.config.validate().is_ok());
        assert_eq!(
            resolution.selection.selected_resources,
            vec![
                DEFAULT_BOOTSTRAP_WORKFLOW_ID.to_string(),
                DEFAULT_BOOTSTRAP_PROMPT_ID.to_string()
            ]
        );
        assert!(resolution.selection.startup_only);
        assert!(matches!(
            resolution.selection.prompt_source,
            ResolvedPromptSource::PromptTemplate { .. }
        ));
    }

    fn test_prompt_resource() -> StartupResource {
        StartupResource {
            id: "prompt:test",
            kind: ResourceKind::PromptTemplate,
            summary: "test prompt",
            goal: "test prompt goal",
            selector_eligible: true,
            materialize_on_sync: false,
            composition_role: CompositionRole::Prompt,
            prompt_mode: ResourcePromptMode::IdleCapable,
            relative_path: "prompts/test.md",
            content: Some("test bootstrap prompt"),
        }
    }

    fn test_workflow_resource(id: &'static str, content: &'static str) -> StartupResource {
        StartupResource {
            id,
            kind: ResourceKind::WorkflowPreset,
            summary: "test workflow",
            goal: "test workflow goal",
            selector_eligible: true,
            materialize_on_sync: false,
            composition_role: CompositionRole::Workflow,
            prompt_mode: ResourcePromptMode::RequiresTaskInput,
            relative_path: "workflows/test.yml",
            content: Some(content),
        }
    }

    #[test]
    fn workflow_prompt_template_resolution_supports_parallel_workflow() {
        let workflow = test_workflow_resource(
            "workflow:parallel-test",
            r#"
event_loop:
  completion_promise: "LOOP_COMPLETE"
  max_iterations: 2
cli:
  backend: "custom"
  command: "true"
parallel:
  enabled: true
"#,
        );
        let prompt = test_prompt_resource();

        let config = resolve_workflow_with_prompt_template(&workflow, &prompt).unwrap();

        assert!(config.parallel.enabled);
        assert_eq!(
            config.event_loop.prompt.as_deref(),
            Some("test bootstrap prompt")
        );
        assert!(config.event_loop.prompt_file.is_empty());
        assert!(config.validate().is_ok());
    }

    #[test]
    fn self_contained_inline_prompt_workflow_is_not_overwritten_by_idle_bootstrap() {
        let workflow = test_workflow_resource(
            "workflow:inline-test",
            r#"
event_loop:
  prompt: "self contained task"
  completion_promise: "LOOP_COMPLETE"
  max_iterations: 1
cli:
  backend: "custom"
  command: "true"
"#,
        );
        let prompt = test_prompt_resource();

        let config = resolve_workflow_with_prompt_template(&workflow, &prompt).unwrap();

        assert_eq!(
            config.event_loop.prompt.as_deref(),
            Some("self contained task")
        );
        assert!(config.event_loop.prompt_file.is_empty());
        assert_eq!(
            resolve_prompt_source(&config.event_loop, None),
            ResolvedPromptSource::InlineConfig
        );
        assert!(config.validate().is_ok());
    }

    #[test]
    fn prompt_source_resolver_covers_inline_file_template_and_idle_bootstrap() {
        let mut event_loop = EventLoopConfig::default();

        event_loop.prompt = Some("config inline".to_string());
        event_loop.prompt_file.clear();
        assert_eq!(
            resolve_prompt_source(&event_loop, None),
            ResolvedPromptSource::InlineConfig
        );
        assert_eq!(
            resolve_prompt_source(&event_loop, Some(DEFAULT_BOOTSTRAP_PROMPT_ID)),
            ResolvedPromptSource::PromptTemplate {
                resource_id: DEFAULT_BOOTSTRAP_PROMPT_ID.to_string()
            }
        );

        event_loop.prompt = None;
        event_loop.prompt_file = "PROMPT.md".to_string();
        assert_eq!(
            resolve_prompt_source(&event_loop, None),
            ResolvedPromptSource::PromptFile {
                path: "PROMPT.md".to_string()
            }
        );

        event_loop.prompt_file.clear();
        assert_eq!(
            resolve_prompt_source(&event_loop, None),
            ResolvedPromptSource::IdleBootstrap {
                resource_id: DEFAULT_BOOTSTRAP_PROMPT_ID.to_string()
            }
        );
    }

    #[test]
    fn missing_default_config_bootstrap_gate_is_narrow() {
        let temp = TempDir::new().unwrap();
        let old_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp.path()).unwrap();

        let check =
            |config_path, config_was_explicit, has_cli_prompt_text, has_cli_prompt_file, resume| {
                should_bootstrap_missing_default_config(MissingDefaultConfigBootstrapInput {
                    config_path,
                    config_was_explicit,
                    has_cli_prompt_text,
                    has_cli_prompt_file,
                    resume,
                })
            };

        assert!(check(
            Path::new(DEFAULT_CONFIG_FILE),
            false,
            false,
            false,
            false
        ));
        assert!(!check(
            Path::new(DEFAULT_CONFIG_FILE),
            true,
            false,
            false,
            false
        ));
        assert!(!check(Path::new("custom.yml"), false, false, false, false));
        assert!(!check(
            Path::new(DEFAULT_CONFIG_FILE),
            false,
            true,
            false,
            false
        ));
        assert!(!check(
            Path::new(DEFAULT_CONFIG_FILE),
            false,
            false,
            true,
            false
        ));
        assert!(!check(
            Path::new(DEFAULT_CONFIG_FILE),
            false,
            false,
            false,
            true
        ));

        std::env::set_current_dir(old_cwd).unwrap();
    }

    #[test]
    fn write_bootstrap_artifacts_creates_resolved_config_and_selection_json() {
        let temp = TempDir::new().unwrap();
        let resolution = resolve_default_bootstrap_with_root(ResourceRoot {
            path: temp.path().join("resources"),
            source: "test".to_string(),
        })
        .unwrap();

        write_bootstrap_artifacts(temp.path(), &resolution.selection, &resolution.config).unwrap();

        let selection =
            std::fs::read_to_string(temp.path().join(".ralph/bootstrap-selection.json")).unwrap();
        let config =
            std::fs::read_to_string(temp.path().join(".ralph/resolved-config.yml")).unwrap();
        assert!(selection.contains(DEFAULT_BOOTSTRAP_WORKFLOW_ID));
        assert!(selection.contains("\"startup_only\": true"));
        assert!(config.contains("event_loop:"));
        assert!(config.contains("prompt:"));
    }
}
