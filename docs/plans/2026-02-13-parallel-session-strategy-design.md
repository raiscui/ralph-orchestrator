# 设计: 并行模式下的动态会话策略(session_strategy) (2026-02-13)

## 背景

在 parallel 模式里:

- `ralph#1/#2` 目前走 Codex MCP 常驻会话,可以持续对话(复用 thread).
- 其他 hat instance 目前走 `codex exec` 单次 invocation,天然无状态.

这会带来一个协作断层:

- 需要多轮追问的工作,如果落到 exec hat 上,会频繁重复上下文,沟通成本高.
- 如果我们把所有 hat 都改成常驻,又会增加资源占用和状态管理复杂度.

因此我们选择混合模式(C),但不使用静态配置写死"哪个 hat 永久常驻".
我们希望由 ralph 在发布事件时,动态决定是否需要更强的对话连续性.

## 目标

- 由 ralph 在 `<event ...>` 上显式表达"这次投递希望走 exec 还是 mcp".
- 保持 replay 可复现,避免依赖隐式 thread 状态导致回放失真.
- 方案1(只升级,不降级): 同一 instance 一旦进入 mcp,后续保持 mcp,避免上下文分裂.

## 非目标

- 不做显式降级(mcp -> exec)与 reset 协议.
- 不做跨进程恢复 threadId(进程重启后 thread 仍需重建).
- 不把所有 backend 统一成"持久会话"; 当前只把它作为 executor 的选择提示.

## 方案概述

### 1) 在 Event 中新增字段: session_strategy

新增 enum:

- `SessionStrategy = exec | mcp`

并在 `Event` 增加可选字段:

- `session_strategy: Option<SessionStrategy>`
  - 缺失时等价于 `exec`.

### 2) 扩展 `<event ...>` 属性解析

允许 agent 输出:

```text
<event topic="build.task" target="writer" session_strategy="mcp">...</event>
```

EventParser 负责把 `session_strategy` 解析进结构化 Event.

### 3) 并行 HatInstance 的合并与 sticky 规则(方案1)

同一 instance 的 pending events 会合并成一个 job.
合并规则:

- 默认 `exec`.
- 只要 pending 里出现任意 `session_strategy=mcp`,本次 job 视为 `mcp`.

sticky 规则(只升级,不降级):

- instance 初始为 exec.
- 一旦某次 job 进入 mcp,该 instance 永久 sticky 到 mcp.
- 后续即使事件不再显式写 `session_strategy`,也仍保持 mcp.

### 4) executor 选择 mcp/exec

并行模式 executor 根据 HatJob 的 `session_strategy` 选择运行方式:

- `session_strategy=exec` -> 走一次性 `codex exec ...`
- `session_strategy=mcp` -> 走 `codex mcp-server` 常驻会话,并按 instance 复用 thread

保留现有行为:

- `ralph#1/#2` 固定走 MCP(不依赖事件是否显式请求)

## 使用建议(给 ralph 的决策启发)

当事件可能触发多轮协作时,推荐 ralph 给该事件加 `session_strategy="mcp"`:

- 需要连续追问,需要澄清,需要迭代性输出.
- 需要把“人类反馈”与“先前输出”结合起来继续推进.

当工作明确且一次性可交付时,保持默认 exec:

- 明确的单步实现/单次改动.
- 纯机械化生成,不需要追问.

## 测试与验证

- 单元测试:
  - EventParser 能解析 `session_strategy`.
  - HatInstanceActor 的 merge 规则与 sticky 行为正确.
- 回归验证:
  - `cargo test` 全量通过.

