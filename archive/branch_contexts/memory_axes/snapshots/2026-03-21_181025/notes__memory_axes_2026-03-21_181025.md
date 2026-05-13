## [2026-03-20 16:56:05] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 笔记: 现有 memories / tasks / 六文件 与双轴 memory 设想的关系

## 来源

### 来源1: 当前文档与设计

- 文件:
  - `docs/concepts/memories-and-tasks.md`
  - `docs/advanced/memory-system.md`
  - `specs/ralph-memories/design.md`
  - `tasks/context-file-injection.code-task.md`
  - `config/all_hat.md`
- 要点:
  - 当前官方口径里,`.agent/memories.md` 是跨 session learning
  - `.agent/tasks.jsonl` 是 runtime work tracking
  - 六文件 / `.agent/*.md` 更接近 richer context files
  - `config/all_hat.md` 已经提出过一种“按 `ralph_hat_instance_id` 分目录存文件上下文”的思路

### 来源2: 当前实现

- 文件:
  - `crates/ralph-core/src/event_loop/mod.rs`
  - `crates/ralph-core/src/memory_store.rs`
  - `crates/ralph-core/src/task_store.rs`
- 要点:
  - memories 当前会从 `.agent/memories.md` 自动注入 prompt
  - tasks 当前从 `.agent/tasks.jsonl` 做完成性校验
  - `.agent/memories.md` 是单文件 markdown store
  - `.agent/tasks.jsonl` 是单文件 JSONL store

## 综合发现

### 现有系统其实已经有 3 层 state,只是边界没完全说透

1. `.agent/memories.md`
   - 长期、可复用、可注入
   - 适合短而稳定的 pattern / decision / fix / context
2. `.agent/tasks.jsonl`
   - 当前运行期任务状态
   - 不是 memory,而是 work queue / completion ledger
3. 六文件 / context files
   - 长文本、研究过程、阶段结论、交付记录
   - 更像外部工作记忆与协作上下文

### 用户现在提的,更像是把第 3 层再拆成双轴

- 角色轴:
  - 每个 hat 一份自己的 WORKLOG
  - 放在 `.ralph/` 之下
  - 关注“这个角色干了什么、看到了什么、学到了什么”
- 话题轴:
  - 每个 topic / task 一组六文件或至少一个 `WORKLOG__topic.md`
  - 放在项目目录
  - 关注“围绕这个话题,多 hat 最后收敛出了什么”

### 这个方向和当前系统并不冲突

- `.ralph/` 本来就已经是 runtime evidence / orchestration artifacts 的家
- 项目根的六文件本来就更偏 human-facing / collaborative context
- 所以“角色维度进 `.ralph/`、话题维度留项目根”在概念上是顺的

### 但有一个关键风险: 不要把双轴都做成同等级 primary write target

如果每个 hat 对每个 topic 都直接同时写:

- 角色 log
- 话题 log

那很容易出现:

- 双写
- 漂移
- 并发冲突
- 一边更新了另一边忘了

### 更稳的分层像这样

- 角色轴 = raw append-only ledger
  - 记“谁做了什么”
  - 适合追加,适合保留细节
- 话题轴 = shared synthesis
  - 记“这个 topic 当前对外结论是什么”
  - 应该更克制,更偏汇总和阶段结论
- 长期 memory = distilled stable knowledge
  - 从 topic 关闭后再蒸馏进 `.agent/memories.md`

### 关于目录 key,`hat_id` 和 `instance_id` 要分清

- 如果目的是“角色维度记忆”,目录 key 更像应该是 `hat_id`
  - 例如 `.ralph/roles/writer/WORKLOG.md`
- 如果目的是“实例级调试轨迹”,目录 key 更像是 `ralph_hat_instance_id`
  - 例如 `.ralph/log/writer#2/...`

当前 `config/all_hat.md` 更偏后者。
但用户这次说的是“角色维度”,所以长期视角下我更偏向:

- 目录按 `hat_id`
- 每条记录写清 `instance_id`

### 当前推荐口径

推荐做成 4 层,不要只说“memory 分两部分”:

1. `tasks.jsonl`
   - 运行期任务图
2. `.ralph/roles/<hat_id>/WORKLOG.md`
   - 角色维度 raw log
3. `WORKLOG__topic.md` / `notes__topic.md` / `task_plan__topic.md`
   - 话题维度 shared context
4. `.agent/memories.md`
   - 稳定、短、可注入的 distilled memory

### 一个值得继续深挖的边界

“不同 hat 共同维护 topic 文件”更适合作为语义目标,不适合简单实现成“所有 hat 都直接写同一个文件”。

更稳的可能是:

- 每个 hat 先写自己的角色 log
- 由 coordinator / topic owner / designated summarizer 把它们收敛进 `WORKLOG__topic.md`

这样仍然是“多 hat 共同维护 topic”,但不会变成裸文件并发写。

## [2026-03-20 22:36:07] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 笔记: `experience.md` 命名与 topic canonical writer 规则

## 来源

### 来源1: 当前 prompt / context 注入设计

- 文件:
  - `tasks/context-file-injection.code-task.md`
  - `crates/ralph-core/src/hatless_ralph.rs`
- 要点:
  - 当前已有一个明确方向: 让 `.agent/*.md` 作为 richer context files 被发现和使用
  - 当前长期记忆自动注入仍围绕 `.agent/memories.md`
  - 也就是说,系统已经开始区分“短经验”与“长上下文”,只是命名和多轴规则还没定型

### 来源2: 当前 hat / instance / 文件上下文设想

- 文件:
  - `config/all_hat.md`
- 要点:
  - 该文档已经提出过一个强相关约定:
    - 如果有 `ralph_hat_instance_id`,文件上下文可落到 `./ralph/log/{ralph_hat_instance_id}`
  - 这说明“按 agent 身份分目录持有上下文”并不是横空新增的想法,而是已有雏形

## 综合发现

### 先把“现状事实”和“目标命名”分开

- 现状事实:
  - 当前实现仍是 `.agent/memories.md`
  - 当前自动注入逻辑也仍围绕这个名字
- 目标命名:
  - 用户已明确长期可复用经验应该叫 `experience.md`
- 因此后续设计和 OpenSpec 应该写成:
  - “把当前 `.agent/memories.md` 语义升级/迁移为 `experience.md`”
  - 而不是误写成“系统现在已经有 `experience.md`”

### 我更推荐的 4 层状态口径,现在要把第 4 层正式换名

1. `.agent/tasks.jsonl`
   - runtime work graph
   - 负责“还有哪些工作未闭环”
2. `.ralph/roles/<hat_id>/WORKLOG.md`
   - 角色维度 raw log
   - 负责“这个 hat 做了什么,看到了什么”
3. `task_plan__topic.md` / `notes__topic.md` / `WORKLOG__topic.md`
   - 话题维度 shared context
   - 负责“围绕这个 topic,当前对外结论是什么”
4. `experience.md`
   - 长期可复用经验
   - 负责“哪些结论已经稳定到值得跨 topic 复用”

### canonical writer 不是可选优化,而是 topic 轴成立的前提

如果 topic 轴没有 single-writer 纪律,共享文件会立刻退化成:

- 原始日志堆积
- 并发直写
- 口径飘移
- 很难区分“最新对外结论”和“中间草稿”

所以 topic 轴必须有一个 canonical writer。

### canonical writer 的 4 个候选

1. `ralph#1`
   - 优点:
     - 最清楚全局路由和跨 hat 状态
     - 最适合在“还没选定 workflow”时兜底
   - 缺点:
     - 容易变成所有 topic 文件的瓶颈
     - 对某个 workflow 的领域语义不一定最细

2. workflow owner / finalizer hat
   - 优点:
     - 最理解该 workflow 的 ready 条件和收敛语义
     - topic 文件由真正负责收尾的角色维护,口径最自然
   - 缺点:
     - 前提是 workflow 已经被选出来
     - 对无根目录 `ralph.yml` 的 ad-hoc direct hat 场景,需要先决定 owner

3. 专门 summarizer hat
   - 优点:
     - 职责很纯
   - 缺点:
     - 新增一个抽象层
     - 很容易为了“记文件”而引入额外编排复杂度

4. human 手工维护
   - 优点:
     - 最可控
   - 缺点:
     - 自动化太弱
     - 会破坏“ralph#1 自行判断选 workflow / 选 hat / 运行 ad-hoc hat”的目标

### 当前推荐: “owner-first, `ralph#1` fallback”

我更推荐这样定义:

- 若已选定 workflow:
  - topic shared files 的 canonical writer = 该 workflow 的 owner / lead / finalizer hat
- 若还没选定 workflow:
  - 先由 `ralph#1` 临时担任 canonical writer
- 若是 ad-hoc direct hat 调用:
  - `ralph#1` 先根据用户请求决定:
    - 直接把某个 hat 当作临时 topic owner
    - 或者继续自己担任 canonical writer,直到确认 owner

这样比“永远都让 `ralph#1` 写”更可扩展。
也比“所有 hat 都能写 topic 文件”更稳。

### 推荐的写入纪律

- 所有 hat:
  - 允许写自己的 role log
  - 不直接写 topic shared files
- canonical writer:
  - 负责维护 `task_plan__topic.md`
  - 负责维护 `notes__topic.md`
  - 负责维护 `WORKLOG__topic.md`
  - 必要时从 role log 和 event stream 做摘要与归并

也就是说:

- “多 hat 共同维护 topic”是语义层的共同维护
- “共享文件实际落笔”应当是单 writer 收敛

### topic -> `experience.md` 的晋升条件

不是每个 topic 完成都要写进 `experience.md`。
更合理的是满足以下条件再晋升:

- 跨 topic 可复用
- 不是某一次任务的临时状态
- 能压缩成短而稳定的经验
- 未来值得自动注入给 LLM

举例:

## [2026-03-21 00:40:47] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 笔记: scoped experience injection 首批实现的代码落点与验证结论

## 来源

### 来源1: OpenSpec change `scoped-experience-system`

- 文件:
  - `openspec/changes/scoped-experience-system/design.md`
  - `openspec/changes/scoped-experience-system/specs/experience-injection/spec.md`
  - `openspec/changes/scoped-experience-system/tasks.md`
- 要点:
  - 普通 hat 需要按 `project -> role -> topic -> instance -> runtime` 的顺序注入
  - `ralph#1` 需要 metadata-first:
    - 先项目经验
    - 再 workflow / hat 描述
    - scope 缩小后再读 role experience
  - topic / instance 读取必须 summary-first,并避免把所有历史 topic 全塞进 prompt

### 来源2: 本轮代码实现

- 文件:
  - `crates/ralph-core/src/experience_injection.rs`
  - `crates/ralph-core/src/event_loop/mod.rs`
  - `crates/ralph-core/src/event_loop/tests.rs`
- 要点:
  - 新增 `ScopedPromptMode`
    - `Ordinary { role_hat_id, instance_id }`
    - `Coordinator { owner_role_hint }`
  - 注入实现被统一收口到 `inject_scoped_context(...)`
  - 旧的 `prepend_memories()` 不再只处理 `.agent/memories.md`,而是变成 scoped context 的总入口

## 综合发现

### 现象

- 本轮之前:
  - project / role experience 已有路径和 parser/store
  - 但 prompt 构建链路里仍只有 legacy `.agent/memories.md`
- 本轮之后:
  - 普通 hat 的 prompt 已经能拿到:
    - project experience
    - role experience
    - unique topic summary
    - instance summary
    - runtime task state
  - Ralph 的 prompt 已经能拿到:
    - pre-routing project experience
    - post-metadata owner role experience
    - topic summary
    - runtime task state

### 当前实现策略

1. legacy memories 继续兼容
   - `.agent/memories.md` 仍会读
   - 但 prompt 中显式标记为 `Legacy Memories (Compatibility)`
2. topic summary 走保守唯一组策略
   - 扫描工作区根目录的:
     - `task_plan__*.md`
     - `notes__*.md`
     - `WORKLOG__*.md`
   - 只有当 suffix group 唯一时才注入
   - 如果存在多个 topic group,直接不 eager inject
3. instance summary 走固定候选文件策略
   - `.ralph/log/<instance_id>/SUMMARY.md`
   - `.ralph/log/<instance_id>/WORKLOG.md`
   - `.ralph/log/<instance_id>/notes.md`
   - `.ralph/log/<instance_id>/task_plan.md`
4. runtime task state 直接读取 `.agent/tasks.jsonl`
   - 只做 open / ready 摘要
   - 作为最后一层 runtime state 注入

### 动态验证结论

- `cargo test -p ralph-core --lib`
  - 472 个单测全部通过
  - 新增的 injection tests 也全部通过
- `cargo test -p ralph-core smoke_runner`
  - 12 个 smoke runner tests 全部通过

### 这轮修过的两个真实问题

1. Ralph metadata-first 测试锚点写错
   - 现象:
     - 测试原本找 `## HATS`
     - 实际 prompt 在该场景里给的是 `## ACTIVE HAT`
   - 结论:
     - 不是实现没注入 role experience
     - 是测试用错了 prompt 结构锚点
2. legacy memories compatibility 测试样例不符合旧 parser 规范
   - 现象:
     - prompt 里完全没有 legacy memories section
   - 静态证据:
     - `memory_parser.rs` 要求 memory id 满足 `mem-<unix>-<4hex>`
     - metadata 时间只接受 `YYYY-MM-DD`
   - 结论:
     - 不是 compatibility 逻辑失效
     - 是测试样例本身没被 parser 接受

- 可以进 `experience.md`:
  - “客户顾问委员会类 workflow 里,host/agenda/logistics 未齐前不要触发 final confirmation”
  - “当 topic 仍未选定 owner 时,由 `ralph#1` 暂代 canonical writer 最稳”
- 不应进 `experience.md`:
  - “这次 CAB 用户选了 cohort A”
  - “这轮 writer#2 跑超时后重试成功”

### 一个很实用的规则

可以把三种写入对象理解成:

- role log = 证据层
- topic files = 协作结论层
- `experience.md` = 稳定规律层

这样边界最清楚,也最不容易双写失控。

## [2026-03-20 22:52:27] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 笔记: 岗位级 `experience.md` 会把 role 轴再拆成“实例轨迹”和“角色经验”

## 来源

### 来源1: 用户新增要求

- 用户明确要求:
  - `.ralph/roles/<hat_id>` 下也要有基于这个岗位的 `experience.md`

### 来源2: 现有角色上下文雏形

- 文件:
  - `config/all_hat.md`
- 要点:
  - 该文档已经建议按 `ralph_hat_instance_id` 建立独立文件上下文目录
  - 这说明“实例级上下文”本来就比“角色级共享工作文件”更接近当前思路

## 综合发现

### 这条新增要求会把旧的 v2 模型再推进一步

我之前的 v2 口径是:

- `.ralph/roles/<hat_id>/WORKLOG.md`
- `WORKLOG__topic.md`
- `experience.md`

但现在用户要求 `roles/<hat_id>/experience.md` 也存在。
这会暴露一个问题:

- 如果 role 目录同时放:
  - raw `WORKLOG.md`
  - stable `experience.md`
- 而且同一个 hat 还可能有多个 instance

那么 role 目录会同时承担:

- 多实例执行轨迹汇流
- 角色稳定规律沉淀

这两件事混在一起,边界会变差。

### 所以我更推荐升级成 v3 口径

1. instance 级
   - 位置:
     - 更推荐 `./ralph/log/<instance_id>/...`
     - 或未来统一命名成 `.ralph/instances/<instance_id>/...`
   - 内容:
     - `task_plan.md`
     - `notes.md`
     - `WORKLOG.md`
   - 职责:
     - 保存单个 agent instance 的原始执行轨迹与临时工作记忆

2. role 级
   - 位置:
     - `.ralph/roles/<hat_id>/experience.md`
   - 内容:
     - 这个岗位长期适用的做法、判断准则、收敛偏好
   - 职责:
     - 保存“作为这个 hat,以后普遍该怎么做”

3. topic 级
   - 位置:
     - 项目根 `task_plan__topic.md` / `notes__topic.md` / `WORKLOG__topic.md`
   - 内容:
     - 多 hat 协作后的当前共享结论
   - 职责:
     - 保存“围绕这个 topic,现在一致认可的状态是什么”

4. project 级
   - 位置:
     - 项目根 `experience.md`
   - 内容:
     - 跨角色、跨 topic 都成立的稳定经验
   - 职责:
     - 保存“这个项目以后都值得默认记住的规律”

5. runtime 级
   - 位置:
     - `.agent/tasks.jsonl`
   - 职责:
     - 运行期 work graph

### 这个 v3 模型更顺

因为它把“谁在产生信息”与“谁在沉淀规律”拆开了:

- instance 产生原始轨迹
- role 沉淀岗位经验
- topic 沉淀协作结论
- project 沉淀通用经验

这样不会再让 role 目录既像日志池,又像知识库。

### 岗位级 `experience.md` 应该写什么

适合放进 `.ralph/roles/<hat_id>/experience.md` 的内容:

- 这个 hat 的默认判断偏好
- 这个 hat 常见的收敛条件
- 这个 hat 在历史上反复证明有效的方法
- 这个 hat 特有的禁忌和护栏

例子:

- `cab_program_lead`:
  - “未收齐 cohort / agenda / host / logistics 之前,不要发布 final confirmation”
- `spec_reviewer`:
  - “发现 requirement 第一行没有 MUST/SHALL 时,优先先拦住归档”

不适合放进去的:

- 某一次 topic 的临时状态
- 某个客户、本轮任务、某次运行才成立的细节

### 谁来写 role 级 `experience.md`

这里也不能放任“所有同岗位实例都直接写”。

我更推荐:

- role experience 的 canonical writer = 该 hat 的 primary owner
- 在并行实例存在时:
  - 其他 instance 先写自己的 instance log
  - 由 primary owner 或 `ralph#1` 做角色级经验提炼

也就是说:

- topic 级有 topic canonical writer
- role 级也应有 role canonical writer

只是二者负责的对象不同:

- role canonical writer 负责岗位规律
- topic canonical writer 负责当前话题结论

### 推荐注入顺序

如果未来某个 hat 开始工作,我建议注入顺序是:

1. 项目级 `experience.md`
   - 先给全局稳定规律
2. 岗位级 `.ralph/roles/<hat_id>/experience.md`
   - 再给这个岗位自己的长期经验
3. 当前 topic 相关 shared files 摘要
   - 再给这次任务的共享上下文
4. 当前 instance 自己的临时上下文
   - 最后给本轮局部轨迹

这样顺序是:

- 先全局规律
- 再角色规律
- 再任务状态
- 最后局部临时信息

LLM 读起来会更稳。

### 一个更完整的层级图

```text
project experience.md
        │
        ▼
.ralph/roles/<hat_id>/experience.md
        │
        ▼
task_plan__topic.md / notes__topic.md / WORKLOG__topic.md
        │
        ▼
.ralph/log/<instance_id>/{task_plan,notes,WORKLOG}.md
        │
        ▼
.agent/tasks.jsonl
```

### 这版口径对之前结论的修正

所以我会把之前那句:

- “`.ralph/roles/<hat_id>/WORKLOG.md` = 角色维度 raw log”

修正成:

- “instance 级文件更适合承载 raw log”
- “role 级文件更适合承载 stable role experience”

这是一次结构改良,不是推翻方向。
方向还是对的,只是 role 轴的边界被你这条新要求进一步校正了。

## [2026-03-20 22:55:38] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 笔记: 经验晋升规则与 `memories.path` 现状核对

## 来源

### 来源1: 当前 memory 文档

- 文件:
  - `docs/concepts/memories-and-tasks.md`
  - `docs/advanced/memory-system.md`
  - `specs/ralph-memories/design.md`
- 要点:
  - 文档主口径仍以 `.agent/memories.md` 为中心
  - `docs/advanced/memory-system.md` 已出现:
    - `memories.path: .agent/memories.md`
  - 这意味着文档已经在朝“memory path 可配置”方向表达

### 来源2: 当前代码实现

- 文件:
  - `crates/ralph-core/src/config.rs`
  - `crates/ralph-core/src/event_loop/mod.rs`
- 要点:
  - `MemoriesConfig` 当前只有:
    - `enabled`
    - `inject`
    - `budget`
    - `filter`
  - 还没有 `path` 字段
  - `prepend_memories()` 当前直接:
    - `MarkdownMemoryStore::with_default_path(workspace_root)`
    - `workspace_root.join(\".agent/memories.md\")`

## 综合发现

### 先钉住一个事实: 文档口径已经先于实现

- 已验证事实:
  - 文档里已经出现 `memories.path`
  - 当前实现里还没有真正可配置的 memory path
- 这件事的重要性在于:
  - 后面如果要把 `experience.md`、role experience、topic context 带进正式设计
  - 必须从“代码当前还是单一 `.agent/memories.md`”这个现实出发

### 我推荐的经验晋升漏斗

```text
instance 轨迹
   │
   ▼
topic shared context
   │
   ├── 若是岗位特定规律 ──► role experience
   │
   └── 若是跨角色规律 ───► project experience

role experience
   │
   └── 若后来证明跨角色通用 ─► project experience
```

### 默认策略: “先窄后宽”

如果一个经验到底应该进 role 还是 project 不确定,默认先放窄的那层:

- 能只对一个 hat 生效:
  - 先放 role experience
- 只有在后来确认:
  - 多个 hat 都依赖它
  - 或 `ralph#1` 在路由前就需要知道它
  - 再升到 project experience

这样更稳。
因为 project experience 一旦被自动注入给所有 agent,污染面更大。

### topic -> role experience: 什么时候该升

满足下面大多数条件时,就应该从 topic 提炼到 `.ralph/roles/<hat_id>/experience.md`:

- 这条经验明显属于某一个 hat 的职责域
- 它描述的是该 hat 的长期判断方式,不是本轮临时状态
- 未来同岗位在别的 topic 里仍然大概率会用到
- 它足够稳定,可以压缩成短规则
- 让这个 hat 下次一开始就知道,会明显减少试错

典型例子:

- `cab_program_lead`
  - “host / agenda / logistics 没齐前,不要进入 final confirmation”
- `spec_reviewer`
  - “Requirement 第一行没出现 MUST/SHALL 时,先阻断归档”

不该升的:

- “这次 CAB 用户选了 cohort B”
- “这轮 spec reviewer 先看了 design 再看 proposal”

### topic -> project experience: 什么时候可以直接升

只有当经验从 topic 中看起来已经不是“某个岗位的手艺”,而是“整个项目都该默认知道”的规则时,才直接进入项目根 `experience.md`。

典型条件:

- 多个 hat 都会用到
- `ralph#1` 在做 workflow 选择前就需要知道
- 这是项目级协作约束,而不是某个角色的小技巧
- 注入给所有 agent 的收益大于噪音

典型例子:

- “当 topic owner 未明确时,由 `ralph#1` 暂代 canonical writer”
- “topic shared files 只能由 canonical writer 落笔,其他 hat 只提供证据”

### role experience -> project experience: 什么时候再升一级

role experience 不应轻易升 project。
我建议至少满足以下之一:

- 同一规律已经在两个以上角色里分别出现
- 该规律不再是岗位特定技巧,而变成系统级约束
- `ralph#1` 在选择 workflow / 选 hat / 注入上下文时必须提前知道
- 如果不升到 project,跨角色协作会重复踩坑

例子:

- 原本只在 `cab_program_lead` 里成立:
  - “未收齐 ready inputs 不要发 final confirmation”
- 如果后来发现:
  - proposal assembly
  - launch readiness
  - onboarding activation
  都遵循同一收敛律
- 那它就从 role experience 晋升成 project experience

### 一个实用的判定问题

每次想晋升经验时,先问 3 个问题:

1. 下次如果换一个 topic,这条规则还成立吗?
2. 下次如果换一个 hat,这条规则还成立吗?
3. 下次如果让 `ralph#1` 先做路由,它需不需要提前知道?

判定方式:

- 只有第1题是:
  - 留在 topic
- 第1题和第2题是:
  - 升到 role experience
- 第1题、第2题、第3题多数都是:
  - 升到 project experience

### 我建议加一个“候选经验”心智模型,但不一定先做成文件

为了避免“还不够稳,但又怕忘”,可以先把它们留在:

- `notes__topic.md`
- `WORKLOG__topic.md`

并在语义上把它们视作:

- candidate role experience
- candidate project experience

先不急着正式晋升。
这样既不会丢,也不会过早污染长期经验层。

### 当前收敛后的 5 层模型

1. `.agent/tasks.jsonl`
   - runtime work graph
2. instance 级上下文
   - 原始轨迹与临时记忆
3. topic shared files
   - 当前协作结论
4. `.ralph/roles/<hat_id>/experience.md`
   - 岗位级长期经验
5. 项目根 `experience.md`
   - 项目级长期经验

### 一句话总结

我现在最推荐的口径是:

- topic 负责保存“这次发生了什么”
- role experience 负责保存“这个岗位以后一般该怎么做”
- project experience 负责保存“整个项目以后都该怎么做”

## [2026-03-20 23:08:29] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 笔记: memory / experience 体系准设计稿

## 设计目标

- 支持:
  - 无根目录 `PROMPT.md`
  - 无根目录 `ralph.yml`
  - `ralph#1` 依据 workflow 描述、hat 描述、项目经验自动选择工作流
- 同时避免:
  - 多 writer 并发改同一共享文件
  - 角色技巧污染全局经验
  - 全量文件无差别注入导致 prompt 失控

## 核心对象

### 1. runtime work graph

- 文件:
  - `.agent/tasks.jsonl`
- 用途:
  - 当前 run 里还有哪些工作未闭环

### 2. instance context

- 位置:
  - `.ralph/log/<instance_id>/...`
- 内容:
  - `task_plan.md`
  - `notes.md`
  - `WORKLOG.md`
- 用途:
  - 单个 agent instance 的原始轨迹和临时工作记忆

### 3. topic shared context

- 位置:
  - 项目根 `task_plan__topic.md`
  - 项目根 `notes__topic.md`
  - 项目根 `WORKLOG__topic.md`
- 用途:
  - 当前 topic 的共享结论

### 4. role experience

- 位置:
  - `.ralph/roles/<hat_id>/experience.md`
- 用途:
  - 某岗位长期适用的稳定经验

### 5. project experience

- 位置:
  - 项目根 `experience.md`
- 用途:
  - 整个项目级别的长期稳定经验

## canonical writer 设计

### A. topic canonical writer

优先级:

1. workflow 明确声明的 owner / finalizer hat
2. ad-hoc direct hat 场景中,由 `ralph#1` 指定的临时 topic owner
3. `ralph#1` 兜底

写入权限:

- topic shared files 只允许 topic canonical writer 落笔
- 其他 hats:
  - 只写各自 instance context
  - 或发布 evidence / ready / blocked 事件

### B. role canonical writer

优先级:

1. 该 hat 的 primary owner
2. 若无 primary owner,由 `ralph#1` 临时担任

写入权限:

- `.ralph/roles/<hat_id>/experience.md` 只允许 role canonical writer 更新
- 其他同岗位实例:
  - 只能提供 instance 轨迹
  - 不能直接改 role experience

### C. project canonical writer

推荐规则:

- 默认只允许 `ralph#1` 写项目根 `experience.md`
- 如果以后要放开,也应该是显式配置,而不是默认开放

原因:

- project experience 一旦自动注入,影响面最大
- 它不是某个岗位的技巧库,而是全局知识层

## canonical writer 交接规则

### topic writer 交接

触发条件:

- workflow 在运行中才被选定 owner
- 当前 writer 退出 / 卡死 / 被替换
- `ralph#1` 从临时 owner 退回协调者角色

交接动作:

1. 旧 writer 在 topic 文件尾部追加 handoff summary
2. summary 至少包含:
   - 当前结论
   - 未完成事项
   - 依赖哪些 evidence / role logs
   - 当前 owner 变更原因
3. 新 writer 接手后:
   - 先读 topic 文件尾部 handoff summary
   - 再按需读取相关 instance logs
   - 然后才继续写 topic shared files

### role writer 交接

触发条件:

- 某岗位 primary owner 不可用
- 该岗位由新 owner 接管

交接动作:

1. 老的 role writer 不直接把 instance 轨迹搬进 role experience
2. 只追加一条“已验证岗位经验”摘要
3. 新 writer 读取 role experience 尾部最近若干条 active 经验后继续

### project writer 交接

默认不鼓励复杂化。
如果未来必须支持:

- 仍建议通过 `ralph#1` 单点完成
- 不建议把 project experience 写入权分散给普通 hats

## experience 的统一格式建议

我更推荐:

- role experience
- project experience

都使用同一套 entry 结构。
区别由“文件位置”表达,而不是再设计两套格式。

建议字段:

- `id`
- `summary`
- `scope`
- `source_topics`
- `source_hats`
- `status`
  - `candidate`
  - `active`
  - `deprecated`
- `confidence`
- `created_at`
- `updated_at`
- `supersedes`

这样做的好处:

- parser 只需要一套
- injection 逻辑也更容易复用
- 以后做升降级时,不必重写文件协议

## 晋升规则

### topic -> role experience

默认条件:

- 该规律只明显属于某一个 hat
- 跨 topic 仍能复用
- 不是本轮任务的临时信息
- 可以压缩成短规则

### topic -> project experience

默认条件:

- 多个 hats 都会用到
- `ralph#1` 路由前就该知道
- 这是项目级协作规则
- 全局注入收益大于噪音

### role experience -> project experience

默认条件:

- 同一规律在两个以上角色中重复出现
- 已从岗位技巧演化成系统级约束
- 不升会让跨角色协作反复踩坑

## 降级 / 回收规则

这部分很关键,因为“误晋升”比“不晋升”更危险。

### project -> role

当发现某条 project experience:

- 实际只对单一岗位成立
- 注入给其他 hats 只会制造噪音

则应:

1. 把 project experience 标记为 `deprecated`
2. 在 `supersedes` / 注释里指向对应 role experience
3. 后续注入时不再默认带上该 project entry

### role -> topic

当发现某条 role experience:

- 其实只是某个具体 topic 的临时处理法
- 并不具备岗位长期复用价值

则应:

1. role entry 标记为 `deprecated`
2. 保留来源 topic 链接
3. 不再自动注入给该 role

### 不建议物理删除

更稳的是:

- 逻辑降级
- 状态失活
- 保留审计链路

而不是直接删掉。

因为这类知识系统后面一定会遇到:

- “为什么以前有,现在没了”
- “是谁判断它不再成立”

如果直接删,这条因果链就断了。

## 默认注入顺序

### 对普通 hat

1. 项目根 `experience.md`
2. `.ralph/roles/<hat_id>/experience.md`
3. 当前 topic shared context 的摘要
4. 当前 instance context 尾部摘要
5. runtime tasks 相关状态

### 对 `ralph#1`

1. 项目根 `experience.md`
2. 当前候选 workflows / presets 的简要描述
3. 若已选中 workflow,再读对应 owner hat 的 role experience
4. 当前 topic shared context 摘要
5. 当前 event / tasks 状态

关键点:

- `ralph#1` 在还没选 workflow 前,不应把所有 role experience 都读进来
- 应该先靠:
  - project experience
  - workflow / hat descriptions
  做首轮筛选

## 默认读取策略

### 读取原则

- 永远按需读取
- 永远先摘要,后全文
- 永远先窄范围,后广范围

### 建议顺序

1. 先看当前 event / topic 是否已明确
2. 若未明确:
   - 先读 project experience
   - 先读 workflows / hats 的 description
3. 若 topic 已明确:
   - 读 topic shared files 的最新摘要
4. 若 hat 已明确:
   - 再读对应 role experience
5. 只有需要追证据时:
   - 才回读 instance logs

### 明确不建议

- 不建议启动时把所有 topic 文件全读进 prompt
- 不建议启动时把所有 role experiences 全注入给所有 hats
- 不建议把 instance logs 当成默认全局记忆

## 无 `PROMPT.md` / 无 `ralph.yml` 的启动语义

如果用户直接给 `ralph#1` 一条消息:

1. `ralph#1` 先读项目根 `experience.md`
2. 再看内嵌 presets / workflows / hats 的 description
3. 判断是:
   - 选一整套 workflow
   - 只启用某个 hat
   - 还是临时拼一组 hats
4. workflow 一旦确定:
   - topic canonical writer 也跟着确定
5. 后续再按需读取 role experience 和 topic context

这样才符合你前面提出的目标:

- Ralph 先知道有哪些“可调用的 workflow / hat”
- 再根据用户消息自行决定用哪套
- 而不是强依赖根目录存在 `ralph.yml`

## 我现在最推荐的最小实现哲学

- 不要一开始就做“所有层级都能自动晋升”
- 先把:
  - writer 权限
  - 注入顺序
  - 晋升/降级状态
  这三个骨架搭稳
- 只要这三件事对了,后面的自动化程度可以慢慢加

## [2026-03-21 01:00:28] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 笔记: canonical writer / promotion / demotion 的实现落点与一个关键修正

## 来源

### 来源1: 新实现代码

- 文件:
  - `crates/ralph-core/src/experience_governance.rs`
  - `crates/ralph-core/src/experience_promotion.rs`
  - `crates/ralph-core/src/experience.rs`
  - `crates/ralph-core/src/experience_parser.rs`
  - `crates/ralph-core/src/experience_store.rs`
- 要点:
  - canonical writer 被做成了独立治理层,而不是散在 event loop / CLI 里
  - writer ownership 落盘到 `.ralph/canonical-writers/`
  - promotion / demotion 通过 `ScopedExperienceService` 执行
  - demotion 增加了 `replaced_by` 链路,与已有 `supersedes` 互补

### 来源2: 验证与失败回合

- 命令:
  - `cargo test -p ralph-core --lib`
  - `cargo test -p ralph-core smoke_runner`
  - `cargo test -p ralph-cli doctor_`
  - `cargo test`
  - `openspec validate scoped-experience-system --type change`
- 要点:
  - 第一轮失败不是设计错误,而是 demotion 函数里的 Rust 借用顺序问题
  - 修正后全部验证通过

## 综合发现

### 1. writer ownership 现在终于有了稳定事实源

- 不是靠 prompt 猜
- 不是靠 runtime 临时内存猜
- 而是通过:
  - `.ralph/canonical-writers/project.json`
  - `.ralph/canonical-writers/roles/<hat_id>.json`
  - `.ralph/canonical-writers/topics/<suffix>.json`
  持久化当前 canonical writer

这使得:

- doctor/debug 可以直接展示真实 owner
- 后续 runtime capability invocation 不必重新发明 ownership 存储

### 2. topic handoff 和 role handoff 不能一概都塞进目标 experience 文件

我这轮遇到的关键实现发现是:

- topic handoff 追加到 `WORKLOG__topic.md` 没问题
- 但 role handoff 如果直接追加到 `.ralph/roles/<hat_id>/experience.md`
  - 会被 `MarkdownExperienceStore::append/load/write_all` 这类“整文件重写”流程冲掉

因此这轮改成:

- topic handoff -> `WORKLOG__topic.md`
- role handoff -> `.ralph/roles/<hat_id>/handoff.md`

这个修正很重要。
它不是风格问题,而是避免 handoff 摘要被后续正常 experience 写入静默覆盖。

### 3. demotion 如果只有 `supersedes`,追链方向还是不够顺手

旧协议里只有:

- 新条目 `supersedes` 旧条目

但在真实排障里,经常是从“旧条目为什么失活了”往后追。

所以这轮补了:

- 旧条目 `replaced_by`

这样 project -> role / role -> topic 的降级审计链就变成双向可读:

- 新条目知道自己替代了谁
- 旧条目也知道后来被谁或哪个 topic 取代

### 4. follow-on changes 的接入边界现在更清楚了

- `startup-resource-bootstrap`
  - 负责释放默认资源
  - 但不应绕开治理层直接写共享知识
- `runtime-capability-invocation`
  - 负责 runtime 选 workflow / hat / ad-hoc capability
  - 一旦要写 topic / role / project 共享知识,必须复用:
    - canonical writer ownership
    - promotion / demotion service
    - handoff append-only summary

### 5. 这轮实现后,`scoped-experience-system` 的状态已经从“只会读”升级到“会治理、会晋升、会降级”

现在已经不只是:

- project / role experience 能注入

而是还具备:

- non-owner 写入拒绝
- writer handoff
- topic -> role / project promotion
- role -> project promotion
- project -> role / role -> topic demotion
- doctor 可见性

## [2026-03-21 12:41:58] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 笔记: examples E2E 实跑验证与证据边界

## 来源

### 来源1: `crates/ralph-e2e/README.md`

- 要点:
  - examples 目录已经被接成 Tier 8 的真实 E2E scenarios
  - `ralph-e2e` 的自然入口就是 `cargo run -p ralph-e2e -- <backend> --filter ...`

### 来源2: `cargo run -p ralph-e2e -- --list`

- 要点:
  - 当前可见 `parallel-*-example` 场景共 26 条
  - 统一都属于 `Tier 8: Parallel Runtime`
  - `--filter example` 可以一次性命中整批 examples 场景

### 来源3: `rg -n -A4 "fn supported_backends\\(&self\\) -> Vec<Backend> \\{" crates/ralph-e2e/src/scenarios/parallel_*example.rs`

- 要点:
  - 这些 example scenarios 当前统一声明 `vec![Backend::Codex]`
  - 所以 examples 真后端验证应优先用 `codex`

### 来源4: 实际执行命令

- 命令:
  - `cargo run -p ralph-e2e -- codex --filter example --report both --skip-analysis`
- 要点:
  - runner 启动时明确显示 `Running 26 scenarios...`
  - 说明这不是单场景误匹配,而是完整 examples 批量验证入口

### 来源5: `.e2e-tests/report-live.md`

- 要点:
  - 本轮 live report 明确写到:
    - `Progress: 1/26 scenarios completed`
    - `Passed: 1 | Failed: 0`
    - `parallel-trigger-routing-example (159.9s)`

### 来源6: `.e2e-tests/parallel-trigger-routing-example/.ralph/events.jsonl`

- 要点:
  - 第一条 example 真实走过:
    - `spec.start`
    - `spec.ready`
    - `spec.rejected`
    - 再次 `spec.ready`
  - 被拒绝原因是 payload 内含 `version: 1`
  - 修正到 `version: 2` 后,整条 example 最终通过

### 来源7: `.e2e-tests/parallel-experimental-dev-engine-example/.ralph/events.jsonl` 与进程树

- 要点:
  - 第二条 example 不是静态拷贝,而是真正启动了:
    - `target/release/ralph run -c examples/parallel-experimental-dev-engine/ralph.yml --max-iterations 40 --no-tui`
    - `codex app-server --listen stdio:// ...`
  - 已观察到 `ralph#1` 派发:
    - `experiment.task` for `exp-001`
    - `experiment.task` for `exp-002`
  - 同时 `agents.json` 显示:
    - `experiment_runner#1` running
    - `experiment_runner#3` running

### 来源8: 中断后的进程检查

- 命令:
  - 对 `cargo run -p ralph-e2e -- codex --filter example --report both --skip-analysis` 发送 `Ctrl-C`
  - `ps -axo pid,ppid,etime,command | rg 'target/release/ralph run -c examples/parallel-experimental-dev-engine/ralph.yml|codex app-server'`
- 要点:
  - `ralph-e2e` 主进程退出后,子进程一度仍在
  - 需要额外 `kill <ralph-pid>` 才能把这组残留一起清干净

## 综合发现

### 1. `--filter example` 是正确入口,但真后端批跑在交互式会话里非常重

- 这是已验证结论,不是猜测:
  - runner 明确匹配到 26 条 examples scenarios
  - 第一条单独就耗时 159.9 秒
- 这意味着:
  - 全量 examples 真后端批跑更适合无人值守窗口
  - 交互式验证更适合先跑“代表性 example 子集”

### 2. `parallel-trigger-routing-example` 已拿到完整通过证据

- 静态证据:
  - `report-live.md` 记录该场景通过,耗时 159.9 秒
- 动态证据:
  - 事件流中清楚看到:
    - 初稿 `spec.ready(version: 1)`
    - reviewer 以固定规则拒绝
    - writer 修正为 `version: 2`
  - 说明这个 example 不只是“启动成功”,而是真实跑完了一段有 backpressure 的闭环

### 3. `parallel-experimental-dev-engine-example` 至少已经确认“派发成功并进入并行执行”

- 当前结论要严格表述为:
  - 已确认它完成了 `experiment.task` fanout
  - 但在本轮观察窗口内,还没有看到 `experiment.result` 回流
- 不能把它表述成“卡死”或“失败”
- 更准确的说法是:
  - 它在被人工中断前,停留在“已派发、结果未回流”的阶段

### 4. 运行中的或被中断的 examples 批次,不能信任旧的 `report.json` / `report.md`

- 本轮一开始就撞到了一个很容易误判的点:
  - `.e2e-tests/report.json`
  - `.e2e-tests/report.md`
  仍然停留在 3 月 12 日的历史快照
- 真正反映本轮状态的是:
  - `.e2e-tests/report-live.md`
  - 当前 workspace 下的 `.ralph/events.jsonl`
  - 当前 workspace 下的 `.ralph/agents.json`

### 5. 中断 `ralph-e2e` 时,要额外检查是否遗留 `ralph run` / `codex app-server`

- 这是本轮观察到的现象:
  - `Ctrl-C` 让 `ralph-e2e` 退了
  - 但子进程没有立刻跟着消失
- 当前只可把它记成“操作层现象”
- 还不能直接下结论说:
  - “根因已经确认是 e2e harness 没有 kill process group”
- 如果后面要修,需要再补:
  - 中断路径的静态调用链
  - SIGINT / SIGTERM 的动态验证

## [2026-03-21 17:32:42] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 笔记: `parallel-experimental-dev-engine` 没回流的根因收敛

## 来源

### 来源1: 手动录制文件 `.e2e-tests/parallel-experimental-dev-engine-recording/.ralph/parallel-experimental-dev-engine-recording.jsonl`

- 要点:
  - `experiment_runner#1` 在录制中出现了:
    - `stdout=false` 的 `<event topic="experiment.result" reply="k_BsMWRaK40r">`
    - 没有对应的 `bus.publish(topic=experiment.result)`
  - `experiment_runner#3` 在录制中先出现 `stdout=false` 的 event 文本,随后又出现:
    - `stdout=true` 的标准 `<event topic="experiment.result" reply="yZBUyCKO8AMd">`
    - 紧接着出现真实 durable 记录:
      - `bus.publish(topic=experiment.result, source_instance=experiment_runner#3)`

### 来源2: worktree 现场

- 路径:
  - `.e2e-tests/parallel-experimental-dev-engine-recording/.ralph/worktrees/experiment_runner_1/job-1`
  - `.e2e-tests/parallel-experimental-dev-engine-recording/.ralph/worktrees/experiment_runner_3/job-1`
- 要点:
  - `experiment_runner#1`:
    - 存在 `e2e_marker_exp_001.txt`
    - `git log -1 --oneline` 为 `9bd8bcc exp-001: e2e marker file`
  - `experiment_runner#3`:
    - 存在 `e2e_marker_exp_002.txt`
    - 真实上报 payload 中的 commit 为 `85ac8d9c27649c7c4f6b47d4bcbc5dd5add68df5`
  - 结论:
    - “没回流”不是因为 runner 没有执行实现或没做验证
    - 至少一个 runner 已经证明实现链路是通的

### 来源3: `crates/ralph-cli/src/parallel_runner.rs`

- 关键代码:
  - `finalize_output_for_parsing()` 只把 `stdout_output` 交给 event parser
  - `handle_stream_line()` 明确说明 stderr 只用于可观测输出,不会进入 `HatJobResult.output`
- 要点:
  - 这是已知且正确的护栏
  - 不能为了“捡回 stderr 里的 event”去放松这个约束,否则会把 prompt 示例、MCP 启动日志、warning 混成假事件

### 来源4: `config/all_hat.md`

- 要点:
  - 当前存在“外部事件注入: `ralph emit`”这段通用说明
  - 对具备 shell/tool 能力的 hat 来说,这容易把“正常 workflow event 发射”误解成“也可以通过命令执行去发”
  - 目前缺少一条明确规则:
    - 正常 workflow event 必须直接由 assistant 最终 stdout 发出
    - 不能通过 shell/tool transcript 间接输出

## 综合发现

### 现象

- `parallel-experimental-dev-engine` 在单独录制时,长期只有:
  - `experiment.task`
  - 没有稳定出现两条 `experiment.result`
- 但 worktree 已经出现实验产物和独立 commit

### 主假设

- `experiment_runner#1` 把 `experiment.result` 发到了错误通道:
  - event 文本出现在 shell/tool transcript 的 stderr 里
  - 没有进入并行 runner 用于解析的 stdout

### 最强备选解释

- parser 可能无法吃下“多行分块的 stdout 事件”

### 验证

- 反证备选解释:
  - `experiment_runner#3` 的标准多行 stdout event 最终被成功解析为 `bus.publish(topic=experiment.result)`
  - 所以 parser 并不是普遍不能处理多行 stdout event
- 支撑主假设:
  - `experiment_runner#1` 的 event 只出现在 `stdout=false` 片段
  - 同一 run 中没有它的 `bus.publish(topic=experiment.result)`

### 结论

- 已验证结论:
  - 当前“没回流”的直接原因不是 runner 没做事
  - 也不是并行 runner 的 stdout-only parser 本身坏了
  - 而是至少 `experiment_runner#1` 把 workflow event 发到了 stderr/tool transcript,导致 supervisor 根本收不到这条 `experiment.result`
- 设计层结论:
  - 修复点应该放在 prompt / hat 行为约束
  - 不应该把 stderr 重新并回事件解析通道

## [2026-03-21 17:53:19] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 笔记: 新录制里的补充动态证据 - 旧 `ralph emit` 文案仍在 worker prompt 中

## 来源

### 来源1: 正在运行的新录制 stdout/stderr 观察

- 命令:
  - `RALPH_DIAGNOSTICS=1 cargo run --manifest-path Cargo.toml -p ralph-cli --bin ralph -- run -c examples/parallel-experimental-dev-engine/ralph.yml --max-iterations 40 --no-tui --record-session .ralph/parallel-experimental-dev-engine-recording-fixed.jsonl`
- 要点:
  - `ralph#1` 已重新发出两条 `experiment.task`
  - `experiment_runner#1` / `experiment_runner#3` 启动时打印出的 prompt 片段里,同时存在两类说明:
    - 旧说明: “外部事件注入: `ralph emit`”
    - 新说明: “正常 workflow event 发射(关键,不要和 `ralph emit` 混淆)”

### 来源2: 已修改文件

- 文件:
  - `config/all_hat.md`
  - `examples/parallel-experimental-dev-engine/ralph.yml`
- 要点:
  - 这两处都已经补了“workflow event 必须直接走最终 assistant stdout”的新护栏
  - 但从动态 prompt 看,现在不是“只剩新护栏”,而是“新旧两套说明并存”

## 综合发现

### 现象

- 新录制并没有丢掉 fanout 能力
- 但 worker 在真正执行前看到的提示里,仍然包含鼓励使用 `ralph emit` 做外部注入的完整说明

### 当前假设

- 这段旧说明不是来自 example 层的局部文案,而是来自更靠近核心 prompt 组装链路的共享来源
- 所以单改 `examples/parallel-experimental-dev-engine/ralph.yml` 或 `config/all_hat.md` 还不够,最多只能形成“新增一个纠偏规则”

### 当前还缺的证据

- 还没有拿到那段旧说明在源码中的精确拼装位置
- 也还没有拿到本轮新录制最终是否产生 durable `experiment.result` 的结论

### 临时结论

- 这条新证据不会推翻上一轮“stdout-only parser 没问题”的结论
- 它只是在提示:
  - prompt 行为约束目前仍有冲突源
  - 若本轮回流继续异常,下一步修复落点应继续上探到 prompt 组装源头

## [2026-03-21 17:59:06] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 笔记: 新录制继续下潜后的真实断点 - `integration.task` 使用了 `<\\/event>`

## 来源

### 来源1: 新录制 durable 事件流

- 文件:
  - `.e2e-tests/parallel-experimental-dev-engine-recording-fixed-20260321-174345/.ralph/events.jsonl`
- 要点:
  - 当前 durable topic 已到:
    - `experiment_runner#1 -> experiment.result`
    - `experiment_auditor#1 -> experiment.reviewed`
    - `experiment_runner#3 -> experiment.result`
    - `experiment_auditor#1 -> experiment.reviewed`
  - 也就是说,`exp-001` 和 `exp-002` 两条实验链现在都已经成功回流到 coordinator 之前。
  - 但 durable 流里仍然没有 `integration.task` / `integration.applied` / `experiment.complete`。

### 来源2: 新录制 record-session 原始 stdout

- 文件:
  - `.e2e-tests/parallel-experimental-dev-engine-recording-fixed-20260321-174345/.ralph/parallel-experimental-dev-engine-recording-fixed.jsonl`
- 关键 `repr(text)`:
  - `'<event topic="integration.task" reply="ptv4KEHRkaZG">{"run_id":"e2e","objective":"e2e: parallel experimental dev engine","experiment_id":"exp-001","commit":"7ff73f65dc9313718512fdf9d338e6aed8cd85d0","final_verification":"rg -n \\"exp-00[12]\\" . -g \\'e2e_marker_exp_*.txt\\'"}<\\\\/event>\\n'`
- 要点:
  - `ralph#1` 确实从 stdout 发出了 `integration.task`。
  - 但关闭标签不是 `</event>`。
  - 而是字面量 `<\\/event>`。

### 来源3: `crates/ralph-core/src/event_parser.rs`

- 关键代码:
  - `const EVENT_CLOSE_TAG: &str = "</event>";`
  - `let close_idx = content_start.find(EVENT_CLOSE_TAG);`
- 要点:
  - 当前 parser 只查找严格的 `</event>`。
  - 对 `<\\/event>` 没有任何兼容分支。

### 来源4: `examples/parallel-experimental-dev-engine/ralph.yml`

- 关键规则:
  - `integration.task` 明确是 `ralph#1` 进入集成时必须发布的 workflow event。
  - example 里的“发事件格式”展示区仍然偏向“文本示例 + 真实输出还原”这一类说明。
- 要点:
  - 这类示例本身不是直接 bug。
  - 但当 payload 是 JSON 时,模型很容易把结束标签也写成 JSON/HTML 风格的 `<\\/event>`。

## 综合发现

### 现象

- 原问题“没有回流”在 worker 层已经不成立:
  - 两条 `experiment.result`
  - 两条 `experiment.reviewed`
  - 都已 durable 落盘
- 新的 durable 断点发生在 coordinator 准备把批准实验交给 integrator 的那一步。

### 当前主假设

- `ralph#1` 发出的 `integration.task` 因为关闭标签被转义成 `<\\/event>`。
- `EventParser` 只识别 `</event>`。
- 所以 supervisor 没有把这条 stdout 事件解析成真正的 bus event。

### 最强备选解释

- `ralph#1` 可能即使发出了合法 `integration.task`,也会因为 duplicate dispatch / 后续策略漂移导致 integrator 仍未接活。

### 验证

- 支撑主假设的动态证据:
  - record-session 中确实存在 `stdout=true` 的 `integration.task` 原始文本。
  - 该文本的结束标签确实是 `<\\/event>`。
  - 同一时刻 durable `events.jsonl` 中没有对应 `integration.task`。
- 支撑主假设的静态证据:
  - `EventParser` 当前只查找 `</event>` 常量。
- 对备选解释的当前判定:
  - duplicate dispatch 确实是一个新现象。
  - 但在当前时点,它还不足以解释“为什么连第一条 integration.task 都没有 durable 落盘”。
  - 所以它更像后续要继续观察的副作用,不是当前主断点。

### 结论

- 已验证结论:
  - 这次 example 录制里,最初的“worker 不回流”问题已经修住。
  - 现在真正阻断 integration 的,是 coordinator 输出了 `<\\/event>` 风格关闭标签,而 parser 只接受 `</event>`。
- 修复方向:
  - 最稳妥的是给 `EventParser` 增加对 `<\\/event>` 的兼容解析与测试。
  - prompt 侧仍可继续加强“不要转义关闭标签”的约束,但那只能当软护栏,不能替代 parser 容错。
