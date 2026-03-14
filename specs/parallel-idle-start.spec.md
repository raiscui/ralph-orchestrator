# Spec: Parallel Idle Start(无 PROMPT.md 待机)

## 背景

Ralph 的并行运行时(Parallel Supervisor + HatInstance)是事件驱动的。
在 codex app-server 模式下,它天然适合常驻会话,并通过外部事件(`ralph emit`/TUI chat)持续注入 human 指令。

但当前实现存在两个阻塞点:

1. `ralph run` 在缺失 prompt 时会直接失败(例如 `PROMPT.md` 不存在)。
2. 并行 Supervisor 启动后会立刻投递 `task.start -> ralph#1`。
   这会触发一次真实后端 job,与“启动后待机(0 token)”目标冲突。

## 目标

实现并行模式的 "Idle Start" 启动方式:

- 当没有 `PROMPT.md` 且未传 `-p/-P` 时,并行 **TUI** 模式应自动进入待机,不要报错退出。
- 0 token 真待机:
  - 待机期间不启动任何 hat job(不 spawn codex app-server,不消耗 token)。
  - 不通过“先让模型输出 `LOOP_COMPLETE`”来进入暂停态。
- 无 `PROMPT.md` 的 idle_start 会话不受 Supervisor 级 `max_runtime_seconds` 限制。
  - 这包括“等待第一条外部消息之前”以及“第一条消息之后的持续对话阶段”。
  - 该模式仍保留其他收尾/护栏,例如 `job_timeout_secs`、`max_iterations`、人工中断。
- headless/CI/E2E 必须显式开关:
  - 新增 `ralph run --idle-start`。
  - 默认 headless 仍保持“缺 prompt 就报错”,避免脚本静默挂住。

## 非目标

- 串行运行时的待机模式(串行 TUI 是 observation-only,不具备并行 TUI 的外部事件交互模型)。
- 新增新的业务 topic 或强制改变现有事件协议(尽量通过最小开关实现)。

## 用户可见接口

### CLI

新增:

- `ralph run --idle-start`

约束:

- `--idle-start` 与 `--continue` 冲突(禁止同时使用)。
- `--idle-start` 仅允许在 `parallel.enabled=true` 时使用。

### TUI 自动待机(并行模式)

当满足以下条件时,无需显式 `--idle-start` 也应进入待机:

- 并行模式 + TTY(TUI 实际启用)。
- 没有 CLI prompt 覆盖(未传 `-p/-P`)。
- 默认 prompt 来源为 `PROMPT.md` 且文件缺失或内容为空白。

其他情况(例如用户显式指定 `prompt_file=foo.md` 但文件缺失)应保持原有报错,避免隐藏配置错误。

## 运行时语义

### Idle Start 启动

在 fresh run 且 idle_start=true 时:

- Supervisor **不自动投递** `task.start`。
- Supervisor 仍然:
  - `spawn_instances()`
  - 写入 `.ralph/agents.json` 快照(可观测)
  - 读取 `.ralph/current-events` 指向的外部事件 JSONL
- 只有当外部事件文件中出现新事件(例如 `human.message`)并被路由到某个实例时,才会触发第一次 job。

### max_runtime 计时

- idle_start / 无 `PROMPT.md` 会话:
  - Supervisor 级 `max_runtime` MUST 保持禁用。
  - 不论 `ralph#1` 是否已经开始处理第一条 `human.message`,都 MUST NOT 因 `max_runtime_seconds` 触发 `MaxRuntime`。
- 非 idle_start 的普通并行 run:
  - 行为保持现有语义: 超过 `max_runtime_seconds` 则以 `MaxRuntime` 终止并 cancel/shutdown。

## 验收标准

### 单元测试(0 token,确定性)

- `idle_start` 模式下:
  - Supervisor 启动后在一段 wall time 内不触发任何 job。
  - 超过 `max_runtime_seconds` 的 wall time 也不应退出(证明 idle 不计时)。
  - 注入一条 `human.message` 后,即使继续等待超过 `max_runtime_seconds`,也不应因 `MaxRuntime` 退出。
  - 注入一条 `human.message` 后,仍应能触发 ralph#1 job 并按其他退出条件正常收敛。

### E2E: fake codex(确定性)

新增场景:

- `parallel-app-server-idle-start`:
  - 不提供 prompt,workspace 内也没有 `PROMPT.md`。
  - 以 `--idle-start` 启动并行 ralph。
  - 在确认 ralph#1 持续处于 Idle 后,注入 `human.message`(包含 `121+43=?` 与 `10+5=?`)。
  - fake codex app-server 输出可审计的 `TASK_FEEDBACK: answer: 164/15` 与 `LOOP_COMPLETE`。

### E2E: live codex app-server(真实)

新增场景:

- `parallel-app-server-idle-start-live`:
  - 同样不提供 prompt,以 `--idle-start` 启动并行 ralph。
  - 注入 `human.message` 后,应在 stdout 中看到 `answer: 164`/`answer: 15` 与 `LOOP_COMPLETE`。
