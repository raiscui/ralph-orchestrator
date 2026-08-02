//! Tier 8: Parallel Runtime - 真实并行例子的覆盖。
//! 目标: 覆盖 `examples/parallel-customer-advisory-board-prep` 并验证四条 lane 收敛到 `cab.packet.ready`。

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

/// 覆盖 `examples/parallel-customer-advisory-board-prep` 的端到端验证场景。
pub struct ParallelCustomerAdvisoryBoardPrepExampleScenario {
    id: String,
    description: String,
    tier: String,
}

impl ParallelCustomerAdvisoryBoardPrepExampleScenario {
    /// 构建默认场景描述。
    pub fn new() -> Self {
        Self {
            id: "parallel-customer-advisory-board-prep-example".to_string(),
            description:
                "直接运行 examples/parallel-customer-advisory-board-prep,并断言四条 CAB 准备 lane 收敛到 cab.packet.ready"
                    .to_string(),
            tier: "Tier 8: Parallel Runtime".to_string(),
        }
    }

    fn parallel_mode_visible(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let visible = result.stdout.contains("[supervisor] instances");
        let builder = AssertionBuilder::new("Parallel mode visible (customer advisory board prep)")
            .expected("stdout 里包含 supervisor 实例 banner")
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

    fn agents_snapshot_written(&self, executor: &RalphExecutor) -> crate::models::Assertion {
        let snapshot = match read_agents_snapshot(executor.workspace()) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                return AssertionBuilder::new(
                    "Agents snapshot present (customer advisory board prep)",
                )
                .expected(".ralph/agents.json 可读")
                .actual(error)
                .failed()
                .build();
            }
        };

        // -----------------------------------------------------------------
        // 验证 4 条 lane hat 和 finalizer 都启动,确保 topic 名称不是空挂。
        // -----------------------------------------------------------------
        let instance_count = snapshot.instances.len();
        let has_cohort = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "customer_cohort_owner");
        let has_agenda = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "agenda_shaping_owner");
        let has_host = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "executive_host_owner");
        let has_logistics = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "logistics_readiness_owner");
        let has_lead = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "cab_program_lead");

        let ok = instance_count >= 5
            && has_cohort
            && has_agenda
            && has_host
            && has_logistics
            && has_lead;

        let builder = AssertionBuilder::new("Agents snapshot content (customer advisory board prep)")
            .expected("agents.json 包含 4 条 lane + cab_program_lead")
            .actual(format!(
                "count={instance_count} cohort={has_cohort} agenda={has_agenda} host={has_host} logistics={has_logistics} lead={has_lead}"
            ));

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn required_topic_chain_observed(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let required = [
            "cab.customer.cohort.review",
            "cab.agenda.shaping.review",
            "cab.exec.host.prep.review",
            "cab.logistics.readiness.review",
            "cohort.ready",
            "agenda.ready",
            "host.ready",
            "logistics.ready",
            "cab.packet.request",
            "cab.packet.ready",
        ];

        let missing = required
            .iter()
            .filter(|topic| !result.events.iter().any(|event| event.topic == **topic))
            .copied()
            .collect::<Vec<_>>();

        let ok = missing.is_empty();
        let builder =
            AssertionBuilder::new("Required topic chain observed (customer advisory board prep)")
                .expected("CAB lane topic + request + final ready 都要出现")
                .actual(format!("missing={missing:?}"));

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn final_payload_expected(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let payload =
            extract_last_parallel_out_payload_for_topic(&result.stdout, "cab.packet.ready")
                .or_else(|| {
                    result
                        .events
                        .iter()
                        .rev()
                        .find(|event| event.topic == "cab.packet.ready")
                        .map(|event| event.payload.clone())
                })
                .unwrap_or_default();

        let ok = cab_payload_matches(&payload);
        let builder = AssertionBuilder::new(
            "Final payload matches requirements (customer advisory board prep)",
        )
        .expected("cab.packet.ready payload 包含固定字段与约束")
        .actual(if payload.is_empty() {
            "cab.packet.ready payload 缺失".to_string()
        } else {
            payload
        });

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
        let builder = AssertionBuilder::new("No unexpected gates (customer advisory board prep)")
            .expected("不出现 gate.* 或 approval.requested topic")
            .actual(format!("gate_topics={gates:?}"));

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
            AssertionBuilder::new("No new jobs after LOOP_COMPLETE (customer advisory board prep)")
                .expected("LOOP_COMPLETE 之后不要再启动新 job")
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

fn cab_payload_matches(payload: &str) -> bool {
    if payload.is_empty() {
        return false;
    }

    if let Ok(value) = serde_json::from_str::<Value>(payload) {
        let status = value.get("cab_status").and_then(|value| value.as_str());
        let region = value.get("event_region").and_then(|value| value.as_str());
        let owner = value.get("next_owner").and_then(|value| value.as_str());
        let focus = value.get("packet_focus").and_then(|value| value.as_str());
        let summary = value.get("summary").and_then(|value| value.as_str());

        return status == Some("READY_TO_CONFIRM")
            && region == Some("APJ")
            && owner == Some("customer-marketing")
            && focus.is_some()
            && summary.is_some();
    }

    payload.contains("cab_status: READY_TO_CONFIRM")
        && payload.contains("event_region: APJ")
        && payload.contains("next_owner: customer-marketing")
        && payload.contains("packet_focus")
        && payload.contains("summary")
}

impl Default for ParallelCustomerAdvisoryBoardPrepExampleScenario {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TestScenario for ParallelCustomerAdvisoryBoardPrepExampleScenario {
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
            "parallel-customer-advisory-board-prep",
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
            include_str!("../../../../examples/parallel-customer-advisory-board-prep/ralph.yml");

        assert!(
            !config.contains("<event") && !config.contains("</event>"),
            "example config must not embed raw event tags"
        );
    }

    #[test]
    fn example_config_requires_silent_wait_before_all_ready_lanes() {
        let config =
            include_str!("../../../../examples/parallel-customer-advisory-board-prep/ralph.yml");

        assert!(
            config.contains("当 4 条 ready 还没有全部到齐时:")
                && config.contains("你必须保持静默,空输出是合法也是首选")
                && config.contains("`LOOP_COMPLETE` 这个字符串只能在最终收尾那一行出现一次"),
            "config must explicitly forbid interim prose before all ready lanes arrive"
        );
    }

    #[test]
    fn example_config_forbids_self_closing_events() {
        let config =
            include_str!("../../../../examples/parallel-customer-advisory-board-prep/ralph.yml");

        assert!(
            config.contains("不要使用自闭合 `&lt;event .../&gt;` 形式")
                && config.contains("不要把业务字段塞进 opening tag 属性"),
            "config must forbid self-closing events and attribute-only payloads"
        );
    }

    #[test]
    fn payload_matcher_accepts_json_and_line_payloads() {
        let json_payload = r#"{"packet_id":"CAB-APJ-2026-04","cab_status":"READY_TO_CONFIRM","event_region":"APJ","next_owner":"customer-marketing","packet_focus":"expand_critical_accounts","summary":"ready for confirmation"}"#;
        let line_payload = "packet_id: CAB-APJ-2026-04\n\
cab_status: READY_TO_CONFIRM\n\
event_region: APJ\n\
next_owner: customer-marketing\n\
packet_focus: expand_critical_accounts\n\
summary: ready for confirmation";

        assert!(super::cab_payload_matches(json_payload));
        assert!(super::cab_payload_matches(line_payload));
    }
}
