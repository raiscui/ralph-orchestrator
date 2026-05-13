//! Scoped experience types shared by role and project experience files.
//!
//! 这层只负责“稳定经验条目”的统一数据结构。
//! topic 文件和 instance 日志仍然保持各自语义,不会复用这里的 entry 结构。

use serde::{Deserialize, Serialize};

/// Reusable experience scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExperienceScope {
    /// 某个岗位长期适用的经验。
    Role,
    /// 整个项目范围都适用的经验。
    Project,
}

impl std::fmt::Display for ExperienceScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Role => write!(f, "role"),
            Self::Project => write!(f, "project"),
        }
    }
}

impl std::str::FromStr for ExperienceScope {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "role" => Ok(Self::Role),
            "project" => Ok(Self::Project),
            _ => Err(format!(
                "Invalid experience scope: '{s}'. Valid scopes: role, project"
            )),
        }
    }
}

/// Lifecycle state for an experience entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ExperienceStatus {
    /// 候选经验: 还未成为默认注入的稳定知识。
    Candidate,
    /// 生效经验: 可以参与默认注入。
    #[default]
    Active,
    /// 失活经验: 保留审计链路,但默认不再注入。
    Deprecated,
}

impl std::fmt::Display for ExperienceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Candidate => write!(f, "candidate"),
            Self::Active => write!(f, "active"),
            Self::Deprecated => write!(f, "deprecated"),
        }
    }
}

impl std::str::FromStr for ExperienceStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "candidate" => Ok(Self::Candidate),
            "active" => Ok(Self::Active),
            "deprecated" => Ok(Self::Deprecated),
            _ => Err(format!(
                "Invalid experience status: '{s}'. Valid statuses: candidate, active, deprecated"
            )),
        }
    }
}

/// Confidence level for a persisted experience.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ExperienceConfidence {
    Low,
    #[default]
    Medium,
    High,
}

impl std::fmt::Display for ExperienceConfidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => write!(f, "low"),
            Self::Medium => write!(f, "medium"),
            Self::High => write!(f, "high"),
        }
    }
}

impl std::str::FromStr for ExperienceConfidence {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            _ => Err(format!(
                "Invalid experience confidence: '{s}'. Valid confidence levels: low, medium, high"
            )),
        }
    }
}

/// A single reusable scoped experience entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExperienceEntry {
    /// 唯一 ID,便于 supersedes 和审计链路引用。
    pub id: String,
    /// 人类可读的稳定规则摘要。
    pub summary: String,
    /// 该经验属于 role 还是 project 作用域。
    pub scope: ExperienceScope,
    /// 来源 topic 列表。
    pub source_topics: Vec<String>,
    /// 来源 hats / instances 列表。
    pub source_hats: Vec<String>,
    /// 当前生命周期状态。
    pub status: ExperienceStatus,
    /// 当前置信度。
    pub confidence: ExperienceConfidence,
    /// 创建时间,使用 RFC3339 UTC 字符串。
    pub created_at: String,
    /// 最近更新时间,使用 RFC3339 UTC 字符串。
    pub updated_at: String,
    /// 被哪些旧经验替代 / 继承。
    pub supersedes: Vec<String>,
    /// 当前条目后来被哪些更窄或更新的结论取代。
    ///
    /// 这里允许记录:
    /// - 新经验条目的 ID
    /// - 更窄作用域的逻辑引用,例如 `topic:memory-axes`
    pub replaced_by: Vec<String>,
}

impl ExperienceEntry {
    /// Creates a new active experience entry with generated ID and timestamps.
    #[must_use]
    pub fn new(scope: ExperienceScope, summary: impl Into<String>) -> Self {
        let now = chrono::Utc::now().to_rfc3339();

        Self {
            id: Self::generate_id(),
            summary: summary.into(),
            scope,
            source_topics: Vec::new(),
            source_hats: Vec::new(),
            status: ExperienceStatus::Active,
            confidence: ExperienceConfidence::Medium,
            created_at: now.clone(),
            updated_at: now,
            supersedes: Vec::new(),
            replaced_by: Vec::new(),
        }
    }

    /// Generates a unique scoped experience ID.
    #[must_use]
    pub fn generate_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};

        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");

        let timestamp = duration.as_secs();
        let micros = duration.subsec_micros();
        let hex_suffix = format!("{:04x}", micros % 0x10000);

        format!("exp-{timestamp}-{hex_suffix}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_roundtrip() {
        assert_eq!(
            "role".parse::<ExperienceScope>().unwrap(),
            ExperienceScope::Role
        );
        assert_eq!(
            "project".parse::<ExperienceScope>().unwrap(),
            ExperienceScope::Project
        );
        assert_eq!(ExperienceScope::Role.to_string(), "role");
        assert!("unknown".parse::<ExperienceScope>().is_err());
    }

    #[test]
    fn status_roundtrip() {
        assert_eq!(
            "candidate".parse::<ExperienceStatus>().unwrap(),
            ExperienceStatus::Candidate
        );
        assert_eq!(ExperienceStatus::Active.to_string(), "active");
        assert!("other".parse::<ExperienceStatus>().is_err());
    }

    #[test]
    fn confidence_roundtrip() {
        assert_eq!(
            "high".parse::<ExperienceConfidence>().unwrap(),
            ExperienceConfidence::High
        );
        assert_eq!(ExperienceConfidence::Medium.to_string(), "medium");
        assert!("other".parse::<ExperienceConfidence>().is_err());
    }

    #[test]
    fn new_entry_has_defaults() {
        let entry = ExperienceEntry::new(ExperienceScope::Role, "Keep role-specific rules tight");

        assert!(entry.id.starts_with("exp-"));
        assert_eq!(entry.scope, ExperienceScope::Role);
        assert_eq!(entry.status, ExperienceStatus::Active);
        assert_eq!(entry.confidence, ExperienceConfidence::Medium);
        assert_eq!(entry.summary, "Keep role-specific rules tight");
        assert!(entry.source_topics.is_empty());
        assert!(entry.source_hats.is_empty());
        assert!(entry.supersedes.is_empty());
        assert!(entry.replaced_by.is_empty());
    }
}
