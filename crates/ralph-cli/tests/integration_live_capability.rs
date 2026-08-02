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
    // 第一轮 `ralph#1` 先检查 stdin prompt 中是否存在 capability catalog selection UX,
    // 再选择 `hat:focused-reviewer` 并发出结构化请求。
    // 第二轮它收敛退出。证据链由 event log / artifacts / inspect 断言。
    // ─────────────────────────────────────────────────────────────────────
    let script = r#"#!/bin/sh
set -eu
mkdir -p .ralph/dogfood
if [ "${RALPH_CAPABILITY_CHILD:-}" = "1" ]; then
  cat > .ralph/dogfood/capability-child.prompt.txt
  printf 'focused reviewer child executed
'
  printf 'LOOP_COMPLETE
'
  exit 0
fi
instance="${RALPH_HAT_INSTANCE_ID:-unknown}"
case "$instance" in
  ralph#1)
    prompt_capture=".ralph/dogfood/ralph-prompt.txt"
    cat > "$prompt_capture"
    count_file=".ralph/dogfood/ralph.count"
    count=0
    if [ -f "$count_file" ]; then
      count=$(cat "$count_file")
    fi
    count=$((count + 1))
    printf '%s\n' "$count" > "$count_file"
    if [ "$count" -eq 1 ]; then
      grep -q '## Runtime Capability Catalog' "$prompt_capture"
      grep -q 'capability.request' "$prompt_capture"
      grep -q 'request_id' "$prompt_capture"
      grep -q 'capability_id' "$prompt_capture"
      grep -q 'hat:focused-reviewer' "$prompt_capture"
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

fn write_human_reply_backend_script(path: &Path) -> Result<()> {
    // ─────────────────────────────────────────────────────────────────────
    // 这个脚本覆盖下一条产品线:
    // - 第一轮 `ralph#1` 触发 live capability invocation。
    // - 第二轮它观察到 parent-visible `capability.result` 后,
    //   再显式发 `reply.human.message` 给人看。
    // ─────────────────────────────────────────────────────────────────────
    let script = r#"#!/bin/sh
set -eu
mkdir -p .ralph/dogfood
if [ "${RALPH_CAPABILITY_CHILD:-}" = "1" ]; then
  cat > .ralph/dogfood/capability-child.prompt.txt
  printf 'focused reviewer child executed
'
  printf 'LOOP_COMPLETE
'
  exit 0
fi
instance="${RALPH_HAT_INSTANCE_ID:-unknown}"
case "$instance" in
  ralph#1)
    count_file=".ralph/dogfood/ralph-human-reply.count"
    count=0
    if [ -f "$count_file" ]; then
      count=$(cat "$count_file")
    fi
    count=$((count + 1))
    printf '%s\n' "$count" > "$count_file"
    if [ "$count" -eq 1 ]; then
      prompt_capture=".ralph/dogfood/ralph-human-reply-turn1.prompt.txt"
      cat > "$prompt_capture"
      grep -q '## Runtime Capability Catalog' "$prompt_capture"
      grep -q 'hat:focused-reviewer' "$prompt_capture"
      printf '<event id="cap-human-req-event-1" topic="capability.request">{"request_id":"cap-human-req-1","capability_id":"hat:focused-reviewer","input":"review this patch for the human"}</event>\n'
    else
      prompt_capture=".ralph/dogfood/ralph-human-reply-turn2.prompt.txt"
      cat > "$prompt_capture"
      grep -q 'capability.result' "$prompt_capture"
      grep -q 'cap-human-req-1' "$prompt_capture"
      grep -q 'hat:focused-reviewer' "$prompt_capture"
      printf '<event id="cap-human-reply-1" topic="reply.human.message">final human answer: focused review completed</event>\n'
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

fn write_multi_step_backend_script(path: &Path) -> Result<()> {
    // ─────────────────────────────────────────────────────────────────────
    // 这条脚本覆盖方向 B:
    // - turn1 发 capability request A
    // - turn2 看到 result A 后发 capability request B
    // - turn3 看到 result B 后显式发最终 `reply.human.message`
    // ─────────────────────────────────────────────────────────────────────
    let script = r#"#!/bin/sh
set -eu
mkdir -p .ralph/dogfood
if [ "${RALPH_CAPABILITY_CHILD:-}" = "1" ]; then
  cat > .ralph/dogfood/capability-child.prompt.txt
  printf 'focused reviewer child executed
'
  printf 'LOOP_COMPLETE
'
  exit 0
fi
instance="${RALPH_HAT_INSTANCE_ID:-unknown}"
case "$instance" in
  ralph#1)
    count_file=".ralph/dogfood/ralph-multi-step.count"
    count=0
    if [ -f "$count_file" ]; then
      count=$(cat "$count_file")
    fi
    count=$((count + 1))
    printf '%s\n' "$count" > "$count_file"
    prompt_capture=".ralph/dogfood/ralph-multi-step-turn-${count}.prompt.txt"
    cat > "$prompt_capture"
    case "$count" in
      1)
        grep -q '## Runtime Capability Catalog' "$prompt_capture"
        grep -q 'hat:focused-reviewer' "$prompt_capture"
        printf '<event id="cap-multi-req-event-1" topic="capability.request">{"request_id":"cap-multi-step-1","capability_id":"hat:focused-reviewer","input":"step one review"}</event>\n'
        ;;
      2)
        grep -q 'capability.result' "$prompt_capture"
        grep -q 'cap-multi-step-1' "$prompt_capture"
        printf '<event id="cap-multi-req-event-2" topic="capability.request">{"request_id":"cap-multi-step-2","capability_id":"hat:focused-reviewer","input":"step two review after step one"}</event>\n'
        ;;
      3)
        grep -q 'capability.result' "$prompt_capture"
        grep -q 'cap-multi-step-2' "$prompt_capture"
        printf '<event id="cap-multi-human-reply-1" topic="reply.human.message">final human answer: two capability steps completed</event>\n'
        printf 'LOOP_COMPLETE\n'
        ;;
      *)
        printf 'LOOP_COMPLETE\n'
        ;;
    esac
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

fn write_failure_fallback_backend_script(path: &Path) -> Result<()> {
    // ─────────────────────────────────────────────────────────────────────
    // 这条脚本覆盖方向 B.2:
    // - turn1 发一个无效 capability request,强制得到 parent-visible failure
    // - turn2 看到 `capability.failed` 后发 fallback 有效 request
    // - turn3 看到 fallback `capability.result` 后显式发最终 `reply.human.message`
    // ─────────────────────────────────────────────────────────────────────
    let script = r#"#!/bin/sh
set -eu
mkdir -p .ralph/dogfood
if [ "${RALPH_CAPABILITY_CHILD:-}" = "1" ]; then
  cat > .ralph/dogfood/capability-child.prompt.txt
  printf 'focused reviewer child executed
'
  printf 'LOOP_COMPLETE
'
  exit 0
fi
instance="${RALPH_HAT_INSTANCE_ID:-unknown}"
case "$instance" in
  ralph#1)
    count_file=".ralph/dogfood/ralph-failure-fallback.count"
    count=0
    if [ -f "$count_file" ]; then
      count=$(cat "$count_file")
    fi
    count=$((count + 1))
    printf '%s\n' "$count" > "$count_file"
    prompt_capture=".ralph/dogfood/ralph-failure-fallback-turn-${count}.prompt.txt"
    cat > "$prompt_capture"
    case "$count" in
      1)
        grep -q '## Runtime Capability Catalog' "$prompt_capture"
        printf '<event id="cap-fallback-req-event-1" topic="capability.request">{"request_id":"cap-fallback-step-1","capability_id":"hat:missing-reviewer","input":"this request should fail first"}</event>\n'
        ;;
      2)
        grep -q 'capability.failed' "$prompt_capture"
        grep -q 'invalid_capability_id' "$prompt_capture"
        grep -q 'cap-fallback-step-1' "$prompt_capture"
        grep -q 'hat:missing-reviewer' "$prompt_capture"
        printf '<event id="cap-fallback-req-event-2" topic="capability.request">{"request_id":"cap-fallback-step-2","capability_id":"hat:focused-reviewer","input":"fallback review after failure"}</event>\n'
        ;;
      3)
        grep -q 'capability.result' "$prompt_capture"
        grep -q 'cap-fallback-step-2' "$prompt_capture"
        printf '<event id="cap-fallback-human-reply-1" topic="reply.human.message">final human answer: fallback capability recovered after failure</event>\n'
        printf 'LOOP_COMPLETE\n'
        ;;
      *)
        printf 'LOOP_COMPLETE\n'
        ;;
    esac
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

fn write_malformed_request_diagnostic_backend_script(path: &Path) -> Result<()> {
    // ─────────────────────────────────────────────────────────────────────
    // 这条脚本覆盖方向 B.4:
    // - turn1 发一个结构错误的 capability.request,缺少 capability_id。
    // - turn2 看到 `failure_class=malformed_request` 后,不重试、不 fallback,
    //   而是显式发 diagnostic `reply.human.message`。
    // ─────────────────────────────────────────────────────────────────────
    let script = r#"#!/bin/sh
set -eu
mkdir -p .ralph/dogfood
if [ "${RALPH_CAPABILITY_CHILD:-}" = "1" ]; then
  cat > .ralph/dogfood/capability-child.prompt.txt
  printf 'focused reviewer child executed
'
  printf 'LOOP_COMPLETE
'
  exit 0
fi
instance="${RALPH_HAT_INSTANCE_ID:-unknown}"
case "$instance" in
  ralph#1)
    count_file=".ralph/dogfood/ralph-malformed-diagnostic.count"
    count=0
    if [ -f "$count_file" ]; then
      count=$(cat "$count_file")
    fi
    count=$((count + 1))
    printf '%s\n' "$count" > "$count_file"
    prompt_capture=".ralph/dogfood/ralph-malformed-diagnostic-turn-${count}.prompt.txt"
    cat > "$prompt_capture"
    case "$count" in
      1)
        grep -q '## Runtime Capability Catalog' "$prompt_capture"
        printf '<event id="cap-malformed-req-event-1" topic="capability.request">{"request_id":"cap-malformed-step-1","input":"missing capability_id should become diagnostic"}</event>\n'
        ;;
      2)
        grep -q 'capability.failed' "$prompt_capture"
        grep -q 'malformed_request' "$prompt_capture"
        grep -q 'cap-malformed-step-1' "$prompt_capture"
        printf '<event id="cap-malformed-human-reply-1" topic="reply.human.message">final human answer: malformed capability request diagnostic without retry</event>\n'
        printf 'LOOP_COMPLETE\n'
        ;;
      *)
        printf 'LOOP_COMPLETE\n'
        ;;
    esac
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

    let captured_prompt = fs::read_to_string(workspace.join(".ralph/dogfood/ralph-prompt.txt"))?;
    assert!(
        captured_prompt.contains("## Runtime Capability Catalog"),
        "ralph#1 prompt should include capability catalog: {captured_prompt}"
    );
    assert!(
        captured_prompt.contains("capability.request")
            && captured_prompt.contains("request_id")
            && captured_prompt.contains("capability_id")
            && captured_prompt.contains("input"),
        "ralph#1 prompt should include capability.request contract: {captured_prompt}"
    );
    assert!(
        captured_prompt.contains("hat:focused-reviewer"),
        "ralph#1 prompt should include focused reviewer capability id: {captured_prompt}"
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

#[test]
fn parallel_capability_result_can_become_explicit_human_reply() -> Result<()> {
    // ─────────────────────────────────────────────────────────────────────
    // 这条 gate 把两条已经存在的产品线串起来:
    // - Phase 4: parent run 触发 isolated capability invocation。
    // - B.1: human-visible answer 仍必须靠显式 `reply.human.message`。
    // ─────────────────────────────────────────────────────────────────────
    let temp_dir = TempDir::new()?;
    let workspace = temp_dir.path();
    let script_path = workspace.join("live-capability-human-reply-backend.sh");
    let config_path = workspace.join("ralph.yml");
    let record_path = workspace.join("session.jsonl");

    write_prompt(workspace)?;
    write_human_reply_backend_script(&script_path)?;
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
        "capability-result -> human-reply dogfood run should succeed.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("final human answer: focused review completed"),
        "CLI output should expose the explicit human-facing reply: {stdout}"
    );

    let records = read_event_records(workspace)?;
    assert!(
        records
            .iter()
            .any(|record| record.topic == "capability.request"),
        "event log should preserve capability.request: {records:#?}"
    );
    assert!(
        records
            .iter()
            .any(|record| record.topic == "reply.human.message"),
        "event log should preserve explicit reply.human.message: {records:#?}"
    );

    let result_payload = records
        .iter()
        .filter(|record| record.topic == "capability.result")
        .filter_map(|record| payload_json(record).ok())
        .find(|payload| payload["request_id"] == "cap-human-req-1")
        .context(
            "parent event log should contain parent-visible capability.result for cap-human-req-1",
        )?;
    let invocation_id = result_payload["invocation_id"]
        .as_str()
        .context("capability.result should include invocation_id")?;
    assert_eq!(result_payload["capability_id"], "hat:focused-reviewer");
    assert_eq!(result_payload["parent_topology_unchanged"], true);

    let events = fs::read_to_string(workspace.join(".ralph/events.jsonl"))?;
    assert!(
        events.contains("\"topic\":\"capability.result\"")
            && events.contains("\"topic\":\"reply.human.message\""),
        "events.jsonl should preserve capability.result and explicit human reply separately: {events}"
    );

    let inspect_output = Command::new(env!("CARGO_BIN_EXE_ralph"))
        .args(["tools", "capability", "inspect", invocation_id, "--json"])
        .current_dir(workspace)
        .output()?;
    assert!(
        inspect_output.status.success(),
        "capability inspect should still succeed for invocation id.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&inspect_output.stdout),
        String::from_utf8_lossy(&inspect_output.stderr)
    );
    let inspect_report: Value = serde_json::from_slice(&inspect_output.stdout)?;
    assert_eq!(inspect_report["invocation_id"], invocation_id);
    assert_eq!(inspect_report["status"], "entries");

    let record_session = fs::read_to_string(&record_path)?;
    assert!(
        record_session.contains("reply.human.message")
            && record_session.contains("_meta.termination"),
        "record-session should preserve explicit human-facing reply publication and termination: {record_session}"
    );

    assert_eq!(fs::read_to_string(&config_path)?, config_before);

    Ok(())
}

#[test]
fn parallel_parent_run_can_orchestrate_multiple_capability_results_before_final_human_reply()
-> Result<()> {
    // ─────────────────────────────────────────────────────────────────────
    // 方向 B gate:
    // - 同一个 parent run 连续做两步 capability invocation。
    // - 每一步都等上一步 result 进入 parent context 后再继续。
    // - 最终仍然只靠显式 `reply.human.message` 对人输出答案。
    // ─────────────────────────────────────────────────────────────────────
    let temp_dir = TempDir::new()?;
    let workspace = temp_dir.path();
    let script_path = workspace.join("live-capability-multi-step-backend.sh");
    let config_path = workspace.join("ralph.yml");
    let record_path = workspace.join("session.jsonl");

    write_prompt(workspace)?;
    write_multi_step_backend_script(&script_path)?;
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
        "multi-step capability orchestration run should succeed.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("final human answer: two capability steps completed"),
        "CLI output should only expose the final human-facing payload after both capability steps: {stdout}"
    );

    let records = read_event_records(workspace)?;
    let request_count = records
        .iter()
        .filter(|record| record.topic == "capability.request")
        .count();
    assert_eq!(
        request_count, 2,
        "event log should contain exactly two capability requests: {records:#?}"
    );

    let result_payload_1 = records
        .iter()
        .filter(|record| record.topic == "capability.result")
        .filter_map(|record| payload_json(record).ok())
        .find(|payload| payload["request_id"] == "cap-multi-step-1")
        .context("missing capability.result for cap-multi-step-1")?;
    let result_payload_2 = records
        .iter()
        .filter(|record| record.topic == "capability.result")
        .filter_map(|record| payload_json(record).ok())
        .find(|payload| payload["request_id"] == "cap-multi-step-2")
        .context("missing capability.result for cap-multi-step-2")?;

    let invocation_id_1 = result_payload_1["invocation_id"]
        .as_str()
        .context("capability.result for step1 should include invocation_id")?;
    let invocation_id_2 = result_payload_2["invocation_id"]
        .as_str()
        .context("capability.result for step2 should include invocation_id")?;
    assert_ne!(
        invocation_id_1, invocation_id_2,
        "each capability step should produce a distinct invocation id"
    );

    let events = fs::read_to_string(workspace.join(".ralph/events.jsonl"))?;
    assert!(
        events.contains("cap-multi-step-1")
            && events.contains("cap-multi-step-2")
            && events.contains("\"topic\":\"reply.human.message\""),
        "events.jsonl should preserve both capability steps and the final explicit human reply: {events}"
    );

    for invocation_id in [invocation_id_1, invocation_id_2] {
        let inspect_output = Command::new(env!("CARGO_BIN_EXE_ralph"))
            .args(["tools", "capability", "inspect", invocation_id, "--json"])
            .current_dir(workspace)
            .output()?;
        assert!(
            inspect_output.status.success(),
            "capability inspect should succeed for {invocation_id}.\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&inspect_output.stdout),
            String::from_utf8_lossy(&inspect_output.stderr)
        );
        let inspect_report: Value = serde_json::from_slice(&inspect_output.stdout)?;
        assert_eq!(inspect_report["invocation_id"], invocation_id);
        assert_eq!(inspect_report["status"], "entries");
    }

    let record_session = fs::read_to_string(&record_path)?;
    assert!(
        record_session.contains("reply.human.message")
            && record_session.contains("_meta.termination"),
        "record-session should preserve the final explicit human-facing reply and termination: {record_session}"
    );

    assert_eq!(fs::read_to_string(&config_path)?, config_before);

    Ok(())
}

#[test]
fn parallel_parent_run_can_fallback_after_capability_failed_before_final_human_reply() -> Result<()>
{
    // ─────────────────────────────────────────────────────────────────────
    // 方向 B.2 gate:
    // - 第一步 capability request 故意失败。
    // - parent `ralph#1` 在看到 `capability.failed` 后继续发 fallback request。
    // - 最终仍然只靠显式 `reply.human.message` 对人输出答案。
    // ─────────────────────────────────────────────────────────────────────
    let temp_dir = TempDir::new()?;
    let workspace = temp_dir.path();
    let script_path = workspace.join("live-capability-failure-fallback-backend.sh");
    let config_path = workspace.join("ralph.yml");
    let record_path = workspace.join("session.jsonl");

    write_prompt(workspace)?;
    write_failure_fallback_backend_script(&script_path)?;
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
        "failure fallback dogfood run should succeed.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("final human answer: fallback capability recovered after failure"),
        "CLI output should expose only the final explicit human-facing payload after fallback recovery: {stdout}"
    );

    let records = read_event_records(workspace)?;
    let failed_payload = records
        .iter()
        .filter(|record| record.topic == "capability.failed")
        .filter_map(|record| payload_json(record).ok())
        .find(|payload| payload["request_id"] == "cap-fallback-step-1")
        .context("missing capability.failed for cap-fallback-step-1")?;
    assert_eq!(failed_payload["status"], "failed");
    assert_eq!(failed_payload["failure_class"], "invalid_capability_id");
    assert_eq!(failed_payload["capability_id"], "hat:missing-reviewer");
    assert_eq!(failed_payload["parent_topology_unchanged"], true);
    assert!(
        failed_payload["invocation_id"].is_null(),
        "invalid capability id path should fail before creating an invocation id: {failed_payload:#?}"
    );

    let fallback_result = records
        .iter()
        .filter(|record| record.topic == "capability.result")
        .filter_map(|record| payload_json(record).ok())
        .find(|payload| payload["request_id"] == "cap-fallback-step-2")
        .context("missing fallback capability.result for cap-fallback-step-2")?;
    let fallback_invocation_id = fallback_result["invocation_id"]
        .as_str()
        .context("fallback capability.result should include invocation_id")?;
    assert_eq!(fallback_result["parent_topology_unchanged"], true);

    let events = fs::read_to_string(workspace.join(".ralph/events.jsonl"))?;
    assert!(
        events.contains("\"topic\":\"capability.failed\"")
            && events.contains("cap-fallback-step-2")
            && events.contains("\"topic\":\"reply.human.message\""),
        "events.jsonl should preserve failure, fallback success, and final explicit human reply separately: {events}"
    );

    let inspect_output = Command::new(env!("CARGO_BIN_EXE_ralph"))
        .args([
            "tools",
            "capability",
            "inspect",
            fallback_invocation_id,
            "--json",
        ])
        .current_dir(workspace)
        .output()?;
    assert!(
        inspect_output.status.success(),
        "capability inspect should succeed for fallback invocation {fallback_invocation_id}.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&inspect_output.stdout),
        String::from_utf8_lossy(&inspect_output.stderr)
    );
    let inspect_report: Value = serde_json::from_slice(&inspect_output.stdout)?;
    assert_eq!(inspect_report["invocation_id"], fallback_invocation_id);
    assert_eq!(inspect_report["status"], "entries");

    let record_session = fs::read_to_string(&record_path)?;
    assert!(
        record_session.contains("reply.human.message")
            && record_session.contains("fallback capability recovered after failure")
            && record_session.contains("_meta.termination"),
        "record-session should preserve the final human-facing reply after failure fallback: {record_session}"
    );

    assert_eq!(fs::read_to_string(&config_path)?, config_before);

    Ok(())
}

#[test]
fn parallel_parent_run_can_emit_diagnostic_reply_for_malformed_capability_request_without_retry()
-> Result<()> {
    // ─────────────────────────────────────────────────────────────────────
    // 方向 B.4 gate:
    // - `malformed_request` 不是 recoverable fallback 分支。
    // - parent 看到结构化 class 后,显式发 diagnostic human reply。
    // - 运行时不需要 retry engine,也不需要 fallback capability.result。
    // ─────────────────────────────────────────────────────────────────────
    let temp_dir = TempDir::new()?;
    let workspace = temp_dir.path();
    let script_path = workspace.join("live-capability-malformed-diagnostic-backend.sh");
    let config_path = workspace.join("ralph.yml");
    let record_path = workspace.join("session.jsonl");

    write_prompt(workspace)?;
    write_malformed_request_diagnostic_backend_script(&script_path)?;
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
        "malformed-request diagnostic branch should succeed.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout
            .contains("final human answer: malformed capability request diagnostic without retry"),
        "CLI output should expose the explicit diagnostic human-facing reply: {stdout}"
    );

    let records = read_event_records(workspace)?;
    let request_count = records
        .iter()
        .filter(|record| record.topic == "capability.request")
        .count();
    assert_eq!(
        request_count, 1,
        "malformed branch should not emit fallback capability requests: {records:#?}"
    );

    let failed_payload = records
        .iter()
        .filter(|record| record.topic == "capability.failed")
        .filter_map(|record| payload_json(record).ok())
        .find(|payload| payload["request_id"] == "cap-malformed-step-1")
        .context("missing capability.failed for cap-malformed-step-1")?;
    assert_eq!(failed_payload["status"], "failed");
    assert_eq!(failed_payload["failure_class"], "malformed_request");
    assert!(
        failed_payload["capability_id"].is_null(),
        "malformed request missing capability_id should not invent one: {failed_payload:#?}"
    );
    assert!(
        failed_payload["invocation_id"].is_null(),
        "malformed request should fail before creating an invocation id: {failed_payload:#?}"
    );
    assert_eq!(failed_payload["parent_topology_unchanged"], true);

    assert!(
        !records
            .iter()
            .any(|record| record.topic == "capability.result"),
        "malformed diagnostic branch should not require fallback capability.result: {records:#?}"
    );
    assert!(
        records
            .iter()
            .any(|record| record.topic == "reply.human.message"),
        "diagnostic branch must still emit explicit reply.human.message: {records:#?}"
    );

    let events = fs::read_to_string(workspace.join(".ralph/events.jsonl"))?;
    assert!(
        events.contains("\"topic\":\"capability.failed\"")
            && events.contains("malformed_request")
            && events.contains("\"topic\":\"reply.human.message\"")
            && !events.contains("\"topic\":\"capability.result\""),
        "events.jsonl should preserve malformed failure and diagnostic reply without fallback result: {events}"
    );

    let invocation_root = workspace.join(".ralph/capability-invocations");
    assert!(
        !invocation_root.exists(),
        "malformed request should not create invocation artifacts: {}",
        invocation_root.display()
    );

    let record_session = fs::read_to_string(&record_path)?;
    assert!(
        record_session.contains("reply.human.message")
            && record_session.contains("malformed capability request diagnostic without retry")
            && record_session.contains("_meta.termination"),
        "record-session should preserve diagnostic human reply and termination: {record_session}"
    );

    assert_eq!(fs::read_to_string(&config_path)?, config_before);

    Ok(())
}
