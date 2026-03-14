use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use tempfile::TempDir;

fn write_min_parallel_config(path: &Path) -> Result<()> {
    // 说明:
    // - `--idle-start` 仅在 parallel.enabled=true 时允许.
    // - 本测试只验证 record-session 的中断契约,因此不需要 hats/backend 可用.
    let content = r#"
parallel:
  enabled: true
"#;
    fs::write(path, content.trim_start())?;
    Ok(())
}

fn wait_for_file_contains(path: &Path, needle: &str, timeout: Duration) -> Result<()> {
    let start = Instant::now();
    loop {
        if start.elapsed() > timeout {
            anyhow::bail!(
                "Timeout waiting for file to contain needle: file={} needle={needle}",
                path.display()
            );
        }

        if let Ok(content) = fs::read_to_string(path) {
            if content.contains(needle) {
                return Ok(());
            }
        }

        std::thread::sleep(Duration::from_millis(20));
    }
}

fn wait_for_child_exit(child: &mut std::process::Child, timeout: Duration) -> Result<()> {
    let start = Instant::now();
    loop {
        if let Some(_status) = child.try_wait()? {
            return Ok(());
        }
        if start.elapsed() > timeout {
            let _ = child.kill();
            anyhow::bail!("Timeout waiting for child to exit");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn record_command_help_is_discoverable() -> Result<()> {
    let output = Command::new(env!("CARGO_BIN_EXE_ralph"))
        .arg("--help")
        .output()?;
    assert!(output.status.success(), "ralph --help should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("record"), "help should mention record");

    let output = Command::new(env!("CARGO_BIN_EXE_ralph"))
        .args(["record", "--help"])
        .output()?;
    assert!(
        output.status.success(),
        "ralph record --help should succeed"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("summary"),
        "record help should mention summary"
    );
    assert!(stdout.contains("watch"), "record help should mention watch");
    Ok(())
}

#[test]
#[cfg(unix)]
fn sigint_leaves_record_session_parseable_and_writes_termination_and_pointer() -> Result<()> {
    use nix::sys::signal::{Signal, kill};
    use nix::unistd::Pid;

    let temp_dir = TempDir::new()?;
    let temp_path = temp_dir.path();

    let config_path = temp_path.join("ralph.yml");
    write_min_parallel_config(&config_path)?;

    let record_path = temp_path.join("session.jsonl");

    let mut child = Command::new(env!("CARGO_BIN_EXE_ralph"))
        .args([
            "run",
            "--config",
            config_path.to_string_lossy().as_ref(),
            "--idle-start",
            "--no-tui",
            "--record-session",
            record_path.to_string_lossy().as_ref(),
        ])
        .current_dir(temp_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn ralph run")?;

    // 等待 recorder 写入 meta(必须尽快落盘,否则后续断言没意义).
    wait_for_file_contains(&record_path, "_meta.session_start", Duration::from_secs(5))?;
    wait_for_file_contains(&record_path, "_meta.loop_start", Duration::from_secs(5))?;

    // 发送 SIGINT.
    let pid = child.id();
    #[allow(clippy::cast_possible_wrap)]
    let pid = Pid::from_raw(pid as i32);
    kill(pid, Signal::SIGINT).context("Failed to send SIGINT")?;

    wait_for_child_exit(&mut child, Duration::from_secs(10))?;

    // 1) JSONL 必须逐行可解析 + 关键 meta 必须存在.
    let content =
        fs::read_to_string(&record_path).with_context(|| record_path.display().to_string())?;
    let mut has_session_start = false;
    let mut has_loop_start = false;
    let mut has_termination_interrupted = false;

    for (i, line) in content.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let v: serde_json::Value = serde_json::from_str(line)
            .with_context(|| format!("Invalid JSON at line {}: {}", i + 1, line))?;
        let event = v.get("event").and_then(|v| v.as_str()).unwrap_or_default();
        if event == "_meta.session_start" {
            has_session_start = true;
        }
        if event == "_meta.loop_start" {
            has_loop_start = true;
        }
        if event == "_meta.termination"
            && v.get("data")
                .and_then(|d| d.get("reason"))
                .and_then(|r| r.as_str())
                == Some("Interrupted")
        {
            has_termination_interrupted = true;
        }
    }

    assert!(has_session_start, "must contain _meta.session_start");
    assert!(has_loop_start, "must contain _meta.loop_start");
    assert!(
        has_termination_interrupted,
        "must contain _meta.termination(reason=Interrupted)"
    );

    // 2) `.ralph/record-session.latest` 指针必须写入且可解析.
    let pointer_path = temp_path.join(".ralph/record-session.latest");
    assert!(
        pointer_path.exists(),
        "pointer must exist: {}",
        pointer_path.display()
    );
    let raw = fs::read_to_string(&pointer_path)?;
    let trimmed = raw.trim();
    assert!(!trimmed.is_empty(), "pointer must not be empty");

    let target = PathBuf::from(trimmed);
    let target = if target.is_absolute() {
        target
    } else {
        temp_path.join(target)
    };

    let record_abs = fs::canonicalize(&record_path)?;
    let target_abs = fs::canonicalize(&target)?;
    assert_eq!(
        target_abs, record_abs,
        "pointer should resolve to record path"
    );

    Ok(())
}

#[test]
fn record_watch_auto_locates_latest_pointer_and_streams_lines() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let temp_path = temp_dir.path();

    // 写一个最小 record 文件 + pointer.
    let record_path = temp_path.join("session.jsonl");
    fs::write(
        &record_path,
        "{\"ts\":1,\"event\":\"_meta.session_start\",\"data\":{}}\n",
    )?;
    fs::create_dir_all(temp_path.join(".ralph"))?;
    fs::write(
        temp_path.join(".ralph/record-session.latest"),
        format!("{}\n", record_path.display()),
    )?;

    // 在子目录执行,验证无参 watch 能向上定位到 pointer.
    let nested = temp_path.join("a/b/c");
    fs::create_dir_all(&nested)?;

    let mut child = Command::new(env!("CARGO_BIN_EXE_ralph"))
        .args(["record", "watch", "--from-start", "--interval-ms", "50"])
        .current_dir(&nested)
        .stdout(Stdio::piped())
        .spawn()?;

    std::thread::sleep(Duration::from_millis(200));
    let _ = child.kill();
    let output = child.wait_with_output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("_meta.session_start"),
        "watch should stream existing lines"
    );

    Ok(())
}

#[test]
fn record_watch_until_event_exits_zero_in_quiet_mode() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let temp_path = temp_dir.path();

    // 写一个最小 record 文件,包含我们要等待的事件.
    let record_path = temp_path.join("session.jsonl");
    fs::write(
        &record_path,
        "{\"ts\":1,\"event\":\"_meta.session_start\",\"data\":{}}\n",
    )?;

    let output = Command::new(env!("CARGO_BIN_EXE_ralph"))
        .args([
            "record",
            "watch",
            record_path.to_string_lossy().as_ref(),
            "--from-start",
            "--interval-ms",
            "10",
            "--until-event",
            "_meta.session_start",
            "--timeout-secs",
            "1",
            "--quiet",
        ])
        .current_dir(temp_path)
        .output()?;

    assert!(
        output.status.success(),
        "watch should exit 0 when condition satisfied"
    );

    Ok(())
}

#[test]
fn record_watch_timeout_exits_two_in_quiet_mode() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let temp_path = temp_dir.path();

    // 写一个最小 record 文件,不包含目标 topic.
    let record_path = temp_path.join("session.jsonl");
    fs::write(
        &record_path,
        "{\"ts\":1,\"event\":\"_meta.session_start\",\"data\":{}}\n",
    )?;

    let output = Command::new(env!("CARGO_BIN_EXE_ralph"))
        .args([
            "record",
            "watch",
            record_path.to_string_lossy().as_ref(),
            "--from-start",
            "--interval-ms",
            "10",
            "--until-topic",
            "reply.human.message",
            "--timeout-secs",
            "1",
            "--quiet",
        ])
        .current_dir(temp_path)
        .output()?;

    assert_eq!(
        output.status.code(),
        Some(2),
        "watch should exit code 2 on timeout (agent automation contract)"
    );

    Ok(())
}
