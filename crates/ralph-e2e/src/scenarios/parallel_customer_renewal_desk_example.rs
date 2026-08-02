//! Tier 8: Parallel Runtime (experimental) - real-world example coverage.
//!
//! 目标:
//! - 直接覆盖 `examples/parallel-customer-renewal-desk`
//! - 验证 customer renewal desk 多输入线并行推进后由 finalizer 汇总 `renewal.plan.ready`

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

/// 直接覆盖 `examples/parallel-customer-renewal-desk` 的端到端场景。
pub struct ParallelCustomerRenewalDeskExampleScenario {
    id: String,
    description: String,
    tier: String,
}

impl ParallelCustomerRenewalDeskExampleScenario {
    pub fn new() -> Self {
        Self {
            id: "parallel-customer-renewal-desk-example".to_string(),
            description: "Directly runs examples/parallel-customer-renewal-desk and asserts renewal lanes converge into renewal.plan.ready".to_string(),
            tier: "Tier 8: Parallel Runtime".to_string(),
        }
    }

    fn parallel_mode_visible(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let visible = result.stdout.contains("[supervisor] instances");
        let builder =
            AssertionBuilder::new("Parallel mode visible (customer renewal desk example)")
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
                    "Agents snapshot written (customer renewal desk example)",
                )
                .expected(".ralph/agents.json exists and is valid JSON")
                .actual(error)
                .failed()
                .build();
            }
        };

        let instance_count = snapshot.instances.len();
        let has_adoption = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "adoption_reviewer");

        let has_support = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "support_health_reviewer");

        let has_commercial = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "commercial_owner");

        let has_sponsor = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "sponsor_mapper");

        let has_strategist = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "renewal_strategist");

        let ok = instance_count >= 5
            && has_adoption
            && has_support
            && has_commercial
            && has_sponsor
            && has_strategist;

        let builder = AssertionBuilder::new("Agents snapshot written (customer renewal desk example)")
            .expected("agents.json contains 4 lane hats + renewal strategist")
            .actual(format!("instance_count={instance_count}, adoption={has_adoption}, support={has_support}, commercial={has_commercial}, sponsor={has_sponsor}, strategist={has_strategist}"));

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn required_topic_chain_observed(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let required = [
            "renewal.adoption.review",
            "renewal.support.health",
            "renewal.commercial.review",
            "renewal.sponsor.map",
            "adoption.ready",
            "support.ready",
            "commercial.ready",
            "sponsor.ready",
            "renewal.plan.request",
            "renewal.plan.ready",
        ];

        let missing = required
            .iter()
            .filter(|topic| !result.events.iter().any(|event| event.topic == **topic))
            .copied()
            .collect::<Vec<_>>();

        let ok = missing.is_empty();
        let builder =
            AssertionBuilder::new("Required topic chain observed (customer renewal desk example)")
                .expected("all renewal lane topics + renewal.plan.ready are present")
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
        let builder = AssertionBuilder::new("No unexpected gates (customer renewal desk example)")
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
            .find(|event| event.topic == "renewal.plan.ready")
            .map(|event| event.payload.clone())
            .unwrap_or_default();

        let ok = renewal_payload_matches(&payload);
        let builder = AssertionBuilder::new("Final payload expected (customer renewal desk example)")
            .expected("renewal.plan.ready payload semantically contains SAVE_AND_RENEW, MEDIUM, and schedule_qbr")
            .actual(if payload.is_empty() {
                "renewal.plan.ready payload missing".to_string()
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
            "No new jobs after LOOP_COMPLETE (customer renewal desk example)",
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

fn renewal_payload_matches(payload: &str) -> bool {
    if payload.is_empty() {
        return false;
    }

    if let Ok(value) = serde_json::from_str::<serde_json::Value>(payload) {
        let renewal_decision = value
            .get("renewal_decision")
            .and_then(|value| value.as_str());
        let risk_level = value.get("risk_level").and_then(|value| value.as_str());
        let next_exec_action = value
            .get("next_exec_action")
            .and_then(|value| value.as_str());

        return renewal_decision == Some("SAVE_AND_RENEW")
            && risk_level == Some("MEDIUM")
            && next_exec_action == Some("schedule_qbr");
    }

    payload.contains("renewal_decision: SAVE_AND_RENEW")
        && payload.contains("risk_level: MEDIUM")
        && payload.contains("next_exec_action: schedule_qbr")
}

impl Default for ParallelCustomerRenewalDeskExampleScenario {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TestScenario for ParallelCustomerRenewalDeskExampleScenario {
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
            "parallel-customer-renewal-desk",
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
        let config = include_str!("../../../../examples/parallel-customer-renewal-desk/ralph.yml");

        assert!(
            !config.contains("<event") && !config.contains("</event>"),
            "example config must not contain raw event tags; use escaped display text instead"
        );
    }

    #[test]
    fn example_config_requires_silent_wait_before_all_ready_lanes() {
        let config = include_str!("../../../../examples/parallel-customer-renewal-desk/ralph.yml");

        assert!(
            config.contains("当 4 条 ready 还没有全部到齐时:")
                && config.contains("你必须保持静默,空输出是合法且首选的")
                && config.contains("`LOOP_COMPLETE` 这个字符串只能在最终收尾那一行出现一次"),
            "parallel-customer-renewal-desk config must explicitly forbid interim prose before all ready lanes arrive"
        );
    }

    #[test]
    fn example_config_forbids_self_closing_events() {
        let config = include_str!("../../../../examples/parallel-customer-renewal-desk/ralph.yml");

        assert!(
            config.contains("不要使用自闭合 `&lt;event .../&gt;` 形式。")
                && config.contains("不要把业务字段塞进 opening tag 属性。"),
            "renewal example must forbid self-closing events and attribute-only payloads"
        );
    }

    #[test]
    fn payload_matcher_accepts_json_and_line_payloads() {
        let json_payload = r#"{"account_id":"CUST-2048","renewal_decision":"SAVE_AND_RENEW","risk_level":"MEDIUM","next_exec_action":"schedule_qbr"}"#;
        let line_payload = "account_id: CUST-2048
renewal_decision: SAVE_AND_RENEW
risk_level: MEDIUM
next_exec_action: schedule_qbr";

        assert!(super::renewal_payload_matches(json_payload));
        assert!(super::renewal_payload_matches(line_payload));
    }
}
