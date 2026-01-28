//! Tier 8: Parallel Runtime (experimental) test scenarios.
//!
//! 说明：
//! - 这些场景用于验证 **parallel hat instances** 在“真实后端”上的端到端行为。
//! - 与 replay smoke tests 的差异：
//!   - E2E 会覆盖真实 CLI、真实认证、真实网络与真实模型漂移带来的风险
//!   - 代价更高、速度更慢，因此场景应尽量“短、稳、可排障”

use super::{AssertionBuilder, Assertions, ScenarioError, TestScenario};
use crate::Backend;
use crate::executor::{ExecutionResult, PromptSource, RalphExecutor, ScenarioConfig};
use crate::models::TestResult;
use async_trait::async_trait;
use std::path::Path;
use std::time::Duration;

// =============================================================================
// ParallelHatInstancesScenario - Validate parallel hat instances end-to-end
// =============================================================================

/// 验证并行 HatInstance（headless）在真实后端上能跑通最小闭环。
///
/// 关注点（E2E 维度）：
/// - `parallel.enabled=true` 时，Supervisor 能启动多个实例（含同一 hat 的多实例）
/// - 不写 `topic_contracts` 时，默认按 `hats.*.triggers` 路由（topic → hats fanout）
/// - 输出归因可观测（stdout 带 `[writer#1:out]` 这类前缀）
/// - `<event ...>` 能被解析并写入 `.ralph/events*.jsonl`
/// - 目标校验失败会触发 `routing.escalate`（可观测信号）
///
/// 注意：
/// - E2E 环境无法交互回答 `gate.request`，因此该场景不启用 worktree/hooks 权限 gate。
pub struct ParallelHatInstancesScenario {
    id: String,
    description: String,
    tier: String,
}

impl ParallelHatInstancesScenario {
    pub fn new() -> Self {
        Self {
            id: "parallel-hat-instances".to_string(),
            // 说明：
            // - 这个场景最初用于验证 parallel hat instances。
            // - 现在也覆盖 `parallel-trigger-routing`：不写 topic_contracts 时的 triggers 默认路由、
            //   strict target 校验、以及 autoscale 的可观测闭环。
            description: "Validates parallel-trigger-routing in parallel runtime (triggers fanout + autoscale + strict target)"
                .to_string(),
            tier: "Tier 8: Parallel Runtime".to_string(),
        }
    }

    fn parallel_mode_visible(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let stdout = &result.stdout;
        let visible = stdout.contains("[supervisor] instances");
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

    fn attributed_outputs_visible(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let stdout = &result.stdout;

        // 这里不依赖模型在 payload 里回显 instance_id，而是依赖 runner 的“日志归因前缀”。
        let has_writer_1 = stdout.contains("[writer#1:out]") || stdout.contains("[writer#1:state]");
        let has_writer_2 = stdout.contains("[writer#2:out]") || stdout.contains("[writer#2:state]");
        let has_tester_1 = stdout.contains("[tester#1:out]") || stdout.contains("[tester#1:state]");
        let ok = has_writer_1 && has_writer_2 && has_tester_1;

        let builder = AssertionBuilder::new("Attributed instance output")
            .expected("stdout shows writer#1 + writer#2 + tester#1 output/state prefixes")
            .actual(format!(
                "writer#1: {}, writer#2: {}, tester#1: {}",
                has_writer_1, has_writer_2, has_tester_1
            ));

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn expected_events_recorded(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let build_task_count = result
            .events
            .iter()
            .filter(|e| e.topic == "build.task")
            .count();
        let build_done_count = result
            .events
            .iter()
            .filter(|e| e.topic == "build.done")
            .count();
        let test_done_count = result
            .events
            .iter()
            .filter(|e| e.topic == "test.done")
            .count();

        // 说明：
        // - 本场景会触发两次 build.task：
        //   1) fanout -> writer#1 + tester#1
        //   2) target=writer -> 触发 autoscale，期望出现 writer#2
        let ok = build_task_count >= 2 && build_done_count >= 2 && test_done_count >= 1;
        let builder = AssertionBuilder::new("Parallel events recorded")
            .expected("events.jsonl contains >=2 build.task, >=2 build.done, >=1 test.done")
            .actual(format!(
                "build.task: {}, build.done: {}, test.done: {}",
                build_task_count, build_done_count, test_done_count
            ));

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn routing_escalate_recorded(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let has_escalate = result.events.iter().any(|e| e.topic == "routing.escalate");

        let builder = AssertionBuilder::new("routing.escalate recorded")
            .expected("events.jsonl contains routing.escalate (invalid target must be rejected)")
            .actual(if has_escalate {
                "Found routing.escalate".to_string()
            } else {
                "Missing routing.escalate".to_string()
            });

        if has_escalate {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }
}

impl Default for ParallelHatInstancesScenario {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TestScenario for ParallelHatInstancesScenario {
    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn tier(&self) -> &str {
        &self.tier
    }

    /// 目前先限制在 Codex：
    /// - 并行场景对 headless/并发稳定性更敏感
    /// - 先把 Codex 跑稳，后续再扩展到更多后端
    fn supported_backends(&self) -> Vec<Backend> {
        vec![Backend::Codex]
    }

    fn setup(&self, workspace: &Path, backend: Backend) -> Result<ScenarioConfig, ScenarioError> {
        // 创建 `.agent/`（某些代码路径会假设其存在）
        let agent_dir = workspace.join(".agent");
        std::fs::create_dir_all(&agent_dir).map_err(|e| {
            ScenarioError::SetupError(format!("failed to create .agent directory: {e}"))
        })?;

        // 配置目标：
        // - 启用 parallel runtime（不写 topic_contracts）
        // - 依赖 triggers 默认路由：build.task fanout 到 writer/tester
        // - 第二个 build.task 通过 target=writer 收敛到 writer，并触发 autoscale（writer#2）
        // - 插入一个非法 target，验证 strict target 校验会触发 routing.escalate
        let config_content = format!(
            r#"# Parallel hat instances E2E config for {backend}
cli:
  backend: {cli_backend}

event_loop:
  completion_promise: "LOOP_COMPLETE"
  max_iterations: 6
  # 并行模式下如果模型不输出 completion promise，必须有硬退出护栏，避免 E2E 卡死
  max_runtime_seconds: 120

parallel:
  enabled: true

  # E2E 场景中不启用 ask gate，避免“等待 human”导致卡住
  permissions:
    worktree: allow
    hooks: allow

hats:
  writer:
    name: "Writer"
    description: "Writes a short output and emits build.done."
    instances: 1
    triggers: ["build.task"]
    publishes: ["build.done"]
    instructions: |
      You are Writer.

      When you receive a build.task event:
      IMPORTANT (E2E harness):
      - Do NOT run tests, do NOT run shell commands/tools, do NOT edit files.
      - Emit the output line + the event immediately.

      1) Print 30 short lines that include the word "writer" (slow down a bit to exercise autoscale)
      2) Emit ONE build.done event using this exact XML format:

      <event topic="build.done">
      status: ok
      </event>

      Do NOT output LOOP_COMPLETE.

  tester:
    name: "Tester"
    description: "Emits test.done quickly."
    instances: 1
    triggers: ["build.task"]
    publishes: ["test.done"]
    instructions: |
      You are Tester.

      When you receive a build.task event:
      IMPORTANT (E2E harness):
      - Do NOT run tests, do NOT run shell commands/tools, do NOT edit files.
      - Emit the output line + the event immediately.

      1) Print one short line that includes the word "tester"
      2) Emit ONE test.done event using this exact XML format:

      <event topic="test.done">
      status: ok
      </event>

      Do NOT output LOOP_COMPLETE.
"#,
            backend = backend,
            cli_backend = backend.as_config_str(),
        );

        let config_path = workspace.join("ralph.yml");
        std::fs::write(&config_path, config_content)
            .map_err(|e| ScenarioError::SetupError(format!("failed to write ralph.yml: {e}")))?;

        // Prompt 目标：
        // - 触发一次 build.task fanout（writer + tester 并发）
        // - 插入一点“非事件文本”作为时间缝隙，让 writer#1 进入 Running
        // - 再触发一次 build.task 且 target=writer，期望触发 autoscale（writer#2）
        // - 再触发一次非法 target，期望生成 routing.escalate
        // - 最后输出 LOOP_COMPLETE
        //
        // 说明：
        // - parallel runtime 会在检测到 completion_promise 后做短暂 drain，
        //   给 writer/tester 把 build.done/test.done 跑完并落盘的机会。
        let prompt = r#"You are running an E2E test for Ralph's EXPERIMENTAL parallel hat instances runtime.

STEP 1: Emit this event EXACTLY as shown (including whitespace):
<event topic="build.task">
Task: Print one short line and emit completion events
</event>

STEP 2: Output 5 lines of plain text (NO events). This creates a small timing gap.

STEP 3: Emit this event EXACTLY as shown (including whitespace):
<event topic="build.task" target="writer">
Task: Second task to exercise autoscale (writer#2)
</event>

STEP 4: Emit this event EXACTLY as shown (including whitespace):
<event topic="build.task" target="ghost_hat">
Task: This must be rejected and should trigger routing.escalate
</event>

STEP 5: Output LOOP_COMPLETE on its own line.

IMPORTANT:
- Do not output any other events.
- Do not wrap the event in code fences.
"#;

        Ok(ScenarioConfig {
            config_file: "ralph.yml".into(),
            prompt: PromptSource::Inline(prompt.to_string()),
            max_iterations: backend.default_max_iterations().max(6),
            // 与 ralph.yml 的 max_runtime_seconds 对齐：避免 E2E 在模型漂移时挂到 10 分钟。
            timeout: std::cmp::min(backend.default_timeout(), Duration::from_secs(300)),
            extra_args: vec![],
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

        let assertions = vec![
            Assertions::response_received(&execution),
            Assertions::exit_code_success_or_limit(&execution),
            Assertions::no_timeout(&execution),
            self.parallel_mode_visible(&execution),
            self.attributed_outputs_visible(&execution),
            self.expected_events_recorded(&execution),
            self.routing_escalate_recorded(&execution),
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
