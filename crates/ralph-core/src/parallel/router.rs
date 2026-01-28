//! TopicContract 解析与路由计划生成。
//!
//! 说明：
//! - 这里先解决“给定 topic -> 找到对应 TopicContract”的问题
//! - 具体的 recipients 计算（audience_override、queue/fanout 等）在 supervisor 里完成

use ralph_proto::{Topic, TopicContract};
use std::collections::HashMap;

/// 路由相关错误。
#[derive(Debug, thiserror::Error)]
pub enum RouteError {
    #[error("No TopicContract matched topic: {0}")]
    MissingContract(String),
}

/// 解析后的 TopicContract 存储（支持 pattern 匹配）。
#[derive(Debug, Clone)]
pub struct TopicContractStore {
    // 说明：
    // - 使用 Vec 保持顺序可控（排序后“最匹配的”排在前面）。
    // - key 用 ralph_proto::Topic 复用其 glob 匹配逻辑。
    patterns: Vec<(Topic, TopicContract)>,
}

impl TopicContractStore {
    /// 从配置加载 TopicContract map。
    pub fn new(contracts: &HashMap<String, TopicContract>) -> Self {
        let mut patterns: Vec<(Topic, TopicContract)> = contracts
            .iter()
            .map(|(k, v)| (Topic::new(k), v.clone()))
            .collect();

        // 排序：更具体的 pattern 优先。
        // 规则（从高到低）：
        // 1) 非 global wildcard 优先
        // 2) wildcard 越少越具体
        // 3) pattern 越长越具体（同 wildcard 数量时）
        patterns.sort_by(|(a, _), (b, _)| {
            let a_key = Self::specificity_key(a.as_str());
            let b_key = Self::specificity_key(b.as_str());
            a_key.cmp(&b_key)
        });

        Self { patterns }
    }

    /// 解析一个 topic 对应的 TopicContract（按最具体匹配）。
    pub fn resolve(&self, topic: &str) -> Result<&TopicContract, RouteError> {
        self.patterns
            .iter()
            .find(|(pattern, _)| pattern.matches_str(topic))
            .map(|(_, contract)| contract)
            .ok_or_else(|| RouteError::MissingContract(topic.to_string()))
    }

    fn specificity_key(pattern: &str) -> (u8, usize, usize) {
        let is_global = pattern == "*";
        let wildcard_count = pattern.split('.').filter(|p| *p == "*").count();

        // sort_by 默认升序，因此这里把“更具体”映射成更小的 key。
        (
            u8::from(is_global),
            wildcard_count,
            // 越长越具体，因此取反让更长更靠前
            usize::MAX - pattern.len(),
        )
    }
}
