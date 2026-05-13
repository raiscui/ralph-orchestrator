//! Promotion and demotion helpers for scoped experience.
//!
//! 这一层把规格里的“先窄后宽、失活而非硬删”落成可复用的服务:
//! - topic -> role / project 的晋升评估
//! - role -> project 的晋升评估
//! - project -> role / role -> topic 的降级与审计链路

use std::io;
use std::path::PathBuf;

use thiserror::Error;

use crate::config::CoreConfig;
use crate::experience::{ExperienceConfidence, ExperienceEntry, ExperienceScope, ExperienceStatus};
use crate::experience_governance::CanonicalWriterStore;
use crate::experience_store::MarkdownExperienceStore;

/// Why a rule may deserve project scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectPromotionReason {
    /// 多个角色都会复用。
    CrossRoleReuse,
    /// `ralph#1` 在路由前就应该知道。
    NeededBeforeRouting,
    /// 这是项目级协作约束。
    CollaborationConstraint,
}

/// Decision for a topic-derived candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromotionDecision {
    /// 留在 topic 层,不做长期经验晋升。
    StayInTopic { reason: String },
    /// 晋升到某个 role 的 experience。
    PromoteToRole {
        hat_id: String,
        reasons: Vec<String>,
    },
    /// 晋升到 project experience。
    PromoteToProject {
        reasons: Vec<ProjectPromotionReason>,
    },
}

/// Decision for a role-derived candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RolePromotionDecision {
    /// 仍然留在 role 范围。
    StayInRole { reason: String },
    /// 晋升到 project experience。
    PromoteToProject {
        reasons: Vec<ProjectPromotionReason>,
    },
}

/// Signals used to decide whether a topic finding should be promoted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopicPromotionSignals {
    /// 当前 topic suffix。
    pub topic_suffix: String,
    /// 待晋升摘要。
    pub summary: String,
    /// 来源 hats / instances。
    pub source_hats: Vec<String>,
    /// 附加来源 topics。
    pub source_topics: Vec<String>,
    /// 若这是岗位经验,它属于哪个 hat。
    pub role_hat_id: Option<String>,
    /// 是否已经证明这是稳定的 role 规律。
    pub stable_for_role: bool,
    /// 是否仍然只是 topic 局部状态。
    pub topic_local_only: bool,
    /// 是否已经证明跨角色复用。
    pub cross_role_reuse: bool,
    /// 是否属于 `ralph#1` 路由前就应知道的规则。
    pub needed_before_routing: bool,
    /// 是否属于项目级协作约束。
    pub collaboration_constraint: bool,
    /// 当前经验置信度。
    pub confidence: ExperienceConfidence,
}

/// Signals used to decide whether a role finding should be promoted further.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RolePromotionSignals {
    /// 该经验当前所属 role。
    pub role_hat_id: String,
    /// 待晋升摘要。
    pub summary: String,
    /// 来源 topic 列表。
    pub source_topics: Vec<String>,
    /// 来源 hats / instances。
    pub source_hats: Vec<String>,
    /// 是否已经证明跨角色复用。
    pub cross_role_reuse: bool,
    /// 是否属于 `ralph#1` 路由前就应知道的规则。
    pub needed_before_routing: bool,
    /// 是否属于项目级协作约束。
    pub collaboration_constraint: bool,
    /// 当前经验置信度。
    pub confidence: ExperienceConfidence,
}

/// Result of a successful promotion write.
#[derive(Debug, Clone)]
pub struct PromotionOutcome {
    /// The evaluated decision.
    pub decision: PromotionDecision,
    /// Persisted entry if a promotion happened.
    pub persisted_entry: Option<ExperienceEntry>,
    /// Destination path if a promotion happened.
    pub persisted_path: Option<PathBuf>,
}

/// Result of a successful role-to-project promotion write.
#[derive(Debug, Clone)]
pub struct RolePromotionOutcome {
    /// The evaluated decision.
    pub decision: RolePromotionDecision,
    /// Persisted entry if a promotion happened.
    pub persisted_entry: Option<ExperienceEntry>,
    /// Destination path if a promotion happened.
    pub persisted_path: Option<PathBuf>,
}

/// Result of a demotion flow.
#[derive(Debug, Clone)]
pub struct DemotionOutcome {
    /// Deprecated original entry after demotion.
    pub deprecated_entry: ExperienceEntry,
    /// Optional replacement entry when demoting into another experience file.
    pub replacement_entry: Option<ExperienceEntry>,
    /// Optional replacement reference for topic-local history.
    pub replacement_reference: Option<String>,
}

/// Promotion / demotion failures.
#[derive(Debug, Error)]
pub enum ScopedExperienceError {
    /// Writer governance rejection.
    #[error(transparent)]
    Governance(#[from] crate::experience_governance::WriterGovernanceError),
    /// IO errors from markdown stores.
    #[error(transparent)]
    Io(#[from] io::Error),
    /// Requested experience entry was not found.
    #[error("Experience entry not found: {0}")]
    EntryNotFound(String),
}

/// Workspace service that performs guarded promotion and demotion.
#[derive(Debug, Clone)]
pub struct ScopedExperienceService {
    core: CoreConfig,
    writers: CanonicalWriterStore,
}

impl ScopedExperienceService {
    /// Creates a new scoped experience service.
    #[must_use]
    pub fn new(core: &CoreConfig) -> Self {
        Self {
            core: core.clone(),
            writers: CanonicalWriterStore::new(core),
        }
    }

    /// Evaluates and, when allowed, persists a topic-derived promotion.
    pub fn promote_topic_candidate(
        &self,
        actor: &str,
        signals: &TopicPromotionSignals,
        role_primary_owner_hint: Option<&str>,
    ) -> Result<PromotionOutcome, ScopedExperienceError> {
        let decision = evaluate_topic_promotion(signals);

        match &decision {
            PromotionDecision::StayInTopic { .. } => Ok(PromotionOutcome {
                decision,
                persisted_entry: None,
                persisted_path: None,
            }),
            PromotionDecision::PromoteToRole { hat_id, .. } => {
                self.writers
                    .authorize_role_write(hat_id, actor, role_primary_owner_hint)?;

                let mut entry =
                    ExperienceEntry::new(ExperienceScope::Role, signals.summary.clone());
                entry.source_topics =
                    normalized_topic_sources(&signals.topic_suffix, &signals.source_topics);
                entry.source_hats = signals.source_hats.clone();
                entry.confidence = signals.confidence;

                let store =
                    MarkdownExperienceStore::new(self.core.resolve_role_experience_path(hat_id));
                store.append(&entry)?;

                Ok(PromotionOutcome {
                    decision,
                    persisted_entry: Some(entry),
                    persisted_path: Some(store.path().to_path_buf()),
                })
            }
            PromotionDecision::PromoteToProject { .. } => {
                self.writers.authorize_project_write(actor)?;

                let mut entry =
                    ExperienceEntry::new(ExperienceScope::Project, signals.summary.clone());
                entry.source_topics =
                    normalized_topic_sources(&signals.topic_suffix, &signals.source_topics);
                entry.source_hats = signals.source_hats.clone();
                entry.confidence = signals.confidence;

                let store =
                    MarkdownExperienceStore::new(self.core.resolve_project_experience_path());
                store.append(&entry)?;

                Ok(PromotionOutcome {
                    decision,
                    persisted_entry: Some(entry),
                    persisted_path: Some(store.path().to_path_buf()),
                })
            }
        }
    }

    /// Evaluates and, when allowed, persists a role-derived promotion to project scope.
    pub fn promote_role_candidate_to_project(
        &self,
        actor: &str,
        signals: &RolePromotionSignals,
    ) -> Result<RolePromotionOutcome, ScopedExperienceError> {
        let decision = evaluate_role_to_project_promotion(signals);

        match &decision {
            RolePromotionDecision::StayInRole { .. } => Ok(RolePromotionOutcome {
                decision,
                persisted_entry: None,
                persisted_path: None,
            }),
            RolePromotionDecision::PromoteToProject { .. } => {
                self.writers.authorize_project_write(actor)?;

                let mut entry =
                    ExperienceEntry::new(ExperienceScope::Project, signals.summary.clone());
                entry.source_topics = signals.source_topics.clone();
                entry.source_hats = signals.source_hats.clone();
                entry.confidence = signals.confidence;

                let store =
                    MarkdownExperienceStore::new(self.core.resolve_project_experience_path());
                store.append(&entry)?;

                Ok(RolePromotionOutcome {
                    decision,
                    persisted_entry: Some(entry),
                    persisted_path: Some(store.path().to_path_buf()),
                })
            }
        }
    }

    /// Demotes a project entry into a role-specific replacement while preserving audit links.
    pub fn demote_project_entry_to_role(
        &self,
        actor: &str,
        project_entry_id: &str,
        role_hat_id: &str,
        replacement_summary: &str,
        replacement_source_topics: Vec<String>,
        replacement_source_hats: Vec<String>,
        role_primary_owner_hint: Option<&str>,
        confidence: ExperienceConfidence,
    ) -> Result<DemotionOutcome, ScopedExperienceError> {
        self.writers.authorize_project_write(actor)?;
        self.writers
            .authorize_role_write(role_hat_id, actor, role_primary_owner_hint)?;

        let project_store =
            MarkdownExperienceStore::new(self.core.resolve_project_experience_path());
        let role_store =
            MarkdownExperienceStore::new(self.core.resolve_role_experience_path(role_hat_id));

        let mut project_entries = project_store.load()?;
        let entry_index = project_entries
            .iter()
            .position(|entry| entry.id == project_entry_id)
            .ok_or_else(|| ScopedExperienceError::EntryNotFound(project_entry_id.to_string()))?;
        let original_entry = project_entries[entry_index].clone();

        let mut replacement =
            ExperienceEntry::new(ExperienceScope::Role, replacement_summary.to_string());
        replacement.source_topics = if replacement_source_topics.is_empty() {
            original_entry.source_topics.clone()
        } else {
            replacement_source_topics
        };
        replacement.source_hats = if replacement_source_hats.is_empty() {
            original_entry.source_hats.clone()
        } else {
            replacement_source_hats
        };
        replacement.confidence = confidence;
        replacement.supersedes.push(original_entry.id.clone());

        {
            let entry = &mut project_entries[entry_index];
            entry.status = ExperienceStatus::Deprecated;
            entry.updated_at = chrono::Utc::now().to_rfc3339();
            push_unique(&mut entry.replaced_by, replacement.id.clone());
        }

        let deprecated_entry = project_entries[entry_index].clone();

        project_store.write_all(&project_entries)?;
        role_store.append(&replacement)?;

        Ok(DemotionOutcome {
            deprecated_entry,
            replacement_entry: Some(replacement),
            replacement_reference: None,
        })
    }

    /// Demotes a role entry back to topic-local history while preserving traceability.
    pub fn demote_role_entry_to_topic(
        &self,
        actor: &str,
        role_hat_id: &str,
        role_entry_id: &str,
        topic_suffix: &str,
        role_primary_owner_hint: Option<&str>,
    ) -> Result<DemotionOutcome, ScopedExperienceError> {
        self.writers
            .authorize_role_write(role_hat_id, actor, role_primary_owner_hint)?;

        let role_store =
            MarkdownExperienceStore::new(self.core.resolve_role_experience_path(role_hat_id));
        let mut role_entries = role_store.load()?;

        let entry_index = role_entries
            .iter()
            .position(|entry| entry.id == role_entry_id)
            .ok_or_else(|| ScopedExperienceError::EntryNotFound(role_entry_id.to_string()))?;

        let topic_reference = format!("topic:{topic_suffix}");
        {
            let entry = &mut role_entries[entry_index];
            entry.status = ExperienceStatus::Deprecated;
            entry.updated_at = chrono::Utc::now().to_rfc3339();
            push_unique(&mut entry.source_topics, topic_suffix.to_string());
            push_unique(&mut entry.replaced_by, topic_reference.clone());
        }

        let deprecated_entry = role_entries[entry_index].clone();

        role_store.write_all(&role_entries)?;

        Ok(DemotionOutcome {
            deprecated_entry,
            replacement_entry: None,
            replacement_reference: Some(topic_reference),
        })
    }
}

/// Evaluates whether a topic-derived finding should stay local, promote to role, or promote to project.
#[must_use]
pub fn evaluate_topic_promotion(signals: &TopicPromotionSignals) -> PromotionDecision {
    if signals.topic_local_only {
        return PromotionDecision::StayInTopic {
            reason: "The finding still looks topic-local, so it must stay in shared topic context."
                .to_string(),
        };
    }

    let project_reasons = collect_project_reasons(
        signals.cross_role_reuse,
        signals.needed_before_routing,
        signals.collaboration_constraint,
    );
    if !project_reasons.is_empty() {
        return PromotionDecision::PromoteToProject {
            reasons: project_reasons,
        };
    }

    if signals.stable_for_role {
        if let Some(role_hat_id) = &signals.role_hat_id {
            return PromotionDecision::PromoteToRole {
                hat_id: role_hat_id.clone(),
                reasons: vec![
                    "Stable role-specific reuse is justified.".to_string(),
                    "Project-wide value is not demonstrated yet, so narrower scope wins."
                        .to_string(),
                ],
            };
        }

        return PromotionDecision::StayInTopic {
            reason: "Role reuse looks plausible, but no target role is identified yet.".to_string(),
        };
    }

    PromotionDecision::StayInTopic {
        reason: "Broader reusable value is not demonstrated yet, so the safer default is to stay in topic context.".to_string(),
    }
}

/// Evaluates whether a role-derived finding should be promoted to project scope.
#[must_use]
pub fn evaluate_role_to_project_promotion(signals: &RolePromotionSignals) -> RolePromotionDecision {
    let project_reasons = collect_project_reasons(
        signals.cross_role_reuse,
        signals.needed_before_routing,
        signals.collaboration_constraint,
    );

    if project_reasons.is_empty() {
        RolePromotionDecision::StayInRole {
            reason: "The rule still looks role-local, so it should remain in role experience."
                .to_string(),
        }
    } else {
        RolePromotionDecision::PromoteToProject {
            reasons: project_reasons,
        }
    }
}

fn collect_project_reasons(
    cross_role_reuse: bool,
    needed_before_routing: bool,
    collaboration_constraint: bool,
) -> Vec<ProjectPromotionReason> {
    let mut reasons = Vec::new();

    if cross_role_reuse {
        reasons.push(ProjectPromotionReason::CrossRoleReuse);
    }
    if needed_before_routing {
        reasons.push(ProjectPromotionReason::NeededBeforeRouting);
    }
    if collaboration_constraint {
        reasons.push(ProjectPromotionReason::CollaborationConstraint);
    }

    reasons
}

fn normalized_topic_sources(topic_suffix: &str, extra_topics: &[String]) -> Vec<String> {
    let mut topics = extra_topics.to_vec();
    push_unique(&mut topics, topic_suffix.to_string());
    topics
}

fn push_unique(items: &mut Vec<String>, value: String) {
    if !items.iter().any(|existing| existing == &value) {
        items.push(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::experience_governance::WriterHandoffSummary;
    use tempfile::TempDir;

    fn core_config(root: &TempDir) -> CoreConfig {
        CoreConfig::default().with_workspace_root(root.path())
    }

    fn role_signals() -> TopicPromotionSignals {
        TopicPromotionSignals {
            topic_suffix: "memory_axes".to_string(),
            summary: "Spec reviewers should cite concrete evidence before rejecting.".to_string(),
            source_hats: vec!["spec_reviewer".to_string()],
            source_topics: vec![],
            role_hat_id: Some("spec_reviewer".to_string()),
            stable_for_role: true,
            topic_local_only: false,
            cross_role_reuse: false,
            needed_before_routing: false,
            collaboration_constraint: false,
            confidence: ExperienceConfidence::High,
        }
    }

    #[test]
    fn topic_local_finding_stays_in_topic() {
        let mut signals = role_signals();
        signals.topic_local_only = true;

        let decision = evaluate_topic_promotion(&signals);
        assert!(matches!(decision, PromotionDecision::StayInTopic { .. }));
    }

    #[test]
    fn topic_promotion_prefers_role_before_project_when_scope_is_uncertain() {
        let decision = evaluate_topic_promotion(&role_signals());
        assert!(matches!(
            decision,
            PromotionDecision::PromoteToRole { ref hat_id, .. } if hat_id == "spec_reviewer"
        ));
    }

    #[test]
    fn cross_role_topic_rule_promotes_to_project() {
        let mut signals = role_signals();
        signals.role_hat_id = None;
        signals.stable_for_role = false;
        signals.cross_role_reuse = true;

        let decision = evaluate_topic_promotion(&signals);
        assert!(matches!(
            decision,
            PromotionDecision::PromoteToProject { .. }
        ));
    }

    #[test]
    fn role_rule_stays_in_role_without_project_value() {
        let signals = RolePromotionSignals {
            role_hat_id: "spec_reviewer".to_string(),
            summary: "Reviewers prefer SHALL/MUST checks first.".to_string(),
            source_topics: vec!["memory_axes".to_string()],
            source_hats: vec!["spec_reviewer".to_string()],
            cross_role_reuse: false,
            needed_before_routing: false,
            collaboration_constraint: false,
            confidence: ExperienceConfidence::Medium,
        };

        let decision = evaluate_role_to_project_promotion(&signals);
        assert!(matches!(decision, RolePromotionDecision::StayInRole { .. }));
    }

    #[test]
    fn service_promotes_topic_rule_to_role_store() {
        let temp_dir = TempDir::new().unwrap();
        let core = core_config(&temp_dir);
        let writers = CanonicalWriterStore::new(&core);
        let handoff = WriterHandoffSummary::new(
            "ralph#1",
            "spec_reviewer",
            "Reviewer owner is now explicit.",
            vec!["Promote one role rule".to_string()],
            vec!["topic:memory_axes".to_string()],
            "Owner handoff",
        );
        writers
            .transfer_role_writer(
                "spec_reviewer",
                "ralph#1",
                "spec_reviewer",
                None,
                Some(handoff),
            )
            .unwrap();

        let service = ScopedExperienceService::new(&core);
        let outcome = service
            .promote_topic_candidate("spec_reviewer", &role_signals(), Some("spec_reviewer"))
            .unwrap();

        assert!(matches!(
            outcome.decision,
            PromotionDecision::PromoteToRole { .. }
        ));
        let persisted = outcome
            .persisted_entry
            .expect("role entry should be persisted");
        assert_eq!(persisted.scope, ExperienceScope::Role);

        let role_store =
            MarkdownExperienceStore::new(core.resolve_role_experience_path("spec_reviewer"));
        let entries = role_store.load().unwrap();
        assert!(entries.iter().any(
            |entry| entry.summary == persisted.summary && entry.scope == ExperienceScope::Role
        ));
    }

    #[test]
    fn service_promotes_cross_role_rule_to_project_store_only_for_ralph() {
        let temp_dir = TempDir::new().unwrap();
        let core = core_config(&temp_dir);
        let service = ScopedExperienceService::new(&core);

        let mut signals = role_signals();
        signals.role_hat_id = None;
        signals.stable_for_role = false;
        signals.cross_role_reuse = true;

        let err = service
            .promote_topic_candidate("spec_reviewer", &signals, None)
            .unwrap_err();
        assert!(matches!(err, ScopedExperienceError::Governance(_)));

        let outcome = service
            .promote_topic_candidate("ralph#1", &signals, None)
            .unwrap();
        assert!(matches!(
            outcome.decision,
            PromotionDecision::PromoteToProject { .. }
        ));

        let project_store = MarkdownExperienceStore::new(core.resolve_project_experience_path());
        let entries = project_store.load().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].scope, ExperienceScope::Project);
    }

    #[test]
    fn project_demotion_to_role_preserves_audit_links() {
        let temp_dir = TempDir::new().unwrap();
        let core = core_config(&temp_dir);
        let service = ScopedExperienceService::new(&core);

        let project_store = MarkdownExperienceStore::new(core.resolve_project_experience_path());
        let mut project_entry = ExperienceEntry::new(
            ExperienceScope::Project,
            "Only canonical writers may update shared topic files.",
        );
        project_entry.source_topics = vec!["memory_axes".to_string()];
        project_store.append(&project_entry).unwrap();

        let outcome = service
            .demote_project_entry_to_role(
                "ralph#1",
                &project_entry.id,
                "spec_reviewer",
                "Spec reviewers may update review heuristics only through the role writer.",
                vec!["memory_axes".to_string()],
                vec!["spec_reviewer".to_string()],
                None,
                ExperienceConfidence::High,
            )
            .unwrap();

        assert_eq!(
            outcome.deprecated_entry.status,
            ExperienceStatus::Deprecated
        );
        let replacement = outcome
            .replacement_entry
            .expect("replacement role entry should exist");
        assert!(
            outcome
                .deprecated_entry
                .replaced_by
                .contains(&replacement.id)
        );
        assert!(replacement.supersedes.contains(&project_entry.id));
    }

    #[test]
    fn role_demotion_to_topic_keeps_topic_reference() {
        let temp_dir = TempDir::new().unwrap();
        let core = core_config(&temp_dir);
        let writers = CanonicalWriterStore::new(&core);
        writers
            .transfer_role_writer(
                "spec_reviewer",
                "ralph#1",
                "spec_reviewer",
                None,
                Some(WriterHandoffSummary::new(
                    "ralph#1",
                    "spec_reviewer",
                    "Role owner ready.",
                    vec![],
                    vec!["topic:memory_axes".to_string()],
                    "Owner assignment",
                )),
            )
            .unwrap();

        let role_store =
            MarkdownExperienceStore::new(core.resolve_role_experience_path("spec_reviewer"));
        let role_entry = ExperienceEntry::new(
            ExperienceScope::Role,
            "Topic-local memory axes guidance should not remain in shared role experience.",
        );
        role_store.append(&role_entry).unwrap();

        let service = ScopedExperienceService::new(&core);
        let outcome = service
            .demote_role_entry_to_topic(
                "spec_reviewer",
                "spec_reviewer",
                &role_entry.id,
                "memory_axes",
                Some("spec_reviewer"),
            )
            .unwrap();

        assert_eq!(
            outcome.deprecated_entry.status,
            ExperienceStatus::Deprecated
        );
        assert_eq!(
            outcome.replacement_reference.as_deref(),
            Some("topic:memory_axes")
        );
        assert!(
            outcome
                .deprecated_entry
                .source_topics
                .contains(&"memory_axes".to_string())
        );
    }
}
