use anyhow::{Context, Result};
use ralph_core::{
    AgentsSnapshot, EventRecord, EvidenceArtifactKind, EvidenceIndexReader, EvidenceLookup,
    EvidenceStatus,
};
use ralph_proto::{RuntimeDeliveryRecord, RuntimeLifecycleRecord};
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn write_prompt(workspace: &Path) -> Result<()> {
    // 这里的 prompt 只负责启动 parent coordinator。
    // 真正的 spawn 请求由测试 backend 在 ralph#1 第一轮输出。
    fs::write(
        workspace.join("PROMPT.md"),
        "topology spawn integration dogfood\n",
    )?;
    Ok(())
}

fn write_backend_script(path: &Path) -> Result<()> {
    // ─────────────────────────────────────────────────────────────────────
    // 这个 backend 把父级 topology mutation 和 worker 返回拆开:
    // - ralph#1 第 1 轮发 `topology.spawn_group`。
    // - 新创建的 builder#2/#3/#4 分别返回 `analysis.done`。
    // - ralph#1 第 2 轮等待 worker 结束后发 completion promise。
    // ─────────────────────────────────────────────────────────────────────
    let script = r#"#!/bin/sh
set -eu
mkdir -p .ralph/dogfood
instance="${RALPH_HAT_INSTANCE_ID:-unknown}"
case "$instance" in
  ralph#1)
    count_file=".ralph/dogfood/ralph.count"
    count=0
    if [ -f "$count_file" ]; then
      count=$(cat "$count_file")
    fi
    count=$((count + 1))
    printf '%s\n' "$count" > "$count_file"
    prompt_capture=".ralph/dogfood/ralph-turn-${count}.prompt.txt"
    cat > "$prompt_capture"
    if [ "$count" -eq 1 ]; then
      grep -q 'topology.spawn_group' "$prompt_capture"
      printf '<event id="spawn-group-dogfood-1" topic="topology.spawn_group">{"request_id":"spawn-dogfood-1","hat":"builder","delivery_topic":"build.task","instances":[{"role":"功能补充","task":"补充 feature A","role_contract":{"role_name":"功能补充","objective":"补充 feature A","input_contract":"Handle build.task for feature supplementation.","output_contract":"Publish analysis.done with feature supplementation findings.","allowed_topics":["analysis.done"],"forbidden_responsibilities":["Do not coordinate globally"],"success_criteria":["analysis.done published"],"identity_source":"task-derived"}},{"role":"功能完善","task":"完善 feature B","role_contract":{"role_name":"功能完善","objective":"完善 feature B","input_contract":"Handle build.task for feature refinement.","output_contract":"Publish analysis.done with feature refinement findings.","allowed_topics":["analysis.done"],"forbidden_responsibilities":["Do not coordinate globally"],"success_criteria":["analysis.done published"],"identity_source":"task-derived"}},{"role":"review","task":"review the proposal","fixed_role":true,"role_contract":{"role_name":"review","objective":"review the proposal","input_contract":"Handle build.task for review.","output_contract":"Publish analysis.done with review findings.","allowed_topics":["analysis.done"],"forbidden_responsibilities":["Do not coordinate globally"],"success_criteria":["analysis.done published"],"identity_source":"task-derived"}}]}</event>\n'
    else
      sleep 1
      printf 'LOOP_COMPLETE\n'
    fi
    ;;
  builder#*)
    safe_id=$(printf '%s' "$instance" | sed 's/[^A-Za-z0-9_-]/-/g')
    prompt_capture=".ralph/dogfood/${safe_id}.prompt.txt"
    cat > "$prompt_capture"
    grep -q 'topology_request_id' "$prompt_capture"
    grep -q 'spawn-dogfood-1' "$prompt_capture"
    grep -q '### ROLE CONTRACT' "$prompt_capture"
    grep -q 'identity_source: task-derived' "$prompt_capture"
    grep -q 'source_spawn_request_id: spawn-dogfood-1' "$prompt_capture"
    grep -q 'Allowed result topics:' "$prompt_capture"
    grep -q 'analysis.done' "$prompt_capture"
    printf '<event id="done-%s" topic="analysis.done">{"instance":"%s","summary":"analysis done"}</event>\n' "$safe_id" "$instance"
    ;;
  *)
    cat >/dev/null
    printf 'LOOP_COMPLETE\n'
    ;;
esac
"#;

    fs::write(path, script)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        // custom backend 走真实子进程执行,脚本必须可执行。
        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms)?;
    }
    Ok(())
}

fn write_config(path: &Path, script_path: &Path) -> Result<()> {
    // ─────────────────────────────────────────────────────────────────────
    // 这份配置保留一个静态 builder#1。
    // guardrail 要证明 spawn_group 不会把三个 runtime role 折叠回 builder#1。
    // ─────────────────────────────────────────────────────────────────────
    let content = format!(
        r#"
event_loop:
  prompt_file: "PROMPT.md"
  completion_promise: "LOOP_COMPLETE"
  max_iterations: 6
  max_runtime_seconds: 30

cli:
  backend: "custom"
  command: "{}"
  prompt_mode: "stdin"

parallel:
  enabled: true
  autoscale:
    max_running_jobs: 4
    dynamic_idle_ttl_secs: 30

hats:
  builder:
    name: "Builder"
    description: "Executes a parent-visible runtime topology task."
    triggers: ["build.task"]
    publishes: ["analysis.done"]
    instructions: "Return analysis.done after finishing the assigned runtime task."
"#,
        script_path.display()
    );
    fs::write(path, content.trim_start())?;
    Ok(())
}

fn read_event_records(workspace: &Path) -> Result<Vec<EventRecord>> {
    // `.ralph/events.jsonl` 是 runtime durable stream。
    // 这里按 JSONL 严格解析,避免只靠 stdout 文本判断行为。
    let events = fs::read_to_string(workspace.join(".ralph/events.jsonl"))?;
    events
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str::<EventRecord>(line).map_err(anyhow::Error::from))
        .collect()
}

fn payload_json(record: &EventRecord) -> Result<Value> {
    serde_json::from_str::<Value>(&record.payload).map_err(anyhow::Error::from)
}

fn runtime_deliveries(records: &[EventRecord]) -> Result<Vec<RuntimeDeliveryRecord>> {
    records
        .iter()
        .filter(|record| record.topic == ralph_proto::TOPIC_RUNTIME_DELIVERY)
        .map(|record| {
            serde_json::from_str::<RuntimeDeliveryRecord>(&record.payload)
                .map_err(anyhow::Error::from)
        })
        .collect()
}

fn runtime_lifecycles(records: &[EventRecord]) -> Result<Vec<RuntimeLifecycleRecord>> {
    records
        .iter()
        .filter(|record| record.topic == ralph_proto::TOPIC_RUNTIME_LIFECYCLE)
        .map(|record| {
            serde_json::from_str::<RuntimeLifecycleRecord>(&record.payload)
                .map_err(anyhow::Error::from)
        })
        .collect()
}

#[test]
fn parallel_parent_visible_spawn_materializes_dynamic_agents_without_redelivery() -> Result<()> {
    // ─────────────────────────────────────────────────────────────────────
    // 这条 guardrail 故意跨真实 CLI 边界:
    // - parent 输出 `topology.spawn_group`
    // - runtime 落 `.ralph/events.jsonl` 和 `.ralph/agents.json`
    // - record summary 可以回看同一次 run 的拓扑证据
    // ─────────────────────────────────────────────────────────────────────
    let temp_dir = TempDir::new()?;
    let workspace = temp_dir.path();
    let script_path = workspace.join("topology-spawn-backend.sh");
    let config_path = workspace.join("ralph.yml");
    let record_path = workspace.join("session.jsonl");

    write_prompt(workspace)?;
    write_backend_script(&script_path)?;
    write_config(&config_path, &script_path)?;

    let output = Command::new(env!("CARGO_BIN_EXE_ralph"))
        .args([
            "run",
            "--config",
            config_path.to_string_lossy().as_ref(),
            "--no-tui",
            "--record-session",
            record_path.to_string_lossy().as_ref(),
        ])
        .current_dir(workspace)
        .output()?;
    assert!(
        output.status.success(),
        "topology spawn dogfood run should succeed.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // stdout 是即时显示层。
    // 它应该直接说明 parent topology 已经变更,但 durable 断言仍在下方文件证据里。
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("[supervisor:event] topology.spawn.result")
            && stdout.contains("parent_topology_unchanged=false")
            && stdout.contains("builder#2:功能补充")
            && stdout.contains("identity_source=task-derived")
            && stdout.contains("persistence=temporary")
            && stdout.contains("contract_schema_version=1")
            && stdout.contains("source_spawn_request_id=spawn-dogfood-1"),
        "plain output should expose spawned parent-visible instances: {stdout}"
    );

    let records = read_event_records(workspace)?;
    let spawn_result_position = records
        .iter()
        .position(|record| record.topic == "topology.spawn.result")
        .context("events stream should contain topology.spawn.result")?;

    // 原始 build.task 只允许由 spawn_group 产生三条 member delivery。
    // 如果收到 spawn result 后又 replay 原始任务,这里会立刻多出后置 build.task。
    let build_task_records = records
        .iter()
        .enumerate()
        .filter(|(_, record)| record.topic == "build.task")
        .collect::<Vec<_>>();
    assert_eq!(
        build_task_records.len(),
        3,
        "spawn_group should only log the three direct member deliveries: {records:#?}"
    );
    assert!(
        build_task_records
            .iter()
            .all(|(index, _)| *index < spawn_result_position),
        "topology.spawn.result must not cause post-ack build.task redelivery: {records:#?}"
    );

    let result_payload = records
        .iter()
        .filter(|record| record.topic == "topology.spawn.result")
        .filter_map(|record| payload_json(record).ok())
        .find(|payload| payload["request_id"] == "spawn-dogfood-1")
        .context("spawn result should carry the topology request id")?;
    assert_eq!(result_payload["status"], "spawned");
    assert_eq!(result_payload["parent_topology_unchanged"], false);
    assert_eq!(
        result_payload["spawned"]
            .as_array()
            .context("spawned should be an array")?
            .len(),
        3
    );
    let spawned = result_payload["spawned"]
        .as_array()
        .context("spawned should be an array")?;
    for item in spawned {
        let summary = &item["role_contract_summary"];
        assert_eq!(summary["identity_source"], "task-derived");
        assert_eq!(
            summary["allowed_result_topics"],
            serde_json::json!(["analysis.done"])
        );
        assert_eq!(summary["contract_schema_version"], 1);
        assert!(
            summary["role_contract_hash"]
                .as_str()
                .is_some_and(|hash| hash.starts_with("erc-")),
            "spawned item should carry a role contract hash: {item}"
        );
        assert_eq!(summary["source_spawn_request_id"], "spawn-dogfood-1");
    }

    let deliveries = runtime_deliveries(&records)?;
    let build_recipients = deliveries
        .iter()
        .filter(|delivery| delivery.topic == "build.task")
        .map(|delivery| delivery.recipient.to_string())
        .collect::<Vec<_>>();
    assert_eq!(
        build_recipients,
        vec![
            "builder#2".to_string(),
            "builder#3".to_string(),
            "builder#4".to_string()
        ],
        "spawn_group should deliver only to new dynamic builder instances"
    );
    assert!(
        deliveries.iter().any(|delivery| {
            delivery.topic == "topology.spawn.result" && delivery.recipient.as_str() == "ralph#1"
        }),
        "spawn acknowledgement should return to the parent coordinator: {deliveries:#?}"
    );

    let lifecycles = runtime_lifecycles(&records)?;
    let dynamic_spawns = lifecycles
        .iter()
        .filter(|record| {
            record.kind == ralph_proto::RuntimeLifecycleKind::Spawn
                && record.dynamic
                && record.instance_id.as_str().starts_with("builder#")
        })
        .map(|record| record.instance_id.to_string())
        .collect::<Vec<_>>();
    assert_eq!(
        dynamic_spawns,
        vec![
            "builder#2".to_string(),
            "builder#3".to_string(),
            "builder#4".to_string()
        ],
        "runtime lifecycle should preserve the three parent-visible dynamic spawns"
    );

    let snapshot_path = workspace.join(".ralph/agents.json");
    let snapshot: AgentsSnapshot = serde_json::from_str(&fs::read_to_string(&snapshot_path)?)
        .with_context(|| snapshot_path.display().to_string())?;
    let dynamic_agents = snapshot
        .instances
        .iter()
        .filter(|instance| instance.hat_id == "builder" && instance.is_dynamic)
        .map(|instance| instance.instance_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        dynamic_agents,
        vec!["builder#2", "builder#3", "builder#4"],
        "agents sidecar must expose parent-visible dynamic builder instances"
    );

    let fixed_review = snapshot
        .instances
        .iter()
        .find(|instance| instance.instance_id == "builder#4")
        .context("builder#4 should exist in agents snapshot")?;
    assert_eq!(fixed_review.fixed_role_label.as_deref(), Some("review"));
    assert_eq!(
        fixed_review.identity_source,
        ralph_core::IdentitySource::TaskDerived
    );
    let fixed_summary = fixed_review
        .role_contract_summary
        .as_ref()
        .context("fixed review should have a role contract summary")?;
    assert_eq!(
        fixed_summary.persistence,
        ralph_core::RolePersistence::Fixed
    );
    assert_eq!(
        fixed_summary.identity_source,
        ralph_core::IdentitySource::TaskDerived
    );
    assert!(
        snapshot
            .instances
            .iter()
            .filter(|instance| matches!(instance.instance_id.as_str(), "builder#2" | "builder#3"))
            .all(|instance| instance.fixed_role_label.is_none()),
        "temporary topology roles should not become fixed agents roles"
    );
    for instance in snapshot
        .instances
        .iter()
        .filter(|instance| instance.hat_id == "builder" && instance.is_dynamic)
    {
        assert_eq!(
            instance.identity_source,
            ralph_core::IdentitySource::TaskDerived
        );
        let summary = instance.role_contract_summary.as_ref().with_context(|| {
            format!("{} should have role contract summary", instance.instance_id)
        })?;
        assert_eq!(
            summary.allowed_result_topics,
            vec!["analysis.done".to_string()]
        );
        assert!(summary.role_contract_hash.starts_with("erc-"));
        assert_eq!(summary.source_spawn_request_id, "spawn-dogfood-1");
        if instance.instance_id == "builder#4" {
            assert_eq!(summary.persistence, ralph_core::RolePersistence::Fixed);
        } else {
            assert_eq!(summary.persistence, ralph_core::RolePersistence::Temporary);
        }
    }
    let snapshot_json = fs::read_to_string(&snapshot_path)?;
    assert!(snapshot_json.contains("\"role_contract_summary\""));
    assert!(snapshot_json.contains("\"role_contract_hash\""));
    assert!(snapshot_json.contains("\"source_spawn_request_id\""));
    assert!(
        !snapshot_json.contains("input_contract")
            && !snapshot_json.contains("output_contract")
            && !snapshot_json.contains("forbidden_responsibilities")
            && !snapshot_json.contains("### ROLE CONTRACT"),
        "agents snapshot must stay summary-only: {snapshot_json}"
    );

    // 三个 worker 必须真的跑完并发出结果,否则这个测试只证明了 create 没证明执行。
    let mut analysis_done_sources = records
        .iter()
        .filter(|record| record.topic == "analysis.done")
        .filter_map(|record| record.source_instance.as_deref())
        .collect::<Vec<_>>();
    analysis_done_sources.sort_unstable();
    assert_eq!(
        analysis_done_sources,
        vec!["builder#2", "builder#3", "builder#4"],
        "spawned builders should each publish analysis.done"
    );

    let summary_output = Command::new(env!("CARGO_BIN_EXE_ralph"))
        .args([
            "record",
            "summary",
            record_path.to_string_lossy().as_ref(),
            "--agents-file",
            snapshot_path.to_string_lossy().as_ref(),
        ])
        .current_dir(workspace)
        .output()?;
    assert!(
        summary_output.status.success(),
        "record summary should inspect topology evidence.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&summary_output.stdout),
        String::from_utf8_lossy(&summary_output.stderr)
    );
    let summary = String::from_utf8_lossy(&summary_output.stdout);
    assert!(
        summary.contains("Evidence Inspect")
            && summary.contains("topology.spawn_group: 1")
            && summary.contains("builder#2")
            && summary.contains("identity_source=task-derived")
            && summary.contains("persistence=temporary")
            && summary.contains("persistence=fixed")
            && summary.contains("contract_schema_version=1")
            && summary.contains("role_contract_hash=erc-")
            && summary.contains("source_spawn_request_id=spawn-dogfood-1")
            && summary.contains("analysis.done: 3")
            && summary.contains("reason: CompletionPromise"),
        "record summary should replay topology, result and termination evidence: {summary}"
    );

    let evidence_path = workspace.join(".ralph/evidence-index.jsonl");
    assert!(
        evidence_path.exists(),
        "topology spawn dogfood should write evidence index: {}",
        evidence_path.display()
    );

    // evidence-index 只做关联索引:
    // - request id 能列出动态 child instances。
    // - role contract hash 能指回 event log / agents snapshot。
    // - result_topic 是轻量 correlation metadata,不是 result payload。
    let spawn_lookup = EvidenceIndexReader::new(&evidence_path)
        .find_by_correlation("spawn-dogfood-1")
        .with_context(|| evidence_path.display().to_string())?;
    let spawn_entries = spawn_lookup.entries();
    assert!(matches!(spawn_lookup, EvidenceLookup::Entries(_)));
    for instance_id in ["builder#2", "builder#3", "builder#4"] {
        assert!(
            spawn_entries.iter().any(|entry| {
                entry.status == EvidenceStatus::Success
                    && entry.child_correlation_id.as_deref() == Some(instance_id)
                    && entry.artifact_path.ends_with(".ralph/events.jsonl")
            }),
            "spawn request id should resolve child instance {instance_id}: {spawn_entries:#?}"
        );
    }

    let first_role_hash = spawned[0]["role_contract_summary"]["role_contract_hash"]
        .as_str()
        .context("spawned role should have role_contract_hash")?;
    let role_lookup = EvidenceIndexReader::new(&evidence_path)
        .find_by_correlation(first_role_hash)
        .with_context(|| evidence_path.display().to_string())?;
    let role_entries = role_lookup.entries();
    assert!(matches!(role_lookup, EvidenceLookup::Entries(_)));
    assert!(
        role_entries.iter().any(|entry| {
            entry.artifact_kind == EvidenceArtifactKind::EventLogJsonl
                && entry.parent_correlation_id.as_deref() == Some("spawn-dogfood-1")
                && entry.child_correlation_id.as_deref() == Some("builder#2")
                && entry.result_topic.as_deref() == Some("analysis.done")
                && entry.artifact_path.ends_with(".ralph/events.jsonl")
        }),
        "role hash should resolve to event log artifact: {role_entries:#?}"
    );
    assert!(
        role_entries.iter().any(|entry| {
            entry.artifact_kind == EvidenceArtifactKind::AgentsSnapshotJson
                && entry.parent_correlation_id.as_deref() == Some("spawn-dogfood-1")
                && entry.child_correlation_id.as_deref() == Some("builder#2")
                && entry.result_topic.as_deref() == Some("analysis.done")
                && entry.artifact_path.ends_with(".ralph/agents.json")
        }),
        "role hash should resolve to agents snapshot artifact: {role_entries:#?}"
    );
    let evidence_index_jsonl = fs::read_to_string(&evidence_path)?;
    assert!(
        !evidence_index_jsonl.contains("input_contract")
            && !evidence_index_jsonl.contains("output_contract")
            && !evidence_index_jsonl.contains("forbidden_responsibilities"),
        "evidence index must not duplicate full role contract: {evidence_index_jsonl}"
    );

    Ok(())
}
