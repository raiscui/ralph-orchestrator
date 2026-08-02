//! Tier 8: Parallel Runtime - 真实例子覆盖。
//! 目标是验证 `examples/parallel-multi-region-pipeline-sync` 的并行流程和 finalizer 收敛。

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

/// 并行多区域 pipeline 校准的 E2E 场景。
pub struct ParallelMultiRegionPipelineSyncExampleScenario {
    id: String,
    description: String,
    tier: String,
}

impl ParallelMultiRegionPipelineSyncExampleScenario {
    /// 创建默认的场景描述。
    pub fn new() -> Self {
        Self {
            id: "parallel-multi-region-pipeline-sync-example".to_string(),
            description:
                "直接运行 examples/parallel-multi-region-pipeline-sync,并断言四个区域 lane 收敛到 pipeline.sync.ready"
                    .to_string(),
            tier: "Tier 8: Parallel Runtime".to_string(),
        }
    }

    fn parallel_mode_visible(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let visible = result.stdout.contains("[supervisor] instances");
        let builder =
            AssertionBuilder::new("Parallel mode banner visible (multi-region pipeline sync)")
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
                    "Agents snapshot present (multi-region pipeline sync)",
                )
                .expected(".ralph/agents.json 可读")
                .actual(error)
                .failed()
                .build();
            }
        };

        let instance_count = snapshot.instances.len();
        let has_amer = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "amer_pipeline_reviewer");
        let has_emea = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "emea_pipeline_reviewer");
        let has_apj = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "apj_pipeline_reviewer");
        let has_latam = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "latam_pipeline_reviewer");
        let has_lead = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "global_pipeline_sync_lead");

        let ok = instance_count >= 5 && has_amer && has_emea && has_apj && has_latam && has_lead;

        let builder = AssertionBuilder::new(
            "Agents snapshot content (multi-region pipeline sync)",
        )
        .expected("agents.json 包含 4 条区域 lane + global_pipeline_sync_lead")
        .actual(format!(
            "count={instance_count} amer={has_amer} emea={has_emea} apj={has_apj} latam={has_latam} lead={has_lead}"
        ));

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn required_topic_chain_observed(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let required = [
            "pipeline.amer.review",
            "pipeline.emea.review",
            "pipeline.apj.review",
            "pipeline.latam.review",
            "amer.ready",
            "emea.ready",
            "apj.ready",
            "latam.ready",
            "pipeline.sync.packet.request",
            "pipeline.sync.ready",
        ];

        let missing = required
            .iter()
            .filter(|topic| !result.events.iter().any(|event| event.topic == **topic))
            .copied()
            .collect::<Vec<_>>();

        let ok = missing.is_empty();
        let builder =
            AssertionBuilder::new("Required topic chain observed (multi-region pipeline sync)")
                .expected("所有区域 lane topic + request + final ready 都出现")
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
        let builder = AssertionBuilder::new("No unexpected gates (multi-region pipeline sync)")
            .expected("no gate.* or approval.requested topics")
            .actual(format!("gate_topics={gates:?}"));

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn final_payload_expected(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let payload =
            extract_last_parallel_out_payload_for_topic(&result.stdout, "pipeline.sync.ready")
                .or_else(|| {
                    result
                        .events
                        .iter()
                        .rev()
                        .find(|event| event.topic == "pipeline.sync.ready")
                        .map(|event| event.payload.clone())
                })
                .unwrap_or_default();

        let ok = pipeline_sync_payload_matches(&payload);
        let builder = AssertionBuilder::new(
            "Final payload matches requirements (multi-region pipeline sync)",
        )
        .expected("pipeline.sync.ready payload 含固定字段")
        .actual(if payload.is_empty() {
            "pipeline.sync.ready payload 缺失".to_string()
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
            AssertionBuilder::new("No new jobs after LOOP_COMPLETE (multi-region pipeline sync)")
                .expected("LOOP_COMPLETE 之后没有新的 job")
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

fn pipeline_sync_payload_matches(payload: &str) -> bool {
    if payload.is_empty() {
        return false;
    }

    if let Ok(value) = serde_json::from_str::<Value>(payload) {
        let sync_status = value.get("sync_status").and_then(|field| field.as_str())
            == Some("READY_FOR_GLOBAL_FORECAST_CALL");
        let forecast_week =
            value.get("forecast_week").and_then(|field| field.as_str()) == Some("FY26_W15");
        let sync_owner = value.get("sync_owner").and_then(|field| field.as_str())
            == Some("global-revenue-operations");

        return sync_status && forecast_week && sync_owner;
    }

    payload.contains("sync_status: READY_FOR_GLOBAL_FORECAST_CALL")
        && payload.contains("forecast_week: FY26_W15")
        && payload.contains("sync_owner: global-revenue-operations")
}

impl Default for ParallelMultiRegionPipelineSyncExampleScenario {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TestScenario for ParallelMultiRegionPipelineSyncExampleScenario {
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
            "parallel-multi-region-pipeline-sync",
            20,
        )
    }

    async fn run(
        &self,
        executor: &RalphExecutor,
        config: &ScenarioConfig,
    ) -> Result<TestResult, ScenarioError> {
        let start = std::time::Instant::now();
        let execution = executor
            .run(config)
            .await
            .map_err(|error| ScenarioError::ExecutionError(format!("ralph 执行失败: {error}")))?;
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
            include_str!("../../../../examples/parallel-multi-region-pipeline-sync/ralph.yml");

        assert!(
            !config.contains("<event") && !config.contains("</event>"),
            "example config 不应直接包含事件标签"
        );
    }

    #[test]
    fn example_config_requires_silent_wait_before_all_ready_lanes() {
        let config =
            include_str!("../../../../examples/parallel-multi-region-pipeline-sync/ralph.yml");

        assert!(
            config.contains("当 4 条 ready 还没有全部到齐时:")
                && config.contains("你必须保持完全静默")
                && config.contains("`LOOP_COMPLETE` 只能在 `pipeline.sync.ready` 之后")
                && config.contains("pipeline.sync.packet.request"),
            "parallel-multi-region-pipeline-sync config 必须明确静默等待和 sync 请求时点"
        );
    }

    #[test]
    fn example_config_forbids_self_closing_events() {
        let config =
            include_str!("../../../../examples/parallel-multi-region-pipeline-sync/ralph.yml");

        assert!(
            config.contains("禁止自闭合 `&lt;event .../&gt;`")
                && config.contains("把字段塞进 opening tag 属性"),
            "multi-region pipeline sync 需要禁止自闭合及属性式 payload"
        );
    }

    #[test]
    fn example_config_requires_compact_latam_event_shape() {
        let config =
            include_str!("../../../../examples/parallel-multi-region-pipeline-sync/ralph.yml");

        assert!(
            config.contains("整个回复必须是一条单行真实事件")
                && config.contains("payload 请使用紧凑 JSON 对象")
                && config.contains("结束标签必须精确写成 `&lt;/event&gt;`"),
            "latam reviewer 必须被锁定为单行 JSON 事件,避免 closing tag 漂移"
        );
    }

    #[test]
    fn payload_matcher_accepts_json_and_line_payloads() {
        let json_payload = r#"{"sync_id":"MRP-2026-W15","sync_status":"READY_FOR_GLOBAL_FORECAST_CALL","forecast_week":"FY26_W15","sync_owner":"global-revenue-operations"}"#;
        let line_payload = "sync_id: MRP-2026-W15
sync_status: READY_FOR_GLOBAL_FORECAST_CALL
forecast_week: FY26_W15
sync_owner: global-revenue-operations";

        assert!(super::pipeline_sync_payload_matches(json_payload));
        assert!(super::pipeline_sync_payload_matches(line_payload));
    }
}
