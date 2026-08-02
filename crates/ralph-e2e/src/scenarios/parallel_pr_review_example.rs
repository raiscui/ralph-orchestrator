//! Tier 8: Parallel Runtime (experimental) - real-world example coverage.
//!
//! 目标:
//! - 直接覆盖 `examples/parallel-pr-review`
//! - 验证真实“多 reviewer 并行 -> synthesizer 收敛”的链路

use super::parallel::{
    parse_parallel_job_line, read_agents_snapshot, setup_prompt_file_example_workspace,
};
use super::{AssertionBuilder, Assertions, ScenarioError, TestScenario};
use crate::Backend;
use crate::executor::{ExecutionResult, RalphExecutor, ScenarioConfig};
use crate::models::TestResult;
use async_trait::async_trait;
use std::collections::HashSet;
use std::path::Path;

/// 直接覆盖 `examples/parallel-pr-review` 的端到端场景。
///
/// 关注点:
/// - 3 条 review lane 是否真的 fanout 出去
/// - synthesizer 是否在 3 条 reviewer 结果到齐后才工作
/// - 最终 verdict 是否与示例 packet 保持一致
pub struct ParallelPrReviewExampleScenario {
    id: String,
    description: String,
    tier: String,
}

impl ParallelPrReviewExampleScenario {
    pub fn new() -> Self {
        Self {
            id: "parallel-pr-review-example".to_string(),
            description: "Directly runs examples/parallel-pr-review and asserts multi-reviewer fanout plus synthesis completion".to_string(),
            tier: "Tier 8: Parallel Runtime".to_string(),
        }
    }

    fn parallel_mode_visible(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let visible = result.stdout.contains("[supervisor] instances");
        let builder = AssertionBuilder::new("Parallel mode visible (pr review example)")
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

    fn agents_snapshot_written(&self, executor: &RalphExecutor) -> crate::models::Assertion {
        // -----------------------------------------------------------------
        // 说明:
        // - 这个 example 的核心就是多个 reviewer 同时存在。
        // - 因此快照里至少要看到 correctness/security/architecture/synthesizer。
        // -----------------------------------------------------------------
        let snapshot = match read_agents_snapshot(executor.workspace()) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                return AssertionBuilder::new("Agents snapshot written (pr review example)")
                    .expected(".ralph/agents.json exists and is valid JSON")
                    .actual(error)
                    .failed()
                    .build();
            }
        };

        let instance_count = snapshot.instances.len();
        let has_correctness = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "correctness_reviewer");
        let has_security = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "security_reviewer");
        let has_architecture = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "architecture_reviewer");
        let has_synthesizer = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "review_synthesizer");

        let ok = instance_count >= 4
            && has_correctness
            && has_security
            && has_architecture
            && has_synthesizer;

        let builder = AssertionBuilder::new("Agents snapshot written (pr review example)")
            .expected("agents.json contains 3 reviewers + synthesizer")
            .actual(format!(
                "instance_count={instance_count}, correctness={has_correctness}, security={has_security}, architecture={has_architecture}, synthesizer={has_synthesizer}"
            ));

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn required_topic_chain_observed(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let required = [
            "review.correctness",
            "review.security",
            "review.architecture",
            "correctness.done",
            "security.done",
            "architecture.done",
            "synthesis.request",
            "review.complete",
        ];

        let missing = required
            .iter()
            .filter(|topic| !result.events.iter().any(|event| event.topic == **topic))
            .copied()
            .collect::<Vec<_>>();

        let correctness_count = result
            .events
            .iter()
            .filter(|event| event.topic == "correctness.done")
            .count();
        let security_count = result
            .events
            .iter()
            .filter(|event| event.topic == "security.done")
            .count();
        let architecture_count = result
            .events
            .iter()
            .filter(|event| event.topic == "architecture.done")
            .count();

        let ok = missing.is_empty()
            && correctness_count >= 1
            && security_count >= 1
            && architecture_count >= 1;

        let builder = AssertionBuilder::new("Required topic chain observed (pr review example)")
            .expected("all review lane topics + synthesis.request + review.complete are present")
            .actual(format!(
                "missing={missing:?}, counts: correctness={correctness_count}, security={security_count}, architecture={architecture_count}"
            ));

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn final_verdict_expected(&self, result: &ExecutionResult) -> crate::models::Assertion {
        // -----------------------------------------------------------------
        // 说明:
        // - `PROMPT.md` 里的示例 packet 明确要求最终结论为 `REQUEST_CHANGES`。
        // - 这里直接锁这个契约,避免 example 退化成“只要结束就算通过”。
        // -----------------------------------------------------------------
        let payload = result
            .events
            .iter()
            .rev()
            .find(|event| event.topic == "review.complete")
            .map(|event| event.payload.clone())
            .unwrap_or_default();

        let ok = payload.contains("REQUEST_CHANGES");
        let builder = AssertionBuilder::new("Final verdict expected (pr review example)")
            .expected("review.complete payload contains REQUEST_CHANGES")
            .actual(if payload.is_empty() {
                "review.complete payload missing".to_string()
            } else {
                payload
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

            if !completion_seen
                && line.trim_end().ends_with(completion_promise)
                && line.trim_start().starts_with("[ralph#")
                && line.contains(":out:job=")
            {
                completion_seen = true;
            }
        }

        let mut new_jobs = new_jobs_after.into_iter().collect::<Vec<_>>();
        new_jobs.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));

        let ok = completion_seen && new_jobs.is_empty();
        let builder = AssertionBuilder::new("No new jobs after LOOP_COMPLETE (pr review example)")
            .expected("After LOOP_COMPLETE, no new job_id should appear in stdout")
            .actual(format!(
                "completion_seen={completion_seen}, new_jobs_after={new_jobs:?}"
            ));

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }
}

impl Default for ParallelPrReviewExampleScenario {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TestScenario for ParallelPrReviewExampleScenario {
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
        setup_prompt_file_example_workspace(workspace, backend, "parallel-pr-review", 18)
    }

    async fn run(
        &self,
        executor: &RalphExecutor,
        config: &ScenarioConfig,
    ) -> Result<TestResult, ScenarioError> {
        let start = std::time::Instant::now();
        let execution = executor.run(config).await.map_err(|error| {
            ScenarioError::ExecutionError(format!("ralph execution failed: {error}"))
        })?;
        let duration = start.elapsed();

        let assertions = vec![
            Assertions::response_received(&execution),
            Assertions::exit_code(&execution, 0),
            Assertions::no_timeout(&execution),
            self.parallel_mode_visible(&execution),
            self.agents_snapshot_written(executor),
            self.required_topic_chain_observed(&execution),
            self.final_verdict_expected(&execution),
            self.no_new_jobs_started_after_loop_complete(&execution),
        ];

        let all_passed = assertions.iter().all(|assertion| assertion.passed);

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

#[cfg(test)]
mod tests {
    #[test]
    fn example_config_does_not_embed_raw_event_blocks() {
        let config = include_str!("../../../../examples/parallel-pr-review/ralph.yml");

        assert!(
            !config.contains("<event") && !config.contains("</event>"),
            "example config must not contain raw event tags; use escaped display text instead"
        );
    }
}
