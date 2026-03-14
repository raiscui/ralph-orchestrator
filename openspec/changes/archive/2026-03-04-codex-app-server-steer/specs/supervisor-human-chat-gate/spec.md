## ADDED Requirements

### Requirement: Chat supports !steer for in-flight instruction injection
Supervisor TUI 的 chat MUST 支持 `!steer` 命令,用于在目标实例运行中追加指令(真 steer).

要求:

- `!steer` MUST 写入一条外部事件(topic=`human.message`).
- 该事件 MUST 可定向到某个实例:
  - 若命令显式包含 `@<HatInstanceId>`,则 MUST 使用该实例作为 `target_instance`.
  - 否则 MUST 默认定向到当前选中实例(`selected_instance`).
- 该事件 MUST 携带会话与动作信号,以便运行时选择 App Server 并走 `turn/steer`:
  - `session_strategy="app_server"`
  - `turn_action="steer"`

#### Scenario: !steer defaults to selected instance
- **GIVEN** 当前选中实例为 `writer#2`
- **WHEN** 用户输入 `!steer 继续按方案 B 往下做`
- **THEN** 系统写入一条 `human.message` 外部事件
- **THEN** 该事件 MUST 包含 `target_instance=writer#2`
- **THEN** 该事件 MUST 包含 `session_strategy="app_server"` 与 `turn_action="steer"`

#### Scenario: !steer can target a specific instance
- **WHEN** 用户输入 `!steer @writer#1 请立即停下,改用方案 A`
- **THEN** 系统写入一条 `human.message` 外部事件
- **THEN** 该事件 MUST 包含 `target_instance=writer#1`
- **THEN** 该事件 MUST 包含 `session_strategy="app_server"` 与 `turn_action="steer"`

---

### Requirement: Chat supports !interrupt for canceling the active turn
Supervisor TUI 的 chat MUST 支持 `!interrupt` 命令,用于中断目标实例的 in-flight turn.

要求:

- `!interrupt` MUST 写入一条外部事件,并定向到目标实例(规则同 `!steer`).
- 该事件 MUST 携带 `turn_action="interrupt"`,让运行时对当前 turn 执行中断.

#### Scenario: !interrupt interrupts the selected instance
- **GIVEN** 当前选中实例为 `reviewer#1`
- **WHEN** 用户输入 `!interrupt`
- **THEN** 系统写入一条外部事件,且其 `target_instance=reviewer#1`
- **THEN** 该事件 MUST 包含 `turn_action="interrupt"`
