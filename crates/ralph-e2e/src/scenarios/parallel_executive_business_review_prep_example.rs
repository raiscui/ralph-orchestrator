//! Tier 8: Parallel Runtime - 高层业务回顾材料准备示例。
//! 目标是验证 `examples/parallel-executive-business-review-prep` 的并行 lane 收敛与 finalizer 产出。

use super::parallel::{
    extract_last_parallel_out_payload_for_topic, parse_parallel_job_line, read_agents_snapshot,
    setup_prompt_file_example_workspace,
};
use super::{AssertionBuilder, Assertions, ScenarioError, TestScenario};
use crate::Backend;
use crate::executor::{ExecutionResult, RalphExecutor, ScenarioConfig};
use crate::models::TestResult;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashSet;
use std::path::Path;

/// 高层业务回顾材料准备示例的 E2E 场景。
pub struct ParallelExecutiveBusinessReviewPrepExampleScenario {
    id: String,
    description: String,
    tier: String,
}

impl ParallelExecutiveBusinessReviewPrepExampleScenario {
    /// 构造默认场景。
    pub fn new() -> Self {
        Self {
            id: "parallel-executive-business-review-prep-example".to_string(),
            description:
                "直接运行 examples/parallel-executive-business-review-prep,并断言四条 EBR 材料 lane 收敛到 ebr.packet.ready"
                    .to_string(),
            tier: "Tier 8: Parallel Runtime".to_string(),
        }
    }

    fn parallel_mode_visible(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let visible = result.stdout.contains("[supervisor] instances");
        let builder = AssertionBuilder::new("Parallel mode banner visible (EBR prep)")
            .expected("stdout 包含 supervisor instances banner")
            .actual(if visible {
                "发现 supervisor banner".to_string()
            } else {
                "缺少 supervisor banner".to_string()
            });

        if visible {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn agents_snapshot_contains_hats(&self, executor: &RalphExecutor) -> crate::models::Assertion {
        let snapshot = match read_agents_snapshot(executor.workspace()) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                return AssertionBuilder::new("Agents snapshot present (EBR prep)")
                    .expected(".ralph/agents.json 存在且包含 hat 实例")
                    .actual(error)
                    .failed()
                    .build();
            }
        };

        let instance_count = snapshot.instances.len();
        let has_revenue = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "revenue_narrative_owner");
        let has_adoption = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "product_adoption_owner");
        let has_risk = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "risk_outlook_owner");
        let has_asks = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "executive_asks_owner");
        let has_chief = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "ebr_chief_of_staff");

        let ok =
            instance_count >= 5 && has_revenue && has_adoption && has_risk && has_asks && has_chief;

        let builder = AssertionBuilder::new("Agents snapshot content (EBR prep)")
            .expected("包含四个 lane hat 以及 finalizer")
            .actual(format!(
                "count={instance_count} revenue={has_revenue} adoption={has_adoption} risk={has_risk} asks={has_asks} chief={has_chief}"
            ));

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn required_topics_present(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let required = [
            "ebr.revenue.narrative.review",
            "ebr.product.adoption.review",
            "ebr.risk.outlook.review",
            "ebr.exec.asks.review",
            "revenue.ready",
            "adoption.ready",
            "risk.ready",
            "asks.ready",
            "ebr.packet.request",
            "ebr.packet.ready",
        ];

        let missing: Vec<_> = required
            .iter()
            .filter(|topic| !result.events.iter().any(|event| event.topic == **topic))
            .copied()
            .collect();

        let ok = missing.is_empty();
        let builder = AssertionBuilder::new("Required topic chain observed (EBR prep)")
            .expected("所有 lane topic 以及 final topic 都存在")
            .actual(format!("missing={missing:?}"));

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn final_payload_matches(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let payload =
            extract_last_parallel_out_payload_for_topic(&result.stdout, "ebr.packet.ready")
                .or_else(|| {
                    result
                        .events
                        .iter()
                        .rev()
                        .find(|event| event.topic == "ebr.packet.ready")
                        .map(|event| event.payload.clone())
                })
                .unwrap_or_default();

        let ok = ebr_payload_matches(&payload);
        let builder = AssertionBuilder::new("Final payload expected (EBR prep)")
            .expected("ebr.packet.ready payload 包含固定字段")
            .actual(if payload.is_empty() {
                "ebr.packet.ready payload 缺失".to_string()
            } else {
                payload
            });

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn no_jobs_after_loop_complete(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let completion = "LOOP_COMPLETE";
        let mut completion_seen = false;
        let mut jobs_before = HashSet::new();
        let mut jobs_after = HashSet::new();

        for line in result.stdout.lines() {
            if let Some((instance_id, job_id)) = parse_parallel_job_line(line) {
                let key = (instance_id, job_id);
                if completion_seen {
                    if !jobs_before.contains(&key) {
                        jobs_after.insert(key);
                    }
                } else {
                    jobs_before.insert(key);
                }
            }

            if !completion_seen
                && line.trim_start().starts_with("[ralph#")
                && line.contains(":out:job=")
                && let Some((_prefix, payload)) = line.split_once("] ")
                && payload.trim() == completion
            {
                completion_seen = true;
            }
        }

        let mut new_jobs = jobs_after.into_iter().collect::<Vec<_>>();
        new_jobs.sort();

        let ok = completion_seen && new_jobs.is_empty();
        let builder = AssertionBuilder::new("No new jobs after LOOP_COMPLETE (EBR prep)")
            .expected("LOOP_COMPLETE 之后没有新 job")
            .actual(format!(
                "completion_seen={completion_seen}, new_jobs={new_jobs:?}"
            ));

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }
}

fn ebr_payload_matches(payload: &str) -> bool {
    if payload.is_empty() {
        return false;
    }

    if let Ok(value) = serde_json::from_str::<Value>(payload) {
        let status = value.get("ebr_status").and_then(|value| value.as_str());
        let tier = value.get("meeting_tier").and_then(|value| value.as_str());
        let owner = value
            .get("narrative_owner")
            .and_then(|value| value.as_str());
        let summary = value.get("packet_summary");
        let next_owner = value
            .get("next_action_owner")
            .and_then(|value| value.as_str());

        return status == Some("READY_FOR_EXEC_REVIEW")
            && tier == Some("Q2_BUSINESS_REVIEW")
            && owner == Some("gm-office")
            && summary.is_some()
            && next_owner == Some("gm-office");
    }

    payload.contains("ebr_status: READY_FOR_EXEC_REVIEW")
        && payload.contains("meeting_tier: Q2_BUSINESS_REVIEW")
        && payload.contains("narrative_owner: gm-office")
        && payload.contains("packet_summary:")
        && payload.contains("next_action_owner: gm-office")
}

impl Default for ParallelExecutiveBusinessReviewPrepExampleScenario {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TestScenario for ParallelExecutiveBusinessReviewPrepExampleScenario {
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
            "parallel-executive-business-review-prep",
            18,
        )
    }

    async fn run(
        &self,
        executor: &RalphExecutor,
        config: &ScenarioConfig,
    ) -> Result<TestResult, ScenarioError> {
        let execution = executor.run(config).await.map_err(|error| {
            ScenarioError::ExecutionError(format!("ralph execution failed: {error}"))
        })?;

        let assertions = vec![
            Assertions::response_received(&execution),
            Assertions::exit_code(&execution, 0),
            Assertions::no_timeout(&execution),
            self.parallel_mode_visible(&execution),
            self.agents_snapshot_contains_hats(executor),
            self.required_topics_present(&execution),
            self.final_payload_matches(&execution),
            self.no_jobs_after_loop_complete(&execution),
        ];

        let all_passed = assertions.iter().all(|assertion| assertion.passed);

        Ok(TestResult {
            scenario_id: self.id.clone(),
            scenario_description: self.description.clone(),
            backend: String::new(),
            tier: self.tier.clone(),
            passed: all_passed,
            assertions,
            duration: execution.duration,
        })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn config_missing_event_blocks() {
        let config =
            include_str!("../../../../examples/parallel-executive-business-review-prep/ralph.yml");
        assert!(
            !config.contains("<event") && !config.contains("</event>"),
            "example config must not embed raw event tags"
        );
    }

    #[test]
    fn config_requires_silent_waiting() {
        let config =
            include_str!("../../../../examples/parallel-executive-business-review-prep/ralph.yml");
        assert!(
            config.contains("四条 ready 到齐之前") && config.contains("必须保持静默"),
            "config must describe the silent waiting requirement"
        );
    }

    #[test]
    fn config_forbids_self_closing_events() {
        let config =
            include_str!("../../../../examples/parallel-executive-business-review-prep/ralph.yml");
        assert!(
            config.contains("不要使用自闭合 `&lt;event .../&gt;`")
                && config.contains("不要把业务字段塞进 opening tag"),
            "config must forbid self-closing events and attribute-only payloads"
        );
    }

    #[test]
    fn config_requires_fixed_final_fields() {
        let config =
            include_str!("../../../../examples/parallel-executive-business-review-prep/ralph.yml");
        assert!(
            config.contains("`ebr_status: READY_FOR_EXEC_REVIEW`")
                && config.contains("`meeting_tier: Q2_BUSINESS_REVIEW`")
                && config.contains("`narrative_owner: gm-office`")
                && config.contains("`next_action_owner: gm-office`"),
            "config must lock the fixed EBR final fields"
        );
    }

    #[test]
    fn payload_matcher_accepts_json_and_line_payloads() {
        let json_payload = r#"{"ebr_status":"READY_FOR_EXEC_REVIEW","meeting_tier":"Q2_BUSINESS_REVIEW","narrative_owner":"gm-office","packet_summary":"four lanes aligned","next_action_owner":"gm-office"}"#;
        let line_payload = "ebr_status: READY_FOR_EXEC_REVIEW
meeting_tier: Q2_BUSINESS_REVIEW
narrative_owner: gm-office
packet_summary: four lanes aligned
next_action_owner: gm-office";

        assert!(super::ebr_payload_matches(json_payload));
        assert!(super::ebr_payload_matches(line_payload));
    }
}
