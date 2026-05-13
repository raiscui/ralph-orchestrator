//! Routing-related types for parallel hat instances.
//!
//! 说明：
//! - 这里放的是“协议/配置层”的基础类型，供 ralph-core / ralph-cli / ralph-tui 复用。
//! - 重点是把“事件投递语义”显式化：queue / fanout，以及实例级受众限制。

use crate::{HatId, HatInstanceId, HatInstanceState};
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

/// 系统事件：记录一次真实 runtime delivery。
///
/// 说明：
/// - 这是给 V2 durable replay graph 使用的“纯记录事件”。
/// - 一条成功投递写一条记录；fanout 会为每个 recipient 写一条记录。
/// - payload 为 `RuntimeDeliveryRecord` 的 JSON 字符串。
pub const TOPIC_RUNTIME_DELIVERY: &str = "runtime.delivery";

/// 系统事件：记录一次 runtime lifecycle / control-plane 动作。
///
/// 说明：
/// - 这是给 V2 durable replay graph 使用的“纯记录事件”。
/// - 覆盖 create / spawn / state / freeze / cancel / shutdown 等实例生命周期证据。
/// - payload 为 `RuntimeLifecycleRecord` 的 JSON 字符串。
pub const TOPIC_RUNTIME_LIFECYCLE: &str = "runtime.lifecycle";

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

/// Runtime delivery record 的投递类别。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeDeliveryKind {
    /// `target_instance` 或 Supervisor 控制面直达。
    Direct,
    /// TopicContract queue 选中一个最终 recipient。
    Queue,
    /// Fanout 的单个 recipient 投递。
    Fanout,
    /// `reply.hat.message` 解析 requester 后的回送投递。
    Reply,
}

impl RuntimeDeliveryKind {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::Queue => "queue",
            Self::Fanout => "fanout",
            Self::Reply => "reply",
        }
    }
}

/// 一条真实投递的 durable evidence。
///
/// 说明：
/// - `event_id` 关联原始业务事件。
/// - `recipient` 是路由完成后的最终实例,而不是事件里可能存在的 hint。
/// - `mode` 表示这条投递边来自 direct / queue / fanout / reply 哪种语义。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeDeliveryRecord {
    /// 被投递事件的 id。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    /// 若这是 reply,则保存被回复的 request event id。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply: Option<String>,
    /// 被投递事件的 topic。
    pub topic: String,
    /// 发布该事件的源实例。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_instance: Option<HatInstanceId>,
    /// 路由完成后的最终 recipient。
    pub recipient: HatInstanceId,
    /// 投递类别。
    pub mode: RuntimeDeliveryKind,
}

impl RuntimeDeliveryRecord {
    pub fn new(
        event_id: Option<String>,
        reply: Option<String>,
        topic: impl Into<String>,
        source_instance: Option<HatInstanceId>,
        recipient: HatInstanceId,
        mode: RuntimeDeliveryKind,
    ) -> Self {
        Self {
            event_id,
            reply,
            topic: topic.into(),
            source_instance,
            recipient,
            mode,
        }
    }
}

/// Runtime lifecycle / control record 的类别。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeLifecycleKind {
    /// 静态或兜底实例被创建。
    Create,
    /// 动态实例被创建。
    Spawn,
    /// 实例状态变化。
    State,
    /// completion promise 后冻结 pending 工作。
    Freeze,
    /// Supervisor 请求取消当前 job。
    Cancel,
    /// Supervisor 请求实例关闭。
    Shutdown,
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// 一条实例 lifecycle / control-plane durable evidence。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeLifecycleRecord {
    /// 发生 lifecycle 动作的实例。
    pub instance_id: HatInstanceId,
    /// lifecycle / control 类型。
    pub kind: RuntimeLifecycleKind,
    /// 如果该记录携带状态,保存在这里。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<HatInstanceState>,
    /// 是否是动态实例。
    #[serde(default, skip_serializing_if = "is_false")]
    pub dynamic: bool,
    /// 如果该动作由某条事件触发,保存源 event id。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_event_id: Option<String>,
    /// 简短原因,用于 replay / 排障时解释控制边。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl RuntimeLifecycleRecord {
    pub fn new(instance_id: HatInstanceId, kind: RuntimeLifecycleKind) -> Self {
        Self {
            instance_id,
            kind,
            state: None,
            dynamic: false,
            source_event_id: None,
            reason: None,
        }
    }

    #[must_use]
    pub fn with_state(mut self, state: HatInstanceState) -> Self {
        self.state = Some(state);
        self
    }

    #[must_use]
    pub fn with_dynamic(mut self, dynamic: bool) -> Self {
        self.dynamic = dynamic;
        self
    }

    #[must_use]
    pub fn with_source_event_id(mut self, source_event_id: impl Into<String>) -> Self {
        self.source_event_id = Some(source_event_id.into());
        self
    }

    #[must_use]
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }
}
