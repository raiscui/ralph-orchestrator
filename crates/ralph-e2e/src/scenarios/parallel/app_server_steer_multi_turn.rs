use super::super::{AssertionBuilder, Assertions, ScenarioError, TestScenario};
use crate::Backend;
use crate::executor::{ExecutionResult, PromptSource, RalphExecutor, ScenarioConfig};
use crate::models::TestResult;
use async_trait::async_trait;
use ralph_proto::HatInstanceState;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

// =============================================================================
// ParallelAppServerSteerMultiTurnScenario - Validate Codex App Server turn/steer
// =============================================================================

/// E2E: 验证并行模式下,`ralph#1` 走 `session_strategy=app_server` 时可以在 turn in-flight
/// 期间通过外部 `ralph emit --turn-action steer` 注入多轮输入,并最终收敛到 `LOOP_COMPLETE`。
///
/// 设计目标(稳定性优先,不依赖真实网络/真实模型):
/// - 在 workspace 内生成一个 fake `codex` shim,实现:
///   - `codex app-server --listen stdio://` (最小 JSON-RPC 协议)
///   - `codex exec ... <prompt>` (兜底,避免意外路径报错)
/// - fake app-server 在收到 2 次 `turn/steer` 之前不会发送 `turn/completed`。
///   因此:
///   - steer 若未走 in-flight 控制通道,测试会稳定卡住并以 MaxRuntime 失败
///   - steer 若走通,会输出两次 marker 并输出 `LOOP_COMPLETE` 退出
pub struct ParallelAppServerSteerMultiTurnScenario {
    id: String,
    description: String,
    tier: String,
}

/// Fake codex app-server 启动时写到 stderr 的标记.
const FAKE_APP_SERVER_READY: &str = "FAKE_CODEX_APP_SERVER_READY";

/// 两轮 steer 的 marker(用于强匹配,避免“看起来像成功”的假阳性).
const STEER_MARKER_1: &str = "E2E_STEER_MARKER_1_42";
const STEER_MARKER_2: &str = "E2E_STEER_MARKER_2_42";

/// 用于验证“任务请求/执行/反馈”的具体内容.
const STEER_QUESTION_1: &str = "121+43=?";
const STEER_ANSWER_1: &str = "164";
const STEER_QUESTION_2: &str = "10+5=?";
const STEER_ANSWER_2: &str = "15";

impl ParallelAppServerSteerMultiTurnScenario {
    pub fn new() -> Self {
        Self {
            id: "parallel-app-server-steer-multi-turn".to_string(),
            description: "Validates Codex App Server turn/steer works in-flight with multiple steers (parallel runtime)".to_string(),
            tier: "Tier 8: Parallel Runtime".to_string(),
        }
    }

    fn write_fake_codex_shim(workspace: &Path) -> Result<PathBuf, ScenarioError> {
        // ---------------------------------------------------------------------
        // 说明:
        // - 我们不把 fake codex 落在仓库里(避免污染用户 PATH/系统环境)。
        // - 而是在 E2E workspace 内生成一个可执行脚本,并在本场景运行期间临时把它加到 PATH 的最前面。
        // - 这样可以做到:
        //   1) 场景确定性(不依赖真实 codex / 不消耗 token)
        //   2) 覆盖真实代码路径(CodexAppServerRuntime 会 spawn `codex app-server`)
        // ---------------------------------------------------------------------
        let bin_dir = workspace.join(".e2e/bin");
        std::fs::create_dir_all(&bin_dir).map_err(|e| {
            ScenarioError::SetupError(format!("failed to create {:?}: {e}", bin_dir))
        })?;

        let codex_path = bin_dir.join("codex");

        // 注意: 这里的脚本需要同时支持 `exec` 与 `app-server` 两种入口。
        // - `exec`: 兜底输出,避免某些非预期路径调用 codex 时直接报错
        // - `app-server`: 实现最小 JSON-RPC,供 CodexAppServerRuntime 驱动
        //
        // 关键时序:
        // - `turn/start` 后延迟发送 `turn/started`,以覆盖 pending_steers flush 分支
        // - turn 直到收到 2 次 steer 才完成,用于强验证“steer 必须走 in-flight”
        let script = r#"#!/usr/bin/env python3
# -*- coding: utf-8 -*-
#
# fake codex shim for ralph-e2e (deterministic, no network)
#
# 设计目标:
# - 覆盖 ralph-cli 的 CodexAppServerRuntime 真实代码路径:
#   - `codex app-server --listen stdio://` 走 JSON-RPC
#   - `codex exec ... <prompt>` 走一次性输出(兜底)
# - turn/steer 的验证策略:
#   - turn/start 后不立刻 completed,直到收到 2 次 turn/steer
#   - 若 steer 没有走 in-flight 控制通道,ralph#1 会卡住并以 MaxRuntime 失败

import json
import sys
import time

FAKE_READY = "__FAKE_READY__"

def eprint(msg: str) -> None:
    print(msg, file=sys.stderr, flush=True)

def send(obj) -> None:
    sys.stdout.write(json.dumps(obj, ensure_ascii=False) + "\n")
    sys.stdout.flush()

def run_exec(argv) -> int:
    # 兜底: 模拟 `codex exec ... <prompt>` 的最小输出
    eprint("FAKE_CODEX_EXEC_READY")
    sys.stdout.write("FAKE_CODEX_EXEC_OUTPUT\n")
    sys.stdout.flush()
    return 0

def run_app_server(argv) -> int:
    # app-server: 最小 JSON-RPC over stdio
    eprint(FAKE_READY)

    thread_id = "thread-1"
    turn_id = "turn-1"
    steer_count = 0
    turn_started_sent = False

    for raw in sys.stdin:
        line = raw.strip()
        if not line:
            continue

        try:
            msg = json.loads(line)
        except Exception as e:
            eprint(f"[fake-codex] invalid json: {e}")
            continue

        method = msg.get("method")
        msg_id = msg.get("id")

        # notifications: {method, params} (no id)
        if msg_id is None and method:
            # 这类通知通常是 client->server 的 lifecycle 信号(例如 initialized)。
            # 我们做轻量日志,便于人类确认 runner 真的在“收消息”。
            eprint(f"[fake-codex] recv notify method={method}")
            # client sends {"method":"initialized"}; we ignore.
            continue

        # requests: {id, method, params}
        if msg_id is None or method is None:
            continue

        # 人类可读 RPC trace: 让 human-log.md 能看到 runner 收到/回复了哪些请求
        eprint(f"[fake-codex] recv request method={method} id={msg_id}")

        if method == "initialize":
            send({"id": msg_id, "result": {}})
            eprint(f"[fake-codex] send response id={msg_id} result=ok")
            continue

        if method == "thread/start":
            # 先回 response,再发通知(更贴近真实形态)
            send({"id": msg_id, "result": {}})
            eprint(f"[fake-codex] send response id={msg_id} result=ok")
            send({"method": "thread/started", "params": {"thread": {"id": thread_id}}})
            eprint("[fake-codex] send notify method=thread/started")
            continue

        if method == "turn/start":
            send({"id": msg_id, "result": {}})
            eprint(f"[fake-codex] send response id={msg_id} result=ok")

            # 为了覆盖 pending_steers 缓冲分支:
            # - 这里刻意延迟 turn/started 通知
            time.sleep(0.8)

            turn_started_sent = True
            send({"method": "turn/started", "params": {"turn": {"id": turn_id}}})
            eprint("[fake-codex] send notify method=turn/started")
            send({"method": "item/agentMessage/delta", "params": {"delta": "WAITING_FOR_STEER\n"}})
            eprint("[fake-codex] send notify method=item/agentMessage/delta delta=WAITING_FOR_STEER")

            # 重要:
            # - 真实 codex app-server 会发送 `codex/event/task_started`,
            #   Ralph 会用它作为“可安全 steer”的门槛.
            # - 如果 fake 不发,并且 turn 在等待 steer 时不再有额外 notify,
            #   可能导致 pending_steers 无法被及时 flush(尤其当 steer 发生在前 2s 内)。
            #
            # 因此这里模拟真实语义,在 turn/started 之后立刻发布 task_started.
            send({"method": "codex/event/task_started", "params": {"msg": {"turn_id": turn_id}}})
            eprint("[fake-codex] send notify method=codex/event/task_started")

            # 重要: 此处不发送 completed,让 turn 保持 in-flight,直到收到 2 次 steer
            continue

        if method == "turn/steer":
            send({"id": msg_id, "result": {}})
            eprint(f"[fake-codex] send response id={msg_id} result=ok")
            params = msg.get("params", {})
            input_items = params.get("input", [])

            # 提取 steer 文本(我们只关心 text 类型)
            parts = []
            for item in input_items:
                if isinstance(item, dict) and item.get("type") == "text":
                    parts.append(item.get("text", ""))
            steer_text = "".join(parts).strip()
            eprint(f"[fake-codex] steer text={steer_text}")

            # ---------------------------------------------------------
            # 任务执行(确定性):
            # - 从 steer_text 中提取形如 "121+43=?" 的加法表达式
            # - 计算结果并作为 feedback 输出
            #
            # 这用于验证:
            # - 任务请求(steer payload)真实到达 runner
            # - runner 做了“可核对”的计算
            # - runner 把结果作为反馈输出
            # ---------------------------------------------------------
            def try_eval_addition(text: str):
                import re
                # 只支持最小表达式: "<int> + <int> = ?"
                m = re.search(r"(\d+)\s*\+\s*(\d+)\s*=\?", text)
                if not m:
                    return None
                return int(m.group(1)) + int(m.group(2))

            steer_count += 1
            send({"method": "item/agentMessage/delta", "params": {"delta": f"TASK_REQUEST[{steer_count}]: {steer_text}\n"}})
            eprint(f"[fake-codex] send notify method=item/agentMessage/delta delta=TASK_REQUEST[{steer_count}]")

            answer = try_eval_addition(steer_text)
            if answer is None:
                exec_line = "TASK_EXECUTE: no-addition-expression"
                feedback = "answer: <unknown>"
            else:
                exec_line = f"TASK_EXECUTE: addition"
                feedback = f"answer: {answer}"

            send({"method": "item/agentMessage/delta", "params": {"delta": f"{exec_line}\n"}})
            send({"method": "item/agentMessage/delta", "params": {"delta": f"TASK_FEEDBACK[{steer_count}]: {feedback}\n"}})
            eprint(f"[fake-codex] send notify method=item/agentMessage/delta delta=TASK_FEEDBACK[{steer_count}]")

            # 收到两轮 steer 后才完成 turn,并输出 completion promise
            if steer_count >= 2:
                send({"method": "item/agentMessage/delta", "params": {"delta": "LOOP_COMPLETE\n"}})
                eprint("[fake-codex] send notify method=item/agentMessage/delta delta=LOOP_COMPLETE")
                send({"method": "turn/completed", "params": {"turn": {"id": turn_id}}})
                eprint("[fake-codex] send notify method=turn/completed")
            continue

        if method == "turn/interrupt":
            send({"id": msg_id, "result": {}})
            eprint(f"[fake-codex] send response id={msg_id} result=ok")
            if turn_started_sent:
                send({"method": "turn/completed", "params": {"turn": {"id": turn_id}}})
                eprint("[fake-codex] send notify method=turn/completed")
            continue

        # 未知请求: 返回一个空结果,避免 client 卡死
        send({"id": msg_id, "result": {}})
        eprint(f"[fake-codex] send response id={msg_id} result=ok")

    return 0

def main(argv) -> int:
    # argv[0] 是脚本路径
    if len(argv) >= 2 and argv[1] == "app-server":
        return run_app_server(argv[1:])
    return run_exec(argv[1:])

if __name__ == "__main__":
    sys.exit(main(sys.argv))
"#
        .replace("__FAKE_READY__", FAKE_APP_SERVER_READY);

        std::fs::write(&codex_path, script).map_err(|e| {
            ScenarioError::SetupError(format!(
                "failed to write fake codex shim {:?}: {e}",
                codex_path
            ))
        })?;

        // Unix: 设置可执行位
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&codex_path)
                .map_err(|e| {
                    ScenarioError::SetupError(format!("failed to stat {:?}: {e}", codex_path))
                })?
                .permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&codex_path, perms).map_err(|e| {
                ScenarioError::SetupError(format!(
                    "failed to chmod +x fake codex shim {:?}: {e}",
                    codex_path
                ))
            })?;
        }

        Ok(bin_dir)
    }

    fn fake_app_server_used(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let ok = result.stdout.contains(FAKE_APP_SERVER_READY);
        let builder = AssertionBuilder::new("Fake codex app-server used")
            .expected(format!(
                "stdout contains app-server stderr marker: {FAKE_APP_SERVER_READY}"
            ))
            .actual(if ok {
                "Found marker in stdout".to_string()
            } else {
                "Missing marker in stdout".to_string()
            });
        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn steer_markers_observed(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let m1 = result.stdout.contains(STEER_MARKER_1);
        let m2 = result.stdout.contains(STEER_MARKER_2);
        let ok = m1 && m2;
        let builder = AssertionBuilder::new("Steer markers observed")
            .expected("stdout contains both steer markers (multi-turn)")
            .actual(format!("marker1={m1}, marker2={m2}"));
        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn steer_events_recorded(&self, result: &ExecutionResult) -> crate::models::Assertion {
        // 说明:
        // - 我们用 ExecutionResult.events 合并视角确认:
        //   1) `ralph emit` 确实执行了(外部事件落盘)
        //   2) payload 中包含 marker(便于排障)
        let m1 = result.events.iter().any(|e| {
            e.topic == "e2e.steer"
                && e.payload.contains(STEER_MARKER_1)
                && e.payload.contains(STEER_QUESTION_1)
        });
        let m2 = result.events.iter().any(|e| {
            e.topic == "e2e.steer"
                && e.payload.contains(STEER_MARKER_2)
                && e.payload.contains(STEER_QUESTION_2)
        });
        let ok = m1 && m2;

        let topics = result
            .events
            .iter()
            .map(|e| e.topic.as_str())
            .collect::<Vec<_>>();

        let builder = AssertionBuilder::new("Steer events recorded")
            .expected("events include two e2e.steer records with both markers+questions")
            .actual(format!("marker1={m1}, marker2={m2}, topics={topics:?}"));
        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn steer_answers_observed(&self, result: &ExecutionResult) -> crate::models::Assertion {
        // 说明:
        // - 这是本次补强的核心断言:
        //   - steer payload 里带了具体 question
        //   - fake app-server 必须输出可核对的 answer
        let a1 = result.stdout.contains(&format!("answer: {STEER_ANSWER_1}"));
        let a2 = result.stdout.contains(&format!("answer: {STEER_ANSWER_2}"));
        let ok = a1 && a2;

        let builder = AssertionBuilder::new("Steer answers observed")
            .expected("stdout contains both computed answers (task feedback)")
            .actual(format!(
                "answer1={a1}({STEER_ANSWER_1}), answer2={a2}({STEER_ANSWER_2})"
            ));
        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn loop_complete_detected(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let detected = result.termination_reason.as_deref() == Some("LOOP_COMPLETE");
        let builder = AssertionBuilder::new("LOOP_COMPLETE detected")
            .expected("termination_reason is LOOP_COMPLETE")
            .actual(format!(
                "termination_reason={:?}",
                result.termination_reason
            ));
        if detected {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn human_log_written(&self, executor: &RalphExecutor) -> crate::models::Assertion {
        let path = executor.workspace().join(".e2e/human-log.md");
        let content = std::fs::read_to_string(&path).ok();
        let ok = content.as_deref().is_some_and(|s| {
            !s.trim().is_empty()
                && s.contains(STEER_MARKER_1)
                && s.contains(STEER_MARKER_2)
                && s.contains(STEER_ANSWER_1)
                && s.contains(STEER_ANSWER_2)
        });

        let builder = AssertionBuilder::new("Human log written")
            .expected(".e2e/human-log.md exists and contains both steer markers")
            .actual(match content {
                Some(s) => format!("bytes={}", s.len()),
                None => format!("missing: {}", path.display()),
            });

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn write_human_log(
        &self,
        executor: &RalphExecutor,
        execution: &ExecutionResult,
        emit_cmds: &[String],
    ) -> Result<(), std::io::Error> {
        let dir = executor.workspace().join(".e2e");
        std::fs::create_dir_all(&dir)?;

        let evidence_lines = execution
            .stdout
            .lines()
            .filter(|l| {
                l.contains(FAKE_APP_SERVER_READY)
                    || l.contains("[fake-codex]")
                    || l.contains(STEER_MARKER_1)
                    || l.contains(STEER_MARKER_2)
                    || l.contains("TASK_REQUEST")
                    || l.contains("TASK_EXECUTE")
                    || l.contains("TASK_FEEDBACK")
                    || l.contains("answer:")
                    || l.contains("LOOP_COMPLETE")
            })
            .take(80)
            .map(|l| format!("- `{}`", l.trim_end()))
            .collect::<Vec<_>>()
            .join("\n");

        // `ralph emit` 的 stdout/stderr 也纳入 human log,让人类能看到“注入已被 runner 接受”。
        let emit_1_out = std::fs::read_to_string(dir.join("emit-1.stdout.txt")).ok();
        let emit_1_err = std::fs::read_to_string(dir.join("emit-1.stderr.txt")).ok();
        let emit_2_out = std::fs::read_to_string(dir.join("emit-2.stdout.txt")).ok();
        let emit_2_err = std::fs::read_to_string(dir.join("emit-2.stderr.txt")).ok();

        fn summarize_emit(io: Option<String>) -> String {
            let Some(text) = io else {
                return "(missing)".to_string();
            };

            let lines = text
                .lines()
                .map(|l| l.trim_end())
                .filter(|l| !l.is_empty())
                .take(5)
                .collect::<Vec<_>>();

            if lines.is_empty() {
                "(empty)".to_string()
            } else {
                lines.join(" / ")
            }
        }

        let content = format!(
            r#"# E2E Human Log: {id}

## 目标

- 验证 parallel runtime 下 `CodexAppServerRuntime` 的 `turn/steer` 能力能闭环。
- 通过外部 `ralph emit --turn-action steer` 在 turn in-flight 时注入两轮输入。

## Marker

- `{m1}`
- `{m2}`

## 任务内容(注入 payload)

- steer-1: question=`{q1}`, expect answer=`{a1}`
- steer-2: question=`{q2}`, expect answer=`{a2}`

## Fake app-server

- stderr marker: `{ready}`

## 注入命令(执行过)

```bash
{emit_cmds}
```

## 注入命令输出(摘录)

- emit-1 stdout: `{emit1_out}`
- emit-1 stderr: `{emit1_err}`
- emit-2 stdout: `{emit2_out}`
- emit-2 stderr: `{emit2_err}`

## 关键证据(摘录)

{evidence}

## 结论

- termination_reason: `{term:?}`
- exit_code: `{exit_code:?}`

## 产物路径

- stdout: `.e2e/stdout.txt`
- stderr: `.e2e/stderr.txt`
- emit-1 stdout: `.e2e/emit-1.stdout.txt`
- emit-1 stderr: `.e2e/emit-1.stderr.txt`
- emit-2 stdout: `.e2e/emit-2.stdout.txt`
- emit-2 stderr: `.e2e/emit-2.stderr.txt`
- 本文件: `.e2e/human-log.md`
"#,
            id = self.id,
            m1 = STEER_MARKER_1,
            m2 = STEER_MARKER_2,
            q1 = STEER_QUESTION_1,
            a1 = STEER_ANSWER_1,
            q2 = STEER_QUESTION_2,
            a2 = STEER_ANSWER_2,
            ready = FAKE_APP_SERVER_READY,
            emit_cmds = emit_cmds.join("\n"),
            emit1_out = summarize_emit(emit_1_out),
            emit1_err = summarize_emit(emit_1_err),
            emit2_out = summarize_emit(emit_2_out),
            emit2_err = summarize_emit(emit_2_err),
            evidence = if evidence_lines.trim().is_empty() {
                "(missing)".to_string()
            } else {
                evidence_lines
            },
            term = execution.termination_reason,
            exit_code = execution.exit_code,
        );

        std::fs::write(dir.join("human-log.md"), content)?;
        Ok(())
    }

    async fn wait_for_ralph_running(workspace: &Path) -> Result<(), String> {
        let deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < deadline {
            if let Ok(snapshot) = super::read_agents_snapshot(workspace) {
                if snapshot
                    .instances
                    .iter()
                    .any(|i| i.instance_id == "ralph#1" && i.state == HatInstanceState::Running)
                {
                    return Ok(());
                }
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        Err("timeout: ralph#1 did not reach Running state in agents.json".to_string())
    }

    async fn emit_steer(
        ralph_bin: &Path,
        workspace: &Path,
        marker: &str,
        question: &str,
    ) -> Result<(String, String), String> {
        use tokio::process::Command;

        let payload = format!("marker: {marker}; question: {question}");
        let output = Command::new(ralph_bin)
            .arg("emit")
            .arg("e2e.steer")
            .arg(payload)
            .arg("--target-instance")
            .arg("ralph#1")
            .arg("--turn-action")
            .arg("steer")
            .arg("--session-strategy")
            .arg("app_server")
            .current_dir(workspace)
            .output()
            .await
            .map_err(|e| format!("failed to run ralph emit: {e}"))?;

        if !output.status.success() {
            return Err(format!(
                "ralph emit failed: status={:?}, stderr={}",
                output.status.code(),
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        Ok((
            String::from_utf8_lossy(&output.stdout).to_string(),
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
    }
}

impl Default for ParallelAppServerSteerMultiTurnScenario {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TestScenario for ParallelAppServerSteerMultiTurnScenario {
    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn tier(&self) -> &str {
        &self.tier
    }

    fn supported_backends(&self) -> Vec<Backend> {
        // 说明:
        // - 该能力只在 Codex App Server runtime 下存在,因此仅对 Codex backend 开启。
        // - 但本场景用 fake codex shim 做确定性回归,不会实际发网请求。
        vec![Backend::Codex]
    }

    fn setup(&self, workspace: &Path, _backend: Backend) -> Result<ScenarioConfig, ScenarioError> {
        // 创建 `.agent/`（部分路径假设其存在）
        std::fs::create_dir_all(workspace.join(".agent")).map_err(|e| {
            ScenarioError::SetupError(format!("failed to create .agent directory: {e}"))
        })?;

        // 写入 fake codex shim(生成在 workspace 内,后续通过 PATH 注入)
        let _shim_dir = Self::write_fake_codex_shim(workspace)?;

        // 最小并行配置: 只需要 ralph#1 跑起来,并能被 steer 注入即可。
        let config_content = r#"cli:
  backend: "codex"

event_loop:
  prompt: |
    # E2E: parallel-app-server-steer-multi-turn

    你正在运行一个 E2E 场景。
    该场景会在运行中对 `ralph#1` 注入两次 turn/steer。
    当你观察到输出包含两次 marker 后,输出 `LOOP_COMPLETE` 并停止。
  completion_promise: "LOOP_COMPLETE"
  max_iterations: 6
  max_runtime_seconds: 60

parallel:
  enabled: true
  autoscale:
    max_running_jobs: 2
    dynamic_idle_ttl_secs: 30
  permissions:
    worktree: allow
    hooks: allow

hats: {}
"#;

        std::fs::write(workspace.join("ralph.yml"), config_content)
            .map_err(|e| ScenarioError::SetupError(format!("failed to write ralph.yml: {e}")))?;

        Ok(ScenarioConfig {
            config_file: "ralph.yml".into(),
            prompt: PromptSource::Config,
            max_iterations: 10,
            timeout: Duration::from_secs(180),
            extra_args: vec!["--no-tui".to_string()],
        })
    }

    async fn run(
        &self,
        executor: &RalphExecutor,
        config: &ScenarioConfig,
    ) -> Result<TestResult, ScenarioError> {
        // -----------------------------------------------------------------
        // PATH 注入:
        // - CodexAppServerRuntime 内部会 spawn `codex app-server ...`
        // - 因此我们必须让本场景的 ralph 进程优先找到 workspace 内的 fake `codex`
        // -----------------------------------------------------------------
        let workspace = executor.workspace().clone();
        let fake_bin_dir = workspace.join(".e2e/bin");
        let old_path = std::env::var("PATH").unwrap_or_default();
        let injected_path = format!("{}:{}", fake_bin_dir.display(), old_path);
        let extra_env = vec![("PATH".to_string(), injected_path)];

        // 并发注入 steer:
        // - 等 `ralph#1` 进入 Running 后再注入,避免降级成普通 pending 事件
        let ralph_bin = executor.ralph_binary();
        let inject_workspace = workspace.clone();
        let inject = tokio::spawn(async move {
            Self::wait_for_ralph_running(&inject_workspace).await?;

            let cmd1 = format!(
                "ralph emit e2e.steer \"marker: {STEER_MARKER_1}; question: {STEER_QUESTION_1}\" --target-instance ralph#1 --turn-action steer --session-strategy app_server"
            );
            let (out1, err1) = Self::emit_steer(
                &ralph_bin,
                &inject_workspace,
                STEER_MARKER_1,
                STEER_QUESTION_1,
            )
            .await?;

            // 让 turn/started 有机会先到达,覆盖“pending_steers flush + 直接 steer”两条路径。
            tokio::time::sleep(Duration::from_millis(1200)).await;

            let cmd2 = format!(
                "ralph emit e2e.steer \"marker: {STEER_MARKER_2}; question: {STEER_QUESTION_2}\" --target-instance ralph#1 --turn-action steer --session-strategy app_server"
            );
            let (out2, err2) = Self::emit_steer(
                &ralph_bin,
                &inject_workspace,
                STEER_MARKER_2,
                STEER_QUESTION_2,
            )
            .await?;

            Ok::<_, String>((vec![cmd1, cmd2], out1, err1, out2, err2))
        });

        let start = Instant::now();
        let execution = executor
            .run_with_extra_env(config, &extra_env)
            .await
            .map_err(|e| ScenarioError::ExecutionError(format!("ralph execution failed: {e}")))?;

        let duration = start.elapsed();

        // 如果 inject 失败,也尽量保留 stdout/stderr artifacts,然后把失败转成断言。
        let inject_res: Result<(Vec<String>, String, String, String, String), String> =
            match inject.await {
                Ok(res) => res,
                Err(e) => Err(format!("steer injector task panicked: {e}")),
            };

        let mut emit_cmds = Vec::new();
        if let Ok((cmds, out1, err1, out2, err2)) = &inject_res {
            emit_cmds = cmds.clone();
            // 把 emit 的 stdout/stderr 也落盘一份,便于排障.
            let dir = executor.workspace().join(".e2e");
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::fs::write(dir.join("emit-1.stdout.txt"), out1);
            let _ = std::fs::write(dir.join("emit-1.stderr.txt"), err1);
            let _ = std::fs::write(dir.join("emit-2.stdout.txt"), out2);
            let _ = std::fs::write(dir.join("emit-2.stderr.txt"), err2);
        }

        // Human log：即使断言失败也尽量落盘,方便排障.
        let _ = self.write_human_log(executor, &execution, &emit_cmds);

        let mut assertions = vec![
            Assertions::response_received(&execution),
            Assertions::exit_code_success_or_limit(&execution),
            Assertions::no_timeout(&execution),
            Assertions::duration_within(&execution, Duration::from_secs(120)),
            self.fake_app_server_used(&execution),
            self.steer_events_recorded(&execution),
            self.steer_markers_observed(&execution),
            self.steer_answers_observed(&execution),
            self.loop_complete_detected(&execution),
            self.human_log_written(executor),
        ];

        // inject 本身也要变成断言,否则会出现“输出碰巧包含 marker 但注入失败”的假阳性。
        let inject_ok = inject_res.is_ok();
        assertions.push({
            let builder = AssertionBuilder::new("Steer injector succeeded")
                .expected("ralph emit (steer) runs twice successfully")
                .actual(match &inject_res {
                    Ok(_) => "ok=true".to_string(),
                    Err(e) => format!("ok=false, error={e}"),
                });
            if inject_ok {
                builder.passed().build()
            } else {
                builder.failed().build()
            }
        });

        let all_passed = assertions.iter().all(|a| a.passed);

        Ok(TestResult {
            scenario_id: self.id.clone(),
            scenario_description: self.description.clone(),
            backend: String::new(), // runner 会填充
            tier: self.tier.clone(),
            passed: all_passed,
            assertions,
            duration,
        })
    }
}
