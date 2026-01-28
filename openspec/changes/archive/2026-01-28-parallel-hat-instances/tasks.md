## 1. 基础类型与状态机

- [x] 1.1 梳理现有 `EventLoop`/hat 执行路径，标出需要替换/扩展的入口点（确保不把 orchestrator 变成平台）
- [x] 1.2 定义 `HatInstanceId` 与实例状态枚举（create/running/idle/done/failed）
- [x] 1.3 定义 TopicContract 数据结构：`delivery=queue|fanout`、AudienceSelector、queue_selection、missing_instance_policy 等
- [x] 1.4 定义并落盘“路由决策记录”事件结构（候选集 + 选择结果 + 可选原因），用于 replay

## 2. HatInstance 运行时（并行 headless job）

- [x] 2.1 实现 HatInstance actor 骨架：inbox/outbox、最小状态机、可并发运行多个实例
- [x] 2.2 定义 HatJob：一次 headless CLI invocation 的输入/输出/退出码/超时等元数据
- [x] 2.3 实现 CLI 进程 runner：流式采集 stdout/stderr，并把输出归因到 `HatInstanceId`
- [x] 2.4 支持 job 取消/超时，确保实例可独立结束且不阻塞其他实例

## 3. 事件路由（queue / fanout）

- [x] 3.1 加载并验证 TopicContract 配置（要求显式声明 delivery 语义，不允许隐式 broadcast）
- [x] 3.2 实现 recipients 计算：`TopicContract.audience ∩ Event.audience_override`（有 override 时）
- [x] 3.3 实现 `audience_override.instances` 默认 best-effort：missing 时走 `missing_instance_policy`
- [x] 3.4 实现 `audience_override.require_delivery=true`：missing 时视为投递失败并触发 escalate
- [x] 3.5 实现 queue 选择：支持 `deterministic`（round-robin/least-busy）与 `llm`（决策型 job）
- [x] 3.6 对 queue 选择强制落盘：记录候选集与最终选择，replay 时不重算
- [x] 3.7 实现 fanout：向所有 recipients 投递，并保证每个 recipient 都可观测收到事件

## 4. Human gate（异步 + 可选超时）

- [x] 4.1 定义 gate 协议事件：`gate.request` / `gate.resolve` / `gate.timeout`
- [x] 4.2 实现 gate 状态机：等待 human 时不阻塞其他 HatInstance
- [x] 4.3 实现超时路径：超时后由决策型 job 继续推进，并落盘决策结果

## 5. Workspace 策略与权限

- [x] 5.1 定义 workspace 策略：shared/patch/worktree（至少覆盖 shared 与 worktree）
- [x] 5.2 实现 capability 白名单与 permission 策略（allow/ask/deny），覆盖高风险操作
- [x] 5.3 实现 worktree acquire/release，并支持 hooks（on_acquire/on_release）
- [x] 5.4 hooks 失败时发布 `workspace.hook_failed`，实现 bounded retry + escalate 的自愈回路

## 6. Supervisor 展示与输出聚合

- [x] 6.1 TUI/日志：展示实例列表与实例状态（running/idle/done/failed）
- [x] 6.2 TUI/日志：支持按实例查看输出（输出必须带 `HatInstanceId` 标识）
- [x] 6.3 增加 human async chat 的最小可用通道（能发送/接收并落盘为事件）

## 7. Replay smoke tests（确定性护栏）

- [x] 7.1 新增 fixture：`writer#1` 与 `tester#1` 并行执行（验证“真并行 + headless”）
- [x] 7.2 新增 fixture：best-effort missing instance（验证 missing_instance_policy 行为）
- [x] 7.3 新增 fixture：require_delivery missing instance（验证 escalate 行为）
- [x] 7.4 新增 fixture：queue 选择落盘 + replay 不重算（验证确定性）
- [x] 7.5 跑通并固化 smoke tests：`cargo test -p ralph-core smoke_runner`

## 8. 文档与收尾验证

- [x] 8.1 更新相关用户文档/配置示例，明确 BREAKING 行为与迁移建议
- [x] 8.2 全量检查：`cargo fmt --check`、`cargo clippy`、`cargo test`
