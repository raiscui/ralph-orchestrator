//! 并行运行态的 Agent/Instance 状态快照.
//!
//! 说明：
//! - 该快照用于 `ralph agents` 命令,让用户在另一个终端查看“当前有哪些实例,它们在做什么”。  
//! - 这不是核心协议的一部分,更像是运行时可观测性产物: `.ralph/agents.json`。
//! - 字段设计原则：
//!   - 只写“可公开且可审计”的摘要,避免把完整 prompt/payload 全量落盘造成噪音或泄露风险。
//!   - 保持 JSON 可读性,便于人类排障。

use crate::prompt_surface::{IdentitySource, RoleContractSummary};
use ralph_proto::HatInstanceState;
use serde::{Deserialize, Serialize};

/// `.ralph/agents.json` 的顶层结构.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentsSnapshot {
    /// 快照生成时间(ISO 8601).
    pub generated_at: String,

    /// 当前已注册的实例列表.
    ///
    /// 注意：
    /// - 这里表达的是 current registry,也就是“仍可被 runtime 投递/观察到”的实例。
    /// - 已完成并被动态回收的实例不应混回这里,否则用户会误以为还能继续投递。
    pub instances: Vec<AgentInstanceSnapshot>,

    /// 已完成并从 current registry 回收的动态实例 tombstone。
    ///
    /// 说明：
    /// - 这是观察面历史区,不是可投递实例列表。
    /// - 用于解释“record-session 里证明它跑过,但 current instances 里已经看不到”的情况。
    /// - 仍然只保存 summary-only 字段,不落完整 prompt / raw role contract。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub completed_dynamic_instances: Vec<AgentCompletedDynamicInstanceSnapshot>,

    /// isolated child run 的轻量观测摘要。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub child_runs: Vec<AgentChildRunSnapshot>,
}

/// 单个 hat instance 的状态摘要.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInstanceSnapshot {
    /// 运行时实例 id,例如 `writer#1`.
    pub instance_id: String,

    /// hat id,例如 `writer`.
    pub hat_id: String,

    /// 生命周期状态.
    pub state: HatInstanceState,

    /// 是否为动态实例(autoscale 或显式 spawn 产生).
    pub is_dynamic: bool,

    /// 身份来源,用于区分静态配置、模板派生、任务派生和运行时扩容.
    #[serde(default = "default_identity_source")]
    pub identity_source: IdentitySource,

    /// 固定角色标签。仅当 coordinator 显式把运行时角色提升为固定角色时写入。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixed_role_label: Option<String>,

    /// 固定角色写入原因。用于解释为什么这个角色不是一次性临时视角。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixed_role_reason: Option<String>,

    /// task-derived dynamic role contract 的轻量摘要。
    ///
    /// 注意：
    /// - 这里只放 hash / 来源 / preview 等可审计摘要。
    /// - 不写完整 prompt,也不写完整 raw spawn payload。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role_contract_summary: Option<RoleContractSummary>,

    /// 最近一次收到的输入事件摘要(用于回答“它在做什么”).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_input: Option<AgentLastInput>,

    /// 当前实例相关的 recoverable agent CLI failure 摘要。
    ///
    /// 注意：
    /// - 这是从 `.ralph/recoverable-failures.jsonl` / live runtime map 派生的观察面。
    /// - 这里只保存 compact metadata,不保存 prompt、原始 event stream 或完整 stderr。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recoverable_failures: Vec<AgentRecoverableFailureSummary>,
}

fn default_identity_source() -> IdentitySource {
    IdentitySource::ConfigDerived
}

/// 已完成动态实例的 tombstone 摘要。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCompletedDynamicInstanceSnapshot {
    /// 运行时实例 id,例如 `builder#4`.
    pub instance_id: String,

    /// hat id,例如 `builder`.
    pub hat_id: String,

    /// 最终生命周期状态。通常是 `done`,但保留字段避免未来扩展失败态回收。
    pub final_state: HatInstanceState,

    /// 身份来源,用于继续区分 task-derived / template-derived / autoscale 等来源。
    #[serde(default = "default_identity_source")]
    pub identity_source: IdentitySource,

    /// 固定角色标签。仅当 coordinator 显式提升为固定角色时写入。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixed_role_label: Option<String>,

    /// 固定角色写入原因。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixed_role_reason: Option<String>,

    /// task-derived dynamic role contract 的轻量摘要。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role_contract_summary: Option<RoleContractSummary>,

    /// 最近一次收到的输入事件摘要。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_input: Option<AgentLastInput>,

    /// 被写入 tombstone 的时间(ISO 8601)。
    pub completed_at: String,

    /// 为什么从 current registry 中移除。
    pub retirement_reason: String,
}

/// 最近一次输入事件摘要.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLastInput {
    /// 记录时间(ISO 8601).
    pub ts: String,

    /// 输入事件 topic.
    pub topic: String,

    /// 输入内容预览(截断后的单行文本).
    pub preview: String,
}

/// 单个 recoverable failure lifecycle 的人类可读摘要。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRecoverableFailureSummary {
    /// Recoverable lifecycle id。
    pub failure_id: String,

    /// Runtime job id。
    pub job_id: u64,

    /// 生命周期状态,例如 `retry_scheduled` / `continued_by_human` / `exhausted`。
    pub status: String,

    /// 确定性分类,例如 `rate_limited`。
    pub failure_kind: String,

    /// 当前尝试序号。
    pub attempt: u32,

    /// 最大尝试次数。
    pub max_attempts: u32,

    /// 下一次 retry 延迟毫秒数。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_after_ms: Option<u64>,

    /// 下一次 retry 的绝对时间。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_retry_at: Option<String>,

    /// compact ledger evidence path。
    pub ledger_path: String,

    /// 有界 stderr 摘要预览。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stderr_preview: Option<String>,
}

/// isolated child/micro-run 的状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentChildRunStatus {
    /// 已请求并开始执行。
    Running,
    /// 子运行已成功结束。
    Done,
    /// 子运行失败。
    Failed,
}

impl AgentChildRunStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Done => "done",
            Self::Failed => "failed",
        }
    }
}

/// `ralph agents` 使用的 child-run 轻量摘要。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentChildRunSnapshot {
    /// parent request id。
    pub request_id: String,

    /// child invocation id。请求刚开始时可能还未知。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invocation_id: Option<String>,

    /// capability id。
    pub capability_id: String,

    /// 当前状态。
    pub status: AgentChildRunStatus,

    /// 最新摘要,成功时通常是 result summary,失败时通常是 error。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,

    /// 主要证据路径,例如 result.json / failed.json / invoke.json。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact: Option<String>,

    /// 最近更新时间。
    pub updated_at: String,
}
