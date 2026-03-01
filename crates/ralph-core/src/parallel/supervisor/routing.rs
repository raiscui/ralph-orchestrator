//! 3.x 事件路由实现（TopicContract / audience / missing / queue selection）。
//!
//! 说明：
//! - 本文件位于 `supervisor` 子模块内，便于访问 `ParallelSupervisor` 的私有字段。
//! - 目标是把并行模式的路由语义做成“可依赖 + 可回放”的机械规则：
//!   - recipients = `TopicContract.audience ∩ Event.audience_override`（如果有）
//!   - queue：支持 deterministic / llm，并强制 `dispatch.decision` 落盘
//!   - missing：best-effort + require_delivery 分支清晰，避免 silent reroute

use super::super::{HatInstanceCommand, HatInstanceHandle, HatJob, JobBackend};
use super::ParallelSupervisor;
use crate::EventParser;
use crate::config::HatConfig;
use crate::event_logger::EventHistory;
use crate::prompt_overlay;
use anyhow::Context;
use ralph_proto::{
    Delivery, Event, GateRequest, GateResolve, Hat, HatId, HatInstanceId, HatInstanceState,
    MissingInstancePolicy, QueueDecisionRecord, QueueSelection, SessionStrategy,
    TOPIC_DISPATCH_DECISION, TOPIC_GATE_REQUEST, TOPIC_GATE_RESOLVE, TOPIC_GATE_TIMEOUT, Topic,
    TopicContract, TurnAction, new_event_id,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, watch};

impl ParallelSupervisor {
    pub(super) async fn route_event(&mut self, event: Event) -> anyhow::Result<()> {
        let mut event = event;
        self.ensure_event_id(&mut event);

        // ---------------------------------------------------------------------
        // reply.human.message 语义护栏(并行 TUI chat 输出):
        // - `reply.human.message` 表示 hat -> human 的“回复消息”(输出)。
        // - 该 topic 的唯一目的应是 UI 展示/日志证据,而不是再次参与路由。
        // - 否则 ralph(订阅 "*") 会再次收到它,形成自问自答循环。
        // ---------------------------------------------------------------------
        if event.topic.as_str() == "reply.human.message" {
            if let Some(observer) = &self.event_observer {
                observer(&event);
            }
            return Ok(());
        }

        // ---------------------------------------------------------------------
        // human.message 语义护栏(并行 TUI idle chat):
        // - `human.message` 是“外部输入事件”(human -> hats)。
        // - 如果某个 hat 反向发布 `human.message`(带 source/source_instance),
        //   该事件应只用于 UI 展示,不应再次参与路由。
        // - 否则会形成“ralph#1 回复 -> 事件被路由回 ralph -> ralph#1 再回复”的自我对话回路。
        // ---------------------------------------------------------------------
        if event.topic.as_str() == "human.message"
            && (event.source.is_some() || event.source_instance.is_some())
        {
            if let Some(observer) = &self.event_observer {
                observer(&event);
            }
            return Ok(());
        }

        // ---------------------------------------------------------------------
        // Ralph 双实例路由补偿:
        // - 如果事件显式指向 ralph#1,但它当前正在 Running,
        //   则自动改投 ralph#2(按需创建)。
        // - 这样可避免“协调面事件全部堵在 ralph#1”。
        // ---------------------------------------------------------------------
        self.rewrite_target_for_busy_ralph(&mut event)?;

        // 内部“纯记录”事件：dispatch.decision 不参与业务路由，只用于 replay/观测。
        if event.topic.as_str() == TOPIC_DISPATCH_DECISION {
            self.on_dispatch_decision_record(&event);
            return Ok(());
        }

        // gate.request / gate.resolve：先更新本地 gate 状态机（不依赖路由配置）
        match event.topic.as_str() {
            TOPIC_GATE_REQUEST => {
                if let Ok(request) = serde_json::from_str::<GateRequest>(&event.payload) {
                    let first = self.gates.register(request)?;
                    if !first {
                        tracing::warn!("duplicate gate.request ignored");
                    }
                } else {
                    tracing::warn!(
                        "gate.request payload is not valid JSON; ignored by GateManager"
                    );
                }
            }
            TOPIC_GATE_RESOLVE => {
                if let Ok(resolve) = serde_json::from_str::<GateResolve>(&event.payload) {
                    // gate.resolve 优先回送给发起者（requested_by）
                    if let Some(requested_by) = self.gates.resolve(&resolve) {
                        event.target_instance = Some(requested_by);
                        event.target = None;
                    }
                } else {
                    tracing::warn!(
                        "gate.resolve payload is not valid JSON; ignored by GateManager"
                    );
                }
            }
            _ => {}
        }

        // =====================================================================
        // TUI/观测：把事件推送给上层（例如并行 Supervisor TUI 的 gate 面板）。
        // =====================================================================
        if let Some(observer) = &self.event_observer {
            observer(&event);
        }

        // =========================================================================
        // 2.3：strict target 校验（parallel-trigger-routing）
        // =========================================================================
        //
        // 说明：
        // - event.target / event.target_instance 是“收敛语义”，不允许绕过订阅拓扑任意投递。
        // - 对少数控制面 topic 做特例（例如 gate.*），避免把运行时信号误判为非法。
        let control_plane = Self::is_control_plane_topic(event.topic.as_str());

        // =====================================================================
        // spawn_instance：显式请求“新实例投递”(上下文隔离)
        // =====================================================================
        //
        // 说明：
        // - 这是一个“路由提示信号”，用于实现消息的 3 种投递模式之一：new_instance。
        // - 语义约束：
        //   - 与 target_instance 互斥。
        //   - 推荐必须同时提供 target(目标 hat)，否则无法确定为哪个 hat 开新实例。
        //
        // 策略：
        // - 成功: 创建动态实例并把 event.target_instance 指向它(直达投递)。
        // - 失败: best-effort escalate + 降级为普通路由(避免丢消息)。
        if matches!(event.spawn_instance, Some(true)) {
            // 清空该字段: 不应该进入 LLM prompt 的业务事件列表.
            event.spawn_instance = None;

            if event.target_instance.is_some() {
                self.escalate_delivery_failure(
                    &event,
                    "invalid spawn_instance: target_instance is already set (mutually exclusive)"
                        .to_string(),
                    &[],
                    &[],
                )
                .await?;
            } else if let Some(target_hat) = event.target.clone() {
                // 对非控制面 topic，spawn 之前先做一次订阅校验，避免“先 spawn 再被 strict target 拒绝”。
                if !control_plane && !self.hat_is_subscriber(&target_hat, &event.topic) {
                    self.escalate_delivery_failure(
                        &event,
                        format!(
                            "invalid spawn_instance: hat \"{}\" is not subscribed to topic \"{}\"",
                            target_hat, event.topic
                        ),
                        &[],
                        &[],
                    )
                    .await?;
                } else if target_hat.as_str() == "ralph" {
                    // ralph 有专门的双实例路由补偿,不再额外支持显式 spawn。
                    self.escalate_delivery_failure(
                        &event,
                        "invalid spawn_instance: explicit spawn is not supported for hat \"ralph\""
                            .to_string(),
                        &[],
                        &[],
                    )
                    .await?;
                } else {
                    match self.spawn_dynamic_instance(&target_hat) {
                        Ok(instance_id) => {
                            event.target_instance = Some(instance_id);
                        }
                        Err(e) => {
                            self.escalate_delivery_failure(
                                &event,
                                format!("spawn_instance failed: {e}"),
                                &[],
                                &[],
                            )
                            .await?;
                        }
                    }
                }
            } else {
                self.escalate_delivery_failure(
                    &event,
                    "invalid spawn_instance: missing target hat id; add target=\"<hat_id>\""
                        .to_string(),
                    &[],
                    &[],
                )
                .await?;
            }
        }

        if !control_plane {
            if let Some(target_hat) = &event.target
                && !self.hat_is_subscriber(target_hat, &event.topic)
            {
                tracing::warn!(
                    event_id = %event.id.as_deref().unwrap_or("<none>"),
                    topic = %event.topic,
                    target = %target_hat,
                    "event.target is not subscribed to topic; rejected"
                );
                self.escalate_delivery_failure(
                    &event,
                    format!(
                        "invalid target: hat \"{}\" is not subscribed to topic \"{}\"",
                        target_hat, event.topic
                    ),
                    &[],
                    &[],
                )
                .await?;
                return Ok(());
            }

            if let Some(target_instance) = &event.target_instance {
                if !self.instances.contains_key(target_instance) {
                    tracing::warn!(
                        event_id = %event.id.as_deref().unwrap_or("<none>"),
                        topic = %event.topic,
                        target_instance = %target_instance,
                        "event.target_instance does not exist; rejected"
                    );
                    self.escalate_delivery_failure(
                        &event,
                        format!(
                            "invalid target_instance: instance \"{}\" does not exist",
                            target_instance
                        ),
                        &[],
                        &[],
                    )
                    .await?;
                    return Ok(());
                }

                let Some(hat_id_str) = target_instance.split_hat_id() else {
                    self.escalate_delivery_failure(
                        &event,
                        format!(
                            "invalid target_instance: \"{}\" is missing hat prefix (expected {{hat}}#{{key}})",
                            target_instance
                        ),
                        &[],
                        &[],
                    )
                    .await?;
                    return Ok(());
                };

                let hat_id = HatId::new(hat_id_str);
                if !self.hat_is_subscriber(&hat_id, &event.topic) {
                    tracing::warn!(
                        event_id = %event.id.as_deref().unwrap_or("<none>"),
                        topic = %event.topic,
                        target_instance = %target_instance,
                        hat = %hat_id,
                        "event.target_instance owner hat is not subscribed to topic; rejected"
                    );
                    self.escalate_delivery_failure(
                        &event,
                        format!(
                            "invalid target_instance: hat \"{}\" is not subscribed to topic \"{}\"",
                            hat_id, event.topic
                        ),
                        &[],
                        &[],
                    )
                    .await?;
                    return Ok(());
                }
            }
        }

        // target_instance：实例直达（优先级最高）
        //
        // 说明：
        // - 直达投递不应该被 TopicContract 配置“意外阻断”（尤其是 gate.resolve 回送）
        // - strict 校验失败已在上游拦住；这里假设 instance 存在（否则直接 fail-fast）
        if let Some(target_instance) = event.target_instance.clone() {
            // 可观测性: 记录“该实例最近一次收到的输入”(best-effort).
            if self.instances.contains_key(&target_instance) {
                self.record_agents_last_input(&target_instance, &event);
            }

            let Some(handle) = self.instances.get(&target_instance) else {
                self.escalate_delivery_failure(
                    &event,
                    format!(
                        "target_instance delivery failed: instance \"{}\" not found",
                        target_instance
                    ),
                    &[],
                    &[],
                )
                .await?;
                return Ok(());
            };
            handle
                .cmd_tx
                .send(HatInstanceCommand::Deliver(Box::new(event)))
                .await
                .context("Failed to deliver event to target_instance")?;
            self.mark_inflight_delivery(&target_instance);
            self.write_agents_snapshot_best_effort();
            return Ok(());
        }

        // =========================================================================
        // 2.2/2.4：TopicContract 可选覆盖 + triggers 默认 fanout（到 hat）
        // =========================================================================
        let topic_str = event.topic.to_string();
        let contract = self.contracts.resolve(topic_str.as_str()).ok().cloned();
        let Some(contract) = contract else {
            return self.route_event_via_triggers(event).await;
        };

        // 2) 计算 base audience（来自 TopicContract）
        let mut base_desired = self.contract_audience_instances(&contract);

        // 3.1：TopicContract 需要显式 audience（否则会变成“隐式 broadcast/隐式 none”）
        if base_desired.is_empty() {
            anyhow::bail!(
                "TopicContract.audience 为空：topic=\"{topic_str}\"。并行模式要求显式配置 audience（instances / instance_prefixes / hats 至少一个非空）。"
            );
        }

        // 2.1) target hat：进一步收缩（只投递到指定 hat）
        if let Some(target_hat) = &event.target {
            base_desired.retain(|id| id.split_hat_id() == Some(target_hat.as_str()));
        }

        // 基于 base_desired 计算“现有可投递候选”
        let base_existing: Vec<HatInstanceId> = base_desired
            .iter()
            .filter(|id| self.instances.contains_key(*id))
            .cloned()
            .collect();

        // 3) 计算最终 recipients（TopicContract.audience ∩ Event.audience_override）
        let (mut recipients, missing_in_base, missing_outside_base, require_delivery) =
            self.apply_audience_override(&event, &base_desired, &base_existing);

        // 3.4：require_delivery=true：任何缺失都要 escalate（不允许静默 reroute）
        if require_delivery && (!missing_in_base.is_empty() || !missing_outside_base.is_empty()) {
            self.escalate_delivery_failure(
                &event,
                "delivery failure (require_delivery)".to_string(),
                &missing_in_base,
                &missing_outside_base,
            )
            .await?;
            return Ok(());
        }

        if !missing_outside_base.is_empty() {
            tracing::warn!(
                event_id = %event.id.as_deref().unwrap_or("<none>"),
                topic = %event.topic,
                missing = ?missing_outside_base,
                "audience_override 指向了不属于 TopicContract.audience 的实例：best-effort 下会忽略这些实例（require_delivery=true 时会 escalate）"
            );
        }

        // 3.3：best-effort：missing 时按 missing_instance_policy 处理
        recipients = self
            .apply_missing_instance_policy(
                &contract,
                &event,
                recipients,
                &base_existing,
                &missing_in_base,
            )
            .await?;

        // missing_instance_policy 可能会让 recipients 为空：
        // - drop/escalate：直接停止，不进入 delivery
        // - queue/spawn：若仍为空，则认为“无法投递”，交给 escalation 兜底（不算 silent）
        if recipients.is_empty() {
            match contract.missing_instance_policy {
                MissingInstancePolicy::Drop | MissingInstancePolicy::Escalate => return Ok(()),
                MissingInstancePolicy::Spawn | MissingInstancePolicy::Queue => {
                    self.escalate_delivery_failure(
                        &event,
                        "delivery failure (empty recipients)".to_string(),
                        &[],
                        &[],
                    )
                    .await?;
                    return Ok(());
                }
            }
        }

        // 4) delivery 语义
        match contract.delivery {
            Delivery::Fanout => self.deliver_fanout(event, &recipients).await?,
            Delivery::Queue => {
                self.deliver_queue(event, topic_str.as_str(), &contract, &recipients)
                    .await?
            }
        }

        Ok(())
    }

    async fn route_event_via_triggers(&mut self, event: Event) -> anyhow::Result<()> {
        let topic = event.topic.clone();

        // 有明确 target 时，直接收敛到该 hat（2.3 已做 strict 校验）。
        let recipient_hats = if let Some(target_hat) = &event.target {
            vec![target_hat.clone()]
        } else {
            self.trigger_subscriber_hats(&topic)
        };

        if recipient_hats.is_empty() {
            return Ok(());
        }

        let mut chosen_instances: Vec<HatInstanceId> = Vec::new();

        for hat_id in recipient_hats {
            // -----------------------------------------------------------------
            // Ralph 专用实例选择策略:
            // - ralph#1 非 Running: 一律优先 ralph#1
            // - ralph#1 Running: 尝试切给 ralph#2(按需创建)
            // - 两个都 Running: 回退 ralph#1(由实例内 pending 队列吸收)
            // -----------------------------------------------------------------
            if hat_id.as_str() == "ralph" {
                chosen_instances.push(self.choose_ralph_instance_for_delivery()?);
                continue;
            }

            let Some(instances) = self.instances_by_hat.get(&hat_id) else {
                tracing::warn!(
                    event_id = %event.id.as_deref().unwrap_or("<none>"),
                    topic = %event.topic,
                    hat = %hat_id,
                    "Trigger-driven routing found hat but no instances were registered"
                );
                continue;
            };

            let mut candidates = instances.clone();
            candidates.sort_by(|a, b| a.as_str().cmp(b.as_str()));

            let has_idle_or_created = candidates.iter().any(|id| {
                matches!(
                    self.effective_state(id),
                    HatInstanceState::Idle | HatInstanceState::Created
                )
            });

            // 3.3：autoscale
            // - hat 全忙（无 Idle/Created）
            // - 全局并发未达上限（permit 还有余量）
            // => 创建动态实例并把事件投递给它
            let chosen = if !has_idle_or_created
                && hat_id.as_str() != "ralph"
                && self.available_permits_for_routing() > 0
            {
                match self.spawn_dynamic_instance(&hat_id) {
                    Ok(new_instance) => new_instance,
                    Err(e) => {
                        tracing::warn!(
                            event_id = %event.id.as_deref().unwrap_or("<none>"),
                            topic = %event.topic,
                            hat = %hat_id,
                            error = %e,
                            "Autoscale spawn failed; falling back to deterministic selection"
                        );
                        self.choose_deterministic(event.topic.as_str(), &candidates)
                    }
                }
            } else {
                self.choose_deterministic(event.topic.as_str(), &candidates)
            };
            chosen_instances.push(chosen);
        }

        // 2.4：hat-level fanout / instance-level queue
        // - 每个 hat 只选择一个实例执行
        // - 最终以“实例列表”的 fanout 形式投递
        chosen_instances.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        chosen_instances.dedup();

        self.deliver_fanout(event, &chosen_instances).await
    }

    fn trigger_subscriber_hats(&self, topic: &Topic) -> Vec<HatId> {
        // 对齐顺序模式 EventBus 的语义：specific subscriptions > global wildcard fallback
        let mut specific: Vec<HatId> = Vec::new();
        let mut fallback: Vec<HatId> = Vec::new();

        for hat in self.registry.all() {
            if hat.has_specific_subscription(topic) {
                specific.push(hat.id.clone());
            } else if hat.is_subscribed(topic) {
                fallback.push(hat.id.clone());
            }
        }

        // 链式拓扑（老板兜底）语义：
        // - 若存在 specific subscriber：只投递给 specific（不打扰 wildcard/老板）
        // - 若无 specific 但存在 wildcard subscriber（例如经理）：只投递给 wildcard（不额外打扰老板）
        // - 若完全无人订阅：才视为 orphan，升级给 ralph#1
        let mut chosen = if !specific.is_empty() {
            specific
        } else if !fallback.is_empty() {
            fallback
        } else {
            vec![HatId::new("ralph")]
        };

        chosen.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        chosen.dedup();
        chosen
    }

    fn hat_is_subscriber(&self, hat_id: &HatId, topic: &Topic) -> bool {
        if hat_id.as_str() == "ralph" {
            // 并行 Supervisor 始终注册 ralph#1，且订阅 "*" 作为兜底协调者
            return true;
        }

        let Some(hat) = self.registry.get(hat_id) else {
            return false;
        };

        // 并行模式下：默认补齐 `human.message` 订阅，用于通过 strict target 校验。
        //
        // 重要：
        // - 这里“只补齐订阅存在”，不改变 triggers-driven fanout 行为。
        // - fanout/queue 仍由 `target_instance` / contracts / triggers 决定。
        if topic.as_str() == "human.message" {
            return true;
        }

        hat.is_subscribed(topic)
    }

    fn is_control_plane_topic(topic: &str) -> bool {
        // 1.3：控制面 topic 特例（绕过 strict target 校验）
        //
        // 默认列表（明确列出，避免“隐式约定”）：
        // - gate.request
        // - gate.resolve
        // - gate.timeout
        matches!(
            topic,
            TOPIC_GATE_REQUEST | TOPIC_GATE_RESOLVE | TOPIC_GATE_TIMEOUT
        ) || topic.starts_with("gate.")
    }

    pub(super) fn ensure_event_id(&mut self, event: &mut Event) {
        if event.id.is_some() {
            return;
        }

        // 说明：
        // - HatInstance 产出的事件 id 通常由实例侧补齐.
        // - Supervisor 自己生成的事件（例如 task.start）也需要可引用 id，便于 reply/诊断/决策记录关联。
        event.id = Some(new_event_id());
    }

    fn available_permits_for_routing(&self) -> usize {
        if self.routing_batch_depth == 0 {
            return self.job_semaphore.available_permits();
        }

        // 说明：
        // - batch 内我们会“乐观地”认为已投递的实例会很快进入 Running 并获取 permit。
        // - 这能避免在同一批事件里过度扩容（available_permits 尚未被实例侧消耗的 race）。
        self.job_semaphore
            .available_permits()
            .saturating_sub(self.routing_inflight_instances.len())
    }

    fn mark_inflight_delivery(&mut self, instance_id: &HatInstanceId) {
        if self.routing_batch_depth == 0 {
            return;
        }
        self.routing_inflight_instances.insert(instance_id.clone());
    }

    fn effective_state(&self, instance_id: &HatInstanceId) -> HatInstanceState {
        if self.routing_batch_depth > 0 && self.routing_inflight_instances.contains(instance_id) {
            return HatInstanceState::Running;
        }

        self.instance_states
            .get(instance_id)
            .copied()
            .unwrap_or(HatInstanceState::Running)
    }

    fn contract_audience_instances(&self, contract: &TopicContract) -> Vec<HatInstanceId> {
        let selector = &contract.audience;
        let mut set: HashSet<HatInstanceId> = HashSet::new();

        // 1) 显式实例
        for id in &selector.instances {
            set.insert(id.clone());
        }

        // 2) hats -> instances
        for hat_id in &selector.hats {
            if let Some(instances) = self.instances_by_hat.get(hat_id) {
                for id in instances {
                    set.insert(id.clone());
                }
            }
        }

        // 3) prefixes -> instances（仅对“当前已存在实例”生效）
        if !selector.instance_prefixes.is_empty() {
            for id in self.instances.keys() {
                if selector
                    .instance_prefixes
                    .iter()
                    .any(|prefix| id.as_str().starts_with(prefix))
                {
                    set.insert(id.clone());
                }
            }
        }

        let mut vec: Vec<HatInstanceId> = set.into_iter().collect();
        vec.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        vec
    }

    fn apply_audience_override(
        &self,
        event: &Event,
        base_desired: &[HatInstanceId],
        base_existing: &[HatInstanceId],
    ) -> (
        Vec<HatInstanceId>,
        Vec<HatInstanceId>,
        Vec<HatInstanceId>,
        bool,
    ) {
        let Some(override_) = &event.audience_override else {
            // 没有 override：recipients 就是 base_existing（best-effort missing 由后续策略处理）
            return (
                base_existing.to_vec(),
                Self::missing_from_desired(base_desired, base_existing),
                Vec::new(),
                false,
            );
        };

        if override_.instances.is_empty() {
            return (
                base_existing.to_vec(),
                Self::missing_from_desired(base_desired, base_existing),
                Vec::new(),
                override_.require_delivery,
            );
        }

        // 目标公式：TopicContract.audience ∩ Event.audience_override
        let base_set: HashSet<HatInstanceId> = base_desired.iter().cloned().collect();
        let requested_set: HashSet<HatInstanceId> = override_.instances.iter().cloned().collect();

        let desired_intersection: HashSet<HatInstanceId> =
            base_set.intersection(&requested_set).cloned().collect();

        // 3.3/3.4：区分两类“缺失”
        // - missing_in_base：在交集中，但实例不存在（可能需要 spawn / queue / escalate / drop）
        // - missing_outside_base：override 指向了不在 contract audience 的实例（语义上不可投递）
        let missing_outside_base: Vec<HatInstanceId> =
            requested_set.difference(&base_set).cloned().collect();
        let mut missing_outside_base = missing_outside_base;
        missing_outside_base.sort_by(|a, b| a.as_str().cmp(b.as_str()));

        let mut recipients: Vec<HatInstanceId> = desired_intersection
            .iter()
            .filter(|id| self.instances.contains_key(*id))
            .cloned()
            .collect();
        recipients.sort_by(|a, b| a.as_str().cmp(b.as_str()));

        let mut missing_in_base: Vec<HatInstanceId> = desired_intersection
            .iter()
            .filter(|id| !self.instances.contains_key(*id))
            .cloned()
            .collect();
        missing_in_base.sort_by(|a, b| a.as_str().cmp(b.as_str()));

        (
            recipients,
            missing_in_base,
            missing_outside_base,
            override_.require_delivery,
        )
    }

    fn missing_from_desired(
        desired: &[HatInstanceId],
        existing: &[HatInstanceId],
    ) -> Vec<HatInstanceId> {
        let existing_set: HashSet<HatInstanceId> = existing.iter().cloned().collect();
        let mut missing: Vec<HatInstanceId> = desired
            .iter()
            .filter(|id| !existing_set.contains(*id))
            .cloned()
            .collect();
        missing.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        missing
    }

    async fn apply_missing_instance_policy(
        &mut self,
        contract: &TopicContract,
        event: &Event,
        mut recipients: Vec<HatInstanceId>,
        base_existing: &[HatInstanceId],
        missing_in_base: &[HatInstanceId],
    ) -> anyhow::Result<Vec<HatInstanceId>> {
        if missing_in_base.is_empty() {
            return Ok(recipients);
        }

        match contract.missing_instance_policy {
            MissingInstancePolicy::Spawn => {
                // 说明：仅对“在 audience 内但不存在”的实例尝试 spawn。
                for missing in missing_in_base {
                    if self.instances.contains_key(missing) {
                        continue;
                    }
                    self.spawn_instance(missing.clone(), false)?;
                    recipients.push(missing.clone());
                }
                recipients.sort_by(|a, b| a.as_str().cmp(b.as_str()));
                Ok(recipients)
            }
            MissingInstancePolicy::Queue => {
                // 说明：
                // - best-effort 下，如果 override 指向缺失实例，则允许回退到 base_existing
                // - 这样不会把缺失当成“硬失败”，但也不会 silent drop（仍会继续投递）
                if recipients.is_empty() {
                    return Ok(base_existing.to_vec());
                }
                Ok(recipients)
            }
            MissingInstancePolicy::Escalate => {
                self.escalate_delivery_failure(
                    event,
                    "missing instances (escalate)".to_string(),
                    missing_in_base,
                    &[],
                )
                .await?;
                Ok(Vec::new())
            }
            MissingInstancePolicy::Drop => {
                // best-effort drop：直接忽略缺失实例
                Ok(recipients)
            }
        }
    }

    async fn deliver_fanout(
        &mut self,
        event: Event,
        recipients: &[HatInstanceId],
    ) -> anyhow::Result<()> {
        for instance_id in recipients {
            self.record_agents_last_input(instance_id, &event);
            if let Some(handle) = self.instances.get(instance_id) {
                handle
                    .cmd_tx
                    .send(HatInstanceCommand::Deliver(Box::new(event.clone())))
                    .await
                    .context("Failed to deliver fanout event")?;
                self.mark_inflight_delivery(instance_id);
            }
        }
        self.write_agents_snapshot_best_effort();
        Ok(())
    }

    async fn deliver_queue(
        &mut self,
        event: Event,
        topic: &str,
        contract: &TopicContract,
        recipients: &[HatInstanceId],
    ) -> anyhow::Result<()> {
        let Some(event_id) = event.id.clone() else {
            // ensure_event_id 已经补齐，这里只是兜底
            anyhow::bail!("queue delivery requires event.id, but it was missing");
        };

        // recipients 为空的情况已在上游处理，这里只做防御性保护
        if recipients.is_empty() {
            tracing::warn!(event_id = %event_id, topic = %event.topic, "queue delivery skipped: no recipients");
            return Ok(());
        }

        // 3.6：replay 时不重算 —— 如果已经有决策记录，优先复用
        if let Some(chosen) = self.queue_decisions.get(&event_id)
            && recipients.iter().any(|id| id == chosen)
        {
            self.deliver_to_instance_id(event, chosen.clone()).await?;
            return Ok(());
        }

        let (chosen, reason) = match contract.queue_selection {
            QueueSelection::Deterministic => {
                let chosen = self.choose_deterministic(topic, recipients);
                (chosen, Some("deterministic".to_string()))
            }
            QueueSelection::Llm => {
                if recipients.len() == 1 {
                    (recipients[0].clone(), Some("single_candidate".to_string()))
                } else {
                    match self.choose_llm(&event, recipients).await {
                        Ok((chosen, reason)) => (chosen, reason),
                        Err(e) => {
                            // 兜底：LLM 不可用/超时/输出不合法 -> deterministic（同样要落盘）
                            tracing::warn!(event_id = %event_id, error = %e, "LLM queue selection failed, falling back to deterministic");
                            let chosen = self.choose_deterministic(topic, recipients);
                            (
                                chosen,
                                Some("llm_failed_fallback_deterministic".to_string()),
                            )
                        }
                    }
                }
            }
        };

        // 3.6：强制落盘（候选集 + 结果 + 可选原因），保证 replay 不重算
        let decision = QueueDecisionRecord::new(
            event_id.clone(),
            recipients.to_vec(),
            chosen.clone(),
            reason,
        );
        self.event_logger
            .log_queue_decision(0, "supervisor", &decision)
            .context("Failed to log queue decision (dispatch.decision)")?;
        self.queue_decisions.insert(event_id, chosen.clone());

        self.deliver_to_instance_id(event, chosen).await
    }

    async fn deliver_to_instance_id(
        &mut self,
        event: Event,
        instance_id: HatInstanceId,
    ) -> anyhow::Result<()> {
        self.record_agents_last_input(&instance_id, &event);
        let Some(handle) = self.instances.get(&instance_id) else {
            anyhow::bail!("deliver_to_instance_id: instance not found: {instance_id}");
        };
        handle
            .cmd_tx
            .send(HatInstanceCommand::Deliver(Box::new(event)))
            .await
            .context("Failed to deliver event to chosen instance")?;
        self.mark_inflight_delivery(&instance_id);
        self.write_agents_snapshot_best_effort();
        Ok(())
    }

    fn choose_deterministic(&mut self, topic: &str, recipients: &[HatInstanceId]) -> HatInstanceId {
        // 说明：
        // - deterministic 需要同时满足 round-robin 与 least-busy 的诉求：
        //   1) 优先选 Idle/Created（更不忙）
        //   2) 同一“忙闲等级”内按 round-robin 公平轮转
        // - candidates 的顺序要求稳定：上游 contract_audience_instances 已按字符串排序
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        enum BusyRank {
            Idle = 0,
            Created = 1,
            Running = 2,
            Failed = 3,
            Done = 4,
        }

        let rank_of = |state: HatInstanceState| match state {
            HatInstanceState::Idle => BusyRank::Idle,
            HatInstanceState::Created => BusyRank::Created,
            HatInstanceState::Running => BusyRank::Running,
            HatInstanceState::Failed => BusyRank::Failed,
            HatInstanceState::Done => BusyRank::Done,
        };

        let mut best_rank = BusyRank::Done;
        let mut buckets: HashMap<BusyRank, Vec<HatInstanceId>> = HashMap::new();

        for id in recipients {
            let state = self.effective_state(id);
            let rank = rank_of(state);
            if rank < best_rank {
                best_rank = rank;
            }
            buckets.entry(rank).or_default().push(id.clone());
        }

        let best = buckets
            .remove(&best_rank)
            .unwrap_or_else(|| recipients.to_vec());

        let cursor = self
            .rr_cursor_by_topic
            .entry(topic.to_string())
            .or_insert(0);
        let chosen = best[*cursor % best.len()].clone();
        *cursor = cursor.saturating_add(1);
        chosen
    }

    async fn choose_llm(
        &mut self,
        event: &Event,
        recipients: &[HatInstanceId],
    ) -> anyhow::Result<(HatInstanceId, Option<String>)> {
        let Some(output_tx) = self.output_tx.clone() else {
            anyhow::bail!("choose_llm requires output_tx to be initialized");
        };

        let Some(event_id) = event.id.as_ref() else {
            anyhow::bail!("choose_llm requires event.id");
        };

        // 为了让 LLM 更容易选“least-busy”，把实例状态也写进 prompt。
        let mut candidate_lines = String::new();
        for id in recipients {
            let state = self.effective_state(id);
            candidate_lines.push_str(&format!("- {id} (state={state})\n"));
        }

        let job_id = self.next_decision_job_id;
        self.next_decision_job_id = self.next_decision_job_id.saturating_add(1);
        let decider_instance_id = HatInstanceId::from_parts("ralph", format!("decider-{job_id}"));
        let prompt = format!(
            "ralph_hat_instance_id:\"{hat_instance_id}\"\n\n\
You are Ralph acting ONLY as a dispatch decider.\n\
Return exactly ONE <event> tag with topic=\"{topic}\".\n\
\n\
The payload MUST be valid JSON with fields:\n\
  event_id: string\n\
  candidates: string[]\n\
  chosen_instance: string\n\
  reason?: string\n\
\n\
Hard rules:\n\
- chosen_instance MUST be one of candidates.\n\
- Do NOT output any other <event> tags.\n\
\n\
Event:\n\
- id: {event_id}\n\
- topic: {event_topic}\n\
- payload: {event_payload}\n\
\n\
Candidates:\n\
{candidates}\n",
            hat_instance_id = decider_instance_id,
            topic = TOPIC_DISPATCH_DECISION,
            event_id = event_id,
            event_topic = event.topic,
            event_payload = event.payload,
            candidates = candidate_lines
        );
        let prompt = prompt_overlay::inject_all_hat_prompt(prompt, self.all_hat_prompt.as_deref());

        let job = HatJob {
            job_id,
            instance_id: decider_instance_id,
            hat_id: HatId::new("ralph"),
            prompt,
            backend: JobBackend::Default,
            session_strategy: SessionStrategy::Exec,
            timeout: Some(Duration::from_secs(20)),
            // decider job 保持“硬超时”语义：到时间就终止（避免决策 job 挂住拖垮并行调度）。
            output_stale_timeout: None,
            workdir: None,
        };

        let (_cancel_tx, cancel_rx) = watch::channel(false);
        let (_control_tx, control_rx) = mpsc::channel(1);
        let result = self
            .executor
            .execute(job, output_tx, cancel_rx, control_rx)
            .await?;

        if !result.success {
            anyhow::bail!(
                "LLM decider job failed: success={} exit_code={:?} timed_out={} canceled={}",
                result.success,
                result.exit_code,
                result.timed_out,
                result.canceled
            );
        }

        let parsed = EventParser::new().parse(&result.output_for_parsing);
        let Some(decision_event) = parsed
            .into_iter()
            .find(|e| e.topic.as_str() == TOPIC_DISPATCH_DECISION)
        else {
            anyhow::bail!("LLM decider output did not contain <event topic=\"dispatch.decision\">");
        };

        let decision: QueueDecisionRecord = serde_json::from_str(&decision_event.payload)
            .context("Failed to parse dispatch.decision payload as QueueDecisionRecord")?;

        // 安全校验：必须在候选集中
        if !recipients.iter().any(|id| id == &decision.chosen_instance) {
            anyhow::bail!(
                "LLM chose an instance not in candidates: chosen={} candidates={:?}",
                decision.chosen_instance,
                recipients
            );
        }

        Ok((decision.chosen_instance, decision.reason))
    }

    fn on_dispatch_decision_record(&mut self, event: &Event) {
        // 允许两种来源：
        // 1) supervisor 自己 log_queue_decision 写入 events.jsonl（不会走到这里）
        // 2) 外部 agent 输出 `<event topic="dispatch.decision">...`（例如 decider job）
        if let Ok(decision) = serde_json::from_str::<QueueDecisionRecord>(&event.payload) {
            self.queue_decisions
                .insert(decision.event_id.clone(), decision.chosen_instance.clone());
        }
    }

    pub(super) fn load_queue_decisions_from_history(&mut self) -> anyhow::Result<()> {
        let history = EventHistory::new(self.event_logger.path());
        let records = history.read_all().context("Failed to read event history")?;
        for record in records {
            if record.topic != TOPIC_DISPATCH_DECISION {
                continue;
            }
            match serde_json::from_str::<QueueDecisionRecord>(&record.payload) {
                Ok(decision) => {
                    self.queue_decisions
                        .insert(decision.event_id.clone(), decision.chosen_instance);
                }
                Err(e) => {
                    tracing::warn!(error = %e, payload_len = record.payload.len(), "Failed to parse dispatch.decision payload from history");
                }
            }
        }
        Ok(())
    }

    fn spawn_dynamic_instance(&mut self, hat_id: &HatId) -> anyhow::Result<HatInstanceId> {
        let next = self
            .next_instance_seq_by_hat
            .entry(hat_id.clone())
            .or_insert(2);
        let key = *next;
        *next = next.saturating_add(1);

        let instance_id = HatInstanceId::from_parts(hat_id.as_str(), key.to_string());
        self.spawn_instance(instance_id.clone(), true)?;
        Ok(instance_id)
    }

    fn spawn_instance(
        &mut self,
        instance_id: HatInstanceId,
        is_dynamic: bool,
    ) -> anyhow::Result<()> {
        if self.instances.contains_key(&instance_id) {
            return Ok(());
        }

        let dynamic_idle_ttl = self.effective_dynamic_idle_ttl();

        let output_tx = self
            .output_tx
            .clone()
            .expect("output_tx must be set before spawn_instance()");
        let instance_tx = self
            .instance_tx
            .clone()
            .expect("instance_tx must be set before spawn_instance()");

        let hat_id_str = instance_id
            .split_hat_id()
            .context("Cannot spawn instance: missing hat id prefix (expected {hat}#{key})")?;
        let hat_id = HatId::new(hat_id_str);

        let (hat, hat_config) = if hat_id.as_str() == "ralph" {
            // 说明:
            // - ralph#2 是按需创建的"协调者备用实例"(busy ralph#1 时接管投递).
            // - 如果不给它注入与 ralph#1 等价的 coordinator 指令,它会退回到极小兜底 prompt,
            //   从而更容易漂移,发布不在协议内的 topic(例如 integration.done),导致 autopilot/CI 硬断言失败.
            let hat = Hat::new("ralph", "Ralph")
                .with_description(
                    "Parallel coordinator: handles true-orphan events and makes completion decisions",
                )
                .subscribe("*")
                .with_instructions(self.build_ralph_coordinator_instructions(&instance_id));
            (hat, None::<HatConfig>)
        } else {
            let hat = self
                .registry
                .get(&hat_id)
                .cloned()
                .with_context(|| format!("Cannot spawn instance: unknown hat id: {hat_id}"))?;
            let cfg = self.registry.get_config(&hat_id).cloned();
            (hat, cfg)
        };

        let job_timeout = self.resolve_job_timeout(hat_config.as_ref());
        let job_output_stale_timeout = self.resolve_output_stale_timeout(hat_config.as_ref());

        let handle = HatInstanceHandle::spawn(
            instance_id.clone(),
            hat,
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
            output_tx,
            instance_tx,
            Arc::clone(&self.job_semaphore),
            Arc::clone(&self.command_queue),
            is_dynamic,
            dynamic_idle_ttl,
        );

        self.instances.insert(instance_id.clone(), handle);
        self.instance_states
            .insert(instance_id.clone(), HatInstanceState::Created);
        self.instances_by_hat
            .entry(hat_id.clone())
            .or_default()
            .push(instance_id.clone());

        if is_dynamic {
            self.dynamic_instances.insert(instance_id.clone());
        }

        // 3.1：实例 key 单调递增且永不复用（对 allocator 做“下界修正”，避免与显式实例冲突）。
        if let Some(key_str) = instance_id.split_instance_key()
            && let Ok(key_num) = key_str.parse::<u64>()
        {
            let next = key_num.saturating_add(1);
            self.next_instance_seq_by_hat
                .entry(hat_id)
                .and_modify(|v| *v = (*v).max(next))
                .or_insert(next);
        }

        Ok(())
    }

    async fn escalate_delivery_failure(
        &mut self,
        event: &Event,
        summary: String,
        missing_in_base: &[HatInstanceId],
        missing_outside_base: &[HatInstanceId],
    ) -> anyhow::Result<()> {
        // 说明：
        // - 现在先做“最小可用的 escalate”：交给 ralph#1（coordinator）处理
        // - 4.x 会把它升级为 gate.request / gate.resolve / gate.timeout 的完整状态机
        let mut payload = String::new();
        payload.push_str(&summary);
        payload.push('\n');
        payload.push_str(&format!(
            "event_id: {}\n",
            event.id.as_deref().unwrap_or("<none>")
        ));
        payload.push_str(&format!("topic: {}\n", event.topic));
        if !missing_in_base.is_empty() {
            payload.push_str(&format!("missing_in_base: {:?}\n", missing_in_base));
        }
        if !missing_outside_base.is_empty() {
            payload.push_str(&format!(
                "missing_outside_base: {:?}\n",
                missing_outside_base
            ));
        }

        let mut escalation_event = Event::new("routing.escalate", payload);
        self.ensure_event_id(&mut escalation_event);

        // 写入事件日志，便于 replay/排查
        let _ = self.event_logger.log_event(
            0,
            "supervisor",
            &escalation_event,
            Some(&HatId::new("ralph")),
        );

        // 直接投递到 ralph 协调实例（不走 TopicContract，避免“配置没写 gate topic”导致二次失败）
        let ralph_instance = self.choose_ralph_instance_for_delivery()?;
        if let Some(handle) = self.instances.get(&ralph_instance) {
            handle
                .cmd_tx
                .send(HatInstanceCommand::Deliver(Box::new(escalation_event)))
                .await?;
        } else {
            tracing::warn!("No available ralph instance found; escalation event was not delivered");
        }

        Ok(())
    }

    fn rewrite_target_for_busy_ralph(&mut self, event: &mut Event) -> anyhow::Result<()> {
        let Some(target_instance) = event.target_instance.clone() else {
            return Ok(());
        };

        if target_instance.as_str() != "ralph#1" {
            return Ok(());
        }

        // turn/steer 与 turn/interrupt 属于“运行时 in-flight 控制信号”：
        // - 它们必须直达目标实例(例如 ralph#1),否则会变成“改投到 ralph#2 但无法影响正在运行的 turn”。
        // - 这会造成 steer/interrupt 看似成功写入外部事件文件,但实际无效的黑盒体验。
        if matches!(
            event.turn_action,
            Some(TurnAction::Steer | TurnAction::Interrupt)
        ) {
            return Ok(());
        }

        let primary = HatInstanceId::from_parts("ralph", "1");
        if self.effective_state(&primary) != HatInstanceState::Running {
            return Ok(());
        }

        let chosen = self.choose_ralph_instance_for_delivery()?;
        if chosen.as_str() != primary.as_str() {
            event.target_instance = Some(chosen);
            event.target = None;
        }

        Ok(())
    }

    fn choose_ralph_instance_for_delivery(&mut self) -> anyhow::Result<HatInstanceId> {
        let primary = HatInstanceId::from_parts("ralph", "1");
        if !self.instances.contains_key(&primary) {
            self.spawn_instance(primary.clone(), false)?;
        }

        // 主实例优先: 只要不是 Running,就固定走 ralph#1。
        if self.effective_state(&primary) != HatInstanceState::Running {
            return Ok(primary);
        }

        let secondary = HatInstanceId::from_parts("ralph", "2");
        if !self.instances.contains_key(&secondary) {
            self.spawn_instance(secondary.clone(), false)?;
        }

        if self.effective_state(&secondary) != HatInstanceState::Running {
            return Ok(secondary);
        }

        // 双忙兜底: 保持主实例优先,由实例内部 pending 队列吸收。
        Ok(primary)
    }
}
