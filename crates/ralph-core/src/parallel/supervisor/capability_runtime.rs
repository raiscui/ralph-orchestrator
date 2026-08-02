//! Parent-run runtime capability invocation glue.
//!
//! 说明:
//! - `ParallelSupervisor` 只识别 `ralph#1` 输出的 `capability.request`。
//! - 真正的 isolated child/micro-run 由上层 adapter 注入。
//! - 这里负责幂等、坏请求失败事件、以及把 result/failure 定向回 parent coordinator。

use super::ParallelSupervisor;
use crate::{
    AgentChildRunSnapshot, AgentChildRunStatus, CapabilityFailureClass,
    CapabilityParentFailedRecord, CapabilityParentResultRecord, CapabilityRequestRecord,
    TOPIC_CAPABILITY_FAILED, TOPIC_CAPABILITY_REQUEST, TOPIC_CAPABILITY_RESULT,
};
use ralph_proto::{Event, HatId, HatInstanceId};

impl ParallelSupervisor {
    /// 处理 `ralph#1` parent output 中的 `capability.request`。
    ///
    /// 说明:
    /// - 这里只识别真实 coordinator hat 的输出,避免普通 hat 伪造 runtime action。
    /// - handled set 以 `request_id` 为幂等键,同一个 parent run 内不会重复启动 child/micro-run。
    /// - 返回的 result/failure 事件会由调用方写入 parent event log,并继续路由回 parent run。
    pub(super) async fn handle_parent_capability_requests(
        &mut self,
        hat_id: &HatId,
        events: &[Event],
    ) -> anyhow::Result<Vec<Event>> {
        if hat_id.as_str() != "ralph" {
            return Ok(Vec::new());
        }

        let mut return_events = Vec::new();

        for event in events
            .iter()
            .filter(|event| event.topic.as_str() == TOPIC_CAPABILITY_REQUEST)
        {
            let request = match CapabilityRequestRecord::parse_payload(&event.payload) {
                Ok(request) => request,
                Err(error) => {
                    let mut failed_event = parent_failed_event(
                        Some(event),
                        CapabilityParentFailedRecord {
                            status: "failed".to_string(),
                            failure_class: CapabilityFailureClass::MalformedRequest,
                            request_id: error.request_id,
                            invocation_id: None,
                            capability_id: error.capability_id,
                            error: error.error,
                            artifacts: None,
                            parent_topology_unchanged: true,
                        },
                    )?;
                    self.ensure_event_id(&mut failed_event);
                    return_events.push(failed_event);
                    continue;
                }
            };

            if !self
                .handled_capability_request_ids
                .insert(request.request_id.clone())
            {
                tracing::debug!(
                    request_id = %request.request_id,
                    capability_id = %request.capability_id,
                    "Duplicate capability.request ignored"
                );
                continue;
            }

            self.mark_child_run_running(&request);
            self.write_agents_snapshot_best_effort();

            let Some(invoker) = self.runtime_capability_invoker.clone() else {
                self.mark_child_run_failed(
                    &request.request_id,
                    &request.capability_id,
                    None,
                    "runtime capability invoker is not configured".to_string(),
                    None,
                );
                self.write_agents_snapshot_best_effort();

                let mut failed_event = parent_failed_event(
                    Some(event),
                    CapabilityParentFailedRecord {
                        status: "failed".to_string(),
                        failure_class: CapabilityFailureClass::InvokerUnavailable,
                        request_id: Some(request.request_id),
                        invocation_id: None,
                        capability_id: Some(request.capability_id),
                        error: "runtime capability invoker is not configured".to_string(),
                        artifacts: None,
                        parent_topology_unchanged: true,
                    },
                )?;
                self.ensure_event_id(&mut failed_event);
                return_events.push(failed_event);
                continue;
            };

            let request_for_failure = request.clone();
            let mut result_event = match invoker.invoke(request).await {
                Ok(event) => event,
                Err(error) => parent_failed_event(
                    None,
                    CapabilityParentFailedRecord {
                        status: "failed".to_string(),
                        failure_class: CapabilityFailureClass::Other,
                        request_id: Some(request_for_failure.request_id.clone()),
                        invocation_id: None,
                        capability_id: Some(request_for_failure.capability_id.clone()),
                        error: format!("{error:#}"),
                        artifacts: None,
                        parent_topology_unchanged: true,
                    },
                )?,
            };
            self.update_child_run_from_capability_event(&result_event, &request_for_failure);
            self.write_agents_snapshot_best_effort();

            result_event = result_event
                .with_reply(event.id.clone().unwrap_or_default())
                .with_target_instance(HatInstanceId::from_parts("ralph", "1"));
            self.ensure_event_id(&mut result_event);
            return_events.push(result_event);
        }

        Ok(return_events)
    }

    fn mark_child_run_running(&mut self, request: &CapabilityRequestRecord) {
        self.upsert_child_run_snapshot(AgentChildRunSnapshot {
            request_id: request.request_id.clone(),
            invocation_id: None,
            capability_id: request.capability_id.clone(),
            status: AgentChildRunStatus::Running,
            summary: None,
            artifact: None,
            updated_at: chrono::Utc::now().to_rfc3339(),
        });
    }

    fn mark_child_run_failed(
        &mut self,
        request_id: &str,
        capability_id: &str,
        invocation_id: Option<String>,
        summary: String,
        artifact: Option<String>,
    ) {
        self.upsert_child_run_snapshot(AgentChildRunSnapshot {
            request_id: request_id.to_string(),
            invocation_id,
            capability_id: capability_id.to_string(),
            status: AgentChildRunStatus::Failed,
            summary: Some(summary),
            artifact,
            updated_at: chrono::Utc::now().to_rfc3339(),
        });
    }

    fn update_child_run_from_capability_event(
        &mut self,
        event: &Event,
        fallback_request: &CapabilityRequestRecord,
    ) {
        match event.topic.as_str() {
            TOPIC_CAPABILITY_RESULT => {
                let Ok(result) =
                    serde_json::from_str::<CapabilityParentResultRecord>(&event.payload)
                else {
                    return;
                };
                self.upsert_child_run_snapshot(AgentChildRunSnapshot {
                    request_id: result.request_id,
                    invocation_id: Some(result.invocation_id),
                    capability_id: result.capability_id,
                    status: AgentChildRunStatus::Done,
                    summary: Some(result.result_summary),
                    artifact: result
                        .artifacts
                        .result_json
                        .or(Some(result.artifacts.invoke_json)),
                    updated_at: chrono::Utc::now().to_rfc3339(),
                });
            }
            TOPIC_CAPABILITY_FAILED => {
                let Ok(failed) =
                    serde_json::from_str::<CapabilityParentFailedRecord>(&event.payload)
                else {
                    return;
                };
                let artifact = failed
                    .artifacts
                    .as_ref()
                    .and_then(|artifacts| artifacts.failed_json.clone())
                    .or_else(|| {
                        failed
                            .artifacts
                            .as_ref()
                            .map(|artifacts| artifacts.invoke_json.clone())
                    });
                self.upsert_child_run_snapshot(AgentChildRunSnapshot {
                    request_id: failed
                        .request_id
                        .unwrap_or_else(|| fallback_request.request_id.clone()),
                    invocation_id: failed.invocation_id,
                    capability_id: failed
                        .capability_id
                        .unwrap_or_else(|| fallback_request.capability_id.clone()),
                    status: AgentChildRunStatus::Failed,
                    summary: Some(failed.error),
                    artifact,
                    updated_at: chrono::Utc::now().to_rfc3339(),
                });
            }
            _ => {}
        }
    }

    fn upsert_child_run_snapshot(&mut self, snapshot: AgentChildRunSnapshot) {
        if !self.child_runs.contains_key(&snapshot.request_id) {
            self.child_run_order.push(snapshot.request_id.clone());
        }
        self.child_runs
            .insert(snapshot.request_id.clone(), snapshot);
    }
}

fn parent_failed_event(
    source_event: Option<&Event>,
    failed: CapabilityParentFailedRecord,
) -> anyhow::Result<Event> {
    let mut event = Event::new(TOPIC_CAPABILITY_FAILED, serde_json::to_string(&failed)?);

    if let Some(source_event) = source_event {
        event = event
            .with_reply(source_event.id.clone().unwrap_or_default())
            .with_target_instance(HatInstanceId::from_parts("ralph", "1"));
    }

    Ok(event)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parallel::{HatJob, HatJobControl, HatJobExecutor, HatJobOutputChunk, HatJobResult};
    use crate::{
        CapabilityParentResultRecord, RalphConfig, RuntimeCapabilityInvoker,
        TOPIC_CAPABILITY_RESULT,
    };
    use async_trait::async_trait;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use tokio::sync::{mpsc, watch};

    #[derive(Debug)]
    struct NoopExecutor;

    #[async_trait]
    impl HatJobExecutor for NoopExecutor {
        async fn execute(
            &self,
            _job: HatJob,
            _output_tx: mpsc::Sender<HatJobOutputChunk>,
            _cancel_rx: watch::Receiver<bool>,
            _control_rx: mpsc::Receiver<HatJobControl>,
        ) -> anyhow::Result<HatJobResult> {
            Ok(HatJobResult {
                output_for_parsing: String::new(),
                observed_stderr: String::new(),
                success: true,
                exit_code: Some(0),
                timed_out: false,
                canceled: false,
            })
        }
    }

    #[derive(Debug)]
    struct CountingInvoker {
        calls: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl RuntimeCapabilityInvoker for CountingInvoker {
        async fn invoke(&self, request: CapabilityRequestRecord) -> anyhow::Result<Event> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let result = CapabilityParentResultRecord {
                status: "result".to_string(),
                request_id: request.request_id,
                invocation_id: "cap-test-1".to_string(),
                capability_id: request.capability_id,
                result_summary: "ok".to_string(),
                artifacts: crate::CapabilityParentArtifactPaths {
                    invoke_json: ".ralph/capability-invocations/cap-test-1/invoke.json".to_string(),
                    result_json: Some(
                        ".ralph/capability-invocations/cap-test-1/result.json".to_string(),
                    ),
                    failed_json: None,
                    resolved_config: ".ralph/capability-invocations/cap-test-1/resolved-config.yml"
                        .to_string(),
                    events_jsonl: ".ralph/events.jsonl".to_string(),
                    evidence_index: ".ralph/evidence-index.jsonl".to_string(),
                },
                parent_topology_unchanged: true,
            };
            Ok(Event::new(
                TOPIC_CAPABILITY_RESULT,
                serde_json::to_string(&result)?,
            ))
        }
    }

    fn test_supervisor(calls: Arc<AtomicUsize>) -> ParallelSupervisor {
        let mut supervisor = ParallelSupervisor::new(
            RalphConfig::default(),
            "prompt".to_string(),
            Arc::new(NoopExecutor),
        )
        .expect("supervisor");
        supervisor.runtime_capability_invoker = Some(Arc::new(CountingInvoker { calls }));
        supervisor
    }

    fn request_event(id: &str, request_id: &str) -> Event {
        Event::new(
            TOPIC_CAPABILITY_REQUEST,
            format!(
                r#"{{"request_id":"{request_id}","capability_id":"hat:focused-reviewer","input":"review"}}"#
            ),
        )
        .with_id(id)
    }

    #[tokio::test]
    async fn parent_capability_request_from_ralph_invokes_once_per_request_id() {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut supervisor = test_supervisor(Arc::clone(&calls));
        let events = vec![
            request_event("event-1", "cap-req-1"),
            request_event("event-2", "cap-req-1"),
        ];

        let returned = supervisor
            .handle_parent_capability_requests(&HatId::new("ralph"), &events)
            .await
            .unwrap();

        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(returned.len(), 1);
        assert_eq!(returned[0].topic.as_str(), TOPIC_CAPABILITY_RESULT);
        assert_eq!(
            returned[0]
                .target_instance
                .as_ref()
                .map(HatInstanceId::as_str),
            Some("ralph#1")
        );

        let snapshot = supervisor.build_agents_snapshot();
        assert_eq!(snapshot.child_runs.len(), 1);
        assert_eq!(snapshot.child_runs[0].request_id, "cap-req-1");
        assert_eq!(
            snapshot.child_runs[0].invocation_id.as_deref(),
            Some("cap-test-1")
        );
        assert_eq!(snapshot.child_runs[0].status, AgentChildRunStatus::Done);
        assert_eq!(snapshot.child_runs[0].summary.as_deref(), Some("ok"));
    }

    #[tokio::test]
    async fn malformed_capability_request_returns_failure_without_invoking() {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut supervisor = test_supervisor(Arc::clone(&calls));
        let events = vec![
            Event::new(
                TOPIC_CAPABILITY_REQUEST,
                r#"{"request_id":"cap-req-bad","input":"review"}"#,
            )
            .with_id("bad-event"),
        ];

        let returned = supervisor
            .handle_parent_capability_requests(&HatId::new("ralph"), &events)
            .await
            .unwrap();

        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(returned.len(), 1);
        assert_eq!(returned[0].topic.as_str(), TOPIC_CAPABILITY_FAILED);
        assert!(returned[0].payload.contains("capability_id"));
        assert!(returned[0].payload.contains("malformed_request"));
        assert_eq!(
            returned[0]
                .target_instance
                .as_ref()
                .map(HatInstanceId::as_str),
            Some("ralph#1")
        );
    }

    #[tokio::test]
    async fn capability_request_from_non_parent_hat_is_ignored() {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut supervisor = test_supervisor(Arc::clone(&calls));
        let events = vec![request_event("event-1", "cap-req-1")];

        let returned = supervisor
            .handle_parent_capability_requests(&HatId::new("reviewer"), &events)
            .await
            .unwrap();

        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert!(returned.is_empty());
    }

    #[tokio::test]
    async fn unavailable_invoker_marks_child_run_failed_in_agents_snapshot() {
        let mut supervisor = ParallelSupervisor::new(
            RalphConfig::default(),
            "prompt".to_string(),
            Arc::new(NoopExecutor),
        )
        .expect("supervisor");
        let events = vec![request_event("event-1", "cap-req-no-invoker")];

        let returned = supervisor
            .handle_parent_capability_requests(&HatId::new("ralph"), &events)
            .await
            .unwrap();

        assert_eq!(returned.len(), 1);
        assert_eq!(returned[0].topic.as_str(), TOPIC_CAPABILITY_FAILED);

        let snapshot = supervisor.build_agents_snapshot();
        assert_eq!(snapshot.child_runs.len(), 1);
        assert_eq!(snapshot.child_runs[0].request_id, "cap-req-no-invoker");
        assert_eq!(snapshot.child_runs[0].capability_id, "hat:focused-reviewer");
        assert_eq!(snapshot.child_runs[0].status, AgentChildRunStatus::Failed);
        assert!(
            snapshot.child_runs[0]
                .summary
                .as_deref()
                .is_some_and(|summary| summary.contains("invoker is not configured"))
        );
    }
}
