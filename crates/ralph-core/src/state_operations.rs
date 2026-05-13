//! Runtime workflow state operations.
//!
//! 这个模块只负责 Ralph 的 runtime workflow lifecycle state。
//! 它不替代 `.agent/memories.md`、`.agent/tasks.jsonl`、事件 JSONL 或 record-session。

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use thiserror::Error;

/// Ralph state root relative to workspace root.
const STATE_ROOT_SEGMENT: &str = ".ralph/state";

/// Process-unique counter for temporary file names.
static TMP_WRITE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Path-level write locks.
static PATH_WRITE_LOCKS: OnceLock<Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>> = OnceLock::new();

fn write_locks() -> &'static Mutex<HashMap<PathBuf, Arc<Mutex<()>>>> {
    PATH_WRITE_LOCKS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn lock_unpoisoned<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn write_lock_for(path: &Path) -> Arc<Mutex<()>> {
    let mut locks = lock_unpoisoned(write_locks());
    locks
        .entry(path.to_path_buf())
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone()
}

fn validate_session_id(session_id: &str) -> Result<&str, StateOperationError> {
    if session_id.is_empty()
        || session_id == "."
        || session_id == ".."
        || session_id.contains('/')
        || session_id.contains('\\')
        || !session_id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    {
        return Err(StateOperationError::InvalidSessionId {
            session_id: session_id.to_string(),
        });
    }

    Ok(session_id)
}

fn write_json_pretty(path: &Path, value: &impl Serialize) -> Result<(), StateOperationError> {
    let parent = path
        .parent()
        .ok_or_else(|| StateOperationError::InvalidStatePath {
            path: path.to_path_buf(),
        })?;
    fs::create_dir_all(parent).map_err(|source| StateOperationError::Io {
        path: parent.to_path_buf(),
        source,
    })?;

    let tmp_name = format!(
        "{}.tmp.{}.{}",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("state"),
        std::process::id(),
        TMP_WRITE_COUNTER.fetch_add(1, Ordering::Relaxed),
    );
    let tmp_path = path.with_file_name(tmp_name);

    let json = serde_json::to_string_pretty(value).map_err(|source| {
        StateOperationError::SerializeJson {
            path: path.to_path_buf(),
            source,
        }
    })?;

    fs::write(&tmp_path, json).map_err(|source| StateOperationError::Io {
        path: tmp_path.clone(),
        source,
    })?;

    match fs::rename(&tmp_path, path) {
        Ok(()) => Ok(()),
        Err(source) => {
            let _ = fs::remove_file(&tmp_path);
            Err(StateOperationError::Io {
                path: path.to_path_buf(),
                source,
            })
        }
    }
}

fn read_json_record(path: &Path) -> Result<RuntimeStateRecord, StateOperationError> {
    let content = fs::read_to_string(path).map_err(|source| StateOperationError::Io {
        path: path.to_path_buf(),
        source,
    })?;

    serde_json::from_str(&content).map_err(|source| StateOperationError::MalformedJson {
        path: path.to_path_buf(),
        source,
    })
}

fn remove_if_exists(path: &Path) -> Result<bool, StateOperationError> {
    if !path.exists() {
        return Ok(false);
    }

    fs::remove_file(path).map_err(|source| StateOperationError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(true)
}

fn record_status(path: &Path, mode: StateMode) -> StateStatus {
    match read_json_record(path) {
        Ok(record) => StateStatus {
            mode,
            active: Some(record.active),
            current_phase: record.current_phase,
            run_outcome: record.run_outcome,
            lifecycle_outcome: record.lifecycle_outcome,
            path: path.to_path_buf(),
            error: None,
        },
        Err(error) => StateStatus {
            mode,
            active: None,
            current_phase: None,
            run_outcome: None,
            lifecycle_outcome: None,
            path: path.to_path_buf(),
            error: Some(error.to_string()),
        },
    }
}

/// State record modes supported by the v1 contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StateMode {
    Ralph,
    Ralplan,
    Team,
    DeepInterview,
    CapabilityInvocation,
}

impl StateMode {
    /// Returns all supported modes in a deterministic order.
    #[must_use]
    pub const fn all() -> [Self; 5] {
        [
            Self::Ralph,
            Self::Ralplan,
            Self::Team,
            Self::DeepInterview,
            Self::CapabilityInvocation,
        ]
    }

    /// Returns the canonical string form.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ralph => "ralph",
            Self::Ralplan => "ralplan",
            Self::Team => "team",
            Self::DeepInterview => "deep-interview",
            Self::CapabilityInvocation => "capability-invocation",
        }
    }
}

impl std::fmt::Display for StateMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for StateMode {
    type Err = StateOperationError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "ralph" => Ok(Self::Ralph),
            "ralplan" => Ok(Self::Ralplan),
            "team" => Ok(Self::Team),
            "deep-interview" => Ok(Self::DeepInterview),
            "capability-invocation" => Ok(Self::CapabilityInvocation),
            other => Err(StateOperationError::UnsupportedMode {
                mode: other.to_string(),
            }),
        }
    }
}

/// Runtime workflow run outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunOutcome {
    Continue,
    Finish,
    BlockedOnUser,
    Failed,
    Cancelled,
}

impl RunOutcome {
    /// Returns the canonical string form.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Continue => "continue",
            Self::Finish => "finish",
            Self::BlockedOnUser => "blocked_on_user",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

impl std::fmt::Display for RunOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for RunOutcome {
    type Err = StateOperationError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "continue" => Ok(Self::Continue),
            "finish" => Ok(Self::Finish),
            "blocked_on_user" => Ok(Self::BlockedOnUser),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            other => Err(StateOperationError::InvalidRunOutcome {
                value: other.to_string(),
            }),
        }
    }
}

/// Runtime workflow lifecycle outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleOutcome {
    Finished,
    Blocked,
    Failed,
    #[serde(rename = "userinterlude")]
    UserInterlude,
    #[serde(rename = "askuser_question")]
    AskUserQuestion,
}

impl LifecycleOutcome {
    /// Returns the canonical string form.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Finished => "finished",
            Self::Blocked => "blocked",
            Self::Failed => "failed",
            Self::UserInterlude => "userinterlude",
            Self::AskUserQuestion => "askuser_question",
        }
    }
}

impl std::fmt::Display for LifecycleOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for LifecycleOutcome {
    type Err = StateOperationError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "finished" => Ok(Self::Finished),
            "blocked" => Ok(Self::Blocked),
            "failed" => Ok(Self::Failed),
            "userinterlude" => Ok(Self::UserInterlude),
            "askuser_question" => Ok(Self::AskUserQuestion),
            other => Err(StateOperationError::InvalidLifecycleOutcome {
                value: other.to_string(),
            }),
        }
    }
}

/// Runtime workflow state stored under `.ralph/state`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeStateRecord {
    pub mode: StateMode,

    #[serde(default)]
    pub active: bool,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_phase: Option<String>,

    pub updated_at: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_outcome: Option<RunOutcome>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifecycle_outcome: Option<LifecycleOutcome>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,

    #[serde(default)]
    pub state: Map<String, Value>,
}

impl RuntimeStateRecord {
    /// Creates a minimal record.
    #[must_use]
    pub fn new(mode: StateMode, session_id: Option<String>) -> Self {
        Self {
            mode,
            active: false,
            current_phase: None,
            updated_at: Utc::now().to_rfc3339(),
            run_outcome: None,
            lifecycle_outcome: None,
            session_id,
            state: Map::new(),
        }
    }
}

/// State write request.
#[derive(Debug, Clone)]
pub struct StateWriteRequest {
    pub mode: StateMode,
    pub session_id: Option<String>,
    pub active: Option<bool>,
    pub current_phase: Option<String>,
    pub run_outcome: Option<RunOutcome>,
    pub lifecycle_outcome: Option<LifecycleOutcome>,
    pub updated_at: Option<String>,
    pub state: Map<String, Value>,
}

impl StateWriteRequest {
    /// Creates a request for the given mode.
    #[must_use]
    pub fn new(mode: StateMode) -> Self {
        Self {
            mode,
            session_id: None,
            active: None,
            current_phase: None,
            run_outcome: None,
            lifecycle_outcome: None,
            updated_at: None,
            state: Map::new(),
        }
    }

    /// Adds a session scope.
    #[must_use]
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Marks the state as active or inactive.
    #[must_use]
    pub fn with_active(mut self, active: bool) -> Self {
        self.active = Some(active);
        self
    }

    /// Sets the current phase.
    #[must_use]
    pub fn with_current_phase(mut self, current_phase: impl Into<String>) -> Self {
        self.current_phase = Some(current_phase.into());
        self
    }

    /// Sets the run outcome.
    #[must_use]
    pub fn with_run_outcome(mut self, run_outcome: RunOutcome) -> Self {
        self.run_outcome = Some(run_outcome);
        self
    }

    /// Sets the lifecycle outcome.
    #[must_use]
    pub fn with_lifecycle_outcome(mut self, lifecycle_outcome: LifecycleOutcome) -> Self {
        self.lifecycle_outcome = Some(lifecycle_outcome);
        self
    }

    /// Overrides the timestamp used for the write.
    #[must_use]
    pub fn with_updated_at(mut self, updated_at: impl Into<String>) -> Self {
        self.updated_at = Some(updated_at.into());
        self
    }

    /// Replaces the custom state payload.
    #[must_use]
    pub fn with_state(mut self, state: Map<String, Value>) -> Self {
        self.state = state;
        self
    }
}

/// State clear request.
#[derive(Debug, Clone)]
pub struct StateClearRequest {
    pub mode: StateMode,
    pub session_id: Option<String>,
    pub all_sessions: bool,
}

impl StateClearRequest {
    /// Creates a request for the given mode.
    #[must_use]
    pub fn new(mode: StateMode) -> Self {
        Self {
            mode,
            session_id: None,
            all_sessions: false,
        }
    }

    /// Adds a session scope.
    #[must_use]
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Clears all sessions for the mode.
    #[must_use]
    pub fn with_all_sessions(mut self, all_sessions: bool) -> Self {
        self.all_sessions = all_sessions;
        self
    }
}

/// State read result.
#[derive(Debug, Clone, PartialEq)]
pub struct StateReadResult {
    pub mode: StateMode,
    pub path: Option<PathBuf>,
    pub record: Option<RuntimeStateRecord>,
}

impl StateReadResult {
    /// Returns whether a record exists.
    #[must_use]
    pub fn exists(&self) -> bool {
        self.record.is_some()
    }
}

/// State write result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateWriteResult {
    pub mode: StateMode,
    pub path: PathBuf,
}

/// State clear result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateClearResult {
    pub mode: StateMode,
    pub removed_paths: Vec<PathBuf>,
}

/// State summary for a mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateStatus {
    pub mode: StateMode,
    pub active: Option<bool>,
    pub current_phase: Option<String>,
    pub run_outcome: Option<RunOutcome>,
    pub lifecycle_outcome: Option<LifecycleOutcome>,
    pub path: PathBuf,
    pub error: Option<String>,
}

/// Unified runtime workflow state store.
#[derive(Debug, Clone)]
pub struct StateOperationStore {
    workspace_root: PathBuf,
}

impl StateOperationStore {
    /// Creates a new state store rooted at the given workspace.
    #[must_use]
    pub fn new(workspace_root: impl AsRef<Path>) -> Self {
        Self {
            workspace_root: workspace_root.as_ref().to_path_buf(),
        }
    }

    /// Returns the root directory used for state files.
    #[must_use]
    pub fn state_root(&self) -> PathBuf {
        self.workspace_root.join(STATE_ROOT_SEGMENT)
    }

    /// Resolves the concrete state path for a mode and optional session.
    pub fn state_path(
        &self,
        mode: StateMode,
        session_id: Option<&str>,
    ) -> Result<PathBuf, StateOperationError> {
        match session_id {
            Some(session_id) => Ok(self
                .state_root()
                .join("sessions")
                .join(validate_session_id(session_id)?)
                .join(format!("{}-state.json", mode.as_str()))),
            None => Ok(self
                .state_root()
                .join(format!("{}-state.json", mode.as_str()))),
        }
    }

    /// Reads runtime workflow state.
    pub fn state_read(
        &self,
        mode: StateMode,
        session_id: Option<&str>,
    ) -> Result<StateReadResult, StateOperationError> {
        let path = self.resolve_read_path(mode, session_id)?;
        match path {
            Some(path) => Ok(StateReadResult {
                mode,
                path: Some(path.clone()),
                record: Some(read_json_record(&path)?),
            }),
            None => Ok(StateReadResult {
                mode,
                path: None,
                record: None,
            }),
        }
    }

    /// Writes runtime workflow state.
    pub fn state_write(
        &self,
        request: StateWriteRequest,
    ) -> Result<StateWriteResult, StateOperationError> {
        let path = self.state_path(request.mode, request.session_id.as_deref())?;
        let lock = write_lock_for(&path);
        let _guard = lock_unpoisoned(&lock);

        let mut record = if path.exists() {
            read_json_record(&path)?
        } else {
            RuntimeStateRecord::new(request.mode, request.session_id.clone())
        };

        record.mode = request.mode;
        record.session_id = request.session_id.clone();
        record.active = request.active.unwrap_or(record.active);
        if let Some(current_phase) = request.current_phase {
            record.current_phase = Some(current_phase);
        }
        if let Some(run_outcome) = request.run_outcome {
            record.run_outcome = Some(run_outcome);
        }
        if let Some(lifecycle_outcome) = request.lifecycle_outcome {
            record.lifecycle_outcome = Some(lifecycle_outcome);
        }
        record.updated_at = request
            .updated_at
            .unwrap_or_else(|| Utc::now().to_rfc3339());
        for (key, value) in request.state {
            record.state.insert(key, value);
        }

        write_json_pretty(&path, &record)?;

        Ok(StateWriteResult {
            mode: request.mode,
            path,
        })
    }

    /// Clears runtime workflow state.
    pub fn state_clear(
        &self,
        request: StateClearRequest,
    ) -> Result<StateClearResult, StateOperationError> {
        let mut removed_paths = Vec::new();

        if request.all_sessions {
            let global_path = self.state_path(request.mode, None)?;
            if remove_if_exists(&global_path)? {
                removed_paths.push(global_path);
            }

            let sessions_root = self.state_root().join("sessions");
            if sessions_root.exists() {
                let entries =
                    fs::read_dir(&sessions_root).map_err(|source| StateOperationError::Io {
                        path: sessions_root.clone(),
                        source,
                    })?;
                for entry in entries {
                    let entry = entry.map_err(|source| StateOperationError::Io {
                        path: sessions_root.clone(),
                        source,
                    })?;
                    let path = entry
                        .path()
                        .join(format!("{}-state.json", request.mode.as_str()));
                    if remove_if_exists(&path)? {
                        removed_paths.push(path);
                    }
                }
            }
        } else {
            let path = self.state_path(request.mode, request.session_id.as_deref())?;
            if remove_if_exists(&path)? {
                removed_paths.push(path);
            }
        }

        Ok(StateClearResult {
            mode: request.mode,
            removed_paths,
        })
    }

    /// Returns a deterministic list of active modes.
    pub fn state_list_active(
        &self,
        session_id: Option<&str>,
    ) -> Result<Vec<StateMode>, StateOperationError> {
        let statuses = self.state_get_status(None, session_id)?;
        Ok(statuses
            .into_iter()
            .filter_map(|(mode, status)| status.active.filter(|active| *active).map(|_| mode))
            .collect())
    }

    /// Returns status for one mode or all modes.
    pub fn state_get_status(
        &self,
        mode: Option<StateMode>,
        session_id: Option<&str>,
    ) -> Result<BTreeMap<StateMode, StateStatus>, StateOperationError> {
        let mut statuses = BTreeMap::new();

        match mode {
            Some(mode) => {
                if let Some(path) = self.resolve_read_path(mode, session_id)? {
                    statuses.insert(mode, record_status(&path, mode));
                }
            }
            None => {
                for supported_mode in StateMode::all() {
                    if let Some(path) = self.resolve_read_path(supported_mode, session_id)? {
                        statuses.insert(supported_mode, record_status(&path, supported_mode));
                    }
                }
            }
        }

        Ok(statuses)
    }

    fn resolve_read_path(
        &self,
        mode: StateMode,
        session_id: Option<&str>,
    ) -> Result<Option<PathBuf>, StateOperationError> {
        if let Some(session_id) = session_id {
            let session_path = self.state_path(mode, Some(session_id))?;
            if session_path.exists() {
                return Ok(Some(session_path));
            }
        }

        let global_path = self.state_path(mode, None)?;
        if global_path.exists() {
            return Ok(Some(global_path));
        }

        Ok(None)
    }
}

/// Errors returned by the state operation layer.
#[derive(Debug, Error)]
pub enum StateOperationError {
    #[error("unsupported state mode: {mode}")]
    UnsupportedMode { mode: String },

    #[error("invalid session id: {session_id}")]
    InvalidSessionId { session_id: String },

    #[error("invalid state path: {path}")]
    InvalidStatePath { path: PathBuf },

    #[error("invalid run outcome: {value}")]
    InvalidRunOutcome { value: String },

    #[error("invalid lifecycle outcome: {value}")]
    InvalidLifecycleOutcome { value: String },

    #[error("io error for {path}: {source}")]
    Io { path: PathBuf, source: io::Error },

    #[error("failed to serialize state for {path}: {source}")]
    SerializeJson {
        path: PathBuf,
        source: serde_json::Error,
    },

    #[error("malformed state file {path}: {source}")]
    MalformedJson {
        path: PathBuf,
        source: serde_json::Error,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;
    use std::sync::Arc;
    use std::thread;
    use tempfile::TempDir;

    fn state_object(value: serde_json::Value) -> serde_json::Map<String, serde_json::Value> {
        value
            .as_object()
            .cloned()
            .expect("test state must be object")
    }

    #[test]
    fn valid_write_read_roundtrip_uses_global_state_path() {
        let tmp = TempDir::new().unwrap();
        let store = StateOperationStore::new(tmp.path());

        let request = StateWriteRequest::new(StateMode::Ralph)
            .with_active(true)
            .with_current_phase("running")
            .with_run_outcome(RunOutcome::Continue)
            .with_lifecycle_outcome(LifecycleOutcome::Finished)
            .with_state(state_object(json!({"owner": "ralph#1"})));

        let write = store.state_write(request).unwrap();
        assert_eq!(write.path, tmp.path().join(".ralph/state/ralph-state.json"));

        let read = store.state_read(StateMode::Ralph, None).unwrap();
        let record = read.record.expect("state should exist after write");
        assert_eq!(record.mode, StateMode::Ralph);
        assert!(record.active);
        assert_eq!(record.current_phase.as_deref(), Some("running"));
        assert_eq!(record.run_outcome, Some(RunOutcome::Continue));
        assert_eq!(record.lifecycle_outcome, Some(LifecycleOutcome::Finished));
        assert_eq!(record.state.get("owner"), Some(&json!("ralph#1")));
    }

    #[test]
    fn unsupported_mode_and_invalid_outcomes_are_rejected() {
        assert!(matches!(
            "unknown".parse::<StateMode>(),
            Err(StateOperationError::UnsupportedMode { .. })
        ));
        assert!(matches!(
            "askuserQuestion".parse::<LifecycleOutcome>(),
            Err(StateOperationError::InvalidLifecycleOutcome { .. })
        ));
        assert!(matches!(
            "retry".parse::<RunOutcome>(),
            Err(StateOperationError::InvalidRunOutcome { .. })
        ));
    }

    #[test]
    fn session_scoped_write_uses_session_state_path() {
        let tmp = TempDir::new().unwrap();
        let store = StateOperationStore::new(tmp.path());

        let write = store
            .state_write(
                StateWriteRequest::new(StateMode::DeepInterview)
                    .with_session_id("session-1")
                    .with_active(true),
            )
            .unwrap();

        assert_eq!(
            write.path,
            tmp.path()
                .join(".ralph/state/sessions/session-1/deep-interview-state.json")
        );
        assert!(
            !tmp.path()
                .join(".ralph/state/deep-interview-state.json")
                .exists()
        );
    }

    #[test]
    fn malformed_json_read_returns_structured_error_and_status_reports_it() {
        let tmp = TempDir::new().unwrap();
        let store = StateOperationStore::new(tmp.path());
        let path = store.state_path(StateMode::Team, None).unwrap();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "not json").unwrap();

        let err = store.state_read(StateMode::Team, None).unwrap_err();
        assert!(matches!(err, StateOperationError::MalformedJson { .. }));

        let statuses = store.state_get_status(Some(StateMode::Team), None).unwrap();
        let status = statuses.get(&StateMode::Team).unwrap();
        assert_eq!(status.path, path);
        assert!(status.error.as_deref().unwrap().contains("malformed"));
    }

    #[test]
    fn state_write_merges_custom_state_without_overwriting_unspecified_fields() {
        let tmp = TempDir::new().unwrap();
        let store = StateOperationStore::new(tmp.path());

        store
            .state_write(
                StateWriteRequest::new(StateMode::Ralplan)
                    .with_active(true)
                    .with_current_phase("draft")
                    .with_state(state_object(json!({"keep": 1, "replace": "old"}))),
            )
            .unwrap();

        store
            .state_write(
                StateWriteRequest::new(StateMode::Ralplan)
                    .with_state(state_object(json!({"replace": "new", "added": 2}))),
            )
            .unwrap();

        let record = store
            .state_read(StateMode::Ralplan, None)
            .unwrap()
            .record
            .unwrap();
        assert!(record.active, "unspecified active should be preserved");
        assert_eq!(record.current_phase.as_deref(), Some("draft"));
        assert_eq!(record.state.get("keep"), Some(&json!(1)));
        assert_eq!(record.state.get("replace"), Some(&json!("new")));
        assert_eq!(record.state.get("added"), Some(&json!(2)));
    }

    #[test]
    fn session_clear_does_not_delete_global_or_other_sessions() {
        let tmp = TempDir::new().unwrap();
        let store = StateOperationStore::new(tmp.path());

        store
            .state_write(StateWriteRequest::new(StateMode::Ralph).with_active(true))
            .unwrap();
        store
            .state_write(
                StateWriteRequest::new(StateMode::Ralph)
                    .with_session_id("session-a")
                    .with_active(true),
            )
            .unwrap();
        store
            .state_write(
                StateWriteRequest::new(StateMode::Ralph)
                    .with_session_id("session-b")
                    .with_active(true),
            )
            .unwrap();

        let cleared = store
            .state_clear(StateClearRequest::new(StateMode::Ralph).with_session_id("session-a"))
            .unwrap();
        assert_eq!(cleared.removed_paths.len(), 1);
        assert!(store.state_path(StateMode::Ralph, None).unwrap().exists());
        assert!(
            !store
                .state_path(StateMode::Ralph, Some("session-a"))
                .unwrap()
                .exists()
        );
        assert!(
            store
                .state_path(StateMode::Ralph, Some("session-b"))
                .unwrap()
                .exists()
        );
    }

    #[test]
    fn clear_all_sessions_removes_global_and_session_scoped_paths() {
        let tmp = TempDir::new().unwrap();
        let store = StateOperationStore::new(tmp.path());

        store
            .state_write(StateWriteRequest::new(StateMode::Team).with_active(true))
            .unwrap();
        store
            .state_write(
                StateWriteRequest::new(StateMode::Team)
                    .with_session_id("session-a")
                    .with_active(true),
            )
            .unwrap();
        store
            .state_write(
                StateWriteRequest::new(StateMode::Team)
                    .with_session_id("session-b")
                    .with_active(true),
            )
            .unwrap();

        let cleared = store
            .state_clear(StateClearRequest::new(StateMode::Team).with_all_sessions(true))
            .unwrap();
        assert_eq!(cleared.removed_paths.len(), 3);
        assert!(!store.state_path(StateMode::Team, None).unwrap().exists());
        assert!(
            !store
                .state_path(StateMode::Team, Some("session-a"))
                .unwrap()
                .exists()
        );
        assert!(
            !store
                .state_path(StateMode::Team, Some("session-b"))
                .unwrap()
                .exists()
        );
    }

    #[test]
    fn list_active_uses_session_scope_before_global_scope() {
        let tmp = TempDir::new().unwrap();
        let store = StateOperationStore::new(tmp.path());

        store
            .state_write(StateWriteRequest::new(StateMode::Team).with_active(true))
            .unwrap();
        store
            .state_write(
                StateWriteRequest::new(StateMode::Team)
                    .with_session_id("session-a")
                    .with_active(false),
            )
            .unwrap();
        store
            .state_write(
                StateWriteRequest::new(StateMode::Ralph)
                    .with_session_id("session-a")
                    .with_active(true),
            )
            .unwrap();

        assert_eq!(
            store.state_list_active(None).unwrap(),
            vec![StateMode::Team]
        );
        assert_eq!(
            store.state_list_active(Some("session-a")).unwrap(),
            vec![StateMode::Ralph]
        );
    }

    #[test]
    fn concurrent_writes_to_same_path_keep_valid_json() {
        let tmp = TempDir::new().unwrap();
        let store = Arc::new(StateOperationStore::new(tmp.path()));

        let handles: Vec<_> = (0..16)
            .map(|index| {
                let store = Arc::clone(&store);
                thread::spawn(move || {
                    store
                        .state_write(
                            StateWriteRequest::new(StateMode::CapabilityInvocation)
                                .with_active(true)
                                .with_current_phase(format!("phase-{index}"))
                                .with_state(state_object(json!({"index": index}))),
                        )
                        .unwrap();
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        let path = store
            .state_path(StateMode::CapabilityInvocation, None)
            .unwrap();
        let content = fs::read_to_string(&path).unwrap();
        let record: RuntimeStateRecord = serde_json::from_str(&content).unwrap();
        assert!(record.active);
        let index = record.state.get("index").and_then(Value::as_u64).unwrap();
        assert!(index < 16);
    }
}
