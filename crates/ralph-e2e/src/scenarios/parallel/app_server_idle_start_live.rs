use super::super::{AssertionBuilder, Assertions, ScenarioError, TestScenario};
use crate::Backend;
use crate::executor::{ExecutionResult, PromptSource, RalphExecutor, ScenarioConfig};
use crate::models::TestResult;
use async_trait::async_trait;
use ralph_proto::HatInstanceState;
use std::path::Path;
use std::time::{Duration, Instant};

// =============================================================================
// ParallelAppServerIdleStartLiveScenario - Idle start + two human.messages (live)
// =============================================================================

/// E2E(live):
/// - 使用 **真实** `codex app-server` 验证 `--idle-start` 的“待机启动”语义在真实后端下可用:
///   - 启动后不自动触发任何 job
///   - 收到第一条 `human.message` 后只做 warmup ack,但不 `LOOP_COMPLETE`
///   - 首次 turn 结束后再等待超过 `max_runtime_seconds`,验证“首条消息后”也不会被 `MaxRuntime` 收掉
///   - 第二条 `human.message` 再输出 answer 与 `LOOP_COMPLETE`
///
/// 注意:
/// - 该场景会消耗真实网络与 token。
/// - 断言会同时检查 warmup ack 与最终 `answer: 164/15`，用于证明两轮消息都真的生效。
pub struct ParallelAppServerIdleStartLiveScenario {
    id: String,
    description: String,
    tier: String,
}

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

/// Ralph 侧输出的 client-side RPC trace 前缀(来自 `CodexAppServerSession`).
const RPC_TRACE_PREFIX: &str = "[app-server-rpc]";

impl ParallelAppServerIdleStartLiveScenario {
    pub fn new() -> Self {
        Self {
            id: "parallel-app-server-idle-start-live".to_string(),
            description: "Validates REAL codex app-server keeps idle-start session alive across pre/post-first-message max_runtime windows and completes on the second human.message".to_string(),
            tier: "Tier 8: Parallel Runtime".to_string(),
        }
    }

    fn rpc_trace_present(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let ok = result.stdout.contains(RPC_TRACE_PREFIX);
        let builder = AssertionBuilder::new("RPC trace present")
            .expected(format!(
                "stdout contains client-side RPC trace prefix: {RPC_TRACE_PREFIX}"
            ))
            .actual(if ok {
                "Found RPC trace prefix in stdout".to_string()
            } else {
                "Missing RPC trace prefix in stdout".to_string()
            });
        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn marker_observed_in_out(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let ok = result
            .stdout
            .lines()
            .any(|l| l.starts_with("[ralph#1:out:job=") && l.contains(IDLE_START_MARKER));

        let builder = AssertionBuilder::new("Marker observed in stdout(out)")
            .expected("stdout(out lines) contains the idle-start marker")
            .actual(format!("marker_present={ok}"));
        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn warmup_ack_observed_in_out(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let ack = result
            .stdout
            .lines()
            .any(|l| l.starts_with("[ralph#1:out:job=") && l.contains(WARMUP_ACK));
        let waiting = result
            .stdout
            .lines()
            .any(|l| l.starts_with("[ralph#1:out:job=") && l.contains(WAITING_FOR_SECOND_MESSAGE));
        let ok = ack && waiting;

        let builder = AssertionBuilder::new("Warmup ack observed in stdout(out)")
            .expected(
                "stdout(out lines) contains warmup ack and waiting-for-second-message markers",
            )
            .actual(format!("warmup_ack={ack}, waiting_for_second={waiting}"));
        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn answers_observed_in_out(&self, result: &ExecutionResult) -> crate::models::Assertion {
        // 说明:
        // - 只统计 hat runner 的 stdout(`[ralph#1:out:job=...]`),避免把 RPC trace(灰色)里的数字当成“回复”.
        fn answer_seen(out_line: &str, answer: &str) -> bool {
            let lower = out_line.to_ascii_lowercase();
            (lower.contains("answer") || out_line.contains("答案")) && out_line.contains(answer)
        }

        let out_lines = result
            .stdout
            .lines()
            .filter(|l| l.starts_with("[ralph#1:out:job="));

        let mut has_164 = false;
        let mut has_15 = false;
        for line in out_lines {
            if answer_seen(line, ANSWER_1) {
                has_164 = true;
            }
            if answer_seen(line, ANSWER_2) {
                has_15 = true;
            }
        }

        let ok = has_164 && has_15;
        let builder = AssertionBuilder::new("Answers observed in stdout(out)")
            .expected("stdout(out lines) contains answers for both questions (164 and 15)")
            .actual(format!("answer164={has_164}, answer15={has_15}"));
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
                && s.contains(RPC_TRACE_PREFIX)
                && s.contains("emit-2 stdout")
                && s.contains("[ralph#1:out:job=")
        });

        let builder = AssertionBuilder::new("Human log written")
            .expected(".e2e/human-log.md exists and contains warmup ack + final answers + LOOP_COMPLETE + RPC trace + emit-2 evidence + at least one [ralph#1:out:job=...] line")
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

        let pick = |pred: fn(&str) -> bool| {
            execution
                .stdout
                .lines()
                .filter(|l| pred(l))
                .take(80)
                .map(|l| format!("- `{}`", l.trim_end()))
                .collect::<Vec<_>>()
                .join("\n")
        };

        let state_lines =
            pick(|l| l.contains("[supervisor] instances") || l.contains("[ralph#1:state]"));
        let rpc_lines = pick(|l| l.contains(RPC_TRACE_PREFIX));
        let out_lines = pick(|l| l.starts_with("[ralph#1:out:job="));

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

- 验证 `ralph run --idle-start` 在真实 codex app-server 下可以待机启动。
- 第一次超时级等待后,emit warmup `human.message`,只输出 ack,不 `LOOP_COMPLETE`。
- 首次 turn 结束后再次等待超过 max_runtime_seconds,验证“首条消息后”也不会被 `MaxRuntime` 收掉。
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

## Evidence: supervisor state (excerpt)

{state_lines}

## Evidence: app-server RPC trace (excerpt)

{rpc_lines}

## Evidence: runner stdout(out) (excerpt)

{out_lines}

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
            state_lines = if state_lines.trim().is_empty() {
                "(missing)".to_string()
            } else {
                state_lines
            },
            rpc_lines = if rpc_lines.trim().is_empty() {
                "(missing)".to_string()
            } else {
                rpc_lines
            },
            out_lines = if out_lines.trim().is_empty() {
                "(missing)".to_string()
            } else {
                out_lines
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
        let deadline = Instant::now() + Duration::from_secs(60);
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

impl Default for ParallelAppServerIdleStartLiveScenario {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TestScenario for ParallelAppServerIdleStartLiveScenario {
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

        // 注意:
        // - 不提供 event_loop.prompt(也不提供 PROMPT.md),用于验证 `--idle-start` 可以在真实后端下启动。
        // - 通过 ralph_prompt 锁定两轮输出格式,避免模型漂移导致难以判断“有没有真的跨过第二段等待窗口”。
        let config_content = r#"cli:
  backend: "codex"

event_loop:
  ralph_prompt: |
    # E2E: parallel-app-server-idle-start-live (REAL codex app-server)

    你处于 idle-start 模式.
    启动时不要做任何事,只等待收到 human.message.

    要求(必须严格遵守):
    - 不要调用任何工具,不要读写任何文件,不要提出问题。
    - 只输出下面定义的行,每行独占一行,不要添加任何解释性文字。

    当 human.message 包含 `phase: warmup` 时,你必须只输出:
    MARKER: E2E_IDLE_START_MARKER_42
    IDLE_START_WARMUP_ACK
    WAITING_FOR_SECOND_MESSAGE

    当 human.message 包含 `phase: finish` 时,你必须只输出:
    MARKER: E2E_IDLE_START_MARKER_42
    TASK_REQUEST[1]: question: 121+43=?
    TASK_EXECUTE[1]: addition
    TASK_FEEDBACK[1]: answer: 164
    TASK_REQUEST[2]: question: 10+5=?
    TASK_EXECUTE[2]: addition
    TASK_FEEDBACK[2]: answer: 15
    LOOP_COMPLETE

    当 phase 不匹配时,你必须只输出:
    UNKNOWN_PHASE
  completion_promise: "LOOP_COMPLETE"
  max_iterations: 6
  max_runtime_seconds: 6

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
            timeout: Duration::from_secs(240),
            extra_args: vec!["--no-tui".to_string(), "--idle-start".to_string()],
        })
    }

    async fn run(
        &self,
        executor: &RalphExecutor,
        config: &ScenarioConfig,
    ) -> Result<TestResult, ScenarioError> {
        // 开启 client-side RPC trace,让 human-log 能看到 app-server 确实工作.
        let extra_env = vec![("RALPH_CODEX_APP_SERVER_TRACE".to_string(), "1".to_string())];

        let workspace = executor.workspace().clone();
        let ralph_bin = executor.ralph_binary();
        let inject_workspace = workspace.clone();
        let pre_wait = Duration::from_secs(7);
        let post_warmup_wait = Duration::from_secs(7);
        let inject = tokio::spawn(async move {
            let agents_before = Self::wait_for_ralph_idle(&inject_workspace).await?;

            // 先等过第一段超时窗口,证明 idle_start 在首条消息前不计 runtime.
            tokio::time::sleep(pre_wait).await;
            Self::assert_still_idle(&inject_workspace)?;

            let warmup_payload = format!("marker: {IDLE_START_MARKER}; phase: {WARMUP_PHASE}");
            let warmup_cmd = format!(
                "ralph emit human.message \"{warmup_payload}\" --target-instance ralph#1 --session-strategy app_server"
            );
            let (warmup_out, warmup_err) =
                Self::emit_human_message(&ralph_bin, &inject_workspace, &warmup_payload).await?;

            // 等首轮 turn 真正经历过 Running 并回到 Idle。
            Self::wait_for_ralph_running_then_idle(&inject_workspace).await?;

            // 再等过第二段超时窗口。
            // 若旧语义还在,这里会在第二次 emit 前被 MaxRuntime 收尾。
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
            Assertions::duration_within(&execution, Duration::from_secs(240)),
            self.rpc_trace_present(&execution),
            self.marker_observed_in_out(&execution),
            self.warmup_ack_observed_in_out(&execution),
            self.answers_observed_in_out(&execution),
            self.survived_two_runtime_windows(&execution, pre_wait, post_warmup_wait),
            self.loop_complete_detected(&execution),
            self.human_log_written(executor),
        ];

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
