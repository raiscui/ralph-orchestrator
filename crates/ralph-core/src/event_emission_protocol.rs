//! 并行运行时的事件发射协议提示。
//!
//! 说明：
//! - 这里是 Ralph 自己维护的事件 envelope 真相源。
//! - 具体 workflow 的 topic 与 payload 字段仍由 `ralph.yml` / prompt 负责。
//! - 这样执行目录配置不需要反复复制 `<event ...>` 教程，避免协议演进后过期。

use crate::{
    TOPIC_CAPABILITY_FAILED, TOPIC_CAPABILITY_INVOKE, TOPIC_CAPABILITY_REQUEST,
    TOPIC_CAPABILITY_RESULT, TOPIC_RECOVERABLE_CONTINUE, TOPIC_TOPOLOGY_SPAWN_FAILED,
    TOPIC_TOPOLOGY_SPAWN_GROUP, TOPIC_TOPOLOGY_SPAWN_RESULT,
};
use ralph_proto::{
    TOPIC_DISPATCH_DECISION, TOPIC_GATE_REQUEST, TOPIC_GATE_RESOLVE, TOPIC_GATE_TIMEOUT,
    TOPIC_REPLY_HAT_MESSAGE, TOPIC_REQUESTER_RETURN, TOPIC_RUNTIME_DELIVERY,
    TOPIC_RUNTIME_LIFECYCLE,
};
use std::fmt::Write as _;

/// 运行时 prompt 中用于测试与排查的稳定锚点。
pub const EVENT_EMISSION_PROTOCOL_HEADING: &str = "## RALPH EVENT EMISSION PROTOCOL";

/// runtime 协议 topic 的权威分类。
///
/// 说明：
/// - 该分类是 runtime protocol 的单一真相源。
/// - prompt、config validation、role contract normalization 和 routing 特例都应复用这里。
/// - workflow-specific 业务 topic 不在这里枚举,统一归为 `WorkflowResult`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeTopicClass {
    /// `task.start` / `task.resume`: runtime 初始化握手事件。
    RuntimeEntry,
    /// 只能由 parent coordinator 发起的控制面请求。
    CoordinatorOnlyControl,
    /// 运行时/观测面事件,不能作为普通 worker 业务结果或业务触发器。
    ObserverOnly,
    /// 外部人类输入事件。
    HumanInput,
    /// 面向人类的输出回复事件。
    HumanReply,
    /// hat-to-hat 请求回复事件,由 requester-return 路由处理。
    HatReply,
    /// 普通 workflow 业务结果 topic。
    WorkflowResult,
}

impl RuntimeTopicClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RuntimeEntry => "runtime-entry",
            Self::CoordinatorOnlyControl => "coordinator-only-control",
            Self::ObserverOnly => "observer-only",
            Self::HumanInput => "human-input",
            Self::HumanReply => "human-reply",
            Self::HatReply => "hat-reply",
            Self::WorkflowResult => "workflow-result",
        }
    }
}

/// topic 分类结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeTopicClassification {
    pub class: RuntimeTopicClass,
}

impl RuntimeTopicClassification {
    /// 普通 hat 是否禁止把该 topic 写进 triggers。
    pub const fn is_reserved_for_ordinary_hat_trigger(self) -> bool {
        !matches!(self.class, RuntimeTopicClass::WorkflowResult)
    }

    /// task-derived role contract 是否允许把该 topic 作为业务结果 allowlist。
    pub const fn is_allowed_role_result_topic(self) -> bool {
        matches!(self.class, RuntimeTopicClass::WorkflowResult)
    }

    /// 是否属于控制/观测面,而不是普通业务 workflow topic。
    pub const fn is_runtime_control_or_observer_topic(self) -> bool {
        !matches!(self.class, RuntimeTopicClass::WorkflowResult)
    }
}

/// 对 topic 做 runtime protocol 分类。
pub fn classify_runtime_topic(topic: &str) -> RuntimeTopicClassification {
    let trimmed = topic.trim();
    let class = match trimmed {
        "task.start" | "task.resume" => RuntimeTopicClass::RuntimeEntry,
        TOPIC_TOPOLOGY_SPAWN_GROUP | TOPIC_CAPABILITY_REQUEST => {
            RuntimeTopicClass::CoordinatorOnlyControl
        }
        TOPIC_TOPOLOGY_SPAWN_RESULT
        | TOPIC_TOPOLOGY_SPAWN_FAILED
        | TOPIC_CAPABILITY_INVOKE
        | TOPIC_CAPABILITY_RESULT
        | TOPIC_CAPABILITY_FAILED
        | TOPIC_DISPATCH_DECISION
        | TOPIC_REQUESTER_RETURN
        | TOPIC_RUNTIME_DELIVERY
        | TOPIC_RUNTIME_LIFECYCLE
        | TOPIC_GATE_REQUEST
        | TOPIC_GATE_RESOLVE
        | TOPIC_GATE_TIMEOUT
        | TOPIC_RECOVERABLE_CONTINUE => RuntimeTopicClass::ObserverOnly,
        "human.message" => RuntimeTopicClass::HumanInput,
        "reply.human.message" => RuntimeTopicClass::HumanReply,
        TOPIC_REPLY_HAT_MESSAGE => RuntimeTopicClass::HatReply,
        _ if trimmed.starts_with("topology.") || trimmed.starts_with("capability.") => {
            RuntimeTopicClass::CoordinatorOnlyControl
        }
        _ if trimmed.starts_with("runtime.")
            || trimmed.starts_with("dispatch.")
            || trimmed.starts_with("routing.")
            || trimmed.starts_with("recoverable.")
            || trimmed.starts_with("gate.") =>
        {
            RuntimeTopicClass::ObserverOnly
        }
        _ => RuntimeTopicClass::WorkflowResult,
    };

    RuntimeTopicClassification { class }
}

/// ordinary hat 的 trigger 是否为保留 runtime topic。
pub fn is_reserved_hat_trigger(topic: &str) -> bool {
    classify_runtime_topic(topic).is_reserved_for_ordinary_hat_trigger()
}

/// role contract 输出 allowlist 是否允许该 topic。
pub fn is_allowed_role_result_topic(topic: &str) -> bool {
    classify_runtime_topic(topic).is_allowed_role_result_topic()
}

/// runtime 控制/观测面 topic 判断。
pub fn is_runtime_control_or_observer_topic(topic: &str) -> bool {
    classify_runtime_topic(topic).is_runtime_control_or_observer_topic()
}

/// strict target 校验的 runtime 旁路。
///
/// 注意：
/// - 不是所有 control-plane topic 都能绕过 strict target。
/// - 当前只有 gate.* 需要这个旁路,因为 gate.resolve/request/timeout 由运行时权限系统消费。
pub fn runtime_topic_bypasses_strict_target(topic: &str) -> bool {
    matches!(
        topic.trim(),
        TOPIC_GATE_REQUEST | TOPIC_GATE_RESOLVE | TOPIC_GATE_TIMEOUT
    ) || topic.trim().starts_with("gate.")
}

/// prompt 中共享的 runtime topic matrix。
pub fn render_runtime_topic_matrix() -> String {
    [
        (
            "`task.start` / `task.resume`",
            RuntimeTopicClass::RuntimeEntry,
            "runtime entry handshake; not ordinary hat triggers",
        ),
        (
            "`event_loop.starting_event`",
            RuntimeTopicClass::CoordinatorOnlyControl,
            "workflow entry hint after coordination, never the first runtime event",
        ),
        (
            "`topology.spawn_group` / `capability.request`",
            RuntimeTopicClass::CoordinatorOnlyControl,
            "parent coordinator control requests",
        ),
        (
            "`topology.spawn.result|failed`, `capability.result|failed`, `runtime.*`, `dispatch.*`, `routing.*`, `gate.*`",
            RuntimeTopicClass::ObserverOnly,
            "runtime evidence or observer-only control feedback",
        ),
        (
            "`human.message`",
            RuntimeTopicClass::HumanInput,
            "external human input, not a human-facing reply",
        ),
        (
            "`reply.human.message`",
            RuntimeTopicClass::HumanReply,
            "human-facing output; observer-only for routing",
        ),
        (
            "`reply.hat.message`",
            RuntimeTopicClass::HatReply,
            "hat-to-hat answer routed to the requester",
        ),
        (
            "workflow topics from `hats.*.publishes`",
            RuntimeTopicClass::WorkflowResult,
            "ordinary worker-publishable result topics",
        ),
    ]
    .into_iter()
    .fold(String::new(), |mut output, (topic, class, meaning)| {
        let _ = writeln!(output, "- {topic}: `{}` — {meaning}", class.as_str());
        output
    })
}

/// 渲染给并行 hat / coordinator 的内置事件发射协议。
///
/// `publish_topics` 只用于显示当前角色允许/预期发布的业务 topic。
/// 它不会推断 workflow-specific payload schema，因为那些字段属于业务协议。
pub(crate) fn render_event_emission_protocol<'a>(
    publish_topics: impl IntoIterator<Item = &'a str>,
) -> String {
    let mut topics = publish_topics
        .into_iter()
        .map(str::trim)
        .filter(|topic| !topic.is_empty())
        .collect::<Vec<_>>();
    topics.sort_unstable();
    topics.dedup();
    let can_spawn_topology_group = topics.contains(&TOPIC_TOPOLOGY_SPAWN_GROUP);
    let workflow_topics = topics
        .iter()
        .copied()
        .filter(|topic| is_allowed_role_result_topic(topic))
        .collect::<Vec<_>>();
    let runtime_topics = topics
        .iter()
        .copied()
        .filter(|topic| !is_allowed_role_result_topic(topic))
        .collect::<Vec<_>>();

    let mut out = String::new();
    out.push_str(EVENT_EMISSION_PROTOCOL_HEADING);
    out.push('\n');
    out.push_str(
        "- Emit normal workflow events as raw stdout text in your final assistant reply.\n",
    );
    out.push_str("- Canonical envelope: `<event topic=\"topic.name\">payload</event>`.\n");
    out.push_str(
        "- Payload may be text, JSON, or YAML-like text when the workflow contract asks for it.\n",
    );
    out.push_str("- Do not emit normal workflow events through shell `echo`/`printf`, file writes, stderr, or tool transcripts.\n");
    out.push_str("- Every event must include `topic` and a closing `</event>` tag.\n");
    out.push_str("- You may emit multiple complete `<event ...>...</event>` blocks in one assistant reply.\n");
    out.push_str("- `LOOP_COMPLETE` is a completion promise only when it appears outside event tags on its own line; do not put it inside event payloads.\n");
    out.push_str("- Supported attributes: `id`, `reply`, `target`, `target_instance`, `audience_instances`, `require_delivery`, `workspace_strategy`, `session_strategy`, `turn_action`, `spawn_instance`.\n");
    out.push_str("- Use `reply=\"EVENT_ID\"` when answering a specific incoming event. Use exactly one reply id.\n");
    out.push_str("- Runtime start topics are always `task.start` for fresh runs and `task.resume` for resume runs.\n");
    out.push_str("- `event_loop.starting_event` is only a workflow entry hint published after coordination; it is not the first runtime event.\n");
    out.push_str("- Reserved runtime/control topics MUST NOT be treated as ordinary worker business result topics.\n");

    if workflow_topics.is_empty() {
        out.push_str("- This role has no configured workflow `publishes` topics; only emit events when your instructions explicitly require one.\n");
    } else {
        out.push_str("- Configured workflow topics this role may publish:\n");
        for topic in workflow_topics {
            out.push_str("  - `");
            out.push_str(topic);
            out.push_str("`\n");
        }
    }

    if !runtime_topics.is_empty() {
        out.push_str("- Runtime topic matrix:\n");
        out.push_str(&render_runtime_topic_matrix());
        out.push_str("- Authorized runtime/control topics in this prompt:\n");
        for topic in runtime_topics {
            out.push_str("  - `");
            out.push_str(topic);
            out.push_str("` (`");
            out.push_str(classify_runtime_topic(topic).class.as_str());
            out.push_str("`)\n");
        }
    }

    if can_spawn_topology_group {
        out.push_str("- Parent-visible dynamic group spawn: use `topology.spawn_group` when the human asks to create visible hat instances in the parent TUI.\n");
        out.push_str(
            "  - Required JSON fields: `request_id`, `hat`, `delivery_topic`, `instances`.\n",
        );
        out.push_str("  - Each `instances[]` item MUST include `role` and `task`; optional sibling fields: `input`, `fixed_role`, `role_contract`.\n");
        out.push_str("  - `input` MUST be a string when present. Do NOT put `role_contract` inside `input`; `role_contract` is a sibling field on the same `instances[]` item.\n");
        let topology_spawn_group_example = r#"<event topic="topology.spawn_group">{"request_id":"spawn-1","hat":"builder","delivery_topic":"build.task","instances":[{"role":"review","task":"review the proposal","input":"optional task context","role_contract":{"role_name":"review","objective":"review the proposal","input_contract":"Handle build.task for review.","output_contract":"Publish build.done with review findings.","allowed_topics":["build.done"],"forbidden_responsibilities":["Do not coordinate globally"],"success_criteria":["build.done published"],"identity_source":"task-derived"}}]}</event>"#;
        out.push_str("  - Example: ");
        out.push_str(topology_spawn_group_example);
        out.push('\n');
        out.push_str("  - This creates real parent-visible HatInstance entries; it is different from `capability.request` isolated child-runs.\n");
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_emission_protocol_documents_envelope_and_attributes() {
        let rendered = render_event_emission_protocol(["build.done", "build.blocked"]);

        assert!(rendered.contains(EVENT_EMISSION_PROTOCOL_HEADING));
        assert!(rendered.contains("<event"));
        assert!(rendered.contains("topic"));
        assert!(rendered.contains("stdout"));
        assert!(rendered.contains("LOOP_COMPLETE"));
        assert!(rendered.contains("build.done"));
        assert!(rendered.contains("build.blocked"));

        for attr in [
            "id",
            "reply",
            "target",
            "target_instance",
            "audience_instances",
            "require_delivery",
            "session_strategy",
            "workspace_strategy",
            "turn_action",
            "spawn_instance",
        ] {
            assert!(
                rendered.contains(attr),
                "protocol should document supported attribute `{attr}`: {rendered}"
            );
        }
    }

    #[test]
    fn topology_spawn_prompt_documents_parent_visible_group_spawn_contract() {
        let rendered = render_event_emission_protocol([TOPIC_TOPOLOGY_SPAWN_GROUP]);

        assert!(rendered.contains("topology.spawn_group"));
        assert!(rendered.contains("Parent-visible dynamic group spawn"));
        assert!(rendered.contains("request_id"));
        assert!(rendered.contains("instances"));
        assert!(rendered.contains("role_contract"));
        assert!(rendered.contains("input` MUST be a string"));
        assert!(rendered.contains("Do NOT put `role_contract` inside `input`"));
        assert!(rendered.contains("\"role_contract\":{\"role_name\":\"review\""));
        assert!(rendered.contains("real parent-visible HatInstance"));
        assert!(rendered.contains("different from `capability.request`"));
    }

    #[test]
    fn runtime_topic_classification_is_single_source_for_reserved_topics() {
        for (topic, class) in [
            ("task.start", RuntimeTopicClass::RuntimeEntry),
            ("task.resume", RuntimeTopicClass::RuntimeEntry),
            (
                TOPIC_TOPOLOGY_SPAWN_GROUP,
                RuntimeTopicClass::CoordinatorOnlyControl,
            ),
            (
                TOPIC_CAPABILITY_REQUEST,
                RuntimeTopicClass::CoordinatorOnlyControl,
            ),
            (TOPIC_TOPOLOGY_SPAWN_RESULT, RuntimeTopicClass::ObserverOnly),
            (TOPIC_CAPABILITY_RESULT, RuntimeTopicClass::ObserverOnly),
            (TOPIC_RUNTIME_DELIVERY, RuntimeTopicClass::ObserverOnly),
            (TOPIC_GATE_REQUEST, RuntimeTopicClass::ObserverOnly),
            ("human.message", RuntimeTopicClass::HumanInput),
            ("reply.human.message", RuntimeTopicClass::HumanReply),
            (TOPIC_REPLY_HAT_MESSAGE, RuntimeTopicClass::HatReply),
            ("analysis.done", RuntimeTopicClass::WorkflowResult),
        ] {
            let got = classify_runtime_topic(topic);
            assert_eq!(
                got.class, class,
                "{topic} should be classified as {class:?}"
            );
        }

        assert!(is_reserved_hat_trigger("topology.spawn_group"));
        assert!(is_reserved_hat_trigger("reply.human.message"));
        assert!(!is_reserved_hat_trigger("analysis.done"));
        assert!(is_allowed_role_result_topic("analysis.done"));
        assert!(!is_allowed_role_result_topic("capability.result"));
        assert!(runtime_topic_bypasses_strict_target("gate.resolve"));
        assert!(!runtime_topic_bypasses_strict_target(
            "topology.spawn_group"
        ));
    }

    #[test]
    fn event_emission_protocol_does_not_list_reserved_topics_as_workflow_results() {
        let rendered = render_event_emission_protocol([
            "analysis.done",
            "reply.human.message",
            TOPIC_TOPOLOGY_SPAWN_GROUP,
        ]);

        assert!(
            rendered
                .contains("Configured workflow topics this role may publish:\n  - `analysis.done`")
        );
        assert!(rendered.contains("Authorized runtime/control topics in this prompt:"));
        assert!(rendered.contains("`reply.human.message` (`human-reply`)"));
        assert!(rendered.contains("`topology.spawn_group` (`coordinator-only-control`)"));
    }
}
