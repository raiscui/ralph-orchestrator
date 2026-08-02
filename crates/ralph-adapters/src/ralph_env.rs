//! Ralph 子进程环境隔离。
//!
//! 当 Ralph 自己运行在 hat worker 或 capability child 环境里时,父进程会携带
//! `RALPH_HAT_INSTANCE_ID` / `RALPH_CAPABILITY_CHILD` 这类运行时身份变量。
//! 这些变量不能原样泄露给 Ralph 再次启动的子命令,否则子命令会误判自己的身份。

/// 这些变量描述父级 Ralph worker/capability 运行时,不应该被普通子命令继承。
///
/// 说明:
/// - 并行 job 会在清理后重新写入自己的 `RALPH_HAT_*` 身份。
/// - capability child 会在清理后重新写入自己的 `RALPH_CAPABILITY_*` 身份。
/// - 这里不清理 `RALPH_HOME` / `RALPH_DIAGNOSTICS` 等配置或调试变量。
pub const RALPH_PARENT_WORKER_ENV_VARS: &[&str] = &[
    "RALPH_HAT_INSTANCE_ID",
    "RALPH_HAT_ID",
    "RALPH_CAPABILITY_CHILD",
    "RALPH_CAPABILITY_MODE",
];

/// 清理 `std::process::Command` 上的父级 Ralph worker/capability 变量。
pub fn scrub_ralph_parent_worker_env(command: &mut std::process::Command) {
    for key in RALPH_PARENT_WORKER_ENV_VARS {
        command.env_remove(key);
    }
}

/// 清理 `tokio::process::Command` 上的父级 Ralph worker/capability 变量。
pub fn scrub_ralph_parent_worker_env_tokio(command: &mut tokio::process::Command) {
    for key in RALPH_PARENT_WORKER_ENV_VARS {
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
    fn std_command_scrubs_parent_ralph_worker_env() {
        let mut command = std::process::Command::new("ralph");
        scrub_ralph_parent_worker_env(&mut command);

        for key in RALPH_PARENT_WORKER_ENV_VARS {
            assert!(
                std_command_removed_env(&command, key),
                "{key} should be explicitly removed from child command"
            );
        }
    }
}
