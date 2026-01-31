# Spec: E2E — starting_event 未配置时的入口事件推测

> 状态：APPROVED（按你在 2026-01-31 的请求执行）
>
> 更新时间：2026-01-31

## 0. 背景与动机

在 multi-hat / parallel runtime 下：

- fresh run 的初始化握手事件固定为 `task.start`（控制面 handshake）
- `event_loop.starting_event` **不是** “第一个事件”，而是 “协调后 workflow entry 提示”
- 当 `event_loop.starting_event` **未设置** 时，入口事件应该由 `ralph#1`（LLM 协调者）基于目标与 hats 拓扑自行决定

该语义非常容易在实现/重构时被误读，导致：

- 入口事件选择不稳定（发出 orphan topic / 无订阅者 topic）
- 工作流无法启动或跑偏
- 后续改 prompt/路由逻辑时缺少端到端回归保护

因此需要一个 **端到端（E2E）** 场景来覆盖 “starting_event 未配置时的入口推测”。

## 1. 目标（Goals）

1. 新增一个 `ralph-e2e` 场景，验证：
   - `event_loop.starting_event` 未配置时，`ralph#1` 能从拓扑推测并发布正确的 workflow entry 事件
   - 该入口事件能触发预期的 hat 链路，并在可控事件上收敛到 `LOOP_COMPLETE`
2. 场景优先覆盖 **parallel runtime**：
   - 因为 parallel supervisor prompt 内会生成 “derived entry candidates”，模型选择空间更可控，更适合作为稳定性回归

## 2. 非目标（Non-goals）

- 不验证 “多个候选入口事件时，模型一定选我们主观认为的那个最佳候选”（这会非常 flaky）
  - 但允许通过“明确 workflow 约束”把选择变成可判定问题（例如：要求 Planner 必须先跑），从而做稳定回归
- 不覆盖顺序 runtime 的 starting_event 推测（可作为后续补充场景）
- 不要求该场景在所有后端都稳定（先限制 Codex，后续再扩展）

## 3. 核心需求（Requirements）

### Requirement: E2E 场景必须覆盖入口推测

该新增场景 MUST 满足：

1. 配置 `parallel.enabled=true`
2. `event_loop.starting_event` 为空（不配置）
3. hats 拓扑设计成：
   - derived entry candidates **退化为单元素**（例如只剩 `spec.start`）
   - `ralph#1` 在 `task.start` 后发布的第一个 workflow entry topic 可稳定断言
4. 断言必须至少包含：
   - `.ralph/events.jsonl` 中出现 `spec.start`（或该场景定义的唯一候选入口 topic）
   - workflow 后续事件出现（例如 `build.task` → `build.done`）
   - 最终检测到 `LOOP_COMPLETE`（或 exit code 0）

### Requirement: 变体场景必须覆盖“多入口候选下的可判定选择”

该变体场景 MUST 满足：

1. 仍然启用 `parallel.enabled=true`
2. `event_loop.starting_event` 仍为空（不配置）
3. hats 拓扑里存在至少两个 derived entry candidates（例如同时存在 `spec.start` 与 `docs.start`）
4. prompt 必须给出明确的 workflow 顺序约束（例如 “Planner 必须先跑，再跑 Builder”）
5. 断言必须至少包含：
   - `task.start` 后，`ralph#1` 发出的第一个 workflow entry event 必须是能触发 Planner 的入口（在示例拓扑中即 `spec.start`）
   - `spec.start → build.task → build.done` 链路必须发生
   - 最终检测到 `LOOP_COMPLETE`

### Requirement: 验证门槛

该变更 MUST 通过：

- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test -p ralph-e2e`

## 4. 实现建议（Implementation Notes）

- 新 scenario 放在 `crates/ralph-e2e/src/scenarios/parallel.rs`（Tier 8），并注册到：
  - `crates/ralph-e2e/src/scenarios/mod.rs`
  - `crates/ralph-e2e/src/lib.rs`（re-export）
  - `crates/ralph-e2e/src/main.rs`（get_all_scenarios）
- 通过 `event_loop.complete_publishes` 固化收敛条件（例如 `build.done`），让 `ralph#1` 在观察到该事件时输出 `LOOP_COMPLETE`。

## 5. 可选项：mock-mode cassette

如果本机具备后端认证并可运行 live E2E：

- SHOULD 录制 cassette 到 `cassettes/e2e/`（例如 `parallel-starting-event-inference-codex.jsonl`）
- SHOULD 为变体场景也录制 cassette（例如 `parallel-starting-event-inference-multi-candidate-codex.jsonl`）
- 使得 `cargo run -p ralph-e2e -- --mock --filter parallel-starting-event-inference` 能做零成本回归

如果无法录制（缺少认证/CLI 不可用）：

- MUST 在 `notes.md` / `WORKLOG.md` 记录原因与后续补录方式
