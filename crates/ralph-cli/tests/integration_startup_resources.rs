use anyhow::{Context, Result};
use serde_json::Value;
use serde_yaml::{Mapping, Value as YamlValue};
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn record_has_termination_reason(path: &Path, expected_reason: &str) -> Result<bool> {
    let content = fs::read_to_string(path).with_context(|| path.display().to_string())?;
    for (i, line) in content.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }

        let value: Value = serde_json::from_str(line)
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

fn record_has_loop_start_ux_mode(path: &Path, expected_ux_mode: &str) -> Result<bool> {
    let content = fs::read_to_string(path).with_context(|| path.display().to_string())?;
    for (i, line) in content.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }

        let value: Value = serde_json::from_str(line)
            .with_context(|| format!("Invalid JSON at line {}: {}", i + 1, line))?;
        if value.get("event").and_then(|v| v.as_str()) != Some("_meta.loop_start") {
            continue;
        }

        if value
            .get("data")
            .and_then(|data| data.get("ux_mode"))
            .and_then(|ux_mode| ux_mode.as_str())
            == Some(expected_ux_mode)
        {
            return Ok(true);
        }
    }

    Ok(false)
}

fn write_live_prompt_capture_backend(path: &Path) -> Result<()> {
    // ─────────────────────────────────────────────────────────────────────
    // 这个 backend 只负责两件事:
    // 1. 把真实 `ralph#1` stdin prompt 落盘,供测试断言 startup + protocol marker。
    // 2. 立即输出 `LOOP_COMPLETE`,让这条 gate 保持最小 live runtime 形态。
    // ─────────────────────────────────────────────────────────────────────
    let script = r#"#!/bin/sh
set -eu
mkdir -p .ralph/dogfood
instance="${RALPH_HAT_INSTANCE_ID:-unknown}"
case "$instance" in
  ralph#1)
    prompt_capture=".ralph/dogfood/ralph#1.prompt.txt"
    cat > "$prompt_capture"
    printf 'startup bootstrap live gate complete.\n'
    printf 'LOOP_COMPLETE\n'
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

fn inject_custom_backend_into_resolved_config(
    resolved_config_path: &Path,
    script_path: &Path,
) -> Result<()> {
    // ─────────────────────────────────────────────────────────────────────
    // 只替换执行表面:
    // - workflow / hats / parallel topology 保持 startup bootstrap 原样
    // - backend 切到 custom+stdin,用于可控 prompt capture
    // 这样 gate 验证的仍然是 startup 产物,不是手写第二份配置。
    // ─────────────────────────────────────────────────────────────────────
    let raw = fs::read_to_string(resolved_config_path)
        .with_context(|| resolved_config_path.display().to_string())?;
    let mut root: YamlValue = serde_yaml::from_str(&raw)?;

    let root_map = root
        .as_mapping_mut()
        .context("resolved config should be a YAML mapping")?;
    let cli_key = YamlValue::String("cli".to_string());

    let cli_value = root_map
        .entry(cli_key)
        .or_insert_with(|| YamlValue::Mapping(Mapping::new()));
    let cli_map = cli_value
        .as_mapping_mut()
        .context("resolved config cli should be a YAML mapping")?;

    cli_map.insert(
        YamlValue::String("backend".to_string()),
        YamlValue::String("custom".to_string()),
    );
    cli_map.insert(
        YamlValue::String("command".to_string()),
        YamlValue::String(script_path.display().to_string()),
    );
    cli_map.insert(
        YamlValue::String("prompt_mode".to_string()),
        YamlValue::String("stdin".to_string()),
    );

    let rendered = serde_yaml::to_string(&root)?;
    fs::write(resolved_config_path, rendered)
        .with_context(|| resolved_config_path.display().to_string())?;
    Ok(())
}

#[test]
fn missing_default_config_and_prompt_bootstraps_before_dry_run() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let workspace = temp_dir.path().join("workspace");
    let ralph_home = temp_dir.path().join("ralph-home");
    fs::create_dir_all(&workspace)?;

    // 说明:
    // - workspace 中故意不创建 `ralph.yml` / `PROMPT.md`。
    // - `RALPH_HOME` 指向临时目录,避免测试污染用户真实资源目录。
    let output = Command::new(env!("CARGO_BIN_EXE_ralph"))
        .args(["run", "--dry-run", "--no-tui"])
        .env("RALPH_HOME", &ralph_home)
        .current_dir(&workspace)
        .output()?;

    assert!(
        output.status.success(),
        "startup bootstrap dry-run should succeed.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let selection_path = workspace.join(".ralph/bootstrap-selection.json");
    let resolved_config_path = workspace.join(".ralph/resolved-config.yml");
    assert!(
        selection_path.exists(),
        "bootstrap selection artifact should be written before dry-run exits"
    );
    assert!(
        resolved_config_path.exists(),
        "resolved config artifact should be written before dry-run exits"
    );

    let selection: Value = serde_json::from_str(&fs::read_to_string(selection_path)?)?;
    assert_eq!(selection["startup_only"], true);
    assert_eq!(selection["resource_root_source"], "RALPH_HOME");
    assert_eq!(
        selection["selected_resources"][0],
        "workflow:feature-minimal"
    );
    assert_eq!(
        selection["selected_resources"][1],
        "prompt:bootstrap-default-task"
    );

    let resolved_config = fs::read_to_string(resolved_config_path)?;
    assert!(resolved_config.contains("event_loop:"));
    assert!(resolved_config.contains("Act as Ralph's startup bootstrap coordinator"));
    assert!(
        resolved_config.contains("parallel:") && resolved_config.contains("enabled: true"),
        "无配置启动生成的 resolved config 应默认启用并行模式,实际为:
{resolved_config}"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Prompt: inline text"),
        "dry-run output should show the resolved inline bootstrap prompt, got: {stdout}"
    );

    Ok(())
}

#[test]
fn startup_bootstrap_live_gate_captures_real_coordinator_prompt_and_record_session() -> Result<()> {
    // ─────────────────────────────────────────────────────────────────────
    // 这是方向1的 repo-native gate:
    // - 第一步: 真实 no-config/no-prompt startup bootstrap,产出 artifact
    // - 第二步: 复用 resolved config 做 live run,抓 `ralph#1` prompt 与 record-session
    // - 不引入 capability / 多 hat 业务语义,只证明 startup bootstrap 与内置协议已接通
    // ─────────────────────────────────────────────────────────────────────
    let temp_dir = TempDir::new()?;
    let workspace = temp_dir.path().join("workspace");
    let ralph_home = temp_dir.path().join("ralph-home");
    let script_path = temp_dir.path().join("bootstrap-live-backend.sh");
    let record_path = workspace.join("live-session.jsonl");
    fs::create_dir_all(&workspace)?;

    write_live_prompt_capture_backend(&script_path)?;

    let bootstrap_output = Command::new(env!("CARGO_BIN_EXE_ralph"))
        .args(["run", "--dry-run", "--no-tui"])
        .env("RALPH_HOME", &ralph_home)
        .current_dir(&workspace)
        .output()?;

    assert!(
        bootstrap_output.status.success(),
        "startup bootstrap dry-run should succeed before live gate.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&bootstrap_output.stdout),
        String::from_utf8_lossy(&bootstrap_output.stderr)
    );

    let selection_path = workspace.join(".ralph/bootstrap-selection.json");
    let resolved_config_path = workspace.join(".ralph/resolved-config.yml");
    let selection: Value = serde_json::from_str(&fs::read_to_string(&selection_path)?)?;
    assert_eq!(selection["startup_only"], true);
    assert_eq!(
        selection["selected_resources"][0],
        "workflow:feature-minimal"
    );
    assert_eq!(
        selection["selected_resources"][1],
        "prompt:bootstrap-default-task"
    );

    let resolved_before_live = fs::read_to_string(&resolved_config_path)?;
    assert!(
        resolved_before_live.contains("Act as Ralph's startup bootstrap coordinator"),
        "bootstrap resolved config should contain startup coordinator prompt: {resolved_before_live}"
    );
    assert!(
        resolved_before_live.contains("parallel:")
            && resolved_before_live.contains("enabled: true"),
        "bootstrap resolved config should enable parallel mode by default: {resolved_before_live}"
    );

    inject_custom_backend_into_resolved_config(&resolved_config_path, &script_path)?;

    let live_output = Command::new(env!("CARGO_BIN_EXE_ralph"))
        .args([
            "run",
            "--config",
            resolved_config_path.to_string_lossy().as_ref(),
            "--no-tui",
            "--record-session",
            record_path.to_string_lossy().as_ref(),
        ])
        .env("RALPH_HOME", &ralph_home)
        .current_dir(&workspace)
        .output()?;

    assert!(
        live_output.status.success(),
        "bootstrap live gate run should succeed.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&live_output.stdout),
        String::from_utf8_lossy(&live_output.stderr)
    );

    let captured_prompt = fs::read_to_string(workspace.join(".ralph/dogfood/ralph#1.prompt.txt"))?;
    assert!(
        captured_prompt.contains("Act as Ralph's startup bootstrap coordinator"),
        "live ralph#1 prompt should include startup bootstrap coordinator instructions: {captured_prompt}"
    );
    assert!(
        captured_prompt.contains("## RALPH EVENT EMISSION PROTOCOL"),
        "live ralph#1 prompt should include built-in event emission protocol: {captured_prompt}"
    );
    assert!(
        captured_prompt.contains("reply.human.message"),
        "live ralph#1 prompt should include reply.human.message contract anchor: {captured_prompt}"
    );

    assert!(
        record_has_loop_start_ux_mode(&record_path, "parallel-cli")?,
        "record-session should capture `parallel-cli` ux_mode: {}",
        fs::read_to_string(&record_path)?
    );
    assert!(
        record_has_termination_reason(&record_path, "CompletionPromise")?,
        "record-session should capture `CompletionPromise` termination: {}",
        fs::read_to_string(&record_path)?
    );

    Ok(())
}

#[test]
fn explicit_missing_config_does_not_bootstrap() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let workspace = temp_dir.path().join("workspace");
    let ralph_home = temp_dir.path().join("ralph-home");
    fs::create_dir_all(&workspace)?;

    // 说明:
    // - 显式 `--config ralph.yml` 即使路径名等于默认值,也代表用户明确选择了 config source。
    // - 因此 selector 不能把这个缺失文件吞成默认 bootstrap。
    let output = Command::new(env!("CARGO_BIN_EXE_ralph"))
        .args(["run", "--config", "ralph.yml", "--dry-run", "--no-tui"])
        .env("RALPH_HOME", &ralph_home)
        .current_dir(&workspace)
        .output()?;

    assert!(
        output.status.success(),
        "legacy explicit missing config fallback should still dry-run successfully.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !workspace.join(".ralph/bootstrap-selection.json").exists(),
        "explicit config source must bypass startup bootstrap selector"
    );
    assert!(
        !workspace.join(".ralph/resolved-config.yml").exists(),
        "explicit config source must not write bootstrap resolved config artifact"
    );

    Ok(())
}
