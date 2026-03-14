## Context

当前并行运行时已经支持 `session_strategy=exec|mcp`:

- `exec`: 每次 job 都是一次性 `codex exec`(或其他 CLI)调用.
- `mcp`: 通过 `codex mcp-server` 复用 thread,让对话连续性更好(尤其是 `ralph#1/#2`).

但 `mcp` 仍是"请求-响应"模型.
当某个 hat 正在产出输出时,新的指令只能排队到下一轮 job.
这与 Codex 的 Steer(运行中立即追加输入)体验不一致,也是你当前沟通成本高的根因.

Codex App Server 协议提供:

- `turn/start`: 开始一个 turn
- `turn/steer`: 在同一个 in-flight turn 里追加输入(需要 `expectedTurnId`)
- `turn/interrupt`: 中断当前 turn(不中断 thread)

因此我们需要把 App Server 接入并行运行时,并在 HatInstance actor 与执行器之间引入"运行中控制通道",让 `turn/steer` 能在 job 进行期间送达.

约束:

- "Disk is state, Git is memory": 关键决策必须可落盘、可回放、可诊断.
- "改良胜过新增": 尽量复用现有 event/instance/job 结构,避免重新发明一套平行系统.
- ralph-core 仍保持"薄协调层": 不直接依赖 Codex 协议细节,具体 runtime 放在 ralph-cli.

## Goals / Non-Goals

**Goals:**

- 增加 `SessionStrategy=app_server`,并把 sticky(只升级)规则扩展为 `exec < mcp < app_server`.
- 增加 turn 级控制语义,使事件可以显式表达 `start|steer|interrupt`.
- ralph-cli 新增 Codex App Server runtime:
  - 每个 instance 复用一个 thread.
  - `turn/start` 时记录 active `turnId`.
  - 支持 `turn/steer` 与 `turn/interrupt`.
  - 把 agent message delta 流式转发到 Supervisor 输出面板.
- Supervisor TUI 增加最小入口:
  - `!steer <text>`: 对当前选中/定向实例做 in-flight steer.
  - `!interrupt`: 中断当前选中/定向实例的 in-flight turn.
- 测试覆盖:
  - event 解析: `session_strategy="app_server"` + `turn_action="..."`.
  - core 合并/sticky: `exec<mcp<app_server`.
  - TUI chat 命令解析与外部事件写入字段.

**Non-Goals:**

- 不把现有 `mcp` 全量迁移到 `app_server`(保留为 fallback).
- 不在本 change 里实现完整的 App Server "动态工具调用"生态(仅做 headless 必需的最小响应策略).
- 不引入跨进程的 session 持久化(例如 threadId 落盘恢复)作为本次硬要求.

## Decisions

### 1) 用什么信号表达 steer/interrupt

**选择:** 在 `Event` 协议里新增可选字段 `turn_action`,取值 `start|steer|interrupt`.

**原因:**

- 这是一等协议信号,可回放可诊断,不依赖"当时是否 Running"的隐式状态.
- 允许 ralph 在发布 `<event ...>` 时动态决定行为(符合你"动态决定"的方向).

**备选(未选):**

- 只用新 topic(例如 `turn.steer` / `turn.interrupt`)表达控制.
  - 缺点: topic 语义膨胀,并且与现有 `human.message`/路由约定更难对齐.

### 2) in-flight 控制通道如何打通到执行器

**选择:** 扩展 `HatJobExecutor::execute(...)`,额外传入一个 `control_rx`,用于在 job 运行期间接收 `Steer` 控制消息.

**原因:**

- HatInstance actor 天生就是"串行执行 job,但可并发接收命令"的模型.
- `cancel_rx` 已用于 interrupt(取消),`control_rx` 专用于 steer(追加输入),语义更清晰.
- core 只依赖抽象通道,不需要理解 Codex 协议细节,符合"薄协调层".

**备选(未选):**

- 让 Supervisor 同一 instance 并发启动第二个 job 来发送 steer.
  - 缺点: 破坏"每个 instance 串行执行 job"的模型,复杂且容易产生竞态.

### 3) sticky(只升级)规则扩展

**选择:** `SessionStrategy` 通过枚举顺序表达强弱,并在实例侧记录 `session_locked_to: SessionStrategy`.

规则:

- 合并: pending events 里取最大值.
- sticky: `job.session_strategy = max(session_locked_to, merged_pending)`.
- 执行后: `session_locked_to = max(session_locked_to, job.session_strategy)`.

### 4) App Server runtime 结构

**选择:** 在 `crates/ralph-cli` 新增 `CodexAppServerRuntime`,按 instance 维护 `CodexAppServerSession`.

每个 session 负责:

- spawn `codex app-server` 子进程(stdio transport)
- JSON-RPC 写入(stdin) + 读取(stdout)
- `thread/start` 后缓存 `threadId`
- `turn/start` 后缓存 active `turnId`
- `turn/steer`:
  - 发送 `expectedTurnId=active_turn_id`
  - input 以 `UserInput` 数组表达(文本消息)
- `turn/interrupt`:
  - 发送 `turnId=active_turn_id`(best-effort)

输出:

- 订阅/解析 server notifications,优先消费 `item/agentMessage/delta`(把 delta 文本作为 stdout chunk 流式转发).
- 以 `turn/completed` 作为 job 完成信号,组装完整输出用于 EventParser.

审批:

- 若 server 发起 `item/commandExecution/requestApproval` / `item/fileChange/requestApproval`,默认自动 accept(保持 headless).
- 其他未知 request 先以可观测日志记录,并返回 best-effort 的失败响应(避免死锁).

### 5) steer 降级策略

**选择:** best-effort + 可观测.

- 若 instance 当前存在 in-flight turn(并且 job 是 app_server):
  - 立即 `turn/steer`.
- 否则:
  - 把该事件按普通事件入队,等待下一轮 job 处理(不丢消息).
  - 记录 warning 日志(让你知道"这次没真 steer").

### 6) TUI 命令入口

**选择:** chat 解析新增两条命令:

- `!steer <text...>`: 写入外部事件 `human.message`,并附带 `session_strategy=app_server` 与 `turn_action=steer`.
- `!interrupt`: 写入外部事件(同样可带 target_instance),并附带 `turn_action=interrupt`.

这样 human 的操作仍然是"写外部事件 JSONL",符合并行模式的可回放原则.

## Risks / Trade-offs

- [资源占用上升] `app_server` 可能为每个启用实例持有常驻子进程 → 缓解: 仅在事件显式请求时升级,并保持其他实例默认 `exec`.
- [协议漂移] `codex app-server` 为实验特性,通知/字段可能变化 → 缓解: 只依赖最小子集,对未知字段做兼容忽略并打日志.
- [工具调用复杂度] 可能出现 `item/tool/call` 等请求 → 缓解: 先实现 best-effort 失败响应并可观测; 后续若确实遇到,再按真实需求补齐.
