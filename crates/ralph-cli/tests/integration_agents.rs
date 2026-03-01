use anyhow::Result;
use ralph_core::{AgentInstanceSnapshot, AgentLastInput, AgentsSnapshot};
use ralph_proto::HatInstanceState;
use std::fs;
use std::process::{Command, Stdio};
use std::time::Duration;
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
        }],
    };
    fs::write(&snapshot_path, serde_json::to_string_pretty(&snapshot)?)?;

    // 说明:
    // - `--watch` 是一个无限循环,因此这里用 spawn + kill 的方式验证它至少能输出一次表格.
    // - stdout 是 pipe(非 TTY)时,实现不会输出清屏控制序列,更适合测试做字符串断言.
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
        .stdout(Stdio::piped())
        .spawn()?;

    std::thread::sleep(Duration::from_millis(300));
    let _ = child.kill();
    let output = child.wait_with_output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Watching"), "should print watch header");
    assert!(stdout.contains("writer#1"), "should print instance row");
    assert!(
        stdout.contains("build.task"),
        "should print last input topic"
    );

    Ok(())
}
