//! Markdown parser for scoped experience files.
//!
//! 支持 role/project 复用同一条 entry 协议。
//! 解析失败的 metadata 会按保守默认值回退,避免单条坏数据拖垮整个文件。

use crate::experience::{ExperienceConfidence, ExperienceEntry, ExperienceScope, ExperienceStatus};

/// Parse an experience markdown document into entries.
#[must_use]
pub fn parse_experiences(markdown: &str) -> Vec<ExperienceEntry> {
    let mut entries = Vec::new();
    let mut current_id: Option<String> = None;
    let mut current_summary: Vec<String> = Vec::new();
    let mut current_scope = ExperienceScope::Project;
    let mut current_source_topics: Vec<String> = Vec::new();
    let mut current_source_hats: Vec<String> = Vec::new();
    let mut current_status = ExperienceStatus::Active;
    let mut current_confidence = ExperienceConfidence::Medium;
    let mut current_created_at: Option<String> = None;
    let mut current_updated_at: Option<String> = None;
    let mut current_supersedes: Vec<String> = Vec::new();
    let mut current_replaced_by: Vec<String> = Vec::new();

    for line in markdown.lines() {
        if let Some(id) = line.strip_prefix("### ") {
            flush_experience(
                &mut entries,
                &mut current_id,
                &mut current_summary,
                &mut current_scope,
                &mut current_source_topics,
                &mut current_source_hats,
                &mut current_status,
                &mut current_confidence,
                &mut current_created_at,
                &mut current_updated_at,
                &mut current_supersedes,
                &mut current_replaced_by,
            );

            if id.starts_with("exp-") {
                current_id = Some(id.trim().to_string());
            }
        } else if let Some(content) = line.strip_prefix("> ") {
            current_summary.push(content.to_string());
        } else if line.starts_with("<!-- ") && line.ends_with(" -->") {
            apply_metadata(
                line,
                &mut current_scope,
                &mut current_source_topics,
                &mut current_source_hats,
                &mut current_status,
                &mut current_confidence,
                &mut current_created_at,
                &mut current_updated_at,
                &mut current_supersedes,
                &mut current_replaced_by,
            );
        }
    }

    flush_experience(
        &mut entries,
        &mut current_id,
        &mut current_summary,
        &mut current_scope,
        &mut current_source_topics,
        &mut current_source_hats,
        &mut current_status,
        &mut current_confidence,
        &mut current_created_at,
        &mut current_updated_at,
        &mut current_supersedes,
        &mut current_replaced_by,
    );

    entries
}

#[allow(clippy::too_many_arguments)]
fn flush_experience(
    entries: &mut Vec<ExperienceEntry>,
    current_id: &mut Option<String>,
    current_summary: &mut Vec<String>,
    current_scope: &mut ExperienceScope,
    current_source_topics: &mut Vec<String>,
    current_source_hats: &mut Vec<String>,
    current_status: &mut ExperienceStatus,
    current_confidence: &mut ExperienceConfidence,
    current_created_at: &mut Option<String>,
    current_updated_at: &mut Option<String>,
    current_supersedes: &mut Vec<String>,
    current_replaced_by: &mut Vec<String>,
) {
    if let Some(id) = current_id.take()
        && !current_summary.is_empty()
    {
        let now = chrono::Utc::now().to_rfc3339();

        entries.push(ExperienceEntry {
            id,
            summary: current_summary.join("\n"),
            scope: *current_scope,
            source_topics: std::mem::take(current_source_topics),
            source_hats: std::mem::take(current_source_hats),
            status: *current_status,
            confidence: *current_confidence,
            created_at: current_created_at.take().unwrap_or_else(|| now.clone()),
            updated_at: current_updated_at.take().unwrap_or(now),
            supersedes: std::mem::take(current_supersedes),
            replaced_by: std::mem::take(current_replaced_by),
        });
    }

    current_summary.clear();
    *current_scope = ExperienceScope::Project;
    *current_status = ExperienceStatus::Active;
    *current_confidence = ExperienceConfidence::Medium;
}

#[allow(clippy::too_many_arguments)]
fn apply_metadata(
    line: &str,
    current_scope: &mut ExperienceScope,
    current_source_topics: &mut Vec<String>,
    current_source_hats: &mut Vec<String>,
    current_status: &mut ExperienceStatus,
    current_confidence: &mut ExperienceConfidence,
    current_created_at: &mut Option<String>,
    current_updated_at: &mut Option<String>,
    current_supersedes: &mut Vec<String>,
    current_replaced_by: &mut Vec<String>,
) {
    let inner = line
        .strip_prefix("<!-- ")
        .and_then(|s| s.strip_suffix(" -->"))
        .unwrap_or_default();

    for segment in inner.split(" | ") {
        let Some((key, raw_value)) = segment.split_once(": ") else {
            continue;
        };

        match key.trim() {
            "scope" => {
                if let Ok(scope) = raw_value.trim().parse::<ExperienceScope>() {
                    *current_scope = scope;
                }
            }
            "source_topics" => {
                *current_source_topics = split_csv(raw_value);
            }
            "source_hats" => {
                *current_source_hats = split_csv(raw_value);
            }
            "status" => {
                if let Ok(status) = raw_value.trim().parse::<ExperienceStatus>() {
                    *current_status = status;
                }
            }
            "confidence" => {
                if let Ok(confidence) = raw_value.trim().parse::<ExperienceConfidence>() {
                    *current_confidence = confidence;
                }
            }
            "created_at" => {
                *current_created_at = Some(raw_value.trim().to_string());
            }
            "updated_at" => {
                *current_updated_at = Some(raw_value.trim().to_string());
            }
            "supersedes" => {
                *current_supersedes = split_csv(raw_value);
            }
            "replaced_by" => {
                *current_replaced_by = split_csv(raw_value);
            }
            _ => {}
        }
    }
}

fn split_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToString::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_experience_entry() {
        let markdown = r"# Experience

### exp-1737372000-a1b2
> Only the canonical writer may update shared topic files.
<!-- scope: project | source_topics: memory-axes | source_hats: ralph#1 | status: active | confidence: high | created_at: 2026-03-21T00:00:00Z | updated_at: 2026-03-21T00:10:00Z | supersedes: exp-1000-abcd | replaced_by: exp-2000-bbee -->
";

        let entries = parse_experiences(markdown);
        assert_eq!(entries.len(), 1);

        let entry = &entries[0];
        assert_eq!(entry.id, "exp-1737372000-a1b2");
        assert_eq!(entry.scope, ExperienceScope::Project);
        assert_eq!(entry.status, ExperienceStatus::Active);
        assert_eq!(entry.confidence, ExperienceConfidence::High);
        assert_eq!(entry.source_topics, vec!["memory-axes"]);
        assert_eq!(entry.source_hats, vec!["ralph#1"]);
        assert_eq!(entry.supersedes, vec!["exp-1000-abcd"]);
        assert_eq!(entry.replaced_by, vec!["exp-2000-bbee"]);
    }

    #[test]
    fn parse_multiline_summary() {
        let markdown = r"# Experience

### exp-1737372000-a1b2
> First line
> Second line
<!-- scope: role | source_topics: topic-a | source_hats: spec_reviewer | status: active | confidence: medium | created_at: 2026-03-21T00:00:00Z | updated_at: 2026-03-21T00:00:00Z | supersedes:  | replaced_by:  -->
";

        let entries = parse_experiences(markdown);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].summary, "First line\nSecond line");
        assert!(entries[0].supersedes.is_empty());
        assert!(entries[0].replaced_by.is_empty());
    }

    #[test]
    fn parse_uses_defaults_for_missing_metadata() {
        let markdown = r"# Experience

### exp-1737372000-a1b2
> Summary without explicit metadata.
";

        let entries = parse_experiences(markdown);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].scope, ExperienceScope::Project);
        assert_eq!(entries[0].status, ExperienceStatus::Active);
        assert_eq!(entries[0].confidence, ExperienceConfidence::Medium);
    }
}
