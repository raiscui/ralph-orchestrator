//! Tier 8: Parallel Runtime - 续费风险预测校准示例。
//! 目标是验证 `examples/parallel-renewal-risk-calibration` 的并行 lane 收敛与 finalizer 产出。

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

/// 续费风险预测校准示例的 E2E 场景。
pub struct ParallelRenewalRiskCalibrationExampleScenario {
    id: String,
    description: String,
    tier: String,
}

impl ParallelRenewalRiskCalibrationExampleScenario {
    /// 构造默认场景。
    pub fn new() -> Self {
        Self {
            id: "parallel-renewal-risk-calibration-example".to_string(),
            description:
                "直接运行 examples/parallel-renewal-risk-calibration,并断言四条 forecast 校准 lane 收敛到 renewal.calibration.ready"
                    .to_string(),
            tier: "Tier 8: Parallel Runtime".to_string(),
        }
    }

    fn parallel_mode_visible(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let visible = result.stdout.contains("[supervisor] instances");
        let builder =
            AssertionBuilder::new("Parallel mode banner visible (renewal risk calibration)")
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
                return AssertionBuilder::new("Agents snapshot present (renewal risk calibration)")
                    .expected(".ralph/agents.json 存在且包含 hat 实例")
                    .actual(error)
                    .failed()
                    .build();
            }
        };

        let instance_count = snapshot.instances.len();
        let has_usage = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "usage_signal_reviewer");
        let has_sponsor = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "sponsor_coverage_reviewer");
        let has_blocker = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "commercial_blocker_reviewer");
        let has_success = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "success_plan_reviewer");
        let has_lead = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "renewal_calibration_lead");

        let ok = instance_count >= 5
            && has_usage
            && has_sponsor
            && has_blocker
            && has_success
            && has_lead;

        let builder = AssertionBuilder::new(
            "Agents snapshot content (renewal risk calibration)",
        )
        .expected("包含四个 lane hat 以及 finalizer")
        .actual(format!(
            "count={instance_count} usage={has_usage} sponsor={has_sponsor} blocker={has_blocker} success={has_success} lead={has_lead}"
        ));

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn required_topics_present(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let required = [
            "renewal.usage.signal.review",
            "renewal.sponsor.coverage.review",
            "renewal.commercial.blocker.review",
            "renewal.success.plan.review",
            "usage.ready",
            "sponsor.ready",
            "blocker.ready",
            "success.ready",
            "renewal.calibration.packet.request",
            "renewal.calibration.ready",
        ];

        let missing: Vec<_> = required
            .iter()
            .filter(|topic| !result.events.iter().any(|event| event.topic == **topic))
            .copied()
            .collect();

        let ok = missing.is_empty();
        let builder =
            AssertionBuilder::new("Required topic chain observed (renewal risk calibration)")
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
        let builder = AssertionBuilder::new("No unexpected gates (renewal risk calibration)")
            .expected("不应出现 gate.* 或 approval.requested topic")
            .actual(format!("gate_topics={gates:?}"));

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn final_payload_matches(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let payload = extract_last_parallel_out_payload_for_topic(
            &result.stdout,
            "renewal.calibration.ready",
        )
        .or_else(|| {
            result
                .events
                .iter()
                .rev()
                .find(|event| event.topic == "renewal.calibration.ready")
                .map(|event| event.payload.clone())
        })
        .unwrap_or_default();

        let ok = calibration_payload_matches(&payload);
        let builder = AssertionBuilder::new("Final payload expected (renewal risk calibration)")
            .expected("renewal.calibration.ready payload 包含固定字段")
            .actual(if payload.is_empty() {
                "renewal.calibration.ready payload 缺失".to_string()
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
        let builder =
            AssertionBuilder::new("No new jobs after LOOP_COMPLETE (renewal risk calibration)")
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

fn calibration_payload_matches(payload: &str) -> bool {
    if payload.is_empty() {
        return false;
    }

    if let Ok(value) = serde_json::from_str::<Value>(payload) {
        let status = value
            .get("calibration_status")
            .and_then(|value| value.as_str());
        let window = value
            .get("forecast_window")
            .and_then(|value| value.as_str());
        let owner = value.get("forecast_owner").and_then(|value| value.as_str());
        let summary = value.get("calibration_summary");

        return status == Some("READY_FOR_FORECAST_COMMIT")
            && window == Some("Q3_RENEWAL_CALIBRATION")
            && owner == Some("retention-ops")
            && summary.is_some();
    }

    payload.contains("calibration_status: READY_FOR_FORECAST_COMMIT")
        && payload.contains("forecast_window: Q3_RENEWAL_CALIBRATION")
        && payload.contains("forecast_owner: retention-ops")
        && payload.contains("calibration_summary:")
}

impl Default for ParallelRenewalRiskCalibrationExampleScenario {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TestScenario for ParallelRenewalRiskCalibrationExampleScenario {
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
            "parallel-renewal-risk-calibration",
            20,
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

        let passed = assertions.iter().all(|assertion| assertion.passed);

        Ok(TestResult {
            scenario_id: self.id.clone(),
            scenario_description: self.description.clone(),
            backend: String::new(),
            tier: self.tier.clone(),
            passed,
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
            include_str!("../../../../examples/parallel-renewal-risk-calibration/ralph.yml");
        assert!(
            !config.contains("<event") && !config.contains("</event>"),
            "example config must not embed raw event tags"
        );
    }

    #[test]
    fn config_requires_silent_waiting() {
        let config =
            include_str!("../../../../examples/parallel-renewal-risk-calibration/ralph.yml");
        assert!(
            config.contains("当 4 条 ready 还没有全部到齐时:")
                && config.contains("你必须保持完全静默")
                && config.contains("`renewal.calibration.packet.request`"),
            "config must describe the silent waiting requirement"
        );
    }

    #[test]
    fn config_forbids_self_closing_events() {
        let config =
            include_str!("../../../../examples/parallel-renewal-risk-calibration/ralph.yml");
        assert!(
            config.contains("禁止自闭合 `&lt;event .../&gt;`")
                && config.contains("不要把字段塞进 opening tag 属性"),
            "config must forbid self-closing events and attribute-only payloads"
        );
    }

    #[test]
    fn config_requires_compact_success_event_shape() {
        let config =
            include_str!("../../../../examples/parallel-renewal-risk-calibration/ralph.yml");
        assert!(
            config.contains("整个回复必须是一条单行真实事件")
                && config.contains("payload 请使用紧凑 JSON 对象")
                && config.contains("结束标签必须精确写成 `&lt;/event&gt;`")
                && config.contains("唯一允许的输出模板如下")
                && config.contains("risk_playbooks_assigned"),
            "success plan reviewer 必须被锁定为更机械化的单行 JSON 事件模板"
        );
    }

    #[test]
    fn payload_matcher_accepts_json_and_line_payloads() {
        let json_payload = r#"{"calibration_status":"READY_FOR_FORECAST_COMMIT","forecast_window":"Q3_RENEWAL_CALIBRATION","forecast_owner":"retention-ops","calibration_summary":"four lanes aligned"}"#;
        let line_payload = "calibration_status: READY_FOR_FORECAST_COMMIT
forecast_window: Q3_RENEWAL_CALIBRATION
forecast_owner: retention-ops
calibration_summary: four lanes aligned";

        assert!(super::calibration_payload_matches(json_payload));
        assert!(super::calibration_payload_matches(line_payload));
    }
}
