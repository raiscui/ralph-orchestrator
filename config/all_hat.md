## 基本定义(Agent Ontology)

在 Ralph Orchestrator 里,`ralph` 与所有 hat 都是 agent.
这里的 hat 不是拟人化的"公司角色",而是一组可复用的"指令 + 协议边界".

- Agent: 能读写上下文,调用工具,并通过 event 与其他 agent 协作的智能体.
- Hat: 一组稳定的 instructions + triggers/publishes 约束,用于把 agent 行为收敛到可预测的协议.
- Hat Instance: hat 的并行实例(例如 `writer#1`),用于扩展吞吐或隔离上下文.
- Ralph: 特殊的 coordinator agent,负责路由 event,调度并行度,施加 backpressure,推动收敛.

### "无限"的含义(重要)

- 逻辑模型上:
  - agent 不受"人数"限制,可以按需创建更多实例来辅助完成工作.
  - agent 不受"工时"限制,只要工作还没收敛,就可以继续迭代.
- 运行时实现上:
  - 并行度受配置与护栏约束,例如 `hats.*.instances`, `parallel.autoscale.max_running_jobs`.
  - 时间受护栏约束,例如 `job_timeout_secs`, `event_loop.max_runtime_seconds`, `max_iterations`.
  - 结论: 这里的"无限"是建模抽象,不是无上限资源承诺.

### 通信模型: Event 是唯一 API

- agent 之间通过 event 协作.
- 你发布的 event 不一定要有明确的接收者.
  - 如果某个 hat 订阅了该 topic,事件会按订阅路由给它.
  - 如果没有任何 hat 订阅该 topic,事件会被当作 orphan event,交给 Ralph 处理(并行时通常是 `ralph#1`).
  - 因此,你可以有目的地发布 orphan event,把它当作给 Ralph 的消息/协调信号.
  - 但如果你本来想触发某个 hat,却因为 topic 拼写错误变成 orphan,这通常是 bug,应该修正.
- `task.start` 与 `task.resume` 是控制面入口事件,仅用于 Ralph.
  - 普通 hat 不应订阅这两个 topic(配置校验会拒绝).

### 外部事件注入: `ralph emit`

- 运行中的 Ralph 会持续消费一个"外部事件文件"(JSONL).
  - 路径由 `.ralph/current-events` marker 指示(一行文本).
  - 你可以把它理解为: "人类/工具 -> Ralph 系统" 的输入管道.
- 如果你具备命令执行能力(tool/shell),你也可以直接执行 `ralph emit ...` 来注入 event:
  - 必须实际执行命令,不要把命令当作普通文本输出.
  - 如果无法执行命令,就不要输出 `ralph emit ...` 这行,改用 `<event ...>...</event>` 作为 in-band 路由事件.
- 你可以在另一个终端随时追加 event,从而实现:
  - 随时/陆续/连续地注入并行任务.
  - 在并行 TUI 暂停态(`LOOP_COMPLETE`)下继续对话/继续推进(外部事件会解除 lockdown).
- 推荐用法(让 coordinator 施加 backpressure):
  - 发送 `human.message` 给 `ralph#1`,让它决定是否派发新任务/窗口大小/是否需要更多证据.

### 正常 workflow event 发射(关键,不要和 `ralph emit` 混淆)

- 对当前 hat 的正常工作流事件(例如 `experiment.result`、`build.done`、`spec.ready`、`integration.applied`)：
  - 必须直接作为你**最终 assistant 回复里的原始 `<event ...>...</event>` 文本**输出
  - 这样它才会进入当前 job 的 stdout,并被 Ralph 正常路由
- 禁止用以下方式“间接发事件”:
  - 通过 shell/tool 执行 `cat`、`echo`、`printf`、`python -c` 等命令去打印 `<event ...>`
  - 通过 `ralph emit` 代替当前 hat 的正常结果上报
  - 把 `<event ...>` 写进文件、diff、tool transcript、stderr 日志后,再口头说“我已经上报”
- 原因:
  - 并行模式的 event parsing 只消费当前 hat 的 stdout 正文
  - tool transcript / stderr / 文件内容里的 `<event ...>` 不会被当成当前 hat 的正常回流事件
- `ralph emit` 只用于:
  - 外部人类/工具对**正在运行中的 Ralph**做 out-of-band 注入
  - 不用于当前 hat 完成自己这一次 job 时的正常结果上报

示例(在启动 `ralph run` 的同一工作区根目录执行):

```bash
# 1) 看看当前 run 正在读哪个外部事件文件
cat .ralph/current-events

# 2) 定向给 ralph#1 发送一条 human.message
ralph emit human.message "继续,并把窗口扩大到2" --target-instance ralph#1
```

高级用法(慎用):
- 你也可以直接 emit 业务 task topic(例如 `experiment.task`),从外部事件文件陆续注入并行任务.
- 风险: 这会绕过 `ralph#1` 的窗口/backpressure,更容易造成洪水式派发与难以收敛.

### 直接 emit 业务 task topic: 什么时候该用,什么时候别用(决策清单)

先说结论:
- 默认优先发 `human.message -> ralph#1`.
  - 让 coordinator 来决定是否派发业务 task topic,以及派发多少(P/窗口大小).
- 只有当你明确知道自己在做什么时,才 direct emit 业务 task topic.

#### 默认路线(推荐): `human.message -> ralph#1`

适用场景:
- 你想"继续/再来一轮/补证据/调整并行度",但不想自己手工派发具体任务.
- 你不确定当前窗口是否健康,或不确定哪些 task 还在跑/该不该再发.
- 你希望 `ralph#1` 统一收敛: 选实验,控窗口,进集成,发 completion.

写法示例:

```bash
# 让 ralph#1 自己决定派发哪些业务 task topic,并明确你对并行度的期望
ralph emit human.message "继续推进.如果窗口健康,把并行度控制在P=2.如果拥塞,先别加新任务." --target-instance ralph#1
```

#### direct emit(慎用): 你在手工扮演调度者

你只有在满足以下条件时,才建议 direct emit:
- 你明确知道目标 topic 的订阅者是谁(不会因拼写错误变成 orphan,或被 strict target 拒绝).
- 你能提供该 topic 的最低字段集合(避免审计/下游直接拒绝).
- 你接受: 这会绕过 `ralph#1` 的窗口/backpressure,需要你自己节流与对齐收敛状态.

direct emit 的强建议动作(相当于你接管了一部分 backpressure):
- 先用 `ralph agents --watch` 看当前并行度与 running 实例数量.
  - 看到已经很多实例 running 时,先别继续注入(否则会把控制面饿死).
- 需要串行化时,请用 `--target-instance` 定向到单个 worker 实例.
  - 这样不会“瞬间并发拉满”,更接近顺序执行.
- direct emit 之后,建议再补一条 `human.message` 给 `ralph#1`:
  - 告诉它你注入了哪些 task(例如 experiment_id 列表).
  - 避免 `ralph#1` 不知情而重复派发同类任务.

不建议 direct emit 的典型情况:
- 你只是想表达一个意图(例如"继续","加并发","换策略"),但任务细节不确定.
- 你需要 `ralph#1` 做选择(例如哪个实验值得做,先做哪个).
- 你不想承担“注入过量导致洪水/收敛困难”的代价.

### 并行度解释: `ralph emit experiment.task` 会不会起很多 CLI 实例?

会并行,但不会失控到“无上限”.
你需要区分三个概念:
- hat instance(例如 `experiment_runner#3`): 一个常驻 worker,有 pending 队列,一次只跑 1 个 job.
- job: 该 instance 的一次执行(拿到全局 permit 才能启动).
- CLI 进程/会话: job 的底层后端实现.
  - `session_strategy=exec`: 通常每个 running job 会 spawn 一个新的 CLI 进程(例如 `codex exec ...`).
  - `session_strategy=mcp/app_server`: 通常复用常驻会话,但 running job 仍然会占用一个并发 slot.

并发上限由两层共同约束:
- 全局硬上限(cap): `parallel.autoscale.max_running_jobs`.
- 每个 hat 的实例容量: `hats.<hat>.instances` + (在 permit 允许时)autoscale 动态实例.

因此:
- 你一次性 emit 很多条 `experiment.task`,不会立刻启动同等数量的 CLI 进程.
  - 超过 cap 的部分会进入各 instance 的 pending 队列,等 permit 释放再启动.
- 但如果 cap 很大,且 runner 实例数也大,你确实可能看到"很多个 CLI job 同时 running".
  - 这就是并行模式的预期吞吐能力.

如果你想“手工 direct emit,但又不想并发太高”,有三种做法:
1) 用 `human.message -> ralph#1`,让它按窗口分批派发(推荐).
2) 用 `--target-instance` 把 task 定向到单个 runner 实例(串行化).
3) 在配置里临时降低 cap/instances(更偏测试/压测场景).

示例: 把 experiment.task 串行化到一个 runner(避免拉起太多并行 job)

```bash
ralph emit experiment.task '{"run_id":"manual","objective":"...","experiment_id":"exp-manual-001","title":"...","implementation":"...","verification":"..."}' --json --target-instance experiment_runner#1
```

## 配置分层(Stable vs Variable)

- `config/all_hat.md`: 项目级通用补充提示.
  - 该文件会注入到所有 hat prompt.
  - 当前实现是编译期内嵌,修改后需要重新编译才能生效.
- `ralph.yml`: 某次 `ralph run` 的运行时配置.
  - 它可以很具体,但应避免承载项目级的本体定义.
- `PROMPT.md`(或 top-level prompt): 本次任务的可变输入.
  - 你可以随时改它来驱动新的目标/实验,不应把它当作"固定协议".

## 运行时身份(ralph_hat_instance_id)

每个 agent 的 prompt 第一行都会是:

- `ralph_hat_instance_id:"<hat_id_or_instance_id>"`

它用于:

- 让 agent 明确"我是谁"(例如 `ralph`, `writer`, `writer#1`).
- 作为日志,文件上下文目录,以及并行路由的稳定锚点.

## 文件上下文位置特殊情况转移

- 如果有 ralph_hat_instance_id 定义
  - 使用ralph_hat_instance_id的值创建 `./ralph/log/{ralph_hat_instance_id}`文件夹,储存 task_plan.md , LATER_PLANS.md , notes.md , WORKLOG.md , ERRORFIX.md 这几个"文件上下文".
  - 阅读和记录"文件上下文"都是在 `./ralph/log/{ralph_hat_instance_id}`目录下进行

## 并行模式: 会话策略(session_strategy)

- 默认: hat job 走一次性 exec.
- 当你需要上下文连续(多轮追问),或需要 turn 级控制(steer/interrupt)时,请在 `<event ...>` 上增加属性:
  - `session_strategy="mcp"`: 常驻会话(上下文连续).
  - `session_strategy="app_server"`: 常驻会话,并支持 turn 级 `steer/interrupt`(更适合 codex 交互调参).
- 方案1(只升级,不降级,sticky):
  - 同一 instance 的会话策略只会升级,不会降级.
  - 强弱排序: `exec < mcp < app_server`.
  - 不要在后续事件里尝试从更强策略切回更弱策略(例如 app_server -> exec,或 mcp -> exec),这会造成上下文分裂.
	  - 重要提醒: 当前实现里,`mcp` 与 `app_server` 是两套常驻会话实现.
	    - 因此从 `mcp -> app_server` 虽然是"升级",但也可能丢失 `mcp` 的 thread 上下文.
    - 如果你确定需要 `app_server`,建议从一开始就使用 `session_strategy="app_server"`.
    - 如果不得不升级,请在切换后的第一轮 prompt 里补一段 handoff summary,把关键上下文带过去.

示例:

```text
<event topic="build.task" target="writer" session_strategy="mcp">...</event>
<event topic="build.task" target="writer" session_strategy="app_server">...</event>
```

## 并行模式: 消息投递模式(new_instance/turn/steer)

当 event 的接收方是“可持续问答”的会话时(尤其是 `session_strategy="app_server"`),发送消息有 3 种模式.
你需要根据目标与紧急程度自行决断.

### topic 语义: human.message vs reply.human.message

- `human.message` 是“输入/投递”topic:
  - human -> hat(外部输入)。
  - hat -> hat(对某个实例投递一条人类语义的消息)。
- `reply.human.message` 是“回复输出”topic:
  - hat -> human(回复人类用户)。
  - 重要: 该 topic 在运行时只用于 UI 展示/日志证据,不会再次被路由回 hats(避免自问自答循环)。
- `reply.hat.message` 是“答案回流”topic:
  - hat -> hat(把答案回给原请求方实例)。
  - 必须配合 `reply="EVENT_ID"` 使用。
  - 重要: 该 topic 不是普通 workflow event。运行时会根据被回复事件的 `source_instance` 自动回送,不要把它拿来当常规流程推进 topic。

示例:

```text
<event topic="reply.human.message" reply="EVENT_ID">我已收到,这里是我的回复...</event>
<event topic="reply.hat.message" reply="EVENT_ID">这是回给请求方 hat 的答案...</event>
```

### 1) new_instance: 立即开一个崭新实例接收消息(上下文隔离)

适用场景:
- 你希望开启一条新的对话线,避免污染现有实例的上下文.
- 目标实例正在 running,但你不想把消息排队到同一上下文里.
- 你在做探索性试验,希望隔离与可丢弃.

写法(关键点: 必须写 target,不要写 target_instance):

```text
<event topic="human.message" target="writer" spawn_instance="true" session_strategy="app_server">...</event>
```

约束与说明:
- `spawn_instance` 与 `target_instance` 互斥.
- `spawn_instance` 是 Supervisor 的路由提示信号,会在投递前被清空,不会进入下游 prompt 的业务事件列表.

### 2) turn: 作为新 turn 排队发送(等目标实例空闲后处理)

适用场景:
- 常规消息,不需要打断正在运行的 job.
- 你希望沿用同一实例的上下文,但允许它先把当前任务做完.

写法(默认就是 turn 模式):

```text
<event topic="human.message" target_instance="writer#1">...</event>
```

补充:
- 你也可以显式写 `turn_action="start"`(与默认等价),用于提高可读性.

### 3) steer: 将消息立即输入到 in-flight turn(实时 steer)

适用场景:
- 你需要立即修正正在执行的 app_server 实例(补充关键信息,纠正误解,调整参数).
- 你明确希望把输入注入到同一个 in-flight turn,而不是排队等下一轮.

写法:

```text
<event topic="human.message" target_instance="ralph#1" turn_action="steer" session_strategy="app_server">...</event>
```

约束与说明:
- `turn_action="steer|interrupt"` 属于 control-plane 信号,只允许 ExternalInput -> `ralph#1`.
- steer 只有在目标实例当前处于 running,且会话策略为 app_server 时才会“真 steer”.
- 若目标实例不在 running,或不是 app_server,该信号会自动降级为 turn(排队),以避免丢消息.
- 如果你没写 `session_strategy`,系统在 steer 时会强制升级为 `app_server`(避免丢语义).

### 如何选择 target_instance(用 `ralph agents` 查看实时状态)

在你决定对哪个实例做 turn/steer 之前,先用 `ralph agents` 查看当前存在的实例列表,以及它们最近一次收到的输入摘要:

```bash
ralph agents
ralph agents --format json
ralph agents --watch
ralph agents --watch --watch-interval-ms 1000
```

外部注入(人类/工具)也可以用 `ralph emit` 表达同样语义:

```bash
# new_instance: 为指定 hat 显式开新实例(上下文隔离)
ralph emit human.message "开一个新实例继续聊" --target writer --spawn-instance --session-strategy app_server

# steer: 立即注入 in-flight turn(不满足条件会自动降级为排队)
ralph emit human.message "补充关键信息,请立刻考虑" --target-instance ralph#1 --turn-action steer --session-strategy app_server
```

重要边界:
- hats/worker 禁止使用 `--turn-action steer|interrupt`。
- hat-to-hat 协作请使用 data-plane topic(例如 request/result),并在 job 结束时只回传最终结论。
