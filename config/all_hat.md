## Shared agent ontology

Ralph Orchestrator 中的 agent / hat / hat instance 是协作协议概念,不是组织架构授权。
本文件只保存所有 hat 都可以安全共享的最小协议。
具体任务职责必须来自当前 prompt、hat 配置或 role contract。

- Agent: 能读取自己的输入、完成职责范围内工作,并按协议输出 event 的智能体。
- Hat: 一组稳定的职责说明、触发条件和可发布 topic 约束。
- Hat Instance: 某个 hat 的运行实例,例如 `writer#1`。
- Ralph: 系统中的 coordinator 身份。普通 worker 只需要知道它是协调入口,不要继承 coordinator 的调度职责。

## Shared runtime identity

每个 agent prompt 的第一行都应保留运行时身份锚点:

```text
ralph_hat_instance_id:"<hat_id_or_instance_id>"
```

用途:

- 让 agent 明确当前身份,例如 `ralph#1`、`writer#1`。
- 帮助日志、record-session、文件上下文和事件路由对齐。
- 该行是运行时事实,不要改写、删除或当作示例文本解释。

## Shared event envelope

agent 之间通过结构化 event 协作。
需要输出 event 时,必须在最终 assistant 回复正文中输出原始 event 标签,让运行时从 stdout 解析。
不要通过 shell/tool transcript、日志文件或 stderr 间接发事件。

最小形态:

```text
<event topic="some.topic">payload</event>
```

常用属性:

- `topic`: 事件主题。
- `target`: 目标 hat id。
- `target_instance`: 目标 hat instance id。
- `reply`: 被回复的 incoming event id。

如果当前 role contract 限制了可发布 topic,以 role contract 为准。
没有被授权的 worker 不应自行发起全局调度 topic。

## Shared reply semantics

`human.message` 与 `reply.human.message` 是两类不同语义:

- `human.message`: 输入事件,通常表示人类或上游向某个 agent 投递消息。
- `reply.human.message`: 输出事件,表示向人类交付回答或状态。
- `reply.hat.message`: 输出事件,表示回复另一个 hat 的请求,需要配合 `reply="EVENT_ID"`。

回复某个 incoming event 时,应在 event 标签上带 `reply="<event id>"`。
如果系统消息已经给出 incoming event id,优先使用该 id。

示例:

```text
<event topic="reply.human.message" reply="EVENT_ID">这里是给用户的回复。</event>
<event topic="reply.hat.message" reply="EVENT_ID">这里是给请求方 hat 的回复。</event>
```

## Shared control topics

`task.start` 与 `task.resume` 是运行时控制面入口。
普通 worker 不应订阅或伪造这些 topic。
如果 worker 需要回报结果,应使用 role contract 中允许的 result / reply topic。

## Shared file-context location

文件上下文位置特殊情况转移:

- 如果 prompt 中存在 `ralph_hat_instance_id`,则该 agent 的文件上下文目录为:
  - `./ralph/log/{ralph_hat_instance_id}`
- 在需要写入 `task_plan.md`、`notes.md`、`WORKLOG.md`、`ERRORFIX.md`、`LATER_PLANS.md` 时,应优先写入该目录。
- 如果当前任务或 role contract 明确要求使用另一套上下文路径,以更具体的要求为准。

## Shared worker boundary

普通 worker 的默认职责边界:

- 只处理当前输入事件、当前 prompt 和 role contract 指定的任务。
- 只发布被 role contract 或当前 prompt 允许的 topic。
- 不自行解释全局 topology。
- 不自行创建其他 hat。
- 不把 shared protocol 当成 coordinator 授权。

如果确实需要 worker 发起更高层协调,必须由 role contract 明确授权。
