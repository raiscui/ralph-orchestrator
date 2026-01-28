## ADDED Requirements

### Requirement: 并行模式可启动 Supervisor TUI
当 `parallel.enabled=true` 且运行环境为可交互终端（TTY）时，系统 MUST 启动 Supervisor TUI 来展示并行实例状态与输出。
当不满足 TTY 条件时，系统 MUST 自动降级为日志模式，并给出清晰的降级原因。

#### Scenario: TTY 环境下启用 TUI
- **WHEN** 用户以并行模式运行，并显式启用 TUI（例如未传 `--no-tui` 且 stdin/stdout 为 TTY）
- **THEN** 系统启动 Supervisor TUI（而不是仅输出 “parallel mode has no TUI” 的警告）

#### Scenario: 非 TTY 环境下自动降级
- **WHEN** 用户在非交互环境运行（例如 stdout 不是 TTY）但请求启用 TUI
- **THEN** 系统不启动 TUI，并以日志提示“已降级为 log mode”与原因

---

### Requirement: Supervisor TUI 展示 HatInstance 列表并支持选择
Supervisor TUI MUST 展示当前已注册的 HatInstance 列表。
列表每一项 MUST 至少包含：

- `HatInstanceId`（例如 `writer#1`）
- 当前状态（例如 `Created/Idle/Running/Failed/Done`）
- 最近一次输出或事件的时间线索（用于“是否卡住”的快速判断）

Supervisor TUI MUST 支持通过键盘在实例列表中移动选择，并将“当前选中实例”同步到详情视图。

#### Scenario: 列表展示与选择同步
- **WHEN** Supervisor 已创建多个实例，并产生状态变更（Running/Idle/Done 等）
- **THEN** TUI 的实例列表可见这些实例及其状态
- **THEN** 用户切换选中项后，详情区展示对应实例的输出视图

---

### Requirement: Supervisor TUI 展示实例输出并支持滚动与搜索
Supervisor TUI MUST 按实例归因展示输出流。
输出视图 MUST 支持：

- 滚动查看历史输出
- 文本搜索（沿用现有 TUI 的 `/` 搜索心智）

#### Scenario: 输出实时追加且可搜索
- **WHEN** 某个实例持续产生 stdout/stderr 输出
- **THEN** TUI 详情区实时追加可见的输出内容
- **THEN** 用户输入 `/query` 后，TUI 能定位到匹配的输出行

---

### Requirement: 输出按 job 分段并可在 job 历史间切换
为了对齐并行模式的“多次 CLI invocation”语义，Supervisor TUI MUST 将每个实例的输出按 HatJob 分段。
同一实例的输出视图 MUST 支持在 job 历史之间切换，并明确显示当前正在查看的 job。

#### Scenario: 同一实例多次 job 的输出可分段查看
- **WHEN** 同一 HatInstance 连续执行两次及以上 HatJob
- **THEN** TUI 将其输出分成多个 job 段
- **THEN** 用户可以在这些 job 段之间切换查看
