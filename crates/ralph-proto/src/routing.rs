//! Routing-related types for parallel hat instances.
//!
//! 说明：
//! - 这里放的是“协议/配置层”的基础类型，供 ralph-core / ralph-cli / ralph-tui 复用。
//! - 重点是把“事件投递语义”显式化：queue / fanout，以及实例级受众限制。

use crate::{HatId, HatInstanceId};
use serde::{Deserialize, Serialize};

/// 系统事件：记录 queue 派发决策（候选集 + 结果 + 可选原因）。
///
/// 说明：
/// - 这是给 replay/观测使用的“纯记录事件”，不会直接触发业务 hat。
/// - 对齐 `specs/parallel-hat-instances.spec.md` 的命名：`dispatch.decision`
/// - 建议 payload 为 `QueueDecisionRecord` 的 JSON 字符串。
pub const TOPIC_DISPATCH_DECISION: &str = "dispatch.decision";

/// 系统事件：hat -> hat 的显式答案回流 topic。
///
/// 说明：
/// - 这不是普通 workflow event。
/// - 该 topic 需要结合 `reply="<request_event_id>"` 使用，运行时会把它回送给原请求方实例。
pub const TOPIC_REPLY_HAT_MESSAGE: &str = "reply.hat.message";

/// 系统事件：记录 requester-return 解析结果（成功目标 / 未解析原因）。
///
/// 说明：
/// - 这是 observer / diagnostics 用的“纯记录事件”，不会参与业务路由。
/// - payload 建议为紧凑 JSON，便于 grep / 回放 / 排障。
pub const TOPIC_REQUESTER_RETURN: &str = "routing.requester_return";

/// Delivery semantics for a topic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Delivery {
    #[default]
    Queue,
    Fanout,
}

/// How queue delivery chooses a single recipient.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum QueueSelection {
    /// Ask an LLM/decider job to choose (with deterministic fallback).
    #[default]
    Llm,
    /// Deterministic selection (round-robin / least-busy / etc).
    Deterministic,
}

/// What to do when an event references a missing instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MissingInstancePolicy {
    /// Spawn a new instance (if allowed) and deliver.
    Spawn,
    /// Re-queue (deliver to any eligible existing instance).
    #[default]
    Queue,
    /// Escalate (e.g., human gate) and do not silently reroute.
    Escalate,
    /// Drop the delivery.
    Drop,
}

/// Audience selector for routing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AudienceSelector {
    /// Explicit instance IDs, e.g. `["writer#1", "reviewer#2"]`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub instances: Vec<HatInstanceId>,

    /// Instance id prefixes, e.g. `["writer#"]` meaning all writer instances.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub instance_prefixes: Vec<String>,

    /// Hat IDs, e.g. `["reviewer", "tester"]`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hats: Vec<HatId>,
}

/// Per-event audience override.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AudienceOverride {
    /// Override recipients to these instances (best-effort by default).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub instances: Vec<HatInstanceId>,

    /// If true, missing instances are treated as delivery failures (must escalate).
    #[serde(default)]
    pub require_delivery: bool,
}

/// Topic contract defining routing semantics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopicContract {
    /// How events are delivered: queue (choose one) or fanout (deliver to all).
    #[serde(default)]
    pub delivery: Delivery,

    /// Base audience selection for this topic.
    #[serde(default)]
    pub audience: AudienceSelector,

    /// How queue selection is performed when multiple candidates exist.
    #[serde(default)]
    pub queue_selection: QueueSelection,

    /// What to do when referenced instances are missing.
    #[serde(default)]
    pub missing_instance_policy: MissingInstancePolicy,
}

impl Default for TopicContract {
    fn default() -> Self {
        Self {
            delivery: Delivery::Queue,
            audience: AudienceSelector::default(),
            queue_selection: QueueSelection::Llm,
            missing_instance_policy: MissingInstancePolicy::Queue,
        }
    }
}

// ============================================================================
// Routing decision records (for replay / observability)
// ============================================================================

/// Record of a queue routing decision.
///
/// 说明：
/// - 用于把“候选集 + 选择结果 + 可选原因”落盘到事件日志，保证 replay 不重算。
/// - 该结构本身不绑定具体落盘方式（Event payload / JSONL / etc），只负责定义字段语义。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueDecisionRecord {
    /// 被派发的“原始事件”ID（用于把决策与事件关联起来，保证 replay 不重算）。
    pub event_id: String,
    /// Candidate instances considered.
    pub candidates: Vec<HatInstanceId>,
    /// The chosen recipient instance.
    pub chosen_instance: HatInstanceId,
    /// Optional short rationale (best-effort).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl QueueDecisionRecord {
    pub fn new(
        event_id: impl Into<String>,
        candidates: Vec<HatInstanceId>,
        chosen_instance: HatInstanceId,
        reason: Option<String>,
    ) -> Self {
        Self {
            event_id: event_id.into(),
            candidates,
            chosen_instance,
            reason,
        }
    }
}
