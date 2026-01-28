//! Human gate 状态机（最小可用）。
//!
//! 说明：
//! - gate.request / gate.resolve / gate.timeout 都是“事件协议”；
//! - GateManager 只负责：记录 open gates、计算 timeout、以及把 resolve 回送给请求者。
//! - 具体的“UI 怎么展示 / human 怎么输入”在 ralph-cli/后续任务里逐步补齐。

use ralph_proto::{GateRequest, GateResolve, GateTimeout, HatInstanceId};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// gate 的运行态记录。
#[derive(Debug, Clone)]
struct OpenGate {
    request: GateRequest,
    deadline: Option<Instant>,
    timeout_emitted: bool,
}

/// gate 管理器（状态机）。
#[derive(Debug, Default)]
pub(super) struct GateManager {
    open: HashMap<String, OpenGate>,
}

impl GateManager {
    pub(super) fn new() -> Self {
        Self {
            open: HashMap::new(),
        }
    }

    /// 注册一个 gate.request。
    ///
    /// 返回值：
    /// - Ok(true)：首次注册
    /// - Ok(false)：gate_id 已存在（本次被忽略）
    pub(super) fn register(&mut self, request: GateRequest) -> anyhow::Result<bool> {
        if self.open.contains_key(&request.gate_id) {
            return Ok(false);
        }

        let deadline = request
            .timeout_seconds
            .filter(|secs| *secs > 0)
            .map(|secs| Instant::now() + Duration::from_secs(secs));

        self.open.insert(
            request.gate_id.clone(),
            OpenGate {
                request,
                deadline,
                timeout_emitted: false,
            },
        );

        Ok(true)
    }

    /// 处理 gate.resolve：移除 open gate，并返回应该回送给哪个实例。
    pub(super) fn resolve(&mut self, resolve: &GateResolve) -> Option<HatInstanceId> {
        self.open
            .remove(&resolve.gate_id)
            .map(|g| g.request.requested_by)
            .or_else(|| resolve.requested_by.clone())
    }

    /// 轮询 timeout：返回所有“首次超时”的 gate.timeout payload。
    ///
    /// 说明：
    /// - timeout 只会返回一次（通过 timeout_emitted 去重）
    /// - gate 仍保持 open，直到收到 gate.resolve
    pub(super) fn poll_timeouts(&mut self) -> Vec<GateTimeout> {
        self.poll_timeouts_at(Instant::now())
    }

    /// 测试友好版本：允许注入 now，避免依赖真实时间与 tokio test-util。
    pub(super) fn poll_timeouts_at(&mut self, now: Instant) -> Vec<GateTimeout> {
        let mut timeouts = Vec::new();

        for gate in self.open.values_mut() {
            let Some(deadline) = gate.deadline else {
                continue;
            };
            if gate.timeout_emitted {
                continue;
            }
            if now < deadline {
                continue;
            }

            gate.timeout_emitted = true;
            timeouts.push(GateTimeout {
                gate_id: gate.request.gate_id.clone(),
                requested_by: Some(gate.request.requested_by.clone()),
            });
        }

        timeouts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ralph_proto::{GateKind, GateResolvedBy};

    fn request_with_timeout(id: &str, secs: u64) -> GateRequest {
        GateRequest {
            gate_id: id.to_string(),
            thread_id: None,
            requested_by: HatInstanceId::from("writer#1"),
            kind: GateKind::Approval,
            timeout_seconds: Some(secs),
            prompt: "approve?".to_string(),
            proposed_default: Some("approve".to_string()),
        }
    }

    #[test]
    fn test_timeout_emits_once() {
        let mut gates = GateManager::new();
        gates.register(request_with_timeout("g1", 10)).unwrap();

        assert!(gates.poll_timeouts_at(Instant::now()).is_empty());

        let timeouts = gates.poll_timeouts_at(Instant::now() + Duration::from_secs(11));
        assert_eq!(timeouts.len(), 1);
        assert_eq!(timeouts[0].gate_id, "g1");

        // 再 poll 不应重复返回
        assert!(
            gates
                .poll_timeouts_at(Instant::now() + Duration::from_secs(12))
                .is_empty()
        );
    }

    #[test]
    fn test_resolve_removes_gate() {
        let mut gates = GateManager::new();
        gates.register(request_with_timeout("g1", 10)).unwrap();

        let resolved = GateResolve {
            gate_id: "g1".to_string(),
            resolved_by: GateResolvedBy::Human,
            decision: serde_json::Value::String("approve".to_string()),
            requested_by: None,
        };

        let requested_by = gates.resolve(&resolved);
        assert_eq!(requested_by, Some(HatInstanceId::from("writer#1")));
        assert!(
            gates
                .poll_timeouts_at(Instant::now() + Duration::from_secs(12))
                .is_empty()
        );
    }
}
