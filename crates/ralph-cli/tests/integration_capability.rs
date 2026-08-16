use anyhow::{Context, Result};
use ralph_core::{EvidenceArtifactKind, EvidenceIndexReader, EvidenceLookup, EvidenceStatus};
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::process::Output;
use std::thread::sleep;
use std::time::{Duration, Instant};
use tempfile::TempDir;

fn write_capability_child_backend(path: &Path) -> Result<()> {
    let script = r#"#!/bin/sh
set -eu
mkdir -p .ralph/dogfood
cat > .ralph/dogfood/capability-child.prompt.txt
test "${RALPH_CAPABILITY_CHILD:-}" = "1"
test "${RALPH_CAPABILITY_MODE:-}" = "execute"
grep -q 'Runtime hat capability invocation' .ralph/dogfood/capability-child.prompt.txt
grep -q 'review this patch' .ralph/dogfood/capability-child.prompt.txt
printf 'focused reviewer executed real child path\n'
printf 'LOOP_COMPLETE\n'
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

fn write_workflow_child_backend(path: &Path) -> Result<()> {
    let script = r##"#!/usr/bin/env python3
import json
import re
import sys
from pathlib import Path

dogfood_dir = Path.cwd() / ".ralph" / "dogfood"
dogfood_dir.mkdir(parents=True, exist_ok=True)


def prompt_identity(prompt_text: str) -> str:
    match = re.search(r'ralph_hat_instance_id:"([^"]+)"', prompt_text)
    return match.group(1) if match else "unknown"


def event_for_prompt(identity: str, prompt_text: str) -> str:
    if identity == "ralph#1":
        if "topic=workflow.complete" in prompt_text:
            return "LOOP_COMPLETE\n"
        return '<event id="wf-dogfood-build-task-1" topic="build.task">workflow dogfood task from ralph</event>\n'
    if identity == "worker#1":
        return '<event id="wf-dogfood-build-done-1" topic="build.done">workflow dogfood worker done</event>\n'
    if identity == "confessor#1":
        return '<event id="wf-dogfood-confession-clean-1" topic="confession.clean">confidence: 95, summary: workflow dogfood clean</event>\n'
    if identity == "confession_handler#1":
        return '<event id="wf-dogfood-workflow-complete-1" topic="workflow.complete">workflow dogfood complete</event>\n'
    return f'unexpected identity {identity}\nLOOP_COMPLETE\n'


def send(obj):
    sys.stdout.write(json.dumps(obj, ensure_ascii=False) + "\n")
    sys.stdout.flush()


def extract_text(value):
    if isinstance(value, dict):
        parts = []
        for key, item in value.items():
            if key == "text" and isinstance(item, str):
                parts.append(item)
            else:
                parts.append(extract_text(item))
        return "".join(parts)
    if isinstance(value, list):
        return "".join(extract_text(item) for item in value)
    return ""


def handle_app_server():
    thread_id = "thread-workflow-dogfood"
    turn_counter = 0
    for raw in sys.stdin:
        line = raw.strip()
        if not line:
            continue
        try:
            msg = json.loads(line)
        except Exception:
            continue
        method = msg.get("method")
        msg_id = msg.get("id")
        if msg_id is None or method is None:
            continue

        if method == "initialize":
            send({"id": msg_id, "result": {}})
            continue
        if method == "thread/start":
            send({"id": msg_id, "result": {}})
            send({"method": "thread/started", "params": {"thread": {"id": thread_id}}})
            continue
        if method == "turn/start":
            turn_counter += 1
            prompt_text = extract_text(msg.get("params", {}).get("input", []))
            identity = prompt_identity(prompt_text)
            safe_identity = identity.replace("#", "_").replace("/", "_")
            (dogfood_dir / f"{safe_identity}-{turn_counter}.prompt.txt").write_text(
                prompt_text,
                encoding="utf-8",
            )
            send({"id": msg_id, "result": {}})
            turn_id = f"turn-{safe_identity}-{turn_counter}"
            send({"method": "turn/started", "params": {"turn": {"id": turn_id}}})
            send({"method": "codex/event/task_started", "params": {"msg": {"turn_id": turn_id}}})
            send({
                "method": "item/agentMessage/delta",
                "params": {"delta": event_for_prompt(identity, prompt_text)},
            })
            send({"method": "turn/completed", "params": {"turn": {"id": turn_id}}})
            continue
        if method in ("turn/interrupt", "turn/steer"):
            send({"id": msg_id, "result": {}})
            continue
        send({"id": msg_id, "result": {}})
    return 0


def handle_plain_exec():
    prompt_text = sys.argv[-1] if len(sys.argv) > 1 else ""
    identity = prompt_identity(prompt_text)
    (dogfood_dir / f"{identity}.prompt.txt").write_text(prompt_text, encoding="utf-8")
    sys.stdout.write(event_for_prompt(identity, prompt_text))
    sys.stdout.flush()
    return 0


if __name__ == "__main__":
    if len(sys.argv) >= 2 and sys.argv[1] == "app-server":
        raise SystemExit(handle_app_server())
    raise SystemExit(handle_plain_exec())
"##;
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

fn run_command_with_timeout(mut command: Command, timeout: Duration) -> Result<Output> {
    let mut child = command
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("failed to spawn command")?;

    let start = Instant::now();
    loop {
        if let Some(_status) = child.try_wait()? {
            return child
                .wait_with_output()
                .context("failed to collect command output");
        }

        if start.elapsed() > timeout {
            let _ = child.kill();
            let output = child
                .wait_with_output()
                .context("failed to collect timed-out command output")?;
            anyhow::bail!(
                "command timed out after {:?}\nstdout:\n{}\nstderr:\n{}",
                timeout,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }

        sleep(Duration::from_millis(50));
    }
}

#[test]
fn tools_capability_list_exposes_lightweight_metadata() -> Result<()> {
    let output = Command::new(env!("CARGO_BIN_EXE_ralph"))
        .args(["tools", "capability", "list", "--json"])
        .output()?;

    assert!(
        output.status.success(),
        "capability list should succeed.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let values: Vec<Value> = serde_json::from_slice(&output.stdout)?;
    assert!(
        values
            .iter()
            .any(|value| value["kind"] == "workflow_capability")
    );
    assert!(values.iter().any(|value| value["kind"] == "hat_capability"));
    assert!(values.iter().all(|value| value.get("summary").is_some()));
    assert!(
        values
            .iter()
            .all(|value| value.get("input_contract").is_some())
    );

    Ok(())
}

#[test]
fn tools_capability_invoke_writes_isolated_artifacts_and_events() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let workspace = temp_dir.path();
    let parent_config = "core: {}\n# parent topology sentinel\n";
    fs::write(workspace.join("ralph.yml"), parent_config)?;

    let output = Command::new(env!("CARGO_BIN_EXE_ralph"))
        .args([
            "tools",
            "capability",
            "invoke",
            "--id",
            "hat:focused-reviewer",
            "--input",
            "review this patch",
            "--preview",
            "--json",
        ])
        .current_dir(workspace)
        .output()?;

    assert!(
        output.status.success(),
        "capability invoke should succeed.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let report: Value = serde_json::from_slice(&output.stdout)?;
    let invocation_id = report["invocation"]["invocation_id"]
        .as_str()
        .expect("invocation id");
    assert_eq!(
        report["invocation"]["capability"]["invocation_mode"],
        "isolated_micro_run"
    );
    assert_eq!(report["invocation"]["parent_topology_unchanged"], true);
    assert_eq!(report["result"]["parent_topology_unchanged"], true);
    assert_eq!(
        report["invocation"]["role_contract"]["identity_source"],
        "task-derived"
    );

    let invocation_dir = workspace
        .join(".ralph/capability-invocations")
        .join(invocation_id);
    assert!(invocation_dir.join("invoke.json").exists());
    assert!(invocation_dir.join("result.json").exists());
    assert!(invocation_dir.join("resolved-config.yml").exists());

    let events = fs::read_to_string(workspace.join(".ralph/events.jsonl"))?;
    assert!(events.contains("capability.invoke"));
    assert!(events.contains("capability.result"));

    let invoke_json: Value =
        serde_json::from_slice(&fs::read(invocation_dir.join("invoke.json"))?)?;
    assert_eq!(
        invoke_json["role_contract"]["identity_source"],
        "task-derived"
    );

    // Phase 3: capability invocation 不能只留下离散文件。
    // evidence index 必须能用 invocation id 把 child/micro-run 证据串起来。
    let evidence_path = workspace.join(".ralph/evidence-index.jsonl");
    let evidence_lookup = EvidenceIndexReader::new(&evidence_path)
        .find_by_correlation(invocation_id)
        .map_err(anyhow::Error::from)?;
    assert!(matches!(evidence_lookup, EvidenceLookup::Entries(_)));
    let evidence_entries = evidence_lookup.entries();

    assert!(
        evidence_entries.iter().any(|entry| {
            entry.artifact_kind == EvidenceArtifactKind::CapabilityInvokeJson
                && entry.status == EvidenceStatus::Success
                && entry.artifact_path.ends_with("invoke.json")
        }),
        "invocation id should find invoke.json evidence: {evidence_entries:#?}"
    );
    assert!(
        evidence_entries.iter().any(|entry| {
            entry.artifact_kind == EvidenceArtifactKind::CapabilityResultJson
                && entry.status == EvidenceStatus::Success
                && entry.artifact_path.ends_with("result.json")
        }),
        "invocation id should find result.json evidence: {evidence_entries:#?}"
    );
    assert!(
        evidence_entries.iter().any(|entry| {
            entry.artifact_kind == EvidenceArtifactKind::ResolvedConfig
                && entry.status == EvidenceStatus::Success
                && entry.artifact_path.ends_with("resolved-config.yml")
        }),
        "invocation id should find resolved config evidence: {evidence_entries:#?}"
    );
    assert!(
        evidence_entries.iter().any(|entry| {
            entry.artifact_kind == EvidenceArtifactKind::EventLogJsonl
                && entry.status == EvidenceStatus::Success
                && entry.artifact_path.ends_with(".ralph/events.jsonl")
        }),
        "invocation id should find event log evidence: {evidence_entries:#?}"
    );
    assert_eq!(
        fs::read_to_string(workspace.join("ralph.yml"))?,
        parent_config
    );

    Ok(())
}

#[test]
fn tools_capability_invoke_records_task_derived_role_contract() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let workspace = temp_dir.path();

    let output = Command::new(env!("CARGO_BIN_EXE_ralph"))
        .args([
            "tools",
            "capability",
            "invoke",
            "--id",
            "hat:focused-reviewer",
            "--input",
            "review this patch",
            "--preview",
            "--json",
        ])
        .current_dir(workspace)
        .output()?;

    assert!(
        output.status.success(),
        "capability invoke should succeed.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let report: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(
        report["invocation"]["role_contract"]["identity_source"],
        "task-derived"
    );
    assert_eq!(
        report["invocation"]["role_contract"]["role_name"],
        "hat:focused-reviewer"
    );
    assert_eq!(
        report["invocation"]["role_contract"]["allowed_topics"]
            .as_array()
            .expect("allowed_topics array")
            .len(),
        2
    );

    let invocation_id = report["invocation"]["invocation_id"]
        .as_str()
        .context("invocation id")?;
    let invocation_dir = workspace
        .join(".ralph/capability-invocations")
        .join(invocation_id);
    let invoke_json: Value =
        serde_json::from_slice(&fs::read(invocation_dir.join("invoke.json"))?)?;
    assert_eq!(
        invoke_json["role_contract"]["identity_source"],
        "task-derived"
    );

    Ok(())
}

#[test]
fn tools_capability_invoke_hat_executes_by_default_and_preview_is_explicit() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let workspace = temp_dir.path();
    let backend_path = workspace.join("capability-child-backend.sh");
    write_capability_child_backend(&backend_path)?;
    fs::write(
        workspace.join("ralph.yml"),
        format!(
            r#"
cli:
  backend: "custom"
  command: "{}"
  prompt_mode: "stdin"
event_loop:
  max_iterations: 2
  max_runtime_seconds: 30
"#,
            backend_path.display()
        )
        .trim_start(),
    )?;

    let output = Command::new(env!("CARGO_BIN_EXE_ralph"))
        .args([
            "tools",
            "capability",
            "invoke",
            "--id",
            "hat:focused-reviewer",
            "--input",
            "review this patch",
            "--json",
        ])
        .current_dir(workspace)
        .output()?;

    assert!(
        output.status.success(),
        "hat capability invoke should execute by default.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let report: Value = serde_json::from_slice(&output.stdout)?;
    assert!(
        report["result"]["stdout_summary"]
            .as_str()
            .is_some_and(|summary| summary.contains("focused reviewer executed real child path")),
        "default invoke should capture real child stdout: {report:#?}"
    );
    assert!(
        report["result"]["result_summary"]
            .as_str()
            .is_some_and(|summary| summary.contains("focused reviewer executed real child path")),
        "parent-visible result summary should include child output: {report:#?}"
    );
    let child_prompt =
        fs::read_to_string(workspace.join(".ralph/dogfood/capability-child.prompt.txt"))?;
    assert!(
        child_prompt.contains("Runtime hat capability invocation"),
        "hat execute should send the task-derived capability prompt to the backend:\n{child_prompt}"
    );
    assert!(
        !child_prompt.contains("You are Ralph."),
        "hat execute must not inject the Ralph coordinator prompt:\n{child_prompt}"
    );

    let invocation_id = report["invocation"]["invocation_id"]
        .as_str()
        .context("invocation id")?;
    let resolved_config_path = workspace
        .join(".ralph/capability-invocations")
        .join(invocation_id)
        .join("resolved-config.yml");
    let resolved_config_raw = fs::read_to_string(&resolved_config_path)
        .with_context(|| resolved_config_path.display().to_string())?;
    let resolved_config: Value = serde_yaml::from_str(&resolved_config_raw)?;
    assert_eq!(
        resolved_config["core"]["runtime_capabilities_enabled"],
        false
    );
    assert_eq!(resolved_config["parallel"]["enabled"], false);
    assert_ne!(
        resolved_config["cli"]["command"].as_str(),
        Some("true"),
        "default execute must not keep the old no-op command=true stub:\n{resolved_config_raw}"
    );

    Ok(())
}

#[test]
fn tools_capability_invoke_materializes_default_parallel_workflow_config() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let workspace = temp_dir.path();
    let input = "请从三个独立视角分析项目演进方向";

    let output = Command::new(env!("CARGO_BIN_EXE_ralph"))
        .args([
            "tools",
            "capability",
            "invoke",
            "--id",
            "workflow:default-parallel",
            "--input",
            input,
            "--preview",
            "--json",
        ])
        .current_dir(workspace)
        .output()?;

    assert!(
        output.status.success(),
        "workflow capability invoke should succeed.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let report: Value = serde_json::from_slice(&output.stdout)?;
    let invocation_id = report["invocation"]["invocation_id"]
        .as_str()
        .context("invocation id")?;
    assert_eq!(
        report["invocation"]["capability"]["invocation_mode"],
        "isolated_child_run"
    );
    assert_eq!(report["invocation"]["parent_topology_unchanged"], true);

    let resolved_config_path = workspace
        .join(".ralph/capability-invocations")
        .join(invocation_id)
        .join("resolved-config.yml");
    let resolved_config_raw = fs::read_to_string(&resolved_config_path)
        .with_context(|| resolved_config_path.display().to_string())?;
    let resolved_config: Value = serde_yaml::from_str(&resolved_config_raw)
        .with_context(|| format!("failed to parse {}", resolved_config_path.display()))?;

    assert_eq!(
        resolved_config["parallel"]["enabled"], true,
        "workflow:default-parallel capability 应物化真实并行配置,实际为:\n{resolved_config_raw}"
    );

    let hats = resolved_config["hats"]
        .as_object()
        .context("resolved workflow capability config should contain hats")?;
    for expected_hat in ["worker", "confessor", "confession_handler"] {
        assert!(
            hats.contains_key(expected_hat),
            "resolved workflow capability config should contain `{expected_hat}`, actual hats: {hats:#?}"
        );
    }

    assert_eq!(resolved_config["event_loop"]["prompt"], input);
    assert_eq!(resolved_config["event_loop"]["prompt_file"], "");
    assert_eq!(
        resolved_config["core"]["runtime_capabilities_enabled"], false,
        "child workflow resolved config 必须关闭 runtime capabilities,避免 nested child 再次收到 capability catalog"
    );
    assert_eq!(
        resolved_config["event_loop"]["complete_publishes"], "workflow.complete",
        "default workflow 必须声明 completion candidate,否则 child workflow 会路由完成但不自然退出"
    );
    assert!(
        resolved_config["hats"]["confession_handler"]["publishes"]
            .as_array()
            .is_some_and(|topics| topics.iter().any(|topic| topic == "workflow.complete")),
        "confession_handler 必须声明发布 completion candidate,实际为:\n{resolved_config_raw}"
    );

    Ok(())
}

#[test]
fn tools_capability_invoke_workflow_execute_records_child_session_dogfood() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let workspace = temp_dir.path();
    let bin_dir = workspace.join("bin");
    fs::create_dir_all(&bin_dir)?;
    let codex_path = bin_dir.join("codex");
    write_workflow_child_backend(&codex_path)?;

    let old_path = std::env::var_os("PATH").unwrap_or_default();
    let new_path = std::env::join_paths(
        std::iter::once(bin_dir.as_os_str().to_owned())
            .chain(std::env::split_paths(&old_path).map(|path| path.into_os_string())),
    )?;

    let mut command = Command::new(env!("CARGO_BIN_EXE_ralph"));
    command
        .args([
            "tools",
            "capability",
            "invoke",
            "--id",
            "workflow:default-parallel",
            "--input",
            "workflow dogfood backend probe",
            "--json",
        ])
        .env("PATH", new_path)
        .current_dir(workspace);

    let output = run_command_with_timeout(command, Duration::from_secs(20))?;
    assert!(
        output.status.success(),
        "workflow capability execute should complete.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let report: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(report["result"]["exit_code"], 0);
    let result_summary = report["result"]["result_summary"]
        .as_str()
        .context("result summary")?;
    assert!(
        result_summary.contains("termination=CompletionPromise")
            || result_summary.contains("termination=WorkflowCompletionEvent"),
        "workflow result summary should report completion termination, got: {result_summary}"
    );
    assert!(
        result_summary.contains("workflow.complete"),
        "workflow result summary should include completion topic, got: {result_summary}"
    );
    assert!(
        !result_summary.contains("BEGIN PROMPT"),
        "workflow result summary should not contain prompt echo, got: {result_summary}"
    );
    let invocation_id = report["invocation"]["invocation_id"]
        .as_str()
        .context("invocation id")?;
    let invocation_dir = workspace
        .join(".ralph/capability-invocations")
        .join(invocation_id);
    let child_record_session_path = invocation_dir.join("child-record-session.jsonl");
    assert!(
        child_record_session_path.exists(),
        "workflow execute should leave child record-session at {}",
        child_record_session_path.display()
    );

    let child_record_session = fs::read_to_string(&child_record_session_path)?;
    // Termination reason:
    // - 旧路径: ralph#1 输出 LOOP_COMPLETE 字符串 → CompletionPromise
    // - 新路径(fix/completion-via-event): `complete_publishes` topic 触发 WorkflowCompletionEvent
    // 任一皆可,只要不是 MaxRuntime / Interrupted。
    let has_completion_termination = child_record_session.contains("CompletionPromise")
        || child_record_session.contains("WorkflowCompletionEvent");
    assert!(
        has_completion_termination,
        "child record-session should contain a completion termination reason (CompletionPromise or WorkflowCompletionEvent):\n{child_record_session}"
    );
    assert!(
        child_record_session.contains("_meta.termination"),
        "child record-session should contain `_meta.termination`:\n{child_record_session}"
    );
    for needle in [
        "build.task",
        "build.done",
        "confession.clean",
        "workflow.complete",
    ] {
        assert!(
            child_record_session.contains(needle),
            "child record-session should contain `{needle}`:\n{child_record_session}"
        );
    }

    let evidence_lookup = EvidenceIndexReader::new(workspace.join(".ralph/evidence-index.jsonl"))
        .find_by_correlation(invocation_id)
        .map_err(anyhow::Error::from)?;
    let evidence_entries = evidence_lookup.entries();
    assert!(
        evidence_entries.iter().any(|entry| {
            entry.artifact_kind == EvidenceArtifactKind::RecordSessionJsonl
                && entry.status == EvidenceStatus::Success
                && entry.artifact_path.ends_with("child-record-session.jsonl")
        }),
        "invocation id should find child record-session evidence: {evidence_entries:#?}"
    );

    Ok(())
}

#[test]
fn tools_capability_inspect_reports_invocation_evidence() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let workspace = temp_dir.path();
    fs::write(
        workspace.join("ralph.yml"),
        "core: {}\n# parent topology sentinel\n",
    )?;

    let invoke_output = Command::new(env!("CARGO_BIN_EXE_ralph"))
        .args([
            "tools",
            "capability",
            "invoke",
            "--id",
            "hat:focused-reviewer",
            "--input",
            "review this patch",
            "--preview",
            "--json",
        ])
        .current_dir(workspace)
        .output()?;

    assert!(
        invoke_output.status.success(),
        "capability invoke should succeed.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&invoke_output.stdout),
        String::from_utf8_lossy(&invoke_output.stderr)
    );

    let report: Value = serde_json::from_slice(&invoke_output.stdout)?;
    let invocation_id = report["invocation"]["invocation_id"]
        .as_str()
        .expect("invocation id");

    // Phase 3.1: inspect 是面向人和 agent 的 lookup UX。
    // 它不能重新解释 artifact,只应该把 evidence index 中的链接稳定展示出来。
    let inspect_json_output = Command::new(env!("CARGO_BIN_EXE_ralph"))
        .args(["tools", "capability", "inspect", invocation_id, "--json"])
        .current_dir(workspace)
        .output()?;

    assert!(
        inspect_json_output.status.success(),
        "capability inspect --json should succeed.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&inspect_json_output.stdout),
        String::from_utf8_lossy(&inspect_json_output.stderr)
    );

    let inspect_report: Value = serde_json::from_slice(&inspect_json_output.stdout)?;
    assert_eq!(inspect_report["invocation_id"], invocation_id);
    assert_eq!(inspect_report["status"], "entries");
    assert!(
        inspect_report["index_path"]
            .as_str()
            .expect("index path")
            .ends_with(".ralph/evidence-index.jsonl")
    );
    let entries = inspect_report["entries"].as_array().expect("entries array");
    assert!(entries.iter().any(|entry| {
        entry["artifact_kind"] == "capability_invoke_json"
            && entry["artifact_path"]
                .as_str()
                .is_some_and(|path| path.ends_with("invoke.json"))
    }));
    assert!(entries.iter().any(|entry| {
        entry["artifact_kind"] == "capability_result_json"
            && entry["artifact_path"]
                .as_str()
                .is_some_and(|path| path.ends_with("result.json"))
    }));
    assert!(entries.iter().any(|entry| {
        entry["artifact_kind"] == "resolved_config"
            && entry["artifact_path"]
                .as_str()
                .is_some_and(|path| path.ends_with("resolved-config.yml"))
    }));
    assert!(entries.iter().any(|entry| {
        entry["artifact_kind"] == "event_log_jsonl"
            && entry["artifact_path"]
                .as_str()
                .is_some_and(|path| path.ends_with(".ralph/events.jsonl"))
    }));

    let inspect_human_output = Command::new(env!("CARGO_BIN_EXE_ralph"))
        .args(["tools", "capability", "inspect", invocation_id])
        .current_dir(workspace)
        .output()?;

    assert!(
        inspect_human_output.status.success(),
        "capability inspect should succeed.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&inspect_human_output.stdout),
        String::from_utf8_lossy(&inspect_human_output.stderr)
    );
    let human_stdout = String::from_utf8_lossy(&inspect_human_output.stdout);
    assert!(human_stdout.contains(invocation_id));
    assert!(human_stdout.contains("capability_invoke_json"));
    assert!(human_stdout.contains(".ralph/capability-invocations"));
    assert!(human_stdout.contains(".ralph/events.jsonl"));

    Ok(())
}

#[test]
fn tools_capability_inspect_fails_for_unknown_invocation_id() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let workspace = temp_dir.path();
    fs::create_dir_all(workspace.join(".ralph"))?;
    fs::write(workspace.join(".ralph/evidence-index.jsonl"), "")?;

    let output = Command::new(env!("CARGO_BIN_EXE_ralph"))
        .args([
            "tools",
            "capability",
            "inspect",
            "missing-invocation-id",
            "--json",
        ])
        .current_dir(workspace)
        .output()?;

    assert!(
        !output.status.success(),
        "unknown invocation id should fail.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("missing-invocation-id"));
    assert!(stderr.contains(".ralph/evidence-index.jsonl"));

    Ok(())
}
