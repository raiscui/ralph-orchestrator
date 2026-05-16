//! ParallelSupervisor：并行 HatInstance 调度与路由（最小闭环）。
//!
//! 说明：
//! - 该实现目标是先把并行的“骨架”跑通：
//!   - 多实例并发执行 headless job
//!   - 解析 `<event ...>` 并继续路由
//! - 3.x 任务会把路由语义补齐到“可依赖”：显式 TopicContract + recipients 计算 + missing 分支 + 决策落盘。

mod capability_runtime;
mod gate;
mod routing;
#[cfg(test)]
mod routing_tests;

use super::{
    CommandQueue, HatInstanceCommand, HatInstanceEvent, HatInstanceHandle, HatJobExecutor,
    HatJobOutputChunk, TopicContractStore,
};
use crate::config::{HatBackend, HatConfig, RalphConfig};
use crate::event_emission_protocol;
use crate::event_logger::EventLogger;
use crate::evidence_index::EvidenceIndexWriter;
use crate::hat_registry::HatRegistry;
use crate::instructions::InstructionBuilder;
use crate::prompt_overlay;
use crate::{
    AgentInstanceSnapshot, AgentLastInput, AgentsSnapshot, CapabilityMetadata, EventParser,
    EventReader as FileEventReader, RuntimeCapabilityInvoker, TerminationReason,
    render_parent_capability_catalog,
};
use anyhow::Context;
use ralph_proto::{
    Event, Hat, HatId, HatInstanceId, HatInstanceState, RuntimeDeliveryKind, RuntimeDeliveryRecord,
    RuntimeLifecycleKind, RuntimeLifecycleRecord, SessionStrategy, TurnAction, WorkspaceStrategy,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Semaphore, mpsc, watch};

/// 并行运行的结果摘要。
#[derive(Debug, Clone)]
pub struct ParallelRunResult {
    pub termination: Option<TerminationReason>,
    /// 并行模式的 iteration 口径: ralph#1 的 job 完成次数(用于 termination 诊断).
    pub ralph_iterations: u32,
    pub instance_states: HashMap<HatInstanceId, HatInstanceState>,
    pub output_chunks: usize,
}

/// Runtime graph delivery mode for live and durable observability.
pub type RuntimeDeliveryMode = RuntimeDeliveryKind;

/// Best-effort live delivery observation for runtime graph recording.
#[derive(Debug, Clone)]
pub struct RuntimeDeliveryObservation {
    pub topic: String,
    pub source_instance: Option<HatInstanceId>,
    pub recipient: HatInstanceId,
    pub mode: RuntimeDeliveryMode,
}

/// 并行调度器。
pub struct ParallelSupervisor {
    config: RalphConfig,
    registry: HatRegistry,
    instruction_builder: Arc<InstructionBuilder>,
    prompt_prelude: String,
    all_hat_prompt: Option<String>,
    executor: Arc<dyn HatJobExecutor>,
    contracts: TopicContractStore,
    event_logger: EventLogger,
    evidence_index_writer: EvidenceIndexWriter,

    // 运行态
    instances: HashMap<HatInstanceId, HatInstanceHandle>,
    instances_by_hat: HashMap<HatId, Vec<HatInstanceId>>,
    instance_states: HashMap<HatInstanceId, HatInstanceState>,
    rr_cursor_by_topic: HashMap<String, usize>,

    // agents 快照(供 `ralph agents` 查询)
    //
    // 说明：
    // - 只有在 ralph-cli 显式启用时才会落盘,避免测试/库用法污染工作区。
    agents_snapshot_path: Option<std::path::PathBuf>,
    agents_last_inputs: HashMap<HatInstanceId, AgentLastInput>,

    // 路由批次内的“乐观运行态”：
    // - 在同一个 job 输出里可能一次解析出多条事件。
    // - Supervisor 顺序路由这些事件时，无法同时消费 `StateChanged`，因此后续事件可能看不到
    //   “刚启动的 job 已进入 Running”的状态更新。
    // - 这里用批次级别 inflight 集合作为补偿：只在 batch 内生效，用完即清空。
    routing_batch_depth: u32,
    routing_inflight_instances: HashSet<HatInstanceId>,

    // autoscale / 动态实例管理
    dynamic_instances: HashSet<HatInstanceId>,
    next_instance_seq_by_hat: HashMap<HatId, u64>,
    job_semaphore: Arc<Semaphore>,
    command_queue: Arc<CommandQueue>,

    // 并行运行时通道（用于按需 spawn 新实例）
    output_tx: Option<mpsc::Sender<HatJobOutputChunk>>,
    instance_tx: Option<mpsc::Sender<HatInstanceEvent>>,

    // queue 决策缓存（event_id -> chosen_instance），用于 replay/恢复时不重算
    queue_decisions: HashMap<String, HatInstanceId>,

    // request-reply 原请求索引（event_id -> source_instance）
    //
    // 说明：
    // - `reply.hat.message` 需要根据 `reply=<request_event_id>` 找回原请求方实例；
    // - 这里只存最薄的一层信息，避免引入完整事件缓存。
    request_reply_origins: HashMap<String, Option<HatInstanceId>>,

    // Human gate 状态机（open gates + timeout）
    gates: gate::GateManager,

    // parent-run capability invocation:
    // - catalog 是 parent-side selection UX,只注入给 coordinator prompt。
    // - handled ids 是 parent run 进程内幂等表。
    // - invoker 由 CLI 注入,core 不直接知道 child/micro-run 如何执行。
    runtime_capability_catalog: Vec<CapabilityMetadata>,
    handled_capability_request_ids: HashSet<String>,
    runtime_capability_invoker: Option<Arc<dyn RuntimeCapabilityInvoker>>,

    // Supervisor 自己生成的任务序号（用于 decider job 的稳定命名）
    next_decision_job_id: u64,

    // UX/运行策略（由 ralph-cli 在启动时注入；不落盘到配置文件）
    //
    // 说明：
    // - 这些开关用于区分 parallel-cli（无人值守）与 parallel-tui（交互式会话）。
    // - 我们刻意不把它们做成 config 字段，避免用户 YAML 复杂度上升，也避免 CI/E2E 行为漂移。
    pause_on_completion_promise: bool,
    disable_dynamic_instance_reap: bool,
    /// Idle start: Supervisor 启动后先待机,不自动投递 `task.start`。
    ///
    /// 说明：
    /// - 该模式用于“启动后等待 human 第一条指令”的交互式体验(0 token 待机)。
    /// - 仅对 fresh run 生效；resume/continue 语义保持不变。
    /// - 这是运行时 UX 开关,由 ralph-cli 注入,不落盘到配置文件。
    idle_start: bool,

    output_observer: Option<Arc<dyn Fn(&HatJobOutputChunk) + Send + Sync>>,
    instance_state_observer: Option<Arc<dyn Fn(&HatInstanceId, HatInstanceState) + Send + Sync>>,
    /// 事件观察者（用于并行 TUI 展示 gate/chat 等控制面事件）。
    event_observer: Option<Arc<dyn Fn(&Event) + Send + Sync>>,
    /// 投递观察者（用于 live runtime graph 记录 source -> recipient 关系）。
    delivery_observer: Option<Arc<dyn Fn(&RuntimeDeliveryObservation) + Send + Sync>>,
}

impl ParallelSupervisor {
    /// 创建一个并行调度器。
    ///
    /// `prompt_prelude` 通常来自 PROMPT.md（或 `event_loop.prompt` 覆盖）。
    pub fn new(
        config: RalphConfig,
        prompt_prelude: String,
        executor: Arc<dyn HatJobExecutor>,
    ) -> anyhow::Result<Self> {
        let registry = HatRegistry::from_config(&config);

        let instruction_builder = Arc::new(InstructionBuilder::with_events(
            &config.event_loop.completion_promise,
            config.core.clone(),
            config.events.clone(),
        ));

        // 说明（parallel-trigger-routing）：TopicContract 现在是可选覆盖层。
        // - 命中 contract：按 contract 路由
        // - 未命中 / contracts 为空：回退到 triggers 默认路由（topic → hats fanout）
        let contracts = TopicContractStore::new(&config.parallel.topic_contracts);
        let max_running_jobs = config.parallel.autoscale.max_running_jobs.max(1);
        let job_semaphore = Arc::new(Semaphore::new(max_running_jobs));
        let command_queue = Arc::new(CommandQueue::new());
        let all_hat_prompt = prompt_overlay::load_all_hat_prompt(&config.core)
            .map_err(anyhow::Error::msg)
            .context("failed to load configured all-hat overlay")?;

        let evidence_index_path = config.core.resolve_path(EvidenceIndexWriter::DEFAULT_PATH);

        Ok(Self {
            config,
            registry,
            instruction_builder,
            prompt_prelude,
            all_hat_prompt,
            executor,
            contracts,
            event_logger: EventLogger::default_path(),
            evidence_index_writer: EvidenceIndexWriter::new(evidence_index_path),
            instances: HashMap::new(),
            instances_by_hat: HashMap::new(),
            instance_states: HashMap::new(),
            rr_cursor_by_topic: HashMap::new(),
            agents_snapshot_path: None,
            agents_last_inputs: HashMap::new(),
            routing_batch_depth: 0,
            routing_inflight_instances: HashSet::new(),
            dynamic_instances: HashSet::new(),
            next_instance_seq_by_hat: HashMap::new(),
            job_semaphore,
            command_queue,
            output_tx: None,
            instance_tx: None,
            queue_decisions: HashMap::new(),
            request_reply_origins: HashMap::new(),
            gates: gate::GateManager::new(),
            runtime_capability_catalog: Vec::new(),
            handled_capability_request_ids: HashSet::new(),
            runtime_capability_invoker: None,
            next_decision_job_id: 1,
            pause_on_completion_promise: false,
            disable_dynamic_instance_reap: false,
            idle_start: false,
            output_observer: None,
            instance_state_observer: None,
            event_observer: None,
            delivery_observer: None,
        })
    }

    /// 设置输出观察者（例如 ralph-cli 打印 `[writer#1] ...`）。
    pub fn with_output_observer(
        mut self,
        observer: Arc<dyn Fn(&HatJobOutputChunk) + Send + Sync>,
    ) -> Self {
        self.output_observer = Some(observer);
        self
    }

    /// 设置实例状态观察者（用于 CLI/TUI 展示）。
    pub fn with_instance_state_observer(
        mut self,
        observer: Arc<dyn Fn(&HatInstanceId, HatInstanceState) + Send + Sync>,
    ) -> Self {
        self.instance_state_observer = Some(observer);
        self
    }

    /// 设置事件观察者（用于 TUI 展示 gate/chat 等事件）。
    pub fn with_event_observer(mut self, observer: Arc<dyn Fn(&Event) + Send + Sync>) -> Self {
        self.event_observer = Some(observer);
        self
    }

    /// 设置投递观察者（用于 live runtime graph 记录）。
    pub fn with_delivery_observer(
        mut self,
        observer: Arc<dyn Fn(&RuntimeDeliveryObservation) + Send + Sync>,
    ) -> Self {
        self.delivery_observer = Some(observer);
        self
    }

    /// 设置 parent-visible runtime capability catalog。
    ///
    /// 说明:
    /// - 这里只是 `ralph#1` 的选择 UX,不创建、不替换、不注入 live topology。
    /// - 真正执行仍由 `with_runtime_capability_invoker()` 注入的 isolated adapter 完成。
    #[must_use]
    pub fn with_runtime_capability_catalog(
        mut self,
        catalog: impl IntoIterator<Item = CapabilityMetadata>,
    ) -> Self {
        self.runtime_capability_catalog = catalog.into_iter().collect();
        self
    }

    fn coordinator_publish_topics(&self) -> Vec<String> {
        let mut topics = self
            .config
            .hats
            .values()
            .flat_map(|hat| hat.triggers.iter().cloned())
            .collect::<Vec<_>>();

        topics.push("human.message".to_string());
        topics.push("reply.human.message".to_string());
        topics.push("reply.hat.message".to_string());
        topics.push(crate::TOPIC_CAPABILITY_REQUEST.to_string());

        if let Some(starting_event) = self.config.event_loop.starting_event.as_deref() {
            topics.push(starting_event.to_string());
        }

        topics.sort();
        topics.dedup();
        topics
    }

    /// 设置 runtime capability invocation adapter。
    ///
    /// 说明:
    /// - 这个 hook 只负责处理 `ralph#1` 在 parent run 中产出的 `capability.request`。
    /// - adapter 必须自己保证 isolated child/micro-run 执行,不能热改 parent topology。
    #[must_use]
    pub fn with_runtime_capability_invoker(
        mut self,
        invoker: Arc<dyn RuntimeCapabilityInvoker>,
    ) -> Self {
        self.runtime_capability_invoker = Some(invoker);
        self
    }

    /// 启用 `.ralph/agents.json` 落盘(供 `ralph agents` 命令查询)。
    ///
    /// 说明：
    /// - 该快照是运行时可观测性产物,默认不落盘(避免库用法/测试污染工作区)。
    /// - ralph-cli 在 parallel 模式启动时会显式启用。
    #[must_use]
    pub fn with_agents_snapshot_to_default_path(mut self) -> Self {
        self.agents_snapshot_path = Some(self.config.core.resolve_path(".ralph/agents.json"));
        self
    }

    /// 启用 agents 快照落盘到自定义路径(用于测试或外部集成)。
    #[must_use]
    pub fn with_agents_snapshot_path(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.agents_snapshot_path = Some(path.into());
        self
    }

    /// 并行 TUI：将 completion promise（默认 `LOOP_COMPLETE`）视为“暂停信号”，而不是“退出信号”。
    ///
    /// 说明：
    // - 仅影响并行 Supervisor（parallel mode）。
    // - 典型用法：ralph-cli 在 enable_tui=true 时开启，避免交互式会话被 `LOOP_COMPLETE` 强制中断。
    #[must_use]
    pub fn with_pause_on_completion_promise(mut self, enabled: bool) -> Self {
        self.pause_on_completion_promise = enabled;
        self
    }

    /// Idle start: 启动后待机,等待 external events 触发第一次 job。
    #[must_use]
    pub fn with_idle_start(mut self, enabled: bool) -> Self {
        self.idle_start = enabled;
        self
    }

    /// 并行 TUI：禁用动态实例 idle TTL 回收（避免 instance 进入 `done` 断对话）。
    ///
    /// 说明：
    // - 只影响 autoscale 动态实例（is_dynamic=true）。
    // - 静态实例本来就不会因 TTL 自动回收。
    #[must_use]
    pub fn with_disable_dynamic_instance_reap(mut self, disabled: bool) -> Self {
        self.disable_dynamic_instance_reap = disabled;
        self
    }

    fn effective_dynamic_idle_ttl(&self) -> Duration {
        // 说明：
        // - 动态实例的 idle 回收是为了“无人值守”场景降低资源占用；
        // - 但在交互式 TUI 会话里，回收会造成 instance 进入 `done`，
        //   从而让 human message（默认定向到 selected instance）变得不可达。
        //
        // 因此：
        // - parallel-tui：用一个极大 TTL 达到“等价禁用回收”的效果。
        // - parallel-cli：保持配置值（默认 30s），避免实例无限增长。
        if self.disable_dynamic_instance_reap {
            Duration::from_secs(u64::MAX)
        } else {
            Duration::from_secs(self.config.parallel.autoscale.dynamic_idle_ttl_secs.max(1))
        }
    }

    /// 启动并运行，直到收到完成信号或被打断。
    pub async fn run(self, resume: bool) -> anyhow::Result<ParallelRunResult> {
        // 说明:
        // - 默认 run() 不接收外部中断信号,因此这里创建一个永不触发的 watch channel 作为占位.
        // - 这样我们可以在同一份 run_inner 里统一处理“可选中断”逻辑,避免重复实现.
        let (_interrupt_tx, interrupt_rx) = watch::channel(false);
        Self::run_inner(self, resume, interrupt_rx).await
    }

    /// 启动并运行,并监听外部中断信号(例如 Ctrl+C/SIGTERM)。
    pub async fn run_with_interrupt(
        self,
        resume: bool,
        interrupt_rx: watch::Receiver<bool>,
    ) -> anyhow::Result<ParallelRunResult> {
        Self::run_inner(self, resume, interrupt_rx).await
    }

    async fn run_inner(
        mut self,
        resume: bool,
        mut interrupt_rx: watch::Receiver<bool>,
    ) -> anyhow::Result<ParallelRunResult> {
        let (output_tx, mut output_rx) = mpsc::channel::<HatJobOutputChunk>(256);
        let (instance_tx, mut instance_rx) = mpsc::channel::<HatInstanceEvent>(256);

        // =====================================================================
        // 并行模式的“硬退出护栏”（Backpressure over prescription）
        // =====================================================================
        //
        // 说明：
        // - 并行 Supervisor 不能只依赖 `completion_promise`（例如 LOOP_COMPLETE）。
        // - 在 E2E / CI / 无人值守环境中，只要模型漂移导致不输出 promise，就会无限跑下去。
        // - 这里对齐串行 event_loop 的 safeguard：max_iterations / max_runtime。
        //
        // 迭代语义（并行模式的当前版本）：
        // - 先用“ralph#1 job 完成次数”作为 iteration 计数的近似。
        // - 这能覆盖绝大多数无人值守场景：ralph#1 负责协调与收敛，迭代次数也主要由它驱动。
        // max_runtime 计时（并行模式硬退出护栏）：
        //
        // 说明：
        // - parallel-cli/CI：从 run 启动开始计时（与串行一致）。
        // - idle_start：这是“无 prompt 的常驻会话”模式。
        //   在该模式下,Supervisor 级 max_runtime 整个会话都禁用:
        //   - 等待第一条 external event 之前不计时
        //   - 第一条 human.message 之后也不重新开始计时
        //   这样主 `ralph#1` 不会因为长时间对话/待机被 MaxRuntime 收掉。
        // - parallel-tui：当进入 `LOOP_COMPLETE` 暂停态后，计时会重置并暂停；
        //   直到任意实例再次进入 Running（新的 job 启动）才重新开始计时（见 specs）。
        let max_runtime_disabled = self.idle_start;
        let mut max_runtime_started_at = std::time::Instant::now();
        let mut max_runtime_waiting_for_running =
            !max_runtime_disabled && !resume && self.idle_start;
        let mut max_runtime_counting = !max_runtime_disabled && !max_runtime_waiting_for_running;
        let mut ralph_iterations: u32 = 0;
        // completion_promise 属于“软退出信号”：
        // - 不应当立刻 break，否则同一轮输出里解析出的事件可能还没来得及路由/触发下游 job。
        // - 这里用一个很短的 drain 窗口，给并行实例“把最后一波事件跑完并落盘”的机会。
        let completion_drain_min = Duration::from_millis(500);
        // 最大 drain 窗口要比 tick/min 大很多：
        // - 真实后端（尤其是 Codex）在冷启动/高负载时，单次 job 可能轻松超过 10-20s
        // - 如果窗口太短，会导致“ralph 提前输出 completion -> 下游 job 还没来得及产出事件就被 cancel”
        let completion_drain_max = Duration::from_secs(60);
        let mut completion_promise_seen_at: Option<std::time::Instant> = None;

        // completion lock（并行 TUI 暂停态）：
        //
        // 说明：
        // - 目的不是“退出”，而是把并行运行时停在一个稳定点：
        //   - 不再因为内部延迟事件继续派生新 job（保留收敛护栏）
        //   - 仍然允许 human 通过 external events 恢复继续对话/继续工作
        //
        // - parallel-cli：不会使用该锁（看到 completion 直接走 termination+drain）。
        let mut completion_lockdown = false;

        // 保存通道句柄，后续 missing_instance_policy=spawn 需要动态创建实例。
        self.output_tx = Some(output_tx.clone());
        self.instance_tx = Some(instance_tx.clone());

        // replay/恢复：先从 events.jsonl 读入 dispatch.decision，避免重算 queue 决策。
        if resume {
            self.load_queue_decisions_from_history()?;
            self.load_request_reply_origins_from_history()?;
        }

        self.spawn_instances()?;
        self.write_agents_snapshot_best_effort();

        // 初始事件：task.start / task.resume
        //
        // 说明：
        // - `task.start`/`task.resume` 属于控制面 handshake 事件，payload 是 top-level prompt。
        // - 为了避免 wildcard hat 收到该 payload 造成“角色污染”，并行模式下强制投递给 ralph#1。
        // - idle_start(fresh): 不自动投递 `task.start`，从而实现 0 token 待机(等待 human 第一条指令)。
        if resume || !self.idle_start {
            let start_topic = if resume { "task.resume" } else { "task.start" };
            let mut start_event = Event::new(start_topic, &self.prompt_prelude)
                .with_target_instance(HatInstanceId::from_parts("ralph", "1"));
            self.ensure_event_id(&mut start_event);
            self.route_event(start_event).await?;
        }

        let mut termination: Option<TerminationReason> = None;
        let mut output_chunks = 0usize;

        // 外部事件输入（human / `ralph emit`）：
        // - 遵循现有约定：读取 `.ralph/current-events` 指向的 JSONL
        // - 若 marker 不存在，则回退到 `.ralph/events.jsonl`
        let marker_path = self.config.core.resolve_path(".ralph/current-events");
        let events_path_str = std::fs::read_to_string(&marker_path)
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| EventLogger::DEFAULT_PATH.to_string());
        let events_path = self.config.core.resolve_path(&events_path_str);
        let mut external_event_reader = FileEventReader::new(events_path.clone());

        // tick 用于：poll 外部事件 + 检查 gate timeout（不要阻塞其他 HatInstance）
        let mut tick = tokio::time::interval(Duration::from_millis(200));

        loop {
            tokio::select! {
                changed = interrupt_rx.changed() => {
                    if changed.is_ok() && *interrupt_rx.borrow() {
                        termination = Some(TerminationReason::Interrupted);
                        break;
                    }
                }
                chunk = output_rx.recv() => {
                    let Some(chunk) = chunk else { break };
                    output_chunks += 1;
                    if let Some(observer) = &self.output_observer {
                        observer(&chunk);
                    }
                }
                msg = instance_rx.recv() => {
                    let Some(msg) = msg else { break };
                    match msg {
                        HatInstanceEvent::StateChanged { instance_id, state } => {
                            self.instance_states.insert(instance_id.clone(), state);
                            self.notify_instance_state_observer(&instance_id, state);
                            self.log_runtime_lifecycle_state(&instance_id, state);

                            // max_runtime：暂停态(例如 idle_start / parallel-tui completion pause)下，
                            // 直到看到任意实例进入 Running 才重新开始计时。
                            if !max_runtime_disabled
                                && max_runtime_waiting_for_running
                                && state == HatInstanceState::Running
                            {
                                max_runtime_started_at = std::time::Instant::now();
                                max_runtime_counting = true;
                                max_runtime_waiting_for_running = false;
                            }

                            // 3.5：动态实例 idle 回收后会 self-shutdown 并进入 Done。
                            // Supervisor 负责把它从“可投递 registry”里移除，避免后续路由继续选中。
                            if state == HatInstanceState::Done
                                && self.dynamic_instances.contains(&instance_id)
                            {
                                self.unregister_dynamic_instance(&instance_id);
                            }

                            // 状态变化属于最关键的可观测信号：及时刷新 agents 快照。
                            self.write_agents_snapshot_best_effort();
                        }
                        HatInstanceEvent::JobCompleted { instance_id, hat_id, result, events } => {
                            // 记录：把解析到的事件写入 debug events.jsonl（方便排查）
                            // 注意：这里先用 iteration=0（并行模式下“迭代”语义后续再收敛）。
                            for event in &events {
                                let triggered = self.registry.find_by_trigger(event.topic.as_str());
                                let _ = self.event_logger.log_event(0, hat_id.as_str(), event, triggered);
                            }

                            let capability_return_events = self
                                .handle_parent_capability_requests(&hat_id, &events)
                                .await?;
                            for event in &capability_return_events {
                                let triggered = self.registry.find_by_trigger(event.topic.as_str());
                                let _ = self.event_logger.log_event(0, "capability", event, triggered);
                            }
                            let routable_events = if hat_id.as_str() == "ralph" {
                                events
                                    .into_iter()
                                    .filter(|event| {
                                        event.topic.as_str() != crate::TOPIC_CAPABILITY_REQUEST
                                    })
                                    .collect()
                            } else {
                                events
                            };

                            // 完成判断：仅 Ralph 的输出可触发 completion promise（沿用现有规则）。
                            // 注意：不要在这里立刻 break（见上方 drain 说明）。
                            let completion_promise = hat_id.as_str() == "ralph"
                                && EventParser::contains_promise(
                                    &result.output_for_parsing,
                                    &self.config.event_loop.completion_promise,
                                );

                            if completion_promise {
                                if self.pause_on_completion_promise {
                                    // TUI：进入暂停态（但不退出）。
                                    completion_lockdown = true;

                                    // 同时重置并暂停 max_runtime 计时：
                                    // - 暂停态期间不计时（允许长时间等待 human 输入）
                                    // - 直到任意实例再次 Running 才恢复计时
                                    if !max_runtime_disabled {
                                        max_runtime_started_at = std::time::Instant::now();
                                        max_runtime_counting = false;
                                        max_runtime_waiting_for_running = true;
                                    }
                                } else if completion_promise_seen_at.is_none() {
                                    // CLI/CI：进入收敛退出态。
                                    self.freeze_pending_on_all_instances();
                                    termination = Some(TerminationReason::CompletionPromise);
                                    completion_promise_seen_at = Some(std::time::Instant::now());
                                }
                            }

                            let stop_spawning =
                                matches!(termination, Some(TerminationReason::CompletionPromise))
                                    || completion_lockdown;

                            // 将新事件继续路由
                            //
                            // 说明：
                            // - completion promise 之后，我们不再派生任何新 job（避免“已收敛但仍继续跑”的假活跃）。
                            // - 新语义下，CLI/CI 在看到 completion 后会立刻冻结所有 instance 的 pending，
                            //   因此连“触发 completion 的这一批输出事件”也不再继续路由。
                            // - 仅 parallel-tui 的 pause 模式保留旧行为：允许当前 completion 批里的事件继续路由，
                            //   因为用户可能希望在暂停态下继续对话并恢复运行。
                            // - 在顺序路由这些事件时，我们无法同时消费 `StateChanged`，
                            //   因此需要在 batch 内维护一份“乐观运行态”，避免后续事件误判实例仍然空闲。
                            if !stop_spawning
                                || (completion_promise && self.pause_on_completion_promise)
                            {
                                self.route_events_batch(routable_events).await?;
                                if !capability_return_events.is_empty() {
                                    self.route_events_batch(capability_return_events).await?;
                                }
                            }

                            // 迭代上限：以 ralph#1 的 job 完成次数为准（见上方说明）。
                            if hat_id.as_str() == "ralph" {
                                ralph_iterations = ralph_iterations.saturating_add(1);
                                if ralph_iterations >= self.config.event_loop.max_iterations {
                                    termination = Some(TerminationReason::MaxIterations);
                                    break;
                                }
                            }

                            // TODO: 当所有实例都 idle 且无新事件时，允许自然结束（需要更可靠的“无事可做”判断）。
                            let _ = instance_id; // 保留给后续：输出归因/状态统计
                        }
                        HatInstanceEvent::Published { instance_id, hat_id, mut event } => {
                            let _ = instance_id; // 保留给后续：输出归因/状态统计

                            self.ensure_event_id(&mut event);

                            // Published 事件同样需要落盘，保证 replay/排查时可追溯。
                            let triggered = self.registry.find_by_trigger(event.topic.as_str());
                            let _ = self
                                .event_logger
                                .log_event(0, hat_id.as_str(), &event, triggered);

                            // completion promise（或 TUI 暂停态）之后不再派生新 job（但仍可落盘，便于排障）。
                            let stop_spawning =
                                matches!(termination, Some(TerminationReason::CompletionPromise))
                                    || completion_lockdown;
                            if !stop_spawning {
                                self.route_event(event).await?;
                            }
                        }
                    }
                }
                _ = tick.tick() => {
                    // max_runtime：超时后直接退出（并触发 cancel/shutdown），避免无人值守卡死。
                    if !max_runtime_disabled
                        && max_runtime_counting
                        && max_runtime_started_at.elapsed()
                            >= Duration::from_secs(self.config.event_loop.max_runtime_seconds)
                    {
                        termination = Some(TerminationReason::MaxRuntime);
                        break;
                    }

                    // completion_promise drain：给并行实例一个很短的“收尾窗口”，避免同轮输出的事件来不及触发下游。
                    if matches!(termination, Some(TerminationReason::CompletionPromise))
                        && let Some(at) = completion_promise_seen_at
                    {
                        // 最少等一小会儿，避免 race：下游还没来得及把 state 从 Created 切到 Running。
                        if at.elapsed() >= completion_drain_min {
                            let any_running = self
                                .instance_states
                                .values()
                                .any(|s| *s == HatInstanceState::Running);

                            if !any_running {
                                break;
                            }
                        }

                        // 兜底：别无限等（即使 job-level timeout 配置很大，也要在 completion 收尾时尽快退出）。
                        if at.elapsed() >= completion_drain_max {
                            break;
                        }
                    }

                    // completion promise 之后进入“收敛态”：
                    // - 不再接收/派发任何新事件（包括 external/gate.timeout）
                    // - 只做 drain：等待在跑的 job 自然结束，或 hit completion_drain_max
                    if matches!(termination, Some(TerminationReason::CompletionPromise)) {
                        continue;
                    }

                    // 1) 读取外部事件（human/工具写入的 JSONL）
                    match external_event_reader.read_new_events() {
                        Ok(parse) => {
                            // 暂停态：只要 human 注入了外部事件，就视为“继续对话/继续工作”，解除 lockdown。
                            if completion_lockdown && !parse.events.is_empty() {
                                completion_lockdown = false;

                                // 说明：
                                // - 解除暂停态并不意味着“立刻开始计时”：
                                //   你要求必须等到任意实例进入 Running 才开始重新计时。
                                // - 但如果此时已经存在 Running（极端竞态/实现差异），我们就直接开始计时，避免永久暂停。
                                if !max_runtime_disabled
                                    && self.pause_on_completion_promise
                                    && max_runtime_waiting_for_running
                                    && self
                                        .instance_states
                                        .values()
                                        .any(|s| *s == HatInstanceState::Running)
                                {
                                    max_runtime_started_at = std::time::Instant::now();
                                    max_runtime_counting = true;
                                    max_runtime_waiting_for_running = false;
                                }
                            }

                            for raw in parse.events {
                                let payload = raw.payload.unwrap_or_default();
                                let mut event = Event::new(raw.topic, payload);
                                if let Some(target) = raw.target {
                                    event = event.with_target(target);
                                }
                                if let Some(target_instance) = raw.target_instance {
                                    event = event.with_target_instance(target_instance);
                                }
                                if matches!(raw.spawn_instance, Some(true)) {
                                    event = event.with_spawn_instance(true);
                                }
                                if let Some(strategy) = raw
                                    .workspace_strategy
                                    .as_deref()
                                    .and_then(parse_workspace_strategy)
                                {
                                    event = event.with_workspace_strategy(strategy);
                                }
                                if let Some(strategy) = raw
                                    .session_strategy
                                    .as_deref()
                                    .and_then(parse_session_strategy)
                                {
                                    event = event.with_session_strategy(strategy);
                                }
                                if let Some(action) =
                                    raw.turn_action.as_deref().and_then(parse_turn_action)
                                {
                                    event = event.with_turn_action(action);
                                }
                                self.ensure_event_id(&mut event);

                                // 外部事件同样写入 observer 日志，便于回放/排查（best-effort）
                                if let Err(e) = self.event_logger.log_event(0, "external", &event, None) {
                                    tracing::warn!(error = %e, "Failed to log external event");
                                }

                                self.route_event(event).await?;
                            }

                            if !parse.malformed.is_empty() {
                                tracing::warn!(
                                    malformed = parse.malformed.len(),
                                    "External events file contains malformed JSONL lines"
                                );
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                error = %e,
                                path = ?events_path,
                                "Failed to read external events file"
                            );
                        }
                    }

                    // 暂停态：冻结自动推进（例如 gate.timeout），避免“无人输入但系统又自己动起来”。
                    //
                    // 说明：
                    // - 正常情况下 completion 后不应存在 open gate；
                    // - 若确实存在，human 仍可以显式输入 `!approve/!deny/!resolve` 来推进。
                    if completion_lockdown && self.pause_on_completion_promise {
                        continue;
                    }

                    // 2) gate timeout：超时后发布 gate.timeout（后续应由决策型 job 产出 gate.resolve）
                    for timeout in self.gates.poll_timeouts() {
                        let payload = serde_json::to_string(&timeout).map_err(|e| {
                            anyhow::anyhow!("Failed to serialize GateTimeout payload: {e}")
                        })?;

                        let mut timeout_event = Event::new(ralph_proto::TOPIC_GATE_TIMEOUT, payload)
                            .with_target_instance(HatInstanceId::from_parts("ralph", "1"));
                        self.ensure_event_id(&mut timeout_event);

                        // gate.timeout 是 Supervisor 生成的事件：需要落盘，保证 replay 可追溯
                        self.event_logger
                            .log_event(0, "supervisor", &timeout_event, Some(&HatId::new("ralph")))
                            .map_err(|e| anyhow::anyhow!("Failed to log gate.timeout event: {e}"))?;

                        self.route_event(timeout_event).await?;
                    }
                }
            }
        }

        // 关闭所有实例（best-effort）。
        // 说明：即便某些实例还在跑 job，也要先发 cancel，再发 shutdown。
        self.shutdown_instances().await;

        // shutdown-drain：等待实例上报终态，避免 Supervisor 提前退出导致 instance 侧
        // `StateChanged(Done)` 发送失败，从而产生收尾 warning（Failed to send StateChanged...）。
        self.drain_shutdown(&mut output_rx, &mut instance_rx, &mut output_chunks)
            .await;

        Ok(ParallelRunResult {
            termination,
            ralph_iterations,
            instance_states: self.instance_states,
            output_chunks,
        })
    }

    fn resolve_job_timeout(&self, hat_config: Option<&HatConfig>) -> Option<Duration> {
        // 1) per-hat override（并行模式专用）
        if let Some(override_secs) = hat_config.and_then(|c| c.job_timeout_secs) {
            return (override_secs > 0).then(|| Duration::from_secs(override_secs));
        }

        // 2) 继承 adapters.<backend>.timeout（按 hat.backend 或 cli.backend 推导）
        let backend = match hat_config.and_then(|c| c.backend.as_ref()) {
            Some(HatBackend::Named(name)) => name.as_str(),
            Some(HatBackend::NamedWithArgs { backend_type, .. }) => backend_type.as_str(),
            Some(HatBackend::KiroAgent { backend_type, .. }) => backend_type.as_str(),
            // custom backend：尽可能按 command 推导到已知 adapter（避免走 fallback）
            Some(HatBackend::Custom { command, .. }) if command == "codex" => "codex",
            Some(HatBackend::Custom { .. }) => "custom",
            None => self.config.cli.backend.as_str(),
        };

        // 说明：core 层的 effective_backend() 目前不做 auto resolve（仍返回原值）。
        // 这里在并行 job timeout 上补齐一个“足够好”的 auto 解析：
        // - 按 agent_priority 顺序选择第一个 enabled 的 adapter
        // - 若找不到，则回退 claude
        let resolved_backend = if backend == "auto" {
            self.config
                .get_agent_priority()
                .into_iter()
                .find(|b| self.config.adapter_settings(b).enabled)
                .unwrap_or("claude")
        } else {
            backend
        };

        let timeout_secs = self.config.adapter_settings(resolved_backend).timeout;
        (timeout_secs > 0).then(|| Duration::from_secs(timeout_secs))
    }

    fn resolve_output_stale_timeout(&self, hat_config: Option<&HatConfig>) -> Option<Duration> {
        let backend = match hat_config.and_then(|c| c.backend.as_ref()) {
            Some(HatBackend::Named(name)) => name.as_str(),
            Some(HatBackend::NamedWithArgs { backend_type, .. }) => backend_type.as_str(),
            Some(HatBackend::KiroAgent { backend_type, .. }) => backend_type.as_str(),
            // custom backend：尽可能按 command 推导到已知 adapter（避免走 fallback）
            Some(HatBackend::Custom { command, .. }) if command == "codex" => "codex",
            Some(HatBackend::Custom { .. }) => "custom",
            None => self.config.cli.backend.as_str(),
        };

        let resolved_backend = if backend == "auto" {
            self.config
                .get_agent_priority()
                .into_iter()
                .find(|b| self.config.adapter_settings(b).enabled)
                .unwrap_or("claude")
        } else {
            backend
        };

        let secs = self
            .config
            .adapter_settings(resolved_backend)
            .output_stale_timeout_secs;
        (secs > 0).then(|| Duration::from_secs(secs))
    }

    fn spawn_instances(&mut self) -> anyhow::Result<()> {
        let output_tx = self
            .output_tx
            .clone()
            .expect("output_tx must be set before spawn_instances()");
        let instance_tx = self
            .instance_tx
            .clone()
            .expect("instance_tx must be set before spawn_instances()");

        let dynamic_idle_ttl = self.effective_dynamic_idle_ttl();
        let mut configured_lifecycle_instances = Vec::new();

        // 先注册 config 里定义的 hats
        for hat in self.registry.all() {
            let hat_id = hat.id.clone();
            let hat_config = self.registry.get_config(&hat_id).cloned();
            let instances = hat_config.as_ref().map_or(1, |c| c.instances).max(1);
            let job_timeout = self.resolve_job_timeout(hat_config.as_ref());
            let job_output_stale_timeout = self.resolve_output_stale_timeout(hat_config.as_ref());

            let mut ids = Vec::new();
            for i in 1..=instances {
                let instance_id = HatInstanceId::from_parts(hat_id.as_str(), i.to_string());
                let handle = HatInstanceHandle::spawn(
                    instance_id.clone(),
                    hat.clone(),
                    hat_config.clone(),
                    self.config.parallel.workspace.clone(),
                    self.config.parallel.permissions.clone(),
                    self.config.parallel.gate.default_timeout_secs,
                    job_timeout,
                    job_output_stale_timeout,
                    self.prompt_prelude.clone(),
                    self.all_hat_prompt.clone(),
                    Arc::clone(&self.instruction_builder),
                    Arc::clone(&self.executor),
                    output_tx.clone(),
                    instance_tx.clone(),
                    Arc::clone(&self.job_semaphore),
                    Arc::clone(&self.command_queue),
                    false,
                    dynamic_idle_ttl,
                );
                self.instance_states
                    .insert(instance_id.clone(), HatInstanceState::Created);
                self.notify_instance_state_observer(&instance_id, HatInstanceState::Created);
                configured_lifecycle_instances.push(instance_id.clone());
                self.instances.insert(instance_id.clone(), handle);
                ids.push(instance_id);
            }
            self.instances_by_hat.insert(hat_id.clone(), ids);
            self.next_instance_seq_by_hat.insert(
                hat_id,
                u64::try_from(instances).unwrap_or(1).saturating_add(1),
            );
        }

        for instance_id in configured_lifecycle_instances {
            self.log_runtime_lifecycle_created(&instance_id, false, None, Some("configured"));
        }

        // 始终注册 Ralph fallback（即使 config 没写）
        let ralph_id = HatId::new("ralph");
        let ralph_instance = HatInstanceId::from_parts("ralph", "1");
        let ralph_hat = Hat::new("ralph", "Ralph")
            .with_description(
                "Parallel coordinator: handles true-orphan events and makes completion decisions",
            )
            .subscribe("*")
            .with_instructions(self.build_ralph_coordinator_instructions(&ralph_instance));
        let ralph_job_timeout = self.resolve_job_timeout(None);
        let ralph_job_output_stale_timeout = self.resolve_output_stale_timeout(None);
        let handle = HatInstanceHandle::spawn(
            ralph_instance.clone(),
            ralph_hat,
            None::<HatConfig>,
            self.config.parallel.workspace.clone(),
            self.config.parallel.permissions.clone(),
            self.config.parallel.gate.default_timeout_secs,
            ralph_job_timeout,
            ralph_job_output_stale_timeout,
            self.prompt_prelude.clone(),
            self.all_hat_prompt.clone(),
            Arc::clone(&self.instruction_builder),
            Arc::clone(&self.executor),
            output_tx,
            instance_tx,
            Arc::clone(&self.job_semaphore),
            Arc::clone(&self.command_queue),
            false,
            dynamic_idle_ttl,
        );
        self.instance_states
            .insert(ralph_instance.clone(), HatInstanceState::Created);
        self.notify_instance_state_observer(&ralph_instance, HatInstanceState::Created);
        self.log_runtime_lifecycle_created(&ralph_instance, false, None, Some("fallback"));
        self.instances.insert(ralph_instance.clone(), handle);
        self.instances_by_hat.insert(ralph_id, vec![ralph_instance]);
        self.next_instance_seq_by_hat.insert(HatId::new("ralph"), 2);

        Ok(())
    }

    fn notify_instance_state_observer(&self, instance_id: &HatInstanceId, state: HatInstanceState) {
        if let Some(observer) = &self.instance_state_observer {
            observer(instance_id, state);
        }
    }

    fn notify_delivery_observer(&self, observation: RuntimeDeliveryObservation) {
        if let Some(observer) = &self.delivery_observer {
            observer(&observation);
        }
    }

    fn log_runtime_delivery_record(&mut self, record: RuntimeDeliveryRecord) {
        if let Err(error) = self
            .event_logger
            .log_runtime_delivery(0, "supervisor", &record)
        {
            tracing::warn!(%error, "Failed to log runtime delivery record");
        }
    }

    fn log_runtime_lifecycle_record(&mut self, record: RuntimeLifecycleRecord) {
        if let Err(error) = self
            .event_logger
            .log_runtime_lifecycle(0, "supervisor", &record)
        {
            tracing::warn!(%error, "Failed to log runtime lifecycle record");
        }
    }

    fn log_runtime_lifecycle_created(
        &mut self,
        instance_id: &HatInstanceId,
        dynamic: bool,
        source_event_id: Option<String>,
        reason: Option<&str>,
    ) {
        let kind = if dynamic {
            RuntimeLifecycleKind::Spawn
        } else {
            RuntimeLifecycleKind::Create
        };
        let mut record = RuntimeLifecycleRecord::new(instance_id.clone(), kind)
            .with_state(HatInstanceState::Created)
            .with_dynamic(dynamic);

        if let Some(source_event_id) = source_event_id {
            record = record.with_source_event_id(source_event_id);
        }
        if let Some(reason) = reason {
            record = record.with_reason(reason);
        }

        self.log_runtime_lifecycle_record(record);
    }

    fn log_runtime_lifecycle_state(
        &mut self,
        instance_id: &HatInstanceId,
        state: HatInstanceState,
    ) {
        let record = RuntimeLifecycleRecord::new(instance_id.clone(), RuntimeLifecycleKind::State)
            .with_state(state);
        self.log_runtime_lifecycle_record(record);
    }

    fn log_runtime_lifecycle_control(
        &mut self,
        instance_id: &HatInstanceId,
        kind: RuntimeLifecycleKind,
        reason: &str,
    ) {
        let record =
            RuntimeLifecycleRecord::new(instance_id.clone(), kind).with_reason(reason.to_string());
        self.log_runtime_lifecycle_record(record);
    }

    fn build_ralph_coordinator_instructions(&self, instance_id: &HatInstanceId) -> String {
        // =====================================================================
        // Ralph#1（并行协调者）prompt：把“官方语义锚点”写死，减少 demo prompt 依赖
        // =====================================================================
        //
        // 目标：
        // - 让 parallel 模式的 Ralph#1 拥有接近 HatlessRalph 的“强约束、可预测协调语义”
        // - 只把顶层 prompt 注入给 Ralph（避免 prompt pollution）
        // - 明确 starting_event / complete_publishes 的语义与使用方式

        let completion_promise = self.config.event_loop.completion_promise.as_str();

        let starting_event = self.config.event_loop.starting_event.as_deref();
        let complete_publishes = self.config.event_loop.complete_publishes.as_deref();

        // 生成一个稳定的 hats 拓扑表（只包含用户配置 hats；不包含 ralph 自己）
        let mut hats: Vec<_> = self.registry.all().collect();
        hats.sort_by(|a, b| a.id.as_str().cmp(b.id.as_str()));

        // 入口 topic 候选（用于 starting_event 未配置时的兜底选择）
        // 粗略规则：
        // - 只考虑“精确 topic”（不含 `*` 的订阅）
        // - 排除 task.start/task.resume（控制面 handshake）
        // - 排除已被某个 hat publishes 的 topic（更像是链路中间态）
        let mut published_exact: HashSet<String> = HashSet::new();
        for hat in &hats {
            for t in &hat.publishes {
                let s = t.as_str();
                if !s.contains('*') {
                    published_exact.insert(s.to_string());
                }
            }
        }

        let mut entry_candidates: Vec<String> = Vec::new();
        for hat in &hats {
            for t in &hat.subscriptions {
                let s = t.as_str();
                if s.contains('*') {
                    continue;
                }
                if matches!(s, "task.start" | "task.resume") {
                    continue;
                }
                if published_exact.contains(s) {
                    continue;
                }
                entry_candidates.push(s.to_string());
            }
        }
        entry_candidates.sort();
        entry_candidates.dedup();

        let mut out = String::new();

        out.push_str(&format!(
            "You are Ralph (coordinator) running in PARALLEL mode as instance {instance_id}.\n\n"
        ));

        out.push_str("## ROLE\n");
        out.push_str("- You MUST NOT implement code.\n");
        out.push_str("- You MUST coordinate by publishing events.\n");
        out.push_str(
	            "- You MAY publish events in-band (`<event ...>...</event>`) or out-of-band via `ralph emit` when command execution is available.\n",
	        );
        out.push_str("- You MUST keep output short and action-oriented.\n\n");

        if let Some(catalog) = render_parent_capability_catalog(&self.runtime_capability_catalog) {
            out.push_str(&catalog);
            out.push_str("\n");
        }

        // =====================================================================
        // Ralph-only prompt 注入（config: event_loop.ralph_prompt）
        // - 只给 ralph#1（协调者），避免污染其他 hats
        // - 内容仅为空白时跳过，避免生成空段落
        // =====================================================================
        if let Some(extra) = self.config.event_loop.ralph_prompt.as_deref()
            && !extra.trim().is_empty()
        {
            out.push_str("## RALPH PROMPT (CONFIG)\n");
            out.push_str(extra);
            out.push_str("\n\n");
        }

        out.push_str("## KEY SEMANTICS (OFFICIAL)\n");
        out.push_str("- Runtime handshake start topics are always: `task.start` (fresh) / `task.resume` (resume).\n");
        out.push_str("- `event_loop.starting_event` is an OPTIONAL workflow entry event after coordination.\n");
        out.push_str("  - If set: you MUST publish it as the workflow entry topic.\n");
        out.push_str("  - If not set: you MUST decide a workflow entry topic from the hats topology (prefer derived candidates).\n");
        out.push_str("- `event_loop.complete_publishes` is an OPTIONAL workflow completion candidate event topic.\n");
        out.push_str(&format!(
            "- The ONLY hard shutdown signal is the completion promise: `{completion_promise}`.\n\n"
        ));

        let coordinator_topics = self.coordinator_publish_topics();
        out.push_str(&event_emission_protocol::render_event_emission_protocol(
            coordinator_topics.iter().map(String::as_str),
        ));
        out.push_str("\n");
        out.push_str("## OUT-OF-BAND EVENT INJECTION\n");
        out.push_str("- If you can execute shell/tool commands, you MAY run `ralph emit` for out-of-band event injection.\n");
        out.push_str("- If you choose out-of-band `ralph emit`, you MUST actually execute the command. Do NOT print the command as plain text.\n");
        out.push_str("- Out-of-band `ralph emit` does NOT require the current job/turn to finish; it can be used at any time, including in-flight steer/interrupt.\n");
        out.push_str("- If you cannot execute commands, use the in-band event protocol above.\n");
        out.push_str("- Example command: `ralph emit human.message \"your message\" --target-instance writer#1`.\n\n");
        out.push_str("After publishing the required event(s), you MUST stop. The supervisor will route them and run the next job with fresh context.\n\n");

        out.push_str("## HUMAN CHAT (INPUT VS REPLY)\n");
        out.push_str("- `human.message` is an input message (human -> hats) OR a message addressed to a hat instance.\n");
        out.push_str("- If you want to reply back to the human user, you MUST publish `reply.human.message`.\n");
        out.push_str("  - Example: <event topic=\"reply.human.message\" reply=\"EVENT_ID\">your reply</event>\n");
        out.push_str("- When you observe an external `human.message` event (no `source`/`source_instance`), you MUST reply to the human.\n");
        out.push_str("  - Emit exactly one `reply.human.message` event with `reply=\"<incoming_event.id>\"`.\n");
        out.push_str("- If you want to return an answer to another hat that asked you something, you MUST publish `reply.hat.message`.\n");
        out.push_str("  - Example: <event topic=\"reply.hat.message\" reply=\"EVENT_ID\">your answer</event>\n");
        out.push_str("  - Runtime will route it back to the original requester instance from the referenced event.\n");
        out.push_str("  - Use `reply.hat.message` only for hat-to-hat answer return, not for normal workflow progression.\n");
        out.push_str("- You MUST NOT use `human.message` as a human-facing reply topic.\n");
        out.push_str("  - Reason: `human.message` is treated as an input topic and can cause self-chat loops in parallel mode.\n\n");

        out.push_str("## CONFIG (THIS RUN)\n");
        match starting_event {
            Some(topic) => {
                out.push_str(&format!("- starting_event: `{topic}`\n"));
            }
            None => {
                out.push_str("- starting_event: (not set)\n");
            }
        }
        match complete_publishes {
            Some(topic) => {
                out.push_str(&format!("- complete_publishes: `{topic}`\n"));
            }
            None => {
                out.push_str("- complete_publishes: (not set)\n");
            }
        }
        out.push('\n');

        out.push_str("## HATS TOPOLOGY (CONFIGURED)\n");
        if hats.is_empty() {
            out.push_str("- (no custom hats configured)\n\n");
        } else {
            out.push_str("| hat_id | triggers | publishes | description |\n");
            out.push_str("|--------|----------|-----------|-------------|\n");
            for hat in &hats {
                let triggers = hat
                    .subscriptions
                    .iter()
                    .map(|t| t.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                let publishes = hat
                    .publishes
                    .iter()
                    .map(|t| t.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                out.push_str(&format!(
                    "| {} | {} | {} | {} |\n",
                    hat.id.as_str(),
                    triggers,
                    publishes,
                    hat.description.replace('\n', " ")
                ));
            }
            out.push('\n');
        }

        out.push_str("## WHAT TO DO\n");
        out.push_str("### If you receive `task.start` (fresh)\n");
        if let Some(topic) = starting_event {
            out.push_str(&format!(
                "1) Do initial coordination quickly.\n2) Emit EXACTLY ONE workflow entry event with topic `{topic}`.\n3) Stop. Do NOT output the completion promise.\n\n"
            ));
        } else if !entry_candidates.is_empty() {
            out.push_str("1) Do initial coordination quickly.\n");
            out.push_str(
                "2) Choose ONE workflow entry topic (prefer an external entrypoint topic).\n",
            );
            out.push_str("   Candidates (derived):\n");
            for t in &entry_candidates {
                out.push_str(&format!("   - `{t}`\n"));
            }
            out.push_str("3) Emit EXACTLY ONE workflow entry event.\n");
            out.push_str("4) Stop. Do NOT output the completion promise.\n\n");
        } else {
            out.push_str("1) Do initial coordination quickly.\n");
            out.push_str("2) Choose ONE workflow entry event topic from the hats table above.\n");
            out.push_str("3) Emit EXACTLY ONE workflow entry event.\n");
            out.push_str("4) Stop. Do NOT output the completion promise.\n\n");
        }

        out.push_str("### If you receive the completion candidate topic\n");
        if let Some(topic) = complete_publishes {
            out.push_str(&format!(
                "When you observe an event with topic `{topic}`:\n- If the workflow is truly complete, output `{completion_promise}` on its own line and stop.\n- Otherwise, emit follow-up events and stop.\n\n"
            ));
        } else {
            out.push_str(
                "No completion candidate is configured.\n- You may still output the completion promise when the objective is complete.\n- Prefer emitting follow-up events when more work is needed.\n\n",
            );
        }

        out.push_str("### If you receive any other event\n");
        out.push_str("- Treat it as an orphan (no subscribers) or an explicitly targeted control-plane event.\n");
        out.push_str("- Decide which hat should handle it next and emit ONE event to delegate.\n");
        out.push_str("- Stop.\n");

        out
    }

    async fn route_events_batch(&mut self, events: Vec<Event>) -> anyhow::Result<()> {
        self.routing_batch_depth = self.routing_batch_depth.saturating_add(1);
        if self.routing_batch_depth == 1 {
            self.routing_inflight_instances.clear();
        }

        for event in events {
            self.route_event(event).await?;
        }

        self.routing_batch_depth = self.routing_batch_depth.saturating_sub(1);
        if self.routing_batch_depth == 0 {
            self.routing_inflight_instances.clear();
        }

        Ok(())
    }

    fn unregister_dynamic_instance(&mut self, instance_id: &HatInstanceId) {
        self.instances.remove(instance_id);
        self.dynamic_instances.remove(instance_id);
        self.agents_last_inputs.remove(instance_id);

        // 从 hat -> instances 索引里移除，避免后续路由继续选中该实例。
        if let Some(hat_id_str) = instance_id.split_hat_id() {
            let hat_id = HatId::new(hat_id_str);
            if let Some(list) = self.instances_by_hat.get_mut(&hat_id) {
                list.retain(|id| id != instance_id);
            }
        }
    }

    // =====================================================================
    // `.ralph/agents.json` 快照写入(供 `ralph agents`)
    // =====================================================================
    //
    // 说明：
    // - 该快照是“观察面”功能,不应该影响核心调度语义。
    // - 因此所有写盘都采用 best-effort：失败只 warn,不让 Supervisor 退出。

    fn record_agents_last_input(&mut self, instance_id: &HatInstanceId, event: &Event) {
        // 未启用快照时,不做任何额外工作.
        if self.agents_snapshot_path.is_none() {
            return;
        }

        let topic = event.topic.to_string();

        // 预览策略：
        // - 把 payload 压成单行(便于表格展示)
        // - 只截断到固定长度,避免把大块 prompt 落盘
        let collapsed = event
            .payload
            .replace('\r', " ")
            .replace('\n', " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let preview = crate::truncate_with_ellipsis(&collapsed, 160);

        self.agents_last_inputs.insert(
            instance_id.clone(),
            AgentLastInput {
                ts: chrono::Utc::now().to_rfc3339(),
                topic,
                preview,
            },
        );
    }

    fn build_agents_snapshot(&self) -> AgentsSnapshot {
        let mut instance_ids: Vec<HatInstanceId> = self.instances.keys().cloned().collect();
        instance_ids.sort_by(|a, b| a.as_str().cmp(b.as_str()));

        let instances = instance_ids
            .into_iter()
            .map(|instance_id| {
                let hat_id = instance_id
                    .split_hat_id()
                    .unwrap_or("<unknown>")
                    .to_string();
                let state = self
                    .instance_states
                    .get(&instance_id)
                    .copied()
                    .unwrap_or(HatInstanceState::Created);
                let is_dynamic = self.dynamic_instances.contains(&instance_id);
                let last_input = self.agents_last_inputs.get(&instance_id).cloned();

                AgentInstanceSnapshot {
                    instance_id: instance_id.to_string(),
                    hat_id,
                    state,
                    is_dynamic,
                    last_input,
                }
            })
            .collect::<Vec<_>>();

        AgentsSnapshot {
            generated_at: chrono::Utc::now().to_rfc3339(),
            instances,
        }
    }

    fn write_agents_snapshot_best_effort(&self) {
        let Some(path) = &self.agents_snapshot_path else {
            return;
        };

        if let Err(e) = self.write_agents_snapshot_to_path(path) {
            tracing::warn!(error = %e, path = ?path, "Failed to write agents snapshot");
        }
    }

    fn write_agents_snapshot_to_path(&self, path: &std::path::Path) -> anyhow::Result<()> {
        let snapshot = self.build_agents_snapshot();
        let json = serde_json::to_string_pretty(&snapshot)
            .context("Failed to serialize AgentsSnapshot as JSON")?;

        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent).with_context(|| {
                format!(
                    "Failed to create agents snapshot directory: {}",
                    parent.display()
                )
            })?;
        }

        // 原子写入(尽量)：先写 tmp,再 rename 覆盖.
        let tmp_path = path.with_extension("json.tmp");
        std::fs::write(&tmp_path, json).with_context(|| {
            format!(
                "Failed to write tmp agents snapshot: {}",
                tmp_path.display()
            )
        })?;

        // Windows 上 rename 到已存在文件可能失败,这里做一次 remove+retry 的兼容兜底.
        match std::fs::rename(&tmp_path, path) {
            Ok(()) => Ok(()),
            Err(e) => {
                let _ = std::fs::remove_file(path);
                std::fs::rename(&tmp_path, path).with_context(|| {
                    format!(
                        "Failed to rename tmp agents snapshot to final: {} -> {} ({e})",
                        tmp_path.display(),
                        path.display()
                    )
                })?;
                Ok(())
            }
        }
    }

    async fn shutdown_instances(&mut self) {
        let targets: Vec<(HatInstanceId, mpsc::Sender<HatInstanceCommand>)> = self
            .instances
            .iter()
            .map(|(instance_id, handle)| (instance_id.clone(), handle.cmd_tx.clone()))
            .collect();

        for (instance_id, cmd_tx) in targets {
            self.log_runtime_lifecycle_control(
                &instance_id,
                RuntimeLifecycleKind::Cancel,
                "supervisor_shutdown",
            );
            let _ = cmd_tx.send(HatInstanceCommand::CancelCurrentJob).await;
            self.log_runtime_lifecycle_control(
                &instance_id,
                RuntimeLifecycleKind::Shutdown,
                "supervisor_shutdown",
            );
            let _ = cmd_tx.send(HatInstanceCommand::Shutdown).await;
        }
    }

    fn freeze_pending_on_all_instances(&mut self) {
        let instance_ids: Vec<HatInstanceId> = self.instances.keys().cloned().collect();
        for instance_id in instance_ids {
            if let Some(handle) = self.instances.get(&instance_id) {
                handle.request_completion_freeze();
            }
            self.log_runtime_lifecycle_control(
                &instance_id,
                RuntimeLifecycleKind::Freeze,
                "completion_promise",
            );
        }
    }

    /// Supervisor 退出前的“收尾 drain”：
    ///
    /// 目标：
    /// - 让各个 HatInstance 有机会把 `StateChanged(Done/Failed)` 发回 Supervisor；
    /// - 避免 Supervisor 先 drop receiver，导致 instance 侧 send 失败并打出 warning；
    /// - 同时让 `final states` 快照更可信（尽量收敛到终态）。
    async fn drain_shutdown(
        &mut self,
        output_rx: &mut mpsc::Receiver<HatJobOutputChunk>,
        instance_rx: &mut mpsc::Receiver<HatInstanceEvent>,
        output_chunks: &mut usize,
    ) {
        // 经验值：Shutdown 是“控制面”信号，instance actor 应当能很快处理并退出。
        // 这里给一个小窗口即可，避免无人值守时卡在收尾阶段。
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        let mut output_closed = false;
        let mut instance_closed = false;

        loop {
            if self.all_instances_in_terminal_state() {
                break;
            }
            if output_closed && instance_closed {
                break;
            }

            tokio::select! {
                _ = tokio::time::sleep_until(deadline) => break,
                chunk = output_rx.recv(), if !output_closed => {
                    match chunk {
                        Some(chunk) => {
                            *output_chunks += 1;
                            if let Some(observer) = &self.output_observer {
                                observer(&chunk);
                            }
                        }
                        None => output_closed = true,
                    }
                }
                msg = instance_rx.recv(), if !instance_closed => {
                    match msg {
                        Some(msg) => match msg {
                            HatInstanceEvent::StateChanged { instance_id, state } => {
                                self.instance_states.insert(instance_id.clone(), state);
                                if let Some(observer) = &self.instance_state_observer {
                                    observer(&instance_id, state);
                                }

                                // 动态实例进入 Done 后会被移出“可投递 registry”。
                                if state == HatInstanceState::Done
                                    && self.dynamic_instances.contains(&instance_id)
                                {
                                    self.unregister_dynamic_instance(&instance_id);
                                }
                            }
                            // 收尾阶段只关心状态收敛。JobCompleted/Published 等事件不再继续路由，
                            // 避免“已决定退出却又被新事件拉起”。
                            HatInstanceEvent::JobCompleted { .. } | HatInstanceEvent::Published { .. } => {}
                        },
                        None => instance_closed = true,
                    }
                }
            }
        }
    }

    fn all_instances_in_terminal_state(&self) -> bool {
        self.instances.keys().all(|instance_id| {
            matches!(
                self.instance_states.get(instance_id),
                Some(HatInstanceState::Done | HatInstanceState::Failed)
            )
        })
    }
}

fn parse_workspace_strategy(raw: &str) -> Option<WorkspaceStrategy> {
    match raw.trim() {
        "shared" => Some(WorkspaceStrategy::Shared),
        "patch" => Some(WorkspaceStrategy::Patch),
        "worktree" => Some(WorkspaceStrategy::Worktree),
        _ => None,
    }
}

fn parse_session_strategy(raw: &str) -> Option<SessionStrategy> {
    match raw.trim() {
        "exec" => Some(SessionStrategy::Exec),
        "mcp" => Some(SessionStrategy::Mcp),
        "app_server" => Some(SessionStrategy::AppServer),
        _ => None,
    }
}

fn parse_turn_action(raw: &str) -> Option<TurnAction> {
    match raw.trim() {
        "start" => Some(TurnAction::Start),
        "steer" => Some(TurnAction::Steer),
        "interrupt" => Some(TurnAction::Interrupt),
        _ => None,
    }
}
