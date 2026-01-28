//! 并行 Supervisor TUI 的 chat 输入解析。
//!
//! 说明：
//! - 这里的目标是“把一行输入解析成要写入外部 JSONL 的事件意图”。
//! - 解析逻辑应保持简单、可测试，避免把 UI 交互细节与 IO 写入耦合在一起。

use serde_json::Value;

/// 用户在 chat 输入框提交的一次“意图”。
#[derive(Debug, Clone, PartialEq)]
pub enum ChatSubmit {
    /// 普通 human.message（可选定向）。
    HumanMessage {
        target_instance: Option<String>,
        payload: String,
    },
    /// gate.resolve（由 `!approve/!deny/!resolve` 触发）。
    GateResolve { gate_id: String, decision: Value },
}

/// 解析 chat 输入文本。
///
/// 支持：
/// - `@writer#2 hello` → human.message(target_instance=writer#2, payload="hello")
/// - `hello` → human.message(payload="hello")
/// - `!approve <gate_id>` → gate.resolve(decision=true)
/// - `!deny <gate_id>` → gate.resolve(decision=false)
/// - `!resolve <gate_id> <text...>` → gate.resolve(decision="<text...>")
pub fn parse_chat_submit(input: &str) -> Result<ChatSubmit, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("empty input".to_string());
    }

    // 1) gate commands
    if let Some(rest) = trimmed.strip_prefix('!') {
        let mut parts = rest.split_whitespace();
        let cmd = parts
            .next()
            .ok_or_else(|| "missing command after '!'".to_string())?;

        match cmd {
            "approve" => {
                let gate_id = parts
                    .next()
                    .ok_or_else(|| "usage: !approve <gate_id>".to_string())?;
                return Ok(ChatSubmit::GateResolve {
                    gate_id: gate_id.to_string(),
                    decision: Value::Bool(true),
                });
            }
            "deny" => {
                let gate_id = parts
                    .next()
                    .ok_or_else(|| "usage: !deny <gate_id>".to_string())?;
                return Ok(ChatSubmit::GateResolve {
                    gate_id: gate_id.to_string(),
                    decision: Value::Bool(false),
                });
            }
            "resolve" => {
                let gate_id = parts
                    .next()
                    .ok_or_else(|| "usage: !resolve <gate_id> <text>".to_string())?;

                // 注意：这里用 split_whitespace，会丢掉多余空格。
                // 但对“人类输入事件”来说，语义优先，空格保真不是刚需。
                let after = parts.collect::<Vec<_>>().join(" ");
                if after.trim().is_empty() {
                    return Err("usage: !resolve <gate_id> <text>".to_string());
                }

                return Ok(ChatSubmit::GateResolve {
                    gate_id: gate_id.to_string(),
                    decision: Value::String(after),
                });
            }
            _ => {
                return Err(format!("unknown command: !{cmd}"));
            }
        }
    }

    // 2) @instance prefix for directed messages
    if let Some(rest) = trimmed.strip_prefix('@') {
        let mut split_idx: Option<usize> = None;
        for (i, ch) in rest.char_indices() {
            if ch.is_whitespace() {
                split_idx = Some(i);
                break;
            }
        }

        let Some(i) = split_idx else {
            return Err("usage: @<instance_id> <message>".to_string());
        };

        let (instance_id, msg) = rest.split_at(i);
        let target = instance_id.trim();
        let payload = msg.trim();

        if target.is_empty() || payload.is_empty() {
            return Err("usage: @<instance_id> <message>".to_string());
        }

        return Ok(ChatSubmit::HumanMessage {
            target_instance: Some(target.to_string()),
            payload: payload.to_string(),
        });
    }

    // 3) default human message
    Ok(ChatSubmit::HumanMessage {
        target_instance: None,
        payload: trimmed.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_default_message() {
        let submit = parse_chat_submit("hello").unwrap();
        assert_eq!(
            submit,
            ChatSubmit::HumanMessage {
                target_instance: None,
                payload: "hello".to_string()
            }
        );
    }

    #[test]
    fn parse_directed_message() {
        let submit = parse_chat_submit("@writer#2 hello world").unwrap();
        assert_eq!(
            submit,
            ChatSubmit::HumanMessage {
                target_instance: Some("writer#2".to_string()),
                payload: "hello world".to_string()
            }
        );
    }

    #[test]
    fn parse_directed_message_requires_payload() {
        let err = parse_chat_submit("@writer#2").unwrap_err();
        assert!(err.contains("usage: @<instance_id> <message>"));
    }

    #[test]
    fn parse_approve_gate() {
        let submit = parse_chat_submit("!approve gate-1").unwrap();
        assert_eq!(
            submit,
            ChatSubmit::GateResolve {
                gate_id: "gate-1".to_string(),
                decision: Value::Bool(true)
            }
        );
    }

    #[test]
    fn parse_deny_gate() {
        let submit = parse_chat_submit("!deny gate-1").unwrap();
        assert_eq!(
            submit,
            ChatSubmit::GateResolve {
                gate_id: "gate-1".to_string(),
                decision: Value::Bool(false)
            }
        );
    }

    #[test]
    fn parse_resolve_gate_with_text() {
        let submit = parse_chat_submit("!resolve gate-1 please do it").unwrap();
        assert_eq!(
            submit,
            ChatSubmit::GateResolve {
                gate_id: "gate-1".to_string(),
                decision: Value::String("please do it".to_string())
            }
        );
    }

    #[test]
    fn parse_resolve_gate_requires_text() {
        let err = parse_chat_submit("!resolve gate-1").unwrap_err();
        assert!(err.contains("usage: !resolve <gate_id> <text>"));
    }
}
