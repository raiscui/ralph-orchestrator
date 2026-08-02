//! headless job 执行: spawn 外部 CLI 进程, 流式采集 stdout/stderr。
//!
//! 说明:
//! - 这是 `HatJobExecutor` 三个后端形态之一(一次性 exec)。
//! - 只消费 stdout 做事件解析; stderr 仅作可观测输出(见 `handle_output_line`)。
//! - Unix 下每个 job 独立进程组, cancel/timeout 时整组终止, 避免残留进程。

use crate::cli_backend::CliBackend;
use crate::codex_env::scrub_codex_parent_session_env_tokio;
use crate::ralph_env::scrub_ralph_parent_worker_env_tokio;
use anyhow::Context;
use ralph_core::{HatJob, HatJobOutputChunk, HatJobResult, OutputStream};
use ralph_proto::HatInstanceId;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, watch};
use tracing::{debug, warn};

/// 执行一次 headless job: spawn 进程、流式采集、超时/取消处理、产出 HatJobResult。
pub(crate) async fn spawn_headless_job(
    backend: &CliBackend,
    job: &HatJob,
    output_tx: mpsc::Sender<HatJobOutputChunk>,
    mut cancel_rx: watch::Receiver<bool>,
) -> anyhow::Result<HatJobResult> {

    let (cmd, args, stdin_input, _temp_file) = backend.build_command(&job.prompt, false);

    let mut command = Command::new(&cmd);
    command.args(&args);
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    scrub_codex_parent_session_env_tokio(&mut command, &cmd);
    scrub_ralph_parent_worker_env_tokio(&mut command);

    // Unix: 让每个 job 成为一个新的进程组 leader。
    //
    // 为什么:
    // - 后端进程可能会再 spawn 子进程.
    // - cancel/timeout/中断时,我们需要“一刀切”杀掉整组,避免残留污染下一次 run/test.
    // - 同时避免“kill 自己所在进程组”导致 orchestrator 自杀,从而来不及写 `_meta.termination`.
    #[cfg(unix)]
    command.process_group(0);

    // 并行模式回放/诊断：把实例信息传给后端（custom backend 可用来做输出分流）
    command.env("RALPH_HAT_INSTANCE_ID", job.instance_id.as_str());
    command.env("RALPH_HAT_ID", job.hat_id.as_str());

    if let Some(workdir) = &job.workdir {
        command.current_dir(workdir);
    }

    if stdin_input.is_some() {
        command.stdin(Stdio::piped());
    }

    debug!(
        instance = %job.instance_id,
        hat = %job.hat_id,
        command = %cmd,
        args = ?args,
        workdir = ?job.workdir,
        "Spawning headless job"
    );

    let mut child = command.spawn().with_context(|| {
        format!(
            "Failed to spawn backend process: cmd={cmd} args={args:?} workdir={:?}",
            job.workdir
        )
    })?;

    // 写 stdin（如需要）
    if let Some(input) = stdin_input
        && let Some(mut stdin) = child.stdin.take()
    {
        stdin.write_all(input.as_bytes()).await?;
        drop(stdin);
    }

    // 并发读取 stdout/stderr，避免 pipe buffer deadlock
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let (line_tx, mut line_rx) = tokio::sync::mpsc::channel::<(OutputStream, String)>(256);

    let spawn_reader =
        |stream: OutputStream,
         handle: Option<tokio::process::ChildStdout>,
         tx: tokio::sync::mpsc::Sender<(OutputStream, String)>| async move {
            if let Some(handle) = handle {
                let reader = BufReader::new(handle);
                let mut lines = reader.lines();
                while let Some(line) = lines.next_line().await? {
                    if tx.send((stream, line)).await.is_err() {
                        break;
                    }
                }
            }
            Ok::<(), std::io::Error>(())
        };

    // stdout
    let tx1 = line_tx.clone();
    let stdout_task = tokio::spawn(spawn_reader(OutputStream::Stdout, stdout, tx1));
    // stderr（类型不同：ChildStderr）
    let stderr_tx = line_tx.clone();
    let stderr_task = tokio::spawn(async move {
        if let Some(stderr) = stderr {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Some(line) = lines.next_line().await? {
                if stderr_tx.send((OutputStream::Stderr, line)).await.is_err() {
                    break;
                }
            }
        }
        Ok::<(), std::io::Error>(())
    });

    // 释放主 sender，让 rx 能在两个 task 结束后自然关闭
    drop(line_tx);

    let mut stdout_output = String::new();
    let mut stderr_output = String::new();
    let mut timed_out = false;
    let mut canceled = false;
    let mut last_output_changed_at = std::time::Instant::now();

    // 收集输出，直到流结束 or 被取消/超时
    match job.timeout.filter(|d| !d.is_zero()) {
        Some(check_interval) => {
            // “检测超时”语义：
            // - check_interval 到期时不立刻 kill
            // - 只有当输出停滞超过 `output_stale_timeout` 才判定 timed_out
            // - 若输出仍在变化：判定通过，并把检测窗口从此刻重新计时
            let mut next_check_deadline = tokio::time::Instant::now() + check_interval;
            let sleep = tokio::time::sleep_until(next_check_deadline);
            tokio::pin!(sleep);

            loop {
                tokio::select! {
                    biased;
                    // 取消优先
                    changed = cancel_rx.changed() => {
                        if changed.is_ok() && *cancel_rx.borrow() {
                            canceled = true;
                            terminate_child(&mut child, "canceled").await?;
                            break;
                        }
                    }
                    // 检测窗口到期：根据“输出是否停滞”决定是否超时
                    _ = &mut sleep => {
                        match job.output_stale_timeout {
                            Some(stale_timeout) => {
                                if last_output_changed_at.elapsed() >= stale_timeout {
                                    timed_out = true;
                                    terminate_child(&mut child, "timed_out").await?;
                                    break;
                                }

                                // 检测通过：检测窗口重新计时（从现在开始）
                                next_check_deadline = tokio::time::Instant::now() + check_interval;
                                sleep.as_mut().reset(next_check_deadline);
                            }
                            None => {
                                // 兼容兜底：若未提供 stale 阈值，则退化为“硬超时”
                                timed_out = true;
                                terminate_child(&mut child, "timed_out").await?;
                                break;
                            }
                        }
                    }
                    line = line_rx.recv() => {
                        let Some((stream, line)) = line else {
                            break;
                        };

                        last_output_changed_at = std::time::Instant::now();
                        handle_output_line(
                            job.job_id,
                            &job.instance_id,
                            &output_tx,
                            backend,
                            &mut stdout_output,
                            &mut stderr_output,
                            stream,
                            line,
                        )
                        .await;
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
                        terminate_child(&mut child, "canceled").await?;
                        break;
                    }
                }
                line = line_rx.recv() => {
                    let Some((stream, line)) = line else {
                        break;
                    };
                    handle_output_line(
                        job.job_id,
                        &job.instance_id,
                        &output_tx,
                        backend,
                        &mut stdout_output,
                        &mut stderr_output,
                        stream,
                        line,
                    )
                    .await;
                }
            }
        },
    }

    // 确保子进程被 reap（避免僵尸进程）
    let status = child.wait().await?;

    // 等待读取 task 收尾（best-effort）
    let _ = stdout_task.await;
    let _ = stderr_task.await;

    let output_for_parsing = finalize_output_for_parsing(backend, &stdout_output);

    if backend.emits_structured_response()
        && let Some(display_output) =
            backend.finalize_structured_stdout_for_display(&stdout_output)
    {
        emit_final_structured_output(
            job.job_id,
            &job.instance_id,
            &output_tx,
            &display_output,
        )
        .await;
    }

    Ok(HatJobResult {
        output_for_parsing,
        observed_stderr: stderr_output,
        success: status.success() && !timed_out && !canceled,
        exit_code: status.code(),
        timed_out,
        canceled,
    })
}

    fn finalize_output_for_parsing(backend: &CliBackend, stdout_output: &str) -> String {
        // -----------------------------------------------------------------
        // 说明：
        // - 并行模式下，事件解析只能消费 stdout-only 的稳定正文。
        // - stderr 经常带 prompt transcript、后端日志、warning，甚至会出现 `<event ...>` 示例。
        // - 一旦把 stderr 混回解析输入，就会制造伪事件、重复路由和假 completion。
        // - 对结构化 backend，仍然要先从 stdout 中提取最终 response，再交给 EventParser。
        // -----------------------------------------------------------------
        if backend.emits_structured_response()
            && let Some(response) = backend.finalize_structured_stdout_for_display(stdout_output)
        {
            return normalize_codex_leading_escaped_event_output(backend, &response)
                .unwrap_or(response);
        }

        normalize_codex_leading_escaped_event_output(backend, stdout_output)
            .unwrap_or_else(|| stdout_output.to_string())
    }
    fn normalize_codex_leading_escaped_event_output(
        backend: &CliBackend,
        output: &str,
    ) -> Option<String> {
        // -----------------------------------------------------------------
        // 说明：
        // - 这里故意不去改 `EventParser` 的通用协议。
        // - 仓库里大量 prompt / overlay / README 会把 `<event ...>` 转义成
        //   `&lt;event ...&gt;` 作为“展示文本”，它们默认不应被当成真实事件。
        // - 但真实 Codex 最终回复偶尔会把“本来就要发到 stdout 的真实事件”
        //   HTML 转义后再吐出来，导致 durable 主流漏事件。
        // - 因此这里只做一个很窄的恢复：
        //   - 仅限 `codex`
        //   - 仅限去掉前导空白后就直接以 `&lt;event` 开头的回复
        //   - 仅解码 tag 边界，不做全量 HTML unescape
        // -----------------------------------------------------------------
        const ESCAPED_OPEN_TAG: &str = "&lt;event";
        const ESCAPED_OPEN_END: &str = "&gt;";
        const ESCAPED_CLOSE_TAG: &str = "&lt;/event&gt;";
        const ESCAPED_CLOSE_TAG_JSON_STYLE: &str = "&lt;\\/event&gt;";

        if backend.command != "codex" {
            return None;
        }

        let trimmed = output.trim_start_matches(|ch: char| ch.is_whitespace());
        if !trimmed.starts_with(ESCAPED_OPEN_TAG) {
            return None;
        }

        let leading_whitespace_len = output.len() - trimmed.len();
        let mut remaining = trimmed;
        let mut normalized = String::with_capacity(output.len());
        normalized.push_str(&output[..leading_whitespace_len]);
        let mut converted_any = false;

        loop {
            if !remaining.starts_with(ESCAPED_OPEN_TAG) {
                break;
            }

            let Some(open_end_idx) = remaining.find(ESCAPED_OPEN_END) else {
                break;
            };
            let opening_attrs = &remaining[ESCAPED_OPEN_TAG.len()..open_end_idx];
            let content_start = &remaining[open_end_idx + ESCAPED_OPEN_END.len()..];

            let standard_close = content_start
                .find(ESCAPED_CLOSE_TAG)
                .map(|idx| (idx, ESCAPED_CLOSE_TAG, "</event>"));
            let json_style_close = content_start
                .find(ESCAPED_CLOSE_TAG_JSON_STYLE)
                .map(|idx| (idx, ESCAPED_CLOSE_TAG_JSON_STYLE, "<\\/event>"));

            let Some((close_idx, close_raw, close_normalized)) =
                (match (standard_close, json_style_close) {
                    (Some(left), Some(right)) => Some(if left.0 <= right.0 { left } else { right }),
                    (Some(found), None) | (None, Some(found)) => Some(found),
                    (None, None) => None,
                })
            else {
                break;
            };

            normalized.push_str("<event");
            normalized.push_str(opening_attrs);
            normalized.push('>');
            normalized.push_str(&content_start[..close_idx]);
            normalized.push_str(close_normalized);
            remaining = &content_start[close_idx + close_raw.len()..];
            converted_any = true;

            // 只继续吞连续的 leading escaped event block。
            // 一旦进入普通 prose，就立即停下，避免把正文里引用的示例也“扶正”为真实事件。
            let next_trimmed = remaining.trim_start_matches(|ch: char| ch.is_whitespace());
            let whitespace_len = remaining.len() - next_trimmed.len();
            normalized.push_str(&remaining[..whitespace_len]);
            remaining = next_trimmed;
            if !remaining.starts_with(ESCAPED_OPEN_TAG) {
                break;
            }
        }

        if !converted_any {
            return None;
        }

        normalized.push_str(remaining);
        Some(normalized)
    }
    async fn handle_output_line(
        job_id: u64,
        instance_id: &HatInstanceId,
        output_tx: &tokio::sync::mpsc::Sender<HatJobOutputChunk>,
        backend: &CliBackend,
        stdout_output: &mut String,
        stderr_output: &mut String,
        stream: OutputStream,
        line: String,
    ) {
        match stream {
            OutputStream::Stdout => {
                stdout_output.push_str(&line);
                stdout_output.push('\n');
            }
            OutputStream::Stderr => {
                stderr_output.push_str(&line);
                stderr_output.push('\n');
            }
            OutputStream::Activity => {}
        }

        // JSON backend 的 stdout 是结构化负载, 不适合逐行原样透传.
        // 我们先缓存,等进程结束后再提取稳定正文并一次性发给上层.
        if backend.emits_structured_response() && stream == OutputStream::Stdout {
            return;
        }

        // 1) 流式输出给 Supervisor（带 instance_id 归因）
        let _ = output_tx
            .send(HatJobOutputChunk {
                job_id,
                instance_id: instance_id.clone(),
                stream,
                line: line.clone(),
            })
            .await;

        // 2) 组装完整 output（用于 event parser）
        match stream {
            OutputStream::Stdout => {
                // 普通文本 backend: stdout 已在上面缓存,这里无需重复拼接。
            }
            OutputStream::Stderr => {
                // 重要：并行模式下，stderr 往往包含“后端自身的日志”（例如 Codex 会回显 user prompt、
                // MCP 启动日志、warnings 等）。这些内容可能包含 `<event ...>` 字样（来自 prompt 本身），
                // 如果把 stderr 拼进 `output`，会导致 EventParser 把“输入/日志”误判为“已发出事件”，
                // 从而造成重复路由、假阳性 completion、E2E 波动等问题。
                //
                // 因此：
                // - stderr 仍然会通过 `output_tx` 传给 Supervisor 做可观测输出（`[hat#n:err] ...`）
                // - 但不会进入 `HatJobResult.output`，以保证 event parsing 只基于 stdout（模型最终输出）
            }
            OutputStream::Activity => {
                // Activity 是纯状态信号,不参与 event parsing。
            }
        }
    }
    async fn emit_final_structured_output(
        job_id: u64,
        instance_id: &HatInstanceId,
        output_tx: &tokio::sync::mpsc::Sender<HatJobOutputChunk>,
        display_output: &str,
    ) {
        for line in display_output.lines() {
            let _ = output_tx
                .send(HatJobOutputChunk {
                    job_id,
                    instance_id: instance_id.clone(),
                    stream: OutputStream::Stdout,
                    line: line.to_string(),
                })
                .await;
        }
    }
    async fn terminate_child(
        child: &mut tokio::process::Child,
        reason: &str,
    ) -> std::io::Result<()> {
        #[cfg(unix)]
        {
            use nix::sys::signal::{Signal, killpg};
            use nix::unistd::Pid;

            if let Some(pid) = child.id() {
                #[allow(clippy::cast_possible_wrap)]
                let pid = Pid::from_raw(pid as i32);
                // 注意: 我们在 spawn 时已通过 `process_group(0)` 让该 pid 成为进程组 leader.
                // 因此这里用 killpg 终止整组(包含后端派生的子进程).
                let _ = killpg(pid, Signal::SIGTERM);
            }
        }

        #[cfg(not(unix))]
        {
            let _ = child.start_kill();
        }

        // grace period
        let wait_res = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
        match wait_res {
            Ok(Ok(_status)) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(_) => {
                warn!(%reason, "Job did not exit after SIGTERM, forcing kill");
                #[cfg(unix)]
                {
                    use nix::sys::signal::{Signal, killpg};
                    use nix::unistd::Pid;
                    if let Some(pid) = child.id() {
                        #[allow(clippy::cast_possible_wrap)]
                        let pid = Pid::from_raw(pid as i32);
                        let _ = killpg(pid, Signal::SIGKILL);
                    }
                }
                #[cfg(not(unix))]
                {
                    let _ = child.start_kill();
                }
                let _ = child.wait().await;
                Ok(())
            }
        }
    }


#[cfg(test)]
mod tests {
    use super::*;
    use ralph_core::EventParser;

    #[test]
    fn finalize_output_for_parsing_keeps_text_backend_stdout_only() {
        let backend = CliBackend::codex();
        let stdout = "<event topic=\"spec.ready\">ok</event>\n";

        let output = finalize_output_for_parsing(&backend, stdout);

        assert_eq!(output, stdout);
    }


    #[test]
    fn finalize_output_for_parsing_extracts_structured_stdout_only() {
        let backend = CliBackend::gemini();
        let stdout = r#"{"response":"<event topic=\"spec.ready\">ok</event>"}"#;

        let output = finalize_output_for_parsing(&backend, stdout);

        assert_eq!(output, r#"<event topic="spec.ready">ok</event>"#);
    }


    #[test]
    fn finalize_output_for_parsing_normalizes_leading_escaped_codex_event_block() {
        let backend = CliBackend::codex();
        let stdout = concat!(
            "&lt;event topic=\"experiment.result\" reply=\"E1\"&gt;\n",
            "comparison: 2 &gt; 1\n",
            "&lt;/event&gt;\n",
            "- trailing prose stays visible\n",
        );

        let output = finalize_output_for_parsing(&backend, stdout);

        assert_eq!(
            output,
            concat!(
                "<event topic=\"experiment.result\" reply=\"E1\">\n",
                "comparison: 2 &gt; 1\n",
                "</event>\n",
                "- trailing prose stays visible\n",
            )
        );
    }


    #[test]
    fn finalize_output_for_parsing_does_not_normalize_escaped_event_after_prose() {
        let backend = CliBackend::codex();
        let stdout = concat!(
            "Here is an example event block:\n",
            "&lt;event topic=\"experiment.result\"&gt;demo&lt;/event&gt;\n",
        );

        let output = finalize_output_for_parsing(&backend, stdout);

        assert_eq!(output, stdout);
    }


    async fn parallel_output_for_event_parsing_is_stdout_only() {
        // ------------------------------------------------------------------
        // 目标:
        // - 锁死 parallel 模式的关键不变量: EventParser 的输入必须是 stdout-only.
        // - stderr 可能包含 prompt transcript / MCP 日志 / warnings 等,它们经常含 `<event ...>` 字样。
        //   一旦把 stderr 拼进 `HatJobResult.output`,就会产生“假事件/假 completion/重复路由”的回归。
        // ------------------------------------------------------------------

        let (output_tx, mut output_rx) = tokio::sync::mpsc::channel::<HatJobOutputChunk>(8);
        let instance_id = HatInstanceId::from("writer#1");
        let job_id = 42_u64;
        let backend = CliBackend::kiro();
        let mut stdout_output = String::new();
        let mut stderr_output = String::new();

        // 1) stdout: 必须进入 output(用于事件解析).
        let stdout_line = "<event topic=\"build.done\">ok</event>".to_string();
        handle_output_line(
            job_id,
            &instance_id,
            &output_tx,
            &backend,
            &mut stdout_output,
            &mut stderr_output,
            OutputStream::Stdout,
            stdout_line.clone(),
        )
        .await;

        assert_eq!(stdout_output, format!("{stdout_line}\n"));
        let chunk = output_rx.recv().await.expect("should receive stdout chunk");
        assert_eq!(chunk.job_id, job_id);
        assert_eq!(chunk.instance_id, instance_id);
        assert_eq!(chunk.stream, OutputStream::Stdout);
        assert_eq!(chunk.line, stdout_line);

        let parser = ralph_core::EventParser::new();
        let events = parser.parse(&finalize_output_for_parsing(
            &backend,
            &stdout_output,
        ));
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].topic.as_str(), "build.done");
        assert_eq!(events[0].payload, "ok");

        // 2) stderr: 仍要流式转发给 supervisor 做可观测输出,但绝不能污染 output.
        let stderr_line = "<event topic=\"build.task\">should_not_parse</event>".to_string();
        handle_output_line(
            job_id,
            &instance_id,
            &output_tx,
            &backend,
            &mut stdout_output,
            &mut stderr_output,
            OutputStream::Stderr,
            stderr_line.clone(),
        )
        .await;

        assert_eq!(
            stdout_output, "<event topic=\"build.done\">ok</event>\n",
            "REGRESSION: stderr must NOT be appended to output used for event parsing"
        );
        assert_eq!(stderr_output, format!("{stderr_line}\n"));
        let chunk = output_rx.recv().await.expect("should receive stderr chunk");
        assert_eq!(chunk.job_id, job_id);
        assert_eq!(chunk.instance_id, instance_id);
        assert_eq!(chunk.stream, OutputStream::Stderr);
        assert_eq!(chunk.line, stderr_line);

        let events = parser.parse(&finalize_output_for_parsing(
            &backend,
            &stdout_output,
        ));
        assert_eq!(
            events.len(),
            1,
            "REGRESSION: stderr event text must not create extra parsed events"
        );
        assert_eq!(events[0].topic.as_str(), "build.done");
    }
}
