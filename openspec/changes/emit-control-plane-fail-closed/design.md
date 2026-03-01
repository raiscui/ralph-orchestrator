## Context

当前 parallel 模式下,外部事件有两条输入路径:

1. hats/job 进程里执行工具 `ralph emit ...`,向外部事件文件(JSONL)追加一行.
2. ExternalInput(人类/TUI/脚本)同样可以向外部事件文件(JSONL)追加一行.

Supervisor 会持续轮询该 JSONL,把每行映射为 `ralph_proto::Event` 并路由到目标 hat/instance.
其中 `turn_action=steer|interrupt` 会触发 in-flight 运行时控制,这和普通 data-plane topic 的风险等级不同.

在“偏无人值守 + 少量 human 干预”的运行环境中,我们更关心:

- 任何模型误触发都不应打断/劫持运行时(尤其是 coordinator `ralph#1`).
- hats 必须能互相通信,但通信应该是 data-plane,而不是 in-flight 控制面信号.

本 change 的核心是把这条边界做成 fail-closed,并让错误能被发起方(尤其是 hat)立刻理解与自纠.

当前相关实现点(用于定位实现落点,不要求逐行对齐):

- `ralph emit` CLI 参数与写入 JSONL:
  - `crates/ralph-cli/src/main.rs`
- hat job 环境变量注入(用于识别“这是 hat 内部执行”):
  - `crates/ralph-cli/src/parallel_runner.rs` 注入 `RALPH_HAT_INSTANCE_ID` / `RALPH_HAT_ID`
- Supervisor 外部事件读取与 turn_action 映射:
  - `crates/ralph-core/src/parallel/supervisor.rs`
  - `crates/ralph-core/src/event_reader.rs`
- turn_action 的实例侧语义(steer/interrupt 的执行与降级):
  - `crates/ralph-core/src/parallel/instance.rs`
- TUI 直接写 JSONL(不走 `ralph emit` 子进程):
  - `crates/ralph-tui/src/external_event_writer.rs`

## Goals / Non-Goals

**Goals:**

- 明确并固化 data-plane vs control-plane 边界:
  - data-plane: hats 之间与 hat->ralph 的业务沟通,只用普通 `topic/payload`。
  - control-plane: `turn_action=steer|interrupt`,仅允许 ExternalInput 对 `ralph#1` 使用。
- 对所有“可能产生 control-plane 效果”的入口做 fail-closed:
  - 在 hat job 环境里,禁止 `ralph emit --turn-action steer|interrupt`。
  - 任意来源一旦携带 `turn_action=steer|interrupt`,必须显式且仅能 `target_instance=ralph#1`。
- 错误反馈要可行动:
  - hat 误用时,工具调用应直接失败并输出“怎么改”的信息。
  - Supervisor 侧拒绝时,至少要有可观测日志,并建议对 `ralph#1` 给出可见告警。
- 防御纵深(defense-in-depth):
  - CLI 层拦截是“最快反馈”。
  - Supervisor 层拦截是“最终裁判”,覆盖 TUI/手工 JSONL 注入。

**Non-Goals:**

- 不做 guard token/签名这类“强认证”(属于后续 4.1/4.3 增强)。
- 不支持对非 `ralph#1` 的 worker hats 做 in-flight steer/interrupt。
- 不改变 `TurnAction::Steer|Interrupt` 在实例侧的执行语义(保持现有降级与取消逻辑)。
- 不引入新的消息协议来替代 hats 间普通 emit(仅收敛边界,不发明平台)。

## Decisions

### D1: 主要拦截点放在 `ralph emit` CLI(面向 hat 的最快自纠)

**Decision**

- 当检测到运行于 hat job 环境(存在 `RALPH_HAT_INSTANCE_ID`)时:
  - `ralph emit --turn-action steer|interrupt` 必须硬拒绝,并返回错误(非 warning)。

**Rationale**

- 这是“最接近误触发源头”的位置,可以把误触发变成一次失败的 tool call。
- 对 hat 来说,它能从 stderr/stdout 立刻学会“不要这样用”,并改成 data-plane 事件.

**Alternatives considered**

- 仅 Supervisor 拦截:
  - 能挡住风险,但 hat 看不到“自己哪里错了”,会产生“为什么无回应”的黑盒体验.
- guard token:
  - 更强但工程量更大,需要跨 `<event>`/JSONL/TUI/CLI 多处改动,不符合 4.2 的高 ROI 目标.

### D2: `turn_action=steer|interrupt` 必须显式且仅能投递到 `ralph#1`

**Decision**

- 只要 `--turn-action` 为 `steer|interrupt`,就强制要求:
  - `--target-instance ralph#1`
  - 且禁止使用 `--target <hat>` 这类“触发式路由”(必须直达 instance)

**Rationale**

- control-plane 信号一旦被“默认路由”或“模糊投递”,最容易发生误投递与越权.
- 把“必须显式指向 `ralph#1`”写死,能极大降低 operator/脚本误用的概率.

**Trade-off**

- 牺牲了“人类中途 steer 某个 worker”的能力.
  - 我们接受这个 trade-off,因为本运行环境更需要稳定与可预期.

### D3: Supervisor 对外部事件做最终 fail-closed 校验(覆盖 TUI/手工 JSONL)

**Decision**

- Supervisor 在把 JSONL 行映射为 `ralph_proto::Event` 前,对 `turn_action=steer|interrupt` 做校验:
  - 缺失 `target_instance` 或 `target_instance != ralph#1` 时,拒绝该事件(不路由)。
  - 并输出可观测 warning(日志)。
  - 推荐同时向 `ralph#1` 发布一条“被拒绝的 control-plane 注入”告警事件,避免无人值守时静默吞掉.

**Rationale**

- TUI 写 JSONL 不经过 `ralph emit`,仅靠 CLI 拦截是不完整的.
- “最终裁判”必须在 Supervisor,否则只要有人手工追加 JSONL 就能绕过约束.

**Alternative**

- 拒绝时把 `turn_action` 清空,降级为普通 data-plane 消息.
  - 这会把“控制面误用”伪装成“正常消息”,不符合 fail-closed 的目标.

### D4: TUI `!steer/!interrupt` 只允许作用于 `ralph#1`(本地预检)

**Decision**

- TUI 在解析 `!steer`/`!interrupt` 时,如果目标实例不是 `ralph#1`,应当在本地直接报错并不写入 JSONL.

**Rationale**

- 把错误尽可能前移到交互层,减少“写入了但被 Supervisor 拒绝”的排障成本.

### D5: Hat-to-hat 子任务通信采用 request/result,且只回传最终结论(不在中途 reply)

**Decision**

- 当 A hat 通过 data-plane(普通 `ralph emit topic=...`)触发 B hat 的一个子任务时:
  - B hat MUST NOT 在 job/turn 进行中用 `ralph emit` 向 A hat 回传“中间进度/半成品结论”。
  - B hat MUST 在自己的 job/turn 完成时,仅回传一次最终结论(例如 `subtask.result`),让 A hat 在下一轮 job/turn 中消费并推进。

**Rationale**

- 避免 A 被中途半成品驱动继续推进,导致“先推进后修正”的反复与不稳定.
- 在并行运行时,目标实例即使处于 Running,Supervisor 也会把 data-plane 事件入队,稳定地在下一轮消费.
- 该约束不依赖 control-plane(不需要 `turn_action=steer|interrupt`),因此不会引入额外安全面.

**Alternatives considered**

- B 在中途持续回传进度:
  - 对人类交互友好,但对无人值守的 code agent 容易造成上游误判与流程漂移.
- 用 `turn_action=steer` 强行“插话”到 A 的 in-flight:
  - 这是 control-plane,风险等级更高,且本 change 目标是把这类信号做 fail-closed 收敛.

## Risks / Trade-offs

- [风险] hat 仍可能通过“手工 unset 环境变量”绕过 CLI 侧拦截.
  - Mitigation:
    - 该 change 的 threat model 以“误触发/非对抗”优先.
    - 后续可用 guard token 或 source attribution 增强(见 Open Questions).
- [风险] 禁止 steer/interrupt 作用于 worker,降低了人类实时纠偏能力.
  - Mitigation:
    - 用 data-plane 普通消息通知 worker.
    - 或对 `ralph#1` steer,让 coordinator 通过新任务派发来改变 worker 行为.
- [风险] 既有脚本可能依赖“turn_action 但未显式 target_instance”的写法.
  - Mitigation:
    - CLI 给出明确错误与可复制的修正命令.
    - Supervisor 拒绝时产生日志/告警,避免“看起来无回应”.

## Migration Plan

1. 实现 CLI fail-closed 校验(优先):
   - hat 环境禁止 `--turn-action steer|interrupt`
   - `--turn-action steer|interrupt` 强制 `--target-instance ralph#1`
2. 实现 Supervisor 侧最终校验(防御纵深).
3. 收敛 TUI 命令:
   - `!steer/!interrupt` 仅允许 `ralph#1`
4. 更新文档:
   - `specs/parallel-event-channels.spec.md` 补充“谁可以使用 turn_action”的边界.
5. 补回归测试:
   - CLI: 环境变量存在时必须失败.
   - Core: Supervisor 拒绝无 `target_instance`/非 `ralph#1` 的 turn_action 行.

## Open Questions

- 是否要把 `turn_action=start` 也视为 control-plane 并在 hat 环境禁止?
  - 倾向: 不需要,start 本质是“默认语义”,且实例侧会清空该字段避免 prompt 污染.
- 是否要为 Supervisor 的拒绝引入稳定 topic(例如 `control.reject`)?
  - 倾向: 先复用已有 escalation/diagnostics 机制,避免新增协议面.
- 未来的 4.1/4.3 增强:
  - guard token(ExternalInput 才有)或 source attribution(写入 source_instance/source_kind),
    以便把“只有 ExternalInput 能 steer/interrupt”做成更强的可验证约束.
