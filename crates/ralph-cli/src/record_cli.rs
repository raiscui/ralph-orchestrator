//! `ralph record` 命令族(面向人类排障)。
//!
//! 说明:
//! - `summary` 走 strict parse,用于“已完成”的 JSONL 证据文件。
//! - `watch` 默认 raw follow,用于“增长中”的 JSONL,像 `tail -f` 一样输出新增完整行。

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use ralph_core::AgentsSnapshot;
use serde_json::Value;
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Parser, Debug)]
pub(crate) struct RecordArgs {
    #[command(subcommand)]
    command: RecordCommand,
}

#[derive(Subcommand, Debug)]
enum RecordCommand {
    /// Strict parse and print a human-readable summary for a record-session JSONL
    Summary(RecordSummaryArgs),
    /// Follow a growing record-session JSONL and print newly appended complete lines
    Watch(RecordWatchArgs),
}

#[derive(Parser, Debug)]
struct RecordSummaryArgs {
    /// Path to record-session JSONL file
    #[arg(value_name = "FILE")]
    file: PathBuf,

    /// Optional agents snapshot sidecar. Defaults to `<workspace_root>/.ralph/agents.json`.
    #[arg(long, value_name = "FILE")]
    agents_file: Option<PathBuf>,
}

#[derive(Parser, Debug)]
struct RecordWatchArgs {
    /// Path to record-session JSONL file. If omitted, use `.ralph/record-session.latest`.
    #[arg(value_name = "FILE")]
    file: Option<PathBuf>,

    /// Poll interval in milliseconds
    #[arg(long, value_name = "MS", default_value_t = 250)]
    interval_ms: u64,

    /// Print from start of file (default: follow from current EOF)
    #[arg(long)]
    from_start: bool,

    /// Exit once a record with this `event` is observed (agent automation).
    ///
    /// Examples:
    /// - `_meta.termination`
    /// - `bus.publish`
    #[arg(long, value_name = "EVENT")]
    until_event: Option<String>,

    /// Exit once a `bus.publish` record with this `data.topic` is observed (agent automation).
    ///
    /// Example:
    /// - `reply.human.message`
    #[arg(long, value_name = "TOPIC")]
    until_topic: Option<String>,

    /// Timeout in seconds (requires --until-event or --until-topic).
    ///
    /// Exit code:
    /// - 0: condition satisfied
    /// - 2: timed out
    #[arg(long, value_name = "SECS")]
    timeout_secs: Option<u64>,

    /// Do not print streamed lines (useful for scripting; rely on exit code).
    #[arg(short, long)]
    quiet: bool,
}

pub(crate) async fn execute(args: RecordArgs) -> Result<()> {
    match args.command {
        RecordCommand::Summary(args) => summary_command(args),
        RecordCommand::Watch(args) => watch_command(args).await,
    }
}

fn summary_command(args: RecordSummaryArgs) -> Result<()> {
    let agg = ralph_core::aggregate_session(&args.file)?;
    let agents_probe = load_agents_snapshot_for_summary(&args, &agg);

    // ------------------------------------------------------------------
    // 输出策略:
    // - 尽量短,但信息密度高,便于“扫一眼”.
    // - 不追求稳定机器格式(若你后续需要,我们可以另加 `--json`).
    // ------------------------------------------------------------------
    let mut out = std::io::stdout().lock();

    writeln!(out, "Record Summary")?;
    writeln!(out, "  file: {}", args.file.display())?;

    if let Some(meta) = &agg.session_start {
        writeln!(out, "Meta")?;
        if let Some(cwd) = meta.cwd.as_deref() {
            writeln!(out, "  cwd: {cwd}")?;
        }
        if let Some(root) = meta.workspace_root.as_deref() {
            writeln!(out, "  workspace_root: {root}")?;
        }
        if let Some(argv_joined) = meta.argv_joined.as_deref() {
            writeln!(out, "  argv_joined: {argv_joined}")?;
        } else if !meta.argv.is_empty() {
            writeln!(out, "  argv: {}", meta.argv.join(" "))?;
        }
        writeln!(out, "  pid: {}", meta.pid)?;
        if let Some(current_exe) = meta.current_exe.as_deref() {
            writeln!(out, "  current_exe: {current_exe}")?;
        }
        if let Some(version) = meta.version.as_deref() {
            writeln!(out, "  version: {version}")?;
        }
    }

    if let Some(loop_start) = &agg.loop_start {
        writeln!(out, "Loop")?;
        writeln!(out, "  prompt_file: {}", loop_start.prompt_file)?;
        writeln!(out, "  max_iterations: {}", loop_start.max_iterations)?;
        writeln!(out, "  ux_mode: {}", loop_start.ux_mode)?;
    }

    writeln!(out, "Termination")?;
    match agg.termination.as_ref().and_then(|t| t.reason.as_deref()) {
        Some(reason) => writeln!(out, "  reason: {reason}")?,
        None => writeln!(out, "  reason: <missing>")?,
    }

    // topic top N
    let mut pairs = agg.topic_counts.iter().collect::<Vec<_>>();
    pairs.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));

    writeln!(out, "Topics (top 10)")?;
    for (i, (topic, count)) in pairs.into_iter().take(10).enumerate() {
        writeln!(out, "  {:>2}. {}: {}", i + 1, topic, count)?;
    }

    let evidence_text = match &agents_probe {
        AgentsSnapshotProbe::Loaded { path, snapshot } => {
            let path = path.display().to_string();
            crate::record_session::render_evidence_inspect(
                &agg,
                crate::record_session::AgentsSnapshotInspect::Loaded {
                    path: &path,
                    snapshot,
                },
            )?
        }
        AgentsSnapshotProbe::Missing { searched } => {
            crate::record_session::render_evidence_inspect(
                &agg,
                crate::record_session::AgentsSnapshotInspect::Missing {
                    searched: searched
                        .iter()
                        .map(|path| path.display().to_string())
                        .collect(),
                },
            )?
        }
        AgentsSnapshotProbe::Invalid { path, error } => {
            let path = path.display().to_string();
            crate::record_session::render_evidence_inspect(
                &agg,
                crate::record_session::AgentsSnapshotInspect::Invalid { path: &path, error },
            )?
        }
    };
    write!(out, "{evidence_text}")?;

    // stdout tail
    writeln!(out, "Stdout Tail")?;
    if agg.stdout_tail.trim().is_empty() {
        writeln!(out, "  <empty>")?;
    } else {
        // tail 可能含多行,这里不额外缩进,避免破坏原始对齐.
        writeln!(out, "{}", agg.stdout_tail.trim_end())?;
    }

    Ok(())
}

#[derive(Debug)]
enum AgentsSnapshotProbe {
    Loaded {
        path: PathBuf,
        snapshot: AgentsSnapshot,
    },
    Missing {
        searched: Vec<PathBuf>,
    },
    Invalid {
        path: PathBuf,
        error: String,
    },
}

fn load_agents_snapshot_for_summary(
    args: &RecordSummaryArgs,
    agg: &ralph_core::RecordSessionAggregate,
) -> AgentsSnapshotProbe {
    let candidates = agents_snapshot_candidates(args, agg);

    for path in &candidates {
        if !path.exists() {
            continue;
        }

        let content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(error) => {
                return AgentsSnapshotProbe::Invalid {
                    path: path.clone(),
                    error: format!("failed to read: {error}"),
                };
            }
        };

        match serde_json::from_str::<AgentsSnapshot>(&content) {
            Ok(snapshot) => {
                return AgentsSnapshotProbe::Loaded {
                    path: path.clone(),
                    snapshot,
                };
            }
            Err(error) => {
                return AgentsSnapshotProbe::Invalid {
                    path: path.clone(),
                    error: format!("invalid JSON: {error}"),
                };
            }
        }
    }

    AgentsSnapshotProbe::Missing {
        searched: candidates,
    }
}

fn agents_snapshot_candidates(
    args: &RecordSummaryArgs,
    agg: &ralph_core::RecordSessionAggregate,
) -> Vec<PathBuf> {
    if let Some(path) = &args.agents_file {
        return vec![path.clone()];
    }

    let mut candidates = Vec::new();

    if let Some(root) = agg
        .session_start
        .as_ref()
        .and_then(|meta| meta.workspace_root.as_deref())
        .filter(|root| !root.trim().is_empty())
    {
        candidates.push(PathBuf::from(root).join(".ralph/agents.json"));
    }

    if let Some(cwd) = agg
        .session_start
        .as_ref()
        .and_then(|meta| meta.cwd.as_deref())
        .filter(|cwd| !cwd.trim().is_empty())
    {
        candidates.push(PathBuf::from(cwd).join(".ralph/agents.json"));
    }

    if let Some(found) = crate::find_file_in_parents(".ralph/agents.json") {
        candidates.push(found);
    }

    // record-session 可能位于 workspace 根目录或临时目录。这里作为最后兜底,
    // 只用于给 missing message 提供搜索路径,不把它当成强绑定真相源。
    if let Some(parent) = args.file.parent() {
        candidates.push(parent.join(".ralph/agents.json"));
    }

    dedup_paths(candidates)
}

fn dedup_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();

    for path in paths {
        let key = path.to_string_lossy().to_string();
        if seen.insert(key) {
            out.push(path);
        }
    }

    out
}

async fn watch_command(args: RecordWatchArgs) -> Result<()> {
    let file = match args.file.as_ref() {
        Some(path) => path.clone(),
        None => resolve_record_session_latest_pointer_in_parents()?,
    };

    // ------------------------------------------------------------------
    // 参数约束:
    // - timeout 的语义是 “在等待某个证据出现时,最多等多久”.
    // - 因此当用户没指定任何 until 条件时,timeout 会变得语义不清晰(永远都会超时).
    // - 这里 fail-fast,避免误用.
    // ------------------------------------------------------------------
    let has_until = args.until_event.is_some() || args.until_topic.is_some();
    if args.timeout_secs.is_some() && !has_until {
        anyhow::bail!("--timeout-secs requires --until-event or --until-topic");
    }

    let interval = Duration::from_millis(args.interval_ms.max(10));
    let timeout = args.timeout_secs.map(Duration::from_secs);

    let outcome = raw_follow_jsonl(&file, args.from_start, interval, &args, timeout).await?;
    if outcome == RecordWatchOutcome::TimedOut {
        // 说明:
        // - 这是专门给“编程智能体自动化”用的退出码.
        // - 让上层脚本能用 exit code 直接判断是否命中证据,而不是解析文本.
        std::process::exit(2);
    }
    Ok(())
}

fn resolve_record_session_latest_pointer_in_parents() -> Result<PathBuf> {
    let Some(pointer_path) = crate::find_file_in_parents(".ralph/record-session.latest") else {
        anyhow::bail!(
            "No `.ralph/record-session.latest` found in parent directories. Pass FILE explicitly, or run `ralph run --record-session <FILE>` first."
        );
    };

    let raw = std::fs::read_to_string(&pointer_path)
        .with_context(|| format!("Failed to read pointer: {}", pointer_path.display()))?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        anyhow::bail!(
            "Pointer file is empty: {}. Re-run `ralph run --record-session <FILE>`.",
            pointer_path.display()
        );
    }

    let configured = PathBuf::from(trimmed);
    if configured.is_absolute() {
        return Ok(configured);
    }

    // pointer 文件位于 `<workspace_root>/.ralph/record-session.latest`.
    let workspace_root = pointer_path
        .parent()
        .and_then(|p| p.parent())
        .context("Invalid pointer path layout (expected <root>/.ralph/record-session.latest)")?;
    Ok(workspace_root.join(configured))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecordWatchOutcome {
    Completed,
    TimedOut,
}

async fn raw_follow_jsonl(
    path: &Path,
    from_start: bool,
    interval: Duration,
    args: &RecordWatchArgs,
    timeout: Option<Duration>,
) -> Result<RecordWatchOutcome> {
    // ------------------------------------------------------------------
    // 语义:
    // - 只输出“新增的完整行”(以 '\n' 结尾)。
    // - 末尾半行(无 '\n')留在 buffer,等待后续补齐,不得提前输出。
    // - I/O 抖动/短暂错误: 打 stderr 并继续 follow,不要退出。
    // ------------------------------------------------------------------
    let started_at = Instant::now();
    let mut offset = if from_start {
        0_u64
    } else {
        std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
    };
    let mut partial: Vec<u8> = Vec::new();

    loop {
        if let Some(timeout) = timeout
            && started_at.elapsed() >= timeout
        {
            if !args.quiet {
                eprintln!(
                    "[record watch] timed out after {:?}: file={}",
                    timeout,
                    path.display()
                );
            }
            return Ok(RecordWatchOutcome::TimedOut);
        }

        // 文件可能还没创建(例如你先 watch,再启动 run),这里 best-effort 重试.
        let mut file = match std::fs::File::open(path) {
            Ok(f) => f,
            Err(e) => {
                if !args.quiet {
                    eprintln!("[record watch] open failed: {}: {e}", path.display());
                }
                tokio::time::sleep(interval).await;
                continue;
            }
        };

        let len = match file.metadata() {
            Ok(m) => m.len(),
            Err(e) => {
                if !args.quiet {
                    eprintln!("[record watch] stat failed: {}: {e}", path.display());
                }
                tokio::time::sleep(interval).await;
                continue;
            }
        };
        if len < offset {
            // 文件被截断/重建: 重置 offset 与 partial.
            offset = 0;
            partial.clear();
        }

        if let Err(e) = file.seek(SeekFrom::Start(offset)) {
            if !args.quiet {
                eprintln!(
                    "[record watch] seek failed: {} offset={offset}: {e}",
                    path.display()
                );
            }
            tokio::time::sleep(interval).await;
            continue;
        }

        let mut buf = Vec::new();
        match std::io::Read::read_to_end(&mut file, &mut buf) {
            Ok(_) => {}
            Err(e) => {
                if !args.quiet {
                    eprintln!("[record watch] read failed: {}: {e}", path.display());
                }
                tokio::time::sleep(interval).await;
                continue;
            }
        }

        if !buf.is_empty() {
            offset = offset.saturating_add(buf.len() as u64);
            partial.extend_from_slice(&buf);

            // 输出/匹配完整行.
            //
            // 说明:
            // - 默认 raw follow: 原样输出.
            // - 若指定了 until 条件: 额外做 best-effort JSON parse 用于匹配,不影响原样输出语义.
            let mut stdout = std::io::stdout().lock();
            while let Some(pos) = partial.iter().position(|b| *b == b'\n') {
                let line = partial.drain(..=pos).collect::<Vec<_>>();

                if !args.quiet {
                    let _ = stdout.write_all(&line);
                }

                if should_stop_after_line(&line, args)? {
                    let _ = stdout.flush();
                    return Ok(RecordWatchOutcome::Completed);
                }
            }

            if !args.quiet {
                let _ = stdout.flush();
            }
        }

        tokio::time::sleep(interval).await;
    }
}

fn should_stop_after_line(line: &[u8], args: &RecordWatchArgs) -> Result<bool> {
    let until_event = args.until_event.as_deref();
    let until_topic = args.until_topic.as_deref();

    if until_event.is_none() && until_topic.is_none() {
        return Ok(false);
    }

    // best-effort: JSONL 每行应当是 JSON object.
    let v: Value = match serde_json::from_slice(line) {
        Ok(v) => v,
        Err(_) => return Ok(false),
    };

    let event = v.get("event").and_then(Value::as_str);
    if let Some(expected) = until_event
        && event == Some(expected)
    {
        return Ok(true);
    }

    if let Some(expected) = until_topic
        && event == Some("bus.publish")
    {
        let topic = v
            .get("data")
            .and_then(|d| d.get("topic"))
            .and_then(Value::as_str);
        if topic == Some(expected) {
            return Ok(true);
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_follow_split_does_not_emit_incomplete_line() {
        // 说明:
        // - watch 的核心不变量: 只输出完整行(以 '\n' 结尾).
        // - 末尾半行必须留在 buffer,等待后续补齐.
        let mut partial = Vec::<u8>::new();
        let mut out = Vec::<Vec<u8>>::new();

        feed_bytes(&mut partial, b"{\"a\":1}\n{\"b\":2", &mut out);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0], b"{\"a\":1}\n");
        assert_eq!(partial, b"{\"b\":2");

        feed_bytes(&mut partial, b"}\n", &mut out);
        assert_eq!(out.len(), 2);
        assert_eq!(out[1], b"{\"b\":2}\n");
        assert!(partial.is_empty());
    }

    fn feed_bytes(partial: &mut Vec<u8>, bytes: &[u8], out: &mut Vec<Vec<u8>>) {
        partial.extend_from_slice(bytes);
        while let Some(pos) = partial.iter().position(|b| *b == b'\n') {
            out.push(partial.drain(..=pos).collect::<Vec<_>>());
        }
    }

    #[test]
    fn should_stop_after_line_matches_until_event() -> Result<()> {
        let args = RecordWatchArgs {
            file: None,
            interval_ms: 250,
            from_start: true,
            until_event: Some("_meta.loop_start".to_string()),
            until_topic: None,
            timeout_secs: None,
            quiet: true,
        };

        let line = br#"{"ts":1,"event":"_meta.loop_start","data":{"max_iterations":1,"prompt_file":"PROMPT.md","ux_mode":"cli"}}
"#;
        assert!(should_stop_after_line(line, &args)?);
        Ok(())
    }

    #[test]
    fn should_stop_after_line_matches_until_topic() -> Result<()> {
        let args = RecordWatchArgs {
            file: None,
            interval_ms: 250,
            from_start: true,
            until_event: None,
            until_topic: Some("reply.human.message".to_string()),
            timeout_secs: None,
            quiet: true,
        };

        let line = br#"{"ts":1,"event":"bus.publish","data":{"topic":"reply.human.message","payload":"hi","id":"x","source":null,"target":null}}
"#;
        assert!(should_stop_after_line(line, &args)?);
        Ok(())
    }
}
