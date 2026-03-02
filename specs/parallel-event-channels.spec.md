# Spec: 并行事件发布通道(in-band `<event>` vs out-of-band `ralph emit`)

## 背景

在并行模式里,你会同时看到两种“看起来都像事件”的东西:

1) agent 在 stdout 里输出的 `<event ...>...</event>` 标签。
2) 人类或工具在另一个终端执行 `ralph emit ...`,把一行 JSONL 追加到外部事件文件。

这两者的语义不同,边界也不同。
如果不把边界写清楚,很容易产生误解,例如:

- 误以为 `<event>` 可以在同一轮 turn/job 进行中“随时触发”下游工作。
- 在 `.ralph/worktrees/...` 等子目录执行 `ralph emit` 时写错文件,导致看起来“没有注入/没有回复”。

本 spec 的目标是把“何时能被系统识别并路由”写死,并回答你之前的关键问题:

- app-server 要产生一个 `<event ...>` 是否必须完结一轮 turn?
- 期间能不能像 tool 调用/MCP 那样在一轮中多次交互?
- 如果要在 turn in-flight 期间多次发消息,应该用什么通道?

---

## 目标(Goals)

1) 明确 in-band `<event>` 的解析边界:
   - 事件何时被解析?
   - 解析基于 stdout 还是 stderr?

2) 明确 out-of-band `ralph emit` 的消费边界:
   - 是否可以在 turn/job in-flight 期间多次注入?
   - 注入是否会被并发上限/backpressure 延迟?

3) 明确并行 coordinator 的“单轮多事件”能力:
   - 允许一次输出多条 `<event ...>...</event>`。

4) 明确 `.ralph/current-events` marker 的定位规则:
   - 在子目录(worktree 等)执行 `ralph emit` 也能写入 active run 的 events 文件。

---

## 非目标(Non-Goals)

- 不实现“流式解析 stdout 中的 `<event>` 并立即路由”(streaming event parsing)。
- 不把 stderr 当作可解析事件来源(避免 `<event>` 假事件污染)。
- 不保证所有后端都支持 turn in-flight 的 steer/interrupt。
  - 该能力是“运行时控制信号”,依赖具体 `session_strategy` 与后端实现。

---

## 术语(Terms)

- Job(HatJob): Ralph 并行运行时的最小执行单元。
  - 一个 HatInstance 一次只跑 1 个 job。
  - job 内部可能包含 tool 调用,也可能是一次性 CLI 执行,取决于后端。

- Turn: 后端会话层的“对话轮次”概念。
  - `session_strategy=exec`: 通常每个 job 就是一轮 turn(一次进程调用)。
  - `session_strategy=mcp|app_server`: 通常是常驻会话中的一轮 turn。

- In-band event: agent 通过 stdout 输出的 `<event ...>...</event>` 标签。
  - 解析与路由由 Ralph 的 EventParser 在 job 完成后统一执行。

- Out-of-band event: 通过外部事件文件(JSONL)注入的 event。
  - 标准写入方式是 `ralph emit ...`。
  - Supervisor 会持续轮询该文件,并把新事件路由到目标实例。

- 控制信号(turn_action): 用于影响 in-flight job/turn 的特殊字段。
  - 例如 `steer`/`interrupt`。
  - 这不是“业务事件 topic”,而是投递时的运行时行为提示。

---

## 核心语义(必须遵守)

### S1: `<event ...>...</event>` 的解析时机 = job 完成之后

并行模式里,EventParser 的调用点在“job 完成回调”中。
当前实现是:

- 解析发生在 `HatInstanceActor::on_job_completed()`:
  - 文件: `crates/ralph-core/src/parallel/instance.rs`
  - 关键逻辑: `EventParser::new().parse(&result.output_for_parsing)`

因此:

- 你从系统视角看到“某个 `<event>` 被识别并路由”的前提,是该 job 已经结束。
- 换句话说,**in-band `<event>` 不是“中途消息总线”**。
  - 它更像是“本次 job 的最终产物里包含了若干路由指令”。

对你之前的问题的直接回答:

- app-server 要产生一个 `<event ...>` 并被 Ralph 路由,**必须等该轮 turn/job 完结**。
- 在这轮 turn/job 进行期间,你当然可以发生 tool 调用/MCP 交互(如果后端支持)。
  - 但这不改变 `<event>` 的路由边界: 仍然要等 job 完成才解析并路由。

### S2: 并行模式的事件解析是 stdout-only

并行模式下,stderr 常常包含“后端自身日志”或“prompt transcript”。
这些内容可能包含 `<event ...>` 字样(例如 prompt 示例),如果参与解析会造成:

- 重复路由
- completion 假阳性
- E2E 波动

当前实现的约束是:

- `HatJobResult.output_for_parsing` 在并行模式中只拼接 stdout。
  - stderr 会被流式展示与录制,但不会进入 `HatJobResult.output_for_parsing`。
  - 文件: `crates/ralph-cli/src/parallel_runner.rs`
  - 关键逻辑: `handle_output_line(..., stream=Stderr, ...)` 不会 `push_str` 到 `output_for_parsing`。

结论:

- `<event>` 只从 stdout 解析。
- stderr 只用于可观测输出与诊断证据,不参与路由。

### S3: 单个 job 输出可以包含多条 `<event>`

允许一个 job 的 stdout 同时包含多条 `<event ...>...</event>`。
系统会在 job 完成后统一解析并路由这些事件。

因此,并行 coordinator(`ralph#1`)可以在一次输出里:

- 发布多条任务派发事件(例如同时派发给 writer/tester)
- 然后停止输出,等待下一轮 fresh context

### S4: `ralph emit` 可以在 turn/job in-flight 期间多次注入

Supervisor 会在主循环里周期性轮询外部事件文件(JSONL),并把新事件路由出去。
该行为与“是否有实例正在 Running”无关:

- 即使某个实例正在运行 job,外部事件仍然会被读取与路由。
- 事件是否能“立刻生效”,取决于:
  - 目标实例是否正在 Running(可能入队或走 steer 控制通道)
  - 全局并发上限与 hat 的实例容量(可能延迟启动新 job)

### S5: `turn_action=steer|interrupt` 是运行时控制信号

在并行模式里,`turn_action` 不是普通业务事件。
它会在投递到 HatInstance 时触发特殊逻辑:

- `turn_action=interrupt`:
  - best-effort 取消当前 job(如果正在 Running)。

- `turn_action=steer`:
  - 如果目标实例正在 Running 且其会话策略为 `app_server`,
    则把 payload 作为“追加输入”发送到 in-flight turn。
  - 如果不满足条件(例如实例不在 Running,或不是 app_server),
    则会降级为普通消息入队,等下一次 job 处理(避免丢消息)。

实现位置:

- 文件: `crates/ralph-core/src/parallel/instance.rs`
- 关键逻辑: `HatInstanceCommand::Deliver(...)` 分支里对 `event.turn_action` 的处理。

### S5.1: external control-plane 必须 fail-closed 且仅允许 `ralph#1`

对于 out-of-band external JSONL 注入(例如 `ralph emit` 或 TUI chat 写入外部事件文件):

- `turn_action=steer|interrupt` 仅允许显式 `target_instance=ralph#1`。
- 出现以下任一条件时必须拒绝(不路由)并告警到 `ralph#1`:
  - 缺失 `target_instance`
  - `target_instance != ralph#1`
  - 同时携带 hat-level 路由提示(`target` 或 `spawn_instance=true`)
- 在 hat job 环境里(`RALPH_HAT_INSTANCE_ID` 存在),`ralph emit --turn-action steer|interrupt` 必须被 CLI 直接拒绝。

约束目的:

- 把 control-plane 信号限制为“ExternalInput -> ralph#1”的窄边界。
- 避免 worker/hat 误触发 in-flight 打断导致流程漂移。

### S5.2: hat-to-hat 子任务回传采用 request/result,且只回最终结论

当 A hat 触发 B hat 执行子任务时:

- B hat 应通过 data-plane 普通 topic 回传结果(例如 `subtask.result`)。
- B hat 只在自身 job/turn 结束时回传一次最终结论。
- B hat 不应在 job/turn 中途回传半成品结论或进度消息驱动 A 继续推进。

### S6: `.ralph/current-events` marker 的定位规则

外部事件文件的路径由 marker 指示:

- marker: `.ralph/current-events`
- 内容: 一行路径
  - 常见情况: 相对路径(例如 `.ralph/events-<run_id>.jsonl`)
  - 也允许绝对路径

定位规则:

- `ralph emit` 与 `ralph events` 必须支持在子目录执行:
  - 从当前工作目录开始,向上遍历父目录,寻找最近的 `.ralph/current-events`。
- marker 内容为相对路径时:
  - 必须以 workspace root(包含 `.ralph/` 的目录)为基准解析。

实现位置:

- 文件: `crates/ralph-cli/src/main.rs`
- 关键逻辑: `resolve_events_file_from_marker_in_parents(...)`

---

## 使用指南(推荐实践)

### 什么时候用 `<event>`(in-band)

适用:

- hat 的正常工作产物要触发后续路由时。
- 你希望由 Supervisor 在 job 完成后统一解析并派发。

注意:

- `<event>` 必须闭合 `</event>`。
- 优先单行,不要把 `<event>` 放进 code fence。

### 什么时候用 `ralph emit`(out-of-band)

适用:

- 人类在另一个终端向任意实例注入消息。
- 需要在 turn/job in-flight 期间多次注入输入,尤其是:
  - `turn_action=steer`
  - `turn_action=interrupt`
- 你希望“立刻把消息写进系统”,并由 Supervisor 异步路由。

注意:

- `ralph emit` 必须实际执行命令。
  - 不要把命令当作普通文本打印出来。
- 推荐显式加 `--target-instance <hat#n>`。
  - 否则事件可能被 fanout 或变成 orphan,造成误解。

---

## 示例

### 例1: coordinator 单轮发多条 `<event>`

```text
<event topic="build.task" target="writer">实现修复 A</event>
<event topic="test.task" target="tester">回归测试</event>
```

### 例2: human 在子目录向 ralph#1 注入消息

```bash
cd .ralph/worktrees/writer#1  # 示例: 你当前在 worktree 子目录
ralph emit human.message "继续推进,并把并行度控制在 P=2" --target-instance ralph#1
```

### 例3: app-server in-flight steer(追加输入)

```bash
ralph emit human.message "补充: 只需要改 routing.rs,不要动其他文件" \
  --target-instance ralph#1 \
  --turn-action steer \
  --session-strategy app_server
```

### 例4: gate.resolve(异步权限确认)

```bash
ralph emit -j gate.resolve '{"gate_id":"...","decision":true}' --target-instance writer#1
```

---

## 验收标准(Acceptance Criteria)

当一个新同事读完本 spec 后,他应当能无歧义地回答:

1) `<event ...>...</event>` 是否能在 job 进行中立即触发路由?
   - 不能。必须等 job 完成后才解析并路由。

2) 并行事件解析是否会读取 stderr?
   - 不会。解析是 stdout-only。

3) 是否允许单轮输出多条 `<event>`?
   - 允许。系统会在 job 完成后批量解析并路由。

4) 如果需要在 turn in-flight 期间多次发消息或 steer,应该用什么?
   - 用 `ralph emit ...`(out-of-band 外部事件注入)。
