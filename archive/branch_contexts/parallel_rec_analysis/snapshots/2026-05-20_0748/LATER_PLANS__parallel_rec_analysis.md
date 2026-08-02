# 后续计划: parallel_rec / runtime capability

## [2026-05-18 00:11:00] [Session ID: omx-1779004640353-blcixq] 后续事项: child workflow 真执行与递归保护

### 背景
- 本轮只修复 `workflow:default-parallel` capability 的 `resolved-config.yml` materialization。
- `run_child_dry_run()` 仍然只执行 `ralph run --dry-run --no-tui --prompt ...`。

### 后续建议
- 设计并实现 workflow capability 的 execute 模式,明确什么时候从 dry-run 切换到真实 child run。
- 为 child run 增加 runtime capability re-entry guard,避免 child 里的 `ralph#1` 再次触发相同 capability 导致递归。
- 补 record-session evidence gate,证明 child workflow 真正发布了预期事件,并且 parent 仍只接收结构化 `capability.result` / `capability.failed`。

### 完成判据
- 有明确配置或 CLI flag 区分 dry-run / execute。
- execute 模式下 `workflow:default-parallel` 的 child run 能真实启动三个 hats 或按配置路由事件。
- parent topology 仍不被热改。
- record-session 中能看到 child invocation 的 termination/result evidence。
## [2026-05-18 10:34:11] [Session ID: omx-1779004640353-blcixq] 后续事项: 拆分 Ralph coordinator prompt 与 child hat worker prompt

### 背景
- `parallel_rec.jsonl` 显示简单问题也会被大量 coordinator / 文件上下文 / memory / evidence 规则拖长。
- 用户明确提出: Ralph 应该只决定任务如何分发、安排和分配,不应该真正解决问题。

### 建议方向
- 设计 `coordinator_prompt` 只给 `ralph#1`:
  - 判断任务类型。
  - 选择分发策略。
  - 发出 workflow / capability / hat task event。
  - 收集结果并合并回复。
- 设计 `worker_prompt` 只给非 Ralph hat:
  - 明确角色目标和输入输出契约。
  - 不携带完整 Ralph 调度职责。
  - 不做任务再分发,除非 role contract 显式允许。
- 抽出极小 `shared_protocol_prompt`:
  - event envelope。
  - reply 属性。
  - 当前角色允许 publish 的 topics。
  - 必要 completion / stop 语义。

### 完成判据
- Ralph 的首轮行为默认是分发 event,不是直接解决业务问题。
- 非 Ralph hat 的 prompt 不包含 runtime capability catalog / coordinator topology / task distribution policy。
- 有测试断言 Ralph prompt 与 worker prompt 的差异,防止未来重新混在一起。
## [2026-05-18 10:39:32] [Session ID: omx-1779004640353-blcixq] 后续事项: 支持 task-derived dynamic hat 身份,但隔离 coordinator prompt

### 背景
- 用户明确补充: Ralph 创建 hat 实例时,来源不应只有项目模板或 `ralph.yml` 静态 hats,还应允许根据任务性质实时生成 hat 身份角色。

### 建议方向
- 在调度层建立 hat 身份来源优先级/选择策略:
  1. 优先复用 `ralph.yml` 中明确配置的 hat,因为它有稳定契约。
  2. 若项目模板/preset 有合适角色,从模板派生。
  3. 若两者都不合适,由 Ralph 根据当前任务性质生成 task-derived dynamic hat。
- task-derived dynamic hat 只携带最小 worker contract:
  - role name。
  - objective。
  - input contract。
  - output contract。
  - allowed publish topics。
  - stop/completion rule。
- 禁止动态 hat 继承完整 Ralph coordinator prompt:
  - 不注入 runtime capability catalog。
  - 不注入全局任务分发策略。
  - 不注入文件上下文治理职责,除非该 hat 的 role contract 明确需要。

### 完成判据
- 测试能断言 `ralph#1` prompt 与动态 worker prompt 不同。
- 动态 worker prompt 中不存在 coordinator-only 段落。
- Ralph 能在 record/evidence 中说明该 hat 的来源是 config/template/task-derived。

## [2026-05-18 14:12:00] [Session ID: omx-1779004640353-blcixq] 后续事项状态更新: prompt role layering 已落地,但 child workflow execute 仍保留

### 已落地
- Ralph/worker prompt surface 分层已落地。
- `all_hat_prompt` 已纳入 shared-only 审计。
- worker prompt 已有 coordinator-only 排除回归测试。
- `hat:focused-reviewer` micro-run artifact 已记录 `role_contract.identity_source = task-derived`。
- runtime autoscale instance 已在 agents snapshot 中记录 `identity_source = runtime-autoscale`。
- `coordinator.no_event_first_turn` durable diagnostic 已落地。

### 仍需保留的后续工程
- `workflow:default-parallel` 从 dry-run 切换到真正 execute child run 的完整设计与递归保护仍未实施。
- 更完整的 “Ralph 根据任务实时生成新 worker hat 并纳入 live topology” 仍未实施;本轮只把 micro-run provenance 和 prompt 隔离证据先落地。

### 完成判据保持
- child workflow execute 需要 record-session 证明 child run 真执行并终止。
- live task-derived dynamic hat 需要证明 parent topology 变化受控,且 worker 不继承 coordinator prompt。

## [2026-05-18 17:24:00] [Session ID: omx-1779004640353-blcixq] 后续事项状态更新: `hat:*` execute 已落地,workflow dogfood 仍可保留

### 本次已落地
- `hat:*` capability 默认 execute 已落地。
- `hat:*` execute 不再嵌套 Ralph coordinator loop,改为直接调用底层 backend。
- `--preview` 保留旧 dry-run inspect/debug 行为。
- live parent capability invocation 相关集成测试已通过。

### 仍建议后续继续
- 对 `workflow:*` capability 做一次 record-session dogfood,证明真实 workflow child run 在非 preview 模式下能完整终止并产生 parent 可消费结果。
- 继续设计更完整的 task-derived dynamic hat live topology,而不只是 capability micro-run。
- 若要进一步优化产品体验,可以把 direct backend 的 stream-json / JSON 输出整理为更干净的 `result_summary`,目前先保持原始 backend 输出摘要。


## [2026-05-18 17:39:46] [Session ID: omx-1779004640353-blcixq] 后续事项状态更新: `workflow:*` record-session dogfood 已完成

### 已完成
- `workflow:default-parallel` 非 preview execute 已通过真实 CLI dogfood。
- invocation artifact 现在包含 `child-record-session.jsonl`。
- evidence index 现在登记 `record_session_jsonl`。
- default workflow 现在通过 `workflow.complete` completion candidate 回到 `ralph#1` 收敛。

### 仍可后续优化
- result summary 目前仍会截取 child stdout 前 500 字符,可能包含较多 prompt echo。若后续要提升 UX,可以专门整理 workflow child 的 concise summary。

## [2026-05-18 19:08:00] [Session ID: omx-1779004640353-blcixq] 后续事项: 收敛 `spawn_instance` 协议提示与 task-derived dynamic hat 产品契约

### 背景
- 当前 `spawn_instance` 是 `Option<bool>`。
- 合法显式 spawn 需要 `spawn_instance="true"` 且同时给 `target="<hat_id>"`。
- 但 runtime prompt 只列出 `spawn_instance` 为 supported attribute,没有直接说明它不是数量、不是实例列表。
- 本轮 child `ralph#1` 猜出了 `spawn_instance="3"` 和 `spawn_instance="builder#1,builder#2,builder#3"`,导致用户期望的三实例任务没有立即 materialize。

### 建议方向
- 短期: 强化 event emission protocol 文案和测试,明确 `spawn_instance` 的合法形态。
- 中期: 增加 invalid `spawn_instance` diagnostic,让错误直接进入 record/evidence,不要只表现为“没新实例”。
- 长期: 设计 task-derived dynamic hat creation 的正式协议,让 `ralph#1` 可以基于任务生成 role contract 和 runtime lifecycle evidence。

### 完成判据
- 模型不再生成 `spawn_instance="3"` / 实例列表这种无效属性值。
- 若事件属性无效,record-session 和 `.ralph/events.jsonl` 有明确 diagnostic。
- 若用户要求实时创建三类视角 hat,agents snapshot 能显示 task-derived role contract,而不只是 config-derived builder 实例。


## [2026-05-19 09:11:12] [Session ID: omx-1779004640353-blcixq] 后续事项: 收敛 task-derived 三实例创建与 audience override 诊断

### 背景
- 当前 `workflow:default-parallel` capability 是 isolated child run,父 topology 不变。
- child 输出 `audience_instances="builder#功能补充,builder#功能完善,builder#review"`,但 child config 只有 `builder#1`,且无 `topic_contracts`。
- trigger fallback 路由最终把任务投递到已有的 `builder#1`,没有创建三个 dynamic instances。

### 建议方向
- 短期: 在 event emission protocol 中明确 `audience_instances` 不是创建实例,`spawn_instance` 只能是 boolean。
- 短期: 对“无 TopicContract 但带 audience_override/require_delivery”的 fallback 路径增加可见 diagnostic,避免用户误以为 TUI 漏显示。
- 中期: 设计正式的 task-derived dynamic hat/instance 创建协议,支持一次请求中声明多个 role contract,并在 `.ralph/agents.json` 与 `runtime.lifecycle` 中可见。
- 中期: 或者给 `workflow:*` child run 加 parent-visible pseudo status,明确这是 isolated child,不是 parent topology mutation。

### 完成判据
- 用户要求 3 个实例时,record/evidence 中必须能直接看出是以下哪一种:
  - parent-visible dynamic HatInstance 已创建。
  - isolated child run 正在执行,父 topology 不会变化。
  - 请求属性无效或无法物化,已产生 diagnostic。

## [2026-05-19 12:43:00] [Session ID: omx-1779158263949-kticiv] 后续事项状态更新: parent-visible spawn 与 child-run projection 已落地,task-derived hat 仍可继续深化

### 已落地
- `topology.spawn_group` 已支持在父级 runtime 创建真实动态 HatInstance。
- TUI / `ralph agents` 已支持 isolated child-run 的 parent-observable projection。
- coordinator prompt 已明确 parent-visible group spawn 与 isolated capability 的选择边界。

### 仍可后续深化
- 当前 `topology.spawn_group` 的 `role` 是临时运行时标签,目标 hat 仍来自已有配置,例如 `builder`。
- 真正的 task-derived dynamic hat identity / role contract 仍可作为后续工程继续设计。
- 如果要让 LLM 运行时创建全新 hat 类型,还需要补 role contract schema、prompt isolation、agents snapshot provenance 和 E2E dogfood。

## [2026-05-20 00:22:10] [Session ID: omx-1779158263949-kticiv] 后续事项: dogfood worker 收敛与支线上下文续档

### 事项1: live dogfood worker 收敛仍需单独优化

- 本轮 `topology.spawn.result` 重复派发已修复并通过动态验证。
- 但 no-TUI dogfood 仍以 `MaxRuntime` 结束,部分 analyst worker 没有稳定产出 `analysis.done`。
- 后续如继续强化 E2E,应单独分析 worker prompt、gate timeout、read-only tool noise 和失败状态回写,不要混进 topology spawn guardrail 修复。

### 事项2: 支线 notes 文件已超过 1000 行

- `notes__parallel_rec_analysis.md` 当前已超过 1000 行。
- 按六文件上下文规则,后续应执行 continuous-learning 提炼后再续档/归档。
- 本轮先完成正在进行的 prompt guardrail 修复,避免把上下文归档改动混入 bug fix diff。
