//! Tier 8: Parallel Runtime (experimental) - real-world example coverage.
//!
//! 目标:
//! - 直接覆盖 `examples/parallel-human-approval-gate`
//! - 验证自动化准备完成后,run 会等待外部审批事件
//! - 验证 `ralph emit approval.granted` 能把流程接回到 `deployment.ready`

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
use std::time::Duration;

/// 外部审批事件的固定 payload。
const APPROVAL_JSON: &str = r#"{"approved_by":"release-manager","window":"2026-03-10 10:00 UTC"}"#;

/// 直接覆盖 `examples/parallel-human-approval-gate` 的端到端场景。
pub struct ParallelHumanApprovalGateExampleScenario {
    id: String,
    description: String,
    tier: String,
}

impl ParallelHumanApprovalGateExampleScenario {
    pub fn new() -> Self {
        Self {
            id: "parallel-human-approval-gate-example".to_string(),
            description: "Directly runs examples/parallel-human-approval-gate and injects approval.granted after approval.requested".to_string(),
            tier: "Tier 8: Parallel Runtime".to_string(),
        }
    }

    async fn wait_for_topic(workspace: &Path, topic: &str) -> Result<(), String> {
        // -----------------------------------------------------------------
        // 说明:
        // - 这里直接盯 `.ralph/events.jsonl`。
        // - 一旦看到了 `approval.requested`,说明自动化准备已经走到“等待人类批准”的边界。
        // -----------------------------------------------------------------
        let debug_events = workspace.join(".ralph/events.jsonl");
        // -----------------------------------------------------------------
        // 说明:
        // - 真实 Codex 下,`approval.requested` 并不是立刻就会出现。
        // - `ralph#1` 需要先等 3 条自动化检查都完成,再自己进入下一轮收敛判断。
        // - 90s 在真实后端下过于贴边,容易出现“事件刚写出,injector 已经超时”的假失败。
        // -----------------------------------------------------------------
        let deadline = tokio::time::Instant::now() + Duration::from_secs(180);

        loop {
            if tokio::time::Instant::now() > deadline {
                return Err(format!("timed out waiting for topic `{topic}`"));
            }

            if let Ok(content) = tokio::fs::read_to_string(&debug_events).await {
                let found = content
                    .lines()
                    .filter(|line| !line.trim().is_empty())
                    .any(|line| {
                        serde_json::from_str::<serde_json::Value>(line)
                            .ok()
                            .and_then(|value| {
                                value
                                    .get("topic")
                                    .and_then(|topic| topic.as_str())
                                    .map(str::to_string)
                            })
                            .as_deref()
                            == Some(topic)
                    });

                if found {
                    return Ok(());
                }
            }

            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }

    async fn emit_approval_granted(
        ralph_bin: &Path,
        workspace: &Path,
    ) -> Result<(String, String), String> {
        use tokio::process::Command;

        let output = Command::new(ralph_bin)
            .current_dir(workspace)
            .arg("emit")
            .arg("approval.granted")
            .arg("--json")
            .arg(APPROVAL_JSON)
            .arg("--target-instance")
            .arg("ralph#1")
            .output()
            .await
            .map_err(|error| format!("failed to run ralph emit: {error}"))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !output.status.success() {
            return Err(format!(
                "ralph emit failed: status={:?}, stderr={stderr}",
                output.status.code()
            ));
        }

        Ok((stdout, stderr))
    }

    fn write_human_log(
        &self,
        executor: &RalphExecutor,
        execution: &ExecutionResult,
        emit_cmd: &str,
    ) -> Result<(), String> {
        let out_dir = executor.workspace().join(".e2e");
        std::fs::create_dir_all(&out_dir)
            .map_err(|error| format!("failed to create {}: {error}", out_dir.display()))?;

        let approval_requested = execution
            .events
            .iter()
            .position(|event| event.topic == "approval.requested")
            .map(|index| index.to_string())
            .unwrap_or_else(|| "missing".to_string());
        let approval_granted = execution
            .events
            .iter()
            .position(|event| event.topic == "approval.granted")
            .map(|index| index.to_string())
            .unwrap_or_else(|| "missing".to_string());

        let log = format!(
            "# Human Approval Gate Log\n\n\
## Emit Command\n\n\
```bash\n{emit_cmd}\n```\n\n\
## Topic Positions\n\n\
- approval.requested: {approval_requested}\n\
- approval.granted: {approval_granted}\n\n\
## Stdout Excerpt\n\n\
```text\n{}\n```\n",
            execution
                .stdout
                .lines()
                .take(80)
                .collect::<Vec<_>>()
                .join("\n")
        );

        std::fs::write(out_dir.join("human-log.md"), log)
            .map_err(|error| format!("failed to write human-log.md: {error}"))
    }

    fn parallel_mode_visible(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let visible = result.stdout.contains("[supervisor] instances");
        let builder = AssertionBuilder::new("Parallel mode visible (human approval gate example)")
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
                    "Agents snapshot written (human approval gate example)",
                )
                .expected(".ralph/agents.json exists and is valid JSON")
                .actual(error)
                .failed()
                .build();
            }
        };

        let instance_count = snapshot.instances.len();
        let has_deploy = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "deploy_checker");
        let has_rollback = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "rollback_checker");
        let has_comms = snapshot
            .instances
            .iter()
            .any(|instance| instance.hat_id == "comms_checker");

        let ok = instance_count >= 3 && has_deploy && has_rollback && has_comms;
        let builder =
            AssertionBuilder::new("Agents snapshot written (human approval gate example)")
                .expected("agents.json contains deploy/rollback/comms checkers")
                .actual(format!(
                    "instance_count={instance_count}, deploy={has_deploy}, rollback={has_rollback}, comms={has_comms}"
                ));

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn required_topic_chain_observed(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let required = [
            "deployment.plan.check",
            "rollback.plan.check",
            "comms.plan.check",
            "deployment.checked",
            "rollback.checked",
            "comms.checked",
            "approval.requested",
            "approval.granted",
            "deployment.ready",
        ];

        let missing = required
            .iter()
            .filter(|topic| !result.events.iter().any(|event| event.topic == **topic))
            .copied()
            .collect::<Vec<_>>();

        let ok = missing.is_empty();
        let builder = AssertionBuilder::new(
            "Required topic chain observed (human approval gate example)",
        )
        .expected("all automated checks + approval.requested + approval.granted + deployment.ready are present")
        .actual(format!("missing={missing:?}"));

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn approval_order_is_correct(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let requested_index = result
            .events
            .iter()
            .position(|event| event.topic == "approval.requested");
        let granted_index = result
            .events
            .iter()
            .position(|event| event.topic == "approval.granted");
        let ready_index = result
            .events
            .iter()
            .position(|event| event.topic == "deployment.ready");

        let ok = matches!(
            (requested_index, granted_index, ready_index),
            (Some(requested), Some(granted), Some(ready)) if requested < granted && granted < ready
        );

        let builder = AssertionBuilder::new(
            "Approval order is correct (human approval gate example)",
        )
        .expected("approval.requested < approval.granted < deployment.ready")
        .actual(format!(
            "requested={requested_index:?}, granted={granted_index:?}, ready={ready_index:?}"
        ));

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn deployment_ready_payload_present(
        &self,
        result: &ExecutionResult,
    ) -> crate::models::Assertion {
        let payload = result
            .events
            .iter()
            .rev()
            .find(|event| event.topic == "deployment.ready")
            .map(|event| event.payload.clone())
            .unwrap_or_default();

        let ok = payload.contains("rollout-2026-03-10-01");
        let builder =
            AssertionBuilder::new("Deployment ready payload present (human approval gate example)")
                .expected("deployment.ready payload contains rollout-2026-03-10-01")
                .actual(if payload.is_empty() {
                    "deployment.ready payload missing".to_string()
                } else {
                    payload
                });

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn human_log_written(&self, executor: &RalphExecutor) -> crate::models::Assertion {
        let path = executor.workspace().join(".e2e/human-log.md");
        let exists = path.exists();

        let builder = AssertionBuilder::new("Human log written (human approval gate example)")
            .expected(".e2e/human-log.md exists")
            .actual(if exists {
                path.display().to_string()
            } else {
                format!("missing: {}", path.display())
            });

        if exists {
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
        let builder =
            AssertionBuilder::new("No new jobs after LOOP_COMPLETE (human approval gate example)")
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

impl Default for ParallelHumanApprovalGateExampleScenario {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TestScenario for ParallelHumanApprovalGateExampleScenario {
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
        let mut config = setup_prompt_file_example_workspace(
            workspace,
            backend,
            "parallel-human-approval-gate",
            20,
        )?;
        config.timeout = std::cmp::min(backend.default_timeout(), Duration::from_secs(360));
        Ok(config)
    }

    async fn run(
        &self,
        executor: &RalphExecutor,
        config: &ScenarioConfig,
    ) -> Result<TestResult, ScenarioError> {
        let workspace = executor.workspace().clone();
        let ralph_bin = executor.ralph_binary();

        // -----------------------------------------------------------------
        // 说明:
        // - run 还在执行时就要发审批事件,因此这里和 `executor.run()` 并发。
        // - 一旦 `approval.requested` 出现,我们立刻执行 `ralph emit approval.granted`。
        // -----------------------------------------------------------------
        let inject_workspace = workspace.clone();
        let inject = tokio::spawn(async move {
            Self::wait_for_topic(&inject_workspace, "approval.requested").await?;

            let emit_cmd = format!(
                "ralph emit approval.granted --json '{APPROVAL_JSON}' --target-instance ralph#1"
            );
            let (stdout, stderr) =
                Self::emit_approval_granted(&ralph_bin, &inject_workspace).await?;

            Ok::<_, String>((emit_cmd, stdout, stderr))
        });

        let start = std::time::Instant::now();
        let execution = executor.run(config).await.map_err(|error| {
            ScenarioError::ExecutionError(format!("ralph execution failed: {error}"))
        })?;
        let duration = start.elapsed();

        let inject_res: Result<(String, String, String), String> = match inject.await {
            Ok(result) => result,
            Err(error) => Err(format!("approval injector task panicked: {error}")),
        };

        let out_dir = executor.workspace().join(".e2e");
        let _ = std::fs::create_dir_all(&out_dir);

        let mut emit_cmd = String::new();
        if let Ok((command, stdout, stderr)) = &inject_res {
            emit_cmd = command.clone();
            let _ = std::fs::write(out_dir.join("emit-approval.stdout.txt"), stdout);
            let _ = std::fs::write(out_dir.join("emit-approval.stderr.txt"), stderr);
        }

        let _ = self.write_human_log(executor, &execution, &emit_cmd);

        let mut assertions = vec![
            Assertions::response_received(&execution),
            Assertions::exit_code(&execution, 0),
            Assertions::no_timeout(&execution),
            self.parallel_mode_visible(&execution),
            self.agents_snapshot_written(executor),
            self.required_topic_chain_observed(&execution),
            self.approval_order_is_correct(&execution),
            self.deployment_ready_payload_present(&execution),
            self.human_log_written(executor),
            self.no_new_jobs_started_after_loop_complete(&execution),
        ];

        assertions.push({
            let builder = AssertionBuilder::new("Approval injector succeeded")
                .expected("ralph emit approval.granted succeeds after approval.requested")
                .actual(match &inject_res {
                    Ok(_) => "ok=true".to_string(),
                    Err(error) => format!("ok=false, error={error}"),
                });

            if inject_res.is_ok() {
                builder.passed().build()
            } else {
                builder.failed().build()
            }
        });

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
        let config = include_str!("../../../../examples/parallel-human-approval-gate/ralph.yml");

        assert!(
            !config.contains("<event") && !config.contains("</event>"),
            "example config must not contain raw event tags; use escaped display text instead"
        );
    }
}
