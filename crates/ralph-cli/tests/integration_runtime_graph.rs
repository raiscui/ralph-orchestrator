use anyhow::{Context, Result};
use ralph_core::EventLogger;
use ralph_proto::{
    Event, HatInstanceId, HatInstanceState, RuntimeDeliveryKind, RuntimeDeliveryRecord,
    RuntimeLifecycleKind, RuntimeLifecycleRecord,
};
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn write_prompt(workspace: &Path) -> Result<()> {
    fs::write(workspace.join("PROMPT.md"), "runtime graph test\n")?;
    Ok(())
}

fn write_serial_config(path: &Path) -> Result<()> {
    let content = r#"
event_loop:
  prompt_file: "PROMPT.md"
  completion_promise: "LOOP_COMPLETE"
  max_iterations: 1
  max_runtime_seconds: 30

cli:
  backend: "custom"
  command: "true"
"#;
    fs::write(path, content.trim_start())?;
    Ok(())
}

fn write_parallel_runtime_graph_config(path: &Path) -> Result<()> {
    let content = r#"
event_loop:
  prompt_file: "PROMPT.md"
  completion_promise: "LOOP_COMPLETE"
  max_iterations: 5
  max_runtime_seconds: 30

cli:
  backend: "custom"
  command: "sh"
  prompt_mode: "stdin"
  args:
    - "-c"
    - "printf 'LOOP_COMPLETE\n'"

parallel:
  enabled: true
"#;
    fs::write(path, content.trim_start())?;
    Ok(())
}

fn record_has_termination_reason(path: &Path, expected_reason: &str) -> Result<bool> {
    let content = fs::read_to_string(path).with_context(|| path.display().to_string())?;
    for (i, line) in content.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }

        let value: serde_json::Value = serde_json::from_str(line)
            .with_context(|| format!("Invalid JSON at line {}: {}", i + 1, line))?;
        if value.get("event").and_then(|v| v.as_str()) != Some("_meta.termination") {
            continue;
        }

        if value
            .get("data")
            .and_then(|data| data.get("reason"))
            .and_then(|reason| reason.as_str())
            == Some(expected_reason)
        {
            return Ok(true);
        }
    }

    Ok(false)
}

fn write_runtime_graph_replay_events(path: &Path) -> Result<()> {
    let mut logger = EventLogger::new(path);
    let event = Event::new("build.task", "do work")
        .with_id("event-1")
        .with_source_instance(HatInstanceId::new("planner#1"));
    logger.log_event(0, "planner", &event, None)?;

    let delivery = RuntimeDeliveryRecord::new(
        Some("event-1".to_string()),
        None,
        "build.task",
        Some(HatInstanceId::new("planner#1")),
        HatInstanceId::new("writer#1"),
        RuntimeDeliveryKind::Queue,
    );
    logger.log_runtime_delivery(0, "supervisor", &delivery)?;

    for lifecycle in [
        RuntimeLifecycleRecord::new(HatInstanceId::new("writer#1"), RuntimeLifecycleKind::Create)
            .with_state(HatInstanceState::Created),
        RuntimeLifecycleRecord::new(
            HatInstanceId::new("writer#1"),
            RuntimeLifecycleKind::Shutdown,
        )
        .with_reason("supervisor_shutdown"),
    ] {
        logger.log_runtime_lifecycle(0, "supervisor", &lifecycle)?;
    }

    Ok(())
}

#[test]
fn runtime_graph_rrd_requires_parallel_mode() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let temp_path = temp_dir.path();
    let config_path = temp_path.join("ralph.yml");
    let runtime_graph_path = temp_path.join("runtime.rrd");

    write_serial_config(&config_path)?;
    write_prompt(temp_path)?;

    let output = Command::new(env!("CARGO_BIN_EXE_ralph"))
        .args([
            "run",
            "--config",
            config_path.to_string_lossy().as_ref(),
            "--no-tui",
            "--runtime-graph-rrd",
            runtime_graph_path.to_string_lossy().as_ref(),
        ])
        .current_dir(temp_path)
        .output()?;

    assert!(
        !output.status.success(),
        "serial mode should reject --runtime-graph-rrd"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("`--runtime-graph-rrd` requires `parallel.enabled=true` in config."),
        "stderr should explain the parallel-only guard, got: {stderr}"
    );

    Ok(())
}

#[test]
fn runtime_graph_rrd_writes_non_empty_artifact_for_parallel_run() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let temp_path = temp_dir.path();
    let config_path = temp_path.join("ralph.yml");
    let record_path = temp_path.join("session.jsonl");
    let runtime_graph_path = temp_path.join("runtime.rrd");

    write_parallel_runtime_graph_config(&config_path)?;
    write_prompt(temp_path)?;

    let output = Command::new(env!("CARGO_BIN_EXE_ralph"))
        .args([
            "run",
            "--config",
            config_path.to_string_lossy().as_ref(),
            "--no-tui",
            "--record-session",
            record_path.to_string_lossy().as_ref(),
            "--runtime-graph-rrd",
            runtime_graph_path.to_string_lossy().as_ref(),
        ])
        .current_dir(temp_path)
        .output()?;

    assert!(
        output.status.success(),
        "parallel run should succeed.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let metadata = fs::metadata(&runtime_graph_path)
        .with_context(|| runtime_graph_path.display().to_string())?;
    assert!(
        metadata.len() > 0,
        "runtime graph artifact should be non-empty: {}",
        runtime_graph_path.display()
    );

    assert!(
        record_has_termination_reason(&record_path, "CompletionPromise")?,
        "record-session should capture CompletionPromise termination"
    );

    Ok(())
}

#[test]
fn runtime_graph_replay_writes_non_empty_artifact_from_durable_events() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let temp_path = temp_dir.path();
    let events_path = temp_path.join("events.jsonl");
    let runtime_graph_path = temp_path.join("runtime-replay.rrd");

    write_runtime_graph_replay_events(&events_path)?;

    let output = Command::new(env!("CARGO_BIN_EXE_ralph"))
        .args([
            "runtime-graph",
            "replay",
            "--events",
            events_path.to_string_lossy().as_ref(),
            "--output",
            runtime_graph_path.to_string_lossy().as_ref(),
            "--format",
            "json",
        ])
        .current_dir(temp_path)
        .output()?;

    assert!(
        output.status.success(),
        "runtime graph replay should succeed.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let metadata = fs::metadata(&runtime_graph_path)
        .with_context(|| runtime_graph_path.display().to_string())?;
    assert!(
        metadata.len() > 0,
        "runtime graph replay artifact should be non-empty: {}",
        runtime_graph_path.display()
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\"full_fidelity\":true"),
        "replay JSON should mark full fidelity when V2 records exist: {stdout}"
    );

    Ok(())
}
