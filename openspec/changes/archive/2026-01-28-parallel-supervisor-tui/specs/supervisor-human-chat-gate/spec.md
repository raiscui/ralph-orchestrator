## ADDED Requirements

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
