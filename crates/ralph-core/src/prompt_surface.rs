//! Prompt role layering 的薄语义层.
//!
//! 目标:
//! - 把 coordinator / worker / shared protocol 的边界写成可测试的真相源。
//! - 给 overlay 审计、artifact provenance、prompt 回归测试提供统一的语义标签。

use crate::event_emission_protocol::EVENT_EMISSION_PROTOCOL_HEADING;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Prompt 的受众。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptAudience {
    /// 只给 Ralph/coordinator。
    Coordinator,
    /// 只给 non-Ralph worker。
    Worker,
}

impl PromptAudience {
    /// 当前受众允许的 prompt surface.
    pub const fn allows_surface(self, surface: PromptSurface) -> bool {
        matches!(
            (self, surface),
            (
                Self::Coordinator,
                PromptSurface::CoordinatorOnly | PromptSurface::SharedProtocol
            ) | (
                Self::Worker,
                PromptSurface::WorkerOnly | PromptSurface::SharedProtocol
            )
        )
    }
}

impl fmt::Display for PromptAudience {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Coordinator => f.write_str("coordinator"),
            Self::Worker => f.write_str("worker"),
        }
    }
}

/// Prompt section 的 surface 分类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PromptSurface {
    /// 只允许注入给 coordinator。
    CoordinatorOnly,
    /// 只允许注入给 worker。
    WorkerOnly,
    /// coordinator / worker 共同可见的最小协议。
    SharedProtocol,
}

impl PromptSurface {
    /// 是否为 shared protocol.
    pub const fn is_shared(self) -> bool {
        matches!(self, Self::SharedProtocol)
    }
}

impl fmt::Display for PromptSurface {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CoordinatorOnly => f.write_str("coordinator-only"),
            Self::WorkerOnly => f.write_str("worker-only"),
            Self::SharedProtocol => f.write_str("shared-protocol"),
        }
    }
}

/// prompt section 的结构化元数据.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PromptSectionSpec {
    pub heading: &'static str,
    pub surface: PromptSurface,
}

impl PromptSectionSpec {
    pub const fn new(heading: &'static str, surface: PromptSurface) -> Self {
        Self { heading, surface }
    }
}

/// 运行时 prompt section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptSection {
    pub heading: String,
    pub surface: PromptSurface,
    pub body: String,
}

impl PromptSection {
    pub fn render(&self) -> String {
        format!("{}\n\n{}\n\n", self.heading, self.body)
    }
}

/// identity provenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IdentitySource {
    /// 来自 `ralph.yml` 的静态配置.
    ConfigDerived,
    /// 来自项目模板 / startup preset.
    TemplateDerived,
    /// 由当前任务即时合成.
    TaskDerived,
    /// 来自 runtime autoscale / 动态扩容.
    RuntimeAutoscale,
}

impl fmt::Display for IdentitySource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfigDerived => f.write_str("config-derived"),
            Self::TemplateDerived => f.write_str("template-derived"),
            Self::TaskDerived => f.write_str("task-derived"),
            Self::RuntimeAutoscale => f.write_str("runtime-autoscale"),
        }
    }
}

/// task-derived role 的持久化语义。
///
/// 说明：
/// - `identity_source` 描述身份从哪里来。
/// - `persistence` 只描述这个运行时 role label 是否被提升为固定角色展示。
/// - 二者不能混用：`fixed` 不代表 `task-derived` 变成了 `template-derived`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RolePersistence {
    /// 一次性运行时视角,默认不写入固定角色标签。
    Temporary,
    /// coordinator 明确提升为固定角色标签。
    Fixed,
}

impl fmt::Display for RolePersistence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Temporary => f.write_str("temporary"),
            Self::Fixed => f.write_str("fixed"),
        }
    }
}

/// worker / capability 的 role contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleContract {
    pub role_name: String,
    pub objective: String,
    pub input_contract: String,
    pub output_contract: String,
    pub allowed_topics: Vec<String>,
    pub forbidden_responsibilities: Vec<String>,
    pub success_criteria: Vec<String>,
    pub identity_source: IdentitySource,
}

impl RoleContract {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        role_name: impl Into<String>,
        objective: impl Into<String>,
        input_contract: impl Into<String>,
        output_contract: impl Into<String>,
        allowed_topics: Vec<String>,
        forbidden_responsibilities: Vec<String>,
        success_criteria: Vec<String>,
        identity_source: IdentitySource,
    ) -> Self {
        Self {
            role_name: role_name.into(),
            objective: objective.into(),
            input_contract: input_contract.into(),
            output_contract: output_contract.into(),
            allowed_topics,
            forbidden_responsibilities,
            success_criteria,
            identity_source,
        }
    }
}

/// runtime 归一化后的 role contract。
///
/// 关键语义：
/// - raw `topology.spawn_group.instances[].role_contract` 只是输入 hint。
/// - 只有 `EffectiveRoleContract` 可以被 worker prompt、agents snapshot 和 record summary 消费。
/// - hash 使用稳定的 FNV-1a 64-bit over canonical text,避免为 evidence hash 引入新依赖。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectiveRoleContract {
    /// schema 版本,用于未来演进时明确 evidence 语义。
    pub contract_schema_version: u32,
    /// 归一化后的业务 contract。
    pub contract: RoleContract,
    /// 临时 / 固定展示语义。
    pub persistence: RolePersistence,
    /// 来源 spawn request id。
    pub source_spawn_request_id: String,
    /// 来源 spawn event id,若原事件没有 id 则为空。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_spawn_event_id: Option<String>,
    /// deterministic evidence hash。
    pub role_contract_hash: String,
}

impl EffectiveRoleContract {
    /// 当前 schema 版本。
    pub const SCHEMA_VERSION: u32 = 1;

    /// 创建 runtime canonical contract。
    ///
    /// 说明：
    /// - 调用方必须先完成语义校验和 canonicalization。
    /// - 本函数只负责包装 metadata 与生成稳定 hash。
    pub fn new(
        contract: RoleContract,
        persistence: RolePersistence,
        source_spawn_request_id: impl Into<String>,
        source_spawn_event_id: Option<String>,
    ) -> Self {
        let source_spawn_request_id = source_spawn_request_id.into();
        let role_contract_hash = role_contract_hash(
            Self::SCHEMA_VERSION,
            &contract,
            persistence,
            &source_spawn_request_id,
            source_spawn_event_id.as_deref(),
        );

        Self {
            contract_schema_version: Self::SCHEMA_VERSION,
            contract,
            persistence,
            source_spawn_request_id,
            source_spawn_event_id,
            role_contract_hash,
        }
    }

    /// 面向 `.ralph/agents.json` / TUI / summary 的轻量摘要。
    ///
    /// 注意：不要把完整 prompt 或完整 raw payload 写入 snapshot。
    pub fn summary(&self) -> RoleContractSummary {
        RoleContractSummary {
            role_name: self.contract.role_name.clone(),
            objective_preview: preview_chars(&self.contract.objective, 160),
            allowed_result_topics: self.contract.allowed_topics.clone(),
            identity_source: self.contract.identity_source,
            persistence: self.persistence,
            contract_schema_version: self.contract_schema_version,
            role_contract_hash: self.role_contract_hash.clone(),
            source_spawn_request_id: self.source_spawn_request_id.clone(),
        }
    }

    /// 渲染 worker-only ROLE CONTRACT section。
    pub fn render_worker_section(&self) -> String {
        let allowed_topics = if self.contract.allowed_topics.is_empty() {
            "- <none>".to_string()
        } else {
            self.contract
                .allowed_topics
                .iter()
                .map(|topic| format!("- {topic}"))
                .collect::<Vec<_>>()
                .join("\n")
        };
        let forbidden = if self.contract.forbidden_responsibilities.is_empty() {
            "- <none>".to_string()
        } else {
            self.contract
                .forbidden_responsibilities
                .iter()
                .map(|item| format!("- {item}"))
                .collect::<Vec<_>>()
                .join("\n")
        };
        let success = if self.contract.success_criteria.is_empty() {
            "- <none>".to_string()
        } else {
            self.contract
                .success_criteria
                .iter()
                .map(|item| format!("- {item}"))
                .collect::<Vec<_>>()
                .join("\n")
        };

        format!(
            "### ROLE CONTRACT\n\
             role_name: {role_name}\n\
             identity_source: {identity_source}\n\
             persistence: {persistence}\n\
             contract_schema_version: {schema}\n\
             role_contract_hash: {hash}\n\
             source_spawn_request_id: {request_id}\n\n\
             Objective:\n{objective}\n\n\
             Input contract:\n{input_contract}\n\n\
             Output contract:\n{output_contract}\n\n\
             Allowed result topics:\n{allowed_topics}\n\n\
             Forbidden responsibilities:\n{forbidden}\n\n\
             Success criteria:\n{success}\n",
            role_name = self.contract.role_name,
            identity_source = self.contract.identity_source,
            persistence = self.persistence,
            schema = self.contract_schema_version,
            hash = self.role_contract_hash,
            request_id = self.source_spawn_request_id,
            objective = self.contract.objective,
            input_contract = self.contract.input_contract,
            output_contract = self.contract.output_contract,
        )
    }
}

/// agents snapshot / display 使用的 role contract 摘要。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleContractSummary {
    pub role_name: String,
    pub objective_preview: String,
    pub allowed_result_topics: Vec<String>,
    pub identity_source: IdentitySource,
    pub persistence: RolePersistence,
    pub contract_schema_version: u32,
    pub role_contract_hash: String,
    pub source_spawn_request_id: String,
}

fn preview_chars(value: &str, max_chars: usize) -> String {
    let mut out = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        out.push('…');
    }
    out
}

fn role_contract_hash(
    schema_version: u32,
    contract: &RoleContract,
    persistence: RolePersistence,
    source_spawn_request_id: &str,
    source_spawn_event_id: Option<&str>,
) -> String {
    let mut allowed_topics = contract.allowed_topics.clone();
    allowed_topics.sort();
    let mut forbidden = contract.forbidden_responsibilities.clone();
    forbidden.sort();
    let mut success = contract.success_criteria.clone();
    success.sort();

    let canonical = format!(
        "schema={schema_version}\nrole_name={}\nobjective={}\ninput_contract={}\noutput_contract={}\nallowed_topics={}\nforbidden={}\nsuccess={}\nidentity_source={}\npersistence={persistence}\nsource_spawn_request_id={source_spawn_request_id}\nsource_spawn_event_id={}\n",
        contract.role_name,
        contract.objective,
        contract.input_contract,
        contract.output_contract,
        allowed_topics.join("\u{1f}"),
        forbidden.join("\u{1f}"),
        success.join("\u{1f}"),
        contract.identity_source,
        source_spawn_event_id.unwrap_or(""),
    );

    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in canonical.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("erc-{hash:016x}")
}

pub const ALL_HAT_PROMPT_HEADING: &str = "## ALL HAT PROMPT (config/all_hat.md)";

pub const COORDINATOR_ONLY_SECTION_SPECS: &[PromptSectionSpec] = &[
    PromptSectionSpec::new("## OBJECTIVE", PromptSurface::CoordinatorOnly),
    PromptSectionSpec::new("## WORKFLOW", PromptSurface::CoordinatorOnly),
    PromptSectionSpec::new("## HATS", PromptSurface::CoordinatorOnly),
    PromptSectionSpec::new("### TASK BREAKDOWN", PromptSurface::CoordinatorOnly),
    PromptSectionSpec::new("### STATE MANAGEMENT", PromptSurface::CoordinatorOnly),
    PromptSectionSpec::new("### RALPH PROMPT", PromptSurface::CoordinatorOnly),
    PromptSectionSpec::new(
        "## Runtime Capability Catalog",
        PromptSurface::CoordinatorOnly,
    ),
    PromptSectionSpec::new(
        "## KEY SEMANTICS (OFFICIAL)",
        PromptSurface::CoordinatorOnly,
    ),
    PromptSectionSpec::new(
        "## OUT-OF-BAND EVENT INJECTION",
        PromptSurface::CoordinatorOnly,
    ),
    PromptSectionSpec::new(
        "## HUMAN CHAT (INPUT VS REPLY)",
        PromptSurface::CoordinatorOnly,
    ),
    PromptSectionSpec::new("## CONFIG (THIS RUN)", PromptSurface::CoordinatorOnly),
    PromptSectionSpec::new(
        "## HATS TOPOLOGY (CONFIGURED)",
        PromptSurface::CoordinatorOnly,
    ),
    PromptSectionSpec::new("## WHAT TO DO", PromptSurface::CoordinatorOnly),
    PromptSectionSpec::new("## DONE", PromptSurface::CoordinatorOnly),
];

pub const WORKER_ONLY_SECTION_SPECS: &[PromptSectionSpec] = &[
    PromptSectionSpec::new("### 0. ORIENTATION", PromptSurface::WorkerOnly),
    PromptSectionSpec::new("### 1. EXECUTE", PromptSurface::WorkerOnly),
    PromptSectionSpec::new("### 2. VERIFY", PromptSurface::WorkerOnly),
    PromptSectionSpec::new("### 3. REPORT", PromptSurface::WorkerOnly),
];

pub const SHARED_PROTOCOL_SECTION_SPECS: &[PromptSectionSpec] = &[
    PromptSectionSpec::new(ALL_HAT_PROMPT_HEADING, PromptSurface::SharedProtocol),
    PromptSectionSpec::new(
        EVENT_EMISSION_PROTOCOL_HEADING,
        PromptSurface::SharedProtocol,
    ),
];

pub const COORDINATOR_ONLY_HEADINGS: &[&str] = &[
    "## OBJECTIVE",
    "## WORKFLOW",
    "## HATS",
    "### TASK BREAKDOWN",
    "### STATE MANAGEMENT",
    "### RALPH PROMPT",
    "## Runtime Capability Catalog",
    "## KEY SEMANTICS (OFFICIAL)",
    "## OUT-OF-BAND EVENT INJECTION",
    "## HUMAN CHAT (INPUT VS REPLY)",
    "## CONFIG (THIS RUN)",
    "## HATS TOPOLOGY (CONFIGURED)",
    "## WHAT TO DO",
    "## DONE",
];

pub const WORKER_ONLY_HEADINGS: &[&str] = &[
    "### 0. ORIENTATION",
    "### 1. EXECUTE",
    "### 2. VERIFY",
    "### 3. REPORT",
];

pub const SHARED_PROTOCOL_HEADINGS: &[&str] =
    &[ALL_HAT_PROMPT_HEADING, EVENT_EMISSION_PROTOCOL_HEADING];

pub fn section_specs_for_surface(surface: PromptSurface) -> &'static [PromptSectionSpec] {
    match surface {
        PromptSurface::CoordinatorOnly => COORDINATOR_ONLY_SECTION_SPECS,
        PromptSurface::WorkerOnly => WORKER_ONLY_SECTION_SPECS,
        PromptSurface::SharedProtocol => SHARED_PROTOCOL_SECTION_SPECS,
    }
}

pub fn headings_for_surface(surface: PromptSurface) -> &'static [&'static str] {
    match surface {
        PromptSurface::CoordinatorOnly => COORDINATOR_ONLY_HEADINGS,
        PromptSurface::WorkerOnly => WORKER_ONLY_HEADINGS,
        PromptSurface::SharedProtocol => SHARED_PROTOCOL_HEADINGS,
    }
}

pub fn surface_for_heading(heading: &str) -> Option<PromptSurface> {
    for spec in COORDINATOR_ONLY_SECTION_SPECS {
        if spec.heading == heading {
            return Some(spec.surface);
        }
    }
    for spec in WORKER_ONLY_SECTION_SPECS {
        if spec.heading == heading {
            return Some(spec.surface);
        }
    }
    for spec in SHARED_PROTOCOL_SECTION_SPECS {
        if spec.heading == heading {
            return Some(spec.surface);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_audience_allows_only_expected_surfaces() {
        assert!(PromptAudience::Coordinator.allows_surface(PromptSurface::CoordinatorOnly));
        assert!(PromptAudience::Coordinator.allows_surface(PromptSurface::SharedProtocol));
        assert!(!PromptAudience::Coordinator.allows_surface(PromptSurface::WorkerOnly));

        assert!(PromptAudience::Worker.allows_surface(PromptSurface::WorkerOnly));
        assert!(PromptAudience::Worker.allows_surface(PromptSurface::SharedProtocol));
        assert!(!PromptAudience::Worker.allows_surface(PromptSurface::CoordinatorOnly));
    }

    #[test]
    fn prompt_surface_headings_are_single_source_of_truth() {
        for heading in COORDINATOR_ONLY_HEADINGS {
            assert_eq!(
                surface_for_heading(heading),
                Some(PromptSurface::CoordinatorOnly),
                "{heading} should be coordinator-only"
            );
        }

        for heading in WORKER_ONLY_HEADINGS {
            assert_eq!(
                surface_for_heading(heading),
                Some(PromptSurface::WorkerOnly),
                "{heading} should be worker-only"
            );
        }

        for heading in SHARED_PROTOCOL_HEADINGS {
            assert_eq!(
                surface_for_heading(heading),
                Some(PromptSurface::SharedProtocol),
                "{heading} should be shared protocol"
            );
        }
    }

    #[test]
    fn identity_source_serializes_as_contract_values() {
        assert_eq!(
            serde_json::to_string(&IdentitySource::ConfigDerived).expect("serialize"),
            "\"config-derived\""
        );
        assert_eq!(
            serde_json::to_string(&IdentitySource::TemplateDerived).expect("serialize"),
            "\"template-derived\""
        );
        assert_eq!(
            serde_json::to_string(&IdentitySource::TaskDerived).expect("serialize"),
            "\"task-derived\""
        );
        assert_eq!(
            serde_json::to_string(&IdentitySource::RuntimeAutoscale).expect("serialize"),
            "\"runtime-autoscale\""
        );
    }

    #[test]
    fn effective_role_contract_summary_is_hash_only_and_previewed() {
        let contract = RoleContract::new(
            "功能补充",
            "a".repeat(180),
            "input contract should not enter summary",
            "output contract should not enter summary",
            vec!["analysis.done".to_string()],
            vec!["do not coordinate globally".to_string()],
            vec!["publish analysis.done".to_string()],
            IdentitySource::TaskDerived,
        );
        let effective = EffectiveRoleContract::new(
            contract,
            RolePersistence::Temporary,
            "spawn-req-1",
            Some("spawn-event-1".to_string()),
        );

        let summary = effective.summary();
        assert_eq!(summary.role_name, "功能补充");
        assert_eq!(summary.objective_preview.chars().count(), 161);
        assert!(summary.objective_preview.ends_with('…'));
        assert_eq!(
            summary.allowed_result_topics,
            vec!["analysis.done".to_string()]
        );
        assert_eq!(summary.identity_source, IdentitySource::TaskDerived);
        assert_eq!(summary.persistence, RolePersistence::Temporary);
        assert_eq!(
            summary.contract_schema_version,
            EffectiveRoleContract::SCHEMA_VERSION
        );
        assert!(summary.role_contract_hash.starts_with("erc-"));
        assert_eq!(summary.source_spawn_request_id, "spawn-req-1");
    }

    #[test]
    fn effective_role_contract_hash_is_stable_for_reordered_list_fields() {
        let first = EffectiveRoleContract::new(
            RoleContract::new(
                "review",
                "review task",
                "input",
                "output",
                vec!["review.done".to_string(), "analysis.done".to_string()],
                vec!["b".to_string(), "a".to_string()],
                vec!["2".to_string(), "1".to_string()],
                IdentitySource::TaskDerived,
            ),
            RolePersistence::Fixed,
            "spawn-req-1",
            None,
        );
        let second = EffectiveRoleContract::new(
            RoleContract::new(
                "review",
                "review task",
                "input",
                "output",
                vec!["analysis.done".to_string(), "review.done".to_string()],
                vec!["a".to_string(), "b".to_string()],
                vec!["1".to_string(), "2".to_string()],
                IdentitySource::TaskDerived,
            ),
            RolePersistence::Fixed,
            "spawn-req-1",
            None,
        );

        assert_eq!(first.role_contract_hash, second.role_contract_hash);
    }

    #[test]
    fn effective_role_contract_worker_section_contains_contract_boundaries() {
        let effective = EffectiveRoleContract::new(
            RoleContract::new(
                "功能补充",
                "补充 feature A",
                "Handle build.task.",
                "Publish analysis.done.",
                vec!["analysis.done".to_string()],
                vec!["Do not create or spawn additional hats.".to_string()],
                vec!["Publish allowed result.".to_string()],
                IdentitySource::TaskDerived,
            ),
            RolePersistence::Temporary,
            "spawn-req-1",
            Some("spawn-event-1".to_string()),
        );

        let section = effective.render_worker_section();
        assert!(section.contains("### ROLE CONTRACT"));
        assert!(section.contains("role_name: 功能补充"));
        assert!(section.contains("identity_source: task-derived"));
        assert!(section.contains("persistence: temporary"));
        assert!(section.contains("source_spawn_request_id: spawn-req-1"));
        assert!(section.contains("Allowed result topics:\n- analysis.done"));
        assert!(
            section
                .contains("Forbidden responsibilities:\n- Do not create or spawn additional hats.")
        );
        assert!(section.contains("Success criteria:\n- Publish allowed result."));
    }
}
