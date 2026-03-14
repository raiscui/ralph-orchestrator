//! Tier 8: Parallel Runtime (experimental) - real-world example coverage.
//!
//! 目标:
//! - 直接覆盖 `examples/parallel-release-checklist`
//! - 验证多检查项并行完成后,`ralph#1` 才会发布 `release.ready`

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

/// 直接覆盖 `examples/parallel-release-checklist` 的端到端场景。
pub struct ParallelReleaseChecklistExampleScenario {
    id: String,
    description: String,
    tier: String,
}

impl ParallelReleaseChecklistExampleScenario {
    pub fn new() -> Self {
        Self {
            id: "parallel-release-checklist-example".to_string(),
            description: "Directly runs examples/parallel-release-checklist and asserts three readiness lanes converge into release.ready".to_string(),
            tier: "Tier 8: Parallel Runtime".to_string(),
        }
    }

    fn parallel_mode_visible(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let visible = result.stdout.contains("[supervisor] instances");
        let builder = AssertionBuilder::new("Parallel mode visible (release checklist example)")
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
        let snapshot = match read_agents_snapshot(executor.workspace()) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                return AssertionBuilder::new(
                    "Agents snapshot written (release checklist example)",
                )
                .expected(".ralph/agents.json exists and is valid JSON")
                .actual(error)
                .failed()
                .build();
            }
        };

        let instance_count = snapshot.instances.len();
        let has_qa = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "qa_checker");
        let has_docs = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "docs_checker");
        let has_ops = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "ops_checker");

        let has_synthesizer = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "release_synthesizer");
        let ok = instance_count >= 4 && has_qa && has_docs && has_ops && has_synthesizer;
        let builder = AssertionBuilder::new("Agents snapshot written (release checklist example)")
            .expected("agents.json contains qa/docs/ops checkers plus release_synthesizer")
            .actual(format!(
                "instance_count={instance_count}, qa={has_qa}, docs={has_docs}, ops={has_ops}, synthesizer={has_synthesizer}"
            ));

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn required_topic_chain_observed(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let required = [
            "release.qa.check",
            "release.docs.check",
            "release.ops.check",
            "qa.ready",
            "docs.ready",
            "ops.ready",
            "release.summary.request",
            "release.ready",
        ];

        let missing = required
            .iter()
            .filter(|topic| !result.events.iter().any(|event| event.topic == **topic))
            .copied()
            .collect::<Vec<_>>();

        let ok = missing.is_empty();
        let builder =
            AssertionBuilder::new("Required topic chain observed (release checklist example)")
                .expected("all three check tasks and ready topics plus release.ready are present")
                .actual(format!("missing={missing:?}"));

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn no_unexpected_gates(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let found = result
            .events
            .iter()
            .filter(|event| event.topic.starts_with("gate."))
            .map(|event| event.topic.clone())
            .collect::<Vec<_>>();

        let ok = found.is_empty();
        let builder =
            AssertionBuilder::new("No unexpected gate topics (release checklist example)")
                .expected("no gate.* topics should appear in this example")
                .actual(format!("found={found:?}"));

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn release_ready_payload_present(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let payload = result
            .events
            .iter()
            .rev()
            .find(|event| event.topic == "release.ready")
            .map(|event| event.payload.clone())
            .unwrap_or_default();

        let ok = payload.contains("v2.4.0");
        let builder =
            AssertionBuilder::new("Release ready payload present (release checklist example)")
                .expected("release.ready payload contains version v2.4.0")
                .actual(if payload.is_empty() {
                    "release.ready payload missing".to_string()
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
        let builder =
            AssertionBuilder::new("No new jobs after LOOP_COMPLETE (release checklist example)")
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

impl Default for ParallelReleaseChecklistExampleScenario {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn example_config_does_not_embed_raw_event_blocks() {
        let config = include_str!("../../../../examples/parallel-release-checklist/ralph.yml");

        assert!(
            !config.contains("<event") && !config.contains("</event>"),
            "example config must not contain raw event tags; use escaped display text instead"
        );
    }
}

#[async_trait]
impl TestScenario for ParallelReleaseChecklistExampleScenario {
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
        setup_prompt_file_example_workspace(workspace, backend, "parallel-release-checklist", 16)
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
            self.no_unexpected_gates(&execution),
            self.release_ready_payload_present(&execution),
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
