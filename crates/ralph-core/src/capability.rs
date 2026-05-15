//! Runtime capability metadata 与 invocation artifact。
//!
//! 这个模块只定义结构化协议和可审计记录。
//! 真实执行入口由 CLI / runtime layer 负责,并且 v1 必须保持隔离执行,不能热改父 run topology。

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ralph_proto::Event;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

/// 控制面 topic: parent run 请求调用 capability。
pub const TOPIC_CAPABILITY_REQUEST: &str = "capability.request";

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

/// parent run 发出的 capability request。
///
/// 说明:
/// - 这是 `ralph#1` 输出 `<event topic="capability.request">...</event>` 时的 payload 契约。
/// - `request_id` 是 parent run 内的幂等键,运行时必须用它避免重复启动 isolated invocation。
/// - `capability_id` 和 `input` 是 child/micro-run 的最小执行输入。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityRequestRecord {
    pub request_id: String,
    pub capability_id: String,
    pub input: String,
}

impl CapabilityRequestRecord {
    /// 解析并校验 `capability.request` payload。
    ///
    /// 说明:
    /// - 不直接用 `serde_json::from_str::<CapabilityRequestRecord>` 的原因是:
    ///   失败事件仍需要尽量带上 payload 里已经存在的 `request_id` / `capability_id`。
    /// - 因此这里先读成 `Value`,再逐字段校验。
    pub fn parse_payload(payload: &str) -> Result<Self, CapabilityRequestParseError> {
        let value = serde_json::from_str::<Value>(payload).map_err(|error| {
            CapabilityRequestParseError::new(None, None, format!("invalid JSON payload: {error}"))
        })?;

        let request_id = string_field(&value, "request_id");
        let capability_id = string_field(&value, "capability_id");
        let input = string_field(&value, "input");

        let missing = [
            ("request_id", request_id.as_deref()),
            ("capability_id", capability_id.as_deref()),
            ("input", input.as_deref()),
        ]
        .into_iter()
        .filter_map(|(field, value)| value.is_none_or(str::is_empty).then_some(field))
        .collect::<Vec<_>>();

        if !missing.is_empty() {
            return Err(CapabilityRequestParseError::new(
                request_id,
                capability_id,
                format!("missing or empty field(s): {}", missing.join(", ")),
            ));
        }

        Ok(Self {
            request_id: request_id.expect("validated request_id"),
            capability_id: capability_id.expect("validated capability_id"),
            input: input.expect("validated input"),
        })
    }
}

fn string_field(value: &Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

/// `capability.request` 解析失败时保留的上下文。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityRequestParseError {
    pub request_id: Option<String>,
    pub capability_id: Option<String>,
    pub error: String,
}

impl CapabilityRequestParseError {
    fn new(request_id: Option<String>, capability_id: Option<String>, error: String) -> Self {
        Self {
            request_id,
            capability_id,
            error,
        }
    }
}

/// parent-facing result/failure event 中的 artifact 链接。
///
/// 说明:
/// - 这里使用 invocation 产物路径,让 parent run 可以从 event 直接跳回 durable artifacts。
/// - 真相源仍是 artifact 文件和 evidence index;这个结构只是 parent run 的跳转索引。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityParentArtifactPaths {
    pub invoke_json: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_json: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_json: Option<String>,
    pub resolved_config: String,
    pub events_jsonl: String,
    pub evidence_index: String,
}

/// parent run 可消费的 `capability.result` payload。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityParentResultRecord {
    pub status: String,
    pub request_id: String,
    pub invocation_id: String,
    pub capability_id: String,
    pub result_summary: String,
    pub artifacts: CapabilityParentArtifactPaths,
    pub parent_topology_unchanged: bool,
}

/// parent run 可消费的 `capability.failed` payload。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityParentFailedRecord {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invocation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capability_id: Option<String>,
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifacts: Option<CapabilityParentArtifactPaths>,
    pub parent_topology_unchanged: bool,
}

/// parent runtime capability request 的执行 adapter。
///
/// 说明:
/// - core 只负责识别 runtime action 和路由 result/failure。
/// - 真正的 isolated child/micro-run 由 CLI 或其他宿主注入,避免 core 反向依赖进程执行细节。
#[async_trait]
pub trait RuntimeCapabilityInvoker: Send + Sync {
    async fn invoke(&self, request: CapabilityRequestRecord) -> anyhow::Result<Event>;
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

    #[test]
    fn capability_request_payload_requires_structured_fields() {
        let request = CapabilityRequestRecord::parse_payload(
            r#"{"request_id":"req-1","capability_id":"hat:focused-reviewer","input":"review"}"#,
        )
        .unwrap();

        assert_eq!(request.request_id, "req-1");
        assert_eq!(request.capability_id, "hat:focused-reviewer");
        assert_eq!(request.input, "review");
    }

    #[test]
    fn capability_request_parse_error_preserves_available_ids() {
        let error =
            CapabilityRequestRecord::parse_payload(r#"{"request_id":"req-1","input":"review"}"#)
                .unwrap_err();

        assert_eq!(error.request_id.as_deref(), Some("req-1"));
        assert_eq!(error.capability_id, None);
        assert!(error.error.contains("capability_id"));
    }
}
