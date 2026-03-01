//! Event types for pub/sub messaging.

use crate::{AudienceOverride, HatId, HatInstanceId, Topic};
use serde::{Deserialize, Serialize};

// =====================================================================
// Event id 生成规则
// =====================================================================
//
// 说明:
// - Event.id 用于"可引用主键",支持 reply(in-reply-to) 链路与诊断关联.
// - 我们选择 nanoid 的 SAFE 字符集,避免出现引号/空格等会破坏 `<event ...>` 属性的字符.
// - 长度不追求密码学意义的全局唯一,但需要在单次运行内极低碰撞概率,同时尽量短以降低 token 噪音.
const DEFAULT_EVENT_ID_LEN: usize = 12;

/// Generates a new URL-safe event id (nanoid).
///
/// 说明:
/// - 默认长度: 12
/// - 字符集: `nanoid::alphabet::SAFE`(URL-safe)
#[must_use]
pub fn new_event_id() -> String {
    nanoid::nanoid!(DEFAULT_EVENT_ID_LEN, &nanoid::alphabet::SAFE)
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_EVENT_ID_LEN, new_event_id};

    #[test]
    fn new_event_id_is_short_and_url_safe() {
        let id = new_event_id();

        assert_eq!(
            id.len(),
            DEFAULT_EVENT_ID_LEN,
            "nanoid length should stay stable to avoid token noise drift"
        );

        assert!(
            id.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'),
            "nanoid should be URL-safe, got: {id}"
        );
    }
}

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

/// Session strategy override for a single event/job.
///
/// 说明：
/// - 这是运行时"会话形态"的选择,用于决定 hat job 走一次性 `exec` ,还是可复用 thread 的 `mcp` .
/// - 该字段必须是显式信号,不能只依赖隐式 thread 状态,否则 replay/诊断会失真.
/// - 在方案1(只升级,不降级)里:
///   - 默认 `exec`.
///   - 任意事件请求 `mcp` 后,同一 instance 将 sticky 到 `mcp`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SessionStrategy {
    /// 一次性会话(默认).
    #[default]
    Exec,
    /// 持续会话(复用 thread,例如 Codex MCP).
    Mcp,
    /// App Server 会话(复用 thread + 支持 turn/steer/interrupt).
    ///
    /// 说明：
    /// - 该策略用于表达“真 steer”能力：在同一 in-flight turn 内追加输入。
    /// - 排序上它强于 `mcp`：`exec < mcp < app_server`。
    AppServer,
}

/// Turn-level action semantic for App Server sessions.
///
/// 说明：
/// - 这是“turn 级控制语义”，用于表达该事件希望如何作用于当前会话：
///   - start: 新开 turn（默认）
///   - steer: 对 in-flight turn 追加输入
///   - interrupt: 中断当前 in-flight turn
/// - 该字段是显式信号,用于保证 replay/诊断一致性。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TurnAction {
    /// 新开 turn（默认）。
    #[default]
    Start,
    /// 对 in-flight turn 追加输入（真 steer）。
    Steer,
    /// 中断当前 in-flight turn。
    Interrupt,
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

    /// Optional in-reply-to event id (single value).
    ///
    /// 说明：
    /// - 用于把本条事件与"被回复的事件"建立关联（in-reply-to）。
    /// - 该字段为单值：一次只回复一条 event.id（避免形成多父关系导致歧义）。
    /// - 若为空或缺失，表示该事件不是对某条特定事件的回复。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply: Option<String>,

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

    /// Optional per-event session strategy override.
    ///
    /// 说明：
    /// - 该字段用于并行模式下的"动态会话选择":
    ///   - ralph 可以在发布 `<event ...>` 时按需指定 `session_strategy="mcp"` .
    ///   - hat instance 会在首次进入 mcp 后 sticky,避免 exec/mcp 来回切换造成上下文分裂.
    /// - 若缺失: 等价于 `exec`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_strategy: Option<SessionStrategy>,

    /// Optional per-event turn action (App Server only).
    ///
    /// 说明：
    /// - 当 `session_strategy=app_server` 时，该字段用于表达 turn 级控制：
    ///   - `steer`: 对 in-flight turn 追加输入
    ///   - `interrupt`: 中断当前 turn
    /// - 当缺失时,等价于 `start`（新开 turn）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_action: Option<TurnAction>,

    /// Optional routing hint: spawn a fresh hat instance for this delivery (parallel mode).
    ///
    /// 说明：
    /// - 该字段用于表达“我要一个崭新的实例接收这条消息”(上下文隔离)。
    /// - 这是 Supervisor 的路由提示信号,不是业务事件的一部分。
    /// - 推荐用法：
    ///   - `spawn_instance=true` + `target="<hat_id>"`：强制为该 hat 创建动态实例并直达投递。
    /// - 约束：
    ///   - 与 `target_instance` 互斥（已经指定实例就不需要 spawn）。
    ///   - 若缺少 `target`，Supervisor 会降级为普通路由（并 best-effort escalate 一条 routing.escalate）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spawn_instance: Option<bool>,
}

impl Event {
    /// Creates a new event with the given topic and payload.
    pub fn new(topic: impl Into<Topic>, payload: impl Into<String>) -> Self {
        Self {
            id: None,
            reply: None,
            topic: topic.into(),
            payload: payload.into(),
            source: None,
            source_instance: None,
            target: None,
            target_instance: None,
            audience_override: None,
            workspace_strategy: None,
            session_strategy: None,
            turn_action: None,
            spawn_instance: None,
        }
    }

    /// Sets a stable identifier for this event.
    #[must_use]
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Sets the reply-to event id for this event.
    ///
    /// 说明：
    /// - `reply` 语义是单值 in-reply-to。
    /// - 传入空字符串会被规范化为 None（等价于“不回复任何事件”）。
    #[must_use]
    pub fn with_reply(mut self, reply: impl Into<String>) -> Self {
        let reply = reply.into();
        if reply.trim().is_empty() {
            self.reply = None;
        } else {
            self.reply = Some(reply);
        }
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

    /// Sets the per-event session strategy override.
    #[must_use]
    pub fn with_session_strategy(mut self, strategy: SessionStrategy) -> Self {
        self.session_strategy = Some(strategy);
        self
    }

    /// Sets the per-event turn action.
    #[must_use]
    pub fn with_turn_action(mut self, action: TurnAction) -> Self {
        self.turn_action = Some(action);
        self
    }

    /// Requests the Supervisor to spawn a fresh instance for delivery (parallel mode).
    ///
    /// 说明：
    /// - 传入 `false` 会被规范化为 None（等价于“不请求 spawn”），避免落盘噪音。
    #[must_use]
    pub fn with_spawn_instance(mut self, enabled: bool) -> Self {
        self.spawn_instance = if enabled { Some(true) } else { None };
        self
    }
}
