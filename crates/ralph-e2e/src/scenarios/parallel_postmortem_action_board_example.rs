//! Tier 8: Parallel Runtime (experimental) - real-world example coverage.
//!
//! 目标:
//! - 直接覆盖 `examples/parallel-postmortem-action-board`
//! - 验证 postmortem 多输入线并行推进后由 facilitator 汇总 `postmortem.board.ready`

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

/// 直接覆盖 `examples/parallel-postmortem-action-board` 的端到端场景。
pub struct ParallelPostmortemActionBoardExampleScenario {
    id: String,
    description: String,
    tier: String,
}

impl ParallelPostmortemActionBoardExampleScenario {
    pub fn new() -> Self {
        Self {
            id: "parallel-postmortem-action-board-example".to_string(),
            description: "Directly runs examples/parallel-postmortem-action-board and asserts postmortem lanes converge into postmortem.board.ready".to_string(),
            tier: "Tier 8: Parallel Runtime".to_string(),
        }
    }

    fn parallel_mode_visible(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let visible = result.stdout.contains("[supervisor] instances");
        let builder =
            AssertionBuilder::new("Parallel mode visible (postmortem action board example)")
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
                    "Agents snapshot written (postmortem action board example)",
                )
                .expected(".ralph/agents.json exists and is valid JSON")
                .actual(error)
                .failed()
                .build();
            }
        };

        let instance_count = snapshot.instances.len();
        let has_timeline = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "timeline_curator");
        let has_root_cause = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "root_cause_editor");
        let has_actions = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "action_owner_mapper");
        let has_customer = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "customer_recap_writer");
        let has_facilitator = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "board_facilitator");

        let ok = instance_count >= 5
            && has_timeline
            && has_root_cause
            && has_actions
            && has_customer
            && has_facilitator;

        let builder = AssertionBuilder::new(
            "Agents snapshot written (postmortem action board example)",
        )
        .expected("agents.json contains 4 lane hats + board facilitator")
        .actual(format!(
            "instance_count={instance_count}, timeline={has_timeline}, root_cause={has_root_cause}, actions={has_actions}, customer={has_customer}, facilitator={has_facilitator}"
        ));

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn required_topic_chain_observed(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let required = [
            "pm.timeline.build",
            "pm.root_cause.review",
            "pm.action.map",
            "pm.customer.recap",
            "timeline.ready",
            "root_cause.ready",
            "actions.ready",
            "customer.recap.ready",
            "pm.board.request",
            "postmortem.board.ready",
        ];

        let missing = required
            .iter()
            .filter(|topic| !result.events.iter().any(|event| event.topic == **topic))
            .copied()
            .collect::<Vec<_>>();

        let ok = missing.is_empty();
        let builder = AssertionBuilder::new(
            "Required topic chain observed (postmortem action board example)",
        )
        .expected("all postmortem lane topics + postmortem.board.ready are present")
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
            AssertionBuilder::new("No unexpected gates (postmortem action board example)")
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
            .find(|event| event.topic == "postmortem.board.ready")
            .map(|event| event.payload.clone())
            .unwrap_or_default();

        let ok = postmortem_board_payload_matches(&payload);
        let builder = AssertionBuilder::new(
            "Final payload expected (postmortem action board example)",
        )
        .expected(
            "postmortem.board.ready payload semantically contains READY_FOR_REVIEW, add_completion_promise_guardrail, and runtime-platform",
        )
        .actual(if payload.is_empty() {
            "postmortem.board.ready payload missing".to_string()
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
            "No new jobs after LOOP_COMPLETE (postmortem action board example)",
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

fn postmortem_board_payload_matches(payload: &str) -> bool {
    if payload.is_empty() {
        return false;
    }

    if let Ok(value) = serde_json::from_str::<serde_json::Value>(payload) {
        let status = value.get("status").and_then(|value| value.as_str());
        let top_action = value.get("top_action").and_then(|value| value.as_str());
        let owner = value.get("owner").and_then(|value| value.as_str());

        return status == Some("READY_FOR_REVIEW")
            && top_action == Some("add_completion_promise_guardrail")
            && owner == Some("runtime-platform");
    }

    payload.contains("status: READY_FOR_REVIEW")
        && payload.contains("top_action: add_completion_promise_guardrail")
        && payload.contains("owner: runtime-platform")
}

impl Default for ParallelPostmortemActionBoardExampleScenario {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn example_config_does_not_embed_raw_event_blocks() {
        let config =
            include_str!("../../../../examples/parallel-postmortem-action-board/ralph.yml");

        assert!(
            !config.contains("<event") && !config.contains("</event>"),
            "example config must not contain raw event tags; use escaped display text instead"
        );
    }

    #[test]
    fn example_config_requires_event_only_workers() {
        let config =
            include_str!("../../../../examples/parallel-postmortem-action-board/ralph.yml");

        assert!(
            config.contains(
                "禁止输出 `&lt;event`、`&gt;`、代码块、前言、后续建议或任何事件外 prose。"
            ),
            "postmortem example must forbid escaped event display text and extra prose"
        );
        assert!(
            config.contains("你的最终回复必须直接从 `actions.ready` 的真实 event 开始标签开始。"),
            "action lane must be forced to emit a real actions.ready event"
        );
    }

    #[test]
    fn postmortem_board_payload_matches_json_and_line_payloads() {
        let json_payload = r#"{"postmortem_id":"PM-2026-0307","status":"READY_FOR_REVIEW","top_action":"add_completion_promise_guardrail","owner":"runtime-platform"}"#;
        let line_payload = "postmortem_id: PM-2026-0307\nstatus: READY_FOR_REVIEW\ntop_action: add_completion_promise_guardrail\nowner: runtime-platform";

        assert!(super::postmortem_board_payload_matches(json_payload));
        assert!(super::postmortem_board_payload_matches(line_payload));
    }
}

#[async_trait]
impl TestScenario for ParallelPostmortemActionBoardExampleScenario {
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
            "parallel-postmortem-action-board",
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
