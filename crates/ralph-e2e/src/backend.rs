//! Backend detection and authentication checking.
//!
//! This module provides functionality to detect which AI backends are available
//! and whether they are properly authenticated.

use std::fmt;
use std::time::Duration;

/// Supported AI backends for E2E testing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Backend {
    /// Claude CLI backend
    Claude,
    /// Kiro CLI backend
    Kiro,
    /// Codex CLI backend
    Codex,
    /// OpenCode CLI backend
    OpenCode,
}

impl Backend {
    /// Returns the CLI command name for this backend.
    pub fn command(&self) -> &'static str {
        match self {
            Backend::Claude => "claude",
            Backend::Kiro => "kiro-cli",
            Backend::Codex => "codex",
            Backend::OpenCode => "opencode",
        }
    }

    /// Returns all available backends.
    pub fn all() -> &'static [Backend] {
        &[
            Backend::Claude,
            Backend::Kiro,
            Backend::Codex,
            Backend::OpenCode,
        ]
    }

    /// Returns the default timeout for this backend.
    pub fn default_timeout(&self) -> Duration {
        match self {
            Backend::Claude => Duration::from_secs(600), // 10 minutes - Claude iterations can take 60-120s each
            Backend::Codex => Duration::from_secs(600), // 10 minutes - Codex runs can be slow (network + model)
            Backend::Kiro | Backend::OpenCode => Duration::from_secs(300), // 5 minutes
        }
    }

    /// Returns the default max iterations for this backend.
    pub fn default_max_iterations(&self) -> u32 {
        match self {
            Backend::Claude => 5, // Extra buffer for LLM non-determinism
            Backend::Codex => 5,  // Similar buffer for Codex variability
            Backend::Kiro | Backend::OpenCode => 3,
        }
    }

    /// Returns the backend name in lowercase (for config files).
    pub fn as_config_str(&self) -> &'static str {
        match self {
            Backend::Claude => "claude",
            Backend::Kiro => "kiro",
            Backend::Codex => "codex",
            Backend::OpenCode => "opencode",
        }
    }
}

impl fmt::Display for Backend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Backend::Claude => write!(f, "Claude"),
            Backend::Kiro => write!(f, "Kiro"),
            Backend::Codex => write!(f, "Codex"),
            Backend::OpenCode => write!(f, "OpenCode"),
        }
    }
}
