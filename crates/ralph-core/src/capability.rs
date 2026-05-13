//! Runtime capability metadata 与 invocation artifact。
//!
//! 这个模块只定义结构化协议和可审计记录。
//! 真实执行入口由 CLI / runtime layer 负责,并且 v1 必须保持隔离执行,不能热改父 run topology。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// 控制面 topic: capability invocation 开始。
pub const TOPIC_CAPABILITY_INVOKE: &str = "capability.invoke";

/// 控制面 topic: capability invocation 成功结束。
pub const TOPIC_CAPABILITY_RESULT: &str = "capability.result";

/// 控制面 topic: capability invocation 失败。
pub const TOPIC_CAPABILITY_FAILED: &str = "capability.failed";

/// Runtime capability 类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityKind {
    /// 整套 workflow capability。
    WorkflowCapability,
    /// 单个 hat capability。
    HatCapability,
}

impl fmt::Display for CapabilityKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WorkflowCapability => f.write_str("workflow_capability"),
            Self::HatCapability => f.write_str("hat_capability"),
        }
    }
}

/// Runtime capability 的隔离调用模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityInvocationMode {
    /// 通过隔离 child run 调用 workflow。
    IsolatedChildRun,
    /// 通过隔离 micro-run 调用 hat。
    IsolatedMicroRun,
}

impl fmt::Display for CapabilityInvocationMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IsolatedChildRun => f.write_str("isolated_child_run"),
            Self::IsolatedMicroRun => f.write_str("isolated_micro_run"),
        }
    }
}

/// 轻量 capability metadata。
///
/// 说明:
/// - 这是启动/运行时注入给 `ralph#1` 的摘要层。
/// - 它不包含完整 workflow YAML 或完整 hat instructions。
/// - selector / invoker 依赖这些结构化字段,不依赖 YAML 注释。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityMetadata {
    /// 稳定 capability id。
    pub id: String,
    /// capability 类型。
    pub kind: CapabilityKind,
    /// 短摘要。
    pub summary: String,
    /// 目标/产出意图。
    pub goal: String,
    /// 何时应该使用。
    pub when_to_use: String,
    /// 输入契约。
    pub input_contract: String,
    /// 输出契约。
    pub output_contract: String,
    /// v1 调用模式。
    pub invocation_mode: CapabilityInvocationMode,
}

/// capability chooser 结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityChoice {
    /// 被选中的 capability。
    pub capability_id: String,
    /// 选择依据。
    pub reason: String,
    /// 使用的 chooser 版本。
    pub chooser_version: String,
}

/// capability.invoke artifact。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityInvocationRecord {
    /// 本次调用 id。
    pub invocation_id: String,
    /// 记录时间。
    pub ts: DateTime<Utc>,
    /// capability metadata 摘要。
    pub capability: CapabilityMetadata,
    /// 选择依据。
    pub choice: CapabilityChoice,
    /// 实际输入。
    pub input: String,
    /// 输入契约快照。
    pub input_contract: String,
    /// 解析出的配置 artifact 相对/绝对路径。
    pub resolved_config_path: String,
    /// 父 topology 是否保持稳定。
    pub parent_topology_unchanged: bool,
}

/// capability.result artifact。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityResultRecord {
    /// 本次调用 id。
    pub invocation_id: String,
    /// 记录时间。
    pub ts: DateTime<Utc>,
    /// capability id。
    pub capability_id: String,
    /// 结果摘要。
    pub result_summary: String,
    /// 子执行状态码。
    pub exit_code: Option<i32>,
    /// stdout 摘要。
    pub stdout_summary: String,
    /// stderr 摘要。
    pub stderr_summary: String,
    /// 输出契约快照。
    pub output_contract: String,
    /// 父 topology 是否保持稳定。
    pub parent_topology_unchanged: bool,
}

/// capability.failed artifact。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityFailedRecord {
    /// 本次调用 id。
    pub invocation_id: String,
    /// 记录时间。
    pub ts: DateTime<Utc>,
    /// capability id。
    pub capability_id: String,
    /// 错误摘要。
    pub error: String,
    /// 父 topology 是否保持稳定。
    pub parent_topology_unchanged: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_metadata_serializes_snake_case_kind_and_mode() {
        let metadata = CapabilityMetadata {
            id: "workflow:feature-minimal".to_string(),
            kind: CapabilityKind::WorkflowCapability,
            summary: "Feature workflow".to_string(),
            goal: "Build a feature".to_string(),
            when_to_use: "When a user asks for implementation".to_string(),
            input_contract: "Natural-language task".to_string(),
            output_contract: "Summary plus evidence".to_string(),
            invocation_mode: CapabilityInvocationMode::IsolatedChildRun,
        };

        let json = serde_json::to_string(&metadata).unwrap();

        assert!(json.contains("workflow_capability"));
        assert!(json.contains("isolated_child_run"));
    }
}
