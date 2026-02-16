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
