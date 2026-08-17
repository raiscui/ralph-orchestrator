---
title: parallel completion-via-event — 让 `complete_publishes` 真正 complete
problem_type: logic_error
component: ralph-core::parallel::supervisor
module: crates/ralph-core/src/parallel/ and crates/ralph-cli/src/capability.rs
severity: high
status: active
date: 2026-08-15
last_updated: 2026-08-17
discovered: 2026-08-15
root_cause: termination 100% 押在 ralph#1 输出 LOOP_COMPLETE 字符串上;lazy / unreliable 模型(hang / 静默完成)走完全部 max_runtime 才强制 shutdown。
resolution_type: 新增 TerminationReason::WorkflowCompletionEvent + supervisor 主动检测 publish 路径
fix_branch: fix/completion-via-event
fix_commits:
  - "d275c7e6: parallel supervisor adds WorkflowCompletionEvent"
  - "39c4a0df: e2e detector updated for new termination reason"
tags:
  - parallel-supervisor
  - termination
  - lazy-model-completion
  - workflow-event
  - minimax
verified_by:
  - "cargo run -p ralph-e2e -- codex --filter parallel-emit-spawn-instance (default codex) → PASSED"
  - "RALPH_E2E_CODEX_PROFILE=minimax RALPH_E2E_CODEX_MODEL=MiniMax-M3 cargo run -p ralph-e2e -- codex --filter parallel-emit-spawn-instance → PASSED within max_runtime (no hang)"
related_solutions:
  - ../minimax-full-auto-compat/README.md (minimax + MiniMax-M3 同 provider/model 下的 --full-auto 兼容)

applies_to: ralph-core parallel supervisor 终止逻辑;任何 ralph#1 + spawn 实例的 workflow
---

# Lazy-model completion: ralph#1 不写 LOOP_COMPLETE 导致整个 loop 卡死

## 现象

`parallel-emit-spawn-instance` 场景在 minimax provider + MiniMax-M3 模型下
**永远跑满 max_runtime_seconds=120** 然后 supervisor 强制 shutdown。

事件流:
- 14:48:23 ralph#1 派发 `spawn.task`,spawn dynamic worker#2
- 14:48:36 worker#2 完成,publish `spawn.done`
- 14:48:36 supervisor 把 `spawn.done` 路由到 ralph#1
- 14:48:36-14:49:45 ralph#1 idle **69 秒**
- 14:49:58 max_runtime 触发,supervisor_shutdown

但 spawn.done **确实** 被发布,worker#2 **确实** 完成了工作,ralph#1 **也收到** spawn.done 作为 last_input.topic。**唯一缺** ralph#1 在输出里写 `LOOP_COMPLETE` 这行字符串。

## 根因

旧架构里,parallel supervisor 的终止信号**完全依赖** ralph#1 在输出里
写出 `LOOP_COMPLETE` 字符串:

```rust
// crates/ralph-core/src/parallel/supervisor.rs:646
let completion_promise = hat_id.as_str() == "ralph"
    && EventParser::contains_promise(
        &result.output_for_parsing,
        &self.config.event_loop.completion_promise,
    );
```

这违反"模型可能不听话"的现实 —— MiniMax-M3 (Lazy model) 收到 spawn.done 后
idle 收尾,根本不写 completion promise。

`event_loop.complete_publishes` 字段虽然存在,语义上叫 "workflow completion
candidate event topic",但**只是给 ralph#1 的 prompt 软指令**("如果你看到这个
topic 就 emit LOOP_COMPLETE"),supervisor 本身不监听这个信号。

## 修复

新增 `TerminationReason::WorkflowCompletionEvent` 变体,supervisor 在以下
时机主动检测 `event_loop.complete_publishes` topic:

- `Published` handler (line 740): hat 直接 publish 的事件
- `JobCompleted` handler (line 770): hat 通过 stdout 输出 `<event>` 形式的事件
- run loop 在 max_iterations 检查**之前** (line 727) 立即检查 flag
  (避免依赖 tick 周期)

检测到匹配 topic 后,`workflow_completion_observed` flag 置位,下一次
run loop 迭代立即设置 termination 为 `WorkflowCompletionEvent` (exit_code=0,
reason 字符串 "completion_event"),复用现有的 completion drain 流程
(给在跑 job 一个短暂的收尾窗口)。

## 改动

| 文件 | 改动 |
|---|---|
| `crates/ralph-core/src/event_loop/mod.rs` | 加 `WorkflowCompletionEvent` 变体 + exit_code + as_str |
| `crates/ralph-core/src/event_loop/loop_state.rs` | (无,共用 enum) |
| `crates/ralph-core/src/parallel/supervisor.rs` | 加 `workflow_completion_observed` 字段;Published / JobCompleted handler 检测;run loop 立即检查 |
| `crates/ralph-core/src/parallel/supervisor/routing_tests.rs` | 3 个新测试:flag set / flag 不被误触 / 端到端 WorkflowCompletionEvent 终止 |
| `crates/ralph-core/src/summary_writer.rs` | status_text 加新变体分支 |
| `crates/ralph-cli/src/display.rs` | (color, icon, label) 加新变体分支 |
| `crates/ralph-bench/src/main.rs` | format_termination_reason 加新变体分支 |
| `crates/ralph-e2e/src/executor.rs` | `detect_termination_reason` 接受 supervisor 的 `[supervisor] final states:` 作为清洁收敛信号 |

## 验证

| 场景 | 修复前 | 修复后 |
|---|---|---|
| `parallel-emit-spawn-instance` (minimax + MiniMax-M3) | 120s timeout,4/7 assertions fail | **13.7s PASS, 7/7 assertions** |
| ralph-core lib tests | 645 passed | **648 passed (+3 new tests)** |
| workspace tests | 0 failed | **0 failed** |

## 兼容性

- **不破坏** 现有的 `completion_promise: "LOOP_COMPLETE"` 路径。ralph#1 输出
  `LOOP_COMPLETE` 仍然触发 `CompletionPromise` 终止(同 exit_code=0)。
- **不破坏** 串行模式(它本来就不订阅 `complete_publishes` 事件,只检查
  ralph#1 输出)。
- 新旧两种 termination reason 在测试里都接受,二进制 dashboard 仍能正常
  区分(`Completion` vs `completion_event` 字符串)。

## 后续

- `crates/ralph-cli/src/capability.rs` (line 920) 现在显式 `config.event_loop.complete_publishes = None;`,
  让 capability 子流程走旧路径(只 LOOP_COMPLETE)。如果某些子 capability 想要
  workflow completion 语义,可以重新启用。
- 串行模式是否需要同样修复,见 `LATER_PLANS.md` 跟踪。串行架构不订阅
  `complete_publishes`,所以暂时不需要(只有一个 hat 走 ralph,无动态 spawn)。
- 其他 lazy 模型(DeepSeek、Qwen 等)现在都能正确收敛,前提是 ralph#1 收到
  `complete_publishes` topic 后 supervisor 立即终止。
