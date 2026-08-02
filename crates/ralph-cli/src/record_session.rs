//! record-session 的渲染层: Evidence Inspect 文本、agents sidecar 渲染、指针写入。
//!
//! 说明:
//! - strict 解析与聚合已下沉到 `ralph-core::record_aggregate`(窄入口 `aggregate_session`)。
//! - 本模块只负责"把聚合渲染成人可读文本"与"命令面的指针/定位"。
//! - autopilot / record summary / capability 都通过 `aggregate_session` 取聚合,口径单一。

use anyhow::{Context, Result};
use ralph_core::{
    AgentsSnapshot, EvidenceInspectAggregate, RecordSessionAggregate,
    truncate_with_ellipsis,
};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

pub(crate) enum AgentsSnapshotInspect<'a> {
    Loaded {
        path: &'a str,
        snapshot: &'a AgentsSnapshot,
    },
    Missing {
        searched: Vec<String>,
    },
    Invalid {
        path: &'a str,
        error: &'a str,
    },
}
pub(crate) fn render_evidence_inspect(
    aggregate: &RecordSessionAggregate,
    agents: AgentsSnapshotInspect<'_>,
) -> Result<String> {
    let mut out = String::new();
    let evidence = &aggregate.evidence;

    writeln!(out, "Evidence Inspect")?;
    writeln!(out, "  Termination")?;
    writeln!(out, "    semantic_source: record-session _meta.termination")?;
    if let Some(termination) = &aggregate.termination {
        writeln!(
            out,
            "    reason: {}",
            termination.reason.as_deref().unwrap_or("<missing>")
        )?;
        writeln!(
            out,
            "    iterations: {}",
            termination
                .iterations
                .map(|value| value.to_string())
                .unwrap_or_else(|| "<missing>".to_string())
        )?;
        writeln!(
            out,
            "    elapsed_secs: {}",
            termination
                .elapsed_secs
                .map(|value| format!("{value:.3}"))
                .unwrap_or_else(|| "<missing>".to_string())
        )?;
    } else {
        writeln!(out, "    <missing>")?;
        writeln!(
            out,
            "    semantic_completion: missing; do not infer completion from topology spawn success, stdout tail, wrapper exit status, or display state"
        )?;
    }

    writeln!(out, "  Topology")?;
    writeln!(
        out,
        "    topology.spawn_group: {}",
        evidence.topology_spawn_groups.len()
    )?;
    for item in &evidence.topology_spawn_groups {
        writeln!(
            out,
            "      - line={} event={} request_id={} hat={} delivery_topic={} requested_instances={}",
            item.record_index + 1,
            item.event_id.as_deref().unwrap_or("-"),
            item.request.request_id,
            item.request.hat,
            item.request.delivery_topic,
            item.request.instances.len()
        )?;
    }

    writeln!(
        out,
        "    topology.spawn.result: {}",
        evidence.topology_spawn_results.len()
    )?;
    for item in &evidence.topology_spawn_results {
        let spawned = item
            .result
            .spawned
            .iter()
            .map(|spawned| {
                let fixed = if spawned.fixed_role == Some(true) {
                    ",fixed"
                } else {
                    ""
                };
                let contract = spawned
                    .role_contract_summary
                    .as_ref()
                    .map(|summary| {
                        format!(
                            ",identity_source={},persistence={},contract_schema_version={},role_contract_hash={},source_spawn_request_id={}",
                            summary.identity_source,
                            summary.persistence,
                            summary.contract_schema_version,
                            short_hash(&summary.role_contract_hash),
                            summary.source_spawn_request_id
                        )
                    })
                    .unwrap_or_default();
                format!(
                    "{}:{}{}{}",
                    spawned.instance_id, spawned.role, fixed, contract
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        writeln!(
            out,
            "      - line={} event={} request_id={} status={} hat={} delivery_topic={} parent_topology_unchanged={} spawned=[{}] failed={}",
            item.record_index + 1,
            item.event_id.as_deref().unwrap_or("-"),
            item.result.request_id,
            item.result.status,
            item.result.hat,
            item.result.delivery_topic,
            item.result.parent_topology_unchanged,
            if spawned.is_empty() {
                "-".to_string()
            } else {
                spawned
            },
            item.result.failed.len()
        )?;
        if !item.result.failed.is_empty() {
            for failed in &item.result.failed {
                writeln!(
                    out,
                    "        failed_member index={} role={} request_id={} instance_id={} phase={} recovery_hint={} error={}",
                    failed.index,
                    failed.role,
                    failed.request_id.as_deref().unwrap_or("-"),
                    failed.instance_id.as_deref().unwrap_or("-"),
                    failed.phase.as_deref().unwrap_or("-"),
                    failed
                        .recovery_hint
                        .as_deref()
                        .map(one_line)
                        .unwrap_or_else(|| "-".to_string()),
                    one_line(&failed.error)
                )?;
            }
        }
    }

    render_dynamic_result_coverage(&mut out, evidence)?;

    writeln!(
        out,
        "    topology.spawn.failed: {}",
        evidence.topology_spawn_failures.len()
    )?;
    for item in &evidence.topology_spawn_failures {
        writeln!(
            out,
            "      - line={} event={} request_id={} hat={} delivery_topic={} parent_topology_unchanged={} error={}",
            item.record_index + 1,
            item.event_id.as_deref().unwrap_or("-"),
            item.failed.request_id.as_deref().unwrap_or("-"),
            item.failed.hat.as_deref().unwrap_or("-"),
            item.failed.delivery_topic.as_deref().unwrap_or("-"),
            item.failed.parent_topology_unchanged,
            one_line(&item.failed.error)
        )?;
    }

    render_agents_snapshot(&mut out, agents)?;
    render_capability_events(&mut out, evidence)?;
    render_result_topics(&mut out, evidence)?;

    if !evidence.parse_errors.is_empty() {
        writeln!(out, "  Parse Warnings")?;
        for error in &evidence.parse_errors {
            writeln!(out, "    - {}", one_line(error))?;
        }
    }

    Ok(out)
}

fn render_agents_snapshot(out: &mut String, agents: AgentsSnapshotInspect<'_>) -> Result<()> {
    writeln!(out, "  Agents Snapshot")?;
    match agents {
        AgentsSnapshotInspect::Loaded { path, snapshot } => {
            writeln!(out, "    path: {path}")?;
            writeln!(out, "    generated_at: {}", snapshot.generated_at)?;
            writeln!(
                out,
                "    instances: {} (current registry)",
                snapshot.instances.len()
            )?;
            if snapshot.instances.is_empty() {
                writeln!(out, "      <none>")?;
            }
            for instance in &snapshot.instances {
                let dynamic = if instance.is_dynamic {
                    "dynamic"
                } else {
                    "static"
                };
                let fixed_role = instance.fixed_role_label.as_deref().unwrap_or("-");
                let contract = instance
                    .role_contract_summary
                    .as_ref()
                    .map(|summary| {
                        format!(
                            "{}:{}:contract_schema_version={}:role_contract_hash={}:source_spawn_request_id={}",
                            summary.identity_source,
                            summary.persistence,
                            summary.contract_schema_version,
                            short_hash(&summary.role_contract_hash),
                            summary.source_spawn_request_id
                        )
                    })
                    .unwrap_or_else(|| "-".to_string());
                let last_topic = instance
                    .last_input
                    .as_ref()
                    .map(|input| input.topic.as_str())
                    .unwrap_or("-");
                writeln!(
                    out,
                    "      - {} hat={} state={} kind={} source={} fixed_role={} role_contract={} last_input={}",
                    instance.instance_id,
                    instance.hat_id,
                    instance.state.as_str(),
                    dynamic,
                    instance.identity_source,
                    fixed_role,
                    contract,
                    last_topic
                )?;
            }

            render_recoverable_failures(out, snapshot)?;

            render_completed_dynamic_instances(out, snapshot)?;

            writeln!(out, "  Child Runs")?;
            writeln!(out, "    child_runs: {}", snapshot.child_runs.len())?;
            if snapshot.child_runs.is_empty() {
                writeln!(out, "      <none>")?;
            }
            for child in &snapshot.child_runs {
                writeln!(
                    out,
                    "      - request_id={} capability={} status={} invocation={} artifact={} summary={}",
                    child.request_id,
                    child.capability_id,
                    child.status.as_str(),
                    child.invocation_id.as_deref().unwrap_or("-"),
                    child.artifact.as_deref().unwrap_or("-"),
                    child
                        .summary
                        .as_deref()
                        .map(|summary| truncate_with_ellipsis(&one_line(summary), 96))
                        .unwrap_or_else(|| "-".to_string())
                )?;
            }
        }
        AgentsSnapshotInspect::Missing { searched } => {
            writeln!(out, "    <missing>")?;
            if !searched.is_empty() {
                writeln!(out, "    searched: {}", searched.join(", "))?;
            }
            writeln!(out, "  Completed Dynamic Instances")?;
            writeln!(
                out,
                "    completed_dynamic_instances: <unknown: agents snapshot missing>"
            )?;
            writeln!(out, "  Recoverable Failures")?;
            writeln!(
                out,
                "    recoverable_failures: <unknown: agents snapshot missing>"
            )?;
            writeln!(out, "  Child Runs")?;
            writeln!(out, "    child_runs: <unknown: agents snapshot missing>")?;
        }
        AgentsSnapshotInspect::Invalid { path, error } => {
            writeln!(out, "    <invalid> path={path} error={}", one_line(error))?;
            writeln!(out, "  Completed Dynamic Instances")?;
            writeln!(
                out,
                "    completed_dynamic_instances: <unknown: agents snapshot invalid>"
            )?;
            writeln!(out, "  Recoverable Failures")?;
            writeln!(
                out,
                "    recoverable_failures: <unknown: agents snapshot invalid>"
            )?;
            writeln!(out, "  Child Runs")?;
            writeln!(out, "    child_runs: <unknown: agents snapshot invalid>")?;
        }
    }

    Ok(())
}

fn render_recoverable_failures(out: &mut String, snapshot: &AgentsSnapshot) -> Result<()> {
    writeln!(out, "  Recoverable Failures")?;

    let failures = snapshot
        .instances
        .iter()
        .flat_map(|instance| {
            instance
                .recoverable_failures
                .iter()
                .map(move |failure| (instance.instance_id.as_str(), failure))
        })
        .collect::<Vec<_>>();

    writeln!(out, "    recoverable_failures: {}", failures.len())?;
    if failures.is_empty() {
        writeln!(out, "      <none>")?;
        return Ok(());
    }

    for (instance_id, failure) in failures {
        writeln!(
            out,
            "      - failure_id={} instance={} job_id={} status={} kind={} attempt={}/{} retry_after_ms={} next_retry_at={} ledger={} stderr={}",
            failure.failure_id,
            instance_id,
            failure.job_id,
            failure.status,
            failure.failure_kind,
            failure.attempt,
            failure.max_attempts,
            failure
                .retry_after_ms
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string()),
            failure.next_retry_at.as_deref().unwrap_or("-"),
            failure.ledger_path,
            failure
                .stderr_preview
                .as_deref()
                .map(one_line)
                .unwrap_or_else(|| "-".to_string())
        )?;
    }

    Ok(())
}

fn render_completed_dynamic_instances(out: &mut String, snapshot: &AgentsSnapshot) -> Result<()> {
    writeln!(out, "  Completed Dynamic Instances")?;
    writeln!(
        out,
        "    completed_dynamic_instances: {}",
        snapshot.completed_dynamic_instances.len()
    )?;
    if snapshot.completed_dynamic_instances.is_empty() {
        writeln!(out, "      <none>")?;
    }

    for instance in &snapshot.completed_dynamic_instances {
        let fixed_role = instance.fixed_role_label.as_deref().unwrap_or("-");
        let contract = instance
            .role_contract_summary
            .as_ref()
            .map(|summary| {
                format!(
                    "{}:{}:contract_schema_version={}:role_contract_hash={}:source_spawn_request_id={}",
                    summary.identity_source,
                    summary.persistence,
                    summary.contract_schema_version,
                    short_hash(&summary.role_contract_hash),
                    summary.source_spawn_request_id
                )
            })
            .unwrap_or_else(|| "-".to_string());
        let last_topic = instance
            .last_input
            .as_ref()
            .map(|input| input.topic.as_str())
            .unwrap_or("-");
        writeln!(
            out,
            "      - {} hat={} final_state={} source={} fixed_role={} role_contract={} last_input={} completed_at={} retirement_reason={}",
            instance.instance_id,
            instance.hat_id,
            instance.final_state.as_str(),
            instance.identity_source,
            fixed_role,
            contract,
            last_topic,
            instance.completed_at,
            instance.retirement_reason
        )?;
    }

    Ok(())
}

fn render_dynamic_result_coverage(
    out: &mut String,
    evidence: &EvidenceInspectAggregate,
) -> Result<()> {
    writeln!(out, "    Dynamic Result Coverage")?;

    let mut rendered_any = false;
    for item in &evidence.topology_spawn_results {
        for spawned in &item.result.spawned {
            rendered_any = true;
            let Some(summary) = spawned.role_contract_summary.as_ref() else {
                writeln!(
                    out,
                    "      - request_id={} instance={} role={} expected=<unknown: no role_contract_summary> covered=<unknown> missing=<unknown>",
                    item.result.request_id, spawned.instance_id, spawned.role
                )?;
                continue;
            };

            let expected = summary.allowed_result_topics.clone();
            let covered = expected
                .iter()
                .filter(|topic| {
                    evidence
                        .result_topics
                        .get(topic.as_str())
                        .is_some_and(|topic_evidence| {
                            topic_evidence
                                .source_instances
                                .contains(spawned.instance_id.as_str())
                        })
                })
                .cloned()
                .collect::<Vec<_>>();
            let missing = expected
                .iter()
                .filter(|topic| !covered.contains(topic))
                .cloned()
                .collect::<Vec<_>>();

            writeln!(
                out,
                "      - request_id={} instance={} role={} role_contract_hash={} expected={} covered={} missing={}",
                item.result.request_id,
                spawned.instance_id,
                spawned.role,
                short_hash(&summary.role_contract_hash),
                list_or_dash(expected),
                list_or_dash(covered),
                list_or_dash(missing)
            )?;
        }
    }

    if !rendered_any {
        writeln!(out, "      <none>")?;
    }

    Ok(())
}

fn render_capability_events(out: &mut String, evidence: &EvidenceInspectAggregate) -> Result<()> {
    writeln!(out, "  Capability Events")?;
    writeln!(
        out,
        "    capability.request: {}",
        evidence.capability_requests.len()
    )?;
    for item in &evidence.capability_requests {
        writeln!(
            out,
            "      - line={} event={} request_id={} capability={}",
            item.record_index + 1,
            item.event_id.as_deref().unwrap_or("-"),
            item.request.request_id,
            item.request.capability_id
        )?;
    }
    writeln!(
        out,
        "    capability.result: {}",
        evidence.capability_results.len()
    )?;
    for item in &evidence.capability_results {
        writeln!(
            out,
            "      - line={} event={} request_id={} invocation={} capability={} parent_topology_unchanged={} summary={}",
            item.record_index + 1,
            item.event_id.as_deref().unwrap_or("-"),
            item.result.request_id,
            item.result.invocation_id,
            item.result.capability_id,
            item.result.parent_topology_unchanged,
            truncate_with_ellipsis(&one_line(&item.result.result_summary), 96)
        )?;
    }
    writeln!(
        out,
        "    capability.failed: {}",
        evidence.capability_failures.len()
    )?;
    for item in &evidence.capability_failures {
        writeln!(
            out,
            "      - line={} event={} request_id={} invocation={} capability={} class={} parent_topology_unchanged={} error={}",
            item.record_index + 1,
            item.event_id.as_deref().unwrap_or("-"),
            item.failed.request_id.as_deref().unwrap_or("-"),
            item.failed.invocation_id.as_deref().unwrap_or("-"),
            item.failed.capability_id.as_deref().unwrap_or("-"),
            item.failed.failure_class,
            item.failed.parent_topology_unchanged,
            truncate_with_ellipsis(&one_line(&item.failed.error), 96)
        )?;
    }
    Ok(())
}

fn render_result_topics(out: &mut String, evidence: &EvidenceInspectAggregate) -> Result<()> {
    writeln!(out, "  Result Topics")?;
    if evidence.result_topics.is_empty() {
        writeln!(out, "    <none>")?;
        return Ok(());
    }

    for (topic, item) in &evidence.result_topics {
        let sources = if item.source_instances.is_empty() {
            "-".to_string()
        } else {
            item.source_instances
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(",")
        };
        writeln!(
            out,
            "    - {topic}: {} source_instances={sources}",
            item.count
        )?;
    }

    Ok(())
}

fn one_line(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn short_hash(value: &str) -> String {
    value.chars().take(12).collect()
}

fn list_or_dash(values: Vec<String>) -> String {
    if values.is_empty() {
        "-".to_string()
    } else {
        values.join(",")
    }
}

pub(crate) fn write_record_session_latest_pointer(
    workspace_root: &Path,
    record_path: &Path,
) -> Result<PathBuf> {
    // 说明:
    // - record-session 文件不一定放在 `.ralph/` 里.
    // - 但 `.ralph/record-session.latest` 作为“证据指针”应位于 workspace_root 下,
    //   便于在子目录执行 `ralph record watch` 时向上自动定位.
    let pointer_path = workspace_root.join(".ralph/record-session.latest");
    if let Some(parent) = pointer_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create pointer directory: {}", parent.display()))?;
    }

    // 优先写绝对路径,避免 watch 时的歧义.
    let absolute = if record_path.is_absolute() {
        record_path.to_path_buf()
    } else {
        workspace_root.join(record_path)
    };

    // canonicalize 是 best-effort:
    // - 失败时仍写一个“可解析路径”(absolute).
    let target = std::fs::canonicalize(&absolute).unwrap_or(absolute);

    std::fs::write(&pointer_path, format!("{}\n", target.display())).with_context(|| {
        format!(
            "Failed to write record-session latest pointer: {}",
            pointer_path.display()
        )
    })?;

    Ok(pointer_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ralph_core::SessionRecorder;
    use ralph_core::{
        AgentChildRunSnapshot, AgentChildRunStatus, AgentCompletedDynamicInstanceSnapshot,
        AgentInstanceSnapshot, AgentLastInput, AgentsSnapshot, CapabilityParentArtifactPaths,
        CapabilityParentResultRecord, TopologySpawnFailedMember, TopologySpawnGroupResult,
        TopologySpawnedInstance,
    };
    use ralph_core::{
        TopologySpawnResultEvidence, aggregate_record_session, load_session_player_strict, Record,
    };
    use ralph_proto::Event;
    use ralph_proto::HatInstanceState;
    use ralph_proto::{TerminalWrite, UxEvent};



    #[test]
    fn aggregate_collects_evidence_inspect() -> Result<()> {
        let dir = tempfile::TempDir::new()?;
        let path = dir.path().join("session.jsonl");

        let file = std::fs::File::create(&path)?;
        let recorder = SessionRecorder::new(std::io::BufWriter::new(file));

        let argv = vec!["ralph".to_string(), "run".to_string()];
        recorder.record_meta(Record::meta_session_start(
            Some("/tmp/project"),
            Some("/tmp/project"),
            &argv,
            42,
            Some("/tmp/bin/ralph"),
            Some("0.0.0-test"),
        ));

        let spawn_group = Event::new(
            ralph_core::TOPIC_TOPOLOGY_SPAWN_GROUP,
            r#"{
                "request_id":"spawn-1",
                "hat":"analyst",
                "delivery_topic":"analysis.task",
                "instances":[
                    {"role":"功能补充","task":"补功能"},
                    {"role":"review","task":"review","fixed_role":true}
                ]
            }"#,
        )
        .with_id("evt-spawn")
        .with_source_instance("ralph#1");
        recorder.record_bus_event(&spawn_group);

        let spawn_result = TopologySpawnGroupResult {
            status: "spawned".to_string(),
            request_id: "spawn-1".to_string(),
            hat: "analyst".to_string(),
            delivery_topic: "analysis.task".to_string(),
            spawned: vec![
                TopologySpawnedInstance {
                    index: 0,
                    instance_id: "analyst#2".to_string(),
                    role: "功能补充".to_string(),
                    fixed_role: None,
                    role_contract_summary: Some(ralph_core::RoleContractSummary {
                        role_name: "功能补充".to_string(),
                        objective_preview: "补功能".to_string(),
                        allowed_result_topics: vec!["analysis.done".to_string()],
                        identity_source: ralph_core::IdentitySource::TaskDerived,
                        persistence: ralph_core::RolePersistence::Temporary,
                        contract_schema_version: 1,
                        role_contract_hash: "erc-aaaabbbbccccdddd".to_string(),
                        source_spawn_request_id: "spawn-1".to_string(),
                    }),
                },
                TopologySpawnedInstance {
                    index: 1,
                    instance_id: "analyst#3".to_string(),
                    role: "review".to_string(),
                    fixed_role: Some(true),
                    role_contract_summary: Some(ralph_core::RoleContractSummary {
                        role_name: "review".to_string(),
                        objective_preview: "review".to_string(),
                        allowed_result_topics: vec!["analysis.done".to_string()],
                        identity_source: ralph_core::IdentitySource::TaskDerived,
                        persistence: ralph_core::RolePersistence::Fixed,
                        contract_schema_version: 1,
                        role_contract_hash: "erc-ddddccccbbbbaaaa".to_string(),
                        source_spawn_request_id: "spawn-1".to_string(),
                    }),
                },
            ],
            failed: Vec::new(),
            parent_topology_unchanged: false,
        };
        recorder.record_bus_event(
            &Event::new(
                ralph_core::TOPIC_TOPOLOGY_SPAWN_RESULT,
                serde_json::to_string(&spawn_result)?,
            )
            .with_reply("evt-spawn")
            .with_source_instance("ralph#1"),
        );

        recorder.record_bus_event(
            &Event::new("analysis.done", r#"{"role":"功能补充","ok":true}"#)
                .with_source_instance("analyst#2"),
        );

        let capability_result = CapabilityParentResultRecord {
            status: "result".to_string(),
            request_id: "cap-req-1".to_string(),
            invocation_id: "cap-inv-1".to_string(),
            capability_id: "workflow:default-parallel".to_string(),
            result_summary: "child summary".to_string(),
            artifacts: CapabilityParentArtifactPaths {
                invoke_json: ".ralph/capability-invocations/cap-inv-1/invoke.json".to_string(),
                result_json: Some(
                    ".ralph/capability-invocations/cap-inv-1/result.json".to_string(),
                ),
                failed_json: None,
                resolved_config: ".ralph/capability-invocations/cap-inv-1/resolved-config.yml"
                    .to_string(),
                events_jsonl: ".ralph/capability-invocations/cap-inv-1/events.jsonl".to_string(),
                evidence_index: ".ralph/evidence-index.jsonl".to_string(),
            },
            parent_topology_unchanged: true,
        };
        recorder.record_bus_event(
            &Event::new(
                ralph_core::TOPIC_CAPABILITY_RESULT,
                serde_json::to_string(&capability_result)?,
            )
            .with_source_instance("ralph#1"),
        );
        recorder.record_meta(Record::meta_termination("CompletionPromise", 4, 12.5, 10));
        recorder.flush().ok();

        let player = load_session_player_strict(&path)?;
        let agg = aggregate_record_session(&player)?;

        assert_eq!(agg.evidence.topology_spawn_groups.len(), 1);
        assert_eq!(
            agg.evidence.topology_spawn_groups[0].request.request_id,
            "spawn-1"
        );
        assert_eq!(agg.evidence.topology_spawn_results.len(), 1);
        assert_eq!(
            agg.evidence.topology_spawn_results[0].result.spawned[1].instance_id,
            "analyst#3"
        );
        assert_eq!(agg.evidence.capability_results.len(), 1);
        assert_eq!(
            agg.evidence.capability_results[0].result.capability_id,
            "workflow:default-parallel"
        );
        assert_eq!(
            agg.evidence
                .result_topics
                .get("analysis.done")
                .map(|topic| topic.count),
            Some(1)
        );
        assert_eq!(
            agg.evidence
                .result_topics
                .get("capability.result")
                .map(|topic| topic.count),
            Some(1)
        );

        let snapshot = AgentsSnapshot {
            generated_at: "2026-05-20T00:00:00Z".to_string(),
            instances: vec![AgentInstanceSnapshot {
                instance_id: "analyst#2".to_string(),
                hat_id: "analyst".to_string(),
                state: HatInstanceState::Idle,
                is_dynamic: true,
                identity_source: ralph_core::IdentitySource::RuntimeAutoscale,
                fixed_role_label: None,
                fixed_role_reason: None,
                role_contract_summary: Some(ralph_core::RoleContractSummary {
                    role_name: "功能补充".to_string(),
                    objective_preview: "补功能".to_string(),
                    allowed_result_topics: vec!["analysis.done".to_string()],
                    identity_source: ralph_core::IdentitySource::TaskDerived,
                    persistence: ralph_core::RolePersistence::Temporary,
                    contract_schema_version: 1,
                    role_contract_hash: "erc-aaaabbbbccccdddd".to_string(),
                    source_spawn_request_id: "spawn-1".to_string(),
                }),
                last_input: Some(AgentLastInput {
                    ts: "2026-05-20T00:00:00Z".to_string(),
                    topic: "analysis.task".to_string(),
                    preview: "repo-grounded task".to_string(),
                }),
                recoverable_failures: vec![
                    ralph_core::AgentRecoverableFailureSummary {
                        failure_id: "failure-scheduled".to_string(),
                        job_id: 7,
                        status: "retry_scheduled".to_string(),
                        failure_kind: "retry_limit_exceeded".to_string(),
                        attempt: 1,
                        max_attempts: 3,
                        retry_after_ms: Some(30_000),
                        next_retry_at: Some("2026-05-20T00:00:30Z".to_string()),
                        ledger_path: ".ralph/recoverable-failures.jsonl".to_string(),
                        stderr_preview: Some(
                            "ERROR: exceeded retry limit, last status: 429 Too Many Requests"
                                .to_string(),
                        ),
                    },
                    ralph_core::AgentRecoverableFailureSummary {
                        failure_id: "failure-continued".to_string(),
                        job_id: 8,
                        status: "continued_by_human".to_string(),
                        failure_kind: "rate_limited".to_string(),
                        attempt: 2,
                        max_attempts: 3,
                        retry_after_ms: Some(0),
                        next_retry_at: None,
                        ledger_path: ".ralph/recoverable-failures.jsonl".to_string(),
                        stderr_preview: Some("manual continue requested by human".to_string()),
                    },
                    ralph_core::AgentRecoverableFailureSummary {
                        failure_id: "failure-exhausted".to_string(),
                        job_id: 9,
                        status: "exhausted".to_string(),
                        failure_kind: "retry_limit_exceeded".to_string(),
                        attempt: 3,
                        max_attempts: 3,
                        retry_after_ms: None,
                        next_retry_at: None,
                        ledger_path: ".ralph/recoverable-failures.jsonl".to_string(),
                        stderr_preview: Some("recoverable attempts exhausted".to_string()),
                    },
                ],
            }],
            completed_dynamic_instances: Vec::new(),
            child_runs: vec![AgentChildRunSnapshot {
                request_id: "cap-req-1".to_string(),
                invocation_id: Some("cap-inv-1".to_string()),
                capability_id: "workflow:default-parallel".to_string(),
                status: AgentChildRunStatus::Done,
                summary: Some("child summary".to_string()),
                artifact: Some(".ralph/capability-invocations/cap-inv-1/result.json".to_string()),
                updated_at: "2026-05-20T00:00:01Z".to_string(),
            }],
        };
        let rendered = render_evidence_inspect(
            &agg,
            AgentsSnapshotInspect::Loaded {
                path: ".ralph/agents.json",
                snapshot: &snapshot,
            },
        )?;

        assert!(rendered.contains("Evidence Inspect"));
        assert!(rendered.contains("topology.spawn_group: 1"));
        assert!(rendered.contains("analyst#2"));
        assert!(rendered.contains("identity_source=task-derived"));
        assert!(rendered.contains("persistence=temporary"));
        assert!(rendered.contains("contract_schema_version=1"));
        assert!(rendered.contains("role_contract_hash=erc-aaaabbbb"));
        assert!(rendered.contains("source_spawn_request_id=spawn-1"));
        assert!(rendered.contains("semantic_source: record-session _meta.termination"));
        assert!(rendered.contains("Dynamic Result Coverage"));
        assert!(
            rendered.contains(
                "request_id=spawn-1 instance=analyst#2 role=功能补充 role_contract_hash=erc-aaaabbbb expected=analysis.done covered=analysis.done missing=-"
            ),
            "covered dynamic result should be explicit: {rendered}"
        );
        assert!(rendered.contains("Recoverable Failures"));
        assert!(rendered.contains("recoverable_failures: 3"));
        assert!(rendered.contains("failure_id=failure-scheduled"));
        assert!(rendered.contains("status=retry_scheduled"));
        assert!(rendered.contains("failure_id=failure-continued"));
        assert!(rendered.contains("status=continued_by_human"));
        assert!(rendered.contains("failure_id=failure-exhausted"));
        assert!(rendered.contains("status=exhausted"));
        assert!(rendered.contains("ledger=.ralph/recoverable-failures.jsonl"));
        assert!(
            rendered.contains(
                "request_id=spawn-1 instance=analyst#3 role=review role_contract_hash=erc-ddddcccc expected=analysis.done covered=- missing=analysis.done"
            ),
            "missing dynamic result coverage should be explicit: {rendered}"
        );
        assert!(rendered.contains("Completed Dynamic Instances"));
        assert!(rendered.contains("completed_dynamic_instances: 0"));
        assert!(rendered.contains("child_runs: 1"));
        assert!(rendered.contains("analysis.done: 1"));
        assert!(rendered.contains("reason: CompletionPromise"));
        Ok(())
    }

    #[test]
    fn evidence_inspect_renders_completed_dynamic_instances_from_agents_snapshot() -> Result<()> {
        let aggregate = RecordSessionAggregate::default();
        let snapshot = AgentsSnapshot {
            generated_at: "2026-05-22T00:00:00Z".to_string(),
            instances: Vec::new(),
            completed_dynamic_instances: vec![AgentCompletedDynamicInstanceSnapshot {
                instance_id: "builder#4".to_string(),
                hat_id: "builder".to_string(),
                final_state: HatInstanceState::Done,
                identity_source: ralph_core::IdentitySource::TaskDerived,
                fixed_role_label: Some("review".to_string()),
                fixed_role_reason: Some("coordinator promoted this role".to_string()),
                role_contract_summary: Some(ralph_core::RoleContractSummary {
                    role_name: "review".to_string(),
                    objective_preview: "review".to_string(),
                    allowed_result_topics: vec!["analysis.done".to_string()],
                    identity_source: ralph_core::IdentitySource::TaskDerived,
                    persistence: ralph_core::RolePersistence::Fixed,
                    contract_schema_version: 1,
                    role_contract_hash: "erc-aaaabbbbccccdddd".to_string(),
                    source_spawn_request_id: "spawn-1".to_string(),
                }),
                last_input: Some(AgentLastInput {
                    ts: "2026-05-22T00:00:00Z".to_string(),
                    topic: "build.task".to_string(),
                    preview: "review task".to_string(),
                }),
                completed_at: "2026-05-22T00:00:01Z".to_string(),
                retirement_reason: "dynamic_instance_unregistered_after_done".to_string(),
            }],
            child_runs: Vec::new(),
        };

        let rendered = render_evidence_inspect(
            &aggregate,
            AgentsSnapshotInspect::Loaded {
                path: ".ralph/agents.json",
                snapshot: &snapshot,
            },
        )?;

        assert!(rendered.contains("Completed Dynamic Instances"));
        assert!(rendered.contains("completed_dynamic_instances: 1"));
        assert!(rendered.contains("builder#4"));
        assert!(rendered.contains("final_state=done"));
        assert!(rendered.contains("fixed_role=review"));
        assert!(rendered.contains("role_contract_hash=erc-aaaabbbb"));
        assert!(rendered.contains("retirement_reason=dynamic_instance_unregistered_after_done"));
        Ok(())
    }

    #[test]
    fn evidence_inspect_missing_termination_does_not_imply_workflow_completion() -> Result<()> {
        let dir = tempfile::TempDir::new()?;
        let path = dir.path().join("session.jsonl");

        let file = std::fs::File::create(&path)?;
        let recorder = SessionRecorder::new(std::io::BufWriter::new(file));

        let spawn_result = TopologySpawnGroupResult {
            status: "spawned".to_string(),
            request_id: "spawn-no-termination".to_string(),
            hat: "builder".to_string(),
            delivery_topic: "build.task".to_string(),
            spawned: vec![TopologySpawnedInstance {
                index: 0,
                instance_id: "builder#2".to_string(),
                role: "analysis".to_string(),
                fixed_role: None,
                role_contract_summary: Some(ralph_core::RoleContractSummary {
                    role_name: "analysis".to_string(),
                    objective_preview: "analyze".to_string(),
                    allowed_result_topics: vec!["analysis.done".to_string()],
                    identity_source: ralph_core::IdentitySource::TaskDerived,
                    persistence: ralph_core::RolePersistence::Temporary,
                    contract_schema_version: 1,
                    role_contract_hash: "erc-missingtermination".to_string(),
                    source_spawn_request_id: "spawn-no-termination".to_string(),
                }),
            }],
            failed: Vec::new(),
            parent_topology_unchanged: false,
        };
        recorder.record_bus_event(&Event::new(
            ralph_core::TOPIC_TOPOLOGY_SPAWN_RESULT,
            serde_json::to_string(&spawn_result)?,
        ));
        recorder.flush().ok();

        let player = load_session_player_strict(&path)?;
        let agg = aggregate_record_session(&player)?;
        let rendered = render_evidence_inspect(
            &agg,
            AgentsSnapshotInspect::Missing {
                searched: Vec::new(),
            },
        )?;

        assert!(rendered.contains("reason: <missing>") || rendered.contains("    <missing>"));
        assert!(rendered.contains("semantic_source: record-session _meta.termination"));
        assert!(
            rendered.contains(
                "semantic_completion: missing; do not infer completion from topology spawn success"
            ),
            "summary must not imply spawn success means workflow completion: {rendered}"
        );
        Ok(())
    }

    #[test]
    fn evidence_inspect_renders_partial_failed_member_phase_evidence() -> Result<()> {
        let aggregate = RecordSessionAggregate {
            evidence: EvidenceInspectAggregate {
                topology_spawn_results: vec![TopologySpawnResultEvidence {
                    record_index: 4,
                    event_id: Some("evt-spawn-result".to_string()),
                    result: TopologySpawnGroupResult {
                        status: "partial".to_string(),
                        request_id: "spawn-partial".to_string(),
                        hat: "builder".to_string(),
                        delivery_topic: "build.task".to_string(),
                        spawned: Vec::new(),
                        failed: vec![
                            TopologySpawnFailedMember::new(
                                1,
                                "review",
                                "role_contract allowed result topics include control-plane topic(s): topology.spawn_group",
                            )
                            .with_request_id("spawn-partial")
                            .with_phase(ralph_core::TOPOLOGY_SPAWN_PHASE_MEMBER_VALIDATION_FAILED)
                            .with_recovery_hint("Fix this role_contract and retry the failed member."),
                        ],
                        parent_topology_unchanged: true,
                    },
                }],
                ..Default::default()
            },
            ..Default::default()
        };

        let rendered = render_evidence_inspect(
            &aggregate,
            AgentsSnapshotInspect::Missing {
                searched: Vec::new(),
            },
        )?;

        assert!(rendered.contains("failed_member index=1 role=review"));
        assert!(rendered.contains("request_id=spawn-partial"));
        assert!(rendered.contains("phase=member_validation_failed"));
        assert!(rendered.contains("recovery_hint=Fix this role_contract"));
        Ok(())
    }
}
