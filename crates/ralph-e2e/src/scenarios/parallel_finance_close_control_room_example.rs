//! Tier 8: Parallel Runtime (experimental) - real-world example coverage.
//!
//! 目标:
//! - 直接覆盖 `examples/parallel-finance-close-control-room`
//! - 验证 finance close 多输入线并行推进后由 finalizer 汇总 `close.packet.ready`

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

/// 直接覆盖 `examples/parallel-finance-close-control-room` 的端到端场景。
pub struct ParallelFinanceCloseControlRoomExampleScenario {
    id: String,
    description: String,
    tier: String,
}

impl ParallelFinanceCloseControlRoomExampleScenario {
    pub fn new() -> Self {
        Self {
            id: "parallel-finance-close-control-room-example".to_string(),
            description: "Directly runs examples/parallel-finance-close-control-room and asserts finance close lanes converge into close.packet.ready".to_string(),
            tier: "Tier 8: Parallel Runtime".to_string(),
        }
    }

    fn parallel_mode_visible(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let visible = result.stdout.contains("[supervisor] instances");
        let builder =
            AssertionBuilder::new("Parallel mode visible (finance close control room example)")
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
                    "Agents snapshot written (finance close control room example)",
                )
                .expected(".ralph/agents.json exists and is valid JSON")
                .actual(error)
                .failed()
                .build();
            }
        };

        let instance_count = snapshot.instances.len();
        let has_revenue = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "revenue_reconciler");
        let has_expense = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "expense_accrual_reviewer");
        let has_cash = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "cash_controller");
        let has_anomaly = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "anomaly_watch_reviewer");
        let has_conductor = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "close_conductor");

        let ok = instance_count >= 5
            && has_revenue
            && has_expense
            && has_cash
            && has_anomaly
            && has_conductor;

        let builder = AssertionBuilder::new(
            "Agents snapshot written (finance close control room example)",
        )
        .expected("agents.json contains 4 lane hats + close conductor")
        .actual(format!(
            "instance_count={instance_count}, revenue={has_revenue}, expense={has_expense}, cash={has_cash}, anomaly={has_anomaly}, conductor={has_conductor}"
        ));

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn required_topic_chain_observed(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let required = [
            "close.revenue.reconcile",
            "close.expense.accrual.review",
            "close.cash.position.check",
            "close.anomaly.watch.review",
            "revenue.ready",
            "expense.ready",
            "cash.ready",
            "anomaly.ready",
            "close.packet.request",
            "close.packet.ready",
        ];

        let missing = required
            .iter()
            .filter(|topic| !result.events.iter().any(|event| event.topic == **topic))
            .copied()
            .collect::<Vec<_>>();

        let ok = missing.is_empty();
        let builder = AssertionBuilder::new(
            "Required topic chain observed (finance close control room example)",
        )
        .expected("all finance close lane topics + close.packet.ready are present")
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
            AssertionBuilder::new("No unexpected gates (finance close control room example)")
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
            .find(|event| event.topic == "close.packet.ready")
            .map(|event| event.payload.clone())
            .unwrap_or_default();

        let ok = close_packet_payload_matches(&payload);
        let builder = AssertionBuilder::new(
            "Final payload expected (finance close control room example)",
        )
        .expected(
            "close.packet.ready payload semantically contains READY_TO_CLOSE, WITHIN_THRESHOLD, and finance-ops",
        )
        .actual(if payload.is_empty() {
            "close.packet.ready payload missing".to_string()
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
            "No new jobs after LOOP_COMPLETE (finance close control room example)",
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

fn close_packet_payload_matches(payload: &str) -> bool {
    if payload.is_empty() {
        return false;
    }

    if let Ok(value) = serde_json::from_str::<serde_json::Value>(payload) {
        let close_status = value.get("close_status").and_then(|value| value.as_str());
        let materiality = value.get("materiality").and_then(|value| value.as_str());
        let owner = value.get("owner").and_then(|value| value.as_str());

        return close_status == Some("READY_TO_CLOSE")
            && materiality == Some("WITHIN_THRESHOLD")
            && owner == Some("finance-ops");
    }

    payload.contains("close_status: READY_TO_CLOSE")
        && payload.contains("materiality: WITHIN_THRESHOLD")
        && payload.contains("owner: finance-ops")
}

impl Default for ParallelFinanceCloseControlRoomExampleScenario {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn example_config_does_not_embed_raw_event_blocks() {
        let config =
            include_str!("../../../../examples/parallel-finance-close-control-room/ralph.yml");

        assert!(
            !config.contains("<event") && !config.contains("</event>"),
            "example config must not contain raw event tags; use escaped display text instead"
        );
    }

    #[test]
    fn example_config_requires_silent_wait_before_all_ready_lanes() {
        let config =
            include_str!("../../../../examples/parallel-finance-close-control-room/ralph.yml");

        assert!(
            config.contains("当 4 条 ready 还没有全部到齐时:")
                && config.contains("你必须保持静默,空输出是合法且首选的")
                && config.contains("`LOOP_COMPLETE` 这个字符串只能在最终收尾那一行出现一次"),
            "parallel-finance-close-control-room config must explicitly forbid interim prose before all ready lanes arrive"
        );
    }

    #[test]
    fn example_config_forbids_self_closing_events() {
        let config =
            include_str!("../../../../examples/parallel-finance-close-control-room/ralph.yml");

        assert!(
            config.contains("不要使用自闭合 `&lt;event .../&gt;` 形式。")
                && config.contains("不要把业务字段塞进 opening tag 属性。"),
            "finance close example must forbid self-closing events and attribute-only payloads"
        );
    }

    #[test]
    fn payload_matcher_accepts_json_and_line_payloads() {
        let json_payload = r#"{"close_id":"CLOSE-2026-03","close_status":"READY_TO_CLOSE","materiality":"WITHIN_THRESHOLD","owner":"finance-ops"}"#;
        let line_payload = "close_id: CLOSE-2026-03
close_status: READY_TO_CLOSE
materiality: WITHIN_THRESHOLD
owner: finance-ops";

        assert!(super::close_packet_payload_matches(json_payload));
        assert!(super::close_packet_payload_matches(line_payload));
    }
}

#[async_trait]
impl TestScenario for ParallelFinanceCloseControlRoomExampleScenario {
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
            "parallel-finance-close-control-room",
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
