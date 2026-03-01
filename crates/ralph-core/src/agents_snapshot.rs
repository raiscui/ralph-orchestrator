//! 并行运行态的 Agent/Instance 状态快照.
//!
//! 说明：
//! - 该快照用于 `ralph agents` 命令,让用户在另一个终端查看“当前有哪些实例,它们在做什么”。  
//! - 这不是核心协议的一部分,更像是运行时可观测性产物: `.ralph/agents.json`。
//! - 字段设计原则：
//!   - 只写“可公开且可审计”的摘要,避免把完整 prompt/payload 全量落盘造成噪音或泄露风险。
//!   - 保持 JSON 可读性,便于人类排障。

use ralph_proto::HatInstanceState;
use serde::{Deserialize, Serialize};

/// `.ralph/agents.json` 的顶层结构.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentsSnapshot {
    /// 快照生成时间(ISO 8601).
    pub generated_at: String,

    /// 当前已注册的实例列表.
    pub instances: Vec<AgentInstanceSnapshot>,
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

    /// 最近一次收到的输入事件摘要(用于回答“它在做什么”).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_input: Option<AgentLastInput>,
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
