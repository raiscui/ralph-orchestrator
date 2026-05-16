//! 并行运行时的事件发射协议提示。
//!
//! 说明：
//! - 这里是 Ralph 自己维护的事件 envelope 真相源。
//! - 具体 workflow 的 topic 与 payload 字段仍由 `ralph.yml` / prompt 负责。
//! - 这样执行目录配置不需要反复复制 `<event ...>` 教程，避免协议演进后过期。

/// 运行时 prompt 中用于测试与排查的稳定锚点。
pub const EVENT_EMISSION_PROTOCOL_HEADING: &str = "## RALPH EVENT EMISSION PROTOCOL";

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

    let mut out = String::new();
    out.push_str(EVENT_EMISSION_PROTOCOL_HEADING);
    out.push_str("\n");
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

    if topics.is_empty() {
        out.push_str("- This role has no configured workflow `publishes` topics; only emit events when your instructions explicitly require one.\n");
    } else {
        out.push_str("- Configured workflow topics this role may publish:\n");
        for topic in topics {
            out.push_str("  - `");
            out.push_str(topic);
            out.push_str("`\n");
        }
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
}
