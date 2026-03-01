use super::super::{AssertionBuilder, Assertions, ScenarioError, TestScenario};
use crate::Backend;
use crate::executor::{ExecutionResult, PromptSource, RalphExecutor, ScenarioConfig};
use crate::models::TestResult;
use async_trait::async_trait;
use ralph_proto::HatInstanceState;
use std::path::Path;
use std::time::{Duration, Instant};

// =============================================================================
// ParallelAppServerSteerLiveReplyMultiTurnScenario - REAL Codex app-server + reply
// =============================================================================

/// E2E(live-reply):
/// - 使用 **真实 codex app-server** 验证 in-flight `turn/steer` 不仅能 send/recv,
///   还会进入模型并生成“用户可见回复”(stdout)。
///
/// 重要说明:
/// - 该场景比 transport 场景更“敏感”(更依赖模型遵循输出格式),因此更适合:
///   - 排障: 你想知道“为什么看不到回复/到底输出了什么”。
///   - 手动验证: 在迭代 Codex app-server 适配时做回归。
///
/// 对比:
/// - `parallel-app-server-steer-multi-turn-live`(transport):
///   只验证 RPC send/recv + 收敛,不强依赖模型输出 answer。
/// - 本场景(live-reply):
///   要求模型实际输出 answer(164/15),用于证明“steer → 回复”端到端闭环。
pub struct ParallelAppServerSteerLiveReplyMultiTurnScenario {
    id: String,
    description: String,
    tier: String,
}

const LIVE_REPLY_MARKER_1: &str = "E2E_LIVE_REPLY_STEER_MARKER_1_42";
const LIVE_REPLY_MARKER_2: &str = "E2E_LIVE_REPLY_STEER_MARKER_2_42";

// 具体任务内容(可核对)
const LIVE_REPLY_QUESTION_1: &str = "121+43=?";
const LIVE_REPLY_ANSWER_1: &str = "164";
const LIVE_REPLY_QUESTION_2: &str = "10+5=?";
const LIVE_REPLY_ANSWER_2: &str = "15";

/// Step-2 触发事件(topic).
///
/// 说明:
/// - 我们用外部 `ralph emit` 发布该事件,把“回答任务”的工作放到下一次迭代/下一次 turn。
/// - 这更贴近真实 app-server 的语义: steer 输入会进入 thread 历史,但不一定能中断并立刻改变当前输出。
const STEP2_TOPIC: &str = "e2e.reply.step2";

/// Ralph 侧输出的 client-side RPC trace 前缀(来自 `CodexAppServerSession`).
const RPC_TRACE_PREFIX: &str = "[app-server-rpc]";

impl ParallelAppServerSteerLiveReplyMultiTurnScenario {
    pub fn new() -> Self {
        Self {
            // 注意: id 刻意把 `live-reply` 放在 `multi-turn` 之前:
            // - 这样用户用 `--filter parallel-app-server-steer-multi-turn` 跑 fake 场景时,
            //   不会误匹配到本场景(避免意外消耗真实 token)。
            id: "parallel-app-server-steer-live-reply-multi-turn".to_string(),
            description:
                "Validates REAL codex app-server turn/steer produces visible reply output (answers)"
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
        // - 必须是成功 response,否则说明 steer 时序/门槛不满足。
        let ok_count = result
            .stdout
            .lines()
            .filter(|l| {
                l.contains(RPC_TRACE_PREFIX)
                    && l.contains("recv response")
                    && l.contains("method=turn/steer")
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

    fn steer_payload_seen_in_trace(&self, result: &ExecutionResult) -> crate::models::Assertion {
        // 说明:
        // - 不依赖模型是否复述 marker,而是从 RPC trace 的 input_preview 审计 payload。
        let steer1 = result.stdout.contains(LIVE_REPLY_MARKER_1)
            && result.stdout.contains(LIVE_REPLY_QUESTION_1);
        let steer2 = result.stdout.contains(LIVE_REPLY_MARKER_2)
            && result.stdout.contains(LIVE_REPLY_QUESTION_2);
        let ok = steer1 && steer2;

        let builder = AssertionBuilder::new("Steer payload seen in RPC trace")
            .expected(
                "stdout contains both steer marker+question (via RPC trace steer input_preview)",
            )
            .actual(format!("steer1={steer1}, steer2={steer2}"));
        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn answers_observed_in_stdout(&self, result: &ExecutionResult) -> crate::models::Assertion {
        // 说明:
        // - 只统计 hat runner 的 stdout(`[ralph#1:out:job=...]`),避免把 RPC trace(灰色)里的数字当成“回复”。
        // - 为了容忍少量格式漂移:
        //   - 接受 "answer" (大小写不敏感) 或中文 "答案" 作为标签。
        fn answer_seen(out_line: &str, answer: &str) -> bool {
            let lower = out_line.to_ascii_lowercase();
            let has_label = lower.contains("answer") || out_line.contains("答案");
            has_label && out_line.contains(answer)
        }

        let out_lines = result
            .stdout
            .lines()
            .filter(|l| l.starts_with("[ralph#1:out:job="));

        let mut has_164 = false;
        let mut has_15 = false;
        for line in out_lines {
            if answer_seen(line, LIVE_REPLY_ANSWER_1) {
                has_164 = true;
            }
            if answer_seen(line, LIVE_REPLY_ANSWER_2) {
                has_15 = true;
            }
        }

        let ok = has_164 && has_15;
        let builder = AssertionBuilder::new("Answers observed in stdout")
            .expected("stdout (out lines) contains answers for both questions (164 and 15)")
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

    fn human_log_written(&self, executor: &RalphExecutor) -> crate::models::Assertion {
        let path = executor.workspace().join(".e2e/human-log.md");
        let content = std::fs::read_to_string(&path).ok();
        let ok = content.as_deref().is_some_and(|s| {
            !s.trim().is_empty()
                && s.contains(LIVE_REPLY_MARKER_1)
                && s.contains(LIVE_REPLY_MARKER_2)
                && s.contains(LIVE_REPLY_QUESTION_1)
                && s.contains(LIVE_REPLY_QUESTION_2)
                && s.contains(LIVE_REPLY_ANSWER_1)
                && s.contains(LIVE_REPLY_ANSWER_2)
                && s.contains(RPC_TRACE_PREFIX)
                && s.contains("[ralph#1:out:job=")
        });

        let builder = AssertionBuilder::new("Human log written")
            .expected(".e2e/human-log.md exists and contains markers+questions+answers + RPC trace + at least one [ralph#1:out:job=...] line")
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

        // 说明:
        // - 使用小摘录把“runner 状态/输出/收发回执”集中在一个文件里,方便人类定位“无回复”的根因。
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

        let state_lines =
            pick(|l| l.contains("[supervisor] instances") || l.contains("[ralph#1:state]"));

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
                    || l.contains(LIVE_REPLY_MARKER_1)
                    || l.contains(LIVE_REPLY_MARKER_2)
                    || l.contains("answer")
                    || l.contains("答案")
                    || l.contains("LOOP_COMPLETE")
            })
            .take(140)
            .map(|l| format!("- `{}`", l.trim_end()))
            .collect::<Vec<_>>()
            .join("\n");

        // `ralph emit` 的 stdout/stderr 摘录
        let emit_1_out = std::fs::read_to_string(dir.join("emit-1.stdout.txt")).ok();
        let emit_1_err = std::fs::read_to_string(dir.join("emit-1.stderr.txt")).ok();
        let emit_2_out = std::fs::read_to_string(dir.join("emit-2.stdout.txt")).ok();
        let emit_2_err = std::fs::read_to_string(dir.join("emit-2.stderr.txt")).ok();
        let emit_3_out = std::fs::read_to_string(dir.join("emit-3.stdout.txt")).ok();
        let emit_3_err = std::fs::read_to_string(dir.join("emit-3.stderr.txt")).ok();

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
- 与 transport 场景不同,本场景要求 **实际输出 answer**(证明 steer 进入模型并生成回复)。
- 关键证据采用 client-side RPC trace(`{rpc_prefix}`) + hat runner stdout 摘录。

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
 - emit-3 stdout: `{emit3_out}`
 - emit-3 stderr: `{emit3_err}`

## 关键证据(摘录)

### 精选(握手 + steer 回执)

{handshake}

### 详细(前 140 条匹配行)

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
            m1 = LIVE_REPLY_MARKER_1,
            m2 = LIVE_REPLY_MARKER_2,
            q1 = LIVE_REPLY_QUESTION_1,
            a1 = LIVE_REPLY_ANSWER_1,
            q2 = LIVE_REPLY_QUESTION_2,
            a2 = LIVE_REPLY_ANSWER_2,
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
            emit3_out = summarize_emit(emit_3_out),
            emit3_err = summarize_emit(emit_3_err),
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

    async fn emit_step2(ralph_bin: &Path, workspace: &Path) -> Result<(String, String), String> {
        use tokio::process::Command;

        let output = Command::new(ralph_bin)
            .arg("emit")
            .arg(STEP2_TOPIC)
            .arg("step2")
            .arg("--target-instance")
            .arg("ralph#1")
            .arg("--turn-action")
            .arg("start")
            .arg("--session-strategy")
            .arg("app_server")
            .current_dir(workspace)
            .output()
            .await
            .map_err(|e| format!("failed to run ralph emit(step2): {e}"))?;

        if !output.status.success() {
            return Err(format!(
                "ralph emit(step2) failed: status={:?}, stderr={}",
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

impl Default for ParallelAppServerSteerLiveReplyMultiTurnScenario {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TestScenario for ParallelAppServerSteerLiveReplyMultiTurnScenario {
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
        // - 这里使用真实 codex app-server。
        // - prompt 目标: 收到两次 steer 后输出 answer 并收敛到 LOOP_COMPLETE。
        //
        // 注意:
        // - 该 prompt 比 transport 场景更“强约束”,可能更容易受模型漂移影响。
        // - 但这是我们刻意的: 用户希望看到“实际回复”,用于定位无回复原因。
        let config_content = r#"cli:
  backend: "codex"

event_loop:
  prompt: |
    # E2E: parallel-app-server-steer-live-reply-multi-turn (REAL codex app-server)

    你正在运行一个 E2E 场景(真实 codex app-server)。
    本场景用于验证两件事:
    1) `turn/steer` 的输入能在真实 app-server 下成功 send/recv。
    2) steer 输入会进入 thread 历史,并在 **下一次 turn** 中可被模型读取并产生回复(answer)。

    关键约束(必须严格遵守):
    - 不要调用任何工具,不要读写任何文件,不要提出问题。
    - 不要输出 Markdown,不要输出空行。

    你必须根据 `## PENDING EVENTS` 里的事件 topic 决定输出内容:

    A) 如果本轮 PENDING EVENTS 里包含 `[task.start]`:
       - 输出 30 行: STEER_WINDOW_OPEN
       - 然后立刻停止输出。
       - 绝对不要输出 LOOP_COMPLETE(否则测试会提前结束)。

    B) 如果本轮 PENDING EVENTS 里包含 `[e2e.reply.step2]`:
       - 你需要从 thread 历史中找到最近的两条用户输入,它们形如:
         marker: E2E_...; question: 121+43=?
       - 对每条输入,输出两行(每行独占一行):
         - TASK_REQUEST[n]: <该输入的原文>
         - TASK_FEEDBACK[n]: answer: <加法结果>
         n 从 1 开始递增。
       - 当你输出两次 TASK_FEEDBACK 后,输出最后一行: LOOP_COMPLETE,然后停止输出。

    除上述行外,不要输出任何其他文本。
  completion_promise: "LOOP_COMPLETE"
  max_iterations: 4
  max_runtime_seconds: 90

parallel:
  enabled: true
  autoscale:
    max_running_jobs: 1
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
            timeout: Duration::from_secs(300),
            extra_args: vec!["--no-tui".to_string()],
        })
    }

    async fn run(
        &self,
        executor: &RalphExecutor,
        config: &ScenarioConfig,
    ) -> Result<TestResult, ScenarioError> {
        // 启用 client-side RPC trace:
        // - 让 stdout 能看到 turn/steer 的 send/recv 证据,并包含 input_preview(用于 marker/question 审计)。
        let extra_env = vec![
            ("RALPH_CODEX_APP_SERVER_TRACE".to_string(), "1".to_string()),
            (
                "RALPH_CODEX_APP_SERVER_TRACE_STEER_INPUT".to_string(),
                "1".to_string(),
            ),
        ];

        let workspace = executor.workspace().clone();
        let ralph_bin = executor.ralph_binary();
        let inject_workspace = workspace.clone();
        let inject = tokio::spawn(async move {
            Self::wait_for_ralph_running(&inject_workspace).await?;

            // 说明:
            // - 第一条 steer 尽量早发,让它落在 in-flight window 内。
            // - 第二条稍微延迟,避免两条 steer 过于贴近导致模型难以分辨。
            let cmd1 = format!(
                "ralph emit e2e.steer \"marker: {LIVE_REPLY_MARKER_1}; question: {LIVE_REPLY_QUESTION_1}\" --target-instance ralph#1 --turn-action steer --session-strategy app_server"
            );
            let (out1, err1) = Self::emit_steer(
                &ralph_bin,
                &inject_workspace,
                LIVE_REPLY_MARKER_1,
                LIVE_REPLY_QUESTION_1,
            )
            .await?;

            tokio::time::sleep(Duration::from_millis(600)).await;

            let cmd2 = format!(
                "ralph emit e2e.steer \"marker: {LIVE_REPLY_MARKER_2}; question: {LIVE_REPLY_QUESTION_2}\" --target-instance ralph#1 --turn-action steer --session-strategy app_server"
            );
            let (out2, err2) = Self::emit_steer(
                &ralph_bin,
                &inject_workspace,
                LIVE_REPLY_MARKER_2,
                LIVE_REPLY_QUESTION_2,
            )
            .await?;

            // step2: 触发下一次 iteration/turn,让模型从 thread 历史里读取两条 steer 输入并回复 answer。
            tokio::time::sleep(Duration::from_millis(250)).await;
            let cmd3 = format!(
                "ralph emit {STEP2_TOPIC} \"step2\" --target-instance ralph#1 --turn-action start --session-strategy app_server"
            );
            let (out3, err3) = Self::emit_step2(&ralph_bin, &inject_workspace).await?;

            Ok::<_, String>((vec![cmd1, cmd2, cmd3], out1, err1, out2, err2, out3, err3))
        });

        let start = Instant::now();
        let execution = executor
            .run_with_extra_env(config, &extra_env)
            .await
            .map_err(|e| ScenarioError::ExecutionError(format!("ralph execution failed: {e}")))?;
        let duration = start.elapsed();

        let inject_res: Result<
            (Vec<String>, String, String, String, String, String, String),
            String,
        > = match inject.await {
            Ok(res) => res,
            Err(e) => Err(format!("steer injector task panicked: {e}")),
        };

        let mut emit_cmds = Vec::new();
        if let Ok((cmds, out1, err1, out2, err2, out3, err3)) = &inject_res {
            emit_cmds = cmds.clone();
            let dir = executor.workspace().join(".e2e");
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::fs::write(dir.join("emit-1.stdout.txt"), out1);
            let _ = std::fs::write(dir.join("emit-1.stderr.txt"), err1);
            let _ = std::fs::write(dir.join("emit-2.stdout.txt"), out2);
            let _ = std::fs::write(dir.join("emit-2.stderr.txt"), err2);
            let _ = std::fs::write(dir.join("emit-3.stdout.txt"), out3);
            let _ = std::fs::write(dir.join("emit-3.stderr.txt"), err3);
        }

        let _ = self.write_human_log(executor, &execution, &emit_cmds);

        let mut assertions = vec![
            Assertions::response_received(&execution),
            Assertions::exit_code_success_or_limit(&execution),
            Assertions::no_timeout(&execution),
            Assertions::duration_within(&execution, Duration::from_secs(260)),
            self.rpc_trace_present(&execution),
            self.steer_rpc_sent_twice(&execution),
            self.steer_rpc_responded_twice(&execution),
            self.steer_payload_seen_in_trace(&execution),
            self.answers_observed_in_stdout(&execution),
            self.loop_complete_detected(&execution),
            self.human_log_written(executor),
        ];

        // inject 也作为断言,避免“刚好输出 answer 但注入失败”的假阳性。
        let inject_ok = inject_res.is_ok();
        assertions.push({
            let builder = AssertionBuilder::new("Injector succeeded")
                .expected("ralph emit runs: steer-1, steer-2, and step2 trigger")
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
