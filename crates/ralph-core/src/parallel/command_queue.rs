//! 并行运行时的 in-process command queue(借鉴 openclaw).
//!
//! 目标:
//! - 用 lane 把 orchestrator 自己执行的“副作用动作”串行化/隔离(例如 workspace/git).
//! - 用 generation 在 reset/early-exit 后忽略 stale release,避免计数卡死.
//! - 用 draining 明确拒绝新任务,避免“排队了但进程退出时被静默 kill”.

use anyhow::Context;
use std::collections::VecDeque;
use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, oneshot};

/// 默认 lane(保持“无显式选择时”的兼容行为)。
pub const COMMAND_LANE_MAIN: &str = "main";

/// workspace/git lane: 用于会修改主仓库 git 元数据的动作(例如 worktree add/remove)。
pub const COMMAND_LANE_WORKSPACE_GIT: &str = "workspace.git";

/// 当排队中的 waiter 被 clear_lane() 清理时返回的错误。
#[derive(Debug, Clone)]
pub struct CommandLaneClearedError {
    lane: Option<String>,
}

impl CommandLaneClearedError {
    pub fn new(lane: impl Into<Option<String>>) -> Self {
        Self { lane: lane.into() }
    }
}

impl fmt::Display for CommandLaneClearedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.lane {
            Some(lane) => write!(f, "Command lane \"{lane}\" cleared"),
            None => write!(f, "Command lane cleared"),
        }
    }
}

impl std::error::Error for CommandLaneClearedError {}

/// 当队列处于 draining 状态时,新 acquire 会被拒绝的错误。
#[derive(Debug, Clone)]
pub struct GatewayDrainingError;

impl fmt::Display for GatewayDrainingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Gateway is draining; new tasks are not accepted")
    }
}

impl std::error::Error for GatewayDrainingError {}

/// 并行运行时 command queue。
///
/// 说明:
/// - 这是一个 in-process 的最小队列,不做持久化.
/// - acquire-based: 调用方拿到 permit 后在临界区内执行任意 async 操作(无需把 task 变成 'static closure)。
#[derive(Debug, Clone, Default)]
pub struct CommandQueue {
    inner: Arc<Mutex<Inner>>,
}

#[derive(Debug, Default)]
struct Inner {
    gateway_draining: bool,
    lanes: std::collections::HashMap<String, LaneState>,
}

#[derive(Debug)]
struct LaneState {
    lane: String,
    max_concurrent: usize,
    generation: u64,
    active: usize,
    waiters: VecDeque<LaneWaiter>,
}

#[derive(Debug)]
struct LaneWaiter {
    enqueued_at: Instant,
    warn_after: Duration,
    tx: oneshot::Sender<anyhow::Result<PermitToken>>,
}

#[derive(Debug, Clone)]
struct PermitToken {
    lane: String,
    generation: u64,
}

/// lane permit: drop 时自动 release 并唤醒后续 waiter。
#[derive(Debug)]
pub struct CommandLanePermit {
    queue: CommandQueue,
    lane: String,
    generation: u64,
    released: bool,
}

impl CommandQueue {
    pub fn new() -> Self {
        Self::default()
    }

    /// 标记全局 draining: 后续新 acquire 会失败,避免“排队后被静默 kill”。
    pub async fn mark_draining(&self) {
        let mut inner = self.inner.lock().await;
        inner.gateway_draining = true;
    }

    /// 清除全局 draining 标记(通常用于 reset 后恢复)。
    pub async fn clear_draining(&self) {
        let mut inner = self.inner.lock().await;
        inner.gateway_draining = false;
    }

    /// 设置某个 lane 的并发度(最小为 1)。
    pub async fn set_lane_concurrency(&self, lane: &str, max_concurrent: usize) {
        let resolved = normalize_lane(lane);
        let mut inner = self.inner.lock().await;
        let state = get_lane_state_mut(&mut inner, &resolved);
        state.max_concurrent = max_concurrent.max(1);
        drain_lane_locked(state);
    }

    /// 读取某个 lane 的队列规模(active + queued),用于测试与诊断。
    pub async fn get_queue_size(&self, lane: &str) -> usize {
        let resolved = normalize_lane(lane);
        let inner = self.inner.lock().await;
        inner
            .lanes
            .get(&resolved)
            .map(|s| s.active + s.waiters.len())
            .unwrap_or(0)
    }

    /// 读取所有 lane 的总队列规模(active + queued)。
    pub async fn get_total_queue_size(&self) -> usize {
        let inner = self.inner.lock().await;
        inner
            .lanes
            .values()
            .map(|s| s.active + s.waiters.len())
            .sum()
    }

    /// 清空某个 lane 的排队 waiter(不会影响已持有 permit 的 active)。
    ///
    /// 返回值: 被拒绝的 waiter 数量。
    pub async fn clear_lane(&self, lane: &str) -> usize {
        let resolved = normalize_lane(lane);
        let mut inner = self.inner.lock().await;
        let Some(state) = inner.lanes.get_mut(&resolved) else {
            return 0;
        };

        let removed = state.waiters.len();
        let pending = std::mem::take(&mut state.waiters);
        for waiter in pending {
            let _ = waiter
                .tx
                .send(Err(anyhow::Error::new(CommandLaneClearedError::new(Some(
                    resolved.clone(),
                )))));
        }
        removed
    }

    /// Reset 所有 lane 的运行态(用于“极端情况下”的自救):
    ///
    /// - bump generation,忽略旧 permit 的 stale release.
    /// - 清空 active 计数,避免 stale active 永久卡死 drain.
    /// - 保留 waiters 队列,并在 reset 后立刻 drain.
    pub async fn reset_all_lanes(&self) {
        let mut inner = self.inner.lock().await;
        inner.gateway_draining = false;

        let mut lanes_to_drain = Vec::new();
        for state in inner.lanes.values_mut() {
            state.generation = state.generation.saturating_add(1);
            state.active = 0;
            if !state.waiters.is_empty() {
                lanes_to_drain.push(state.lane.clone());
            }
        }

        for lane in lanes_to_drain {
            if let Some(state) = inner.lanes.get_mut(&lane) {
                drain_lane_locked(state);
            }
        }
    }

    /// acquire 某个 lane 的 permit.
    ///
    /// - 当 lane 未饱和时立即返回.
    /// - 当 lane 饱和时排队等待.
    /// - draining=true 时,新 acquire 会被拒绝.
    pub async fn acquire(&self, lane: &str) -> anyhow::Result<CommandLanePermit> {
        let resolved = normalize_lane(lane);

        // --------------------------
        // fast path: 立即拿到 permit
        // --------------------------
        {
            let mut inner = self.inner.lock().await;
            if inner.gateway_draining {
                return Err(anyhow::Error::new(GatewayDrainingError));
            }

            let state = get_lane_state_mut(&mut inner, &resolved);
            if state.active < state.max_concurrent && state.waiters.is_empty() {
                state.active += 1;
                return Ok(CommandLanePermit {
                    queue: self.clone(),
                    lane: resolved,
                    generation: state.generation,
                    released: false,
                });
            }
        }

        // --------------------------
        // slow path: 排队等待 drain
        // --------------------------
        let (tx, rx) = oneshot::channel::<anyhow::Result<PermitToken>>();
        {
            let mut inner = self.inner.lock().await;
            if inner.gateway_draining {
                return Err(anyhow::Error::new(GatewayDrainingError));
            }

            let state = get_lane_state_mut(&mut inner, &resolved);
            state.waiters.push_back(LaneWaiter {
                enqueued_at: Instant::now(),
                warn_after: Duration::from_secs(2),
                tx,
            });
            drain_lane_locked(state);
        }

        let token = rx
            .await
            .context("CommandQueue waiter canceled before permit granted")??;

        Ok(CommandLanePermit {
            queue: self.clone(),
            lane: token.lane,
            generation: token.generation,
            released: false,
        })
    }

    async fn release(&self, lane: &str, generation: u64) {
        let resolved = normalize_lane(lane);
        let mut inner = self.inner.lock().await;
        let Some(state) = inner.lanes.get_mut(&resolved) else {
            return;
        };

        // generation 不匹配: 视为 stale release,直接忽略.
        if state.generation != generation {
            return;
        }

        state.active = state.active.saturating_sub(1);
        drain_lane_locked(state);
    }
}

impl CommandLanePermit {
    /// 主动释放 permit(用于测试或需要确定释放时序的场景)。
    pub async fn release(mut self) {
        if self.released {
            return;
        }
        self.released = true;
        self.queue.release(&self.lane, self.generation).await;
    }
}

impl Drop for CommandLanePermit {
    fn drop(&mut self) {
        if self.released {
            return;
        }

        // Drop 不能 await,因此用 spawn 异步释放 permit.
        let queue = self.queue.clone();
        let lane = self.lane.clone();
        let generation = self.generation;
        self.released = true;

        tokio::spawn(async move {
            queue.release(&lane, generation).await;
        });
    }
}

fn normalize_lane(lane: &str) -> String {
    let cleaned = lane.trim();
    if cleaned.is_empty() {
        COMMAND_LANE_MAIN.to_string()
    } else {
        cleaned.to_string()
    }
}

fn get_lane_state_mut<'a>(inner: &'a mut Inner, lane: &str) -> &'a mut LaneState {
    if !inner.lanes.contains_key(lane) {
        inner.lanes.insert(
            lane.to_string(),
            LaneState {
                lane: lane.to_string(),
                max_concurrent: 1,
                generation: 0,
                active: 0,
                waiters: VecDeque::new(),
            },
        );
    }
    inner
        .lanes
        .get_mut(lane)
        .expect("lane state must exist after insertion")
}

fn drain_lane_locked(state: &mut LaneState) {
    while state.active < state.max_concurrent {
        let Some(waiter) = state.waiters.pop_front() else {
            break;
        };

        let waited_ms = waiter.enqueued_at.elapsed().as_millis();
        if waiter.enqueued_at.elapsed() >= waiter.warn_after {
            tracing::warn!(
                lane = %state.lane,
                waited_ms,
                queued_ahead = state.waiters.len(),
                "command lane wait exceeded"
            );
        }

        let token = PermitToken {
            lane: state.lane.clone(),
            generation: state.generation,
        };

        match waiter.tx.send(Ok(token)) {
            Ok(()) => {
                state.active += 1;
            }
            Err(_dropped) => {
                // waiter 已被取消(Receiver drop),直接忽略并继续 drain.
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::sync::Notify;

    #[tokio::test]
    async fn default_lane_serializes_acquire() {
        let queue = CommandQueue::new();

        let active = Arc::new(AtomicUsize::new(0));
        let max_active = Arc::new(AtomicUsize::new(0));
        let notify = Arc::new(Notify::new());

        let t1 = {
            let queue = queue.clone();
            let active = Arc::clone(&active);
            let max_active = Arc::clone(&max_active);
            let notify = Arc::clone(&notify);
            tokio::spawn(async move {
                let _permit = queue
                    .acquire(COMMAND_LANE_WORKSPACE_GIT)
                    .await
                    .expect("permit");
                notify.notify_one();
                let current = active.fetch_add(1, Ordering::SeqCst) + 1;
                max_active.fetch_max(current, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(200)).await;
                active.fetch_sub(1, Ordering::SeqCst);
            })
        };

        // 确保 t1 已持有 permit,再启动 t2(否则测试可能变成“先后两次都立即拿到”,看不出串行性)。
        notify.notified().await;

        let t2 = {
            let queue = queue.clone();
            let active = Arc::clone(&active);
            let max_active = Arc::clone(&max_active);
            tokio::spawn(async move {
                let _permit = queue
                    .acquire(COMMAND_LANE_WORKSPACE_GIT)
                    .await
                    .expect("permit");
                let current = active.fetch_add(1, Ordering::SeqCst) + 1;
                max_active.fetch_max(current, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(50)).await;
                active.fetch_sub(1, Ordering::SeqCst);
            })
        };

        t1.await.expect("t1 join");
        t2.await.expect("t2 join");

        assert_eq!(
            max_active.load(Ordering::SeqCst),
            1,
            "Expected same-lane acquire to be serialized"
        );
    }

    #[tokio::test]
    async fn mark_draining_rejects_new_acquire() {
        let queue = CommandQueue::new();
        queue.mark_draining().await;

        let err = queue
            .acquire(COMMAND_LANE_MAIN)
            .await
            .expect_err("acquire should be rejected while draining");
        assert!(
            err.downcast_ref::<GatewayDrainingError>().is_some(),
            "Expected GatewayDrainingError, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn clear_lane_rejects_queued_waiters() {
        let queue = CommandQueue::new();

        let permit = queue
            .acquire(COMMAND_LANE_WORKSPACE_GIT)
            .await
            .expect("permit");

        let queue2 = queue.clone();
        let waiter = tokio::spawn(async move { queue2.acquire(COMMAND_LANE_WORKSPACE_GIT).await });

        // 等待 waiter 入队(队列规模应为: active=1 + queued=1)。
        for _ in 0..50 {
            if queue.get_queue_size(COMMAND_LANE_WORKSPACE_GIT).await >= 2 {
                break;
            }
            tokio::task::yield_now().await;
        }

        let removed = queue.clear_lane(COMMAND_LANE_WORKSPACE_GIT).await;
        assert_eq!(removed, 1, "Expected one queued waiter to be cleared");

        drop(permit);

        let err = waiter
            .await
            .expect("waiter join")
            .expect_err("waiter should be rejected after clear_lane");
        assert!(
            err.downcast_ref::<CommandLaneClearedError>().is_some(),
            "Expected CommandLaneClearedError, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn reset_all_lanes_ignores_stale_release() {
        let queue = CommandQueue::new();

        // 先拿到一个旧 generation 的 permit(不释放,用于模拟 stale release)。
        let old = queue
            .acquire(COMMAND_LANE_WORKSPACE_GIT)
            .await
            .expect("permit");

        // reset: bump generation + active=0,此时新 acquire 应立即成功。
        queue.reset_all_lanes().await;

        let new = queue
            .acquire(COMMAND_LANE_WORKSPACE_GIT)
            .await
            .expect("new permit");

        // drop old: stale release 不应影响新 generation 的 active 计数。
        drop(old);
        tokio::task::yield_now().await;

        // 由于 new 仍持有 permit,第三个 acquire 在很短时间内应拿不到。
        let blocked = tokio::time::timeout(
            Duration::from_millis(50),
            queue.acquire(COMMAND_LANE_WORKSPACE_GIT),
        )
        .await;
        assert!(
            blocked.is_err(),
            "Expected third acquire to be blocked while new permit is held"
        );

        drop(new);
    }
}
