//! Scoped experience governance helpers.
//!
//! 这层负责两件事:
//! - 统一维护 topic / role / project 三层共享知识的 canonical writer 规则
//! - 提供 handoff / inspection / topic group 探测这些“围绕写入”的基础能力
//!
//! 目前 runtime 里的 prompt 注入已经能读取 scoped experience。
//! 这里补的是“谁有资格落笔”和“如何交接”的协议层,避免后续把权限判断散落到
//! event loop、CLI、workflow runtime 等多个位置。

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::config::CoreConfig;

/// Default canonical writer id for project scope and fallback ownership.
pub const DEFAULT_CANONICAL_WRITER_ID: &str = "ralph#1";

const TOPIC_FILE_PATTERNS: [(TopicContextFileKind, &str); 3] = [
    (TopicContextFileKind::TaskPlan, "task_plan__"),
    (TopicContextFileKind::Notes, "notes__"),
    (TopicContextFileKind::Worklog, "WORKLOG__"),
];

/// Topic shared context file kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TopicContextFileKind {
    /// 话题计划文件。
    TaskPlan,
    /// 话题研究笔记文件。
    Notes,
    /// 话题阶段工作记录文件。
    Worklog,
}

impl TopicContextFileKind {
    /// Human-readable label for prompt and debug output.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::TaskPlan => "Task Plan",
            Self::Notes => "Notes",
            Self::Worklog => "Worklog",
        }
    }

    /// File name for a topic suffix.
    #[must_use]
    pub fn file_name(self, suffix: &str) -> String {
        match self {
            Self::TaskPlan => format!("task_plan__{suffix}.md"),
            Self::Notes => format!("notes__{suffix}.md"),
            Self::Worklog => format!("WORKLOG__{suffix}.md"),
        }
    }
}

/// One topic shared context file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopicContextFile {
    /// Which shared file this is.
    pub kind: TopicContextFileKind,
    /// Resolved file path.
    pub path: PathBuf,
}

/// Group of topic files that share the same suffix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopicContextGroup {
    /// Shared suffix, e.g. `memory_axes`.
    pub suffix: String,
    /// Files found for that suffix.
    pub files: Vec<TopicContextFile>,
}

/// Discover all topic groups under the workspace root.
#[must_use]
pub fn detect_topic_groups(root: &Path) -> Vec<TopicContextGroup> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };

    let mut groups: BTreeMap<String, Vec<TopicContextFile>> = BTreeMap::new();

    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        for (kind, prefix) in TOPIC_FILE_PATTERNS {
            let Some(suffix) = file_name
                .strip_prefix(prefix)
                .and_then(|rest| rest.strip_suffix(".md"))
            else {
                continue;
            };

            groups
                .entry(suffix.to_string())
                .or_default()
                .push(TopicContextFile {
                    kind,
                    path: path.clone(),
                });
        }
    }

    let mut groups = groups
        .into_iter()
        .map(|(suffix, mut files)| {
            files.sort_by_key(|file| file.kind);
            TopicContextGroup { suffix, files }
        })
        .collect::<Vec<_>>();
    groups.sort_by(|left, right| left.suffix.cmp(&right.suffix));
    groups
}

/// Discover the unique topic group if and only if there is exactly one.
#[must_use]
pub fn detect_unique_topic_group(root: &Path) -> Option<TopicContextGroup> {
    let mut groups = detect_topic_groups(root);
    if groups.len() == 1 {
        groups.pop()
    } else {
        None
    }
}

/// Shared knowledge scopes that need writer governance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "scope", rename_all = "snake_case")]
pub enum SharedKnowledgeScope {
    /// Topic shared files keyed by suffix.
    Topic { suffix: String },
    /// Role experience keyed by hat id.
    Role { hat_id: String },
    /// Project-level reusable experience.
    Project,
}

impl SharedKnowledgeScope {
    #[must_use]
    pub fn display_name(&self) -> String {
        match self {
            Self::Topic { suffix } => format!("topic:{suffix}"),
            Self::Role { hat_id } => format!("role:{hat_id}"),
            Self::Project => "project".to_string(),
        }
    }
}

/// Where the current writer assignment came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WriterOwnerSource {
    /// 显式 owner / temporary owner hint。
    ExplicitHint,
    /// 没有 owner 时由 `ralph#1` 兜底。
    RalphFallback,
    /// 项目级 experience 的默认 owner。
    ProjectDefault,
}

/// Resumable handoff summary between canonical writers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriterHandoffSummary {
    /// 当前已收敛的官方结论。
    pub current_conclusion: String,
    /// 还未完成的事项。
    pub unfinished_work: Vec<String>,
    /// 关键证据来源,可以是文件路径、事件 topic、instance id 等。
    pub evidence_sources: Vec<String>,
    /// 本次交接的原因。
    pub transfer_reason: String,
    /// 交出方。
    pub from_writer: String,
    /// 接手方。
    pub to_writer: String,
    /// 交接时间。
    pub created_at: String,
}

impl WriterHandoffSummary {
    /// Creates a new handoff summary with current timestamp.
    #[must_use]
    pub fn new(
        from_writer: impl Into<String>,
        to_writer: impl Into<String>,
        current_conclusion: impl Into<String>,
        unfinished_work: Vec<String>,
        evidence_sources: Vec<String>,
        transfer_reason: impl Into<String>,
    ) -> Self {
        Self {
            current_conclusion: current_conclusion.into(),
            unfinished_work,
            evidence_sources,
            transfer_reason: transfer_reason.into(),
            from_writer: from_writer.into(),
            to_writer: to_writer.into(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Formats the handoff summary as append-only markdown.
    #[must_use]
    pub fn to_markdown_block(&self, title: &str) -> String {
        let unfinished_work = if self.unfinished_work.is_empty() {
            "- none".to_string()
        } else {
            self.unfinished_work
                .iter()
                .map(|item| format!("- {item}"))
                .collect::<Vec<_>>()
                .join("\n")
        };

        let evidence_sources = if self.evidence_sources.is_empty() {
            "- none".to_string()
        } else {
            self.evidence_sources
                .iter()
                .map(|item| format!("- {item}"))
                .collect::<Vec<_>>()
                .join("\n")
        };

        format!(
            "## {title} Handoff Summary\n\n- from: {}\n- to: {}\n- created_at: {}\n- reason: {}\n\n### Current Conclusion\n{}\n\n### Unfinished Work\n{}\n\n### Evidence Sources\n{}\n",
            self.from_writer,
            self.to_writer,
            self.created_at,
            self.transfer_reason,
            self.current_conclusion,
            unfinished_work,
            evidence_sources,
        )
    }
}

/// Persisted canonical writer assignment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalWriterRecord {
    /// Which shared scope this record protects.
    pub scope: SharedKnowledgeScope,
    /// Active writer.
    pub owner: String,
    /// Why this owner is active.
    pub owner_source: WriterOwnerSource,
    /// Last update timestamp.
    pub updated_at: String,
    /// Previous writer if a transfer happened.
    pub previous_owner: Option<String>,
    /// Most recent handoff summary, if any.
    pub last_handoff: Option<WriterHandoffSummary>,
}

impl CanonicalWriterRecord {
    #[must_use]
    fn new(scope: SharedKnowledgeScope, owner: String, owner_source: WriterOwnerSource) -> Self {
        Self {
            scope,
            owner,
            owner_source,
            updated_at: chrono::Utc::now().to_rfc3339(),
            previous_owner: None,
            last_handoff: None,
        }
    }
}

/// Inspection payload for debug / doctor visibility.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopedExperienceInspection {
    /// Legacy migration path.
    pub legacy_memories_path: PathBuf,
    /// Project-level experience path.
    pub project_experience_path: PathBuf,
    /// Role experience root.
    pub role_experience_root: PathBuf,
    /// Canonical writer metadata root.
    pub writer_root: PathBuf,
    /// Project writer record or default.
    pub project_writer: CanonicalWriterRecord,
    /// Role writer records or defaults.
    pub role_writers: Vec<CanonicalWriterRecord>,
    /// Topic writer records or defaults.
    pub topic_writers: Vec<CanonicalWriterRecord>,
}

/// Canonical writer governance failures.
#[derive(Debug, Error)]
pub enum WriterGovernanceError {
    /// Non-owner attempted to write a shared scope.
    #[error("Actor '{actor}' is not the canonical writer for {scope}; active owner is '{owner}'")]
    Unauthorized {
        actor: String,
        scope: String,
        owner: String,
    },
    /// Handoff summary is required for ownership transfer.
    #[error("Writer handoff for {scope} from '{from}' to '{to}' requires a handoff summary")]
    MissingHandoffSummary {
        scope: String,
        from: String,
        to: String,
    },
    /// Handoff summary does not match the requested transfer.
    #[error(
        "Writer handoff for {scope} has mismatched summary; expected {expected_from} -> {expected_to}, got {got_from} -> {got_to}"
    )]
    InvalidHandoffSummary {
        scope: String,
        expected_from: String,
        expected_to: String,
        got_from: String,
        got_to: String,
    },
    /// IO failure while reading or writing governance state.
    #[error(transparent)]
    Io(#[from] io::Error),
    /// Serialization failure while persisting governance state.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// Workspace-local store for canonical writer state.
#[derive(Debug, Clone)]
pub struct CanonicalWriterStore {
    core: CoreConfig,
}

impl CanonicalWriterStore {
    /// Creates a new writer store.
    #[must_use]
    pub fn new(core: &CoreConfig) -> Self {
        Self { core: core.clone() }
    }

    /// Root directory for persisted writer ownership metadata.
    #[must_use]
    pub fn writer_root(&self) -> PathBuf {
        self.core.resolve_path(".ralph/canonical-writers")
    }

    /// Returns a read-only inspection of resolved paths and current writer ownership.
    pub fn inspect(
        &self,
        hat_ids: impl IntoIterator<Item = String>,
    ) -> Result<ScopedExperienceInspection, WriterGovernanceError> {
        let mut role_writers = hat_ids
            .into_iter()
            .map(|hat_id| self.peek_role_writer(&hat_id, None))
            .collect::<Result<Vec<_>, _>>()?;
        role_writers.sort_by(|left, right| {
            left.scope
                .display_name()
                .cmp(&right.scope.display_name())
                .then_with(|| left.owner.cmp(&right.owner))
        });

        let mut topic_writers = detect_topic_groups(&self.core.workspace_root)
            .into_iter()
            .map(|group| self.peek_topic_writer(&group.suffix, None))
            .collect::<Result<Vec<_>, _>>()?;
        topic_writers.sort_by_key(|writer| writer.scope.display_name());

        Ok(ScopedExperienceInspection {
            legacy_memories_path: self.core.resolve_legacy_memories_path(),
            project_experience_path: self.core.resolve_project_experience_path(),
            role_experience_root: self.core.resolve_path(".ralph/roles"),
            writer_root: self.writer_root(),
            project_writer: self.peek_project_writer()?,
            role_writers,
            topic_writers,
        })
    }

    /// Returns the topic writer record, creating the default record if needed.
    pub fn resolve_topic_writer(
        &self,
        suffix: &str,
        owner_hint: Option<&str>,
    ) -> Result<CanonicalWriterRecord, WriterGovernanceError> {
        self.resolve_scope(
            SharedKnowledgeScope::Topic {
                suffix: suffix.to_string(),
            },
            owner_hint,
        )
    }

    /// Returns the role writer record, creating the default record if needed.
    pub fn resolve_role_writer(
        &self,
        hat_id: &str,
        primary_owner_hint: Option<&str>,
    ) -> Result<CanonicalWriterRecord, WriterGovernanceError> {
        self.resolve_scope(
            SharedKnowledgeScope::Role {
                hat_id: hat_id.to_string(),
            },
            primary_owner_hint,
        )
    }

    /// Returns the project writer record, creating the default record if needed.
    pub fn resolve_project_writer(&self) -> Result<CanonicalWriterRecord, WriterGovernanceError> {
        self.resolve_scope(SharedKnowledgeScope::Project, None)
    }

    /// Returns the topic writer record without mutating the filesystem.
    pub fn peek_topic_writer(
        &self,
        suffix: &str,
        owner_hint: Option<&str>,
    ) -> Result<CanonicalWriterRecord, WriterGovernanceError> {
        self.peek_scope(
            SharedKnowledgeScope::Topic {
                suffix: suffix.to_string(),
            },
            owner_hint,
        )
    }

    /// Returns the role writer record without mutating the filesystem.
    pub fn peek_role_writer(
        &self,
        hat_id: &str,
        primary_owner_hint: Option<&str>,
    ) -> Result<CanonicalWriterRecord, WriterGovernanceError> {
        self.peek_scope(
            SharedKnowledgeScope::Role {
                hat_id: hat_id.to_string(),
            },
            primary_owner_hint,
        )
    }

    /// Returns the project writer record without mutating the filesystem.
    pub fn peek_project_writer(&self) -> Result<CanonicalWriterRecord, WriterGovernanceError> {
        self.peek_scope(SharedKnowledgeScope::Project, None)
    }

    /// Authorizes a topic shared-file write.
    pub fn authorize_topic_write(
        &self,
        suffix: &str,
        actor: &str,
        owner_hint: Option<&str>,
    ) -> Result<CanonicalWriterRecord, WriterGovernanceError> {
        self.authorize_scope(
            SharedKnowledgeScope::Topic {
                suffix: suffix.to_string(),
            },
            actor,
            owner_hint,
        )
    }

    /// Authorizes a role experience write.
    pub fn authorize_role_write(
        &self,
        hat_id: &str,
        actor: &str,
        primary_owner_hint: Option<&str>,
    ) -> Result<CanonicalWriterRecord, WriterGovernanceError> {
        self.authorize_scope(
            SharedKnowledgeScope::Role {
                hat_id: hat_id.to_string(),
            },
            actor,
            primary_owner_hint,
        )
    }

    /// Authorizes a project experience write.
    pub fn authorize_project_write(
        &self,
        actor: &str,
    ) -> Result<CanonicalWriterRecord, WriterGovernanceError> {
        self.authorize_scope(SharedKnowledgeScope::Project, actor, None)
    }

    /// Appends an authorized update to a topic shared file.
    pub fn append_topic_shared_update(
        &self,
        suffix: &str,
        kind: TopicContextFileKind,
        actor: &str,
        owner_hint: Option<&str>,
        markdown: &str,
    ) -> Result<PathBuf, WriterGovernanceError> {
        self.authorize_topic_write(suffix, actor, owner_hint)?;
        let path = self.core.workspace_root.join(kind.file_name(suffix));
        append_markdown_block(&path, markdown)?;
        Ok(path)
    }

    /// Transfers topic writer ownership and records a resumable handoff summary.
    pub fn transfer_topic_writer(
        &self,
        suffix: &str,
        actor: &str,
        new_owner: &str,
        owner_hint: Option<&str>,
        handoff: Option<WriterHandoffSummary>,
    ) -> Result<CanonicalWriterRecord, WriterGovernanceError> {
        let current = self.authorize_topic_write(suffix, actor, owner_hint)?;
        self.transfer_scope(
            current,
            new_owner,
            handoff,
            Some(
                self.core
                    .workspace_root
                    .join(format!("WORKLOG__{suffix}.md")),
            ),
            "Topic Writer",
        )
    }

    /// Transfers role writer ownership and records a resumable handoff summary.
    pub fn transfer_role_writer(
        &self,
        hat_id: &str,
        actor: &str,
        new_owner: &str,
        primary_owner_hint: Option<&str>,
        handoff: Option<WriterHandoffSummary>,
    ) -> Result<CanonicalWriterRecord, WriterGovernanceError> {
        let current = self.authorize_role_write(hat_id, actor, primary_owner_hint)?;
        self.transfer_scope(
            current,
            new_owner,
            handoff,
            Some(self.core.resolve_role_dir(hat_id).join("handoff.md")),
            "Role Writer",
        )
    }

    /// Returns the last handoff summary for a scope if present.
    pub fn latest_handoff(
        &self,
        scope: &SharedKnowledgeScope,
    ) -> Result<Option<WriterHandoffSummary>, WriterGovernanceError> {
        Ok(self.load(scope)?.and_then(|record| record.last_handoff))
    }

    fn authorize_scope(
        &self,
        scope: SharedKnowledgeScope,
        actor: &str,
        owner_hint: Option<&str>,
    ) -> Result<CanonicalWriterRecord, WriterGovernanceError> {
        let record = self.resolve_scope(scope.clone(), owner_hint)?;
        if record.owner == actor {
            Ok(record)
        } else {
            Err(WriterGovernanceError::Unauthorized {
                actor: actor.to_string(),
                scope: scope.display_name(),
                owner: record.owner,
            })
        }
    }

    fn resolve_scope(
        &self,
        scope: SharedKnowledgeScope,
        owner_hint: Option<&str>,
    ) -> Result<CanonicalWriterRecord, WriterGovernanceError> {
        if let Some(record) = self.load(&scope)? {
            return Ok(record);
        }

        let record = self.default_record(&scope, owner_hint);
        self.save(&record)?;
        Ok(record)
    }

    fn peek_scope(
        &self,
        scope: SharedKnowledgeScope,
        owner_hint: Option<&str>,
    ) -> Result<CanonicalWriterRecord, WriterGovernanceError> {
        Ok(self
            .load(&scope)?
            .unwrap_or_else(|| self.default_record(&scope, owner_hint)))
    }

    fn default_record(
        &self,
        scope: &SharedKnowledgeScope,
        owner_hint: Option<&str>,
    ) -> CanonicalWriterRecord {
        match scope {
            SharedKnowledgeScope::Project => CanonicalWriterRecord::new(
                SharedKnowledgeScope::Project,
                DEFAULT_CANONICAL_WRITER_ID.to_string(),
                WriterOwnerSource::ProjectDefault,
            ),
            SharedKnowledgeScope::Topic { suffix } => CanonicalWriterRecord::new(
                SharedKnowledgeScope::Topic {
                    suffix: suffix.clone(),
                },
                owner_hint
                    .filter(|hint| !hint.trim().is_empty())
                    .unwrap_or(DEFAULT_CANONICAL_WRITER_ID)
                    .to_string(),
                owner_source_for_hint(owner_hint),
            ),
            SharedKnowledgeScope::Role { hat_id } => CanonicalWriterRecord::new(
                SharedKnowledgeScope::Role {
                    hat_id: hat_id.clone(),
                },
                owner_hint
                    .filter(|hint| !hint.trim().is_empty())
                    .unwrap_or(DEFAULT_CANONICAL_WRITER_ID)
                    .to_string(),
                owner_source_for_hint(owner_hint),
            ),
        }
    }

    fn load(
        &self,
        scope: &SharedKnowledgeScope,
    ) -> Result<Option<CanonicalWriterRecord>, WriterGovernanceError> {
        let path = self.record_path(scope);
        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(path)?;
        Ok(Some(serde_json::from_str(&content)?))
    }

    fn save(&self, record: &CanonicalWriterRecord) -> Result<(), WriterGovernanceError> {
        let path = self.record_path(&record.scope);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(record)?;
        fs::write(path, format!("{content}\n"))?;
        Ok(())
    }

    fn record_path(&self, scope: &SharedKnowledgeScope) -> PathBuf {
        let root = self.writer_root();
        match scope {
            SharedKnowledgeScope::Topic { suffix } => {
                root.join("topics").join(format!("{suffix}.json"))
            }
            SharedKnowledgeScope::Role { hat_id } => {
                root.join("roles").join(format!("{hat_id}.json"))
            }
            SharedKnowledgeScope::Project => root.join("project.json"),
        }
    }

    fn transfer_scope(
        &self,
        current: CanonicalWriterRecord,
        new_owner: &str,
        handoff: Option<WriterHandoffSummary>,
        append_path: Option<PathBuf>,
        handoff_title: &str,
    ) -> Result<CanonicalWriterRecord, WriterGovernanceError> {
        if current.owner == new_owner {
            return Ok(current);
        }

        let Some(handoff) = handoff else {
            return Err(WriterGovernanceError::MissingHandoffSummary {
                scope: current.scope.display_name(),
                from: current.owner,
                to: new_owner.to_string(),
            });
        };

        if handoff.from_writer != current.owner || handoff.to_writer != new_owner {
            return Err(WriterGovernanceError::InvalidHandoffSummary {
                scope: current.scope.display_name(),
                expected_from: current.owner,
                expected_to: new_owner.to_string(),
                got_from: handoff.from_writer,
                got_to: handoff.to_writer,
            });
        }

        if let Some(path) = append_path {
            ensure_markdown_container(&path)?;
            append_markdown_block(&path, &handoff.to_markdown_block(handoff_title))?;
        }

        let updated = CanonicalWriterRecord {
            scope: current.scope.clone(),
            owner: new_owner.to_string(),
            owner_source: owner_source_for_actor(&current.scope, new_owner),
            updated_at: chrono::Utc::now().to_rfc3339(),
            previous_owner: Some(current.owner),
            last_handoff: Some(handoff),
        };
        self.save(&updated)?;
        Ok(updated)
    }
}

fn owner_source_for_hint(owner_hint: Option<&str>) -> WriterOwnerSource {
    if owner_hint.is_some_and(|hint| !hint.trim().is_empty()) {
        WriterOwnerSource::ExplicitHint
    } else {
        WriterOwnerSource::RalphFallback
    }
}

fn owner_source_for_actor(scope: &SharedKnowledgeScope, actor: &str) -> WriterOwnerSource {
    match scope {
        SharedKnowledgeScope::Project => WriterOwnerSource::ProjectDefault,
        SharedKnowledgeScope::Topic { .. } | SharedKnowledgeScope::Role { .. } => {
            if actor == DEFAULT_CANONICAL_WRITER_ID {
                WriterOwnerSource::RalphFallback
            } else {
                WriterOwnerSource::ExplicitHint
            }
        }
    }
}

fn ensure_markdown_container(path: &Path) -> io::Result<()> {
    if path.exists() {
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let initial = if path.file_name().and_then(|name| name.to_str()) == Some("experience.md") {
        "# Experience\n".to_string()
    } else {
        String::new()
    };

    fs::write(path, initial)
}

fn append_markdown_block(path: &Path, markdown: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let existing = fs::read_to_string(path).unwrap_or_default();
    let block = markdown.trim();
    if block.is_empty() {
        return Ok(());
    }

    let content = if existing.trim().is_empty() {
        format!("{block}\n")
    } else {
        format!("{}\n\n{block}\n", existing.trim_end())
    };

    fs::write(path, content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn core_config(root: &TempDir) -> CoreConfig {
        CoreConfig::default().with_workspace_root(root.path())
    }

    #[test]
    fn detect_topic_groups_orders_files_by_kind() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("notes__alpha.md"), "notes").unwrap();
        fs::write(temp_dir.path().join("WORKLOG__alpha.md"), "worklog").unwrap();
        fs::write(temp_dir.path().join("task_plan__alpha.md"), "plan").unwrap();

        let groups = detect_topic_groups(temp_dir.path());
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].suffix, "alpha");
        assert_eq!(
            groups[0]
                .files
                .iter()
                .map(|file| file.kind)
                .collect::<Vec<_>>(),
            vec![
                TopicContextFileKind::TaskPlan,
                TopicContextFileKind::Notes,
                TopicContextFileKind::Worklog
            ]
        );
        assert!(detect_unique_topic_group(temp_dir.path()).is_some());
    }

    #[test]
    fn non_owner_cannot_write_topic_shared_files() {
        let temp_dir = TempDir::new().unwrap();
        let store = CanonicalWriterStore::new(&core_config(&temp_dir));

        let err = store
            .append_topic_shared_update(
                "alpha",
                TopicContextFileKind::Notes,
                "spec_reviewer",
                Some("cab_program_lead"),
                "## Notes\n- reviewer wants to write directly\n",
            )
            .unwrap_err();

        assert!(matches!(err, WriterGovernanceError::Unauthorized { .. }));
    }

    #[test]
    fn role_writer_falls_back_to_ralph_when_no_primary_owner_exists() {
        let temp_dir = TempDir::new().unwrap();
        let store = CanonicalWriterStore::new(&core_config(&temp_dir));

        let ok = store.authorize_role_write("spec_reviewer", "ralph#1", None);
        assert!(
            ok.is_ok(),
            "ralph fallback should own unassigned role experience"
        );

        let err = store
            .authorize_role_write("spec_reviewer", "spec_reviewer", None)
            .unwrap_err();
        assert!(matches!(err, WriterGovernanceError::Unauthorized { .. }));
    }

    #[test]
    fn project_writer_defaults_to_ralph_only() {
        let temp_dir = TempDir::new().unwrap();
        let store = CanonicalWriterStore::new(&core_config(&temp_dir));

        assert!(store.authorize_project_write("ralph#1").is_ok());
        let err = store.authorize_project_write("builder").unwrap_err();
        assert!(matches!(err, WriterGovernanceError::Unauthorized { .. }));
    }

    #[test]
    fn topic_writer_transfer_requires_handoff_summary() {
        let temp_dir = TempDir::new().unwrap();
        let store = CanonicalWriterStore::new(&core_config(&temp_dir));

        let err = store
            .transfer_topic_writer("alpha", "ralph#1", "cab_program_lead", None, None)
            .unwrap_err();

        assert!(matches!(
            err,
            WriterGovernanceError::MissingHandoffSummary { .. }
        ));
    }

    #[test]
    fn topic_writer_transfer_persists_handoff_and_new_owner() {
        let temp_dir = TempDir::new().unwrap();
        let store = CanonicalWriterStore::new(&core_config(&temp_dir));
        let handoff = WriterHandoffSummary::new(
            "ralph#1",
            "cab_program_lead",
            "Agenda, host, and logistics are ready for owner takeover.",
            vec!["Confirm final packet".to_string()],
            vec!["WORKLOG__alpha.md".to_string(), "ralph#1".to_string()],
            "Workflow owner became explicit",
        );

        let record = store
            .transfer_topic_writer("alpha", "ralph#1", "cab_program_lead", None, Some(handoff))
            .unwrap();

        assert_eq!(record.owner, "cab_program_lead");
        assert_eq!(record.previous_owner.as_deref(), Some("ralph#1"));

        let latest = store
            .latest_handoff(&SharedKnowledgeScope::Topic {
                suffix: "alpha".to_string(),
            })
            .unwrap()
            .expect("handoff should be saved");
        assert_eq!(latest.to_writer, "cab_program_lead");

        let worklog = fs::read_to_string(temp_dir.path().join("WORKLOG__alpha.md")).unwrap();
        assert!(worklog.contains("Topic Writer Handoff Summary"));
        assert!(worklog.contains("Workflow owner became explicit"));
    }

    #[test]
    fn role_writer_transfer_appends_handoff_to_role_experience_file() {
        let temp_dir = TempDir::new().unwrap();
        let store = CanonicalWriterStore::new(&core_config(&temp_dir));
        let handoff = WriterHandoffSummary::new(
            "ralph#1",
            "spec_reviewer",
            "Requirement wording rules are stable enough for the role owner.",
            vec!["Promote two candidate rules".to_string()],
            vec!["topic:scoped-experience".to_string()],
            "Primary owner became available",
        );

        let record = store
            .transfer_role_writer(
                "spec_reviewer",
                "ralph#1",
                "spec_reviewer",
                None,
                Some(handoff),
            )
            .unwrap();

        assert_eq!(record.owner, "spec_reviewer");
        let role_path = temp_dir
            .path()
            .join(".ralph/roles/spec_reviewer/handoff.md");
        let content = fs::read_to_string(role_path).unwrap();
        assert!(content.contains("Role Writer Handoff Summary"));
    }

    #[test]
    fn inspection_reports_paths_and_default_owners_without_writing_files() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("task_plan__alpha.md"), "## Active\n").unwrap();
        let store = CanonicalWriterStore::new(&core_config(&temp_dir));

        let inspection = store
            .inspect(vec!["spec_reviewer".to_string(), "builder".to_string()])
            .unwrap();

        assert!(
            inspection
                .project_experience_path
                .ends_with(Path::new("experience.md"))
        );
        assert_eq!(inspection.project_writer.owner, "ralph#1");
        assert_eq!(inspection.role_writers.len(), 2);
        assert_eq!(inspection.topic_writers.len(), 1);
        assert_eq!(inspection.topic_writers[0].owner, "ralph#1");

        assert!(
            !inspection.writer_root.join("project.json").exists(),
            "read-only inspection should not create writer files"
        );
    }
}
