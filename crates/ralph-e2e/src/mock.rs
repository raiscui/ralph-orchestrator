//! mock-mode：用于“零成本、确定性”的 E2E 测试。
//!
//! 这个模块提供了一个 mock CLI adapter：通过回放预录制的 JSONL cassette，
//! 来替代真实 AI 后端（Claude/Kiro/Codex/...）的调用，从而实现：
//! - 可复现：同一 cassette 输出恒定
//! - 零成本：CI 不消耗 API credits
//! - 更快：无网络延迟（可加速回放）
//!
//! ## 架构概览
//!
//! ```text
//! ralph-e2e --mock
//!     │
//!     ├─ CassetteResolver: 为 scenario+backend 定位 cassette
//!     │
//!     └─ mock-cli 子命令：将 cassette 回放为“伪后端”输出
//!         │
//!         ├─ SessionPlayer: 读取 JSONL，提取 terminal write
//!         │
//!         └─ AllowlistExecutor: 可选执行白名单内的本地命令（tasks/memories 等）
//! ```
//!
//! ## Cassette 命名约定
//!
//! Cassette 默认存放在 `cassettes/e2e/`，并按如下顺序解析：
//! - `<scenario-id>-<backend>.jsonl`（backend-specific，优先）
//! - `<scenario-id>.jsonl`（generic fallback）
//!
//! ## 示例
//!
//! ```bash
//! # 运行 mock-mode（会要求每个被选中的 scenario 都能找到 cassette）
//! ralph-e2e --mock
//!
//! # 10x 加速回放
//! ralph-e2e --mock --mock-speed 10.0
//!
//! # mock-cli 通常由 ralph 作为 custom backend 调用，也可手工运行
//! ralph-e2e mock-cli --cassette cassettes/e2e/connect.jsonl
//! ```

use crate::Backend;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// 默认 cassette 目录（相对 repo root）。
pub const DEFAULT_CASSETTE_DIR: &str = "cassettes/e2e";

/// Cassette 解析过程中可能出现的错误。
#[derive(Debug, Error)]
pub enum CassetteError {
    /// 找不到 cassette 文件（会给出尝试过的候选路径）。
    #[error("cassette not found for scenario '{scenario}' backend '{backend}': tried {tried:?}")]
    NotFound {
        scenario: String,
        backend: String,
        tried: Vec<PathBuf>,
    },

    /// cassette 文件存在，但无法读取（权限/损坏等）。
    #[error("cassette file unreadable: {path}: {source}")]
    Unreadable {
        path: PathBuf,
        source: std::io::Error,
    },

    /// cassette 文件 JSONL 格式不合法（解析失败）。
    #[error("cassette parse error in {path}: {message}")]
    ParseError { path: PathBuf, message: String },
}

/// mock-mode 的运行配置。
#[derive(Debug, Clone)]
pub struct MockConfig {
    /// cassette 目录。
    pub cassette_dir: PathBuf,

    /// 回放速度倍率（1.0=实时；10.0=10x；0.0=尽可能快）。
    pub speed: f32,

    /// 允许执行的命令前缀白名单（逗号分隔）。
    ///
    /// 例：`"ralph task add,ralph tools memory add"`
    pub allow_commands: Option<String>,
}

impl Default for MockConfig {
    fn default() -> Self {
        Self {
            cassette_dir: PathBuf::from(DEFAULT_CASSETTE_DIR),
            // CI 默认尽可能快（不引入 sleep）
            speed: 0.0,
            // -----------------------------------------------------------------
            // 说明:
            // - mock-mode 下允许执行的本地命令白名单(逗号分隔).
            // - 默认只开放 E2E 需要的最小集合,避免 cassette 被滥用执行任意命令.
            // - `ralph emit` 用于并行场景里测试“外部事件注入 → 动态实例 spawn”的闭环.
            // -----------------------------------------------------------------
            allow_commands: Some(
                "ralph task add,ralph task close,ralph tools memory add,ralph emit".into(),
            ),
        }
    }
}

impl MockConfig {
    /// 创建一个 mock 配置，并设置 cassette 目录。
    pub fn new(cassette_dir: impl Into<PathBuf>) -> Self {
        Self {
            cassette_dir: cassette_dir.into(),
            ..Default::default()
        }
    }

    /// 设置回放速度倍率（最小为 0.0=尽可能快）。
    pub fn with_speed(mut self, speed: f32) -> Self {
        self.speed = speed.max(0.0);
        self
    }

    /// 设置允许执行的命令前缀白名单（逗号分隔）。
    pub fn with_allow_commands(mut self, commands: impl Into<String>) -> Self {
        self.allow_commands = Some(commands.into());
        self
    }

    /// 禁止在回放过程中执行任何本地命令。
    pub fn without_commands(mut self) -> Self {
        self.allow_commands = None;
        self
    }

    /// 将 cassette 目录解析为“尽量绝对”的路径。
    ///
    /// - 若 `cassette_dir` 已是绝对路径：直接返回
    /// - 若是相对路径：尝试相对 repo root（通过 `Cargo.toml` 向上探测）
    /// - 若探测失败：退回原相对路径（由上层自行决定是否报错）
    pub fn resolve_cassette_dir(&self) -> PathBuf {
        if self.cassette_dir.is_absolute() {
            return self.cassette_dir.clone();
        }

        // 尝试按 repo root 解析
        if let Some(root) = crate::executor::find_workspace_root() {
            root.join(&self.cassette_dir)
        } else {
            self.cassette_dir.clone()
        }
    }

    /// 显式指定“repo root”，用于测试时覆盖自动探测逻辑。
    pub fn with_workspace_root(mut self, root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        if self.cassette_dir.is_relative() {
            self.cassette_dir = root.join(&self.cassette_dir);
        }
        self
    }
}

/// 为 scenario 解析 cassette 文件路径。
///
/// 查找顺序：
/// 1. `<cassette_dir>/<scenario>-<backend>.jsonl`（backend-specific）
/// 2. `<cassette_dir>/<scenario>.jsonl`（generic fallback）
///
/// 两者都不存在则返回错误。
#[derive(Debug, Clone)]
pub struct CassetteResolver {
    /// Base directory for cassette files.
    cassette_dir: PathBuf,
}

impl CassetteResolver {
    /// 创建 resolver。
    pub fn new(cassette_dir: impl Into<PathBuf>) -> Self {
        Self {
            cassette_dir: cassette_dir.into(),
        }
    }

    /// 解析 scenario+backend 对应的 cassette 路径。
    ///
    /// 解析顺序：
    /// 1. `<scenario>-<backend>.jsonl`（例如 `connect-claude.jsonl`）
    /// 2. `<scenario>.jsonl`（例如 `connect.jsonl`）
    pub fn resolve(&self, scenario: &str, backend: Backend) -> Result<PathBuf, CassetteError> {
        let mut tried = Vec::new();

        // 先尝试 backend-specific
        let backend_specific =
            self.cassette_dir
                .join(format!("{}-{}.jsonl", scenario, backend.as_config_str()));
        tried.push(backend_specific.clone());

        if backend_specific.exists() {
            return Ok(backend_specific);
        }

        // 再回退 generic
        let generic = self.cassette_dir.join(format!("{}.jsonl", scenario));
        tried.push(generic.clone());

        if generic.exists() {
            return Ok(generic);
        }

        Err(CassetteError::NotFound {
            scenario: scenario.to_string(),
            backend: backend.as_config_str().to_string(),
            tried,
        })
    }

    /// 返回该解析策略会尝试的所有候选路径（用于 debug/dry-run）。
    pub fn candidates(&self, scenario: &str, backend: Backend) -> Vec<PathBuf> {
        vec![
            self.cassette_dir
                .join(format!("{}-{}.jsonl", scenario, backend.as_config_str())),
            self.cassette_dir.join(format!("{}.jsonl", scenario)),
        ]
    }

    /// 返回 cassette 目录。
    pub fn cassette_dir(&self) -> &Path {
        &self.cassette_dir
    }
}

/// 构造 `mock-cli` 子命令的参数列表（用于写入 scenario workspace 的 `ralph.yml`）。
pub fn build_mock_cli_args(cassette_path: &Path, config: &MockConfig) -> Vec<String> {
    let mut args = vec![
        "mock-cli".to_string(),
        "--cassette".to_string(),
        cassette_path.to_string_lossy().to_string(),
    ];

    if config.speed > 0.0 {
        args.push("--speed".to_string());
        args.push(config.speed.to_string());
    }

    if let Some(allow) = &config.allow_commands {
        args.push("--allow".to_string());
        args.push(allow.clone());
    }

    args
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_cassette(dir: &Path, name: &str) {
        let cassette_path = dir.join(name);
        fs::write(
            &cassette_path,
            r#"{"ts":1000,"event":"ux.terminal.write","data":{"bytes":"UE9ORw==","stdout":true,"offset_ms":0}}"#,
        )
        .unwrap();
    }

    #[test]
    fn test_resolver_finds_backend_specific() {
        let temp = TempDir::new().unwrap();
        let cassette_dir = temp.path().join("cassettes/e2e");
        fs::create_dir_all(&cassette_dir).unwrap();

        create_test_cassette(&cassette_dir, "connect-claude.jsonl");
        create_test_cassette(&cassette_dir, "connect.jsonl");

        let resolver = CassetteResolver::new(&cassette_dir);
        let path = resolver.resolve("connect", Backend::Claude).unwrap();

        assert!(path.ends_with("connect-claude.jsonl"));
    }

    #[test]
    fn test_resolver_falls_back_to_generic() {
        let temp = TempDir::new().unwrap();
        let cassette_dir = temp.path().join("cassettes/e2e");
        fs::create_dir_all(&cassette_dir).unwrap();

        create_test_cassette(&cassette_dir, "connect.jsonl");

        let resolver = CassetteResolver::new(&cassette_dir);
        let path = resolver.resolve("connect", Backend::Kiro).unwrap();

        assert!(path.ends_with("connect.jsonl"));
    }

    #[test]
    fn test_resolver_returns_error_when_missing() {
        let temp = TempDir::new().unwrap();
        let cassette_dir = temp.path().join("cassettes/e2e");
        fs::create_dir_all(&cassette_dir).unwrap();

        let resolver = CassetteResolver::new(&cassette_dir);
        let result = resolver.resolve("nonexistent", Backend::Claude);

        assert!(matches!(result, Err(CassetteError::NotFound { .. })));

        if let Err(CassetteError::NotFound { tried, .. }) = result {
            assert_eq!(tried.len(), 2);
        }
    }

    #[test]
    fn test_resolver_candidates() {
        let resolver = CassetteResolver::new("/cassettes/e2e");
        let candidates = resolver.candidates("connect", Backend::Claude);

        assert_eq!(candidates.len(), 2);
        assert!(
            candidates[0]
                .to_string_lossy()
                .contains("connect-claude.jsonl")
        );
        assert!(candidates[1].to_string_lossy().contains("connect.jsonl"));
    }

    #[test]
    fn test_mock_config_defaults() {
        let config = MockConfig::default();

        assert_eq!(config.cassette_dir, PathBuf::from(DEFAULT_CASSETTE_DIR));
        assert!((config.speed - 0.0).abs() < f32::EPSILON);
        assert!(config.allow_commands.is_some());
    }

    #[test]
    fn test_mock_config_builder() {
        let config = MockConfig::new("/custom/cassettes")
            .with_speed(10.0)
            .with_allow_commands("ralph task add");

        assert_eq!(config.cassette_dir, PathBuf::from("/custom/cassettes"));
        assert!((config.speed - 10.0).abs() < f32::EPSILON);
        assert_eq!(config.allow_commands, Some("ralph task add".into()));
    }

    #[test]
    fn test_mock_config_negative_speed_clamped() {
        let config = MockConfig::default().with_speed(-5.0);
        assert!((config.speed - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_build_mock_cli_args() {
        let config = MockConfig::default().with_speed(10.0);
        let cassette = PathBuf::from("/path/to/cassette.jsonl");

        let args = build_mock_cli_args(&cassette, &config);

        assert!(args.contains(&"mock-cli".to_string()));
        assert!(args.contains(&"--cassette".to_string()));
        assert!(args.contains(&"/path/to/cassette.jsonl".to_string()));
        assert!(args.contains(&"--speed".to_string()));
        assert!(args.contains(&"10".to_string()));
        assert!(args.contains(&"--allow".to_string()));
    }

    #[test]
    fn test_build_mock_cli_args_instant() {
        let config = MockConfig::default(); // speed = 0.0
        let cassette = PathBuf::from("/path/to/cassette.jsonl");

        let args = build_mock_cli_args(&cassette, &config);

        // Should not include --speed when speed is 0 (instant)
        assert!(!args.contains(&"--speed".to_string()));
    }

    #[test]
    fn test_build_mock_cli_args_no_commands() {
        let config = MockConfig::default().without_commands();
        let cassette = PathBuf::from("/path/to/cassette.jsonl");

        let args = build_mock_cli_args(&cassette, &config);

        assert!(!args.contains(&"--allow".to_string()));
    }

    /// Test that MockConfig::resolve_cassette_dir returns an absolute path
    /// even when initialized with a relative path.
    ///
    /// This tests the fix for the bug where cassette resolution fails when
    /// running from a directory other than the workspace root.
    #[test]
    fn test_mock_config_resolve_cassette_dir_returns_absolute_path() {
        // Create a mock config with the default relative path
        let config = MockConfig::default();

        // The resolved cassette directory should be absolute
        let resolved = config.resolve_cassette_dir();
        assert!(
            resolved.is_absolute(),
            "resolve_cassette_dir() should return an absolute path, got: {}",
            resolved.display()
        );
    }

    /// Test that CassetteResolver works correctly with workspace-relative paths
    /// when the cassette directory is resolved relative to workspace root.
    #[test]
    fn test_resolver_with_workspace_relative_path() {
        let temp = TempDir::new().unwrap();

        // Simulate a workspace structure
        let workspace_root = temp.path();
        let cassette_dir = workspace_root.join("cassettes/e2e");
        fs::create_dir_all(&cassette_dir).unwrap();
        create_test_cassette(&cassette_dir, "connect.jsonl");

        // Create a Cargo.toml with [workspace] to make this a workspace root
        fs::write(
            workspace_root.join("Cargo.toml"),
            "[workspace]\nmembers = []\n",
        )
        .unwrap();

        // Create a MockConfig and resolve the cassette directory
        let config = MockConfig::default().with_workspace_root(workspace_root);
        let resolved_dir = config.resolve_cassette_dir();

        // The resolved directory should be the absolute path to cassettes/e2e
        assert_eq!(resolved_dir, cassette_dir);

        // Now verify the resolver can find cassettes using this resolved path
        let resolver = CassetteResolver::new(&resolved_dir);
        let result = resolver.resolve("connect", Backend::Claude);
        assert!(
            result.is_ok(),
            "Should find cassette when using resolved absolute path"
        );
    }
}
