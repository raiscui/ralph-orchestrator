//! Codex App Server 常驻会话运行时（parallel 模式专用）。
//!
//! 设计目标：
//! - 为指定 hat instance 提供 `session_strategy=app_server` 的执行通路；
//! - 支持 turn 级 in-flight 控制：
//!   - `turn/steer`: 运行中追加输入（真 steer）
//!   - `turn/interrupt`: 中断当前 turn（不中断 thread）
//! - 维持与现有并行模型一致的可观测输出：
//!   - 只解析 stdout(事件解析 stdout-only)。
//!   - stderr 仅用于可观测输出/诊断(灰色),并可被 cassette 录制,但绝不参与事件解析。

use anyhow::{Context, Result};
use ralph_adapters::CliBackend;
use ralph_core::{HatJob, HatJobControl, HatJobOutputChunk, HatJobResult, OutputStream};
use ralph_proto::HatInstanceId;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::{Mutex, broadcast, mpsc, watch};
use tracing::{debug, warn};

#[derive(Debug, Clone, Default)]
struct CodexAppServerOptions {
    model: Option<String>,
    profile: Option<String>,
    cwd: Option<String>,
    sandbox: Option<String>,
    approval_policy: Option<String>,
}

fn parse_codex_app_server_options(
    backend: &CliBackend,
    workdir: Option<&Path>,
) -> CodexAppServerOptions {
    // ------------------------------------------------------------------
    // 说明:
    // - 这里解析的是 codex CLI 风格参数（与 `codex exec` 兼容的那组）。
    // - 我们只提取 App Server 运行时真正需要的参数，未知参数不透传。
    // ------------------------------------------------------------------
    let mut options = CodexAppServerOptions::default();
    let mut iter = backend.args.iter();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            // `codex exec` 子命令在 app-server 通道里不需要。
            "exec" => {}
            "--full-auto" => {
                // 与 Codex MCP runtime 保持一致的映射，避免行为漂移。
                options.sandbox = Some("workspace-write".to_string());
                options.approval_policy = Some("on-request".to_string());
            }
            "--model" | "-m" => {
                if let Some(model) = iter.next() {
                    options.model = Some(model.clone());
                }
            }
            "--sandbox" | "-s" => {
                if let Some(sandbox) = iter.next() {
                    options.sandbox = Some(sandbox.clone());
                }
            }
            "--ask-for-approval" | "-a" => {
                if let Some(policy) = iter.next() {
                    options.approval_policy = Some(policy.clone());
                }
            }
            "--profile" | "-p" => {
                if let Some(profile) = iter.next() {
                    options.profile = Some(profile.clone());
                }
            }
            "--cd" | "-C" => {
                if let Some(cwd) = iter.next() {
                    options.cwd = Some(cwd.clone());
                }
            }
            _ => {}
        }
    }

    // 如果未显式指定 cwd,则使用 job.workdir.
    if options.cwd.is_none()
        && let Some(workdir) = workdir
    {
        options.cwd = Some(workdir.display().to_string());
    }

    options
}

#[derive(Debug)]
struct CodexAppServerSession {
    instance_id: String,
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    /// stderr 流式通道(只用于可观测输出,不参与事件解析).
    ///
    /// 说明:
    /// - Codex app-server 的 stderr 通常是运行时日志/诊断信息.
    /// - 我们需要在 parallel 模式下把它也显示出来(灰色),并可选录制到 cassette.
    /// - 但必须保证它永远不进入 `HatJobResult.output`，避免 `<event ...>` 假事件污染解析.
    stderr_tx: broadcast::Sender<String>,
    next_request_id: u64,
    initialized: bool,
    thread_id: Option<String>,
    active_turn_id: Option<String>,
}

impl CodexAppServerSession {
    fn spawn(instance_id: &str) -> Result<Self> {
        let mut command = Command::new("codex");
        command.arg("app-server");
        command.arg("--listen");
        command.arg("stdio://");
        command.stdin(Stdio::piped());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        command.kill_on_drop(true);

        let mut child = command
            .spawn()
            .with_context(|| format!("Failed to spawn codex app-server for {instance_id}"))?;

        let stdin = child
            .stdin
            .take()
            .context("Failed to take codex app-server stdin")?;
        let stdout = child
            .stdout
            .take()
            .context("Failed to take codex app-server stdout")?;

        // ------------------------------------------------------------------
        // stderr 流式通道:
        // - 后台持续消费 stderr,避免 pipe buffer 堵塞；
        // - 把 stderr 行通过 broadcast 转发给当前 job,用于可观测输出(灰色).
        // - 重要: stderr 不参与事件解析,仅用于诊断/可视化.
        // ------------------------------------------------------------------
        let (stderr_tx, _stderr_rx) = broadcast::channel::<String>(256);

        if let Some(stderr) = child.stderr.take() {
            let stderr_instance = instance_id.to_string();
            let stderr_forward = stderr_tx.clone();
            tokio::spawn(async move {
                let mut reader = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    // best-effort: 转发给订阅者(无人订阅会返回 Err,可忽略).
                    let _ = stderr_forward.send(line.clone());
                    // 说明:
                    // - stderr 的主要观测面在 parallel 输出流(灰色).
                    // - 这里保留 debug 级日志,用于排障时按需开启,避免默认刷屏.
                    debug!(instance = %stderr_instance, line = %line, "codex app-server stderr");
                }
            });
        }

        Ok(Self {
            instance_id: instance_id.to_string(),
            child,
            stdin,
            stdout: BufReader::new(stdout),
            stderr_tx,
            next_request_id: 1,
            initialized: false,
            thread_id: None,
            active_turn_id: None,
        })
    }

    fn subscribe_stderr(&self) -> broadcast::Receiver<String> {
        self.stderr_tx.subscribe()
    }

    async fn shutdown(&mut self) {
        let _ = self.child.start_kill();
        let _ = self.child.wait().await;
    }

    async fn write_json(&mut self, value: &Value) -> Result<()> {
        let line = serde_json::to_string(value).context("Failed to serialize app-server json")?;
        self.stdin
            .write_all(line.as_bytes())
            .await
            .context("Failed to write app-server json line")?;
        self.stdin
            .write_all(b"\n")
            .await
            .context("Failed to write app-server newline")?;
        self.stdin.flush().await.ok(); // best-effort
        Ok(())
    }

    async fn read_json_line(&mut self) -> Result<Option<Value>> {
        let mut line = String::new();
        let bytes = self
            .stdout
            .read_line(&mut line)
            .await
            .context("Failed to read app-server stdout line")?;
        if bytes == 0 {
            return Ok(None);
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Ok(Some(json!({})));
        }

        let value: Value =
            serde_json::from_str(trimmed).context("Failed to parse app-server json")?;
        Ok(Some(value))
    }

    async fn ensure_initialized(&mut self) -> Result<()> {
        if self.initialized {
            return Ok(());
        }

        let id = self.next_request_id;
        self.next_request_id = self.next_request_id.saturating_add(1);

        // 初始化握手: initialize -> (response) -> initialized(notification)
        let req = json!({
            "id": id,
            "method": "initialize",
            "params": {
                "clientInfo": {
                    "name": "ralph",
                    "version": env!("CARGO_PKG_VERSION"),
                }
            }
        });
        self.write_json(&req).await?;

        loop {
            let Some(msg) = self.read_json_line().await? else {
                anyhow::bail!("codex app-server exited before initialize completed");
            };
            if msg.get("id") == Some(&json!(id)) && msg.get("result").is_some() {
                break;
            }
            // initialize 阶段可能也会收到通知/请求,统一走 handler,避免卡死。
            self.handle_server_message_for_lifecycle(&msg).await?;
        }

        let initialized = json!({ "method": "initialized" });
        self.write_json(&initialized).await?;
        self.initialized = true;
        Ok(())
    }

    async fn ensure_thread_started(&mut self, options: &CodexAppServerOptions) -> Result<()> {
        if self.thread_id.is_some() {
            return Ok(());
        }

        let id = self.next_request_id;
        self.next_request_id = self.next_request_id.saturating_add(1);

        let mut params = serde_json::Map::new();
        params.insert("ephemeral".to_string(), Value::Bool(true));

        if let Some(model) = &options.model {
            params.insert("model".to_string(), Value::String(model.clone()));
        }
        if let Some(cwd) = &options.cwd {
            params.insert("cwd".to_string(), Value::String(cwd.clone()));
        }
        if let Some(sandbox) = &options.sandbox {
            params.insert("sandbox".to_string(), Value::String(sandbox.clone()));
        }
        if let Some(policy) = &options.approval_policy {
            params.insert("approvalPolicy".to_string(), Value::String(policy.clone()));
        }

        let req = json!({
            "id": id,
            "method": "thread/start",
            "params": Value::Object(params),
        });
        self.write_json(&req).await?;

        // threadId 通过 thread/started 通知回传（更稳定）。
        loop {
            let Some(msg) = self.read_json_line().await? else {
                anyhow::bail!("codex app-server exited before thread started");
            };
            self.handle_server_message_for_lifecycle(&msg).await?;
            if self.thread_id.is_some() {
                break;
            }
        }

        Ok(())
    }

    async fn start_turn(
        &mut self,
        prompt: &str,
        options: &CodexAppServerOptions,
        workdir: Option<&Path>,
    ) -> Result<()> {
        let thread_id = self
            .thread_id
            .clone()
            .context("thread_id is missing (thread not started)")?;

        self.active_turn_id = None;

        let id = self.next_request_id;
        self.next_request_id = self.next_request_id.saturating_add(1);

        let mut params = serde_json::Map::new();
        params.insert("threadId".to_string(), Value::String(thread_id));
        params.insert(
            "input".to_string(),
            Value::Array(vec![json!({"type":"text","text": prompt})]),
        );

        // per-job workdir override（worktree 模式下必须随 job 变化）
        if let Some(workdir) = workdir {
            params.insert(
                "cwd".to_string(),
                Value::String(workdir.display().to_string()),
            );
        } else if let Some(cwd) = &options.cwd {
            // 如果 job 没有 workdir,则回退到 thread-level cwd.
            params.insert("cwd".to_string(), Value::String(cwd.clone()));
        }

        if let Some(model) = &options.model {
            params.insert("model".to_string(), Value::String(model.clone()));
        }
        if let Some(policy) = &options.approval_policy {
            params.insert("approvalPolicy".to_string(), Value::String(policy.clone()));
        }

        let req = json!({
            "id": id,
            "method": "turn/start",
            "params": Value::Object(params),
        });
        self.write_json(&req).await?;

        Ok(())
    }

    async fn steer_turn(&mut self, input: &str) -> Result<()> {
        let thread_id = self
            .thread_id
            .clone()
            .context("thread_id is missing (thread not started)")?;
        let turn_id = self
            .active_turn_id
            .clone()
            .context("active_turn_id is missing (no in-flight turn)")?;

        let id = self.next_request_id;
        self.next_request_id = self.next_request_id.saturating_add(1);

        let req = json!({
            "id": id,
            "method": "turn/steer",
            "params": {
                "threadId": thread_id,
                "expectedTurnId": turn_id,
                "input": [
                    {"type":"text","text": input}
                ],
            }
        });
        self.write_json(&req).await?;
        Ok(())
    }

    async fn interrupt_turn(&mut self) -> Result<()> {
        let Some(thread_id) = self.thread_id.clone() else {
            return Ok(());
        };
        let Some(turn_id) = self.active_turn_id.clone() else {
            return Ok(());
        };

        let id = self.next_request_id;
        self.next_request_id = self.next_request_id.saturating_add(1);

        let req = json!({
            "id": id,
            "method": "turn/interrupt",
            "params": {
                "threadId": thread_id,
                "turnId": turn_id,
            }
        });
        self.write_json(&req).await?;
        Ok(())
    }

    async fn handle_server_message_for_lifecycle(&mut self, msg: &Value) -> Result<()> {
        // notifications: {method, params}
        if let Some(method) = msg.get("method").and_then(Value::as_str) {
            match method {
                "thread/started" => {
                    if let Some(id) = msg
                        .pointer("/params/thread/id")
                        .and_then(Value::as_str)
                        .map(|s| s.to_string())
                    {
                        self.thread_id = Some(id);
                    }
                }
                "turn/started" => {
                    if let Some(id) = msg
                        .pointer("/params/turn/id")
                        .and_then(Value::as_str)
                        .map(|s| s.to_string())
                    {
                        self.active_turn_id = Some(id);
                    }
                }
                "error" => {
                    warn!(instance = %self.instance_id, msg = %msg, "codex app-server error notification");
                }
                _ => {}
            }
        }

        // server request: {id, method, params}
        if msg.get("id").is_some() && msg.get("method").is_some() && msg.get("params").is_some() {
            self.handle_server_request(msg).await?;
        }

        Ok(())
    }

    async fn handle_server_request(&mut self, msg: &Value) -> Result<()> {
        let Some(id_value) = msg.get("id").cloned() else {
            return Ok(());
        };
        let method = msg
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or_default();

        // ------------------------------------------------------------------
        // 说明:
        // - 并行 hat 是 headless: 我们默认自动批准,避免卡住。
        // - 对于我们不理解的 request,也要回复一个 result,避免 server 等待超时。
        // ------------------------------------------------------------------
        let result_payload = match method {
            // NEW APIs
            "item/commandExecution/requestApproval" => json!({ "decision": "accept" }),
            "item/fileChange/requestApproval" => json!({ "decision": "accept" }),

            // legacy APIs
            "execCommandApproval" => json!({ "decision": "approved" }),
            "applyPatchApproval" => json!({ "decision": "approved" }),

            // dynamic tools (best-effort: mark unsupported)
            "item/tool/call" => json!({
                "success": false,
                "contentItems": [
                    {"type":"inputText","text":"tool call is not supported in ralph headless app-server runtime"}
                ]
            }),
            "item/tool/requestUserInput" => json!({ "answers": {} }),

            _ => {
                warn!(instance = %self.instance_id, method = %method, "unknown codex app-server request; replying with empty result");
                json!({})
            }
        };

        let resp = json!({
            "id": id_value,
            "result": result_payload,
        });
        self.write_json(&resp).await?;
        Ok(())
    }
}

async fn flush_pending_output(
    job_id: u64,
    instance_id: &HatInstanceId,
    stream_pending: &mut String,
    output_tx: &mpsc::Sender<HatJobOutputChunk>,
) {
    while let Some(idx) = stream_pending.find('\n') {
        let line = stream_pending[..idx].to_string();
        let remain = stream_pending[idx + 1..].to_string();
        *stream_pending = remain;

        let _ = output_tx
            .send(HatJobOutputChunk {
                job_id,
                instance_id: instance_id.clone(),
                stream: OutputStream::Stdout,
                line,
            })
            .await;
    }
}

#[derive(Debug, Clone, Default)]
pub struct CodexAppServerRuntime {
    sessions: Arc<Mutex<HashMap<String, Arc<Mutex<CodexAppServerSession>>>>>,
}

impl CodexAppServerRuntime {
    pub async fn execute_job(
        &self,
        job: &HatJob,
        backend: &CliBackend,
        output_tx: mpsc::Sender<HatJobOutputChunk>,
        mut cancel_rx: watch::Receiver<bool>,
        mut control_rx: mpsc::Receiver<HatJobControl>,
    ) -> Result<HatJobResult> {
        let session = self.get_or_spawn_session(&job.instance_id).await?;
        let mut guard = session.lock().await;
        let mut stderr_rx = guard.subscribe_stderr();

        guard.ensure_initialized().await?;
        let options = parse_codex_app_server_options(backend, job.workdir.as_deref());
        guard.ensure_thread_started(&options).await?;

        guard
            .start_turn(&job.prompt, &options, job.workdir.as_deref())
            .await?;

        let mut output = String::new();
        let mut stream_pending = String::new();
        let mut canceled = false;
        let mut completed = false;
        let mut pending_steers: Vec<String> = Vec::new();

        loop {
            tokio::select! {
                stderr_line = stderr_rx.recv() => {
                    match stderr_line {
                        Ok(line) => {
                            // 说明:
                            // - stderr 只做可观测输出(灰色),不参与事件解析。
                            // - 因此这里仅发送 chunk,不拼进 `output`.
                            let _ = output_tx
                                .send(HatJobOutputChunk {
                                    job_id: job.job_id,
                                    instance_id: job.instance_id.clone(),
                                    stream: OutputStream::Stderr,
                                    line,
                                })
                                .await;
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => {
                            // 说明:
                            // - receiver 落后时会丢消息,这属于 best-effort 可观测输出的正常退化。
                            // - 我们选择忽略,避免阻塞主循环。
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            // sender 被关闭(通常表示 session 正在 shutdown).
                            // best-effort: 不影响 job 收敛,继续走 stdout 通道即可.
                        }
                    }
                }
                changed = cancel_rx.changed() => {
                    if changed.is_ok() && *cancel_rx.borrow() {
                        canceled = true;
                        // best-effort: interrupt turn,但不 kill thread.
                        if let Err(e) = guard.interrupt_turn().await {
                            warn!(instance = %job.instance_id, error = %e, "turn/interrupt failed");
                        }
                    }
                }
                control = control_rx.recv() => {
                    let Some(control) = control else { continue };
                    match control {
                        HatJobControl::Steer { input } => {
                            if guard.active_turn_id.is_some() {
                                if let Err(e) = guard.steer_turn(&input).await {
                                    warn!(instance = %job.instance_id, error = %e, "turn/steer failed");
                                }
                            } else {
                                // turn/started 还没到：先缓存,等 turnId 可用后再发。
                                pending_steers.push(input);
                            }
                        }
                    }
                }
                msg = guard.read_json_line() => {
                    let Some(msg) = msg? else {
                        anyhow::bail!("codex app-server exited while job is running");
                    };

                    // responses: ignore (best-effort)
                    if msg.get("id").is_some() && msg.get("result").is_some() {
                        continue;
                    }

                    // server request: auto-approve
                    if msg.get("id").is_some() && msg.get("method").is_some() && msg.get("params").is_some() {
                        guard.handle_server_request(&msg).await?;
                        continue;
                    }

                    // notifications
                    if let Some(method) = msg.get("method").and_then(Value::as_str) {
                        match method {
                            "turn/started" => {
                                guard.handle_server_message_for_lifecycle(&msg).await?;
                                // turnId 就绪后,把缓存 steer 逐条补发（best-effort）
                                if guard.active_turn_id.is_some() && !pending_steers.is_empty() {
                                    for steer in pending_steers.drain(..) {
                                        if let Err(e) = guard.steer_turn(&steer).await {
                                            warn!(instance = %job.instance_id, error = %e, "turn/steer (flush) failed");
                                        }
                                    }
                                }
                            }
                            "item/agentMessage/delta" => {
                                if let Some(delta) = msg.pointer("/params/delta").and_then(Value::as_str) {
                                    output.push_str(delta);
                                    stream_pending.push_str(delta);
                                    flush_pending_output(job.job_id, &job.instance_id, &mut stream_pending, &output_tx).await;
                                }
                            }
                            "turn/completed" => {
                                // turn/completed: 以当前 active_turn_id 为完成条件
                                let completed_id = msg.pointer("/params/turn/id").and_then(Value::as_str);
                                if completed_id.is_some() && guard.active_turn_id.as_deref() == completed_id {
                                    completed = true;
                                }
                            }
                            "error" => {
                                warn!(instance = %job.instance_id, msg = %msg, "codex app-server error notification");
                            }
                            _ => {
                                // 其他通知不影响 job 收敛
                            }
                        }
                    }

                    if completed {
                        break;
                    }
                }
            }
        }

        // flush tail (no trailing newline)
        if !stream_pending.trim().is_empty() {
            let _ = output_tx
                .send(HatJobOutputChunk {
                    job_id: job.job_id,
                    instance_id: job.instance_id.clone(),
                    stream: OutputStream::Stdout,
                    line: stream_pending.clone(),
                })
                .await;
        }

        Ok(HatJobResult {
            output,
            success: completed && !canceled,
            exit_code: Some(0),
            timed_out: false,
            canceled,
        })
    }

    pub async fn shutdown_all(&self) {
        let sessions = {
            let mut guard = self.sessions.lock().await;
            guard
                .drain()
                .map(|(_, session)| session)
                .collect::<Vec<_>>()
        };

        for session in sessions {
            let mut guard = session.lock().await;
            guard.shutdown().await;
        }
    }

    async fn get_or_spawn_session(
        &self,
        instance_id: &HatInstanceId,
    ) -> Result<Arc<Mutex<CodexAppServerSession>>> {
        let key = instance_id.to_string();
        if let Some(existing) = self.sessions.lock().await.get(&key).cloned() {
            return Ok(existing);
        }

        let session = CodexAppServerSession::spawn(&key)?;
        let session = Arc::new(Mutex::new(session));
        let mut guard = self.sessions.lock().await;
        let entry = guard.entry(key).or_insert_with(|| Arc::clone(&session));
        Ok(Arc::clone(entry))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_codex_app_server_options_maps_full_auto_and_model() {
        let backend = CliBackend {
            command: "codex".to_string(),
            args: vec![
                "exec".to_string(),
                "--full-auto".to_string(),
                "--model".to_string(),
                "gpt-5.2-codex".to_string(),
            ],
            prompt_mode: ralph_adapters::PromptMode::Arg,
            prompt_flag: None,
            output_format: ralph_adapters::OutputFormat::Text,
        };

        let opts = parse_codex_app_server_options(&backend, None);
        assert_eq!(opts.sandbox.as_deref(), Some("workspace-write"));
        assert_eq!(opts.approval_policy.as_deref(), Some("on-request"));
        assert_eq!(opts.model.as_deref(), Some("gpt-5.2-codex"));
    }
}
