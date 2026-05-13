## [2026-03-20 16:56:05] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 主题: “共同维护 topic memory”不等于“所有 hat 并发直写同一文件”

### 发现来源
- 探索“角色维度 WORKLOG + 话题维度 WORKLOG__topic”双轴 memory 时,对照了当前 `.agent/memories.md`、`.agent/tasks.jsonl`、六文件体系和 `config/all_hat.md`。

### 核心问题
- 从协作语义看,一个 topic 确实是由多个 hat 共同推进的。
- 但如果把这个语义直接实现成“多个 hat 直接 append 同一个 `WORKLOG__topic.md`”,就会遇到:
  - 双写
  - 漂移
  - 并发冲突
  - 汇总口径失真

### 为什么重要
- 这是双轴 memory 是否会变得可维护的分水岭。
- 方向本身是好的,但如果没有写入层级,最后会得到一套比现在更难管理的日志系统。

### 未来风险
- 一旦 topic 文件同时承担:
  - 原始执行轨迹
  - 阶段性汇总
  - 用户可读结论
  它就会很快失控。

### 当前结论
- 更稳的模型是:
  - 角色轴写 raw append-only log
  - 话题轴写 shared synthesis
- 也就是说:
  - “共同维护”是语义上的共同维护
  - 不一定是所有 hat 都直接改同一个话题文件

### 后续讨论入口
- 如果后面真要做 OpenSpec:
  - 要先定义 topic 文件的 canonical writer 是谁
  - 以及 role log 到 topic synthesis 的收敛规则是什么

## [2026-03-20 22:36:07] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 主题: topic 轴一旦存在,就必须显式定义 canonical writer

### 发现来源
- 在继续 explore `experience.md` 命名和多维度记忆时,对照了:
  - `tasks/context-file-injection.code-task.md`
  - `config/all_hat.md`
  - `crates/ralph-core/src/hatless_ralph.rs`
- 用户同时提出:
  - root 下可能没有 `ralph.yml`
  - `ralph#1` 要能够自行判断用哪套 workflow / 哪个 hat / 是否实时创建 ad-hoc hat

### 核心问题
- 一旦 Ralph 支持:
  - 动态选择 workflow
  - 动态启用单个 hat
  - 动态混合多个 workflow 中的 hat
  topic shared files 就不再天然对应某个静态 `ralph.yml`。
- 这时如果没有 canonical writer 规则,topic 文件会失去“谁来代表当前官方结论”的锚点。

### 为什么重要
- 这不是文档写法细节,而是未来动态 workflow 体系能不能稳定扩展的基础约束。
- 没有这个约束,越动态越容易乱。

### 未来风险
- 任何 hat 都能改 topic 文件,最后 topic 文件会同时混入:
  - 原始证据
  - 中间猜测
  - 阶段总结
  - 过期结论
- LLM 下轮再读时,反而更难判断哪部分才是“现在应该相信的状态”。

### 当前结论
- 推荐把 topic 轴做成“single canonical writer, multi-source evidence”模型:
  - 多 hat 提供证据和局部判断
  - 但只有 canonical writer 负责 shared topic files 的官方落盘
- canonical writer 的推荐优先级:
  1. workflow owner / finalizer
  2. `ralph#1` 临时兜底
- `experience.md` 只接收 topic 关闭后的稳定蒸馏,不直接承接运行时噪音

### 后续讨论入口
- 如果后面进入 OpenSpec:
  - 要把 canonical writer 的选举 / 接管 / 交接规则写成正式设计
  - 要定义哪些 topic close 条件满足后,经验才能晋升到 `experience.md`

## [2026-03-20 22:52:27] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 主题: 一旦有岗位级 `experience.md`,raw log 最好下沉到 instance 级

### 发现来源
- 用户新增要求:
  - `.ralph/roles/<hat_id>` 下也要有基于岗位的 `experience.md`
- 同时结合了此前关于多实例并发写的担忧。

### 核心问题
- 如果 role 目录同时保存:
  - 多实例原始 `WORKLOG`
  - 岗位级稳定 `experience.md`
  它会重新混淆“轨迹”和“规律”。

### 为什么重要
- 这是 role 轴是否会像 topic 轴一样再次出现双写和口径漂移的关键点。
- 越是并行实例多,这个问题越会放大。

### 未来风险
- 同岗位多个实例都在写 role 目录时:
  - raw log 会冲掉经验沉淀的可读性
  - 角色经验也会被一次性任务噪音污染

### 当前结论
- 更稳的分工是:
  - instance 级目录保存原始轨迹
  - role 级目录保存岗位稳定经验
  - topic 级目录保存共享结论
  - project 根 `experience.md` 保存跨角色稳定经验

### 后续讨论入口
- 如果后面进入 OpenSpec:
  - 要补 role canonical writer 规则
  - 要补“role experience 与 project experience 的晋升边界”

## [2026-03-20 22:55:38] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 主题: `experience.md` 体系必须采用“先窄后宽”的晋升纪律

### 发现来源
- 继续讨论岗位级 `experience.md` 后,回读了:
  - `docs/concepts/memories-and-tasks.md`
  - `docs/advanced/memory-system.md`
  - `specs/ralph-memories/design.md`
  - `crates/ralph-core/src/config.rs`
  - `crates/ralph-core/src/event_loop/mod.rs`

### 核心问题
- 一旦 project 根 `experience.md` 会被普遍自动注入,它就不是普通记录文件。
- 它是高影响面的全局知识层。
- 如果把 role 特定经验过早升到 project,污染会非常快。

### 为什么重要
- 这会直接影响后续:
  - `ralph#1` 的 workflow 选择质量
  - 普通 hats 的 prompt 噪音水平
  - 长期经验层是否还能保持可信

### 未来风险
- 如果没有“先窄后宽”纪律:
  - 项目级 experience 会越积越厚
  - 角色特定技巧会被错误注入给无关 hats
  - 以后清理和降级会比晋升更难

### 当前结论
- 推荐晋升顺序:
  - topic
  - role experience
  - project experience
- 只有当经验已被证明:
  - 跨角色成立
  - 或 `ralph#1` 路由前必须知道
  才允许升到项目根 `experience.md`

### 后续讨论入口
- 如果后面进入 OpenSpec:
  - 要把“晋升”和“降级/回收”一起设计
  - 尤其是 project experience 误晋升后的回退机制

## [2026-03-20 23:08:29] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 主题: project 根 `experience.md` 最好默认只让 `ralph#1` 写

### 发现来源
- 在把设计稿继续收敛到 writer / promotion / demotion / injection 这几个骨架时,发现 project experience 和 role/topic 最大的不同,不是格式,而是影响面。

### 核心问题
- project 根 `experience.md` 一旦默认自动注入给所有 agent,它就是系统的最高影响面知识层。
- 如果写入权也和 topic / role 一样分散,全局知识会很快被噪音污染。

### 为什么重要
- 这直接决定:
  - workflow 选择质量
  - prompt 噪音控制
  - 经验系统是否还能长期可信

### 未来风险
- 如果普通 hats 也能随手写 project experience:
  - 局部经验会被误抬成全局规则
  - 以后会出现“全员都带着错误先验开工”的问题

### 当前结论
- 默认应由 `ralph#1` 独占 project 根 `experience.md` 的 canonical writer 权限。
- 其他 hats 最多只能提供:
  - candidate evidence
  - promotion suggestion
  不能直接落笔全局经验。

### 后续讨论入口
- 如果后面进入 OpenSpec:
  - 要明确“candidate evidence 如何提交给 `ralph#1`”
  - 以及 project experience 的审核门槛是否要支持 human gate

## [2026-03-21 01:00:28] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 主题: append-only handoff 与结构化 experience store 不应混在同一文件协议里

### 发现来源
- 在正式实现 role canonical writer handoff 时,对照了:
  - `crates/ralph-core/src/experience_store.rs`
  - `crates/ralph-core/src/experience_parser.rs`
  - `crates/ralph-core/src/experience_governance.rs`

### 核心问题
- 当前 `experience.md` store 的写入模型是:
  - 解析条目
  - 修改内存中的 entry 列表
  - 再整文件重写
- 如果把 handoff summary 直接追加到 `experience.md` 本体:
  - 下一次正常 append / rewrite experience entry 时
  - handoff 段落会被静默覆盖掉

### 为什么重要
- 这不是单个实现疏忽,而是两种文件语义天然冲突:
  - append-only 审计摘要
  - 结构化整文件重写 store
- 如果不分开,writer transfer 的 resumable trail 会反复丢失

### 未来风险
- 如果后续忘了这个边界:
  - 你会以为 handoff 已经落盘并可恢复
  - 但其实一次普通经验写入就会把它擦掉
- 这会直接损坏:
  - writer handoff auditability
  - resume / takeover 的可靠性

## [2026-03-24 12:39:51] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 主题: 验证 completion 尾巴前,必须先证明 scenario 已真正到达 completion 前夜

### 发现来源
- 在对 `parallel-experimental-dev-engine-example` 做真后端复核时,目标本来是验证旧 `job 5` 尾巴是否还出现。
- 但本轮 run 的 durable 事件链只推进到:
  - `experiment.task(exp-001)`
  - `experiment.result(exp-001)`
  - `experiment.reviewed(exp-001)`

### 核心问题
- 如果 scenario 根本没有进入 integration / `experiment.complete` / `LOOP_COMPLETE` 区域,
  任何“completion 后还有没有新 job”的讨论都失去地基。
- 这类时候最容易犯的错是:
  - 因为没看到旧尾巴,就误以为修复已被真后端证实
  - 或者因为 run 卡住,就把所有问题都重新归咎到 completion 语义上

### 为什么重要
- 这是并行 runtime 回归分析里的一个验证纪律:
  - 先确认 workflow 真的跑到了你要观察的边界
  - 再分析边界附近的行为
- 否则不同层级的问题会被混成一个“模糊的大失败”

### 未来风险
- 如果后续忽略这条纪律:
  - 会把“前置 workflow 没走通”和“completion 尾巴回归”混淆
  - 会让修复优先级失焦
  - 还会让真后端证据被过度解读

### 当前结论
- 当前更前置的新现象是:
  - `parallel-experimental-dev-engine-example` 本轮停在 `exp-001 reviewed` 之后
  - `exp-002` 未进入 durable 事件流
- 因此:
  - 这轮 run 不能用于证明或反驳旧 `job 5` 尾巴
  - 下一步应先修或解释这个前置卡点

### 后续讨论入口
- 下次继续时优先看:
  - `notes__memory_axes.md` 最新一条关于 stalled run 的记录
  - `.e2e-tests/parallel-experimental-dev-engine-example/.ralph/events.jsonl`
  - `.e2e-tests/parallel-experimental-dev-engine-example/ralph/log/ralph#1/task_plan.md`

### 当前结论
- topic handoff 追加到 `WORKLOG__topic.md` 是安全的
- role handoff 应该落 sidecar:
  - `.ralph/roles/<hat_id>/handoff.md`
- 类似“append-only 控制面记录”不要和“结构化经验正文”混用同一文件协议

### 后续讨论入口
- 如果未来 project scope 也出现 handoff 需求:
  - 先检查 project experience 是否也会遭遇整文件重写冲突
  - 优先考虑 sidecar 或 canonical writer metadata store,不要先把摘要硬塞进 `experience.md`

## [2026-03-21 12:41:58] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 主题: examples 批量 E2E 进行中时,`report-live.md` 才是当前 run 的事实面板

### 发现来源
- 本轮直接执行了:
  - `cargo run -p ralph-e2e -- codex --filter example --report both --skip-analysis`
- 同时回看了:
  - `.e2e-tests/report-live.md`
  - `.e2e-tests/report.json`
  - `.e2e-tests/report.md`
  - 当前 workspace 下的 `.ralph/events.jsonl` 与 `.ralph/agents.json`

### 核心问题
- 当 examples 批量 E2E 还在运行,或者被中途打断时:
  - `report.json`
  - `report.md`
  可能仍然停在旧 run 的历史快照
- 如果不额外核对 live report 和 workspace 证据,很容易把旧结果误当成本轮结果

### 为什么重要
- 这是验证口径的问题,不是展示细节
- 一旦拿错报告,你会对“本轮到底跑了几条,通过了几条”产生错误结论

### 未来风险
- 中断批量运行后,人会以为:
  - “report.json 里写 1/1 passed,说明这轮也全绿了”
- 但真实情况可能只是:
  - live report 刚到 `1/26`
  - 其余场景还没跑,或者被中断了

### 当前结论
- examples E2E 进行中时,优先信任:
  - `.e2e-tests/report-live.md`
  - 当前 workspace 的 `.ralph/events.jsonl`
  - 当前 workspace 的 `.ralph/agents.json`
- `report.json` / `report.md` 更适合作为“完整 run 结束后的最终快照”
- 另外,中断 `ralph-e2e` 后要主动检查是否遗留 `ralph run` / `codex app-server` 子进程

### 后续讨论入口
- 如果后面要把 examples 全量回归做成更顺手的日常命令:
  - 可以继续评估 report snapshot 的中途刷新策略
  - 以及 `Ctrl-C` 后是否应该自动回收子进程组

## [2026-03-21 22:00:49] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 主题: completion promise 当前只停止“新路由”,并不会自动冻结实例里已排队的 pending

### 发现来源
- 在单独跟踪 `parallel-experimental-dev-engine-example` 旧 `job 5` 尾巴时,同时看了:
  - `ParallelSupervisor` completion drain 逻辑
  - `HatInstanceActor` 的 `pending -> maybe_start_job` 路径
  - 新增的最小动态测试 `supervisor_allows_prequeued_ralph_job_to_start_after_completion_promise`

### 核心问题
- 当前 completion 语义容易被人误以为是:
  - “看到 `LOOP_COMPLETE` 后,系统不会再起任何新 job”
- 但真实语义更窄:
  - Supervisor 只是不再继续路由新的事件
  - 对于 completion 之前已经进入某个 instance `pending` 的工作,并不会自动冻结

### 为什么重要
- 这会制造一种很隐蔽的错觉:
  - 看起来像是“completion 之后又来了新 job”
  - 实际上可能只是“旧事件在 completion 前已经排队好,随后在 drain 窗口内起跑”
- 如果忘了这条边界,后面分析类似 flaky 时很容易把问题误判成:
  - parser 误判
  - completion 后错误路由
  - supervisor 多派发

### 未来风险
- 如果不显式记住这条规律:
  - 你会继续拿 mixed stdout 或 job 计数去猜
  - 却忽略 instance 内部 `pending` 本身就是一个独立状态面
- 这会让 future flaky 的调查成本持续偏高

### 当前结论
- 已验证:
  - prequeued `ralph` job 在 completion 后继续起跑,在当前 runtime 里是可能发生的
- 尚未确认:
  - 历史那次旧 `job 5` 是否就是这条机制直接导致

### 后续讨论入口
- 如果要把 completion 语义收紧成更符合直觉的产品行为:
  - 应优先讨论“只 drain running,冻结 pending”而不是继续只补 scenario 断言

## [2026-03-25 21:09:31] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 主题: example 类 E2E 只隔离 git/worktree 还不够,还必须隔离 workspace 根 `AGENTS.md`

### 发现来源
- 在修复 `parallel-experimental-dev-engine-example` 的 integrator 长尾时,对照了:
  - 旧失败 run 的 `.e2e/stdout.txt`
  - 当前 scenario setup 代码
  - 修复后真后端复跑的 `.ralph/events.jsonl` 与 report

### 核心问题
- E2E workspace 即使已经是隔离 clone:
  - worker 仍然会继承 clone 根目录的 `AGENTS.md`
- 对 example 场景来说,仓库根 `AGENTS.md` 往往服务于“开发本仓库”的重型流程:
  - 六文件
  - 持续学习
  - 文档同步
  - 项目级排障纪律
- 这些并不是 example workflow 本身想测的内容

### 为什么重要
- 如果不隔离这层规则面:
  - 你以为自己在验证 example workflow
  - 实际上在验证“example workflow + 仓库级开发流程”的混合体
- 这会让 E2E 结论失真:
  - 失败时,不容易分清是 example 协议问题,还是 repo 级提示词污染
  - 通过时,耗时也会被额外拉长

### 未来风险
- 以后新增别的 example E2E 时,如果只想到 clone/worktree 隔离,却忘了 `AGENTS.md`:
  - 同类长尾和误漂移还会重复出现
- 尤其是“本来很短的 event 驱动任务”,最容易被仓库级流程拖慢

### 当前结论
- 对 example 类 E2E 的 setup,推荐默认做两层隔离:
  - git/worktree 输入世界隔离
  - workspace 根 `AGENTS.md` 规则面隔离
- 本轮 `parallel-experimental-dev-engine-example` 已用真后端复跑证明:
  - 加上第二层后,scenario 恢复 PASS

### 后续讨论入口
- 后续新增 example E2E 时,优先检查:
  - 是否也需要 workspace 根 `AGENTS.md` override
  - 是否还存在其它 repo-level prompt surface 需要隔离

## [2026-03-31 02:34:16] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 主题: 编译期内嵌 prompt overlay 适合做默认值,不适合做不可覆写常量

### 发现来源
- 在收口 `parallel-experimental-dev-engine-example` 真后端回归时,同时观察了:
  - 编译期内嵌的 `config/all_hat.md`
  - example / E2E 对轻量提示词的需要
  - 最终 PASS 前后的回归测试与 live report

### 核心问题
- 如果 all-hat overlay 只能来自编译期内嵌内容:
  - 默认值和场景特化就被绑死在一起
  - example / E2E / preset 这类低噪音场景只能被迫继承开发型重提示词
- 这不是某个 example 的偶发问题,而是“默认值”和“场景覆写”没有解耦。

### 为什么重要
- 后面用户想做:
  - 无 `PROMPT.md` / 无 `ralph.yml` 的默认工作流
  - presets 首次释放到 `~/.ralph`
  - `ralph#1` 动态挑 workflow / hat / 混合调度
- 这些能力都要求:
  - 默认 prompt 资源可以编译进程序
  - 但运行时又能按场景显式换源

### 未来风险
- 如果继续把 overlay 当成不可覆写常量:
  - 示例场景会越来越慢
  - worker 会吃进过多与当前任务无关的上下文
  - preset / workflow 体系也会被迫通过“改默认资产”这种高耦合方式扩展

### 当前结论
- 更稳的模型是:
  - 编译期内嵌资源负责“默认可用”
  - runtime 配置负责“显式选择来源”
- 这轮已经落地的最小正确形态是:
  - `core.all_hat_prompt.mode = compiled | disabled | inline | file`
- example / E2E 这类场景优先用 `inline` 或 `file` 覆写,而不是直接改默认内嵌资产

### 后续讨论入口
- 如果继续往用户最初的大方案推进:
  - 可以把 presets/workflows 的释放目录与 `core.all_hat_prompt.file` 接起来
  - 再让 `ralph#1` 在运行时决定加载哪套 workflow / 哪个 hat / 是否混用多套 overlay

## [2026-04-02 11:17:41] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 主题: 运行时关系图如果不和静态 topology 图分产品,后续一定会语义混淆

### 发现来源
- 在把 Rerun graph 讨论整理成正式 OpenSpec change 时,同时回看了:
  - `crates/ralph-cli/src/hats.rs`
  - `crates/ralph-core/src/event_logger.rs`
  - `openspec/changes/rerun-runtime-graphs/{proposal.md,design.md,tasks.md}`

### 核心问题
- Ralph 现在已经有一个“图”能力:
  - `ralph hats graph`
- 如果后面再加 Rerun graph,但不明确它是“运行时关系图”,团队很容易把两者混成:
  - 只是两种不同渲染器
  - 或者以为 Rerun graph 会自然替代静态 topology 图
- 同时,如果只把 V1 live graph 写进聊天,而不把 V2 durable replay 一起写进正式 artifact,过一段时间后最容易只剩一个“能看 live 节点动起来”的记忆。

### 为什么重要
- 这不是命名洁癖,而是产品边界问题。
- 一旦边界不清:
  - 文档会混
  - CLI 命名会混
  - 用户会不知道“启动前看哪张图,运行中看哪张图,结束后回放看哪张图”
- 更严重的是:
  - 团队可能把一个 live demo 当成“已经具备完整 replay graph”

### 未来风险
- 如果后续实现时忘了这条边界:
  - 会把 Rerun 当成另一套静态图输出
  - 或者在没有 durable recipient / lifecycle evidence 的前提下,过度宣称 replay fidelity
- 这样做出来的功能表面上很炫,但排障时会迅速失去可信度

### 当前结论
- 最稳的长期口径是:
  - 静态 topology 图 = `ralph hats graph`
  - 运行时动态图 = Rerun runtime graph
  - V1 = live runtime graph
  - V2 = durable replay graph
- 这四条边界必须同时存在于:
  - proposal
  - design
  - tasks
  - spec requirement

### 后续讨论入口
- 下次进入实现前,优先先看:
  - `openspec/changes/rerun-runtime-graphs/design.md`
  - `openspec/changes/rerun-runtime-graphs/specs/runtime-graph-observability/spec.md`
- 然后按顺序推进:
  - 先 V1 live
  - 后 V2 durable replay

## [2026-04-03 01:11:04] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 主题: V1 runtime graph 的 recipient 边不能只靠 durable log 猜

### 发现来源
- 在实现 `rerun-runtime-graphs` 的 V1 live runtime graph MVP 时,对照了:
  - `crates/ralph-core/src/parallel/supervisor.rs`
  - `crates/ralph-core/src/parallel/supervisor/routing.rs`
  - `.ralph/events.jsonl` 当前稳定字段
- 同时用最小 `.rrd` smoke run 验证了 live graph artifact 的真实产出。

### 核心问题
- 当前 durable 证据仍然不完整:
  - 不能稳定给出最终 `target_instance`
  - 不能完整给出 fanout recipients
  - 不能给出 create / spawn lineage 与 lifecycle control durable edges
- 如果 V1 只靠 `.ralph/events.jsonl` 或已有 durable artifact 去“推断 recipient 边”,图会很像对,但并不可信。

### 为什么重要
- 这决定了 V1 和 V2 的边界是否会再次混掉。
- 也是未来 replay graph 是否会被误宣传成“已经完整可审计”的关键分水岭。

### 未来风险
- 如果后面有人忘了这条边界:
  - 很容易把 live graph 的 best-effort 关系当成 durable truth
  - 也很容易在 V2 还没做完时,误以为 replay graph 已经具备 full-fidelity

### 当前结论
- V1 live graph 应该明确依赖最小 live `delivery_observer`,而不是拿 durable log 盲猜 recipient。
- V2 durable replay graph 则必须单独补:
  - `target_instance`
  - fanout recipients
  - create / spawn lineage
  - freeze / cancel / shutdown control edges

### 后续讨论入口
- 如果下一轮继续 `rerun-runtime-graphs`,应当直接从:
  - `3.1`
  - `3.2`
  - `3.3`
  - `3.4`
 继续,不要再回头重做 V1 入口
