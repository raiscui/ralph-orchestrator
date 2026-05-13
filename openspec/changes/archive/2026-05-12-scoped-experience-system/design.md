## Context

Ralph 当前的长期记忆实现仍然围绕单一 `.agent/memories.md` 展开:

- `MemoriesConfig` 只有 `enabled / inject / budget / filter`
- `prepend_memories()` 仍直接读取 `.agent/memories.md`
- 文档已经出现 `memories.path` 口径,但实现还没有真正支持

与此同时,用户希望 Ralph 进入一个更分层的知识体系:

- runtime work graph 继续由 `.agent/tasks.jsonl` 承担
- instance 级原始轨迹保存在 `.ralph/log/<instance_id>/...`
- topic 级共享结论保存在项目根 `task_plan__topic.md` / `notes__topic.md` / `WORKLOG__topic.md`
- role 级稳定经验保存在 `.ralph/roles/<hat_id>/experience.md`
- project 级稳定经验保存在项目根 `experience.md`

这个设计还必须和另外两条 change 保持边界清晰:

- `startup-resource-bootstrap`
  - 负责无 `PROMPT.md` / 无 `ralph.yml` 时的默认启动资源与 bootstrap selector
- `runtime-capability-invocation`
  - 负责 `ralph#1` 运行时按 metadata 选择 workflow / hat capability

本 change 不负责实现 capability chooser 本身,而是为它提供可靠的知识层边界与读取纪律。

## Goals / Non-Goals

**Goals:**

- 定义 5 层 experience / context 体系的职责边界
- 定义 topic / role / project 三层共享知识的 canonical writer 规则
- 定义 topic / role / project 之间的 promotion / demotion 纪律
- 定义普通 hats 与 `ralph#1` 的默认 injection / read policy
- 定义 role experience 与 project experience 可共用的 entry shape
- 以当前 `.agent/memories.md` 现实为起点,提供清晰迁移方向

**Non-Goals:**

- 不在本 change 中实现 runtime capability invocation 引擎
- 不在本 change 中实现 startup bootstrap selector 本体
- 不要求 v1 就支持所有层级的自动 promotion
- 不要求 v1 就把所有现有 `.agent/memories.md` 历史数据自动精确迁移
- 不在本 change 中设计新的复杂分布式一致性协议

## Decisions

### 决策1: 采用 5 层 knowledge / context 模型

系统采用以下 5 层:

1. `.agent/tasks.jsonl`
   - runtime work graph
2. `.ralph/log/<instance_id>/...`
   - instance 原始轨迹与临时上下文
3. `task_plan__topic.md` / `notes__topic.md` / `WORKLOG__topic.md`
   - topic 共享结论
4. `.ralph/roles/<hat_id>/experience.md`
   - role 级长期经验
5. `experience.md`
   - project 级长期经验

选择这个模型,是因为它把“原始轨迹”“当前协作结论”“岗位规律”“项目通用规律”分开了。

**备选方案:**
- 继续沿用单一 `.agent/memories.md`
- 把 role log 和 role experience 混放在同一目录同一文件体系里

**为什么不选:**
- 单一 memories 无法支撑 runtime chooser 的低噪音读取
- role log 与 role experience 混放会在并行 instance 下迅速失控

### 决策2: 共享知识一律 single canonical writer,多方只能提供 evidence

三类共享知识都采用 single-writer 纪律:

- topic shared files
- role experience
- project experience

writer 以外的 agents 只负责:

- 写各自 instance context
- 发布 evidence / ready / blocked / suggestion

**备选方案:**
- 多个 hats 直接 append 同一 topic / role / project 文件

**为什么不选:**
- 会出现双写、漂移、过期结论并存
- 下轮 LLM 无法判断哪段才是当前可信状态

### 决策3: project experience 默认只允许 `ralph#1` 写

project 根 `experience.md` 是最高影响面的全局知识层。
默认只允许 `ralph#1` 作为 canonical writer 更新它。

普通 hats 只能提供:

- candidate evidence
- promotion suggestion

不能直接落笔 project experience。

**备选方案:**
- topic writer 也能直接升 project
- role writer 可以直接改 project experience

**为什么不选:**
- 全局知识层污染代价太大
- 一旦误晋升,影响的是所有后续 agent 的默认先验

### 决策4: promotion 采用“先窄后宽”,demotion 采用“失活而非硬删”

promotion 默认顺序:

- topic
- role experience
- project experience

如果不确定该升到哪层,先放更窄的那层。

demotion 默认规则:

- 标记 `deprecated`
- 保留 `source_topics` / `source_hats` / `supersedes`
- 注入时跳过失活条目

**备选方案:**
- 允许快速直接升 project
- 发现错误后直接物理删除

**为什么不选:**
- project 误晋升比“不晋升”更危险
- 直接删除会丢失审计链路和回退理由

### 决策5: role experience 与 project experience 共享同一 entry shape

role 和 project 经验都采用统一 entry 结构。
差异由文件位置表达,而不是协议格式表达。

推荐字段:

- `id`
- `summary`
- `scope`
- `source_topics`
- `source_hats`
- `status`
- `confidence`
- `created_at`
- `updated_at`
- `supersedes`

**备选方案:**
- role / project 各自设计一套 markdown 格式

**为什么不选:**
- parser 与 injection 逻辑会分叉
- 后续 promotion / demotion 时需要做格式转换

### 决策6: 读取与注入必须 summary-first, on-demand

默认读取原则:

- 先摘要,后全文
- 先窄范围,后广范围
- 非必要不回读 instance logs

普通 hats 的默认注入顺序:

1. 项目根 `experience.md`
2. `.ralph/roles/<hat_id>/experience.md`
3. 当前 topic 摘要
4. 当前 instance 摘要
5. runtime tasks 状态

`ralph#1` 的默认注入顺序:

1. 项目根 `experience.md`
2. workflows / presets / hats 的 description
3. 若 workflow 已明确,再读 owner hat 的 role experience
4. 当前 topic 摘要
5. 当前 event / tasks 状态

**备选方案:**
- 启动时把所有 role experiences、所有 topic files、所有 instance logs 全读入

**为什么不选:**
- token 浪费极大
- 噪音高于收益
- 与“让 `ralph#1` 先靠 metadata 做首轮筛选”的目标相冲突

## Risks / Trade-offs

- [Writer 过于集中] → 先用 single-writer 保证正确性,后续再考虑受控自动化
- [promotion 太保守,经验沉淀变慢] → v1 接受这个成本,优先避免全局经验污染
- [entry 状态越来越多,管理成本上升] → 统一格式并保留最少状态集: `candidate / active / deprecated`
- [旧 `.agent/memories.md` 与新体系长期并存] → 明确把它视作迁移起点,逐步引入 role/project experience
- [topic writer 或 role writer handoff 不完整] → handoff summary 设为必需协议,没有 handoff 不允许接管共享文件

## Migration Plan

1. 保持当前 `.agent/memories.md` 行为不变,避免破坏现有用户
2. 先引入 role experience / project experience / instance context 的目录和协议
3. 让 injection 逻辑在 v1 中支持:
   - 读取新的 role / project experience
   - 仍兼容旧 `.agent/memories.md`
4. 把旧 memories 中明显属于项目级或角色级的稳定知识,逐步迁移为新 entry
5. 待新体系稳定后,再决定:
   - `.agent/memories.md` 是转为兼容层
   - 还是成为 project `experience.md` 的旧路径别名

## Follow-on Integration Guidance

为了避免后续 change 重复发明自己的写入协议,后续集成应直接复用 scoped experience 的治理层:

- `startup-resource-bootstrap`
  - 负责释放默认 presets / workflows / hats 描述资源
  - 但不要直接绕过治理层去写 `experience.md`
  - 如果 bootstrap 需要落默认经验或默认 owner,应通过:
    - canonical writer metadata
    - scoped experience store / promotion service
    保持写入规则一致
- `runtime-capability-invocation`
  - 负责让 `ralph#1` 在 runtime 根据 metadata 选 workflow / hat / ad-hoc capability
  - 一旦 runtime 需要写 topic / role / project 共享知识:
    - topic shared files 先走 canonical writer ownership 校验
    - role / project experience 先走 promotion / demotion 服务
    - handoff 必须落盘到 append-only summary,而不是仅在内存里切 owner
- `ralph doctor` / debug tooling
  - 应以 `.ralph/canonical-writers/` 为事实来源展示当前 owner
  - 不要在 CLI 层自行猜测“谁现在是 writer”

## Open Questions

- v1 是否需要 human gate 才允许 project experience promotion?
- `candidate` 条目是否需要单独文件,还是先只作为逻辑状态存在?
- `experience.md` 最终是否会替代 `.agent/memories.md`,还是保留兼容双入口?
- role canonical writer 的 primary owner 是显式配置,还是由 `ralph#1` 在运行时判定?
