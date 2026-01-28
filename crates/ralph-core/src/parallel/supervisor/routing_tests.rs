//! 并行路由的确定性/回放护栏测试（OpenSpec tasks 7.x）。
//!
//! 说明：
//! - 这些测试不依赖真实 LLM 后端。
//! - 通过 Fake HatJobExecutor 验证：并行、missing 行为、require_delivery escalate、queue 决策 replay。

use super::ParallelSupervisor;
use crate::config::{HatBackend, HatConfig, HatWorkspaceConfig, ParallelConfig, RalphConfig};
use crate::event_logger::EventLogger;
use crate::parallel::{HatJob, HatJobExecutor, HatJobOutputChunk, HatJobResult};
use anyhow::Context;
use ralph_proto::{
    AudienceOverride, AudienceSelector, Delivery, Event, HatInstanceId, MissingInstancePolicy,
    QueueDecisionRecord, QueueSelection, TopicContract,
};
use std::path::PathBuf;
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

#[async_trait::async_trait]
impl HatJobExecutor for NotifyExecutor {
    async fn execute(
        &self,
        job: HatJob,
        _output_tx: mpsc::Sender<HatJobOutputChunk>,
        mut _cancel_rx: tokio::sync::watch::Receiver<bool>,
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
            output,
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

#[async_trait::async_trait]
impl HatJobExecutor for BlockingExecutor {
    async fn execute(
        &self,
        _job: HatJob,
        _output_tx: mpsc::Sender<HatJobOutputChunk>,
        mut cancel_rx: tokio::sync::watch::Receiver<bool>,
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
            output: String::new(),
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
    ) -> anyhow::Result<HatJobResult> {
        {
            let mut seen = self.seen.lock().await;
            seen.push((job.instance_id.to_string(), job.timeout));
        }

        Ok(HatJobResult {
            output: String::new(),
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
    ) -> anyhow::Result<HatJobResult> {
        {
            let mut seen = self.seen.lock().await;
            seen.push(job.instance_id.to_string());
        }
        self.notify.notify_waiters();

        Ok(HatJobResult {
            output: String::new(),
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

    let mut supervisor = make_supervisor(config, Arc::new(executor.clone()), events_path);

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

    let mut supervisor = make_supervisor(config, Arc::new(executor.clone()), events_path);

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
    config.hats.insert(
        "manager".to_string(),
        hat_config("Manager", vec!["*"], 1),
    );

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
    config.hats.insert(
        "manager".to_string(),
        hat_config("Manager", vec!["*"], 1),
    );

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

    // 文件里仍然只有 1 条 dispatch.decision（没有新增记录）
    let content = std::fs::read_to_string(&events_path).unwrap();
    let lines = content.lines().filter(|l| !l.trim().is_empty()).count();
    assert_eq!(lines, 1);
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
        ) -> anyhow::Result<HatJobResult> {
            let _ = self.tx.send(job.instance_id.to_string()).await;
            Ok(HatJobResult {
                output: String::new(),
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
        ) -> anyhow::Result<HatJobResult> {
            assert!(
                !job.instance_id.as_str().contains("decider-"),
                "decider job should NOT be executed when decision is loaded from history"
            );

            let _ = self.tx.send(job.instance_id.to_string()).await;
            Ok(HatJobResult {
                output: String::new(),
                success: true,
                exit_code: Some(0),
                timed_out: false,
                canceled: false,
            })
        }
    }
}
