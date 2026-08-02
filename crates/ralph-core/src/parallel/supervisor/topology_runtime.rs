//! Parent-visible topology mutation runtime.
//!
//! 说明:
//! - `capability.request` 负责 isolated child/micro-run,明确不改父拓扑。
//! - `topology.spawn_group` 负责父级可见的动态实例创建。
//! - 这里把两条协议分开实现,避免把 child-run 投影误当成真实 HatInstance。

use super::{
    DYNAMIC_INSTANCE_RETIREMENT_REASON_DELIVERY_FAILED_AFTER_SPAWN, FixedRoleMetadata,
    ParallelSupervisor, RuntimeDeliveryMode,
};
use crate::{
    EffectiveRoleContract, EvidenceArtifactKind, EvidenceIndexEntry, IdentitySource, RoleContract,
    RoleContractSummary, RolePersistence, TOPIC_TOPOLOGY_SPAWN_FAILED, TOPIC_TOPOLOGY_SPAWN_GROUP,
    TOPIC_TOPOLOGY_SPAWN_RESULT, TOPOLOGY_SPAWN_PHASE_DELIVERY_FAILED_AFTER_SPAWN,
    TOPOLOGY_SPAWN_PHASE_MEMBER_VALIDATION_FAILED, TOPOLOGY_SPAWN_PHASE_SPAWN_FAILED,
    TopologySpawnFailedMember, TopologySpawnGroupFailed, TopologySpawnGroupRequest,
    TopologySpawnGroupResult, TopologySpawnMember, TopologySpawnedInstance,
};
use ralph_proto::{Event, HatId, HatInstanceId, Topic};
use std::collections::HashSet;

impl ParallelSupervisor {
    /// 处理 `ralph#1` 输出的 `topology.spawn_group`。
    ///
    /// 返回值:
    /// - `true`: 当前事件已经被 topology runtime 消费,不应继续进入普通业务路由。
    /// - `false`: 当前事件不是 topology spawn 请求,调用方继续按原路由处理。
    pub(super) async fn handle_topology_spawn_group_event(
        &mut self,
        event: &Event,
    ) -> anyhow::Result<bool> {
        if event.topic.as_str() != TOPIC_TOPOLOGY_SPAWN_GROUP {
            return Ok(false);
        }

        if !Self::is_parent_coordinator_event(event) {
            self.emit_topology_spawn_failed(
                event,
                TopologySpawnGroupFailed {
                    status: "failed".to_string(),
                    request_id: None,
                    hat: None,
                    delivery_topic: None,
                    error: "topology.spawn_group is coordinator-only; expected source_instance from ralph#*".to_string(),
                    parent_topology_unchanged: true,
                },
            )
            .await?;
            return Ok(true);
        }

        let request = match TopologySpawnGroupRequest::parse_payload(&event.payload) {
            Ok(request) => request,
            Err(error) => {
                self.emit_topology_spawn_failed(
                    event,
                    TopologySpawnGroupFailed {
                        status: "failed".to_string(),
                        request_id: error.request_id,
                        hat: error.hat,
                        delivery_topic: error.delivery_topic,
                        error: error.error,
                        parent_topology_unchanged: true,
                    },
                )
                .await?;
                return Ok(true);
            }
        };

        if !self
            .handled_topology_spawn_request_ids
            .insert(request.request_id.clone())
        {
            tracing::debug!(
                request_id = %request.request_id,
                hat = %request.hat,
                "Duplicate topology.spawn_group ignored"
            );
            return Ok(true);
        }

        let hat_id = HatId::new(request.hat.clone());
        let delivery_topic = Topic::new(request.delivery_topic.clone());

        if hat_id.as_str() == "ralph" {
            self.emit_topology_spawn_failed(
                event,
                TopologySpawnGroupFailed {
                    status: "failed".to_string(),
                    request_id: Some(request.request_id),
                    hat: Some(request.hat),
                    delivery_topic: Some(request.delivery_topic),
                    error: "topology.spawn_group does not support spawning coordinator hat `ralph`"
                        .to_string(),
                    parent_topology_unchanged: true,
                },
            )
            .await?;
            return Ok(true);
        }

        if self.registry.get(&hat_id).is_none() {
            self.emit_topology_spawn_failed(
                event,
                TopologySpawnGroupFailed {
                    status: "failed".to_string(),
                    request_id: Some(request.request_id),
                    hat: Some(request.hat),
                    delivery_topic: Some(request.delivery_topic),
                    error: format!("unknown target hat for topology.spawn_group: {hat_id}"),
                    parent_topology_unchanged: true,
                },
            )
            .await?;
            return Ok(true);
        }

        if !self.hat_is_subscriber(&hat_id, &delivery_topic) {
            self.emit_topology_spawn_failed(
                event,
                TopologySpawnGroupFailed {
                    status: "failed".to_string(),
                    request_id: Some(request.request_id),
                    hat: Some(request.hat),
                    delivery_topic: Some(request.delivery_topic),
                    error: format!(
                        "target hat `{hat_id}` is not subscribed to delivery topic `{}`",
                        delivery_topic.as_str()
                    ),
                    parent_topology_unchanged: true,
                },
            )
            .await?;
            return Ok(true);
        }

        self.spawn_group_members(event, &request, &hat_id).await?;
        Ok(true)
    }

    async fn spawn_group_members(
        &mut self,
        source_event: &Event,
        request: &TopologySpawnGroupRequest,
        hat_id: &HatId,
    ) -> anyhow::Result<()> {
        let mut spawned = Vec::new();
        let mut failed = Vec::new();
        let target_publish_topics = self.target_publish_topics(hat_id);

        for (index, member) in request.instances.iter().enumerate() {
            let (effective_contract, contract_warnings) = match self
                .canonicalize_topology_role_contract(
                    source_event,
                    request,
                    member,
                    &target_publish_topics,
                ) {
                Ok(contract) => contract,
                Err(error) => {
                    failed.push(
                        TopologySpawnFailedMember::new(index, member.role.clone(), error)
                            .with_request_id(request.request_id.clone())
                            .with_phase(TOPOLOGY_SPAWN_PHASE_MEMBER_VALIDATION_FAILED)
                            .with_recovery_hint(
                                "Fix this topology.spawn_group member role_contract / allowed topics and retry the failed member or request.",
                            ),
                    );
                    continue;
                }
            };
            let contract_summary = effective_contract.summary();

            let instance_id = match self.spawn_dynamic_instance_with_effective_role_contract(
                hat_id,
                Some(source_event),
                "topology_spawn_group",
                Some(effective_contract),
            ) {
                Ok(instance_id) => instance_id,
                Err(error) => {
                    failed.push(
                        TopologySpawnFailedMember::new(
                            index,
                            member.role.clone(),
                            format!("spawn failed: {error:#}"),
                        )
                        .with_request_id(request.request_id.clone())
                        .with_phase(TOPOLOGY_SPAWN_PHASE_SPAWN_FAILED)
                        .with_recovery_hint(
                            "Inspect target hat configuration and runtime lifecycle records, then retry this failed member.",
                        ),
                    );
                    continue;
                }
            };

            spawned.push(TopologySpawnedInstance {
                index,
                instance_id: instance_id.to_string(),
                role: member.role.clone(),
                fixed_role: member.fixed_role,
                role_contract_summary: Some(contract_summary.clone()),
            });
            if member.fixed_role == Some(true) {
                self.fixed_role_metadata.insert(
                    instance_id.clone(),
                    FixedRoleMetadata {
                        label: member.role.clone(),
                        reason: Some(
                            "topology.spawn_group member marked fixed_role=true".to_string(),
                        ),
                    },
                );
            }
            self.write_agents_snapshot_best_effort();
            self.record_topology_spawn_evidence_index(request, &instance_id, &contract_summary);

            let mut delivery_event = Self::member_delivery_event(
                source_event,
                request,
                member,
                &instance_id,
                &contract_summary,
                &contract_warnings,
            )?;
            self.ensure_event_id(&mut delivery_event);

            if let Some(observer) = &self.event_observer {
                observer(&delivery_event);
            }

            if let Err(error) =
                self.event_logger
                    .log_event(0, "topology", &delivery_event, Some(hat_id))
            {
                tracing::warn!(
                    %error,
                    topic = %delivery_event.topic,
                    target_instance = %instance_id,
                    "Failed to log topology member delivery event"
                );
            }

            if let Err(error) = self
                .deliver_to_instance_id(
                    delivery_event,
                    instance_id.clone(),
                    RuntimeDeliveryMode::Direct,
                )
                .await
            {
                self.mark_dynamic_instance_failed_and_unregister(
                    &instance_id,
                    DYNAMIC_INSTANCE_RETIREMENT_REASON_DELIVERY_FAILED_AFTER_SPAWN,
                );
                self.write_agents_snapshot_best_effort();
                failed.push(
                    TopologySpawnFailedMember::new(
                        index,
                        member.role.clone(),
                        format!("delivery failed after spawn: {error:#}"),
                    )
                    .with_request_id(request.request_id.clone())
                    .with_instance_id(instance_id.to_string())
                    .with_phase(TOPOLOGY_SPAWN_PHASE_DELIVERY_FAILED_AFTER_SPAWN)
                    .with_recovery_hint(
                        "Inspect runtime.delivery records and the agents tombstone; retry the failed member after fixing delivery route or instance state.",
                    ),
                );
            }
        }

        let status = match (spawned.is_empty(), failed.is_empty()) {
            (false, true) => "spawned",
            (false, false) => "partial",
            (true, false) => "failed",
            (true, true) => "spawned",
        }
        .to_string();

        let topology_changed = !spawned.is_empty();
        self.emit_topology_spawn_result(
            source_event,
            TopologySpawnGroupResult {
                status,
                request_id: request.request_id.clone(),
                hat: request.hat.clone(),
                delivery_topic: request.delivery_topic.clone(),
                spawned,
                failed,
                parent_topology_unchanged: !topology_changed,
            },
        )
        .await
    }

    fn member_delivery_event(
        source_event: &Event,
        request: &TopologySpawnGroupRequest,
        member: &TopologySpawnMember,
        instance_id: &HatInstanceId,
        role_contract_summary: &crate::RoleContractSummary,
        role_contract_warnings: &[String],
    ) -> anyhow::Result<Event> {
        let payload = serde_json::json!({
            "topology_request_id": request.request_id.clone(),
            "target_instance": instance_id.to_string(),
            "role": member.role.clone(),
            "task": member.task.clone(),
            "input": member.input.clone(),
            "fixed_role": member.fixed_role,
            "role_contract_summary": role_contract_summary,
            "role_contract_warnings": role_contract_warnings,
        });

        let mut event = Event::new(
            request.delivery_topic.as_str(),
            serde_json::to_string(&payload)?,
        )
        .with_reply(source_event.id.clone().unwrap_or_default())
        .with_target_instance(instance_id.clone());

        if let Some(source) = source_event.source.clone() {
            event = event.with_source(source);
        }
        if let Some(source_instance) = source_event.source_instance.clone() {
            event = event.with_source_instance(source_instance);
        }

        Ok(event)
    }

    fn target_publish_topics(&self, hat_id: &HatId) -> Vec<String> {
        self.registry
            .get(hat_id)
            .map(|hat| {
                hat.publishes
                    .iter()
                    .map(|topic| topic.as_str().to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    }

    fn canonicalize_topology_role_contract(
        &self,
        source_event: &Event,
        request: &TopologySpawnGroupRequest,
        member: &TopologySpawnMember,
        target_publish_topics: &[String],
    ) -> Result<(EffectiveRoleContract, Vec<String>), String> {
        let mut warnings = Vec::new();
        let persistence = if member.fixed_role == Some(true) {
            RolePersistence::Fixed
        } else {
            RolePersistence::Temporary
        };

        if let Some(raw) = &member.role_contract {
            if raw.role_name.trim() != member.role.trim() {
                return Err(format!(
                    "role_contract.role_name `{}` conflicts with member.role `{}`",
                    raw.role_name, member.role
                ));
            }
            if raw.identity_source != IdentitySource::TaskDerived {
                return Err(format!(
                    "role_contract.identity_source must be task-derived, got {}",
                    raw.identity_source
                ));
            }
            if raw.input_contract.trim().is_empty() {
                return Err("role_contract.input_contract must not be empty".to_string());
            }
            if raw.output_contract.trim().is_empty() {
                return Err("role_contract.output_contract must not be empty".to_string());
            }
            if !raw.objective.trim().is_empty() && raw.objective.trim() != member.task.trim() {
                warnings.push(format!(
                    "raw role_contract.objective ignored; canonical objective comes from member.task for request {}",
                    request.request_id
                ));
            }
        }

        let allowed_result_topics = self.canonical_allowed_result_topics(
            member,
            target_publish_topics,
            request.delivery_topic.as_str(),
        )?;
        let raw = member.role_contract.as_ref();
        let input_contract = raw
            .map(|contract| contract.input_contract.trim())
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| {
                format!(
                    "Handle the delivered `{}` event for role `{}`.",
                    request.delivery_topic, member.role
                )
            });
        let output_contract = raw
            .map(|contract| contract.output_contract.trim())
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| {
                format!(
                    "Publish only the allowed result topics: {}.",
                    allowed_result_topics.join(", ")
                )
            });
        let forbidden_responsibilities = raw
            .map(|contract| contract.forbidden_responsibilities.clone())
            .filter(|items| !items.is_empty())
            .unwrap_or_else(default_topology_forbidden_responsibilities);
        let success_criteria = raw
            .map(|contract| contract.success_criteria.clone())
            .filter(|items| !items.is_empty())
            .unwrap_or_else(|| {
                vec![
                    "Complete the assigned role-specific task.".to_string(),
                    "Emit evidence through an allowed result topic.".to_string(),
                ]
            });

        let contract = RoleContract::new(
            member.role.clone(),
            member.task.clone(),
            input_contract,
            output_contract,
            allowed_result_topics,
            forbidden_responsibilities,
            success_criteria,
            IdentitySource::TaskDerived,
        );

        Ok((
            EffectiveRoleContract::new(
                contract,
                persistence,
                request.request_id.clone(),
                source_event.id.clone(),
            ),
            warnings,
        ))
    }

    fn canonical_allowed_result_topics(
        &self,
        member: &TopologySpawnMember,
        target_publish_topics: &[String],
        delivery_topic: &str,
    ) -> Result<Vec<String>, String> {
        let target_publish_set = target_publish_topics
            .iter()
            .map(String::as_str)
            .collect::<HashSet<_>>();
        if target_publish_set.is_empty() {
            return Err(
                "target hat has no publish topics for role contract output allowlist".to_string(),
            );
        }

        let raw_topics = member
            .role_contract
            .as_ref()
            .map(|contract| contract.allowed_topics.clone())
            .filter(|topics| !topics.is_empty())
            .unwrap_or_else(|| target_publish_topics.to_vec());

        let mut denied = raw_topics
            .iter()
            .filter(|topic| !crate::is_allowed_role_result_topic(topic))
            .cloned()
            .collect::<Vec<_>>();
        denied.sort();
        denied.dedup();
        if !denied.is_empty() {
            return Err(format!(
                "role_contract allowed result topics include control-plane topic(s): {}",
                denied.join(", ")
            ));
        }

        let mut allowed = raw_topics
            .into_iter()
            .filter(|topic| target_publish_set.contains(topic.as_str()))
            // `delivery_topic` 是输入 topic,不能被提升为 worker 的输出 publish 权限。
            // 如果目标 hat 同时声明了同名 publish,这里仍然将它从结果 allowlist 中剔除。
            .filter(|topic| topic.as_str() != delivery_topic)
            .collect::<Vec<_>>();
        allowed.sort();
        allowed.dedup();
        if allowed.is_empty() {
            return Err(
                "role_contract allowed result topics have no intersection with target hat publishes"
                    .to_string(),
            );
        }
        Ok(allowed)
    }

    async fn emit_topology_spawn_result(
        &mut self,
        source_event: &Event,
        result: TopologySpawnGroupResult,
    ) -> anyhow::Result<()> {
        let mut event = Event::new(TOPIC_TOPOLOGY_SPAWN_RESULT, serde_json::to_string(&result)?)
            .with_reply(source_event.id.clone().unwrap_or_default())
            .with_target_instance(HatInstanceId::from_parts("ralph", "1"));

        if let Some(source) = source_event.source.clone() {
            event = event.with_source(source);
        }
        if let Some(source_instance) = source_event.source_instance.clone() {
            event = event.with_source_instance(source_instance);
        }

        self.emit_topology_response_event(event).await
    }

    fn record_topology_spawn_evidence_index(
        &mut self,
        request: &TopologySpawnGroupRequest,
        instance_id: &HatInstanceId,
        contract_summary: &RoleContractSummary,
    ) {
        let producer = "parallel.supervisor.topology_spawn";
        let event_log_path = self.event_logger.path().display().to_string();
        let agents_snapshot_path = self
            .agents_snapshot_path
            .as_ref()
            .map(|path| path.display().to_string());

        // request_id -> child instance:
        // reviewer 可以从 topology.spawn_group request 直接列出运行时创建的实例。
        self.record_topology_evidence_index_entry(EvidenceIndexEntry::dynamic_spawn_request(
            request.request_id.clone(),
            EvidenceArtifactKind::EventLogJsonl,
            event_log_path.clone(),
            producer,
            instance_id.to_string(),
        ));

        for result_topic in &contract_summary.allowed_result_topics {
            // role_contract_hash -> event log:
            // event log 保留 topology.spawn.result、delivery 和 result events。
            self.record_topology_evidence_index_entry(
                EvidenceIndexEntry::dynamic_role_result_topic(
                    contract_summary.role_contract_hash.clone(),
                    EvidenceArtifactKind::EventLogJsonl,
                    event_log_path.clone(),
                    producer,
                    request.request_id.clone(),
                    instance_id.to_string(),
                    result_topic.clone(),
                ),
            );

            // role_contract_hash -> agents snapshot:
            // agents snapshot 是 role summary / tombstone 的 sidecar truth source。
            if let Some(path) = &agents_snapshot_path {
                self.record_topology_evidence_index_entry(
                    EvidenceIndexEntry::dynamic_role_result_topic(
                        contract_summary.role_contract_hash.clone(),
                        EvidenceArtifactKind::AgentsSnapshotJson,
                        path.clone(),
                        producer,
                        request.request_id.clone(),
                        instance_id.to_string(),
                        result_topic.clone(),
                    ),
                );
            }
        }
    }

    fn record_topology_evidence_index_entry(&mut self, entry: EvidenceIndexEntry) {
        if let Err(error) = self.evidence_index_writer.record(&entry) {
            tracing::warn!(
                %error,
                correlation_id = %entry.correlation_id,
                "Failed to write topology spawn evidence index entry"
            );
        }
    }

    async fn emit_topology_spawn_failed(
        &mut self,
        source_event: &Event,
        failed: TopologySpawnGroupFailed,
    ) -> anyhow::Result<()> {
        let mut event = Event::new(TOPIC_TOPOLOGY_SPAWN_FAILED, serde_json::to_string(&failed)?)
            .with_reply(source_event.id.clone().unwrap_or_default())
            .with_target_instance(HatInstanceId::from_parts("ralph", "1"));

        if let Some(source) = source_event.source.clone() {
            event = event.with_source(source);
        }
        if let Some(source_instance) = source_event.source_instance.clone() {
            event = event.with_source_instance(source_instance);
        }

        self.emit_topology_response_event(event).await
    }

    async fn emit_topology_response_event(&mut self, mut event: Event) -> anyhow::Result<()> {
        self.ensure_event_id(&mut event);

        if let Some(observer) = &self.event_observer {
            observer(&event);
        }

        let triggered = self.registry.find_by_trigger(event.topic.as_str());
        if let Err(error) = self
            .event_logger
            .log_event(0, "topology", &event, triggered)
        {
            tracing::warn!(
                %error,
                topic = %event.topic,
                "Failed to log topology response event"
            );
        }

        let target_instance = event
            .target_instance
            .clone()
            .unwrap_or_else(|| HatInstanceId::from_parts("ralph", "1"));

        if !self.instances.contains_key(&target_instance) {
            tracing::warn!(
                target_instance = %target_instance,
                topic = %event.topic,
                "Topology response target instance is missing; response stays in event log only"
            );
            return Ok(());
        }

        self.deliver_to_instance_id(event, target_instance, RuntimeDeliveryMode::Direct)
            .await
    }

    fn is_parent_coordinator_event(event: &Event) -> bool {
        if let Some(source_instance) = &event.source_instance {
            return source_instance.split_hat_id() == Some("ralph");
        }

        event
            .source
            .as_ref()
            .is_some_and(|source| source.as_str() == "ralph")
    }
}

fn default_topology_forbidden_responsibilities() -> Vec<String> {
    vec![
        "Do not create or spawn additional hats.".to_string(),
        "Do not publish topology.* or capability.* control-plane events.".to_string(),
        "Do not act as the Ralph coordinator or perform global task decomposition.".to_string(),
        "Do not call runtime capability catalog entries.".to_string(),
    ]
}
