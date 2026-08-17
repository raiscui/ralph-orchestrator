//! Configuration types for the Ralph Orchestrator.
//!
//! This module supports both v1.x flat configuration format and v2.0 nested format.
//! Users can switch from Python v1.x to Rust v2.0 with zero config changes.

use crate::{
    AgentCliRecoverableFailuresConfig, DEFAULT_RECOVERABLE_FAILURE_LEDGER_PATH,
    is_reserved_hat_trigger,
};
pub use ralph_proto::WorkspaceStrategy;
use ralph_proto::{Topic, TopicContract};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tracing::debug;

/// Top-level configuration for Ralph Orchestrator.
///
/// Supports both v1.x flat format and v2.0 nested format:
/// - v1: `agent: claude`, `max_iterations: 100`
/// - v2: `cli: { backend: claude }`, `event_loop: { max_iterations: 100 }`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)] // Configuration struct with multiple feature flags
pub struct RalphConfig {
    /// Event loop configuration (v2 nested style).
    #[serde(default)]
    pub event_loop: EventLoopConfig,

    /// CLI backend configuration (v2 nested style).
    #[serde(default)]
    pub cli: CliConfig,

    /// Core paths and settings shared across all hats.
    #[serde(default)]
    pub core: CoreConfig,

    /// Custom hat definitions (optional).
    /// If empty, default planner and builder hats are used.
    #[serde(default)]
    pub hats: HashMap<String, HatConfig>,

    /// Event metadata definitions (optional).
    /// Defines what each event topic means, enabling auto-derived instructions.
    /// If a hat uses custom events, define them here for proper behavior injection.
    #[serde(default)]
    pub events: HashMap<String, EventMetadata>,

    /// Parallel hat instance runtime configuration.
    ///
    /// 默认关闭（保持现有“单执行者串行”行为）。
    #[serde(default)]
    pub parallel: ParallelConfig,

    /// Agent CLI 可恢复失败 retry 配置。
    ///
    /// 说明:
    /// - 该配置是顶层 runtime policy,不归属于某个具体 hat 或 adapter。
    /// - `.ralph/recoverable-failures.jsonl` 路径仍由 `CoreConfig` 统一解析,这里不提供第二个路径字段。
    #[serde(default)]
    pub agent_cli_recoverable_failures: AgentCliRecoverableFailuresConfig,

    // ─────────────────────────────────────────────────────────────────────────
    // V1 COMPATIBILITY FIELDS (flat format)
    // These map to nested v2 fields for backwards compatibility.
    // ─────────────────────────────────────────────────────────────────────────
    /// V1 field: Backend CLI (maps to cli.backend).
    /// Values: "claude", "kiro", "gemini", "codex", "amp", "auto", or "custom".
    #[serde(default)]
    pub agent: Option<String>,

    /// V1 field: Fallback order for auto-detection.
    #[serde(default)]
    pub agent_priority: Vec<String>,

    /// V1 field: Path to prompt file (maps to `event_loop.prompt_file`).
    #[serde(default)]
    pub prompt_file: Option<String>,

    /// V1 field: Completion detection string (maps to event_loop.completion_promise).
    #[serde(default)]
    pub completion_promise: Option<String>,

    /// V1 field: Maximum loop iterations (maps to event_loop.max_iterations).
    #[serde(default)]
    pub max_iterations: Option<u32>,

    /// V1 field: Maximum runtime in seconds (maps to event_loop.max_runtime_seconds).
    #[serde(default)]
    pub max_runtime: Option<u64>,

    /// V1 field: Maximum cost in USD (maps to event_loop.max_cost_usd).
    #[serde(default)]
    pub max_cost: Option<f64>,

    // ─────────────────────────────────────────────────────────────────────────
    // FEATURE FLAGS
    // ─────────────────────────────────────────────────────────────────────────
    /// Enable verbose output.
    #[serde(default)]
    pub verbose: bool,

    /// Archive prompts after completion (DEFERRED: warn if enabled).
    #[serde(default)]
    pub archive_prompts: bool,

    /// Enable metrics collection (DEFERRED: warn if enabled).
    #[serde(default)]
    pub enable_metrics: bool,

    // ─────────────────────────────────────────────────────────────────────────
    // DROPPED FIELDS (accepted but ignored with warning)
    // ─────────────────────────────────────────────────────────────────────────
    /// V1 field: Token limits (DROPPED: controlled by CLI tool).
    #[serde(default)]
    pub max_tokens: Option<u32>,

    /// V1 field: Retry delay (DROPPED: handled differently in v2).
    #[serde(default)]
    pub retry_delay: Option<u32>,

    /// V1 adapter settings (partially supported).
    #[serde(default)]
    pub adapters: AdaptersConfig,

    // ─────────────────────────────────────────────────────────────────────────
    // WARNING CONTROL
    // ─────────────────────────────────────────────────────────────────────────
    /// Suppress all warnings (for CI environments).
    #[serde(default, rename = "_suppress_warnings")]
    pub suppress_warnings: bool,

    /// TUI configuration.
    #[serde(default)]
    pub tui: TuiConfig,

    /// Memories configuration for persistent learning across sessions.
    #[serde(default)]
    pub memories: MemoriesConfig,

    /// Tasks configuration for runtime work tracking.
    #[serde(default)]
    pub tasks: TasksConfig,
}

// ============================================================================
// Parallel Hat Instances (experimental)
// ============================================================================

fn default_gate_timeout_secs() -> u64 {
    60
}

fn default_worktree_base_dir() -> String {
    ".ralph/worktrees".to_string()
}

/// worktree 的获取方式（影响 sandbox 兼容性与产物可搬运性）。
///
/// 说明：
/// - `worktree`：使用 `git worktree add/remove`。
///   - 优点：创建快、磁盘占用小、commit 天然在同一仓库里（易 cherry-pick）。
///   - 缺点：workdir 的 `.git` 会指向上级仓库的 `.git/worktrees/...`。
///     - 某些工具沙箱只允许写当前目录,会导致 `git commit` 报 `index.lock` 无法创建。
/// - `clone`：使用 `git clone` 创建独立 `.git`。
///   - 优点：更兼容“只能写当前目录”的 sandbox。
///   - 代价：release 时需要把 HEAD 引入主仓库(否则 integrator 无法按 hash cherry-pick)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WorktreeBackend {
    /// 使用 `git worktree add/remove`。
    #[default]
    Worktree,
    /// 使用 `git clone`(独立 `.git`)。
    Clone,
}

/// Permission mode for orchestrator-managed actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PermissionMode {
    /// 允许直接执行（默认）。
    #[default]
    Allow,
    /// 需要 human gate 确认（异步）。
    Ask,
    /// 禁止执行。
    Deny,
}

/// Permission policy configuration (high-risk actions).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PermissionsConfig {
    /// 创建/切换到 worktree 的权限。
    #[serde(default)]
    pub worktree: PermissionMode,

    /// 执行 workspace hooks（on_acquire/on_release）的权限。
    #[serde(default)]
    pub hooks: PermissionMode,
}

/// Human gate configuration defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateConfig {
    /// 默认超时秒数（0 表示不超时）。
    #[serde(default = "default_gate_timeout_secs")]
    pub default_timeout_secs: u64,
}

impl Default for GateConfig {
    fn default() -> Self {
        Self {
            default_timeout_secs: default_gate_timeout_secs(),
        }
    }
}

/// Workspace runtime configuration for parallel mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceRuntimeConfig {
    /// worktree 的落盘目录（相对仓库根目录）。
    #[serde(default = "default_worktree_base_dir")]
    pub worktree_base_dir: String,

    /// worktree 的获取方式（见 `WorktreeBackend` 注释）。
    #[serde(default)]
    pub worktree_backend: WorktreeBackend,
}

impl Default for WorkspaceRuntimeConfig {
    fn default() -> Self {
        Self {
            worktree_base_dir: default_worktree_base_dir(),
            worktree_backend: WorktreeBackend::default(),
        }
    }
}

fn default_parallel_max_running_jobs() -> usize {
    4
}

fn default_parallel_dynamic_idle_ttl_secs() -> u64 {
    30
}

/// Autoscale configuration for parallel mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelAutoscaleConfig {
    /// 全局并发上限：限制同时 Running 的 job 数量（安全刹车）。
    #[serde(default = "default_parallel_max_running_jobs")]
    pub max_running_jobs: usize,

    /// 动态实例 idle 超过该秒数后自动回收。
    #[serde(default = "default_parallel_dynamic_idle_ttl_secs")]
    pub dynamic_idle_ttl_secs: u64,
}

impl Default for ParallelAutoscaleConfig {
    fn default() -> Self {
        Self {
            max_running_jobs: default_parallel_max_running_jobs(),
            dynamic_idle_ttl_secs: default_parallel_dynamic_idle_ttl_secs(),
        }
    }
}

/// Parallel runtime configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParallelConfig {
    /// 是否启用并行 HatInstance 运行时。
    #[serde(default)]
    pub enabled: bool,

    /// Autoscale（默认开启，含全局 cap 与 idle 回收）。
    #[serde(default)]
    pub autoscale: ParallelAutoscaleConfig,

    /// Topic contracts（按 topic pattern 匹配）。
    ///
    /// 说明：
    /// - key 支持 `*` 通配符分段匹配（例如 `build.*`）
    /// - value 定义 delivery/queue_selection/missing_policy 等路由语义
    #[serde(default)]
    pub topic_contracts: HashMap<String, TopicContract>,

    /// Human gate 默认配置。
    #[serde(default)]
    pub gate: GateConfig,

    /// Workspace 运行时配置。
    #[serde(default)]
    pub workspace: WorkspaceRuntimeConfig,

    /// 权限策略（高风险操作）。
    #[serde(default)]
    pub permissions: PermissionsConfig,
}

fn default_true() -> bool {
    true
}

#[allow(clippy::derivable_impls)] // Cannot derive due to serde default functions
impl Default for RalphConfig {
    fn default() -> Self {
        Self {
            event_loop: EventLoopConfig::default(),
            cli: CliConfig::default(),
            core: CoreConfig::default(),
            hats: HashMap::new(),
            events: HashMap::new(),
            parallel: ParallelConfig::default(),
            agent_cli_recoverable_failures: AgentCliRecoverableFailuresConfig::default(),
            // V1 compatibility fields
            agent: None,
            agent_priority: vec![],
            prompt_file: None,
            completion_promise: None,
            max_iterations: None,
            max_runtime: None,
            max_cost: None,
            // Feature flags
            verbose: false,
            archive_prompts: false,
            enable_metrics: false,
            // Dropped fields
            max_tokens: None,
            retry_delay: None,
            adapters: AdaptersConfig::default(),
            // Warning control
            suppress_warnings: false,
            // TUI
            tui: TuiConfig::default(),
            // Memories
            memories: MemoriesConfig::default(),
            // Tasks
            tasks: TasksConfig::default(),
        }
    }
}

/// V1 adapter settings per backend.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AdaptersConfig {
    /// Claude adapter settings.
    #[serde(default)]
    pub claude: AdapterSettings,

    /// Gemini adapter settings.
    #[serde(default)]
    pub gemini: AdapterSettings,

    /// Kiro adapter settings.
    #[serde(default)]
    pub kiro: AdapterSettings,

    /// Codex adapter settings.
    #[serde(default)]
    pub codex: AdapterSettings,

    /// Amp adapter settings.
    #[serde(default)]
    pub amp: AdapterSettings,
}

/// Per-adapter settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterSettings {
    /// 执行超时“检测窗口”（秒）。
    ///
    /// 说明：
    /// - 这是“检测超时”，不是“硬超时”：
    ///   - 当窗口到期时，并不会立刻终止进程；
    ///   - 会再判断输出是否在 `output_stale_timeout_secs` 内持续变化：
    ///     - 若输出已停滞超过阈值：判定超时并终止
    ///     - 若输出仍在变化：判定通过，并把检测窗口重新计时
    /// - 这么做的目的：允许长任务持续运行，只要它确实在产出进展信号，同时避免无人值守卡死。
    #[serde(default = "default_timeout")]
    pub timeout: u64,

    /// 输出停滞阈值（秒）。
    ///
    /// 当 `timeout` 的检测窗口到期时，如果 stdout/stderr 输出在该阈值内没有任何变化，
    /// 则判定进程已“卡住”，会被终止。
    #[serde(default = "default_output_stale_timeout_secs")]
    pub output_stale_timeout_secs: u64,

    /// 该 backend 的上下文窗口大小（tokens）。
    ///
    /// 说明：
    /// - 这是一个“预算/护栏”配置,主要用于 `ralph doctor` 在运行前做上下文窗检查,
    ///   避免在小窗模型上无谓地启动长工作流(浪费时间与 token)。
    /// - Ralph 不会把这个值直接传递给后端 CLI,它只是用于预检与提示。
    /// - 若未配置(None),doctor 会跳过窗口检查(保持默认兼容)。
    #[serde(default)]
    pub context_window_tokens: Option<u32>,

    /// Include in auto-detection.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Tool permissions (DROPPED: CLI tool manages its own permissions).
    #[serde(default)]
    pub tool_permissions: Option<Vec<String>>,
}

fn default_timeout() -> u64 {
    3600 // 1 hour
}

fn default_output_stale_timeout_secs() -> u64 {
    1800 // 30 minutes
}

impl Default for AdapterSettings {
    fn default() -> Self {
        Self {
            timeout: default_timeout(),
            output_stale_timeout_secs: default_output_stale_timeout_secs(),
            context_window_tokens: None,
            enabled: true,
            tool_permissions: None,
        }
    }
}

impl RalphConfig {
    /// Resolves the context window token limit for the active backend.
    ///
    /// 说明：
    /// - Reads `adapters.<backend>.context_window_tokens` if explicitly set.
    /// - Returns 0 if unset (signals: no telemetry / suppress suffix)。
    /// - Caller uses this to initialize `PtyExecutor::set_context_window`。
    pub fn resolve_context_window(&self, backend: &str) -> u64 {
        // 说明：
        // - AdaptersConfig 是结构体, 不是 HashMap, 用 match 直接字段访问
        // - 没显式配置时返回 0 (suppress suffix)
        let settings = match backend {
            "claude" => Some(&self.adapters.claude),
            "gemini" => Some(&self.adapters.gemini),
            "kiro" => Some(&self.adapters.kiro),
            "codex" => Some(&self.adapters.codex),
            "amp" => Some(&self.adapters.amp),
            _ => None,
        };
        if let Some(s) = settings {
            if let Some(tokens) = s.context_window_tokens {
                return tokens as u64;
            }
        }
        0
    }

    /// Loads configuration from a YAML file.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path_ref = path.as_ref();
        debug!(path = %path_ref.display(), "Loading configuration from file");
        let content = std::fs::read_to_string(path_ref)?;
        // 说明：
        // - Hat imports 在 serde 解析到 RalphConfig 之前解决。
        // - 仅本地 file source 才允许 imports; builtin/remote 应在它们自己的 source 入口拒绝。
        let base_dir = path_ref.parent().unwrap_or_else(|| Path::new("."));
        let resolved = Self::resolve_hat_imports(&content, base_dir, &path_ref.display().to_string())?;
        Self::parse_yaml(&resolved)
    }

    /// 解析 hat imports（本地 file source only）.
    ///
    /// 说明:
    /// - builtin / remote source 的 imports 拒绝交给它们自己的 source 入口处理,
    ///   本函数只处理本地 file source.
    pub(crate) fn resolve_hat_imports(
        content: &str,
        base_dir: &Path,
        source_label: &str,
    ) -> std::result::Result<String, ConfigError> {
        use crate::hat_imports::resolve_hat_imports_in_mapping;
        let mut mapping: serde_yaml::Mapping = serde_yaml::from_str(content)?;
        resolve_hat_imports_in_mapping(&mut mapping, base_dir, source_label)
            .map_err(|e| ConfigError::HatImport(e.to_string()))?;
        serde_yaml::to_string(&mapping).map_err(|e| ConfigError::Yaml(e.into()))
    }

    /// Parses configuration from a YAML string.
    pub fn parse_yaml(content: &str) -> Result<Self, ConfigError> {
        let config: Self = serde_yaml::from_str(content)?;
        debug!(
            backend = %config.cli.backend,
            has_v1_fields = config.agent.is_some(),
            custom_hats = config.hats.len(),
            "Configuration loaded"
        );
        Ok(config)
    }

    /// Normalizes v1 flat fields into v2 nested structure.
    ///
    /// V1 flat fields take precedence over v2 nested fields when both are present.
    /// This allows users to use either format or mix them.
    pub fn normalize(&mut self) {
        let mut normalized_count = 0;

        // Map v1 `agent` to v2 `cli.backend`
        if let Some(ref agent) = self.agent {
            debug!(from = "agent", to = "cli.backend", value = %agent, "Normalizing v1 field");
            self.cli.backend = agent.clone();
            normalized_count += 1;
        }

        // Map v1 `prompt_file` to v2 `event_loop.prompt_file`
        if let Some(ref pf) = self.prompt_file {
            debug!(from = "prompt_file", to = "event_loop.prompt_file", value = %pf, "Normalizing v1 field");
            self.event_loop.prompt_file = pf.clone();
            normalized_count += 1;
        }

        // Map v1 `completion_promise` to v2 `event_loop.completion_promise`
        if let Some(ref cp) = self.completion_promise {
            debug!(
                from = "completion_promise",
                to = "event_loop.completion_promise",
                "Normalizing v1 field"
            );
            self.event_loop.completion_promise = cp.clone();
            normalized_count += 1;
        }

        // Map v1 `max_iterations` to v2 `event_loop.max_iterations`
        if let Some(mi) = self.max_iterations {
            debug!(
                from = "max_iterations",
                to = "event_loop.max_iterations",
                value = mi,
                "Normalizing v1 field"
            );
            self.event_loop.max_iterations = mi;
            normalized_count += 1;
        }

        // Map v1 `max_runtime` to v2 `event_loop.max_runtime_seconds`
        if let Some(mr) = self.max_runtime {
            debug!(
                from = "max_runtime",
                to = "event_loop.max_runtime_seconds",
                value = mr,
                "Normalizing v1 field"
            );
            self.event_loop.max_runtime_seconds = mr;
            normalized_count += 1;
        }

        // Map v1 `max_cost` to v2 `event_loop.max_cost_usd`
        if self.max_cost.is_some() {
            debug!(
                from = "max_cost",
                to = "event_loop.max_cost_usd",
                "Normalizing v1 field"
            );
            self.event_loop.max_cost_usd = self.max_cost;
            normalized_count += 1;
        }

        if normalized_count > 0 {
            debug!(
                fields_normalized = normalized_count,
                "V1 to V2 config normalization complete"
            );
        }
    }

    /// Validates the configuration and returns warnings.
    ///
    /// This method checks for:
    /// - Deferred features that are enabled (archive_prompts, enable_metrics)
    /// - Dropped fields that are present (max_tokens, retry_delay, tool_permissions)
    /// - Ambiguous trigger routing across custom hats
    /// - Mutual exclusivity of prompt and prompt_file
    ///
    /// Returns a list of warnings that should be displayed to the user.
    pub fn validate(&self) -> Result<Vec<ConfigWarning>, ConfigError> {
        let mut warnings = Vec::new();
        let warnings_enabled = !self.suppress_warnings;

        // Check for mutual exclusivity of prompt and prompt_file in config
        // Only error if both are explicitly set (not defaults)
        if self.event_loop.prompt.is_some()
            && !self.event_loop.prompt_file.is_empty()
            && self.event_loop.prompt_file != default_prompt_file()
        {
            return Err(ConfigError::MutuallyExclusive {
                field1: "event_loop.prompt".to_string(),
                field2: "event_loop.prompt_file".to_string(),
            });
        }

        // Validate complete_publishes is non-empty when set
        if let Some(ref topic) = self.event_loop.complete_publishes
            && topic.trim().is_empty()
        {
            return Err(ConfigError::InvalidValue {
                field: "event_loop.complete_publishes".to_string(),
                message: "must not be empty".to_string(),
            });
        }

        // Hard gate: when using custom hats, `complete_publishes` 必须有“明确发布者”。
        //
        // 原因：
        // - `complete_publishes` 是收敛候选事件 topic；
        // - 如果没有任何 hat 声明发布它，收敛信号的“生产者”会变成隐式约定，
        //   容易造成 workflow 卡死或拓扑图出现悬空终点。
        if let Some(topic) = self.event_loop.complete_publishes.as_deref() {
            let topic = topic.trim();
            if !topic.is_empty() && !self.hats.is_empty() {
                let has_publisher = self
                    .hats
                    .values()
                    .any(|hat| hat.publishes.iter().any(|p| p == topic));
                if !has_publisher {
                    return Err(ConfigError::InvalidValue {
                        field: "event_loop.complete_publishes".to_string(),
                        message: format!(
                            "topic `{topic}` must be declared in at least one hat's `publishes` (e.g. `hats.<hat_id>.publishes`)"
                        ),
                    });
                }
            }
        }

        // Check custom backend has a command
        if self.cli.backend == "custom" && self.cli.command.as_ref().is_none_or(String::is_empty) {
            return Err(ConfigError::CustomBackendRequiresCommand);
        }

        // Check all-hat overlay configuration is internally consistent.
        match &self.core.all_hat_prompt {
            AllHatPromptConfig::Compiled | AllHatPromptConfig::Disabled => {}
            AllHatPromptConfig::Inline { text } => {
                if text.trim().is_empty() {
                    return Err(ConfigError::InvalidValue {
                        field: "core.all_hat_prompt.text".to_string(),
                        message: "must not be empty when core.all_hat_prompt.mode=inline"
                            .to_string(),
                    });
                }
            }
            AllHatPromptConfig::File { path } => {
                let trimmed = path.trim();
                if trimmed.is_empty() {
                    return Err(ConfigError::InvalidValue {
                        field: "core.all_hat_prompt.path".to_string(),
                        message: "must not be empty when core.all_hat_prompt.mode=file".to_string(),
                    });
                }

                let resolved = self.core.resolve_path(trimmed);
                if !resolved.is_file() {
                    return Err(ConfigError::InvalidValue {
                        field: "core.all_hat_prompt.path".to_string(),
                        message: format!(
                            "resolved file does not exist or is not a file: {}",
                            resolved.display()
                        ),
                    });
                }

                if let Err(error) = std::fs::read_to_string(&resolved) {
                    return Err(ConfigError::InvalidValue {
                        field: "core.all_hat_prompt.path".to_string(),
                        message: format!(
                            "failed to read configured overlay file {}: {error}",
                            resolved.display()
                        ),
                    });
                }
            }
        }

        // Agent CLI recoverable retry policy 必须保持有界且可解释。
        //
        // 说明：
        // - 即使 `enabled=false`,配置文件中出现的非法数值也应该尽早暴露。
        // - 后续 runtime wiring 可以直接信任这里的基本不变量。
        let recoverable = &self.agent_cli_recoverable_failures;
        if recoverable.max_attempts == 0 {
            return Err(ConfigError::InvalidValue {
                field: "agent_cli_recoverable_failures.max_attempts".to_string(),
                message: "must be at least 1".to_string(),
            });
        }
        if recoverable.initial_delay_ms == 0 {
            return Err(ConfigError::InvalidValue {
                field: "agent_cli_recoverable_failures.initial_delay_ms".to_string(),
                message: "must be greater than 0".to_string(),
            });
        }
        if recoverable.max_delay_ms == 0 {
            return Err(ConfigError::InvalidValue {
                field: "agent_cli_recoverable_failures.max_delay_ms".to_string(),
                message: "must be greater than 0".to_string(),
            });
        }
        if recoverable.max_delay_ms < recoverable.initial_delay_ms {
            return Err(ConfigError::InvalidValue {
                field: "agent_cli_recoverable_failures.max_delay_ms".to_string(),
                message: "must be greater than or equal to initial_delay_ms".to_string(),
            });
        }
        if !recoverable.backoff_multiplier.is_finite() || recoverable.backoff_multiplier < 1.0 {
            return Err(ConfigError::InvalidValue {
                field: "agent_cli_recoverable_failures.backoff_multiplier".to_string(),
                message: "must be finite and greater than or equal to 1.0".to_string(),
            });
        }

        // Check for deferred features
        if warnings_enabled && self.archive_prompts {
            warnings.push(ConfigWarning::DeferredFeature {
                field: "archive_prompts".to_string(),
                message: "Feature not yet available in v2".to_string(),
            });
        }

        if warnings_enabled && self.enable_metrics {
            warnings.push(ConfigWarning::DeferredFeature {
                field: "enable_metrics".to_string(),
                message: "Feature not yet available in v2".to_string(),
            });
        }

        // Check for dropped fields
        if warnings_enabled && self.max_tokens.is_some() {
            warnings.push(ConfigWarning::DroppedField {
                field: "max_tokens".to_string(),
                reason: "Token limits are controlled by the CLI tool".to_string(),
            });
        }

        if warnings_enabled && self.retry_delay.is_some() {
            warnings.push(ConfigWarning::DroppedField {
                field: "retry_delay".to_string(),
                reason: "Retry logic handled differently in v2".to_string(),
            });
        }

        // Check adapter tool_permissions (dropped field)
        if warnings_enabled
            && (self.adapters.claude.tool_permissions.is_some()
                || self.adapters.gemini.tool_permissions.is_some()
                || self.adapters.codex.tool_permissions.is_some()
                || self.adapters.amp.tool_permissions.is_some())
        {
            warnings.push(ConfigWarning::DroppedField {
                field: "adapters.*.tool_permissions".to_string(),
                reason: "CLI tool manages its own permissions".to_string(),
            });
        }

        // Check for required description field on all hats
        for (hat_id, hat_config) in &self.hats {
            if hat_config
                .description
                .as_ref()
                .is_none_or(|d| d.trim().is_empty())
            {
                return Err(ConfigError::MissingDescription {
                    hat: hat_id.clone(),
                });
            }
        }

        // Check for reserved runtime/control triggers.
        //
        // Per design:
        // - Ralph handles runtime entry/control topics first.
        // - Ordinary hats should receive delegated workflow events, not runtime handshakes.
        // - The classification lives in event_emission_protocol.rs so prompt/runtime/config
        //   不能各维护一套不同列表。
        for (hat_id, hat_config) in &self.hats {
            for trigger in &hat_config.triggers {
                if is_reserved_hat_trigger(trigger) {
                    return Err(ConfigError::ReservedTrigger {
                        trigger: trigger.clone(),
                        hat: hat_id.clone(),
                    });
                }
            }
        }

        // Check for ambiguous routing: each trigger topic must map to exactly one hat.
        //
        // 说明：
        // - 串行 hats（默认）下：同一 trigger 只能由一个 hat 处理，否则 orchestrator 无法确定路由。
        // - 并行模式（parallel.enabled=true）下：允许多个 hat 共享同一 trigger（例如 build.task fanout）。
        if !self.parallel.enabled && !self.hats.is_empty() {
            let mut trigger_to_hat: HashMap<&str, &str> = HashMap::new();
            for (hat_id, hat_config) in &self.hats {
                for trigger in &hat_config.triggers {
                    if let Some(existing_hat) = trigger_to_hat.get(trigger.as_str()) {
                        return Err(ConfigError::AmbiguousRouting {
                            trigger: trigger.clone(),
                            hat1: (*existing_hat).to_string(),
                            hat2: hat_id.clone(),
                        });
                    }
                    trigger_to_hat.insert(trigger.as_str(), hat_id.as_str());
                }
            }
        }

        Ok(warnings)
    }

    /// Gets the effective backend name, resolving "auto" using the priority list.
    pub fn effective_backend(&self) -> &str {
        &self.cli.backend
    }

    /// Returns the agent priority list for auto-detection.
    /// If empty, returns the default priority order.
    pub fn get_agent_priority(&self) -> Vec<&str> {
        if self.agent_priority.is_empty() {
            vec!["claude", "kiro", "gemini", "codex", "amp"]
        } else {
            self.agent_priority.iter().map(String::as_str).collect()
        }
    }

    /// Gets the adapter settings for a specific backend.
    #[allow(clippy::match_same_arms)] // Explicit match arms for each backend improves readability
    pub fn adapter_settings(&self, backend: &str) -> &AdapterSettings {
        match backend {
            "claude" => &self.adapters.claude,
            "gemini" => &self.adapters.gemini,
            "kiro" => &self.adapters.kiro,
            "codex" => &self.adapters.codex,
            "amp" => &self.adapters.amp,
            // 特殊规则：当 cli.backend=custom 时，按实际 command 推导 timeout profile。
            //
            // 目前我们只做最小映射：command=codex -> adapters.codex。
            // 其余 command 仍回退 claude（保持历史行为，避免误判未知命令）。
            "custom" => match self.cli.command.as_deref() {
                Some("codex") => &self.adapters.codex,
                _ => &self.adapters.claude,
            },
            _ => &self.adapters.claude, // Default fallback
        }
    }
}

/// Configuration warnings emitted during validation.
#[derive(Debug, Clone)]
pub enum ConfigWarning {
    /// Feature is enabled but not yet available in v2.
    DeferredFeature { field: String, message: String },
    /// Field is present but ignored in v2.
    DroppedField { field: String, reason: String },
    /// Field has an invalid value.
    InvalidValue { field: String, message: String },
}

impl std::fmt::Display for ConfigWarning {
    #[allow(clippy::match_same_arms)] // Different arms have different messages despite similar structure
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigWarning::DeferredFeature { field, message }
            | ConfigWarning::InvalidValue { field, message } => {
                write!(f, "Warning [{field}]: {message}")
            }
            ConfigWarning::DroppedField { field, reason } => {
                write!(f, "Warning [{field}]: Field ignored - {reason}")
            }
        }
    }
}

/// Event loop configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventLoopConfig {
    /// Inline prompt text (mutually exclusive with prompt_file).
    pub prompt: Option<String>,

    /// Extra prompt text injected ONLY for Ralph (the coordinator).
    ///
    /// 说明：
    /// - 该字段不参与 prompt precedence（不替代 `prompt/prompt_file`），而是“追加注入”。
    /// - 用于并行/复杂工作流里给 ralph#1 提供固定语义锚点，避免污染其他 hats 的输入。
    pub ralph_prompt: Option<String>,

    /// Path to the prompt file.
    #[serde(default = "default_prompt_file")]
    pub prompt_file: String,

    /// String that signals loop completion.
    #[serde(default = "default_completion_promise")]
    pub completion_promise: String,

    /// Maximum number of iterations before timeout.
    #[serde(default = "default_max_iterations")]
    pub max_iterations: u32,

    /// Maximum runtime in seconds.
    #[serde(default = "default_max_runtime")]
    pub max_runtime_seconds: u64,

    /// Maximum cost in USD before stopping.
    pub max_cost_usd: Option<f64>,

    /// Stop after this many consecutive failures.
    #[serde(default = "default_max_failures")]
    pub max_consecutive_failures: u32,

    /// Starting hat for multi-hat mode (deprecated, use starting_event instead).
    pub starting_hat: Option<String>,

    /// Event to publish after Ralph completes initial coordination.
    ///
    /// When custom hats are defined, Ralph handles `task.start` to do gap analysis
    /// and planning, then publishes this event to delegate to the first hat.
    ///
    /// Example: `starting_event: "tdd.start"` for TDD workflow.
    ///
    /// If not specified and hats are defined, Ralph will determine the appropriate
    /// event from the hat topology.
    pub starting_event: Option<String>,

    /// Workflow completion candidate event topic.
    ///
    /// When set, the coordinator (ralph#1) can treat receiving this topic as a
    /// signal that the workflow has reached a completion point, and may output
    /// `completion_promise` to end the run.
    ///
    /// Example: `complete_publishes: "fix.applied"`
    pub complete_publishes: Option<String>,
}

fn default_prompt_file() -> String {
    "PROMPT.md".to_string()
}

fn default_completion_promise() -> String {
    "LOOP_COMPLETE".to_string()
}

fn default_max_iterations() -> u32 {
    100
}

fn default_max_runtime() -> u64 {
    14400 // 4 hours
}

fn default_max_failures() -> u32 {
    5
}

impl Default for EventLoopConfig {
    fn default() -> Self {
        Self {
            prompt: None,
            ralph_prompt: None,
            prompt_file: default_prompt_file(),
            completion_promise: default_completion_promise(),
            max_iterations: default_max_iterations(),
            max_runtime_seconds: default_max_runtime(),
            max_cost_usd: None,
            max_consecutive_failures: default_max_failures(),
            starting_hat: None,
            starting_event: None,
            complete_publishes: None,
        }
    }
}

/// 所有 hat 共用 overlay 的来源配置。
///
/// 说明：
/// - 默认继续使用编译期内嵌 `config/all_hat.md`，保持现有行为不变。
/// - 当某些 example / E2E 需要降噪时，可以显式切到 `disabled` / `inline` / `file`。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum AllHatPromptConfig {
    /// 使用编译期内嵌的 `config/all_hat.md`。
    #[default]
    Compiled,
    /// 完全关闭 all-hat overlay 注入。
    Disabled,
    /// 直接在配置里提供轻量 overlay 文本。
    Inline { text: String },
    /// 从运行时文件读取 overlay 文本（相对路径按 workspace root 解析）。
    File { path: String },
}

/// Core paths and settings shared across all hats.
///
/// Per spec: "Core behaviors (always injected, can customize paths)"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreConfig {
    /// Path to the scratchpad file (shared state between hats).
    #[serde(default = "default_scratchpad")]
    pub scratchpad: String,

    /// Path to the specs directory (source of truth for requirements).
    #[serde(default = "default_specs_dir")]
    pub specs_dir: String,

    /// Guardrails injected into every prompt (core behaviors).
    ///
    /// Per spec: These are always present regardless of hat.
    #[serde(default = "default_guardrails")]
    pub guardrails: Vec<String>,

    /// 项目级 all-hat overlay 的来源配置。
    ///
    /// 说明：
    /// - 该 overlay 会注入所有 hat（包含 `ralph#1`）。
    /// - 默认保持编译期内嵌；仅在需要降噪或替换时显式覆写。
    #[serde(default)]
    pub all_hat_prompt: AllHatPromptConfig,

    /// 是否允许当前运行时注入 parent-visible runtime capability catalog / invoker。
    ///
    /// 说明:
    /// - 默认开启,这样正常 parent run 可以看到 capability catalog。
    /// - child capability execution 会显式关掉它,避免 prompt 递归注入同一套 capability。
    #[serde(default = "default_runtime_capabilities_enabled")]
    pub runtime_capabilities_enabled: bool,

    /// Root directory for workspace-relative paths (.agent/, memories, etc.).
    ///
    /// All relative paths (scratchpad, specs_dir, memories) are resolved relative
    /// to this directory. Defaults to the current working directory.
    ///
    /// This is especially important for E2E tests that run in isolated workspaces.
    #[serde(skip)]
    pub workspace_root: std::path::PathBuf,
}

fn default_scratchpad() -> String {
    ".agent/scratchpad.md".to_string()
}

fn default_specs_dir() -> String {
    "./specs/".to_string()
}

fn default_legacy_memories_path() -> &'static str {
    ".agent/memories.md"
}

fn default_project_experience_path() -> &'static str {
    "experience.md"
}

fn default_role_experience_root() -> &'static str {
    ".ralph/roles"
}

fn default_instance_context_root() -> &'static str {
    ".ralph/log"
}

fn default_guardrails() -> Vec<String> {
    vec![
        "Fresh context each iteration - scratchpad is memory".to_string(),
        "Don't assume 'not implemented' - search first".to_string(),
        "Backpressure is law - tests/typecheck/lint must pass".to_string(),
    ]
}

fn default_runtime_capabilities_enabled() -> bool {
    true
}

impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            scratchpad: default_scratchpad(),
            specs_dir: default_specs_dir(),
            guardrails: default_guardrails(),
            all_hat_prompt: AllHatPromptConfig::default(),
            runtime_capabilities_enabled: default_runtime_capabilities_enabled(),
            workspace_root: std::env::var("RALPH_WORKSPACE_ROOT")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|_| {
                    std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
                }),
        }
    }
}

impl CoreConfig {
    /// Sets the workspace root for resolving relative paths.
    ///
    /// This is used by E2E tests to point to their isolated test workspace.
    pub fn with_workspace_root(mut self, root: impl Into<std::path::PathBuf>) -> Self {
        self.workspace_root = root.into();
        self
    }

    /// Resolves a relative path against the workspace root.
    ///
    /// If the path is already absolute, it is returned as-is.
    /// Otherwise, it is joined with the workspace root.
    pub fn resolve_path(&self, relative: &str) -> std::path::PathBuf {
        let path = std::path::Path::new(relative);
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.workspace_root.join(path)
        }
    }

    /// Resolves the legacy `.agent/memories.md` compatibility path.
    ///
    /// 这条路径仍是当前实现的基线,后续 scoped experience 迁移也必须兼容它。
    #[must_use]
    pub fn resolve_legacy_memories_path(&self) -> std::path::PathBuf {
        self.resolve_path(default_legacy_memories_path())
    }

    /// Resolves the project-level `experience.md` path.
    #[must_use]
    pub fn resolve_project_experience_path(&self) -> std::path::PathBuf {
        self.resolve_path(default_project_experience_path())
    }

    /// Resolves the role directory for a given hat id.
    #[must_use]
    pub fn resolve_role_dir(&self, hat_id: &str) -> std::path::PathBuf {
        self.resolve_path(default_role_experience_root())
            .join(hat_id)
    }

    /// Resolves the role-level `experience.md` path for a given hat id.
    #[must_use]
    pub fn resolve_role_experience_path(&self, hat_id: &str) -> std::path::PathBuf {
        self.resolve_role_dir(hat_id).join("experience.md")
    }

    /// Resolves the instance context directory for a given instance id.
    #[must_use]
    pub fn resolve_instance_context_dir(&self, instance_id: &str) -> std::path::PathBuf {
        self.resolve_path(default_instance_context_root())
            .join(instance_id)
    }

    /// Resolves a file inside an instance context directory.
    #[must_use]
    pub fn resolve_instance_context_path(
        &self,
        instance_id: &str,
        file_name: &str,
    ) -> std::path::PathBuf {
        self.resolve_instance_context_dir(instance_id)
            .join(file_name)
    }

    /// Resolves the append-only recoverable agent CLI failure ledger path.
    ///
    /// 说明：
    /// - 这是 `.ralph/recoverable-failures.jsonl` 的唯一路径解析入口。
    /// - 其它 runtime 代码应调用这个方法,不要手写相同路径。
    #[must_use]
    pub fn resolve_recoverable_failures_ledger_path(&self) -> std::path::PathBuf {
        self.resolve_path(DEFAULT_RECOVERABLE_FAILURE_LEDGER_PATH)
    }
}

/// Role-aware model reasoning effort.
///
/// 说明:
/// - 这是 Ralph 的语义配置,不是某个具体 CLI 的原生命令行参数。
/// - Codex backend 会把它映射成 `model_reasoning_effort`。
/// - 其他 backend 若暂时没有等价概念,可以安全忽略这个字段。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffort {
    Low,
    Medium,
    High,
    #[serde(rename = "xhigh", alias = "x_high")]
    XHigh,
}

impl ReasoningEffort {
    /// Stable CLI/config spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::XHigh => "xhigh",
        }
    }
}

impl std::fmt::Display for ReasoningEffort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

fn default_coordinator_reasoning_effort() -> ReasoningEffort {
    ReasoningEffort::Medium
}

fn default_worker_reasoning_effort() -> ReasoningEffort {
    ReasoningEffort::High
}

/// Default reasoning-effort policy split by runtime role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleReasoningEffortConfig {
    /// Ralph/coordinator jobs should decide routing quickly, so default to medium.
    #[serde(default = "default_coordinator_reasoning_effort")]
    pub coordinator: ReasoningEffort,

    /// Worker hats do concrete execution/review, so default to high.
    #[serde(default = "default_worker_reasoning_effort")]
    pub worker: ReasoningEffort,
}

impl Default for RoleReasoningEffortConfig {
    fn default() -> Self {
        Self {
            coordinator: default_coordinator_reasoning_effort(),
            worker: default_worker_reasoning_effort(),
        }
    }
}

/// Extra CLI arguments split by runtime role.
///
/// 说明:
/// - 这是追加到 backend argv 的窄配置层,用于表达“coordinator 与 worker
///   需要不同 CLI 参数”的运行策略。
/// - 它不替代 `cli.args`: `cli.args` 仍是所有角色共享的基础参数。
/// - 典型用法是只给 Ralph coordinator 添加 Codex config override,例如
///   `["-c", "features.hooks=false"]`,而 worker 继续保持默认 hooks。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RoleArgsConfig {
    /// Ralph/coordinator jobs receive these extra arguments.
    #[serde(default)]
    pub coordinator: Vec<String>,

    /// Non-Ralph worker jobs receive these extra arguments.
    #[serde(default)]
    pub worker: Vec<String>,
}

impl RoleArgsConfig {
    /// Returns the extra args for the given role label.
    #[must_use]
    pub fn args_for_role(&self, coordinator: bool) -> &[String] {
        if coordinator {
            &self.coordinator
        } else {
            &self.worker
        }
    }
}

/// CLI backend configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliConfig {
    /// Backend to use: "claude", "kiro", "gemini", "codex", "amp", or "custom".
    #[serde(default = "default_backend")]
    pub backend: String,

    /// Custom command (for backend: "custom").
    pub command: Option<String>,

    /// How to pass prompts: "arg" or "stdin".
    #[serde(default = "default_prompt_mode")]
    pub prompt_mode: String,

    /// Execution mode when --interactive not specified.
    /// Values: "autonomous" (default), "interactive"
    #[serde(default = "default_mode")]
    pub default_mode: String,

    /// Idle timeout in seconds for interactive mode.
    /// Process is terminated after this many seconds of inactivity (no output AND no user input).
    /// Set to 0 to disable idle timeout.
    #[serde(default = "default_idle_timeout")]
    pub idle_timeout_secs: u32,

    /// Custom arguments to pass to the CLI command (for backend: "custom").
    /// These are inserted before the prompt argument.
    #[serde(default)]
    pub args: Vec<String>,

    /// Role-aware reasoning effort defaults.
    #[serde(default)]
    pub reasoning_effort: RoleReasoningEffortConfig,

    /// Role-aware extra CLI arguments.
    ///
    /// 说明:
    /// - 追加顺序由 executor 控制: backend 基础 args / hat args / runtime custom args
    ///   之后,reasoning defaults 之前。
    /// - 这样 role_args 中显式写入的 config override 能继续被后续 defaults 识别为
    ///   用户意图,避免重复注入。
    #[serde(default)]
    pub role_args: RoleArgsConfig,

    /// Custom prompt flag for arg mode (for backend: "custom").
    /// If None, defaults to "-p" for arg mode.
    #[serde(default)]
    pub prompt_flag: Option<String>,
}

fn default_backend() -> String {
    "claude".to_string()
}

fn default_prompt_mode() -> String {
    "arg".to_string()
}

fn default_mode() -> String {
    "autonomous".to_string()
}

fn default_idle_timeout() -> u32 {
    30 // 30 seconds per spec
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            backend: default_backend(),
            command: None,
            prompt_mode: default_prompt_mode(),
            default_mode: default_mode(),
            idle_timeout_secs: default_idle_timeout(),
            args: Vec::new(),
            reasoning_effort: RoleReasoningEffortConfig::default(),
            role_args: RoleArgsConfig::default(),
            prompt_flag: None,
        }
    }
}

/// TUI configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuiConfig {
    /// Prefix key combination (e.g., "ctrl-a", "ctrl-b").
    #[serde(default = "default_prefix_key")]
    pub prefix_key: String,

    /// 并行 Supervisor TUI：单个 job 输出缓冲的最大行数（超过即丢弃最旧的行）。
    ///
    /// 说明：
    /// - 该值只影响 TUI 的“回看/搜索窗口”，不会影响 `.ralph/events*.jsonl` 或 `--record-session` 的落盘内容。
    /// - 默认值偏保守：避免长时间运行导致内存无限增长。
    #[serde(default = "default_tui_max_buffer_lines")]
    pub max_buffer_lines: usize,
}

/// Memory injection mode.
///
/// Controls how memories are injected into agent context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InjectMode {
    /// Ralph automatically injects memories at the start of each iteration.
    #[default]
    Auto,
    /// Agent must explicitly run `ralph memory search` to access memories.
    Manual,
    /// Memories feature is disabled.
    None,
}

impl std::fmt::Display for InjectMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Auto => write!(f, "auto"),
            Self::Manual => write!(f, "manual"),
            Self::None => write!(f, "none"),
        }
    }
}

/// Memories configuration.
///
/// Controls the persistent learning system that allows Ralph to accumulate
/// wisdom across sessions. Memories are stored in `.agent/memories.md`.
///
/// When enabled, the memories skill is automatically injected to teach
/// agents how to create and search memories (skill injection is implicit).
///
/// Example configuration:
/// ```yaml
/// memories:
///   enabled: true
///   inject: auto
///   budget: 2000
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoriesConfig {
    /// Whether the memories feature is enabled.
    ///
    /// When true, memories are injected and the skill is taught to the agent.
    #[serde(default)]
    pub enabled: bool,

    /// How memories are injected into agent context.
    #[serde(default)]
    pub inject: InjectMode,

    /// Maximum tokens to inject (0 = unlimited).
    ///
    /// When set, memories are truncated to fit within this budget.
    #[serde(default)]
    pub budget: usize,

    /// Filter configuration for memory injection.
    #[serde(default)]
    pub filter: MemoriesFilter,
}

impl Default for MemoriesConfig {
    fn default() -> Self {
        Self {
            enabled: true, // Memories enabled by default
            inject: InjectMode::Auto,
            budget: 0,
            filter: MemoriesFilter::default(),
        }
    }
}

/// Filter configuration for memory injection.
///
/// Controls which memories are included when priming context.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoriesFilter {
    /// Filter by memory types (empty = all types).
    #[serde(default)]
    pub types: Vec<String>,

    /// Filter by tags (empty = all tags).
    #[serde(default)]
    pub tags: Vec<String>,

    /// Only include memories from the last N days (0 = no time limit).
    #[serde(default)]
    pub recent: u32,
}

/// Tasks configuration.
///
/// Controls the runtime task tracking system that allows Ralph to manage
/// work items across iterations. Tasks are stored in `.agent/tasks.jsonl`.
///
/// When enabled, tasks replace scratchpad for loop completion verification.
///
/// Example configuration:
/// ```yaml
/// tasks:
///   enabled: true
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TasksConfig {
    /// Whether the tasks feature is enabled.
    ///
    /// When true, tasks are used for loop completion verification.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for TasksConfig {
    fn default() -> Self {
        Self {
            enabled: true, // Tasks enabled by default
        }
    }
}

fn default_prefix_key() -> String {
    "ctrl-a".to_string()
}

fn default_tui_max_buffer_lines() -> usize {
    10_000
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            prefix_key: default_prefix_key(),
            max_buffer_lines: default_tui_max_buffer_lines(),
        }
    }
}

impl TuiConfig {
    /// Parses the prefix_key string into KeyCode and KeyModifiers.
    /// Returns an error if the format is invalid.
    pub fn parse_prefix(
        &self,
    ) -> Result<(crossterm::event::KeyCode, crossterm::event::KeyModifiers), String> {
        use crossterm::event::{KeyCode, KeyModifiers};

        let parts: Vec<&str> = self.prefix_key.split('-').collect();
        if parts.len() != 2 {
            return Err(format!(
                "Invalid prefix_key format: '{}'. Expected format: 'ctrl-<key>' (e.g., 'ctrl-a', 'ctrl-b')",
                self.prefix_key
            ));
        }

        let modifier = match parts[0].to_lowercase().as_str() {
            "ctrl" => KeyModifiers::CONTROL,
            _ => {
                return Err(format!(
                    "Invalid modifier: '{}'. Only 'ctrl' is supported (e.g., 'ctrl-a')",
                    parts[0]
                ));
            }
        };

        let key_str = parts[1];
        if key_str.len() != 1 {
            return Err(format!(
                "Invalid key: '{}'. Expected a single character (e.g., 'a', 'b')",
                key_str
            ));
        }

        let key_char = key_str.chars().next().unwrap();
        let key_code = KeyCode::Char(key_char);

        Ok((key_code, modifier))
    }
}

/// Metadata for an event topic.
///
/// Defines what an event means, enabling auto-derived instructions for hats.
/// When a hat triggers on or publishes an event, this metadata is used to
/// generate appropriate behavior instructions.
///
/// Example:
/// ```yaml
/// events:
///   deploy.start:
///     description: "Deployment has been requested"
///     on_trigger: "Prepare artifacts, validate config, check dependencies"
///     on_publish: "Signal that deployment should begin"
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventMetadata {
    /// Brief description of what this event represents.
    #[serde(default)]
    pub description: String,

    /// Instructions for a hat that triggers on (receives) this event.
    /// Describes what the hat should do when it receives this event.
    #[serde(default)]
    pub on_trigger: String,

    /// Instructions for a hat that publishes (emits) this event.
    /// Describes when/how the hat should emit this event.
    #[serde(default)]
    pub on_publish: String,
}

/// Backend configuration for a hat.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HatBackend {
    // 注意：`serde(untagged)` 的匹配顺序很关键，应当把“更具体”的结构放在前面。
    // 否则像 `type: "gemini"` 这种 map 结构可能会被更宽泛的 variant 抢先匹配。
    /// Kiro agent with custom agent name and optional args.
    KiroAgent {
        #[serde(rename = "type")]
        backend_type: String,
        agent: String,
        /// 额外 CLI 参数（例如 model/flags）。
        #[serde(default)]
        args: Vec<String>,
    },
    /// Named backend with args (has `type` but no `agent`).
    NamedWithArgs {
        #[serde(rename = "type")]
        backend_type: String,
        /// 额外 CLI 参数（例如 `["--model", "claude-sonnet-4"]`）。
        #[serde(default)]
        args: Vec<String>,
    },
    /// Named backend (string form, e.g., "claude", "gemini", "kiro").
    Named(String),
    /// Custom backend with command and args.
    Custom {
        command: String,
        /// 额外 CLI 参数；允许缺省为空数组，避免老配置没有 args 时解析失败。
        #[serde(default)]
        args: Vec<String>,
    },
}

impl HatBackend {
    /// Converts to CLI backend string for execution.
    pub fn to_cli_backend(&self) -> String {
        match self {
            HatBackend::Named(name) => name.clone(),
            HatBackend::NamedWithArgs { backend_type, .. } => backend_type.clone(),
            HatBackend::KiroAgent { .. } => "kiro".to_string(),
            HatBackend::Custom { .. } => "custom".to_string(),
        }
    }
}

fn default_hat_instances() -> usize {
    1
}

/// Workspace hooks configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkspaceHooksConfig {
    /// Acquire hook（例如：submodules 初始化、依赖预热）。
    #[serde(default)]
    pub on_acquire: Option<String>,

    /// Release hook（例如：清理临时文件、汇总结果）。
    #[serde(default)]
    pub on_release: Option<String>,
}

/// Workspace configuration for a hat.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HatWorkspaceConfig {
    /// 默认 workspace 策略。
    #[serde(default)]
    pub strategy: WorkspaceStrategy,

    /// 可选 hooks（on_acquire/on_release）。
    #[serde(default)]
    pub hooks: WorkspaceHooksConfig,
}

/// Configuration for a single hat.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HatConfig {
    /// Human-readable name for the hat.
    pub name: String,

    /// Short description of the hat's purpose (required).
    /// Used in the HATS table to help Ralph understand when to delegate to this hat.
    pub description: Option<String>,

    /// Events that trigger this hat to be worn.
    /// Per spec: "Hats define triggers — which events cause Ralph to wear this hat."
    #[serde(default)]
    pub triggers: Vec<String>,

    /// Topics this hat publishes.
    #[serde(default)]
    pub publishes: Vec<String>,

    /// Instructions prepended to prompts.
    #[serde(default)]
    pub instructions: String,

    /// Backend to use for this hat (inherits from cli.backend if not specified).
    #[serde(default)]
    pub backend: Option<HatBackend>,

    /// 单次 headless job 的超时（秒），仅在并行模式下生效。
    ///
    /// 语义：
    /// - 未设置：继承 `adapters.<backend>.timeout`（backend 按 hat.backend 或 cli.backend 推导）
    /// - 设为 `0`：显式禁用 job timeout（None）
    /// - 设为 `>0`：使用该秒数
    #[serde(default)]
    pub job_timeout_secs: Option<u64>,

    /// Default event to publish if hat forgets to write an event.
    #[serde(default)]
    pub default_publishes: Option<String>,

    /// Maximum number of times this hat may be activated in a single loop run.
    ///
    /// When the limit is exceeded, the orchestrator publishes `<hat_id>.exhausted`
    /// instead of activating the hat again.
    pub max_activations: Option<u32>,

    /// 同一种 hat 的实例数（并行模式下生效）。
    #[serde(default = "default_hat_instances")]
    pub instances: usize,

    /// 能力白名单（并行模式下用于 workspace/权限校验）。
    #[serde(default)]
    pub capabilities: Vec<String>,

    /// Workspace 策略与 hooks（并行模式下用于 job 隔离与自愈）。
    #[serde(default)]
    pub workspace: HatWorkspaceConfig,
}

impl HatConfig {
    /// Converts trigger strings to Topic objects.
    pub fn trigger_topics(&self) -> Vec<Topic> {
        self.triggers.iter().map(|s| Topic::new(s)).collect()
    }

    /// Converts publish strings to Topic objects.
    pub fn publish_topics(&self) -> Vec<Topic> {
        self.publishes.iter().map(|s| Topic::new(s)).collect()
    }
}

/// Configuration errors.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("Ambiguous routing: trigger '{trigger}' is claimed by both '{hat1}' and '{hat2}'")]
    AmbiguousRouting {
        trigger: String,
        hat1: String,
        hat2: String,
    },

    #[error("Mutually exclusive fields: '{field1}' and '{field2}' cannot both be specified")]
    MutuallyExclusive { field1: String, field2: String },

    #[error("Custom backend requires a command - set 'cli.command' in config")]
    CustomBackendRequiresCommand,

    #[error(
        "Reserved trigger '{trigger}' used by hat '{hat}' - runtime/control topics are reserved for Ralph or runtime observers. Use a delegated workflow event like 'work.start' instead."
    )]
    ReservedTrigger { trigger: String, hat: String },

    #[error(
        "Hat '{hat}' is missing required 'description' field - add a short description of the hat's purpose"
    )]
    MissingDescription { hat: String },

    #[error("Invalid value for '{field}': {message}")]
    InvalidValue { field: String, message: String },

    #[error("Hat import error: {0}")]
    HatImport(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = RalphConfig::default();
        // Default config has no custom hats (uses default planner+builder)
        assert!(config.hats.is_empty());
        assert_eq!(config.event_loop.max_iterations, 100);
        assert!(!config.verbose);
        assert!(config.agent_cli_recoverable_failures.enabled);
        assert_eq!(config.agent_cli_recoverable_failures.max_attempts, 3);
        assert_eq!(
            config.agent_cli_recoverable_failures.initial_delay_ms,
            30_000
        );
        assert_eq!(
            config.agent_cli_recoverable_failures.backoff_multiplier,
            2.0
        );
        assert_eq!(config.agent_cli_recoverable_failures.max_delay_ms, 300_000);
    }

    #[test]
    fn test_parse_agent_cli_recoverable_failures_policy_override() {
        let yaml = r"
agent_cli_recoverable_failures:
  enabled: false
  max_attempts: 5
  initial_delay_ms: 1000
  backoff_multiplier: 1.5
  max_delay_ms: 60000
";
        let config: RalphConfig = serde_yaml::from_str(yaml).unwrap();

        assert!(!config.agent_cli_recoverable_failures.enabled);
        assert_eq!(config.agent_cli_recoverable_failures.max_attempts, 5);
        assert_eq!(config.agent_cli_recoverable_failures.initial_delay_ms, 1000);
        assert_eq!(
            config.agent_cli_recoverable_failures.backoff_multiplier,
            1.5
        );
        assert_eq!(config.agent_cli_recoverable_failures.max_delay_ms, 60_000);
        assert!(
            config.validate().is_ok(),
            "custom bounded recoverable policy should validate"
        );
    }

    #[test]
    fn test_agent_cli_recoverable_failures_partial_override_keeps_defaults() {
        let yaml = r"
agent_cli_recoverable_failures:
  max_attempts: 4
";
        let config: RalphConfig = serde_yaml::from_str(yaml).unwrap();

        assert!(config.agent_cli_recoverable_failures.enabled);
        assert_eq!(config.agent_cli_recoverable_failures.max_attempts, 4);
        assert_eq!(
            config.agent_cli_recoverable_failures.initial_delay_ms,
            30_000
        );
        assert_eq!(
            config.agent_cli_recoverable_failures.backoff_multiplier,
            2.0
        );
        assert_eq!(config.agent_cli_recoverable_failures.max_delay_ms, 300_000);
    }

    #[test]
    fn test_cli_reasoning_effort_defaults_are_role_aware() {
        let config = RalphConfig::default();

        assert_eq!(
            config.cli.reasoning_effort.coordinator,
            ReasoningEffort::Medium
        );
        assert_eq!(config.cli.reasoning_effort.worker, ReasoningEffort::High);
    }

    #[test]
    fn test_parse_yaml_with_cli_reasoning_effort_overrides() {
        let yaml = r"
cli:
  reasoning_effort:
    coordinator: low
    worker: xhigh
";
        let config: RalphConfig = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(
            config.cli.reasoning_effort.coordinator,
            ReasoningEffort::Low
        );
        assert_eq!(config.cli.reasoning_effort.worker, ReasoningEffort::XHigh);
    }

    #[test]
    fn test_parse_yaml_with_cli_reasoning_effort_partial_override() {
        let yaml = r"
cli:
  reasoning_effort:
    coordinator: high
";
        let config: RalphConfig = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(
            config.cli.reasoning_effort.coordinator,
            ReasoningEffort::High
        );
        assert_eq!(
            config.cli.reasoning_effort.worker,
            ReasoningEffort::High,
            "未显式设置 worker 时应继续使用 high 默认值"
        );
    }

    #[test]
    fn test_cli_role_args_default_to_empty() {
        let config = RalphConfig::default();

        assert!(config.cli.role_args.coordinator.is_empty());
        assert!(config.cli.role_args.worker.is_empty());
    }

    #[test]
    fn test_parse_yaml_with_cli_role_args_overrides() {
        let yaml = r#"
cli:
  role_args:
    coordinator:
      - "-c"
      - "features.hooks=false"
    worker:
      - "--worker-flag"
"#;
        let config: RalphConfig = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(
            config.cli.role_args.coordinator,
            vec!["-c".to_string(), "features.hooks=false".to_string()]
        );
        assert_eq!(
            config.cli.role_args.worker,
            vec!["--worker-flag".to_string()]
        );
    }

    #[test]
    fn test_parse_yaml_with_cli_role_args_partial_override() {
        let yaml = r#"
cli:
  role_args:
    coordinator:
      - "-c"
      - "features.hooks=false"
"#;
        let config: RalphConfig = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(
            config.cli.role_args.coordinator,
            vec!["-c".to_string(), "features.hooks=false".to_string()]
        );
        assert!(
            config.cli.role_args.worker.is_empty(),
            "未显式设置 worker 时不应给 worker 追加任何 role args"
        );
    }

    #[test]
    fn test_default_worktree_backend_is_worktree() {
        let config = RalphConfig::default();
        assert_eq!(
            config.parallel.workspace.worktree_backend,
            WorktreeBackend::Worktree
        );
    }

    #[test]
    fn test_parse_yaml_with_worktree_backend_clone() {
        let yaml = r"
parallel:
  enabled: true
  workspace:
    worktree_backend: clone
";
        let config: RalphConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            config.parallel.workspace.worktree_backend,
            WorktreeBackend::Clone
        );
    }

    #[test]
    fn test_parse_yaml_with_custom_hats() {
        let yaml = r#"
event_loop:
  prompt_file: "TASK.md"
  completion_promise: "DONE"
  max_iterations: 50
cli:
  backend: "claude"
hats:
  implementer:
    name: "Implementer"
    triggers: ["task.*", "review.done"]
    publishes: ["impl.done"]
    instructions: "You are the implementation agent."
"#;
        let config: RalphConfig = serde_yaml::from_str(yaml).unwrap();
        // Custom hats are defined
        assert_eq!(config.hats.len(), 1);
        assert_eq!(config.event_loop.prompt_file, "TASK.md");

        let hat = config.hats.get("implementer").unwrap();
        assert_eq!(hat.triggers.len(), 2);
    }

    #[test]
    fn test_parse_yaml_with_complete_publishes() {
        let yaml = r#"
event_loop:
  prompt_file: "TASK.md"
  completion_promise: "DONE"
  complete_publishes: "fix.applied"
cli:
  backend: "claude"
"#;
        let config: RalphConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            config.event_loop.complete_publishes.as_deref(),
            Some("fix.applied")
        );
    }

    #[test]
    fn test_validate_complete_publishes_empty_string() {
        let mut config = RalphConfig::default();
        config.event_loop.complete_publishes = Some("   ".to_string());

        let result = config.validate();
        assert!(matches!(
            result,
            Err(ConfigError::InvalidValue { field, .. }) if field == "event_loop.complete_publishes"
        ));
    }

    #[test]
    fn test_validate_complete_publishes_requires_hat_publisher_when_custom_hats() {
        // 回归测试（硬门禁）：
        // - 当配置了 `event_loop.complete_publishes` 且存在自定义 hats 时，
        //   completion candidate topic 必须在至少一个 hat 的 `publishes` 里声明。
        let yaml = r#"
event_loop:
  complete_publishes: "fix.applied"
hats:
  runner:
    name: "Runner"
    description: "Does work"
    triggers: ["work.start"]
    publishes: ["work.done"]
"#;
        let config: RalphConfig = serde_yaml::from_str(yaml).unwrap();

        let result = config.validate();
        assert!(matches!(
            result,
            Err(ConfigError::InvalidValue { field, .. }) if field == "event_loop.complete_publishes"
        ));
    }

    #[test]
    fn test_validate_complete_publishes_ok_when_hat_publishes_topic() {
        // 正例：有 hat 明确声明发布 completion candidate topic。
        let yaml = r#"
event_loop:
  complete_publishes: "fix.applied"
hats:
  integrator:
    name: "Integrator"
    description: "Applies patch"
    triggers: ["fix.task"]
    publishes: ["fix.applied"]
"#;
        let config: RalphConfig = serde_yaml::from_str(yaml).unwrap();

        let result = config.validate();
        assert!(result.is_ok(), "expected config to validate successfully");
    }

    #[test]
    fn test_parse_yaml_v1_format() {
        // V1 flat format - identical to Python v1.x config
        let yaml = r#"
agent: gemini
prompt_file: "TASK.md"
completion_promise: "RALPH_DONE"
max_iterations: 75
max_runtime: 7200
max_cost: 10.0
verbose: true
"#;
        let mut config: RalphConfig = serde_yaml::from_str(yaml).unwrap();

        // Before normalization, v2 fields have defaults
        assert_eq!(config.cli.backend, "claude"); // default
        assert_eq!(config.event_loop.max_iterations, 100); // default

        // Normalize v1 -> v2
        config.normalize();

        // After normalization, v2 fields have v1 values
        assert_eq!(config.cli.backend, "gemini");
        assert_eq!(config.event_loop.prompt_file, "TASK.md");
        assert_eq!(config.event_loop.completion_promise, "RALPH_DONE");
        assert_eq!(config.event_loop.max_iterations, 75);
        assert_eq!(config.event_loop.max_runtime_seconds, 7200);
        assert_eq!(config.event_loop.max_cost_usd, Some(10.0));
        assert!(config.verbose);
    }

    #[test]
    fn test_agent_priority() {
        let yaml = r"
agent: auto
agent_priority: [gemini, claude, codex]
";
        let config: RalphConfig = serde_yaml::from_str(yaml).unwrap();
        let priority = config.get_agent_priority();
        assert_eq!(priority, vec!["gemini", "claude", "codex"]);
    }

    #[test]
    fn test_default_agent_priority() {
        let config = RalphConfig::default();
        let priority = config.get_agent_priority();
        assert_eq!(priority, vec!["claude", "kiro", "gemini", "codex", "amp"]);
    }

    #[test]
    fn test_validate_deferred_features() {
        let yaml = r"
archive_prompts: true
enable_metrics: true
";
        let config: RalphConfig = serde_yaml::from_str(yaml).unwrap();
        let warnings = config.validate().unwrap();

        assert_eq!(warnings.len(), 2);
        assert!(warnings
            .iter()
            .any(|w| matches!(w, ConfigWarning::DeferredFeature { field, .. } if field == "archive_prompts")));
        assert!(warnings
            .iter()
            .any(|w| matches!(w, ConfigWarning::DeferredFeature { field, .. } if field == "enable_metrics")));
    }

    #[test]
    fn test_validate_dropped_fields() {
        let yaml = r#"
max_tokens: 4096
retry_delay: 5
adapters:
  claude:
    tool_permissions: ["read", "write"]
"#;
        let config: RalphConfig = serde_yaml::from_str(yaml).unwrap();
        let warnings = config.validate().unwrap();

        assert_eq!(warnings.len(), 3);
        assert!(warnings.iter().any(
            |w| matches!(w, ConfigWarning::DroppedField { field, .. } if field == "max_tokens")
        ));
        assert!(warnings.iter().any(
            |w| matches!(w, ConfigWarning::DroppedField { field, .. } if field == "retry_delay")
        ));
        assert!(warnings
            .iter()
            .any(|w| matches!(w, ConfigWarning::DroppedField { field, .. } if field == "adapters.*.tool_permissions")));
    }

    #[test]
    fn test_suppress_warnings() {
        let yaml = r"
_suppress_warnings: true
archive_prompts: true
max_tokens: 4096
";
        let config: RalphConfig = serde_yaml::from_str(yaml).unwrap();
        let warnings = config.validate().unwrap();

        // All warnings should be suppressed
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_adapter_settings() {
        let yaml = r"
adapters:
  claude:
    timeout: 600
    context_window_tokens: 200000
    enabled: true
  gemini:
    timeout: 300
    enabled: false
";
        let config: RalphConfig = serde_yaml::from_str(yaml).unwrap();

        let claude = config.adapter_settings("claude");
        assert_eq!(claude.timeout, 600);
        assert_eq!(claude.context_window_tokens, Some(200_000));
        assert!(claude.enabled);

        let gemini = config.adapter_settings("gemini");
        assert_eq!(gemini.timeout, 300);
        assert!(!gemini.enabled);
    }

    #[test]
    fn test_adapter_settings_custom_command_codex_maps_to_codex() {
        // 当 cli.backend=custom 且 command=codex 时，应当使用 adapters.codex 的 timeout 配置。
        let yaml = r#"
cli:
  backend: "custom"
  command: "codex"
  prompt_mode: "arg"
adapters:
  claude:
    timeout: 111
    output_stale_timeout_secs: 11
  codex:
    timeout: 222
    output_stale_timeout_secs: 22
"#;
        let config: RalphConfig = serde_yaml::from_str(yaml).unwrap();

        let settings = config.adapter_settings(&config.cli.backend);
        assert_eq!(settings.timeout, 222);
        assert_eq!(settings.output_stale_timeout_secs, 22);
    }

    #[test]
    fn test_unknown_fields_ignored() {
        // Unknown fields should be silently ignored (forward compatibility)
        let yaml = r#"
agent: claude
unknown_field: "some value"
future_feature: true
"#;
        let result: Result<RalphConfig, _> = serde_yaml::from_str(yaml);
        // Should parse successfully, ignoring unknown fields
        assert!(result.is_ok());
    }

    #[test]
    fn test_ambiguous_routing_rejected() {
        // Per spec: "Every trigger maps to exactly one hat | No ambiguous routing"
        // Note: using semantic events since task.start is reserved
        let yaml = r#"
hats:
  planner:
    name: "Planner"
    description: "Plans tasks"
    triggers: ["planning.start", "build.done"]
  builder:
    name: "Builder"
    description: "Builds code"
    triggers: ["build.task", "build.done"]
"#;
        let config: RalphConfig = serde_yaml::from_str(yaml).unwrap();
        let result = config.validate();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(&err, ConfigError::AmbiguousRouting { trigger, .. } if trigger == "build.done"),
            "Expected AmbiguousRouting error for 'build.done', got: {:?}",
            err
        );
    }

    #[test]
    fn test_ambiguous_routing_allowed_in_parallel_mode() {
        // 并行模式下允许多个 hat 共享同一 trigger（例如 fanout）。
        let yaml = r#"
parallel:
  enabled: true
hats:
  writer:
    name: "Writer"
    description: "Writes code"
    triggers: ["build.task"]
  tester:
    name: "Tester"
    description: "Runs tests"
    triggers: ["build.task"]
"#;
        let config: RalphConfig = serde_yaml::from_str(yaml).unwrap();
        let result = config.validate();

        assert!(
            result.is_ok(),
            "Expected ambiguous triggers to be allowed in parallel mode, got: {:?}",
            result.unwrap_err()
        );
    }

    #[test]
    fn test_unique_triggers_accepted() {
        // Valid config: each trigger maps to exactly one hat
        // Note: task.start is reserved for Ralph, so use semantic events
        let yaml = r#"
hats:
  planner:
    name: "Planner"
    description: "Plans tasks"
    triggers: ["planning.start", "build.done", "build.blocked"]
  builder:
    name: "Builder"
    description: "Builds code"
    triggers: ["build.task"]
"#;
        let config: RalphConfig = serde_yaml::from_str(yaml).unwrap();
        let result = config.validate();

        assert!(
            result.is_ok(),
            "Expected valid config, got: {:?}",
            result.unwrap_err()
        );
    }

    #[test]
    fn test_reserved_trigger_task_start_rejected() {
        // Per design: task.start is reserved for Ralph (the coordinator)
        let yaml = r#"
hats:
  my_hat:
    name: "My Hat"
    description: "Test hat"
    triggers: ["task.start"]
"#;
        let config: RalphConfig = serde_yaml::from_str(yaml).unwrap();
        let result = config.validate();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(&err, ConfigError::ReservedTrigger { trigger, hat }
                if trigger == "task.start" && hat == "my_hat"),
            "Expected ReservedTrigger error for 'task.start', got: {:?}",
            err
        );
    }

    #[test]
    fn test_reserved_trigger_task_resume_rejected() {
        // Per design: task.resume is reserved for Ralph (the coordinator)
        let yaml = r#"
hats:
  my_hat:
    name: "My Hat"
    description: "Test hat"
    triggers: ["task.resume", "other.event"]
"#;
        let config: RalphConfig = serde_yaml::from_str(yaml).unwrap();
        let result = config.validate();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(&err, ConfigError::ReservedTrigger { trigger, hat }
                if trigger == "task.resume" && hat == "my_hat"),
            "Expected ReservedTrigger error for 'task.resume', got: {:?}",
            err
        );
    }

    #[test]
    fn test_reserved_runtime_control_triggers_rejected_from_runtime_protocol_ssot() {
        for trigger in [
            "topology.spawn_group",
            "topology.spawn.result",
            "capability.request",
            "capability.result",
            "runtime.delivery",
            "gate.request",
            "human.message",
            "reply.human.message",
        ] {
            let yaml = format!(
                r#"
hats:
  my_hat:
    name: "My Hat"
    description: "Test hat"
    triggers: ["{trigger}"]
"#
            );
            let config: RalphConfig = serde_yaml::from_str(&yaml).unwrap();
            let result = config.validate();

            assert!(result.is_err(), "{trigger} should be rejected");
            let err = result.unwrap_err();
            assert!(
                matches!(&err, ConfigError::ReservedTrigger { trigger: got, hat }
                    if got == trigger && hat == "my_hat"),
                "Expected ReservedTrigger error for '{trigger}', got: {:?}",
                err
            );
        }
    }

    #[test]
    fn test_missing_description_rejected() {
        // Description is required for all hats
        let yaml = r#"
hats:
  my_hat:
    name: "My Hat"
    triggers: ["build.task"]
"#;
        let config: RalphConfig = serde_yaml::from_str(yaml).unwrap();
        let result = config.validate();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(&err, ConfigError::MissingDescription { hat } if hat == "my_hat"),
            "Expected MissingDescription error, got: {:?}",
            err
        );
    }

    #[test]
    fn test_empty_description_rejected() {
        // Empty description should also be rejected
        let yaml = r#"
hats:
  my_hat:
    name: "My Hat"
    description: "   "
    triggers: ["build.task"]
"#;
        let config: RalphConfig = serde_yaml::from_str(yaml).unwrap();
        let result = config.validate();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(&err, ConfigError::MissingDescription { hat } if hat == "my_hat"),
            "Expected MissingDescription error for empty description, got: {:?}",
            err
        );
    }

    #[test]
    fn test_core_config_defaults() {
        let config = RalphConfig::default();
        assert_eq!(config.core.scratchpad, ".agent/scratchpad.md");
        assert_eq!(config.core.specs_dir, "./specs/");
        // Default guardrails per spec
        assert_eq!(config.core.guardrails.len(), 3);
        assert!(config.core.guardrails[0].contains("Fresh context"));
        assert!(config.core.guardrails[1].contains("search first"));
        assert!(config.core.guardrails[2].contains("Backpressure"));
        assert_eq!(config.core.all_hat_prompt, AllHatPromptConfig::Compiled);
    }

    #[test]
    fn test_core_config_customizable() {
        let yaml = r#"
core:
  scratchpad: ".workspace/plan.md"
  specs_dir: "./specifications/"
"#;
        let config: RalphConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.core.scratchpad, ".workspace/plan.md");
        assert_eq!(config.core.specs_dir, "./specifications/");
        // Guardrails should use defaults when not specified
        assert_eq!(config.core.guardrails.len(), 3);
    }

    #[test]
    fn test_core_config_custom_guardrails() {
        let yaml = r#"
core:
  scratchpad: ".agent/scratchpad.md"
  specs_dir: "./specs/"
  guardrails:
    - "Custom rule one"
    - "Custom rule two"
"#;
        let config: RalphConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.core.guardrails.len(), 2);
        assert_eq!(config.core.guardrails[0], "Custom rule one");
        assert_eq!(config.core.guardrails[1], "Custom rule two");
    }

    #[test]
    fn test_parse_yaml_with_inline_all_hat_prompt_override() {
        let yaml = r"
core:
  all_hat_prompt:
    mode: inline
    text: |
      lightweight overlay
";
        let config: RalphConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            config.core.all_hat_prompt,
            AllHatPromptConfig::Inline {
                text: "lightweight overlay\n".to_string()
            }
        );
    }

    #[test]
    fn test_validate_inline_all_hat_prompt_requires_text() {
        let yaml = r#"
core:
  all_hat_prompt:
    mode: inline
    text: "   "
"#;
        let config: RalphConfig = serde_yaml::from_str(yaml).unwrap();
        let result = config.validate();

        assert!(matches!(
            result,
            Err(ConfigError::InvalidValue { field, .. }) if field == "core.all_hat_prompt.text"
        ));
    }

    #[test]
    fn test_validate_file_all_hat_prompt_requires_existing_file() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let existing = temp_dir.path().join("overlay.md");
        std::fs::write(&existing, "file overlay\n").expect("write overlay file");

        let yaml = format!(
            "core:\n  all_hat_prompt:\n    mode: file\n    path: \"{}\"\n",
            existing.display()
        );
        let config: RalphConfig = serde_yaml::from_str(&yaml).unwrap();
        assert!(
            config.validate().is_ok(),
            "existing overlay file should validate"
        );

        let missing_yaml = format!(
            "core:\n  all_hat_prompt:\n    mode: file\n    path: \"{}\"\n",
            temp_dir.path().join("missing.md").display()
        );
        let missing_config: RalphConfig = serde_yaml::from_str(&missing_yaml).unwrap();
        let result = missing_config.validate();

        assert!(matches!(
            result,
            Err(ConfigError::InvalidValue { field, .. }) if field == "core.all_hat_prompt.path"
        ));
    }

    #[test]
    fn test_core_config_resolves_scoped_experience_paths() {
        let core = CoreConfig::default().with_workspace_root("/tmp/ralph-scoped-experience");

        assert_eq!(
            core.resolve_legacy_memories_path(),
            std::path::PathBuf::from("/tmp/ralph-scoped-experience/.agent/memories.md")
        );
        assert_eq!(
            core.resolve_project_experience_path(),
            std::path::PathBuf::from("/tmp/ralph-scoped-experience/experience.md")
        );
        assert_eq!(
            core.resolve_role_experience_path("spec_reviewer"),
            std::path::PathBuf::from(
                "/tmp/ralph-scoped-experience/.ralph/roles/spec_reviewer/experience.md"
            )
        );
        assert_eq!(
            core.resolve_instance_context_dir("writer#1"),
            std::path::PathBuf::from("/tmp/ralph-scoped-experience/.ralph/log/writer#1")
        );
        assert_eq!(
            core.resolve_instance_context_path("writer#1", "WORKLOG.md"),
            std::path::PathBuf::from("/tmp/ralph-scoped-experience/.ralph/log/writer#1/WORKLOG.md")
        );
        assert_eq!(
            core.resolve_recoverable_failures_ledger_path(),
            std::path::PathBuf::from(
                "/tmp/ralph-scoped-experience/.ralph/recoverable-failures.jsonl"
            )
        );
    }

    #[test]
    fn test_validate_recoverable_failures_policy_rejects_zero_attempts() {
        let yaml = r"
agent_cli_recoverable_failures:
  max_attempts: 0
";
        let config: RalphConfig = serde_yaml::from_str(yaml).unwrap();
        let result = config.validate();

        assert!(matches!(
            result,
            Err(ConfigError::InvalidValue { field, .. })
                if field == "agent_cli_recoverable_failures.max_attempts"
        ));
    }

    #[test]
    fn test_validate_recoverable_failures_policy_rejects_zero_initial_delay() {
        let yaml = r"
agent_cli_recoverable_failures:
  initial_delay_ms: 0
";
        let config: RalphConfig = serde_yaml::from_str(yaml).unwrap();
        let result = config.validate();

        assert!(matches!(
            result,
            Err(ConfigError::InvalidValue { field, .. })
                if field == "agent_cli_recoverable_failures.initial_delay_ms"
        ));
    }

    #[test]
    fn test_validate_recoverable_failures_policy_rejects_max_delay_below_initial() {
        let yaml = r"
agent_cli_recoverable_failures:
  initial_delay_ms: 30000
  max_delay_ms: 10000
";
        let config: RalphConfig = serde_yaml::from_str(yaml).unwrap();
        let result = config.validate();

        assert!(matches!(
            result,
            Err(ConfigError::InvalidValue { field, .. })
                if field == "agent_cli_recoverable_failures.max_delay_ms"
        ));
    }

    #[test]
    fn test_validate_recoverable_failures_policy_rejects_backoff_below_one() {
        let yaml = r"
agent_cli_recoverable_failures:
  backoff_multiplier: 0.5
";
        let config: RalphConfig = serde_yaml::from_str(yaml).unwrap();
        let result = config.validate();

        assert!(matches!(
            result,
            Err(ConfigError::InvalidValue { field, .. })
                if field == "agent_cli_recoverable_failures.backoff_multiplier"
        ));
    }

    #[test]
    fn test_prompt_and_prompt_file_mutually_exclusive() {
        // Both prompt and prompt_file specified in config should error
        let yaml = r#"
event_loop:
  prompt: "inline text"
  prompt_file: "custom.md"
"#;
        let config: RalphConfig = serde_yaml::from_str(yaml).unwrap();
        let result = config.validate();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(&err, ConfigError::MutuallyExclusive { field1, field2 }
                if field1 == "event_loop.prompt" && field2 == "event_loop.prompt_file"),
            "Expected MutuallyExclusive error, got: {:?}",
            err
        );
    }

    #[test]
    fn test_prompt_with_default_prompt_file_allowed() {
        // Having inline prompt with default prompt_file value should be OK
        let yaml = r#"
event_loop:
  prompt: "inline text"
"#;
        let config: RalphConfig = serde_yaml::from_str(yaml).unwrap();
        let result = config.validate();

        assert!(
            result.is_ok(),
            "Should allow inline prompt with default prompt_file"
        );
        assert_eq!(config.event_loop.prompt, Some("inline text".to_string()));
        assert_eq!(config.event_loop.prompt_file, "PROMPT.md");
    }

    #[test]
    fn test_ralph_prompt_is_additive_and_does_not_affect_prompt_precedence() {
        // `event_loop.ralph_prompt` 是 Ralph-only 的追加注入，不参与 prompt/prompt_file 的互斥与优先级。
        let yaml = r#"
event_loop:
  prompt: "inline text"
  ralph_prompt: "ralph only"
"#;
        let config: RalphConfig = serde_yaml::from_str(yaml).unwrap();
        let result = config.validate();

        assert!(
            result.is_ok(),
            "Should allow ralph_prompt alongside inline prompt"
        );
        assert_eq!(config.event_loop.prompt, Some("inline text".to_string()));
        assert_eq!(
            config.event_loop.ralph_prompt,
            Some("ralph only".to_string())
        );
        assert_eq!(config.event_loop.prompt_file, "PROMPT.md");
    }

    #[test]
    fn test_custom_backend_requires_command() {
        // Custom backend without command should error
        let yaml = r#"
cli:
  backend: "custom"
"#;
        let config: RalphConfig = serde_yaml::from_str(yaml).unwrap();
        let result = config.validate();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(&err, ConfigError::CustomBackendRequiresCommand),
            "Expected CustomBackendRequiresCommand error, got: {:?}",
            err
        );
    }

    #[test]
    fn test_custom_backend_with_empty_command_errors() {
        // Custom backend with empty command should error
        let yaml = r#"
cli:
  backend: "custom"
  command: ""
"#;
        let config: RalphConfig = serde_yaml::from_str(yaml).unwrap();
        let result = config.validate();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(&err, ConfigError::CustomBackendRequiresCommand),
            "Expected CustomBackendRequiresCommand error, got: {:?}",
            err
        );
    }

    #[test]
    fn test_custom_backend_with_command_succeeds() {
        // Custom backend with valid command should pass validation
        let yaml = r#"
cli:
  backend: "custom"
  command: "my-agent"
"#;
        let config: RalphConfig = serde_yaml::from_str(yaml).unwrap();
        let result = config.validate();

        assert!(
            result.is_ok(),
            "Should allow custom backend with command: {:?}",
            result.unwrap_err()
        );
    }

    #[test]
    fn test_prompt_file_with_no_inline_allowed() {
        // Having only prompt_file specified should be OK
        let yaml = r#"
event_loop:
  prompt_file: "custom.md"
"#;
        let config: RalphConfig = serde_yaml::from_str(yaml).unwrap();
        let result = config.validate();

        assert!(
            result.is_ok(),
            "Should allow prompt_file without inline prompt"
        );
        assert_eq!(config.event_loop.prompt, None);
        assert_eq!(config.event_loop.prompt_file, "custom.md");
    }

    #[test]
    fn test_default_prompt_file_value() {
        let config = RalphConfig::default();
        assert_eq!(config.event_loop.prompt_file, "PROMPT.md");
        assert_eq!(config.event_loop.prompt, None);
    }

    #[test]
    fn test_tui_config_default() {
        let config = RalphConfig::default();
        assert_eq!(config.tui.prefix_key, "ctrl-a");
        assert_eq!(config.tui.max_buffer_lines, 10_000);
    }

    #[test]
    fn test_tui_config_parse_ctrl_b() {
        let yaml = r#"
tui:
  prefix_key: "ctrl-b"
"#;
        let config: RalphConfig = serde_yaml::from_str(yaml).unwrap();
        let (key_code, key_modifiers) = config.tui.parse_prefix().unwrap();

        use crossterm::event::{KeyCode, KeyModifiers};
        assert_eq!(key_code, KeyCode::Char('b'));
        assert_eq!(key_modifiers, KeyModifiers::CONTROL);
    }

    #[test]
    fn test_tui_config_max_buffer_lines_override() {
        let yaml = r"
tui:
  max_buffer_lines: 12345
";
        let config: RalphConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.tui.prefix_key, "ctrl-a");
        assert_eq!(config.tui.max_buffer_lines, 12345);
    }

    #[test]
    fn test_tui_config_parse_invalid_format() {
        let tui_config = TuiConfig {
            prefix_key: "invalid".to_string(),
            max_buffer_lines: TuiConfig::default().max_buffer_lines,
        };
        let result = tui_config.parse_prefix();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid prefix_key format"));
    }

    #[test]
    fn test_tui_config_parse_invalid_modifier() {
        let tui_config = TuiConfig {
            prefix_key: "alt-a".to_string(),
            max_buffer_lines: TuiConfig::default().max_buffer_lines,
        };
        let result = tui_config.parse_prefix();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid modifier"));
    }

    #[test]
    fn test_tui_config_parse_invalid_key() {
        let tui_config = TuiConfig {
            prefix_key: "ctrl-abc".to_string(),
            max_buffer_lines: TuiConfig::default().max_buffer_lines,
        };
        let result = tui_config.parse_prefix();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid key"));
    }

    #[test]
    fn test_hat_backend_named() {
        let yaml = r#""claude""#;
        let backend: HatBackend = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(backend.to_cli_backend(), "claude");
        match backend {
            HatBackend::Named(name) => assert_eq!(name, "claude"),
            _ => panic!("Expected Named variant"),
        }
    }

    #[test]
    fn test_hat_backend_kiro_agent() {
        let yaml = r#"
type: "kiro"
agent: "builder"
"#;
        let backend: HatBackend = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(backend.to_cli_backend(), "kiro");
        match backend {
            HatBackend::KiroAgent {
                backend_type,
                agent,
                args,
            } => {
                assert_eq!(backend_type, "kiro");
                assert_eq!(agent, "builder");
                assert!(args.is_empty());
            }
            _ => panic!("Expected KiroAgent variant"),
        }
    }

    #[test]
    fn test_hat_backend_kiro_agent_with_args() {
        let yaml = r#"
type: "kiro"
agent: "builder"
args: ["--verbose", "--debug"]
"#;
        let backend: HatBackend = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(backend.to_cli_backend(), "kiro");
        match backend {
            HatBackend::KiroAgent {
                backend_type,
                agent,
                args,
            } => {
                assert_eq!(backend_type, "kiro");
                assert_eq!(agent, "builder");
                assert_eq!(args, vec!["--verbose", "--debug"]);
            }
            _ => panic!("Expected KiroAgent variant"),
        }
    }

    #[test]
    fn test_hat_backend_named_with_args() {
        let yaml = r#"
type: "claude"
args: ["--model", "claude-sonnet-4"]
"#;
        let backend: HatBackend = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(backend.to_cli_backend(), "claude");
        match backend {
            HatBackend::NamedWithArgs { backend_type, args } => {
                assert_eq!(backend_type, "claude");
                assert_eq!(args, vec!["--model", "claude-sonnet-4"]);
            }
            _ => panic!("Expected NamedWithArgs variant"),
        }
    }

    #[test]
    fn test_hat_backend_named_with_args_empty() {
        // `type: ...` 但不带 args，也应当能解析（args 默认空数组）。
        let yaml = r#"
type: "gemini"
"#;
        let backend: HatBackend = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(backend.to_cli_backend(), "gemini");
        match backend {
            HatBackend::NamedWithArgs { backend_type, args } => {
                assert_eq!(backend_type, "gemini");
                assert!(args.is_empty());
            }
            _ => panic!("Expected NamedWithArgs variant"),
        }
    }

    #[test]
    fn test_hat_backend_custom() {
        let yaml = r#"
command: "/usr/bin/my-agent"
args: ["--flag", "value"]
"#;
        let backend: HatBackend = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(backend.to_cli_backend(), "custom");
        match backend {
            HatBackend::Custom { command, args } => {
                assert_eq!(command, "/usr/bin/my-agent");
                assert_eq!(args, vec!["--flag", "value"]);
            }
            _ => panic!("Expected Custom variant"),
        }
    }

    #[test]
    fn test_hat_config_with_backend() {
        let yaml = r#"
name: "Custom Builder"
triggers: ["build.task"]
publishes: ["build.done"]
instructions: "Build stuff"
backend: "gemini"
default_publishes: "task.done"
"#;
        let hat: HatConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(hat.name, "Custom Builder");
        assert!(hat.backend.is_some());
        match hat.backend.unwrap() {
            HatBackend::Named(name) => assert_eq!(name, "gemini"),
            _ => panic!("Expected Named backend"),
        }
        assert_eq!(hat.default_publishes, Some("task.done".to_string()));
    }

    #[test]
    fn test_hat_config_without_backend() {
        let yaml = r#"
name: "Default Hat"
triggers: ["task.start"]
publishes: ["task.done"]
instructions: "Do work"
"#;
        let hat: HatConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(hat.name, "Default Hat");
        assert!(hat.backend.is_none());
        assert!(hat.default_publishes.is_none());
    }

    #[test]
    fn test_mixed_backends_config() {
        let yaml = r#"
event_loop:
  prompt_file: "TASK.md"
  max_iterations: 50

cli:
  backend: "claude"

hats:
  planner:
    name: "Planner"
    triggers: ["task.start"]
    publishes: ["build.task"]
    instructions: "Plan the work"
    backend: "claude"
    
  builder:
    name: "Builder"
    triggers: ["build.task"]
    publishes: ["build.done"]
    instructions: "Build the thing"
    backend:
      type: "kiro"
      agent: "builder"
      
  reviewer:
    name: "Reviewer"
    triggers: ["build.done"]
    publishes: ["review.complete"]
    instructions: "Review the work"
    backend:
      command: "/usr/local/bin/custom-agent"
      args: ["--mode", "review"]
    default_publishes: "review.complete"
"#;
        let config: RalphConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.hats.len(), 3);

        // Check planner (Named backend)
        let planner = config.hats.get("planner").unwrap();
        assert!(planner.backend.is_some());
        match planner.backend.as_ref().unwrap() {
            HatBackend::Named(name) => assert_eq!(name, "claude"),
            _ => panic!("Expected Named backend for planner"),
        }

        // Check builder (KiroAgent backend)
        let builder = config.hats.get("builder").unwrap();
        assert!(builder.backend.is_some());
        match builder.backend.as_ref().unwrap() {
            HatBackend::KiroAgent {
                backend_type,
                agent,
                args,
            } => {
                assert_eq!(backend_type, "kiro");
                assert_eq!(agent, "builder");
                assert!(args.is_empty());
            }
            _ => panic!("Expected KiroAgent backend for builder"),
        }

        // Check reviewer (Custom backend)
        let reviewer = config.hats.get("reviewer").unwrap();
        assert!(reviewer.backend.is_some());
        match reviewer.backend.as_ref().unwrap() {
            HatBackend::Custom { command, args } => {
                assert_eq!(command, "/usr/local/bin/custom-agent");
                assert_eq!(args, &vec!["--mode".to_string(), "review".to_string()]);
            }
            _ => panic!("Expected Custom backend for reviewer"),
        }
        assert_eq!(
            reviewer.default_publishes,
            Some("review.complete".to_string())
        );
    }
}

#[cfg(test)]
mod context_window_tests {
    use super::*;

    fn config_with_context_window(backend: &str, tokens: Option<u32>) -> RalphConfig {
        let yaml = format!(
            "cli:\n  backend: {}\n  command: {}\n  args: []\nadapters:\n  {}:\n    context_window_tokens: {}\n",
            backend,
            backend,
            backend,
            tokens.map(|n| n.to_string()).unwrap_or_else(|| "~".to_string())
        );
        serde_yaml::from_str(&yaml).expect("yaml must parse")
    }

    #[test]
    fn resolve_context_window_prefers_explicit_override() {
        let config = config_with_context_window("codex", Some(200_000));
        let resolved = config.resolve_context_window("codex");
        assert_eq!(resolved, 200_000);
    }

    #[test]
    fn resolve_context_window_returns_zero_when_unset() {
        let config = config_with_context_window("codex", None);
        let resolved = config.resolve_context_window("codex");
        assert_eq!(resolved, 0);
    }
}
