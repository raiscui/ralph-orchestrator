//! `PromptExecutor` port 契约测试(Group 5 P2, audit-p3-p4.md 跟踪)。
//!
//! 背景:
//! `3ff4b47 refactor(core): EventLoop 收窄为 run() 窄入口 + PromptExecutor port`
//! 把进程执行从 `EventLoop` 抽到 `core::PromptExecutor` trait。适配层
//! `PtyPromptExecutor` (位于 `ralph-adapters`) 实现该 trait。本测试通过
//! `EventLoop::run` 的对外契约验证该 port 的不变量,确保后续 cherry-pick
//! 上游 loop work 或新 backend 适配时,不会破坏以下行为:
//!
//! 1. `on_iteration_started` 在每轮 `execute_prompt` 之前调用一次。
//! 2. `RunHooks::before_execute` / `after_execute` 与 `execute_prompt`
//!    形成「前/后」沙发布局,并能拿到正确的参数。
//! 3. `PromptOutput::canceled == true` 导致 `TerminationReason::Interrupted`。
//! 4. `PromptOutput::timed_out == true` 导致 `TerminationReason::Stopped`。
//!
//! 实现要点:
//! - `RecordingExecutor` 是一个 stub 实现,通过 `std::sync::Mutex` 持有
//!   共享状态(calls、iteration 计数、next response),可在线程间共享给
//!   `EventLoop::run`。
//! - hooks 通过 `Arc<Mutex<...>>` 在闭包间共享计数,允许 `'static` 借用。
//! - 每个测试用 `#[tokio::test]`(workspace tokio 已启用 macros feature)。
//!
//! 这组测试是**非破坏性**的:不引入新依赖,不修改生产路径,只断言已有契约。

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use ralph_core::{
    EventLoop, HatBackend, PromptExecutor, PromptOutput, RalphConfig, RunHooks,
    TerminationReason,
};
use ralph_proto::HatId;
use tokio::sync::watch;

// ---------------------------------------------------------------------------
// Stub executor
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct RecordedCall {
    iteration: u32,
    prompt: String,
    interactive: bool,
    hat_id: String,
}

struct RecordingExecutor {
    /// 每次 `execute_prompt` 都返回这一份 `PromptOutput`(测试间通过 new 注入)。
    next_response: Mutex<PromptOutput>,
    /// 累积的调用记录。
    calls: Mutex<Vec<RecordedCall>>,
    /// 由 `on_iteration_started` 维护,写入下一次 `execute_prompt` 的 RecordedCall。
    current_iteration: Mutex<u32>,
}

impl RecordingExecutor {
    fn new(next: PromptOutput) -> Self {
        Self {
            next_response: Mutex::new(next),
            calls: Mutex::new(Vec::new()),
            current_iteration: Mutex::new(0),
        }
    }

    fn calls(&self) -> Vec<RecordedCall> {
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait]
impl PromptExecutor for RecordingExecutor {
    fn on_iteration_started(&mut self, iteration: u32) {
        // EventLoop::run 顺序保证:on_iteration_started 总是先于 execute_prompt
        // 同一轮次被调用;确保 RecordedCall::iteration 与 EventLoop 暴露给 hook
        // 的 iteration 一致。
        *self.current_iteration.lock().unwrap() = iteration;
    }

    async fn execute_prompt(
        &mut self,
        prompt: &str,
        interactive: bool,
        hat_id: &HatId,
        _backend: Option<&HatBackend>,
        _interrupt_rx: watch::Receiver<bool>,
    ) -> anyhow::Result<PromptOutput> {
        let iter = *self.current_iteration.lock().unwrap();
        self.calls.lock().unwrap().push(RecordedCall {
            iteration: iter,
            prompt: prompt.to_string(),
            interactive,
            hat_id: hat_id.to_string(),
        });
        Ok(self.next_response.lock().unwrap().clone())
    }
}

// ---------------------------------------------------------------------------
// Minimal config — one hat, max_iterations = 1, no completion-promise triggers
// ---------------------------------------------------------------------------

fn minimal_config() -> RalphConfig {
    let yaml = r#"
event_loop:
  max_iterations: 1
  completion_promise: "<promise>done</promise>"
hats:
  planner:
    name: Planner
    triggers: ["task.start"]
    publishes: ["build.task"]
"#;
    serde_yaml::from_str(yaml)
        .expect("minimal_event_loop_config yaml should deserialize")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_executor_round_trip_with_event_loop_run() {
    // 检查:1 次迭代,success=true,before/after 各调用一次,execute_prompt 被调用
    // 且参数正确,after hook 看到 success=true 的 output。
    let mut event_loop = EventLoop::new(minimal_config());
    let mut executor = RecordingExecutor::new(PromptOutput {
        output: "iteration 1 ok".into(),
        success: true,
        ..PromptOutput::default()
    });
    let (_tx, rx) = watch::channel(false);

    let before_counter = Arc::new(Mutex::new(0u32));
    let after_counter = Arc::new(Mutex::new(0u32));
    let last_after: Arc<Mutex<Option<PromptOutput>>> = Arc::new(Mutex::new(None));

    let bc = Arc::clone(&before_counter);
    let ac = Arc::clone(&after_counter);
    let la = Arc::clone(&last_after);

    let reason = event_loop
        .run(
            &mut executor,
            rx,
            false,
            RunHooks {
                before_execute: Some(Box::new(move |_iteration, _hat, _prompt, _elapsed| {
                    *bc.lock().unwrap() += 1;
                })),
                after_execute: Some(Box::new(move |_iteration, _hat, out| {
                    *ac.lock().unwrap() += 1;
                    *la.lock().unwrap() = Some(out.clone());
                })),
            },
        )
        .await;

    assert_eq!(
        *before_counter.lock().unwrap(),
        1,
        "before_execute must fire exactly once per iteration; max_iterations=1"
    );
    assert_eq!(
        *after_counter.lock().unwrap(),
        1,
        "after_execute must fire exactly once per successful iteration"
    );
    let last = last_after
        .lock()
        .unwrap()
        .take()
        .expect("after_execute observed an output");
    assert!(last.success, "after hook sees success=true from stub");
    assert!(
        last.output.contains("iteration 1 ok"),
        "after hook sees the canned output text"
    );

    let calls = executor.calls();
    assert_eq!(calls.len(), 1, "execute_prompt fires once for max_iterations=1");
    assert_eq!(calls[0].iteration, 1, "first call carries iteration=1");
    // EventLoop::run 把协调用的 hat id(默认 "ralph")传给 executor;
    // 让 executor 知道是哪个 hat 在调度。被调度的子 hat 通过 prompt
    // 文本来表达(`display_hat`),不是 hat_id 参数本身。
    assert_eq!(
        calls[0].hat_id, "ralph",
        "execute_prompt is invoked with the coordinating hat (ralph), not the active sub-hat"
    );
    assert!(!calls[0].prompt.is_empty(), "execute_prompt receives a non-empty prompt");
    assert!(!calls[0].interactive, "interactive=false should propagate to executor");

    // 终止原因不固定(max_iterations、completion-promise detection、或
    // 1 次迭代无后续事件 → Stopped);只需确保 loop 跑完不 panic。
    let _ = reason;
}

#[tokio::test]
async fn test_canceled_propagates_to_interrupted_termination() {
    // 检查:`PromptOutput.canceled == true` → run 返回 Interrupted,
    // 且执行器仍然记录了 1 次调用(说明 EventLoop 没绕过 executor)。
    let mut event_loop = EventLoop::new(minimal_config());
    let mut executor = RecordingExecutor::new(PromptOutput {
        output: String::new(),
        canceled: true,
        ..PromptOutput::default()
    });
    let (_tx, rx) = watch::channel(false);

    let reason = event_loop
        .run(&mut executor, rx, false, RunHooks::none())
        .await;

    assert!(
        matches!(reason, TerminationReason::Interrupted),
        "canceled=true must propagate to TerminationReason::Interrupted, got {reason:?}"
    );

    let calls = executor.calls();
    assert_eq!(
        calls.len(),
        1,
        "execute_prompt is called once even when the response is canceled"
    );
    assert_eq!(calls[0].iteration, 1);
}

#[tokio::test]
async fn test_timed_out_propagates_to_stopped_termination() {
    // 检查:`PromptOutput.timed_out == true` → run 返回 Stopped。
    let mut event_loop = EventLoop::new(minimal_config());
    let mut executor = RecordingExecutor::new(PromptOutput {
        output: String::new(),
        timed_out: true,
        ..PromptOutput::default()
    });
    let (_tx, rx) = watch::channel(false);

    let reason = event_loop
        .run(&mut executor, rx, false, RunHooks::none())
        .await;

    assert!(
        matches!(reason, TerminationReason::Stopped),
        "timed_out=true must propagate to TerminationReason::Stopped, got {reason:?}"
    );

    let calls = executor.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].iteration, 1);
}
