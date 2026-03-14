//! Tier 8: Parallel Runtime (experimental) - real-world example coverage.
//!
//! 目标:
//! - 直接覆盖 `examples/parallel-vendor-security-procurement`
//! - 验证 vendor 多输入线并行推进后由 decider 汇总 `vendor.ready`

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

/// 直接覆盖 `examples/parallel-vendor-security-procurement` 的端到端场景。
pub struct ParallelVendorSecurityProcurementExampleScenario {
    id: String,
    description: String,
    tier: String,
}

impl ParallelVendorSecurityProcurementExampleScenario {
    pub fn new() -> Self {
        Self {
            id: "parallel-vendor-security-procurement-example".to_string(),
            description: "Directly runs examples/parallel-vendor-security-procurement and asserts vendor lanes converge into vendor.ready".to_string(),
            tier: "Tier 8: Parallel Runtime".to_string(),
        }
    }

    fn parallel_mode_visible(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let visible = result.stdout.contains("[supervisor] instances");
        let builder =
            AssertionBuilder::new("Parallel mode visible (vendor security procurement example)")
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
                    "Agents snapshot written (vendor security procurement example)",
                )
                .expected(".ralph/agents.json exists and is valid JSON")
                .actual(error)
                .failed()
                .build();
            }
        };

        let instance_count = snapshot.instances.len();
        let has_security = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "security_assessor");
        let has_privacy = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "privacy_reviewer");
        let has_procurement = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "procurement_owner");
        let has_legal = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "legal_counsel");
        let has_decider = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "vendor_decider");

        let ok = instance_count >= 5
            && has_security
            && has_privacy
            && has_procurement
            && has_legal
            && has_decider;

        let builder = AssertionBuilder::new(
            "Agents snapshot written (vendor security procurement example)",
        )
        .expected("agents.json contains 4 lane hats + vendor decider")
        .actual(format!(
            "instance_count={instance_count}, security={has_security}, privacy={has_privacy}, procurement={has_procurement}, legal={has_legal}, decider={has_decider}"
        ));

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn required_topic_chain_observed(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let required = [
            "vendor.security.assess",
            "vendor.privacy.review",
            "vendor.procurement.check",
            "vendor.legal.review",
            "security.assessed",
            "privacy.ready",
            "procurement.ready",
            "legal.ready",
            "vendor.decision.request",
            "vendor.ready",
        ];

        let missing = required
            .iter()
            .filter(|topic| !result.events.iter().any(|event| event.topic == **topic))
            .copied()
            .collect::<Vec<_>>();

        let ok = missing.is_empty();
        let builder = AssertionBuilder::new(
            "Required topic chain observed (vendor security procurement example)",
        )
        .expected("all vendor lane topics + vendor.ready are present")
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
            AssertionBuilder::new("No unexpected gates (vendor security procurement example)")
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
            .find(|event| event.topic == "vendor.ready")
            .map(|event| event.payload.clone())
            .unwrap_or_default();

        let ok = payload.contains("decision: APPROVE_PILOT")
            && payload.contains("required_controls: sso_scim_audit_logs")
            && payload.contains("procurement_path: msa_plus_security_addendum");
        let builder = AssertionBuilder::new(
            "Final payload expected (vendor security procurement example)",
        )
        .expected(
            "vendor.ready payload contains APPROVE_PILOT, sso_scim_audit_logs, and msa_plus_security_addendum",
        )
        .actual(if payload.is_empty() {
            "vendor.ready payload missing".to_string()
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
        let builder = AssertionBuilder::new(
            "No new jobs after LOOP_COMPLETE (vendor security procurement example)",
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

impl Default for ParallelVendorSecurityProcurementExampleScenario {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn example_config_does_not_embed_raw_event_blocks() {
        let config =
            include_str!("../../../../examples/parallel-vendor-security-procurement/ralph.yml");

        assert!(
            !config.contains("<event") && !config.contains("</event>"),
            "example config must not contain raw event tags; use escaped display text instead"
        );
    }
}

#[async_trait]
impl TestScenario for ParallelVendorSecurityProcurementExampleScenario {
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
            "parallel-vendor-security-procurement",
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
