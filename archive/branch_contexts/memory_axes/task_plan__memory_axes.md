# 任务计划: 记忆系统双轴拆分探索

## 目标

评估 Ralph 现有 memories / tasks / 六文件体系,判断是否应该把“角色维度记忆”和“项目话题维度记忆”拆成两套并行机制,并收敛出推荐方向与边界。

## 阶段

- [x] 阶段1: 回读活跃六文件、OpenSpec 状态与现有 memories/tasks 入口
- [x] 阶段2: 阅读当前 memories / tasks / scratchpad / context-file 设计,确认现有状态模型
- [x] 阶段3: 对照用户提出的“双轴 memory”设想,分析收益、冲突和落点
- [x] 阶段4: 形成推荐分层图,必要时给出适合落 OpenSpec 的 change 方向

## 关键问题

1. 现有 `.agent/memories.md` 和 `.agent/tasks.jsonl` 分别承担什么职责,与六文件体系有没有重叠?
2. “每个 hat 一个 WORKLOG”与“每个话题一个 WORKLOG__xxx”是互补关系,还是会制造双写和漂移?
3. 如果真的做双轴 memory,哪一层应该给机器自动注入,哪一层只做人类/协作外部记忆?

## 做出的决定

- 这轮只做 explore,不写实现代码。
- 使用支线六文件 `__memory_axes` 记录,避免污染主线 OpenSpec 记录。
- 优先先看现有 memory/task/context 设计,再评价新方案,不先入为主。

## 当前状态

**全部完成** - 已确认当前状态模型,并形成推荐分层:

- `tasks.jsonl` = runtime work graph
- `.ralph/roles/<hat_id>/WORKLOG.md` = 角色维度 raw log
- `WORKLOG__topic.md` / `notes__topic.md` / `task_plan__topic.md` = 话题维度 shared context
- `.agent/memories.md` = distilled long-term memory

关键提醒:

- 话题维度不建议让多个 hat 直接并发写同一个文件
- 更稳的是“角色轴先写 raw log,话题轴再做汇总收敛”

## [2026-03-20 22:36:07] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] [记录类型]: 继续 explore - canonical writer 规则与 `experience.md` 命名

- [x] 阶段1: 回读支线六文件,确认上一轮关于双轴 memory 的结论没有丢失
- [x] 阶段2: 回读 `tasks/context-file-injection.code-task.md`、`config/all_hat.md`、`crates/ralph-core/src/hatless_ralph.rs`,确认现有 context / memory 注入边界
- [x] 阶段3: 区分“当前实现仍是 `.agent/memories.md`”和“目标命名改为 `experience.md`”这两层语义
- [x] 阶段4: 收敛 topic shared file 的 canonical writer 推荐规则,以及 topic -> `experience.md` 的晋升条件

- 当前目标:
  - 用户已明确“长期可复用的叫 `experience.md`”。
  - 用户选择先继续路径1,也就是先把 canonical writer 机制讨论清楚,而不是立刻转 OpenSpec change。

- 当前决定:
  - 继续保持 explore 模式,不写实现代码。
  - 讨论口径里,把“长期可复用记忆”统一称为 `experience.md`。
  - 但必须明确标注: 当前代码实现仍然绑定 `.agent/memories.md`,这只是现状,不是本轮推荐命名。

- 当前状态:
  - **全部完成**: 已形成一版更稳的收敛:
    - role 轴负责 raw log
    - topic 轴必须有 canonical writer
    - `experience.md` 只接收 topic 关闭后的稳定经验蒸馏

## [2026-03-20 22:52:27] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] [记录类型]: 继续 explore - 增加岗位级 `.ralph/roles/<hat_id>/experience.md`

- [x] 阶段1: 重新审视“role 轴 = raw log”这个旧结论是否还成立
- [x] 阶段2: 把“岗位级稳定经验”与“实例级原始轨迹”拆开
- [x] 阶段3: 收敛新的层级结构与推荐注入顺序
- [x] 阶段4: 记录 v3 口径,避免后面只记得旧版 role/worklog 模型

- 当前目标:
  - 用户新增要求: `.ralph/roles/<hat_id>` 下也要有基于岗位的 `experience.md`。

- 当前决定:
  - 采纳岗位级 `experience.md`。
  - 同时修正上一版口径: role 目录更适合承载“稳定角色知识”,不再把它当作 raw log 的首选落点。

- 当前状态:
  - **全部完成**: 结构已升级为:
    - instance 级原始轨迹
    - role 级稳定经验
    - topic 级共享结论
    - project 级稳定经验

## [2026-03-20 22:55:38] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] [记录类型]: 继续 explore - 经验晋升规则与现状/文档漂移核对

- [x] 阶段1: 对照现有 memories 文档与代码,确认 `experience.md` 改造的真实起点
- [x] 阶段2: 收敛 topic -> role experience 的晋升边界
- [x] 阶段3: 收敛 role experience -> project experience 的晋升边界
- [x] 阶段4: 形成一版更接近 OpenSpec 设计稿的默认规则

- 当前目标:
  - 继续定义:
    - 哪些结论从 topic 升到 role experience
    - 哪些再从 role experience 升到 project experience

- 当前决定:
  - 明确记录一个事实:
    - 文档里已经出现 `memories.path`
    - 但当前代码实现仍然固定读取 `.agent/memories.md`
  - 经验晋升默认采取保守策略:
    - 能留在 topic 的先留在 topic
    - 能只升到 role 的,先不要直接升到 project

- 当前状态:
  - **全部完成**: 已补齐:
    - 经验晋升漏斗
    - role/project 两级 experience 的边界
    - 文档口径先于实现的漂移提醒

## [2026-03-20 23:08:29] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] [记录类型]: 继续 explore - 准设计稿收敛

- [x] 阶段1: 收敛 canonical writer 的选举优先级
- [x] 阶段2: 收敛 canonical writer 的交接规则
- [x] 阶段3: 收敛 experience 的晋升与降级规则
- [x] 阶段4: 收敛默认注入顺序与按需读取策略

- 当前目标:
  - 把现有探索结果整理成更接近 OpenSpec 的准设计稿。

- 当前决定:
  - project 级 `experience.md` 的写入权要比 role/topic 更严格。
  - 默认由 `ralph#1` 作为 project experience 的 canonical writer。

- 当前状态:
  - **全部完成**: 已形成一版可直接转 OpenSpec 的结构化口径:
    - topic writer
    - role writer
    - project writer
    - promotion / demotion
    - injection / read policy

## [2026-03-20 23:58:51] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] [记录类型]: 继续推进 - explore 结果已转成正式 OpenSpec change

- [x] 阶段1: 创建 `scoped-experience-system` change
- [x] 阶段2: 生成并填写 `proposal.md`
- [x] 阶段3: 生成并填写 `design.md` 与 4 个 capability specs
- [x] 阶段4: 生成 `tasks.md` 并完成 OpenSpec 校验

- 当前目标:
  - 把 memory / experience 体系从探索结论推进到正式可执行规格。

- 当前决定:
  - 新 change 名称定为 `scoped-experience-system`。
  - 用 4 个 capability 拆开骨架:
    - `experience-scopes`
    - `canonical-writer`
    - `experience-promotion`
    - `experience-injection`

- 当前状态:
  - **全部完成**:
    - `openspec status --change "scoped-experience-system"` 显示 4/4 artifacts complete
    - `openspec validate scoped-experience-system --type change` 通过

## [2026-03-21 00:40:47] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] [记录类型]: 继续 apply - 完成 scoped experience 注入链路首批实现

- [x] 阶段1: 回读 OpenSpec `proposal/design/specs/tasks`,确认当前应从 apply 而不是 explore 继续
- [x] 阶段2: 补齐 `ralph-core` 的 scoped experience prompt injection 实现
- [x] 阶段3: 为普通 hat / Ralph / legacy compatibility / topic summary / instance summary 增加回归测试
- [x] 阶段4: 跑完 `cargo test -p ralph-core --lib` 与 `cargo test -p ralph-core smoke_runner`

- 当前目标:
  - 把前一轮只落了“路径与 entry 协议”的基座,继续推进到真正进入 prompt 的注入链路。
  - 让 Ralph 在 workflow / hat 选择前先吃到 project 级经验,普通 hat 则按 project -> role -> topic -> instance -> runtime 顺序拿上下文。

- 当前决定:
  - 本轮先完成 injection/read policy,暂不提前跳去 canonical writer enforcement。
  - topic summary 的 eager 注入先采取保守策略:
    - 只有工作区里能唯一识别出一个 topic 文件组时才注入
    - 多 topic 并存时直接跳过,避免误注入历史话题
  - legacy `.agent/memories.md` 继续保留为 compatibility layer,但在 prompt 中显式标记为 `Legacy Memories (Compatibility)`。

- 当前状态:
  - **已完成并验证**:
    - OpenSpec tasks:
      - `1.1`
      - `1.2`
      - `1.3`
      - `2.1`
      - `2.2`
      - `2.3`
      - `5.1`
      - `5.2`
      - `5.3`
      - `5.4`
      - `6.1`
      - `6.2`
    - 验证结果:
      - `cargo test -p ralph-core --lib` -> 472 passed
      - `cargo test -p ralph-core smoke_runner` -> 12 passed
  - **下一批待接**:
    - `3.1` ~ `3.4` canonical writer enforcement
    - `4.1` ~ `4.4` promotion / demotion flow
    - `6.3` / `6.4`

## [2026-03-21 00:43:35] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] [记录类型]: 继续 apply - writer 约束与 promotion/demotion 主体实现

- [x] 阶段1: 回读支线六文件、OpenSpec apply 指令与剩余 tasks,确认当前应继续实现而不是回到 explore
- [ ] 阶段2: 回读 canonical-writer / experience-promotion specs 与现有 `experience_*` / `event_loop` 代码,确定最合适的落点
- [ ] 阶段3: 先清理误留调试输出,再实现 canonical writer enforcement 与 handoff summary
- [ ] 阶段4: 实现 promotion / demotion 评估与审计链路,补齐测试和必要的 debug 可见性
- [ ] 阶段5: 运行格式化、单元测试、smoke test、OpenSpec validate,并更新 tasks / 支线记录

- 当前目标:
  - 沿着 `scoped-experience-system` 已批准规格继续落地后半段。
  - 本轮优先完成:
    - `3.1` ~ `3.4`
    - 尽可能连带推进 `4.1` ~ `4.4`
    - 视落点顺手补 `6.3`

- 当前决定:
  - 先删掉误留的 `println!` 调试语句,保持测试基线干净。
  - canonical writer 不做散落的 if/else,优先抽成共享策略/授权层。
  - promotion / demotion 要保留审计链路,避免物理删除导致经验历史断裂。

- 当前状态:
  - **进行中** - 已确认 change 仍处于 apply ready,剩余 10 个任务未完成。
  - 下一步先回读:
    - `specs/canonical-writer/spec.md`
    - `specs/experience-promotion/spec.md`
    - `crates/ralph-core/src/experience*.rs`

## [2026-04-03 00:58:58] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] [记录类型]: 继续 apply - `rerun-runtime-graphs` V1 live runtime graph MVP

- [x] 阶段1: 回读 `__memory_axes` 支线记录、`rerun-runtime-graphs` 的 proposal / design / spec / tasks,确认本轮继续的是 V1 live runtime graph,不是 V2 durable replay
- [ ] 阶段2: 审核当前未验证 patch,重点检查 `main.rs`、`parallel_runner.rs`、`runtime_graph.rs`、`supervisor.rs`、`routing.rs` 的接线是否闭合
- [ ] 阶段3: 运行 `cargo fmt`、定向单测和 `cargo build`,把第一轮真实编译错误收敛掉
- [ ] 阶段4: 补齐最小 CLI 验证,确认并行 run 能产出非空 `.rrd` artifact
- [ ] 阶段5: 按验证结果更新 OpenSpec `tasks.md`、支线 `notes__memory_axes.md` / `WORKLOG__memory_axes.md`,必要时补充新的风险记录

- 当前目标:
  - 沿着已批准的 `rerun-runtime-graphs` change,先把 V1 live runtime graph MVP 接通。
  - 这轮先追求“真实可录制、真实可验证”,不抢做 V2 durable replay。

- 当前决定:
  - 继续使用 `__memory_axes` 这套支线文件,不切回默认六文件。
  - 保持 `ralph hats graph` 和 runtime graph 的产品边界清晰:
    - 前者是静态 topology
    - 后者是运行时关系图
  - 先输出 Rerun `.rrd` artifact,暂不把 viewer / TUI 集成混进这一轮。
  - 项目根当前不存在 `EXPERIENCE.md` / `experience.md`,本轮按“无历史经验文件”处理,不强行补空文件。

- 当前状态:
  - **进行中** - OpenSpec apply 指令显示 `15/15` 仍未勾选,但 proposal / design / spec 已经把 V1 / V2 边界写清楚。
  - 下一步:
    - 先审当前 patch diff
    - 然后立即做第一次编译验证

## [2026-04-03 01:11:04] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] [记录类型]: 收口 apply - `rerun-runtime-graphs` V1 live runtime graph MVP

- [x] 阶段1: 回读 `__memory_axes` 支线记录、OpenSpec 上下文与当前 patch,确认只推进 V1 live runtime graph
- [x] 阶段2: 审核并修正 `main.rs`、`parallel_runner.rs`、`runtime_graph.rs`、`supervisor.rs`、`routing.rs` 的接线
- [x] 阶段3: 运行 `cargo fmt`、定向测试、`cargo build` 与整仓 `cargo test`,把编译和回归验证打通
- [x] 阶段4: 执行最小 CLI smoke,确认并行 run 能生成非空 `.rrd` artifact
- [x] 阶段5: 更新 OpenSpec `tasks.md`、proposal / design,并把证据回写到支线 notes / WORKLOG / EPIPHANY

- 当前目标:
  - 完成 `rerun-runtime-graphs` 的 V1 live runtime graph MVP 收口。
  - 把“实现完成到哪里”和“V2 还差什么”同时落盘。

- 当前结论:
  - 当前已完成:
    - OpenSpec tasks `1.1` ~ `1.3`
    - OpenSpec tasks `2.1` ~ `2.5`
    - OpenSpec tasks `4.1` ~ `4.3`
  - 当前剩余:
    - `3.1`
    - `3.2`
    - `3.3`
    - `3.4`
  - `openspec instructions apply --change 'rerun-runtime-graphs' --json` 当前显示:
    - `complete = 11`
    - `remaining = 4`

- 已验证:
  - `cargo test -p ralph-core supervisor`
  - `cargo test -p ralph-cli runtime_graph`
  - `cargo test -p ralph-cli --test integration_runtime_graph`
  - `cargo build`
  - `cargo test -p ralph-core smoke_runner`
  - `cargo test`
  - 最小手工 smoke:
    - `EXIT_CODE=0`
    - `RUNTIME_RRD_BYTES=55951`
    - `Termination reason = CompletionPromise`

- 当前状态:
  - **本轮完成** - V1 live runtime graph MVP 已有:
    - CLI 入口
    - `.rrd` artifact
    - live observer 接线
    - integration test
  - 下一步如果继续,直接进入 V2 durable replay:
    - `target_instance`
    - fanout recipients
    - create / spawn lineage
    - lifecycle control durable evidence

## [2026-04-08 08:22:28] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] [记录类型]: 状态核对 - `rerun-runtime-graphs` 还剩哪些执行项

- [x] 阶段1: 回读 `__memory_axes` 支线记录,确认上轮 V1 live runtime graph MVP 已正式收口
- [x] 阶段2: 重新读取 `openspec status --change 'rerun-runtime-graphs' --json`
- [x] 阶段3: 重新读取 `openspec instructions apply --change 'rerun-runtime-graphs' --json` 与 `tasks.md`
- [x] 阶段4: 输出当前剩余执行项与推荐下一步

- 当前目标:
  - 给出 `rerun-runtime-graphs` 截至今天的真实剩余执行项,避免只依赖旧会话记忆。

- 当前结论:
  - 当前 OpenSpec 进度:
    - `total = 15`
    - `complete = 11`
    - `remaining = 4`
  - 已完成:
    - `1.1` ~ `1.3`
    - `2.1` ~ `2.5`
    - `4.1` ~ `4.3`
  - 剩余全部属于 V2 durable replay graph:
    - `3.1` 盘点 durable 证据缺口
    - `3.2` 设计 delivery-level durable records
    - `3.3` 设计 create/spawn 与 lifecycle control durable lineage
    - `3.4` 定义 replay graph 重建顺序 / 时间轴 / 过滤语义

- 当前状态:
  - **本轮完成** - 当前没有新的 V1 执行项。
  - 如果继续实现,应直接进入 V2 durable replay,不要再回头扩写 V1。

## [2026-03-25 20:58:44] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] [记录类型]: 继续排查 `parallel-experimental-dev-engine-example` integrator 尾巴

- [x] 阶段1: 回读 `__memory_axes` 支线上下文与上一轮真实复跑摘要,确认当前断点已经从 `experiment.result` 前移到 integrator 收尾
- [x] 阶段2: 只围绕 `experiment_integrator#1` 的最终输出做最小证伪,区分“有 escaped 事件未解析”与“根本没有最终事件输出”
- [x] 阶段3: 如果确认输出存在但未入流,收窄到 `output_for_parsing` / parser 归一化问题; 如果确认没有输出,继续追工具完成后为何未进入阶段4
- [x] 阶段4: 针对已验证根因做单点修复,补测试,再复跑 `parallel-experimental-dev-engine-example`
- [x] 阶段5: 回写 `notes__memory_axes.md` / `WORKLOG__memory_axes.md` / `ERRORFIX__memory_axes.md` 的本轮证据与结论

- 当前目标:
  - 不再泛泛看整条场景,而是单独跟踪 `experiment_integrator#1` 在旧失败录制里的最后一小段。
  - 弄清楚为什么 `git cherry-pick` 和 `rg` 都做完了,却没有 durable 的 `integration.applied` / `experiment.complete`。

- 当前决定:
  - 继续使用 `__memory_axes` 支线六文件,不切回默认六文件。
  - 严格按“现象 -> 假设 -> 验证 -> 结论”推进,没有动态证据前不把任何怀疑写成根因。
  - 先做最小证伪,避免又一次把 parser、prompt、worktree 等多个方向混在一起一起改。

- 当前状态:
  - **已完成** - 已完成单点修复与真后端复跑。
  - 结果:
    - `cargo test -p ralph-e2e scenarios::parallel_experimental_dev_engine_example::tests::seeded_workspace_snapshot_commit_makes_patched_prompt_visible_to_worktree -- --exact` 通过
    - `cargo test -p ralph-e2e scenarios::parallel_experimental_dev_engine_example::tests::example_config_requires_structured_commit_fields_for_review_and_integration -- --exact` 通过
    - `cargo run -p ralph-e2e -- codex --filter parallel-experimental-dev-engine-example --keep-workspace --verbose` 通过
  - 已验证当前闭环:
    - `integration.applied`
    - `experiment.complete`
    - `LOOP_COMPLETE`

## [2026-03-25 20:20:47] [Session ID: 8E08D4FA-9BA2-4C21-BDA5-DBB280CCE00F] [记录类型]: 继续追 `parallel-experimental-dev-engine-example` - 回滚旧假设并转向干净 seed 闭环证据

- [x] 阶段1: 回读 `__memory_axes` 支线六文件,确认当前活跃任务已从 scoped experience 实现切到 example 录制取证
- [x] 阶段2: 把本轮新增证据补写到 `notes__memory_axes.md`,明确区分“现象 / 候选假设 / 已验证结论”
- [ ] 阶段3: 更新 `WORKLOG__memory_axes.md`,记录这轮对旧 stalled run、当前 timeout run 与修复动作的证据收敛
- [x] 阶段4: 基于干净/真实场景样本核对 `experiment.reviewed -> integration.task -> integration.applied` 链路,并识别新的前置断点
- [ ] 阶段5: 视结果决定是否追加 `EPIPHANY_LOG__memory_axes.md` 或清理对应 `LATER_PLANS__memory_axes.md`
- [ ] 阶段6: 回读 event parser / parallel stdout 解析代码,实现并验证 HTML 转义 event 的兼容解析

- 当前目标:
  - 不再把“旧 run 缺 `exp-002`”直接当成当前稳定根因。

## [2026-03-31 02:07:03] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] [记录类型]: 继续 apply - 为 example/E2E 引入可配置 all-hat overlay 降噪

- [x] 阶段1: 回读 `__memory_axes` 支线文件与上轮真后端 PASS 证据,确认本轮目标是“进一步降噪/提速”,不是重修闭环
- [x] 阶段2: 静态核对 `prompt_overlay` / `config` / `event_loop` / `parallel supervisor` / example scenario 的 all-hat 注入路径
- [ ] 阶段3: 在配置层增加显式 all-hat overlay 来源/模式,保持默认行为不变
- [ ] 阶段4: 让 `parallel-experimental-dev-engine-example` 在隔离 workspace 显式选择轻量 overlay,并补场景回归测试
- [ ] 阶段5: 跑定向单测、场景测试与真后端复跑,确认闭环仍过且降噪路径可用
- [ ] 阶段6: 回写 `notes__memory_axes.md` / `WORKLOG__memory_axes.md` / `ERRORFIX__memory_axes.md`,并清理对应延后项

- 当前目标:
  - 不直接改轻 `config/all_hat.md`,避免影响整个项目默认行为。
  - 优先做“显式配置 + example 局部启用”的方案,把降噪能力放到 runtime 配置层。

- 当前决定:
  - 候选主方案: 在 `core` 下增加 all-hat overlay 配置源,默认继续使用编译期内嵌版本。
  - example/E2E 只在 patched workspace 的 `ralph.yml` 里切换到轻量 overlay,不碰仓库真实 example 默认体验。
  - 在没有动态证据前,不把 `config/all_hat.md` 直接定性为“唯一根因”; 本轮目标是验证它是否仍是 worker 长尾的重要噪音源。

- 当前状态:
  - **进行中** - 静态路径已确认,下一步开始实现配置与 example 场景补丁。

## [2026-03-31 02:03:55] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] [记录类型]: 继续推进 example/E2E 的 `config/all_hat.md` 降噪

- [x] 阶段1: 回读 `__memory_axes` 支线六文件,确认上轮已通过“workspace 根 AGENTS override”把 example 场景拉回 PASS
- [ ] 阶段2: 回读 `config/all_hat.md` 注入链、prompt overlay 代码和 E2E/example 配置,定位最稳的降噪开关落点
- [ ] 阶段3: 做最小实现,让 example/E2E 能选择轻量 all-hat prompt 或显式关闭开发型 all-hat 注入
- [ ] 阶段4: 补测试并复跑 `parallel-experimental-dev-engine-example`,比较闭环稳定性与耗时
- [ ] 阶段5: 回写 `notes__memory_axes.md` / `WORKLOG__memory_axes.md` / 必要的 `ERRORFIX__memory_axes.md`

- 当前目标:
  - 沿着上轮已经验证过的方向继续收窄 example/E2E 的提示词噪音面。
  - 重点不是再修回流协议,而是减少 `config/all_hat.md` 把 worker 带进开发型流程的副作用。

- 当前决定:
  - 继续只使用 `__memory_axes` 支线六文件。
  - 先拿静态代码路径和现有场景配置做最小证伪,再决定是做“轻量 overlay”还是“显式关闭开关”。
  - 没有动态证据前,不把 `config/all_hat.md` 直接写成唯一根因。

- 当前状态:
  - **进行中** - 已确认本轮是对上轮修复后的进一步降噪,不是重新打开旧 parser/worktree 断点。
  - 下一步先读:
    - `config/all_hat.md`
    - `crates/ralph-core/src/prompt_overlay*.rs`
    - `crates/ralph-core/src/config.rs`
    - `crates/ralph-e2e/src/scenarios/parallel_experimental_dev_engine_example.rs`
  - 先把旧样本、污染样本、干净 seed 样本三者的证据边界写清楚。
  - 然后继续追当前最重要的闭环事实:
    - `experiment.reviewed`
    - `integration.task`
    - `integration.applied`

- 当前决定:
  - 继续只使用 `__memory_axes` 支线上下文。
  - 对旧 stalled run 只保留“真实发生过 durable 断层”的事实。
  - 对“当前代码仍稳定缺 `exp-002`”这条旧主假设,先降级回候选假设失效,除非新证据再次复现。
  - 优先相信干净 seed `/tmp/exp002-prehead-seed.9Pffup` 上的新动态证据,不再用带污染的 workspace 样本推产品结论。

- 当前状态:
  - **进行中** - 已完成六文件回读与两轮 `notes__memory_axes.md` 补写。
  - 下一步:
    - 回读 stdout/event parser 代码,确认 `&lt;event ...&gt;` 为什么没有进入 durable 主流
    - 做最小修复并复跑 `parallel-experimental-dev-engine-example`

## [2026-03-24 12:29:04] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] [记录类型]: 继续验证 - completion freeze 方案做真后端复核

- [x] 阶段1: 回读 `__memory_axes` 支线六文件与当前代码现场,确认这轮不是重新猜根因
- [ ] 阶段2: 把“方案 2 = completion 后冻结 pending,只 drain running”写回支线 notes / worklog / epiphany / later plans
- [ ] 阶段3: 跑 `parallel-experimental-dev-engine-example` 真后端 E2E,检查旧 `job 5` 尾巴是否还会出现
- [ ] 阶段4: 用 record / stdout / events / report 交叉核对,给出“现象 / 验证 / 结论”口径

- 当前目标:
  - 不停留在单测和全仓测试。
  - 继续用真实 `codex` backend 复核:

## [2026-03-25 20:40:48] [Session ID: 0537D10D-AB29-46A7-B336-BC309E3EC274] [记录类型]: 继续 apply - 为 HTML 转义 `<event>` 补 durable parser 兼容

- [x] 阶段1: 回读 `__memory_axes` 支线计划与笔记,确认当前活跃断点已经收敛到 `&lt;event ...&gt;` 未被 durable parser 吃进
- [x] 阶段2: 回读 `crates/ralph-core/src/event_parser.rs` 与现有测试,确认当前 parser 只识别原始 `<event ...>`
- [ ] 阶段3: 先补最小回归测试,锁死 `&lt;event topic="experiment.result"&gt;...&lt;/event&gt;` 的解析缺口
- [ ] 阶段4: 以“有限解码 tag,不做全量 HTML unescape”为原则实现 parser 兼容,并检查 `parse/contains_promise/strip_event_tags` 相关语义
- [ ] 阶段5: 跑最小单测、相关 `ralph-core` 测试,再复跑 `parallel-experimental-dev-engine-example` 真后端 E2E
- [ ] 阶段6: 根据验证结果补写 `WORKLOG__memory_axes.md` 与必要的 `ERRORFIX__memory_axes.md` / `EPIPHANY_LOG__memory_axes.md`

- 当前目标:
  - 先把“为什么 `experiment_runner#3` 明明输出了 `experiment.result` 却没进入 durable 主流”修成一个可被测试复现、可被验证修复的具体问题。
  - 避免直接针对场景补丁,优先在通用 parser 层做有限兼容。

- 当前决定:
  - 当前会话使用 `0537D10D-AB29-46A7-B336-BC309E3EC274` 作为支线记录的 session id。
  - 主假设仍然是 parser 未兼容 HTML 转义 tag。
  - 最强备选解释仍保留:
    - 某条 Codex 输出链会把协议事件 HTML 转义
    - parser 只是在 durable 层没有容错这个变体
  - 先补测试和 parser,不先改 scenario 断言或上游 prompt。

- 当前状态:
  - **进行中** - 已完成静态确认:
    - `find_event_start()` 只找原始 `<event`
    - opening tag 结束也只找原始 `>`
    - closing tag 只兼容 `</event>` 与 `<\\/event>`
  - 下一步先写失败测试,把 `&lt;event ...&gt;` 的缺口锁死。
    - `parallel-experimental-dev-engine-example`
    - 重点看 completion 后是否还会起新的尾巴 job

- 当前决定:
  - 继续沿用 `__memory_axes` 支线文件,不切回默认六文件。
  - 先补记录,再跑真场景,避免后面只记得“测过”,忘了“测的是什么”。
  - 这轮输出必须区分:
    - 已观察到的现象
    - 候选机制
    - 真正被动态证据支撑的结论

- 当前状态:
  - **进行中** - 已确认代码改动和定向测试都在仓库里,但“真后端是否也消灭旧 `job 5` 尾巴”仍待补证。

## [2026-03-24 12:39:33] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] [记录类型]: 真实复核阶段更新 - 本轮 run 停在 completion 之前

- [x] 阶段1: 回读 `__memory_axes` 支线六文件与当前代码现场,确认这轮不是重新猜根因
- [x] 阶段2: 把“方案 2 = completion 后冻结 pending,只 drain running”写回支线 notes / worklog / epiphany / later plans
- [x] 阶段3: 跑 `parallel-experimental-dev-engine-example` 真后端 E2E,检查旧 `job 5` 尾巴是否还会出现
- [ ] 阶段4: 用 record / stdout / events / report 交叉核对,给出“现象 / 验证 / 结论”口径

- 当前观察:
  - 本轮 `cargo run -p ralph-e2e -- codex --filter parallel-experimental-dev-engine-example --keep-workspace --verbose` 已真实启动
  - 动态事件链只推进到:
    - `experiment.task(exp-001)`
    - `experiment.result(exp-001)`
    - `experiment.reviewed(exp-001, evidence_ok=true)`
  - `exp-002` 未出现在主 `.ralph/events.jsonl`
  - 因此 run 没有进入 integration / completion 区域,更没有走到旧 `job 5` 尾巴断言

- 当前决定:
  - 已中断这次 stalled run,避免空等 1800 秒超时
  - 后续结论必须明确写成:
    - 这轮没有复现旧 `job 5`
    - 但也没有获得“completion 后无新 job”的真后端新证据

- 当前状态:
  - **进行中** - 现在转入证据整理与支线记录收尾,并把“前置卡点是 exp-002 未继续派发”作为新的现象单独记录。

## [2026-03-24 12:40:59] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] [记录类型]: 本轮收口 - 已完成 stalled run 复核与证据归档

- [x] 阶段1: 回读 `__memory_axes` 支线六文件与当前代码现场,确认这轮不是重新猜根因
- [x] 阶段2: 把“方案 2 = completion 后冻结 pending,只 drain running”写回支线 notes / worklog / epiphany / later plans
- [x] 阶段3: 跑 `parallel-experimental-dev-engine-example` 真后端 E2E,检查旧 `job 5` 尾巴是否还会出现
- [x] 阶段4: 用 record / stdout / events / report 交叉核对,给出“现象 / 验证 / 结论”口径

- 最终结论:
  - 本轮没有拿到“旧 `job 5` 尾巴再次出现”的证据
  - 但也没有拿到“completion freeze 已在真后端 example 中被补证”的证据
  - 原因是 run 更早停在 `exp-001 reviewed` 之后,没有到 completion

- 当前状态:
  - **本轮已完成** - 证据与后续计划均已写回支线文件,下一步应优先追 `exp-002` 未 durable 派发的问题。

## [2026-03-25 19:59:56] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] [记录类型]: 继续调试 - 先缩小 `exp-002` 未 durable 派发的最小证据面

- [ ] 阶段1: 用最小实验抓 `ralph#1` 首轮输出,确认它到底发了 1 条还是 2 条 `experiment.task`
- [ ] 阶段2: 如果首轮只发了 1 条,再判断是 prompt 约束不够硬,还是 coordinator 运行时处理有偏差
- [ ] 阶段3: 如果首轮其实发了 2 条,继续查第二条为什么没有进入 durable 事件流
- [ ] 阶段4: 根据证据决定是先修 example prompt/scenario,还是先修 runtime / parser / harness

- 当前目标:
  - 继续沿着昨天的 stalled run 往前缩小问题。
  - 不再直接拿大场景 completion 作为入口,而是先抓 coordinator 第一轮的实际输出事实。

- 当前决定:
  - 本轮优先做最小可证伪实验:
    - 在已有 workspace 里直接跑 `ralph run`
    - 加 `--record-session`
    - 把 `--max-iterations` 限到只观察首轮 coordinator 输出
  - 只有拿到“首轮到底发了几条 task”的证据后,才决定下一步改哪里。

- 当前状态:
  - **进行中** - 已回接支线记录,下一步开始读取 systematic-debugging skill 与首轮实验入口。

## [2026-03-21 18:23:54] [Session ID: 68546] [记录类型]: 继续排查 `parallel-experimental-dev-engine-example` 录制无回流与收尾尾巴

- [x] 阶段1: 回读 `__memory_axes` 支线有效上下文与上一轮已验证结论
- [ ] 阶段2: 确认真实验证命令的最终状态,特别是遗留的 `cargo test` 是否干净收尾
- [ ] 阶段3: 重新核对录制证据,把“原无回流问题”和“LOOP_COMPLETE 后新 job”拆成两个现象
- [ ] 阶段4: 如有必要继续缩小 `job 5` 的触发来源,并补支线记录

- 当前目标:
  - 用户要求继续用真实录制来看为什么“没有回流”。
  - 上一轮已经修住了 `worktree` 看不到 E2E seed patch 的断点。
  - 这轮要确认:
    - 原回流链是否已经恢复
    - 当前剩余失败是不是另一个独立问题

- 当前决定:
  - 不直接继续改代码。
  - 先拿动态证据:
    - `cargo test` 收尾状态
    - `.e2e-tests` 里的录制 / report / stdout 证据
  - 只有在证据表明 `LOOP_COMPLETE` 尾巴是稳定可复现且根因足够清晰时,才继续修改实现。

- 当前状态:
  - **进行中** - 已确认工作树中关于 `parallel-experimental-dev-engine-example` 的 snapshot commit 修复仍在。
  - 下一步先核对:
    - 会话 `14273` 的 `cargo test`
    - `report.json` / `report.md`
    - `new_jobs_after=[(\"ralph#1\", 5)]` 的直接来源

## [2026-03-21 12:34:30] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] [记录类型]: 继续验证 - 在 examples 维度执行 E2E 测试

- [x] 阶段1: 回读支线六文件与 `crates/ralph-e2e/README.md`,确认 examples 对应的 E2E 入口
- [x] 阶段2: 搜索 `crates/ralph-e2e/src/scenarios/` 与 `examples/`,确认 examples 场景的过滤方式、后端和推荐命令
- [x] 阶段3: 运行 examples 相关 E2E 测试,保留命令与结果证据
- [x] 阶段4: 回写 `notes__memory_axes.md` / `WORKLOG__memory_axes.md`,记录这轮 examples 验证结论

- 当前目标:
  - 用户要求“在 examples 做 E2E 测试”。
  - 这轮要先确认仓库里 examples 已经接到哪些 `ralph-e2e` 场景,再跑正确的命令。

- 当前决定:
  - 继续使用支线六文件 `__memory_axes`,不切回默认六文件。
  - 优先复用现有 `ralph-e2e` example scenarios,避免自己发明一套临时测试方式。
  - 先查清楚是否能用统一 filter 覆盖 examples,再决定跑单项还是批量。
  - 实跑后确认全量 26 条真后端 batch 过重,本轮按“1 条完整 pass + 1 条复杂场景动态探针”收口,并把全量 fresh report 留到后续无人值守窗口。

- 当前状态:
  - **本轮完成** - 已确认 `--filter example` 命中 26 条 Codex-only example scenarios。

## [2026-03-21 18:10:25] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] [记录类型]: 继续排查 - `parallel-experimental-dev-engine-example` 无回流的根因升级

- [x] 阶段1: 回读支线六文件与最新录制证据,确认“worker 侧完全不回流”旧结论已经被新证据推翻
- [x] 阶段2: 静态回读 `crates/ralph-core/src/parallel/instance.rs`、`crates/ralph-core/src/config.rs`、`crates/ralph-e2e/src/executor.rs`,检查 workspace root / git root / clone source 的真实链路
- [ ] 阶段3: 先对超长 `notes__memory_axes.md` 做续档与持续学习摘要,避免继续污染当前注意力窗口
- [ ] 阶段4: 修复并行 clone/worktree 的 repo root 解析,确保它绑定当前运行 workspace,而不是外层真实仓库
- [ ] 阶段5: 补回归测试并重新录制 `parallel-experimental-dev-engine-example`,核对回流链路是否闭环

- 当前目标:
  - 用户要求我直接跑 `parallel-experimental-dev-engine-example` 录制,看清楚“为什么没有回流”。
  - 到目前为止,旧的 worker 回流问题已经不是主矛盾。
  - 当前要继续确认并修复更深一层的 clone/worktree 根目录错绑问题。

- 当前决定:
  - 继续沿用 `__memory_axes` 支线文件,不切回默认六文件。
  - 先把“`<\\/event>` 只是 parser hardening,不是当前最深根因”明确写回记录。
  - 在改代码前先做 `notes__memory_axes.md` 续档,遵守当前仓库的上下文文件规则。

- 当前状态:
  - **进行中** - 现象、假设、验证链已经升级为:
    - 现象:
      - `exp-001` / `exp-002` 的 `experiment.result -> experiment.reviewed` 已 durable 落盘
      - 但 runner worktree 里混入了外层真实仓库的 `PROMPT.md` 与其他无关产物
    - 当前主假设:
      - `parallel/instance.rs` 的 git root 解析没有绑定 `workspace_root`
      - 导致 clone/worktree 来源错误指向外层仓库
    - 最强备选解释:
      - 仍存在其它 prompt/source 污染链,只是恰好通过 clone 现象暴露出来
    - 下一步:
      - 先完成续档与摘要
      - 再直接修 `git_repo_root` 链路并验证
  - 已拿到:
    - `parallel-trigger-routing-example` 完整通过,耗时 `159.9s`
    - `parallel-experimental-dev-engine-example` 的真实启动与 `exp-001` / `exp-002` fanout 动态证据
  - 已补记:
    - `notes__memory_axes.md`
    - `WORKLOG__memory_axes.md`
    - `EPIPHANY_LOG__memory_axes.md`
    - `LATER_PLANS__memory_axes.md`

## [2026-03-21 12:42:30] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] [记录类型]: 继续调试 - 单独录制 `parallel-experimental-dev-engine-example` 回流缺失

- [x] 阶段1: 回读上一轮 examples E2E 记录,确认当前已知现象与证据边界
- [ ] 阶段2: 回读 scenario 实现、example 配置与 executor 录制路径,确认“结果回流”依赖的协议和证据文件
- [ ] 阶段3: 单独运行 `parallel-experimental-dev-engine-example`,保留 workspace 与录制证据
- [ ] 阶段4: 对照静态实现和动态录制,判断回流停在哪一层,形成“现象 -> 假设 -> 验证 -> 结论”
- [ ] 阶段5: 回写 `notes__memory_axes.md` / `WORKLOG__memory_axes.md` / 必要时 `ERRORFIX__memory_axes.md`

- 当前目标:
  - 用户要求单独跑 `parallel-experimental-dev-engine-example` 并录制,看清为什么没有 `experiment.result` / reviewed / integration 这类回流事件。

- 当前决定:
  - 使用 `systematic-debugging` 流程,先查清录制与断言协议,再复现。
  - 本轮先做根因调查,没有足够证据前不直接改代码。
  - 复现时优先保留 workspace 和录制文件,避免再次只有“跑过了但证据不全”。

- 当前状态:
  - **进行中** - 已知上一轮只有:
    - `experiment.task(exp-001)`
    - `experiment.task(exp-002)`
    没有观察到后续回流事件
  - 下一步先看:
    - `crates/ralph-e2e/src/scenarios/parallel_experimental_dev_engine_example.rs`
    - `crates/ralph-e2e/src/executor.rs`
    - `examples/parallel-experimental-dev-engine/ralph.yml`

## [2026-03-21 01:00:28] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] [记录类型]: 继续 apply - writer governance / promotion flow / doctor visibility 已完成并验证

- [x] 阶段1: 回读支线六文件、OpenSpec apply 指令与剩余 tasks,确认当前应继续实现而不是回到 explore
- [x] 阶段2: 回读 canonical-writer / experience-promotion specs 与现有 `experience_*` / `event_loop` 代码,确定最合适的落点
- [x] 阶段3: 先清理误留调试输出,再实现 canonical writer enforcement 与 handoff summary

- [x] 阶段4: 实现 promotion / demotion 评估与审计链路,补齐测试和必要的 debug 可见性
- [x] 阶段5: 运行格式化、单元测试、smoke test、OpenSpec validate,并更新 tasks / 支线记录

- 当前目标:
  - 已完成 `scoped-experience-system` 剩余 tasks 的主体落地。
  - 这轮重点交付包括:
    - canonical writer metadata/store
    - topic / role handoff summary
    - topic / role / project promotion / demotion 服务
    - `ralph doctor` 的 scoped experience 可见性

- 当前决定:
  - role handoff 不直接写进 `experience.md`,而是落到 `.ralph/roles/<hat_id>/handoff.md`。
  - 原因是 `experience.md` 当前采用“解析条目 -> 整文件重写”的 store,如果把 handoff 混进去,后续正常追加经验会把 handoff 摘要冲掉。
  - demotion 额外补了 `replaced_by` 审计字段,让“旧条目被谁或哪个 topic 取代”能直接追踪,不必靠全文搜索猜。

- 当前状态:
  - **全部完成并验证**:
    - OpenSpec tasks:
      - `3.1` ~ `3.4`
      - `4.1` ~ `4.4`
      - `6.3`
      - `6.4`
    - 验证结果:
      - `cargo fmt`
      - `cargo test -p ralph-core --lib` -> 488 passed
      - `cargo test -p ralph-core smoke_runner` -> 12 passed
      - `cargo test -p ralph-cli doctor_` -> 7 passed
      - `cargo test` -> passed
      - `openspec validate scoped-experience-system --type change` -> valid

## [2026-03-21 17:32:42] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] [记录类型]: 继续调试 - `parallel-experimental-dev-engine` 回流缺失根因已收敛

- [x] 阶段1: 回读上一轮 examples E2E 记录,确认当前已知现象与证据边界
- [x] 阶段2: 回读 scenario 实现、example 配置与 executor 录制路径,确认“结果回流”依赖的协议和证据文件
- [x] 阶段3: 单独运行 `parallel-experimental-dev-engine-example`,保留 workspace 与录制证据
- [x] 阶段4: 对照静态实现和动态录制,判断回流停在哪一层,形成“现象 -> 假设 -> 验证 -> 结论”
- [ ] 阶段5: 修正 prompt 约束并重跑验证,确认 `experiment.result -> experiment.reviewed -> integration.*` 能继续回流

- 当前目标:
  - 不只是解释“为什么没回流”,还要把这次已经定位到的行为约束补进 prompt,避免下次 worker 再把 workflow event 发到错误通道。

- 当前结论:
  - `experiment_runner#1` 与 `experiment_runner#3` 都真实完成了实现与验证,并在各自 worktree 里产出了独立 commit。
  - `experiment_runner#3` 最终通过 **stdout** 发出标准 `<event topic="experiment.result" ...>` 并被系统 durable 成 `bus.publish(topic=experiment.result)`。
  - `experiment_runner#1` 只在 **stderr/tool transcript** 中出现 `experiment.result` 事件文本,没有进入 stdout,因此没有 durable 回流。

- 当前决定:
  - 不改并行 runner 的 `stdout-only` 解析纪律,因为那是防止 stderr 假事件污染的正确护栏。
  - 直接修正 prompt:
    - 明确区分“外部注入 `ralph emit`”与“hat 正常 workflow event 发射”
    - 明确要求 workflow event 必须直接出现在最终 assistant stdout 中
    - 明确禁止通过 shell/tool/`cat`/`echo`/`ralph emit`/文件写入来“间接发事件”

- 当前状态:
  - **进行中** - 已进入“修正 prompt + 单场景重跑验证”阶段。

## [2026-03-21 17:53:19] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] [记录类型]: 继续验证 - 新录制已进入 fanout,同时发现旧官方 emit 文案仍混入 worker prompt

- [x] 阶段1: 继续跟踪新的单场景录制,确认 `ralph#1` 是否已经重新发出 `experiment.task`
- [x] 阶段2: 读取新录制中的 worker prompt 片段,确认前一轮 prompt 修正是否真的完整生效
- [ ] 阶段3: 继续等待 `experiment.result` durable 回流,并同步扫描 record jsonl
- [ ] 阶段4: 若仍无回流,定位那段“外部事件注入 `ralph emit`”旧文案的源码拼装位置并修正
- [ ] 阶段5: 重新录制并确认 `experiment.result -> experiment.reviewed -> integration.*` 闭环

- 当前目标:
  - 把“只修 example / all_hat 文案是否足够”这件事彻底验证清楚。
  - 如果不够,就继续上探到核心 prompt 组装链路,把旧误导源一起修掉。

- 当前观察:
  - 新录制已经再次出现两条 `experiment.task`,说明调度 fanout 仍然正常。
  - 但 worker 启动时打印出的 prompt 里,仍然能看到“外部事件注入: `ralph emit`”整段旧文案。
  - 这说明前一轮修改目前只是新增了“正常 workflow event 必须走 stdout”的护栏,还没有移除旧的误导性提示源。

- 当前状态:
  - **进行中** - 目前正在等待新录制里的第一批 durable 业务 topic。
  - 如果这轮回流仍不稳定,下一步将直接修核心 prompt 来源,而不是继续只在 example 层补约束。

## [2026-03-21 17:59:06] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] [记录类型]: 继续调试 - worker 回流已恢复,当前卡在 `integration.task` 关闭标签被转义

- [x] 阶段1: 继续跟踪新录制,确认 `exp-002` 是否也出现 durable `experiment.result -> experiment.reviewed`
- [x] 阶段2: 对照 `events.jsonl` 与 `record-session.jsonl`,确认 integration 前后的真实 topic 流
- [x] 阶段3: 从录制里提取 `integration.task` 原始 stdout 文本,确认是否存在协议格式异常
- [ ] 阶段4: 修正最小根因并补回归测试
- [ ] 阶段5: 重新验证 `integration.task -> integration.applied -> experiment.complete` 是否恢复 durable 闭环

- 当前目标:
  - 把“为什么还没有最终回流”继续收敛到 integration 层,而不是停留在最初的 worker 层猜测。
  - 若根因已被动态证据和静态代码同时支撑,就直接做最小修复并重录验证。

- 当前结论:
  - `exp-001` 与 `exp-002` 现在都已经发生 durable 回流:
    - `experiment_runner#1 -> experiment.result -> experiment_auditor#1 -> experiment.reviewed`
    - `experiment_runner#3 -> experiment.result -> experiment_auditor#1 -> experiment.reviewed`
  - `ralph#1` 随后确实尝试发布了 `integration.task`,但 record-session 里的原始 stdout 文本是:
    - `<event topic="integration.task" ...>{...}<\\/event>`
  - 这条 `integration.task` 没有进入 `events.jsonl`,说明当前 durable 断点已经从 worker 层前移到了 coordinator -> integrator 之间。

- 当前决定:
  - 先不再继续怀疑 worker 回流链路,因为该链路已经被新录制正面验证通过。
  - 优先修复“`<\\/event>` 不被 EventParser 识别”这一最小根因,而不是继续只在 example prompt 上追加软约束。

- 当前状态:
  - **进行中** - 已拿到 integration 层的静态/动态双证据。
  - 下一步直接修改 parser 容错与回归测试,随后重跑 example 录制验证。

## [2026-03-21 18:38:00] [Session ID: 68546] [记录类型]: 收尾确认 - `parallel-experimental-dev-engine-example` 真实录制“无回流”问题已解除

- [x] 阶段1: 回读 `__memory_axes` 支线记录,确认本轮要核实的是“当前真实录制是否还无回流”
- [x] 阶段2: 回看 `.e2e-tests/report.md` / `.e2e-tests/report.json`,确认 fresh 真后端录制的最终 verdict
- [x] 阶段3: 回看 `.e2e-tests/parallel-experimental-dev-engine-example/.ralph/events.jsonl`,核对 `experiment.* -> integration.*` 主题链是否完整闭环
- [x] 阶段4: 将“原无回流主问题”和“旧 report 里的 `job 5` 尾巴”拆成两个独立结论,避免后续混淆

- 当前目标:
  - 用户要求直接看这次真实录制,确认为什么之前会觉得“没有回流”。
  - 这次收尾要把最终口径固定下来,避免下次重新载入上下文时又把两个问题混成一个。

- 当前决定:
  - 不继续追新的代码修改。
  - 先以 fresh 真录制产物为准收口:
    - `report.md`
    - `report.json`
    - `events.jsonl`
  - 把 `job 5` 尾巴单独降级为后续 flaky 跟踪项,不再归因到“无回流”主问题。

- 当前状态:
  - **本轮完成** - 已验证:
    - `.e2e-tests/report.md` 显示 `parallel-experimental-dev-engine-example` `PASSED`
    - `.e2e-tests/report.json` 显示:
      - `Required topic chain observed (example)` 通过
      - `No new jobs after LOOP_COMPLETE (example)` 通过
    - `.e2e-tests/parallel-experimental-dev-engine-example/.ralph/events.jsonl` 已出现完整链路:
      - `experiment.task` x2
      - `experiment.result` x2
      - `experiment.reviewed` x2
      - `integration.task`
      - `integration.applied`
      - `experiment.complete`
  - 最终结论:
    - 用户最初看到的“没有回流”主问题,当前代码下已经不再复现
    - 真正修住它的关键是:
      - E2E seed 输入先提交进 snapshot `HEAD`
      - worktree job 再从这个 `HEAD` 切出
    - 旧的 `job 5` 尾巴目前只能算未稳定复现的独立 flaky 方向

## [2026-03-21 21:39:02] [Session ID: DBB90F67-C911-405F-A794-32909232C914] [记录类型]: 单独跟踪 `parallel-experimental-dev-engine-example` 旧 `job 5` 尾巴

- [x] 阶段1: 回读 `__memory_axes` 支线上下文,确认“无回流主问题”已与 `job 5` 尾巴分离
- [ ] 阶段2: 搜索旧失败口径的直接来源,定位 `new_jobs_after=[(\"ralph#1\", 5)]` 是在哪一层生成的
- [ ] 阶段3: 对照 scenario 断言逻辑、Supervisor completion 逻辑与 stdout artifact 保留策略,形成主假设与最强备选解释
- [ ] 阶段4: 视证据强度决定是否做最小复现实验或最小仪表增强
- [ ] 阶段5: 回写 `notes__memory_axes.md` / `WORKLOG__memory_axes.md` / 必要时 `LATER_PLANS__memory_axes.md`

- 当前目标:
  - 用户要求把 `parallel-experimental-dev-engine-example` 旧的 `job 5` 尾巴单独跟踪。
  - 这轮不再讨论“为什么没有回流”,而是只讨论:
    - 为什么历史上会出现 `completion_seen=true, new_jobs_after=[("ralph#1", 5)]`
    - 它到底是 runtime 真实多发了新 job,还是断言 / artifact 口径造成的假象

- 当前决定:
  - 先不改代码。
  - 先沿 `systematic-debugging` 路径拿静态 + 动态证据:
    - 搜旧失败口径来源
    - 比对 completion 判定逻辑
    - 再决定要不要跑最小复现实验
  - 如果当前没有稳定复现,就优先收敛成“候选假设 + 证伪条件”,而不是硬下根因结论。

- 当前状态:
  - **进行中** - 已确认:
    - “无回流主问题”当前已解除
    - `job 5` 尾巴仍是独立 flaky 方向
  - 下一步先查:
    - `crates/ralph-e2e/src/scenarios/parallel_experimental_dev_engine_example.rs`
    - `crates/ralph-core/src/parallel/supervisor.rs`
    - 与 `LOOP_COMPLETE` / `new_jobs_after` 相关的断言与 artifact 代码

## [2026-03-21 21:45:32] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] [记录类型]: 继续单独跟踪 `job 5` 尾巴 - 先查 `pending -> 新 job` 静态链路

- [x] 阶段1: 回读 `__memory_axes` 支线上下文,确认这轮只追旧 `job 5` 尾巴
- [ ] 阶段2: 沿 `ParallelSupervisor` / `HatInstanceActor` 静态追踪 `Deliver -> pending -> spawn job` 的链路
- [ ] 阶段3: 判断 `LOOP_COMPLETE` 之后是否仍可能存在已排队的 `ralph#1` job 自然起跑
- [ ] 阶段4: 若静态证据不足,设计最小可证伪测试或最小仪表增强
- [ ] 阶段5: 将本轮证据与候选假设补写到 `notes__memory_axes.md` / `WORKLOG__memory_axes.md`

- 当前目标:
  - 先把“`completion` 后为什么历史上还会看到 `ralph#1 job=5`”收敛到更具体的机制层。
  - 优先区分:
    - runtime 里确实还有 pending job 被启动
    - 还是 scenario 扫 mixed stdout 时把别的可观测行误判成了“completion 后新 job”

- 当前决定:
  - 先不做代码修改。
  - 先拿静态证据把 `pending` 生命周期查清楚,因为这一步最能快速证伪“job 泄漏”假设。
  - 如果静态阅读后仍有两种解释都站得住,再补一个最小测试,而不是直接动主逻辑。

- 当前状态:
  - **进行中** - 已重新读取支线计划、笔记和延后事项。
  - 下一步直接看:
    - `crates/ralph-core/src/parallel/supervisor.rs`
    - `crates/ralph-core/src/parallel/instance.rs`
    - `crates/ralph-core/src/parallel/command_queue.rs`

## [2026-03-21 22:00:49] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] [记录类型]: `job 5` 尾巴阶段性收敛 - 已拿到“prequeued ralph job”动态证据

- [x] 阶段1: 回读 `__memory_axes` 支线上下文,确认这轮只追旧 `job 5` 尾巴
- [x] 阶段2: 沿 `ParallelSupervisor` / `HatInstanceActor` 静态追踪 `Deliver -> pending -> spawn job` 的链路
- [x] 阶段3: 判断 `LOOP_COMPLETE` 之后是否仍可能存在已排队的 `ralph#1` job 自然起跑
- [x] 阶段4: 做最小可证伪测试与最小证据增强
- [x] 阶段5: 将本轮证据与候选假设补写到 `notes__memory_axes.md` / `WORKLOG__memory_axes.md`

- 当前结论:
  - 已验证:
    - `HatInstanceActor` 当前语义下,只要事件在 completion 之前已经进入 `ralph#1.pending`,它就可能在 `LOOP_COMPLETE` 之后继续起跑成下一份 ralph job。
    - 新增的单测
      - `parallel::supervisor::routing_tests::supervisor_allows_prequeued_ralph_job_to_start_after_completion_promise`
      已通过,动态证明了这条机制存在。
    - `crates/ralph-e2e/src/executor.rs` 里 `.e2e/stdout.txt` 之前只保留前段,会把 `LOOP_COMPLETE` / `job 5` 这类尾部证据截掉。
    - 现已改成保留 `head + tail`,并补了对应单测。

- 当前判断:
  - 这还不足以直接宣称“历史上的旧 `job 5` 就是这个根因”。
  - 但它已经把候选假设从“纯静态猜测”升级成了“有动态机制证明的最强候选”。

- 当前状态:
  - **本轮阶段性完成** - 已拿到:
    - 静态链路
    - 最小动态证据
    - 后续排障所需的 artifact 改善
  - 如果继续下一轮,最值得做的是:
    - 决定是否把 completion 语义进一步收紧为“冻结 pending,只 drain running”

## [2026-03-24 12:21:41] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] [记录类型]: 实施方案2 - 收紧 completion 语义为“冻结 pending,只 drain running”

- [x] 阶段1: 回读 `__memory_axes` 支线记录,确认用户已明确选择方案2
- [ ] 阶段2: 重新梳理 Supervisor/Instance 控制面,决定“冻结 pending”落在哪一层最稳
- [ ] 阶段3: 实施 runtime 修复,让 completion 后不再启动任何 pending job
- [ ] 阶段4: 把现有 characterization test 改造成 fix regression,并补必要的新测试
- [ ] 阶段5: 运行最小验证 + 必要回归验证,再回写支线记录

- 当前目标:
  - 不只是解释 completion 语义,而是把它正式改成更符合直觉的产品行为:
    - completion 后只允许 drain 正在 Running 的 job
    - completion 前已排队的 pending 不再继续起跑

- 当前决定:
  - 先不急着写代码。
  - 先比较两种实现面:
    - Supervisor 发控制命令,让 instance 进入“freeze pending”状态
    - Supervisor 直接借用 shutdown 路径,但避免误伤 running job 的自然收尾
  - 目标是做最正确修复,不是最小补丁。

- 当前状态:
  - **进行中** - 现在进入设计与实现阶段。
  - 下一步先读:
    - `crates/ralph-core/src/parallel/mod.rs`
    - `crates/ralph-core/src/parallel/instance.rs`
    - `crates/ralph-core/src/parallel/supervisor.rs`

## [2026-03-24 12:27:21] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] [记录类型]: 实施方案2已完成 - completion 现已冻结 pending 并完成全仓验证

- [x] 阶段1: 回读 `__memory_axes` 支线记录,确认用户已明确选择方案2
- [x] 阶段2: 重新梳理 Supervisor/Instance 控制面,决定“冻结 pending”落在哪一层最稳
- [x] 阶段3: 实施 runtime 修复,让 completion 后不再启动任何 pending job
- [x] 阶段4: 把现有 characterization test 改造成 fix regression,并补必要的新测试
- [x] 阶段5: 运行最小验证 + 必要回归验证,再回写支线记录

- 当前结论:
  - 已落地的新语义:
    - CLI/CI 在看到 completion promise 后,会立刻对所有 instance 请求 completion freeze
    - 已经 Running 的 job 继续 drain
    - 已排队但尚未启动的 pending job 不再起跑
  - parallel-tui pause 模式保持旧语义:
    - 仍允许在暂停态下继续对话/恢复,避免误伤交互模式

- 当前验证:
  - 定向测试:
    - `cargo test -p ralph-core supervisor_freezes_prequeued_ralph_job_after_completion_promise`
    - `cargo test -p ralph-core supervisor_does_not_route_new_events_after_completion_promise`
    - `cargo test -p ralph-core supervisor_pause_on_completion_promise_continues_consuming_external_events_in_tui_mode`
    - `cargo test -p ralph-e2e test_truncate_with_notice_preserves_head_and_tail`
    - `cargo test -p ralph-e2e test_truncate_with_notice_returns_original_when_short_enough`
  - 全仓验证:
    - `cargo test`
  - 结果:
    - 全部通过

- 当前状态:
  - **本轮完成** - 方案2已经从设计、实现到验证闭环。
  - 若继续下一轮,最值得做的是:
    - 真实重跑一次 `parallel-experimental-dev-engine-example`,确认旧 `job 5` 尾巴在真后端场景里也不再出现

## [2026-03-25 20:04:57] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] [记录类型]: 继续调试 - 对比 stalled run 与最小实验,确认 `exp-002` 的 durable 断点

- [x] 阶段1: 回读 `__memory_axes` 支线六文件与上一轮摘要,确认当前活跃问题已经切到 `parallel-experimental-dev-engine-example`
- [ ] 阶段2: 对比 2026-03-24 stalled run 与 2026-03-25 最小实验的 `events.jsonl` / `record-session`,确认 `exp-002` 是“未发出”还是“发出但未 durable”
- [ ] 阶段3: 如有必要,在干净 clone 中再跑一轮完整 example 录制,观察前 1-2 分钟是否稳定出现两条 `experiment.task`
- [ ] 阶段4: 只有在证据收敛后,才决定是否需要改 `supervisor prompt` 或继续深挖 worker/runtime 路径

- 当前目标:
  - 先把 `exp-002` 这层前置卡点查实。
  - 暂时不回到旧 `job 5` 尾巴,避免把“尚未进入 completion 的 run”和“completion 之后还有尾巴”混成一个问题。

- 当前决定:
  - 采用 systematic debugging 口径:
    - 先列现象
    - 再给主假设和备选解释
    - 最后用最小可证伪实验收敛
  - 继续只使用 `__memory_axes` 支线文件记录,不切回默认六文件。

- 当前状态:
  - **进行中** - 已确认:
    - 最小实验里 `ralph#1` 首轮确实可以 durable 发出 `exp-001` + `exp-002`
    - 因此“首轮只发一条 task”已经不能当成默认结论
  - 下一步直接进入旧 stalled run 与最小实验的事件流对比

## [2026-03-31 02:34:16] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] [记录类型]: 收口 `parallel-experimental-dev-engine-example` - all-hat overlay 运行时覆写与最终真后端 PASS

- [x] 阶段1: 回读 `__memory_axes` 支线六文件,确认本轮收口仍沿用同一套上下文
- [x] 阶段2: 核对 `.e2e-tests/report-live.md` / `.e2e-tests/report.json` / `.ralph/events.jsonl`,确认最终真后端结果
- [x] 阶段3: 将“现象 -> 假设 -> 验证 -> 结论”补写到 `notes__memory_axes.md`
- [x] 阶段4: 将代码改动、验证命令与 bug fix 结论补写到 `WORKLOG__memory_axes.md` / `ERRORFIX__memory_axes.md`
- [x] 阶段5: 清理 `LATER_PLANS__memory_axes.md` 中已落地的 all-hat 降噪延期项,并判断是否需要追加 `EPIPHANY_LOG__memory_axes.md`

- 当前目标:
  - 给这轮实现和真后端验证做正式收口。
  - 把“默认内嵌 all-hat overlay + 运行时显式覆写”的最终口径落盘,避免后面只记得 PASS,忘了为什么这样设计。

- 当前结论:
  - 已落地:
    - `core.all_hat_prompt` 运行时配置
    - example / E2E 的轻量 inline overlay
    - `experiment.reviewed` 的结构化 `evidence_ok` 解析
  - 已验证:
    - `parallel-experimental-dev-engine-example` 第二轮真后端复跑通过
    - `report-live.md` / `report.json` 的最新 mtime 均为 `2026-03-31 02:32:08`
    - `Required topic chain observed (example)` 与 `No new jobs after LOOP_COMPLETE (example)` 均为 PASS

- 当前状态:
  - **本轮完成** - 代码、验证、支线记录都已收口。
  - 如后续继续推进,最值得做的是:
    - 把这套 `core.all_hat_prompt` 运行时覆写能力进一步接到“workflow / preset 首次释放到用户目录”的主线设计上

## [2026-03-31 02:34:16] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] [记录类型]: explore Rerun graph 集成 - 可视化 hat instance 生命周期与关系图

- [x] 阶段1: 回读 `__memory_axes` 支线六文件与 openspec-explore 约束,确认本轮只做探索不实现
- [ ] 阶段2: 回读 Ralph 并行 runtime 关键代码,梳理 hat instance 创建、消息传递、停止、workflow 协调的真实关系
- [ ] 阶段3: 调研 Rerun graph / node-like diagram 当前能力,确认适合承载哪些节点、边、时序属性
- [ ] 阶段4: 形成一版“谁创建谁、谁向谁发消息、谁停止谁”的图模型草案,并评估落点与风险

- 当前目标:
  - 探索如何把 Ralph 的并行 runtime 关系,用 Rerun 的 node-like diagram 表达出来。
  - 用户关心的重点是:
    - hat instance 创建
    - 消息传递
    - instance 停止
    - workflow
    - 谁创建谁

- 当前决定:
  - 严格保持 explore 模式,不写实现代码。
  - 先拿到当前 runtime 的真实关系图,再讨论 Rerun 里的图数据模型,避免先空想 UI。

- 当前状态:
  - **进行中** - 下一步直接查看:
    - `openspec list --json`
    - `crates/ralph-core/src/parallel/*`
    - Rerun 官方 graph 示例与文档

- 进展补充:
  - 已确认 Rerun 官方 graph 适合:
    - time-varying graph
    - directed / undirected edges
    - force-based layout
  - 已确认 Ralph 当前的 durable 关系证据存在一个关键缺口:
    - `events.jsonl` 能稳定给出 `source_instance` / `reply`
    - 但不能完整给出所有最终 `target_instance` / fanout recipients

- 当前状态:
  - **本轮完成** - 已形成探索结论与图模型草案。
  - 如果下一轮继续推进,最值得做的是二选一:
    - 先做 live graph 方案,用 observer 快速跑通
    - 或先补 delivery 级 durable 观测,再做离线 replay graph

## [2026-04-02 11:17:41] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] [记录类型]: 收口 Rerun runtime graph explore 并落正式 OpenSpec change

- [x] 阶段1: 回读 `__memory_axes` 支线记录与已生成的 Rerun graph 草案,确认本轮目标是“正式成稿”,不是继续口头讨论
- [x] 阶段2: 创建独立的 `rerun-runtime-graphs` OpenSpec change,补齐 `proposal.md` / `design.md` / `tasks.md`
- [x] 阶段3: 增加最小 spec delta `runtime-graph-observability`,把 V1 / V2 边界写成正式 requirement
- [x] 阶段4: 重新执行 OpenSpec 校验并确认 change 已进入 `openspec list`

- 当前目标:
  - 把“Rerun 运行时关系图”从 explore 结论,收口成一份以后不会失联的正式 OpenSpec 方案。
  - 尤其把用户强调的 V1 / V2 都写进 artifact,避免后面只记得 live demo,忘了 durable replay。

- 当前决定:
  - 新 change 独立命名为 `rerun-runtime-graphs`,不并入:
    - `startup-resource-bootstrap`
    - `runtime-capability-invocation`
  - 保留双图体系:
    - `ralph hats graph` 继续负责静态 topology
    - Rerun graph 负责运行时关系图
  - 正式记录分期:
    - V1 = live runtime graph
    - V2 = durable replay graph

- 当前状态:
  - **本轮完成** - 已落盘:
    - `openspec/changes/rerun-runtime-graphs/proposal.md`
    - `openspec/changes/rerun-runtime-graphs/design.md`
    - `openspec/changes/rerun-runtime-graphs/tasks.md`
    - `openspec/changes/rerun-runtime-graphs/specs/runtime-graph-observability/spec.md`
  - **已验证**:
    - `openspec validate rerun-runtime-graphs --type change` -> `Change 'rerun-runtime-graphs' is valid`
    - `openspec list --json` -> `rerun-runtime-graphs` 显示 `status: in-progress`, `totalTasks: 15`
  - 如果下一轮继续推进,最值得做的是:
    - 先按本次 design 实施 V1 live runtime graph
    - 同时把 V2 需要的 durable delivery / lifecycle evidence 缺口单列补齐

## [2026-04-03 00:42:59] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] [记录类型]: 继续 apply - `rerun-runtime-graphs` V1 live runtime graph MVP

- [x] 阶段1: 回读 `rerun-runtime-graphs` 的 proposal / design / tasks,确认本轮从“正式成稿”转入“最小实现”
- [ ] 阶段2: 盘点现有 runtime observers、CLI 入口与依赖,确认 V1 最小落点
- [ ] 阶段3: 实现 V1 live runtime graph 的最小骨架,优先覆盖 runtime topology / workflow / reply 的基本数据流
- [ ] 阶段4: 补测试与文档,运行格式化、测试、OpenSpec 校验

- 当前目标:
  - 从上一轮已经批准的 OpenSpec change 继续推进,落第一版可运行的 V1 live runtime graph MVP。
  - 默认优先做“最小但真的可用”的一版,而不是一口气追完整 V2 replay。

- 当前决定:
  - 继续使用 `__memory_axes` 支线记录,不切回默认六文件。
  - 本轮默认先找最稳的实现入口:
    - runtime observers
    - CLI 命令/导出入口
    - Rerun 依赖接入方式
  - 如果代码现实和 design 有偏差,先按代码真相收缩 MVP,再补 notes 说明。

- 当前状态:
  - **进行中** - 下一步先查:
    - 是否已有 Rerun 依赖或相关模块
    - `parallel/supervisor` / `event_logger` / observer 的可挂点
    - CLI 里最合适的新入口
