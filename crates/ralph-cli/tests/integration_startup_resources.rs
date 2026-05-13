use anyhow::Result;
use serde_json::Value;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

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

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Prompt: inline text"),
        "dry-run output should show the resolved inline bootstrap prompt, got: {stdout}"
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
