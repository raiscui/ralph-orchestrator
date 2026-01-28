//! Human gate protocol types.
//!
//! 说明：
//! - gate 是“事件化的人类介入点”，用于 async loop 里的咨询/审批。
//! - 关键目标是：等待 human 时不阻塞其他 HatInstance，并且 timeout 后可继续推进且可回放。
//! - payload 建议使用 JSON（便于 UI/日志/测试解析）。

use crate::HatInstanceId;
use serde::{Deserialize, Serialize};

/// Gate 协议事件 topic：发起一个 gate 请求。
pub const TOPIC_GATE_REQUEST: &str = "gate.request";
/// Gate 协议事件 topic：gate 被解决（human 或 timeout 自决）。
pub const TOPIC_GATE_RESOLVE: &str = "gate.resolve";
/// Gate 协议事件 topic：gate 超时（后续应由决策型 job 产出 gate.resolve）。
pub const TOPIC_GATE_TIMEOUT: &str = "gate.timeout";

/// gate 的语义类型：咨询 vs 审批。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum GateKind {
    #[default]
    Consult,
    Approval,
}

/// gate.resolve 的来源。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum GateResolvedBy {
    #[default]
    Human,
    LlmTimeout,
}

/// gate.request 的结构化 payload。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateRequest {
    /// gate 的唯一 ID（用于匹配 gate.resolve / gate.timeout）。
    pub gate_id: String,

    /// 可选 thread_id（长期路由主键），用于 UI/对话串联。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,

    /// 发起该 gate 的实例（例如 writer#1）。
    pub requested_by: HatInstanceId,

    /// 咨询 or 审批。
    #[serde(default)]
    pub kind: GateKind,

    /// 超时秒数：
    /// - null：普通 gate（一直等 human）
    /// - 60：超时 gate（示例）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<u64>,

    /// 给 human 的问题（尽量短，但要包含必要上下文）。
    pub prompt: String,

    /// LLM 建议的默认倾向（可选）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposed_default: Option<String>,
}

/// gate.resolve 的结构化 payload。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GateResolve {
    pub gate_id: String,
    #[serde(default)]
    pub resolved_by: GateResolvedBy,

    /// 决策内容（允许 string/object/array 等，保持协议灵活）。
    #[serde(default)]
    pub decision: serde_json::Value,

    /// 可选：显式指定要回送给哪个实例（用于“恢复现场”或跨进程重放）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_by: Option<HatInstanceId>,
}

/// gate.timeout 的结构化 payload。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateTimeout {
    pub gate_id: String,

    /// 可选：谁发起的 gate（便于 timeout 后路由给 decider）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_by: Option<HatInstanceId>,
}
