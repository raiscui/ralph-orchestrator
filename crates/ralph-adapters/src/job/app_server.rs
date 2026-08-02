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

use ralph_display::colors;
use anyhow::{Context, Result};
use crate::{
    CliBackend, scrub_codex_parent_session_env_tokio, scrub_ralph_parent_worker_env_tokio,
};
use ralph_core::{
    HatJob, HatJobControl, HatJobOutputChunk, HatJobResult, OutputStream, clean_activity_label,
    normalize_activity_label,
};
use ralph_proto::HatInstanceId;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::{Mutex, broadcast, mpsc, watch};
use tracing::{debug, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CodexAppServerOutputSource {
    /// 传统通道: 直接的 agent message 输出(delta).
    ///
    /// 说明:
    /// - 这是 fake app-server 与部分真实版本会使用的输出通道。
    /// - 该通道通常承载“用户可见输出”(包括 completion_promise)。
    AgentMessageDelta,
    /// 新通道: reasoning summary 的 text delta.
    ///
    /// 说明:
    /// - 在部分真实 codex app-server 版本中,会持续推送 `item/reasoning/summaryTextDelta`,
    ///   作为 UI 上“可见的摘要输出”。
    /// - 我们在没有看到 `item/agentMessage/delta` 时使用它作为 fallback,以避免 job 永远没有 output。
    ReasoningSummaryTextDelta,
}

fn env_flag_is_true(name: &str) -> bool {
    // ------------------------------------------------------------------
    // 说明:
    // - 这是一个“调试开关”解析器,用于 E2E/排障.
    // - 约定: 1/true/yes/on 视为 true；其他值或缺失视为 false。
    // ------------------------------------------------------------------
    match std::env::var(name) {
        Ok(raw) => {
            let v = raw.trim().to_ascii_lowercase();
            matches!(v.as_str(), "1" | "true" | "yes" | "on")
        }
        Err(_) => false,
    }
}

#[derive(Debug, Clone, Default)]
struct CodexAppServerOptions {
    model: Option<String>,
    profile: Option<String>,
    cwd: Option<String>,
    sandbox: Option<String>,
    approval_policy: Option<String>,
    config_overrides: Vec<String>,
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
            "--config" | "-c" => {
                if let Some(config) = iter.next() {
                    options.config_overrides.push(config.clone());
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

fn build_codex_app_server_process_args(options: &CodexAppServerOptions) -> Vec<String> {
    let mut args = vec![
        "app-server".to_string(),
        "--listen".to_string(),
        "stdio://".to_string(),
    ];

    // 注意: `codex app-server`(截至 0.146)只支持 `--listen` 与 `--config`,
    // 不接受 `--profile`。直接透传会导致 app-server 启动即失败:
    //   error: unexpected argument '--profile' found
    // profile 的等价语义(模型/provider 等)应通过 `--config key=value` 表达。
    // 若未来 codex app-server 支持 --profile,可在此恢复透传。
    if let Some(profile) = &options.profile {
        tracing::warn!(
            profile = %profile,
            "codex app-server does not support --profile (codex <= 0.146); profile ignored, use --config overrides instead"
        );
    }

    // 让 app-server 进程本身也继承 `codex exec -c ...` 的配置覆写。
    // 否则 turn/thread 请求里即使改了 model,仍可能继续吃到全局 config 的旧推理参数。
    for config in &options.config_overrides {
        args.push("--config".to_string());
        args.push(config.clone());
    }

    args
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
    /// 本端发出的 request id -> method 映射(用于把 response 归因到具体 method,提升可读性).
    pending_request_methods: HashMap<u64, String>,
    initialized: bool,
    thread_id: Option<String>,
    /// 当前 thread 是否已经收到过“首 turn 的完整 prompt”。
    ///
    /// 说明:
    /// - 同一个 thread 的后续 turn 可以只发送增量输入。
    /// - 一旦 session/thread 重建,该标记会自然回到 false。
    seeded_first_turn_prompt: bool,
    active_turn_id: Option<String>,
    /// RPC trace 开关(可选): 把 send/recv 事件写入 stderr_tx,便于人类审计 turn/steer 是否真的发生.
    trace_rpc: bool,
    /// trace 时是否包含 turn/steer 的 input 文本(默认关闭,避免意外泄露敏感信息).
    trace_steer_input: bool,
    /// trace 时是否附带原始 JSON 预览(默认关闭,仅用于深度排障/协议对齐).
    trace_json_preview: bool,
}

impl CodexAppServerSession {
    fn spawn(
        instance_id: &str,
        codex_command: &str,
        options: &CodexAppServerOptions,
    ) -> Result<Self> {
        let mut command = Command::new(codex_command);
        command.args(build_codex_app_server_process_args(options));
        command.stdin(Stdio::piped());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        command.kill_on_drop(true);
        scrub_ralph_parent_worker_env_tokio(&mut command);
        scrub_codex_parent_session_env_tokio(&mut command, codex_command);

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
        // ------------------------------------------------------------------
        // 说明:
        // - stderr_rx 在 job 内会被持续消费,但在高频 notify(delta) 场景下仍可能出现短暂堆积。
        // - broadcast 默认容量过小会导致 trace 行被丢弃,影响人类审计(e2e/human-log)。
        // - 这里适当加大缓冲,以换取更稳定的“可读证据”。(默认不启用 trace 时几乎无影响)
        // ------------------------------------------------------------------
        let (stderr_tx, _stderr_rx) = broadcast::channel::<String>(4096);

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
            pending_request_methods: HashMap::new(),
            initialized: false,
            thread_id: None,
            seeded_first_turn_prompt: false,
            active_turn_id: None,
            trace_rpc: env_flag_is_true("RALPH_CODEX_APP_SERVER_TRACE"),
            trace_steer_input: env_flag_is_true("RALPH_CODEX_APP_SERVER_TRACE_STEER_INPUT"),
            trace_json_preview: env_flag_is_true("RALPH_CODEX_APP_SERVER_TRACE_JSON"),
        })
    }

    fn subscribe_stderr(&self) -> broadcast::Receiver<String> {
        self.stderr_tx.subscribe()
    }

    fn trace_line(&self, line: impl Into<String>) {
        if !self.trace_rpc {
            return;
        }

        // best-effort: 无订阅者会 Err,忽略即可.
        let _ = self.stderr_tx.send(line.into());
    }

    fn json_preview(value: &Value, max_chars: usize) -> String {
        // ------------------------------------------------------------------
        // 说明:
        // - 仅用于 trace 输出,因此这里允许分配与截断.
        // - 统一输出为单行,避免破坏行级日志的可读性.
        // ------------------------------------------------------------------
        let raw = serde_json::to_string(value).unwrap_or_else(|_| "<invalid-json>".to_string());
        let preview: String = raw.chars().take(max_chars).collect();
        if raw.chars().count() > max_chars {
            format!("{preview}...(truncated)")
        } else {
            preview
        }
    }

    fn record_pending_request_method(&mut self, value: &Value) {
        // ------------------------------------------------------------------
        // 说明:
        // - 真实 codex app-server 的 response 只有 `{id, result}` / `{id, error}`，
        //   不包含 `method` 字段。
        // - 因此我们需要在发送 request 时记录 `id -> method`，
        //   以便在收到 error response 时能给出可读的错误信息,
        //   并决定是否需要 fail-fast / restart session。
        // - 这条映射不依赖 trace 开关: 即使不 trace，也必须能定位错误方法。
        // ------------------------------------------------------------------
        let Some(method) = value.get("method").and_then(Value::as_str) else {
            return;
        };
        let Some(id) = value.get("id").and_then(Value::as_u64) else {
            return;
        };

        self.pending_request_methods.insert(id, method.to_string());
    }

    fn trace_send(&mut self, value: &Value) {
        if !self.trace_rpc {
            return;
        }

        let method = value.get("method").and_then(Value::as_str);
        let id = value.get("id").and_then(Value::as_u64);

        // 只记录结构化摘要,避免把 prompt/敏感信息刷到日志里.
        if let Some(method) = method {
            // notifications: {method, ...} (no id)
            if id.is_none() {
                self.trace_line(format!("[app-server-rpc] send notify method={method}"));
                return;
            }

            if method == "turn/steer" {
                let input_text = value
                    .pointer("/params/input/0/text")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let input_len = input_text.len();

                if self.trace_steer_input {
                    // 只截断 steer input(最多 160 字符),用于 E2E marker 排障.
                    let preview: String = input_text.chars().take(160).collect();
                    self.trace_line(format!(
                        "[app-server-rpc] send request id={id:?} method={method} input_len={input_len} input_preview={preview:?}"
                    ));
                } else {
                    self.trace_line(format!(
                        "[app-server-rpc] send request id={id:?} method={method} input_len={input_len}"
                    ));
                }
                return;
            }

            if method == "turn/start" {
                let input_text = value
                    .pointer("/params/input/0/text")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let input_len = input_text.len();

                if self.trace_steer_input {
                    // 说明:
                    // - 你在排障时常需要看到“注入的完整 prompt”是否正确(包含 ralph prompt / sys/user 信息)。
                    // - 这里仅输出一个截断预览,并且必须显式开启 trace_steer_input 才会打印,避免默认泄露。
                    let preview: String = input_text.chars().take(600).collect();
                    self.trace_line(format!(
                        "[app-server-rpc] send request id={id:?} method={method} input_len={input_len} input_preview={preview:?}"
                    ));
                } else {
                    self.trace_line(format!(
                        "[app-server-rpc] send request id={id:?} method={method} input_len={input_len}"
                    ));
                }
                return;
            }

            self.trace_line(format!(
                "[app-server-rpc] send request id={id:?} method={method}"
            ));
            return;
        }

        // responses: {id, result}
        if id.is_some() && value.get("result").is_some() {
            self.trace_line(format!("[app-server-rpc] send response id={id:?}"));
        }
    }

    fn trace_recv(&mut self, value: &Value) {
        if !self.trace_rpc {
            return;
        }

        let method = value.get("method").and_then(Value::as_str);
        let id = value.get("id").and_then(Value::as_u64);

        if let Some(method) = method {
            if let Some(id) = id {
                self.trace_line(format!(
                    "[app-server-rpc] recv request id={id} method={method}"
                ));
            } else {
                // notifications
                match method {
                    "thread/started" => {
                        let tid = value
                            .pointer("/params/thread/id")
                            .and_then(Value::as_str)
                            .unwrap_or("<missing>");
                        self.trace_line(format!(
                            "[app-server-rpc] recv notify method=thread/started thread_id={tid}"
                        ));
                    }
                    "turn/started" => {
                        let turn_id = value
                            .pointer("/params/turn/id")
                            .and_then(Value::as_str)
                            .unwrap_or("<missing>");
                        self.trace_line(format!(
                            "[app-server-rpc] recv notify method=turn/started turn_id={turn_id}"
                        ));
                    }
                    "turn/completed" => {
                        let turn_id = value
                            .pointer("/params/turn/id")
                            .and_then(Value::as_str)
                            .unwrap_or("<missing>");
                        self.trace_line(format!(
                            "[app-server-rpc] recv notify method=turn/completed turn_id={turn_id}"
                        ));
                    }
                    _ => {
                        self.trace_line(format!("[app-server-rpc] recv notify method={method}"));
                    }
                }
            }

            // 深度排障: 对部分方法附带 JSON 预览(默认关闭).
            if self.trace_json_preview {
                // 只对“我们关心可能承载文本输出/完成信号”的通知打印,避免刷屏.
                let should_preview = method.starts_with("codex/event/")
                    || matches!(
                        method,
                        "item/started"
                            | "item/completed"
                            | "thread/status/changed"
                            | "item/reasoning/summaryTextDelta"
                            | "item/reasoning/summaryPartAdded"
                    );
                if should_preview {
                    let preview = Self::json_preview(value, 600);
                    self.trace_line(format!(
                        "[app-server-rpc] recv notify json_preview={preview}"
                    ));
                }
            }
            return;
        }

        // responses: {id, result} / {id, error}
        if let Some(id) = id {
            if value.get("result").is_some() {
                let method = self
                    .pending_request_methods
                    .get(&id)
                    .cloned()
                    .unwrap_or_else(|| "<unknown>".to_string());
                self.trace_line(format!(
                    "[app-server-rpc] recv response id={id} method={method}"
                ));
            } else if value.get("error").is_some() {
                let method = self
                    .pending_request_methods
                    .get(&id)
                    .cloned()
                    .unwrap_or_else(|| "<unknown>".to_string());
                let code = value.pointer("/error/code").and_then(Value::as_i64);
                let message = value
                    .pointer("/error/message")
                    .and_then(Value::as_str)
                    .unwrap_or("<missing>");

                // 错误消息可能很长,这里做截断,避免污染可读日志.
                let preview: String = message.chars().take(160).collect();
                self.trace_line(format!(
                    "[app-server-rpc] recv response id={id} method={method} error_code={code:?} error_message={preview:?}"
                ));
            }
        }
    }

    async fn shutdown(&mut self) {
        let _ = self.child.start_kill();
        let _ = self.child.wait().await;
    }

    async fn write_json(&mut self, value: &Value) -> Result<()> {
        self.record_pending_request_method(value);
        self.trace_send(value);
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
        self.trace_recv(&value);
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

            if msg.get("id").and_then(Value::as_u64) == Some(id) {
                // initialize response: {id, result} / {id, error}
                let _ = self.pending_request_methods.remove(&id);

                if msg.get("result").is_some() {
                    break;
                }
                if msg.get("error").is_some() {
                    let code = msg.pointer("/error/code").and_then(Value::as_i64);
                    let message = msg
                        .pointer("/error/message")
                        .and_then(Value::as_str)
                        .unwrap_or("<missing>");
                    anyhow::bail!(
                        "codex app-server initialize failed: error_code={code:?} error_message={message}"
                    );
                }
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

            if msg.get("id").and_then(Value::as_u64) == Some(id) {
                // thread/start response: {id, result} / {id, error}
                let _ = self.pending_request_methods.remove(&id);

                if msg.get("error").is_some() {
                    let code = msg.pointer("/error/code").and_then(Value::as_i64);
                    let message = msg
                        .pointer("/error/message")
                        .and_then(Value::as_str)
                        .unwrap_or("<missing>");
                    anyhow::bail!(
                        "codex app-server thread/start failed: error_code={code:?} error_message={message}"
                    );
                }

                // 兼容: 某些实现可能只在 response.result 里返回 thread id，不发 thread/started。
                if self.thread_id.is_none() {
                    let tid = msg
                        .pointer("/result/thread/id")
                        .and_then(Value::as_str)
                        .or_else(|| msg.pointer("/result/threadId").and_then(Value::as_str))
                        .or_else(|| msg.pointer("/result/thread_id").and_then(Value::as_str));
                    if let Some(tid) = tid {
                        self.thread_id = Some(tid.to_string());
                    }
                }
            }

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

    fn turn_input_for_job<'a>(&self, job: &'a HatJob) -> (&'a str, &'static str) {
        if self.seeded_first_turn_prompt
            && let Some(continuation_prompt) = job.continuation_prompt.as_deref()
        {
            return (continuation_prompt, "turn/start continuation input");
        }

        (&job.prompt, "turn/start input")
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
    stream: OutputStream,
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
                stream,
                line,
            })
            .await;
    }
}

fn build_prompt_transcript_lines(
    label: &str,
    instance_id: &HatInstanceId,
    job_id: u64,
    prompt: &str,
    use_colors: bool,
) -> Vec<String> {
    // ------------------------------------------------------------------
    // 说明:
    // - 真实 codex app-server 自身不会像 `codex exec` 那样默认把“注入的 messages/prompt”
    //   打到 stderr,这会让排障非常困难。
    // - 这里我们在 ralph 侧主动回显 turn/start 的 prompt,并尽量保持 `codex exec` 的观感:
    //   - 输出到 stderr 流
    //   - 可选 ANSI 色彩(受 `--color` 控制)
    // - 输出为多行 transcript,避免 JSON escape 破坏可读性。
    // ------------------------------------------------------------------
    let prompt_len = prompt.chars().count();

    let mut out = Vec::new();

    if use_colors {
        out.push(format!(
            "{b}{c}[codex-app-server] {label} (instance={instance_id} job={job_id} chars={prompt_len}){r}",
            b = colors::BOLD,
            c = colors::CYAN,
            r = colors::RESET,
        ));
        out.push(format!(
            "{d}{g}----- BEGIN PROMPT -----{r}",
            d = colors::DIM,
            g = colors::GRAY,
            r = colors::RESET,
        ));
    } else {
        out.push(format!(
            "[codex-app-server] {label} (instance={instance_id} job={job_id} chars={prompt_len})"
        ));
        out.push("----- BEGIN PROMPT -----".to_string());
    }

    // 使用 split 而不是 lines(): 保留尾部空行(如果 prompt 以 \\n 结尾).
    for line in prompt.split('\n') {
        out.push(line.to_string());
    }

    if use_colors {
        out.push(format!(
            "{d}{g}------ END PROMPT ------{r}",
            d = colors::DIM,
            g = colors::GRAY,
            r = colors::RESET,
        ));
    } else {
        out.push("------ END PROMPT ------".to_string());
    }

    out
}

async fn emit_prompt_transcript(
    output_tx: &mpsc::Sender<HatJobOutputChunk>,
    job_id: u64,
    instance_id: &HatInstanceId,
    label: &str,
    prompt: &str,
    use_colors: bool,
) {
    let lines = build_prompt_transcript_lines(label, instance_id, job_id, prompt, use_colors);
    for line in lines {
        let _ = output_tx
            .send(HatJobOutputChunk {
                job_id,
                instance_id: instance_id.clone(),
                stream: OutputStream::Stderr,
                line,
            })
            .await;
    }
}

async fn emit_activity(
    output_tx: &mpsc::Sender<HatJobOutputChunk>,
    job_id: u64,
    instance_id: &HatInstanceId,
    label: &str,
) {
    let label = clean_activity_label(label);
    if label.is_empty() {
        return;
    }

    let _ = output_tx
        .send(HatJobOutputChunk {
            job_id,
            instance_id: instance_id.clone(),
            stream: OutputStream::Activity,
            line: label,
        })
        .await;
}

async fn emit_activity_if_changed(
    output_tx: &mpsc::Sender<HatJobOutputChunk>,
    job_id: u64,
    instance_id: &HatInstanceId,
    current_activity_label: &mut Option<String>,
    label: &str,
) {
    let label = clean_activity_label(label);
    if label.is_empty() || current_activity_label.as_deref() == Some(label.as_str()) {
        return;
    }

    *current_activity_label = Some(label.clone());
    emit_activity(output_tx, job_id, instance_id, &label).await;
}

async fn emit_activity_from_text_if_changed(
    output_tx: &mpsc::Sender<HatJobOutputChunk>,
    job_id: u64,
    instance_id: &HatInstanceId,
    current_activity_label: &mut Option<String>,
    text: &str,
) {
    if let Some(label) = normalize_activity_label(text) {
        emit_activity_if_changed(
            output_tx,
            job_id,
            instance_id,
            current_activity_label,
            &label,
        )
        .await;
    }
}

#[derive(Debug, Clone)]
pub struct CodexAppServerRuntime {
    sessions: Arc<Mutex<HashMap<String, Arc<Mutex<CodexAppServerSession>>>>>,
    use_colors: bool,
    codex_command: String,
}

impl Default for CodexAppServerRuntime {
    fn default() -> Self {
        Self::new(true)
    }
}

impl CodexAppServerRuntime {
    pub fn new(use_colors: bool) -> Self {
        Self::new_with_command(use_colors, "codex")
    }

    pub fn new_with_command(use_colors: bool, codex_command: impl Into<String>) -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            use_colors,
            codex_command: codex_command.into(),
        }
    }

    pub async fn execute_job(
        &self,
        job: &HatJob,
        backend: &CliBackend,
        output_tx: mpsc::Sender<HatJobOutputChunk>,
        mut cancel_rx: watch::Receiver<bool>,
        mut control_rx: mpsc::Receiver<HatJobControl>,
    ) -> Result<HatJobResult> {
        let timed_out_restart = {
            let options = parse_codex_app_server_options(backend, job.workdir.as_deref());
            let session = self
                .get_or_spawn_session(&job.instance_id, &options)
                .await?;
            let mut guard = session.lock().await;
            let mut stderr_rx = guard.subscribe_stderr();

            guard.ensure_initialized().await?;
            guard.ensure_thread_started(&options).await?;

            let (turn_input, transcript_label) = guard.turn_input_for_job(job);

            // 默认回显 turn/start 的实际输入,对齐 `codex exec` 的 stderr 可观测性。
            // 重要: 这不依赖 trace env 开关,因为你希望默认就能看到“这次 turn 真正注入了什么”.
            emit_prompt_transcript(
                &output_tx,
                job.job_id,
                &job.instance_id,
                transcript_label,
                turn_input,
                self.use_colors,
            )
            .await;

            guard
                .start_turn(turn_input, &options, job.workdir.as_deref())
                .await?;
            guard.seeded_first_turn_prompt = true;

            let mut output = String::new();
            let mut stream_pending = String::new();
            let mut canceled = false;
            let mut timed_out = false;
            let mut completed = false;
            let mut restart_session = false;
            let mut pending_steers: Vec<String> = Vec::new();
            let mut task_started = false;
            let mut turn_started_at: Option<Instant> = None;
            let mut output_source: Option<CodexAppServerOutputSource> = None;
            let mut last_summary_index: Option<u64> = None;
            let mut thinking_pending = String::new();
            let mut thinking_last_summary_index: Option<u64> = None;
            let mut current_activity_label: Option<String> = None;
            let mut last_output_changed_at = Instant::now();

            // 当输出/stderr 通道关闭时,recv 会立刻返回,可能导致 busy loop.
            // 这里在关闭后禁用对应分支,避免空转占用 CPU。
            let mut stderr_closed = false;
            let mut control_closed = false;
            let mut cancel_closed = false;

            let check_interval = job.timeout.filter(|d| !d.is_zero());
            let mut next_check_deadline = check_interval.map(|d| tokio::time::Instant::now() + d);
            let sleep = tokio::time::sleep_until(
                next_check_deadline
                    .unwrap_or_else(|| tokio::time::Instant::now() + Duration::from_hours(8760)),
            );
            tokio::pin!(sleep);

            loop {
                tokio::select! {
                    // timeout/stale watchdog：与其他 backend 的 HatJob.timeout/output_stale_timeout 语义对齐。
                    _ = &mut sleep, if check_interval.is_some() => {
                        let Some(check_interval) = check_interval else { unreachable!("sleep branch guarded by check_interval.is_some()"); };

                        match job.output_stale_timeout {
                            Some(stale_timeout) => {
                                if last_output_changed_at.elapsed() >= stale_timeout {
                                    timed_out = true;
                                    restart_session = true;
                                    // best-effort: interrupt turn,但不依赖它来完成收敛(超时应立即返回)。
                                    if let Err(e) = guard.interrupt_turn().await {
                                        warn!(instance = %job.instance_id, error = %e, "turn/interrupt failed after timeout");
                                    }

                                    // 可审计证据: 把超时原因写到 stderr(不参与事件解析)。
                                    let _ = output_tx
                                        .send(HatJobOutputChunk {
                                            job_id: job.job_id,
                                            instance_id: job.instance_id.clone(),
                                            // 说明:
                                            // - 这是关键可见性信号(超时原因),不应该被 `--hide-stderr` 隐藏。
                                            // - 因此用 stdout 输出,但不进入 `output_for_parsing`。
                                            stream: OutputStream::Stdout,
                                            line: format!(
                                                "[codex-app-server] job timed out: output stale for {:?} (instance={} job={})",
                                                stale_timeout,
                                                job.instance_id.as_str(),
                                                job.job_id
                                            ),
                                        })
                                        .await;

                                    break;
                                }

                                next_check_deadline = Some(tokio::time::Instant::now() + check_interval);
                                if let Some(deadline) = next_check_deadline {
                                    sleep.as_mut().reset(deadline);
                                }
                            }
                            None => {
                                // 兼容兜底: 若未提供 stale 阈值,则退化为“硬超时”语义。
                                timed_out = true;
                                restart_session = true;
                                if let Err(e) = guard.interrupt_turn().await {
                                    warn!(instance = %job.instance_id, error = %e, "turn/interrupt failed after hard timeout");
                                }

                                let _ = output_tx
                                    .send(HatJobOutputChunk {
                                        job_id: job.job_id,
                                        instance_id: job.instance_id.clone(),
                                        // 同上: 超时原因是关键可见性信号,用 stdout 避免被 hide-stderr 吞掉。
                                        stream: OutputStream::Stdout,
                                        line: format!(
                                            "[codex-app-server] job timed out: hard timeout fired (instance={} job={})",
                                            job.instance_id.as_str(),
                                            job.job_id
                                        ),
                                    })
                                    .await;

                                break;
                            }
                        }
                    }
                    stderr_line = stderr_rx.recv(), if !stderr_closed => {
                        match stderr_line {
                            Ok(line) => {
                                last_output_changed_at = Instant::now();
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
                                // 重要: Closed 会导致 recv 立即返回;这里禁用分支,避免 busy loop.
                                stderr_closed = true;
                            }
                        }
                    }
                    changed = cancel_rx.changed(), if !cancel_closed => {
                        match changed {
                            Ok(()) => {
                                if *cancel_rx.borrow() {
                                    canceled = true;
                                    // best-effort: interrupt turn,但不 kill thread.
                                    if let Err(e) = guard.interrupt_turn().await {
                                        warn!(instance = %job.instance_id, error = %e, "turn/interrupt failed");
                                    }

                                    // 可见性: 取消原因同样应可见(避免 UI 看起来“啥也没发生”)。
                                    let _ = output_tx
                                        .send(HatJobOutputChunk {
                                            job_id: job.job_id,
                                            instance_id: job.instance_id.clone(),
                                            stream: OutputStream::Stdout,
                                            line: format!(
                                                "[codex-app-server] job canceled (instance={} job={})",
                                                job.instance_id.as_str(),
                                                job.job_id
                                            ),
                                        })
                                        .await;
                                    break;
                                }
                            }
                            Err(_) => {
                                // sender 被关闭: 禁用分支,避免 busy loop.
                                cancel_closed = true;
                            }
                        }
                    }
                    control = control_rx.recv(), if !control_closed => {
                        let Some(control) = control else {
                            // sender dropped: 禁用分支,避免 busy loop.
                            control_closed = true;
                            continue;
                        };

                        match control {
                            HatJobControl::Steer { input } => {
                                // ------------------------------------------------------------------
                                // 说明:
                                // - 真实 Codex app-server 在部分时序下:
                                //   - `turn/started` 已到,但 task 还没真正进入 active 状态；
                                //   - 过早发送 `turn/steer` 可能返回 `no active turn to steer`。
                                // - 这里的策略:
                                //   - 优先等到 `codex/event/task_started` 再发(更接近“in-flight steer”语义)；
                                //   - 但也提供一个短的兜底窗口(2s),避免某些版本不发 task_started 时永远不 flush。
                                // ------------------------------------------------------------------
                                let can_send_now = guard.active_turn_id.is_some()
                                    && (task_started
                                        || turn_started_at
                                            .is_some_and(|at| at.elapsed() >= Duration::from_secs(2)));

                                if can_send_now {
                                    if let Err(e) = guard.steer_turn(&input).await {
                                        warn!(instance = %job.instance_id, error = %e, "turn/steer failed");
                                    }
                                } else {
                                    // turnId 还没就绪,或者 task 还没进入 active：先缓存,后续再 flush。
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
                        if let Some(id) = msg.get("id").and_then(Value::as_u64)
                            && (msg.get("result").is_some() || msg.get("error").is_some())
                        {
                            // 对所有 response 做一次 id->method 清理,避免 map 无限增长.
                            let method = guard
                                .pending_request_methods
                                .remove(&id)
                                .unwrap_or_else(|| "<unknown>".to_string());

                            // error response: 必须可见,且对关键方法 fail-fast，避免“无输出卡死”.
                            if msg.get("error").is_some() {
                                last_output_changed_at = Instant::now();
                                let code = msg.pointer("/error/code").and_then(Value::as_i64);
                                let message = msg
                                    .pointer("/error/message")
                                    .and_then(Value::as_str)
                                    .unwrap_or("<missing>");

                                let mut preview: String = message.chars().take(400).collect();
                                if message.chars().count() > 400 {
                                    preview.push_str("...(truncated)");
                                }

                                // 重要:
                                // - 这里用 stdout 发送,确保 `--hide-stderr` 时仍可见(否则用户会看到“空白”).
                                // - 但我们不会把它拼进 `output_for_parsing`,避免污染事件解析。
                                let _ = output_tx
                                    .send(HatJobOutputChunk {
                                        job_id: job.job_id,
                                        instance_id: job.instance_id.clone(),
                                        stream: OutputStream::Stdout,
                                        line: format!(
                                            "[codex-app-server] ERROR: request failed (instance={} job={} method={} id={} code={code:?} message={preview})",
                                            job.instance_id.as_str(),
                                            job.job_id,
                                            method,
                                            id,
                                        ),
                                    })
                                    .await;

                                let is_fatal = matches!(method.as_str(), "initialize" | "thread/start" | "turn/start" | "<unknown>");
                                if is_fatal {
                                    restart_session = true;
                                    break;
                                }
                            }
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
                                    // turnId 已就绪：记录时间戳,后续用它判断“是否可以 flush steer”。
                                    turn_started_at = Some(Instant::now());
                                }
                                "codex/event/task_started" => {
                                    // 说明:
                                    // - 该事件是“模型任务真正开始跑”的信号。
                                    // - 我们用它作为 steer flush 的主要门槛,降低过早 steer 导致的协议错误。
                                    let Some(active_turn_id) = guard.active_turn_id.clone() else {
                                        continue;
                                    };

                                    let turn_id_in_msg = msg
                                        .pointer("/params/msg/turn_id")
                                        .and_then(Value::as_str)
                                        .or_else(|| msg.pointer("/params/msg/turnId").and_then(Value::as_str))
                                        .or_else(|| msg.pointer("/params/id").and_then(Value::as_str));

                                    let ok = match turn_id_in_msg {
                                        Some(tid) => tid == active_turn_id,
                                        None => true,
                                    };

                                    if ok {
                                        task_started = true;
                                        emit_activity_if_changed(
                                            &output_tx,
                                            job.job_id,
                                            &job.instance_id,
                                            &mut current_activity_label,
                                            "Working",
                                        )
                                        .await;
                                    }
                                }
                                "item/agentMessage/delta" => {
                                    if let Some(delta) = msg.pointer("/params/delta").and_then(Value::as_str) {
                                        last_output_changed_at = Instant::now();
                                        // 说明:
                                        // - 一旦我们观察到 agentMessage/delta,就优先使用它作为输出源。
                                        // - 如果此前我们临时用 summaryTextDelta 做 fallback,这里要立即切回
                                        //   agentMessage/delta,否则会丢失真正的 agent 输出(包括事件与 completion)。
                                        // - 为了避免“summary 里提到 completion_promise”导致误触发收敛,
                                        //   切回时会清空此前累计的 output(仅保留 agentMessage 输出)。
                                        if output_source != Some(CodexAppServerOutputSource::AgentMessageDelta) {
                                            output_source = Some(CodexAppServerOutputSource::AgentMessageDelta);
                                            output.clear();
                                            stream_pending.clear();
                                            last_summary_index = None;
                                        }

                                        output.push_str(delta);
                                        stream_pending.push_str(delta);
                                        emit_activity_from_text_if_changed(
                                            &output_tx,
                                            job.job_id,
                                            &job.instance_id,
                                            &mut current_activity_label,
                                            delta,
                                        )
                                        .await;
                                        flush_pending_output(
                                            job.job_id,
                                            &job.instance_id,
                                            OutputStream::Stdout,
                                            &mut stream_pending,
                                            &output_tx,
                                        )
                                        .await;
                                    }
                                }
                                "item/reasoning/summaryPartAdded" => {
                                    let Some(summary_index) =
                                        msg.pointer("/params/summaryIndex").and_then(Value::as_u64)
                                    else {
                                        continue;
                                    };

                                    last_output_changed_at = Instant::now();

                                    // --------------------------------------------------------------
                                    // 说明:
                                    // - summaryPartAdded 本身不带文本,但它是“分段”的可靠信号。
                                    // - 当我们已经进入 AgentMessage 输出源时,仍然把 reasoning summary 当作 thinking,
                                    //   持续回显到 stderr（对齐 `codex exec` 的“思考过程可见”体验）。
                                    // - 但 thinking 绝不进入 `HatJobResult.output`，避免干扰事件解析与收敛检测。
                                    // --------------------------------------------------------------
                                    if output_source == Some(CodexAppServerOutputSource::AgentMessageDelta) {
                                        if let Some(prev) = thinking_last_summary_index
                                            && summary_index > prev
                                        {
                                            thinking_pending.push_str("\n\n");
                                            flush_pending_output(
                                                job.job_id,
                                                &job.instance_id,
                                                OutputStream::Stderr,
                                                &mut thinking_pending,
                                                &output_tx,
                                            )
                                            .await;
                                        }
                                        thinking_last_summary_index = Some(summary_index);
                                    }

                                    // --------------------------------------------------------------
                                    // fallback: 如果当前输出源就是 summaryTextDelta,保持原有行为(插入空行).
                                    // --------------------------------------------------------------
                                    if output_source
                                        == Some(CodexAppServerOutputSource::ReasoningSummaryTextDelta)
                                    {
                                        if let Some(prev) = last_summary_index
                                            && summary_index > prev
                                        {
                                            output.push_str("\n\n");
                                            stream_pending.push_str("\n\n");
                                            flush_pending_output(
                                                job.job_id,
                                                &job.instance_id,
                                                OutputStream::Stdout,
                                                &mut stream_pending,
                                                &output_tx,
                                            )
                                            .await;
                                        }

                                        last_summary_index = Some(summary_index);
                                    }
                                }
                                "item/reasoning/summaryTextDelta" => {
                                    // 说明:
                                    // - 真实 Codex app-server 在某些版本下,不会推送 `item/agentMessage/delta`,
                                    //   而是推送 reasoning summary 的 text delta 作为 UI 可见输出.
                                    // - 我们把它作为 fallback 输出源,确保 `HatJobResult.output` 有内容可用于:
                                    //   1) completion_promise 检测(例如 LOOP_COMPLETE)
                                    //   2) 人类可读日志/排障(至少能看到“它在说什么”)
                                    // - 同时避免解析 `codex/event/reasoning_content_delta` 等内部通道,减少噪音与误判风险.
                                    let Some(delta) = msg.pointer("/params/delta").and_then(Value::as_str) else {
                                        continue;
                                    };

                                    last_output_changed_at = Instant::now();

                                    // --------------------------------------------------------------
                                    // 当我们已经进入 AgentMessage 输出源时:
                                    // - summaryTextDelta 视为 thinking,回显到 stderr（不进入 output）。
                                    // --------------------------------------------------------------
                                    if output_source == Some(CodexAppServerOutputSource::AgentMessageDelta) {
                                        thinking_pending.push_str(delta);
                                        emit_activity_from_text_if_changed(
                                            &output_tx,
                                            job.job_id,
                                            &job.instance_id,
                                            &mut current_activity_label,
                                            &thinking_pending,
                                        )
                                        .await;
                                        flush_pending_output(
                                            job.job_id,
                                            &job.instance_id,
                                            OutputStream::Stderr,
                                            &mut thinking_pending,
                                            &output_tx,
                                        )
                                        .await;
                                        continue;
                                    }

                                    let current = output_source.get_or_insert(
                                        CodexAppServerOutputSource::ReasoningSummaryTextDelta,
                                    );
                                    if *current != CodexAppServerOutputSource::ReasoningSummaryTextDelta {
                                        continue;
                                    }

                                    output.push_str(delta);
                                    stream_pending.push_str(delta);
                                    emit_activity_from_text_if_changed(
                                        &output_tx,
                                        job.job_id,
                                        &job.instance_id,
                                        &mut current_activity_label,
                                        &output,
                                    )
                                    .await;
                                    flush_pending_output(
                                        job.job_id,
                                        &job.instance_id,
                                        OutputStream::Stdout,
                                        &mut stream_pending,
                                        &output_tx,
                                    )
                                    .await;
                                }
                                "codex/event/task_complete" | "codex/event/task_completed" => {
                                    // ------------------------------------------------------------------
                                    // 说明:
                                    // - 部分真实 codex app-server 不会发送 `turn/completed`，而是发送 task_complete。
                                    // - 如果我们只等 `turn/completed`，job 可能永远不结束,只能依赖 Supervisor cancel.
                                    // - 这里做协议兼容: 在看到 task_complete 时,把当前 in-flight turn 视为完成。
                                    // ------------------------------------------------------------------
                                    let Some(active_turn_id) = guard.active_turn_id.clone() else {
                                        continue;
                                    };

                                    // best-effort: 如果 payload 里能找到 turn_id,就做一次匹配.
                                    let turn_id_in_msg = msg
                                        .pointer("/params/msg/turn_id")
                                        .and_then(Value::as_str)
                                        .or_else(|| msg.pointer("/params/msg/turnId").and_then(Value::as_str))
                                        .or_else(|| msg.pointer("/params/id").and_then(Value::as_str));

                                    let ok = match turn_id_in_msg {
                                        Some(tid) => tid == active_turn_id,
                                        None => true, // 缺失 turn_id 时,按“当前唯一 in-flight turn”处理。
                                    };

                                    if ok {
                                        completed = true;
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
                                    last_output_changed_at = Instant::now();

                                    // best-effort: 把 error 通知也回显给人类(同样走 stdout,避免 hide-stderr 下完全不可见)。
                                    let preview = CodexAppServerSession::json_preview(&msg, 400);
                                    let _ = output_tx
                                        .send(HatJobOutputChunk {
                                            job_id: job.job_id,
                                            instance_id: job.instance_id.clone(),
                                            stream: OutputStream::Stdout,
                                            line: format!(
                                                "[codex-app-server] ERROR: notification received (instance={} job={} json={preview})",
                                                job.instance_id.as_str(),
                                                job.job_id
                                            ),
                                        })
                                        .await;
                                }
                                _ => {
                                    // 其他通知不影响 job 收敛
                                }
                            }
                        }

                        // ------------------------------------------------------------------
                        // steer flush(统一出口):
                        // - 避免分散在多个分支里重复 drain 逻辑。
                        // ------------------------------------------------------------------
                        if guard.active_turn_id.is_some()
                            && !pending_steers.is_empty()
                            && (task_started
                                || turn_started_at
                                    .is_some_and(|at| at.elapsed() >= Duration::from_secs(2)))
                        {
                            for steer in pending_steers.drain(..) {
                                if let Err(e) = guard.steer_turn(&steer).await {
                                    warn!(instance = %job.instance_id, error = %e, "turn/steer (flush) failed");
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
            if !thinking_pending.trim().is_empty() {
                let _ = output_tx
                    .send(HatJobOutputChunk {
                        job_id: job.job_id,
                        instance_id: job.instance_id.clone(),
                        stream: OutputStream::Stderr,
                        line: thinking_pending.clone(),
                    })
                    .await;
            }

            let result = HatJobResult {
                output_for_parsing: output,
                observed_stderr: String::new(),
                success: completed && !canceled && !timed_out,
                exit_code: completed
                    .then_some(0)
                    .or((!canceled && !timed_out && !completed).then_some(1)),
                timed_out,
                canceled,
            };

            (result, restart_session)
        };

        let (result, should_restart_session) = timed_out_restart;
        if should_restart_session {
            self.restart_session(&job.instance_id).await;
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
        options: &CodexAppServerOptions,
    ) -> Result<Arc<Mutex<CodexAppServerSession>>> {
        let key = instance_id.to_string();
        if let Some(existing) = self.sessions.lock().await.get(&key).cloned() {
            return Ok(existing);
        }

        let session = CodexAppServerSession::spawn(&key, &self.codex_command, options)?;
        let session = Arc::new(Mutex::new(session));
        let mut guard = self.sessions.lock().await;
        let entry = guard.entry(key).or_insert_with(|| Arc::clone(&session));
        Ok(Arc::clone(entry))
    }

    async fn restart_session(&self, instance_id: &HatInstanceId) {
        // ------------------------------------------------------------------
        // 说明:
        // - app-server job 超时后,我们倾向于重启 session:
        //   - 避免残留的 active turn/thread 污染后续 job。
        //   - 让“超时”成为确定性失败,而不是把系统拖入半死不活状态。
        // - 锁顺序: sessions(map) -> session(mutex),避免与 shutdown_all 形成死锁。
        // ------------------------------------------------------------------
        let key = instance_id.to_string();
        let session = {
            let mut guard = self.sessions.lock().await;
            guard.remove(&key)
        };
        let Some(session) = session else {
            return;
        };

        let mut guard = session.lock().await;
        guard.shutdown().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ralph_proto::{HatId, SessionStrategy};

    #[test]
    fn parse_codex_app_server_options_maps_full_auto_and_model() {
        let backend = CliBackend {
            command: "codex".to_string(),
            args: vec![
                "exec".to_string(),
                "--full-auto".to_string(),
                "--model".to_string(),
                "gpt-5.5".to_string(),
                "--config".to_string(),
                "model_reasoning_effort=\"low\"".to_string(),
            ],
            prompt_mode: crate::PromptMode::Arg,
            prompt_flag: None,
            output_format: crate::OutputFormat::Text,
        };

        let opts = parse_codex_app_server_options(&backend, None);
        assert_eq!(opts.sandbox.as_deref(), Some("workspace-write"));
        assert_eq!(opts.approval_policy.as_deref(), Some("on-request"));
        assert_eq!(opts.model.as_deref(), Some("gpt-5.5"));
        assert_eq!(
            opts.config_overrides,
            vec!["model_reasoning_effort=\"low\"".to_string()]
        );
    }

    #[test]
    fn build_codex_app_server_process_args_forwards_profile_and_config_overrides() {
        let options = CodexAppServerOptions {
            profile: Some("e2e".to_string()),
            config_overrides: vec![
                "model_reasoning_effort=\"low\"".to_string(),
                "model_reasoning_summary=\"none\"".to_string(),
            ],
            ..Default::default()
        };

        let args = build_codex_app_server_process_args(&options);
        assert_eq!(
            args,
            vec![
                "app-server".to_string(),
                "--listen".to_string(),
                "stdio://".to_string(),
                "--profile".to_string(),
                "e2e".to_string(),
                "--config".to_string(),
                "model_reasoning_effort=\"low\"".to_string(),
                "--config".to_string(),
                "model_reasoning_summary=\"none\"".to_string(),
            ]
        );
    }

    #[test]
    fn build_prompt_transcript_lines_includes_ansi_and_preserves_empty_lines() {
        let instance_id = HatInstanceId::from("ralph#1");
        let lines = build_prompt_transcript_lines(
            "turn/start input",
            &instance_id,
            7,
            "hello\nworld\n",
            true,
        );

        assert!(
            lines.first().is_some_and(|l| l.contains("\x1b[")),
            "Expected ANSI header when use_colors=true: {lines:?}"
        );
        assert!(
            lines.iter().any(|l| l.contains("BEGIN PROMPT")),
            "Expected BEGIN marker: {lines:?}"
        );
        assert!(
            lines.iter().any(|l| l.contains("END PROMPT")),
            "Expected END marker: {lines:?}"
        );

        // prompt 以 \\n 结尾时,必须保留尾部空行(否则 transcript 与真实输入不一致).
        let empty_lines = lines.iter().filter(|l| l.is_empty()).count();
        assert_eq!(
            empty_lines, 1,
            "Expected one trailing empty line: {lines:?}"
        );
    }

    #[test]
    fn build_prompt_transcript_lines_is_plain_when_colors_disabled() {
        let instance_id = HatInstanceId::from("ralph#1");
        let lines = build_prompt_transcript_lines("turn/start input", &instance_id, 1, "hi", false);
        assert!(
            !lines.iter().any(|l| l.contains("\x1b[")),
            "Expected no ANSI when use_colors=false: {lines:?}"
        );
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "current_thread")]
    async fn app_server_timeout_triggers_and_returns_timed_out() -> Result<()> {
        use std::os::unix::fs::PermissionsExt;
        use tempfile::tempdir;

        // ------------------------------------------------------------------
        // 目标:
        // - 锁死 app-server runtime 必须尊重 HatJob.timeout/output_stale_timeout:
        //   - 当长时间没有任何输出时,必须确定性返回 timed_out=true。
        // - 使用 fake `codex app-server` shim,避免依赖真实 codex。
        // ------------------------------------------------------------------

        let dir = tempdir().context("Failed to create tempdir")?;
        let fake_codex = dir.path().join("codex");

        let script = r#"#!/usr/bin/env python3
import json
import sys
import time

def send(obj) -> None:
    sys.stdout.write(json.dumps(obj, ensure_ascii=False) + "\n")
    sys.stdout.flush()

def run_app_server() -> int:
    thread_id = "thread-1"
    turn_id = "turn-1"

    for raw in sys.stdin:
        line = raw.strip()
        if not line:
            continue

        try:
            msg = json.loads(line)
        except Exception:
            # stdout 必须保持为 JSON,否则 client 侧会 parse 失败;这里直接忽略坏输入
            continue

        method = msg.get("method")
        msg_id = msg.get("id")

        # notifications: {method, params} (no id)
        if msg_id is None and method:
            continue

        # requests: {id, method, params}
        if msg_id is None or method is None:
            continue

        if method == "initialize":
            send({"id": msg_id, "result": {}})
            continue

        if method == "thread/start":
            send({"id": msg_id, "result": {}})
            send({"method": "thread/started", "params": {"thread": {"id": thread_id}}})
            continue

        if method == "turn/start":
            send({"id": msg_id, "result": {}})
            send({"method": "turn/started", "params": {"turn": {"id": turn_id}}})
            # 关键: 不发送任何 delta/completed,制造“无输出卡死”场景,等待 client 超时 kill。
            while True:
                time.sleep(1)

        if method in ("turn/interrupt", "turn/steer"):
            send({"id": msg_id, "result": {}})
            continue

        # unknown method: respond ok to avoid client hanging on a response
        send({"id": msg_id, "result": {}})

    return 0

def main() -> int:
    argv = sys.argv
    if len(argv) >= 2 and argv[1] == "app-server":
        return run_app_server()
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
"#;

        std::fs::write(&fake_codex, script).context("Failed to write fake codex script")?;
        let mut perms = std::fs::metadata(&fake_codex)
            .context("Failed to stat fake codex script")?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&fake_codex, perms)
            .context("Failed to chmod fake codex script")?;

        let runtime =
            CodexAppServerRuntime::new_with_command(false, fake_codex.display().to_string());

        let job = HatJob {
            job_id: 1,
            instance_id: HatInstanceId::from("writer#1"),
            hat_id: HatId::new("writer"),
            prompt: "hello".to_string(),
            continuation_prompt: Some("continue".to_string()),
            backend: ralph_core::JobBackend::Default,
            session_strategy: SessionStrategy::AppServer,
            timeout: Some(Duration::from_millis(200)),
            output_stale_timeout: Some(Duration::from_millis(50)),
            workdir: None,
        };

        let backend = CliBackend {
            command: "codex".to_string(),
            args: vec![],
            prompt_mode: crate::PromptMode::Arg,
            prompt_flag: None,
            output_format: crate::OutputFormat::Text,
        };

        // output_tx: 丢弃 receiver,避免 channel backpressure 影响测试(我们只关心 timed_out 语义)。
        let (output_tx, output_rx) = mpsc::channel::<HatJobOutputChunk>(1);
        drop(output_rx);

        let (_cancel_tx, cancel_rx) = watch::channel(false);
        let (_control_tx, control_rx) = mpsc::channel::<HatJobControl>(1);

        let result = tokio::time::timeout(
            Duration::from_secs(2),
            runtime.execute_job(&job, &backend, output_tx, cancel_rx, control_rx),
        )
        .await
        .expect("execute_job should not hang")?;

        assert!(result.timed_out, "Expected timed_out=true: {result:?}");
        assert!(
            !result.success,
            "Expected success=false when timed_out=true: {result:?}"
        );

        runtime.shutdown_all().await;
        Ok(())
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "current_thread")]
    async fn app_server_turn_start_error_is_visible_and_fails_fast() -> Result<()> {
        use std::os::unix::fs::PermissionsExt;
        use tempfile::tempdir;

        // ------------------------------------------------------------------
        // 目标:
        // - 锁死: 当 app-server 对关键 request(turn/start)返回 `{id, error}` 时:
        //   1) 必须把错误写到 streaming output(且默认可见,不再“静默卡死”).
        //   2) job 必须 fail-fast 返回(success=false, timed_out=false).
        // ------------------------------------------------------------------

        let dir = tempdir().context("Failed to create tempdir")?;
        let fake_codex = dir.path().join("codex");

        let script = r#"#!/usr/bin/env python3
import json
import sys
import time

def send(obj) -> None:
    sys.stdout.write(json.dumps(obj, ensure_ascii=False) + "\n")
    sys.stdout.flush()

def run_app_server() -> int:
    thread_id = "thread-1"
    for raw in sys.stdin:
        line = raw.strip()
        if not line:
            continue

        try:
            msg = json.loads(line)
        except Exception:
            continue

        method = msg.get("method")
        msg_id = msg.get("id")

        if msg_id is None or method is None:
            continue

        if method == "initialize":
            send({"id": msg_id, "result": {}})
            continue

        if method == "thread/start":
            send({"id": msg_id, "result": {}})
            send({"method": "thread/started", "params": {"thread": {"id": thread_id}}})
            continue

        if method == "turn/start":
            send({"id": msg_id, "error": {"code": 400, "message": "bad request: turn/start failed (test)"}})
            while True:
                time.sleep(1)

        if method in ("turn/interrupt", "turn/steer"):
            send({"id": msg_id, "result": {}})
            continue

        send({"id": msg_id, "result": {}})

    return 0

def main() -> int:
    argv = sys.argv
    if len(argv) >= 2 and argv[1] == "app-server":
        return run_app_server()
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
"#;

        std::fs::write(&fake_codex, script).context("Failed to write fake codex script")?;
        let mut perms = std::fs::metadata(&fake_codex)
            .context("Failed to stat fake codex script")?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&fake_codex, perms)
            .context("Failed to chmod fake codex script")?;

        let runtime =
            CodexAppServerRuntime::new_with_command(false, fake_codex.display().to_string());

        let job = HatJob {
            job_id: 1,
            instance_id: HatInstanceId::from("writer#1"),
            hat_id: HatId::new("writer"),
            prompt: "hello".to_string(),
            continuation_prompt: Some("continue".to_string()),
            backend: ralph_core::JobBackend::Default,
            session_strategy: SessionStrategy::AppServer,
            timeout: Some(Duration::from_secs(2)),
            output_stale_timeout: Some(Duration::from_secs(2)),
            workdir: None,
        };

        let backend = CliBackend {
            command: "codex".to_string(),
            args: vec![],
            prompt_mode: crate::PromptMode::Arg,
            prompt_flag: None,
            output_format: crate::OutputFormat::Text,
        };

        let (output_tx, mut output_rx) = mpsc::channel::<HatJobOutputChunk>(128);
        let (_cancel_tx, cancel_rx) = watch::channel(false);
        let (_control_tx, control_rx) = mpsc::channel::<HatJobControl>(1);

        let collector = tokio::spawn(async move {
            let mut lines = Vec::<HatJobOutputChunk>::new();
            while let Some(chunk) = output_rx.recv().await {
                lines.push(chunk);
            }
            lines
        });

        let result = tokio::time::timeout(
            Duration::from_secs(2),
            runtime.execute_job(&job, &backend, output_tx, cancel_rx, control_rx),
        )
        .await
        .expect("execute_job should not hang")?;

        let chunks = collector.await.expect("collector task should not panic");

        assert!(
            chunks.iter().any(|c| {
                c.stream == OutputStream::Stdout
                    && c.line.contains("ERROR: request failed")
                    && c.line.contains("method=turn/start")
            }),
            "Expected visible turn/start error on stdout: chunks={chunks:?}"
        );

        assert!(!result.timed_out, "Expected timed_out=false: {result:?}");
        assert!(!result.canceled, "Expected canceled=false: {result:?}");
        assert!(!result.success, "Expected success=false: {result:?}");
        assert_eq!(
            result.exit_code,
            Some(1),
            "Expected exit_code=1 for fatal error: {result:?}"
        );

        runtime.shutdown_all().await;
        Ok(())
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "current_thread")]
    async fn app_server_task_started_and_reasoning_summary_emit_activity_chunks() -> Result<()> {
        use std::os::unix::fs::PermissionsExt;
        use tempfile::tempdir;

        // ------------------------------------------------------------------
        // 目标:
        // - Codex app-server 的稳定 lifecycle / reasoning 信号应当进入 activity 流。
        // - 这样并行 TUI 才能显示类似 Codex 的 `Working` / `Inspecting ...` 状态。
        // ------------------------------------------------------------------

        let dir = tempdir().context("Failed to create tempdir")?;
        let fake_codex = dir.path().join("codex");

        let script = r#"#!/usr/bin/env python3
import json
import sys

def send(obj) -> None:
    sys.stdout.write(json.dumps(obj, ensure_ascii=False) + "\n")
    sys.stdout.flush()

def run_app_server() -> int:
    thread_id = "thread-1"
    turn_id = "turn-1"

    for raw in sys.stdin:
        line = raw.strip()
        if not line:
            continue

        try:
            msg = json.loads(line)
        except Exception:
            continue

        method = msg.get("method")
        msg_id = msg.get("id")
        if msg_id is None or method is None:
            continue

        if method == "initialize":
            send({"id": msg_id, "result": {}})
            continue

        if method == "thread/start":
            send({"id": msg_id, "result": {}})
            send({"method": "thread/started", "params": {"thread": {"id": thread_id}}})
            continue

        if method == "turn/start":
            send({"id": msg_id, "result": {}})
            send({"method": "turn/started", "params": {"turn": {"id": turn_id}}})
            send({"method": "codex/event/task_started", "params": {"msg": {"turn_id": turn_id}}})
            send({"method": "item/reasoning/summaryTextDelta", "params": {"delta": "Inspecting current code behavior"}})
            send({"method": "turn/completed", "params": {"turn": {"id": turn_id}}})
            continue

        if method in ("turn/interrupt", "turn/steer"):
            send({"id": msg_id, "result": {}})
            continue

        send({"id": msg_id, "result": {}})

    return 0

def main() -> int:
    argv = sys.argv
    if len(argv) >= 2 and argv[1] == "app-server":
        return run_app_server()
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
"#;

        std::fs::write(&fake_codex, script).context("Failed to write fake codex script")?;
        let mut perms = std::fs::metadata(&fake_codex)
            .context("Failed to stat fake codex script")?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&fake_codex, perms)
            .context("Failed to chmod fake codex script")?;

        let runtime =
            CodexAppServerRuntime::new_with_command(false, fake_codex.display().to_string());

        let job = HatJob {
            job_id: 1,
            instance_id: HatInstanceId::from("writer#1"),
            hat_id: HatId::new("writer"),
            prompt: "hello".to_string(),
            continuation_prompt: Some("continue".to_string()),
            backend: ralph_core::JobBackend::Default,
            session_strategy: SessionStrategy::AppServer,
            timeout: Some(Duration::from_secs(2)),
            output_stale_timeout: Some(Duration::from_secs(2)),
            workdir: None,
        };

        let backend = CliBackend {
            command: "codex".to_string(),
            args: vec![],
            prompt_mode: crate::PromptMode::Arg,
            prompt_flag: None,
            output_format: crate::OutputFormat::Text,
        };

        let (output_tx, mut output_rx) = mpsc::channel::<HatJobOutputChunk>(128);
        let (_cancel_tx, cancel_rx) = watch::channel(false);
        let (_control_tx, control_rx) = mpsc::channel::<HatJobControl>(1);

        let collector = tokio::spawn(async move {
            let mut chunks = Vec::<HatJobOutputChunk>::new();
            while let Some(chunk) = output_rx.recv().await {
                chunks.push(chunk);
            }
            chunks
        });

        let result = tokio::time::timeout(
            Duration::from_secs(2),
            runtime.execute_job(&job, &backend, output_tx, cancel_rx, control_rx),
        )
        .await
        .expect("execute_job should not hang")?;

        let chunks = collector.await.expect("collector task should not panic");

        assert!(
            result.success,
            "Expected successful fake app-server job: {result:?}"
        );
        assert!(
            chunks
                .iter()
                .any(|c| c.stream == OutputStream::Activity && c.line == "Working"),
            "Expected working activity chunk: {chunks:?}"
        );
        assert!(
            chunks.iter().any(|c| {
                c.stream == OutputStream::Activity && c.line == "Inspecting current code behavior"
            }),
            "Expected reasoning summary activity chunk: {chunks:?}"
        );

        runtime.shutdown_all().await;
        Ok(())
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "current_thread")]
    async fn app_server_reuses_session_but_switches_to_continuation_input_after_first_turn()
    -> Result<()> {
        use std::os::unix::fs::PermissionsExt;
        use tempfile::tempdir;

        let dir = tempdir().context("Failed to create tempdir")?;
        let fake_codex = dir.path().join("codex");
        let transcript_file = dir.path().join("turn-inputs.jsonl");

        let script = format!(
            r#"#!/usr/bin/env python3
import json
import sys
from pathlib import Path

TRANSCRIPT = Path({transcript_path:?})

def send(obj) -> None:
    sys.stdout.write(json.dumps(obj, ensure_ascii=False) + "\n")
    sys.stdout.flush()

def append_turn_input(text: str) -> None:
    with TRANSCRIPT.open("a", encoding="utf-8") as f:
        f.write(json.dumps({{"text": text}}, ensure_ascii=False) + "\n")

def extract_all_text(value) -> str:
    parts = []
    if isinstance(value, dict):
        for k, v in value.items():
            if k == "text" and isinstance(v, str):
                parts.append(v)
            else:
                parts.append(extract_all_text(v))
    elif isinstance(value, list):
        for item in value:
            parts.append(extract_all_text(item))
    return "".join(parts)

def run_app_server() -> int:
    thread_id = "thread-1"
    turn_counter = 0

    for raw in sys.stdin:
        line = raw.strip()
        if not line:
            continue

        try:
            msg = json.loads(line)
        except Exception:
            continue

        method = msg.get("method")
        msg_id = msg.get("id")
        if msg_id is None or method is None:
            continue

        if method == "initialize":
            send({{"id": msg_id, "result": {{}}}})
            continue

        if method == "thread/start":
            send({{"id": msg_id, "result": {{}}}})
            send({{"method": "thread/started", "params": {{"thread": {{"id": thread_id}}}}}})
            continue

        if method == "turn/start":
            turn_counter += 1
            send({{"id": msg_id, "result": {{}}}})
            params = msg.get("params", {{}})
            append_turn_input(extract_all_text(params.get("input", [])))
            turn_id = f"turn-{{turn_counter}}"
            send({{"method": "turn/started", "params": {{"turn": {{"id": turn_id}}}}}})
            send({{"method": "codex/event/task_started", "params": {{"msg": {{"turn_id": turn_id}}}}}})
            send({{"method": "item/agentMessage/delta", "params": {{"delta": f"done-{{turn_counter}}\\n"}}}})
            send({{"method": "turn/completed", "params": {{"turn": {{"id": turn_id}}}}}})
            continue

        if method in ("turn/interrupt", "turn/steer"):
            send({{"id": msg_id, "result": {{}}}})
            continue

        send({{"id": msg_id, "result": {{}}}})

    return 0

def main() -> int:
    argv = sys.argv
    if len(argv) >= 2 and argv[1] == "app-server":
        return run_app_server()
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
"#,
            transcript_path = transcript_file.display().to_string()
        );

        std::fs::write(&fake_codex, script).context("Failed to write fake codex script")?;
        let mut perms = std::fs::metadata(&fake_codex)
            .context("Failed to stat fake codex script")?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&fake_codex, perms)
            .context("Failed to chmod fake codex script")?;

        let runtime =
            CodexAppServerRuntime::new_with_command(false, fake_codex.display().to_string());

        let backend = CliBackend {
            command: "codex".to_string(),
            args: vec![],
            prompt_mode: crate::PromptMode::Arg,
            prompt_flag: None,
            output_format: crate::OutputFormat::Text,
        };

        let make_job = |job_id: u64, prompt: &str, continuation_prompt: &str| HatJob {
            job_id,
            instance_id: HatInstanceId::from("ralph#1"),
            hat_id: HatId::new("ralph"),
            prompt: prompt.to_string(),
            continuation_prompt: Some(continuation_prompt.to_string()),
            backend: ralph_core::JobBackend::Default,
            session_strategy: SessionStrategy::AppServer,
            timeout: Some(Duration::from_secs(2)),
            output_stale_timeout: Some(Duration::from_secs(2)),
            workdir: None,
        };

        for (job_id, prompt, continuation_prompt) in [
            (
                1,
                "FULL PROMPT 1\n## ALL HAT PROMPT\n### Incoming Events\n- id=E1 topic=human.message payload=first",
                "CONTINUE 1\n### Incoming Events\n- id=E1 topic=human.message payload=first",
            ),
            (
                2,
                "FULL PROMPT 2\n## ALL HAT PROMPT\n### Incoming Events\n- id=E2 topic=human.message payload=second",
                "CONTINUE 2\n### Incoming Events\n- id=E2 topic=human.message payload=second",
            ),
        ] {
            let (output_tx, output_rx) = mpsc::channel::<HatJobOutputChunk>(64);
            drop(output_rx);
            let (_cancel_tx, cancel_rx) = watch::channel(false);
            let (_control_tx, control_rx) = mpsc::channel::<HatJobControl>(1);
            let job = make_job(job_id, prompt, continuation_prompt);
            let result = runtime
                .execute_job(&job, &backend, output_tx, cancel_rx, control_rx)
                .await?;
            assert!(
                result.success,
                "Expected successful fake app-server job: {result:?}"
            );
        }

        let transcript = std::fs::read_to_string(&transcript_file)
            .context("Failed to read fake app-server transcript")?;
        let turn_inputs = transcript
            .lines()
            .map(serde_json::from_str::<Value>)
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to parse recorded turn inputs")?;

        assert_eq!(
            turn_inputs.len(),
            2,
            "Expected exactly two turn/start inputs"
        );

        let first_input = turn_inputs[0]
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or("<missing>");
        let second_input = turn_inputs[1]
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or("<missing>");

        assert!(
            first_input.contains("FULL PROMPT 1"),
            "First turn should receive the full prompt: {first_input:?}"
        );
        assert!(
            first_input.contains("## ALL HAT PROMPT"),
            "First turn should still include the large base prompt context: {first_input:?}"
        );
        assert!(
            second_input.contains("CONTINUE 2"),
            "Second turn should use continuation input: {second_input:?}"
        );
        assert!(
            !second_input.contains("FULL PROMPT 2"),
            "Second turn must not resend the full prompt: {second_input:?}"
        );
        assert!(
            !second_input.contains("## ALL HAT PROMPT"),
            "Second turn must not resend all-hat prompt overlay: {second_input:?}"
        );

        runtime.shutdown_all().await;
        Ok(())
    }
}
