## 1. 协议与解析

- [x] 1.1 扩展 `SessionStrategy` 支持 `app_server`(含解析与序列化)
- [x] 1.2 协议层新增 `turn_action`(start/steer/interrupt),并扩展 `<event ...>` 解析
- [x] 1.3 扩展外部 JSONL 事件结构,支持写入/读取 `session_strategy` 与 `turn_action`

## 2. core 并行运行时(合并规则 + in-flight 控制)

- [x] 2.1 session_strategy 合并与 sticky 规则扩展为 `exec<mcp<app_server`
- [x] 2.2 为 HatJob 增加 turn_action 运行时字段(默认 start)
- [x] 2.3 扩展 `HatJobExecutor`/HatInstance actor,支持 in-flight `Steer` 控制通道
- [x] 2.4 实现 `turn_action=interrupt` 的控制面语义(不进入 LLM pending 列表)

## 3. ralph-cli: Codex App Server runtime

- [x] 3.1 新增 `CodexAppServerRuntime`(spawn app-server + thread/start + turn/start + 输出 delta)
- [x] 3.2 打通 `turn/steer`(expectedTurnId) 与 `turn/interrupt`(turnId) 到运行态控制通道
- [x] 3.3 并行执行器按 `session_strategy=app_server` 选择 App Server runtime

## 4. TUI: steer/interrupt 入口

- [x] 4.1 chat 解析新增 `!steer` 与 `!interrupt` 命令(支持可选 `@instance`)
- [x] 4.2 外部事件写入: `!steer/!interrupt` 落盘携带 `session_strategy/turn_action`

## 5. 测试与验证

- [x] 5.1 单元测试: 解析 `session_strategy=app_server` 与 `turn_action`
- [x] 5.2 单元测试: core 合并/sticky 与 steer 降级策略
- [x] 5.3 单元测试: TUI chat 命令解析 + 外部事件 JSONL 序列化
- [x] 5.4 验证: `cargo fmt` + `cargo test` + `cargo test -p ralph-core smoke_runner`

## 6. 收尾同步

- [x] 6.1 更新 `WORKLOG.md` 记录实现与验证证据
- [x] 6.2 从 `LATER_PLANS.md` 清理已落地条目,保留未完成后续项
