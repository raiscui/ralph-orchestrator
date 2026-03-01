---
name: self-learning.ralph-codex-app-server-steer-live-reply-two-turn
description: |
  修复/规避在 Ralph 并行模式 + 真实 Codex app-server 下验证 turn/steer 时出现的“有 ACK 但看起来无回复”的不稳定问题。
  适用场景: (1) `turn/steer` send/recv 正常,但 stdout/human-log 没有 `answer`/`LOOP_COMPLETE`;
  (2) 输出主要是 reasoning/summary delta,导致你无法判断是否真的回复;
  (3) 你需要可审计的“任务请求->执行->任务反馈(answer)”证据。
  解决方案: 拆分 transport vs reply 两类 E2E,并用“两轮 turn/iteration(step2)”让 reply 场景稳定产出可见 answer。
author: Ralph contributors
version: 1.0.0
date: 2026-02-24
---

# Ralph + Codex app-server: steer 验证用“两轮 turn”稳定拿到可见 reply

## 问题

在真实 Codex app-server 下,你可能会看到:

- `turn/steer` request 已发送,也收到了 response(ACK)。
- 但 stdout/human-log 里没有你期望的“可见回复(answer)”,看起来像“无回复”。
- 甚至输出主要是 reasoning/summary 的 delta,不进入你用于断言的可见输出通道。

这会导致两类风险:

1) 你无法区分“模型没回复”与“只是没被我们摘录/没进入可见输出节奏”。
2) E2E 变得 flaky: 你把“steer 必须立即打断当前 turn 并产生可见 reply”当成协议前提,但真实后端不保证。

## 上下文 / 触发条件

当你遇到以下任一情况,就该用这个 skill:

- 真实 codex app-server 的 `turn/steer` 测试里:
  - human-log 里能看到 `[app-server-rpc] ... method=turn/steer` 的 send/recv,
  - 但看不到 `TASK_FEEDBACK`/`answer`/`LOOP_COMPLETE`。
- 你希望 E2E 注入的消息包含具体内容,例如 `121+43=?`,并验证 answer。
- 你尝试把 app-server item type 改成 `inputText`,但收到:
  - `-32600 unknown variant inputText, expected text/image/localImage/skill/mention`。

## 解决方案

### 1) 把验证拆成两类场景: transport vs reply

- transport 场景(稳定,低 flake):
  - 只验证 `turn/steer` 的 send/recv response(ACK) + 最终收敛。
  - steer payload 可以带 `marker + question`,但不要强依赖模型输出 answer(避免模型输出漂移)。

- reply 场景(更敏感,用于排障/证明“确实回复”):
  - 必须在 stdout 中观察到 `answer: ...` 与 `LOOP_COMPLETE`。
  - 关键技巧是“两轮 turn/iteration”,不要强求 steer 当场打断并回复。

### 2) reply 场景使用“两轮 turn/iteration(step2)”强制闭环

核心思路:

1) 第 1 轮(`[task.start]`):
   - 只输出 `STEER_WINDOW_OPEN`(建议多行),作为 keep-alive 与“窗口已打开”的证据。
   - 不输出 `LOOP_COMPLETE`(避免提前结束 loop)。

2) 外部注入 2 次 `turn/steer`(同一 in-flight window 内):
   - payload 例子:
     - `marker: ...; question: 121+43=?`
     - `marker: ...; question: 10+5=?`

3) 第 2 轮(emit 一个显式事件,例如 `e2e.reply.step2`):
   - 要求模型从 thread 历史中找到最近两条输入,并输出:
     - `TASK_REQUEST[n]: <原文>`
     - `TASK_FEEDBACK[n]: answer: <计算结果>`
   - 输出两条反馈后,最后输出 `LOOP_COMPLETE`。

这样做的好处:

- 你验证的是“steer 输入进入 thread 历史且可在下一次 turn 被读取并产生 reply”。
- 不依赖“steer 立刻影响当前输出”的不稳定行为。

### 3) human-log 必须包含 runner stdout/state(排障硬门槛)

仅有 RPC trace 不够:

- RPC trace 只能证明“控制通道收发正常”。
- runner stdout/state 摘录才能证明:
  - 是否真的有可见输出。
  - answer 是否出现。
  - completion 是否发生(`LOOP_COMPLETE`)。

建议在 human-log 中至少包含:

- `[supervisor] instances ...`
- `[ralph#1:state] ...`
- `[ralph#1:out:job=...]` 的 head/tail(包含 `answer` 与 `LOOP_COMPLETE`)。

### 4) 保持 app-server item type 使用受支持的枚举值

真实 codex app-server 当前不支持 `inputText`。
如果要发文本,继续使用 `type=text`。

## 验证

- 单测/编译:
  - `cargo test -p ralph-e2e`
- 真实 codex app-server:
  - `cargo run -p ralph-e2e -- codex --filter parallel-app-server-steer-live-reply-multi-turn`
- 证据检查:
  - `.e2e-tests/artifacts/parallel-app-server-steer-live-reply-multi-turn/human-log.md` 中能看到:
    - `turn/steer` send/recv
    - `TASK_FEEDBACK[*]: answer: 164/15`
    - `LOOP_COMPLETE`

## 示例

- steer-1: `marker: E2E_...; question: 121+43=?` -> `answer: 164`
- steer-2: `marker: E2E_...; question: 10+5=?` -> `answer: 15`

## 备注

- exit code:
  - 在 E2E 断言侧通常使用 `exit_code_success_or_limit`(0 或 2 都可接受)。
  - 不要仅凭 exit_code 推断“是否真的回复”,要看 human-log 的 stdout 摘录与 answer 行。
