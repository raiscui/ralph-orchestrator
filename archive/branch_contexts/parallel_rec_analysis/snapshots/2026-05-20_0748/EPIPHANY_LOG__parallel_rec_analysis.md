## [2026-05-18 11:54:26] [Session ID: omx-1779004640353-blcixq] 主题: all_hat_prompt 必须纳入 shared-only prompt surface 审计

### 发现来源
- `$ralplan specs/ralph-prompt-role-layering.md` 的 Architect 反馈和最终共识计划。

### 核心问题
- 如果只测试 worker prompt 不含若干字符串,而不约束 `all_hat_prompt`,全局 overlay 仍然可能把 coordinator-only policy 注入给 worker。

### 为什么重要
- 这会让 Ralph/worker 的职责分层重新失效,并让 simple task 再次被全局流程性 instruction 拖成长推理。

### 未来风险
- 后续新增 `config/all_hat.md` 内容时,可能无意加入 topology policy、task decomposition policy、runtime capability catalog 等 coordinator-only surface。

### 当前结论
- 必须引入很薄的 `PromptSurface` 语义层,并把 `all_hat_prompt` 约束为 `SharedProtocol` / universal safety。
- 不能只靠字符串 contains/excludes 测试。

### 后续讨论入口
- 先看 `.omx/plans/ralph-prompt-role-layering-consensus-plan.md` 的 Slice A1/A2。

## [2026-05-18 17:26:00] [Session ID: omx-1779004640353-blcixq] 主题: hat capability 不能用 nested Ralph loop 假装 worker

### 发现来源
- 方案 B 实现过程中,`hat:*` 默认 execute 的 focused integration 失败。
- 手工复现显示 backend 已输出结果,但 nested Ralph loop 仍因 consecutive completion / max_iterations 失败。

### 核心问题
- 为了复用 child run 入口而把 `hat:*` worker 包进 `ralph run`,会让一个 bounded reviewer 重新继承 Ralph coordinator prompt 和 loop completion 规则。

### 为什么重要
- 这正是用户反馈“简单问题持续 thinking 但没有结果”的机制之一。
- 即使代码层面不再 dry-run,如果执行模型仍是 nested coordinator,token 消耗和职责漂移仍然存在。

### 未来风险
- 后续如果新增 task-derived dynamic hat,不能默认塞进现有 Ralph loop。
- 需要先判断它是完整 workflow 还是 transient worker。
- transient worker 应优先走窄 prompt + direct backend execution。

### 当前结论
- `hat:*` execute 已改为 direct backend execution。
- `workflow:*` 仍保留 isolated child run,因为语义不同。

### 后续讨论入口
- 继续设计 task-derived dynamic hat live topology 时,先读 `specs/hat-capability-execute-preview.md` 和本条记录。


## [2026-05-18 17:39:46] [Session ID: omx-1779004640353-blcixq] 主题: parallel workflow 的硬终止必须由 `ralph#1` 统一输出

### 发现来源
- `workflow:*` record-session dogfood 中,`confession_handler#1` 输出 `LOOP_COMPLETE` 但 child run 不退出。

### 核心问题
- 并行 Supervisor 的硬终止信号只接受 `ralph` hat 输出的 completion promise。
- 如果 worker 直接输出 completion promise,UI/stdout 看起来已经完成,但 record-session 不会出现 `_meta.termination`。

### 为什么重要
- 这会再次制造“持续输出/看似完成但没有结果”的体验问题。
- 也会让 capability child run 没有可审计 termination,无法证明 invocation 真正收敛。

### 当前结论
- worker 应发布 `event_loop.complete_publishes` 配置的 completion candidate topic。
- `ralph#1` 观察 completion candidate 后输出 `event_loop.completion_promise`。
- `workflow:*` child run 必须携带 `--record-session`,把这个收敛过程保存为主证据流。

### 后续讨论入口
- 继续看 `openspec/specs/parallel-hat-instances/spec.md` 的 completion candidate 语义。
- 继续看 `openspec/specs/capability-invocation/spec.md` 的 child record-session evidence 要求。

## [2026-05-18 19:08:00] [Session ID: omx-1779004640353-blcixq] 主题: 事件属性只列名字不列 schema 会诱导模型编造无效协议

### 发现来源
- 分析正在执行的 `parallel_rec.jsonl` 和 `.ralph/capability-invocations/cap-1779101957035/child-record-session.jsonl`。

### 核心问题
- runtime prompt 只告诉模型 supported attributes 包括 `spawn_instance`,但没有在同一处说明它是 boolean,且必须配合 `target="<hat_id>"` 使用。
- 模型因此生成了 `spawn_instance="3"` 和 `spawn_instance="builder#1,builder#2,builder#3"` 这种看起来合理、但 runtime 不支持的写法。

### 为什么重要
- 这会让用户看到“我已经反馈 event 了”,但 runtime 没有对应的新实例 evidence。
- 如果没有 record/evidence 解析,很容易误判成 TUI 没刷新或实例创建慢。

### 未来风险
- 任何只列属性名、不列类型/必填组合/反例的事件协议,都可能被 LLM 按自然语言猜扩展。
- 这类错误不会总是显式失败;更糟的是会表现为无动作、长 thinking、或 child run 无 termination。

### 当前结论
- 当前 `spawn_instance` 不是“创建 N 个实例”的产品接口。
- 当前要显式开新实例必须使用合法的 boolean + target hat 组合。
- 如果产品要支持“按任务派生 3 个新 hat 身份”,需要独立设计 task-derived dynamic hat 协议,不能复用这个模糊字段。

### 后续讨论入口
- 先看 `crates/ralph-core/src/event_emission_protocol.rs`。
- 再看 `crates/ralph-core/src/parallel/supervisor/routing.rs` 中 explicit spawn 分支。
- 最后对齐 `openspec/specs/parallel-hat-instances/spec.md` 与 task-derived dynamic hat 设计。

## [2026-05-18 22:25:13] [Session ID: omx-1779004640353-blcixq] 主题: `ralph#1` 不能按普通 hat backend 配置来理解

### 发现来源
- 读取 `crates/ralph-core/src/parallel/supervisor.rs` 的 `spawn_instances()` / `spawn_instance()`。
- 读取 `crates/ralph-core/src/parallel/instance.rs` 的 job backend 合并逻辑。
- 读取 `crates/ralph-core/src/event_loop/mod.rs` 的 `get_hat_backend()`。

### 核心问题
- `ralph#1` 是 fallback synthetic coordinator,不是 `HatRegistry` 里按配置正常生成的 hat 实例。
- 它在特例分支里被直接构造,并且 `hat_config` 传 `None`。
- 这意味着 `hats.ralph.backend` 这类配置直觉在当前实现里会失效或至少不可靠。

### 为什么重要
- 用户很容易以为“只要写一个 `hats.ralph.backend` 就能单独关 hooks”。
- 但当前代码的真相源不是这个配置项,而是 fallback special-case。
- 如果不先确认这一点,后续任何 YAML 方案都可能是假生效。

### 未来风险
- 以后再做 coordinator-only backend / hooks split 时,很容易把它误当成普通 hat 的继承问题。
- 这会导致错误地把改动写到全局 `cli.args`,从而误伤所有 hats。

### 当前结论
- 当前最稳的静态结论是: `ralph` 的 backend/hook 行为需要单独设计,不能直接复用普通 hat 配置路径。
- 如果不改代码,只能依赖外层 wrapper 按 `RALPH_HAT_ID` 分流参数。

### 后续讨论入口
- 下次如果要继续做方案设计,先看 `crates/ralph-core/src/parallel/supervisor.rs:888-987` 和 `crates/ralph-core/src/parallel/instance.rs:801-809`。


## [2026-05-19 09:11:12] [Session ID: omx-1779004640353-blcixq] 主题: `audience_instances` 不是动态实例创建协议

### 发现来源
- 当前 `parallel_rec.jsonl` 与 `.ralph/capability-invocations/cap-1779152487480/child-record-session.jsonl`。
- 源码: `event_parser.rs`, `routing.rs`, `capability.rs`。

### 核心问题
- coordinator 可能把 `audience_instances` 理解成“我要这些实例存在并执行”。
- 但当前 runtime 中它只是 audience override,不是 instance creation。
- 当没有 TopicContract 时,fallback trigger 路由甚至会忽略这个 override 的创建期待,最终把任务交给已有 hat 实例。

### 为什么重要
- 这会制造“模型说创建了 3 个实例,但 runtime 没看到”的高混淆体验。
- 如果只看 stdout,很容易误判为 TUI 没刷新。
- 真相必须看 `runtime.lifecycle`, `runtime.delivery`, `.ralph/agents.json`,以及 invocation artifacts。

### 当前结论
- 本轮已经验证: `workflow:default-parallel` 是 isolated child run,父 topology 不变。
- 本轮已经验证: 三个视角被放进 `builder#1` 的任务 payload,没有 materialize 成 `builder#功能补充` 等实例。

### 后续讨论入口
- 继续设计 task-derived dynamic hat protocol 时,优先处理 `audience_instances` / `spawn_instance` / role contract 的边界。

## [2026-05-19 10:17:56] [Session ID: omx-1779004640353-blcixq] 主题: parent_topology_unchanged 不是配置开关,而是结果证据

### 发现来源
- 在分析 capability.request 和 dynamic spawn 的边界时,反复核对了 runtime capability, routing, TUI 和 record-session。

### 核心问题
- 当前 capability.request 的官方语义是 isolated child/micro-run,明确不应修改 parent topology。
- 如果把 parent_topology_unchanged 当成可配置开关,会把“确实创建了实例”与“只是 child run”混为一谈。

### 为什么重要
- 这直接决定用户看到的东西到底是真实例,还是一个可观测投影。
- 也决定了后续 UI 是否需要单独展示 child run 状态,而不是把它塞进 instances 列表。

### 未来风险
- 如果没有把这条边界写进协议和 UI,后续 coordinator 仍然会继续发出“看起来像多实例,实际上只是一个 child run”的请求。
- 这会让用户一直误判“event 已经发出却没跑实例”,并反复追着 UI/刷新查问题。

### 当前结论
- 真实例创建要走真正的 spawn 语义和 runtime lifecycle。
- child run 要走独立的 parent-observable 轨道。
- 两者都需要可见,但不能混在同一真相源里。

### 后续讨论入口
- 下次如果要落地,优先先写 specs,把 parent-visible spawn 和 parent-observable child run 的协议拆开。
