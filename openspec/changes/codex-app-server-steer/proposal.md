## Why

并行模式下,我们已经支持 `session_strategy=exec|mcp` 来决定 hat 的会话形态.
但 `mcp` 仍然是"本轮请求-本轮响应"的交互模型.
当某个 hat 正在执行时,新的指令只能排队到下一轮,无法做到"立即追加输入并改变正在运行的轨迹".

你要的核心体验是 Codex 的 Steer(立即发消息)能力.
这类能力在协议层对应的是 Codex App Server 的 `turn/steer` 与 `turn/interrupt`.
因此需要把 App Server 作为并行运行时的第三种会话策略接入,并把 turn 级控制语义打通到 HatInstance 的 in-flight job.

## What Changes

- 协议层扩展:
  - `SessionStrategy` 增加 `app_server`(优先级: `exec < mcp < app_server`).
  - 事件增加 turn 级动作语义(例如 `turn_action=start|steer|interrupt`),用于表达"新开 turn"或"对 in-flight turn 追加/中断".
- 并行 core 侧:
  - session 合并与 sticky(只升级不降级)规则扩展到 `app_server`.
  - HatInstance actor 增加 in-flight 控制通道,使 `steer` 能在 job 运行期间送达.
- ralph-cli 侧:
  - 新增 Codex App Server runtime(类似 `CodexMcpRuntime`),支持 `thread/start` + `turn/start`.
  - 打通 `turn/steer` 与 `turn/interrupt`:
    - `turn/steer` 作为追加输入通道(需要 `expectedTurnId`).
    - `turn/interrupt` 用于中断当前 turn(不中断 thread).
- Supervisor TUI(最小可用入口):
  - chat 增加 `!steer ...` 与 `!interrupt` 命令,用于对"当前选中实例"进行 in-flight 控制.
- 可观测与回放:
  - 关键字段(例如 session_strategy/turn_action/turnId)需要被记录到日志/诊断输出中,避免黑盒.
- 测试与 smoke tests:
  - 解析 `<event ... session_strategy="app_server">` 与 `turn_action` 的回归测试.
  - 并行路由/合并规则对 `app_server` 的回归测试.

## Capabilities

### New Capabilities

（无）

### Modified Capabilities

- `parallel-hat-instances`: 增加 `session_strategy=app_server` 与 turn 级控制语义(steer/interrupt),并要求其可回放可诊断.
- `supervisor-human-chat-gate`: 扩展 chat 的控制命令集,提供最小 `!steer`/`!interrupt` 入口.

## Impact

- 受影响代码区域:
  - `crates/ralph-proto`: 协议字段扩展(新增枚举值/字段).
  - `crates/ralph-core`: 并行 HatInstance actor 与调度模型需要支持 in-flight 控制.
  - `crates/ralph-cli`: 新增/接入 Codex App Server runtime,并在并行执行器里按策略选择.
  - `crates/ralph-tui`: chat 输入解析与外部事件写入需要扩展命令语义.
- 运行时影响:
  - `app_server` 模式可能为每个启用的实例引入一个常驻子进程(资源占用上升,但换来真 steer).
  - 需要 codex CLI 版本支持 `codex app-server`(实验特性).
