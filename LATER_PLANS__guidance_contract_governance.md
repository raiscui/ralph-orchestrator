
## [2026-05-11 17:42:01] [Session ID: omx-1778475786175-ogndry] 后续计划: guidance governance 第二阶段

- 下一阶段建议从 `agent-guidance-manifest.toml` 扩展到更完整的 agent asset catalog,但不要一次性纳入所有 docs。
- 优先候选:
  1. 为 `.agents/skills` / `.codex/skills` 中项目自有 skill 增加 manifest 条目。
  2. 将 verifier 扩展为可由 CLI 或脚本单独调用,例如未来的 `cargo xtask verify-agent-assets`。
  3. 评估是否把 `docs/prompt-contract.md` 的 output contract 和现有 `InstructionBuilder` / hat prompt tests 对齐。
  4. 进入 oh-my-codex 建议中的 state operation layer,但必须另开 OpenSpec change,不要混入本次 governance change。
- 当前不做原因:
  - 本次目标是最小闭环,已通过 docs + manifest + verifier + cargo test 形成第一道门禁。
  - 直接继续扩大到 runtime state/question obligation/team pipeline 会越过本次 change 的非目标边界。

## [2026-05-11 19:14:12] [Session ID: omx-1778475786175-ogndry] 后续计划更新: 第二阶段已完成部分

- 已完成:
  - 为 `.agents/skills` / `.codex/skills` 中项目自有 skill 增加 manifest 条目。
  - 将 verifier 扩展为可由 CLI 单独调用: `ralph verify agent-guidance`。
- 仍建议另开 change 继续:
  1. 评估是否把 `docs/prompt-contract.md` 的 output contract 和现有 `InstructionBuilder` / hat prompt tests 对齐。
  2. 进入 oh-my-codex 建议中的 state operation layer,但必须另开 OpenSpec change,不要混入 guidance catalog/CLI change。

## [2026-05-11 20:49:30] [Session ID: omx-1778475786175-ogndry] 后续计划更新: prompt contract 对齐已完成

- 已完成:
  - `docs/prompt-contract.md` 的 output contract 已和 `InstructionBuilder` / hat prompt tests 对齐。
  - `prompt-contract-runtime-alignment` 已作为独立 OpenSpec change 落地,没有混入 state operation layer。
- 仍建议另开 change 继续:
  1. 进入 oh-my-codex 借鉴建议中的 state operation layer。
  2. 该 change 应单独定义 state operation 的边界、读写接口、验证门禁和回滚策略。
  3. 不要把它塞回 guidance catalog/CLI 或 prompt contract alignment change。

## [2026-05-11 22:54:00] [Session ID: omx-1778475786175-ogndry] 后续计划更新: state operation layer 已进入 OpenSpec

- 已完成:
  - `state-operation-layer` 已作为独立 OpenSpec change 创建。
  - proposal、design、spec、tasks 已落盘并通过校验。
  - 该 change 明确不混入 guidance catalog/CLI 或 prompt contract alignment。
- 仍建议后续实施:
  1. 先在 `ralph-core` 实现 state operation 数据结构、mode 校验、路径解析、原子写和 per-path 写队列。
  2. 补齐 core 单测后,再决定是否新增 `ralph state ...` CLI。
  3. question obligation runtime state 和 runtime capability invocation state 应在 state operation layer 实现后,各自作为独立接入 change 推进。

## [2026-05-12 09:23:00] [Session ID: omx-1778475786175-ogndry] 后续计划更新: state operation adapter 保持独立

- 已完成:
  - `ralph-core` state operation core API 已实现并通过 focused tests。
- 本轮仍不做:
  1. `ralph state ...` CLI adapter。
  2. MCP state tools。
  3. question obligation runtime state 接入。
  4. runtime capability invocation state 接入。
- 后续建议:
  - 如果继续推进,先开独立 change 做 `ralph state` CLI,只调用 `ralph-core` 的 `StateOperationStore`,不要在 CLI 里直接读写 JSON。

## [2026-05-12 09:35:00] [Session ID: omx-1778475786175-ogndry] 后续计划更新: state operation core 已完成

- 已完成:
  - `ralph-core` state operation core API 已实现。
  - OpenSpec `state-operation-layer` tasks 已全部勾选。
- 仍建议另开 change:
  1. `ralph state ...` CLI adapter。
  2. MCP state tools adapter。
  3. deep-interview question obligation runtime state 接入。
  4. runtime capability invocation state 接入。
- 关键边界:
  - 上述 adapter 必须复用 `StateOperationStore`。
  - 不要在 adapter 中重新实现 JSON 路径、merge、clear 或 atomic write。

## [2026-05-12 10:34:00] [Session ID: omx-1778475786175-ogndry] 后续计划更新: state CLI adapter 已完成

- 已完成:
  - `ralph state status`。
  - `ralph state read <mode>`。
  - `ralph state clear <mode>`。
  - OpenSpec change `ralph-state-cli-adapter` 已落地并通过验证。
- 仍建议后续单独推进:
  1. MCP state tools adapter。
  2. deep-interview question obligation runtime state 接入。
  3. runtime capability invocation state 接入。
  4. team/runtime lifecycle 自动写入 state operation layer。
- 关键边界:
  - 后续 adapter 必须复用 `StateOperationStore`。
  - 不要在 adapter 中重新实现 JSON path、merge、clear 或 atomic write。
