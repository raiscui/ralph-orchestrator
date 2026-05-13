//! Agent guidance manifest verifier.
//!
//! 这个模块负责验证仓库里的 agent-facing guidance 资产清单。
//! 它不参与 runtime 调度,只把“哪些指导文件是正式资产”变成可测试契约。

use serde::Deserialize;
use std::collections::HashSet;
use std::fmt;
use std::path::{Component, Path};
use thiserror::Error;

/// 当前支持的 guidance manifest schema 版本。
const SUPPORTED_SCHEMA_VERSION: u32 = 1;

/// 默认 manifest 文件名。
pub const DEFAULT_AGENT_GUIDANCE_MANIFEST: &str = "agent-guidance-manifest.toml";

/// 解析后的 guidance manifest。
#[derive(Debug, Deserialize)]
struct GuidanceManifest {
    schema_version: u32,
    #[serde(default)]
    assets: Vec<GuidanceAsset>,
}

/// 单个 guidance 资产条目。
#[derive(Debug, Deserialize)]
struct GuidanceAsset {
    id: String,
    #[serde(rename = "type")]
    asset_type: String,
    path: String,
    status: String,
    summary: String,
    #[serde(default)]
    required_in_agents_index: bool,
}

/// guidance manifest 验证报告。
///
/// CLI 和测试门禁都使用同一份报告入口,避免出现“两套 verifier”的漂移。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuidanceManifestReport {
    /// 被验证的 manifest 路径。保持调用方传入的仓库相对形式。
    pub manifest_path: String,
    /// manifest 中检查过的资产总数。
    pub asset_count: usize,
    /// manifest 中检查过的非 archived skill 数量。
    pub skill_count: usize,
}

/// skill frontmatter 的最小机器可读字段。
///
/// 这里只验证治理契约必需字段,不把 verifier 做成完整 Markdown linter。
#[derive(Debug, Deserialize)]
struct SkillFrontmatter {
    name: Option<String>,
    description: Option<String>,
}

/// manifest 验证过程中的累积状态。
struct VerificationState {
    seen_ids: HashSet<String>,
    seen_skill_names: HashSet<String>,
    asset_count: usize,
    skill_count: usize,
}

impl VerificationState {
    /// 创建空状态。每次验证单独创建,保证一次命令就是一次独立事实。
    fn new() -> Self {
        Self {
            seen_ids: HashSet::new(),
            seen_skill_names: HashSet::new(),
            asset_count: 0,
            skill_count: 0,
        }
    }

    /// 转成对外报告。
    fn into_report(self, manifest_path: &str) -> GuidanceManifestReport {
        GuidanceManifestReport {
            manifest_path: manifest_path.to_string(),
            asset_count: self.asset_count,
            skill_count: self.skill_count,
        }
    }
}

/// manifest 验证失败。
#[derive(Debug, Error)]
pub enum GuidanceManifestError {
    /// manifest 文件读取失败。
    #[error("failed to read guidance manifest `{path}`: {source}")]
    ReadManifest {
        path: String,
        source: std::io::Error,
    },

    /// AGENTS.md 读取失败。
    #[error("failed to read AGENTS.md for guidance manifest verification: {0}")]
    ReadAgents(std::io::Error),

    /// guidance asset 读取失败。
    #[error("failed to read guidance asset `{path}` for asset `{id}`: {source}")]
    ReadAsset {
        id: AssetLabel,
        path: String,
        source: std::io::Error,
    },

    /// TOML 解析失败。
    #[error("failed to parse guidance manifest TOML: {0}")]
    ParseToml(toml::de::Error),

    /// schema 版本不支持。
    #[error("unsupported guidance manifest schema_version `{found}`, expected `{expected}`")]
    UnsupportedSchemaVersion { found: u32, expected: u32 },

    /// 没有任何资产。
    #[error("guidance manifest must contain at least one asset")]
    EmptyAssets,

    /// 资产字段不合法。
    #[error("asset `{id}` has {problem}")]
    InvalidAsset { id: AssetLabel, problem: String },
}

/// 错误信息中的 asset 标识。
#[derive(Debug, Clone)]
pub struct AssetLabel(String);

impl AssetLabel {
    /// 用尽量有帮助的标签展示坏条目。
    fn new(id: &str) -> Self {
        if id.trim().is_empty() {
            Self("<empty-id>".to_string())
        } else {
            Self(id.to_string())
        }
    }
}

impl fmt::Display for AssetLabel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// 验证默认 guidance manifest。
pub fn verify_default_manifest(repo_root: &Path) -> Result<(), GuidanceManifestError> {
    verify_manifest_at(repo_root, DEFAULT_AGENT_GUIDANCE_MANIFEST)
}

/// 验证默认 guidance manifest,并返回统计报告。
pub fn verify_default_manifest_with_report(
    repo_root: &Path,
) -> Result<GuidanceManifestReport, GuidanceManifestError> {
    verify_manifest_at_with_report(repo_root, DEFAULT_AGENT_GUIDANCE_MANIFEST)
}

/// 验证指定 guidance manifest。
pub fn verify_manifest_at(
    repo_root: &Path,
    manifest_path: &str,
) -> Result<(), GuidanceManifestError> {
    verify_manifest_at_with_report(repo_root, manifest_path).map(|_| ())
}

/// 验证指定 guidance manifest,并返回统计报告。
pub fn verify_manifest_at_with_report(
    repo_root: &Path,
    manifest_path: &str,
) -> Result<GuidanceManifestReport, GuidanceManifestError> {
    let manifest_file = repo_root.join(manifest_path);
    let raw_manifest = std::fs::read_to_string(&manifest_file).map_err(|source| {
        GuidanceManifestError::ReadManifest {
            path: manifest_path.to_string(),
            source,
        }
    })?;
    let manifest: GuidanceManifest =
        toml::from_str(&raw_manifest).map_err(GuidanceManifestError::ParseToml)?;

    if manifest.schema_version != SUPPORTED_SCHEMA_VERSION {
        return Err(GuidanceManifestError::UnsupportedSchemaVersion {
            found: manifest.schema_version,
            expected: SUPPORTED_SCHEMA_VERSION,
        });
    }

    if manifest.assets.is_empty() {
        return Err(GuidanceManifestError::EmptyAssets);
    }

    let agents_index = std::fs::read_to_string(repo_root.join("AGENTS.md"))
        .map_err(GuidanceManifestError::ReadAgents)?;
    let mut state = VerificationState::new();

    for asset in &manifest.assets {
        validate_asset(repo_root, asset, &agents_index, &mut state)?;
    }

    Ok(state.into_report(manifest_path))
}

/// 验证单个 asset。
fn validate_asset(
    repo_root: &Path,
    asset: &GuidanceAsset,
    agents_index: &str,
    state: &mut VerificationState,
) -> Result<(), GuidanceManifestError> {
    let id = asset.id.trim();
    let label = AssetLabel::new(id);

    if id.is_empty() {
        return invalid(label, "empty id");
    }

    if !state.seen_ids.insert(id.to_string()) {
        return invalid(label, "duplicate asset id");
    }

    if !is_kebab_like_id(id) {
        return invalid(label, "id must use kebab-case characters");
    }

    if !is_valid_asset_type(asset.asset_type.trim()) {
        return invalid(label, format!("invalid asset type `{}`", asset.asset_type));
    }

    if !is_valid_status(asset.status.trim()) {
        return invalid(label, format!("invalid asset status `{}`", asset.status));
    }

    if asset.summary.trim().is_empty() {
        return invalid(label, "empty summary");
    }

    if asset.path.trim().is_empty() {
        return invalid(label, "empty path");
    }

    if path_escapes_repo(asset.path.trim()) {
        return invalid(label, "path escapes repository root");
    }

    let asset_path = repo_root.join(asset.path.trim());
    if asset.status.trim() != "archived" && !asset_path.is_file() {
        return invalid(label, format!("missing active asset file `{}`", asset.path));
    }

    if asset.required_in_agents_index && !agents_index.contains(asset.path.trim()) {
        return invalid(
            label,
            format!("missing AGENTS.md index reference `{}`", asset.path),
        );
    }

    state.asset_count += 1;

    if asset.asset_type.trim() == "skill" && asset.status.trim() != "archived" {
        validate_skill_asset(repo_root, asset, id, label.clone(), state)?;
    }

    Ok(())
}

/// 验证 skill asset 的专属契约。
fn validate_skill_asset(
    repo_root: &Path,
    asset: &GuidanceAsset,
    id: &str,
    label: AssetLabel,
    state: &mut VerificationState,
) -> Result<(), GuidanceManifestError> {
    let skill_path = asset.path.trim();

    if !is_allowed_skill_path(skill_path) {
        return invalid(
            label,
            "skill path must be under `.agents/skills/*/SKILL.md` or `.codex/skills/*/SKILL.md`",
        );
    }

    let raw_skill = std::fs::read_to_string(repo_root.join(skill_path)).map_err(|source| {
        GuidanceManifestError::ReadAsset {
            id: AssetLabel::new(id),
            path: skill_path.to_string(),
            source,
        }
    })?;

    let frontmatter = parse_skill_frontmatter(&raw_skill, &label, skill_path)?;
    let skill_name =
        required_skill_frontmatter_field(&frontmatter.name, &label, skill_path, "name")?;

    required_skill_frontmatter_field(&frontmatter.description, &label, skill_path, "description")?;

    if !state.seen_skill_names.insert(skill_name.clone()) {
        return invalid(label, format!("duplicate skill name `{skill_name}`"));
    }

    state.skill_count += 1;
    Ok(())
}

/// 解析 `SKILL.md` 顶部 frontmatter。
fn parse_skill_frontmatter(
    raw_skill: &str,
    label: &AssetLabel,
    skill_path: &str,
) -> Result<SkillFrontmatter, GuidanceManifestError> {
    let Some(rest) = raw_skill.strip_prefix("---") else {
        return invalid(
            label.clone(),
            format!("missing skill frontmatter in `{skill_path}`"),
        );
    };

    let Some((frontmatter, _body)) = rest.split_once("\n---") else {
        return invalid(
            label.clone(),
            format!("unterminated skill frontmatter in `{skill_path}`"),
        );
    };

    serde_yaml::from_str(frontmatter).map_err(|err| GuidanceManifestError::InvalidAsset {
        id: label.clone(),
        problem: format!("invalid skill frontmatter in `{skill_path}`: {err}"),
    })
}

/// 读取必填 frontmatter 字段。
fn required_skill_frontmatter_field(
    value: &Option<String>,
    label: &AssetLabel,
    skill_path: &str,
    field_name: &str,
) -> Result<String, GuidanceManifestError> {
    let Some(value) = value.as_ref().map(String::as_str).map(str::trim) else {
        return invalid(
            label.clone(),
            format!("missing skill frontmatter field `{field_name}` in `{skill_path}`"),
        );
    };

    if value.is_empty() {
        return invalid(
            label.clone(),
            format!("missing skill frontmatter field `{field_name}` in `{skill_path}`"),
        );
    }

    Ok(value.to_string())
}

/// 构造单个 asset 错误。
fn invalid<T>(id: AssetLabel, problem: impl Into<String>) -> Result<T, GuidanceManifestError> {
    Err(GuidanceManifestError::InvalidAsset {
        id,
        problem: problem.into(),
    })
}

/// asset id 使用保守 kebab-case,方便错误定位和跨文件引用。
fn is_kebab_like_id(id: &str) -> bool {
    id.chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
        && !id.starts_with('-')
        && !id.ends_with('-')
        && !id.contains("--")
}

/// 第一阶段允许的 asset 类型。
fn is_valid_asset_type(asset_type: &str) -> bool {
    matches!(
        asset_type,
        "root_contract"
            | "experience"
            | "schema_doc"
            | "prompt_contract"
            | "openspec_change"
            | "skill"
            | "report"
            | "runbook"
    )
}

/// 第一阶段允许的生命周期状态。
fn is_valid_status(status: &str) -> bool {
    matches!(status, "active" | "draft" | "archived")
}

/// 禁止绝对路径、父目录逃逸、Windows 前缀和空组件。
fn path_escapes_repo(path: &str) -> bool {
    let path = Path::new(path);
    path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
}

/// `skill` asset 只能指向项目自有 skill roots。
fn is_allowed_skill_path(path: &str) -> bool {
    let mut components = Path::new(path).components();

    let Some(Component::Normal(root)) = components.next() else {
        return false;
    };

    if root != ".agents" && root != ".codex" {
        return false;
    }

    if !matches!(components.next(), Some(Component::Normal(skills)) if skills == "skills") {
        return false;
    }

    if !matches!(components.next(), Some(Component::Normal(_skill_dir))) {
        return false;
    }

    matches!(components.next(), Some(Component::Normal(file)) if file == "SKILL.md")
        && components.next().is_none()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_file(root: &std::path::Path, path: &str, contents: &str) {
        let full_path = root.join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).expect("create parent dir");
        }
        fs::write(full_path, contents).expect("write test file");
    }

    fn valid_manifest() -> &'static str {
        r#"
schema_version = 1

[[assets]]
id = "agents-root-contract"
type = "root_contract"
path = "AGENTS.md"
status = "active"
summary = "Root operating contract for repository agents."
required_in_agents_index = false

[[assets]]
id = "project-experience"
type = "experience"
path = "EXPERIENCE.md"
status = "active"
summary = "Project-level reusable lessons for agents."
required_in_agents_index = true
"#
    }

    fn valid_repo() -> TempDir {
        let dir = TempDir::new().expect("temp dir");
        write_file(
            dir.path(),
            "AGENTS.md",
            "## Project Knowledge Index\n- `EXPERIENCE.md`: project experience.\n",
        );
        write_file(dir.path(), "EXPERIENCE.md", "# EXPERIENCE.md\n");
        write_file(dir.path(), "agent-guidance-manifest.toml", valid_manifest());
        dir
    }

    #[test]
    fn valid_manifest_passes() {
        let dir = valid_repo();
        verify_manifest_at(dir.path(), "agent-guidance-manifest.toml").unwrap();
    }

    #[test]
    fn duplicate_ids_fail() {
        let dir = valid_repo();
        let manifest = valid_manifest().replace("project-experience", "agents-root-contract");
        write_file(dir.path(), "agent-guidance-manifest.toml", &manifest);

        let err = verify_manifest_at(dir.path(), "agent-guidance-manifest.toml").unwrap_err();
        assert!(err.to_string().contains("duplicate asset id"));
    }

    #[test]
    fn missing_active_file_fails() {
        let dir = valid_repo();
        fs::remove_file(dir.path().join("EXPERIENCE.md")).expect("remove experience");

        let err = verify_manifest_at(dir.path(), "agent-guidance-manifest.toml").unwrap_err();
        assert!(err.to_string().contains("missing active asset file"));
    }

    #[test]
    fn invalid_type_fails() {
        let dir = valid_repo();
        let manifest = valid_manifest().replace("type = \"experience\"", "type = \"unknown_type\"");
        write_file(dir.path(), "agent-guidance-manifest.toml", &manifest);

        let err = verify_manifest_at(dir.path(), "agent-guidance-manifest.toml").unwrap_err();
        assert!(err.to_string().contains("invalid asset type"));
    }

    #[test]
    fn path_escape_fails() {
        let dir = valid_repo();
        let manifest = valid_manifest().replace("EXPERIENCE.md", "../EXPERIENCE.md");
        write_file(dir.path(), "agent-guidance-manifest.toml", &manifest);

        let err = verify_manifest_at(dir.path(), "agent-guidance-manifest.toml").unwrap_err();
        assert!(err.to_string().contains("escapes repository root"));
    }

    #[test]
    fn empty_summary_fails() {
        let dir = valid_repo();
        let manifest =
            valid_manifest().replace("Project-level reusable lessons for agents.", "   ");
        write_file(dir.path(), "agent-guidance-manifest.toml", &manifest);

        let err = verify_manifest_at(dir.path(), "agent-guidance-manifest.toml").unwrap_err();
        assert!(err.to_string().contains("empty summary"));
    }

    #[test]
    fn missing_agents_index_reference_fails() {
        let dir = valid_repo();
        write_file(dir.path(), "AGENTS.md", "## Project Knowledge Index\n");

        let err = verify_manifest_at(dir.path(), "agent-guidance-manifest.toml").unwrap_err();
        assert!(
            err.to_string()
                .contains("missing AGENTS.md index reference")
        );
    }

    fn valid_skill_manifest() -> &'static str {
        r#"
schema_version = 1

[[assets]]
id = "skill-code-assist"
type = "skill"
path = ".agents/skills/code-assist/SKILL.md"
status = "active"
summary = "Code assist workflow."
required_in_agents_index = false
"#
    }

    fn valid_skill_file(name: &str, description: &str) -> String {
        format!("---\nname: {name}\ndescription: {description}\n---\n\n# Skill\n")
    }

    fn valid_skill_repo() -> TempDir {
        let dir = TempDir::new().expect("temp dir");
        write_file(dir.path(), "AGENTS.md", "# AGENTS\n");
        write_file(
            dir.path(),
            ".agents/skills/code-assist/SKILL.md",
            &valid_skill_file("code-assist", "Code assist workflow."),
        );
        write_file(
            dir.path(),
            "agent-guidance-manifest.toml",
            valid_skill_manifest(),
        );
        dir
    }

    #[test]
    fn skill_manifest_report_counts_assets_and_skills() {
        let dir = valid_skill_repo();
        let report = verify_manifest_at_with_report(dir.path(), "agent-guidance-manifest.toml")
            .expect("valid skill manifest should pass");

        assert_eq!(report.manifest_path, "agent-guidance-manifest.toml");
        assert_eq!(report.asset_count, 1);
        assert_eq!(report.skill_count, 1);
    }

    #[test]
    fn skill_outside_allowed_roots_fails() {
        let dir = valid_skill_repo();
        write_file(
            dir.path(),
            "docs/code-assist/SKILL.md",
            &valid_skill_file("code-assist", "Code assist workflow."),
        );
        let manifest = valid_skill_manifest().replace(
            ".agents/skills/code-assist/SKILL.md",
            "docs/code-assist/SKILL.md",
        );
        write_file(dir.path(), "agent-guidance-manifest.toml", &manifest);

        let err = verify_manifest_at(dir.path(), "agent-guidance-manifest.toml").unwrap_err();
        assert!(err.to_string().contains("skill path must be under"));
    }

    #[test]
    fn skill_missing_frontmatter_name_fails() {
        let dir = valid_skill_repo();
        write_file(
            dir.path(),
            ".agents/skills/code-assist/SKILL.md",
            "---\ndescription: Code assist workflow.\n---\n\n# Skill\n",
        );

        let err = verify_manifest_at(dir.path(), "agent-guidance-manifest.toml").unwrap_err();
        assert!(
            err.to_string()
                .contains("missing skill frontmatter field `name`")
        );
    }

    #[test]
    fn skill_missing_frontmatter_description_fails() {
        let dir = valid_skill_repo();
        write_file(
            dir.path(),
            ".agents/skills/code-assist/SKILL.md",
            "---\nname: code-assist\n---\n\n# Skill\n",
        );

        let err = verify_manifest_at(dir.path(), "agent-guidance-manifest.toml").unwrap_err();
        assert!(
            err.to_string()
                .contains("missing skill frontmatter field `description`")
        );
    }

    #[test]
    fn duplicate_skill_names_fail() {
        let dir = valid_skill_repo();
        write_file(
            dir.path(),
            ".codex/skills/code-assist-copy/SKILL.md",
            &valid_skill_file("code-assist", "Duplicate name."),
        );
        let manifest = format!(
            "{}\n\n[[assets]]\nid = \"skill-code-assist-copy\"\ntype = \"skill\"\npath = \".codex/skills/code-assist-copy/SKILL.md\"\nstatus = \"active\"\nsummary = \"Duplicate code assist workflow.\"\nrequired_in_agents_index = false\n",
            valid_skill_manifest()
        );
        write_file(dir.path(), "agent-guidance-manifest.toml", &manifest);

        let err = verify_manifest_at(dir.path(), "agent-guidance-manifest.toml").unwrap_err();
        assert!(
            err.to_string()
                .contains("duplicate skill name `code-assist`")
        );
    }

    #[test]
    fn repository_manifest_passes() {
        let crate_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let repo_root = crate_dir
            .parent()
            .and_then(std::path::Path::parent)
            .expect("ralph-core crate should live under crates/ralph-core");

        verify_default_manifest(repo_root).unwrap();
    }
}
