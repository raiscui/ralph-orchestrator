use anyhow::{Context, Result};
use ralph_core::{EventRecord, EvidenceIndexReader, EvidenceLookup};
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn write_prompt(workspace: &Path) -> Result<()> {
    fs::write(workspace.join("PROMPT.md"), "live capability dogfood\n")?;
    Ok(())
}

fn write_backend_script(path: &Path) -> Result<()> {
    // ─────────────────────────────────────────────────────────────────────
    // 这个脚本模拟真实 parent coordinator。
    // 第一轮 `ralph#1` 选择 capability 并发出结构化请求。
    // 第二轮它收敛退出。证据链由 event log / artifacts / inspect 断言。
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
    if [ "$count" -eq 1 ]; then
      printf '<event id="cap-req-event-1" topic="capability.request">{"request_id":"cap-req-dogfood-1","capability_id":"hat:focused-reviewer","input":"review this patch"}</event>\n'
    else
      printf 'LOOP_COMPLETE\n'
    fi
    ;;
  *)
    printf 'LOOP_COMPLETE\n'
    ;;
esac
"#;
    fs::write(path, script)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms)?;
    }
    Ok(())
}

fn write_config(path: &Path, script_path: &Path) -> Result<()> {
    // ─────────────────────────────────────────────────────────────────────
    // 只启用 parallel runtime,不配置任何业务 hat。
    // 这样测试目标更窄: 真正的 `ralph#1` parent run 触发 capability invocation。
    // ─────────────────────────────────────────────────────────────────────
    let content = format!(
        r#"
event_loop:
  prompt_file: "PROMPT.md"
  completion_promise: "LOOP_COMPLETE"
  max_iterations: 4
  max_runtime_seconds: 30

cli:
  backend: "custom"
  command: "{}"
  prompt_mode: "stdin"

parallel:
  enabled: true
  autoscale:
    max_running_jobs: 1
    dynamic_idle_ttl_secs: 30
"#,
        script_path.display()
    );
    fs::write(path, content.trim_start())?;
    Ok(())
}

fn read_event_records(workspace: &Path) -> Result<Vec<EventRecord>> {
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

#[test]
fn parallel_parent_run_triggers_live_capability_invocation_and_inspect_evidence() -> Result<()> {
    // ─────────────────────────────────────────────────────────────────────
    // 这是 Phase 4 dogfood:
    // - 真实 `ralph run` + parallel supervisor
    // - `ralph#1` 发出 capability.request
    // - runtime 走 isolated micro-run 写 artifacts/evidence
    // - parent event log 收到带 request_id/invocation_id 的 result event
    // - Phase 3.1 inspect 能查询同一 invocation id
    // ─────────────────────────────────────────────────────────────────────
    let temp_dir = TempDir::new()?;
    let workspace = temp_dir.path();
    let script_path = workspace.join("live-capability-backend.sh");
    let config_path = workspace.join("ralph.yml");
    let record_path = workspace.join("session.jsonl");

    write_prompt(workspace)?;
    write_backend_script(&script_path)?;
    write_config(&config_path, &script_path)?;
    let config_before = fs::read_to_string(&config_path)?;

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
        "live capability dogfood run should succeed.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let records = read_event_records(workspace)?;
    assert!(
        records
            .iter()
            .any(|record| record.topic == "capability.request"),
        "parent event log should contain capability.request: {records:#?}"
    );

    let result_payload = records
        .iter()
        .filter(|record| record.topic == "capability.result")
        .filter_map(|record| payload_json(record).ok())
        .find(|payload| payload["request_id"] == "cap-req-dogfood-1")
        .context("parent event log should contain parent-return capability.result")?;
    assert_eq!(result_payload["request_id"], "cap-req-dogfood-1");
    assert_eq!(result_payload["capability_id"], "hat:focused-reviewer");
    assert_eq!(result_payload["parent_topology_unchanged"], true);
    let invocation_id = result_payload["invocation_id"]
        .as_str()
        .context("capability.result should include invocation_id")?;

    let invocation_dir = workspace
        .join(".ralph/capability-invocations")
        .join(invocation_id);
    assert!(invocation_dir.join("invoke.json").exists());
    assert!(invocation_dir.join("result.json").exists());
    assert!(invocation_dir.join("resolved-config.yml").exists());

    let evidence_path = workspace.join(".ralph/evidence-index.jsonl");
    let evidence_lookup = EvidenceIndexReader::new(&evidence_path)
        .find_by_correlation(invocation_id)
        .with_context(|| evidence_path.display().to_string())?;
    assert!(matches!(evidence_lookup, EvidenceLookup::Entries(_)));

    let inspect_output = Command::new(env!("CARGO_BIN_EXE_ralph"))
        .args(["tools", "capability", "inspect", invocation_id, "--json"])
        .current_dir(workspace)
        .output()?;
    assert!(
        inspect_output.status.success(),
        "capability inspect should succeed.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&inspect_output.stdout),
        String::from_utf8_lossy(&inspect_output.stderr)
    );
    let inspect_report: Value = serde_json::from_slice(&inspect_output.stdout)?;
    assert_eq!(inspect_report["invocation_id"], invocation_id);
    assert_eq!(inspect_report["status"], "entries");

    assert_eq!(fs::read_to_string(&config_path)?, config_before);

    let record_session = fs::read_to_string(&record_path)?;
    assert!(
        record_session.contains("_meta.termination"),
        "record-session should capture termination: {record_session}"
    );

    Ok(())
}
