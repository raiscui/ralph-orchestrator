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
        /// 可选会话策略覆盖(例如 app_server).
        session_strategy: Option<String>,
        /// 可选 turn 动作(start/steer/interrupt).
        turn_action: Option<String>,
    },
    /// gate.resolve（由 `!approve/!deny/!resolve` 触发）。
    GateResolve { gate_id: String, decision: Value },
    /// recoverable.continue（由 `!continue [failure_id]` 触发）。
    RecoverableContinue { failure_id: Option<String> },
}

/// 解析 chat 输入文本。
///
/// 支持：
/// - `@writer#2 hello` → human.message(target_instance=writer#2, payload="hello")
/// - `hello` → human.message(payload="hello")
/// - `!approve <gate_id>` → gate.resolve(decision=true)
/// - `!deny <gate_id>` → gate.resolve(decision=false)
/// - `!resolve <gate_id> <text...>` → gate.resolve(decision="<text...>")
/// - `!steer [@writer#2] <text...>` → human.message(session_strategy=app_server, turn_action=steer)
/// - `!interrupt [@writer#2]` → human.message(turn_action=interrupt)
/// - `!continue [failure_id]` → recoverable.continue(failure_id?)
pub fn parse_chat_submit(input: &str) -> Result<ChatSubmit, String> {
    let trimmed_all = input.trim();
    if trimmed_all.is_empty() {
        return Err("empty input".to_string());
    }

    // 仅解析第一行的控制前缀（!command / @instance）。
    // 其余行原样保留在 payload 里（用于多行 human message / resolve 文本）。
    let mut split = trimmed_all.splitn(2, '\n');
    let first_line = split.next().unwrap_or("");
    let rest = split.next().unwrap_or("");
    let first_trimmed = first_line.trim();

    // 1) gate commands
    if let Some(rest_first) = first_trimmed.strip_prefix('!') {
        let mut parts = rest_first.split_whitespace();
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
                let mut after = parts.collect::<Vec<_>>().join(" ");
                if !rest.is_empty() {
                    if !after.is_empty() {
                        after.push('\n');
                    }
                    after.push_str(rest);
                }
                if after.trim().is_empty() {
                    return Err("usage: !resolve <gate_id> <text>".to_string());
                }

                return Ok(ChatSubmit::GateResolve {
                    gate_id: gate_id.to_string(),
                    decision: Value::String(after),
                });
            }
            "steer" => {
                // 语法：
                // - !steer <text...>           (默认定向到 selected_instance)
                // - !steer @writer#2 <text...> (显式定向)
                let mut parts_vec = parts.collect::<Vec<_>>();
                let mut target_instance: Option<String> = None;

                if let Some(first) = parts_vec.first().copied()
                    && first.starts_with('@')
                {
                    target_instance = Some(first.trim_start_matches('@').to_string());
                    parts_vec.remove(0);
                }

                let mut after = parts_vec.join(" ");
                if !rest.is_empty() {
                    if !after.is_empty() {
                        after.push('\n');
                    }
                    after.push_str(rest);
                }

                if after.trim().is_empty() {
                    return Err("usage: !steer [@<instance_id>] <message>".to_string());
                }

                return Ok(ChatSubmit::HumanMessage {
                    target_instance,
                    payload: after,
                    session_strategy: Some("app_server".to_string()),
                    turn_action: Some("steer".to_string()),
                });
            }
            "interrupt" => {
                // 语法：
                // - !interrupt           (默认定向到 selected_instance)
                // - !interrupt @writer#2 (显式定向)
                let mut parts_vec = parts.collect::<Vec<_>>();
                let mut target_instance: Option<String> = None;

                if let Some(first) = parts_vec.first().copied()
                    && first.starts_with('@')
                {
                    target_instance = Some(first.trim_start_matches('@').to_string());
                    parts_vec.remove(0);
                }

                let mut after = parts_vec.join(" ");
                if !rest.is_empty() {
                    if !after.is_empty() {
                        after.push('\n');
                    }
                    after.push_str(rest);
                }

                return Ok(ChatSubmit::HumanMessage {
                    target_instance,
                    payload: after,
                    session_strategy: None,
                    turn_action: Some("interrupt".to_string()),
                });
            }
            "continue" => {
                // 语法：
                // - !continue              (由 Supervisor 根据当前 paused/scheduled failure 消歧)
                // - !continue failure-123  (显式指定 recoverable failure lifecycle)
                //
                // 注意:
                // - 普通中文 "继续分析这个问题" 不带 `!`,会继续走 human.message。
                // - control action 不接收多行 payload,避免把普通聊天误写成 retry 控制。
                let parts_vec = parts.collect::<Vec<_>>();
                if parts_vec.len() > 1 || !rest.trim().is_empty() {
                    return Err("usage: !continue [failure_id]".to_string());
                }

                let failure_id = parts_vec
                    .first()
                    .map(|value| value.trim())
                    .filter(|value| !value.is_empty())
                    .map(str::to_string);

                return Ok(ChatSubmit::RecoverableContinue { failure_id });
            }
            _ => {
                return Err(format!("unknown command: !{cmd}"));
            }
        }
    }

    // 2) @instance prefix for directed messages
    if let Some(rest_first) = first_trimmed.strip_prefix('@') {
        let mut split_idx: Option<usize> = None;
        for (i, ch) in rest_first.char_indices() {
            if ch.is_whitespace() {
                split_idx = Some(i);
                break;
            }
        }

        let (instance_id, msg_first) = if let Some(i) = split_idx {
            rest_first.split_at(i)
        } else {
            (rest_first, "")
        };
        let target = instance_id.trim();
        let msg_first = msg_first.trim();

        let mut payload = String::new();
        if !msg_first.is_empty() {
            payload.push_str(msg_first);
        }
        if !rest.is_empty() {
            if !payload.is_empty() {
                payload.push('\n');
            }
            payload.push_str(rest);
        }

        if target.is_empty() || payload.trim().is_empty() {
            return Err("usage: @<instance_id> <message>".to_string());
        }

        return Ok(ChatSubmit::HumanMessage {
            target_instance: Some(target.to_string()),
            payload,
            session_strategy: None,
            turn_action: None,
        });
    }

    // 3) default human message
    Ok(ChatSubmit::HumanMessage {
        target_instance: None,
        payload: trimmed_all.to_string(),
        session_strategy: None,
        turn_action: None,
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
                payload: "hello".to_string(),
                session_strategy: None,
                turn_action: None,
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
                payload: "hello world".to_string(),
                session_strategy: None,
                turn_action: None,
            }
        );
    }

    #[test]
    fn parse_directed_message_requires_payload() {
        let err = parse_chat_submit("@writer#2").unwrap_err();
        assert!(err.contains("usage: @<instance_id> <message>"));
    }

    #[test]
    fn parse_default_message_multiline_keeps_newlines() {
        let submit = parse_chat_submit("hello\nline2").unwrap();
        assert_eq!(
            submit,
            ChatSubmit::HumanMessage {
                target_instance: None,
                payload: "hello\nline2".to_string(),
                session_strategy: None,
                turn_action: None,
            }
        );
    }

    #[test]
    fn parse_directed_message_multiline_keeps_newlines() {
        let submit = parse_chat_submit("@writer#2 hello\nline2\nline3").unwrap();
        assert_eq!(
            submit,
            ChatSubmit::HumanMessage {
                target_instance: Some("writer#2".to_string()),
                payload: "hello\nline2\nline3".to_string(),
                session_strategy: None,
                turn_action: None,
            }
        );
    }

    #[test]
    fn parse_directed_message_allows_payload_in_following_lines() {
        let submit = parse_chat_submit("@writer#2\nline2").unwrap();
        assert_eq!(
            submit,
            ChatSubmit::HumanMessage {
                target_instance: Some("writer#2".to_string()),
                payload: "line2".to_string(),
                session_strategy: None,
                turn_action: None,
            }
        );
    }

    #[test]
    fn parse_steer_command_defaults_to_no_target() {
        let submit = parse_chat_submit("!steer go").unwrap();
        assert_eq!(
            submit,
            ChatSubmit::HumanMessage {
                target_instance: None,
                payload: "go".to_string(),
                session_strategy: Some("app_server".to_string()),
                turn_action: Some("steer".to_string()),
            }
        );
    }

    #[test]
    fn parse_steer_command_with_target() {
        let submit = parse_chat_submit("!steer @writer#2 go").unwrap();
        assert_eq!(
            submit,
            ChatSubmit::HumanMessage {
                target_instance: Some("writer#2".to_string()),
                payload: "go".to_string(),
                session_strategy: Some("app_server".to_string()),
                turn_action: Some("steer".to_string()),
            }
        );
    }

    #[test]
    fn parse_interrupt_command() {
        let submit = parse_chat_submit("!interrupt").unwrap();
        assert_eq!(
            submit,
            ChatSubmit::HumanMessage {
                target_instance: None,
                payload: String::new(),
                session_strategy: None,
                turn_action: Some("interrupt".to_string()),
            }
        );
    }

    #[test]
    fn parse_continue_command_without_failure_id() {
        let submit = parse_chat_submit("!continue").unwrap();
        assert_eq!(submit, ChatSubmit::RecoverableContinue { failure_id: None });
    }

    #[test]
    fn parse_continue_command_with_failure_id() {
        let submit = parse_chat_submit("!continue failure-123").unwrap();
        assert_eq!(
            submit,
            ChatSubmit::RecoverableContinue {
                failure_id: Some("failure-123".to_string())
            }
        );
    }

    #[test]
    fn parse_plain_chinese_continue_remains_human_message() {
        let submit = parse_chat_submit("继续分析这个问题").unwrap();
        assert_eq!(
            submit,
            ChatSubmit::HumanMessage {
                target_instance: None,
                payload: "继续分析这个问题".to_string(),
                session_strategy: None,
                turn_action: None,
            }
        );
    }

    #[test]
    fn parse_continue_command_rejects_extra_payload() {
        let err = parse_chat_submit("!continue failure-123 extra").unwrap_err();
        assert!(err.contains("usage: !continue [failure_id]"));
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
