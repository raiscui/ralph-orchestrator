## ADDED Requirements

### Requirement: Parallel runtime supports session_strategy=app_server
在并行模式下,事件 MUST 支持显式声明 `session_strategy="app_server"`.
当某次 job 合并的 pending events 中存在任意 `app_server` 请求时,该 job MUST 以 `app_server` 会话形态执行.

同时系统 MUST 保持 sticky(只升级不降级)规则,按强弱排序:

`exec < mcp < app_server`.

#### Scenario: Event requests app_server session
- **WHEN** 某个 hat instance 输出 `<event topic="build.task" session_strategy="app_server">...</event>`
- **THEN** 并行运行时 MUST 将该事件解析为 `Event.session_strategy=app_server`
- **THEN** 该事件被路由到的实例在执行对应 job 时 MUST 选择 `app_server` 会话形态

---

### Requirement: App Server turn control supports steer and interrupt
在 `session_strategy=app_server` 下,系统 MUST 支持 turn 级控制语义:

- `turn_action="start"`: 新开 turn(默认行为).
- `turn_action="steer"`: 对 in-flight turn 追加输入,使用 App Server 的 `turn/steer`.
- `turn_action="interrupt"`: 中断当前 turn,使用 App Server 的 `turn/interrupt`.

#### Scenario: In-flight steer appends input to the same turn
- **GIVEN** 某个实例正在以 `session_strategy=app_server` 执行 job,并存在 in-flight turn
- **WHEN** 系统投递一条带 `turn_action="steer"` 的事件到该实例
- **THEN** 运行时 MUST 对该实例执行 `turn/steer`(而不是等本轮结束再新开 turn)

#### Scenario: Interrupt cancels only the active turn
- **GIVEN** 某个实例正在以 `session_strategy=app_server` 执行 job,并存在 in-flight turn
- **WHEN** 系统投递一条带 `turn_action="interrupt"` 的事件到该实例
- **THEN** 运行时 MUST 执行 `turn/interrupt` 来中断当前 turn

---

### Requirement: Steer degrades safely when no in-flight turn exists
当 `turn_action="steer"` 被投递到一个没有 in-flight turn 的实例时(例如实例空闲,或当前 job 非 app_server),系统 MUST 采取安全降级策略:

- 不丢消息.
- 允许该输入在后续 turn 被处理(例如作为下一次 job 的普通 pending event).

#### Scenario: Steer is queued when no active turn exists
- **GIVEN** 目标实例当前不存在 in-flight turn
- **WHEN** 系统投递一条带 `turn_action="steer"` 的事件到该实例
- **THEN** 系统 MUST 不丢弃该事件,并保证其仍会在后续执行中被处理(以 best-effort 方式)
