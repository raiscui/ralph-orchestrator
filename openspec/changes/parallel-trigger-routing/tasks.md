## 1. 配置与协议

- [x] 1.1 设计并加入并行 autoscale 配置项（默认 max=4、idle_ttl=30s），并在配置校验/README 示例中体现默认语义
- [x] 1.2 扩展 `Event` 协议，加入 per-event `workspace_strategy` override（shared|patch|worktree），并补齐序列化/反序列化与文档
- [x] 1.3 定义并实现“控制面 topic 特例”配置/常量（用于绕过 target 校验），并写清默认列表

## 2. 并行路由：TopicContract 可选 + triggers 默认 fanout

- [x] 2.1 移除并行启动对 `parallel.topic_contracts` 的硬依赖：允许为空，并移除/调整 “必需 topic contract” 的启动校验
- [x] 2.2 实现 trigger-driven 的 hat 选择（对齐 `EventBus` 的 specific > wildcard 优先级），作为 TopicContract 未命中时的默认路由
- [x] 2.3 实现严格校验：`event.target` / `event.target_instance` 必须订阅该 topic；失败时 warn + escalate，并禁止投递（控制面 topic 走特例）
- [x] 2.4 实现“hat-level fanout / instance-level queue”：每个订阅 hat 只选择 1 个实例执行（不对该 hat 的所有实例 fanout）

## 3. 实例调度：idle-first + 自动扩缩容 + 全局并发上限

- [x] 3.1 增加 per-hat 的实例序号分配器（单调递增、永不复用），并区分 base 实例与 dynamic 实例
- [x] 3.2 实现实例选择策略：idle-first + deterministic tie-break（同 rank 多候选时按稳定顺序）
- [x] 3.3 实现 autoscale：当该 hat 全忙且全局并发未达上限时，动态创建新实例并投递
- [x] 3.4 实现全局并发上限（默认 4）：用 permit/semaphore 约束进入 Running 的 job 数量，避免 oversubscribe
- [x] 3.5 实现 dynamic 实例 idle 回收（默认 30s）：超时后 shutdown 并从 registry 移除（只回收 dynamic，不回收 base）

## 4. Workspace override：Event 字段 + 合并规则

- [x] 4.1 扩展 `<event ...>` parser：支持解析 `workspace_strategy` 属性，并把它写入 Event（外部 events.jsonl 同样支持）
- [x] 4.2 HatInstance 合并 pending events 成 job 时，按 `worktree > patch > shared` 规则计算 job 的最终 workspace_strategy
- [x] 4.3 将 workspace override 与现有 capability/permission gate 结合：若 override 请求 worktree 但被拒绝，定义清晰的降级/升级路径（并落盘可观测）

## 5. 单测与并行路由护栏

- [x] 5.1 新增单测：无 topic_contracts 时，按 triggers fanout 到多个 hats，并能并发启动多个 job
- [x] 5.2 新增单测：同一 hat 多实例时，默认只选择 1 个实例执行（instance-level queue）
- [x] 5.3 新增单测：`event.target`/`target_instance` 指向未订阅者时必须拒绝，并触发 escalation
- [x] 5.4 新增单测：autoscale 在 cap 未达时会创建新实例，在 cap 达到时不会创建（且实例 key 单调递增）
- [x] 5.5 新增单测：dynamic 实例 idle 30s 自动回收（使用 tokio 时间控制，避免 flaky）
- [x] 5.6 新增单测：workspace merge 规则 `worktree > patch > shared` 生效

## 6. E2E 与 Smoke Fixtures

- [x] 6.1 更新并行 E2E 场景：支持最小配置（不写 topic_contracts）也能跑通 fanout 并发闭环，并校验事件落盘
- [x] 6.2 新增 replay smoke fixture：覆盖“triggers 默认路由 + 并发执行 + 事件解析/落盘”的确定性回放
- [x] 6.3 更新现有并行 E2E 断言：补充 autoscale（实例数增长）与 target 校验失败的可观测信号（如 routing.escalate）

## 7. 文档同步（避免语义分裂）

- [x] 7.1 更新 `README.md` 的并行章节：移除“必须显式 topic_contracts”，补充默认 triggers 路由与 overrides 写法
- [x] 7.2 更新 `specs/parallel-hat-instances.spec.md`：同步默认路由语义（topic→hats fanout、hat→instance queue）、autoscale（max=4/30s）、workspace override 与 target 校验规则
