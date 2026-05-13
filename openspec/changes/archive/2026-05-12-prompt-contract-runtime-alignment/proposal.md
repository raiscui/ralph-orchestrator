## Why

`docs/prompt-contract.md` 已经定义了 agent 最终交付的 output contract: outcome、evidence、changed files、known gaps、next suggestions。但当前 `InstructionBuilder::build_custom_hat` 的 REPORT 阶段只要求发布带 evidence 的 result event,没有把这些字段变成 runtime prompt 的可测试锚点。

这会造成文档和运行时提示漂移: prompt author 看到文档要求完整输出契约,但实际 hat prompt 只强化了 evidence。现在需要把文档契约落到 `InstructionBuilder` 和 hat prompt tests 里。

## What Changes

- 在 `InstructionBuilder::build_custom_hat` 生成的 REPORT 阶段加入明确 output contract。
- 对 custom hat prompt tests 增加断言,确认 prompt 包含 outcome、evidence、changed files、known gaps、next suggestions。
- 对 `EventLoop::build_prompt` 的 hat prompt 集成测试增加同样的契约锚点,证明 runtime 路径实际使用了对齐后的 `InstructionBuilder`。
- 视真实代码需要,补充 Ralph solo/coordinator prompt 的 completion/output 锚点,但不改变 runtime state、event routing、question obligation 或 team/tmux 逻辑。
- 同步 `docs/prompt-contract.md` 的措辞,说明这些字段也是 runtime prompt 测试锚点。

## Capabilities

### New Capabilities

- `prompt-contract-runtime-alignment`: 将 prompt contract 文档中的 output contract 对齐到 runtime prompt builder 和 hat prompt tests。

### Modified Capabilities

- None.

## Impact

- 受影响区域:
  - `docs/prompt-contract.md`: 补充 runtime prompt alignment 说明。
  - `crates/ralph-core/src/instructions.rs`: 强化 custom hat REPORT 阶段。
  - `crates/ralph-core/src/event_loop/tests.rs`: 增加集成 prompt 断言。
  - 可能涉及 `crates/ralph-core/src/hatless_ralph.rs`: 仅在需要补 coordinator/solo prompt output anchors 时修改。
  - `agent-guidance-manifest.toml`: 登记本 OpenSpec change。
- 不做的事情:
  - 不实现 state operation layer。
  - 不实现 question obligation runtime state。
  - 不改 team/tmux runtime。
  - 不改事件协议或完成条件解析。
