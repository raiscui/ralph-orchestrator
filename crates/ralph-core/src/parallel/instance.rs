//! HatInstance actor（最小可用骨架）。
//!
//! 说明：
//! - inbox：Supervisor 投递的事件
//! - outbox：实例向 Supervisor 回传的状态变更 / 解析出的事件
//! - 每个实例串行执行自己的 job，但多个实例之间可并行

use super::{HatJob, HatJobExecutor, HatJobOutputChunk, HatJobResult, JobBackend};
use crate::config::{
    HatConfig, PermissionMode, PermissionsConfig, WorkspaceRuntimeConfig, WorkspaceStrategy,
    WorktreeBackend,
};
use crate::event_parser::EventParser;
use crate::instructions::InstructionBuilder;
use anyhow::Context;
use ralph_proto::{
    Event, GateKind, GateRequest, GateResolve, Hat, HatId, HatInstanceId, HatInstanceState,
    TOPIC_GATE_REQUEST, TOPIC_GATE_RESOLVE,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;
use tokio::sync::{Semaphore, mpsc, watch};
use tokio::time::Instant;

/// Supervisor -> HatInstance 的命令。
#[derive(Debug)]
pub enum HatInstanceCommand {
    /// 投递一个事件到实例 inbox。
    Deliver(Box<Event>),
    /// 取消当前正在运行的 job（best-effort）。
    CancelCurrentJob,
    /// 关闭实例（best-effort）。
    Shutdown,
}

/// HatInstance -> Supervisor 的事件。
#[derive(Debug)]
pub enum HatInstanceEvent {
    /// 状态变更。
    StateChanged {
        instance_id: HatInstanceId,
        state: HatInstanceState,
    },
    /// 本次 job 完成，并解析出了新的事件。
    JobCompleted {
        instance_id: HatInstanceId,
        hat_id: HatId,
        result: HatJobResult,
        events: Vec<Event>,
    },
    /// 实例在 orchestrator 内部发布的事件（不依赖外部 job 输出）。
    ///
    /// 说明：
    /// - 用于 workspace/gate 这类“必须由 orchestrator 执行”的动作与记录。
    /// - Supervisor 收到后会负责：落盘 + 继续路由。
    Published {
        instance_id: HatInstanceId,
        hat_id: HatId,
        event: Event,
    },
}

/// HatInstance 的对外句柄。
#[derive(Debug)]
pub struct HatInstanceHandle {
    pub cmd_tx: mpsc::Sender<HatInstanceCommand>,
}

impl HatInstanceHandle {
    /// 创建并启动一个 HatInstance actor。
    #[allow(clippy::too_many_arguments)]
    pub fn spawn(
        instance_id: HatInstanceId,
        hat: Hat,
        hat_config: Option<HatConfig>,
        workspace_runtime: WorkspaceRuntimeConfig,
        permissions: PermissionsConfig,
        gate_default_timeout_secs: u64,
        job_timeout: Option<Duration>,
        job_output_stale_timeout: Option<Duration>,
        prompt_prelude: String,
        instruction_builder: Arc<InstructionBuilder>,
        executor: Arc<dyn HatJobExecutor>,
        output_tx: mpsc::Sender<HatJobOutputChunk>,
        supervisor_tx: mpsc::Sender<HatInstanceEvent>,
        job_semaphore: Arc<Semaphore>,
        is_dynamic: bool,
        dynamic_idle_ttl: Duration,
    ) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel(128);

        let actor_instance_id = instance_id.clone();
        tokio::spawn(async move {
            let mut actor = HatInstanceActor {
                instance_id: actor_instance_id,
                hat,
                hat_config,
                workspace_runtime,
                permissions,
                gate_default_timeout_secs,
                job_timeout,
                job_output_stale_timeout,
                prompt_prelude,
                instruction_builder,
                executor,
                output_tx,
                supervisor_tx,
                job_semaphore,
                is_dynamic,
                dynamic_idle_ttl,
                cmd_rx,
                state: HatInstanceState::Created,
                pending: Vec::new(),
                next_job_id: 1,
                next_event_seq: 1,
                dynamic_idle_since: None,
                pending_permission_gate: None,
                worktree_override: None,
                hooks_override: None,
                running_workspace: None,
                running: None,
                cancel_tx: None,
            };

            if let Err(e) = actor.run().await {
                // 说明：actor 自己崩了也要尽量把状态打回给 Supervisor，避免黑盒。
                let _ = actor
                    .supervisor_tx
                    .send(HatInstanceEvent::StateChanged {
                        instance_id: actor.instance_id.clone(),
                        state: HatInstanceState::Failed,
                    })
                    .await;
                tracing::warn!(instance = %actor.instance_id, error = %e, "HatInstance actor exited with error");
            }
        });

        Self { cmd_tx }
    }
}

struct HatInstanceActor {
    instance_id: HatInstanceId,
    hat: Hat,
    hat_config: Option<HatConfig>,
    workspace_runtime: WorkspaceRuntimeConfig,
    permissions: PermissionsConfig,
    gate_default_timeout_secs: u64,
    /// 单次 job 的超时（由 Supervisor 预先计算并注入）。
    ///
    /// 说明：
    /// - 并行模式下“某个 hat 卡住”是高概率事故点，因此需要 job-level timeout 作为第一道止损。
    /// - 真正的超时/kill 行为由 HatJobExecutor 执行（这里仅把值写进 HatJob）。
    job_timeout: Option<Duration>,
    /// 输出停滞阈值（由 Supervisor 预先计算并注入）。
    ///
    /// 说明：
    /// - `job_timeout` 到期后，并不会立刻终止进程；
    /// - 只有当 stdout/stderr 输出在该阈值内没有任何变化，才会判定为超时并终止。
    job_output_stale_timeout: Option<Duration>,
    prompt_prelude: String,
    instruction_builder: Arc<InstructionBuilder>,
    executor: Arc<dyn HatJobExecutor>,
    output_tx: mpsc::Sender<HatJobOutputChunk>,
    supervisor_tx: mpsc::Sender<HatInstanceEvent>,
    /// 全局并发上限的 semaphore（permit 持有期间代表一个 Running job）。
    job_semaphore: Arc<Semaphore>,
    /// 是否为动态实例（由 autoscale 创建，空闲可回收）。
    is_dynamic: bool,
    /// 动态实例的 idle 回收阈值。
    dynamic_idle_ttl: Duration,
    cmd_rx: mpsc::Receiver<HatInstanceCommand>,
    state: HatInstanceState,
    pending: Vec<Event>,
    next_job_id: u64,
    next_event_seq: u64,
    /// 动态实例进入“真正空闲”状态的起始时间（使用 tokio 时间，便于测试 time control）。
    dynamic_idle_since: Option<Instant>,

    // =====================================================================
    // Permissions / gate（用于 worktree & hooks）
    // =====================================================================
    pending_permission_gate: Option<PendingPermissionGate>,
    worktree_override: Option<PermissionOverride>,
    hooks_override: Option<PermissionOverride>,

    // =====================================================================
    // Workspace（用于 worktree acquire/release）
    // =====================================================================
    running_workspace: Option<RunningWorkspace>,

    running: Option<tokio::task::JoinHandle<anyhow::Result<HatJobResult>>>,
    cancel_tx: Option<watch::Sender<bool>>,
}

// ============================================================================
// Permissions / gate / workspace（并行模式）
// ============================================================================

/// 需要 human gate（ask）确认的动作类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PermissionAction {
    Worktree,
    Hooks,
}

const CAP_WORKTREE: &str = "workspace.worktree";
const CAP_HOOKS: &str = "workspace.hooks";

/// 单次 job 的权限决策缓存（用于 ask gate 回来后继续推进）。
#[derive(Debug, Clone, Copy)]
struct PermissionOverride {
    job_id: u64,
    approved: bool,
}

/// 正在等待的权限 gate（ask 模式）。
#[derive(Debug, Clone)]
struct PendingPermissionGate {
    gate_id: String,
    action: PermissionAction,
    job_id: u64,
}

/// 当前运行 job 关联的 workspace 信息（用于 release/cleanup）。
#[derive(Debug, Clone)]
struct RunningWorkspace {
    job_id: u64,
    strategy: WorkspaceStrategy,
    workdir: Option<PathBuf>,
    hooks_allowed: bool,
    on_release_hook: Option<String>,
}

impl HatInstanceActor {
    async fn run(&mut self) -> anyhow::Result<()> {
        self.set_state(HatInstanceState::Idle).await?;

        // tick 用于：
        // - permit 释放后重试启动 pending job（避免“只靠 Deliver 触发”导致卡住）
        // - 动态实例的 idle 回收（只在 truly-idle 时计时）
        let mut tick = tokio::time::interval(Duration::from_millis(200));

        loop {
            tokio::select! {
                _ = tick.tick() => {
                    if self.should_reap_dynamic_instance() {
                        break;
                    }
                    self.maybe_start_job().await?;
                }
                cmd = self.cmd_rx.recv() => {
                    let Some(cmd) = cmd else { break };
                    match cmd {
                        HatInstanceCommand::Deliver(event) => {
                            // 权限 gate 的 resolve 是“运行时控制信号”，不应该进入 LLM 的业务事件列表。
                            if self.try_handle_permission_gate_resolve(event.as_ref()).await? {
                                // gate 已处理完毕（可能解锁了 job），继续 loop
                                continue;
                            }

                            self.pending.push(*event);
                            self.maybe_start_job().await?;
                        }
                        HatInstanceCommand::CancelCurrentJob => {
                            if let Some(cancel_tx) = &self.cancel_tx {
                                let _ = cancel_tx.send(true);
                            }
                        }
                        HatInstanceCommand::Shutdown => {
                            if let Some(cancel_tx) = &self.cancel_tx {
                                let _ = cancel_tx.send(true);
                            }
                            break;
                        }
                    }
                }
                res = async {
                    match &mut self.running {
                        Some(handle) => Some(handle.await),
                        None => None,
                    }
                }, if self.running.is_some() => {
                    self.running = None;
                    self.cancel_tx = None;

                    let Some(res) = res else { continue };
                    let job_result = res.context("JoinHandle await failed")??;
                    self.on_job_completed(job_result).await?;
                }
            }
        }

        self.set_state(HatInstanceState::Done).await?;
        Ok(())
    }

    fn should_reap_dynamic_instance(&mut self) -> bool {
        if !self.is_dynamic {
            return false;
        }

        // “真正空闲”的判定：不在跑、不在等 gate、也没有待处理事件。
        let truly_idle = self.state == HatInstanceState::Idle
            && self.running.is_none()
            && self.pending_permission_gate.is_none()
            && self.pending.is_empty();

        if !truly_idle {
            self.dynamic_idle_since = None;
            return false;
        }

        let since = self.dynamic_idle_since.get_or_insert_with(Instant::now);
        since.elapsed() >= self.dynamic_idle_ttl
    }

    /// 尝试处理“权限 gate”的 resolve。
    ///
    /// 返回：
    /// - Ok(true)：该事件属于权限 gate 控制信号，已被消费，不应进入 LLM 事件列表
    /// - Ok(false)：不是权限 gate 的 resolve，交给正常事件路径处理
    async fn try_handle_permission_gate_resolve(&mut self, event: &Event) -> anyhow::Result<bool> {
        if event.topic.as_str() != TOPIC_GATE_RESOLVE {
            return Ok(false);
        }

        let Some(pending) = self.pending_permission_gate.clone() else {
            return Ok(false);
        };

        let resolve: GateResolve = serde_json::from_str(&event.payload).with_context(|| {
            format!(
                "Failed to parse gate.resolve payload as JSON: instance={} payload_len={}",
                self.instance_id,
                event.payload.len()
            )
        })?;

        if resolve.gate_id != pending.gate_id {
            return Ok(false);
        }

        let approved = match &resolve.decision {
            serde_json::Value::Bool(b) => *b,
            serde_json::Value::Object(map) => map
                .get("approved")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            serde_json::Value::String(s) => {
                matches!(s.as_str(), "allow" | "approved" | "yes" | "true")
            }
            _ => false,
        };

        let override_ = PermissionOverride {
            job_id: pending.job_id,
            approved,
        };

        match pending.action {
            PermissionAction::Worktree => {
                self.worktree_override = Some(override_);
            }
            PermissionAction::Hooks => {
                self.hooks_override = Some(override_);
            }
        }

        self.pending_permission_gate = None;

        // gate 已回来了，可能解锁 job（继续尝试启动）。
        self.maybe_start_job().await?;

        Ok(true)
    }

    async fn set_state(&mut self, state: HatInstanceState) -> anyhow::Result<()> {
        if self.state == state {
            return Ok(());
        }

        self.state = state;
        self.supervisor_tx
            .send(HatInstanceEvent::StateChanged {
                instance_id: self.instance_id.clone(),
                state,
            })
            .await
            .context("Failed to send StateChanged to supervisor")?;
        Ok(())
    }

    async fn maybe_start_job(&mut self) -> anyhow::Result<()> {
        if self.running.is_some() {
            return Ok(());
        }
        if self.pending.is_empty() {
            return Ok(());
        }
        if self.pending_permission_gate.is_some() {
            return Ok(());
        }

        // job_id 在真正启动前不要自增。
        // 说明：ask gate 可能会阻塞一段时间，我们要保证 gate 的 job_id 与最终启动的 job 对齐。
        let job_id = self.next_job_id;

        // 4.2：workspace_strategy 合并规则（最强隔离优先：worktree > patch > shared）
        let strategy = self.merged_workspace_strategy_for_pending();
        let (hooks_on_acquire, hooks_on_release) = self.requested_workspace_hooks();

        // 1) worktree 权限判定（必要时发 gate.request 并阻塞）
        let mut final_strategy = strategy;
        let mut hooks_allowed = false;
        let mut on_release_hook: Option<String> = None;

        if strategy == WorkspaceStrategy::Worktree {
            // capability：没有能力就不允许切到 worktree（降级为 shared）
            if self.has_capability(CAP_WORKTREE) {
                match self
                    .permission_or_request_gate(
                        PermissionAction::Worktree,
                        job_id,
                        self.permissions.worktree,
                        self.worktree_gate_prompt(),
                    )
                    .await?
                {
                    Some(true) => {
                        // 允许 worktree（5.3 会真正 acquire）
                    }
                    Some(false) => {
                        tracing::warn!(
                            instance = %self.instance_id,
                            hat = %self.hat.id,
                            "worktree permission denied; downgrade to shared"
                        );
                        final_strategy = WorkspaceStrategy::Shared;
                    }
                    None => {
                        // gate 已发出，等待 resolve
                        return Ok(());
                    }
                }
            } else {
                tracing::warn!(
                    instance = %self.instance_id,
                    hat = %self.hat.id,
                    "workspace.strategy=worktree but capability \"{CAP_WORKTREE}\" is missing; downgrade to shared"
                );
                final_strategy = WorkspaceStrategy::Shared;
            }

            // 2) hooks 权限判定（仅在最终策略仍是 worktree 时才考虑）
            if final_strategy == WorkspaceStrategy::Worktree {
                let hooks_configured = hooks_on_acquire.is_some() || hooks_on_release.is_some();
                if hooks_configured {
                    if self.has_capability(CAP_HOOKS) {
                        match self
                            .permission_or_request_gate(
                                PermissionAction::Hooks,
                                job_id,
                                self.permissions.hooks,
                                self.hooks_gate_prompt(
                                    hooks_on_acquire.as_ref(),
                                    hooks_on_release.as_ref(),
                                ),
                            )
                            .await?
                        {
                            Some(true) => {
                                hooks_allowed = true;
                                on_release_hook = hooks_on_release.clone();
                            }
                            Some(false) => {
                                tracing::warn!(
                                    instance = %self.instance_id,
                                    hat = %self.hat.id,
                                    "workspace hooks permission denied; hooks will be skipped"
                                );
                            }
                            None => {
                                return Ok(());
                            }
                        }
                    } else {
                        tracing::warn!(
                            instance = %self.instance_id,
                            hat = %self.hat.id,
                            "workspace.hooks configured but capability \"{CAP_HOOKS}\" is missing; hooks will be skipped"
                        );
                    }
                }
            }
        }

        // 3.4：全局并发上限（permit/semaphore）
        //
        // 说明：
        // - 没拿到 permit 代表“全局 Running job 数量已达上限”，此时不应启动新 job。
        // - 这里不阻塞等待（避免实例 actor 卡住）；交给 tick 重试。
        let Ok(permit) = self.job_semaphore.clone().try_acquire_owned() else {
            return Ok(());
        };

        self.running_workspace = Some(RunningWorkspace {
            job_id,
            strategy: final_strategy,
            workdir: None,
            hooks_allowed,
            on_release_hook,
        });

        // 5.3：worktree acquire（以及可选 hooks）。
        if final_strategy == WorkspaceStrategy::Worktree {
            let workdir = self
                .acquire_worktree(job_id, hooks_allowed, hooks_on_acquire.as_ref())
                .await?;
            if let Some(ws) = &mut self.running_workspace {
                ws.workdir = Some(workdir);
            }
        }

        let events = std::mem::take(&mut self.pending);
        let prompt = self.build_prompt(&events);

        let backend = self
            .hat_config
            .as_ref()
            .and_then(|c| c.backend.clone())
            .map_or(JobBackend::Default, JobBackend::Hat);

        // timeout/stale_timeout 由 Supervisor 在 spawn 时根据 config.adapters + per-hat override 计算并注入。
        let timeout = self.job_timeout;
        let output_stale_timeout = self.job_output_stale_timeout;

        let job = HatJob {
            job_id,
            instance_id: self.instance_id.clone(),
            hat_id: self.hat.id.clone(),
            prompt,
            backend,
            timeout,
            output_stale_timeout,
            workdir: self
                .running_workspace
                .as_ref()
                .and_then(|w| w.workdir.clone()),
        };
        self.next_job_id += 1;

        let (cancel_tx, cancel_rx) = watch::channel(false);
        self.cancel_tx = Some(cancel_tx);

        let executor = Arc::clone(&self.executor);
        let output_tx = self.output_tx.clone();

        self.set_state(HatInstanceState::Running).await?;
        self.running = Some(tokio::spawn(async move {
            let _permit = permit;
            executor.execute(job, output_tx, cancel_rx).await
        }));
        Ok(())
    }

    fn requested_workspace_strategy(&self) -> WorkspaceStrategy {
        self.hat_config
            .as_ref()
            .map(|c| c.workspace.strategy)
            .unwrap_or_default()
    }

    fn merged_workspace_strategy_for_pending(&self) -> WorkspaceStrategy {
        let mut merged = self.requested_workspace_strategy();

        for event in &self.pending {
            if let Some(override_) = event.workspace_strategy {
                merged = merged.max(override_);
            }
        }

        merged
    }

    fn requested_workspace_hooks(&self) -> (Option<String>, Option<String>) {
        let Some(cfg) = self.hat_config.as_ref() else {
            return (None, None);
        };

        let on_acquire = cfg
            .workspace
            .hooks
            .on_acquire
            .clone()
            .filter(|s| !s.trim().is_empty());
        let on_release = cfg
            .workspace
            .hooks
            .on_release
            .clone()
            .filter(|s| !s.trim().is_empty());
        (on_acquire, on_release)
    }

    fn has_capability(&self, capability: &str) -> bool {
        self.hat_config
            .as_ref()
            .map(|c| c.capabilities.iter().any(|c| c == capability))
            .unwrap_or(false)
    }

    fn permission_override(&self, action: PermissionAction, job_id: u64) -> Option<bool> {
        let ov = match action {
            PermissionAction::Worktree => self.worktree_override,
            PermissionAction::Hooks => self.hooks_override,
        }?;

        if ov.job_id != job_id {
            return None;
        }
        Some(ov.approved)
    }

    fn worktree_gate_prompt(&self) -> String {
        format!(
            "实例 {} 请求创建/切换到 worktree（策略=worktree）。\n\n是否允许？请用 gate.resolve 决策：true/false 或 {{\"approved\":true/false}}。",
            self.instance_id
        )
    }

    fn hooks_gate_prompt(
        &self,
        on_acquire: Option<&String>,
        on_release: Option<&String>,
    ) -> String {
        let mut prompt = String::new();
        prompt.push_str(&format!(
            "实例 {} 请求执行 workspace hooks（on_acquire/on_release）。\n\n是否允许？请用 gate.resolve 决策：true/false 或 {{\"approved\":true/false}}。\n",
            self.instance_id
        ));

        if let Some(cmd) = on_acquire {
            prompt.push_str(&format!("\n- on_acquire: {cmd}"));
        }
        if let Some(cmd) = on_release {
            prompt.push_str(&format!("\n- on_release: {cmd}"));
        }

        prompt
    }

    async fn permission_or_request_gate(
        &mut self,
        action: PermissionAction,
        job_id: u64,
        mode: PermissionMode,
        prompt: String,
    ) -> anyhow::Result<Option<bool>> {
        match mode {
            PermissionMode::Allow => Ok(Some(true)),
            PermissionMode::Deny => Ok(Some(false)),
            PermissionMode::Ask => {
                if let Some(approved) = self.permission_override(action, job_id) {
                    return Ok(Some(approved));
                }

                // 同一时刻只允许等待一个 gate，避免并行问多个问题导致 UX 混乱。
                if self.pending_permission_gate.is_some() {
                    return Ok(None);
                }

                let gate_id = format!(
                    "{}:{}:{}",
                    self.instance_id,
                    job_id,
                    match action {
                        PermissionAction::Worktree => "worktree",
                        PermissionAction::Hooks => "hooks",
                    }
                );

                self.pending_permission_gate = Some(PendingPermissionGate {
                    gate_id: gate_id.clone(),
                    action,
                    job_id,
                });

                let timeout_seconds = if self.gate_default_timeout_secs == 0 {
                    None
                } else {
                    Some(self.gate_default_timeout_secs)
                };

                let request = GateRequest {
                    gate_id,
                    thread_id: None,
                    requested_by: self.instance_id.clone(),
                    kind: GateKind::Approval,
                    timeout_seconds,
                    prompt,
                    proposed_default: Some("deny".to_string()),
                };

                let payload = serde_json::to_string(&request)
                    .context("Failed to serialize gate.request payload as JSON")?;

                // 说明：
                // - 对权限类 gate，我们把请求“显式投递到 ralph#1”，保证无需额外 TopicContract 也能看见。
                // - 同时 Supervisor 会落盘该事件，human 也能通过 events.jsonl 直接观察/回应。
                let event = Event::new(TOPIC_GATE_REQUEST, payload)
                    .with_target_instance(HatInstanceId::from_parts("ralph", "1"));

                self.publish_internal_event(event).await?;

                Ok(None)
            }
        }
    }

    async fn publish_internal_event(&mut self, event: Event) -> anyhow::Result<()> {
        let event = self.decorate_outgoing_event(event);
        self.supervisor_tx
            .send(HatInstanceEvent::Published {
                instance_id: self.instance_id.clone(),
                hat_id: self.hat.id.clone(),
                event,
            })
            .await
            .context("Failed to send Published event to supervisor")?;
        Ok(())
    }

    // =====================================================================
    // Workspace：worktree acquire/release + hooks（5.3 / 5.4）
    // =====================================================================

    async fn acquire_worktree(
        &mut self,
        job_id: u64,
        hooks_allowed: bool,
        on_acquire_hook: Option<&String>,
    ) -> anyhow::Result<PathBuf> {
        let repo_root = self.git_repo_root().await?;

        let workdir = self.worktree_dir(repo_root.as_path(), job_id);
        if let Some(parent) = workdir.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| format!("Failed to create worktree parent dir: {parent:?}"))?;
        }

        // 若目录已存在，说明上次可能异常退出。
        // 我们先清理旧目录，避免 acquire 直接失败。
        if tokio::fs::try_exists(&workdir).await.unwrap_or(false) {
            tracing::warn!(
                instance = %self.instance_id,
                job_id,
                path = ?workdir,
                backend = ?self.workspace_runtime.worktree_backend,
                "worktree dir already exists; attempting cleanup before acquire"
            );

            match self.workspace_runtime.worktree_backend {
                WorktreeBackend::Worktree => {
                    // git worktree 模式下，目录可能仍被主仓库登记为 worktree，需要先 remove。
                    let _ = self.git_worktree_remove(&repo_root, &workdir).await;
                }
                WorktreeBackend::Clone => {
                    // clone 模式下，没有主仓库 worktree 登记，直接删目录即可。
                }
            }

            tokio::fs::remove_dir_all(&workdir).await.with_context(|| {
                format!("Failed to cleanup existing workdir before acquire: {workdir:?}")
            })?;
        }

        match self.workspace_runtime.worktree_backend {
            WorktreeBackend::Worktree => {
                self.git_worktree_add(&repo_root, &workdir).await?;
            }
            WorktreeBackend::Clone => {
                self.git_clone_repo(&repo_root, &workdir).await?;
            }
        }

        // hooks（on_acquire）
        if hooks_allowed && let Some(cmd) = on_acquire_hook {
            let max_attempts = 3;
            if let Err(e) = self
                .run_hook_with_retry(job_id, "on_acquire", cmd, &workdir, max_attempts, true)
                .await
            {
                // hook 失败时要尽量回收 workdir，避免留下脏目录
                match self.workspace_runtime.worktree_backend {
                    WorktreeBackend::Worktree => {
                        let _ = self.git_worktree_remove(&repo_root, &workdir).await;
                    }
                    WorktreeBackend::Clone => {
                        let _ = tokio::fs::remove_dir_all(&workdir).await;
                    }
                }

                return Err(e);
            }
        }

        Ok(workdir)
    }

    async fn release_worktree(
        &mut self,
        job_id: u64,
        workdir: &PathBuf,
        hooks_allowed: bool,
        on_release_hook: Option<&String>,
    ) -> anyhow::Result<()> {
        // hooks（on_release）：release hook 失败不应阻止 workdir 回收（best-effort）
        if hooks_allowed && let Some(cmd) = on_release_hook {
            let max_attempts = 3;
            let _ = self
                .run_hook_with_retry(job_id, "on_release", cmd, workdir, max_attempts, false)
                .await;
        }

        let repo_root = self.git_repo_root().await?;

        match self.workspace_runtime.worktree_backend {
            WorktreeBackend::Worktree => {
                self.git_worktree_remove(&repo_root, workdir).await?;
            }
            WorktreeBackend::Clone => {
                // clone 模式下: runner 的 commit 在 clone repo 内。
                // 为了让 integrator 仍能用 commit hash cherry-pick,我们在删除目录前把 HEAD 引入主仓库。
                self.import_clone_head_into_main_repo(job_id, &repo_root, workdir)
                    .await?;

                tokio::fs::remove_dir_all(workdir)
                    .await
                    .with_context(|| format!("Failed to remove clone workdir: {workdir:?}"))?;
            }
        }

        Ok(())
    }

    async fn git_repo_root(&self) -> anyhow::Result<PathBuf> {
        let output = Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .output()
            .await
            .context("Failed to run git rev-parse --show-toplevel")?;

        if !output.status.success() {
            anyhow::bail!(
                "git rev-parse --show-toplevel failed: exit_code={:?} stderr={}",
                output.status.code(),
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(PathBuf::from(root))
    }

    fn worktree_dir(&self, repo_root: &Path, job_id: u64) -> PathBuf {
        let instance_dir = self.instance_id.to_string().replace('#', "_");
        repo_root
            .join(&self.workspace_runtime.worktree_base_dir)
            .join(instance_dir)
            .join(format!("job-{job_id}"))
    }

    async fn git_clone_repo(&self, repo_root: &PathBuf, workdir: &PathBuf) -> anyhow::Result<()> {
        let output = Command::new("git")
            .args(["clone", "--no-hardlinks"])
            .arg(repo_root)
            .arg(workdir)
            .output()
            .await
            .with_context(|| format!("Failed to run git clone for {workdir:?}"))?;

        if !output.status.success() {
            anyhow::bail!(
                "git clone failed: workdir={workdir:?} exit_code={:?} stdout={} stderr={}",
                output.status.code(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            );
        }

        Ok(())
    }

    async fn import_clone_head_into_main_repo(
        &self,
        job_id: u64,
        repo_root: &PathBuf,
        workdir: &PathBuf,
    ) -> anyhow::Result<()> {
        let repo_head = self.git_rev_parse_head(repo_root).await?;
        let clone_head = self.git_rev_parse_head(workdir).await?;

        // 没有产生新 commit 时,无需引入。
        if repo_head == clone_head {
            return Ok(());
        }

        // 说明：
        // - clone 模式下，commit 对象只存在于 workdir 的 `.git/`。
        // - 如果我们直接删除 workdir，integrator 将无法按 hash cherry-pick。
        // - 因此这里把 clone 的 HEAD 显式 fetch 进主仓库,并写入一个可追溯 ref。
        let instance_dir = self.instance_id.to_string().replace('#', "_");
        let refname = format!("refs/ralph/workspaces/{instance_dir}/job-{job_id}");

        let output = Command::new("git")
            .current_dir(repo_root)
            .args(["fetch", "--no-tags"])
            .arg(workdir)
            .arg(format!("+HEAD:{refname}"))
            .output()
            .await
            .with_context(|| {
                format!(
                    "Failed to fetch clone HEAD into main repo: workdir={workdir:?} ref={refname}"
                )
            })?;

        if !output.status.success() {
            anyhow::bail!(
                "git fetch (clone->main) failed: workdir={workdir:?} ref={refname} exit_code={:?} stdout={} stderr={}",
                output.status.code(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            );
        }

        tracing::info!(
            instance = %self.instance_id,
            job_id,
            clone_head = %clone_head,
            refname = %refname,
            "Imported clone HEAD into main repo before cleanup"
        );

        Ok(())
    }

    async fn git_rev_parse_head(&self, dir: &PathBuf) -> anyhow::Result<String> {
        let output = Command::new("git")
            .current_dir(dir)
            .args(["rev-parse", "HEAD"])
            .output()
            .await
            .with_context(|| format!("Failed to run git rev-parse HEAD in {dir:?}"))?;

        if !output.status.success() {
            anyhow::bail!(
                "git rev-parse HEAD failed: dir={dir:?} exit_code={:?} stderr={}",
                output.status.code(),
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    async fn git_worktree_add(&self, repo_root: &PathBuf, workdir: &PathBuf) -> anyhow::Result<()> {
        let output = Command::new("git")
            .current_dir(repo_root)
            .args(["worktree", "add", "--detach"])
            .arg(workdir)
            .arg("HEAD")
            .output()
            .await
            .with_context(|| format!("Failed to run git worktree add for {workdir:?}"))?;

        if !output.status.success() {
            anyhow::bail!(
                "git worktree add failed: workdir={workdir:?} exit_code={:?} stdout={} stderr={}",
                output.status.code(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            );
        }

        Ok(())
    }

    async fn git_worktree_remove(
        &self,
        repo_root: &PathBuf,
        workdir: &PathBuf,
    ) -> anyhow::Result<()> {
        let output = Command::new("git")
            .current_dir(repo_root)
            .args(["worktree", "remove", "-f"])
            .arg(workdir)
            .output()
            .await
            .with_context(|| format!("Failed to run git worktree remove for {workdir:?}"))?;

        if !output.status.success() {
            anyhow::bail!(
                "git worktree remove failed: workdir={workdir:?} exit_code={:?} stdout={} stderr={}",
                output.status.code(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            );
        }

        Ok(())
    }

    async fn run_hook_with_retry(
        &mut self,
        job_id: u64,
        phase: &str,
        cmd: &str,
        workdir: &PathBuf,
        max_attempts: u32,
        fatal: bool,
    ) -> anyhow::Result<()> {
        for attempt in 1..=max_attempts {
            let output = Command::new("sh")
                .arg("-lc")
                .arg(cmd)
                .current_dir(workdir)
                .output()
                .await
                .with_context(|| {
                    format!("Failed to spawn hook command: phase={phase} attempt={attempt}")
                })?;

            if output.status.success() {
                return Ok(());
            }

            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();

            self.publish_workspace_hook_failed(
                job_id,
                phase,
                cmd,
                attempt,
                max_attempts,
                output.status.code(),
                &stdout,
                &stderr,
            )
            .await?;

            if attempt < max_attempts {
                tokio::time::sleep(Duration::from_millis(200)).await;
                continue;
            }

            if fatal {
                anyhow::bail!(
                    "workspace hook failed after {max_attempts} attempts: phase={phase} workdir={workdir:?}"
                );
            } else {
                return Ok(());
            }
        }

        Ok(())
    }

    async fn publish_workspace_hook_failed(
        &mut self,
        job_id: u64,
        phase: &str,
        cmd: &str,
        attempt: u32,
        max_attempts: u32,
        exit_code: Option<i32>,
        stdout: &str,
        stderr: &str,
    ) -> anyhow::Result<()> {
        // 为了避免 events.jsonl 里被截断后 JSON 不可解析，这里只写 preview + total_len。
        let preview = |s: &str| -> String {
            const LIMIT: usize = 160;
            if s.len() <= LIMIT {
                return s.to_string();
            }
            let mut cut = LIMIT;
            while cut > 0 && !s.is_char_boundary(cut) {
                cut -= 1;
            }
            format!(
                "{}... [truncated, {} chars total]",
                &s[..cut],
                s.chars().count()
            )
        };

        let payload = serde_json::json!({
            "instance_id": self.instance_id.to_string(),
            "hat_id": self.hat.id.to_string(),
            "job_id": job_id,
            "phase": phase,
            "attempt": attempt,
            "max_attempts": max_attempts,
            "command": cmd,
            "exit_code": exit_code,
            "stdout_len": stdout.chars().count(),
            "stderr_len": stderr.chars().count(),
            "stdout_preview": preview(stdout),
            "stderr_preview": preview(stderr),
        })
        .to_string();

        // 说明：把 hook_failed 事件显式投递给 ralph#1，确保无需额外 TopicContract 也能看见。
        let event = Event::new("workspace.hook_failed", payload)
            .with_target_instance(HatInstanceId::from_parts("ralph", "1"));
        self.publish_internal_event(event).await?;
        Ok(())
    }

    fn build_prompt(&self, events: &[Event]) -> String {
        let mut events_context = String::new();
        for event in events {
            // 说明：这里尽量保持“可读 + 可解析”，不强制 agent 必须使用特定格式。
            // 后续如果需要更强约束，可以改成统一的 `<event ...>` 输入格式。
            events_context.push_str(&format!("- {}: {}\n", event.topic, event.payload));
        }

        let is_ralph = self.hat.id.as_str() == "ralph";

        // =====================================================================
        // 并行模式的 prompt 组装规则（关键语义）
        // =====================================================================
        //
        // 说明：
        // - `prompt_prelude` 来自 `ralph run -p ...`（或 event_loop.prompt / PROMPT.md）。
        // - 在并行模式里，这段“顶层 prompt”通常只应该影响协调者（ralph#1）。
        // - 其他 hat 的目标应该由 **事件路由**驱动（TopicContract + incoming events），
        //   否则容易出现“全员收到同一段顶层 prompt -> 角色污染 -> 不按 hat instructions 行事”的问题。
        //
        // 结论：
        // - 只有 ralph#1 注入 `prompt_prelude`
        // - 其他 hat 只看自己的 instructions + incoming events
        let prelude = if is_ralph {
            self.prompt_prelude.as_str()
        } else {
            ""
        };

        let hat_instructions = if is_ralph {
            // Ralph#1（并行协调者）：
            // - 优先使用 Supervisor 生成的“强约束协调语义”指令（含 starting_event / complete_publishes）
            // - 若未注入，则回退到最小兜底指令
            if self.hat.instructions.trim().is_empty() {
                "You are Ralph (coordinator). Handle orphaned events and decide next actions.\n"
                    .to_string()
            } else {
                self.hat.instructions.clone()
            }
        } else if !self.hat.instructions.is_empty() {
            // 说明：
            // - 并行模式更偏“事件驱动的多角色协作”，hat 的 instructions 往往已经足够具体。
            // - 如果我们再用 InstructionBuilder 的“重型模板”包一层（尤其是 VERIFY: MUST run tests），
            //   很容易把 E2E/小任务变成“跑 cargo test / 写 plan 文件”等不必要动作，导致卡死或超时。
            // - 因此：当 hat 明确提供了 instructions 时，优先尊重原文，不再额外包裹。
            self.hat.instructions.clone()
        } else {
            self.instruction_builder
                .build_custom_hat(&self.hat, &events_context)
        };

        format!(
            "{prelude}\n\n{instructions}\n\n### Incoming Events\n{events}\n",
            prelude = prelude,
            instructions = hat_instructions,
            events = events_context
        )
    }

    async fn on_job_completed(&mut self, result: HatJobResult) -> anyhow::Result<()> {
        // 将 stderr 也带上 prefix，便于 event parser / 人类阅读一致。
        // 注意：executor 里已经把 stderr 标成 chunk；这里的 output 是“拼接后的完整输出”。
        let parsed_events = EventParser::new().parse(&result.output);

        let events = parsed_events
            .into_iter()
            .map(|event| self.decorate_outgoing_event(event))
            .collect::<Vec<_>>();

        // 5.3：worktree release（best-effort；失败需要可观测，但不应吞掉 job 完成事件）
        if let Some(ws) = self.running_workspace.take()
            && ws.strategy == WorkspaceStrategy::Worktree
            && let Some(workdir) = ws.workdir
        {
            if let Err(e) = self
                .release_worktree(
                    ws.job_id,
                    &workdir,
                    ws.hooks_allowed,
                    ws.on_release_hook.as_ref(),
                )
                .await
            {
                tracing::warn!(
                    instance = %self.instance_id,
                    job_id = ws.job_id,
                    workdir = ?workdir,
                    error = %e,
                    "Failed to release worktree (best-effort)"
                );
            }
        } else {
            self.running_workspace = None;
        }

        // 成功/失败映射到实例状态：失败时先标 failed，再允许 Supervisor 决策下一步。
        if result.success {
            self.set_state(HatInstanceState::Idle).await?;
        } else {
            self.set_state(HatInstanceState::Failed).await?;
        }

        self.supervisor_tx
            .send(HatInstanceEvent::JobCompleted {
                instance_id: self.instance_id.clone(),
                hat_id: self.hat.id.clone(),
                result,
                events,
            })
            .await
            .context("Failed to send JobCompleted to supervisor")?;

        Ok(())
    }

    fn decorate_outgoing_event(&mut self, mut event: Event) -> Event {
        // 归因：补上 source（hat_id）与 source_instance（hat_id#n）。
        //
        // 说明：
        // - TUI 的 Hat Graph Radar 需要知道“发布者 hat”才能做边动画与 box 高亮；
        // - `.ralph/events.jsonl` 也会记录 `hat` / `source_instance`，两者语义应保持一致；
        // - agent 输出的 `<event ...>` 通常不带 source，因此这里在缺失时补齐即可（不要覆盖已有值）。
        if event.source.is_none() {
            event = event.with_source(self.hat.id.clone());
        }

        // 归因：补上 source_instance，便于后续路由与回放。
        event = event.with_source_instance(self.instance_id.clone());

        // 事件主键：如果 agent 没有显式提供 id，则由实例按序生成。
        // 说明：
        // - 选择 `{instance_id}:{seq}` 的形式，保证“同一实例内的事件序”稳定且可引用
        // - 这样 dispatch.decision 可以用 event_id 做关联，而不依赖全局调度顺序
        if event.id.is_none() {
            let id = format!("{}:{}", self.instance_id, self.next_event_seq);
            self.next_event_seq = self.next_event_seq.saturating_add(1);
            event = event.with_id(id);
        }

        event
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CoreConfig, EventMetadata, HatWorkspaceConfig};
    use std::collections::HashMap;

    #[derive(Debug)]
    struct NoopExecutor;

    #[async_trait::async_trait]
    impl HatJobExecutor for NoopExecutor {
        async fn execute(
            &self,
            _job: HatJob,
            _output_tx: mpsc::Sender<HatJobOutputChunk>,
            mut _cancel_rx: watch::Receiver<bool>,
        ) -> anyhow::Result<HatJobResult> {
            Ok(HatJobResult {
                output: String::new(),
                success: true,
                exit_code: Some(0),
                timed_out: false,
                canceled: false,
            })
        }
    }

    #[tokio::test(start_paused = true)]
    async fn dynamic_instance_reaped_after_idle_ttl() {
        let (output_tx, _output_rx) = mpsc::channel(8);
        let (supervisor_tx, mut supervisor_rx) = mpsc::channel(8);

        let instruction_builder = Arc::new(InstructionBuilder::with_events(
            "LOOP_COMPLETE",
            CoreConfig::default(),
            HashMap::<String, EventMetadata>::new(),
        ));

        let _handle = HatInstanceHandle::spawn(
            HatInstanceId::new("writer#2"),
            Hat::new("writer", "Writer").subscribe("build.task"),
            None::<HatConfig>,
            WorkspaceRuntimeConfig::default(),
            PermissionsConfig::default(),
            60,
            None,
            None,
            String::new(),
            instruction_builder,
            Arc::new(NoopExecutor),
            output_tx,
            supervisor_tx,
            Arc::new(Semaphore::new(1)),
            true,
            Duration::from_secs(30),
        );

        // 先推进一次 tick，让 idle_since 从“当前 tokio 时间”开始计时。
        tokio::time::advance(Duration::from_millis(250)).await;
        tokio::task::yield_now().await;

        // 再推进超过 30s，触发动态实例的 idle 回收。
        tokio::time::advance(Duration::from_secs(31)).await;
        tokio::task::yield_now().await;

        let mut seen_done = false;
        for _ in 0..32 {
            while let Ok(msg) = supervisor_rx.try_recv() {
                if let HatInstanceEvent::StateChanged { instance_id, state } = msg
                    && instance_id.as_str() == "writer#2"
                    && state == HatInstanceState::Done
                {
                    seen_done = true;
                    break;
                }
            }

            if seen_done {
                break;
            }

            tokio::task::yield_now().await;
        }

        assert!(
            seen_done,
            "Expected dynamic instance to self-shutdown after idle TTL"
        );
    }

    #[test]
    fn workspace_strategy_merge_rule_is_strongest_isolation_wins() {
        let hat_config = HatConfig {
            name: "Writer".to_string(),
            description: Some("writer test".to_string()),
            triggers: vec!["build.task".to_string()],
            publishes: Vec::new(),
            instructions: String::new(),
            backend: None,
            job_timeout_secs: None,
            default_publishes: None,
            max_activations: None,
            instances: 1,
            capabilities: Vec::new(),
            workspace: HatWorkspaceConfig {
                strategy: WorkspaceStrategy::Shared,
                ..HatWorkspaceConfig::default()
            },
        };

        let (cmd_tx, cmd_rx) = mpsc::channel(1);
        let _ = cmd_tx;
        let (output_tx, _output_rx) = mpsc::channel(1);
        let (supervisor_tx, _supervisor_rx) = mpsc::channel(1);

        let instruction_builder = Arc::new(InstructionBuilder::with_events(
            "LOOP_COMPLETE",
            CoreConfig::default(),
            HashMap::<String, EventMetadata>::new(),
        ));

        let mut actor = HatInstanceActor {
            instance_id: HatInstanceId::new("writer#1"),
            hat: Hat::new("writer", "Writer").subscribe("build.task"),
            hat_config: Some(hat_config),
            workspace_runtime: WorkspaceRuntimeConfig::default(),
            permissions: PermissionsConfig::default(),
            gate_default_timeout_secs: 60,
            job_timeout: None,
            job_output_stale_timeout: None,
            prompt_prelude: String::new(),
            instruction_builder,
            executor: Arc::new(NoopExecutor),
            output_tx,
            supervisor_tx,
            job_semaphore: Arc::new(Semaphore::new(1)),
            is_dynamic: false,
            dynamic_idle_ttl: Duration::from_secs(30),
            cmd_rx,
            state: HatInstanceState::Idle,
            pending: vec![
                Event::new("x", "a").with_workspace_strategy(WorkspaceStrategy::Shared),
                Event::new("y", "b").with_workspace_strategy(WorkspaceStrategy::Patch),
            ],
            next_job_id: 1,
            next_event_seq: 1,
            dynamic_idle_since: None,
            pending_permission_gate: None,
            worktree_override: None,
            hooks_override: None,
            running_workspace: None,
            running: None,
            cancel_tx: None,
        };

        assert_eq!(
            actor.merged_workspace_strategy_for_pending(),
            WorkspaceStrategy::Patch
        );

        actor
            .pending
            .push(Event::new("z", "c").with_workspace_strategy(WorkspaceStrategy::Worktree));
        assert_eq!(
            actor.merged_workspace_strategy_for_pending(),
            WorkspaceStrategy::Worktree
        );
    }

    #[test]
    fn decorate_outgoing_event_sets_source_instance_and_id_and_source_hat() {
        let (cmd_tx, cmd_rx) = mpsc::channel(1);
        let _ = cmd_tx;
        let (output_tx, _output_rx) = mpsc::channel(1);
        let (supervisor_tx, _supervisor_rx) = mpsc::channel(1);

        let instruction_builder = Arc::new(InstructionBuilder::with_events(
            "LOOP_COMPLETE",
            CoreConfig::default(),
            HashMap::<String, EventMetadata>::new(),
        ));

        let mut actor = HatInstanceActor {
            instance_id: HatInstanceId::new("writer#1"),
            hat: Hat::new("writer", "Writer").subscribe("build.task"),
            hat_config: None,
            workspace_runtime: WorkspaceRuntimeConfig::default(),
            permissions: PermissionsConfig::default(),
            gate_default_timeout_secs: 60,
            job_timeout: None,
            job_output_stale_timeout: None,
            prompt_prelude: String::new(),
            instruction_builder,
            executor: Arc::new(NoopExecutor),
            output_tx,
            supervisor_tx,
            job_semaphore: Arc::new(Semaphore::new(1)),
            is_dynamic: false,
            dynamic_idle_ttl: Duration::from_secs(30),
            cmd_rx,
            state: HatInstanceState::Idle,
            pending: Vec::new(),
            next_job_id: 1,
            next_event_seq: 1,
            dynamic_idle_since: None,
            pending_permission_gate: None,
            worktree_override: None,
            hooks_override: None,
            running_workspace: None,
            running: None,
            cancel_tx: None,
        };

        let event = Event::new("build.task", "hello");
        let decorated = actor.decorate_outgoing_event(event);

        assert_eq!(
            decorated
                .source_instance
                .as_ref()
                .expect("source_instance must be set")
                .as_str(),
            "writer#1"
        );
        assert_eq!(
            decorated
                .source
                .as_ref()
                .expect("source must be set")
                .as_str(),
            "writer",
            "并行模式下也应为事件补齐 source（发布者 hat），便于 UI/路由/诊断一致"
        );
        assert!(
            decorated.id.is_some(),
            "event id should be generated when missing"
        );
    }
}
