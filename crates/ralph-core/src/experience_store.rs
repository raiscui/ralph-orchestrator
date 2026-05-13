//! Markdown-based scoped experience storage.
//!
//! role experience 与 project experience 都走同一套 store。
//! 区别只在路径不同,而不在协议不同。

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::experience::ExperienceEntry;
use crate::experience_parser::parse_experiences;

/// Default path for project experience relative to workspace root.
pub const DEFAULT_PROJECT_EXPERIENCE_PATH: &str = "experience.md";
/// Default root for role experience files relative to workspace root.
pub const DEFAULT_ROLE_EXPERIENCE_ROOT: &str = ".ralph/roles";

/// Shared markdown store for role and project experience files.
#[derive(Debug, Clone)]
pub struct MarkdownExperienceStore {
    path: PathBuf,
}

impl MarkdownExperienceStore {
    /// Creates a new store at the given path.
    #[must_use]
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    /// Creates a store for the project-level experience file.
    #[must_use]
    pub fn with_project_path(root: impl AsRef<Path>) -> Self {
        Self::new(root.as_ref().join(DEFAULT_PROJECT_EXPERIENCE_PATH))
    }

    /// Creates a store for a role-level experience file.
    #[must_use]
    pub fn with_role_path(root: impl AsRef<Path>, hat_id: &str) -> Self {
        Self::new(
            root.as_ref()
                .join(DEFAULT_ROLE_EXPERIENCE_ROOT)
                .join(hat_id)
                .join("experience.md"),
        )
    }

    /// Returns the underlying file path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns true if the store file exists.
    #[must_use]
    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    /// Loads all entries from the markdown file.
    pub fn load(&self) -> io::Result<Vec<ExperienceEntry>> {
        if !self.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&self.path)?;
        Ok(parse_experiences(&content))
    }

    /// Overwrites the file with the given entries.
    pub fn write_all(&self, entries: &[ExperienceEntry]) -> io::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&self.path, format_experiences_as_markdown(entries))
    }

    /// Appends one entry by re-reading and rewriting the full document.
    pub fn append(&self, entry: &ExperienceEntry) -> io::Result<()> {
        let mut entries = self.load()?;
        entries.push(entry.clone());
        self.write_all(&entries)
    }
}

/// Formats experience entries as markdown.
#[must_use]
pub fn format_experiences_as_markdown(entries: &[ExperienceEntry]) -> String {
    if entries.is_empty() {
        return "# Experience\n".to_string();
    }

    let mut output = String::from("# Experience\n");

    for entry in entries {
        let supersedes = if entry.supersedes.is_empty() {
            String::new()
        } else {
            entry.supersedes.join(", ")
        };
        let replaced_by = if entry.replaced_by.is_empty() {
            String::new()
        } else {
            entry.replaced_by.join(", ")
        };

        output.push_str(&format!(
            "\n### {}\n{}\n<!-- scope: {} | source_topics: {} | source_hats: {} | status: {} | confidence: {} | created_at: {} | updated_at: {} | supersedes: {} | replaced_by: {} -->\n",
            entry.id,
            entry
                .summary
                .lines()
                .map(|line| format!("> {line}"))
                .collect::<Vec<_>>()
                .join("\n"),
            entry.scope,
            entry.source_topics.join(", "),
            entry.source_hats.join(", "),
            entry.status,
            entry.confidence,
            entry.created_at,
            entry.updated_at,
            supersedes,
            replaced_by,
        ));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::experience::{ExperienceConfidence, ExperienceScope, ExperienceStatus};
    use tempfile::TempDir;

    fn sample_entry(scope: ExperienceScope) -> ExperienceEntry {
        ExperienceEntry {
            id: "exp-1737372000-a1b2".to_string(),
            summary: "Keep shared knowledge narrow before promoting it.".to_string(),
            scope,
            source_topics: vec!["memory-axes".to_string()],
            source_hats: vec!["ralph#1".to_string()],
            status: ExperienceStatus::Active,
            confidence: ExperienceConfidence::High,
            created_at: "2026-03-21T00:00:00Z".to_string(),
            updated_at: "2026-03-21T00:00:00Z".to_string(),
            supersedes: vec!["exp-1737371000-abcd".to_string()],
            replaced_by: vec!["exp-1737373000-cdef".to_string()],
        }
    }

    #[test]
    fn roundtrip_markdown_format() {
        let markdown = format_experiences_as_markdown(&[sample_entry(ExperienceScope::Project)]);
        let entries = parse_experiences(&markdown);

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0], sample_entry(ExperienceScope::Project));
    }

    #[test]
    fn shared_protocol_loads_project_and_role_files() {
        let temp_dir = TempDir::new().unwrap();
        let project_store = MarkdownExperienceStore::with_project_path(temp_dir.path());
        let role_store = MarkdownExperienceStore::with_role_path(temp_dir.path(), "spec_reviewer");

        project_store
            .write_all(&[sample_entry(ExperienceScope::Project)])
            .unwrap();
        role_store
            .write_all(&[sample_entry(ExperienceScope::Role)])
            .unwrap();

        let project_entries = project_store.load().unwrap();
        let role_entries = role_store.load().unwrap();

        assert_eq!(project_entries.len(), 1);
        assert_eq!(role_entries.len(), 1);
        assert_eq!(project_entries[0].scope, ExperienceScope::Project);
        assert_eq!(role_entries[0].scope, ExperienceScope::Role);
    }
}
