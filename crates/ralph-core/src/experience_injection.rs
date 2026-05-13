//! Scoped experience and context injection helpers.
//!
//! 这一层把 project / role / topic / instance / runtime 五层上下文
//! 组装成 prompt 可消费的文本,并保持“先宽后窄、摘要优先”的读取纪律。

use std::fs;
use std::path::Path;

use crate::config::{InjectMode, MemoriesConfig};
use crate::experience::ExperienceStatus;
use crate::experience_governance::detect_unique_topic_group;
use crate::experience_store::{MarkdownExperienceStore, format_experiences_as_markdown};
use crate::memory_store::{MarkdownMemoryStore, format_memories_as_markdown, truncate_to_budget};
use crate::task_store::TaskStore;

const INSTANCE_FILE_PATTERNS: [(&str, &str); 4] = [
    ("SUMMARY.md", "Summary"),
    ("WORKLOG.md", "Worklog"),
    ("notes.md", "Notes"),
    ("task_plan.md", "Task Plan"),
];

const TOPIC_SUMMARY_BUDGET_TOKENS: usize = 300;
const INSTANCE_SUMMARY_BUDGET_TOKENS: usize = 220;
const RUNTIME_TASK_BUDGET_TOKENS: usize = 180;

/// Prompt injection mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScopedPromptMode<'a> {
    /// 普通 hat: 直接按可复用范围从宽到窄注入。
    Ordinary {
        /// 当前 hat 的 role 经验所属 id。
        role_hat_id: &'a str,
        /// 当前实例上下文目录的 id。
        instance_id: &'a str,
    },
    /// Ralph 协调者: 先给项目经验,再在 prompt 正文之后按需补窄范围上下文。
    Coordinator {
        /// 如果已经收敛到单一 owner role,才允许补该 role 的经验。
        owner_role_hint: Option<&'a str>,
    },
}

/// Builds the scoped prompt envelope and returns the fully injected prompt.
pub(crate) fn inject_scoped_context(
    prompt: String,
    core: &crate::config::CoreConfig,
    memories: &MemoriesConfig,
    mode: ScopedPromptMode<'_>,
    memories_skill: &str,
) -> String {
    if !memories.enabled || memories.inject != InjectMode::Auto {
        return prompt;
    }

    let prefix_sections = match mode {
        ScopedPromptMode::Ordinary {
            role_hat_id,
            instance_id,
        } => build_ordinary_prefix_sections(core, role_hat_id, instance_id),
        ScopedPromptMode::Coordinator { .. } => build_coordinator_prefix_sections(core),
    };

    let suffix_sections = match mode {
        ScopedPromptMode::Ordinary { .. } => Vec::new(),
        ScopedPromptMode::Coordinator { owner_role_hint } => {
            build_coordinator_suffix_sections(core, owner_role_hint)
        }
    };

    if prefix_sections.is_empty() && suffix_sections.is_empty() {
        return prompt;
    }

    let prefix = join_sections(&prefix_sections);
    let suffix = join_sections(&suffix_sections);

    let prefix = apply_head_budget(prefix, memories.budget);
    let suffix = apply_head_budget(suffix, memories.budget);

    let mut final_prompt = String::new();

    if !prefix.is_empty() {
        final_prompt.push_str(&prefix);
        final_prompt.push_str("\n\n");
    }

    // 兼容旧行为: 只要确实注入了某层经验,就继续补这一段 usage skill。
    final_prompt.push_str(memories_skill);
    final_prompt.push_str("\n\n");
    final_prompt.push_str(&prompt);

    if !suffix.is_empty() {
        final_prompt.push_str("\n\n");
        final_prompt.push_str(&suffix);
    }

    final_prompt
}

fn build_ordinary_prefix_sections(
    core: &crate::config::CoreConfig,
    role_hat_id: &str,
    instance_id: &str,
) -> Vec<String> {
    let mut sections = Vec::new();

    if let Some(project_experience) = load_project_experience_section(core) {
        sections.push(project_experience);
    }

    if let Some(role_experience) = load_role_experience_section(core, role_hat_id) {
        sections.push(role_experience);
    }

    if let Some(legacy_memories) = load_legacy_memories_section(core) {
        sections.push(legacy_memories);
    }

    if let Some(topic_summary) = load_unique_topic_summary_section(core) {
        sections.push(topic_summary);
    }

    if let Some(instance_summary) = load_instance_summary_section(core, instance_id) {
        sections.push(instance_summary);
    }

    if let Some(runtime_tasks) = load_runtime_tasks_section(core) {
        sections.push(runtime_tasks);
    }

    sections
}

fn build_coordinator_prefix_sections(core: &crate::config::CoreConfig) -> Vec<String> {
    let mut sections = Vec::new();

    if let Some(project_experience) = load_project_experience_section(core) {
        sections.push(project_experience);
    }

    sections
}

fn build_coordinator_suffix_sections(
    core: &crate::config::CoreConfig,
    owner_role_hint: Option<&str>,
) -> Vec<String> {
    let mut sections = Vec::new();

    if let Some(legacy_memories) = load_legacy_memories_section(core) {
        sections.push(legacy_memories);
    }

    if let Some(owner_role_hint) = owner_role_hint
        && let Some(role_experience) = load_role_experience_section(core, owner_role_hint)
    {
        sections.push(role_experience);
    }

    if let Some(topic_summary) = load_unique_topic_summary_section(core) {
        sections.push(topic_summary);
    }

    if let Some(runtime_tasks) = load_runtime_tasks_section(core) {
        sections.push(runtime_tasks);
    }

    sections
}

fn load_project_experience_section(core: &crate::config::CoreConfig) -> Option<String> {
    let store = MarkdownExperienceStore::new(core.resolve_project_experience_path());
    load_experience_section("Project Experience", &store)
}

fn load_role_experience_section(core: &crate::config::CoreConfig, hat_id: &str) -> Option<String> {
    let store = MarkdownExperienceStore::new(core.resolve_role_experience_path(hat_id));
    load_experience_section(&format!("Role Experience ({hat_id})"), &store)
}

fn load_experience_section(title: &str, store: &MarkdownExperienceStore) -> Option<String> {
    let entries = store
        .load()
        .ok()?
        .into_iter()
        .filter(|entry| entry.status == ExperienceStatus::Active)
        .collect::<Vec<_>>();

    if entries.is_empty() {
        return None;
    }

    Some(
        format_experiences_as_markdown(&entries)
            .replacen("# Experience", &format!("## {title}"), 1)
            .trim_end()
            .to_string(),
    )
}

fn load_legacy_memories_section(core: &crate::config::CoreConfig) -> Option<String> {
    let store = MarkdownMemoryStore::new(core.resolve_legacy_memories_path());
    let memories = store.load().ok()?;

    if memories.is_empty() {
        return None;
    }

    Some(
        format_memories_as_markdown(&memories)
            .replacen("# Memories", "## Legacy Memories (Compatibility)", 1)
            .trim_end()
            .to_string(),
    )
}

fn load_unique_topic_summary_section(core: &crate::config::CoreConfig) -> Option<String> {
    let topic_group = detect_unique_topic_group(&core.workspace_root)?;
    let mut sections = Vec::new();

    for file in topic_group.files {
        let label = file.kind.label();
        let path = file.path;
        let summary = summarize_file_tail(&path, TOPIC_SUMMARY_BUDGET_TOKENS)?;
        sections.push(format!("### {label}\n{summary}"));
    }

    if sections.is_empty() {
        return None;
    }

    Some(format!(
        "## Topic Context Summary ({})\n\n{}",
        topic_group.suffix,
        sections.join("\n\n")
    ))
}

fn load_instance_summary_section(
    core: &crate::config::CoreConfig,
    instance_id: &str,
) -> Option<String> {
    let instance_dir = core.resolve_instance_context_dir(instance_id);
    if !instance_dir.exists() {
        return None;
    }

    let mut sections = Vec::new();
    for (file_name, label) in INSTANCE_FILE_PATTERNS {
        let path = instance_dir.join(file_name);
        let Some(summary) = summarize_file_tail(&path, INSTANCE_SUMMARY_BUDGET_TOKENS) else {
            continue;
        };
        sections.push(format!("### {label}\n{summary}"));
    }

    if sections.is_empty() {
        return None;
    }

    Some(format!(
        "## Instance Context Summary ({instance_id})\n\n{}",
        sections.join("\n\n")
    ))
}

fn load_runtime_tasks_section(core: &crate::config::CoreConfig) -> Option<String> {
    let task_store_path = core.resolve_path(".agent/tasks.jsonl");
    let store = TaskStore::load(&task_store_path).ok()?;
    let mut open_tasks = store.open();

    if open_tasks.is_empty() {
        return None;
    }

    open_tasks.sort_by(|left, right| {
        left.priority
            .cmp(&right.priority)
            .then_with(|| left.title.cmp(&right.title))
    });

    let mut section = String::from("## Runtime Task State\n\n");
    section.push_str(&format!(
        "- open tasks: {}\n- ready tasks: {}\n",
        open_tasks.len(),
        store.ready().len()
    ));

    for task in open_tasks.iter().take(5) {
        section.push_str(&format!("- [p{}] {}\n", task.priority, task.title));
    }

    Some(truncate_to_budget(&section, RUNTIME_TASK_BUDGET_TOKENS))
}

fn join_sections(sections: &[String]) -> String {
    sections
        .iter()
        .filter(|section| !section.trim().is_empty())
        .map(|section| section.trim_end().to_string())
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn apply_head_budget(content: String, budget_tokens: usize) -> String {
    if budget_tokens == 0 {
        return content;
    }

    let max_chars = budget_tokens.saturating_mul(4);
    let total_chars = content.chars().count();

    if total_chars <= max_chars {
        return content;
    }

    let kept = content.chars().take(max_chars).collect::<String>();
    format!("{kept}\n\n[_Later scoped context truncated to fit budget._]")
}

fn summarize_file_tail(path: &Path, budget_tokens: usize) -> Option<String> {
    if !path.exists() {
        return None;
    }

    let content = fs::read_to_string(path).ok()?;
    let content = content.trim();

    if content.is_empty() {
        return None;
    }

    Some(truncate_to_budget(content, budget_tokens))
}
