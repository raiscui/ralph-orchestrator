//! Tier 8: Parallel Runtime (experimental) - real-world example coverage.
//!
//! 目标:
//! - 直接覆盖 `examples/parallel-security-exception-review`
//! - 验证 security exception review 多输入线并行推进后由 finalizer 汇总 `exception.ready`

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

/// 直接覆盖 `examples/parallel-security-exception-review` 的端到端场景。
pub struct ParallelSecurityExceptionReviewExampleScenario {
    id: String,
    description: String,
    tier: String,
}

impl ParallelSecurityExceptionReviewExampleScenario {
    pub fn new() -> Self {
        Self {
            id: "parallel-security-exception-review-example".to_string(),
            description: "Directly runs examples/parallel-security-exception-review and asserts exception lanes converge into exception.ready".to_string(),
            tier: "Tier 8: Parallel Runtime".to_string(),
        }
    }

    fn parallel_mode_visible(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let visible = result.stdout.contains("[supervisor] instances");
        let builder =
            AssertionBuilder::new("Parallel mode visible (security exception review example)")
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
                    "Agents snapshot written (security exception review example)",
                )
                .expected(".ralph/agents.json exists and is valid JSON")
                .actual(error)
                .failed()
                .build();
            }
        };

        let instance_count = snapshot.instances.len();
        let has_threat = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "threat_model_reviewer");

        let has_controls = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "compensating_controls_reviewer");

        let has_data_scope = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "data_scope_reviewer");

        let has_expiry = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "expiry_policy_reviewer");

        let has_decider = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "exception_decider");

        let ok = instance_count >= 5
            && has_threat
            && has_controls
            && has_data_scope
            && has_expiry
            && has_decider;

        let builder = AssertionBuilder::new("Agents snapshot written (security exception review example)")
            .expected("agents.json contains 4 lane hats + exception decider")
            .actual(format!("instance_count={instance_count}, threat={has_threat}, controls={has_controls}, data_scope={has_data_scope}, expiry={has_expiry}, decider={has_decider}"));

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn required_topic_chain_observed(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let required = [
            "exception.threat.review",
            "exception.controls.review",
            "exception.data.scope.review",
            "exception.expiry.review",
            "threat.reviewed",
            "controls.reviewed",
            "data.scope.ready",
            "expiry.ready",
            "exception.decision.request",
            "exception.ready",
        ];

        let missing = required
            .iter()
            .filter(|topic| !result.events.iter().any(|event| event.topic == **topic))
            .copied()
            .collect::<Vec<_>>();

        let ok = missing.is_empty();
        let builder = AssertionBuilder::new(
            "Required topic chain observed (security exception review example)",
        )
        .expected("all exception lane topics + exception.ready are present")
        .actual(format!("missing={missing:?}"));

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn no_unexpected_gates(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let gates = result
            .events
            .iter()
            .filter(|event| event.topic.starts_with("gate.") || event.topic == "approval.requested")
            .map(|event| event.topic.clone())
            .collect::<Vec<_>>();
        let ok = gates.is_empty();
        let builder =
            AssertionBuilder::new("No unexpected gates (security exception review example)")
                .expected("no gate.* or approval.requested topics")
                .actual(format!("gate_topics={gates:?}"));

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn final_payload_expected(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let payload = result
            .events
            .iter()
            .rev()
            .find(|event| event.topic == "exception.ready")
            .map(|event| event.payload.clone())
            .unwrap_or_default();

        let ok = exception_payload_matches(&payload);
        let builder = AssertionBuilder::new("Final payload expected (security exception review example)")
            .expected("exception.ready payload semantically contains APPROVE_WITH_COMPENSATING_CONTROLS, waf_rate_limit_plus_audit, and 2026-06-30")
            .actual(if payload.is_empty() {
                "exception.ready payload missing".to_string()
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
                && line.trim_start().starts_with("[ralph#")
                && line.contains(":out:job=")
                && let Some((_prefix, payload)) = line.split_once("] ")
                && payload.trim() == completion_promise
            {
                completion_seen = true;
            }
        }

        let mut new_jobs = new_jobs_after.into_iter().collect::<Vec<_>>();
        new_jobs.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));

        let ok = completion_seen && new_jobs.is_empty();
        let builder = AssertionBuilder::new(
            "No new jobs after LOOP_COMPLETE (security exception review example)",
        )
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

fn exception_payload_matches(payload: &str) -> bool {
    if payload.is_empty() {
        return false;
    }

    if let Ok(value) = serde_json::from_str::<serde_json::Value>(payload) {
        let decision = value.get("decision").and_then(|value| value.as_str());
        let required_controls = value
            .get("required_controls")
            .and_then(|value| value.as_str());
        let expiry_date = value.get("expiry_date").and_then(|value| value.as_str());

        return decision == Some("APPROVE_WITH_COMPENSATING_CONTROLS")
            && required_controls == Some("waf_rate_limit_plus_audit")
            && expiry_date == Some("2026-06-30");
    }

    payload.contains("decision: APPROVE_WITH_COMPENSATING_CONTROLS")
        && payload.contains("required_controls: waf_rate_limit_plus_audit")
        && payload.contains("expiry_date: 2026-06-30")
}

impl Default for ParallelSecurityExceptionReviewExampleScenario {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TestScenario for ParallelSecurityExceptionReviewExampleScenario {
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
        setup_prompt_file_example_workspace(
            workspace,
            backend,
            "parallel-security-exception-review",
            18,
        )
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
            self.final_payload_expected(&execution),
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
        let config =
            include_str!("../../../../examples/parallel-security-exception-review/ralph.yml");

        assert!(
            !config.contains("<event") && !config.contains("</event>"),
            "example config must not contain raw event tags; use escaped display text instead"
        );
    }

    #[test]
    fn example_config_requires_silent_wait_before_all_ready_lanes() {
        let config =
            include_str!("../../../../examples/parallel-security-exception-review/ralph.yml");

        assert!(
            config.contains("当 4 条 ready 还没有全部到齐时:")
                && config.contains("你必须保持静默,空输出是合法且首选的")
                && config.contains("`LOOP_COMPLETE` 这个字符串只能在最终收尾那一行出现一次"),
            "parallel-security-exception-review config must explicitly forbid interim prose before all ready lanes arrive"
        );
    }

    #[test]
    fn payload_matcher_accepts_json_and_line_payloads() {
        let json_payload = r#"{"exception_id":"EXC-2026-17","decision":"APPROVE_WITH_COMPENSATING_CONTROLS","required_controls":"waf_rate_limit_plus_audit","expiry_date":"2026-06-30"}"#;
        let line_payload = "exception_id: EXC-2026-17
decision: APPROVE_WITH_COMPENSATING_CONTROLS
required_controls: waf_rate_limit_plus_audit
expiry_date: 2026-06-30";

        assert!(super::exception_payload_matches(json_payload));
        assert!(super::exception_payload_matches(line_payload));
    }
}
