## Why

Ralph 现在已经有两类“图”能力,但它们都还不是用户现在要的那种运行时关系图:

- `ralph hats graph` 可以在启动前输出 Mermaid / ASCII / Unicode 拓扑图
- `.ralph/events.jsonl`、`.ralph/agents.json`、stdout artifact 可以帮助排障

问题在于,这两类能力分别只覆盖了静态拓扑和原始证据,还没有形成一个真正的“node-like runtime graph”:

- 看不到 `ParallelSupervisor` 何时创建了哪些 hat instances
- 看不到消息是如何在 instances 之间流动的
- 看不到 completion freeze / shutdown 是如何影响实例生命周期的
- 看不到 workflow 事件链和 instance 关系图如何叠在一起
- 看不到“谁创建谁”“谁给谁发消息”“谁让谁停”的一等关系

用户现在希望把这些关系接进 Rerun,参考它的 graph showcase,把 Ralph 并行 runtime 变成一张可观察、可回放、可解释的图。

这件事不能简单理解成“再换一个渲染器”。
因为仓库里已经有 `ralph hats graph` 负责静态拓扑可视化。
新的 Rerun graph 必须明确补的是“运行时动态图”,而不是重复输出另一份静态 Mermaid。

另外,用户明确要求 V1 / V2 都要正式记录下来。
否则很容易只做了一个 V1 的可视化雏形,过一阵子就忘了更完整的 V2 应该是什么。

## What Changes

- 新增一个 Rerun-based runtime graph 方案,专门表达 Ralph 并行 runtime 的动态图关系
- 明确把图模型分成至少三层:
  - runtime topology
  - workflow event graph
  - delivery / reply trace
- 明确与现有 `ralph hats graph` 的边界:
  - `ralph hats graph` 继续负责启动前静态 topology
  - Rerun graph 负责 run 中的实例关系、状态变化和消息流
- 引入分期路线,并正式记录:
  - V1: live runtime graph
  - V2: durable replay graph
- V1 优先复用当前已有的 observer / snapshot / event log 能力
- V1 当前实现入口明确为:
  - `ralph run --runtime-graph-rrd <FILE>`
  - `ralph run --continue --runtime-graph-rrd <FILE>`
  - 仅在 `parallel.enabled=true` 时可用
  - 当前 artifact 形式是 Rerun `.rrd` 文件,不在 V1 承诺 viewer / TUI 集成
- V2 在需要时补齐 delivery 级 durable 观测,支持更完整的离线重建与回放

## Capabilities

### New Capabilities

- `runtime-graph-observability`: 用 Rerun 呈现并行 runtime 的实例关系、生命周期、workflow 链路与消息流

### Modified Capabilities

- `hat-collections`: 保持现有静态 topology 图职责,并在文档上明确与 runtime graph 的边界

## Impact

- 受影响代码区域:
  - `crates/ralph-core`
    - `parallel/supervisor`
    - `parallel/supervisor/routing`
    - `parallel/instance`
    - `event_logger`
  - `crates/ralph-cli`
    - 新的 Rerun runtime graph 开关 / `.rrd` 录制导出入口
  - `crates/ralph-tui`
    - 如果后续要把 live graph 接进 TUI,需要定义边界
  - docs / doctor / debugging guidance
- 受影响行为:
  - 用户可以区分:
    - 静态 topology 图
    - live runtime graph
    - replay graph
  - 并行场景排障时,不再只靠 JSONL 和 stdout 猜测关系
- 风险与注意事项:
  - Rerun 的 graph 是 force-based layout,适合动态图,但布局本身不是协议真相源
  - 当前 durable 事件并不完整记录所有 `target_instance` / fanout recipients
  - 如果不明确 V1 / V2 边界,很容易把“先做一个 live demo”误当成“已经具备完整 replay graph”

## V1 / V2 Roadmap

### V1: Live Runtime Graph

- 目标:
  - 在 run 进行中可视化:
    - instances 的创建/存在/状态
    - workflow 事件链
    - 已知的 reply 链
  - 优先使用当前已有的:
    - `output_observer`
    - `instance_state_observer`
    - `event_observer`
    - `.ralph/agents.json`
    - `.ralph/events.jsonl`
- 特点:
  - 起步快
  - 适合调试 live run
  - 允许存在部分投递边盲区

### V2: Durable Replay Graph

- 目标:
  - 让一次已结束的 run 可以被完整或近完整地重建成关系图
  - 把 “谁创建谁 / 谁投递给谁 / 谁回复谁 / 谁冻结谁 / 谁停止谁” 做成可审计证据
- 需要的增强:
  - delivery 级 durable 记录
  - 更完整的 target / recipients / creator / lifecycle 证据
  - 明确 replay 时序与图更新规则
- 特点:
  - 更完整
  - 更适合 post-mortem 和 CI artifact
  - 需要更强的观测设计,不适合直接塞进 V1
