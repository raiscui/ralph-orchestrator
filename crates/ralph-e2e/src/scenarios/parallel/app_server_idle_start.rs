use super::super::{AssertionBuilder, Assertions, ScenarioError, TestScenario};
use crate::Backend;
use crate::executor::{ExecutionResult, PromptSource, RalphExecutor, ScenarioConfig};
use crate::models::TestResult;
use async_trait::async_trait;
use ralph_proto::HatInstanceState;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

// =============================================================================
// ParallelAppServerIdleStartScenario - Idle start + two human.messages (fake)
// =============================================================================

/// E2E(fake,0 token):
/// - 以 `--idle-start` 启动并行 Ralph,确保启动后不触发任何 job(真正待机).
/// - 确认 `ralph#1` 长时间保持 Idle(超过 max_runtime_seconds 也不退出).
/// - 第一次外部 `human.message` 只触发 warmup turn,输出 ack,但**不** `LOOP_COMPLETE`.
/// - 然后再次等待超过 `max_runtime_seconds`,确认“首条消息后”也不会被 `MaxRuntime` 收掉。
/// - 第二次 `human.message` 再触发 finish turn,输出可核对的 `answer: 164/15` 与 `LOOP_COMPLETE`.
///
/// 设计目标:
/// - 覆盖真实 `CodexAppServerRuntime` 代码路径,但不依赖真实 codex / 不消耗 token:
///   - 在 workspace 内生成 fake `codex` shim,实现 `codex app-server --listen stdio://`。
/// - human-log.md 必须可审计:
///   - 两次注入命令 + emit stdout/stderr 摘录
///   - runner 的 stdout 摘录(含 warmup ack / answers / LOOP_COMPLETE)
///   - idle 期间的 agents.json 证据(ralph#1=Idle)
#[deprecated(since = "2.3.0", note = "use the declarative YAML under scenarios/parallel-app-server-idle-start.yaml")]
pub struct ParallelAppServerIdleStartScenario {
    id: String,
    description: String,
    tier: String,
}

/// Fake codex app-server 启动时写到 stderr 的标记.
const FAKE_APP_SERVER_READY: &str = "FAKE_CODEX_APP_SERVER_IDLE_READY";

/// 用于确保“注入 payload 里包含具体任务内容”的 marker.
const IDLE_START_MARKER: &str = "E2E_IDLE_START_MARKER_42";
const WARMUP_PHASE: &str = "warmup";
const FINISH_PHASE: &str = "finish";
const WARMUP_ACK: &str = "IDLE_START_WARMUP_ACK";
const WAITING_FOR_SECOND_MESSAGE: &str = "WAITING_FOR_SECOND_MESSAGE";

/// 任务内容(可核对).
const QUESTION_1: &str = "121+43=?";
const ANSWER_1: &str = "164";
const QUESTION_2: &str = "10+5=?";
const ANSWER_2: &str = "15";

impl ParallelAppServerIdleStartScenario {
    pub fn new() -> Self {
        Self {
            id: "parallel-app-server-idle-start".to_string(),
            description: "Validates parallel --idle-start survives pre/post-first-message max_runtime windows and completes on second human.message via app-server (fake codex shim)".to_string(),
            tier: "Tier 8: Parallel Runtime".to_string(),
        }
    }

    fn write_fake_codex_shim(workspace: &Path) -> Result<PathBuf, ScenarioError> {
        let bin_dir = workspace.join(".e2e/bin");
        std::fs::create_dir_all(&bin_dir).map_err(|e| {
            ScenarioError::SetupError(format!("failed to create {:?}: {e}", bin_dir))
        })?;

        let codex_path = bin_dir.join("codex");

        // 说明:
        // - 该 fake shim 只实现本场景需要的最小协议:
        //   - initialize / thread/start / turn/start
        // - turn/start 收到 input 后按 `phase` 分两轮:
        //   - `warmup`: 只输出 ack,不输出 `LOOP_COMPLETE`
        //   - `finish`: 输出 TASK_* + answer + `LOOP_COMPLETE`
        let script = r#"#!/usr/bin/env python3
# -*- coding: utf-8 -*-
#
# fake codex shim for ralph-e2e (idle-start)
#
# 目标:
# - 覆盖 `codex app-server` 的真实调用路径,但不依赖真实 codex
# - 在收到 turn/start 的 input 后按 warmup / finish 两轮返回

import json
import re
import sys
import time

FAKE_READY = "__FAKE_READY__"

def eprint(msg: str) -> None:
    print(msg, file=sys.stderr, flush=True)

def send(obj) -> None:
    sys.stdout.write(json.dumps(obj, ensure_ascii=False) + "\n")
    sys.stdout.flush()

def run_exec(argv) -> int:
    # 兜底: 模拟 `codex exec ...` 的最小输出(避免非预期路径报错)
    eprint("FAKE_CODEX_EXEC_READY")
    sys.stdout.write("FAKE_CODEX_EXEC_OUTPUT\n")
    sys.stdout.flush()
    return 0

def extract_all_text(value) -> str:
    # 从任意 JSON 结构中提取所有 text 字段(尽量稳健,不依赖具体 schema)
    parts = []
    if isinstance(value, dict):
        for k, v in value.items():
            if k == "text" and isinstance(v, str):
                parts.append(v)
            else:
                parts.append(extract_all_text(v))
    elif isinstance(value, list):
        for item in value:
            parts.append(extract_all_text(item))
    return "".join(parts)

def run_app_server(argv) -> int:
    eprint(FAKE_READY)

    thread_id = "thread-1"
    turn_counter = 0

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
            eprint(f"[fake-codex] recv notify method={method}")
            continue

        if msg_id is None or method is None:
            continue

        eprint(f"[fake-codex] recv request method={method} id={msg_id}")

        if method == "initialize":
            send({"id": msg_id, "result": {}})
            eprint(f"[fake-codex] send response id={msg_id} result=ok")
            continue

        if method == "thread/start":
            send({"id": msg_id, "result": {}})
            eprint(f"[fake-codex] send response id={msg_id} result=ok")
            send({"method": "thread/started", "params": {"thread": {"id": thread_id}}})
            eprint("[fake-codex] send notify method=thread/started")
            continue

        if method == "turn/start":
            turn_counter += 1
            turn_id = f"turn-{turn_counter}"
            send({"id": msg_id, "result": {}})
            eprint(f"[fake-codex] send response id={msg_id} result=ok")

            # 让 client 有机会进入“active turn”状态
            time.sleep(0.2)
            send({"method": "turn/started", "params": {"turn": {"id": turn_id}}})
            eprint("[fake-codex] send notify method=turn/started")
            send({"method": "codex/event/task_started", "params": {"msg": {"turn_id": turn_id}}})
            eprint("[fake-codex] send notify method=codex/event/task_started")

            params = msg.get("params", {})
            input_items = params.get("input", [])
            all_text = extract_all_text(input_items)

            # 说明:
            # - 不同版本/实现的 app-server schema 可能会变化(例如 input items 的结构/字段名).
            # - 为了让 E2E 更稳,这里用整条 JSON 文本做匹配,避免只在某个字段里找导致漏匹配。
            raw_text = json.dumps(msg, ensure_ascii=False)
            combined = raw_text + "\n" + all_text

            m = re.search(r"marker:\s*([A-Za-z0-9_-]+)", combined)
            marker = m.group(1) if m else "<missing>"
            phase_match = re.search(r"phase:\s*([A-Za-z0-9_-]+)", combined)
            phase = phase_match.group(1) if phase_match else "finish"

            # 输出 marker,用于强审计
            send({"method": "item/agentMessage/delta", "params": {"delta": f"MARKER: {marker}\n"}})
            eprint(f"[fake-codex] parsed marker={marker} phase={phase}")

            # ---------------------------------------------------------
            # warmup turn:
            # - 只输出 ack,不输出 LOOP_COMPLETE
            # - 这样 Ralph 会继续留在 idle-start 会话里等待第二条 human.message
            # ---------------------------------------------------------
            if phase == "warmup":
                send({"method": "item/agentMessage/delta", "params": {"delta": "IDLE_START_WARMUP_ACK\n"}})
                send({"method": "item/agentMessage/delta", "params": {"delta": "WAITING_FOR_SECOND_MESSAGE\n"}})
                eprint("[fake-codex] send warmup ack without LOOP_COMPLETE")
                time.sleep(0.8)
                send({"method": "turn/completed", "params": {"turn": {"id": turn_id}}})
                eprint("[fake-codex] send notify method=turn/completed")
                send({"method": "codex/event/task_complete", "params": {"msg": {"turn_id": turn_id}}})
                eprint("[fake-codex] send notify method=codex/event/task_complete")
                continue

            # 找出所有加法表达式并计算
            exprs = []
            seen_expr = set()
            for m in re.finditer(r"(\d+)\s*\+\s*(\d+)\s*=\s*\?", combined):
                a = int(m.group(1))
                b = int(m.group(2))
                expr = f"{a}+{b}=?"
                if expr in seen_expr:
                    continue
                seen_expr.add(expr)
                exprs.append((expr, a + b))
                if len(exprs) >= 2:
                    break

            # 若没找到表达式,仍输出一条可诊断反馈
            if not exprs:
                send({"method": "item/agentMessage/delta", "params": {"delta": "TASK_FEEDBACK: answer: <unknown>\n"}})
            else:
                for i, (expr, ans) in enumerate(exprs, start=1):
                    send({"method": "item/agentMessage/delta", "params": {"delta": f"TASK_REQUEST[{i}]: question: {expr}\n"}})
                    send({"method": "item/agentMessage/delta", "params": {"delta": f"TASK_EXECUTE[{i}]: addition\n"}})
                    send({"method": "item/agentMessage/delta", "params": {"delta": f"TASK_FEEDBACK[{i}]: answer: {ans}\n"}})

            send({"method": "item/agentMessage/delta", "params": {"delta": "LOOP_COMPLETE\n"}})
            eprint("[fake-codex] send notify method=item/agentMessage/delta delta=LOOP_COMPLETE")

            # 兼容两条完成路径: turn/completed + task_complete
            send({"method": "turn/completed", "params": {"turn": {"id": turn_id}}})
            eprint("[fake-codex] send notify method=turn/completed")
            send({"method": "codex/event/task_complete", "params": {"msg": {"turn_id": turn_id}}})
            eprint("[fake-codex] send notify method=codex/event/task_complete")
            continue

        # 未知请求: 返回空结果避免 client 卡死
        send({"id": msg_id, "result": {}})
        eprint(f"[fake-codex] send response id={msg_id} result=ok")

    return 0

def main(argv) -> int:
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

    fn answers_observed(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let a1 = result.stdout.contains(&format!("answer: {ANSWER_1}"));
        let a2 = result.stdout.contains(&format!("answer: {ANSWER_2}"));
        let ok = a1 && a2;
        let builder = AssertionBuilder::new("Answers observed")
            .expected("stdout contains both computed answers (164 and 15)")
            .actual(format!("answer164={a1}, answer15={a2}"));
        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn warmup_ack_observed(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let ack = result.stdout.contains(WARMUP_ACK);
        let waiting = result.stdout.contains(WAITING_FOR_SECOND_MESSAGE);
        let ok = ack && waiting;
        let builder = AssertionBuilder::new("Warmup ack observed")
            .expected("stdout contains warmup ack and waiting-for-second-message markers")
            .actual(format!("warmup_ack={ack}, waiting_for_second={waiting}"));
        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn marker_observed(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let ok = result.stdout.contains(IDLE_START_MARKER);
        let builder = AssertionBuilder::new("Marker observed")
            .expected(format!("stdout contains marker: {IDLE_START_MARKER}"))
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

    fn survived_two_runtime_windows(
        &self,
        result: &ExecutionResult,
        pre_wait: Duration,
        post_warmup_wait: Duration,
    ) -> crate::models::Assertion {
        let minimum = pre_wait + post_warmup_wait;
        let ok = result.duration >= minimum;
        let builder = AssertionBuilder::new("Session survived pre/post max_runtime windows")
            .expected(format!(
                "execution duration >= {:?} (pre_wait + post_warmup_wait)",
                minimum
            ))
            .actual(format!("duration={:?}", result.duration));
        if ok {
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
                && s.contains(IDLE_START_MARKER)
                && s.contains(WARMUP_ACK)
                && s.contains(WAITING_FOR_SECOND_MESSAGE)
                && s.contains(QUESTION_1)
                && s.contains(QUESTION_2)
                && s.contains(ANSWER_1)
                && s.contains(ANSWER_2)
                && s.contains("LOOP_COMPLETE")
                && s.contains("emit-2 stdout")
                && s.contains("[ralph#1:out:job=")
        });

        let builder = AssertionBuilder::new("Human log written")
            .expected(".e2e/human-log.md exists and contains warmup ack + final answers + LOOP_COMPLETE + emit-2 evidence + at least one [ralph#1:out:job=...] line")
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
        pre_wait: Duration,
        post_warmup_wait: Duration,
        agents_before: Option<String>,
    ) -> Result<(), std::io::Error> {
        let dir = executor.workspace().join(".e2e");
        std::fs::create_dir_all(&dir)?;

        let evidence_lines = execution
            .stdout
            .lines()
            .filter(|l| {
                l.contains(FAKE_APP_SERVER_READY)
                    || l.contains("[fake-codex]")
                    || l.contains("[supervisor] instances")
                    || l.contains("[ralph#1:state]")
                    || l.contains("[ralph#1:out:job=")
                    || l.contains(IDLE_START_MARKER)
                    || l.contains(WARMUP_ACK)
                    || l.contains(WAITING_FOR_SECOND_MESSAGE)
                    || l.contains("TASK_REQUEST")
                    || l.contains("TASK_EXECUTE")
                    || l.contains("TASK_FEEDBACK")
                    || l.contains("answer:")
                    || l.contains("LOOP_COMPLETE")
            })
            .take(120)
            .map(|l| format!("- `{}`", l.trim_end()))
            .collect::<Vec<_>>()
            .join("\n");

        let emit_1_out = std::fs::read_to_string(dir.join("emit-1.stdout.txt")).ok();
        let emit_1_err = std::fs::read_to_string(dir.join("emit-1.stderr.txt")).ok();
        let emit_2_out = std::fs::read_to_string(dir.join("emit-2.stdout.txt")).ok();
        let emit_2_err = std::fs::read_to_string(dir.join("emit-2.stderr.txt")).ok();

        fn summarize(io: Option<String>) -> String {
            let Some(text) = io else {
                return "(missing)".to_string();
            };

            let lines = text
                .lines()
                .map(|l| l.trim_end())
                .filter(|l| !l.is_empty())
                .take(8)
                .collect::<Vec<_>>();

            if lines.is_empty() {
                "(empty)".to_string()
            } else {
                lines.join(" / ")
            }
        }

        let agents_before_summary = agents_before.unwrap_or_else(|| "(missing)".to_string());

        let content = format!(
            r"# E2E Human Log: {id}

## 目标

- 启动并行 Ralph(`--idle-start`)后保持待机(不触发 job)。
- 第一次超时级等待后,emit warmup `human.message`,只输出 ack,不 `LOOP_COMPLETE`。
- 首次 turn 结束后再次等待超过 max_runtime_seconds,证明“首条消息后”也不计时。
- 第二次 emit finish `human.message`,输出最终 answer 与 `LOOP_COMPLETE`。

## Marker & Questions

- marker: `{marker}`
- warmup phase: `{warmup_phase}` -> expect `{warmup_ack}` / `{waiting_for_second}`
- finish phase: `{finish_phase}`
- q1: `{q1}` -> expect `{a1}`
- q2: `{q2}` -> expect `{a2}`

## Wait windows

- pre-first-message wait: `{pre_wait_ms}ms` (should be > max_runtime_seconds)
- post-warmup wait: `{post_wait_ms}ms` (should be > max_runtime_seconds)

## Agents snapshot (before emit)

```json
{agents_before}
```

## Inject commands

```bash
{emit_cmds}
```

## Inject outputs (excerpt)

- emit-1 stdout: `{emit_1_out}`
- emit-1 stderr: `{emit_1_err}`
- emit-2 stdout: `{emit_2_out}`
- emit-2 stderr: `{emit_2_err}`

## Runner evidence (excerpt)

{evidence}

## Conclusion

- duration: `{duration:?}`
- termination_reason: `{term:?}`
- exit_code: `{exit_code:?}`

## Artifacts

- stdout: `.e2e/stdout.txt`
- stderr: `.e2e/stderr.txt`
- emit-1 stdout: `.e2e/emit-1.stdout.txt`
- emit-1 stderr: `.e2e/emit-1.stderr.txt`
- emit-2 stdout: `.e2e/emit-2.stdout.txt`
- emit-2 stderr: `.e2e/emit-2.stderr.txt`
- this file: `.e2e/human-log.md`
",
            id = self.id,
            marker = IDLE_START_MARKER,
            warmup_phase = WARMUP_PHASE,
            warmup_ack = WARMUP_ACK,
            waiting_for_second = WAITING_FOR_SECOND_MESSAGE,
            finish_phase = FINISH_PHASE,
            q1 = QUESTION_1,
            a1 = ANSWER_1,
            q2 = QUESTION_2,
            a2 = ANSWER_2,
            pre_wait_ms = pre_wait.as_millis(),
            post_wait_ms = post_warmup_wait.as_millis(),
            agents_before = agents_before_summary,
            emit_cmds = emit_cmds.join("\n"),
            emit_1_out = summarize(emit_1_out),
            emit_1_err = summarize(emit_1_err),
            emit_2_out = summarize(emit_2_out),
            emit_2_err = summarize(emit_2_err),
            evidence = if evidence_lines.trim().is_empty() {
                "(missing)".to_string()
            } else {
                evidence_lines
            },
            duration = execution.duration,
            term = execution.termination_reason,
            exit_code = execution.exit_code,
        );

        std::fs::write(dir.join("human-log.md"), content)?;
        Ok(())
    }

    async fn wait_for_ralph_idle(workspace: &Path) -> Result<String, String> {
        let deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < deadline {
            if let Ok(snapshot) = super::read_agents_snapshot(workspace)
                && snapshot
                    .instances
                    .iter()
                    .any(|i| i.instance_id == "ralph#1" && i.state == HatInstanceState::Idle)
            {
                return serde_json::to_string_pretty(&snapshot)
                    .map_err(|e| format!("failed to serialize agents snapshot: {e}"));
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        Err("timeout: ralph#1 did not reach Idle state in agents.json".to_string())
    }

    async fn wait_for_ralph_running_then_idle(workspace: &Path) -> Result<(), String> {
        let deadline = Instant::now() + Duration::from_secs(30);
        let mut seen_running = false;

        while Instant::now() < deadline {
            let snapshot = super::read_agents_snapshot(workspace)?;
            let Some(ralph) = snapshot
                .instances
                .iter()
                .find(|i| i.instance_id == "ralph#1")
            else {
                return Err("missing ralph#1 in agents.json".to_string());
            };

            if ralph.state == HatInstanceState::Running {
                seen_running = true;
            }

            if seen_running && ralph.state == HatInstanceState::Idle {
                return Ok(());
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        Err("timeout: ralph#1 did not complete warmup cycle (Running -> Idle)".to_string())
    }

    fn assert_still_idle(workspace: &Path) -> Result<(), String> {
        let snapshot = super::read_agents_snapshot(workspace)?;
        let ralph = snapshot
            .instances
            .iter()
            .find(|i| i.instance_id == "ralph#1")
            .ok_or_else(|| "missing ralph#1 in agents.json".to_string())?;

        if ralph.state == HatInstanceState::Idle {
            Ok(())
        } else {
            Err(format!("expected ralph#1 Idle, got {:?}", ralph.state))
        }
    }

    async fn emit_human_message(
        ralph_bin: &Path,
        workspace: &Path,
        payload: &str,
    ) -> Result<(String, String), String> {
        use tokio::process::Command;

        let output = Command::new(ralph_bin)
            .arg("emit")
            .arg("human.message")
            .arg(payload)
            .arg("--target-instance")
            .arg("ralph#1")
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

impl Default for ParallelAppServerIdleStartScenario {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TestScenario for ParallelAppServerIdleStartScenario {
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
        vec![Backend::Codex]
    }

    fn setup(&self, workspace: &Path, _backend: Backend) -> Result<ScenarioConfig, ScenarioError> {
        std::fs::create_dir_all(workspace.join(".agent")).map_err(|e| {
            ScenarioError::SetupError(format!("failed to create .agent directory: {e}"))
        })?;

        let _shim_dir = Self::write_fake_codex_shim(workspace)?;

        // 注意:
        // - 不提供 event_loop.prompt(也不提供 PROMPT.md),用于验证 `--idle-start` 可以“真待机”启动。
        // - 任务语义由两次注入的 human.message + app-server fake 输出共同决定。
        let config_content = r#"cli:
  backend: "codex"

event_loop:
  ralph_prompt: |
    # E2E: parallel-app-server-idle-start (fake codex shim)

    你会在稍后收到两条 human.message,其中可能包含:
    - marker: E2E_IDLE_START_MARKER_42
    - phase: warmup
    - phase: finish
    - question: 121+43=?
    - question: 10+5=?

    重要:
    - 你不需要做任何事,直到收到那些 human.message。
  completion_promise: "LOOP_COMPLETE"
  max_iterations: 6
  max_runtime_seconds: 3

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
            extra_args: vec!["--no-tui".to_string(), "--idle-start".to_string()],
        })
    }

    async fn run(
        &self,
        executor: &RalphExecutor,
        config: &ScenarioConfig,
    ) -> Result<TestResult, ScenarioError> {
        let workspace = executor.workspace().clone();
        let fake_bin_dir = workspace.join(".e2e/bin");
        let old_path = std::env::var("PATH").unwrap_or_default();
        let injected_path = format!("{}:{}", fake_bin_dir.display(), old_path);
        let extra_env = vec![("PATH".to_string(), injected_path)];

        let ralph_bin = executor.ralph_binary();
        let inject_workspace = workspace.clone();
        let pre_wait = Duration::from_secs(4);
        let post_warmup_wait = Duration::from_secs(6);
        let inject = tokio::spawn(async move {
            let agents_before = Self::wait_for_ralph_idle(&inject_workspace).await?;

            // 等待超过 max_runtime_seconds,验证 idle_start 期间不计时.
            tokio::time::sleep(pre_wait).await;
            Self::assert_still_idle(&inject_workspace)?;

            let warmup_payload = format!("marker: {IDLE_START_MARKER}; phase: {WARMUP_PHASE}");
            let warmup_cmd = format!(
                "ralph emit human.message \"{warmup_payload}\" --target-instance ralph#1 --session-strategy app_server"
            );
            let (warmup_out, warmup_err) =
                Self::emit_human_message(&ralph_bin, &inject_workspace, &warmup_payload).await?;

            // 等待 warmup turn 完整跑过一轮,把“首次 human.message 已触发 Running”这件事坐实。
            Self::wait_for_ralph_running_then_idle(&inject_workspace).await?;

            // 再等待一段超过 max_runtime_seconds 的窗口。
            // 若旧语义仍在,这里会在第二次 emit 前被 MaxRuntime 收掉。
            tokio::time::sleep(post_warmup_wait).await;

            let finish_payload = format!(
                "marker: {IDLE_START_MARKER}; phase: {FINISH_PHASE}; question: {QUESTION_1}; question: {QUESTION_2}"
            );
            let finish_cmd = format!(
                "ralph emit human.message \"{finish_payload}\" --target-instance ralph#1 --session-strategy app_server"
            );
            let (finish_out, finish_err) =
                Self::emit_human_message(&ralph_bin, &inject_workspace, &finish_payload).await?;

            Ok::<_, String>((
                vec![warmup_cmd, finish_cmd],
                agents_before,
                warmup_out,
                warmup_err,
                finish_out,
                finish_err,
            ))
        });

        let start = Instant::now();
        let execution = executor
            .run_with_extra_env(config, &extra_env)
            .await
            .map_err(|e| ScenarioError::ExecutionError(format!("ralph execution failed: {e}")))?;
        let duration = start.elapsed();

        let inject_res: Result<(Vec<String>, String, String, String, String, String), String> =
            match inject.await {
                Ok(res) => res,
                Err(e) => Err(format!("injector task panicked: {e}")),
            };

        // 落盘 emit stdout/stderr,便于 human-log 审计.
        let dir = executor.workspace().join(".e2e");
        let _ = std::fs::create_dir_all(&dir);

        let mut emit_cmds = Vec::new();
        let mut agents_before = None;

        if let Ok((cmds, agents_json, out1, err1, out2, err2)) = &inject_res {
            emit_cmds = cmds.clone();
            agents_before = Some(agents_json.clone());
            let _ = std::fs::write(dir.join("emit-1.stdout.txt"), out1);
            let _ = std::fs::write(dir.join("emit-1.stderr.txt"), err1);
            let _ = std::fs::write(dir.join("emit-2.stdout.txt"), out2);
            let _ = std::fs::write(dir.join("emit-2.stderr.txt"), err2);
            let _ = std::fs::write(dir.join("agents-before.json"), agents_json);
        }

        let _ = self.write_human_log(
            executor,
            &execution,
            &emit_cmds,
            pre_wait,
            post_warmup_wait,
            agents_before,
        );

        let mut assertions = vec![
            Assertions::response_received(&execution),
            Assertions::exit_code_success_or_limit(&execution),
            Assertions::no_timeout(&execution),
            Assertions::duration_within(&execution, Duration::from_secs(120)),
            self.fake_app_server_used(&execution),
            self.marker_observed(&execution),
            self.warmup_ack_observed(&execution),
            self.answers_observed(&execution),
            self.survived_two_runtime_windows(&execution, pre_wait, post_warmup_wait),
            self.loop_complete_detected(&execution),
            self.human_log_written(executor),
        ];

        // 注入本身也要断言,否则会出现“输出碰巧包含答案但注入失败”的假阳性.
        assertions.push({
            let builder = AssertionBuilder::new("Injector succeeded")
                .expected("ralph emit human.message succeeds twice across warmup + finish")
                .actual(match &inject_res {
                    Ok(_) => "ok=true".to_string(),
                    Err(e) => format!("ok=false, error={e}"),
                });
            if inject_res.is_ok() {
                builder.passed().build()
            } else {
                builder.failed().build()
            }
        });

        let all_passed = assertions.iter().all(|a| a.passed);

        Ok(TestResult {
            scenario_id: self.id.clone(),
            scenario_description: self.description.clone(),
            backend: String::new(),
            tier: self.tier.clone(),
            passed: all_passed,
            assertions,
            duration,
        })
    }
}
