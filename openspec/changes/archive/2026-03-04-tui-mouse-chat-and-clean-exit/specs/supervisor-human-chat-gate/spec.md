## ADDED Requirements

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
