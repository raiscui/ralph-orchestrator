//! 并行 HatInstance 运行器（ralph-cli 侧）。
//!
//! 说明：
//! - 该模块把“调度/路由”交给 `ralph-core::ParallelSupervisor`。
//! - 这里实现 `HatJobExecutor`：spawn 外部 headless CLI 进程，流式采集 stdout/stderr。

use anyhow::{Context, Result};
use ralph_adapters::{
    job::{CliHatJobExecutor, CodexAppServerRuntime, CodexMcpRuntime},
    CliBackend,
};
use ralph_core::{
    CapabilityParentFailedRecord, CapabilityParentResultRecord, CapabilityRequestRecord,
    EventLogger, EvidenceIndexWriter, HatRegistry, ParallelSupervisor, RalphConfig, Record,
    SessionRecorder, TOPIC_CAPABILITY_FAILED, TOPIC_CAPABILITY_REQUEST,
    TOPIC_CAPABILITY_RESULT, TOPIC_TOPOLOGY_SPAWN_FAILED, TOPIC_TOPOLOGY_SPAWN_GROUP,
    TOPIC_TOPOLOGY_SPAWN_RESULT, TerminationReason, TopologySpawnGroupFailed,
    TopologySpawnGroupRequest, TopologySpawnGroupResult,
};
use ralph_core::{
    HatJobOutputChunk, OutputStream,
};
use ralph_proto::{HatInstanceId, HatInstanceState, TerminalWrite, UxEvent};
use ralph_tui::{Tui, TuiUpdate};
use std::fs::File;
use std::future::Future;
use std::io::{IsTerminal, Write, stdin, stdout};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::watch;
use tracing::{debug, warn};

use crate::display::colors;
use crate::process_management;
use crate::runtime_graph::RuntimeGraphRecorder;
use crate::{ColorMode, Verbosity};

const PARALLEL_RUNTIME_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(15);

fn should_forward_event_to_tui(event: &ralph_proto::Event) -> bool {
    // 事件转发策略（并行模式）：
    // - gate.* / human.message / reply.human.message：用于控制面 UI（Gate 面板/提示等）；
    // - source_instance 存在：用于运行态可视化（Hat Graph Radar 边动画可据此推导发布者 hat）。
    let topic = event.topic.as_str();
    topic.starts_with("gate.")
        || topic.starts_with("capability.")
        || topic.starts_with("topology.")
        || topic == "human.message"
        || topic == "reply.human.message"
        || event.source_instance.is_some()
        || event.source.is_some()
}

fn runtime_capability_wiring_enabled(config: &RalphConfig) -> bool {
    config.core.runtime_capabilities_enabled
}

fn write_parallel_cli_line<W: Write>(out: &mut W, line: &str) {
    // ------------------------------------------------------------------
    // 说明:
    // - 并行 CLI/log-mode 的 stdout 常被 E2E 父进程、`tee`、shell pipe 直接消费。
    // - 这类场景下 stdout 往往不是 TTY,标准库会做块缓冲。
    // - 如果 run 在 cleanup 阶段卡住并最终被外层 SIGTERM 杀掉,未 flush 的尾部日志会丢失:
    //   - job 计数断言失真
    //   - LOOP_COMPLETE 看起来“没有输出”
    // - 因此这里把“写一行”定义为“写入 + 立即 flush”的耐久性操作。
    // ------------------------------------------------------------------
    let _ = writeln!(out, "{line}");
    let _ = out.flush();
}

fn parallel_cli_event_summary(event: &ralph_proto::Event) -> Option<String> {
    let topic = event.topic.as_str();
    match topic {
        TOPIC_TOPOLOGY_SPAWN_GROUP => {
            let request = TopologySpawnGroupRequest::parse_payload(&event.payload).ok()?;
            Some(format!(
                "[supervisor:event] topology.spawn_group request_id={} hat={} delivery_topic={} requested_instances={}",
                request.request_id,
                request.hat,
                request.delivery_topic,
                request.instances.len()
            ))
        }
        TOPIC_TOPOLOGY_SPAWN_RESULT => {
            let result = serde_json::from_str::<TopologySpawnGroupResult>(&event.payload).ok()?;
            let spawned = result
                .spawned
                .iter()
                .map(|item| {
                    let fixed = if item.fixed_role == Some(true) {
                        ",fixed"
                    } else {
                        ""
                    };
                    let contract = item
                        .role_contract_summary
                        .as_ref()
                        .map(|summary| {
                            format!(
                                ",identity_source={},persistence={},contract_schema_version={},role_contract_hash={},source_spawn_request_id={}",
                                summary.identity_source,
                                summary.persistence,
                                summary.contract_schema_version,
                                short_hash(&summary.role_contract_hash),
                                summary.source_spawn_request_id
                            )
                        })
                        .unwrap_or_default();
                    format!("{}:{}{}{}", item.instance_id, item.role, fixed, contract)
                })
                .collect::<Vec<_>>()
                .join(",");
            Some(format!(
                "[supervisor:event] topology.spawn.result request_id={} status={} parent_topology_unchanged={} spawned=[{}] failed={}",
                result.request_id,
                result.status,
                result.parent_topology_unchanged,
                if spawned.is_empty() {
                    "-".to_string()
                } else {
                    spawned
                },
                result.failed.len()
            ))
        }
        TOPIC_TOPOLOGY_SPAWN_FAILED => {
            let failed = serde_json::from_str::<TopologySpawnGroupFailed>(&event.payload).ok()?;
            Some(format!(
                "[supervisor:event] topology.spawn.failed request_id={} hat={} parent_topology_unchanged={} error={}",
                failed.request_id.as_deref().unwrap_or("-"),
                failed.hat.as_deref().unwrap_or("-"),
                failed.parent_topology_unchanged,
                one_line(&failed.error)
            ))
        }
        TOPIC_CAPABILITY_REQUEST => {
            let request = CapabilityRequestRecord::parse_payload(&event.payload).ok()?;
            Some(format!(
                "[supervisor:event] capability.request request_id={} capability={} status=running parent_topology_unchanged=true",
                request.request_id, request.capability_id
            ))
        }
        TOPIC_CAPABILITY_RESULT => {
            let result =
                serde_json::from_str::<CapabilityParentResultRecord>(&event.payload).ok()?;
            Some(format!(
                "[supervisor:event] capability.result request_id={} invocation={} capability={} status=done parent_topology_unchanged={} summary={}",
                result.request_id,
                result.invocation_id,
                result.capability_id,
                result.parent_topology_unchanged,
                truncate_plain_summary(&result.result_summary)
            ))
        }
        TOPIC_CAPABILITY_FAILED => {
            let failed =
                serde_json::from_str::<CapabilityParentFailedRecord>(&event.payload).ok()?;
            Some(format!(
                "[supervisor:event] capability.failed request_id={} invocation={} capability={} status=failed class={} parent_topology_unchanged={} error={}",
                failed.request_id.as_deref().unwrap_or("-"),
                failed.invocation_id.as_deref().unwrap_or("-"),
                failed.capability_id.as_deref().unwrap_or("-"),
                failed.failure_class,
                failed.parent_topology_unchanged,
                truncate_plain_summary(&failed.error)
            ))
        }
        _ => None,
    }
}

fn truncate_plain_summary(value: &str) -> String {
    let one_line = one_line(value);
    ralph_core::truncate_with_ellipsis(&one_line, 96)
}

fn one_line(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn short_hash(value: &str) -> String {
    value.chars().take(12).collect()
}

fn maybe_parallel_cli_event_summary(
    event: &ralph_proto::Event,
    verbosity: Verbosity,
) -> Option<String> {
    if matches!(verbosity, Verbosity::Quiet) {
        return None;
    }

    parallel_cli_event_summary(event)
}

#[cfg(test)]
fn maybe_write_parallel_cli_event_summary<W: Write>(
    out: &mut W,
    event: &ralph_proto::Event,
    verbosity: Verbosity,
) {
    if let Some(summary) = maybe_parallel_cli_event_summary(event, verbosity) {
        write_parallel_cli_line(out, &summary);
    }
}

fn display_path_for_tui(workspace_root: &Path, path: &Path) -> String {
    let absolute_or_rooted = if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace_root.join(path)
    };

    if let Ok(relative) = absolute_or_rooted.strip_prefix(workspace_root) {
        let display = relative.display().to_string();
        if !display.is_empty() {
            return display;
        }
    }

    path.display().to_string()
}

fn current_events_path_for_tui(config: &RalphConfig) -> PathBuf {
    let marker_path = config.core.resolve_path(".ralph/current-events");
    let events_path = std::fs::read_to_string(&marker_path)
        .map(|value| value.trim().to_string())
        .unwrap_or_else(|_| EventLogger::DEFAULT_PATH.to_string());

    config.core.resolve_path(&events_path)
}

fn parallel_evidence_paths_for_tui(
    config: &RalphConfig,
    record_session_path: Option<&Path>,
) -> ralph_tui::state::ParallelEvidencePaths {
    let workspace_root = &config.core.workspace_root;
    let events_path = current_events_path_for_tui(config);
    let evidence_index_path = config.core.resolve_path(EvidenceIndexWriter::DEFAULT_PATH);
    let agents_snapshot_path = config.core.resolve_path(".ralph/agents.json");

    ralph_tui::state::ParallelEvidencePaths {
        events_path: Some(display_path_for_tui(workspace_root, &events_path)),
        evidence_index_path: Some(display_path_for_tui(workspace_root, &evidence_index_path)),
        agents_snapshot_path: Some(display_path_for_tui(workspace_root, &agents_snapshot_path)),
        record_session_path: record_session_path
            .map(|path| display_path_for_tui(workspace_root, path)),
    }
}

async fn shutdown_parallel_runtime_with_timeout<F>(
    runtime_name: &'static str,
    timeout: Duration,
    future: F,
) -> bool
where
    F: Future<Output = ()>,
{
    match tokio::time::timeout(timeout, future).await {
        Ok(()) => true,
        Err(_) => {
            warn!(
                runtime = runtime_name,
                timeout_secs = timeout.as_secs_f64(),
                "Parallel runtime shutdown timed out; continuing with process exit"
            );
            false
        }
    }
}

async fn shutdown_parallel_runtimes(
    codex_mcp_runtime: &CodexMcpRuntime,
    codex_app_server_runtime: &CodexAppServerRuntime,
) {
    // ------------------------------------------------------------------
    // 说明:
    // - `_meta.termination` 已经写出后,cleanup 就只剩 best-effort 语义。
    // - 如果这里无界等待,会出现“语义已完成,但 CLI 进程迟迟不退出”的假失败。
    // - 因此对 runtime shutdown 加有界超时:
    //   - 能正常收尾时,仍然完整收尾
    //   - 收尾异常卡住时,优先保证主进程退出与证据保留
    // ------------------------------------------------------------------
    let _ = shutdown_parallel_runtime_with_timeout(
        "codex_mcp_runtime",
        PARALLEL_RUNTIME_SHUTDOWN_TIMEOUT,
        codex_mcp_runtime.shutdown_all(),
    )
    .await;
    let _ = shutdown_parallel_runtime_with_timeout(
        "codex_app_server_runtime",
        PARALLEL_RUNTIME_SHUTDOWN_TIMEOUT,
        codex_app_server_runtime.shutdown_all(),
    )
    .await;
}

#[cfg(test)]
mod guardrail_tests {
    use super::*;
    use std::io;

    #[derive(Default)]
    struct CountingWriter {
        bytes: Vec<u8>,
        flushes: usize,
    }

    impl Write for CountingWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.bytes.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            self.flushes += 1;
            Ok(())
        }
    }

    #[test]
    fn write_parallel_cli_line_flushes_immediately() {
        let mut writer = CountingWriter::default();

        write_parallel_cli_line(&mut writer, "[writer#1:out:job=7] hello");

        assert_eq!(
            String::from_utf8(writer.bytes).expect("writer bytes should be utf8"),
            "[writer#1:out:job=7] hello\n"
        );
        assert_eq!(
            writer.flushes, 1,
            "parallel cli line writes must flush immediately in pipe/E2E mode"
        );
    }

    #[test]
    fn parallel_cli_event_summary_shows_topology_spawn_result() {
        let payload = serde_json::to_string(&TopologySpawnGroupResult {
            status: "spawned".to_string(),
            request_id: "spawn-001".to_string(),
            hat: "builder".to_string(),
            delivery_topic: "analysis.task".to_string(),
            spawned: vec![
                ralph_core::TopologySpawnedInstance {
                    index: 0,
                    instance_id: "builder#2".to_string(),
                    role: "功能补充".to_string(),
                    fixed_role: None,
                    role_contract_summary: Some(ralph_core::RoleContractSummary {
                        role_name: "功能补充".to_string(),
                        objective_preview: "补充 feature A".to_string(),
                        allowed_result_topics: vec!["analysis.done".to_string()],
                        identity_source: ralph_core::IdentitySource::TaskDerived,
                        persistence: ralph_core::RolePersistence::Temporary,
                        contract_schema_version: 1,
                        role_contract_hash: "erc-1234567890abcdef".to_string(),
                        source_spawn_request_id: "spawn-001".to_string(),
                    }),
                },
                ralph_core::TopologySpawnedInstance {
                    index: 1,
                    instance_id: "builder#3".to_string(),
                    role: "review".to_string(),
                    fixed_role: Some(true),
                    role_contract_summary: Some(ralph_core::RoleContractSummary {
                        role_name: "review".to_string(),
                        objective_preview: "review".to_string(),
                        allowed_result_topics: vec!["analysis.done".to_string()],
                        identity_source: ralph_core::IdentitySource::TaskDerived,
                        persistence: ralph_core::RolePersistence::Fixed,
                        contract_schema_version: 1,
                        role_contract_hash: "erc-fedcba0987654321".to_string(),
                        source_spawn_request_id: "spawn-001".to_string(),
                    }),
                },
            ],
            failed: Vec::new(),
            parent_topology_unchanged: false,
        })
        .expect("topology spawn result should serialize");
        let event = ralph_proto::Event::new(TOPIC_TOPOLOGY_SPAWN_RESULT, payload);

        let summary = parallel_cli_event_summary(&event)
            .expect("topology.spawn.result should produce a plain summary");

        assert!(summary.contains("topology.spawn.result"));
        assert!(summary.contains("parent_topology_unchanged=false"));
        assert!(summary.contains("builder#2:功能补充"));
        assert!(summary.contains("builder#3:review,fixed"));
        assert!(summary.contains("identity_source=task-derived"));
        assert!(summary.contains("persistence=temporary"));
        assert!(summary.contains("contract_schema_version=1"));
        assert!(summary.contains("role_contract_hash=erc-12345678"));
        assert!(summary.contains("source_spawn_request_id=spawn-001"));
    }

    #[test]
    fn parallel_cli_event_summary_shows_capability_result() {
        let payload = serde_json::to_string(&CapabilityParentResultRecord {
            status: "done".to_string(),
            request_id: "cap-001".to_string(),
            invocation_id: "invoke-001".to_string(),
            capability_id: "workflow:default-parallel".to_string(),
            result_summary: "worker completed\nwith useful result".to_string(),
            artifacts: ralph_core::CapabilityParentArtifactPaths {
                invoke_json: ".ralph/capabilities/invoke.json".to_string(),
                result_json: Some(".ralph/capabilities/result.json".to_string()),
                failed_json: None,
                resolved_config: ".ralph/capabilities/resolved-config.yml".to_string(),
                events_jsonl: ".ralph/capabilities/events.jsonl".to_string(),
                evidence_index: ".ralph/capabilities/evidence-index.jsonl".to_string(),
            },
            parent_topology_unchanged: true,
        })
        .expect("capability result should serialize");
        let event = ralph_proto::Event::new(TOPIC_CAPABILITY_RESULT, payload);

        let summary = parallel_cli_event_summary(&event)
            .expect("capability.result should produce a plain summary");

        assert!(summary.contains("capability.result"));
        assert!(summary.contains("status=done"));
        assert!(summary.contains("parent_topology_unchanged=true"));
        assert!(summary.contains("workflow:default-parallel"));
        assert!(summary.contains("worker completed with useful result"));
    }

    #[test]
    fn parallel_cli_event_summary_shows_capability_failed_class() {
        let payload = serde_json::to_string(&CapabilityParentFailedRecord {
            status: "failed".to_string(),
            failure_class: ralph_core::CapabilityFailureClass::ChildRunFailed,
            request_id: Some("cap-002".to_string()),
            invocation_id: Some("invoke-002".to_string()),
            capability_id: Some("workflow:default-parallel".to_string()),
            error: "child run failed\nsee evidence".to_string(),
            artifacts: None,
            parent_topology_unchanged: true,
        })
        .expect("capability failed should serialize");
        let event = ralph_proto::Event::new(TOPIC_CAPABILITY_FAILED, payload);

        let summary = parallel_cli_event_summary(&event)
            .expect("capability.failed should produce a plain summary");

        assert!(summary.contains("capability.failed"));
        assert!(summary.contains("status=failed"));
        assert!(summary.contains("class=child_run_failed"));
        assert!(summary.contains("parent_topology_unchanged=true"));
        assert!(summary.contains("child run failed see evidence"));
    }

    #[test]
    fn parallel_cli_event_summary_ignores_unrelated_topic() {
        let event = ralph_proto::Event::new("analysis.done", "ok");

        assert!(
            parallel_cli_event_summary(&event).is_none(),
            "plain event summary should only show topology/capability control-plane events"
        );
    }

    #[test]
    fn maybe_write_parallel_cli_event_summary_respects_verbosity() {
        let event = ralph_proto::Event::new(
            TOPIC_CAPABILITY_REQUEST,
            r#"{"request_id":"cap-003","capability_id":"workflow:default-parallel","input":"run"}"#,
        );
        let mut normal_writer = CountingWriter::default();
        let mut quiet_writer = CountingWriter::default();

        maybe_write_parallel_cli_event_summary(&mut normal_writer, &event, Verbosity::Normal);
        maybe_write_parallel_cli_event_summary(&mut quiet_writer, &event, Verbosity::Quiet);

        let normal_output =
            String::from_utf8(normal_writer.bytes).expect("writer bytes should be utf8");
        assert!(normal_output.contains("[supervisor:event] capability.request"));
        assert!(normal_output.contains("status=running"));
        assert_eq!(normal_writer.flushes, 1);
        assert!(quiet_writer.bytes.is_empty());
        assert_eq!(quiet_writer.flushes, 0);
    }

    #[tokio::test]
    async fn shutdown_parallel_runtime_with_timeout_reports_timeout() {
        let completed = shutdown_parallel_runtime_with_timeout(
            "test_runtime",
            Duration::from_millis(1),
            async {
                tokio::time::sleep(Duration::from_millis(20)).await;
            },
        )
        .await;

        assert!(
            !completed,
            "shutdown helper should report timeout for hanging runtime cleanup"
        );
    }

    #[tokio::test]
    async fn shutdown_parallel_runtime_with_timeout_reports_success() {
        let completed = shutdown_parallel_runtime_with_timeout(
            "test_runtime",
            Duration::from_millis(20),
            async {},
        )
        .await;

        assert!(
            completed,
            "shutdown helper should report success when cleanup finishes in time"
        );
    }
}

/// 运行并行 HatInstance 调度器。
///
/// 说明：
/// - 目前 parallel 模式先走“日志输出”路径，TUI 仍沿用旧串行实现。
#[derive(Debug, Clone)]
pub struct ParallelLoopFlags {
    pub resume: bool,
    pub enable_tui: bool,
    pub plain: bool,
    pub show_stderr: bool,
    /// Headless/CI/E2E: 允许在 prompt 缺失/为空时启动并待机。
    pub idle_start: bool,
    /// 并行 TUI: 仅当没有 CLI prompt 覆盖时,才允许自动待机。
    pub allow_tui_auto_idle: bool,
    /// 可选的 Rerun `.rrd` 录制路径（V1 live runtime graph）。
    pub runtime_graph_rrd: Option<PathBuf>,
}

pub async fn run_parallel_loop_impl(
    config: RalphConfig,
    color_mode: ColorMode,
    flags: ParallelLoopFlags,
    verbosity: Verbosity,
    record_session: Option<PathBuf>,
    instance_filters: Vec<String>,
    custom_args: Vec<String>,
) -> Result<TerminationReason> {
    process_management::setup_process_group();

    let resume = flags.resume;
    let plain = flags.plain;
    let show_stderr = flags.show_stderr;
    let runtime_graph_rrd = flags.runtime_graph_rrd.clone();
    let record_session_for_tui = record_session.clone();

    // TUI 需要 stdin/stdout 都是 TTY（crossterm 既要读键盘也要写屏幕）
    let enable_tui = flags.enable_tui && stdin().is_terminal() && stdout().is_terminal();
    let session_recorder: Option<Arc<SessionRecorder<std::io::BufWriter<File>>>> =
        if let Some(record_path) = record_session {
            let file = File::create(&record_path).with_context(|| {
                format!(
                    "Failed to create session recording file (parallel): {:?}",
                    record_path
                )
            })?;
            let recorder = Arc::new(SessionRecorder::new(std::io::BufWriter::new(file)));

            // 写入“最近一次录制路径”指针,用于 `ralph record watch` 无参自动定位.
            crate::record_session::write_record_session_latest_pointer(
                &config.core.workspace_root,
                &record_path,
            )?;

            // 录制 session 基本元信息(目录/命令),用于离线排障与证据对齐.
            //
            // 注意:
            // - 只记录低敏感度信息,不记录 env(避免 token 泄露到 cassette).
            let argv = std::env::args_os()
                .map(|s| s.to_string_lossy().to_string())
                .collect::<Vec<_>>();
            let cwd = std::env::current_dir()
                .ok()
                .map(|p| p.to_string_lossy().to_string());
            let current_exe = std::env::current_exe()
                .ok()
                .map(|p| p.to_string_lossy().to_string());
            let workspace_root = config.core.workspace_root.display().to_string();
            recorder.record_meta(Record::meta_session_start(
                cwd.as_deref(),
                Some(workspace_root.as_str()),
                &argv,
                std::process::id(),
                current_exe.as_deref(),
                Some(env!("CARGO_PKG_VERSION")),
            ));

            // 录制元信息：parallel 模式用更明确的 ux_mode，便于诊断
            recorder.record_meta(Record::meta_loop_start(
                &config.event_loop.prompt_file,
                config.event_loop.max_iterations,
                Some(if enable_tui {
                    "parallel-tui"
                } else {
                    "parallel-cli"
                }),
            ));

            debug!("Session recording enabled (parallel): {:?}", record_path);
            Some(recorder)
        } else {
            None
        };

    let use_colors = color_mode.should_use_colors();

    // prompt 解析(并行模式):
    //
    // 说明：
    // - 默认与串行保持一致：必须有 prompt。
    // - 但为了支持“并行 TUI 启动后待机(0 token)”体验,我们允许在特定条件下进入 idle_start:
    //   - headless/CI/E2E: 显式 `--idle-start`
    //   - 交互式 TUI: 默认 `PROMPT.md` 缺失/为空 + 无 CLI prompt 覆盖时自动待机
    // - idle_start 属于“无 prompt 的常驻会话”语义:
    //   - 不自动投递 `task.start`
    //   - 不受 Supervisor 级 `max_runtime_seconds` 限制
    //
    // 重要：
    // - idle_start 只在 fresh run 生效；resume/continue 不支持该语义。
    // - 即使进入 idle_start,我们仍会创建 `.ralph/current-events` marker 以支持 `ralph emit` 注入。
    let mut idle_start = false;
    let prompt_content = if resume {
        crate::loop_runner::resolve_prompt_content(&config.event_loop)?
    } else {
        // 1) 优先 inline prompt
        let inline = config
            .event_loop
            .prompt
            .as_deref()
            .map(|s| s.to_string())
            .unwrap_or_default();

        if inline.trim().is_empty() {
            // 2) prompt_file
            let prompt_file = config.event_loop.prompt_file.as_str();
            let default_prompt_file = prompt_file == "PROMPT.md";

            // 并行 TUI 的“自动待机”只在默认 PROMPT.md 丢失/为空时触发,避免吞掉显式配置错误。
            let can_auto_idle = enable_tui && flags.allow_tui_auto_idle && default_prompt_file;

            if prompt_file.is_empty() {
                if flags.idle_start {
                    idle_start = true;
                    String::new()
                } else {
                    crate::loop_runner::resolve_prompt_content(&config.event_loop)?
                }
            } else {
                let path = std::path::Path::new(prompt_file);
                if path.exists() {
                    let content = std::fs::read_to_string(path).with_context(|| {
                        format!("Failed to read prompt file (parallel): {}", prompt_file)
                    })?;

                    if content.trim().is_empty() && (flags.idle_start || can_auto_idle) {
                        idle_start = true;
                        String::new()
                    } else if content.trim().is_empty() {
                        anyhow::bail!(
                            "Prompt file '{}' is empty. Add content, or use `ralph run --idle-start` (parallel) to start idle.",
                            prompt_file
                        );
                    } else {
                        content
                    }
                } else if flags.idle_start || can_auto_idle {
                    idle_start = true;
                    String::new()
                } else {
                    crate::loop_runner::resolve_prompt_content(&config.event_loop)?
                }
            }
        } else {
            inline
        }
    };

    // Create termination + interrupt signals for TUI lifecycle
    let (terminated_tx, terminated_rx) = watch::channel(false);
    let (interrupt_tx, interrupt_rx) = watch::channel(false);

    // Fresh run：创建本轮的 current-events marker，供 `ralph emit` 追加事件
    if !resume {
        let run_id = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
        let events_path = format!(".ralph/events-{}.jsonl", run_id);

        std::fs::create_dir_all(".ralph").context("Failed to create .ralph directory")?;
        std::fs::write(".ralph/current-events", &events_path)
            .context("Failed to write .ralph/current-events marker file")?;

        // Fresh run：清理旧 scratchpad，避免历史残留误导本次 objective。
        // 注意：resume/continue 模式下必须保留 scratchpad（作为恢复上下文的一部分）。
        let scratchpad_path = config.core.resolve_path(&config.core.scratchpad);
        crate::loop_runner::clear_scratchpad_for_fresh_run(&scratchpad_path, "parallel")?;
    }

    // headless(无 TUI)下的 idle_start 需要一个“明显信号”,否则看起来像卡住。
    // 注意：这是本地输出,不触发任何后端调用,不消耗 token。
    if idle_start && !enable_tui && !matches!(verbosity, Verbosity::Quiet) {
        let mut out = std::io::stdout().lock();
        write_parallel_cli_line(
            &mut out,
            "[supervisor] idle_start 已启用: 正在等待外部事件,且不受 max_runtime 限制. 例如: ralph emit human.message \"你的任务\" --target-instance ralph#1",
        );
    }

    let default_backend = CliBackend::from_config(&config.cli)
        .map_err(|e| anyhow::anyhow!("Failed to create backend from config: {e}"))?;
    let codex_mcp_runtime = Arc::new(CodexMcpRuntime::default());
    let codex_app_server_runtime = Arc::new(CodexAppServerRuntime::new(use_colors));

    let executor = Arc::new(CliHatJobExecutor {
        default_backend,
        role_args: config.cli.role_args.clone(),
        role_reasoning_effort: config.cli.reasoning_effort,
        custom_args,
        codex_mcp_runtime: Arc::clone(&codex_mcp_runtime),
        codex_app_server_runtime: Arc::clone(&codex_app_server_runtime),
    });

    let instance_filter_set: std::collections::HashSet<String> =
        instance_filters.into_iter().collect();

    // 初始化实例列表（用于 log 模式打印 / TUI 模式预注册）
    let mut initial_instances = Vec::new();
    for (hat_id, hat_cfg) in &config.hats {
        let n = hat_cfg.instances.max(1);
        for i in 1..=n {
            initial_instances.push(format!("{hat_id}#{i}"));
        }
    }
    initial_instances.push("ralph#1".to_string());
    initial_instances.sort();

    // TUI（并行 Supervisor UI）
    let (mut tui_handle, tui_update_tx) = if enable_tui {
        let mut tui = Tui::new_parallel()
            .with_parallel_markdown_rendering(!plain)
            .with_parallel_max_buffer_lines(config.tui.max_buffer_lines)
            .with_parallel_evidence_paths(parallel_evidence_paths_for_tui(
                &config,
                record_session_for_tui.as_deref(),
            ))
            .with_termination_signal(terminated_rx)
            .with_interrupt_tx(interrupt_tx.clone());

        // 右上角 Radar：best-effort 渲染 hats graph（失败不影响主流程）
        let registry = HatRegistry::from_config(&config);
        match crate::hats::render_hat_graph_radar_ascii(&config, &registry) {
            Ok(radar) => {
                tui = tui.with_hat_graph_radar(radar);
            }
            Err(e) => {
                warn!("Failed to render hat graph radar for parallel TUI: {e:#}");
            }
        }

        let update_tx = tui
            .update_sender()
            .expect("Tui::new_parallel() must have update_sender()");

        // 预注册实例（Created），让 UI 一启动就能看到列表
        for id in &initial_instances {
            let _ = update_tx.send(TuiUpdate::ParallelRegisterInstance {
                instance_id: HatInstanceId::from(id.as_str()),
                state: HatInstanceState::Created,
            });
        }

        (
            Some(tokio::spawn(async move { tui.run().await })),
            Some(update_tx),
        )
    } else {
        (None, None)
    };

    // 给 TUI 一点初始化时间（进入 alternate screen / raw mode）
    if tui_handle.is_some() {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // 并行模式下的输出/状态/gate 观察者
    type OutputObserver = Arc<dyn Fn(&HatJobOutputChunk) + Send + Sync>;
    type StateObserver = Arc<dyn Fn(&HatInstanceId, HatInstanceState) + Send + Sync>;
    type EventObserver = Arc<dyn Fn(&ralph_proto::Event) + Send + Sync>;

    let (observer, state_observer, event_observer): (
        OutputObserver,
        StateObserver,
        Option<EventObserver>,
    ) = if enable_tui {
        let update_tx = tui_update_tx.clone().expect("tui_update_tx must exist");

        let output_tx = update_tx.clone();
        let recorder_for_output = session_recorder.clone();
        let observer: OutputObserver = Arc::new(move |chunk: &HatJobOutputChunk| {
            // best-effort：把 stdout/stderr 写入 cassette（并行回放用）
            if let Some(recorder) = &recorder_for_output {
                let offset_ms = recorder.elapsed().as_millis() as u64;
                let mut line = chunk.line.clone();
                line.push('\n');
                let is_stdout = !matches!(chunk.stream, OutputStream::Stderr);
                recorder.record_ux_event(&UxEvent::TerminalWrite(
                    TerminalWrite::new(line.as_bytes(), is_stdout, offset_ms)
                        .with_instance_id(chunk.instance_id.to_string()),
                ));
            }

            // 默认显示 stderr，便于调试。
            // 如需降噪，用 `ralph run --hide-stderr` 显式隐藏。
            //
            // 注意:
            // - 录制 cassette 与 "是否显示" 是两件事。
            // - 因此即使 hide stderr,我们仍然会把 stderr 写入 cassette(用于回放/诊断)。
            if !show_stderr && matches!(chunk.stream, OutputStream::Stderr) {
                return;
            }

            let _ = output_tx.send(TuiUpdate::ParallelOutputChunk(chunk.clone()));
        });

        let state_tx = update_tx.clone();
        let state_observer: StateObserver = Arc::new(move |instance_id: &HatInstanceId, state| {
            let _ = state_tx.send(TuiUpdate::ParallelInstanceState {
                instance_id: instance_id.clone(),
                state,
            });
        });

        let event_tx = update_tx;
        let recorder_for_events = session_recorder.clone();
        let event_observer: EventObserver = Arc::new(move |event: &ralph_proto::Event| {
            // best-effort：把 bus.publish 写入 cassette（便于诊断/提取命令）
            if let Some(recorder) = &recorder_for_events {
                recorder.record_bus_event(event);
            }
            // 事件转发策略：
            // - 控制面事件（gate.* / human.message）必须进 UI（Gate 面板/活跃度等）；
            // - 运行态可视化（Hat Graph Radar 边动画）需要“带 source 的业务事件”进 UI；
            // - 其余事件不转发，避免 UI 被高频噪音刷爆。
            if should_forward_event_to_tui(event) {
                let _ = event_tx.send(TuiUpdate::ParallelEvent(event.clone()));
            }
        });

        (observer, state_observer, Some(event_observer))
    } else {
        // 日志模式：沿用原有 stdout 打印
        // 6.1：实例列表（日志模式下的最小可用展示）
        if !matches!(verbosity, Verbosity::Quiet) {
            let mut out = std::io::stdout().lock();
            write_parallel_cli_line(&mut out, "[supervisor] instances (initial=created):");
            for id in &initial_instances {
                write_parallel_cli_line(&mut out, &format!("  - {id}"));
            }
        }

        // 输出观察者：按实例归因输出
        let stderr_hidden_hint_printed = Arc::new(AtomicBool::new(false));
        let recorder_for_output = session_recorder.clone();
        let observer: OutputObserver = Arc::new(move |chunk: &HatJobOutputChunk| {
            // best-effort：把 stdout/stderr 写入 cassette（并行回放用）
            if let Some(recorder) = &recorder_for_output {
                let offset_ms = recorder.elapsed().as_millis() as u64;
                let mut line = chunk.line.clone();
                line.push('\n');
                let is_stdout = !matches!(chunk.stream, OutputStream::Stderr);
                recorder.record_ux_event(&UxEvent::TerminalWrite(
                    TerminalWrite::new(line.as_bytes(), is_stdout, offset_ms)
                        .with_instance_id(chunk.instance_id.to_string()),
                ));
            }

            if matches!(verbosity, Verbosity::Quiet) {
                return;
            }

            if !instance_filter_set.is_empty()
                && !instance_filter_set.contains(chunk.instance_id.as_str())
            {
                return;
            }

            // 默认显示 stderr 行，便于调试；如需降噪可显式隐藏。
            if !show_stderr && matches!(chunk.stream, OutputStream::Stderr) {
                // 仅在第一次“确实出现 stderr”时提醒一次，避免用户困惑但又不刷屏。
                if !stderr_hidden_hint_printed.swap(true, Ordering::Relaxed) {
                    let mut out = std::io::stdout().lock();
                    write_parallel_cli_line(
                        &mut out,
                        "[supervisor] stderr streaming lines are hidden (via `--hide-stderr`); omit it to show them.",
                    );
                }
                return;
            }

            // 使用 stdout 直接写，避免 tracing 与输出混用导致顺序错乱
            let mut out = std::io::stdout().lock();

            let stream_tag = match chunk.stream {
                OutputStream::Stdout => "out",
                OutputStream::Stderr => "err",
                OutputStream::Activity => "act",
            };

            let line = format!(
                "[{}:{}:job={}] {}",
                chunk.instance_id, stream_tag, chunk.job_id, chunk.line
            );

            // stderr 用灰色显示，提高可读性。
            //
            // 注意:
            // - 如果 stderr 行本身带 ANSI(例如我们回显的 prompt transcript,或后端自身彩色日志),
            //   外层再包一层 GRAY 会破坏原始色彩语义。
            let is_stderr = matches!(chunk.stream, OutputStream::Stderr);
            let line_has_ansi = chunk.line.contains("\x1b[");
            if use_colors && is_stderr && !line_has_ansi {
                write_parallel_cli_line(
                    &mut out,
                    &format!("{}{}{}", colors::GRAY, line, colors::RESET),
                );
            } else {
                write_parallel_cli_line(&mut out, &line);
            }
        });

        // 6.1：状态变更展示（日志模式）
        let state_observer: StateObserver = Arc::new(move |instance_id: &HatInstanceId, state| {
            if matches!(verbosity, Verbosity::Quiet) {
                return;
            }
            let mut out = std::io::stdout().lock();
            write_parallel_cli_line(&mut out, &format!("[{}:state] {}", instance_id, state));
        });

        // 日志模式默认展示低频控制面事件摘要。
        //
        // 说明:
        // - `topology.*` / `capability.*` 是用户判断“实例是否真的创建 / child-run 是否启动”的关键证据。
        // - 这里只输出一行结构化摘要,不把所有业务事件刷到终端,避免重复 record-session 的完整审计职责。
        // - recorder 仍然是完整 bus.publish 的耐久真相源；这里是 display layer。
        let recorder_for_events = session_recorder.clone();
        let event_observer: Option<EventObserver> =
            Some(Arc::new(move |event: &ralph_proto::Event| {
                if let Some(recorder) = &recorder_for_events {
                    recorder.record_bus_event(event);
                }

                if let Some(summary) = maybe_parallel_cli_event_summary(event, verbosity) {
                    let mut out = std::io::stdout().lock();
                    write_parallel_cli_line(&mut out, &summary);
                }
            }) as EventObserver);

        (observer, state_observer, event_observer)
    };

    // ------------------------------------------------------------------
    // 可选的 Rerun live runtime graph 记录器
    //
    // 说明:
    // - 这是 V1 MVP: 基于现有 live observers 直接输出 `.rrd` artifact。
    // - recorder 初始化失败时直接报错:
    //   用户既然显式要求录制,就不应该 silent skip。
    // ------------------------------------------------------------------
    let runtime_graph = runtime_graph_rrd
        .map(RuntimeGraphRecorder::create)
        .transpose()?
        .map(Arc::new);

    if let Some(graph) = &runtime_graph {
        tracing::info!(
            path = %graph.output_path().display(),
            "Runtime graph recording enabled"
        );
    }

    let state_observer: StateObserver = {
        let prior = Arc::clone(&state_observer);
        let runtime_graph = runtime_graph.clone();
        Arc::new(move |instance_id: &HatInstanceId, state| {
            prior(instance_id, state);
            if let Some(graph) = &runtime_graph {
                graph.observe_instance_state(instance_id, state);
            }
        })
    };

    let event_observer: Option<EventObserver> = event_observer.map(|prior| {
        let runtime_graph = runtime_graph.clone();
        Arc::new(move |event: &ralph_proto::Event| {
            prior(event);
            if let Some(graph) = &runtime_graph {
                graph.observe_event(event);
            }
        }) as EventObserver
    });

    // Spawn signal handlers AFTER TUI initialization to avoid deadlock
    // (TUI must enter raw mode and create EventStream before signal handlers are registered)
    let interrupt_tx_sigint = interrupt_tx.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            let _ = interrupt_tx_sigint.send(true);
        }
    });

    #[cfg(unix)]
    {
        let interrupt_tx_sigterm = interrupt_tx.clone();
        tokio::spawn(async move {
            let mut sigterm =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                    .expect("Failed to register SIGTERM handler");
            sigterm.recv().await;
            let _ = interrupt_tx_sigterm.send(true);
        });
    }

    #[cfg(unix)]
    {
        let interrupt_tx_sighup = interrupt_tx.clone();
        tokio::spawn(async move {
            let mut sighup = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup())
                .expect("Failed to register SIGHUP handler");
            sighup.recv().await;
            let _ = interrupt_tx_sighup.send(true);
        });
    }

    // Supervisor
    //
    // Phase 4: runtime capability invocation adapter。
    // 说明:
    // - core supervisor 只识别 `capability.request` 并回传 result/failure event。
    // - 真正 isolated child/micro-run 仍由 CLI capability module 执行。
    // - workspace_root 是 artifact 真相源根目录,不会热改 parent topology。
    let runtime_capabilities_enabled = runtime_capability_wiring_enabled(&config);
    let workspace_root = config.core.workspace_root.clone();
    let runtime_capability_base_config = config.clone();
    let runtime_capability_catalog = if runtime_capabilities_enabled {
        crate::capability::capability_catalog()
    } else {
        debug!(
            "Runtime capability catalog disabled by config; child run will not receive catalog or invoker"
        );
        Vec::new()
    };

    let mut supervisor = ParallelSupervisor::new(config, prompt_content, executor)?
        .with_runtime_capability_catalog(runtime_capability_catalog)
        .with_agents_snapshot_to_default_path()
        .with_output_observer(observer)
        .with_instance_state_observer(state_observer)
        // 并行 TUI：completion promise（LOOP_COMPLETE）进入“暂停”而不是“退出”，并禁用动态实例回收，
        // 这样 human message 可以在会话中持续驱动下一轮对话/工作，而不会被 done/回收打断。
        .with_pause_on_completion_promise(enable_tui)
        .with_disable_dynamic_instance_reap(enable_tui)
        .with_idle_start(idle_start);
    if runtime_capabilities_enabled {
        supervisor = supervisor.with_runtime_capability_invoker(
            crate::capability::runtime_capability_invoker(
                workspace_root,
                runtime_capability_base_config,
            ),
        );
    }
    if let Some(event_observer) = event_observer {
        supervisor = supervisor.with_event_observer(event_observer);
    }
    if let Some(runtime_graph) = runtime_graph.clone() {
        let delivery_observer = Arc::new(move |obs: &ralph_core::RuntimeDeliveryObservation| {
            runtime_graph.observe_delivery(obs);
        });
        supervisor = supervisor.with_delivery_observer(delivery_observer);
    }

    // Ctrl+C/SIGTERM/SIGHUP: 让 TUI 立即退出(如果还在跑)。
    //
    // 说明:
    // - 真实的 runner 收尾(取消 job + shutdown instances)由 core::ParallelSupervisor 执行。
    // - 这里仅做 UI 退出信号,避免用户看到“已中断但 TUI 还挂着”的黑盒体验。
    let terminated_tx_for_interrupt = terminated_tx.clone();
    let mut interrupt_rx_for_termination = interrupt_rx.clone();
    tokio::spawn(async move {
        loop {
            let changed = interrupt_rx_for_termination.changed().await;
            if changed.is_err() {
                break;
            }
            if *interrupt_rx_for_termination.borrow() {
                let _ = terminated_tx_for_interrupt.send(true);
                break;
            }
        }
    });

    let result = supervisor
        .run_with_interrupt(resume, interrupt_rx.clone())
        .await;
    let result = match result {
        Ok(result) => result,
        Err(e) => {
            // best-effort：错误退出时也尽量刷盘,保留可审计证据.
            if let Some(recorder) = &session_recorder {
                let reason_str = format!("Error: {e:#}");
                recorder.record_meta(Record::meta_termination(
                    &reason_str,
                    0,
                    recorder.elapsed().as_secs_f64(),
                    recorder.ux_write_count(),
                ));
                let _ = recorder.flush();
            }
            shutdown_parallel_runtimes(&codex_mcp_runtime, &codex_app_server_runtime).await;
            return Err(e);
        }
    };

    let ralph_core::ParallelRunResult {
        termination,
        ralph_iterations,
        instance_states,
        ..
    } = result;

    if let Some(runtime_graph) = &runtime_graph {
        runtime_graph.finish();
    }

    // 6.1：结束时打印最终状态快照
    if !enable_tui && !matches!(verbosity, Verbosity::Quiet) {
        let mut pairs = instance_states.into_iter().collect::<Vec<_>>();
        pairs.sort_by(|a, b| a.0.as_str().cmp(b.0.as_str()));

        let mut out = std::io::stdout().lock();
        write_parallel_cli_line(&mut out, "[supervisor] final states:");
        for (id, state) in pairs {
            write_parallel_cli_line(&mut out, &format!("  - {id}: {state}"));
        }
    }

    let reason = termination.unwrap_or(TerminationReason::Stopped);

    // 自然结束：如果 TUI 开着，让用户按 q 退出（与串行模式对齐）。
    // Ctrl+C/SIGTERM: 不等待,直接退出(证据完整优先,避免收尾卡住)。
    if reason == TerminationReason::Interrupted {
        let _ = terminated_tx.send(true);
    } else if let Some(handle) = tui_handle.take() {
        let _ = handle.await;
    }

    // best-effort：写入 termination 元信息，便于 cassette 诊断/回放
    if let Some(recorder) = &session_recorder {
        let reason_str = format!("{reason:?}");
        recorder.record_meta(Record::meta_termination(
            &reason_str,
            ralph_iterations,
            recorder.elapsed().as_secs_f64(),
            recorder.ux_write_count(),
        ));
        let _ = recorder.flush();
    }

    shutdown_parallel_runtimes(&codex_mcp_runtime, &codex_app_server_runtime).await;

    Ok(reason)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ralph_proto::{Event, HatId};

    #[test]
    fn parallel_tui_event_forwarding_allows_gate_topics() {
        // gate.* 属于控制面事件：必须转发到 TUI（否则 Gate 面板无法更新）。
        let event = Event::new("gate.request", "");
        assert!(should_forward_event_to_tui(&event));
    }

    #[test]
    fn parallel_tui_event_forwarding_allows_capability_topics() {
        // capability.* 是 isolated child-run 的父级观测事件：没有 source 时也必须进入 TUI。
        let event = Event::new("capability.invoke", "");
        assert!(should_forward_event_to_tui(&event));
    }

    #[test]
    fn parallel_tui_event_forwarding_allows_topology_topics() {
        // topology.* 是 parent-visible 动态实例控制面事件：需要进入 TUI 做状态提示。
        let event = Event::new("topology.spawn.result", "");
        assert!(should_forward_event_to_tui(&event));
    }

    #[test]
    fn parallel_tui_event_forwarding_allows_human_message() {
        // human.message 会影响 TUI 的活跃度与最近事件展示。
        let event = Event::new("human.message", "");
        assert!(should_forward_event_to_tui(&event));
    }

    #[test]
    fn parallel_tui_event_forwarding_allows_events_with_source() {
        // 允许带 source 的事件进入 UI（兼容外部注入/特殊来源事件）。
        let event = Event::new("build.task", "").with_source(HatId::new("builder"));
        assert!(should_forward_event_to_tui(&event));
    }

    #[test]
    fn parallel_tui_event_forwarding_allows_events_with_source_instance() {
        // 并行模式下，业务事件通常只有 source_instance（发布者实例），TUI 可据此推导 hat_id。
        let event = Event::new("build.task", "").with_source_instance("builder#1");
        assert!(should_forward_event_to_tui(&event));
    }

    #[test]
    fn parallel_tui_event_forwarding_filters_noise_without_source_or_instance() {
        // 既没有 source，也没有 source_instance，且不是 gate.* / human.message：认为是 UI 噪音，不转发。
        let event = Event::new("build.task", "");
        assert!(!should_forward_event_to_tui(&event));
    }

    #[test]
    fn runtime_capability_wiring_respects_core_flag() {
        let mut config = RalphConfig::default();
        assert!(runtime_capability_wiring_enabled(&config));

        config.core.runtime_capabilities_enabled = false;
        assert!(!runtime_capability_wiring_enabled(&config));
    }

    #[test]
    fn parallel_evidence_paths_for_tui_use_current_events_marker() {
        let temp = tempfile::TempDir::new().unwrap();
        let mut config = RalphConfig::default();
        config.core.workspace_root = temp.path().to_path_buf();

        let ralph_dir = temp.path().join(".ralph");
        std::fs::create_dir_all(&ralph_dir).unwrap();
        std::fs::write(
            ralph_dir.join("current-events"),
            ".ralph/events-20260517-200000.jsonl\n",
        )
        .unwrap();

        let record_path = temp.path().join("records/session.jsonl");
        let paths = parallel_evidence_paths_for_tui(&config, Some(&record_path));

        assert_eq!(
            paths.events_path.as_deref(),
            Some(".ralph/events-20260517-200000.jsonl")
        );
        assert_eq!(
            paths.evidence_index_path.as_deref(),
            Some(".ralph/evidence-index.jsonl")
        );
        assert_eq!(
            paths.agents_snapshot_path.as_deref(),
            Some(".ralph/agents.json")
        );
        assert_eq!(
            paths.record_session_path.as_deref(),
            Some("records/session.jsonl")
        );
    }

}
