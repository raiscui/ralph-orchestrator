## Context

并行模式下,hat 之间通过 `<event ...>...</event>` 协作.
目前系统已经具备 `Event.id` 字段,并且在并行运行时会为缺失 id 的事件补齐:

- instance 侧: `"{instance_id}:{seq}"`
- supervisor 侧: `"supervisor:{seq}"`

但一个关键断点是: hat 收到的 incoming events prompt 目前只展示 `topic + payload`,不展示 `id`.
因此即便运行时内部已经有 `Event.id`,hat 仍然无法在输出事件里写出 `reply="<id>"` 来形成可关联的回复链路.

本 change 的设计目标是把"可引用 id"与"reply(单值)"做成协议一等字段,并把 id 暴露到 hat 的输入 prompt 中,从而提升并发协作的可解释性与确定性.

## Goals / Non-Goals

**Goals:**

- 并行模式下,每条被发布/路由的 event 都必须拥有可引用 id(复用 `Event.id`).
- 支持 event 的单值回复关系:
  - `<event ... reply=\"<event_id>\">...</event>` 可被解析到协议字段.
- incoming events prompt 必须展示 event id,使 hat 能把 id 原样带入 reply.
- 保持 replay/诊断友好:
  - id 的补齐策略必须是确定性的(不依赖随机数),避免回放不一致.
- 变更尽量收敛在协议层 + parser + prompt 注入层,避免引入新概念或新的 routing 机制.

**Non-Goals:**

- 不在本 change 中接入 Codex App Server 的 `turn/steer`/`turn/interrupt`.
- 不把 reply 升级成完整的“线程/对话树”系统(例如多父依赖、自动聚合、强一致校验).
- 不强制所有 agent 必须显式输出 `id="..."`(运行时会补齐).

## Decisions

### 1) "nanoid" 口径: 复用 `Event.id` 作为唯一主键

**选择**: 不新增 `Event.nanoid`.
把"nanoid"定义为"每条 event 的可引用 id",对应 `Event.id`.

**理由**:

- 代码现状已经存在 `Event.id`,且并行运行时已在补齐.
- 再新增一个 `nanoid` 会产生双主键,容易引入不一致与歧义.

**替代方案**:

- 新增字段 `Event.nanoid` 并逐步迁移:
  - 缺点: 需要长期维护兼容映射,并且需要决定优先级与序列化策略,收益不高.

### 2) id 补齐策略: 使用确定性的序列号,而不是随机 nanoid

**选择**:

- 保持并行 instance 侧的 `"{instance_id}:{seq}"`.
- 保持 supervisor 侧的 `"supervisor:{seq}"`.
- 如果需要覆盖串行/其他路径,也采用相同思路: `"{hat_id}:{seq}"` 或 `"event_loop:{seq}"`.

**理由**:

- replay/fixture 需要可复现.
- 该 id 的用途是“本次 run 内可引用锚点”,不要求跨 run 全局唯一.

**替代方案**:

- 引入真正随机的 nanoid:
  - 缺点: fixture 与回放会失真,debug 也更难复现.

### 3) reply 字段: 单值 `Event.reply: Option<String>`

**选择**: reply 语义为"回复某一个 event.id"(单值).

**理由**:

- 你已确认 reply 是单值.
- 单值语义更易被 LLM 遵守,也更易被 TUI/日志展示与检索.

**替代方案**:

- `reply_ids: Vec<String>`:
  - 缺点: prompt 与输出格式更复杂,也更容易被 agent 写错.

### 4) incoming events 的 prompt 展示: 用纯文本字段,避免注入 `<event ...>` 原文

**选择**: 在 prompt 里展示形如:

- `- id=<id> topic=<topic> payload=<payload>`

而不是直接展示 `<event ...>`.

**理由**:

- 现有 `EventParser` 以 `<event ` 作为扫描起点.
  如果 prompt 中出现 `<event ...>`,模型复述时容易制造“假事件”,导致误解析.
- 纯文本字段足够让 hat 读到 id,同时不会引入 parser 误判风险.

### 5) reply 的强校验策略: 不做强一致校验,仅做关联记录

**选择**:

- 如果 reply 指向一个未知/未见过的 id,系统仍然允许该事件发布与落盘.
- 关联的正确性由 ralph/人类在工作流中判断,不引入硬 gate.

**理由**:

- 并行场景下,跨实例、跨批次的事件到达顺序并不总是严格线性.
- 把 reply 做成软关联可以最大化可用性,并避免引入新的失败模式.

## Risks / Trade-offs

- [Risk] prompt 结构变化可能影响模型行为 → Mitigation: 保持展示格式简短一致,并只新增 `id=` 字段,不引入新标签语法.
- [Risk] 某些路径仍可能产生缺失 id 的 event → Mitigation: 在 event 发布入口统一补齐 id(并加测试覆盖).
- [Risk] reply 被误用导致链路混乱 → Mitigation: 在 `config/all_hat.md` 中写清"reply 必须引用 incoming event 的 id"的约定,并在示例中反复使用.

## Migration Plan

1. 协议层: `Event` 增加 `reply` 字段(可选,默认无).
2. 解析层: `EventParser` 解析 `<event ... reply=\"...\">`.
3. 运行时补齐:
   - 确保发布到 bus 的事件都拥有 id(缺失则补齐).
4. Prompt 注入:
   - 并行模式 incoming events 展示 `id`.
5. 文档与测试:
   - 更新 `config/all_hat.md` 与相关 specs.
   - 更新/新增单元测试与 smoke tests fixture(如有受影响).

回滚策略:

- reply 为可选字段,移除解析/展示不会影响主流程.
- id 补齐属于可观测增强,回滚时需同步回退相关测试断言.

## Open Questions

- 是否需要在 TUI 的事件列表/输出里显式展示 `id` 与 `reply`(便于交互式追踪)?
- 是否需要在 `.ralph/events*.jsonl` 的人类可读记录中增加 `id` 与 `reply` 字段(便于 grep)?
