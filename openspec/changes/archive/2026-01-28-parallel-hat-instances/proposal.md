## Why

当前 Ralph 的多 hat 配置仍受 "Sequential hats / No parallel delegation / Single executor" 约束，实际执行者始终是内置 `ralph`，导致 reviewer/tester/decider 等无法真正并行运行。我们需要把“并行”做成一等能力，用可回放事件与 backpressure 作为硬门槛，在不牺牲可控性的前提下提升吞吐与协作体验。

## What Changes

- 支持多个 hat / 同一 hat 多实例并行执行（HatInstance）。
- 引入 Supervisor 层，用于调度实例、汇总输出与事件，并提供 human async loop。
- 明确定义事件投递语义（queue/fanout）与实例级受众限制，确保路由决策可落盘可回放。
- 增加 human gate（普通等待/可选超时）以保持人工可控；超时场景允许用决策型 hat 继续推进并落盘决策。
- 增强 workspace 隔离策略（共享/补丁/工作树等）与权限/能力声明，避免并行写冲突。
- **BREAKING**：多 hat 执行模型与相关配置/事件时序将从串行“假并行”升级为真并行，部分字段与行为会调整。

## Capabilities

### New Capabilities

- `parallel-hat-instances`: 并行 HatInstance + Supervisor + human async loop 的端到端行为规范（含并发执行、事件路由、gate、workspace 策略与可回放约束）。

### Modified Capabilities

<!-- 无 -->

## Impact

- crates：`ralph-core`（事件循环/调度/事件落盘）、`ralph-cli`（运行器）、`ralph-tui`（展示与交互）。
- 配置/行为：`ralph.yml`（hats/instances/workspace/gate 等）与事件日志格式/时序会受影响。
- 测试：需要更新/新增 replay-based smoke fixtures 覆盖并行场景，确保回放确定性与 backpressure 机制有效。
