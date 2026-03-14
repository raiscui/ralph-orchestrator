## Why

在 parallel 模式下,hats 可以通过工具执行 `ralph emit` 注入外部事件,但 `turn_action=steer|interrupt` 会影响 in-flight turn/job。
在无人值守为主的运行环境里,任何一次模型误触发都可能打断 coordinator 或劫持流程。
因此需要把 control-plane 信号做成默认拒绝(fail-closed),并把 data-plane 与 control-plane 边界写死。

## What Changes

- 定义外部事件的两条平面:
  - data-plane: 仅 `topic/payload(+target/target_instance/...)`,用于 hats 之间或 hat->ralph 的业务沟通。
  - control-plane: `turn_action=steer|interrupt`,仅用于 ExternalInput 对 `ralph#1` 的运行时控制。
- **BREAKING**: `ralph emit` 在检测到运行于 hat job 环境(`RALPH_HAT_INSTANCE_ID` 存在)时,必须拒绝 `--turn-action steer|interrupt`,并输出可行动的错误信息(提示改用普通 emit topic 或删除 turn_action)。
- **BREAKING**: 当 `--turn-action steer|interrupt` 被使用时,必须显式 `--target-instance ralph#1`。
  - 任何缺失 `target_instance` 的用法必须被拒绝(避免控制信号被触发式路由/误投递)。
  - 任何 `target_instance != ralph#1` 的用法必须被拒绝(避免控制面越权与跨实例打断)。
- 对拒绝场景采用 fail-closed:
  - 不做隐式降级(例如把 steer 当普通排队消息)。
  - 只返回明确错误,让发起方(hat 或 operator)可自纠。

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `parallel-hat-instances`: 新增 control-plane 信号边界;将 `turn_action=steer|interrupt` 定义为仅 ExternalInput 可触发,且必须 fail-closed 拒绝 hat 侧误用。
- `parallel-trigger-routing`: 对带 `turn_action` 的外部事件,要求显式 `target_instance` 并拒绝任何默认路由/无目标投递(防止误投递与越权)。

## Impact

- CLI:
  - `crates/ralph-cli/src/main.rs`: `ralph emit` 参数校验与错误输出。
  - `crates/ralph-cli/src/parallel_runner.rs`: hat job 注入环境变量作为判定依据。
- Runtime:
  - `crates/ralph-core/src/parallel/supervisor.rs`: 外部事件读取与路由前校验。
  - `crates/ralph-core/src/parallel/instance.rs`: `turn_action` 执行语义保持不变,但输入面被收敛。
- Docs/Specs:
  - `specs/parallel-event-channels.spec.md`: 补充“谁可以使用 turn_action”的边界说明。
- Tests:
  - 增加回归测试覆盖: hat 环境下 `ralph emit --turn-action` 必须失败; 非 hat 环境 + `target_instance=ralph#1` 时允许。
