use anyhow::Result;
use ralph_core::{
    AgentInstanceSnapshot, AgentLastInput, AgentRecoverableFailureSummary, AgentsSnapshot,
};
use ralph_proto::HatInstanceState;
use std::fs;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use tempfile::TempDir;

#[test]
fn test_agents_command_prints_table() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let temp_path = temp_dir.path();

    let ralph_dir = temp_path.join(".ralph");
    fs::create_dir_all(&ralph_dir)?;

    let snapshot_path = ralph_dir.join("agents.json");
    let snapshot = AgentsSnapshot {
        generated_at: "2026-02-01T00:00:00Z".to_string(),
        instances: vec![AgentInstanceSnapshot {
            instance_id: "writer#1".to_string(),
            hat_id: "writer".to_string(),
            state: HatInstanceState::Running,
            is_dynamic: false,
            last_input: Some(AgentLastInput {
                ts: "2026-02-01T00:00:01Z".to_string(),
                topic: "build.task".to_string(),
                preview: "do it".to_string(),
            }),
            recoverable_failures: Vec::new(),
        }],
    };
    fs::write(&snapshot_path, serde_json::to_string_pretty(&snapshot)?)?;

    let output = Command::new(env!("CARGO_BIN_EXE_ralph"))
        .arg("agents")
        .current_dir(temp_path)
        .output()?;

    assert!(output.status.success(), "Command should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("writer#1"));
    assert!(stdout.contains("writer"));
    assert!(stdout.contains("running"));
    assert!(stdout.contains("build.task"));

    Ok(())
}

#[test]
fn test_agents_command_prints_json() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let temp_path = temp_dir.path();

    let ralph_dir = temp_path.join(".ralph");
    fs::create_dir_all(&ralph_dir)?;

    let snapshot_path = ralph_dir.join("agents.json");
    fs::write(&snapshot_path, r#"{"generated_at":"x","instances":[]}"#)?;

    let output = Command::new(env!("CARGO_BIN_EXE_ralph"))
        .arg("agents")
        .arg("--format")
        .arg("json")
        .current_dir(temp_path)
        .output()?;

    assert!(output.status.success(), "Command should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"generated_at\""));
    assert!(stdout.contains("\"instances\""));

    Ok(())
}

#[test]
fn test_agents_command_prints_recoverable_summary() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let temp_path = temp_dir.path();

    let ralph_dir = temp_path.join(".ralph");
    fs::create_dir_all(&ralph_dir)?;

    let snapshot_path = ralph_dir.join("agents.json");
    let snapshot = AgentsSnapshot {
        generated_at: "2026-02-01T00:00:00Z".to_string(),
        instances: vec![AgentInstanceSnapshot {
            instance_id: "writer#1".to_string(),
            hat_id: "writer".to_string(),
            state: HatInstanceState::Idle,
            is_dynamic: false,
            last_input: None,
            recoverable_failures: vec![AgentRecoverableFailureSummary {
                failure_id: "failure-writer-429".to_string(),
                job_id: 7,
                status: "retry_scheduled".to_string(),
                failure_kind: "retry_limit_exceeded".to_string(),
                attempt: 1,
                max_attempts: 3,
                retry_after_ms: Some(30_000),
                next_retry_at: Some("2026-02-01T00:00:30Z".to_string()),
                ledger_path: ".ralph/recoverable-failures.jsonl".to_string(),
                stderr_preview: Some("ERROR: exceeded retry limit, last status: 429".to_string()),
            }],
        }],
    };
    fs::write(&snapshot_path, serde_json::to_string_pretty(&snapshot)?)?;

    let output = Command::new(env!("CARGO_BIN_EXE_ralph"))
        .arg("agents")
        .current_dir(temp_path)
        .output()?;

    assert!(output.status.success(), "Command should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Recoverable"));
    assert!(stdout.contains("writer#1"));
    assert!(stdout.contains("retry_scheduled"));
    assert!(stdout.contains("1/3"));

    let json_output = Command::new(env!("CARGO_BIN_EXE_ralph"))
        .arg("agents")
        .arg("--format")
        .arg("json")
        .current_dir(temp_path)
        .output()?;

    assert!(json_output.status.success(), "JSON command should succeed");

    let json_snapshot: AgentsSnapshot = serde_json::from_slice(&json_output.stdout)?;
    let writer = json_snapshot
        .instances
        .iter()
        .find(|instance| instance.instance_id == "writer#1")
        .expect("writer#1 should remain visible in agents snapshot JSON");
    let failure = writer
        .recoverable_failures
        .first()
        .expect("writer#1 should expose recoverable failure metadata");

    assert_eq!(failure.failure_id, "failure-writer-429");
    assert_eq!(failure.status, "retry_scheduled");
    assert_eq!(failure.attempt, 1);
    assert_eq!(failure.max_attempts, 3);
    assert_eq!(
        failure.next_retry_at.as_deref(),
        Some("2026-02-01T00:00:30Z")
    );
    assert_eq!(failure.ledger_path, ".ralph/recoverable-failures.jsonl");

    Ok(())
}

#[test]
fn test_agents_command_finds_snapshot_in_parent_directories() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let temp_path = temp_dir.path();

    let ralph_dir = temp_path.join(".ralph");
    fs::create_dir_all(&ralph_dir)?;

    let snapshot_path = ralph_dir.join("agents.json");
    let snapshot = AgentsSnapshot {
        generated_at: "2026-02-01T00:00:00Z".to_string(),
        instances: vec![AgentInstanceSnapshot {
            instance_id: "writer#1".to_string(),
            hat_id: "writer".to_string(),
            state: HatInstanceState::Running,
            is_dynamic: false,
            last_input: Some(AgentLastInput {
                ts: "2026-02-01T00:00:01Z".to_string(),
                topic: "build.task".to_string(),
                preview: "do it".to_string(),
            }),
            recoverable_failures: Vec::new(),
        }],
    };
    fs::write(&snapshot_path, serde_json::to_string_pretty(&snapshot)?)?;

    // 在子目录执行,但仍应定位到父目录的 `.ralph/agents.json`。
    let nested = temp_path.join("a/b/c");
    fs::create_dir_all(&nested)?;

    let output = Command::new(env!("CARGO_BIN_EXE_ralph"))
        .arg("agents")
        .current_dir(&nested)
        .output()?;

    assert!(output.status.success(), "Command should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("writer#1"));
    assert!(stdout.contains("build.task"));

    Ok(())
}

#[test]
fn test_agents_command_watch_prints_output_at_least_once() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let temp_path = temp_dir.path();

    let ralph_dir = temp_path.join(".ralph");
    fs::create_dir_all(&ralph_dir)?;

    let snapshot_path = ralph_dir.join("agents.json");
    let snapshot = AgentsSnapshot {
        generated_at: "2026-02-01T00:00:00Z".to_string(),
        instances: vec![AgentInstanceSnapshot {
            instance_id: "writer#1".to_string(),
            hat_id: "writer".to_string(),
            state: HatInstanceState::Running,
            is_dynamic: false,
            last_input: Some(AgentLastInput {
                ts: "2026-02-01T00:00:01Z".to_string(),
                topic: "build.task".to_string(),
                preview: "do it".to_string(),
            }),
            recoverable_failures: Vec::new(),
        }],
    };
    fs::write(&snapshot_path, serde_json::to_string_pretty(&snapshot)?)?;

    let stdout_path = temp_path.join("watch.stdout");
    let stdout_file = fs::File::create(&stdout_path)?;

    // 说明:
    // - `--watch` 是一个无限循环,因此这里要先等到首轮输出真正出现,再结束子进程。
    // - 不能依赖固定 sleep,否则 CI 一抖就会把"还没来得及打印"误报成失败。
    let mut child = Command::new(env!("CARGO_BIN_EXE_ralph"))
        .args([
            "--color",
            "never",
            "agents",
            "--watch",
            "--watch-interval-ms",
            "50",
        ])
        .current_dir(temp_path)
        .stdout(Stdio::from(stdout_file))
        .spawn()?;

    let deadline = Instant::now() + Duration::from_secs(5);

    while Instant::now() < deadline {
        let captured_stdout = fs::read_to_string(&stdout_path).unwrap_or_default();
        let saw_expected_output = captured_stdout.contains("Watching")
            && captured_stdout.contains("writer#1")
            && captured_stdout.contains("build.task");
        if saw_expected_output {
            break;
        }

        if let Some(status) = child.try_wait()? {
            panic!(
                "watch process exited before producing expected output: status={status}, stdout={captured_stdout}"
            );
        }

        std::thread::sleep(Duration::from_millis(50));
    }

    let _ = child.kill();
    let _status = child.wait()?;

    let captured_stdout = fs::read_to_string(&stdout_path).unwrap_or_default();
    assert!(
        captured_stdout.contains("Watching"),
        "should print watch header, got: {captured_stdout}"
    );
    assert!(
        captured_stdout.contains("writer#1"),
        "should print instance row, got: {captured_stdout}"
    );
    assert!(
        captured_stdout.contains("build.task"),
        "should print last input topic, got: {captured_stdout}"
    );

    Ok(())
}
