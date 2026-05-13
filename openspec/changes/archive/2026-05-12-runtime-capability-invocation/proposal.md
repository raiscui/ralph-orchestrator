## Why

用户现在要的已经不只是“启动时自动选一套默认 workflow”。

还希望:

- 单个 hat 可以像 skill / tool 一样被 `ralph#1` 按需调用
- 整套 workflow(`ralph.yml`) 也可以像 capability 一样被 `ralph#1` 运行时选用
- 初始化时先只给 LLM 注入轻量描述,例如 workflow summary、goal、hat description
- 真正需要时再把对应 workflow / hat 拉起来执行,体验上接近 sub-agent

当前代码边界说明这不能直接塞进 `startup-resource-bootstrap`:

- 串行 `EventLoop` 里,custom hats 现在主要是拓扑定义,真正执行者仍是 `ralph`
- 并行 `ParallelSupervisor` 虽然支持 `spawn_dynamic_instance`,但它只能扩容“当前 config 中已存在的 hat 模板”
- 当前没有正式机制把一套新的 workflow 或新的 hat 定义在真实 run 中注入到 live topology

因此更稳的方向不是“运行中热切换活跃拓扑”,而是单独建设一个 runtime capability invocation 层。

## What Changes

- 在 startup resource catalog 之上新增 runtime capability catalog 语义:
  - `workflow_capability`
  - `hat_capability`
- 为 capability 增加轻量结构化 metadata:
  - summary
  - goal
  - when_to_use
  - input_contract
  - output_contract
- 允许 `ralph#1` 在接到用户消息后:
  - 选择继续使用当前 base workflow
  - 或调用某个 workflow capability
  - 或调用某个 hat capability
- workflow capability 采用隔离 child run / nested run 执行,而不是改写当前 live topology
- hat capability 在 v1 也采用隔离 micro-run 执行,避免直接热改 `HatRegistry` / `Supervisor`
- 明确记录 v1 / v2 路线:
  - v1: 规则驱动 capability chooser + 隔离调用
  - v2: 规则优先 + LLM fallback chooser,并支持多 capability 组合计划

## Capabilities

### New Capabilities

- `capability-invocation`: 让 `ralph#1` 在运行时按 catalog metadata 选择并调用 workflow / hat capability

## Impact

- 受影响代码区域:
  - `crates/ralph-core`: capability metadata, invocation protocol, child run orchestration
  - `crates/ralph-cli`: capability listing, doctor/debug output, invocation artifacts
  - `presets/` / `examples/`: capability metadata source与 materialization 规则
  - docs / getting-started / workflow authoring docs
- 受影响行为:
  - `ralph#1` 不再只能在单套启动拓扑内部协调
  - 运行时可以按需调用隔离 capability run
  - 但当前 live topology 仍保持稳定,不会被直接热切换
