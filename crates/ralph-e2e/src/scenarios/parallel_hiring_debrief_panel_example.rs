//! Tier 8: Parallel Runtime (experimental) - real-world example coverage.
//!
//! 目标:
//! - 直接覆盖 `examples/parallel-hiring-debrief-panel`
//! - 验证 hiring debrief 多输入线并行推进后由 finalizer 汇总 `hiring.packet.ready`

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

/// 直接覆盖 `examples/parallel-hiring-debrief-panel` 的端到端场景。
pub struct ParallelHiringDebriefPanelExampleScenario {
    id: String,
    description: String,
    tier: String,
}

impl ParallelHiringDebriefPanelExampleScenario {
    pub fn new() -> Self {
        Self {
            id: "parallel-hiring-debrief-panel-example".to_string(),
            description: "Directly runs examples/parallel-hiring-debrief-panel and asserts hiring lanes converge into hiring.packet.ready".to_string(),
            tier: "Tier 8: Parallel Runtime".to_string(),
        }
    }

    fn parallel_mode_visible(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let visible = result.stdout.contains("[supervisor] instances");
        let builder = AssertionBuilder::new("Parallel mode visible (hiring debrief panel example)")
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
                    "Agents snapshot written (hiring debrief panel example)",
                )
                .expected(".ralph/agents.json exists and is valid JSON")
                .actual(error)
                .failed()
                .build();
            }
        };

        let instance_count = snapshot.instances.len();
        let has_coding = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "coding_interviewer");
        let has_system = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "system_design_interviewer");
        let has_collaboration = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "collaboration_interviewer");
        let has_reference = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "reference_reviewer");
        let has_facilitator = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "hiring_facilitator");

        let ok = instance_count >= 5
            && has_coding
            && has_system
            && has_collaboration
            && has_reference
            && has_facilitator;

        let builder =
            AssertionBuilder::new("Agents snapshot written (hiring debrief panel example)")
                .expected("agents.json contains 4 lane hats + hiring facilitator")
                .actual(format!(
                    "instance_count={instance_count}, coding={has_coding}, system={has_system}, collaboration={has_collaboration}, reference={has_reference}, facilitator={has_facilitator}"
                ));

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn required_topic_chain_observed(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let required = [
            "hiring.coding.debrief",
            "hiring.system.debrief",
            "hiring.collaboration.debrief",
            "hiring.reference.debrief",
            "coding.ready",
            "system.ready",
            "collaboration.ready",
            "reference.ready",
            "hiring.packet.request",
            "hiring.packet.ready",
        ];

        let missing = required
            .iter()
            .filter(|topic| !result.events.iter().any(|event| event.topic == **topic))
            .copied()
            .collect::<Vec<_>>();

        let ok = missing.is_empty();
        let builder =
            AssertionBuilder::new("Required topic chain observed (hiring debrief panel example)")
                .expected("all hiring lane topics + hiring.packet.ready are present")
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
        let builder = AssertionBuilder::new("No unexpected gates (hiring debrief panel example)")
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
            .find(|event| event.topic == "hiring.packet.ready")
            .map(|event| event.payload.clone())
            .unwrap_or_default();

        let ok = hiring_packet_payload_matches(&payload);
        let builder = AssertionBuilder::new(
            "Final payload expected (hiring debrief panel example)",
        )
        .expected(
            "hiring.packet.ready payload semantically contains STRONG_HIRE, SENIOR, and prepare_offer",
        )
        .actual(if payload.is_empty() {
            "hiring.packet.ready payload missing".to_string()
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
        let builder =
            AssertionBuilder::new("No new jobs after LOOP_COMPLETE (hiring debrief panel example)")
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

fn hiring_packet_payload_matches(payload: &str) -> bool {
    if payload.is_empty() {
        return false;
    }

    if let Ok(value) = serde_json::from_str::<serde_json::Value>(payload) {
        let recommendation = value
            .get("hiring_recommendation")
            .and_then(|value| value.as_str());
        let level = value.get("level").and_then(|value| value.as_str());
        let next_step = value.get("next_step").and_then(|value| value.as_str());

        return recommendation == Some("STRONG_HIRE")
            && level == Some("SENIOR")
            && next_step == Some("prepare_offer");
    }

    payload.contains("hiring_recommendation: STRONG_HIRE")
        && payload.contains("level: SENIOR")
        && payload.contains("next_step: prepare_offer")
}

impl Default for ParallelHiringDebriefPanelExampleScenario {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TestScenario for ParallelHiringDebriefPanelExampleScenario {
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
        setup_prompt_file_example_workspace(workspace, backend, "parallel-hiring-debrief-panel", 18)
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
        let config = include_str!("../../../../examples/parallel-hiring-debrief-panel/ralph.yml");

        assert!(
            !config.contains("<event") && !config.contains("</event>"),
            "example config must not contain raw event tags; use escaped display text instead"
        );
    }

    #[test]
    fn example_config_requires_silent_wait_before_all_ready_lanes() {
        let config = include_str!("../../../../examples/parallel-hiring-debrief-panel/ralph.yml");

        assert!(
            config.contains("当 4 条 ready 还没有全部到齐时:")
                && config.contains("你必须保持静默,空输出是合法且首选的")
                && config.contains("`LOOP_COMPLETE` 这个字符串只能在最终收尾那一行出现一次"),
            "parallel-hiring-debrief-panel config must explicitly forbid interim prose before all ready lanes arrive"
        );
    }

    #[test]
    fn example_config_forbids_self_closing_events() {
        let config = include_str!("../../../../examples/parallel-hiring-debrief-panel/ralph.yml");

        assert!(
            config.contains("不要使用自闭合 `&lt;event .../&gt;` 形式。")
                && config.contains("不要把业务字段塞进 opening tag 属性。"),
            "hiring debrief example must forbid self-closing events and attribute-only payloads"
        );
    }

    #[test]
    fn payload_matcher_accepts_json_and_line_payloads() {
        let json_payload = r#"{"candidate_id":"CAND-2026-17","hiring_recommendation":"STRONG_HIRE","level":"SENIOR","next_step":"prepare_offer"}"#;
        let line_payload = "candidate_id: CAND-2026-17
hiring_recommendation: STRONG_HIRE
level: SENIOR
next_step: prepare_offer";

        assert!(super::hiring_packet_payload_matches(json_payload));
        assert!(super::hiring_packet_payload_matches(line_payload));
    }
}
