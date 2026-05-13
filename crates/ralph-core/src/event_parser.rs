//! Event parsing from CLI output.
//!
//! Parses XML-style event tags from agent output:
//! ```text
//! <event topic="impl.done">payload</event>
//! <event topic="handoff" target="reviewer">payload</event>
//! ```

use ralph_proto::{
    AudienceOverride, Event, HatId, HatInstanceId, SessionStrategy, TurnAction, WorkspaceStrategy,
};

const EVENT_OPEN_TAG: &str = "<event";
const EVENT_CLOSE_TAG: &str = "</event>";
const EVENT_CLOSE_TAG_ESCAPED: &str = "<\\/event>";

/// Strips ANSI escape sequences from a string.
///
/// Handles CSI sequences (\x1b[...m), OSC sequences (\x1b]...\x07),
/// and simple escape sequences (\x1b followed by a single char).
fn strip_ansi(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut result = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == 0x1b {
            // ESC character - start of escape sequence
            i += 1;
            if i >= bytes.len() {
                break;
            }

            match bytes[i] {
                b'[' => {
                    // CSI sequence: ESC [ ... (final byte in 0x40-0x7E range)
                    i += 1;
                    while i < bytes.len() && !(0x40..=0x7E).contains(&bytes[i]) {
                        i += 1;
                    }
                    if i < bytes.len() {
                        i += 1; // Skip final byte
                    }
                }
                b']' => {
                    // OSC sequence: ESC ] ... (terminated by BEL or ST)
                    i += 1;
                    while i < bytes.len() {
                        if bytes[i] == 0x07 {
                            i += 1;
                            break;
                        }
                        if bytes[i] == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'\\' {
                            i += 2;
                            break;
                        }
                        i += 1;
                    }
                }
                _ => {
                    // Simple escape sequence: ESC + single char
                    i += 1;
                }
            }
        } else {
            result.push(bytes[i]);
            i += 1;
        }
    }

    String::from_utf8_lossy(&result).into_owned()
}

/// Evidence of backpressure checks for build.done events.
#[derive(Debug, Clone, PartialEq)]
pub struct BackpressureEvidence {
    pub tests_passed: bool,
    pub lint_passed: bool,
    pub typecheck_passed: bool,
}

impl BackpressureEvidence {
    /// Returns true if all checks passed.
    pub fn all_passed(&self) -> bool {
        self.tests_passed && self.lint_passed && self.typecheck_passed
    }
}

/// Parser for extracting events from CLI output.
#[derive(Debug, Default)]
pub struct EventParser {
    /// The source hat ID to attach to parsed events.
    source: Option<HatId>,
}

impl EventParser {
    /// Creates a new event parser.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the source hat for parsed events.
    pub fn with_source(mut self, source: impl Into<HatId>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Parses events from CLI output text.
    ///
    /// Returns a list of parsed events.
    pub fn parse(&self, output: &str) -> Vec<Event> {
        let mut events = Vec::new();
        let mut remaining = output;

        while let Some(start_idx) = Self::find_event_start(remaining) {
            let after_start = &remaining[start_idx..];

            // Find the end of the opening tag
            let Some(tag_end) = after_start.find('>') else {
                remaining = &remaining[start_idx + EVENT_OPEN_TAG.len()..];
                continue;
            };

            let opening_tag = &after_start[..tag_end + 1];

            // Parse attributes from opening tag
            let id = Self::extract_attr(opening_tag, "id");
            let reply = Self::extract_attr(opening_tag, "reply");
            let topic = Self::extract_attr(opening_tag, "topic");
            let target = Self::extract_attr(opening_tag, "target");
            let target_instance = Self::extract_attr(opening_tag, "target_instance");
            let audience_instances = Self::extract_attr(opening_tag, "audience_instances");
            let require_delivery = Self::extract_attr(opening_tag, "require_delivery");
            let workspace_strategy = Self::extract_attr(opening_tag, "workspace_strategy");
            let session_strategy = Self::extract_attr(opening_tag, "session_strategy");
            let turn_action = Self::extract_attr(opening_tag, "turn_action");
            let spawn_instance = Self::extract_attr(opening_tag, "spawn_instance");

            let Some(topic) = topic else {
                remaining = &remaining[start_idx + tag_end + 1..];
                continue;
            };

            // Find the closing tag
            let content_start = &after_start[tag_end + 1..];
            let nested_event_idx = Self::find_event_start(content_start);
            let close_tag = Self::find_event_close_tag(content_start);

            let (payload, total_consumed) = if let Some((close_idx, close_tag_len)) = close_tag
                && nested_event_idx.is_none_or(|nested| nested > close_idx)
            {
                let payload = content_start[..close_idx].trim().to_string();
                let total_consumed = start_idx + tag_end + 1 + close_idx + close_tag_len;
                (payload, Some(total_consumed))
            } else {
                // ------------------------------------------------------------------
                // 容错:
                // - 在并行 TUI chat 场景里,`reply.human.message` 是 UI-only 的“回复输出 topic”.
                // - 但真实模型偶尔会输出 `<event ...>` 开头,随后因为截断/中止等原因缺失 `</event>`,
                //   导致 EventParser 丢事件,用户体验变成“问了但没回复”.
                // - 这里做一个极小、可控的容错: 仅对 `reply.human.message` 且该 tag 位于输出开头(忽略前导空白),
                //   并且后续不再出现任何新的 `<event ` 时,把 EOF 视为隐式 `</event>`.
                // - 这样可以最大化避免误把普通日志/示例文本解析成事件.
                // ------------------------------------------------------------------
                let prefix_is_blank = remaining[..start_idx].trim().is_empty();
                let is_last_event = Self::find_event_start(content_start).is_none();
                if topic == "reply.human.message" && prefix_is_blank && is_last_event {
                    (content_start.trim().to_string(), None)
                } else {
                    remaining = &remaining[start_idx + tag_end + 1..];
                    continue;
                }
            };

            let mut event = Event::new(topic, payload);

            if let Some(source) = &self.source {
                event = event.with_source(source.clone());
            }

            if let Some(id) = id {
                event = event.with_id(id);
            }

            if let Some(reply) = reply {
                event = event.with_reply(reply);
            }

            if let Some(target) = target {
                event = event.with_target(target);
            }

            if let Some(target_instance) = target_instance {
                event = event.with_target_instance(target_instance);
            }

            if audience_instances.is_some() || require_delivery.is_some() {
                let mut override_ = AudienceOverride::default();

                if let Some(list) = audience_instances {
                    // 逗号分隔：writer#1,writer#2
                    override_.instances = list
                        .split(',')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(HatInstanceId::new)
                        .collect();
                }

                if let Some(flag) = require_delivery {
                    override_.require_delivery = matches!(flag.as_str(), "true" | "1" | "yes");
                }

                event = event.with_audience_override(override_);
            }

            if let Some(strategy) = workspace_strategy
                .as_deref()
                .and_then(parse_workspace_strategy)
            {
                event = event.with_workspace_strategy(strategy);
            }

            if let Some(strategy) = session_strategy.as_deref().and_then(parse_session_strategy) {
                event = event.with_session_strategy(strategy);
            }

            if let Some(action) = turn_action.as_deref().and_then(parse_turn_action) {
                event = event.with_turn_action(action);
            }

            if let Some(flag) = spawn_instance {
                // 说明：
                // - 这是路由提示信号：只在 Supervisor 路由层生效。
                // - 与其他 bool flag 一致：接受 true/1/yes。
                let enabled = matches!(flag.as_str(), "true" | "1" | "yes");
                if enabled {
                    event = event.with_spawn_instance(true);
                }
            }

            events.push(event);

            // Move past this event
            if let Some(total_consumed) = total_consumed {
                remaining = &remaining[total_consumed..];
            } else {
                // EOF 容错分支：我们已经消费到末尾,无需再继续扫描.
                break;
            }
        }

        events
    }

    /// Extracts an attribute value from an XML-like tag.
    fn extract_attr(tag: &str, attr: &str) -> Option<String> {
        let pattern = format!("{attr}=\"");
        let start = tag.find(&pattern)?;
        let value_start = start + pattern.len();
        let rest = &tag[value_start..];
        let end = rest.find('"')?;
        Some(rest[..end].to_string())
    }

    /// 查找真正的 `<event ...>` / `<event>` opening tag 起点。
    ///
    /// 兼容:
    /// - `<event topic="...">`
    /// - `<event\n  topic="...">`
    /// - `<event>`
    ///
    /// 同时显式排除:
    /// - `<eventual>`
    /// - `<event-handler>`
    fn find_event_start(output: &str) -> Option<usize> {
        let mut search_from = 0;

        while let Some(relative_idx) = output[search_from..].find(EVENT_OPEN_TAG) {
            let start_idx = search_from + relative_idx;
            let next_byte = output
                .as_bytes()
                .get(start_idx + EVENT_OPEN_TAG.len())
                .copied();

            if next_byte.is_none_or(|byte| byte == b'>' || byte.is_ascii_whitespace()) {
                return Some(start_idx);
            }

            search_from = start_idx + 1;
        }

        None
    }

    /// 查找 event closing tag。
    ///
    /// 兼容两种形态:
    /// - 标准协议: `</event>`
    /// - 模型偶发 JSON/HTML 风格转义: `<\\/event>`
    fn find_event_close_tag(output: &str) -> Option<(usize, usize)> {
        let standard = output
            .find(EVENT_CLOSE_TAG)
            .map(|idx| (idx, EVENT_CLOSE_TAG.len()));
        let escaped = output
            .find(EVENT_CLOSE_TAG_ESCAPED)
            .map(|idx| (idx, EVENT_CLOSE_TAG_ESCAPED.len()));

        match (standard, escaped) {
            (Some(left), Some(right)) => Some(if left.0 <= right.0 { left } else { right }),
            (Some(found), None) | (None, Some(found)) => Some(found),
            (None, None) => None,
        }
    }

    /// Parses backpressure evidence from build.done event payload.
    ///
    /// Expected format:
    /// ```text
    /// tests: pass
    /// lint: pass
    /// typecheck: pass
    /// ```
    ///
    /// Note: ANSI escape codes are stripped before parsing to handle
    /// colorized CLI output.
    pub fn parse_backpressure_evidence(payload: &str) -> Option<BackpressureEvidence> {
        // Strip ANSI codes before checking for evidence strings
        let clean_payload = strip_ansi(payload);

        let tests_passed = clean_payload.contains("tests: pass");
        let lint_passed = clean_payload.contains("lint: pass");
        let typecheck_passed = clean_payload.contains("typecheck: pass");

        // Only return evidence if at least one check is mentioned
        if clean_payload.contains("tests:")
            || clean_payload.contains("lint:")
            || clean_payload.contains("typecheck:")
        {
            Some(BackpressureEvidence {
                tests_passed,
                lint_passed,
                typecheck_passed,
            })
        } else {
            None
        }
    }

    /// Extracts the payload of the last event matching the given topic.
    ///
    /// 说明:
    /// - 复用 `EventParser::parse()` 的 opening tag / 属性顺序 / 多行兼容逻辑。
    /// - 返回最后一次命中,避免旧事件盖掉最新事件。
    pub fn extract_last_payload_for_topic(output: &str, topic: &str) -> Option<String> {
        Self::new()
            .parse(output)
            .into_iter()
            .rev()
            .find(|event| event.topic.as_str() == topic)
            .map(|event| event.payload)
    }

    /// Checks if output contains the completion promise.
    ///
    /// Per spec: The promise must appear in the agent's final output,
    /// not inside an `<event>` tag payload. This function:
    /// 1. Returns false if the promise appears inside ANY event tag
    ///    (prevents accidental completion when agents discuss the promise)
    /// 2. Otherwise, only accepts the promise when it occupies its own line
    pub fn contains_promise(output: &str, promise: &str) -> bool {
        // Safety check: if promise appears inside any event tag, never complete
        if Self::promise_in_event_tags(output, promise) {
            return false;
        }
        let stripped = Self::strip_event_tags(output);
        // 把 completion promise 维持成控制面 token:
        // - 必须是事件外文本
        // - 还必须独占某一行
        stripped.lines().any(|line| line.trim() == promise)
    }

    /// Checks if the promise appears inside any event tag payload.
    pub fn promise_in_event_tags(output: &str, promise: &str) -> bool {
        let mut remaining = output;

        while let Some(start_idx) = Self::find_event_start(remaining) {
            let after_start = &remaining[start_idx..];

            // Find the end of the opening tag
            let Some(tag_end) = after_start.find('>') else {
                return after_start.contains(promise);
            };

            // Find the closing tag
            let content_start = &after_start[tag_end + 1..];
            let Some((close_idx, close_tag_len)) = Self::find_event_close_tag(content_start) else {
                return content_start.contains(promise);
            };

            let payload = &content_start[..close_idx];
            if payload.contains(promise) {
                return true;
            }

            // Move past this event
            let total_consumed = start_idx + tag_end + 1 + close_idx + close_tag_len;
            remaining = &remaining[total_consumed..];
        }

        false
    }

    /// Strips all `<event ...>...</event>` blocks from output.
    ///
    /// Returns the output with event tags removed, leaving only
    /// the "final output" text that should be checked for promises.
    fn strip_event_tags(output: &str) -> String {
        let mut result = String::with_capacity(output.len());
        let mut remaining = output;

        while let Some(start_idx) = Self::find_event_start(remaining) {
            // Add everything before this event tag
            result.push_str(&remaining[..start_idx]);

            let after_start = &remaining[start_idx..];

            // Find the closing tag
            if let Some((close_idx, close_tag_len)) = Self::find_event_close_tag(after_start) {
                // Skip past the entire event block
                remaining = &after_start[close_idx + close_tag_len..];
            } else {
                // 安全优先:
                // - 未闭合的 opening tag 之后都视为“仍在 event 内”.
                // - 这样 completion 检测不会把残缺 event 的 payload 误当成最终正文.
                remaining = "";
                break;
            }
        }

        // Add any remaining content after the last event
        result.push_str(remaining);
        result
    }
}

fn parse_workspace_strategy(raw: &str) -> Option<WorkspaceStrategy> {
    match raw.trim() {
        "shared" => Some(WorkspaceStrategy::Shared),
        "patch" => Some(WorkspaceStrategy::Patch),
        "worktree" => Some(WorkspaceStrategy::Worktree),
        _ => None,
    }
}

fn parse_session_strategy(raw: &str) -> Option<SessionStrategy> {
    match raw.trim() {
        "exec" => Some(SessionStrategy::Exec),
        "mcp" => Some(SessionStrategy::Mcp),
        "app_server" => Some(SessionStrategy::AppServer),
        _ => None,
    }
}

fn parse_turn_action(raw: &str) -> Option<TurnAction> {
    match raw.trim() {
        "start" => Some(TurnAction::Start),
        "steer" => Some(TurnAction::Steer),
        "interrupt" => Some(TurnAction::Interrupt),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_event() {
        let output = r#"
Some preamble text.
<event topic="impl.done">
Implemented the authentication module.
</event>
Some trailing text.
"#;
        let parser = EventParser::new();
        let events = parser.parse(output);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].topic.as_str(), "impl.done");
        assert!(events[0].payload.contains("authentication module"));
    }

    #[test]
    fn test_parse_event_with_target() {
        let output = r#"<event topic="handoff" target="reviewer">Please review</event>"#;
        let parser = EventParser::new();
        let events = parser.parse(output);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].target.as_ref().unwrap().as_str(), "reviewer");
    }

    #[test]
    fn test_parse_event_with_session_strategy() {
        let output = r#"<event topic="build.task" session_strategy="mcp">Do it</event>"#;
        let parser = EventParser::new();
        let events = parser.parse(output);

        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].session_strategy,
            Some(SessionStrategy::Mcp),
            "session_strategy should be parsed from event attributes"
        );
    }

    #[test]
    fn test_parse_event_with_spawn_instance() {
        let output =
            r#"<event topic="build.task" target="writer" spawn_instance="true">Do it</event>"#;
        let parser = EventParser::new();
        let events = parser.parse(output);

        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].spawn_instance,
            Some(true),
            "spawn_instance should be parsed from event attributes"
        );
    }

    #[test]
    fn test_parse_event_with_reply() {
        let output = r#"<event topic="build.done" reply="writer#1:7">Done</event>"#;
        let parser = EventParser::new();
        let events = parser.parse(output);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].reply.as_deref(), Some("writer#1:7"));
    }

    #[test]
    fn test_parse_event_with_multiline_opening_tag() {
        let output = r#"<event
  topic="reply.human.message"
  reply="E1"
>hello
</event>"#;
        let parser = EventParser::new();
        let events = parser.parse(output);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].topic.as_str(), "reply.human.message");
        assert_eq!(events[0].reply.as_deref(), Some("E1"));
        assert_eq!(events[0].payload, "hello");
    }

    #[test]
    fn test_parse_event_with_escaped_close_tag() {
        let output = r#"<event topic="integration.task">{"run_id":"e2e"}<\/event>"#;
        let parser = EventParser::new();
        let events = parser.parse(output);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].topic.as_str(), "integration.task");
        assert_eq!(events[0].payload, r#"{"run_id":"e2e"}"#);
    }

    #[test]
    fn test_parse_incomplete_reply_human_message_event_is_salvaged_at_eof() {
        let output = r#"<event topic="reply.human.message" reply="E1">hello"#;
        let parser = EventParser::new();
        let events = parser.parse(output);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].topic.as_str(), "reply.human.message");
        assert_eq!(events[0].reply.as_deref(), Some("E1"));
        assert_eq!(events[0].payload, "hello");
    }

    #[test]
    fn test_parse_incomplete_non_reply_event_is_ignored() {
        let output = r#"<event topic="build.task">do it"#;
        let parser = EventParser::new();
        let events = parser.parse(output);

        assert_eq!(events.len(), 0);
    }

    #[test]
    fn test_incomplete_reply_does_not_swallow_following_events() {
        let output = r#"<event topic="reply.human.message" reply="E1">hello
<event topic="impl.done">done</event>"#;
        let parser = EventParser::new();
        let events = parser.parse(output);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].topic.as_str(), "impl.done");
        assert_eq!(events[0].payload, "done");
    }

    #[test]
    fn test_parse_event_with_empty_reply_is_ignored() {
        // reply 为空字符串时,等价于没有 reply(避免落盘/传递无意义的 Some("")).
        let output = r#"<event topic="build.done" reply="">Done</event>"#;
        let parser = EventParser::new();
        let events = parser.parse(output);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].reply, None);
    }

    #[test]
    fn test_parse_multiple_events() {
        let output = r#"
<event topic="impl.started">Starting work</event>
Working on implementation...
<event topic="impl.done">Finished</event>
"#;
        let parser = EventParser::new();
        let events = parser.parse(output);

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].topic.as_str(), "impl.started");
        assert_eq!(events[1].topic.as_str(), "impl.done");
    }

    #[test]
    fn test_parse_with_source() {
        let output = r#"<event topic="impl.done">Done</event>"#;
        let parser = EventParser::new().with_source("implementer");
        let events = parser.parse(output);

        assert_eq!(events[0].source.as_ref().unwrap().as_str(), "implementer");
    }

    #[test]
    fn test_no_events() {
        let output = "Just regular output with no events.";
        let parser = EventParser::new();
        let events = parser.parse(output);

        assert!(events.is_empty());
    }

    #[test]
    fn test_contains_promise() {
        assert!(EventParser::contains_promise(
            "LOOP_COMPLETE",
            "LOOP_COMPLETE"
        ));
        assert!(EventParser::contains_promise(
            "Done.\nLOOP_COMPLETE\n",
            "LOOP_COMPLETE"
        ));
        assert!(!EventParser::contains_promise(
            "prefix LOOP_COMPLETE suffix",
            "LOOP_COMPLETE"
        ));
        assert!(!EventParser::contains_promise(
            "No promise here",
            "LOOP_COMPLETE"
        ));
    }

    #[test]
    fn test_contains_promise_ignores_event_payloads() {
        // Promise inside event payload should NOT be detected
        let output = r#"<event topic="build.task">Fix LOOP_COMPLETE detection</event>"#;
        assert!(!EventParser::contains_promise(output, "LOOP_COMPLETE"));

        // Promise inside event with acceptance criteria mentioning LOOP_COMPLETE
        let output = r#"<event topic="build.task">
## Task: Fix completion promise detection
- Given LOOP_COMPLETE appears inside an event tag
- Then it should be ignored
</event>"#;
        assert!(!EventParser::contains_promise(output, "LOOP_COMPLETE"));
    }

    #[test]
    fn test_contains_promise_ignores_multiline_event_payloads() {
        let output = r#"<event
  topic="build.task"
  target="writer"
>Fix LOOP_COMPLETE detection
</event>"#;
        assert!(EventParser::promise_in_event_tags(output, "LOOP_COMPLETE"));
        assert!(!EventParser::contains_promise(output, "LOOP_COMPLETE"));
    }

    #[test]
    fn test_contains_promise_ignores_incomplete_event_payloads() {
        let output = r#"<event topic="reply.human.message">hello LOOP_COMPLETE"#;
        assert!(EventParser::promise_in_event_tags(output, "LOOP_COMPLETE"));
        assert!(!EventParser::contains_promise(output, "LOOP_COMPLETE"));
    }

    #[test]
    fn test_extract_last_payload_for_topic_accepts_multiline_and_attribute_reordering() {
        let output = r#"
prefix
<event
  id="first"
  source="ralph"
  topic="analyze.complete"
>
{"verdict":"old"}
</event>
<event
  source="ralph"
  id="second"
  topic="analyze.complete"
>
{"verdict":"new"}
</event>
"#;

        let payload = EventParser::extract_last_payload_for_topic(output, "analyze.complete")
            .expect("expected analyze.complete payload");
        assert_eq!(payload, r#"{"verdict":"new"}"#);
    }

    #[test]
    fn test_extract_last_payload_for_topic_accepts_escaped_close_tag() {
        let output = r#"<event topic="analyze.complete">{"verdict":"ok"}<\/event>"#;
        let payload = EventParser::extract_last_payload_for_topic(output, "analyze.complete")
            .expect("expected escaped-close payload");
        assert_eq!(payload, r#"{"verdict":"ok"}"#);
    }

    #[test]
    fn test_extract_last_payload_for_topic_returns_none_when_topic_missing() {
        let output = r#"<event topic="build.done">done</event>"#;
        let payload = EventParser::extract_last_payload_for_topic(output, "analyze.complete");
        assert_eq!(payload, None);
    }

    #[test]
    fn test_contains_promise_detects_outside_events() {
        // Promise in prose should NOT be treated as completion
        let output = r#"<event topic="build.done">Task complete</event>
All done! LOOP_COMPLETE"#;
        assert!(!EventParser::contains_promise(output, "LOOP_COMPLETE"));

        // Promise on its own line outside event tags should still work
        let output = r#"LOOP_COMPLETE
<event topic="summary">Final summary</event>"#;
        assert!(EventParser::contains_promise(output, "LOOP_COMPLETE"));
    }

    #[test]
    fn test_contains_promise_mixed_content() {
        // Promise only in event payload, not in surrounding text
        let output = r#"Working on task...
<event topic="build.task">Fix LOOP_COMPLETE bug</event>
Still working..."#;
        assert!(!EventParser::contains_promise(output, "LOOP_COMPLETE"));

        // Promise in both event and surrounding text - should NOT complete
        // because promise appears inside an event tag (safety mechanism)
        let output = r#"All tasks done. LOOP_COMPLETE
<event topic="summary">Completed LOOP_COMPLETE task</event>"#;
        assert!(!EventParser::contains_promise(output, "LOOP_COMPLETE"));

        // Promise on its own line in mixed content should complete
        let output = r#"All tasks done.
LOOP_COMPLETE
<event topic="summary">Completed task</event>"#;
        assert!(EventParser::contains_promise(output, "LOOP_COMPLETE"));
    }

    #[test]
    fn test_promise_in_event_tags() {
        // Promise inside event payload
        let output = r#"<event topic="build.task">Fix LOOP_COMPLETE bug</event>"#;
        assert!(EventParser::promise_in_event_tags(output, "LOOP_COMPLETE"));

        // Promise not in any event
        let output = r#"<event topic="build.done">Task complete</event>"#;
        assert!(!EventParser::promise_in_event_tags(output, "LOOP_COMPLETE"));

        // No events at all
        let output = "Just regular text with LOOP_COMPLETE";
        assert!(!EventParser::promise_in_event_tags(output, "LOOP_COMPLETE"));

        // Multiple events, promise in second
        let output = r#"<event topic="a">first</event>
<event topic="b">contains LOOP_COMPLETE</event>"#;
        assert!(EventParser::promise_in_event_tags(output, "LOOP_COMPLETE"));
    }

    #[test]
    fn test_promise_in_event_tags_does_not_match_eventual() {
        let output = r#"<eventual>LOOP_COMPLETE</eventual>"#;
        assert!(!EventParser::promise_in_event_tags(output, "LOOP_COMPLETE"));
        assert!(!EventParser::contains_promise(output, "LOOP_COMPLETE"));
    }

    #[test]
    fn test_strip_event_tags() {
        // Single event
        let output = r#"before <event topic="test">payload</event> after"#;
        let stripped = EventParser::strip_event_tags(output);
        assert_eq!(stripped, "before  after");
        assert!(!stripped.contains("payload"));

        // Multiple events
        let output =
            r#"start <event topic="a">one</event> middle <event topic="b">two</event> end"#;
        let stripped = EventParser::strip_event_tags(output);
        assert_eq!(stripped, "start  middle  end");

        // No events
        let output = "just plain text";
        let stripped = EventParser::strip_event_tags(output);
        assert_eq!(stripped, "just plain text");
    }

    #[test]
    fn test_strip_event_tags_drops_incomplete_event_tail() {
        let output = r#"before <event topic="reply.human.message">payload LOOP_COMPLETE"#;
        let stripped = EventParser::strip_event_tags(output);
        assert_eq!(stripped, "before ");
    }

    #[test]
    fn test_strip_event_tags_accepts_escaped_close_tag() {
        let output = r#"before <event topic="test">payload<\/event> after"#;
        let stripped = EventParser::strip_event_tags(output);
        assert_eq!(stripped, "before  after");
    }

    #[test]
    fn test_contains_promise_ignores_payload_in_escaped_close_event() {
        let output = r#"<event topic="summary">contains LOOP_COMPLETE<\/event>"#;
        assert!(EventParser::promise_in_event_tags(output, "LOOP_COMPLETE"));
        assert!(!EventParser::contains_promise(output, "LOOP_COMPLETE"));
    }

    #[test]
    fn test_parse_backpressure_evidence_all_pass() {
        let payload = "tests: pass\nlint: pass\ntypecheck: pass";
        let evidence = EventParser::parse_backpressure_evidence(payload).unwrap();
        assert!(evidence.tests_passed);
        assert!(evidence.lint_passed);
        assert!(evidence.typecheck_passed);
        assert!(evidence.all_passed());
    }

    #[test]
    fn test_parse_backpressure_evidence_some_fail() {
        let payload = "tests: pass\nlint: fail\ntypecheck: pass";
        let evidence = EventParser::parse_backpressure_evidence(payload).unwrap();
        assert!(evidence.tests_passed);
        assert!(!evidence.lint_passed);
        assert!(evidence.typecheck_passed);
        assert!(!evidence.all_passed());
    }

    #[test]
    fn test_parse_backpressure_evidence_missing() {
        let payload = "Task completed successfully";
        let evidence = EventParser::parse_backpressure_evidence(payload);
        assert!(evidence.is_none());
    }

    #[test]
    fn test_parse_backpressure_evidence_partial() {
        let payload = "tests: pass\nSome other text";
        let evidence = EventParser::parse_backpressure_evidence(payload).unwrap();
        assert!(evidence.tests_passed);
        assert!(!evidence.lint_passed);
        assert!(!evidence.typecheck_passed);
        assert!(!evidence.all_passed());
    }

    #[test]
    fn test_parse_backpressure_evidence_with_ansi_codes() {
        let payload = "\x1b[0mtests: pass\x1b[0m\n\x1b[32mlint: pass\x1b[0m\ntypecheck: pass";
        let evidence = EventParser::parse_backpressure_evidence(payload).unwrap();
        assert!(evidence.tests_passed);
        assert!(evidence.lint_passed);
        assert!(evidence.typecheck_passed);
        assert!(evidence.all_passed());
    }

    #[test]
    fn test_strip_ansi_function() {
        // Test the internal strip_ansi function via parse_backpressure_evidence
        // Simple CSI reset sequence
        let payload = "\x1b[0mtests: pass\x1b[0m";
        let evidence = EventParser::parse_backpressure_evidence(payload).unwrap();
        assert!(evidence.tests_passed);

        // Bold green text
        let payload = "\x1b[1m\x1b[32mtests: pass\x1b[0m";
        let evidence = EventParser::parse_backpressure_evidence(payload).unwrap();
        assert!(evidence.tests_passed);

        // Multiple sequences mixed with content
        let payload = "\x1b[31mtests: fail\x1b[0m\n\x1b[32mlint: pass\x1b[0m";
        let evidence = EventParser::parse_backpressure_evidence(payload).unwrap();
        assert!(!evidence.tests_passed); // "tests: fail" not "tests: pass"
        assert!(evidence.lint_passed);
    }
}
