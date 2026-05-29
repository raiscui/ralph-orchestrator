# supervisor-human-chat-gate

## Purpose
（TBD）Supervisor TUI 底部控制面（human async chat + gate）的规格：
- 展示待处理的 gate 列表与倒计时状态
- human 在 TUI 内 resolve gate，并落盘为 `gate.resolve` 外部事件
- human async chat 支持 `@<HatInstanceId>` 定向，并落盘为 `human.message` 外部事件
## Requirements
### Requirement: TUI 展示待处理的 Gate 并持续更新
Supervisor TUI MUST 在底部面板展示当前“待处理”的 gates（来自 `gate.request`）。
每个 gate 条目 MUST 至少展示：

- `gate_id`
- `requested_by`（HatInstanceId）
- `kind`（consult / approval）
- `prompt`（给 human 的问题）

若 `timeout_seconds` 存在，TUI MUST 展示倒计时，并在超时后把该 gate 标记为已超时（等待后续决策或 resolve）。

#### Scenario: gate.request 在面板中可见
- **WHEN** 系统产生 `gate.request` 事件（包含 gate_id / requested_by / prompt）
- **THEN** TUI 的 gate 面板出现对应条目，并显示关键字段

#### Scenario: 超时 gate 显示倒计时并变更状态
- **WHEN** `gate.request.timeout_seconds` 不为 null
- **THEN** TUI 显示倒计时
- **THEN** 倒计时归零后，TUI 将该 gate 标记为 timeout 状态

---

### Requirement: Human 可以在 TUI 内 resolve Gate，并落盘为事件
Supervisor TUI MUST 支持 human 在 UI 内对 gate 做出决策，并以 `gate.resolve` 事件形式写入外部事件流（`.ralph/current-events` 指向的 JSONL）。
对审批类 gate，UI MUST 提供“approve/deny”的快捷输入方式（例如 `!approve <gate_id>` / `!deny <gate_id>`）。
对咨询类 gate，UI MUST 支持输入自由文本并 resolve（例如 `!resolve <gate_id> <text>`）。

#### Scenario: approve 生成 gate.resolve
- **WHEN** 用户在 TUI 中对某个 approval gate 执行 approve 操作
- **THEN** 系统写入一条 `gate.resolve` 事件，且其 payload 能反序列化为 `GateResolve`

#### Scenario: resolve 文本咨询 gate
- **WHEN** 用户在 TUI 中对某个 consult gate 输入文本并提交
- **THEN** 系统写入一条 `gate.resolve` 事件，并包含 human 的决策内容

---

### Requirement: Human async chat 支持定向消息并落盘
Supervisor TUI MUST 提供 human async chat 输入框。
chat 输入 MUST 支持两种模式：

1. **定向消息**：以 `@<HatInstanceId>` 前缀定向到某个实例（例如 `@writer#2 继续尝试方案 B`）
2. **默认消息**：不带前缀时，写入默认 chat 事件（用于全局对话或默认 thread）

无论哪种模式，提交后系统 MUST 把消息写入外部事件流，保证可观测与可回放。
chat 消息事件 MUST 使用 topic `human.message`，且 payload MUST 为原始消息文本。

#### Scenario: 定向消息写入事件并带 target_instance
- **WHEN** 用户输入 `@writer#2 hello`
- **THEN** 系统写入一条事件，且其 `target_instance` 为 `writer#2`
- **THEN** 该事件 topic 为 `human.message`，payload 为 `hello`

#### Scenario: 非定向消息写入默认事件
- **WHEN** 用户输入一条不带 `@instance` 前缀的消息
- **THEN** 系统写入一条 `human.message` 事件，且不包含 `target_instance`

### Requirement: Chat 可用鼠标点击进入输入态并显示提示符
Supervisor TUI MUST 支持用户用鼠标点击 Chat 输入区域进入输入态，并在输入区域展示清晰可见的提示符（prompt）。

#### Scenario: 点击 Chat 区域后进入输入态
- **WHEN** 用户用鼠标点击底部的 Chat 输入区域
- **THEN** TUI 将焦点切换到 Chat 输入框
- **THEN** 输入框展示提示符，并显示可编辑光标（cursor）

---

### Requirement: Chat 输入支持多行与 Shift+Enter 换行
Chat 输入框 MUST 支持多行文本输入，并且 MUST 支持 `Shift+Enter` 插入换行。

#### Scenario: Shift+Enter 插入换行，Enter 提交
- **WHEN** 用户在 Chat 输入框中输入多行内容，并使用 `Shift+Enter` 插入换行
- **THEN** 输入框内容包含换行符并保持可编辑
- **WHEN** 用户按下 `Enter` 提交
- **THEN** 系统写入一条 `human.message` 事件，payload 为用户输入的原始文本（包含换行）

---

### Requirement: Chat 输入支持光标移动与文本选择
Chat 输入框 MUST 支持使用键盘与鼠标移动光标位置，并且 MUST 支持文本选择（包含多行选择与框选）。

#### Scenario: 键盘移动光标并选择文本
- **WHEN** 用户使用方向键在 Chat 输入框内移动光标（左右/上下）
- **THEN** 光标位置按预期移动，并保持在可编辑内容边界内
- **WHEN** 用户使用 `Shift+方向键` 扩展选择范围
- **THEN** 被选择文本在 UI 中可见高亮

#### Scenario: 鼠标点击定位光标并框选文本
- **WHEN** 用户用鼠标点击 Chat 输入框的某个位置
- **THEN** 光标移动到对应位置
- **WHEN** 用户按下鼠标并拖拽形成选择区域
- **THEN** 选择区域在 UI 中可见高亮，且用户后续输入会替换所选内容

---

### Requirement: 默认消息定向到当前选中实例
Supervisor TUI MUST 将“未显式指定 `@<HatInstanceId>` 的 chat 消息”定向投递到当前选中实例（`selected_instance`）。

说明：
- 该规则只影响并行 TUI 的 “human.message 外部事件落盘” 行为。
- 若用户显式输入 `@writer#2 ...`，则以显式 target 为准，不受 `selected_instance` 影响。

#### Scenario: 不写 @ 前缀时默认发给选中实例
- **GIVEN** 当前选中实例为 `writer#2`
- **WHEN** 用户在 chat 输入框中输入 `继续尝试方案 B` 并按下 `Enter` 提交
- **THEN** 系统写入一条 `human.message` 外部事件
- **THEN** 该事件 MUST 包含 `target_instance=writer#2`

#### Scenario: 写 @ 前缀时覆盖默认目标
- **GIVEN** 当前选中实例为 `writer#1`
- **WHEN** 用户输入 `@writer#2 hello` 并按下 `Enter`
- **THEN** 系统写入一条 `human.message` 外部事件
- **THEN** 该事件 MUST 包含 `target_instance=writer#2`（而不是 `writer#1`）

---

### Requirement: Targets chips 可点选切换默认目标
Supervisor TUI MUST 在 chat 面板中展示“目标实例 chips”（Targets），并允许用户鼠标点击以切换当前选中实例（`selected_instance`）。

要求：
- Targets MUST 覆盖当前并行运行时已注册的所有实例（包含 `ralph#1`）。
- Targets MUST 在 UI 中高亮当前选中实例，便于用户确认“默认目标”。
- 用户点击某个 target chip 后：
  - `selected_instance` MUST 切换为该 chip 对应实例；
  - Output 面板 MUST 同步切换为该实例输出（与点击左侧 instances 列表的语义一致）。

#### Scenario: 点击 target chip 切换选中实例
- **GIVEN** targets 列表包含 `@writer#1` 与 `@writer#2`
- **WHEN** 用户鼠标点击 `@writer#2`
- **THEN** 当前选中实例变为 `writer#2`

---

### Requirement: Gate 列表可点选为当前 Gate，并联动选中实例
Supervisor TUI MUST 在存在多个 gate 时，允许用户鼠标点击 gate 列表行，将其设为“当前 gate”（`selected_gate`）。

要求：
- 点击某个 gate 行后：
  - `selected_gate` MUST 设为该 gate；
  - `selected_instance` MUST 自动切换为该 gate 的 `requested_by`（便于立即对话/处理）；
  - Output 面板 MUST 同步切换到该实例输出。
- Chat 面板 MUST 显示当前 gate 的关键信息（至少包括 `gate_id`、`kind`、`requested_by`、`prompt`）。

#### Scenario: 点击 gate 行会选中 gate 并切换到 requested_by
- **GIVEN** gate 列表中存在 `gate_id=g1` 且 `requested_by=writer#2`
- **WHEN** 用户鼠标点击该 gate 行
- **THEN** 当前 gate 变为 `g1`
- **THEN** 当前选中实例变为 `writer#2`

---

### Requirement: Gate actions chips 支持点击预填命令
Supervisor TUI MUST 在 chat 面板中为当前 gate 提供可点击的快捷操作入口（Gate actions），用于快速预填 gate 命令到输入框。

要求：
- Gate actions MUST 至少包含：`!approve`、`!deny`、`!resolve`。
- 点击任意 action 后，TUI MUST 仅做“预填输入框”，不得自动提交发送（发送仍由用户按 `Enter` 触发）。
- 预填内容 MUST 自动包含当前 gate_id：
  - 点击 `!approve` → `!approve <gate_id>`
  - 点击 `!deny` → `!deny <gate_id>`
  - 点击 `!resolve` → `!resolve <gate_id> `（末尾保留一个空格，方便继续输入）

#### Scenario: 点击 approve 预填输入框但不自动发送
- **GIVEN** 当前 gate 为 `g1`
- **WHEN** 用户点击 `!approve`
- **THEN** chat 输入框内容变为 `!approve g1`
- **THEN** 系统不会立刻写入外部事件（需要用户按 `Enter` 提交）

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

### Requirement: Chat supports explicit recoverable continue control
Supervisor human input MUST support an explicit continue control for retrying a paused recoverable agent CLI failure.

The command surface MUST support targeting a specific recoverable `failure_id`, and MUST support a default target selected from the current paused recoverable failure or selected instance when unambiguous. A localized UI label such as `继续` MAY submit this explicit control action, but ordinary free-form chat text MUST NOT implicitly trigger a retry.

#### Scenario: Continue command targets a failure id
- **GIVEN** `.ralph/recoverable-failures.jsonl` contains a paused recoverable failure with `failure_id="failure-123"`
- **WHEN** the human submits `!continue failure-123` through Supervisor chat
- **THEN** the system MUST treat the input as a recoverable retry control action
- **THEN** the system MUST append a `continued_by_human` transition for `failure-123`

#### Scenario: Continue command can use selected paused failure
- **GIVEN** the Supervisor has a selected instance with exactly one paused recoverable failure
- **WHEN** the human submits `!continue` through Supervisor chat
- **THEN** the system MUST resolve the command to that paused recoverable failure
- **THEN** the system MUST enqueue a retry through the recoverable failure scheduler path

### Requirement: Ordinary chat does not implicitly retry failures
Supervisor human input MUST NOT treat ordinary chat messages as recoverable retry controls unless they use the explicit continue command or an equivalent structured UI action.

This requirement prevents ambiguous text such as `继续分析` from accidentally restarting an agent CLI process with side effects.

#### Scenario: Plain continue text remains chat
- **GIVEN** a selected instance has a paused recoverable failure
- **WHEN** the human submits plain chat text `继续分析这个问题`
- **THEN** the system MUST write an ordinary `human.message` event according to the existing chat rules
- **THEN** the system MUST NOT append a `continued_by_human` ledger transition
- **THEN** the system MUST NOT enqueue a retry solely because of that plain text

### Requirement: Continue control is auditable in human-facing evidence
Supervisor continue control MUST be visible as an auditable control transition rather than hidden executor behavior.

When a continue command is accepted, the system MUST expose enough evidence for record-session summaries, agents snapshots, or reports to show which failure was continued and which instance/job was retried.

#### Scenario: Accepted continue appears in evidence
- **WHEN** the human continue control is accepted for a recoverable failure
- **THEN** the recoverable failure ledger MUST include the `continued_by_human` transition
- **THEN** human-facing evidence MUST be able to identify the affected `failure_id`, `instance_id`, and retry attempt

## Change History
- 2026-01-28: Synced from `openspec/changes/archive/2026-01-28-parallel-supervisor-tui/specs/supervisor-human-chat-gate/spec.md`.
