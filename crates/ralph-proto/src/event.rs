//! Event types for pub/sub messaging.

use crate::{AudienceOverride, HatId, HatInstanceId, Topic};
use serde::{Deserialize, Serialize};

/// Workspace strategy override for a single event/job.
///
/// 说明：
/// - 这是运行时“执行环境”的选择，不应编码进 topic 字符串。
/// - 多事件合并成一个 job 时，应按“最强隔离优先”规则合并：
///   `worktree > patch > shared`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceStrategy {
    /// 共享工作区（默认）。
    #[default]
    Shared,
    /// 补丁/受限写入（第一版可先等价于 shared，再逐步增强）。
    Patch,
    /// Git worktree 隔离（适合并行写、多轮迭代）。
    Worktree,
}

/// An event in the pub/sub system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Optional stable identifier for this event.
    ///
    /// 说明：
    /// - 并行调度/回放需要一个“可引用”的事件主键（例如用于 `dispatch.decision.event_id`）。
    /// - 为兼容历史事件，该字段保持可选。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// The routing topic for this event.
    pub topic: Topic,

    /// The content/payload of the event.
    pub payload: String,

    /// The hat that published this event (if any).
    pub source: Option<HatId>,

    /// The hat instance that published this event (if any).
    ///
    /// 说明：
    /// - 在并行 HatInstance 模型下，source_instance 用于更精确的归因与回放。
    /// - 为兼容历史事件，该字段保持可选。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_instance: Option<HatInstanceId>,

    /// Optional target hat for direct handoff.
    pub target: Option<HatId>,

    /// Optional target instance for direct handoff.
    ///
    /// 若 target_instance 存在，Supervisor 应优先按实例路由。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_instance: Option<HatInstanceId>,

    /// Optional audience override for routing.
    ///
    /// 说明：
    /// - 这是“每条事件”级别的覆盖，只缩小/约束投递受众。
    /// - 具体 recipients = TopicContract.audience ∩ audience_override（如果有）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audience_override: Option<AudienceOverride>,

    /// Optional per-event workspace strategy override.
    ///
    /// 说明：
    /// - 如果存在，该字段表示“希望以何种隔离级别执行该事件相关的 job”。最终仍需经过
    ///   capability/permission gate 的判定。
    /// - 若缺失，则使用 hat 的默认 workspace.strategy。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_strategy: Option<WorkspaceStrategy>,
}

impl Event {
    /// Creates a new event with the given topic and payload.
    pub fn new(topic: impl Into<Topic>, payload: impl Into<String>) -> Self {
        Self {
            id: None,
            topic: topic.into(),
            payload: payload.into(),
            source: None,
            source_instance: None,
            target: None,
            target_instance: None,
            audience_override: None,
            workspace_strategy: None,
        }
    }

    /// Sets a stable identifier for this event.
    #[must_use]
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Sets the source hat for this event.
    #[must_use]
    pub fn with_source(mut self, source: impl Into<HatId>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Sets the source hat instance for this event.
    #[must_use]
    pub fn with_source_instance(mut self, source_instance: impl Into<HatInstanceId>) -> Self {
        self.source_instance = Some(source_instance.into());
        self
    }

    /// Sets the target hat for direct handoff.
    #[must_use]
    pub fn with_target(mut self, target: impl Into<HatId>) -> Self {
        self.target = Some(target.into());
        self
    }

    /// Sets the target hat instance for direct handoff.
    #[must_use]
    pub fn with_target_instance(mut self, target_instance: impl Into<HatInstanceId>) -> Self {
        self.target_instance = Some(target_instance.into());
        self
    }

    /// Sets the routing audience override.
    #[must_use]
    pub fn with_audience_override(mut self, override_: AudienceOverride) -> Self {
        self.audience_override = Some(override_);
        self
    }

    /// Sets the per-event workspace strategy override.
    #[must_use]
    pub fn with_workspace_strategy(mut self, strategy: WorkspaceStrategy) -> Self {
        self.workspace_strategy = Some(strategy);
        self
    }
}
