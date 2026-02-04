## Why

在真实项目里，很多任务天然需要“实验性开发”：不断尝试、不断验证、不断调整。
如果只能串行地一个人摸索，效率会非常低，也很容易因为缺少强约束的验证步骤而把问题越改越乱。

因此我们需要研究出一份**适合此类任务的 `ralph.yml` 配置方案**：
它应该在并行 hats 模式下稳定运行。
它要让“并行实现 + 批量验证 + 多轮实验探索”变成一个可复用、可回放、可收敛的工作流。

## What Changes

- 新增一份“并行实验开发永动机”的参考配置（`ralph.yml`），作为这类任务的默认起步方式：
  - 并行实现、并行验证、可多轮迭代探索。
  - 强制 backpressure：每轮必须产出可验证的结果（命令/测试/基准/对比）。
  - 自适应并行度：由 `ralph#1` 根据用户提供的计划/验证强度**自动推断并行上限**，并在运行中动态调参（激进起步 + AIMD 拥塞控制）。
  - 可可靠收敛：明确入口事件、完成候选事件、以及结束条件，避免卡死或漂移。
  - 明确职责拆分：
    - `experiment_runner`：只负责“实现 + 验证 + 产出 patch + 结构化证据”，不负责采纳/合并。
    - `experiment_auditor`：只负责“证据硬门槛审计”，证据不足就拒绝，阻断收敛。
    - `experiment_integrator`：负责“是否采纳 + 如何应用（apply patch / 合并）+ 主工作区最终验证”，并产出最终集成结果。
- 该参考配置将以可复制的方式落盘为一个 example 目录（便于直接运行/改造）：
  - `examples/parallel-experimental-dev-engine/ralph.yml`
  - `examples/parallel-experimental-dev-engine/README.md`
- 增加/补充相应 specs：把这份配置方案的**事件契约**（topics 与 payload 约定）、
  **隔离策略**（worktree/patch/shared）以及**验证/收敛语义**写成明确的 MUST/SHOULD 规则。
  - 引入独立的“结果审计（auditor）”角色：对每条 `experiment.result` 做硬门禁审计，证据不足则不允许收敛。
  - runner 的产物要求收敛到：**必须提供 `patch`**（用于独立审计与后续集成），`commit` 仅作为可选附加信息（便于保留提交历史）。

## Capabilities

### New Capabilities

- `parallel-experimental-dev-engine`: 提供一份可复用的并行 hats 工作流配置方案（`ralph.yml`），专门面向：
  - 并行实现与批量验证
  - 多轮实验性开发（探索/试错/再验证）
  - 强 backpressure（每轮必须有验证证据）
  - 可收敛的结束语义（避免无尽循环或卡死）

### Modified Capabilities

（无）

## Impact

- 受影响代码/文档区域：
  - `examples/parallel-experimental-dev-engine/`：新增可直接使用/复制的并行实验开发配置（`ralph.yml`）与说明文档。
  - `openspec/specs/`：新增上述 capability 的 spec 文件，并引用既有并行基础能力（例如 `parallel-hat-instances` / `parallel-trigger-routing`）。
- 对外 API/行为影响：
  - 以新增能力与示例为主，不预期引入破坏性变更（无 **BREAKING**）。
- 测试与验证影响：
  - 需要新增 replay fixture / smoke test，用来验证该配置方案的事件序列与收敛行为是可回放且确定的。
