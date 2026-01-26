# WORKLOG

> 用于记录本次任务的最终产出与关键结论（在完成时追加到文件末尾）。

## 2026-01-26 14:21:11 +0800｜并行 HatInstance + Supervisor + Human Async Loop（设计稿）

### 产出
- 新增设计规格：`specs/parallel-hat-instances.spec.md`
  - 已包含 `flowchart`（组件/数据流）与 `sequenceDiagram`（一次 worktree job 的完整时序）
  - Mermaid 语法已用 `mermaid-validator` 校验通过

### 关键决定（已对齐）
- 架构：选择 **HatInstance Actor 模型**（tokio task/actor 负责调度与状态；实例之间并行）
- 并行范围：支持“不同 hat 并行”与“同 hat 多实例并行”；并行 hats 全部 headless
- 执行模型：**每个 job = 一次 CLI invocation**（codex/claude code/...），并行的本质是“多个 CLI 子进程并行”
- 事件语义：topic 投递语义 **必须显式声明** `queue | fanout`，并支持 **实例级（HatInstanceId）受众限制**
- Workspace/权限：
  - 临时 worktree（每 job 一次）可用，但必须同时满足：hat capabilities 允许 + LLM preflight 建议
  - capabilities 采用字符串白名单（例如 `workspace.worktree` / `git.merge` / `verify.tests`）
  - 权限条目 1-5 全部存在，初期默认 allow（未来可切 ask/deny）
  - worktree hooks：`on_acquire` / `on_release`（pre/post script），由 hat 设计者配置（包括 submodules 初始化）

### 实现前的硬门槛（spec 已点名）
- headless 执行必须支持“每次 invocation 指定 cwd”（否则 worktree 无法生效）
- headless 输出必须“真流式”（否则 Supervisor TUI 无法实时展示并发实例输出）

## 2026-01-26 15:15:37 +0800｜补充：缺失实例引用的最佳处理

### 背景
- 并行 + 实例可动态结束 + human async loop，会自然产生“消息/事件指向不存在的实例（如 writer#2）”。
- 这不是靠一个全局 A/B/C 就能优雅解决的问题。

### 设计补充（已写入 spec）
- 在 `specs/parallel-hat-instances.spec.md` 增补 5.5：
  - 区分短生命周期目标（HatInstanceId）与长生命周期目标（ThreadId/WorkItemId）
  - human async chat 推荐以 `ThreadId` 为路由主键，实例仅作为可变 owner
  - 缺失策略按消息类型决定，并将路由决策写入事件日志，保证 replay 可复现

## 2026-01-26 15:42:18 +0800｜补充：LLM 决策边界（提议 vs 执行）

### 背景
- 用户问：“可否由 LLM 决策？”
- 并行系统里，如果不明确“谁有最终决策权”，会直接破坏可回放与安全 gate。

### 结论（已写入 spec）
- 推荐模式：**LLM 提议 + Supervisor 校验/执行**。
  - LLM 负责给出策略提议（例如是否启用 worktree、要跑哪些测试、是否 spawn 新实例）。
  - Supervisor 负责 capabilities/permissions/human gate，并做机械执行与全局仲裁。
- 规格落点：`specs/parallel-hat-instances.spec.md` 新增 `7.3 可否由 LLM 决策？（推荐：LLM 提议 + Supervisor 执行）`

## 2026-01-26 15:53:02 +0800｜补充：queue 派发由 LLM 决策 + human gate 超时

### 新增用户决定
- `queue` 派发（投递到哪个具体实例）由 LLM 决策（用户选 B）。
- human gate 支持两种模式：
  - 普通 gate：等待 human
  - 超时 gate：默认 60s，超时后由 LLM 自行决策

### 规格落点
- `specs/parallel-hat-instances.spec.md`：
  - `5.2` 增加 `queue_selection: llm | deterministic`
  - `5.3` 明确 queue 派发必须落盘（候选集 + 选择结果 + 可选原因）
  - `5.4.1` 新增 `gate.request / gate.resolve / gate.timeout` 协议（用于 consult/approval）
  - `8.2` UI 增加 gate 倒计时与 `!resolve` 命令

## 2026-01-26 16:05:41 +0800｜补充：approval 超时后允许 LLM 自决 + human 异步调整需求通道

### 新增用户决定
- `kind=approval` 默认也支持超时 gate（用户选 B）：
  - 等待最多 60s
  - 超时后由 LLM 自行决策 `approve|deny` 并继续
- human 可以随时 async 发送“调整需求/新约束/新信息”，不阻断并行 hats 的运行。
  - 你倾向用文件系统事件/日志做通道，并希望 LLM 能经常读取。

### 规格落点
- `specs/parallel-hat-instances.spec.md`：
  - `5.4.1` 明确 approval 也可超时自决，并保留 `timeout_seconds=null` 的“严格等待 human”能力
  - `5.4.2` 新增“Human 异步调整需求”机制：
    - `events.jsonl` 作为唯一真相（可回放）
    - `.agent/inbox/{hat_instance_id}.jsonl` 作为轻量 inbox（便于 LLM 高频读取）
    - `human.directive(priority=normal|urgent)` 作为事件形态（默认不打断；urgent 才允许 cancel+重启）

## 2026-01-26 16:18:37 +0800｜补充：LLM 决策层的工程落地方式

### 背景
- 用户指出：当前 Ralph orchestrator 并不会真的“调用另一个 LLM 做评审/派发”，所以担心 LLM 决策层无法落地。

### 结论（已写入 spec）
- 现状原因：multi-hat 只是拓扑注入，执行器永远是 ralph（`EventLoop::next_hat()` multi-hat 时总返回 ralph）。
- 目标态：方向1 HatInstance Actor 推翻该限制，让 reviewer/tester/decider 都能真正并行执行。
- 工程落地：不在 Rust 内接 LLM SDK，而是把“LLM 决策”实现成“决策类 HatJob”，同样通过 headless CLI invocation 完成。
  - 事件输出仍使用 `<event ...>`，复用 `EventParser`，并把决策落盘保证 replay。
  - 支持 batch 与 deterministic fallback 做成本/稳定性刹车。
- 规格落点：`specs/parallel-hat-instances.spec.md` 新增 `7.4 LLM 决策层怎么落地？`

## 2026-01-26 16:25:12 +0800｜补充：`ralph` hat 的来源与现状限制

### 发现
- `ralph` hat 是内置的 catch-all coordinator，不是 YAML 配置出来的。
  - `crates/ralph-core/src/event_loop/mod.rs:145` 在 EventLoop 初始化时无条件注册 `ralph` 并 `subscribe("*")`。

### 影响
- 这解释了“为什么现在看不到 reviewer/tester 单独调用 LLM”：现状 multi-hat 只是拓扑注入，执行器仍固定为 `ralph`。

## 2026-01-26 16:34:02 +0800｜决定：LLM 决策层默认使用内置 `ralph` hat

### 决定
- 你确认：第一版不引入新的 `decider` hat 名字。
- 决策类 HatJob（queue 派发、gate 超时自决等）默认使用 `hat_id="ralph"` 运行。

### 意义
- 仍然保持“LLM = 外部 CLI agent invocation”的架构，不把 SDK 接进 Rust。
- 只是在调度层面把“决策”也视为一种 job，并复用现有 `<event ...>` 输出 + `EventParser` + `events.jsonl` 的可回放链路。

## 2026-01-26 16:40:16 +0800｜决定：human async chat 以 `ThreadId` 作为路由主键

### 决定
- 你确认：human async chat 默认用 `ThreadId` 路由（长期存在）。
- `@writer#2` 这类实例引用只作为 UI alias，不作为长期可靠引用。

### 影响
- “指向不存在实例”的问题大幅缓解：实例结束并不影响 thread，消息不会丢，只会进入 thread inbox 等待重新分配。
- 更符合你要的 human-in-async-loop：人类对话像工单而不是进程控制。

## 2026-01-26 16:46:57 +0800｜决定：`audience_override.instances` 默认 best-effort

### 决定
- 你确认：点名实例（例如 `audience_override.instances=["writer#2"]`）默认语义是 **best-effort**。
- 指定实例不存在时：
  - 不视为失败
  - 按 `missing_instance_policy` 处理（spawn/queue/escalate/drop）
- 如需“必须送达”，事件可显式声明 `audience_override.require_delivery=true`，送不到就 `escalate`。

### 规格落点
- `specs/parallel-hat-instances.spec.md`：
  - `5.3` 增补 best-effort 默认语义与 `require_delivery` 开关
  - `13` 从“未决问题”改为“确认清单”，并标记该项已决定

## 2026-01-26 16:54:10 +0800｜决定：hooks 失败由 LLM 优先自愈（bounded）

### 决定
- 你确认：`on_acquire/on_release` hooks 失败后，默认让 LLM 先判断并尽量自我修复。
- 机制（已写入 spec）：
  - 先发布 `workspace.hook_failed`（阶段/attempt/退出码/输出落盘）
  - 再由 `ralph(decision)` 决策恢复动作（retry/repair_then_retry/escalate/abort），Supervisor 机械执行
  - bounded 重试：默认建议 `max_attempts=3`（含首次），超过即 abort 当前 job（不阻断其他 hats）

### 规格落点
- `specs/parallel-hat-instances.spec.md`：
  - `10` hooks 语义补充“自愈回路”
  - `13` 将 hooks 失败策略标记为已决定
  - `6.1` YAML 示例补充 `repair_commands` 与 `retry`（max_attempts/backoff）

## 2026-01-26 20:04:23 +0800｜/opsx:new：准备创建 OpenSpec change scaffold

- 已确认 `openspec` CLI 可用：`/Users/cuiluming/n/bin/openspec`。
- 已确认当前 `openspec/changes/` 目录仅有 `archive/`，暂无活跃 change。
- 下一步：等你确认本次 change 的名称/描述后，我会创建 `openspec/changes/<name>/`，并只输出首个可写工件的模板（不生成任何工件内容）。

## 2026-01-26 20:22:10 +0800｜/opsx:new：已创建 change scaffold，并输出首个工件说明

- 已执行：`openspec new change "parallel-hat-instances"`（schema: `spec-driven`）
  - 生成目录：`openspec/changes/parallel-hat-instances/`
- 工件状态：`openspec status --change "parallel-hat-instances"` → `0/4`
  - `proposal` ready
  - `design` blocked by `proposal`
  - `specs` blocked by `proposal`
  - `tasks` blocked by `design, specs`
- 已输出首个工件 `proposal` 的 instructions/template（未写入任何内容）：
  - 命令：`openspec instructions proposal --change "parallel-hat-instances"`
  - 目标文件：`openspec/changes/parallel-hat-instances/proposal.md`

## 2026-01-26 20:31:13 +0800｜/opsx:continue：创建 proposal.md

- 创建工件：`proposal`（schema：`spec-driven`）
  - 输出：`openspec/changes/parallel-hat-instances/proposal.md`
- 工件状态更新：`0/4` → `1/4`
- 解锁：`design`、`specs`

## 2026-01-26 20:41:09 +0800｜/opsx:continue：创建 design.md

- 创建工件：`design`（schema：`spec-driven`）
  - 输出：`openspec/changes/parallel-hat-instances/design.md`
- 工件状态更新：`1/4` → `2/4`
- 当前阻塞：`tasks` 仍 blocked by `specs`

## 2026-01-26 23:23:21 +0800｜/opsx:ff：补齐 specs + tasks，进入 apply-ready

- 创建工件：`specs`（schema：`spec-driven`）
  - 输出：`openspec/changes/parallel-hat-instances/specs/parallel-hat-instances/spec.md`
- 创建工件：`tasks`（schema：`spec-driven`）
  - 输出：`openspec/changes/parallel-hat-instances/tasks.md`
- 最终状态：`4/4 artifacts complete`（All artifacts complete）

- 校验：`openspec validate parallel-hat-instances --type change` → valid（2026-01-26 23:25:29 +0800）
