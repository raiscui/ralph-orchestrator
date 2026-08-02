//! Tier 8: Parallel Runtime (experimental) - example coverage scenarios.
//!
//! 目标：
//! - 直接跑仓库自带的 example：`examples/parallel-trigger-routing`
//! - 用“并行 stdout 的 job_id 去重统计”来断言 hat 的运行次数

use super::parallel::{
    JobRunCounts, parse_parallel_job_line, read_agents_snapshot, replace_top_level_yaml_block,
};
use super::{AssertionBuilder, Assertions, ScenarioError, TestScenario};
use crate::Backend;
use crate::executor::{ExecutionResult, PromptSource, RalphExecutor, ScenarioConfig};
use crate::models::TestResult;
use async_trait::async_trait;
use std::collections::HashSet;
use std::path::Path;
use std::time::Duration;

/// 直接覆盖 `examples/parallel-trigger-routing` 的端到端（E2E）场景。
///
/// 关注点：
/// - **不改示例配置**（hats/triggers/workflow 原样使用），只把示例的 `ralph.yml` 拷贝到 E2E workspace
/// - 统计并行 stdout 的 `job_id`，断言 `spec_writer` 总运行次数 == 2（跨 instance 汇总）
pub struct ParallelTriggerRoutingExampleScenario {
    id: String,
    description: String,
    tier: String,
}

impl ParallelTriggerRoutingExampleScenario {
    pub fn new() -> Self {
        Self {
            id: "parallel-trigger-routing-example".to_string(),
            description:
                "Directly runs examples/parallel-trigger-routing and asserts deterministic hat job run counts"
                    .to_string(),
            tier: "Tier 8: Parallel Runtime".to_string(),
        }
    }

    fn patch_example_config_for_e2e(
        config_content: &str,
        backend: Backend,
    ) -> Result<String, ScenarioError> {
        // ---------------------------------------------------------------------
        // 说明：
        // - 该场景的原则是“example 行为不改”，因此我们不改仓库里的示例文件。
        // - 但在真实 Codex 下，默认配置可能会输出长篇思考/总结，导致 E2E 噪音变大、耗时抖动。
        // - 这里仅在 E2E workspace 里覆写 `cli` 段，注入 `codex exec -c ...` 降噪/提速参数。
        // ---------------------------------------------------------------------
        if backend != Backend::Codex {
            return Ok(config_content.to_string());
        }

        let model = super::parallel::codex_e2e_model();
        let cli_block = format!(
            r#"cli:
  # E2E: 覆写 Codex 参数,降噪/提速(不影响仓库 example 原文件).
  backend: custom
  command: codex
  args:
    - exec
    - -m
    - {model}
    - --full-auto
    - -c
    - 'model_reasoning_effort="low"'
    - -c
    - 'model_reasoning_summary="none"'
    - -c
    - 'rmcp_client=false'
    - -c
    - 'features.hooks=false'

"#
        );

        replace_top_level_yaml_block(config_content, "cli:", &cli_block).map_err(|e| {
            ScenarioError::SetupError(format!(
                "failed to patch example ralph.yml cli block for e2e: {e}"
            ))
        })
    }

    fn parallel_mode_visible(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let stdout = &result.stdout;
        let visible = stdout.contains("[supervisor] instances");

        let builder = AssertionBuilder::new("Parallel mode visible (example)")
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

    fn hat_run_counts_expected(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let counts = JobRunCounts::from_stdout(&result.stdout);

        // 示例闭环（确定性）：
        // - spec_writer：2 次（spec.start -> version:1；spec.rejected -> version:2）
        // - spec_reviewer：2 次（两次 spec.ready：一次 reject，一次 approve）
        // - spec_logger：3 次（2×spec.ready + 1×spec.rejected；instances=2，但总次数应固定）
        let expected = [("spec_writer", 2), ("spec_reviewer", 2), ("spec_logger", 3)];

        let mut mismatches = Vec::new();
        for (hat, expected_runs) in expected {
            let got = counts.runs_for_hat(hat);
            if got != expected_runs {
                mismatches.push(format!(
                    "{hat}: expected job_runs {expected_runs}, got {got}"
                ));
            }
        }

        let ok = mismatches.is_empty();
        let builder = AssertionBuilder::new("Hat job run counts (example)")
            .expected(
                "spec_writer_jobs=2, spec_reviewer_jobs=2, spec_logger_jobs=3 (aggregated by hat)",
            )
            .actual(if ok {
                format!(
                    "hats: {}; instances: {}",
                    counts.hat_summary(),
                    counts.summary()
                )
            } else {
                format!(
                    "hats: {}; instances: {}; mismatches: {}",
                    counts.hat_summary(),
                    counts.summary(),
                    mismatches.join("; ")
                )
            });

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn no_new_jobs_started_after_loop_complete(
        &self,
        result: &ExecutionResult,
    ) -> crate::models::Assertion {
        let completion_promise = "LOOP_COMPLETE";
        let mut completion_seen = false;

        let mut jobs_before: HashSet<(String, u64)> = HashSet::new();
        let mut new_jobs_after: HashSet<(String, u64)> = HashSet::new();

        for line in result.stdout.lines() {
            if let Some((instance_id, job_id)) = parse_parallel_job_line(line) {
                let key = (instance_id, job_id);
                if completion_seen {
                    if !jobs_before.contains(&key) {
                        new_jobs_after.insert(key);
                    }
                } else {
                    jobs_before.insert(key);
                }
            }

            // 注意：必须在解析 job_id 之后再判断 completion，
            // 这样 `[ralph#1:out:job=...] LOOP_COMPLETE` 会被算作 completion 之前的 job。
            //
            // 同时我们只认可 **协调者** 输出的 completion promise：
            // - 避免 prompt/instructions 中出现的 “不要输出 LOOP_COMPLETE” 误触发断言。
            if !completion_seen
                && line.trim_end().ends_with(completion_promise)
                && line.trim_start().starts_with("[ralph#")
                && line.contains(":out:job=")
            {
                completion_seen = true;
            }
        }

        let mut new_list = new_jobs_after.into_iter().collect::<Vec<_>>();
        new_list.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

        let ok = completion_seen && new_list.is_empty();
        let builder = AssertionBuilder::new("No new jobs after LOOP_COMPLETE (example)")
            .expected("After LOOP_COMPLETE, no new job_id should appear in stdout")
            .actual(format!(
                "completion_seen={}, new_jobs_after={:?}",
                completion_seen, new_list
            ));

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn agents_snapshot_written(&self, executor: &RalphExecutor) -> crate::models::Assertion {
        // -----------------------------------------------------------------
        // 说明:
        // - `.ralph/agents.json` 是并行 Supervisor 的运行态快照,用于 `ralph agents`。
        // - 本 example 的目标之一是展示“多实例 + 多 hat 并行”在运行期可观测,
        //   因此 E2E 要求该快照存在且至少包含关键 hat。
        // -----------------------------------------------------------------
        let snapshot = match read_agents_snapshot(executor.workspace()) {
            Ok(s) => s,
            Err(e) => {
                return AssertionBuilder::new("Agents snapshot written (example)")
                    .expected(".ralph/agents.json exists and is valid JSON")
                    .actual(e)
                    .failed()
                    .build();
            }
        };

        let instance_count = snapshot.instances.len();
        let has_writer = snapshot.instances.iter().any(|i| i.hat_id == "spec_writer");
        let has_reviewer = snapshot
            .instances
            .iter()
            .any(|i| i.hat_id == "spec_reviewer");
        let has_logger = snapshot.instances.iter().any(|i| i.hat_id == "spec_logger");

        let ok = instance_count >= 3 && has_writer && has_reviewer && has_logger;
        let builder = AssertionBuilder::new("Agents snapshot written (example)")
            .expected("agents.json contains spec_writer + spec_reviewer + spec_logger (and instance_count>=3)")
            .actual(format!(
                "instance_count={instance_count}, writer={has_writer}, reviewer={has_reviewer}, logger={has_logger}"
            ));

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }
}

impl Default for ParallelTriggerRoutingExampleScenario {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TestScenario for ParallelTriggerRoutingExampleScenario {
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

        let root = crate::executor::find_workspace_root().ok_or_else(|| {
            ScenarioError::SetupError("failed to find workspace root (Cargo.toml)".to_string())
        })?;

        let example_config_path = root.join("examples/parallel-trigger-routing/ralph.yml");
        let config_content = std::fs::read_to_string(&example_config_path).map_err(|e| {
            ScenarioError::SetupError(format!(
                "failed to read example config {}: {e}",
                example_config_path.display()
            ))
        })?;

        // 原样拷贝示例 config,但会在 E2E workspace 覆写 cli 段的 Codex 参数（只影响 E2E）。
        let patched = Self::patch_example_config_for_e2e(&config_content, backend)?;
        std::fs::write(workspace.join("ralph.yml"), patched).map_err(|e| {
            ScenarioError::SetupError(format!("failed to write workspace ralph.yml: {e}"))
        })?;

        Ok(ScenarioConfig {
            config_file: "ralph.yml".into(),
            // 直接使用 example 配置里的 `event_loop.prompt`，避免 E2E runner 的提示词“改写示例语义”。
            prompt: PromptSource::Config,
            // 与 example 保持一致（当前为 12），避免“E2E 放宽迭代上限”掩盖失控行为。
            max_iterations: 12,
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

        let assertions = vec![
            Assertions::response_received(&execution),
            Assertions::exit_code_success_or_limit(&execution),
            Assertions::no_timeout(&execution),
            self.parallel_mode_visible(&execution),
            self.agents_snapshot_written(executor),
            self.hat_run_counts_expected(&execution),
            self.no_new_jobs_started_after_loop_complete(&execution),
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

#[cfg(test)]
mod tests {
    /// 这条回归专门锁住 example 提示词里的协议示例污染。
    /// 一旦有人把 raw `<event ...>` 写回 example，真实后端很容易把它照抄成伪事件。
    #[test]
    fn example_config_does_not_embed_raw_event_blocks() {
        let config = include_str!("../../../../examples/parallel-trigger-routing/ralph.yml");

        assert!(
            !config.contains("<event") && !config.contains("</event>"),
            "example config must not contain raw event tags; use escaped display text instead"
        );
    }

    #[test]
    fn example_config_does_not_embed_placeholder_payload_templates() {
        let config = include_str!("../../../../examples/parallel-trigger-routing/ralph.yml");

        assert!(
            !config.contains("...规格内容...")
                && !config.contains("&gt;...&lt;/event&gt;")
                && !config.contains("- spec.start: ...")
                && !config.contains("- spec.rejected: ..."),
            "example config must not teach placeholder payloads that the model can echo back"
        );
    }
}
