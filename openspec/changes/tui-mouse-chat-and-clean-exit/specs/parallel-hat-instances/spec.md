## ADDED Requirements

### Requirement: Supervisor shutdown terminates all worker CLI processes
When the parallel supervisor is stopping (due to user quit, interrupt, or cancellation), the system MUST terminate all headless worker CLI processes started for HatJobs, and MUST ensure no orphan processes remain.

#### Scenario: User quits TUI while workers are running
- **WHEN** the user presses `q` in the Supervisor TUI while at least one HatJob process is still running
- **THEN** the runtime terminates those CLI processes (graceful first, then force-kill after a timeout)
- **THEN** the Ralph process exits without leaving orphan worker processes

#### Scenario: Supervisor shutdown does not leak processes
- **WHEN** the supervisor transitions to a terminal shutdown state
- **THEN** all child HatJob processes are terminated and reaped before the supervisor returns

---

### Requirement: 并行模式下所有 hats 默认订阅 human.message
When `parallel.enabled: true`, the system MUST ensure every configured hat subscribes to topic `human.message`, even if it is not explicitly listed in `hats.<id>.triggers`.

说明：
- 目的：保证 Supervisor 的 strict target 校验下，`human.message(target_instance=writer#2)` 这种“实例直达”不会因为“hat 未订阅该 topic”而被拒绝。
- 该规则只要求“订阅存在”，并不要求 `human.message` 必须 broadcast；事件是否 fanout 仍由 `target_instance` / contracts / triggers 决定。

#### Scenario: 并行模式自动补齐 human.message 订阅
- **GIVEN** 配置启用了 `parallel.enabled: true`
- **AND** 某个 hat（例如 `writer`）未显式配置 `triggers: ["human.message"]`
- **WHEN** 系统启动并行 Supervisor
- **THEN** `writer` 在运行时视为已订阅 `human.message`
