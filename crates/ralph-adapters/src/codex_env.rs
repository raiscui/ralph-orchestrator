//! Codex 子进程环境隔离。
//!
//! Ralph 经常在 Codex 自己的会话里被调用。此时父进程会带有
//! `CODEX_THREAD_ID` 这类会话私有变量。若再原样传给子 `codex exec`,
//! 子 Codex 会把自己的 rollout 写回父 thread,从而出现 thread not found。

use portable_pty::CommandBuilder;
use std::path::Path;

/// 这些变量描述父 Codex 会话,不应该跨进程继承给 Ralph 启动的子 Codex。
pub const CODEX_PARENT_SESSION_ENV_VARS: &[&str] = &["CODEX_THREAD_ID", "CODEX_TURN_ID"];

/// 判断一个命令是否是 Codex CLI。
pub fn is_codex_command(program: &str) -> bool {
    matches!(
        Path::new(program)
            .file_name()
            .and_then(|name| name.to_str()),
        Some("codex" | "codex.exe" | "codex.cmd")
    )
}

/// 清理 `std::process::Command` 上的父 Codex 会话变量。
pub fn scrub_codex_parent_session_env(command: &mut std::process::Command, program: &str) {
    if !is_codex_command(program) {
        return;
    }

    for key in CODEX_PARENT_SESSION_ENV_VARS {
        command.env_remove(key);
    }
}

/// 清理 `tokio::process::Command` 上的父 Codex 会话变量。
pub fn scrub_codex_parent_session_env_tokio(command: &mut tokio::process::Command, program: &str) {
    if !is_codex_command(program) {
        return;
    }

    for key in CODEX_PARENT_SESSION_ENV_VARS {
        command.env_remove(key);
    }
}

/// 清理 PTY `CommandBuilder` 上的父 Codex 会话变量。
pub(crate) fn scrub_codex_parent_session_env_pty(command: &mut CommandBuilder, program: &str) {
    if !is_codex_command(program) {
        return;
    }

    for key in CODEX_PARENT_SESSION_ENV_VARS {
        command.env_remove(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;

    fn std_command_removed_env(command: &std::process::Command, key: &str) -> bool {
        command
            .get_envs()
            .any(|(name, value)| name == OsStr::new(key) && value.is_none())
    }

    #[test]
    fn detects_codex_command_by_basename() {
        assert!(is_codex_command("codex"));
        assert!(is_codex_command("/usr/local/bin/codex"));
        assert!(is_codex_command("codex.exe"));
        assert!(!is_codex_command("my-codex-wrapper"));
    }

    #[test]
    fn std_command_scrubs_parent_session_env_only_for_codex() {
        let mut command = std::process::Command::new("codex");
        scrub_codex_parent_session_env(&mut command, "codex");

        assert!(std_command_removed_env(&command, "CODEX_THREAD_ID"));
        assert!(std_command_removed_env(&command, "CODEX_TURN_ID"));

        let mut other = std::process::Command::new("claude");
        scrub_codex_parent_session_env(&mut other, "claude");
        assert!(!std_command_removed_env(&other, "CODEX_THREAD_ID"));
    }

    #[test]
    fn pty_command_builder_scrubs_parent_session_env_for_codex() {
        let mut command = CommandBuilder::new("codex");
        command.env("CODEX_THREAD_ID", "parent-thread");
        command.env("CODEX_TURN_ID", "parent-turn");

        scrub_codex_parent_session_env_pty(&mut command, "codex");

        assert!(command.get_env("CODEX_THREAD_ID").is_none());
        assert!(command.get_env("CODEX_TURN_ID").is_none());
    }
}
