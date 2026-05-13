use anyhow::Result;
use ralph_core::{LifecycleOutcome, RunOutcome, StateMode, StateOperationStore, StateWriteRequest};
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn ralph_bin() -> &'static str {
    env!("CARGO_BIN_EXE_ralph")
}

fn run_ralph(root: &Path, args: &[&str]) -> Result<std::process::Output> {
    Ok(Command::new(ralph_bin())
        .args(args)
        .current_dir(root)
        .output()?)
}

fn write_team_state(root: &Path) -> Result<()> {
    // ─────────────────────────────────────────────────────────────────────
    // 这里必须通过 core store 写测试数据。
    // 这样 CLI 测的是 adapter 行为,不会在测试里暗中固定另一套 JSON 语义。
    // ─────────────────────────────────────────────────────────────────────
    let store = StateOperationStore::new(root);
    store.state_write(
        StateWriteRequest::new(StateMode::Team)
            .with_active(true)
            .with_current_phase("running")
            .with_run_outcome(RunOutcome::Continue)
            .with_lifecycle_outcome(LifecycleOutcome::Finished)
            .with_updated_at("2026-05-12T00:00:00Z"),
    )?;
    Ok(())
}

#[test]
fn state_status_json_reports_core_written_state() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path();
    write_team_state(root)?;

    let output = run_ralph(
        root,
        &[
            "state", "status", "--mode", "team", "--json", "--color", "never",
        ],
    )?;

    assert!(
        output.status.success(),
        "state status should succeed.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let json: Value = serde_json::from_slice(&output.stdout)?;
    let statuses = json["statuses"].as_array().expect("statuses array");
    assert_eq!(
        statuses.len(),
        1,
        "single-mode status should return one entry"
    );
    assert_eq!(statuses[0]["mode"], "team");
    assert_eq!(statuses[0]["exists"], true);
    assert_eq!(statuses[0]["active"], true);
    assert_eq!(statuses[0]["current_phase"], "running");
    assert_eq!(statuses[0]["run_outcome"], "continue");
    assert_eq!(statuses[0]["lifecycle_outcome"], "finished");
    assert!(
        statuses[0]["path"]
            .as_str()
            .unwrap()
            .ends_with("team-state.json"),
        "status path should identify the state file: {}",
        statuses[0]["path"]
    );

    Ok(())
}

#[test]
fn state_read_json_reports_missing_state_without_failure() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let output = run_ralph(
        temp_dir.path(),
        &["state", "read", "team", "--json", "--color", "never"],
    )?;

    assert!(
        output.status.success(),
        "missing state should be a successful empty read.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let json: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(json["mode"], "team");
    assert_eq!(json["exists"], false);
    assert!(json["record"].is_null());

    Ok(())
}

#[test]
fn state_clear_deletes_core_written_state() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path();
    write_team_state(root)?;

    let state_path = root.join(".ralph/state/team-state.json");
    assert!(
        state_path.exists(),
        "precondition: core-written state exists"
    );

    let output = run_ralph(root, &["state", "clear", "team", "--color", "never"])?;

    assert!(
        output.status.success(),
        "state clear should succeed.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Cleared 1 state file"),
        "clear output should report deleted count, got: {stdout}"
    );
    assert!(
        !state_path.exists(),
        "state clear should remove the core-written state file"
    );

    Ok(())
}

#[test]
fn state_read_fails_on_malformed_state_file() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path();
    let state_path = root.join(".ralph/state/team-state.json");
    fs::create_dir_all(state_path.parent().unwrap())?;
    fs::write(&state_path, "{ definitely-not-json")?;

    let output = run_ralph(root, &["state", "read", "team", "--color", "never"])?;

    assert!(
        !output.status.success(),
        "malformed state should fail instead of being treated as missing"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("state read failed"),
        "stderr should include command context, got: {stderr}"
    );
    assert!(
        stderr.contains("malformed state file"),
        "stderr should include core parse error, got: {stderr}"
    );

    Ok(())
}

#[test]
fn state_help_lists_inspection_commands_without_write() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let output = run_ralph(temp_dir.path(), &["state", "--help", "--color", "never"])?;

    assert!(
        output.status.success(),
        "state help should succeed.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("status"),
        "state help should list status, got: {stdout}"
    );
    assert!(
        stdout.contains("read"),
        "state help should list read, got: {stdout}"
    );
    assert!(
        stdout.contains("clear"),
        "state help should list clear, got: {stdout}"
    );
    assert!(
        !stdout.contains("write"),
        "state help must not expose manual state write in v1, got: {stdout}"
    );

    Ok(())
}
