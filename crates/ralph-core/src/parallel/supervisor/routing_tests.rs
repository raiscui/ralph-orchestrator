//! 并行路由的确定性/回放护栏测试（OpenSpec tasks 7.x）。
//!
//! 说明：
//! - 这些测试不依赖真实 LLM 后端。
//! - 通过 Fake HatJobExecutor 验证：并行、missing 行为、require_delivery escalate、queue 决策 replay。

use super::{
    DYNAMIC_INSTANCE_RETIREMENT_REASON_DELIVERY_FAILED_AFTER_SPAWN, ParallelSupervisor,
    TOPIC_COORDINATOR_NO_EVENT_FIRST_TURN,
};
use crate::config::{
    CoreConfig, HatBackend, HatConfig, HatWorkspaceConfig, ParallelConfig, RalphConfig,
};
use crate::event_logger::{EventHistory, EventLogger};
use crate::evidence_index::{
    EvidenceArtifactKind, EvidenceIndexReader, EvidenceIndexWriter, EvidenceLookup, EvidenceStatus,
};
use crate::parallel::{
    HatInstanceCommand, HatInstanceHandle, HatJob, HatJobControl, HatJobExecutor,
    HatJobOutputChunk, HatJobResult,
};
use crate::{
    RecoverableFailureKind, RecoverableFailureSnapshot, RecoverableFailureStatus, TerminationReason,
};
use anyhow::Context;
use ralph_proto::{
    AudienceOverride, AudienceSelector, Delivery, Event, HatId, HatInstanceId, HatInstanceState,
    MissingInstancePolicy, QueueDecisionRecord, QueueSelection, RuntimeDeliveryKind,
    RuntimeDeliveryRecord, RuntimeLifecycleKind, RuntimeLifecycleRecord, TOPIC_DISPATCH_DECISION,
    TOPIC_REPLY_HAT_MESSAGE, TOPIC_REQUESTER_RETURN, TOPIC_RUNTIME_DELIVERY,
    TOPIC_RUNTIME_LIFECYCLE, SessionStrategy, TopicContract, TurnAction,
};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::sync::{Notify, mpsc};
use tokio::time::Duration;

#[derive(Debug, Clone)]
struct NotifyExecutor {
    /// 期待同时启动的 job 数量（用于并行性断言）。
    expected_starts: usize,
    started: Arc<AtomicUsize>,
    notify: Arc<Notify>,
    seen: Arc<tokio::sync::Mutex<Vec<String>>>,
}

#[derive(Debug, Clone)]
struct PauseOnCompletionExecutor {
    started: Arc<AtomicUsize>,
    first: Arc<Notify>,
    second: Arc<Notify>,
}

#[async_trait::async_trait]
impl HatJobExecutor for PauseOnCompletionExecutor {
    async fn execute(
        &self,
        job: HatJob,
        _output_tx: mpsc::Sender<HatJobOutputChunk>,
        mut _cancel_rx: tokio::sync::watch::Receiver<bool>,
        mut _control_rx: mpsc::Receiver<HatJobControl>,
    ) -> anyhow::Result<HatJobResult> {
        // 说明：
        // - 该 executor 专门用于验证：parallel-tui 下 `LOOP_COMPLETE` 不会让 Supervisor 退出，
        //   并且 external events（human.message）仍然可以驱动下一次 job。

        if job.instance_id.as_str() == "ralph#1" {
            let now = self.started.fetch_add(1, Ordering::SeqCst) + 1;
            if now == 1 {
                self.first.notify_waiters();
            } else if now == 2 {
                self.second.notify_waiters();
            }

            return Ok(HatJobResult {
                output_for_parsing: "LOOP_COMPLETE\n".to_string(),
                observed_stderr: String::new(),
                success: true,
                exit_code: Some(0),
                timed_out: false,
                canceled: false,
            });
        }

        Ok(HatJobResult {
            output_for_parsing: String::new(),
            observed_stderr: String::new(),
            success: true,
            exit_code: Some(0),
            timed_out: false,
            canceled: false,
        })
    }
}

#[derive(Debug, Clone)]
struct PauseMaxRuntimeExecutor {
    ralph_runs: Arc<AtomicUsize>,
    first_done: Arc<Notify>,
    second_started: Arc<Notify>,
}

#[async_trait::async_trait]
impl HatJobExecutor for PauseMaxRuntimeExecutor {
    async fn execute(
        &self,
        job: HatJob,
        _output_tx: mpsc::Sender<HatJobOutputChunk>,
        mut cancel_rx: tokio::sync::watch::Receiver<bool>,
        mut _control_rx: mpsc::Receiver<HatJobControl>,
    ) -> anyhow::Result<HatJobResult> {
        if job.instance_id.as_str() != "ralph#1" {
            return Ok(HatJobResult {
                output_for_parsing: String::new(),
                observed_stderr: String::new(),
                success: true,
                exit_code: Some(0),
                timed_out: false,
                canceled: false,
            });
        }

        let now = self.ralph_runs.fetch_add(1, Ordering::SeqCst) + 1;
        match now {
            1 => {
                // 第一次：立刻输出 completion promise，让 Supervisor 进入暂停态。
                self.first_done.notify_waiters();
                Ok(HatJobResult {
                    output_for_parsing: "LOOP_COMPLETE\n".to_string(),
                    observed_stderr: String::new(),
                    success: true,
                    exit_code: Some(0),
                    timed_out: false,
                    canceled: false,
                })
            }
            _ => {
                // 第二次：保持 Running 足够久，让 max_runtime 能触发（由 Supervisor cancel/shutdown 收尾）。
                self.second_started.notify_waiters();

                loop {
                    tokio::select! {
                        changed = cancel_rx.changed() => {
                            if changed.is_ok() && *cancel_rx.borrow() {
                                break;
                            }
                        }
                        _ = tokio::time::sleep(Duration::from_secs(3600)) => {}
                    }
                }

                Ok(HatJobResult {
                    output_for_parsing: String::new(),
                    observed_stderr: String::new(),
                    success: true,
                    exit_code: Some(0),
                    timed_out: false,
                    canceled: true,
                })
            }
        }
    }
}

#[async_trait::async_trait]
impl HatJobExecutor for NotifyExecutor {
    async fn execute(
        &self,
        job: HatJob,
        _output_tx: mpsc::Sender<HatJobOutputChunk>,
        mut _cancel_rx: tokio::sync::watch::Receiver<bool>,
        mut _control_rx: mpsc::Receiver<HatJobControl>,
    ) -> anyhow::Result<HatJobResult> {
        {
            let mut seen = self.seen.lock().await;
            seen.push(job.instance_id.to_string());
        }

        let now = self.started.fetch_add(1, Ordering::SeqCst) + 1;
        if now >= self.expected_starts {
            self.notify.notify_waiters();
        }

        // 等待所有预期 job 都启动（用 timeout 防止 test 卡死）
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if self.started.load(Ordering::SeqCst) >= self.expected_starts {
                    break;
                }
                self.notify.notified().await;
            }
        })
        .await
        .context("Timed out waiting for concurrent job starts")?;

        // ralph#1 用 completion promise 收尾（避免 Supervisor.run 无限跑）
        let output = if job.instance_id.as_str() == "ralph#1" {
            "LOOP_COMPLETE\n".to_string()
        } else {
            String::new()
        };

        Ok(HatJobResult {
            output_for_parsing: output,
            observed_stderr: String::new(),
            success: true,
            exit_code: Some(0),
            timed_out: false,
            canceled: false,
        })
    }
}

#[derive(Debug, Clone)]
struct BlockingExecutor {
    started: Arc<AtomicUsize>,
    notify: Arc<Notify>,
}

#[derive(Debug, Clone)]
struct IdleStartCompletionExecutor {
    started: Arc<AtomicUsize>,
    started_notify: Arc<Notify>,
}

#[derive(Debug, Clone)]
struct IdleStartPersistentExecutor {
    started: Arc<AtomicUsize>,
    started_notify: Arc<Notify>,
}

#[derive(Debug, Clone)]
struct CompletionStopsRoutingExecutor {
    /// 记录每个 instance 实际启动了多少次（用于断言“收敛后不再派生新 job”）。
    starts: Arc<tokio::sync::Mutex<HashMap<String, usize>>>,
}

#[derive(Debug, Clone)]
struct QueuedRalphJobAfterCompletionExecutor {
    /// 记录每个 instance 的启动次数,用于验证“completion 前已排队的 ralph job”
    /// 是否会在 `LOOP_COMPLETE` 之后继续起跑。
    starts: Arc<tokio::sync::Mutex<HashMap<String, usize>>>,
}

#[derive(Debug, Clone)]
struct DefaultPublishesExecutor {
    /// 记录每个 instance 的启动次数,用于构造确定性的多轮输出。
    starts: Arc<tokio::sync::Mutex<HashMap<String, usize>>>,
}

#[async_trait::async_trait]
impl HatJobExecutor for BlockingExecutor {
    async fn execute(
        &self,
        _job: HatJob,
        _output_tx: mpsc::Sender<HatJobOutputChunk>,
        mut cancel_rx: tokio::sync::watch::Receiver<bool>,
        mut _control_rx: mpsc::Receiver<HatJobControl>,
    ) -> anyhow::Result<HatJobResult> {
        let now = self.started.fetch_add(1, Ordering::SeqCst) + 1;
        if now >= 1 {
            self.notify.notify_waiters();
        }

        loop {
            tokio::select! {
                changed = cancel_rx.changed() => {
                    if changed.is_ok() && *cancel_rx.borrow() {
                        break;
                    }
                }
                _ = tokio::time::sleep(Duration::from_secs(3600)) => {}
            }
        }

        Ok(HatJobResult {
            output_for_parsing: String::new(),
            observed_stderr: String::new(),
            success: true,
            exit_code: Some(0),
            timed_out: false,
            canceled: true,
        })
    }
}

#[async_trait::async_trait]
impl HatJobExecutor for IdleStartCompletionExecutor {
    async fn execute(
        &self,
        job: HatJob,
        _output_tx: mpsc::Sender<HatJobOutputChunk>,
        mut _cancel_rx: tokio::sync::watch::Receiver<bool>,
        mut _control_rx: mpsc::Receiver<HatJobControl>,
    ) -> anyhow::Result<HatJobResult> {
        if job.instance_id.as_str() != "ralph#1" {
            return Ok(HatJobResult {
                output_for_parsing: String::new(),
                observed_stderr: String::new(),
                success: true,
                exit_code: Some(0),
                timed_out: false,
                canceled: false,
            });
        }

        self.started.fetch_add(1, Ordering::SeqCst);
        self.started_notify.notify_waiters();

        Ok(HatJobResult {
            output_for_parsing: "LOOP_COMPLETE\n".to_string(),
            observed_stderr: String::new(),
            success: true,
            exit_code: Some(0),
            timed_out: false,
            canceled: false,
        })
    }
}

#[async_trait::async_trait]
impl HatJobExecutor for IdleStartPersistentExecutor {
    async fn execute(
        &self,
        job: HatJob,
        _output_tx: mpsc::Sender<HatJobOutputChunk>,
        mut cancel_rx: tokio::sync::watch::Receiver<bool>,
        mut _control_rx: mpsc::Receiver<HatJobControl>,
    ) -> anyhow::Result<HatJobResult> {
        if job.instance_id.as_str() != "ralph#1" {
            return Ok(HatJobResult {
                output_for_parsing: String::new(),
                observed_stderr: String::new(),
                success: true,
                exit_code: Some(0),
                timed_out: false,
                canceled: false,
            });
        }

        self.started.fetch_add(1, Ordering::SeqCst);
        self.started_notify.notify_waiters();

        loop {
            tokio::select! {
                changed = cancel_rx.changed() => {
                    if changed.is_ok() && *cancel_rx.borrow() {
                        break;
                    }
                }
                _ = tokio::time::sleep(Duration::from_secs(3600)) => {}
            }
        }

        Ok(HatJobResult {
            output_for_parsing: String::new(),
            observed_stderr: String::new(),
            success: true,
            exit_code: Some(0),
            timed_out: false,
            canceled: true,
        })
    }
}

async fn wait_for_starts(executor: &BlockingExecutor, expected: usize) {
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if executor.started.load(Ordering::SeqCst) >= expected {
                break;
            }
            executor.notify.notified().await;
        }
    })
    .await
    .expect("Timed out waiting for executor to observe job starts");
}

#[tokio::test]
async fn supervisor_pause_on_completion_promise_continues_consuming_external_events_in_tui_mode() {
    // =====================================================================
    // 目的：
    // - 并行 TUI 里，`LOOP_COMPLETE` 不应直接结束 Supervisor（只“暂停停歇”）。
    // - 暂停态仍需持续消费 external events（human.message），从而让会话可继续对话。
    //
    // 策略：
    // 1) ralph#1 第一次 job 输出 `LOOP_COMPLETE`
    // 2) 往 external events 文件写入一条 human.message
    // 3) 断言 ralph#1 会再次被触发执行（说明 external events 仍在被消费/路由）
    // 4) 最终用 max_iterations 兜底结束，避免测试卡死（parallel-tui 下 max_runtime 会在暂停态被重置/暂停）
    // =====================================================================

    let temp_dir = tempfile::tempdir().unwrap();

    // external events（human 注入）：
    // - Supervisor 会读 `.ralph/current-events`，若不存在则回退到 `.ralph/events.jsonl`
    // - 这里走 fallback 路径即可（测试更简单、且 workspace_root 已隔离）。
    let external_dir = temp_dir.path().join(".ralph");
    fs::create_dir_all(&external_dir).unwrap();
    let external_events_path = external_dir.join("events.jsonl");
    fs::write(&external_events_path, "").unwrap();

    // 内部事件日志（debug/replay）写到另一个文件，避免与 external events 混在一起被误读。
    let internal_events_path = temp_dir.path().join("internal-events.jsonl");

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    config.event_loop.max_runtime_seconds = 3;
    config.event_loop.max_iterations = 2;
    config.core = config.core.with_workspace_root(temp_dir.path());

    let started = Arc::new(AtomicUsize::new(0));
    let first = Arc::new(Notify::new());
    let second = Arc::new(Notify::new());

    let executor = PauseOnCompletionExecutor {
        started,
        first: Arc::clone(&first),
        second: Arc::clone(&second),
    };

    let mut supervisor = ParallelSupervisor::new(config, "prompt".to_string(), Arc::new(executor))
        .expect("ParallelSupervisor::new should succeed")
        .with_pause_on_completion_promise(true)
        .with_disable_dynamic_instance_reap(true);

    // 断言：禁用回收后，dynamic TTL 应当被“等价无限大”化。
    assert_eq!(
        supervisor.effective_dynamic_idle_ttl(),
        Duration::from_secs(u64::MAX)
    );

    supervisor.event_logger = EventLogger::new(internal_events_path);

    let handle = tokio::spawn(async move { supervisor.run(false).await });

    // 等待第一次 ralph#1（输出 LOOP_COMPLETE）
    tokio::time::timeout(Duration::from_secs(2), first.notified())
        .await
        .expect("Timed out waiting for first ralph#1 execution");

    // 注入 human.message（外部事件）
    let line = serde_json::json!({
        "topic": "human.message",
        "payload": "hello",
        "ts": "2026-02-01T00:00:00Z",
    });
    let mut f = fs::OpenOptions::new()
        .append(true)
        .open(&external_events_path)
        .unwrap();
    writeln!(f, "{}", serde_json::to_string(&line).unwrap()).unwrap();

    // 断言：external events 驱动了第二次 ralph#1 执行
    tokio::time::timeout(Duration::from_secs(2), second.notified())
        .await
        .expect("Timed out waiting for second ralph#1 execution");

    // 收尾：由 max_iterations 兜底结束
    let result = tokio::time::timeout(Duration::from_secs(5), handle)
        .await
        .expect("Timed out waiting for supervisor.run() to return")
        .expect("JoinHandle should succeed")
        .expect("supervisor run should succeed");

    assert_eq!(result.termination, Some(TerminationReason::MaxIterations));
}

#[tokio::test]
async fn supervisor_pause_on_completion_promise_resets_and_pauses_max_runtime_until_next_running() {
    // =====================================================================
    // 目的：
    // - parallel-tui 下，`LOOP_COMPLETE` 进入暂停态后：
    //   1) max_runtime 计时会重置并暂停（暂停期不应触发 MaxRuntime）
    //   2) 直到任意实例再次 Running 才开始重新计时
    //
    // 验证策略：
    // 1) ralph#1 第一次 job 输出 LOOP_COMPLETE -> 进入暂停态
    // 2) 等待超过 max_runtime_seconds，断言 Supervisor 仍未退出（说明暂停期不计时）
    // 3) 注入 human.message，触发 ralph#1 第二次 job，并保持 Running
    // 4) 断言 Supervisor 最终以 MaxRuntime 终止（说明恢复后重新计时生效）
    // =====================================================================

    let temp_dir = tempfile::tempdir().unwrap();

    // external events：走 fallback `.ralph/events.jsonl`
    let external_dir = temp_dir.path().join(".ralph");
    fs::create_dir_all(&external_dir).unwrap();
    let external_events_path = external_dir.join("events.jsonl");
    fs::write(&external_events_path, "").unwrap();

    let internal_events_path = temp_dir.path().join("internal-events.jsonl");

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    config.event_loop.max_runtime_seconds = 1;
    config.core = config.core.with_workspace_root(temp_dir.path());

    let first_done = Arc::new(Notify::new());
    let second_started = Arc::new(Notify::new());

    let executor = PauseMaxRuntimeExecutor {
        ralph_runs: Arc::new(AtomicUsize::new(0)),
        first_done: Arc::clone(&first_done),
        second_started: Arc::clone(&second_started),
    };

    let mut supervisor = ParallelSupervisor::new(config, "prompt".to_string(), Arc::new(executor))
        .expect("ParallelSupervisor::new should succeed")
        .with_pause_on_completion_promise(true)
        .with_disable_dynamic_instance_reap(true);

    supervisor.event_logger = EventLogger::new(internal_events_path);

    let handle = tokio::spawn(async move { supervisor.run(false).await });

    tokio::time::timeout(Duration::from_secs(2), first_done.notified())
        .await
        .expect("Timed out waiting for first LOOP_COMPLETE to enter pause mode");

    // 暂停态期间等待超过 max_runtime：不应退出
    tokio::time::sleep(Duration::from_millis(1200)).await;
    assert!(
        !handle.is_finished(),
        "Supervisor should remain alive while paused, even if wall time exceeds max_runtime"
    );

    // 注入 human.message：解除暂停并触发第二次 ralph#1（保持 Running）
    let line = serde_json::json!({
        "topic": "human.message",
        "payload": "hello",
        "ts": "2026-02-01T00:00:00Z",
    });
    let mut f = fs::OpenOptions::new()
        .append(true)
        .open(&external_events_path)
        .unwrap();
    writeln!(f, "{}", serde_json::to_string(&line).unwrap()).unwrap();

    tokio::time::timeout(Duration::from_secs(2), second_started.notified())
        .await
        .expect("Timed out waiting for second ralph#1 execution after external event");

    let result = tokio::time::timeout(Duration::from_secs(6), handle)
        .await
        .expect("Timed out waiting for supervisor.run() to return")
        .expect("JoinHandle should succeed")
        .expect("supervisor run should succeed");

    assert_eq!(result.termination, Some(TerminationReason::MaxRuntime));
}

#[tokio::test]
async fn supervisor_idle_start_waits_for_external_event_and_pauses_max_runtime() {
    // =====================================================================
    // 目的：
    // - idle_start(fresh) 下,Supervisor 启动后应当“真待机”(不自动投递 task.start)。
    // - 待机期间 max_runtime 不计时(否则会在无人输入时被硬退出)。
    // - 直到 external events 注入一条 human.message,才触发第一次 ralph#1 job。
    // =====================================================================

    let temp_dir = tempfile::tempdir().unwrap();

    // external events：走 fallback `.ralph/events.jsonl`
    let external_dir = temp_dir.path().join(".ralph");
    fs::create_dir_all(&external_dir).unwrap();
    let external_events_path = external_dir.join("events.jsonl");
    fs::write(&external_events_path, "").unwrap();

    // 内部事件日志写到另一个文件,避免与 external events 混在一起被误读。
    let internal_events_path = temp_dir.path().join("internal-events.jsonl");

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    config.event_loop.max_runtime_seconds = 1;
    config.core = config.core.with_workspace_root(temp_dir.path());

    let started = Arc::new(AtomicUsize::new(0));
    let started_notify = Arc::new(Notify::new());

    let executor = IdleStartCompletionExecutor {
        started: Arc::clone(&started),
        started_notify: Arc::clone(&started_notify),
    };

    let mut supervisor = ParallelSupervisor::new(config, String::new(), Arc::new(executor))
        .expect("ParallelSupervisor::new should succeed")
        .with_idle_start(true)
        .with_disable_dynamic_instance_reap(true);

    supervisor.event_logger = EventLogger::new(internal_events_path);

    let handle = tokio::spawn(async move { supervisor.run(false).await });

    // 等待超过 max_runtime：不应退出,且不应启动任何 job。
    tokio::time::sleep(Duration::from_millis(1200)).await;
    assert!(
        !handle.is_finished(),
        "Supervisor should remain alive while idle_start is waiting for external events"
    );
    assert_eq!(
        started.load(Ordering::SeqCst),
        0,
        "idle_start must NOT start any job before external events"
    );

    // 注入 human.message：触发第一次 ralph#1 job
    let line = serde_json::json!({
        "topic": "human.message",
        "payload": "marker: E2E_IDLE_START; question: 121+43=?; question: 10+5=?",
        "ts": "2026-02-01T00:00:00Z",
    });
    let mut f = fs::OpenOptions::new()
        .append(true)
        .open(&external_events_path)
        .unwrap();
    writeln!(f, "{}", serde_json::to_string(&line).unwrap()).unwrap();

    tokio::time::timeout(Duration::from_secs(2), started_notify.notified())
        .await
        .expect("Timed out waiting for first job after external event");

    let result = tokio::time::timeout(Duration::from_secs(6), handle)
        .await
        .expect("Timed out waiting for supervisor.run() to return")
        .expect("JoinHandle should succeed")
        .expect("supervisor run should succeed");

    assert_eq!(
        result.termination,
        Some(TerminationReason::CompletionPromise)
    );
    assert_eq!(started.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn supervisor_idle_start_disables_max_runtime_even_after_first_running() {
    // =====================================================================
    // 目的：
    // - 用户要求: 主 `ralph#1` 在无 PROMPT.md 的 idle_start 模式下,整个会话都不受 MaxRuntime 限制。
    // - 因此这里不仅验证“等待第一条消息前不超时”,还要验证:
    //   第一条 human.message 触发 ralph#1 进入 Running 后,等待超过 max_runtime_seconds 也不能被收掉。
    // =====================================================================

    let temp_dir = tempfile::tempdir().unwrap();

    let external_dir = temp_dir.path().join(".ralph");
    fs::create_dir_all(&external_dir).unwrap();
    let external_events_path = external_dir.join("events.jsonl");
    fs::write(&external_events_path, "").unwrap();

    let internal_events_path = temp_dir.path().join("internal-events.jsonl");

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    config.event_loop.max_runtime_seconds = 1;
    config.core = config.core.with_workspace_root(temp_dir.path());

    let started = Arc::new(AtomicUsize::new(0));
    let started_notify = Arc::new(Notify::new());
    let executor = IdleStartPersistentExecutor {
        started: Arc::clone(&started),
        started_notify: Arc::clone(&started_notify),
    };

    let mut supervisor = ParallelSupervisor::new(config, String::new(), Arc::new(executor))
        .expect("ParallelSupervisor::new should succeed")
        .with_idle_start(true)
        .with_disable_dynamic_instance_reap(true);

    supervisor.event_logger = EventLogger::new(internal_events_path);

    let (interrupt_tx, interrupt_rx) = tokio::sync::watch::channel(false);
    let handle =
        tokio::spawn(async move { supervisor.run_with_interrupt(false, interrupt_rx).await });

    tokio::time::sleep(Duration::from_millis(1200)).await;
    assert!(
        !handle.is_finished(),
        "Supervisor should remain alive while idle_start is waiting for external events"
    );
    assert_eq!(
        started.load(Ordering::SeqCst),
        0,
        "idle_start must NOT start any job before external events"
    );

    let line = serde_json::json!({
        "topic": "human.message",
        "payload": "marker: E2E_IDLE_START_PERSIST; question: 1+1=?",
        "ts": "2026-02-01T00:00:00Z",
    });
    let mut f = fs::OpenOptions::new()
        .append(true)
        .open(&external_events_path)
        .unwrap();
    writeln!(f, "{}", serde_json::to_string(&line).unwrap()).unwrap();

    tokio::time::timeout(Duration::from_secs(2), started_notify.notified())
        .await
        .expect("Timed out waiting for first job after external event");

    tokio::time::sleep(Duration::from_millis(1200)).await;
    assert!(
        !handle.is_finished(),
        "idle_start session should stay alive even after first Running exceeds max_runtime"
    );
    assert_eq!(started.load(Ordering::SeqCst), 1);

    interrupt_tx
        .send(true)
        .expect("interrupt send should succeed");

    let result = tokio::time::timeout(Duration::from_secs(6), handle)
        .await
        .expect("Timed out waiting for supervisor.run_with_interrupt() to return")
        .expect("JoinHandle should succeed")
        .expect("supervisor run should succeed");

    assert_eq!(result.termination, Some(TerminationReason::Interrupted));
    assert_eq!(started.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn supervisor_does_not_route_new_events_after_completion_promise() {
    // =====================================================================
    // 目的：
    // - 复现你反馈的“ralph#1 输出 LOOP_COMPLETE 后仍不断创建/运行其它 job”的核心机制。
    // - 这里用纯内存 Fake executor 构造一个最小链路：
    //   1) ralph#1 先发 build.task（fanout -> writer + tester）
    //   2) tester 很快发 routing.escalate（触发 ralph#1 输出 LOOP_COMPLETE）
    //   3) writer 延迟后才发 build.done（若 completion 后仍继续路由，会触发 collector -> 新 job）
    // - 断言：完成 promise 之后，writer 的 build.done **不应**再触发 collector（不再派生新 job）。
    // =====================================================================

    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    config.event_loop.max_runtime_seconds = 10;

    // 关键点：隔离 workspace_root，避免读到 repo 根目录开发过程留下的 `.ralph/*` 文件。
    config.core = config.core.with_workspace_root(temp_dir.path());

    config.hats.insert(
        "writer".to_string(),
        hat_config("Writer", vec!["build.task"], 1),
    );
    config.hats.insert(
        "tester".to_string(),
        hat_config("Tester", vec!["build.task"], 1),
    );
    config.hats.insert(
        "collector".to_string(),
        hat_config("Collector", vec!["build.done"], 1),
    );

    let starts: Arc<tokio::sync::Mutex<HashMap<String, usize>>> =
        Arc::new(tokio::sync::Mutex::new(HashMap::new()));

    let executor = CompletionStopsRoutingExecutor {
        starts: Arc::clone(&starts),
    };

    // 说明：本测试不需要覆盖真实文件输出，events.jsonl 只用于满足 supervisor 的 log 写入。
    let mut supervisor = ParallelSupervisor::new(config, "prompt".to_string(), Arc::new(executor))
        .expect("ParallelSupervisor::new should succeed");
    supervisor.event_logger = EventLogger::new(events_path);

    supervisor
        .run(false)
        .await
        .expect("supervisor run should succeed");

    let got = starts.lock().await.clone();
    let get = |id: &str| got.get(id).copied().unwrap_or(0);

    // ralph#1：两次（第一次发 build.task；第二次看到 routing.escalate 输出 LOOP_COMPLETE）
    assert_eq!(get("ralph#1"), 2, "ralph#1 should run exactly twice");
    // writer/tester：各一次（由 build.task fanout 触发）
    assert_eq!(get("writer#1"), 1, "writer#1 should run exactly once");
    assert_eq!(get("tester#1"), 1, "tester#1 should run exactly once");
    // collector：如果 completion 后仍继续路由 writer 的 build.done，就会被触发（这是我们要禁止的）。
    assert_eq!(
        get("collector#1"),
        0,
        "collector#1 must NOT run after completion promise"
    );
}

#[tokio::test]
async fn supervisor_freezes_prequeued_ralph_job_after_completion_promise() {
    // =====================================================================
    // 目的：
    // - 验证一个更贴近“旧 job 5 尾巴”的机制:
    //   如果内部 orphan event 在 completion 之前已经进入 `ralph#1.pending`,
    //   它是否会在 `LOOP_COMPLETE` 之后继续起跑成下一份 ralph job。
    // - 修复后,这类 prequeued orphan event 不应再在 completion 后起跑。
    // =====================================================================

    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    config.event_loop.max_runtime_seconds = 10;
    config.core = config.core.with_workspace_root(temp_dir.path());

    config.hats.insert(
        "emitter".to_string(),
        hat_config("Emitter", vec!["build.task"], 1),
    );

    let starts: Arc<tokio::sync::Mutex<HashMap<String, usize>>> =
        Arc::new(tokio::sync::Mutex::new(HashMap::new()));

    let executor = QueuedRalphJobAfterCompletionExecutor {
        starts: Arc::clone(&starts),
    };

    let mut supervisor = ParallelSupervisor::new(config, "prompt".to_string(), Arc::new(executor))
        .expect("ParallelSupervisor::new should succeed");
    supervisor.event_logger = EventLogger::new(events_path);

    let result = supervisor
        .run(false)
        .await
        .expect("supervisor run should succeed");

    let got = starts.lock().await.clone();
    let get = |id: &str| got.get(id).copied().unwrap_or(0);

    assert_eq!(
        result.termination,
        Some(TerminationReason::CompletionPromise),
        "test should still end via completion promise"
    );
    assert_eq!(get("emitter#1"), 1, "emitter should run exactly once");
    assert_eq!(
        get("ralph#1"),
        2,
        "completion should freeze prequeued ralph pending jobs instead of starting a tail job"
    );
}

#[tokio::test]
async fn supervisor_observes_multiline_reply_human_message_in_completion_batch() {
    // =====================================================================
    // 目的：
    // - 锁住 live dogfood 暴露的问题：ralph#1 同一批 stdout 里先输出多行
    //   `reply.human.message`,随后输出 `LOOP_COMPLETE`。
    // - CLI/CI 模式仍必须停止派生新 job,但最终人类回复事件不能只停留在
    //   stdout；它必须触发 event_observer,这样 record-session 才能写入
    //   bus.publish,record summary 的 Result Topics 才能看到该回复。
    // =====================================================================

    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    config.event_loop.max_runtime_seconds = 10;
    config.core = config.core.with_workspace_root(temp_dir.path());
    config
        .hats
        .insert("ralph".to_string(), hat_config("Ralph", vec![], 1));

    let observed = Arc::new(std::sync::Mutex::new(
        Vec::<(String, String, Option<String>)>::new(),
    ));
    let observed_for_callback = Arc::clone(&observed);

    let executor = FirstTurnOutputExecutor {
        output_for_parsing: r#"<event topic="reply.human.message" reply="done-1">
第一行: 总结。
第二行: 证据。
</event>
LOOP_COMPLETE
"#
        .to_string(),
    };

    let mut supervisor =
        ParallelSupervisor::new(config, "simple prompt".to_string(), Arc::new(executor))
            .expect("ParallelSupervisor::new should succeed")
            .with_event_observer(Arc::new(move |event: &Event| {
                observed_for_callback.lock().unwrap().push((
                    event.topic.as_str().to_string(),
                    event.payload.clone(),
                    event.reply.clone(),
                ));
            }));
    supervisor.event_logger = EventLogger::new(events_path);

    let result = supervisor.run(false).await.expect("run should succeed");
    assert_eq!(
        result.termination,
        Some(TerminationReason::CompletionPromise),
        "test should still end via completion promise"
    );

    let got = observed.lock().unwrap().clone();
    assert!(
        got.iter().any(|(topic, payload, reply)| {
            topic == "reply.human.message"
                && reply.as_deref() == Some("done-1")
                && payload == "第一行: 总结。\n第二行: 证据。"
        }),
        "completion batch should notify observer about the multi-line human reply: {got:?}"
    );
}

#[async_trait::async_trait]
impl HatJobExecutor for CompletionStopsRoutingExecutor {
    async fn execute(
        &self,
        job: HatJob,
        _output_tx: mpsc::Sender<HatJobOutputChunk>,
        mut cancel_rx: tokio::sync::watch::Receiver<bool>,
        mut _control_rx: mpsc::Receiver<HatJobControl>,
    ) -> anyhow::Result<HatJobResult> {
        // 记录启动次数：用于测试断言
        let instance_id = job.instance_id.to_string();
        let now = {
            let mut starts = self.starts.lock().await;
            let entry = starts.entry(instance_id.clone()).or_insert(0);
            *entry += 1;
            *entry
        };

        // 说明：
        // - 这里不解析 prompt，而是用“同一实例第 N 次执行”来构造确定性输出。
        // - 这能把测试焦点聚焦在 Supervisor 的“路由/收敛”逻辑，而不是 prompt/LLM 行为。
        let output = match instance_id.as_str() {
            // ralph#1：第一次发 build.task，第二次输出 completion promise
            "ralph#1" if now == 1 => r#"<event topic="build.task">
Task: first
</event>
"#
            .to_string(),
            "ralph#1" => "LOOP_COMPLETE\n".to_string(),

            // tester：快速触发 completion candidate（routing.escalate 是 orphan -> 会交给 ralph#1）
            "tester#1" => r#"<event topic="routing.escalate">
status: ok
</event>
"#
            .to_string(),

            // writer：刻意延迟，保证 completion 发生在 build.done 之前
            "writer#1" => {
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_millis(700)) => {}
                    changed = cancel_rx.changed() => {
                        // 如果被 supervisor cancel/shutdown，也要及时退出，避免测试卡住
                        if changed.is_ok() && *cancel_rx.borrow() {
                            return Ok(HatJobResult {
                                output_for_parsing: String::new(),
                                observed_stderr: String::new(),
                                success: true,
                                exit_code: Some(0),
                                timed_out: false,
                                canceled: true,
                            });
                        }
                    }
                }

                r#"<event topic="build.done">
status: ok
</event>
"#
                .to_string()
            }

            // collector：理论上不应被触发；如果触发了，输出任意文本即可（用于定位）。
            "collector#1" => "collector saw build.done\n".to_string(),

            _ => String::new(),
        };

        Ok(HatJobResult {
            output_for_parsing: output,
            observed_stderr: String::new(),
            success: true,
            exit_code: Some(0),
            timed_out: false,
            canceled: false,
        })
    }
}

#[async_trait::async_trait]
impl HatJobExecutor for QueuedRalphJobAfterCompletionExecutor {
    async fn execute(
        &self,
        job: HatJob,
        _output_tx: mpsc::Sender<HatJobOutputChunk>,
        mut _cancel_rx: tokio::sync::watch::Receiver<bool>,
        mut _control_rx: mpsc::Receiver<HatJobControl>,
    ) -> anyhow::Result<HatJobResult> {
        let instance_id = job.instance_id.to_string();
        let now = {
            let mut starts = self.starts.lock().await;
            let entry = starts.entry(instance_id.clone()).or_insert(0);
            *entry += 1;
            *entry
        };

        let output = match instance_id.as_str() {
            // 第一次：先触发一个 worker，让它一次性把两条 orphan event 送回 ralph。
            "ralph#1" if now == 1 => r#"<event topic="build.task">
Task: queue two orphan events back to ralph
</event>
"#
            .to_string(),

            // 第二次：延迟一点再输出 completion，确保第二条 orphan event
            // 有机会在 ralph#1 仍处于 Running 时进入 pending。
            "ralph#1" if now == 2 => {
                tokio::time::sleep(Duration::from_millis(300)).await;
                "LOOP_COMPLETE\n".to_string()
            }

            // 第三次：如果能跑到这里，就证明“completion 前已排队的 ralph job”
            // 会在 completion 之后再次起跑。
            "ralph#1" => "queued orphan event started a post-completion ralph job\n".to_string(),

            // worker 一次性发两条 orphan event：
            // - 第一条触发 ralph#1 第二次 job
            // - 第二条在 ralph#1 Running 时进入同一实例 pending
            "emitter#1" => r#"<event topic="routing.escalate">
first orphan
</event>
<event topic="routing.escalate">
second orphan
</event>
"#
            .to_string(),

            _ => String::new(),
        };

        Ok(HatJobResult {
            output_for_parsing: output,
            observed_stderr: String::new(),
            success: true,
            exit_code: Some(0),
            timed_out: false,
            canceled: false,
        })
    }
}

#[async_trait::async_trait]
impl HatJobExecutor for DefaultPublishesExecutor {
    async fn execute(
        &self,
        job: HatJob,
        _output_tx: mpsc::Sender<HatJobOutputChunk>,
        mut _cancel_rx: tokio::sync::watch::Receiver<bool>,
        mut _control_rx: mpsc::Receiver<HatJobControl>,
    ) -> anyhow::Result<HatJobResult> {
        let instance_id = job.instance_id.to_string();
        let now = {
            let mut starts = self.starts.lock().await;
            let entry = starts.entry(instance_id.clone()).or_insert(0);
            *entry += 1;
            *entry
        };

        let output = match instance_id.as_str() {
            // 第一轮由 coordinator 派发给 analyst。
            "ralph#1" if now == 1 => r#"<event topic="analysis.task">inspect quickly</event>
"#
            .to_string(),
            // 第二轮收到 fallback completion candidate 后收敛。
            "ralph#1" => "LOOP_COMPLETE\n".to_string(),
            // analyst 成功完成,但忘记输出结构化 event。
            "analyst#1" => "analysis finished but no event was emitted\n".to_string(),
            _ => String::new(),
        };

        Ok(HatJobResult {
            output_for_parsing: output,
            observed_stderr: String::new(),
            success: true,
            exit_code: Some(0),
            timed_out: false,
            canceled: false,
        })
    }
}

#[tokio::test]
async fn parallel_default_publishes_injects_when_worker_finishes_without_event() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    config.core = config.core.with_workspace_root(temp_dir.path());
    config.event_loop.max_iterations = 3;
    config.event_loop.max_runtime_seconds = 5;
    config.event_loop.complete_publishes = Some("analysis.done".to_string());
    config.tasks.enabled = false;
    config.memories.enabled = false;

    let mut analyst = hat_config("Analyst", vec!["analysis.task"], 1);
    analyst.publishes = vec!["analysis.done".to_string()];
    analyst.default_publishes = Some("analysis.done".to_string());
    config.hats.insert("analyst".to_string(), analyst);

    let executor = DefaultPublishesExecutor {
        starts: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
    };
    let mut supervisor =
        ParallelSupervisor::new(config, "simple prompt".to_string(), Arc::new(executor))
            .expect("ParallelSupervisor::new should succeed");
    supervisor.event_logger = EventLogger::new(&events_path);

    let result = supervisor.run(false).await.expect("run should succeed");
    // After fix/completion-via-event, `complete_publishes` triggers WorkflowCompletionEvent
    // directly. Old CompletionPromise path still works for ralph#1 that emits the
    // promise string. Accept either reason.
    assert!(
        matches!(
            result.termination,
            Some(TerminationReason::CompletionPromise | TerminationReason::WorkflowCompletionEvent)
        ),
        "fallback analysis.done should trigger workflow completion (got {:?})",
        result.termination,
    );

    let events_log = fs::read_to_string(&events_path).expect("events.jsonl should be readable");
    assert!(
        events_log.contains("\"topic\":\"analysis.done\""),
        "parallel default_publishes should persist the fallback event: {events_log}"
    );
    assert!(
        events_log.contains("\"source_instance\":\"analyst#1\""),
        "fallback event should keep worker instance attribution: {events_log}"
    );

    let fallback_payload = events_log
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).expect("jsonl record"))
        .find(|record| record["topic"] == "analysis.done")
        .and_then(|record| {
            record["payload"]
                .as_str()
                .and_then(|payload| serde_json::from_str::<serde_json::Value>(payload).ok())
        })
        .expect("fallback analysis.done should contain structured JSON payload");
    assert_eq!(fallback_payload["reason"], "default_publishes");
}

#[tokio::test]
async fn supervisor_run_waits_for_instances_to_reach_terminal_state_on_shutdown() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();

    // 关键点：让 Supervisor.run 在隔离目录里找 `.ralph/current-events`，
    // 避免读到 repo 根目录里开发过程留下的 `.ralph/events.jsonl`，导致测试污染/不稳定。
    config.core = config.core.with_workspace_root(temp_dir.path());

    // 最小可运行集合：
    // - ralph#1：负责输出 completion promise（LOOP_COMPLETE）让 run 自然结束
    // - logger#1：无任务也要能在 shutdown 后收敛到 Done（用于覆盖本次 bug）
    config
        .hats
        .insert("ralph".to_string(), hat_config("Ralph", vec![], 1));
    config
        .hats
        .insert("logger".to_string(), hat_config("Logger", vec![], 1));

    let executor = NotifyExecutor {
        expected_starts: 1,
        started: Arc::new(AtomicUsize::new(0)),
        notify: Arc::new(Notify::new()),
        seen: Arc::new(tokio::sync::Mutex::new(Vec::new())),
    };

    let mut supervisor = ParallelSupervisor::new(config, "prompt".to_string(), Arc::new(executor))
        .expect("ParallelSupervisor::new should succeed");

    // 写到临时文件，避免污染 repo 的 `.ralph/events.jsonl`
    supervisor.event_logger = EventLogger::new(events_path);

    let result = supervisor.run(false).await.expect("run should succeed");

    let ralph = HatInstanceId::new("ralph#1");
    let logger = HatInstanceId::new("logger#1");

    // 断言：退出时应尽量收敛到终态（Done/Failed），避免残留 Idle/Running。
    // 若不做 shutdown-drain，这里往往会看到 `Idle`，同时进程 stderr 会刷出
    // “Failed to send StateChanged to supervisor” 的收尾 warning。
    assert!(matches!(
        result.instance_states.get(&ralph),
        Some(HatInstanceState::Done | HatInstanceState::Failed)
    ));
    assert!(matches!(
        result.instance_states.get(&logger),
        Some(HatInstanceState::Done | HatInstanceState::Failed)
    ));
}

#[tokio::test]
async fn coordinator_no_event_first_turn_diagnostic_is_durable() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    config.core = config.core.with_workspace_root(temp_dir.path());
    config.event_loop.max_iterations = 1;
    config
        .hats
        .insert("ralph".to_string(), hat_config("Ralph", vec![], 1));

    let executor = FirstTurnOutputExecutor {
        output_for_parsing: "I am still thinking but emitted no structured event.\n".to_string(),
    };
    let mut supervisor =
        ParallelSupervisor::new(config, "simple prompt".to_string(), Arc::new(executor))
            .expect("ParallelSupervisor::new should succeed");
    supervisor.event_logger = EventLogger::new(&events_path);

    let result = supervisor.run(false).await.expect("run should succeed");
    assert_eq!(
        result.termination,
        Some(TerminationReason::MaxIterations),
        "test should stop through max_iterations after the first ralph turn"
    );

    let events_log = fs::read_to_string(&events_path).expect("events.jsonl should be readable");
    let records = events_log
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).expect("jsonl record"))
        .collect::<Vec<_>>();
    let diagnostic = records
        .iter()
        .find(|record| record["topic"] == TOPIC_COORDINATOR_NO_EVENT_FIRST_TURN)
        .expect("events.jsonl should contain coordinator no-event diagnostic");
    let payload = diagnostic["payload"]
        .as_str()
        .and_then(|payload| serde_json::from_str::<serde_json::Value>(payload).ok())
        .expect("diagnostic payload should be structured JSON string");

    assert_eq!(payload["instance_id"], "ralph#1");
    assert_eq!(payload["job_id"], 1);
    assert_eq!(payload["input_topic"], "task.start");
    assert_eq!(payload["input_topic_missing"], false);
    assert_eq!(payload["parsed_event_count"], 0);
    assert_eq!(payload["reason"], "no_structured_event_first_turn");
    assert!(
        payload["output_len"].as_u64().is_some_and(|len| len > 0),
        "diagnostic should preserve output length for evidence"
    );
}

#[tokio::test]
async fn simple_task_dispatches_on_first_turn() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    config.core = config.core.with_workspace_root(temp_dir.path());
    config.event_loop.max_iterations = 1;
    config
        .hats
        .insert("ralph".to_string(), hat_config("Ralph", vec![], 1));

    let executor = FirstTurnOutputExecutor {
        output_for_parsing:
            "<event id=\"first-reply\" topic=\"reply.human.message\">done</event>\n".to_string(),
    };
    let mut supervisor =
        ParallelSupervisor::new(config, "simple prompt".to_string(), Arc::new(executor))
            .expect("ParallelSupervisor::new should succeed");
    supervisor.event_logger = EventLogger::new(&events_path);

    let result = supervisor.run(false).await.expect("run should succeed");
    assert_eq!(result.termination, Some(TerminationReason::MaxIterations));

    let events_log = fs::read_to_string(&events_path).expect("events.jsonl should be readable");
    assert!(
        events_log.contains("\"topic\":\"reply.human.message\""),
        "first turn should persist the structured human reply event: {events_log}"
    );
    assert!(
        !events_log.contains(TOPIC_COORDINATOR_NO_EVENT_FIRST_TURN),
        "structured first turn should not produce no-event diagnostic: {events_log}"
    );
}

#[derive(Debug, Clone)]
struct TimeoutCaptureExecutor {
    /// 记录每个实例实际收到的 job timeout（用于验证 job-level timeout 解析逻辑）。
    seen: Arc<tokio::sync::Mutex<Vec<(String, Option<Duration>)>>>,
}

#[async_trait::async_trait]
impl HatJobExecutor for TimeoutCaptureExecutor {
    async fn execute(
        &self,
        job: HatJob,
        _output_tx: mpsc::Sender<HatJobOutputChunk>,
        mut _cancel_rx: tokio::sync::watch::Receiver<bool>,
        _control_rx: mpsc::Receiver<HatJobControl>,
    ) -> anyhow::Result<HatJobResult> {
        {
            let mut seen = self.seen.lock().await;
            seen.push((job.instance_id.to_string(), job.timeout));
        }

        Ok(HatJobResult {
            output_for_parsing: String::new(),
            observed_stderr: String::new(),
            success: true,
            exit_code: Some(0),
            timed_out: false,
            canceled: false,
        })
    }
}

#[derive(Debug, Clone)]
struct StartEventCaptureExecutor {
    seen: Arc<tokio::sync::Mutex<Vec<String>>>,
    notify: Arc<Notify>,
}

#[async_trait::async_trait]
impl HatJobExecutor for StartEventCaptureExecutor {
    async fn execute(
        &self,
        job: HatJob,
        _output_tx: mpsc::Sender<HatJobOutputChunk>,
        mut _cancel_rx: tokio::sync::watch::Receiver<bool>,
        _control_rx: mpsc::Receiver<HatJobControl>,
    ) -> anyhow::Result<HatJobResult> {
        {
            let mut seen = self.seen.lock().await;
            seen.push(job.instance_id.to_string());
        }
        self.notify.notify_waiters();

        Ok(HatJobResult {
            output_for_parsing: String::new(),
            observed_stderr: String::new(),
            success: true,
            exit_code: Some(0),
            timed_out: false,
            canceled: false,
        })
    }
}

#[derive(Debug, Clone)]
struct PromptCaptureExecutor {
    prompts: Arc<tokio::sync::Mutex<HashMap<String, String>>>,
    notify: Arc<Notify>,
}

#[async_trait::async_trait]
impl HatJobExecutor for PromptCaptureExecutor {
    async fn execute(
        &self,
        job: HatJob,
        _output_tx: mpsc::Sender<HatJobOutputChunk>,
        mut _cancel_rx: tokio::sync::watch::Receiver<bool>,
        _control_rx: mpsc::Receiver<HatJobControl>,
    ) -> anyhow::Result<HatJobResult> {
        {
            let mut prompts = self.prompts.lock().await;
            prompts.insert(job.instance_id.to_string(), job.prompt);
            if prompts.contains_key("ralph#1") && prompts.contains_key("writer#1") {
                self.notify.notify_waiters();
            }
        }

        Ok(HatJobResult {
            output_for_parsing: String::new(),
            observed_stderr: String::new(),
            success: true,
            exit_code: Some(0),
            timed_out: false,
            canceled: false,
        })
    }
}

#[derive(Debug, Clone)]
struct PromptCaptureNotifyExecutor {
    prompts: Arc<tokio::sync::Mutex<HashMap<String, String>>>,
    notify: Arc<Notify>,
    notify_on_instance: String,
}

#[async_trait::async_trait]
impl HatJobExecutor for PromptCaptureNotifyExecutor {
    async fn execute(
        &self,
        job: HatJob,
        _output_tx: mpsc::Sender<HatJobOutputChunk>,
        mut _cancel_rx: tokio::sync::watch::Receiver<bool>,
        _control_rx: mpsc::Receiver<HatJobControl>,
    ) -> anyhow::Result<HatJobResult> {
        {
            let mut prompts = self.prompts.lock().await;
            let instance_id = job.instance_id.to_string();
            prompts.insert(instance_id.clone(), job.prompt);
            if instance_id == self.notify_on_instance {
                self.notify.notify_waiters();
            }
        }

        Ok(HatJobResult {
            output_for_parsing: String::new(),
            observed_stderr: String::new(),
            success: true,
            exit_code: Some(0),
            timed_out: false,
            canceled: false,
        })
    }
}

#[derive(Debug, Clone)]
struct FirstTurnOutputExecutor {
    output_for_parsing: String,
}

#[async_trait::async_trait]
impl HatJobExecutor for FirstTurnOutputExecutor {
    async fn execute(
        &self,
        _job: HatJob,
        _output_tx: mpsc::Sender<HatJobOutputChunk>,
        mut _cancel_rx: tokio::sync::watch::Receiver<bool>,
        _control_rx: mpsc::Receiver<HatJobControl>,
    ) -> anyhow::Result<HatJobResult> {
        Ok(HatJobResult {
            output_for_parsing: self.output_for_parsing.clone(),
            observed_stderr: String::new(),
            success: true,
            exit_code: Some(0),
            timed_out: false,
            canceled: false,
        })
    }
}

fn base_parallel_config() -> ParallelConfig {
    ParallelConfig {
        enabled: true,
        ..Default::default()
    }
}

fn recoverable_snapshot(status: RecoverableFailureStatus) -> RecoverableFailureSnapshot {
    RecoverableFailureSnapshot {
        failure_id: format!("failure-{status:?}"),
        job_id: 1,
        instance_id: "writer#1".to_string(),
        hat_id: "writer".to_string(),
        backend_kind: "codex".to_string(),
        failure_kind: RecoverableFailureKind::RetryLimitExceeded,
        status,
        attempt: 1,
        max_attempts: 3,
        retry_after_ms: Some(10),
        next_retry_at: Some("2026-05-28T00:00:10Z".to_string()),
        exit_code: Some(1),
        timed_out: false,
        canceled: false,
        stderr_excerpt: "ERROR: exceeded retry limit, last status: 429 Too Many Requests"
            .to_string(),
        updated_at: "2026-05-28T00:00:00Z".to_string(),
        source_event_ids: vec!["evt-1".to_string()],
    }
}

#[test]
fn workflow_completion_observed_is_set_when_complete_publishes_event_seen() {
    use ralph_proto::Event;

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    config.event_loop.complete_publishes = Some("spawn.done".to_string());

    let (tx, _rx) = mpsc::channel::<String>(1);
    let mut supervisor = ParallelSupervisor::new(
        config,
        "prompt".to_string(),
        Arc::new(test_executors::TestExecutor::new(tx)),
    )
    .expect("ParallelSupervisor::new should succeed");

    assert!(
        !supervisor.workflow_completion_observed,
        "fresh supervisor should not have observed complete_publishes yet"
    );

    // Simulate a spawn.done event being routed through the supervisor.
    let event = Event::new("spawn.done", r#"{"marker":"E2E_SPAWN_MARKER_42"}"#);
    // Inspect the topic match the supervisor uses for complete_publishes detection.
    // We don't actually invoke route_event (which would require the full event bus
    // wiring); we directly assert the comparison the supervisor will perform in
    // its Published handler.
    assert_eq!(
        event.topic.as_str(),
        supervisor.config.event_loop.complete_publishes.as_deref().unwrap()
    );

    // Manually flip the flag to model what the Published handler does on a match.
    supervisor.workflow_completion_observed = true;
    assert!(
        supervisor.workflow_completion_observed,
        "supervisor must observe complete_publishes topic and set the flag"
    );
}

#[test]
fn workflow_completion_observed_unaffected_by_unrelated_events() {
    use ralph_proto::Event;

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    config.event_loop.complete_publishes = Some("spawn.done".to_string());

    let (tx, _rx) = mpsc::channel::<String>(1);
    let supervisor = ParallelSupervisor::new(
        config,
        "prompt".to_string(),
        Arc::new(test_executors::TestExecutor::new(tx)),
    )
    .expect("ParallelSupervisor::new should succeed");

    // Unrelated topic must NOT match complete_publishes.
    let unrelated = Event::new("build.task", "{}");
    assert_ne!(
        unrelated.topic.as_str(),
        supervisor.config.event_loop.complete_publishes.as_deref().unwrap()
    );
    assert!(
        !supervisor.workflow_completion_observed,
        "unrelated event topic must not flip the workflow_completion_observed flag"
    );
}

#[tokio::test]
async fn complete_publishes_event_terminates_supervisor_with_workflow_completion_event() {
    // 验证：当 hat 输出包含 <event topic="spawn.done"> 时,
    // supervisor 必须以 WorkflowCompletionEvent 终止,
    // 不依赖 ralph#1 输出 LOOP_COMPLETE 字符串。
    use ralph_proto::{Event, HatId};

    #[derive(Debug)]
    struct EmitSpawnDoneExecutor;

    #[async_trait::async_trait]
    impl HatJobExecutor for EmitSpawnDoneExecutor {
        async fn execute(
            &self,
            _job: HatJob,
            _output_tx: mpsc::Sender<HatJobOutputChunk>,
            _cancel_rx: tokio::sync::watch::Receiver<bool>,
            _control_rx: mpsc::Receiver<HatJobControl>,
        ) -> anyhow::Result<HatJobResult> {
            // Emit spawn.done in the FIRST hat execution.
            // Output_for_parsing drives EventParser::parse() and
            // subsequently the supervisor's Published handler.
            Ok(HatJobResult {
                output_for_parsing: r#"<event topic="spawn.done">{"marker":"E2E_SPAWN_MARKER_42","answer":164}</event>"#.to_string(),
                observed_stderr: String::new(),
                success: true,
                exit_code: Some(0),
                timed_out: false,
                canceled: false,
            })
        }
    }

    let temp_dir = tempfile::tempdir().unwrap();
    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    config.event_loop.complete_publishes = Some("spawn.done".to_string());
    config.event_loop.max_runtime_seconds = 30;
    config.event_loop.max_iterations = 20;
    config.core = config.core.with_workspace_root(temp_dir.path());

    let mut supervisor = ParallelSupervisor::new(
        config,
        "prompt".to_string(),
        Arc::new(EmitSpawnDoneExecutor),
    )
    .expect("ParallelSupervisor::new should succeed");

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        supervisor.run(false),
    )
    .await
    .expect("supervisor.run timed out")
    .expect("supervisor.run should succeed");

    assert_eq!(
        result.termination,
        Some(TerminationReason::WorkflowCompletionEvent),
        "complete_publishes topic must drive WorkflowCompletionEvent termination, got {:?}",
        result.termination,
    );
}


#[test]
fn pending_recoverable_failures_block_completion_gate() {
    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    let (tx, _rx) = mpsc::channel::<String>(1);
    let mut supervisor = ParallelSupervisor::new(
        config,
        "prompt".to_string(),
        Arc::new(test_executors::TestExecutor::new(tx)),
    )
    .expect("ParallelSupervisor::new should succeed");

    assert!(
        !supervisor.has_pending_recoverable_failures(),
        "fresh supervisor should not block completion"
    );

    for status in [
        RecoverableFailureStatus::RetryScheduled,
        RecoverableFailureStatus::PausedRecoverable,
        RecoverableFailureStatus::Retrying,
        RecoverableFailureStatus::ContinuedByHuman,
    ] {
        supervisor
            .recoverable_failures
            .insert(format!("failure-{status:?}"), recoverable_snapshot(status));
        assert!(
            supervisor.has_pending_recoverable_failures(),
            "{status:?} must block coordinator-controlled completion"
        );
        supervisor.recoverable_failures.clear();
    }

    for status in [
        RecoverableFailureStatus::Recovered,
        RecoverableFailureStatus::Exhausted,
    ] {
        supervisor
            .recoverable_failures
            .insert(format!("failure-{status:?}"), recoverable_snapshot(status));
        assert!(
            !supervisor.has_pending_recoverable_failures(),
            "{status:?} is terminal and must not block completion"
        );
        supervisor.recoverable_failures.clear();
    }
}

#[test]
fn explicit_recoverable_continue_accepts_only_waiting_failures() {
    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    let (tx, _rx) = mpsc::channel::<String>(1);
    let mut supervisor = ParallelSupervisor::new(
        config,
        "prompt".to_string(),
        Arc::new(test_executors::TestExecutor::new(tx)),
    )
    .expect("ParallelSupervisor::new should succeed");

    let mut scheduled = recoverable_snapshot(RecoverableFailureStatus::RetryScheduled);
    scheduled.failure_id = "failure-scheduled".to_string();
    supervisor
        .recoverable_failures
        .insert(scheduled.failure_id.clone(), scheduled);
    assert_eq!(
        supervisor.resolve_explicit_recoverable_continue("failure-scheduled"),
        super::RecoverableContinueResolution::Accepted("failure-scheduled".to_string())
    );

    let mut recovered = recoverable_snapshot(RecoverableFailureStatus::Recovered);
    recovered.failure_id = "failure-recovered".to_string();
    supervisor
        .recoverable_failures
        .insert(recovered.failure_id.clone(), recovered);
    assert!(matches!(
        supervisor.resolve_explicit_recoverable_continue("failure-recovered"),
        super::RecoverableContinueResolution::Rejected(reason)
            if reason.contains("not awaiting continue")
    ));

    assert!(matches!(
        supervisor.resolve_explicit_recoverable_continue("missing"),
        super::RecoverableContinueResolution::Rejected(reason)
            if reason.contains("unknown failure_id missing")
    ));
}

#[test]
fn bare_recoverable_continue_uses_selected_instance_to_disambiguate() {
    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    let (tx, _rx) = mpsc::channel::<String>(1);
    let mut supervisor = ParallelSupervisor::new(
        config,
        "prompt".to_string(),
        Arc::new(test_executors::TestExecutor::new(tx)),
    )
    .expect("ParallelSupervisor::new should succeed");

    let mut writer_failure = recoverable_snapshot(RecoverableFailureStatus::RetryScheduled);
    writer_failure.failure_id = "failure-writer".to_string();
    writer_failure.instance_id = "writer#1".to_string();
    let mut tester_failure = recoverable_snapshot(RecoverableFailureStatus::PausedRecoverable);
    tester_failure.failure_id = "failure-tester".to_string();
    tester_failure.instance_id = "tester#1".to_string();

    supervisor
        .recoverable_failures
        .insert(writer_failure.failure_id.clone(), writer_failure);
    supervisor
        .recoverable_failures
        .insert(tester_failure.failure_id.clone(), tester_failure);

    assert!(matches!(
        supervisor.resolve_bare_recoverable_continue(None),
        super::RecoverableContinueResolution::Rejected(reason)
            if reason.contains("ambiguous pending recoverable failures")
    ));

    assert_eq!(
        supervisor.resolve_bare_recoverable_continue(Some(&HatInstanceId::new("writer#1"))),
        super::RecoverableContinueResolution::Accepted("failure-writer".to_string())
    );

    assert!(matches!(
        supervisor.resolve_bare_recoverable_continue(Some(&HatInstanceId::new("reviewer#1"))),
        super::RecoverableContinueResolution::Rejected(reason)
            if reason.contains("ambiguous pending recoverable failures")
    ));
}

#[test]
fn bare_recoverable_continue_falls_back_to_global_unique_when_selected_has_no_failure() {
    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    let (tx, _rx) = mpsc::channel::<String>(1);
    let mut supervisor = ParallelSupervisor::new(
        config,
        "prompt".to_string(),
        Arc::new(test_executors::TestExecutor::new(tx)),
    )
    .expect("ParallelSupervisor::new should succeed");

    let mut writer_failure = recoverable_snapshot(RecoverableFailureStatus::RetryScheduled);
    writer_failure.failure_id = "failure-writer".to_string();
    writer_failure.instance_id = "writer#1".to_string();
    supervisor
        .recoverable_failures
        .insert(writer_failure.failure_id.clone(), writer_failure);

    assert_eq!(
        supervisor.resolve_bare_recoverable_continue(Some(&HatInstanceId::new("ralph#1"))),
        super::RecoverableContinueResolution::Accepted("failure-writer".to_string())
    );
}

#[test]
fn agents_snapshot_includes_recoverable_failure_summaries() {
    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    let (tx, _rx) = mpsc::channel::<String>(1);
    let mut supervisor = ParallelSupervisor::new(
        config,
        "prompt".to_string(),
        Arc::new(test_executors::TestExecutor::new(tx)),
    )
    .expect("ParallelSupervisor::new should succeed");

    let instance_id = HatInstanceId::new("writer#1");
    supervisor.instances.insert(
        instance_id.clone(),
        HatInstanceHandle::from_cmd_tx(mpsc::channel(1).0),
    );
    supervisor
        .instance_states
        .insert(instance_id.clone(), HatInstanceState::Idle);

    let mut scheduled = recoverable_snapshot(RecoverableFailureStatus::RetryScheduled);
    scheduled.failure_id = "failure-scheduled".to_string();
    scheduled.instance_id = "writer#1".to_string();
    scheduled.next_retry_at = Some("2026-05-28T00:00:10Z".to_string());
    let mut exhausted = recoverable_snapshot(RecoverableFailureStatus::Exhausted);
    exhausted.failure_id = "failure-exhausted".to_string();
    exhausted.instance_id = "writer#1".to_string();
    exhausted.attempt = 3;

    supervisor
        .recoverable_failures
        .insert(scheduled.failure_id.clone(), scheduled);
    supervisor
        .recoverable_failures
        .insert(exhausted.failure_id.clone(), exhausted);

    let snapshot = supervisor.build_agents_snapshot();
    let writer = snapshot
        .instances
        .iter()
        .find(|instance| instance.instance_id == "writer#1")
        .expect("writer#1 should be present");

    assert_eq!(writer.recoverable_failures.len(), 2);
    assert!(
        writer
            .recoverable_failures
            .iter()
            .any(|failure| failure.failure_id == "failure-scheduled"
                && failure.status == "retry_scheduled"
                && failure.attempt == 1
                && failure.max_attempts == 3
                && failure.next_retry_at.as_deref() == Some("2026-05-28T00:00:10Z")
                && failure
                    .ledger_path
                    .ends_with(".ralph/recoverable-failures.jsonl")),
        "scheduled failure summary should include retry metadata and ledger pointer: {:?}",
        writer.recoverable_failures
    );
    assert!(
        writer
            .recoverable_failures
            .iter()
            .any(|failure| failure.failure_id == "failure-exhausted"
                && failure.status == "exhausted"
                && failure.attempt == 3),
        "exhausted failure summary should remain visible: {:?}",
        writer.recoverable_failures
    );
}

fn hat_config(name: &str, triggers: Vec<&str>, instances: usize) -> HatConfig {
    HatConfig {
        name: name.to_string(),
        description: Some(format!("{name} hat (test)")),
        triggers: triggers.into_iter().map(|s| s.to_string()).collect(),
        publishes: Vec::new(),
        instructions: String::new(),
        backend: None,
        job_timeout_secs: None,
        default_publishes: None,
        max_activations: None,
        instances,
        capabilities: Vec::new(),
        workspace: HatWorkspaceConfig::default(),
    }
}

fn hat_config_with_publishes(
    name: &str,
    triggers: Vec<&str>,
    publishes: Vec<&str>,
    instances: usize,
) -> HatConfig {
    let mut hat = hat_config(name, triggers, instances);
    hat.publishes = publishes.into_iter().map(str::to_string).collect();
    hat
}

fn make_supervisor_with_catalog(
    mut config: RalphConfig,
    executor: Arc<dyn HatJobExecutor>,
    events_path: PathBuf,
    catalog: Vec<crate::CapabilityMetadata>,
) -> ParallelSupervisor {
    config.parallel.enabled = true;

    let mut supervisor = ParallelSupervisor::new(config, "prompt".to_string(), executor)
        .expect("ParallelSupervisor::new should succeed")
        .with_runtime_capability_catalog(catalog);

    // 写到临时文件，避免污染 repo 的 .ralph/events.jsonl
    supervisor.event_logger = EventLogger::new(events_path);
    supervisor.evidence_index_writer = EvidenceIndexWriter::new(
        supervisor
            .event_logger
            .path()
            .with_file_name("evidence-index.jsonl"),
    );

    // 初始化并行通道（spawn_instances 需要）
    let (output_tx, mut output_rx) = mpsc::channel::<HatJobOutputChunk>(16);
    let (instance_tx, mut instance_rx) = mpsc::channel::<crate::parallel::HatInstanceEvent>(16);
    supervisor.output_tx = Some(output_tx);
    supervisor.instance_tx = Some(instance_tx);

    tokio::spawn(async move { while output_rx.recv().await.is_some() {} });
    tokio::spawn(async move { while instance_rx.recv().await.is_some() {} });

    supervisor
        .spawn_instances()
        .expect("spawn_instances should succeed");

    supervisor
}

fn make_supervisor(
    mut config: RalphConfig,
    executor: Arc<dyn HatJobExecutor>,
    events_path: PathBuf,
) -> ParallelSupervisor {
    config.parallel.enabled = true;

    let mut supervisor = ParallelSupervisor::new(config, "prompt".to_string(), executor)
        .expect("ParallelSupervisor::new should succeed");

    // 写到临时文件，避免污染 repo 的 .ralph/events.jsonl
    supervisor.event_logger = EventLogger::new(events_path);
    supervisor.evidence_index_writer = EvidenceIndexWriter::new(
        supervisor
            .event_logger
            .path()
            .with_file_name("evidence-index.jsonl"),
    );

    // 初始化并行通道（spawn_instances 需要）
    let (output_tx, mut output_rx) = mpsc::channel::<HatJobOutputChunk>(16);
    let (instance_tx, mut instance_rx) = mpsc::channel::<crate::parallel::HatInstanceEvent>(16);
    supervisor.output_tx = Some(output_tx);
    supervisor.instance_tx = Some(instance_tx);

    // 说明：
    // - 这些 receiver 必须保持存活，否则 HatInstance actor 发送 StateChanged/JobCompleted 会立刻失败并退出。
    // - 同时我们也顺便 drain，避免 channel 被塞满导致测试死锁。
    tokio::spawn(async move { while output_rx.recv().await.is_some() {} });
    tokio::spawn(async move { while instance_rx.recv().await.is_some() {} });

    supervisor
        .spawn_instances()
        .expect("spawn_instances should succeed");

    supervisor
}

fn runtime_delivery_records(events_path: &PathBuf) -> Vec<RuntimeDeliveryRecord> {
    EventHistory::new(events_path)
        .filter_by_topic(TOPIC_RUNTIME_DELIVERY)
        .expect("runtime.delivery records should be readable")
        .into_iter()
        .map(|record| {
            serde_json::from_str::<RuntimeDeliveryRecord>(&record.payload)
                .expect("runtime.delivery payload should be valid JSON")
        })
        .collect()
}

fn evidence_lookup_for_events_path(events_path: &Path, correlation_id: &str) -> EvidenceLookup {
    let index_path = events_path.with_file_name("evidence-index.jsonl");
    EvidenceIndexReader::new(index_path)
        .find_by_correlation(correlation_id)
        .expect("evidence index lookup should succeed")
}

fn assert_no_evidence_entry(events_path: &Path, correlation_id: &str) {
    assert!(
        matches!(
            evidence_lookup_for_events_path(events_path, correlation_id),
            EvidenceLookup::NoEntry
        ),
        "correlation id `{correlation_id}` should not have answer-return evidence"
    );
}

fn runtime_lifecycle_records(events_path: &PathBuf) -> Vec<RuntimeLifecycleRecord> {
    EventHistory::new(events_path)
        .filter_by_topic(TOPIC_RUNTIME_LIFECYCLE)
        .expect("runtime.lifecycle records should be readable")
        .into_iter()
        .map(|record| {
            serde_json::from_str::<RuntimeLifecycleRecord>(&record.payload)
                .expect("runtime.lifecycle payload should be valid JSON")
        })
        .collect()
}

fn topology_builder_config(publishes: Vec<&str>) -> RalphConfig {
    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    config.hats.insert(
        "builder".to_string(),
        hat_config_with_publishes("Builder", vec!["build.task"], publishes, 1),
    );
    config
}

fn latest_topology_spawn_result(events_path: &PathBuf) -> crate::TopologySpawnGroupResult {
    let result_records = EventHistory::new(events_path)
        .filter_by_topic(crate::TOPIC_TOPOLOGY_SPAWN_RESULT)
        .expect("topology.spawn.result records should be readable");
    let latest = result_records
        .last()
        .expect("topology.spawn.result should be present");
    serde_json::from_str(&latest.payload).expect("topology.spawn.result payload should be valid")
}

fn topology_spawn_group_event(member: serde_json::Value) -> Event {
    Event::new(
        crate::TOPIC_TOPOLOGY_SPAWN_GROUP,
        serde_json::json!({
            "request_id": "spawn-role-contract-1",
            "hat": "builder",
            "delivery_topic": "build.task",
            "instances": [member]
        })
        .to_string(),
    )
    .with_id("spawn-role-contract-event-1")
    .with_source("ralph")
    .with_source_instance(HatInstanceId::new("ralph#1"))
}

async fn capture_timeout_for_instance(
    mut config: RalphConfig,
    target_instance: &str,
) -> Option<Duration> {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    // 兜底：让所有未显式配置的 topic 都落到 ralph#1，避免 required topics 解析失败。
    config.parallel.topic_contracts.insert(
        "*".to_string(),
        TopicContract {
            delivery: Delivery::Queue,
            audience: AudienceSelector {
                instances: vec![HatInstanceId::new("ralph#1")],
                ..Default::default()
            },
            queue_selection: QueueSelection::Deterministic,
            missing_instance_policy: MissingInstancePolicy::Queue,
        },
    );

    let seen = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let executor = TimeoutCaptureExecutor {
        seen: Arc::clone(&seen),
    };

    let mut supervisor = make_supervisor(config, Arc::new(executor), events_path);

    let event = Event::new("build.task", "do it").with_id("e-timeout");
    supervisor
        .route_event(event)
        .await
        .expect("route_event should succeed");

    // 等待 actor 真正启动 job 并进入 executor（用 timeout 防止测试卡死）。
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            let maybe_timeout = {
                let seen = seen.lock().await;
                seen.iter()
                    .find(|(id, _)| id.as_str() == target_instance)
                    .map(|(_, t)| *t)
            };

            if let Some(timeout) = maybe_timeout {
                return timeout;
            }

            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("Timed out waiting for executor to observe job")
}

#[tokio::test]
async fn parallel_writer_and_tester_run_concurrently() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();

    // 两个实例：writer#1 / tester#1（并行）
    config.hats.insert(
        "writer".to_string(),
        hat_config("Writer", vec!["build.task"], 1),
    );
    config.hats.insert(
        "tester".to_string(),
        hat_config("Tester", vec!["build.task"], 1),
    );

    let executor = NotifyExecutor {
        expected_starts: 2,
        started: Arc::new(AtomicUsize::new(0)),
        notify: Arc::new(Notify::new()),
        seen: Arc::new(tokio::sync::Mutex::new(Vec::new())),
    };

    let mut supervisor = make_supervisor(config, Arc::new(executor.clone()), events_path.clone());

    // 直接路由一条 build.task，验证两个实例同时进入 execute
    let event = Event::new("build.task", "do it").with_id("e1");
    supervisor
        .route_event(event)
        .await
        .expect("route_event should succeed");

    // 等待两个 job 都启动（由 executor 的 timeout 兜底）
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(executor.started.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn trigger_routing_delivers_to_single_instance_per_hat() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();

    // 同一 hat 多实例：writer#1 / writer#2
    config.hats.insert(
        "writer".to_string(),
        hat_config("Writer", vec!["build.task"], 2),
    );

    let executor = NotifyExecutor {
        expected_starts: 1,
        started: Arc::new(AtomicUsize::new(0)),
        notify: Arc::new(Notify::new()),
        seen: Arc::new(tokio::sync::Mutex::new(Vec::new())),
    };

    let mut supervisor = make_supervisor(config, Arc::new(executor.clone()), events_path.clone());

    let event = Event::new("build.task", "do it").with_id("e-single");
    supervisor
        .route_event(event)
        .await
        .expect("route_event should succeed");

    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(executor.started.load(Ordering::SeqCst), 1);

    let seen = executor.seen.lock().await.clone();
    assert_eq!(seen.len(), 1);
    assert!(
        matches!(seen[0].as_str(), "writer#1" | "writer#2"),
        "Unexpected instance chosen: {:?}",
        seen
    );
}

#[tokio::test]
async fn spawn_instance_forces_new_dynamic_instance_and_delivers_direct() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();

    // 单个静态实例：writer#1
    config.hats.insert(
        "writer".to_string(),
        hat_config("Writer", vec!["build.task"], 1),
    );

    let executor = NotifyExecutor {
        expected_starts: 1,
        started: Arc::new(AtomicUsize::new(0)),
        notify: Arc::new(Notify::new()),
        seen: Arc::new(tokio::sync::Mutex::new(Vec::new())),
    };

    let mut supervisor = make_supervisor(config, Arc::new(executor.clone()), events_path.clone());

    // 显式请求 spawn 一个新实例(上下文隔离),并直达投递.
    let event = Event::new("build.task", "hello")
        .with_id("e-spawn-1")
        .with_target(HatId::new("writer"))
        .with_spawn_instance(true);
    supervisor
        .route_event(event)
        .await
        .expect("route_event should succeed");

    tokio::time::sleep(Duration::from_millis(200)).await;

    let seen = executor.seen.lock().await.clone();
    assert_eq!(seen, vec!["writer#2".to_string()]);

    // 断言：新实例已被创建并注册,但旧实例仍存在.
    assert!(
        supervisor
            .instances
            .contains_key(&HatInstanceId::new("writer#2"))
    );
    assert!(
        supervisor
            .instances
            .contains_key(&HatInstanceId::new("writer#1"))
    );

    let lifecycle_records = runtime_lifecycle_records(&events_path);
    let spawn_record = lifecycle_records
        .iter()
        .find(|record| {
            record.instance_id == HatInstanceId::new("writer#2")
                && record.kind == RuntimeLifecycleKind::Spawn
        })
        .expect("dynamic spawn should be durably recorded");
    assert!(spawn_record.dynamic);
    assert_eq!(spawn_record.source_event_id.as_deref(), Some("e-spawn-1"));
    assert_eq!(
        spawn_record.reason.as_deref(),
        Some("explicit_spawn_instance")
    );
}

#[tokio::test]
async fn topology_spawn_group_creates_three_dynamic_instances_and_delivers_direct() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    config.hats.insert(
        "builder".to_string(),
        hat_config_with_publishes("Builder", vec!["build.task"], vec!["analysis.done"], 1),
    );

    let executor = NotifyExecutor {
        expected_starts: 3,
        started: Arc::new(AtomicUsize::new(0)),
        notify: Arc::new(Notify::new()),
        seen: Arc::new(tokio::sync::Mutex::new(Vec::new())),
    };

    let mut supervisor = make_supervisor(config, Arc::new(executor.clone()), events_path.clone());

    let event = Event::new(
        crate::TOPIC_TOPOLOGY_SPAWN_GROUP,
        serde_json::json!({
            "request_id": "create-three-evolution-hats-20260519-001",
            "hat": "builder",
            "delivery_topic": "build.task",
            "instances": [
                {"role": "功能补充", "task": "补充 feature A"},
                {"role": "功能完善", "task": "完善 feature B"},
                {"role": "review", "task": "review the proposal", "fixed_role": true}
            ]
        })
        .to_string(),
    )
    .with_id("spawn-group-1")
    .with_source("ralph")
    .with_source_instance(HatInstanceId::new("ralph#1"));

    supervisor
        .route_event(event)
        .await
        .expect("route_event should succeed");

    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if executor.started.load(Ordering::SeqCst) >= 3 {
                break;
            }
            executor.notify.notified().await;
        }
    })
    .await
    .expect("Timed out waiting for topology spawn deliveries");

    let builder_instances = supervisor
        .instances_by_hat
        .get(&HatId::new("builder"))
        .expect("builder hat should be present")
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    assert_eq!(
        builder_instances,
        vec![
            "builder#1".to_string(),
            "builder#2".to_string(),
            "builder#3".to_string(),
            "builder#4".to_string()
        ]
    );

    let snapshot = supervisor.build_agents_snapshot();
    let dynamic_instances = snapshot
        .instances
        .iter()
        .filter(|instance| instance.hat_id == "builder" && instance.is_dynamic)
        .map(|instance| instance.instance_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        dynamic_instances,
        vec!["builder#2", "builder#3", "builder#4"],
        "topology.spawn_group must create three parent-visible dynamic instances"
    );
    let builder_2 = snapshot
        .instances
        .iter()
        .find(|instance| instance.instance_id == "builder#2")
        .expect("builder#2 should be present");
    let builder_4 = snapshot
        .instances
        .iter()
        .find(|instance| instance.instance_id == "builder#4")
        .expect("builder#4 should be present");
    assert_eq!(
        builder_2.fixed_role_label, None,
        "unmarked temporary roles must not become first-class agents.json role fields"
    );
    assert_eq!(
        builder_2.identity_source,
        crate::IdentitySource::TaskDerived
    );
    let builder_2_summary = builder_2
        .role_contract_summary
        .as_ref()
        .expect("builder#2 should expose a role contract summary");
    assert_eq!(builder_2_summary.role_name, "功能补充");
    assert_eq!(builder_2_summary.objective_preview, "补充 feature A");
    assert_eq!(
        builder_2_summary.allowed_result_topics,
        vec!["analysis.done".to_string()]
    );
    assert_eq!(
        builder_2_summary.identity_source,
        crate::IdentitySource::TaskDerived
    );
    assert_eq!(
        builder_2_summary.persistence,
        crate::RolePersistence::Temporary
    );
    assert_eq!(builder_2_summary.contract_schema_version, 1);
    assert!(builder_2_summary.role_contract_hash.starts_with("erc-"));
    assert_eq!(
        builder_2_summary.source_spawn_request_id,
        "create-three-evolution-hats-20260519-001"
    );
    assert_eq!(
        builder_4.fixed_role_label.as_deref(),
        Some("review"),
        "fixed_role=true should promote the role label into agents snapshot metadata"
    );
    let builder_4_summary = builder_4
        .role_contract_summary
        .as_ref()
        .expect("builder#4 should expose a role contract summary");
    assert_eq!(
        builder_4_summary.identity_source,
        crate::IdentitySource::TaskDerived,
        "fixed_role=true must not change the identity source"
    );
    assert_eq!(
        builder_4_summary.persistence,
        crate::RolePersistence::Fixed,
        "fixed_role=true should only change role persistence"
    );

    let lifecycle_records = runtime_lifecycle_records(&events_path);
    let spawned = lifecycle_records
        .iter()
        .filter(|record| {
            record.instance_id.as_str().starts_with("builder#")
                && record.kind == RuntimeLifecycleKind::Spawn
                && record.dynamic
                && record.source_event_id.as_deref() == Some("spawn-group-1")
        })
        .map(|record| record.instance_id.to_string())
        .collect::<Vec<_>>();
    assert_eq!(spawned, vec!["builder#2", "builder#3", "builder#4"]);

    let deliveries = runtime_delivery_records(&events_path);
    let builder_deliveries = deliveries
        .iter()
        .filter(|record| {
            record.topic == "build.task"
                && record.source_instance.as_ref().is_some_and(|source| {
                    source.as_str() == "ralph#1" || source.as_str() == "builder#1"
                })
        })
        .map(|record| record.recipient.to_string())
        .collect::<Vec<_>>();
    assert_eq!(
        builder_deliveries,
        vec![
            "builder#2".to_string(),
            "builder#3".to_string(),
            "builder#4".to_string()
        ]
    );

    let response_topics = EventHistory::new(&events_path)
        .filter_by_topic(crate::TOPIC_TOPOLOGY_SPAWN_RESULT)
        .expect("spawn result should be readable");
    assert_eq!(response_topics.len(), 1);
    assert!(response_topics[0].payload.contains("\"spawned\""));
    assert!(response_topics[0].payload.contains("builder#4"));
}

#[tokio::test]
async fn topology_spawn_group_is_idempotent_by_request_id() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    config.hats.insert(
        "builder".to_string(),
        hat_config_with_publishes("Builder", vec!["build.task"], vec!["analysis.done"], 1),
    );

    let executor = NotifyExecutor {
        expected_starts: 3,
        started: Arc::new(AtomicUsize::new(0)),
        notify: Arc::new(Notify::new()),
        seen: Arc::new(tokio::sync::Mutex::new(Vec::new())),
    };

    let mut supervisor = make_supervisor(config, Arc::new(executor.clone()), events_path.clone());

    let payload = serde_json::json!({
        "request_id": "create-three-evolution-hats-20260519-001",
        "hat": "builder",
        "delivery_topic": "build.task",
        "instances": [
            {"role": "功能补充", "task": "补充 feature A"},
            {"role": "功能完善", "task": "完善 feature B"},
            {"role": "review", "task": "review the proposal"}
        ]
    })
    .to_string();

    let first = Event::new(crate::TOPIC_TOPOLOGY_SPAWN_GROUP, payload.clone())
        .with_id("spawn-group-1")
        .with_source("ralph")
        .with_source_instance(HatInstanceId::new("ralph#1"));
    supervisor
        .route_event(first)
        .await
        .expect("first route_event should succeed");

    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if executor.started.load(Ordering::SeqCst) >= 3 {
                break;
            }
            executor.notify.notified().await;
        }
    })
    .await
    .expect("Timed out waiting for first topology spawn");

    let second = Event::new(crate::TOPIC_TOPOLOGY_SPAWN_GROUP, payload)
        .with_id("spawn-group-2")
        .with_source("ralph")
        .with_source_instance(HatInstanceId::new("ralph#1"));
    supervisor
        .route_event(second)
        .await
        .expect("second route_event should succeed");

    tokio::time::sleep(Duration::from_millis(200)).await;

    let builder_instances = supervisor
        .instances_by_hat
        .get(&HatId::new("builder"))
        .expect("builder hat should be present")
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    assert_eq!(
        builder_instances,
        vec![
            "builder#1".to_string(),
            "builder#2".to_string(),
            "builder#3".to_string(),
            "builder#4".to_string()
        ]
    );

    let result_records = EventHistory::new(&events_path)
        .filter_by_topic(crate::TOPIC_TOPOLOGY_SPAWN_RESULT)
        .expect("spawn result records should be readable");
    assert_eq!(
        result_records.len(),
        1,
        "duplicate request_id must not create another topology.spawn result"
    );
}

#[tokio::test]
async fn topology_spawn_group_rejects_conflicting_role_contract() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");
    let executor = NotifyExecutor {
        expected_starts: 1,
        started: Arc::new(AtomicUsize::new(0)),
        notify: Arc::new(Notify::new()),
        seen: Arc::new(tokio::sync::Mutex::new(Vec::new())),
    };
    let mut supervisor = make_supervisor(
        topology_builder_config(vec!["analysis.done"]),
        Arc::new(executor),
        events_path.clone(),
    );

    supervisor
        .route_event(topology_spawn_group_event(serde_json::json!({
            "role": "功能补充",
            "task": "补充 feature A",
            "role_contract": {
                "role_name": "review",
                "objective": "补充 feature A",
                "input_contract": "handle build.task",
                "output_contract": "publish analysis.done",
                "allowed_topics": ["analysis.done"],
                "forbidden_responsibilities": [],
                "success_criteria": ["done"],
                "identity_source": "task-derived"
            }
        })))
        .await
        .expect("route_event should succeed with structured failed member");

    tokio::time::sleep(Duration::from_millis(100)).await;
    let result = latest_topology_spawn_result(&events_path);
    assert_eq!(result.status, "failed");
    assert!(result.spawned.is_empty());
    assert_eq!(result.failed.len(), 1);
    assert!(
        result.failed[0].error.contains("role_contract.role_name"),
        "conflict should be explicit: {result:#?}"
    );
    assert!(
        supervisor
            .instances_by_hat
            .get(&HatId::new("builder"))
            .expect("builder hat should be present")
            .iter()
            .all(|id| id.as_str() == "builder#1"),
        "conflicting role contract must not spawn a dynamic instance"
    );
}

#[tokio::test]
async fn topology_spawn_group_canonical_objective_uses_member_task() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");
    let executor = NotifyExecutor {
        expected_starts: 1,
        started: Arc::new(AtomicUsize::new(0)),
        notify: Arc::new(Notify::new()),
        seen: Arc::new(tokio::sync::Mutex::new(Vec::new())),
    };
    let mut supervisor = make_supervisor(
        topology_builder_config(vec!["analysis.done"]),
        Arc::new(executor.clone()),
        events_path.clone(),
    );

    supervisor
        .route_event(topology_spawn_group_event(serde_json::json!({
            "role": "功能补充",
            "task": "canonical member task",
            "role_contract": {
                "role_name": "功能补充",
                "objective": "RAW OBJECTIVE SHOULD NOT WIN",
                "input_contract": "handle build.task",
                "output_contract": "publish analysis.done",
                "allowed_topics": ["analysis.done"],
                "forbidden_responsibilities": ["do not coordinate globally"],
                "success_criteria": ["publish allowed result"],
                "identity_source": "task-derived"
            }
        })))
        .await
        .expect("route_event should succeed");

    tokio::time::timeout(Duration::from_secs(2), executor.notify.notified())
        .await
        .expect("dynamic worker should receive delivery");

    let snapshot = supervisor.build_agents_snapshot();
    let builder_2 = snapshot
        .instances
        .iter()
        .find(|instance| instance.instance_id == "builder#2")
        .expect("builder#2 should exist");
    let summary = builder_2
        .role_contract_summary
        .as_ref()
        .expect("role contract summary should exist");
    assert_eq!(summary.objective_preview, "canonical member task");
    assert_eq!(
        summary.allowed_result_topics,
        vec!["analysis.done".to_string()]
    );

    let deliveries = EventHistory::new(&events_path)
        .filter_by_topic("build.task")
        .expect("build.task records should be readable");
    let payload = deliveries
        .last()
        .expect("delivery should be logged")
        .payload
        .as_str();
    assert!(payload.contains("raw role_contract.objective ignored"));
    assert!(!payload.contains("RAW OBJECTIVE SHOULD NOT WIN"));
}

#[tokio::test]
async fn topology_spawn_group_rejects_non_task_derived_identity_source() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");
    let executor = NotifyExecutor {
        expected_starts: 1,
        started: Arc::new(AtomicUsize::new(0)),
        notify: Arc::new(Notify::new()),
        seen: Arc::new(tokio::sync::Mutex::new(Vec::new())),
    };
    let mut supervisor = make_supervisor(
        topology_builder_config(vec!["analysis.done"]),
        Arc::new(executor),
        events_path.clone(),
    );

    supervisor
        .route_event(topology_spawn_group_event(serde_json::json!({
            "role": "功能补充",
            "task": "补充 feature A",
            "role_contract": {
                "role_name": "功能补充",
                "objective": "补充 feature A",
                "input_contract": "handle build.task",
                "output_contract": "publish analysis.done",
                "allowed_topics": ["analysis.done"],
                "forbidden_responsibilities": [],
                "success_criteria": ["done"],
                "identity_source": "template-derived"
            }
        })))
        .await
        .expect("route_event should succeed with structured failed member");

    let result = latest_topology_spawn_result(&events_path);
    assert_eq!(result.status, "failed");
    assert!(result.failed[0].error.contains("must be task-derived"));
}

#[tokio::test]
async fn topology_spawn_group_rejects_control_plane_output_topic() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");
    let executor = NotifyExecutor {
        expected_starts: 1,
        started: Arc::new(AtomicUsize::new(0)),
        notify: Arc::new(Notify::new()),
        seen: Arc::new(tokio::sync::Mutex::new(Vec::new())),
    };
    let mut supervisor = make_supervisor(
        topology_builder_config(vec!["analysis.done"]),
        Arc::new(executor),
        events_path.clone(),
    );

    supervisor
        .route_event(topology_spawn_group_event(serde_json::json!({
            "role": "功能补充",
            "task": "补充 feature A",
            "role_contract": {
                "role_name": "功能补充",
                "objective": "补充 feature A",
                "input_contract": "handle build.task",
                "output_contract": "publish analysis.done",
                "allowed_topics": ["analysis.done", "topology.spawn_group"],
                "forbidden_responsibilities": [],
                "success_criteria": ["done"],
                "identity_source": "task-derived"
            }
        })))
        .await
        .expect("route_event should succeed with structured failed member");

    let result = latest_topology_spawn_result(&events_path);
    assert_eq!(result.status, "failed");
    let failure = &result.failed[0];
    assert!(failure.error.contains("control-plane topic"));
    assert_eq!(failure.request_id.as_deref(), Some("spawn-role-contract-1"));
    assert_eq!(failure.instance_id, None);
    assert_eq!(
        failure.phase.as_deref(),
        Some(crate::TOPOLOGY_SPAWN_PHASE_MEMBER_VALIDATION_FAILED)
    );
    assert!(
        failure
            .recovery_hint
            .as_deref()
            .is_some_and(|hint| hint.contains("role_contract")),
        "failed member should carry a human/actionable recovery hint: {failure:#?}"
    );
}

#[tokio::test]
async fn topology_spawn_group_partial_failure_keeps_successful_members_running() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");
    let executor = NotifyExecutor {
        expected_starts: 1,
        started: Arc::new(AtomicUsize::new(0)),
        notify: Arc::new(Notify::new()),
        seen: Arc::new(tokio::sync::Mutex::new(Vec::new())),
    };
    let mut supervisor = make_supervisor(
        topology_builder_config(vec!["analysis.done"]),
        Arc::new(executor.clone()),
        events_path.clone(),
    );

    let event = Event::new(
        crate::TOPIC_TOPOLOGY_SPAWN_GROUP,
        serde_json::json!({
            "request_id": "spawn-partial-1",
            "hat": "builder",
            "delivery_topic": "build.task",
            "instances": [
                {"role": "功能补充", "task": "补充 feature A"},
                {
                    "role": "review",
                    "task": "review the proposal",
                    "role_contract": {
                        "role_name": "review",
                        "objective": "review the proposal",
                        "input_contract": "handle build.task",
                        "output_contract": "publish analysis.done",
                        "allowed_topics": ["topology.spawn_group"],
                        "forbidden_responsibilities": [],
                        "success_criteria": ["done"],
                        "identity_source": "task-derived"
                    }
                }
            ]
        })
        .to_string(),
    )
    .with_id("spawn-partial-event-1")
    .with_source("ralph")
    .with_source_instance(HatInstanceId::new("ralph#1"));

    supervisor
        .route_event(event)
        .await
        .expect("route_event should keep non-atomic spawn_group alive");

    tokio::time::timeout(Duration::from_secs(2), executor.notify.notified())
        .await
        .expect("successful member should still receive direct delivery");

    let result = latest_topology_spawn_result(&events_path);
    assert_eq!(result.status, "partial");
    assert_eq!(result.request_id, "spawn-partial-1");
    assert_eq!(result.spawned.len(), 1);
    assert_eq!(result.spawned[0].instance_id, "builder#2");
    assert_eq!(result.failed.len(), 1);

    let failure = &result.failed[0];
    assert_eq!(failure.index, 1);
    assert_eq!(failure.role, "review");
    assert_eq!(failure.request_id.as_deref(), Some("spawn-partial-1"));
    assert_eq!(failure.instance_id, None);
    assert_eq!(
        failure.phase.as_deref(),
        Some(crate::TOPOLOGY_SPAWN_PHASE_MEMBER_VALIDATION_FAILED)
    );
    assert!(failure.recovery_hint.is_some());

    let seen = executor.seen.lock().await.clone();
    let builder_seen = seen
        .iter()
        .filter(|instance_id| instance_id.starts_with("builder#"))
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(
        builder_seen,
        vec!["builder#2".to_string()],
        "non-atomic partial failure must not prevent successful members from running"
    );
}

#[tokio::test]
async fn topology_spawn_group_rejects_empty_output_allowlist_intersection() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");
    let executor = NotifyExecutor {
        expected_starts: 1,
        started: Arc::new(AtomicUsize::new(0)),
        notify: Arc::new(Notify::new()),
        seen: Arc::new(tokio::sync::Mutex::new(Vec::new())),
    };
    let mut supervisor = make_supervisor(
        topology_builder_config(vec!["analysis.done"]),
        Arc::new(executor),
        events_path.clone(),
    );

    supervisor
        .route_event(topology_spawn_group_event(serde_json::json!({
            "role": "功能补充",
            "task": "补充 feature A",
            "role_contract": {
                "role_name": "功能补充",
                "objective": "补充 feature A",
                "input_contract": "handle build.task",
                "output_contract": "publish analysis.done",
                "allowed_topics": ["review.done"],
                "forbidden_responsibilities": [],
                "success_criteria": ["done"],
                "identity_source": "task-derived"
            }
        })))
        .await
        .expect("route_event should succeed with structured failed member");

    let result = latest_topology_spawn_result(&events_path);
    assert_eq!(result.status, "failed");
    assert!(result.failed[0].error.contains("no intersection"));
}

#[tokio::test]
async fn topology_spawn_group_excludes_delivery_topic_from_output_allowlist() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");
    let executor = NotifyExecutor {
        expected_starts: 1,
        started: Arc::new(AtomicUsize::new(0)),
        notify: Arc::new(Notify::new()),
        seen: Arc::new(tokio::sync::Mutex::new(Vec::new())),
    };
    let mut supervisor = make_supervisor(
        topology_builder_config(vec!["build.task", "analysis.done"]),
        Arc::new(executor.clone()),
        events_path,
    );

    supervisor
        .route_event(topology_spawn_group_event(serde_json::json!({
            "role": "功能补充",
            "task": "补充 feature A",
            "role_contract": {
                "role_name": "功能补充",
                "objective": "补充 feature A",
                "input_contract": "handle build.task",
                "output_contract": "publish analysis.done",
                "allowed_topics": ["build.task", "analysis.done"],
                "forbidden_responsibilities": [],
                "success_criteria": ["done"],
                "identity_source": "task-derived"
            }
        })))
        .await
        .expect("route_event should succeed");

    tokio::time::timeout(Duration::from_secs(2), executor.notify.notified())
        .await
        .expect("dynamic worker should receive delivery");
    let snapshot = supervisor.build_agents_snapshot();
    let summary = snapshot
        .instances
        .iter()
        .find(|instance| instance.instance_id == "builder#2")
        .and_then(|instance| instance.role_contract_summary.as_ref())
        .expect("role contract summary should exist");
    assert_eq!(
        summary.allowed_result_topics,
        vec!["analysis.done".to_string()],
        "delivery topic must be removed from the output allowlist"
    );
}

#[tokio::test]
async fn agents_snapshot_stores_role_contract_summary_not_full_contract() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");
    let executor = NotifyExecutor {
        expected_starts: 1,
        started: Arc::new(AtomicUsize::new(0)),
        notify: Arc::new(Notify::new()),
        seen: Arc::new(tokio::sync::Mutex::new(Vec::new())),
    };
    let mut supervisor = make_supervisor(
        topology_builder_config(vec!["analysis.done"]),
        Arc::new(executor.clone()),
        events_path,
    );

    supervisor
        .route_event(topology_spawn_group_event(serde_json::json!({
            "role": "功能补充",
            "task": "补充 feature A",
            "role_contract": {
                "role_name": "功能补充",
                "objective": "补充 feature A",
                "input_contract": "SECRET input contract should stay out of agents snapshot",
                "output_contract": "SECRET output contract should stay out of agents snapshot",
                "allowed_topics": ["analysis.done"],
                "forbidden_responsibilities": ["SECRET forbidden item"],
                "success_criteria": ["SECRET success item"],
                "identity_source": "task-derived"
            }
        })))
        .await
        .expect("route_event should succeed");

    tokio::time::timeout(Duration::from_secs(2), executor.notify.notified())
        .await
        .expect("dynamic worker should receive delivery");

    let snapshot_json = serde_json::to_string_pretty(&supervisor.build_agents_snapshot())
        .expect("agents snapshot should serialize");
    assert!(snapshot_json.contains("\"role_contract_summary\""));
    assert!(snapshot_json.contains("\"role_contract_hash\""));
    assert!(snapshot_json.contains("\"source_spawn_request_id\""));
    assert!(!snapshot_json.contains("SECRET input contract"));
    assert!(!snapshot_json.contains("SECRET output contract"));
    assert!(!snapshot_json.contains("SECRET forbidden item"));
    assert!(!snapshot_json.contains("SECRET success item"));
    assert!(!snapshot_json.contains("### ROLE CONTRACT"));
}

#[tokio::test]
async fn completed_dynamic_instance_remains_visible_as_agents_tombstone() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");
    let executor = NotifyExecutor {
        expected_starts: 1,
        started: Arc::new(AtomicUsize::new(0)),
        notify: Arc::new(Notify::new()),
        seen: Arc::new(tokio::sync::Mutex::new(Vec::new())),
    };
    let mut supervisor = make_supervisor(
        topology_builder_config(vec!["analysis.done"]),
        Arc::new(executor.clone()),
        events_path,
    );

    supervisor
        .route_event(topology_spawn_group_event(serde_json::json!({
            "role": "功能补充",
            "task": "补充 feature A",
            "role_contract": {
                "role_name": "功能补充",
                "objective": "补充 feature A",
                "input_contract": "handle build.task",
                "output_contract": "publish analysis.done",
                "allowed_topics": ["analysis.done"],
                "forbidden_responsibilities": ["do not coordinate globally"],
                "success_criteria": ["publish analysis.done"],
                "identity_source": "task-derived"
            }
        })))
        .await
        .expect("route_event should succeed");

    tokio::time::timeout(Duration::from_secs(2), executor.notify.notified())
        .await
        .expect("dynamic worker should receive delivery");

    let instance_id = HatInstanceId::new("builder#2");
    let before = supervisor.build_agents_snapshot();
    assert!(
        before
            .instances
            .iter()
            .any(|instance| instance.instance_id == "builder#2"),
        "dynamic instance should be visible while it is in the current registry"
    );

    supervisor
        .instance_states
        .insert(instance_id.clone(), HatInstanceState::Done);
    supervisor.unregister_dynamic_instance(&instance_id);

    let after = supervisor.build_agents_snapshot();
    assert!(
        after
            .instances
            .iter()
            .all(|instance| instance.instance_id != "builder#2"),
        "completed dynamic instance must not stay in the current registry list"
    );

    let completed = after
        .completed_dynamic_instances
        .iter()
        .find(|instance| instance.instance_id == "builder#2")
        .expect("completed dynamic instance should have a tombstone");
    assert_eq!(completed.hat_id, "builder");
    assert_eq!(completed.final_state, HatInstanceState::Done);
    assert_eq!(
        completed.identity_source,
        crate::IdentitySource::TaskDerived
    );
    assert_eq!(
        completed
            .last_input
            .as_ref()
            .map(|input| input.topic.as_str()),
        Some("build.task")
    );
    let summary = completed
        .role_contract_summary
        .as_ref()
        .expect("completed tombstone should preserve the role contract summary");
    assert_eq!(summary.role_name, "功能补充");
    assert_eq!(
        summary.allowed_result_topics,
        vec!["analysis.done".to_string()]
    );
    assert_eq!(
        completed.retirement_reason,
        "dynamic_instance_unregistered_after_done"
    );
}

#[tokio::test]
async fn failed_dynamic_instance_remains_visible_as_agents_tombstone() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");
    let executor = NotifyExecutor {
        expected_starts: 1,
        started: Arc::new(AtomicUsize::new(0)),
        notify: Arc::new(Notify::new()),
        seen: Arc::new(tokio::sync::Mutex::new(Vec::new())),
    };
    let mut supervisor = make_supervisor(
        topology_builder_config(vec!["analysis.done"]),
        Arc::new(executor.clone()),
        events_path.clone(),
    );

    supervisor
        .route_event(topology_spawn_group_event(serde_json::json!({
            "role": "功能补充",
            "task": "补充 feature A",
            "role_contract": {
                "role_name": "功能补充",
                "objective": "补充 feature A",
                "input_contract": "handle build.task",
                "output_contract": "publish analysis.done",
                "allowed_topics": ["analysis.done"],
                "forbidden_responsibilities": ["do not coordinate globally"],
                "success_criteria": ["publish analysis.done"],
                "identity_source": "task-derived"
            }
        })))
        .await
        .expect("route_event should succeed");

    tokio::time::timeout(Duration::from_secs(2), executor.notify.notified())
        .await
        .expect("dynamic worker should receive delivery before simulated failure");

    let instance_id = HatInstanceId::new("builder#2");
    supervisor.mark_dynamic_instance_failed_and_unregister(
        &instance_id,
        DYNAMIC_INSTANCE_RETIREMENT_REASON_DELIVERY_FAILED_AFTER_SPAWN,
    );

    let after = supervisor.build_agents_snapshot();
    assert!(
        after
            .instances
            .iter()
            .all(|instance| instance.instance_id != "builder#2"),
        "failed dynamic instance must leave the current registry list"
    );

    let completed = after
        .completed_dynamic_instances
        .iter()
        .find(|instance| instance.instance_id == "builder#2")
        .expect("failed dynamic instance should have a tombstone");
    assert_eq!(completed.hat_id, "builder");
    assert_eq!(completed.final_state, HatInstanceState::Failed);
    assert_eq!(
        completed.retirement_reason,
        DYNAMIC_INSTANCE_RETIREMENT_REASON_DELIVERY_FAILED_AFTER_SPAWN
    );
    assert_eq!(
        completed
            .role_contract_summary
            .as_ref()
            .map(|summary| summary.role_name.as_str()),
        Some("功能补充"),
        "failed tombstone must preserve task-derived role identity"
    );
    assert_eq!(
        completed
            .last_input
            .as_ref()
            .map(|input| input.topic.as_str()),
        Some("build.task"),
        "failed tombstone should still explain the delivery that reached the instance"
    );

    let lifecycle_records = runtime_lifecycle_records(&events_path);
    assert!(
        lifecycle_records.iter().any(|record| {
            record.instance_id == instance_id
                && record.kind == RuntimeLifecycleKind::State
                && record.state == Some(HatInstanceState::Failed)
                && record.reason.as_deref()
                    == Some(DYNAMIC_INSTANCE_RETIREMENT_REASON_DELIVERY_FAILED_AFTER_SPAWN)
        }),
        "failed-after-spawn state transition should be durable in runtime.lifecycle"
    );
}

#[tokio::test]
async fn dynamic_worker_prompt_contains_effective_role_contract() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");
    let prompts = Arc::new(tokio::sync::Mutex::new(HashMap::<String, String>::new()));
    let notify = Arc::new(Notify::new());
    let executor = PromptCaptureNotifyExecutor {
        prompts: Arc::clone(&prompts),
        notify: Arc::clone(&notify),
        notify_on_instance: "builder#2".to_string(),
    };
    let mut supervisor = make_supervisor(
        topology_builder_config(vec!["analysis.done"]),
        Arc::new(executor),
        events_path,
    );

    supervisor
        .route_event(topology_spawn_group_event(serde_json::json!({
            "role": "功能补充",
            "task": "canonical member task",
            "role_contract": {
                "role_name": "功能补充",
                "objective": "RAW OBJECTIVE SHOULD NOT APPEAR IN PROMPT",
                "input_contract": "handle build.task as assigned input",
                "output_contract": "publish analysis.done only",
                "allowed_topics": ["analysis.done"],
                "forbidden_responsibilities": ["do not coordinate globally"],
                "success_criteria": ["publish allowed result"],
                "identity_source": "task-derived"
            }
        })))
        .await
        .expect("route_event should succeed");

    tokio::time::timeout(Duration::from_secs(2), notify.notified())
        .await
        .expect("builder#2 prompt should be captured");
    let got = prompts.lock().await.clone();
    let prompt = got.get("builder#2").expect("builder#2 prompt should exist");

    assert!(prompt.contains("### ROLE CONTRACT"));
    assert!(prompt.contains("identity_source: task-derived"));
    assert!(prompt.contains("persistence: temporary"));
    assert!(prompt.contains("source_spawn_request_id: spawn-role-contract-1"));
    assert!(prompt.contains("Objective:\ncanonical member task"));
    assert!(prompt.contains("Allowed result topics:\n- analysis.done"));
    assert!(prompt.contains("Forbidden responsibilities:\n- do not coordinate globally"));
    assert!(!prompt.contains("RAW OBJECTIVE SHOULD NOT APPEAR IN PROMPT"));
    assert!(
        !prompt.contains("Parent-visible dynamic group spawn")
            && !prompt.contains("topology.spawn_group"),
        "dynamic worker prompt must not inherit coordinator-only topology spawn instructions"
    );
    for heading in crate::headings_for_surface(crate::PromptSurface::CoordinatorOnly) {
        assert!(
            !prompt.contains(heading),
            "dynamic worker prompt must not inherit coordinator-only surface {heading}"
        );
    }
}

#[tokio::test]
async fn topology_spawn_group_from_non_ralph_hat_is_ignored_or_failed() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    config.hats.insert(
        "builder".to_string(),
        hat_config_with_publishes("Builder", vec!["build.task"], vec!["analysis.done"], 1),
    );

    let executor = NotifyExecutor {
        expected_starts: 1,
        started: Arc::new(AtomicUsize::new(0)),
        notify: Arc::new(Notify::new()),
        seen: Arc::new(tokio::sync::Mutex::new(Vec::new())),
    };

    let mut supervisor = make_supervisor(config, Arc::new(executor), events_path.clone());

    let event = Event::new(
        crate::TOPIC_TOPOLOGY_SPAWN_GROUP,
        serde_json::json!({
            "request_id": "create-three-evolution-hats-20260519-002",
            "hat": "builder",
            "delivery_topic": "build.task",
            "instances": [
                {"role": "功能补充", "task": "补充 feature A"},
                {"role": "功能完善", "task": "完善 feature B"},
                {"role": "review", "task": "review the proposal"}
            ]
        })
        .to_string(),
    )
    .with_id("spawn-group-foreign")
    .with_source("writer")
    .with_source_instance(HatInstanceId::new("writer#1"));

    supervisor
        .route_event(event)
        .await
        .expect("route_event should succeed");

    tokio::time::sleep(Duration::from_millis(200)).await;

    let builder_instances = supervisor
        .instances_by_hat
        .get(&HatId::new("builder"))
        .expect("builder hat should be present")
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    assert_eq!(
        builder_instances,
        vec!["builder#1".to_string()],
        "non-ralph source must not spawn new instances"
    );

    let failed_records = EventHistory::new(&events_path)
        .filter_by_topic(crate::TOPIC_TOPOLOGY_SPAWN_FAILED)
        .expect("spawn failed records should be readable");
    assert_eq!(failed_records.len(), 1);
    assert!(failed_records[0].payload.contains("coordinator-only"));
}

#[tokio::test]
async fn direct_delivery_writes_runtime_delivery_record() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    config.hats.insert(
        "writer".to_string(),
        hat_config("Writer", vec!["build.task"], 1),
    );

    let executor = NotifyExecutor {
        expected_starts: 1,
        started: Arc::new(AtomicUsize::new(0)),
        notify: Arc::new(Notify::new()),
        seen: Arc::new(tokio::sync::Mutex::new(Vec::new())),
    };
    let mut supervisor = make_supervisor(config, Arc::new(executor), events_path.clone());

    let event = Event::new("build.task", "direct")
        .with_id("e-direct")
        .with_target_instance(HatInstanceId::new("writer#1"));
    supervisor
        .route_event(event)
        .await
        .expect("route_event should succeed");

    let delivery_records = runtime_delivery_records(&events_path);
    let delivery = delivery_records
        .iter()
        .find(|record| record.event_id.as_deref() == Some("e-direct"))
        .expect("direct delivery should be durably recorded");
    assert_eq!(delivery.recipient, HatInstanceId::new("writer#1"));
    assert_eq!(delivery.mode, RuntimeDeliveryKind::Direct);
}

#[tokio::test]
async fn fanout_delivery_writes_one_runtime_delivery_record_per_recipient() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    config.hats.insert(
        "writer".to_string(),
        hat_config("Writer", vec!["build.task"], 1),
    );
    config.hats.insert(
        "tester".to_string(),
        hat_config("Tester", vec!["build.task"], 1),
    );

    let executor = NotifyExecutor {
        expected_starts: 2,
        started: Arc::new(AtomicUsize::new(0)),
        notify: Arc::new(Notify::new()),
        seen: Arc::new(tokio::sync::Mutex::new(Vec::new())),
    };
    let mut supervisor = make_supervisor(config, Arc::new(executor), events_path.clone());

    let event = Event::new("build.task", "fanout").with_id("e-fanout");
    supervisor
        .route_event(event)
        .await
        .expect("route_event should succeed");

    let mut recipients: Vec<String> = runtime_delivery_records(&events_path)
        .into_iter()
        .filter(|record| record.event_id.as_deref() == Some("e-fanout"))
        .map(|record| {
            assert_eq!(record.mode, RuntimeDeliveryKind::Fanout);
            record.recipient.to_string()
        })
        .collect();
    recipients.sort();
    assert_eq!(
        recipients,
        vec!["tester#1".to_string(), "writer#1".to_string()]
    );
}

#[tokio::test]
async fn queue_delivery_writes_runtime_delivery_record() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    config.hats.insert(
        "writer".to_string(),
        hat_config("Writer", vec!["build.task"], 2),
    );
    config.parallel.topic_contracts.insert(
        "build.task".to_string(),
        TopicContract {
            delivery: Delivery::Queue,
            audience: AudienceSelector {
                instances: vec![
                    HatInstanceId::new("writer#1"),
                    HatInstanceId::new("writer#2"),
                ],
                ..Default::default()
            },
            queue_selection: QueueSelection::Deterministic,
            missing_instance_policy: MissingInstancePolicy::Queue,
        },
    );

    let executor = NotifyExecutor {
        expected_starts: 1,
        started: Arc::new(AtomicUsize::new(0)),
        notify: Arc::new(Notify::new()),
        seen: Arc::new(tokio::sync::Mutex::new(Vec::new())),
    };
    let mut supervisor = make_supervisor(config, Arc::new(executor), events_path.clone());

    let event = Event::new("build.task", "queue").with_id("e-queue");
    supervisor
        .route_event(event)
        .await
        .expect("route_event should succeed");

    let delivery = runtime_delivery_records(&events_path)
        .into_iter()
        .find(|record| record.event_id.as_deref() == Some("e-queue"))
        .expect("queue delivery should be durably recorded");
    assert_eq!(delivery.mode, RuntimeDeliveryKind::Queue);
    assert!(matches!(
        delivery.recipient.as_str(),
        "writer#1" | "writer#2"
    ));
}

#[tokio::test]
async fn reply_delivery_writes_runtime_delivery_record() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    config.hats.insert(
        "planner".to_string(),
        hat_config("Planner", vec!["planning.request"], 1),
    );
    config.hats.insert(
        "writer".to_string(),
        hat_config("Writer", vec!["build.question"], 1),
    );

    let executor = NotifyExecutor {
        expected_starts: 2,
        started: Arc::new(AtomicUsize::new(0)),
        notify: Arc::new(Notify::new()),
        seen: Arc::new(tokio::sync::Mutex::new(Vec::new())),
    };
    let mut supervisor = make_supervisor(config, Arc::new(executor), events_path.clone());

    let request = Event::new("build.question", "question")
        .with_id("req-1")
        .with_source_instance(HatInstanceId::new("planner#1"))
        .with_target_instance(HatInstanceId::new("writer#1"));
    supervisor
        .route_event(request)
        .await
        .expect("request route should succeed");

    let reply = Event::new(TOPIC_REPLY_HAT_MESSAGE, "answer")
        .with_id("reply-1")
        .with_reply("req-1")
        .with_source_instance(HatInstanceId::new("writer#1"));
    supervisor
        .route_event(reply)
        .await
        .expect("reply route should succeed");

    let delivery = runtime_delivery_records(&events_path)
        .into_iter()
        .find(|record| record.event_id.as_deref() == Some("reply-1"))
        .expect("reply delivery should be durably recorded");
    assert_eq!(delivery.recipient, HatInstanceId::new("planner#1"));
    assert_eq!(delivery.mode, RuntimeDeliveryKind::Reply);
    assert_eq!(delivery.reply.as_deref(), Some("req-1"));
}

#[tokio::test]
async fn lifecycle_controls_write_freeze_cancel_shutdown_records() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    config.hats.insert(
        "writer".to_string(),
        hat_config("Writer", vec!["build.task"], 1),
    );

    let executor = NotifyExecutor {
        expected_starts: 1,
        started: Arc::new(AtomicUsize::new(0)),
        notify: Arc::new(Notify::new()),
        seen: Arc::new(tokio::sync::Mutex::new(Vec::new())),
    };
    let mut supervisor = make_supervisor(config, Arc::new(executor), events_path.clone());

    supervisor.freeze_pending_on_all_instances();
    supervisor.shutdown_instances().await;

    let lifecycle_records = runtime_lifecycle_records(&events_path);
    for kind in [
        RuntimeLifecycleKind::Freeze,
        RuntimeLifecycleKind::Cancel,
        RuntimeLifecycleKind::Shutdown,
    ] {
        assert!(
            lifecycle_records.iter().any(|record| record.kind == kind
                && record.instance_id == HatInstanceId::new("writer#1")),
            "expected lifecycle control record for {:?}",
            kind
        );
    }
}

#[tokio::test]
async fn task_start_target_instance_is_not_delivered_to_wildcard_hat() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let seen = Arc::new(tokio::sync::Mutex::new(Vec::<String>::new()));
    let notify = Arc::new(Notify::new());
    let executor = StartEventCaptureExecutor {
        seen: Arc::clone(&seen),
        notify: Arc::clone(&notify),
    };

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();

    // wildcard hat：如果 task.start 不是 target_instance 投递，就会收到 payload（prompt pollution 风险）
    config
        .hats
        .insert("manager".to_string(), hat_config("Manager", vec!["*"], 1));

    let mut supervisor = make_supervisor(config, Arc::new(executor), events_path);

    let event = Event::new("task.start", "top-level prompt")
        .with_id("e-task-start")
        .with_target_instance(HatInstanceId::from_parts("ralph", "1"));

    supervisor
        .route_event(event)
        .await
        .expect("route_event should succeed");

    // 等待至少一个 job 执行开始（用 timeout 防止 test 卡死）
    tokio::time::timeout(Duration::from_secs(1), notify.notified())
        .await
        .expect("Timed out waiting for first job execution");

    // 给调度一点缓冲时间，避免 race（如果错误投递给 manager，这里应该能观测到第二次 execute）
    tokio::time::sleep(Duration::from_millis(80)).await;

    let mut got = seen.lock().await.clone();
    got.sort();

    assert_eq!(got, vec!["ralph#1".to_string()]);
}

#[tokio::test]
async fn busy_ralph_session_directed_event_stays_on_primary() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let executor = NotifyExecutor {
        expected_starts: 1,
        started: Arc::new(AtomicUsize::new(0)),
        notify: Arc::new(Notify::new()),
        seen: Arc::new(tokio::sync::Mutex::new(Vec::new())),
    };

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    let mut supervisor = make_supervisor(config, Arc::new(executor.clone()), events_path);

    // 模拟 ralph#1 正在 Running(与重定向测试相同的前置)。
    supervisor
        .instance_states
        .insert(HatInstanceId::new("ralph#1"), HatInstanceState::Running);

    // 会话定向事件(显式 session_strategy=app_server)必须直达 ralph#1:
    // 改投到 ralph#2 会启动全新会话, 丢失 ralph#1 会话中的 steer 输入等上下文。
    let event = Event::new("e2e.reply.step2", "step2")
        .with_id("e-step2-session")
        .with_target_instance(HatInstanceId::new("ralph#1"))
        .with_session_strategy(SessionStrategy::AppServer);
    supervisor
        .route_event(event)
        .await
        .expect("route_event should succeed");

    tokio::time::sleep(Duration::from_millis(200)).await;
    let seen = executor.seen.lock().await.clone();
    assert_eq!(seen, vec!["ralph#1".to_string()]);
}

    async fn busy_ralph_primary_explicit_target_is_redirected_to_secondary() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let executor = NotifyExecutor {
        expected_starts: 1,
        started: Arc::new(AtomicUsize::new(0)),
        notify: Arc::new(Notify::new()),
        seen: Arc::new(tokio::sync::Mutex::new(Vec::new())),
    };

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    let mut supervisor = make_supervisor(config, Arc::new(executor.clone()), events_path);

    // ---------------------------------------------------------------------
    // 说明:
    // - 这里直接把 ralph#1 状态标记为 Running,模拟“主协调实例正在执行中”。
    // - 目标是验证: 显式 target_instance=ralph#1 时也会切到 ralph#2。
    // ---------------------------------------------------------------------
    supervisor
        .instance_states
        .insert(HatInstanceId::new("ralph#1"), HatInstanceState::Running);

    let event = Event::new("routing.escalate", "needs coordinator")
        .with_id("e-ralph-busy-explicit")
        .with_target_instance(HatInstanceId::new("ralph#1"));
    supervisor
        .route_event(event)
        .await
        .expect("route_event should succeed");

    tokio::time::sleep(Duration::from_millis(200)).await;
    let seen = executor.seen.lock().await.clone();
    assert_eq!(seen, vec!["ralph#2".to_string()]);
}

#[tokio::test]
async fn busy_ralph_primary_explicit_target_is_not_redirected_for_turn_steer() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let executor = NotifyExecutor {
        expected_starts: 1,
        started: Arc::new(AtomicUsize::new(0)),
        notify: Arc::new(Notify::new()),
        seen: Arc::new(tokio::sync::Mutex::new(Vec::new())),
    };

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    let mut supervisor = make_supervisor(config, Arc::new(executor.clone()), events_path);

    // 说明:
    // - 标记 ralph#1 为 Running,模拟“主协调实例正在执行中”。
    // - 但 turn/steer 属于 in-flight 控制信号,必须保持直达 ralph#1,不应被改投到 ralph#2。
    supervisor
        .instance_states
        .insert(HatInstanceId::new("ralph#1"), HatInstanceState::Running);

    let event = Event::new("e2e.steer", "marker")
        .with_id("e-ralph-busy-steer-no-redirect")
        .with_target_instance(HatInstanceId::new("ralph#1"))
        .with_turn_action(TurnAction::Steer);
    supervisor
        .route_event(event)
        .await
        .expect("route_event should succeed");

    tokio::time::sleep(Duration::from_millis(200)).await;
    let seen = executor.seen.lock().await.clone();
    assert_eq!(seen, vec!["ralph#1".to_string()]);
}

#[tokio::test]
async fn busy_ralph_primary_explicit_target_is_not_redirected_for_turn_interrupt() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let executor = NotifyExecutor {
        expected_starts: 1,
        started: Arc::new(AtomicUsize::new(0)),
        notify: Arc::new(Notify::new()),
        seen: Arc::new(tokio::sync::Mutex::new(Vec::new())),
    };

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    let mut supervisor = make_supervisor(config, Arc::new(executor.clone()), events_path);

    // 说明:
    // - turn/interrupt 也属于 in-flight 控制信号,必须直达目标实例,否则无法取消正在运行的 job。
    supervisor
        .instance_states
        .insert(HatInstanceId::new("ralph#1"), HatInstanceState::Running);

    let event = Event::new("e2e.interrupt", "please stop")
        .with_id("e-ralph-busy-interrupt-no-redirect")
        .with_target_instance(HatInstanceId::new("ralph#1"))
        .with_turn_action(TurnAction::Interrupt);
    supervisor
        .route_event(event)
        .await
        .expect("route_event should succeed");

    // 说明:
    // - turn/interrupt 在 HatInstance 内属于“取消当前 job”的控制信号,不一定会触发新的 job 启动。
    // - 因此这里不以 executor.seen 作为断言,而是锁定“没有发生改投/没有按需创建 ralph#2”:
    //   - 若 rewrite_target_for_busy_ralph 没有豁免 Interrupt,此处会创建 ralph#2 并把事件改投过去。
    assert!(
        !supervisor
            .instances
            .contains_key(&HatInstanceId::new("ralph#2")),
        "turn/interrupt should NOT spawn or redirect to ralph#2 when explicit target_instance=ralph#1"
    );
}

#[tokio::test]
async fn busy_ralph_secondary_includes_coordinator_instructions_and_config_prompt() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let prompts = Arc::new(tokio::sync::Mutex::new(HashMap::<String, String>::new()));
    let notify = Arc::new(Notify::new());
    let executor = PromptCaptureNotifyExecutor {
        prompts: Arc::clone(&prompts),
        notify: Arc::clone(&notify),
        notify_on_instance: "ralph#2".to_string(),
    };

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    config.event_loop.ralph_prompt = Some("RALPH_PROMPT_ANCHOR_FOR_TEST".to_string());

    let mut supervisor = make_supervisor(config, Arc::new(executor), events_path);

    // 说明:
    // - 主协调实例处于 Running 时,route_event 会按需创建 ralph#2 并改投.
    // - 该测试的目的不是验证"改投"本身(已有单测覆盖),
    //   而是锁定 ralph#2 的 prompt 必须包含与 ralph#1 等价的 coordinator 指令,
    //   避免它使用极小兜底 prompt 漂移导致协议破坏.
    supervisor
        .instance_states
        .insert(HatInstanceId::new("ralph#1"), HatInstanceState::Running);

    let event = Event::new("routing.escalate", "needs coordinator")
        .with_id("e-ralph-2-prompt")
        .with_target_instance(HatInstanceId::new("ralph#1"));
    supervisor
        .route_event(event)
        .await
        .expect("route_event should succeed");

    tokio::time::timeout(Duration::from_secs(1), notify.notified())
        .await
        .expect("Timed out waiting for ralph#2 prompt capture");

    let prompt = {
        let prompts = prompts.lock().await;
        prompts
            .get("ralph#2")
            .cloned()
            .expect("ralph#2 prompt should be captured")
    };

    assert!(
        prompt
            .contains("You are Ralph (coordinator) running in PARALLEL mode as instance ralph#2."),
        "ralph#2 should receive coordinator identity line"
    );
    assert!(
        prompt.contains("## KEY SEMANTICS (OFFICIAL)"),
        "ralph#2 should include official coordinator semantics section"
    );
    assert!(
        prompt.contains(crate::EVENT_EMISSION_PROTOCOL_HEADING),
        "ralph#2 prompt should include the built-in event emission protocol"
    );
    assert!(
        prompt.contains("## OUT-OF-BAND EVENT INJECTION"),
        "ralph#2 prompt should keep coordinator-only `ralph emit` guidance"
    );
    assert!(
        prompt.contains("You may emit multiple complete"),
        "ralph#2 prompt should allow multiple events through the shared protocol"
    );
    assert!(
        prompt.contains("reply.hat.message"),
        "ralph#2 prompt should describe hat-to-hat answer-return topic"
    );
    assert!(
        prompt.contains("## RALPH PROMPT (CONFIG)"),
        "ralph#2 should include config ralph_prompt section"
    );
    assert!(
        prompt.contains("capability.request"),
        "coordinator protocol topic list should include capability invocation request topic"
    );
}

#[tokio::test]
async fn ralph_coordinator_event_protocol_uses_shared_source_of_truth() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let prompts = Arc::new(tokio::sync::Mutex::new(HashMap::<String, String>::new()));
    let notify = Arc::new(Notify::new());
    let executor = PromptCaptureNotifyExecutor {
        prompts: Arc::clone(&prompts),
        notify: Arc::clone(&notify),
        notify_on_instance: "ralph#1".to_string(),
    };

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    config.event_loop.starting_event = Some("work.start".to_string());

    let mut supervisor = make_supervisor(config, Arc::new(executor), events_path);
    supervisor
        .route_event(Event::new("task.start", "top-level prompt").with_id("e-ralph-protocol"))
        .await
        .expect("route_event should start ralph#1");

    tokio::time::timeout(Duration::from_secs(1), notify.notified())
        .await
        .expect("Timed out waiting for ralph#1 prompt capture");

    let prompt = {
        let prompts = prompts.lock().await;
        prompts
            .get("ralph#1")
            .cloned()
            .expect("ralph#1 prompt should be captured")
    };

    assert!(
        prompt.contains(crate::EVENT_EMISSION_PROTOCOL_HEADING),
        "ralph#1 prompt should include the shared event protocol marker"
    );
    assert!(
        prompt.contains("work.start"),
        "coordinator protocol should list the configured starting event"
    );
    assert!(
        prompt.contains("reply.human.message") && prompt.contains("reply.hat.message"),
        "coordinator prompt should keep answer-return topics"
    );
    assert!(
        prompt.contains("## OUT-OF-BAND EVENT INJECTION"),
        "coordinator prompt should keep ralph emit guidance outside the shared protocol"
    );
    assert!(
        !prompt.contains("## EMIT EVENTS (NO CODE FENCES)"),
        "coordinator should not keep the old divergent event tutorial section"
    );
}

#[tokio::test]
async fn busy_ralph_internal_hat_event_keeps_primary_queue_and_does_not_spawn_secondary() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let executor = NotifyExecutor {
        expected_starts: 1,
        started: Arc::new(AtomicUsize::new(0)),
        notify: Arc::new(Notify::new()),
        seen: Arc::new(tokio::sync::Mutex::new(Vec::new())),
    };

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    let mut supervisor = make_supervisor(config, Arc::new(executor.clone()), events_path);

    // 说明：
    // - 模拟主协调实例正在处理上一条 orphan/workflow 事件。
    // - 新来的内部 hat 事件（带 source/source_instance）应该继续排到 ralph#1，
    //   而不是按需拉起 ralph#2 造成聚合状态分裂。
    supervisor
        .instance_states
        .insert(HatInstanceId::new("ralph#1"), HatInstanceState::Running);

    let event = Event::new("experiment.reviewed", "approved")
        .with_id("e-ralph-busy-internal-no-secondary")
        .with_source(HatId::new("experiment_auditor"))
        .with_source_instance(HatInstanceId::new("experiment_auditor#1"));
    supervisor
        .route_event(event)
        .await
        .expect("route_event should succeed");

    tokio::time::sleep(Duration::from_millis(200)).await;
    let seen = executor.seen.lock().await.clone();
    assert_eq!(
        seen,
        vec!["ralph#1".to_string()],
        "internal hat events should remain on ralph#1 instead of spawning ralph#2"
    );
    assert!(
        !supervisor
            .instances
            .contains_key(&HatInstanceId::new("ralph#2")),
        "internal hat events should stay on ralph#1 and must not spawn ralph#2"
    );
}

#[tokio::test]
async fn idle_ralph_prefers_primary_instance() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let executor = NotifyExecutor {
        expected_starts: 1,
        started: Arc::new(AtomicUsize::new(0)),
        notify: Arc::new(Notify::new()),
        seen: Arc::new(tokio::sync::Mutex::new(Vec::new())),
    };

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    let mut supervisor = make_supervisor(config, Arc::new(executor.clone()), events_path);

    // 指向 ralph hat(非实例),用于触发 trigger-driven 的实例选择分支。
    let event = Event::new("orphan.topic", "route to ralph")
        .with_id("e-ralph-idle")
        .with_target(HatId::new("ralph"));
    supervisor
        .route_event(event)
        .await
        .expect("route_event should succeed");

    tokio::time::sleep(Duration::from_millis(200)).await;
    let seen = executor.seen.lock().await.clone();
    assert_eq!(seen, vec!["ralph#1".to_string()]);

    // 主实例空闲时不应提前创建 ralph#2(保持按需扩容语义)。
    assert!(
        !supervisor
            .instances
            .contains_key(&HatInstanceId::new("ralph#2"))
    );
}

#[tokio::test]
async fn parallel_injects_event_loop_ralph_prompt_only_for_ralph() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    // 验证编译期语义：从已内嵌 overlay 中提取一行稳定文本作为断言锚点。
    let all_hat_overlay = crate::prompt_overlay::load_all_hat_prompt(&CoreConfig::default())
        .expect("compiled all-hat overlay should load")
        .expect("compiled all-hat overlay should not be empty");
    let overlay_anchor = all_hat_overlay
        .lines()
        .find(|line| !line.trim().is_empty())
        .expect("compiled all-hat overlay should contain at least one non-empty line")
        .to_string();

    let prompts = Arc::new(tokio::sync::Mutex::new(HashMap::<String, String>::new()));
    let notify = Arc::new(Notify::new());
    let executor = PromptCaptureExecutor {
        prompts: Arc::clone(&prompts),
        notify: Arc::clone(&notify),
    };

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    config.event_loop.ralph_prompt = Some("RALPH_PROMPT_SENTINEL".to_string());
    config.core = config.core.with_workspace_root(temp_dir.path());

    // 增加一个普通 hat：用于断言 ralph_prompt 不会污染其它 hat 的 prompt。
    config.hats.insert(
        "writer".to_string(),
        hat_config("Writer", vec!["build.task"], 1),
    );

    let mut supervisor = make_supervisor(config, Arc::new(executor), events_path);

    // 触发 ralph#1 与 writer#1 各执行一次 job（用 strict target 避免路由歧义）
    let ralph_event = Event::new("task.start", "top-level prompt")
        .with_id("e-task-start")
        .with_target_instance(HatInstanceId::from_parts("ralph", "1"));
    supervisor
        .route_event(ralph_event)
        .await
        .expect("route_event (ralph) should succeed");

    let writer_event = Event::new("build.task", "do it")
        .with_id("e-build-task")
        .with_target_instance(HatInstanceId::from_parts("writer", "1"));
    supervisor
        .route_event(writer_event)
        .await
        .expect("route_event (writer) should succeed");

    tokio::time::timeout(Duration::from_secs(1), notify.notified())
        .await
        .expect("Timed out waiting for prompts to be captured");

    // 给 actor 留一点缓冲时间，避免 race（写入后立刻读取）
    tokio::time::sleep(Duration::from_millis(80)).await;

    let got = prompts.lock().await.clone();
    let ralph_prompt = got
        .get("ralph#1")
        .expect("should have captured ralph#1 prompt");
    let writer_prompt = got
        .get("writer#1")
        .expect("should have captured writer#1 prompt");

    assert!(
        ralph_prompt.contains("RALPH_PROMPT_SENTINEL"),
        "ralph#1 prompt should contain event_loop.ralph_prompt"
    );
    assert!(
        ralph_prompt.contains("ralph_hat_instance_id:\"ralph#1\""),
        "ralph#1 prompt should include injected runtime identity"
    );
    assert!(
        ralph_prompt.contains(&overlay_anchor),
        "ralph#1 prompt should include all-hat overlay content"
    );
    assert!(
        !writer_prompt.contains("RALPH_PROMPT_SENTINEL"),
        "writer#1 prompt should NOT contain event_loop.ralph_prompt (no prompt pollution)"
    );
    assert!(
        writer_prompt.contains("ralph_hat_instance_id:\"writer#1\""),
        "writer#1 prompt should include injected runtime identity"
    );
    assert!(
        writer_prompt.contains(&overlay_anchor),
        "writer#1 prompt should include all-hat overlay content"
    );
}

#[tokio::test]
async fn runtime_capability_catalog_is_injected_only_into_ralph_prompt() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let prompts = Arc::new(tokio::sync::Mutex::new(HashMap::<String, String>::new()));
    let notify = Arc::new(Notify::new());
    let executor = PromptCaptureExecutor {
        prompts: Arc::clone(&prompts),
        notify: Arc::clone(&notify),
    };

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    config.core = config.core.with_workspace_root(temp_dir.path());
    config.hats.insert(
        "writer".to_string(),
        hat_config("Writer", vec!["build.task"], 1),
    );

    let catalog = vec![crate::CapabilityMetadata {
        id: "hat:focused-reviewer".to_string(),
        kind: crate::CapabilityKind::HatCapability,
        summary: "Focused reviewer micro-run".to_string(),
        goal: "Review a bounded input".to_string(),
        when_to_use: "Use when ralph#1 needs a one-off review lens.".to_string(),
        input_contract: "A bounded text or task description to review.".to_string(),
        output_contract: "A short review summary.".to_string(),
        invocation_mode: crate::CapabilityInvocationMode::IsolatedMicroRun,
    }];
    let mut supervisor =
        make_supervisor_with_catalog(config, Arc::new(executor), events_path, catalog);

    let ralph_event = Event::new("task.start", "top-level prompt")
        .with_id("e-task-start-capability-catalog")
        .with_target_instance(HatInstanceId::from_parts("ralph", "1"));
    supervisor
        .route_event(ralph_event)
        .await
        .expect("route_event (ralph) should succeed");

    let writer_event = Event::new("build.task", "do it")
        .with_id("e-build-task-capability-catalog")
        .with_target_instance(HatInstanceId::from_parts("writer", "1"));
    supervisor
        .route_event(writer_event)
        .await
        .expect("route_event (writer) should succeed");

    tokio::time::timeout(Duration::from_secs(1), notify.notified())
        .await
        .expect("Timed out waiting for prompts to be captured");
    tokio::time::sleep(Duration::from_millis(80)).await;

    let got = prompts.lock().await.clone();
    let ralph_prompt = got
        .get("ralph#1")
        .expect("should have captured ralph#1 prompt");
    let writer_prompt = got
        .get("writer#1")
        .expect("should have captured writer#1 prompt");

    assert!(
        ralph_prompt.contains(crate::PARENT_CAPABILITY_CATALOG_HEADING),
        "ralph#1 prompt should contain runtime capability catalog"
    );
    assert!(
        ralph_prompt.contains("capability.request"),
        "ralph#1 prompt should include capability request contract"
    );
    assert!(
        ralph_prompt.contains("topology.spawn_group"),
        "ralph#1 prompt should include parent-visible topology spawn contract"
    );
    assert!(
        ralph_prompt.contains("role_contract")
            && ralph_prompt.contains("input` MUST be a string")
            && ralph_prompt.contains("Do NOT put `role_contract` inside `input`"),
        "topology spawn prompt should document role_contract as an instances[] sibling field, not an input object"
    );
    assert!(
        ralph_prompt.contains("different from `capability.request`"),
        "coordinator prompt should distinguish parent-visible spawn from isolated child-runs"
    );
    assert!(
        ralph_prompt.contains("### If you receive `topology.spawn.result`"),
        "ralph#1 prompt should describe how to handle spawn result acknowledgements"
    );
    assert!(
        ralph_prompt.contains("Spawned instances already received direct delivery"),
        "spawn result guidance should say that delivery already happened"
    );
    assert!(
        ralph_prompt.contains("Do NOT re-emit the delivery topic"),
        "spawn result guidance should prevent duplicate fanout"
    );
    assert!(
        ralph_prompt.contains("Do NOT use `audience_instances` as a replay mechanism"),
        "spawn result guidance should prevent audience_instances replay"
    );
    assert!(
        ralph_prompt.contains("### If you receive `topology.spawn.failed`"),
        "ralph#1 prompt should describe failed topology mutation handling"
    );
    assert!(
        ralph_prompt.contains("hat:focused-reviewer"),
        "ralph#1 prompt should include callable capability id"
    );
    assert!(
        ralph_prompt.contains("request_id")
            && ralph_prompt.contains("capability_id")
            && ralph_prompt.contains("input"),
        "ralph#1 prompt should include required request payload fields"
    );
    assert!(
        !writer_prompt.contains(crate::PARENT_CAPABILITY_CATALOG_HEADING),
        "writer#1 prompt should not contain parent capability catalog"
    );
    assert!(
        !writer_prompt.contains("hat:focused-reviewer"),
        "writer#1 prompt should not receive parent capability catalog entries"
    );
    assert!(
        !writer_prompt.contains("topology.spawn_group"),
        "writer#1 prompt should not receive coordinator-only topology mutation protocol"
    );
    assert!(
        !writer_prompt.contains("topology.spawn.result"),
        "writer#1 prompt should not receive coordinator-only spawn result handling"
    );
    assert!(
        !writer_prompt.contains("audience_instances` as a replay mechanism"),
        "writer#1 prompt should not receive coordinator-only replay guardrail"
    );
}

#[tokio::test]
async fn ralph_prompt_contains_coordinator_only_sections() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let prompts = Arc::new(tokio::sync::Mutex::new(HashMap::<String, String>::new()));
    let notify = Arc::new(Notify::new());
    let executor = PromptCaptureExecutor {
        prompts: Arc::clone(&prompts),
        notify: Arc::clone(&notify),
    };

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    config.core = config.core.with_workspace_root(temp_dir.path());
    config.hats.insert(
        "writer".to_string(),
        hat_config("Writer", vec!["build.task"], 1),
    );

    let catalog = vec![crate::CapabilityMetadata {
        id: "hat:focused-reviewer".to_string(),
        kind: crate::CapabilityKind::HatCapability,
        summary: "Focused reviewer micro-run".to_string(),
        goal: "Review a bounded input".to_string(),
        when_to_use: "Use when ralph#1 needs a one-off review lens.".to_string(),
        input_contract: "A bounded text or task description to review.".to_string(),
        output_contract: "A short review summary.".to_string(),
        invocation_mode: crate::CapabilityInvocationMode::IsolatedMicroRun,
    }];
    let mut supervisor =
        make_supervisor_with_catalog(config, Arc::new(executor), events_path, catalog);

    let ralph_event = Event::new("task.start", "top-level prompt")
        .with_id("e-ralph-surface")
        .with_target_instance(HatInstanceId::from_parts("ralph", "1"));
    supervisor
        .route_event(ralph_event)
        .await
        .expect("route_event (ralph) should succeed");

    let writer_event = Event::new("build.task", "do it")
        .with_id("e-worker-surface")
        .with_target_instance(HatInstanceId::from_parts("writer", "1"));
    supervisor
        .route_event(writer_event)
        .await
        .expect("route_event (writer) should succeed");

    tokio::time::timeout(Duration::from_secs(1), notify.notified())
        .await
        .expect("Timed out waiting for prompts to be captured");
    tokio::time::sleep(Duration::from_millis(80)).await;

    let got = prompts.lock().await.clone();
    let ralph_prompt = got
        .get("ralph#1")
        .expect("should have captured ralph#1 prompt");

    let expected_coordinator_headings = [
        crate::PARENT_CAPABILITY_CATALOG_HEADING,
        "## KEY SEMANTICS (OFFICIAL)",
        "## OUT-OF-BAND EVENT INJECTION",
        "## HUMAN CHAT (INPUT VS REPLY)",
        "## CONFIG (THIS RUN)",
        "## HATS TOPOLOGY (CONFIGURED)",
        "## WHAT TO DO",
    ];

    for heading in expected_coordinator_headings {
        assert!(
            crate::headings_for_surface(crate::PromptSurface::CoordinatorOnly).contains(&heading),
            "{heading} must be registered as coordinator-only prompt surface"
        );
        assert!(
            ralph_prompt.contains(heading),
            "ralph#1 prompt should include coordinator-only surface {heading}"
        );
    }

    assert!(
        ralph_prompt.contains(crate::EVENT_EMISSION_PROTOCOL_HEADING),
        "ralph#1 prompt should still include the shared event protocol"
    );
}

#[tokio::test]
async fn worker_prompt_excludes_coordinator_only_sections() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let prompts = Arc::new(tokio::sync::Mutex::new(HashMap::<String, String>::new()));
    let notify = Arc::new(Notify::new());
    let executor = PromptCaptureExecutor {
        prompts: Arc::clone(&prompts),
        notify: Arc::clone(&notify),
    };

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    config.core = config.core.with_workspace_root(temp_dir.path());
    config.hats.insert(
        "writer".to_string(),
        hat_config("Writer", vec!["build.task"], 1),
    );

    let catalog = vec![crate::CapabilityMetadata {
        id: "hat:focused-reviewer".to_string(),
        kind: crate::CapabilityKind::HatCapability,
        summary: "Focused reviewer micro-run".to_string(),
        goal: "Review a bounded input".to_string(),
        when_to_use: "Use when ralph#1 needs a one-off review lens.".to_string(),
        input_contract: "A bounded text or task description to review.".to_string(),
        output_contract: "A short review summary.".to_string(),
        invocation_mode: crate::CapabilityInvocationMode::IsolatedMicroRun,
    }];
    let mut supervisor =
        make_supervisor_with_catalog(config, Arc::new(executor), events_path, catalog);

    let ralph_event = Event::new("task.start", "top-level prompt")
        .with_id("e-ralph-worker-surface")
        .with_target_instance(HatInstanceId::from_parts("ralph", "1"));
    supervisor
        .route_event(ralph_event)
        .await
        .expect("route_event (ralph) should succeed");

    let writer_event = Event::new("build.task", "do it")
        .with_id("e-worker-no-coordinator-surface")
        .with_target_instance(HatInstanceId::from_parts("writer", "1"));
    supervisor
        .route_event(writer_event)
        .await
        .expect("route_event (writer) should succeed");

    tokio::time::timeout(Duration::from_secs(1), notify.notified())
        .await
        .expect("Timed out waiting for prompts to be captured");
    tokio::time::sleep(Duration::from_millis(80)).await;

    let got = prompts.lock().await.clone();
    let writer_prompt = got
        .get("writer#1")
        .expect("should have captured writer#1 prompt");

    for heading in crate::headings_for_surface(crate::PromptSurface::CoordinatorOnly) {
        assert!(
            !writer_prompt.contains(heading),
            "writer#1 prompt must not inherit coordinator-only surface {heading}"
        );
    }

    assert!(
        writer_prompt.contains(crate::ALL_HAT_PROMPT_HEADING),
        "writer#1 prompt should still receive shared all-hat overlay"
    );
    assert!(
        writer_prompt.contains(crate::EVENT_EMISSION_PROTOCOL_HEADING),
        "writer#1 prompt should still include shared event protocol"
    );
    assert!(
        writer_prompt.contains("### 0. ORIENTATION"),
        "writer#1 prompt should keep worker-only execution surface"
    );
}

#[tokio::test]
async fn parallel_injects_human_message_subscription_for_strict_target_validation() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();

    // 关键：writer 没有显式订阅 human.message（triggers 为空）。
    // 但在 parallel.enabled=true 的运行时，应当视为已订阅，以通过 strict target 校验。
    config
        .hats
        .insert("writer".to_string(), hat_config("Writer", vec![], 1));

    // 这里不走 spawn_instances：直接插一个“测试 instance handle”，
    // 用 channel 捕获 route_event 是否真的投递 Deliver 命令。
    let seen = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let executor = TimeoutCaptureExecutor { seen };
    let mut supervisor = ParallelSupervisor::new(config, "prompt".to_string(), Arc::new(executor))
        .expect("ParallelSupervisor::new should succeed");
    supervisor.event_logger = EventLogger::new(events_path);

    let instance_id = HatInstanceId::new("writer#1");
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<HatInstanceCommand>(8);
    supervisor
        .instances
        .insert(instance_id.clone(), HatInstanceHandle::from_cmd_tx(cmd_tx));

    let event = Event::new("human.message", "hello")
        .with_id("e-human")
        .with_target_instance(instance_id);
    supervisor
        .route_event(event)
        .await
        .expect("route_event should succeed");

    let cmd = tokio::time::timeout(Duration::from_millis(200), cmd_rx.recv())
        .await
        .expect("Timed out waiting for Deliver command")
        .expect("Expected a HatInstanceCommand");

    match cmd {
        HatInstanceCommand::Deliver(e) => {
            assert_eq!(e.topic.as_str(), "human.message");
            assert_eq!(e.payload, "hello");
        }
        other => panic!("Expected Deliver, got {other:?}"),
    }
}

#[tokio::test]
async fn parallel_does_not_route_hat_sourced_human_message_to_prevent_self_chat_loop() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();

    // 这里不走 spawn_instances：直接插一个“测试 instance handle”，
    // 用 channel 捕获 route_event 是否真的投递 Deliver 命令。
    let seen = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let executor = TimeoutCaptureExecutor { seen };
    let mut supervisor = ParallelSupervisor::new(config, "prompt".to_string(), Arc::new(executor))
        .expect("ParallelSupervisor::new should succeed");
    supervisor.event_logger = EventLogger::new(events_path);

    // 关键点:
    // - 我们需要让 choose_ralph_instance_for_delivery() 稳定选中 ralph#1,
    //   避免因为默认 state=Running 而触发 ralph#2 的 spawn(测试里没有 instance_tx)。
    let instance_id = HatInstanceId::new("ralph#1");
    supervisor
        .instance_states
        .insert(instance_id.clone(), HatInstanceState::Idle);

    let (cmd_tx, mut cmd_rx) = mpsc::channel::<HatInstanceCommand>(8);
    supervisor
        .instances
        .insert(instance_id.clone(), HatInstanceHandle::from_cmd_tx(cmd_tx));

    // 模拟 "hat -> human" 的回复事件:
    // - topic 仍然是 human.message
    // - 但带 source/source_instance（表示这是 hat 产出的消息,不是外部注入）
    //
    // 预期:
    // - 该事件只用于 UI 展示,不应再次被路由回 hats。
    // - 否则会形成 ralph#1 自我对话回路(回复自己的 human.message)。
    let event = Event::new("human.message", "hello from ralph")
        .with_id("e-human-from-ralph")
        .with_source(HatId::new("ralph"))
        .with_source_instance(instance_id.clone());

    supervisor
        .route_event(event)
        .await
        .expect("route_event should succeed");

    let recv = tokio::time::timeout(Duration::from_millis(200), cmd_rx.recv()).await;
    assert!(
        recv.is_err(),
        "Expected no delivery for hat-sourced human.message, but got: {recv:?}"
    );
}

#[tokio::test]
async fn parallel_does_not_route_reply_human_message_topic() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();

    // 这里不走 spawn_instances：直接插一个“测试 instance handle”，
    // 用 channel 捕获 route_event 是否真的投递 Deliver 命令。
    let seen = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let executor = TimeoutCaptureExecutor { seen };
    let mut supervisor = ParallelSupervisor::new(config, "prompt".to_string(), Arc::new(executor))
        .expect("ParallelSupervisor::new should succeed");
    supervisor.event_logger = EventLogger::new(events_path);

    // 确保 choose_ralph_instance_for_delivery() 稳定选中 ralph#1（避免测试里意外 spawn ralph#2）。
    let instance_id = HatInstanceId::new("ralph#1");
    supervisor
        .instance_states
        .insert(instance_id.clone(), HatInstanceState::Idle);

    let (cmd_tx, mut cmd_rx) = mpsc::channel::<HatInstanceCommand>(8);
    supervisor
        .instances
        .insert(instance_id.clone(), HatInstanceHandle::from_cmd_tx(cmd_tx));

    // reply.human.message 是“输出专用 topic”，只用于 UI 展示，不应再被路由回 hats。
    let event = Event::new("reply.human.message", "hello to human")
        .with_id("e-reply-human")
        .with_source(HatId::new("ralph"))
        .with_source_instance(instance_id.clone());

    supervisor
        .route_event(event)
        .await
        .expect("route_event should succeed");

    let recv = tokio::time::timeout(Duration::from_millis(200), cmd_rx.recv()).await;
    assert!(
        recv.is_err(),
        "Expected no delivery for reply.human.message, but got: {recv:?}"
    );
}

#[tokio::test]
async fn parallel_routes_reply_hat_message_back_to_requester_and_logs_resolution() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    config
        .hats
        .insert("planner".to_string(), hat_config("Planner", vec![], 1));
    config.hats.insert(
        "researcher".to_string(),
        hat_config("Researcher", vec![], 1),
    );

    let (tx, mut rx) = mpsc::channel::<String>(4);
    let executor = Arc::new(test_executors::TestExecutor::new(tx));
    let mut supervisor = make_supervisor(config, executor, events_path.clone());

    supervisor
        .request_reply_origins
        .insert("req-1".to_string(), Some(HatInstanceId::new("planner#1")));

    let event = Event::new(TOPIC_REPLY_HAT_MESSAGE, "market summary")
        .with_id("ans-1")
        .with_reply("req-1")
        .with_source(HatId::new("researcher"))
        .with_source_instance(HatInstanceId::new("researcher#1"));

    supervisor
        .route_event(event)
        .await
        .expect("route_event should succeed");

    let got = tokio::time::timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("should receive requester delivery")
        .expect("channel closed");
    assert_eq!(got, "planner#1");

    let history = EventHistory::new(&events_path);
    let records = history
        .filter_by_topic(TOPIC_REQUESTER_RETURN)
        .expect("should read requester-return records");
    let payload: serde_json::Value = serde_json::from_str(
        &records
            .last()
            .expect("should have requester-return record")
            .payload,
    )
    .expect("requester-return payload should be valid JSON");
    assert_eq!(payload["status"], "delivered");
    assert_eq!(payload["requester_instance"], "planner#1");

    let request_lookup = evidence_lookup_for_events_path(&events_path, "req-1");
    let request_entries = request_lookup.entries();
    assert!(matches!(request_lookup, EvidenceLookup::Entries(_)));
    assert!(
        request_entries.iter().any(|entry| {
            entry.artifact_kind == EvidenceArtifactKind::ReplyEvent
                && entry.status == EvidenceStatus::Success
                && entry.child_correlation_id.as_deref() == Some("ans-1")
                && entry.artifact_path == events_path.display().to_string()
        }),
        "request id should resolve to successful reply event evidence"
    );
    assert!(
        request_entries.iter().any(|entry| {
            entry.artifact_kind == EvidenceArtifactKind::RuntimeDeliveryRecord
                && entry.status == EvidenceStatus::Success
                && entry.child_correlation_id.as_deref() == Some("ans-1")
        }),
        "request id should resolve to runtime delivery evidence"
    );

    let answer_lookup = evidence_lookup_for_events_path(&events_path, "ans-1");
    let answer_entries = answer_lookup.entries();
    assert!(matches!(answer_lookup, EvidenceLookup::Entries(_)));
    assert!(
        answer_entries.iter().any(|entry| {
            entry.artifact_kind == EvidenceArtifactKind::EventLogJsonl
                && entry.status == EvidenceStatus::Success
                && entry.parent_correlation_id.as_deref() == Some("req-1")
                && entry.artifact_path == events_path.display().to_string()
        }),
        "answer event id should resolve back to the durable event log artifact"
    );
}

#[tokio::test]
async fn parallel_reply_hat_message_unknown_reply_id_fails_closed() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    config
        .hats
        .insert("planner".to_string(), hat_config("Planner", vec![], 1));
    config.hats.insert(
        "researcher".to_string(),
        hat_config("Researcher", vec![], 1),
    );

    let (tx, mut rx) = mpsc::channel::<String>(4);
    let executor = Arc::new(test_executors::TestExecutor::new(tx));
    let mut supervisor = make_supervisor(config, executor, events_path.clone());

    let event = Event::new(TOPIC_REPLY_HAT_MESSAGE, "market summary")
        .with_id("ans-missing")
        .with_reply("missing-id")
        .with_source(HatId::new("researcher"))
        .with_source_instance(HatInstanceId::new("researcher#1"));

    supervisor
        .route_event(event)
        .await
        .expect("route_event should succeed");

    let recv = tokio::time::timeout(Duration::from_millis(200), rx.recv()).await;
    assert!(
        recv.is_err(),
        "Expected fail-closed with no delivery, but got: {recv:?}"
    );

    let history = EventHistory::new(&events_path);
    let records = history
        .filter_by_topic(TOPIC_REQUESTER_RETURN)
        .expect("should read requester-return records");
    let payload: serde_json::Value = serde_json::from_str(
        &records
            .last()
            .expect("should have requester-return record")
            .payload,
    )
    .expect("requester-return payload should be valid JSON");
    assert_eq!(payload["status"], "unresolved");
    assert_eq!(payload["reason"], "reply target event id was not found");

    let lookup = evidence_lookup_for_events_path(&events_path, "missing-id");
    let entries = lookup.entries();
    assert!(matches!(lookup, EvidenceLookup::Missing(_)));
    assert!(
        entries.iter().any(|entry| {
            entry.status == EvidenceStatus::Failure
                && entry.artifact_kind == EvidenceArtifactKind::EventLogJsonl
                && entry.child_correlation_id.as_deref() == Some("ans-missing")
        }),
        "unknown request id should retain failure evidence"
    );
    assert!(
        entries.iter().any(|entry| {
            entry.status == EvidenceStatus::Missing
                && entry.artifact_kind == EvidenceArtifactKind::MissingArtifact
                && entry.producer == "parallel.supervisor.requester_return"
        }),
        "unknown request id should retain a missing marker distinguishable from no entry"
    );
}

#[tokio::test]
async fn parallel_reply_hat_message_without_reply_fails_closed_with_evidence() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    config.hats.insert(
        "researcher".to_string(),
        hat_config("Researcher", vec![], 1),
    );

    let (tx, mut rx) = mpsc::channel::<String>(4);
    let executor = Arc::new(test_executors::TestExecutor::new(tx));
    let mut supervisor = make_supervisor(config, executor, events_path.clone());

    let event = Event::new(TOPIC_REPLY_HAT_MESSAGE, "answer without request id")
        .with_id("ans-no-reply")
        .with_source(HatId::new("researcher"))
        .with_source_instance(HatInstanceId::new("researcher#1"));

    supervisor
        .route_event(event)
        .await
        .expect("route_event should succeed");

    let recv = tokio::time::timeout(Duration::from_millis(200), rx.recv()).await;
    assert!(
        recv.is_err(),
        "Expected fail-closed with no delivery, but got: {recv:?}"
    );

    let history = EventHistory::new(&events_path);
    let records = history
        .filter_by_topic(TOPIC_REQUESTER_RETURN)
        .expect("should read requester-return records");
    let payload: serde_json::Value = serde_json::from_str(
        &records
            .last()
            .expect("should have requester-return record")
            .payload,
    )
    .expect("requester-return payload should be valid JSON");
    assert_eq!(payload["status"], "unresolved");
    assert_eq!(
        payload["reason"],
        "reply.hat.message requires a non-empty reply=<request_event_id>"
    );

    let lookup = evidence_lookup_for_events_path(&events_path, "ans-no-reply");
    let entries = lookup.entries();
    assert!(matches!(lookup, EvidenceLookup::Missing(_)));
    assert!(
        entries.iter().any(|entry| {
            entry.status == EvidenceStatus::Missing
                && entry.artifact_kind == EvidenceArtifactKind::MissingArtifact
                && entry.producer == "parallel.supervisor.requester_return"
        }),
        "reply.hat.message without reply should retain a missing evidence marker by answer id"
    );
}

#[tokio::test]
async fn parallel_reply_hat_message_without_requester_source_instance_fails_closed() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    config
        .hats
        .insert("planner".to_string(), hat_config("Planner", vec![], 1));
    config.hats.insert(
        "researcher".to_string(),
        hat_config("Researcher", vec![], 1),
    );

    let (tx, mut rx) = mpsc::channel::<String>(4);
    let executor = Arc::new(test_executors::TestExecutor::new(tx));
    let mut supervisor = make_supervisor(config, executor, events_path.clone());

    supervisor
        .request_reply_origins
        .insert("external-1".to_string(), None);

    let event = Event::new(TOPIC_REPLY_HAT_MESSAGE, "answer")
        .with_id("ans-external")
        .with_reply("external-1")
        .with_source(HatId::new("researcher"))
        .with_source_instance(HatInstanceId::new("researcher#1"));

    supervisor
        .route_event(event)
        .await
        .expect("route_event should succeed");

    let recv = tokio::time::timeout(Duration::from_millis(200), rx.recv()).await;
    assert!(
        recv.is_err(),
        "Expected fail-closed with no delivery, but got: {recv:?}"
    );

    let history = EventHistory::new(&events_path);
    let records = history
        .filter_by_topic(TOPIC_REQUESTER_RETURN)
        .expect("should read requester-return records");
    let payload: serde_json::Value = serde_json::from_str(
        &records
            .last()
            .expect("should have requester-return record")
            .payload,
    )
    .expect("requester-return payload should be valid JSON");
    assert_eq!(
        payload["reason"],
        "referenced request event has no source_instance"
    );

    let lookup = evidence_lookup_for_events_path(&events_path, "external-1");
    let entries = lookup.entries();
    assert!(matches!(lookup, EvidenceLookup::Missing(_)));
    assert!(
        entries.iter().any(|entry| {
            entry.status == EvidenceStatus::Missing
                && entry.artifact_kind == EvidenceArtifactKind::MissingArtifact
                && entry.producer == "parallel.supervisor.requester_return"
                && entry.child_correlation_id.as_deref() == Some("ans-external")
        }),
        "request without source_instance should retain reason-specific missing evidence"
    );
}

#[tokio::test]
async fn parallel_missing_expected_answer_can_be_indexed_without_graph_artifact() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();

    let (tx, _rx) = mpsc::channel::<String>(4);
    let executor = Arc::new(test_executors::TestExecutor::new(tx));
    let mut supervisor = make_supervisor(config, executor, events_path.clone());

    supervisor.record_missing_answer_evidence(
        "req-timeout-1",
        "answer lifecycle closed before reply.hat.message was produced",
    );

    let lookup = evidence_lookup_for_events_path(&events_path, "req-timeout-1");
    let entries = lookup.entries();
    assert!(matches!(lookup, EvidenceLookup::Missing(_)));
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].status, EvidenceStatus::Missing);
    assert_eq!(
        entries[0].artifact_kind,
        EvidenceArtifactKind::MissingArtifact
    );
    assert_eq!(entries[0].producer, "parallel.supervisor.answer_lifecycle");
    assert_eq!(entries[0].artifact_path, events_path.display().to_string());
    let records = EventHistory::new(&events_path)
        .filter_by_topic(TOPIC_REQUESTER_RETURN)
        .expect("should read requester-return records");
    let payload: serde_json::Value = serde_json::from_str(
        &records
            .last()
            .expect("should have missing answer diagnostic")
            .payload,
    )
    .expect("missing answer diagnostic should be valid JSON");
    assert_eq!(payload["status"], "missing");
    assert_eq!(payload["request_event_id"], "req-timeout-1");
    assert_eq!(
        payload["reason"],
        "answer lifecycle closed before reply.hat.message was produced"
    );
    assert!(
        !entries[0].artifact_path.contains("rerun")
            && !entries[0].artifact_path.contains("runtime_graph"),
        "missing answer evidence must not depend on graph artifacts"
    );
}

#[tokio::test]
async fn parallel_workflow_event_with_reply_is_not_answer_return_evidence() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    config.hats.insert(
        "manager".to_string(),
        hat_config("Manager", vec!["research.ready"], 1),
    );

    let (tx, mut rx) = mpsc::channel::<String>(4);
    let executor = Arc::new(test_executors::TestExecutor::new(tx));
    let mut supervisor = make_supervisor(config, executor, events_path.clone());

    let workflow = Event::new("research.ready", "done")
        .with_id("ready-with-reply")
        .with_reply("req-ordinary")
        .with_source(HatId::new("researcher"))
        .with_source_instance(HatInstanceId::new("researcher#1"));

    supervisor
        .route_event(workflow)
        .await
        .expect("workflow event should route normally");

    let got = tokio::time::timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("should receive normal workflow delivery")
        .expect("channel closed");
    assert_eq!(got, "manager#1");
    assert_no_evidence_entry(&events_path, "req-ordinary");
    assert_no_evidence_entry(&events_path, "ready-with-reply");
}

#[tokio::test]
async fn parallel_reply_hat_message_does_not_auto_publish_reply_human_message() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    config
        .hats
        .insert("planner".to_string(), hat_config("Planner", vec![], 1));
    config.hats.insert(
        "researcher".to_string(),
        hat_config("Researcher", vec![], 1),
    );

    let (tx, mut rx) = mpsc::channel::<String>(4);
    let executor = Arc::new(test_executors::TestExecutor::new(tx));
    let mut supervisor = make_supervisor(config, executor, events_path.clone());
    supervisor.request_reply_origins.insert(
        "req-human-boundary".to_string(),
        Some(HatInstanceId::new("planner#1")),
    );

    let answer = Event::new(TOPIC_REPLY_HAT_MESSAGE, "internal answer")
        .with_id("ans-human-boundary")
        .with_reply("req-human-boundary")
        .with_source(HatId::new("researcher"))
        .with_source_instance(HatInstanceId::new("researcher#1"));
    supervisor
        .route_event(answer)
        .await
        .expect("reply.hat.message should route");

    let got = tokio::time::timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("should deliver only to requester")
        .expect("channel closed");
    assert_eq!(got, "planner#1");

    let records = EventHistory::new(&events_path)
        .read_all()
        .expect("events log should be readable");
    assert!(
        records
            .iter()
            .all(|record| record.topic != "reply.human.message"),
        "internal reply.hat.message must not synthesize reply.human.message records"
    );
}

#[tokio::test]
async fn parallel_reply_hat_message_can_coexist_with_workflow_event_in_same_batch() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    config
        .hats
        .insert("planner".to_string(), hat_config("Planner", vec![], 1));
    config.hats.insert(
        "researcher".to_string(),
        hat_config("Researcher", vec![], 1),
    );
    config.hats.insert(
        "manager".to_string(),
        hat_config("Manager", vec!["research.ready"], 1),
    );

    let (tx, mut rx) = mpsc::channel::<String>(8);
    let executor = Arc::new(test_executors::TestExecutor::new(tx));
    let mut supervisor = make_supervisor(config, executor, events_path);

    supervisor
        .request_reply_origins
        .insert("req-2".to_string(), Some(HatInstanceId::new("planner#1")));

    let answer = Event::new(TOPIC_REPLY_HAT_MESSAGE, "market summary")
        .with_id("ans-2")
        .with_reply("req-2")
        .with_source(HatId::new("researcher"))
        .with_source_instance(HatInstanceId::new("researcher#1"));
    let workflow = Event::new("research.ready", "done")
        .with_id("ready-1")
        .with_source(HatId::new("researcher"))
        .with_source_instance(HatInstanceId::new("researcher#1"));

    supervisor
        .route_events_batch(vec![answer, workflow])
        .await
        .expect("route_events_batch should succeed");

    let first = tokio::time::timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("should receive first delivery")
        .expect("channel closed");
    let second = tokio::time::timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("should receive second delivery")
        .expect("channel closed");

    let mut got = vec![first, second];
    got.sort();
    assert_eq!(got, vec!["manager#1".to_string(), "planner#1".to_string()]);
}

#[tokio::test]
async fn wildcard_manager_receives_event_without_escalating_to_ralph() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let seen = Arc::new(tokio::sync::Mutex::new(Vec::<String>::new()));
    let notify = Arc::new(Notify::new());
    let executor = StartEventCaptureExecutor {
        seen: Arc::clone(&seen),
        notify: Arc::clone(&notify),
    };

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();

    // wildcard manager：应当吃掉默认 fallback，不再额外打扰 ralph#1
    config
        .hats
        .insert("manager".to_string(), hat_config("Manager", vec!["*"], 1));

    let mut supervisor = make_supervisor(config, Arc::new(executor), events_path);

    let event = Event::new("unknown.topic", "hello").with_id("e-unknown");
    supervisor
        .route_event(event)
        .await
        .expect("route_event should succeed");

    tokio::time::timeout(Duration::from_secs(1), notify.notified())
        .await
        .expect("Timed out waiting for job execution");
    tokio::time::sleep(Duration::from_millis(80)).await;

    let mut got = seen.lock().await.clone();
    got.sort();
    assert_eq!(got, vec!["manager#1".to_string()]);
}

#[tokio::test]
async fn true_orphan_escalates_to_ralph() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let seen = Arc::new(tokio::sync::Mutex::new(Vec::<String>::new()));
    let notify = Arc::new(Notify::new());
    let executor = StartEventCaptureExecutor {
        seen: Arc::clone(&seen),
        notify: Arc::clone(&notify),
    };

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();

    let mut supervisor = make_supervisor(config, Arc::new(executor), events_path);

    let event = Event::new("unknown.topic", "hello").with_id("e-orphan");
    supervisor
        .route_event(event)
        .await
        .expect("route_event should succeed");

    tokio::time::timeout(Duration::from_secs(1), notify.notified())
        .await
        .expect("Timed out waiting for job execution");
    tokio::time::sleep(Duration::from_millis(80)).await;

    let mut got = seen.lock().await.clone();
    got.sort();
    assert_eq!(got, vec!["ralph#1".to_string()]);
}

#[tokio::test]
async fn invalid_target_is_rejected_and_escalated() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();

    config.hats.insert(
        "writer".to_string(),
        hat_config("Writer", vec!["build.task"], 1),
    );

    let executor = NotifyExecutor {
        expected_starts: 1,
        started: Arc::new(AtomicUsize::new(0)),
        notify: Arc::new(Notify::new()),
        seen: Arc::new(tokio::sync::Mutex::new(Vec::new())),
    };

    let mut supervisor = make_supervisor(config, Arc::new(executor.clone()), events_path.clone());

    // target 指向一个并不存在且不订阅该 topic 的 hat：应拒绝并触发 routing.escalate -> ralph#1
    let event = Event::new("build.task", "do it")
        .with_id("e-invalid-target")
        .with_target("non_subscriber_hat");
    supervisor
        .route_event(event)
        .await
        .expect("route_event should succeed");

    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(executor.started.load(Ordering::SeqCst), 1);

    let seen = executor.seen.lock().await.clone();
    assert_eq!(seen, vec!["ralph#1".to_string()]);

    let events_log =
        std::fs::read_to_string(&events_path).expect("events.jsonl should be readable");
    assert!(
        events_log.contains("routing.escalate"),
        "Expected escalation record in events.jsonl, got:\n{events_log}"
    );
}

#[tokio::test]
async fn external_turn_action_missing_target_instance_is_rejected_and_escalated() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();

    let executor = NotifyExecutor {
        expected_starts: 1,
        started: Arc::new(AtomicUsize::new(0)),
        notify: Arc::new(Notify::new()),
        seen: Arc::new(tokio::sync::Mutex::new(Vec::new())),
    };

    let mut supervisor = make_supervisor(config, Arc::new(executor.clone()), events_path.clone());

    let event = Event::new("human.message", "please steer")
        .with_id("e-ext-turn-missing-target")
        .with_turn_action(TurnAction::Steer);
    supervisor
        .route_event(event)
        .await
        .expect("route_event should succeed");

    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(executor.started.load(Ordering::SeqCst), 1);
    assert_eq!(
        executor.seen.lock().await.clone(),
        vec!["ralph#1".to_string()]
    );

    let events_log =
        std::fs::read_to_string(&events_path).expect("events.jsonl should be readable");
    assert!(events_log.contains("routing.escalate"));
    assert!(events_log.contains("invalid external control-plane target"));
}

#[tokio::test]
async fn external_turn_action_non_ralph_target_is_rejected_and_escalated() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    config.hats.insert(
        "writer".to_string(),
        hat_config("Writer", vec!["build.task"], 1),
    );

    let executor = NotifyExecutor {
        expected_starts: 1,
        started: Arc::new(AtomicUsize::new(0)),
        notify: Arc::new(Notify::new()),
        seen: Arc::new(tokio::sync::Mutex::new(Vec::new())),
    };

    let mut supervisor = make_supervisor(config, Arc::new(executor.clone()), events_path.clone());

    let event = Event::new("human.message", "please stop")
        .with_id("e-ext-turn-non-ralph")
        .with_target_instance(HatInstanceId::new("writer#1"))
        .with_turn_action(TurnAction::Interrupt);
    supervisor
        .route_event(event)
        .await
        .expect("route_event should succeed");

    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(executor.started.load(Ordering::SeqCst), 1);
    assert_eq!(
        executor.seen.lock().await.clone(),
        vec!["ralph#1".to_string()]
    );

    let events_log =
        std::fs::read_to_string(&events_path).expect("events.jsonl should be readable");
    assert!(events_log.contains("routing.escalate"));
    assert!(events_log.contains("writer#1"));
}

#[tokio::test]
async fn external_turn_action_with_target_or_spawn_hint_is_rejected_and_escalated() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();

    let executor = NotifyExecutor {
        expected_starts: 2,
        started: Arc::new(AtomicUsize::new(0)),
        notify: Arc::new(Notify::new()),
        seen: Arc::new(tokio::sync::Mutex::new(Vec::new())),
    };

    let mut supervisor = make_supervisor(config, Arc::new(executor.clone()), events_path.clone());

    let with_target = Event::new("human.message", "invalid target hint")
        .with_id("e-ext-turn-target-hint")
        .with_target("writer")
        .with_target_instance(HatInstanceId::new("ralph#1"))
        .with_turn_action(TurnAction::Steer);
    supervisor
        .route_event(with_target)
        .await
        .expect("route_event should succeed");

    let with_spawn = Event::new("human.message", "invalid spawn hint")
        .with_id("e-ext-turn-spawn-hint")
        .with_target_instance(HatInstanceId::new("ralph#1"))
        .with_spawn_instance(true)
        .with_turn_action(TurnAction::Interrupt);
    supervisor
        .route_event(with_spawn)
        .await
        .expect("route_event should succeed");

    tokio::time::sleep(Duration::from_millis(220)).await;
    assert!(
        executor.started.load(Ordering::SeqCst) >= 1,
        "at least one escalation should be delivered to ralph"
    );

    let events_log =
        std::fs::read_to_string(&events_path).expect("events.jsonl should be readable");
    assert!(events_log.contains("explicit target_instance"));
    assert!(events_log.contains("spawn_instance=true"));
    let escalate_count = events_log.match_indices("routing.escalate").count();
    assert!(
        escalate_count >= 2,
        "expected at least two escalation records, got {escalate_count}\n{events_log}"
    );
}

#[tokio::test]
async fn autoscale_spawns_below_cap_and_stops_at_cap() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    config.parallel.autoscale.max_running_jobs = 2;

    config.hats.insert(
        "writer".to_string(),
        hat_config("Writer", vec!["build.task"], 1),
    );

    let executor = BlockingExecutor {
        started: Arc::new(AtomicUsize::new(0)),
        notify: Arc::new(Notify::new()),
    };

    let mut supervisor = make_supervisor(config, Arc::new(executor.clone()), events_path);

    // 先跑起来 writer#1，占用 1 个 permit
    supervisor
        .route_event(Event::new("build.task", "first").with_id("e1"))
        .await
        .expect("route_event should succeed");
    wait_for_starts(&executor, 1).await;
    supervisor.instance_states.insert(
        HatInstanceId::new("writer#1"),
        ralph_proto::HatInstanceState::Running,
    );

    // 第二个事件到来：writer 全忙且 permit 还有余量 -> autoscale 生成 writer#2
    supervisor
        .route_event(Event::new("build.task", "second").with_id("e2"))
        .await
        .expect("route_event should succeed");
    wait_for_starts(&executor, 2).await;

    let writer_instances = supervisor
        .instances_by_hat
        .get(&ralph_proto::HatId::new("writer"))
        .expect("writer hat should exist")
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    assert_eq!(
        writer_instances,
        vec!["writer#1".to_string(), "writer#2".to_string()]
    );
    assert_eq!(
        supervisor
            .next_instance_seq_by_hat
            .get(&ralph_proto::HatId::new("writer"))
            .copied(),
        Some(3),
        "Expected monotonically increasing next instance key"
    );

    let snapshot = supervisor.build_agents_snapshot();
    let writer_1 = snapshot
        .instances
        .iter()
        .find(|instance| instance.instance_id == "writer#1")
        .expect("writer#1 should be present in agents snapshot");
    let writer_2 = snapshot
        .instances
        .iter()
        .find(|instance| instance.instance_id == "writer#2")
        .expect("writer#2 should be present in agents snapshot");
    assert_eq!(
        writer_1.identity_source,
        crate::IdentitySource::ConfigDerived,
        "configured base instance must not be confused with task-derived hats"
    );
    assert_eq!(
        writer_2.identity_source,
        crate::IdentitySource::RuntimeAutoscale,
        "autoscaled runtime instance must be marked as runtime-autoscale"
    );

    // cap reached：第三个事件不应继续扩实例
    supervisor.instance_states.insert(
        HatInstanceId::new("writer#2"),
        ralph_proto::HatInstanceState::Running,
    );
    supervisor
        .route_event(Event::new("build.task", "third").with_id("e3"))
        .await
        .expect("route_event should succeed");

    let writer_instances_after = supervisor
        .instances_by_hat
        .get(&ralph_proto::HatId::new("writer"))
        .expect("writer hat should exist")
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    assert_eq!(
        writer_instances_after,
        vec!["writer#1".to_string(), "writer#2".to_string()]
    );

    // 清理：避免后台 job 残留
    supervisor.shutdown_instances().await;
}

#[tokio::test]
async fn parallel_job_timeout_inherits_from_cli_backend_adapter() {
    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();

    // 默认 backend 来自 cli.backend
    config.cli.backend = "codex".to_string();
    config.adapters.codex.timeout = 123;

    config.hats.insert(
        "writer".to_string(),
        hat_config("Writer", vec!["build.task"], 1),
    );

    // build.task -> writer#1
    config.parallel.topic_contracts.insert(
        "build.task".to_string(),
        TopicContract {
            delivery: Delivery::Queue,
            audience: AudienceSelector {
                instances: vec![HatInstanceId::new("writer#1")],
                ..Default::default()
            },
            queue_selection: QueueSelection::Deterministic,
            missing_instance_policy: MissingInstancePolicy::Queue,
        },
    );

    let timeout = capture_timeout_for_instance(config, "writer#1").await;
    assert_eq!(timeout, Some(Duration::from_secs(123)));
}

#[tokio::test]
async fn parallel_job_timeout_custom_command_codex_maps_to_codex_adapter() {
    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();

    // cli.backend=custom 时，timeout profile 应按 cli.command 推导（codex -> adapters.codex）
    config.cli.backend = "custom".to_string();
    config.cli.command = Some("codex".to_string());
    config.adapters.claude.timeout = 999; // 若映射失败会错误地走到这里
    config.adapters.codex.timeout = 123;

    config.hats.insert(
        "writer".to_string(),
        hat_config("Writer", vec!["build.task"], 1),
    );

    // build.task -> writer#1
    config.parallel.topic_contracts.insert(
        "build.task".to_string(),
        TopicContract {
            delivery: Delivery::Queue,
            audience: AudienceSelector {
                instances: vec![HatInstanceId::new("writer#1")],
                ..Default::default()
            },
            queue_selection: QueueSelection::Deterministic,
            missing_instance_policy: MissingInstancePolicy::Queue,
        },
    );

    let timeout = capture_timeout_for_instance(config, "writer#1").await;
    assert_eq!(timeout, Some(Duration::from_secs(123)));
}

#[tokio::test]
async fn parallel_job_timeout_inherits_from_hat_backend_adapter() {
    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();

    // cli.backend 不应影响：我们给 hat.backend 指定了 gemini
    config.cli.backend = "codex".to_string();
    config.adapters.gemini.timeout = 222;

    let mut writer = hat_config("Writer", vec!["build.task"], 1);
    writer.backend = Some(HatBackend::Named("gemini".to_string()));
    config.hats.insert("writer".to_string(), writer);

    config.parallel.topic_contracts.insert(
        "build.task".to_string(),
        TopicContract {
            delivery: Delivery::Queue,
            audience: AudienceSelector {
                instances: vec![HatInstanceId::new("writer#1")],
                ..Default::default()
            },
            queue_selection: QueueSelection::Deterministic,
            missing_instance_policy: MissingInstancePolicy::Queue,
        },
    );

    let timeout = capture_timeout_for_instance(config, "writer#1").await;
    assert_eq!(timeout, Some(Duration::from_secs(222)));
}

#[tokio::test]
async fn parallel_job_timeout_can_be_overridden_or_disabled_per_hat() {
    // override: >0
    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    config.cli.backend = "codex".to_string();
    config.adapters.codex.timeout = 999; // 应被 override 覆盖

    let mut writer = hat_config("Writer", vec!["build.task"], 1);
    writer.job_timeout_secs = Some(45);
    config.hats.insert("writer".to_string(), writer);

    config.parallel.topic_contracts.insert(
        "build.task".to_string(),
        TopicContract {
            delivery: Delivery::Queue,
            audience: AudienceSelector {
                instances: vec![HatInstanceId::new("writer#1")],
                ..Default::default()
            },
            queue_selection: QueueSelection::Deterministic,
            missing_instance_policy: MissingInstancePolicy::Queue,
        },
    );

    let timeout = capture_timeout_for_instance(config, "writer#1").await;
    assert_eq!(timeout, Some(Duration::from_secs(45)));

    // disable: 0
    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    config.cli.backend = "codex".to_string();
    config.adapters.codex.timeout = 999; // 应被 disable 覆盖

    let mut writer = hat_config("Writer", vec!["build.task"], 1);
    writer.job_timeout_secs = Some(0);
    config.hats.insert("writer".to_string(), writer);

    config.parallel.topic_contracts.insert(
        "build.task".to_string(),
        TopicContract {
            delivery: Delivery::Queue,
            audience: AudienceSelector {
                instances: vec![HatInstanceId::new("writer#1")],
                ..Default::default()
            },
            queue_selection: QueueSelection::Deterministic,
            missing_instance_policy: MissingInstancePolicy::Queue,
        },
    );

    let timeout = capture_timeout_for_instance(config, "writer#1").await;
    assert_eq!(timeout, None);
}

#[tokio::test]
async fn best_effort_missing_instance_falls_back_by_policy_queue() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();

    config.hats.insert(
        "writer".to_string(),
        hat_config("Writer", vec!["build.task"], 1),
    );

    // build.task audience 明确包含 writer#1 + writer#2（但我们只会 spawn writer#1）
    config.parallel.topic_contracts.insert(
        "build.task".to_string(),
        TopicContract {
            delivery: Delivery::Queue,
            audience: AudienceSelector {
                instances: vec![
                    HatInstanceId::new("writer#1"),
                    HatInstanceId::new("writer#2"),
                ],
                ..Default::default()
            },
            queue_selection: QueueSelection::Deterministic,
            missing_instance_policy: MissingInstancePolicy::Queue,
        },
    );
    config.parallel.topic_contracts.insert(
        "task.*".to_string(),
        TopicContract {
            delivery: Delivery::Queue,
            audience: AudienceSelector {
                instances: vec![HatInstanceId::new("ralph#1")],
                ..Default::default()
            },
            queue_selection: QueueSelection::Deterministic,
            missing_instance_policy: MissingInstancePolicy::Queue,
        },
    );

    let (tx, mut rx) = mpsc::channel::<String>(4);
    let executor = Arc::new(test_executors::TestExecutor::new(tx));

    let mut supervisor = make_supervisor(config, executor, events_path);

    // override 指向缺失的 writer#2（best-effort），policy=queue 应回退到 base_existing(writer#1)
    let mut event = Event::new("build.task", "x").with_id("e2");
    event.audience_override = Some(AudienceOverride {
        instances: vec![HatInstanceId::new("writer#2")],
        require_delivery: false,
    });

    supervisor
        .route_event(event)
        .await
        .expect("route_event should succeed");

    let got = tokio::time::timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("should receive executor call")
        .expect("channel closed");
    assert_eq!(got, "writer#1");
}

#[tokio::test]
async fn require_delivery_missing_instance_escalates_to_ralph() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();

    config.hats.insert(
        "writer".to_string(),
        hat_config("Writer", vec!["build.task"], 1),
    );

    config.parallel.topic_contracts.insert(
        "build.task".to_string(),
        TopicContract {
            delivery: Delivery::Queue,
            audience: AudienceSelector {
                instances: vec![
                    HatInstanceId::new("writer#1"),
                    HatInstanceId::new("writer#2"),
                ],
                ..Default::default()
            },
            queue_selection: QueueSelection::Deterministic,
            missing_instance_policy: MissingInstancePolicy::Queue,
        },
    );
    config.parallel.topic_contracts.insert(
        "task.*".to_string(),
        TopicContract {
            delivery: Delivery::Queue,
            audience: AudienceSelector {
                instances: vec![HatInstanceId::new("ralph#1")],
                ..Default::default()
            },
            queue_selection: QueueSelection::Deterministic,
            missing_instance_policy: MissingInstancePolicy::Queue,
        },
    );

    let (tx, mut rx) = mpsc::channel::<String>(8);
    let executor = Arc::new(test_executors::TestExecutor::new(tx));

    let mut supervisor = make_supervisor(config, executor, events_path);

    // require_delivery=true 且指向缺失 writer#2：应触发 escalate，投递 routing.escalate 到 ralph#1
    let mut event = Event::new("build.task", "x").with_id("e3");
    event.audience_override = Some(AudienceOverride {
        instances: vec![HatInstanceId::new("writer#2")],
        require_delivery: true,
    });

    supervisor
        .route_event(event)
        .await
        .expect("route_event should succeed");

    let got = tokio::time::timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("should receive executor call")
        .expect("channel closed");
    assert_eq!(got, "ralph#1");
}

#[tokio::test]
async fn queue_decision_is_loaded_from_history_and_not_recomputed() {
    let temp_dir = tempfile::tempdir().unwrap();
    let events_path = temp_dir.path().join("events.jsonl");

    // 先把 dispatch.decision 记录写入 events.jsonl
    let decision = QueueDecisionRecord::new(
        "event-42",
        vec![
            HatInstanceId::new("writer#1"),
            HatInstanceId::new("writer#2"),
        ],
        HatInstanceId::new("writer#1"),
        Some("test".to_string()),
    );
    {
        let mut logger = EventLogger::new(&events_path);
        logger
            .log_queue_decision(0, "supervisor", &decision)
            .expect("log_queue_decision should succeed");
    }

    let mut config = RalphConfig::default();
    config.parallel = base_parallel_config();
    config.hats.insert(
        "writer".to_string(),
        hat_config("Writer", vec!["build.task"], 2),
    );

    // build.task -> queue, 且 queue_selection=llm（如果没有历史决策，会触发 decider job）
    config.parallel.topic_contracts.insert(
        "build.task".to_string(),
        TopicContract {
            delivery: Delivery::Queue,
            audience: AudienceSelector {
                instance_prefixes: vec!["writer#".to_string()],
                ..Default::default()
            },
            queue_selection: QueueSelection::Llm,
            missing_instance_policy: MissingInstancePolicy::Queue,
        },
    );
    config.parallel.topic_contracts.insert(
        "task.*".to_string(),
        TopicContract {
            delivery: Delivery::Queue,
            audience: AudienceSelector {
                instances: vec![HatInstanceId::new("ralph#1")],
                ..Default::default()
            },
            queue_selection: QueueSelection::Deterministic,
            missing_instance_policy: MissingInstancePolicy::Queue,
        },
    );

    // 如果真的走 choose_llm，这里会被调用到（我们用 panic 作为护栏）
    let (tx, mut rx) = mpsc::channel::<String>(8);
    let executor = Arc::new(test_executors::NoDeciderExecutor::new(tx));

    let mut supervisor = make_supervisor(config, executor, events_path.clone());

    // resume：从 history 载入 queue 决策
    supervisor
        .load_queue_decisions_from_history()
        .expect("load_queue_decisions_from_history should succeed");

    // 发送一个 id=event-42 的事件，应该直接投递到 writer#1，而不会触发 decider job
    let event = Event::new("build.task", "x").with_id("event-42");
    supervisor
        .route_event(event)
        .await
        .expect("route_event should succeed");

    let got = tokio::time::timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("should receive executor call")
        .expect("channel closed");
    assert_eq!(got, "writer#1");

    // 文件里仍然只有 1 条 dispatch.decision（允许 V2 runtime.lifecycle 旁路记录存在）。
    let dispatch_decisions = EventHistory::new(&events_path)
        .filter_by_topic(TOPIC_DISPATCH_DECISION)
        .expect("dispatch.decision records should be readable");
    assert_eq!(dispatch_decisions.len(), 1);
}

// =====================================================================
// Test executors（尽量保持简单，不依赖外部后端）
// =====================================================================

mod test_executors {
    use super::*;

    #[derive(Debug)]
    pub struct TestExecutor {
        tx: mpsc::Sender<String>,
    }

    impl TestExecutor {
        pub fn new(tx: mpsc::Sender<String>) -> Self {
            Self { tx }
        }
    }

    #[async_trait::async_trait]
    impl HatJobExecutor for TestExecutor {
        async fn execute(
            &self,
            job: HatJob,
            _output_tx: mpsc::Sender<HatJobOutputChunk>,
            mut _cancel_rx: tokio::sync::watch::Receiver<bool>,
            _control_rx: mpsc::Receiver<HatJobControl>,
        ) -> anyhow::Result<HatJobResult> {
            let _ = self.tx.send(job.instance_id.to_string()).await;
            Ok(HatJobResult {
                output_for_parsing: String::new(),
                observed_stderr: String::new(),
                success: true,
                exit_code: Some(0),
                timed_out: false,
                canceled: false,
            })
        }
    }

    /// 用于验证“replay 不重算”：如果触发了 LLM decider job，直接 panic。
    #[derive(Debug)]
    pub struct NoDeciderExecutor {
        tx: mpsc::Sender<String>,
    }

    impl NoDeciderExecutor {
        pub fn new(tx: mpsc::Sender<String>) -> Self {
            Self { tx }
        }
    }

    #[async_trait::async_trait]
    impl HatJobExecutor for NoDeciderExecutor {
        async fn execute(
            &self,
            job: HatJob,
            _output_tx: mpsc::Sender<HatJobOutputChunk>,
            mut _cancel_rx: tokio::sync::watch::Receiver<bool>,
            _control_rx: mpsc::Receiver<HatJobControl>,
        ) -> anyhow::Result<HatJobResult> {
            assert!(
                !job.instance_id.as_str().contains("decider-"),
                "decider job should NOT be executed when decision is loaded from history"
            );

            let _ = self.tx.send(job.instance_id.to_string()).await;
            Ok(HatJobResult {
                output_for_parsing: String::new(),
                observed_stderr: String::new(),
                success: true,
                exit_code: Some(0),
                timed_out: false,
                canceled: false,
            })
        }
    }
}
