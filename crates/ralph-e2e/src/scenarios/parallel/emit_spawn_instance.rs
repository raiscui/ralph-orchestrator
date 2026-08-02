use super::super::{AssertionBuilder, Assertions, ScenarioError, TestScenario};
use crate::Backend;
use crate::executor::{ExecutionResult, PromptSource, RalphExecutor, ScenarioConfig};
use crate::models::TestResult;
use async_trait::async_trait;
use std::path::Path;
use std::time::Duration;

// =============================================================================
// ParallelEmitSpawnInstanceScenario - Validate `ralph emit --spawn-instance` end-to-end
// =============================================================================

/// E2E: 验证并行模式下,`ralph#1` 在运行中通过 `ralph emit --spawn-instance` 创建动态实例,
/// 并在后续收到该动态实例回送的 `spawn.done` 回执,最终收敛到 `LOOP_COMPLETE`.
///
/// 设计目标(稳定性优先):
/// - 不依赖 stdout 文本的“自然语言解释”,只依赖:
///   - `.ralph/agents.json`(动态实例 + last_input)
///   - `.ralph/events.jsonl`(spawn.done 的 source_instance + payload)
/// - mock-mode 下通过 `[E2E_CMD] ...` 从 terminal writes 提取命令并执行,避免依赖模型工具调用.
pub struct ParallelEmitSpawnInstanceScenario {
    id: String,
    description: String,
    tier: String,
}

/// 场景 marker: 用于在 payload/日志里做强匹配(避免“看起来像成功”的假阳性).
const E2E_SPAWN_MARKER: &str = "E2E_SPAWN_MARKER_42";

/// 用于验证“任务请求/执行/反馈”的具体任务内容:
/// - 请求: question=121+43=?
/// - 反馈: answer=164
const E2E_MATH_QUESTION: &str = "121+43=?";
const E2E_MATH_ANSWER: &str = "164";

impl ParallelEmitSpawnInstanceScenario {
    pub fn new() -> Self {
        Self {
            id: "parallel-emit-spawn-instance".to_string(),
            description: "Validates ralph#1 spawns a dynamic worker via `ralph emit --spawn-instance` and receives spawn.done ACK".to_string(),
            tier: "Tier 8: Parallel Runtime".to_string(),
        }
    }

    fn cli_config_yaml(backend: Backend) -> String {
        // ---------------------------------------------------------------------
        // 说明：
        // - 该 helper 只影响 E2E workspace 里的 `ralph.yml`，不影响仓库默认配置。
        // - 本场景在 live(Codex) 下需要执行 `ralph emit ...` 命令，因此显式打开 sandbox。
        // - 同时注入降噪参数，避免 E2E stdout/stderr 被长篇推理淹没。
        // ---------------------------------------------------------------------
        match backend {
            Backend::Codex => {
                let model = super::codex_e2e_model();

                format!(
                    r#"  backend: custom
  command: codex
  args:
    - exec
    - -m
    - {model}
    - --full-auto
    - --sandbox
    - danger-full-access
    - -c
    - 'model_reasoning_effort="low"'
    - -c
    - 'model_reasoning_summary="none"'
    - -c
    - 'rmcp_client=false'
    - -c
    - 'features.hooks=false'
"#
                )
            }
            _ => format!("  backend: {}\n", backend.as_config_str()),
        }
    }

    fn parallel_mode_visible(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let visible = result.stdout.contains("[supervisor] instances");
        let builder = AssertionBuilder::new("Parallel mode visible")
            .expected("stdout contains '[supervisor] instances' banner")
            .actual(if visible {
                "Found supervisor instance banner".to_string()
            } else {
                "Missing supervisor instance banner".to_string()
            });

        if visible {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn e2e_cmd_printed(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let stdout = &result.stdout;

        let has_marker = stdout.contains(E2E_SPAWN_MARKER);
        let has_question = stdout.contains(E2E_MATH_QUESTION);
        let has_cmd = stdout.contains("[E2E_CMD] ralph emit spawn.task")
            && stdout.contains("--target worker")
            && stdout.contains("--spawn-instance");

        let ok = has_cmd && has_marker && has_question;
        let builder = AssertionBuilder::new("E2E command printed")
            .expected("[E2E_CMD] ralph emit spawn.task ... --target worker --spawn-instance (marker+question required)")
            .actual(format!(
                "has_cmd={has_cmd}, has_marker={has_marker}, has_question={has_question}"
            ));

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn human_log_written(&self, executor: &RalphExecutor) -> crate::models::Assertion {
        // -----------------------------------------------------------------
        // 说明:
        // - 本场景要求“留下人类可读日志”,因此这里把 `human-log.md` 作为硬断言之一。
        // - runner 会在 cleanup 前把 `.e2e/*` 复制到 `.e2e-tests/artifacts/<scenario-id>/`，
        //   但该复制属于 runner 逻辑,因此 scenario 侧仍需要保证日志文件本身存在。
        // -----------------------------------------------------------------
        let path = executor.workspace().join(".e2e/human-log.md");
        let content = std::fs::read_to_string(&path).ok();

        let ok = content.as_deref().is_some_and(|s| {
            !s.trim().is_empty()
                && s.contains(E2E_SPAWN_MARKER)
                && s.contains(E2E_MATH_QUESTION)
                && s.contains(E2E_MATH_ANSWER)
        });
        let builder = AssertionBuilder::new("Human log written")
            .expected(".e2e/human-log.md exists and contains marker+question+answer")
            .actual(match content {
                Some(s) => format!(
                    "bytes={}, has_marker={}",
                    s.len(),
                    s.contains(E2E_SPAWN_MARKER)
                ),
                None => format!("missing: {}", path.display()),
            });

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn dynamic_worker_spawned(
        &self,
        executor: &RalphExecutor,
    ) -> (crate::models::Assertion, Option<String>) {
        // -----------------------------------------------------------------
        // 说明:
        // - 我们用 `.ralph/agents.json` 来证明:
        //   1) worker hat 确实出现了一个动态实例(is_dynamic=true)
        //   2) 该实例 id 可用于与 events.jsonl 的 source_instance 做交叉验证
        // -----------------------------------------------------------------
        let snapshot = match super::read_agents_snapshot(executor.workspace()) {
            Ok(s) => s,
            Err(e) => {
                return (
                    AssertionBuilder::new("Dynamic worker instance spawned")
                        .expected(
                            ".ralph/agents.json is readable and contains dynamic worker instance",
                        )
                        .actual(e)
                        .failed()
                        .build(),
                    None,
                );
            }
        };

        let worker_instances = snapshot
            .instances
            .iter()
            .filter(|i| i.hat_id == "worker")
            .map(|i| format!("{}(dynamic={})", i.instance_id, i.is_dynamic))
            .collect::<Vec<_>>();

        let dynamic_worker_id = snapshot
            .instances
            .iter()
            .find(|i| i.hat_id == "worker" && i.is_dynamic)
            .map(|i| i.instance_id.clone());

        let ok = dynamic_worker_id.is_some();
        let builder = AssertionBuilder::new("Dynamic worker instance spawned")
            .expected("agents.json contains at least one worker instance with is_dynamic=true")
            .actual(format!("workers={worker_instances:?}"));

        (
            if ok {
                builder.passed().build()
            } else {
                builder.failed().build()
            },
            dynamic_worker_id,
        )
    }

    fn ralph_received_spawn_done(&self, executor: &RalphExecutor) -> crate::models::Assertion {
        let snapshot = match super::read_agents_snapshot(executor.workspace()) {
            Ok(s) => s,
            Err(e) => {
                return AssertionBuilder::new("ralph#1 received spawn.done")
                    .expected("agents.json readable and ralph#1.last_input.topic == spawn.done (marker required)")
                    .actual(e)
                    .failed()
                    .build();
            }
        };

        let ralph = snapshot
            .instances
            .iter()
            .find(|i| i.instance_id == "ralph#1");
        let (topic, preview) = match ralph.and_then(|i| i.last_input.as_ref()) {
            Some(last) => (last.topic.as_str(), last.preview.as_str()),
            None => ("<none>", "<none>"),
        };

        let ok = topic == "spawn.done"
            && preview.contains(E2E_SPAWN_MARKER)
            && preview.contains(E2E_MATH_ANSWER);
        let builder = AssertionBuilder::new("ralph#1 received spawn.done")
            .expected("ralph#1.last_input.topic=spawn.done and preview contains marker+answer")
            .actual(format!("topic={topic}, preview={preview}"));

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn spawn_done_event_recorded(
        &self,
        result: &ExecutionResult,
        dynamic_worker_instance_id: Option<&str>,
    ) -> crate::models::Assertion {
        // -----------------------------------------------------------------
        // 说明:
        // - `.ralph/events.jsonl` 是最稳的端到端证据:
        //   - spawn.done topic 存在
        //   - payload 含 marker
        //   - source_instance 与动态 worker 实例一致
        // -----------------------------------------------------------------
        let candidates = result
            .events
            .iter()
            .filter(|e| e.topic == "spawn.done" && e.payload.contains(E2E_SPAWN_MARKER))
            .filter(|e| {
                e.payload.contains(E2E_MATH_QUESTION) && e.payload.contains(E2E_MATH_ANSWER)
            })
            .collect::<Vec<_>>();

        let source_instances = candidates
            .iter()
            .map(|e| {
                e.source_instance
                    .clone()
                    .unwrap_or_else(|| "<none>".to_string())
            })
            .collect::<Vec<_>>();

        let ok = match dynamic_worker_instance_id {
            Some(expected) => candidates
                .iter()
                .any(|e| e.source_instance.as_deref() == Some(expected)),
            None => false,
        };

        let builder = AssertionBuilder::new("spawn.done event recorded")
            .expected(
                "spawn.done exists with marker+question+answer and source_instance == dynamic worker instance_id",
            )
            .actual(format!(
                "dynamic_worker={dynamic_worker_instance_id:?}, spawn.done_with_marker_sources={source_instances:?}"
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

    fn write_human_log(
        &self,
        executor: &RalphExecutor,
        execution: &ExecutionResult,
        dynamic_worker_instance_id: Option<&str>,
    ) -> Result<(), std::io::Error> {
        // -----------------------------------------------------------------
        // Human log 目标:
        // - 让人类在不打开 report.json 的情况下,快速看懂本场景在验证什么,以及证据在哪里.
        // - 该文件会被 runner 复制到 `.e2e-tests/artifacts/<scenario-id>/`。
        // -----------------------------------------------------------------
        let dir = executor.workspace().join(".e2e");
        std::fs::create_dir_all(&dir)?;

        // -----------------------------------------------------------------
        // 证据留存(重要):
        // - runner 只会把 `${workspace}/.e2e/*` 复制到 artifacts.
        // - 但本场景的强证据来源是 `.ralph/agents.json` 与 `.ralph/events.jsonl`.
        // - 因此这里把它们 best-effort 复制进 `.e2e/`,保证人类可以“离线审计”.
        // -----------------------------------------------------------------
        let ralph_dir = executor.workspace().join(".ralph");
        let copied_agents =
            copy_if_exists(&ralph_dir.join("agents.json"), &dir.join("agents.json"));
        let copied_events =
            copy_if_exists(&ralph_dir.join("events.jsonl"), &dir.join("events.jsonl"));

        // 小工具: 如果源文件存在,则复制到目标路径(覆盖旧文件).
        // - 返回值用于 human-log 展示“是否复制成功”.
        fn copy_if_exists(from: &Path, to: &Path) -> bool {
            if !from.exists() {
                return false;
            }
            std::fs::copy(from, to).is_ok()
        }

        let cmd = format!(
            "ralph emit spawn.task \"marker: {E2E_SPAWN_MARKER}; question: {E2E_MATH_QUESTION}\" --target worker --spawn-instance"
        );

        // -----------------------------------------------------------------
        // stdout 摘录策略:
        // - 人类最关心的其实是“runner 收到了消息并回了回执”.
        // - 因此这里优先摘录:
        //   1) 初始实例列表(解释 worker#1 已存在,所以新动态实例是 worker#2)
        //   2) 动态 worker 输出的 spawn.done `<event ...>` 片段(收发链路证据)
        // -----------------------------------------------------------------
        let supervisor_initial_block = extract_supervisor_initial_block(&execution.stdout);

        let cmd_line = execution
            .stdout
            .lines()
            .find(|l| l.contains("[E2E_CMD]"))
            .unwrap_or("<missing [E2E_CMD] line>");

        let event_emitted_line = execution
            .stdout
            .lines()
            .find(|l| l.contains("Event emitted: spawn.task"))
            .unwrap_or("<missing 'Event emitted: spawn.task' line>");

        let ack_line = execution
            .stdout
            .lines()
            .find(|l| l.contains("ACK") && l.contains(E2E_SPAWN_MARKER))
            .unwrap_or("<missing ACK line>");

        let loop_complete_line = execution
            .stdout
            .lines()
            .find(|l| l.contains("LOOP_COMPLETE"))
            .unwrap_or("<missing LOOP_COMPLETE line>");

        let dynamic_worker_running_line = dynamic_worker_instance_id
            .and_then(|id| {
                let needle = format!("[{id}:state] running");
                execution.stdout.lines().find(|l| l.contains(&needle))
            })
            .unwrap_or("<missing dynamic worker running line>");

        let dynamic_worker_event_block =
            extract_dynamic_worker_spawn_done_block(&execution.stdout, dynamic_worker_instance_id)
                .unwrap_or_else(|| "<missing dynamic worker spawn.done output block>".to_string());

        let spawn_done_sources = execution
            .events
            .iter()
            .filter(|e| e.topic == "spawn.done")
            .map(|e| {
                e.source_instance
                    .clone()
                    .unwrap_or_else(|| "<none>".to_string())
            })
            .collect::<Vec<_>>();

        let content = format!(
            r"# E2E Human Log: {id}

## 目标

- `ralph#1` 在运行中执行 `ralph emit --spawn-instance`,为 `worker` 创建动态实例.
- 动态实例回送 `spawn.done` 到 `ralph#1`.
- `ralph#1` 输出 `LOOP_COMPLETE` 收敛.
- 验证“任务请求/执行/反馈”:
  - question: `{question}`
  - answer: `{answer}`

## Marker

- `{marker}`

## 关键命令(期望)

```bash
{cmd}
```

## 关键证据(摘录)

### 为什么是 worker#2(而不是 worker#1)

- stdout 的初始实例列表(启动时就已创建了 `worker#1`):

```text
{supervisor_initial_block}
```

### 注入侧: ralph emit 确实执行并返回 ACK

- stdout 中的命令行:
  - `{cmd_line}`
- stdout 中的 emit 回执:
  - `{event_emitted_line}`

### runner 侧: 动态 worker 收到任务并回送 spawn.done

- dynamic worker instance_id(来自 agents.json):
  - `{dynamic_worker_instance_id}`
- dynamic worker 进入 running 的证据:
  - `{dynamic_worker_running_line}`
- dynamic worker 输出的 spawn.done `<event ...>` 片段:

```text
{dynamic_worker_event_block}
```

### ralph#1 收到回执并收敛

- stdout 中的 ACK 行:
  - `{ack_line}`
- stdout 中的 completion 行:
  - `{loop_complete_line}`
- spawn.done source_instance(来自 events.jsonl):
  - `{spawn_done_sources:?}`
- termination_reason:
  - `{termination_reason:?}`

## 证据留存(复制到 .e2e 以便 artifacts 持久化)

- `.e2e/agents.json`(来自 `.ralph/agents.json`): copied={copied_agents}
- `.e2e/events.jsonl`(来自 `.ralph/events.jsonl`): copied={copied_events}

## 产物路径

- stdout: `.e2e/stdout.txt`
- stderr: `.e2e/stderr.txt`
- agents snapshot: `.e2e/agents.json`
- events log: `.e2e/events.jsonl`
- 本文件: `.e2e/human-log.md`
",
            id = self.id,
            marker = E2E_SPAWN_MARKER,
            question = E2E_MATH_QUESTION,
            answer = E2E_MATH_ANSWER,
            cmd = cmd,
            supervisor_initial_block = supervisor_initial_block,
            cmd_line = cmd_line.trim_end(),
            event_emitted_line = event_emitted_line.trim_end(),
            ack_line = ack_line.trim_end(),
            loop_complete_line = loop_complete_line.trim_end(),
            dynamic_worker_instance_id = dynamic_worker_instance_id.unwrap_or("<none>"),
            dynamic_worker_running_line = dynamic_worker_running_line.trim_end(),
            dynamic_worker_event_block = dynamic_worker_event_block.trim_end(),
            spawn_done_sources = spawn_done_sources,
            termination_reason = execution.termination_reason,
            copied_agents = copied_agents,
            copied_events = copied_events,
        );

        std::fs::write(dir.join("human-log.md"), content)?;
        Ok(())
    }
}

// =============================================================================
// Human-log helpers
// =============================================================================

/// 从 stdout 中摘录 supervisor 的“初始实例列表”区块.
///
/// 设计动机:
/// - 该区块能解释一个常见困惑: 为什么动态实例是 `worker#2`,而不是 `worker#1`.
/// - 因为并行 supervisor 启动时会先创建配置里声明的静态实例(例如 `worker#1`).
fn extract_supervisor_initial_block(stdout: &str) -> String {
    let mut lines = Vec::new();
    let mut started = false;

    for line in stdout.lines() {
        if !started && line.contains("[supervisor] instances") {
            started = true;
            lines.push(line);
            continue;
        }

        if started {
            // 只摘录初始实例列表里的子项,避免把后续 state 行也塞进 human-log.
            if line.starts_with("  - ") {
                lines.push(line);
                continue;
            }
            break;
        }
    }

    if lines.is_empty() {
        "<missing supervisor initial instances block>".to_string()
    } else {
        lines.join("\n")
    }
}

/// 从 stdout 中摘录“动态 worker 输出 spawn.done 的 `<event ...>` 片段”.
///
/// 注意:
/// - 我们不依赖“模型解释文本”,只摘录严格格式的 event 块(强证据).
fn extract_dynamic_worker_spawn_done_block(
    stdout: &str,
    dynamic_worker_instance_id: Option<&str>,
) -> Option<String> {
    let instance_id = dynamic_worker_instance_id?;

    let prefix = format!("[{instance_id}:out:");
    let mut lines = Vec::new();

    for line in stdout.lines() {
        // 只收集该 worker 的 out 行,并且必须命中关键 token(避免把无关输出塞进 human-log).
        if line.contains(&prefix)
            && (line.contains("spawn.done")
                || line.contains(E2E_SPAWN_MARKER)
                || line.contains(E2E_MATH_QUESTION)
                || line.contains(E2E_MATH_ANSWER)
                || line.contains("question:")
                || line.contains("answer:")
                || line.contains("</event>"))
        {
            lines.push(line);
        }
    }

    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

impl Default for ParallelEmitSpawnInstanceScenario {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TestScenario for ParallelEmitSpawnInstanceScenario {
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

    fn setup(&self, workspace: &Path, backend: Backend) -> Result<ScenarioConfig, ScenarioError> {
        // 创建 `.agent/`（某些代码路径会假设其存在）
        let agent_dir = workspace.join(".agent");
        std::fs::create_dir_all(&agent_dir).map_err(|e| {
            ScenarioError::SetupError(format!("failed to create .agent directory: {e}"))
        })?;

        let cli_config = Self::cli_config_yaml(backend);
        let cmd = format!(
            "ralph emit spawn.task \"marker: {E2E_SPAWN_MARKER}; question: {E2E_MATH_QUESTION}\" --target worker --spawn-instance"
        );

        // 说明：
        // - `event_loop.ralph_prompt` 是 Ralph-only 注入,不会污染 worker 的 prompt。
        // - 我们把“最强约束语义”放在这里,让场景更稳定。
        let config_content = format!(
            r#"# Parallel emit spawn_instance E2E config for {backend}
cli:
{cli_config}

event_loop:
  completion_promise: "LOOP_COMPLETE"
  # workflow entry/exit（官方并行语义锚点）
  starting_event: "spawn.task"
  complete_publishes: "spawn.done"
  max_iterations: 12
  max_runtime_seconds: 120
  # Ralph-only prompt: 强约束 `ralph#1` 行为,避免 prompt 污染其它 hats
  ralph_prompt: |
    # E2E: parallel-emit-spawn-instance

    Marker: {marker}

    ## If you receive task.start (fresh)
    - You MUST emit the workflow entry event `spawn.task` using the CLI `ralph emit`.
    - You MUST NOT output any `<event ...>` blocks in this job (especially do NOT output `<event topic=\"spawn.task\">`).
    - First print EXACTLY ONE line that starts with `[E2E_CMD]`:
      [E2E_CMD] {cmd}
    - Then execute the exact command above (tool/shell).
    - Then stop. Do NOT output LOOP_COMPLETE.

    ## If you receive spawn.done
    - Output two lines and stop:
      ACK marker: {marker} answer: {answer}
      LOOP_COMPLETE

parallel:
  enabled: true
  autoscale:
    max_running_jobs: 4
    dynamic_idle_ttl_secs: 30

  # E2E 场景中不启用 ask gate，避免“等待 human”导致卡住
  permissions:
    worktree: allow
    hooks: allow

hats:
  worker:
    name: "Worker"
    description: "Replies spawn.done to ralph#1"
    instances: 1
    triggers:
      - spawn.task
    publishes:
      - spawn.done
    instructions: |
      You are Worker.

      IMPORTANT (E2E harness):
      - Do NOT run tests, do NOT run shell commands/tools, do NOT edit files.
      - Do NOT output LOOP_COMPLETE.

      When you receive spawn.task:
      - Emit EXACTLY ONE spawn.done event to ralph#1 using this exact format (no code fences):

      <event topic="spawn.done" target_instance="ralph#1">
      marker: {marker}
      question: {question}
      answer: {answer}
      </event>

      Then stop.
"#,
            backend = backend,
            cli_config = cli_config,
            marker = E2E_SPAWN_MARKER,
            question = E2E_MATH_QUESTION,
            answer = E2E_MATH_ANSWER,
            cmd = cmd,
        );

        std::fs::write(workspace.join("ralph.yml"), config_content)
            .map_err(|e| ScenarioError::SetupError(format!("failed to write ralph.yml: {e}")))?;

        // 说明：
        // - prompt 文件保持极简,避免把 coordinator 的细节提示注入其它 hat。
        // - 真实约束语义放在 `event_loop.ralph_prompt`。
        let prompt_content = r"# E2E Prompt

请严格遵守 `event_loop.ralph_prompt` 的协议.
不要自作主张修改流程.
";
        std::fs::write(workspace.join("PROMPT.md"), prompt_content)
            .map_err(|e| ScenarioError::SetupError(format!("failed to write PROMPT.md: {e}")))?;

        Ok(ScenarioConfig {
            config_file: "ralph.yml".into(),
            prompt: PromptSource::Config,
            max_iterations: 20,
            timeout: std::cmp::min(backend.default_timeout(), Duration::from_secs(300)),
            extra_args: vec!["--no-tui".to_string()],
        })
    }

    async fn run(
        &self,
        executor: &RalphExecutor,
        config: &ScenarioConfig,
    ) -> Result<TestResult, ScenarioError> {
        let start = std::time::Instant::now();

        let execution = executor
            .run(config)
            .await
            .map_err(|e| ScenarioError::ExecutionError(format!("ralph execution failed: {e}")))?;

        let duration = start.elapsed();

        let (dynamic_worker_assertion, dynamic_worker_instance_id) =
            self.dynamic_worker_spawned(executor);

        // Human log：即使断言失败也尽量落盘,方便排障.
        let _ = self.write_human_log(executor, &execution, dynamic_worker_instance_id.as_deref());

        let assertions = vec![
            Assertions::response_received(&execution),
            Assertions::exit_code_success_or_limit(&execution),
            Assertions::no_timeout(&execution),
            self.parallel_mode_visible(&execution),
            self.e2e_cmd_printed(&execution),
            dynamic_worker_assertion,
            self.spawn_done_event_recorded(&execution, dynamic_worker_instance_id.as_deref()),
            self.ralph_received_spawn_done(executor),
            self.loop_complete_detected(&execution),
            self.human_log_written(executor),
        ];

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
