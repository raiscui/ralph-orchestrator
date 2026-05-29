//! Agent CLI 可恢复失败的核心类型与确定性分类器。
//!
//! 这一层只回答两个问题：
//! - 哪些外部 agent CLI 失败“允许进入 retry 生命周期”。
//! - retry 生命周期需要哪些稳定、可序列化、可审计的元数据。
//!
//! 注意：这里不做 IO,也不调度重试。Ledger 写入与 runtime 接线会在后续任务中实现。

use crate::parallel::HatJobResult;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Recoverable failure ledger 的 schema 版本。
pub const RECOVERABLE_FAILURE_LEDGER_SCHEMA_VERSION: u32 = 1;

/// Recoverable failure ledger 的唯一默认落盘路径。
///
/// 说明：
/// - 该路径必须通过 `CoreConfig::resolve_recoverable_failures_ledger_path()` 解析。
/// - 不应在 runtime 其它地方手写同一个字符串,避免形成第二真相源。
pub const DEFAULT_RECOVERABLE_FAILURE_LEDGER_PATH: &str = ".ralph/recoverable-failures.jsonl";

/// 人类显式请求继续 recoverable failure 的控制面 topic。
///
/// 说明:
/// - 这是 Supervisor 消费的 external control event,不是普通 worker workflow topic。
/// - TUI 的 `!continue` 会写入该 topic,避免把普通中文聊天误判为重试控制。
pub const TOPIC_RECOVERABLE_CONTINUE: &str = "recoverable.continue";

/// 默认可恢复失败最大尝试次数。
pub const DEFAULT_RECOVERABLE_FAILURE_MAX_ATTEMPTS: u32 = 3;

/// 默认首次 retry 延迟。
pub const DEFAULT_RECOVERABLE_FAILURE_INITIAL_DELAY_MS: u64 = 30_000;

/// 默认 retry 退避倍数。
pub const DEFAULT_RECOVERABLE_FAILURE_BACKOFF_MULTIPLIER: f64 = 2.0;

/// 默认最大 retry 延迟。
pub const DEFAULT_RECOVERABLE_FAILURE_MAX_DELAY_MS: u64 = 300_000;

/// stderr 摘要默认字符上限。
const DEFAULT_STDERR_EXCERPT_CHARS: usize = 500;

fn default_recoverable_failures_enabled() -> bool {
    true
}

fn default_max_attempts() -> u32 {
    DEFAULT_RECOVERABLE_FAILURE_MAX_ATTEMPTS
}

fn default_initial_delay_ms() -> u64 {
    DEFAULT_RECOVERABLE_FAILURE_INITIAL_DELAY_MS
}

fn default_backoff_multiplier() -> f64 {
    DEFAULT_RECOVERABLE_FAILURE_BACKOFF_MULTIPLIER
}

fn default_max_delay_ms() -> u64 {
    DEFAULT_RECOVERABLE_FAILURE_MAX_DELAY_MS
}

/// Agent CLI 可恢复失败配置。
///
/// 这个配置放在顶层 `agent_cli_recoverable_failures`,因为它描述的是外部 agent CLI
/// 执行失败后的统一 runtime 策略,而不是某一个 hat、parallel 或 adapter 的私有选项。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentCliRecoverableFailuresConfig {
    /// 是否启用自动/手动 retry 策略。
    #[serde(default = "default_recoverable_failures_enabled")]
    pub enabled: bool,

    /// 单个 recoverable lifecycle 最多尝试次数。
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,

    /// 第一次 retry 的延迟毫秒数。
    #[serde(default = "default_initial_delay_ms")]
    pub initial_delay_ms: u64,

    /// 指数退避倍数。`1.0` 表示固定延迟。
    #[serde(default = "default_backoff_multiplier")]
    pub backoff_multiplier: f64,

    /// 单次 retry 延迟上限。
    #[serde(default = "default_max_delay_ms")]
    pub max_delay_ms: u64,
}

impl Default for AgentCliRecoverableFailuresConfig {
    fn default() -> Self {
        Self {
            enabled: default_recoverable_failures_enabled(),
            max_attempts: default_max_attempts(),
            initial_delay_ms: default_initial_delay_ms(),
            backoff_multiplier: default_backoff_multiplier(),
            max_delay_ms: default_max_delay_ms(),
        }
    }
}

/// 可恢复失败的窄分类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoverableFailureKind {
    /// 后端明确返回限流。
    RateLimited,
    /// agent CLI 自身重试耗尽,且最后状态仍是临时失败。
    RetryLimitExceeded,
    /// 明确列入白名单的瞬时网络错误。
    TransientNetwork,
}

impl RecoverableFailureKind {
    /// Stable ledger id spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RateLimited => "rate_limited",
            Self::RetryLimitExceeded => "retry_limit_exceeded",
            Self::TransientNetwork => "transient_network",
        }
    }
}

/// Recoverable failure 生命周期状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoverableFailureStatus {
    /// 已计算 next retry 时间。
    RetryScheduled,
    /// 暂停等待人类或 scheduler。
    PausedRecoverable,
    /// 正在执行 retry。
    Retrying,
    /// retry lifecycle 已通过后续尝试成功闭环。
    Recovered,
    /// 已耗尽 retry 次数,可以转成 terminal failure。
    Exhausted,
    /// 人类显式 continue 后产生的控制面状态。
    ContinuedByHuman,
}

/// Recoverable failure ledger 中的一条状态转移。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecoverableFailureTransition {
    /// JSONL schema version。
    pub schema_version: u32,

    /// 单个 recoverable lifecycle 的稳定 id。
    pub failure_id: String,

    /// Runtime job id。
    pub job_id: u64,

    /// 受影响的 instance id,例如 `writer#1`。
    pub instance_id: String,

    /// 受影响的 hat id,例如 `writer`。
    pub hat_id: String,

    /// 后端类型或命令摘要,例如 `codex` / `claude` / `custom:codex`。
    pub backend_kind: String,

    /// 确定性分类结果。
    pub failure_kind: RecoverableFailureKind,

    /// 本条转移后的生命周期状态。
    pub status: RecoverableFailureStatus,

    /// 当前尝试序号,从 1 开始。
    pub attempt: u32,

    /// 配置允许的最大尝试次数。
    pub max_attempts: u32,

    /// 相对 retry 延迟。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_after_ms: Option<u64>,

    /// 绝对 next retry 时间,采用 RFC3339 字符串。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_retry_at: Option<String>,

    /// 外部进程退出码。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,

    /// 是否由 timeout 结束。
    pub timed_out: bool,

    /// 是否由 cancel 结束。
    pub canceled: bool,

    /// 有界 stderr 摘要。注意: 这是证据,不是 event parsing 输入。
    pub stderr_excerpt: String,

    /// 本条 transition 创建时间,采用 RFC3339 字符串。
    pub created_at: String,

    /// 可选源事件 id,用于和既有 event/record-session 证据关联。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_event_ids: Vec<String>,
}

impl RecoverableFailureTransition {
    /// 创建一个派生 snapshot,用于后续 ledger replay 的最小状态表达。
    #[must_use]
    pub fn to_snapshot(&self) -> RecoverableFailureSnapshot {
        RecoverableFailureSnapshot {
            failure_id: self.failure_id.clone(),
            job_id: self.job_id,
            instance_id: self.instance_id.clone(),
            hat_id: self.hat_id.clone(),
            backend_kind: self.backend_kind.clone(),
            failure_kind: self.failure_kind,
            status: self.status,
            attempt: self.attempt,
            max_attempts: self.max_attempts,
            retry_after_ms: self.retry_after_ms,
            next_retry_at: self.next_retry_at.clone(),
            exit_code: self.exit_code,
            timed_out: self.timed_out,
            canceled: self.canceled,
            stderr_excerpt: self.stderr_excerpt.clone(),
            updated_at: self.created_at.clone(),
            source_event_ids: self.source_event_ids.clone(),
        }
    }
}

/// Recoverable failure ledger replay 后的当前状态。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecoverableFailureSnapshot {
    pub failure_id: String,
    pub job_id: u64,
    pub instance_id: String,
    pub hat_id: String,
    pub backend_kind: String,
    pub failure_kind: RecoverableFailureKind,
    pub status: RecoverableFailureStatus,
    pub attempt: u32,
    pub max_attempts: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_after_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_retry_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub canceled: bool,
    pub stderr_excerpt: String,
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_event_ids: Vec<String>,
}

/// Recoverable failure ledger 的读写错误。
#[derive(Debug, Error)]
pub enum RecoverableFailureLedgerError {
    /// Ledger 文件或目录 IO 失败。
    #[error("recoverable failure ledger IO error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Transition 序列化失败。
    #[error("failed to serialize recoverable failure transition for {path}: {source}")]
    Serialize {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    /// JSONL 某一行不是合法 transition。
    #[error("failed to parse recoverable failure ledger line {line_number} at {path}: {source}")]
    ParseLine {
        path: PathBuf,
        line_number: usize,
        #[source]
        source: serde_json::Error,
    },
}

/// Append-only recoverable failure ledger。
///
/// 设计边界:
/// - 只负责 `.ralph/recoverable-failures.jsonl` 的 compact transition 证据。
/// - 不保存 prompt、raw event stream 或完整 stdout/stderr。
/// - 不负责 retry scheduling,只提供可回放的状态转移。
#[derive(Debug, Clone)]
pub struct RecoverableFailureLedger {
    path: PathBuf,
}

impl RecoverableFailureLedger {
    /// 创建指定路径的 ledger。
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// 返回 ledger path。
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 追加一条 transition。
    ///
    /// 写入前会复制并收紧 stderr excerpt,避免调用方意外把大块 stderr 或 prompt transcript
    /// 写进 evidence ledger。
    pub fn append_transition(
        &self,
        transition: &RecoverableFailureTransition,
    ) -> Result<(), RecoverableFailureLedgerError> {
        let mut transition = transition.clone();
        transition.stderr_excerpt =
            bounded_stderr_excerpt(&transition.stderr_excerpt, DEFAULT_STDERR_EXCERPT_CHARS);

        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|source| RecoverableFailureLedgerError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }

        let mut json_line = serde_json::to_string(&transition).map_err(|source| {
            RecoverableFailureLedgerError::Serialize {
                path: self.path.clone(),
                source,
            }
        })?;
        json_line.push('\n');

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|source| RecoverableFailureLedgerError::Io {
                path: self.path.clone(),
                source,
            })?;

        // JSONL 不变量: 一行是一条完整 transition。
        // 先组装完整 String,再一次性写入,降低半行 JSON 的概率。
        file.write_all(json_line.as_bytes()).map_err(|source| {
            RecoverableFailureLedgerError::Io {
                path: self.path.clone(),
                source,
            }
        })?;
        file.flush()
            .map_err(|source| RecoverableFailureLedgerError::Io {
                path: self.path.clone(),
                source,
            })?;

        Ok(())
    }

    /// 严格读取所有 transition。
    ///
    /// 缺失文件表示还没有 recoverable failure,返回空集合。
    /// 非空 malformed line 会返回带行号的错误,避免 silently corrupt audit trail。
    pub fn read_transitions(
        &self,
    ) -> Result<Vec<RecoverableFailureTransition>, RecoverableFailureLedgerError> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&self.path).map_err(|source| RecoverableFailureLedgerError::Io {
            path: self.path.clone(),
            source,
        })?;
        let reader = BufReader::new(file);
        let mut transitions = Vec::new();

        for (index, line) in reader.lines().enumerate() {
            let line_number = index + 1;
            let line = line.map_err(|source| RecoverableFailureLedgerError::Io {
                path: self.path.clone(),
                source,
            })?;
            if line.trim().is_empty() {
                continue;
            }

            let transition =
                serde_json::from_str::<RecoverableFailureTransition>(&line).map_err(|source| {
                    RecoverableFailureLedgerError::ParseLine {
                        path: self.path.clone(),
                        line_number,
                        source,
                    }
                })?;
            transitions.push(transition);
        }

        Ok(transitions)
    }

    /// replay ledger,按 `failure_id` 派生最新 snapshot。
    pub fn replay_snapshots(
        &self,
    ) -> Result<BTreeMap<String, RecoverableFailureSnapshot>, RecoverableFailureLedgerError> {
        let mut snapshots = BTreeMap::new();

        for transition in self.read_transitions()? {
            snapshots.insert(transition.failure_id.clone(), transition.to_snapshot());
        }

        Ok(snapshots)
    }
}

/// 为 recoverable lifecycle 生成稳定 failure id。
///
/// 说明:
/// - 这里仅使用 runtime correlation metadata,不包含 prompt / payload / stderr。
/// - 后续 runtime 如果已有更强 id,可以直接覆盖 transition.failure_id。
#[must_use]
pub fn stable_recoverable_failure_id(
    job_id: u64,
    instance_id: &str,
    failure_kind: RecoverableFailureKind,
) -> String {
    let sanitized_instance_id = instance_id
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>();

    format!(
        "recoverable-{job_id}-{sanitized_instance_id}-{}",
        failure_kind.as_str()
    )
}

/// 根据策略计算某次失败后的 retry 延迟。
///
/// `failed_attempt` 从 1 开始:
/// - 第 1 次失败后使用 `initial_delay_ms`;
/// - 第 2 次失败后乘以一次 `backoff_multiplier`;
/// - 最终始终受 `max_delay_ms` 约束。
#[must_use]
pub fn recoverable_retry_delay_ms(
    policy: &AgentCliRecoverableFailuresConfig,
    failed_attempt: u32,
) -> u64 {
    let exponent = failed_attempt.saturating_sub(1) as i32;
    let scaled = (policy.initial_delay_ms as f64) * policy.backoff_multiplier.powi(exponent);

    if !scaled.is_finite() {
        return policy.max_delay_ms;
    }

    scaled.ceil().clamp(1.0, policy.max_delay_ms as f64) as u64
}

/// 分类器需要的最小输入。
///
/// 使用这个薄结构,可以让分类器在单元测试、普通 CLI executor 和 parallel HatJobResult
/// 之间复用,同时不把 full prompt 或 event payload 带进 retry 决策。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecoverableFailureInput<'a> {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub canceled: bool,
    pub observed_stderr: &'a str,
}

impl<'a> From<&'a HatJobResult> for RecoverableFailureInput<'a> {
    fn from(result: &'a HatJobResult) -> Self {
        Self {
            success: result.success,
            exit_code: result.exit_code,
            timed_out: result.timed_out,
            canceled: result.canceled,
            observed_stderr: &result.observed_stderr,
        }
    }
}

/// 分类器的确定性输出。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoverableFailureClassification {
    pub failure_kind: RecoverableFailureKind,
    pub matched_pattern: &'static str,
    pub stderr_excerpt: String,
}

/// 从 HatJobResult 直接分类。
#[must_use]
pub fn classify_hat_job_result(result: &HatJobResult) -> Option<RecoverableFailureClassification> {
    classify_recoverable_failure(RecoverableFailureInput::from(result))
}

/// 确定性、窄范围地分类 agent CLI 可恢复失败。
///
/// 重要边界：
/// - stderr 只作为分类证据。
/// - 这里不会把 stderr 交给 EventParser。
/// - success / timeout / canceled 均不会自动进入 retry。
#[must_use]
pub fn classify_recoverable_failure(
    input: RecoverableFailureInput<'_>,
) -> Option<RecoverableFailureClassification> {
    if input.success || input.timed_out || input.canceled {
        return None;
    }

    let stderr = input.observed_stderr.trim();
    if stderr.is_empty() {
        return None;
    }

    let normalized = stderr.to_ascii_lowercase();
    let has_429_too_many_requests = has_429_too_many_requests(&normalized);
    let has_retry_limit = normalized.contains("exceeded retry limit");

    // retry limit 本身不是充分条件,必须伴随明确临时状态。
    if has_retry_limit && has_429_too_many_requests {
        return Some(classification(
            RecoverableFailureKind::RetryLimitExceeded,
            "exceeded retry limit + 429 too many requests",
            stderr,
        ));
    }

    if has_429_too_many_requests {
        return Some(classification(
            RecoverableFailureKind::RateLimited,
            "429 too many requests",
            stderr,
        ));
    }

    transient_network_pattern(&normalized).map(|matched_pattern| {
        classification(
            RecoverableFailureKind::TransientNetwork,
            matched_pattern,
            stderr,
        )
    })
}

fn has_429_too_many_requests(normalized: &str) -> bool {
    normalized.contains("429") && normalized.contains("too many requests")
}

fn transient_network_pattern(normalized: &str) -> Option<&'static str> {
    // 白名单要刻意保守,避免把应用自身错误误判成可恢复的网络错误。
    const PATTERNS: &[&str] = &[
        "connection reset by peer",
        "connection timed out",
        "temporary failure in name resolution",
        "network is unreachable",
    ];

    PATTERNS
        .iter()
        .copied()
        .find(|pattern| normalized.contains(pattern))
}

fn classification(
    failure_kind: RecoverableFailureKind,
    matched_pattern: &'static str,
    stderr: &str,
) -> RecoverableFailureClassification {
    RecoverableFailureClassification {
        failure_kind,
        matched_pattern,
        stderr_excerpt: bounded_stderr_excerpt(stderr, DEFAULT_STDERR_EXCERPT_CHARS),
    }
}

/// 生成 UTF-8 安全的 stderr 摘要。
#[must_use]
pub fn bounded_stderr_excerpt(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let excerpt = chars.by_ref().take(max_chars).collect::<String>();

    if chars.next().is_some() {
        format!("{excerpt}…")
    } else {
        excerpt
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use tempfile::TempDir;

    fn failed_input(stderr: &str) -> RecoverableFailureInput<'_> {
        RecoverableFailureInput {
            success: false,
            exit_code: Some(1),
            timed_out: false,
            canceled: false,
            observed_stderr: stderr,
        }
    }

    fn transition(
        failure_id: &str,
        status: RecoverableFailureStatus,
        attempt: u32,
        stderr_excerpt: &str,
    ) -> RecoverableFailureTransition {
        RecoverableFailureTransition {
            schema_version: RECOVERABLE_FAILURE_LEDGER_SCHEMA_VERSION,
            failure_id: failure_id.to_string(),
            job_id: 42,
            instance_id: "writer#1".to_string(),
            hat_id: "writer".to_string(),
            backend_kind: "codex".to_string(),
            failure_kind: RecoverableFailureKind::RateLimited,
            status,
            attempt,
            max_attempts: 3,
            retry_after_ms: Some(30_000),
            next_retry_at: Some(format!("2026-05-28T00:00:{attempt:02}Z")),
            exit_code: Some(1),
            timed_out: false,
            canceled: false,
            stderr_excerpt: stderr_excerpt.to_string(),
            created_at: format!("2026-05-28T00:00:{attempt:02}Z"),
            source_event_ids: vec![format!("evt-{attempt}")],
        }
    }

    #[test]
    fn classifies_plain_429_too_many_requests_as_rate_limited() {
        let classification =
            classify_recoverable_failure(failed_input("ERROR: last status: 429 Too Many Requests"))
                .expect("429 Too Many Requests should be recoverable");

        assert_eq!(
            classification.failure_kind,
            RecoverableFailureKind::RateLimited
        );
        assert_eq!(classification.matched_pattern, "429 too many requests");
    }

    #[test]
    fn classifies_retry_limit_with_429_as_retry_limit_exceeded() {
        let classification = classify_recoverable_failure(failed_input(
            "ERROR: exceeded retry limit, last status: 429 Too Many Requests",
        ))
        .expect("retry limit with temporary 429 should be recoverable");

        assert_eq!(
            classification.failure_kind,
            RecoverableFailureKind::RetryLimitExceeded
        );
        assert_eq!(
            classification.matched_pattern,
            "exceeded retry limit + 429 too many requests"
        );
    }

    #[test]
    fn retry_limit_without_temporary_status_is_terminal() {
        let classification = classify_recoverable_failure(failed_input(
            "ERROR: exceeded retry limit while applying patch",
        ));

        assert!(classification.is_none());
    }

    #[test]
    fn ordinary_command_failure_remains_terminal() {
        let classification = classify_recoverable_failure(failed_input(
            "error[E0425]: cannot find value `foo` in this scope",
        ));

        assert!(classification.is_none());
    }

    #[test]
    fn timeout_only_failure_is_not_automatically_recoverable() {
        let classification = classify_recoverable_failure(RecoverableFailureInput {
            success: false,
            exit_code: None,
            timed_out: true,
            canceled: false,
            observed_stderr: "ERROR: 429 Too Many Requests",
        });

        assert!(classification.is_none());
    }

    #[test]
    fn cancellation_only_failure_is_not_automatically_recoverable() {
        let classification = classify_recoverable_failure(RecoverableFailureInput {
            success: false,
            exit_code: None,
            timed_out: false,
            canceled: true,
            observed_stderr: "ERROR: 429 Too Many Requests",
        });

        assert!(classification.is_none());
    }

    #[test]
    fn success_result_is_not_recoverable_even_with_stderr() {
        let classification = classify_recoverable_failure(RecoverableFailureInput {
            success: true,
            exit_code: Some(0),
            timed_out: false,
            canceled: false,
            observed_stderr: "warning: 429 Too Many Requests",
        });

        assert!(classification.is_none());
    }

    #[test]
    fn classifies_curated_transient_network_pattern() {
        let classification =
            classify_recoverable_failure(failed_input("request failed: connection reset by peer"))
                .expect("curated transient network pattern should be recoverable");

        assert_eq!(
            classification.failure_kind,
            RecoverableFailureKind::TransientNetwork
        );
    }

    #[test]
    fn stderr_excerpt_is_utf8_safe_and_bounded() {
        let excerpt = bounded_stderr_excerpt("好好好abcdef", 4);

        assert_eq!(excerpt, "好好好a…");
    }

    #[test]
    fn transition_can_derive_snapshot() {
        let transition = RecoverableFailureTransition {
            schema_version: RECOVERABLE_FAILURE_LEDGER_SCHEMA_VERSION,
            failure_id: "failure-1".to_string(),
            job_id: 42,
            instance_id: "writer#1".to_string(),
            hat_id: "writer".to_string(),
            backend_kind: "codex".to_string(),
            failure_kind: RecoverableFailureKind::RateLimited,
            status: RecoverableFailureStatus::RetryScheduled,
            attempt: 1,
            max_attempts: 3,
            retry_after_ms: Some(30_000),
            next_retry_at: Some("2026-05-28T00:00:30Z".to_string()),
            exit_code: Some(1),
            timed_out: false,
            canceled: false,
            stderr_excerpt: "429 Too Many Requests".to_string(),
            created_at: "2026-05-28T00:00:00Z".to_string(),
            source_event_ids: vec!["evt-1".to_string()],
        };

        let snapshot = transition.to_snapshot();

        assert_eq!(snapshot.failure_id, "failure-1");
        assert_eq!(snapshot.status, RecoverableFailureStatus::RetryScheduled);
        assert_eq!(snapshot.source_event_ids, vec!["evt-1".to_string()]);
    }

    #[test]
    fn stable_failure_id_uses_only_correlation_metadata() {
        let failure_id = stable_recoverable_failure_id(
            42,
            "writer#1",
            RecoverableFailureKind::RetryLimitExceeded,
        );

        assert_eq!(failure_id, "recoverable-42-writer-1-retry_limit_exceeded");
        assert!(!failure_id.contains("prompt"));
        assert!(!failure_id.contains("payload"));
    }

    #[test]
    fn ledger_append_creates_parent_and_preserves_order() {
        let temp = TempDir::new().expect("tempdir");
        let path = temp.path().join(".ralph/recoverable-failures.jsonl");
        let ledger = RecoverableFailureLedger::new(&path);

        ledger
            .append_transition(&transition(
                "failure-1",
                RecoverableFailureStatus::RetryScheduled,
                1,
                "first",
            ))
            .expect("first append");
        ledger
            .append_transition(&transition(
                "failure-2",
                RecoverableFailureStatus::PausedRecoverable,
                1,
                "second",
            ))
            .expect("second append");

        let raw = std::fs::read_to_string(&path).expect("ledger file should exist");
        assert_eq!(raw.lines().count(), 2);

        let transitions = ledger.read_transitions().expect("read transitions");
        assert_eq!(transitions.len(), 2);
        assert_eq!(transitions[0].failure_id, "failure-1");
        assert_eq!(transitions[1].failure_id, "failure-2");
    }

    #[test]
    fn ledger_replay_uses_latest_transition_per_failure_id() {
        let temp = TempDir::new().expect("tempdir");
        let ledger =
            RecoverableFailureLedger::new(temp.path().join(".ralph/recoverable-failures.jsonl"));

        ledger
            .append_transition(&transition(
                "failure-1",
                RecoverableFailureStatus::RetryScheduled,
                1,
                "scheduled",
            ))
            .expect("append scheduled");
        ledger
            .append_transition(&transition(
                "failure-2",
                RecoverableFailureStatus::PausedRecoverable,
                1,
                "paused",
            ))
            .expect("append paused");
        ledger
            .append_transition(&transition(
                "failure-1",
                RecoverableFailureStatus::Retrying,
                2,
                "retrying",
            ))
            .expect("append retrying");

        let snapshots = ledger.replay_snapshots().expect("replay snapshots");

        assert_eq!(snapshots.len(), 2);
        let latest_failure_1 = snapshots.get("failure-1").expect("failure-1 snapshot");
        assert_eq!(latest_failure_1.status, RecoverableFailureStatus::Retrying);
        assert_eq!(latest_failure_1.attempt, 2);
        assert_eq!(latest_failure_1.stderr_excerpt, "retrying");
        assert_eq!(
            snapshots
                .get("failure-2")
                .expect("failure-2 snapshot")
                .status,
            RecoverableFailureStatus::PausedRecoverable
        );
    }

    #[test]
    fn retry_delay_uses_bounded_exponential_backoff() {
        let policy = AgentCliRecoverableFailuresConfig {
            enabled: true,
            max_attempts: 5,
            initial_delay_ms: 1_000,
            backoff_multiplier: 2.0,
            max_delay_ms: 2_500,
        };

        assert_eq!(recoverable_retry_delay_ms(&policy, 1), 1_000);
        assert_eq!(recoverable_retry_delay_ms(&policy, 2), 2_000);
        assert_eq!(recoverable_retry_delay_ms(&policy, 3), 2_500);
        assert_eq!(recoverable_retry_delay_ms(&policy, 99), 2_500);
    }

    #[test]
    fn retry_delay_treats_zero_attempt_as_first_attempt() {
        let policy = AgentCliRecoverableFailuresConfig {
            enabled: true,
            max_attempts: 3,
            initial_delay_ms: 750,
            backoff_multiplier: 1.5,
            max_delay_ms: 10_000,
        };

        assert_eq!(recoverable_retry_delay_ms(&policy, 0), 750);
    }

    #[test]
    fn missing_ledger_replays_to_empty_snapshot_map() {
        let temp = TempDir::new().expect("tempdir");
        let ledger =
            RecoverableFailureLedger::new(temp.path().join(".ralph/recoverable-failures.jsonl"));

        let transitions = ledger.read_transitions().expect("missing file is empty");
        let snapshots = ledger.replay_snapshots().expect("missing file is empty");

        assert!(transitions.is_empty());
        assert!(snapshots.is_empty());
    }

    #[test]
    fn malformed_ledger_line_returns_line_number() {
        let temp = TempDir::new().expect("tempdir");
        let path = temp.path().join(".ralph/recoverable-failures.jsonl");
        std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        std::fs::write(&path, "not-json\n").expect("write malformed line");
        let ledger = RecoverableFailureLedger::new(&path);

        let error = ledger
            .read_transitions()
            .expect_err("malformed line should fail strictly");

        assert!(matches!(
            error,
            RecoverableFailureLedgerError::ParseLine { line_number: 1, .. }
        ));
    }

    #[test]
    fn ledger_append_bounds_stderr_excerpt() {
        let temp = TempDir::new().expect("tempdir");
        let ledger =
            RecoverableFailureLedger::new(temp.path().join(".ralph/recoverable-failures.jsonl"));
        let long_stderr = "好".repeat(DEFAULT_STDERR_EXCERPT_CHARS + 10);

        ledger
            .append_transition(&transition(
                "failure-1",
                RecoverableFailureStatus::RetryScheduled,
                1,
                &long_stderr,
            ))
            .expect("append long stderr");

        let transitions = ledger.read_transitions().expect("read bounded transition");
        let excerpt = &transitions[0].stderr_excerpt;

        assert_eq!(excerpt.chars().count(), DEFAULT_STDERR_EXCERPT_CHARS + 1);
        assert!(excerpt.ends_with('…'));
    }

    #[test]
    fn ledger_transition_metadata_is_compact() {
        let transition = transition(
            "failure-compact",
            RecoverableFailureStatus::RetryScheduled,
            1,
            "ERROR: 429 Too Many Requests",
        );
        let value = serde_json::to_value(&transition).expect("serialize transition");
        let object = value.as_object().expect("transition should be object");

        // Ledger 只保存 runtime correlation 和 compact stderr 证据。
        // 不应出现 full prompt、raw payload 或 EventParser 输入字段。
        assert!(!object.contains_key("prompt"));
        assert!(!object.contains_key("payload"));
        assert!(!object.contains_key("output_for_parsing"));
        assert_eq!(
            object.get("failure_id"),
            Some(&Value::String("failure-compact".to_string()))
        );
        assert_eq!(
            object.get("source_event_ids"),
            Some(&Value::Array(vec![Value::String("evt-1".to_string())]))
        );
    }
}
