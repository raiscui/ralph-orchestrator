//! Codex MCP 常驻会话运行时（parallel 模式专用）。
//!
//! 设计目标：
//! - 让 ralph 实例（ralph#1 / ralph#2）在同一进程内复用 `codex mcp-server`；
//! - 首次请求走 `codex` 工具，后续请求走 `codex-reply` 并复用 threadId；
//! - 在不改变并行调度语义的前提下，给 TUI 持续输出可观测文本。

use anyhow::{Context, Result};
use ralph_adapters::CliBackend;
use ralph_core::{HatJob, HatJobOutputChunk, HatJobResult, OutputStream};
use ralph_proto::HatInstanceId;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::{Mutex, mpsc, watch};
use tracing::{debug, warn};

#[derive(Debug, Clone, Default)]
struct CodexMcpToolOptions {
    model: Option<String>,
    profile: Option<String>,
    cwd: Option<String>,
    sandbox: Option<String>,
    approval_policy: Option<String>,
}

impl CodexMcpToolOptions {
    fn to_codex_arguments(&self, prompt: &str) -> Value {
        // ------------------------------------------------------------------
        // 说明:
        // - 这里映射的是 `codex` MCP 工具入参,不是 CLI `exec` 参数列表；
        // - 仅保留并行运行时真正需要的核心字段,其余未知参数不透传。
        // ------------------------------------------------------------------
        let mut args = serde_json::Map::new();
        args.insert("prompt".to_string(), Value::String(prompt.to_string()));

        if let Some(model) = &self.model {
            args.insert("model".to_string(), Value::String(model.clone()));
        }
        if let Some(profile) = &self.profile {
            args.insert("profile".to_string(), Value::String(profile.clone()));
        }
        if let Some(cwd) = &self.cwd {
            args.insert("cwd".to_string(), Value::String(cwd.clone()));
        }
        if let Some(sandbox) = &self.sandbox {
            args.insert("sandbox".to_string(), Value::String(sandbox.clone()));
        }
        if let Some(approval_policy) = &self.approval_policy {
            args.insert(
                "approval-policy".to_string(),
                Value::String(approval_policy.clone()),
            );
        }

        Value::Object(args)
    }
}

#[derive(Debug)]
struct CodexToolCallResult {
    thread_id: Option<String>,
    content: String,
}

#[derive(Debug)]
struct CodexMcpSession {
    instance_id: String,
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_request_id: u64,
    thread_id: Option<String>,
}

impl CodexMcpSession {
    async fn spawn(instance_id: &str) -> Result<Self> {
        let mut command = Command::new("codex");
        command.arg("mcp-server");
        command.stdin(Stdio::piped());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        command.kill_on_drop(true);

        let mut child = command
            .spawn()
            .with_context(|| format!("Failed to spawn codex mcp-server for {instance_id}"))?;

        let stdin = child
            .stdin
            .take()
            .context("Failed to take codex mcp-server stdin")?;
        let stdout = child
            .stdout
            .take()
            .context("Failed to take codex mcp-server stdout")?;

        if let Some(stderr) = child.stderr.take() {
            // ------------------------------------------------------------------
            // 说明:
            // - 后台持续消费 stderr,避免 pipe buffer 堵塞；
            // - 只做可观测日志,不参与事件解析。
            // ------------------------------------------------------------------
            let stderr_instance = instance_id.to_string();
            tokio::spawn(async move {
                let mut reader = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    warn!(instance = %stderr_instance, line = %line, "codex mcp-server stderr");
                }
            });
        }

        let mut session = Self {
            instance_id: instance_id.to_string(),
            child,
            stdin,
            stdout: BufReader::new(stdout),
            next_request_id: 1,
            thread_id: None,
        };
        session.initialize().await?;
        Ok(session)
    }

    async fn initialize(&mut self) -> Result<()> {
        let req_id = self.next_id();
        let init_req = json!({
            "jsonrpc": "2.0",
            "id": req_id,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {
                    "name": "ralph-cli",
                    "version": env!("CARGO_PKG_VERSION"),
                }
            }
        });
        self.send_json(&init_req).await?;

        let response = tokio::time::timeout(Duration::from_secs(15), async {
            loop {
                let value = self.read_json_line().await?;
                if jsonrpc_id_matches(&value, req_id) {
                    return Ok::<Value, anyhow::Error>(value);
                }
            }
        })
        .await
        .context("Timed out waiting for codex mcp initialize response")??;

        if let Some(err) = response.get("error") {
            anyhow::bail!("codex mcp initialize failed: {err}");
        }

        let initialized = json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        });
        self.send_json(&initialized).await?;
        debug!(instance = %self.instance_id, "codex mcp-server initialized");
        Ok(())
    }

    async fn execute_job(
        &mut self,
        job: &HatJob,
        tool_options: &CodexMcpToolOptions,
        output_tx: &mpsc::Sender<HatJobOutputChunk>,
        mut cancel_rx: watch::Receiver<bool>,
    ) -> Result<HatJobResult> {
        let prompt = job.prompt.clone();
        let use_reply = self.thread_id.is_some();

        // 说明:
        // - 同一实例内首轮 `codex`，后续 `codex-reply`；
        // - `codex-reply` 的 threadId 必须来自此前工具返回结果。
        let tool_call = if use_reply {
            let thread_id = self
                .thread_id
                .clone()
                .context("codex-reply requires existing thread_id")?;
            json!({
                "name": "codex-reply",
                "arguments": {
                    "threadId": thread_id,
                    "prompt": prompt,
                }
            })
        } else {
            // 首轮首包仍走 `codex`，参数从后端 args 映射而来。
            let args = tool_options.to_codex_arguments(&prompt);
            json!({
                "name": "codex",
                "arguments": args,
            })
        };

        let req_id = self.next_id();
        let request = json!({
            "jsonrpc": "2.0",
            "id": req_id,
            "method": "tools/call",
            "params": tool_call,
        });
        self.send_json(&request).await?;

        let mut timed_out = false;
        let mut canceled = false;
        let mut last_output_changed_at = Instant::now();
        let mut streamed_text = String::new();
        let mut stream_pending = String::new();

        match job.timeout.filter(|d| !d.is_zero()) {
            Some(check_interval) => {
                let mut next_check_deadline = tokio::time::Instant::now() + check_interval;
                let sleep = tokio::time::sleep_until(next_check_deadline);
                tokio::pin!(sleep);

                loop {
                    tokio::select! {
                        biased;
                        changed = cancel_rx.changed() => {
                            if changed.is_ok() && *cancel_rx.borrow() {
                                canceled = true;
                                break;
                            }
                        }
                        _ = &mut sleep => {
                            match job.output_stale_timeout {
                                Some(stale_timeout) => {
                                    if last_output_changed_at.elapsed() >= stale_timeout {
                                        timed_out = true;
                                        break;
                                    }
                                    next_check_deadline = tokio::time::Instant::now() + check_interval;
                                    sleep.as_mut().reset(next_check_deadline);
                                }
                                None => {
                                    timed_out = true;
                                    break;
                                }
                            }
                        }
                        line = self.read_json_line() => {
                            let value = line?;
                            if let Some(call_result) = self.handle_incoming(
                                job,
                                req_id,
                                value,
                                output_tx,
                                &mut last_output_changed_at,
                                &mut stream_pending,
                                &mut streamed_text,
                            ).await? {
                                self.flush_pending_line(job, output_tx, &mut stream_pending, &mut streamed_text).await;
                                return Ok(self.to_job_result(call_result, streamed_text, false, false));
                            }
                        }
                    }
                }
            }
            None => loop {
                tokio::select! {
                    biased;
                    changed = cancel_rx.changed() => {
                        if changed.is_ok() && *cancel_rx.borrow() {
                            canceled = true;
                            break;
                        }
                    }
                    line = self.read_json_line() => {
                        let value = line?;
                        if let Some(call_result) = self.handle_incoming(
                            job,
                            req_id,
                            value,
                            output_tx,
                            &mut last_output_changed_at,
                            &mut stream_pending,
                            &mut streamed_text,
                        ).await? {
                            self.flush_pending_line(job, output_tx, &mut stream_pending, &mut streamed_text).await;
                            return Ok(self.to_job_result(call_result, streamed_text, false, false));
                        }
                    }
                }
            },
        }

        // 超时或取消后,保持会话存活(不主动终止 mcp-server),仅让当前 job 失败返回。
        self.flush_pending_line(job, output_tx, &mut stream_pending, &mut streamed_text)
            .await;
        Ok(HatJobResult {
            output: String::new(),
            success: false,
            exit_code: None,
            timed_out,
            canceled,
        })
    }

    async fn handle_incoming(
        &mut self,
        job: &HatJob,
        req_id: u64,
        value: Value,
        output_tx: &mpsc::Sender<HatJobOutputChunk>,
        last_output_changed_at: &mut Instant,
        stream_pending: &mut String,
        streamed_text: &mut String,
    ) -> Result<Option<CodexToolCallResult>> {
        if let Some(method) = value.get("method").and_then(Value::as_str) {
            if method == "codex/event" {
                self.handle_codex_event(
                    job,
                    &value,
                    output_tx,
                    last_output_changed_at,
                    stream_pending,
                    streamed_text,
                )
                .await?;
            }
            return Ok(None);
        }

        if !jsonrpc_id_matches(&value, req_id) {
            return Ok(None);
        }

        if let Some(err) = value.get("error") {
            anyhow::bail!("codex mcp tools/call failed: {err}");
        }

        let result = value
            .get("result")
            .cloned()
            .context("codex mcp response missing result field")?;
        let parsed = parse_tool_call_result(result)?;

        if let Some(thread_id) = parsed.thread_id.clone() {
            self.thread_id = Some(thread_id);
        }

        Ok(Some(parsed))
    }

    async fn handle_codex_event(
        &self,
        job: &HatJob,
        value: &Value,
        output_tx: &mpsc::Sender<HatJobOutputChunk>,
        last_output_changed_at: &mut Instant,
        stream_pending: &mut String,
        streamed_text: &mut String,
    ) -> Result<()> {
        let msg = value.pointer("/params/msg").cloned().unwrap_or(Value::Null);
        let msg_type = msg.get("type").and_then(Value::as_str).unwrap_or_default();

        match msg_type {
            "agent_message_delta" => {
                if let Some(delta) = msg.get("delta").and_then(Value::as_str) {
                    *last_output_changed_at = Instant::now();
                    streamed_text.push_str(delta);
                    emit_stream_deltas(
                        job.job_id,
                        &job.instance_id,
                        delta,
                        stream_pending,
                        output_tx,
                    )
                    .await;
                }
            }
            "agent_message" => {
                if let Some(message) = msg.get("message").and_then(Value::as_str) {
                    *last_output_changed_at = Instant::now();
                    if streamed_text.is_empty() {
                        streamed_text.push_str(message);
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    async fn flush_pending_line(
        &self,
        job: &HatJob,
        output_tx: &mpsc::Sender<HatJobOutputChunk>,
        stream_pending: &mut String,
        streamed_text: &mut String,
    ) {
        if stream_pending.is_empty() {
            return;
        }

        let pending = std::mem::take(stream_pending);
        let _ = output_tx
            .send(HatJobOutputChunk {
                job_id: job.job_id,
                instance_id: job.instance_id.clone(),
                stream: OutputStream::Stdout,
                line: pending.clone(),
            })
            .await;
        if !streamed_text.ends_with('\n') {
            streamed_text.push('\n');
        }
    }

    fn to_job_result(
        &self,
        call_result: CodexToolCallResult,
        streamed_text: String,
        timed_out: bool,
        canceled: bool,
    ) -> HatJobResult {
        // ------------------------------------------------------------------
        // 说明:
        // - 事件解析必须基于“最终完整 assistant 内容”；
        // - streamed_text 仅用于 TUI 实时体验,不作为权威解析输入。
        // ------------------------------------------------------------------
        let mut output = call_result.content;
        if !output.ends_with('\n') {
            output.push('\n');
        }

        let _ = streamed_text; // 保留变量用于后续需要时做诊断比对

        HatJobResult {
            output,
            success: !timed_out && !canceled,
            exit_code: None,
            timed_out,
            canceled,
        }
    }

    async fn send_json(&mut self, value: &Value) -> Result<()> {
        let payload =
            serde_json::to_string(value).context("Failed to serialize JSON-RPC payload")?;
        self.stdin
            .write_all(payload.as_bytes())
            .await
            .context("Failed to write JSON-RPC payload")?;
        self.stdin
            .write_all(b"\n")
            .await
            .context("Failed to write JSON-RPC newline")?;
        self.stdin
            .flush()
            .await
            .context("Failed to flush JSON-RPC payload")?;
        Ok(())
    }

    async fn read_json_line(&mut self) -> Result<Value> {
        loop {
            let mut line = String::new();
            let read = self
                .stdout
                .read_line(&mut line)
                .await
                .context("Failed to read codex mcp stdout line")?;

            if read == 0 {
                anyhow::bail!("codex mcp-server stdout closed unexpectedly");
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            match serde_json::from_str::<Value>(trimmed) {
                Ok(value) => return Ok(value),
                Err(e) => {
                    warn!(
                        instance = %self.instance_id,
                        line = %trimmed,
                        error = %e,
                        "Skipping non-JSON line from codex mcp-server"
                    );
                }
            }
        }
    }

    fn next_id(&mut self) -> u64 {
        let id = self.next_request_id;
        self.next_request_id = self.next_request_id.saturating_add(1);
        id
    }

    async fn shutdown(&mut self) {
        let _ = self.child.start_kill();
        let _ = self.child.wait().await;
    }
}

fn parse_tool_call_result(result: Value) -> Result<CodexToolCallResult> {
    // 优先使用 structuredContent（最稳定）
    let thread_id = result
        .pointer("/structuredContent/threadId")
        .and_then(Value::as_str)
        .map(ToString::to_string);

    let mut content = result
        .pointer("/structuredContent/content")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    if content.is_empty() {
        // 兜底: 从 content[] 里拼接 text 字段
        if let Some(items) = result.get("content").and_then(Value::as_array) {
            let mut parts = Vec::new();
            for item in items {
                if let Some(text) = item.get("text").and_then(Value::as_str) {
                    parts.push(text.to_string());
                }
            }
            content = parts.join("\n");
        }
    }

    if content.is_empty() {
        anyhow::bail!("codex mcp tools/call response missing content");
    }

    Ok(CodexToolCallResult { thread_id, content })
}

fn jsonrpc_id_matches(value: &Value, req_id: u64) -> bool {
    match value.get("id") {
        Some(Value::Number(n)) => n.as_u64() == Some(req_id),
        Some(Value::String(s)) => s == req_id.to_string().as_str(),
        _ => false,
    }
}

async fn emit_stream_deltas(
    job_id: u64,
    instance_id: &HatInstanceId,
    delta: &str,
    stream_pending: &mut String,
    output_tx: &mpsc::Sender<HatJobOutputChunk>,
) {
    stream_pending.push_str(delta);

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

fn parse_codex_tool_options(
    backend: &CliBackend,
    workdir: Option<&std::path::Path>,
) -> CodexMcpToolOptions {
    let mut options = CodexMcpToolOptions::default();
    let mut iter = backend.args.iter();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            // `codex exec` 专用子命令在 mcp 调用里不需要。
            "exec" => {}
            "--full-auto" => {
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

    if options.cwd.is_none()
        && let Some(workdir) = workdir
    {
        options.cwd = Some(workdir.display().to_string());
    }

    options
}

fn build_ralph_handoff_prompt(summary: &str, prompt: &str) -> String {
    format!("## Ralph Handoff Summary\n{summary}\n\n## Current Task\n{prompt}")
}

fn truncate_summary(content: &str) -> String {
    const MAX_CHARS: usize = 4000;
    let trimmed = content.trim();
    if trimmed.chars().count() <= MAX_CHARS {
        return trimmed.to_string();
    }

    let tail: String = trimmed
        .chars()
        .rev()
        .take(MAX_CHARS)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    format!("...(truncated)\n{tail}")
}

#[derive(Debug, Clone, Default)]
pub struct CodexMcpRuntime {
    sessions: Arc<Mutex<HashMap<String, Arc<Mutex<CodexMcpSession>>>>>,
    latest_ralph_primary_summary: Arc<Mutex<Option<String>>>,
}

impl CodexMcpRuntime {
    pub async fn execute_job(
        &self,
        job: &HatJob,
        backend: &CliBackend,
        output_tx: mpsc::Sender<HatJobOutputChunk>,
        cancel_rx: watch::Receiver<bool>,
    ) -> Result<HatJobResult> {
        let instance_key = job.instance_id.to_string();
        let session = self.get_or_spawn_session(&job.instance_id).await?;
        let mut guard = session.lock().await;

        let mut prompt = job.prompt.clone();
        if instance_key == "ralph#2" && guard.thread_id.is_none() {
            // ------------------------------------------------------------------
            // 说明:
            // - ralph#2 首次接管时,注入最近的 ralph#1 摘要,降低冷启动成本；
            // - 仍然保持独立 thread,不会和 ralph#1 共用会话状态。
            // ------------------------------------------------------------------
            if let Some(summary) = self.latest_ralph_primary_summary.lock().await.clone() {
                prompt = build_ralph_handoff_prompt(&summary, &prompt);
            }
        }

        let patched_job = HatJob {
            prompt,
            ..job.clone()
        };

        let options = parse_codex_tool_options(backend, patched_job.workdir.as_deref());
        let result = guard
            .execute_job(&patched_job, &options, &output_tx, cancel_rx)
            .await?;

        if instance_key == "ralph#1" && result.success {
            let summary = truncate_summary(&result.output);
            *self.latest_ralph_primary_summary.lock().await = Some(summary);
        }

        Ok(result)
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
    ) -> Result<Arc<Mutex<CodexMcpSession>>> {
        let key = instance_id.to_string();
        if let Some(existing) = self.sessions.lock().await.get(&key).cloned() {
            return Ok(existing);
        }

        let session = CodexMcpSession::spawn(&key).await?;
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
    fn parse_codex_tool_options_maps_full_auto_and_model() {
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

        let opts = parse_codex_tool_options(&backend, None);
        assert_eq!(opts.sandbox.as_deref(), Some("workspace-write"));
        assert_eq!(opts.approval_policy.as_deref(), Some("on-request"));
        assert_eq!(opts.model.as_deref(), Some("gpt-5.2-codex"));
    }

    #[test]
    fn build_ralph_handoff_prompt_includes_summary_and_task() {
        let got = build_ralph_handoff_prompt("summary", "task");
        assert!(got.contains("summary"));
        assert!(got.contains("task"));
    }
}
