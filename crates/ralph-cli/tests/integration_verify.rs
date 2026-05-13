use anyhow::Result;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn ralph_bin() -> &'static str {
    env!("CARGO_BIN_EXE_ralph")
}

fn write_file(root: &Path, path: &str, contents: &str) -> Result<()> {
    let full_path = root.join(path);
    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(full_path, contents)?;
    Ok(())
}

fn write_valid_guidance_repo(root: &Path) -> Result<()> {
    // ─────────────────────────────────────────────────────────────────────
    // 这里构造最小 guidance repo,避免测试依赖真实工作区的大量文件。
    // CLI 只需要当前目录下有 AGENTS、manifest 和被登记的 SKILL.md。
    // ─────────────────────────────────────────────────────────────────────
    write_file(root, "AGENTS.md", "# AGENTS\n")?;
    write_file(
        root,
        ".agents/skills/code-assist/SKILL.md",
        "---\nname: code-assist\ndescription: Code assist workflow.\n---\n\n# Skill\n",
    )?;
    write_file(
        root,
        "agent-guidance-manifest.toml",
        r#"
schema_version = 1

[[assets]]
id = "skill-code-assist"
type = "skill"
path = ".agents/skills/code-assist/SKILL.md"
status = "active"
summary = "Code assist workflow."
required_in_agents_index = false
"#,
    )?;
    Ok(())
}

#[test]
fn verify_agent_guidance_command_reports_counts() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let temp_path = temp_dir.path();
    write_valid_guidance_repo(temp_path)?;

    let output = Command::new(ralph_bin())
        .args([
            "verify",
            "agent-guidance",
            "--manifest",
            "agent-guidance-manifest.toml",
            "--color",
            "never",
        ])
        .current_dir(temp_path)
        .output()?;

    assert!(
        output.status.success(),
        "verify command should succeed.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Agent guidance manifest verified: agent-guidance-manifest.toml"),
        "stdout should include manifest success line, got: {stdout}"
    );
    assert!(
        stdout.contains("Assets checked: 1"),
        "stdout should include asset count, got: {stdout}"
    );
    assert!(
        stdout.contains("Skills checked: 1"),
        "stdout should include skill count, got: {stdout}"
    );

    Ok(())
}

#[test]
fn verify_agent_guidance_command_fails_on_invalid_skill() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let temp_path = temp_dir.path();
    write_valid_guidance_repo(temp_path)?;

    // ─────────────────────────────────────────────────────────────────────
    // 删除 description,验证 CLI 会把 core verifier 的失败传播成非零退出码。
    // 这避免命令层只打印成功摘要,却吞掉真实 drift。
    // ─────────────────────────────────────────────────────────────────────
    write_file(
        temp_path,
        ".agents/skills/code-assist/SKILL.md",
        "---\nname: code-assist\n---\n\n# Skill\n",
    )?;

    let output = Command::new(ralph_bin())
        .args([
            "verify",
            "agent-guidance",
            "--manifest",
            "agent-guidance-manifest.toml",
            "--color",
            "never",
        ])
        .current_dir(temp_path)
        .output()?;

    assert!(
        !output.status.success(),
        "verify command should fail for invalid skill frontmatter"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("agent guidance manifest verification failed"),
        "stderr should include top-level verifier context, got: {stderr}"
    );
    assert!(
        stderr.contains("missing skill frontmatter field `description`"),
        "stderr should include core verifier reason, got: {stderr}"
    );

    Ok(())
}
