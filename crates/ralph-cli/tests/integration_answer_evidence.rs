use anyhow::{Context, Result};
use ralph_core::{EvidenceArtifactKind, EvidenceIndexReader, EvidenceLookup, EvidenceStatus};
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn write_prompt(workspace: &Path) -> Result<()> {
    fs::write(workspace.join("PROMPT.md"), "answer evidence dogfood\n")?;
    Ok(())
}

fn write_backend_script(path: &Path) -> Result<()> {
    // ─────────────────────────────────────────────────────────────────────
    // 这个脚本故意保持极小。
    // 它用 `RALPH_HAT_INSTANCE_ID` 区分主协调者和 researcher hat,
    // 让测试覆盖真实 parallel runtime 的 requester-return 链路。
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
      printf '<event id="req-dogfood-1" topic="research.request" target="researcher">summarize runtime answer evidence</event>\n'
    else
      printf 'LOOP_COMPLETE\n'
    fi
    ;;
  researcher#1)
    printf '<event id="ans-dogfood-1" topic="reply.hat.message" reply="req-dogfood-1">answer evidence is indexed</event>\n'
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
    // 测试配置只打开 parallel runtime 和一个 researcher hat。
    // 单一真相源仍然是 `.ralph/events.jsonl`;
    // evidence index 只负责把 request id / answer id 指回持久化证据。
    // ─────────────────────────────────────────────────────────────────────
    let content = format!(
        r#"
event_loop:
  prompt_file: "PROMPT.md"
  completion_promise: "LOOP_COMPLETE"
  max_iterations: 5
  max_runtime_seconds: 30

cli:
  backend: "custom"
  command: "{}"
  prompt_mode: "stdin"

parallel:
  enabled: true
  autoscale:
    max_running_jobs: 2
    dynamic_idle_ttl_secs: 30

hats:
  researcher:
    name: "Researcher"
    description: "Returns answer evidence to requester."
    triggers: ["research.request"]
    publishes: ["reply.hat.message"]
    instructions: "Return reply.hat.message to requester."
"#,
        script_path.display()
    );
    fs::write(path, content.trim_start())?;
    Ok(())
}

#[test]
fn parallel_run_dogfoods_answer_return_evidence_index() -> Result<()> {
    // ─────────────────────────────────────────────────────────────────────
    // 用真实 CLI 运行一轮 Ralph,而不是直接调用 routing helper。
    // 这样可以同时验证 record-session、events.jsonl 和 evidence-index
    // 三个 runtime 产物是否在同一条链路里闭合。
    // ─────────────────────────────────────────────────────────────────────
    let temp_dir = TempDir::new()?;
    let temp_path = temp_dir.path();
    let script_path = temp_path.join("dogfood-backend.sh");
    let config_path = temp_path.join("ralph.yml");
    let record_path = temp_path.join("session.jsonl");

    write_prompt(temp_path)?;
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
        .current_dir(temp_path)
        .output()?;

    assert!(
        output.status.success(),
        "parallel dogfood run should succeed.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let evidence_path = temp_path.join(".ralph/evidence-index.jsonl");
    assert!(
        evidence_path.exists(),
        "evidence index should be written: {}",
        evidence_path.display()
    );

    // request id 应该能查到 answer 事件和 requester-return delivery 记录。
    // 这里不复制 payload,只验证索引能把人带回 JSONL 真相源。
    let request_lookup = EvidenceIndexReader::new(&evidence_path)
        .find_by_correlation("req-dogfood-1")
        .with_context(|| evidence_path.display().to_string())?;
    let request_entries = request_lookup.entries();
    assert!(matches!(request_lookup, EvidenceLookup::Entries(_)));
    assert!(
        request_entries.iter().any(|entry| {
            entry.artifact_kind == EvidenceArtifactKind::ReplyEvent
                && entry.status == EvidenceStatus::Success
                && entry.child_correlation_id.as_deref() == Some("ans-dogfood-1")
                && entry.artifact_path.ends_with(".ralph/events.jsonl")
        }),
        "request id should resolve to reply event evidence: {request_entries:#?}"
    );
    assert!(
        request_entries.iter().any(|entry| {
            entry.artifact_kind == EvidenceArtifactKind::RuntimeDeliveryRecord
                && entry.status == EvidenceStatus::Success
                && entry.child_correlation_id.as_deref() == Some("ans-dogfood-1")
        }),
        "request id should resolve to runtime delivery evidence: {request_entries:#?}"
    );

    // answer id 也要能反查 request id。
    // 这是后续排查"哪个回答对应哪个请求"时最有用的一跳。
    let answer_lookup = EvidenceIndexReader::new(&evidence_path)
        .find_by_correlation("ans-dogfood-1")
        .with_context(|| evidence_path.display().to_string())?;
    let answer_entries = answer_lookup.entries();
    assert!(matches!(answer_lookup, EvidenceLookup::Entries(_)));
    assert!(
        answer_entries.iter().any(|entry| {
            entry.artifact_kind == EvidenceArtifactKind::EventLogJsonl
                && entry.status == EvidenceStatus::Success
                && entry.parent_correlation_id.as_deref() == Some("req-dogfood-1")
                && entry.artifact_path.ends_with(".ralph/events.jsonl")
        }),
        "answer event id should resolve back to event-log evidence: {answer_entries:#?}"
    );

    // event log 仍是 durable truth。
    // evidence index 不能替代它,只能提供可查询入口。
    let events = fs::read_to_string(temp_path.join(".ralph/events.jsonl"))?;
    assert!(
        events.contains("routing.requester_return") && events.contains("delivered"),
        "event log should contain delivered requester-return record: {events}"
    );

    // record-session 用来证明这是一条完整 runtime run,不是孤立单元测试。
    let record = fs::read_to_string(&record_path)?;
    assert!(
        record.contains("_meta.termination") && record.contains("CompletionPromise"),
        "record-session should capture completion termination: {record}"
    );

    Ok(())
}
