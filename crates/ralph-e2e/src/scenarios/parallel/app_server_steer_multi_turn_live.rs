use super::super::{AssertionBuilder, Assertions, ScenarioError, TestScenario};
use crate::Backend;
use crate::executor::{ExecutionResult, PromptSource, RalphExecutor, ScenarioConfig};
use crate::models::TestResult;
use async_trait::async_trait;
use ralph_proto::HatInstanceState;
use std::path::Path;
use std::time::{Duration, Instant};

// =============================================================================
// ParallelAppServerSteerMultiTurnLiveScenario - Validate REAL Codex app-server
// =============================================================================

/// E2E(live): 验证 **真实** `codex app-server` 通道在并行模式下可以:
/// - 在 turn in-flight 时接收 2 次 `turn/steer` 注入
/// - 并且能够收到对应的 response(通过 client-side RPC trace 证据)
///
/// 重要说明:
/// - 该场景会使用真实 Codex 后端,会消耗网络与 token.
/// - 与 fake 场景相比,它的目标不同:
///   - fake: 确定性验证“协议/时序/路由语义”
///   - live: 验证“真实 codex app-server + Ralph client 实现”的闭环可用性
pub struct ParallelAppServerSteerMultiTurnLiveScenario {
    id: String,
    description: String,
    tier: String,
}

const LIVE_STEER_MARKER_1: &str = "E2E_LIVE_STEER_MARKER_1_42";
const LIVE_STEER_MARKER_2: &str = "E2E_LIVE_STEER_MARKER_2_42";

/// 用于验证“注入 payload 里包含具体任务内容”的最小样例:
/// - live 场景不强依赖模型输出格式,因此这里只把 question 放进 steer payload,
///   主要靠 RPC trace 的 input_preview 做审计证据.
const LIVE_STEER_QUESTION_1: &str = "121+43=?";
const LIVE_STEER_ANSWER_1: &str = "164";
const LIVE_STEER_QUESTION_2: &str = "10+5=?";
const LIVE_STEER_ANSWER_2: &str = "15";

/// Ralph 侧输出的 client-side RPC trace 前缀(来自 `CodexAppServerSession`).
const RPC_TRACE_PREFIX: &str = "[app-server-rpc]";

impl ParallelAppServerSteerMultiTurnLiveScenario {
    pub fn new() -> Self {
        Self {
            id: "parallel-app-server-steer-multi-turn-live".to_string(),
            description:
                "Validates REAL codex app-server turn/steer works in-flight (parallel runtime)"
                    .to_string(),
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

    fn steer_rpc_sent_twice(&self, result: &ExecutionResult) -> crate::models::Assertion {
        // 说明:
        // - 我们用 client-side trace 证明 turn/steer 真的走了 app-server RPC,
        //   而不是降级成普通 pending 事件(下一轮 prompt 才生效)。
        let count = result
            .stdout
            .lines()
            .filter(|l| {
                l.contains(RPC_TRACE_PREFIX)
                    && l.contains("send request")
                    && l.contains("method=turn/steer")
            })
            .count();
        let ok = count >= 2;

        let builder = AssertionBuilder::new("turn/steer RPC sent twice")
            .expected("stdout contains >=2 app-server-rpc send lines for method=turn/steer")
            .actual(format!("count={count}"));
        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn steer_rpc_responded_twice(&self, result: &ExecutionResult) -> crate::models::Assertion {
        // 说明:
        // - 用户关心的是“runner 真的收到了消息并回复了”,因此这里不仅看 send,
        //   也要看 app-server 对 `turn/steer` 的 response 回执。
        let ok_count = result
            .stdout
            .lines()
            .filter(|l| {
                l.contains(RPC_TRACE_PREFIX)
                    && l.contains("recv response")
                    && l.contains("method=turn/steer")
                    // 只接受成功 response: 错误回执说明 steer 时序/门槛不满足,不算“能力可用”。
                    && !l.contains("error_code=")
            })
            .count();
        let err_count = result
            .stdout
            .lines()
            .filter(|l| {
                l.contains(RPC_TRACE_PREFIX)
                    && l.contains("recv response")
                    && l.contains("method=turn/steer")
                    && l.contains("error_code=")
            })
            .count();
        let ok = ok_count >= 2;

        let builder = AssertionBuilder::new("turn/steer RPC responded twice")
            .expected("stdout contains >=2 successful app-server-rpc responses for method=turn/steer (no error_code)")
            .actual(format!("ok_count={ok_count}, err_count={err_count}"));
        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn stdout_has_real_agent_output(&self, result: &ExecutionResult) -> crate::models::Assertion {
        // 说明:
        // - RPC trace 位于 stderr(灰色),会出现在 `[ralph#1:err:job=...]` 行里。
        // - 该断言用于验证“真实 agent 输出”确实走了 stdout 通道,
        //   否则 completion_promise 永远无法被 Supervisor 检测到。
        let count = result
            .stdout
            .lines()
            .filter(|l| l.starts_with("[ralph#1:out:job="))
            .count();
        let ok = count > 0;

        let builder = AssertionBuilder::new("Real agent output present")
            .expected("stdout contains at least one [ralph#1:out:job=...] line")
            .actual(format!("count={count}"));
        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn steer_payload_seen_in_trace(&self, result: &ExecutionResult) -> crate::models::Assertion {
        // 说明:
        // - 该断言不依赖模型是否“复述 marker”(可能漂移),
        // - 而是直接从 client-side trace 的 input_preview 里确认:
        //   我们发送的 steer payload(包含 marker+question)真的被编码进 turn/steer RPC。
        let steer1 = result.stdout.contains(LIVE_STEER_MARKER_1)
            && result.stdout.contains(LIVE_STEER_QUESTION_1);
        let steer2 = result.stdout.contains(LIVE_STEER_MARKER_2)
            && result.stdout.contains(LIVE_STEER_QUESTION_2);
        let ok = steer1 && steer2;

        let builder = AssertionBuilder::new("Steer payload seen in RPC trace")
            .expected("stdout contains both live steer marker+question (via RPC trace steer input_preview)")
            .actual(format!("steer1={steer1}, steer2={steer2}"));
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
                && s.contains(LIVE_STEER_MARKER_1)
                && s.contains(LIVE_STEER_MARKER_2)
                && s.contains(LIVE_STEER_QUESTION_1)
                && s.contains(LIVE_STEER_QUESTION_2)
                && s.contains(RPC_TRACE_PREFIX)
                // 关键补强:
                // - 你反馈 human-log 里“看不到 runner 的输出”,因此这里要求至少包含一条 hat stdout 行,
                //   这样只看 human-log 就能判断“runner 是否在输出/是否无回复”。
                && s.contains("[ralph#1:out:job=")
        });

        let builder = AssertionBuilder::new("Human log written")
            .expected(".e2e/human-log.md exists and contains steer markers+questions + RPC trace prefix + at least one [ralph#1:out:job=...] line")
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

        // 关键握手/steer 证据(精选):
        // - 让 human-log.md 顶部就能看到“发了什么,收到了什么”.
        let pick = |pred: fn(&str) -> bool| {
            execution
                .stdout
                .lines()
                .filter(|l| pred(l))
                .take(40)
                .map(|l| format!("- `{}`", l.trim_end()))
                .collect::<Vec<_>>()
                .join("\n")
        };

        // hat runner 状态变化:
        // - 让 human-log.md 直接看到 ralph#1 是否进入 Running,以及是否有 supervisor 实例列表。
        let state_lines =
            pick(|l| l.contains("[supervisor] instances") || l.contains("[ralph#1:state]"));

        // hat runner 的 stdout 摘录:
        // - 仅摘录 head/tail,避免 human-log 过长.
        let out_head = execution
            .stdout
            .lines()
            .filter(|l| l.starts_with("[ralph#1:out:job="))
            .take(12)
            .map(|l| format!("- `{}`", l.trim_end()))
            .collect::<Vec<_>>()
            .join("\n");

        let mut out_tail_vec = execution
            .stdout
            .lines()
            .filter(|l| l.starts_with("[ralph#1:out:job="))
            .rev()
            .take(12)
            .collect::<Vec<_>>();
        out_tail_vec.reverse();
        let out_tail = out_tail_vec
            .into_iter()
            .map(|l| format!("- `{}`", l.trim_end()))
            .collect::<Vec<_>>()
            .join("\n");

        let handshake_lines = pick(|l| {
            l.contains(RPC_TRACE_PREFIX)
                && (l.contains("method=initialize")
                    || l.contains("method=thread/start")
                    || l.contains("thread/started")
                    || l.contains("method=turn/start")
                    || l.contains("turn/started")
                    || l.contains("method=turn/steer")
                    || l.contains("method=turn/completed"))
        });

        let evidence_lines = execution
            .stdout
            .lines()
            .filter(|l| {
                l.contains(RPC_TRACE_PREFIX)
                    || l.contains(LIVE_STEER_MARKER_1)
                    || l.contains(LIVE_STEER_MARKER_2)
                    || l.contains("LOOP_COMPLETE")
            })
            .take(120)
            .map(|l| format!("- `{}`", l.trim_end()))
            .collect::<Vec<_>>()
            .join("\n");

        // `ralph emit` 的 stdout/stderr 摘录
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
                .take(6)
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

- 使用 **真实 codex app-server** 验证 turn/steer 在 Ralph 并行模式下可用。
- 关键证据采用 client-side RPC trace(`{rpc_prefix}`)。
- 在 turn in-flight 期间,外部注入 2 次 steer,并观察到对应的 request/response。

## Marker

- `{m1}`
- `{m2}`

## 任务内容(注入 payload)

- steer-1: question=`{q1}`, expect answer=`{a1}`
- steer-2: question=`{q2}`, expect answer=`{a2}`

## Hat runner 状态(摘录)

{states}

## Hat runner stdout(摘录)

### head(前 12 条)

{out_head}

### tail(后 12 条)

{out_tail}

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

### 精选(握手 + steer 回执)

{handshake}

### 详细(前 120 条匹配行)

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
            rpc_prefix = RPC_TRACE_PREFIX,
            m1 = LIVE_STEER_MARKER_1,
            m2 = LIVE_STEER_MARKER_2,
            q1 = LIVE_STEER_QUESTION_1,
            a1 = LIVE_STEER_ANSWER_1,
            q2 = LIVE_STEER_QUESTION_2,
            a2 = LIVE_STEER_ANSWER_2,
            states = if state_lines.trim().is_empty() {
                "(missing)".to_string()
            } else {
                state_lines
            },
            out_head = if out_head.trim().is_empty() {
                "(missing)".to_string()
            } else {
                out_head
            },
            out_tail = if out_tail.trim().is_empty() {
                "(missing)".to_string()
            } else {
                out_tail
            },
            emit_cmds = emit_cmds.join("\n"),
            emit1_out = summarize_emit(emit_1_out),
            emit1_err = summarize_emit(emit_1_err),
            emit2_out = summarize_emit(emit_2_out),
            emit2_err = summarize_emit(emit_2_err),
            handshake = if handshake_lines.trim().is_empty() {
                "(missing)".to_string()
            } else {
                handshake_lines
            },
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
        let deadline = Instant::now() + Duration::from_secs(25);
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

impl Default for ParallelAppServerSteerMultiTurnLiveScenario {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TestScenario for ParallelAppServerSteerMultiTurnLiveScenario {
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
        // 创建 `.agent/`（部分路径假设其存在）
        std::fs::create_dir_all(workspace.join(".agent")).map_err(|e| {
            ScenarioError::SetupError(format!("failed to create .agent directory: {e}"))
        })?;

        // 说明:
        // - 这里不注入 fake codex shim,而是使用真实 codex app-server.
        // - prompt 的目标是让 ralph#1 最终输出 LOOP_COMPLETE 收敛,同时给 steer 留出窗口.
        let config_content = r#"cli:
  backend: "codex"

event_loop:
  prompt: |
    # E2E: parallel-app-server-steer-multi-turn-live (REAL codex app-server)

    你正在运行一个 E2E 场景。
    该场景会在你运行期间对 `ralph#1` 注入两次 turn/steer。

    要求(必须严格遵守):
    - 不要调用任何工具,不要读写任何文件,不要提出问题。
    - 即使你收到任何 steer 输入(包含 marker/question),也必须完全忽略它们,继续按固定输出执行。
    - 从现在开始,只输出以下内容,每行独占一行,不要添加编号或解释:
      - 连续输出 30 行: STEER_WINDOW_OPEN
      - 最后一行输出: LOOP_COMPLETE
  completion_promise: "LOOP_COMPLETE"
  max_iterations: 4
  max_runtime_seconds: 90

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
            extra_args: vec!["--no-tui".to_string()],
        })
    }

    async fn run(
        &self,
        executor: &RalphExecutor,
        config: &ScenarioConfig,
    ) -> Result<TestResult, ScenarioError> {
        // -----------------------------------------------------------------
        // 启用 client-side RPC trace:
        // - 让 stdout 能看到 send/recv turn/steer 的证据(真实 codex app-server 侧一般不回显协议日志)
        // -----------------------------------------------------------------
        let extra_env = vec![
            ("RALPH_CODEX_APP_SERVER_TRACE".to_string(), "1".to_string()),
            (
                "RALPH_CODEX_APP_SERVER_TRACE_STEER_INPUT".to_string(),
                "1".to_string(),
            ),
        ];

        // 并发注入 steer:
        // - 只要能捕捉到 >=2 次 `send request method=turn/steer`,即可证明 in-flight 通道生效。
        let workspace = executor.workspace().clone();
        let ralph_bin = executor.ralph_binary();
        let inject_workspace = workspace.clone();
        let inject = tokio::spawn(async move {
            Self::wait_for_ralph_running(&inject_workspace).await?;

            let cmd1 = format!(
                "ralph emit e2e.steer \"marker: {LIVE_STEER_MARKER_1}; question: {LIVE_STEER_QUESTION_1}\" --target-instance ralph#1 --turn-action steer --session-strategy app_server"
            );
            let (out1, err1) = Self::emit_steer(
                &ralph_bin,
                &inject_workspace,
                LIVE_STEER_MARKER_1,
                LIVE_STEER_QUESTION_1,
            )
            .await?;

            // 尽量快地发第二次 steer,提高“命中 in-flight window”的概率。
            tokio::time::sleep(Duration::from_millis(250)).await;

            let cmd2 = format!(
                "ralph emit e2e.steer \"marker: {LIVE_STEER_MARKER_2}; question: {LIVE_STEER_QUESTION_2}\" --target-instance ralph#1 --turn-action steer --session-strategy app_server"
            );
            let (out2, err2) = Self::emit_steer(
                &ralph_bin,
                &inject_workspace,
                LIVE_STEER_MARKER_2,
                LIVE_STEER_QUESTION_2,
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

        let inject_res: Result<(Vec<String>, String, String, String, String), String> =
            match inject.await {
                Ok(res) => res,
                Err(e) => Err(format!("steer injector task panicked: {e}")),
            };

        let mut emit_cmds = Vec::new();
        if let Ok((cmds, out1, err1, out2, err2)) = &inject_res {
            emit_cmds = cmds.clone();
            let dir = executor.workspace().join(".e2e");
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::fs::write(dir.join("emit-1.stdout.txt"), out1);
            let _ = std::fs::write(dir.join("emit-1.stderr.txt"), err1);
            let _ = std::fs::write(dir.join("emit-2.stdout.txt"), out2);
            let _ = std::fs::write(dir.join("emit-2.stderr.txt"), err2);
        }

        let _ = self.write_human_log(executor, &execution, &emit_cmds);

        let mut assertions = vec![
            Assertions::response_received(&execution),
            Assertions::exit_code_success_or_limit(&execution),
            Assertions::no_timeout(&execution),
            Assertions::duration_within(&execution, Duration::from_secs(220)),
            self.rpc_trace_present(&execution),
            self.steer_rpc_sent_twice(&execution),
            self.steer_rpc_responded_twice(&execution),
            self.steer_payload_seen_in_trace(&execution),
            self.stdout_has_real_agent_output(&execution),
            self.loop_complete_detected(&execution),
            self.human_log_written(executor),
        ];

        // inject 本身也要变成断言
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
            backend: String::new(),
            tier: self.tier.clone(),
            passed: all_passed,
            assertions,
            duration,
        })
    }
}
