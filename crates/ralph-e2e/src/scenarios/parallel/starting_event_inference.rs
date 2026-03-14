use super::super::{AssertionBuilder, Assertions, ScenarioError, TestScenario};
use crate::Backend;
use crate::executor::{ExecutionResult, PromptSource, RalphExecutor, ScenarioConfig};
use crate::models::TestResult;
use async_trait::async_trait;
use std::path::Path;
use std::time::Duration;

// =============================================================================
// ParallelStartingEventInferenceScenario - Validate starting_event inference
// =============================================================================

/// starting_event 推测场景的变体类型。
///
/// 说明：
/// - 我们用“同一个 struct + 不同 id/config”的方式，避免复制粘贴两份几乎一样的场景代码。
/// - `SingleCandidate`：derived entry candidates 退化为单元素（稳定强断言）
/// - `MultiCandidate`：存在多个 entry candidates，但 prompt 给出明确 workflow 顺序约束（仍可稳定强断言）
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StartingEventInferenceVariant {
    SingleCandidate,
    MultiCandidate,
}

/// 验证：当 `event_loop.starting_event` 未配置时，`ralph#1` 能从拓扑推测并发布正确入口事件。
///
/// 设计要点（稳定性优先）：
/// - 我们让 derived entry candidates 退化为单元素：`spec.start`
///   - `spec.start`：planner 的 trigger（未被任何 hat publishes）→ entry candidate
///   - `build.task`：builder 的 trigger，但它会被 planner publishes → 不再是 entry candidate
/// - 因此在 `task.start` 后，`ralph#1` 的第一个 workflow entry topic 应稳定为 `spec.start`。
pub struct ParallelStartingEventInferenceScenario {
    id: String,
    description: String,
    tier: String,
    variant: StartingEventInferenceVariant,
}

impl ParallelStartingEventInferenceScenario {
    pub fn new() -> Self {
        Self {
            id: "parallel-starting-event-inference".to_string(),
            description:
                "Validates ralph#1 infers workflow entry event when starting_event is not set"
                    .to_string(),
            tier: "Tier 8: Parallel Runtime".to_string(),
            variant: StartingEventInferenceVariant::SingleCandidate,
        }
    }

    /// 变体：derived entry candidates 为多元素，但 workflow 顺序约束让入口选择变得可判定。
    ///
    /// 说明：
    /// - 该场景的价值在于：更贴近真实配置（存在“无关 hat/入口”），但仍保持稳定断言口径。
    /// - 断言口径：`task.start` 后 `ralph#1` 的第一个 workflow entry event 必须仍为 `spec.start`
    ///   （因为 prompt 明确要求 Planner 先跑，而 Planner 的 trigger 是 `spec.start`）。
    pub fn new_multi_candidate() -> Self {
        Self {
            id: "parallel-starting-event-inference-multi-candidate".to_string(),
            description: "Validates ralph#1 chooses correct workflow entry when multiple entry candidates exist".to_string(),
            tier: "Tier 8: Parallel Runtime".to_string(),
            variant: StartingEventInferenceVariant::MultiCandidate,
        }
    }

    fn cli_config_yaml(backend: Backend) -> String {
        // ---------------------------------------------------------------------
        // 说明：
        // - 该 helper 只影响 E2E workspace 里的 `ralph.yml`，不影响仓库默认配置。
        // - 并行场景在真实 Codex 下容易被“长篇思考/总结输出”拖慢收敛，并污染 stderr 诊断信息。
        // - 这里用 `custom` 后端精确注入 codex 参数，做到降噪/提速而不改默认设置。
        // ---------------------------------------------------------------------
        match backend {
            Backend::Codex => r#"  backend: custom
  command: codex
  args:
    - exec
    - -m
    - gpt-5-codex
    - --full-auto
    - -c
    - 'model_reasoning_effort="low"'
    - -c
    - 'model_reasoning_summary="none"'
    - -c
    - 'rmcp_client=false'
"#
            .to_string(),
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

    fn distractor_not_used(&self, result: &ExecutionResult) -> crate::models::Assertion {
        // 说明：
        // - 仅在 MultiCandidate 变体里存在 `docs.*` hat。
        // - 我们期望 ralph#1 直接选择触发 Planner 的入口事件（spec.start），因此 docs.* 不应出现。
        let has_docs_start = result.events.iter().any(|e| e.topic == "docs.start");
        let has_docs_done = result.events.iter().any(|e| e.topic == "docs.done");

        let ok = !has_docs_start && !has_docs_done;
        let builder = AssertionBuilder::new("Distractor not used (docs.*)")
            .expected("events.jsonl does NOT contain docs.start/docs.done")
            .actual(format!(
                "docs.start={}, docs.done={}",
                has_docs_start, has_docs_done
            ));

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn workflow_entry_inferred(&self, result: &ExecutionResult) -> crate::models::Assertion {
        // 说明：
        // - 该断言只关心 ralph#1 在 `task.start` 后发出的“第一个 workflow entry event”。
        // - `SingleCandidate`：derived candidates 退化为单元素（`spec.start`），可做强断言。
        // - `MultiCandidate`：存在多个 derived candidates，但 prompt 明确要求 Planner 先跑，
        //   因此第一个入口事件仍应稳定为 `spec.start`。
        let mut first_entry: Option<&str> = None;

        for e in &result.events {
            if e.source_instance.as_deref() != Some("ralph#1") {
                continue;
            }
            if matches!(e.topic.as_str(), "task.start" | "task.resume") {
                continue;
            }
            first_entry = Some(e.topic.as_str());
            break;
        }

        let ok = first_entry == Some("spec.start");
        let builder = AssertionBuilder::new("Workflow entry inferred (starting_event not set)")
            .expected("First ralph#1 workflow entry event is 'spec.start'")
            .actual(match first_entry {
                Some(topic) => format!("first_entry={topic}"),
                None => "first_entry=<none>".to_string(),
            });

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn workflow_progressed(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let has_spec_start = result.events.iter().any(|e| e.topic == "spec.start");
        let has_build_task = result.events.iter().any(|e| e.topic == "build.task");
        let has_build_done = result.events.iter().any(|e| e.topic == "build.done");

        let ok = has_spec_start && has_build_task && has_build_done;
        let builder =
            AssertionBuilder::new("Workflow progressed (spec.start → build.task → build.done)")
                .expected("events.jsonl contains spec.start, build.task, build.done")
                .actual(format!(
                    "spec.start={}, build.task={}, build.done={}",
                    has_spec_start, has_build_task, has_build_done
                ));

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn loop_complete_detected(&self, result: &ExecutionResult) -> crate::models::Assertion {
        // 说明：parallel 模式下，completion promise 仍然来自 stdout 文本，因此直接复用 termination_reason。
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

    fn agents_snapshot_written(&self, executor: &RalphExecutor) -> crate::models::Assertion {
        // -----------------------------------------------------------------
        // 说明:
        // - `.ralph/agents.json` 是最近新增的并行可观测性能力.
        // - 这个断言不关心具体实例数量(避免 autoscale/动态实例引入 flaky),
        //   只要求:
        //   1) 文件存在且 JSON 可解析
        //   2) 至少包含本场景的关键 hat: planner/builder
        // -----------------------------------------------------------------
        let snapshot = match super::read_agents_snapshot(executor.workspace()) {
            Ok(s) => s,
            Err(e) => {
                return AssertionBuilder::new("Agents snapshot written")
                    .expected(".ralph/agents.json exists and is valid JSON")
                    .actual(e)
                    .failed()
                    .build();
            }
        };

        let instance_count = snapshot.instances.len();
        let has_planner = snapshot.instances.iter().any(|i| i.hat_id == "planner");
        let has_builder = snapshot.instances.iter().any(|i| i.hat_id == "builder");

        let ok = instance_count >= 2 && has_planner && has_builder;
        let builder = AssertionBuilder::new("Agents snapshot written")
            .expected("agents.json contains planner + builder (and instance_count>=2)")
            .actual(format!(
                "instance_count={instance_count}, has_planner={has_planner}, has_builder={has_builder}"
            ));

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }
}

impl Default for ParallelStartingEventInferenceScenario {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TestScenario for ParallelStartingEventInferenceScenario {
    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn tier(&self) -> &str {
        &self.tier
    }

    /// 说明：
    /// - 先限制在 Codex，减少“多后端 + 并行语义 + 模型漂移”叠加导致的 flaky。
    fn supported_backends(&self) -> Vec<Backend> {
        vec![Backend::Codex]
    }

    fn setup(&self, workspace: &Path, backend: Backend) -> Result<ScenarioConfig, ScenarioError> {
        // 创建 `.agent/`（某些代码路径会假设其存在）
        let agent_dir = workspace.join(".agent");
        std::fs::create_dir_all(&agent_dir).map_err(|e| {
            ScenarioError::SetupError(format!("failed to create .agent directory: {e}"))
        })?;

        // 关键点：starting_event 不配置，让 ralph#1 自行推测入口事件。
        // 收敛：当 ralph#1 观察到 build.done（complete_publishes）后输出 LOOP_COMPLETE。
        let cli_config = Self::cli_config_yaml(backend);
        let config_content = match self.variant {
            StartingEventInferenceVariant::SingleCandidate => format!(
                r#"# Parallel starting_event inference E2E config for {backend}
cli:
{cli_config}

event_loop:
  completion_promise: "LOOP_COMPLETE"
  complete_publishes: "build.done"
  max_iterations: 12
  max_runtime_seconds: 120

parallel:
  enabled: true
  autoscale:
    max_running_jobs: 2
    dynamic_idle_ttl_secs: 30

  permissions:
    worktree: allow
    hooks: allow

hats:
  planner:
    name: "Planner"
    description: "Converts spec.start into build.task"
    triggers:
      - spec.start
    publishes:
      - build.task
    instructions: |
      You are Planner.

      When you receive `spec.start`:
      - Do NOT implement code.
      - Emit EXACTLY ONE `build.task` event using this exact format:

      <event topic="build.task">
      from: planner
      </event>

      Then stop.

  builder:
    name: "Builder"
    description: "Converts build.task into build.done"
    triggers:
      - build.task
    publishes:
      - build.done
    instructions: |
      You are Builder.

      When you receive `build.task`:
      - Do NOT implement code.
      - Emit EXACTLY ONE `build.done` event using this exact format:

      <event topic="build.done">
      status: ok
      </event>

      Then stop.
"#,
                backend = backend,
                cli_config = cli_config,
            ),
            StartingEventInferenceVariant::MultiCandidate => format!(
                r#"# Parallel starting_event inference (multi-candidate) E2E config for {backend}
cli:
{cli_config}

event_loop:
  completion_promise: "LOOP_COMPLETE"
  complete_publishes: "build.done"
  max_iterations: 12
  max_runtime_seconds: 120

parallel:
  enabled: true
  autoscale:
    max_running_jobs: 2
    dynamic_idle_ttl_secs: 30

  permissions:
    worktree: allow
    hooks: allow

hats:
  planner:
    name: "Planner"
    description: "Converts spec.start into build.task"
    triggers:
      - spec.start
    publishes:
      - build.task
    instructions: |
      You are Planner.

      When you receive `spec.start`:
      - Do NOT implement code.
      - Emit EXACTLY ONE `build.task` event using this exact format:

      <event topic="build.task">
      from: planner
      </event>

      Then stop.

  builder:
    name: "Builder"
    description: "Converts build.task into build.done"
    triggers:
      - build.task
    publishes:
      - build.done
    instructions: |
      You are Builder.

      When you receive `build.task`:
      - Do NOT implement code.
      - Emit EXACTLY ONE `build.done` event using this exact format:

      <event topic="build.done">
      status: ok
      </event>

      Then stop.

  docs:
    name: "Docs"
    description: "Distractor: docs.start → docs.done (not part of completion)"
    triggers:
      - docs.start
    publishes:
      - docs.done
    instructions: |
      You are Docs.

      When you receive `docs.start`:
      - Do NOT implement code.
      - Emit EXACTLY ONE `docs.done` event using this exact format:

      <event topic="docs.done">
      status: ok
      </event>

      Then stop.
"#,
                backend = backend,
                cli_config = cli_config,
            ),
        };
        std::fs::write(workspace.join("ralph.yml"), config_content)
            .map_err(|e| ScenarioError::SetupError(format!("failed to write ralph.yml: {e}")))?;

        // 注意：
        // - 不在 prompt 中“点名 spec.start”，避免把测试退化成“照抄答案”。
        // - 目标描述为：启动 planner→builder 的最小链路，并在 build.done 后收敛。
        let prompt: &str = match self.variant {
            StartingEventInferenceVariant::SingleCandidate => {
                r"You are running an E2E test for Ralph's parallel runtime.

Objective:
- `event_loop.starting_event` is intentionally NOT set.
- You MUST infer the correct workflow entry event from the hat topology and start the workflow.
- This workflow is: Planner runs first, then Builder runs.
- When you observe the completion candidate event `build.done`, output `LOOP_COMPLETE` on its own line and stop.

Constraints:
- Do NOT implement code.
- Do NOT run tools or commands.
- Do NOT edit files.
"
            }
            StartingEventInferenceVariant::MultiCandidate => {
                r"You are running an E2E test for Ralph's parallel runtime.

Objective:
- `event_loop.starting_event` is intentionally NOT set.
- You MUST infer the correct workflow entry event from the hat topology and start the workflow.
- This workflow is: Planner runs first, then Builder runs.
- There may be hats that are NOT part of this workflow. Ignore them.
- When you observe the completion candidate event `build.done`, output `LOOP_COMPLETE` on its own line and stop.

Constraints:
- Do NOT implement code.
- Do NOT run tools or commands.
- Do NOT edit files.
"
            }
        };

        Ok(ScenarioConfig {
            config_file: "ralph.yml".into(),
            prompt: PromptSource::Inline(prompt.to_string()),
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

        let mut assertions = vec![
            Assertions::response_received(&execution),
            Assertions::exit_code_success_or_limit(&execution),
            Assertions::no_timeout(&execution),
            self.parallel_mode_visible(&execution),
            self.agents_snapshot_written(executor),
            self.workflow_entry_inferred(&execution),
            self.workflow_progressed(&execution),
            self.loop_complete_detected(&execution),
        ];

        if self.variant == StartingEventInferenceVariant::MultiCandidate {
            assertions.push(self.distractor_not_used(&execution));
        }

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
