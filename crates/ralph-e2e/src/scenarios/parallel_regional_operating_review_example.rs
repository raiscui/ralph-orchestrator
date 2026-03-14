//! Tier 8: Parallel Runtime - 区域经营周会示例。
//! 目标是验证 `examples/parallel-regional-operating-review` 的并行 lane 收敛与 finalizer 产出。

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

/// 区域经营周会示例的 E2E 场景。
pub struct ParallelRegionalOperatingReviewExampleScenario {
    id: String,
    description: String,
    tier: String,
}

impl ParallelRegionalOperatingReviewExampleScenario {
    /// 构造默认场景。
    pub fn new() -> Self {
        Self {
            id: "parallel-regional-operating-review-example".to_string(),
            description:
                "直接运行 examples/parallel-regional-operating-review,并断言四条区域经营 lane 收敛到 regional.review.ready"
                    .to_string(),
            tier: "Tier 8: Parallel Runtime".to_string(),
        }
    }

    fn parallel_mode_visible(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let visible = result.stdout.contains("[supervisor] instances");
        let builder = AssertionBuilder::new("Parallel mode banner visible (regional review)")
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
                return AssertionBuilder::new("Agents snapshot present (regional review)")
                    .expected(".ralph/agents.json 存在且包含 hat 实例")
                    .actual(error)
                    .failed()
                    .build();
            }
        };

        let instance_count = snapshot.instances.len();
        let has_pipeline = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "pipeline_health_reviewer");
        let has_delivery = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "delivery_capacity_reviewer");
        let has_support = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "support_signal_reviewer");
        let has_talent = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "talent_plan_reviewer");
        let has_lead = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "regional_operating_lead");

        let ok = instance_count >= 5
            && has_pipeline
            && has_delivery
            && has_support
            && has_talent
            && has_lead;

        let builder = AssertionBuilder::new("Agents snapshot content (regional review)")
            .expected("包含四个 lane hat 以及 finalizer")
            .actual(format!(
                "count={instance_count} pipeline={has_pipeline} delivery={has_delivery} support={has_support} talent={has_talent} lead={has_lead}"
            ));

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn required_topics_present(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let required = [
            "regional.pipeline.health.review",
            "regional.delivery.capacity.review",
            "regional.support.signal.review",
            "regional.talent.plan.review",
            "pipeline.ready",
            "delivery.ready",
            "support.ready",
            "talent.ready",
            "regional.operating.packet.request",
            "regional.review.ready",
        ];

        let missing: Vec<_> = required
            .iter()
            .filter(|topic| !result.events.iter().any(|event| event.topic == **topic))
            .copied()
            .collect();

        let ok = missing.is_empty();
        let builder = AssertionBuilder::new("Required topic chain observed (regional review)")
            .expected("所有 lane topic 以及 final topic 都存在")
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
        let builder = AssertionBuilder::new("No unexpected gates (regional review)")
            .expected("没有 gate.* 或 approval.requested topic")
            .actual(format!("gate_topics={gates:?}"));

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn final_payload_matches(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let payload =
            extract_last_parallel_out_payload_for_topic(&result.stdout, "regional.review.ready")
                .or_else(|| {
                    result
                        .events
                        .iter()
                        .rev()
                        .find(|event| event.topic == "regional.review.ready")
                        .map(|event| event.payload.clone())
                })
                .unwrap_or_default();

        let ok = regional_payload_matches(&payload);
        let builder = AssertionBuilder::new("Final payload expected (regional review)")
            .expected("regional.review.ready payload 包含固定字段")
            .actual(if payload.is_empty() {
                "regional.review.ready payload 缺失".to_string()
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
        let builder = AssertionBuilder::new("No new jobs after LOOP_COMPLETE (regional review)")
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

fn regional_payload_matches(payload: &str) -> bool {
    if payload.is_empty() {
        return false;
    }

    if let Ok(value) = serde_json::from_str::<Value>(payload) {
        let status = value.get("review_status").and_then(|value| value.as_str());
        let region = value.get("region_code").and_then(|value| value.as_str());
        let owner = value
            .get("operating_owner")
            .and_then(|value| value.as_str());
        let summary = value.get("packet_summary");
        let next_owner = value
            .get("next_action_owner")
            .and_then(|value| value.as_str());

        return status == Some("READY_FOR_REGION_WEEKLY")
            && region == Some("APAC_ENTERPRISE")
            && owner == Some("regional-chief-of-staff")
            && summary.is_some()
            && next_owner == Some("regional-chief-of-staff");
    }

    payload.contains("review_status: READY_FOR_REGION_WEEKLY")
        && payload.contains("region_code: APAC_ENTERPRISE")
        && payload.contains("operating_owner: regional-chief-of-staff")
        && payload.contains("packet_summary:")
        && payload.contains("next_action_owner: regional-chief-of-staff")
}

impl Default for ParallelRegionalOperatingReviewExampleScenario {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn config_missing_event_blocks() {
        let config =
            include_str!("../../../../examples/parallel-regional-operating-review/ralph.yml");
        assert!(
            !config.contains("<event") && !config.contains("</event>"),
            "example config must not embed raw event tags"
        );
    }

    #[test]
    fn config_requires_silent_waiting() {
        let config =
            include_str!("../../../../examples/parallel-regional-operating-review/ralph.yml");
        assert!(
            config.contains("在四条 ready 全部到齐之前必须保持完全静默")
                && config.contains("你必须保持完全静默"),
            "config must describe the silent waiting requirement"
        );
    }

    #[test]
    fn config_forbids_self_closing_events() {
        let config =
            include_str!("../../../../examples/parallel-regional-operating-review/ralph.yml");
        assert!(
            config.contains("禁止自闭合 `&lt;event .../&gt;`")
                && config.contains("把字段塞进 opening tag 属性"),
            "config must forbid self-closing events and attribute-only payloads"
        );
    }

    #[test]
    fn config_requires_compact_delivery_event_shape() {
        let config =
            include_str!("../../../../examples/parallel-regional-operating-review/ralph.yml");
        assert!(
            config.contains("整个回复必须是一条单行真实事件")
                && config.contains("payload 请使用紧凑 JSON 对象")
                && config.contains("结束标签必须精确写成 `&lt;/event&gt;`"),
            "delivery reviewer 必须被锁定为单行 JSON 事件"
        );
    }

    #[test]
    fn payload_matcher_accepts_json_and_line_payloads() {
        let json_payload = r#"{"review_status":"READY_FOR_REGION_WEEKLY","region_code":"APAC_ENTERPRISE","operating_owner":"regional-chief-of-staff","packet_summary":"four lanes aligned","next_action_owner":"regional-chief-of-staff"}"#;
        let line_payload = "review_status: READY_FOR_REGION_WEEKLY
region_code: APAC_ENTERPRISE
operating_owner: regional-chief-of-staff
packet_summary: four lanes aligned
next_action_owner: regional-chief-of-staff";

        assert!(super::regional_payload_matches(json_payload));
        assert!(super::regional_payload_matches(line_payload));
    }
}

#[async_trait]
impl TestScenario for ParallelRegionalOperatingReviewExampleScenario {
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
            "parallel-regional-operating-review",
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
            self.no_unexpected_gates(&execution),
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
