//! parent-visible topology spawn protocol records.
//!
//! 这个模块只负责描述 `topology.spawn_group` 这条运行时协议的结构化 payload。
//! 真正的实例创建、事件投递和 agents snapshot 写入,由 Supervisor 负责。

use crate::{RoleContract, RoleContractSummary};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

/// parent-visible group spawn 的控制面 topic。
pub const TOPIC_TOPOLOGY_SPAWN_GROUP: &str = "topology.spawn_group";

/// group spawn 成功或部分成功后的结果 topic。
pub const TOPIC_TOPOLOGY_SPAWN_RESULT: &str = "topology.spawn.result";

/// group spawn 完全失败时的结果 topic。
pub const TOPIC_TOPOLOGY_SPAWN_FAILED: &str = "topology.spawn.failed";

/// member 在 canonical role contract 校验阶段失败。
pub const TOPOLOGY_SPAWN_PHASE_MEMBER_VALIDATION_FAILED: &str = "member_validation_failed";

/// member 在实例注册/创建阶段失败。
pub const TOPOLOGY_SPAWN_PHASE_SPAWN_FAILED: &str = "spawn_failed";

/// member 已经创建实例,但首次 direct delivery 失败。
pub const TOPOLOGY_SPAWN_PHASE_DELIVERY_FAILED_AFTER_SPAWN: &str = "delivery_failed_after_spawn";

/// member 已经投递,但等待结果时超时。
pub const TOPOLOGY_SPAWN_PHASE_RESULT_TIMEOUT: &str = "result_timeout";

/// member 已经投递,但没有观察到允许的结果 topic。
pub const TOPOLOGY_SPAWN_PHASE_MISSING_RESULT: &str = "missing_result";

/// member 失败后被清理/回收并写入 tombstone。
pub const TOPOLOGY_SPAWN_PHASE_CLEANUP_REAPED_AFTER_FAILURE: &str = "cleanup_reaped_after_failure";

/// `topology.spawn_group` 的运行时请求 payload。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologySpawnGroupRequest {
    /// parent run 内的幂等键。
    pub request_id: String,
    /// 目标 hat id,例如 `builder`。
    pub hat: String,
    /// 新实例收到后应该处理的 delivery topic,例如 `build.task`。
    pub delivery_topic: String,
    /// 运行时输入的实例清单。
    pub instances: Vec<TopologySpawnMember>,
}

impl TopologySpawnGroupRequest {
    /// 解析并校验 `topology.spawn_group` payload。
    pub fn parse_payload(payload: &str) -> Result<Self, TopologySpawnGroupParseError> {
        let value = serde_json::from_str::<Value>(payload).map_err(|error| {
            TopologySpawnGroupParseError::new(
                None,
                None,
                None,
                format!("invalid JSON payload: {error}"),
            )
        })?;

        let request_id = string_field(&value, "request_id");
        let hat = string_field(&value, "hat");
        let delivery_topic = string_field(&value, "delivery_topic");

        let request_id_for_error = request_id.clone();
        let hat_for_error = hat.clone();
        let delivery_topic_for_error = delivery_topic.clone();

        let instances_value = value.get("instances").ok_or_else(|| {
            TopologySpawnGroupParseError::new(
                request_id_for_error.clone(),
                hat_for_error.clone(),
                delivery_topic_for_error.clone(),
                "missing field: instances".to_string(),
            )
        })?;

        let instances = parse_instances(
            instances_value,
            request_id_for_error.clone(),
            hat_for_error.clone(),
            delivery_topic_for_error.clone(),
        )?;

        let missing = [
            ("request_id", request_id.as_deref()),
            ("hat", hat.as_deref()),
            ("delivery_topic", delivery_topic.as_deref()),
        ]
        .into_iter()
        .filter_map(|(field, value)| value.is_none_or(str::is_empty).then_some(field))
        .collect::<Vec<_>>();

        if !missing.is_empty() {
            return Err(TopologySpawnGroupParseError::new(
                request_id,
                hat,
                delivery_topic,
                format!("missing or empty field(s): {}", missing.join(", ")),
            ));
        }

        if instances.is_empty() {
            return Err(TopologySpawnGroupParseError::new(
                request_id,
                hat,
                delivery_topic,
                "instances must not be empty".to_string(),
            ));
        }

        Ok(Self {
            request_id: request_id.expect("validated request_id"),
            hat: hat.expect("validated hat"),
            delivery_topic: delivery_topic.expect("validated delivery_topic"),
            instances,
        })
    }
}

fn string_field(value: &Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn optional_string_field(value: &Value, field: &str) -> Result<Option<String>, String> {
    match value.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(raw)) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                Ok(None)
            } else {
                Ok(Some(trimmed.to_owned()))
            }
        }
        Some(_) => Err(format!("field `{field}` must be a string when present")),
    }
}

fn optional_bool_field(value: &Value, field: &str) -> Result<Option<bool>, String> {
    match value.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(flag)) => Ok(Some(*flag)),
        Some(_) => Err(format!("field `{field}` must be a boolean when present")),
    }
}

fn optional_role_contract_field(value: &Value) -> Result<Option<RoleContract>, String> {
    match value.get("role_contract") {
        None | Some(Value::Null) => Ok(None),
        Some(raw) => serde_json::from_value::<RoleContract>(raw.clone())
            .map(Some)
            .map_err(|error| format!("field `role_contract` is invalid: {error}")),
    }
}

fn parse_instances(
    instances_value: &Value,
    request_id: Option<String>,
    hat: Option<String>,
    delivery_topic: Option<String>,
) -> Result<Vec<TopologySpawnMember>, TopologySpawnGroupParseError> {
    let Some(items) = instances_value.as_array() else {
        return Err(TopologySpawnGroupParseError::new(
            request_id,
            hat,
            delivery_topic,
            "field `instances` must be an array".to_string(),
        ));
    };

    let mut instances = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let Some(object) = item.as_object() else {
            return Err(TopologySpawnGroupParseError::new(
                request_id,
                hat,
                delivery_topic,
                format!("instances[{index}] must be an object"),
            ));
        };

        let item_value = Value::Object(object.clone());
        let role = string_field(&item_value, "role").ok_or_else(|| {
            TopologySpawnGroupParseError::new(
                request_id.clone(),
                hat.clone(),
                delivery_topic.clone(),
                format!("instances[{index}].role is missing or empty"),
            )
        })?;
        let task = string_field(&item_value, "task").ok_or_else(|| {
            TopologySpawnGroupParseError::new(
                request_id.clone(),
                hat.clone(),
                delivery_topic.clone(),
                format!("instances[{index}].task is missing or empty"),
            )
        })?;
        let input = optional_string_field(&item_value, "input").map_err(|error| {
            TopologySpawnGroupParseError::new(
                request_id.clone(),
                hat.clone(),
                delivery_topic.clone(),
                format!("instances[{index}]: {error}"),
            )
        })?;
        let fixed_role = optional_bool_field(&item_value, "fixed_role").map_err(|error| {
            TopologySpawnGroupParseError::new(
                request_id.clone(),
                hat.clone(),
                delivery_topic.clone(),
                format!("instances[{index}]: {error}"),
            )
        })?;
        let role_contract = optional_role_contract_field(&item_value).map_err(|error| {
            TopologySpawnGroupParseError::new(
                request_id.clone(),
                hat.clone(),
                delivery_topic.clone(),
                format!("instances[{index}]: {error}"),
            )
        })?;

        instances.push(TopologySpawnMember {
            role,
            task,
            input,
            fixed_role,
            role_contract,
        });
    }

    Ok(instances)
}

/// `topology.spawn_group` 的单个实例输入。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologySpawnMember {
    /// 人类输入的角色名,例如 `功能补充`.
    pub role: String,
    /// 该实例应该完成的工作说明.
    pub task: String,
    /// 可选的更长任务正文.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<String>,
    /// 可选的固定角色标记.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixed_role: Option<bool>,
    /// 可选 raw role contract hint。
    ///
    /// 注意：这里的 contract 只是 coordinator 提交的输入 hint。
    /// runtime 必须 validate + canonicalize 后生成 `EffectiveRoleContract`,
    /// downstream 不能直接消费这个 raw contract。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role_contract: Option<RoleContract>,
}

/// 运行时实际创建出来的实例摘要.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologySpawnedInstance {
    /// 请求中的下标,用于回溯输入顺序。
    pub index: usize,
    /// 新创建的实例 id,例如 `builder#2`.
    pub instance_id: String,
    /// 对应的运行时角色标签。
    pub role: String,
    /// 若请求成员被标记为固定角色,则保留该信息。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixed_role: Option<bool>,
    /// runtime canonical role contract 的轻量摘要。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role_contract_summary: Option<RoleContractSummary>,
}

/// 部分成功时,失败的成员摘要。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologySpawnFailedMember {
    /// 请求中的下标,用于回溯输入顺序。
    pub index: usize,
    /// 对应的运行时角色标签。
    pub role: String,
    /// parent-visible spawn request id,用于关联原始请求和 result evidence。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// 如果实例已经创建,这里保留运行时 instance id。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    /// 失败发生的运行时阶段。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    /// 失败原因。
    pub error: String,
    /// 给 coordinator / human 的最短恢复提示。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery_hint: Option<String>,
}

impl TopologySpawnFailedMember {
    /// 构造一个失败成员 evidence。
    ///
    /// 说明:
    /// - request / instance / phase / recovery 通过 builder 逐步补齐。
    /// - 这样旧 fixture 仍能反序列化,新 runtime 则写出更完整证据。
    pub fn new(index: usize, role: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            index,
            role: role.into(),
            request_id: None,
            instance_id: None,
            phase: None,
            error: error.into(),
            recovery_hint: None,
        }
    }

    #[must_use]
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    #[must_use]
    pub fn with_instance_id(mut self, instance_id: impl Into<String>) -> Self {
        self.instance_id = Some(instance_id.into());
        self
    }

    #[must_use]
    pub fn with_phase(mut self, phase: impl Into<String>) -> Self {
        self.phase = Some(phase.into());
        self
    }

    #[must_use]
    pub fn with_recovery_hint(mut self, recovery_hint: impl Into<String>) -> Self {
        self.recovery_hint = Some(recovery_hint.into());
        self
    }
}

/// `topology.spawn.result` payload。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologySpawnGroupResult {
    /// 结果状态,建议固定为 `spawned`。
    pub status: String,
    /// parent run 内的幂等键。
    pub request_id: String,
    /// 目标 hat id。
    pub hat: String,
    /// 下发给新实例的 topic。
    pub delivery_topic: String,
    /// 成功创建的实例。
    pub spawned: Vec<TopologySpawnedInstance>,
    /// 失败的成员。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failed: Vec<TopologySpawnFailedMember>,
    /// group spawn 之后父拓扑不再保持不变。
    pub parent_topology_unchanged: bool,
}

/// `topology.spawn.failed` payload。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologySpawnGroupFailed {
    /// 结果状态,建议固定为 `failed`。
    pub status: String,
    /// parent run 内的幂等键。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// 目标 hat id。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hat: Option<String>,
    /// 下发给新实例的 topic。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery_topic: Option<String>,
    /// 失败原因。
    pub error: String,
    /// 失败时通常仍然保持 true,除非已经部分成功但整体失败。
    pub parent_topology_unchanged: bool,
}

/// `topology.spawn_group` 解析失败时保留的上下文。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopologySpawnGroupParseError {
    /// 可用的请求 id.
    pub request_id: Option<String>,
    /// 可用的 hat id.
    pub hat: Option<String>,
    /// 可用的 delivery topic.
    pub delivery_topic: Option<String>,
    /// 错误摘要.
    pub error: String,
}

impl TopologySpawnGroupParseError {
    fn new(
        request_id: Option<String>,
        hat: Option<String>,
        delivery_topic: Option<String>,
        error: String,
    ) -> Self {
        Self {
            request_id,
            hat,
            delivery_topic,
            error,
        }
    }
}

impl fmt::Display for TopologySpawnGroupParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.error)
    }
}

impl std::error::Error for TopologySpawnGroupParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topology_spawn_group_request_parses_runtime_shape() {
        let request = TopologySpawnGroupRequest::parse_payload(
            r#"{
                "request_id":"create-three-evolution-hats-20260519-001",
                "hat":"builder",
                "delivery_topic":"build.task",
                "instances":[
                    {"role":"功能补充","task":"补充 feature A","input":"more context"},
                    {"role":"功能完善","task":"完善 feature B"},
                    {"role":"review","task":"review the proposal","fixed_role":true}
                ]
            }"#,
        )
        .expect("payload should parse");

        assert_eq!(
            request.request_id,
            "create-three-evolution-hats-20260519-001"
        );
        assert_eq!(request.hat, "builder");
        assert_eq!(request.delivery_topic, "build.task");
        assert_eq!(request.instances.len(), 3);
        assert_eq!(request.instances[0].role, "功能补充");
        assert_eq!(request.instances[0].input.as_deref(), Some("more context"));
        assert_eq!(request.instances[2].fixed_role, Some(true));
    }

    #[test]
    fn topology_spawn_group_request_parses_role_contract_hint() {
        let request = TopologySpawnGroupRequest::parse_payload(
            r#"{
                "request_id":"req-contract",
                "hat":"builder",
                "delivery_topic":"build.task",
                "instances":[
                    {
                        "role":"功能补充",
                        "task":"补充 feature A",
                        "role_contract":{
                            "role_name":"功能补充",
                            "objective":"raw objective hint",
                            "input_contract":"Handle the build.task payload.",
                            "output_contract":"Publish analysis.done.",
                            "allowed_topics":["analysis.done"],
                            "forbidden_responsibilities":["Do not spawn hats."],
                            "success_criteria":["Emit evidence."],
                            "identity_source":"task-derived"
                        }
                    }
                ]
            }"#,
        )
        .expect("payload should parse");

        let contract = request.instances[0]
            .role_contract
            .as_ref()
            .expect("role contract hint should parse");
        assert_eq!(contract.role_name, "功能补充");
        assert_eq!(contract.allowed_topics, vec!["analysis.done"]);
        assert_eq!(contract.identity_source, crate::IdentitySource::TaskDerived);
    }

    #[test]
    fn topology_spawn_group_request_rejects_invalid_role_contract_identity_source() {
        let error = TopologySpawnGroupRequest::parse_payload(
            r#"{
                "request_id":"req-contract",
                "hat":"builder",
                "delivery_topic":"build.task",
                "instances":[
                    {
                        "role":"功能补充",
                        "task":"补充 feature A",
                        "role_contract":{
                            "role_name":"功能补充",
                            "objective":"raw objective hint",
                            "input_contract":"Handle the build.task payload.",
                            "output_contract":"Publish analysis.done.",
                            "allowed_topics":["analysis.done"],
                            "forbidden_responsibilities":[],
                            "success_criteria":[],
                            "identity_source":"not-a-real-source"
                        }
                    }
                ]
            }"#,
        )
        .unwrap_err();

        assert!(error.error.contains("role_contract"));
        assert!(error.error.contains("unknown variant"));
    }

    #[test]
    fn topology_spawn_group_request_missing_request_id_fails() {
        let error = TopologySpawnGroupRequest::parse_payload(
            r#"{
                "hat":"builder",
                "delivery_topic":"build.task",
                "instances":[{"role":"功能补充","task":"补充 feature A"}]
            }"#,
        )
        .unwrap_err();

        assert_eq!(error.request_id, None);
        assert_eq!(error.hat.as_deref(), Some("builder"));
        assert!(error.error.contains("request_id"));
    }

    #[test]
    fn topology_spawn_group_request_rejects_empty_instances() {
        let error = TopologySpawnGroupRequest::parse_payload(
            r#"{
                "request_id":"req-1",
                "hat":"builder",
                "delivery_topic":"build.task",
                "instances":[]
            }"#,
        )
        .unwrap_err();

        assert_eq!(error.request_id.as_deref(), Some("req-1"));
        assert!(error.error.contains("instances must not be empty"));
    }

    #[test]
    fn topology_spawn_group_result_serializes_partial_failure() {
        let result = TopologySpawnGroupResult {
            status: "partial".to_string(),
            request_id: "req-1".to_string(),
            hat: "builder".to_string(),
            delivery_topic: "build.task".to_string(),
            spawned: vec![TopologySpawnedInstance {
                index: 0,
                instance_id: "builder#2".to_string(),
                role: "功能补充".to_string(),
                fixed_role: Some(true),
                role_contract_summary: None,
            }],
            failed: vec![
                TopologySpawnFailedMember::new(1, "review", "spawn failed")
                    .with_request_id("req-1")
                    .with_phase(TOPOLOGY_SPAWN_PHASE_SPAWN_FAILED)
                    .with_recovery_hint(
                        "Fix target hat configuration and retry the failed member.",
                    ),
            ],
            parent_topology_unchanged: false,
        };

        let json = serde_json::to_string(&result).expect("serializes");
        assert!(json.contains("\"status\":\"partial\""));
        assert!(json.contains("\"builder#2\""));
        assert!(json.contains("\"spawn failed\""));
        assert!(json.contains("\"phase\":\"spawn_failed\""));
        assert!(json.contains("\"request_id\":\"req-1\""));
    }
}
