## [2026-03-20 16:56:05] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 任务名称: explore 记忆系统双轴拆分

### 任务内容
- 探索 Ralph 现有 memories / tasks / 六文件体系与“角色维度 + 话题维度”双轴 memory 的关系。
- 判断这个新方向应该怎样和 `.agent/memories.md`、`.agent/tasks.jsonl`、`.ralph/` 现有职责对齐。

### 完成过程
- 回读了:
  - `docs/concepts/memories-and-tasks.md`
  - `specs/ralph-memories/design.md`
  - `config/all_hat.md`
  - `crates/ralph-core/src/event_loop/mod.rs`
  - `crates/ralph-core/src/memory_store.rs`
  - `crates/ralph-core/src/task_store.rs`
- 确认当前系统已经有:
  - `.agent/memories.md` 作为长期可注入 memory
  - `.agent/tasks.jsonl` 作为 runtime work graph
  - 六文件 / `.agent/*.md` 作为 richer context
- 最终收敛出推荐分层:
  - `tasks.jsonl`
  - `.ralph/roles/<hat_id>/WORKLOG.md`
  - `WORKLOG__topic.md` / `notes__topic.md` / `task_plan__topic.md`
  - `.agent/memories.md`

### 总结感悟
- 真正有价值的不是简单把 WORKLOG 复制两份,而是把“raw role log”和“shared topic synthesis”分清。
- 如果后面真做这个方向,最需要先防的不是功能缺失,而是双写、漂移和并发写同一 topic 文件。

## [2026-03-20 22:36:07] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 任务名称: explore canonical writer 与 `experience.md` 命名

### 任务内容
- 把用户刚定下的长期经验命名 `experience.md` 正式纳入这条支线记录。
- 继续讨论话题维度 shared files 的 canonical writer 规则,避免以后再次回到上下文时只记得 v1,忘了 v2。

### 完成过程
- 回读了:
  - `task_plan__memory_axes.md`
  - `notes__memory_axes.md`
  - `WORKLOG__memory_axes.md`
  - `EPIPHANY_LOG__memory_axes.md`
  - `tasks/context-file-injection.code-task.md`
  - `config/all_hat.md`
  - `crates/ralph-core/src/hatless_ralph.rs`
- 确认了两层口径必须分开:
  - 当前实现事实仍是 `.agent/memories.md`
  - 探索目标命名已经收敛为 `experience.md`
- 收敛出了新的推荐规则:
  - role log 先写 raw evidence
  - topic shared files 必须由 canonical writer 单点收敛
  - canonical writer 采用“owner-first, `ralph#1` fallback”
  - `experience.md` 只接收 topic 关闭后的稳定经验蒸馏

### 总结感悟
- 真正难的不是“要不要多一类文件”,而是“谁有权把多方信息提升成共享结论”。
- 只要把 role / topic / experience 三层的写入权限分清,这套模型就开始变得可实现了。

## [2026-03-20 22:52:27] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 任务名称: refine 角色经验层 - 引入 `.ralph/roles/<hat_id>/experience.md`

### 任务内容
- 接住用户新增的岗位级经验要求。
- 重新检查 role 目录是否还适合继续承担 raw log 的主落点。

### 完成过程
- 基于用户新增要求,对上一版结构做了 v3 修正:
  - role 目录现在明确承载岗位级 `experience.md`
  - raw log 更推荐下沉到 instance 级目录
  - topic 继续承载共享结论
  - 项目根 `experience.md` 继续承载跨角色通用经验
- 同时补出了新的推荐注入顺序:
  - 项目经验
  - 岗位经验
  - topic 共享上下文
  - instance 临时上下文

### 总结感悟
- 这条新增要求很值钱,因为它把“角色经验”和“实例轨迹”彻底拆开了。
- 拆开之后,整个模型比上一版更顺,也更不容易在并行实例下打架。

## [2026-03-20 22:55:38] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 任务名称: refine 经验晋升漏斗与文档/实现现状

### 任务内容
- 继续收敛:
  - topic -> role experience
  - role -> project experience
- 顺手核对 memory 文档口径和代码现实是否一致。

### 完成过程
- 回读并核实了:
  - `docs/concepts/memories-and-tasks.md`
  - `docs/advanced/memory-system.md`
  - `specs/ralph-memories/design.md`
  - `crates/ralph-core/src/config.rs`
  - `crates/ralph-core/src/event_loop/mod.rs`
- 确认了一个重要现实:
  - 文档里已经出现 `memories.path`
  - 但代码当前仍直接读取 `.agent/memories.md`
- 在此基础上,补齐了经验晋升默认规则:
  - 默认先窄后宽
  - 先 topic,再 role,最后 project
  - 只有明显跨角色、跨 workflow 时才升到项目根 `experience.md`

### 总结感悟
- 真正稳的不是“把经验往上提得快”,而是“只在必须更广泛共享时才往上提”。
- 项目级 experience 一旦存在自动注入,它就是高污染面的全局知识,必须很克制。

## [2026-03-20 23:08:29] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 任务名称: 收敛准设计稿 - writer / promotion / demotion / injection

### 任务内容
- 把探索结果整理成一版更接近 OpenSpec 的结构化设计口径。

### 完成过程
- 正式补齐了:
  - topic / role / project 三类 canonical writer
  - writer 交接规则
  - experience entry 的统一格式建议
  - topic -> role -> project 的晋升规则
  - project -> role / role -> topic 的降级规则
  - 普通 hat 与 `ralph#1` 的默认注入顺序
  - 无 `PROMPT.md` / 无 `ralph.yml` 时的启动语义

### 总结感悟
- 到这一步,这套体系已经不再只是概念讨论,而是有了可以直接转成 OpenSpec 的骨架。
- 最关键的收敛点是: project experience 默认只让 `ralph#1` 写,这样全局知识层才不会迅速失控。

## [2026-03-20 23:58:51] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 任务名称: 创建 OpenSpec change - `scoped-experience-system`

### 任务内容
- 把前面几轮 explore 的结论正式落成 OpenSpec change。

### 完成过程
- 创建了 change:
  - `openspec/changes/scoped-experience-system/`
- 已落盘 artifacts:
  - `proposal.md`
  - `design.md`
  - `specs/experience-scopes/spec.md`
  - `specs/canonical-writer/spec.md`
  - `specs/experience-promotion/spec.md`
  - `specs/experience-injection/spec.md`
  - `tasks.md`
- 已验证:
  - `openspec status --change "scoped-experience-system"` -> 4/4 artifacts complete
  - `openspec validate scoped-experience-system --type change` -> valid

### 总结感悟
- 这轮的关键进展不是新增了多少文档,而是把“记忆体系”从讨论对象变成了正式变更对象。
- 现在后续如果要实现,就不需要再从聊天记录里捞口径了,直接沿着 change 做就行。

## [2026-03-21 00:40:47] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 任务名称: apply `scoped-experience-system` - 注入链路首批落地

### 任务内容
- 把 scoped experience 从“路径 + parser/store 基座”继续推进到真正参与 prompt 构建。
- 完成普通 hat 与 Ralph 两条注入路径的首批实现,并保留 legacy memories 兼容层。

### 完成过程
- 新增:
  - `crates/ralph-core/src/experience_injection.rs`
- 改动:
  - `crates/ralph-core/src/event_loop/mod.rs`
  - `crates/ralph-core/src/event_loop/tests.rs`
  - `crates/ralph-core/src/lib.rs`
  - `openspec/changes/scoped-experience-system/tasks.md`
- 已实现:
  - 普通 hat:
    - `project experience`
    - `role experience`
    - `topic summary`
    - `instance summary`
    - `runtime task state`
  - Ralph:
    - project experience 先于 metadata
    - owner role experience 延后到 metadata 之后
    - topic summary / runtime task state 走后置补充
  - legacy:
    - `.agent/memories.md` 继续兼容注入
    - prompt 中显式标记 `Legacy Memories (Compatibility)`
- 已验证:
  - `cargo test -p ralph-core --lib`
  - `cargo test -p ralph-core smoke_runner`

### 总结感悟
- 这轮最有价值的不是“多读了几类文件”,而是把注入顺序真正变成了一个可测的协议。
- topic summary 先用“唯一 group 才注入”的保守策略是对的,因为它先守住了不误注入历史 topic 这条底线。

## [2026-03-21 01:00:28] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 任务名称: apply `scoped-experience-system` - 完成 writer governance / promotion / demotion / doctor visibility

### 任务内容
- 继续完成 `scoped-experience-system` 剩余实现任务。
- 补齐 canonical writer enforcement、handoff summary、promotion / demotion 流程,以及 `ralph doctor` 的 scoped experience 可见性。

### 完成过程
- 新增:
  - `crates/ralph-core/src/experience_governance.rs`
  - `crates/ralph-core/src/experience_promotion.rs`
- 继续扩展:
  - `crates/ralph-core/src/experience.rs`
  - `crates/ralph-core/src/experience_parser.rs`
  - `crates/ralph-core/src/experience_store.rs`
- 接线与清理:
  - `crates/ralph-core/src/experience_injection.rs`
  - `crates/ralph-core/src/event_loop/tests.rs`
  - `crates/ralph-core/src/lib.rs`
  - `crates/ralph-cli/src/doctor.rs`
- 文档 / OpenSpec 同步:
  - `docs/advanced/memory-system.md`
  - `docs/advanced/architecture.md`
  - `docs/reference/migration-v1.md`
  - `openspec/changes/scoped-experience-system/design.md`
  - `openspec/changes/scoped-experience-system/tasks.md`
- 已落地能力:
  - topic / role / project canonical writer ownership store
  - topic shared file 非 owner 写入拒绝
  - role / project experience 写入授权
  - topic / role handoff summary 持久化
  - topic -> role / project promotion
  - role -> project promotion
  - project -> role / role -> topic demotion
  - `replaced_by` 审计链路
  - `ralph doctor` 展示 scoped experience 路径和 writer ownership
- 已验证:

## [2026-03-24 12:39:51] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 任务名称: completion freeze 真后端复核与 stalled run 证据整理

### 任务内容
- 回接上一轮已落地的 completion freeze 改动,避免只记得“代码已经改了”,忘了“真后端是否已经验证到位”。
- 对 `parallel-experimental-dev-engine-example` 跑一轮真实 `codex` E2E,专门看旧 `job 5` 尾巴是否还能出现。
- 如果 run 没有走到 completion,就把新的前置卡点如实记录下来,不要误报为“已验证通过”。

### 完成过程
- 回读了支线文件与当前代码入口:
  - `task_plan__memory_axes.md`
  - `notes__memory_axes.md`
  - `WORKLOG__memory_axes.md`
  - `LATER_PLANS__memory_axes.md`
  - `EPIPHANY_LOG__memory_axes.md`
  - `crates/ralph-core/src/parallel/instance.rs`
  - `crates/ralph-core/src/parallel/supervisor.rs`
  - `crates/ralph-core/src/parallel/supervisor/routing_tests.rs`
  - `crates/ralph-e2e/src/scenarios/parallel_experimental_dev_engine_example.rs`
- 补写了本轮计划与 notes,明确区分:
  - completion freeze 已经是代码事实
  - 真后端 example 是否验证到 completion 仍是待补证部分
- 真实执行:
  - `cargo run -p ralph-e2e -- codex --filter parallel-experimental-dev-engine-example --keep-workspace --verbose`
- 动态检查了:
  - `.e2e-tests/parallel-experimental-dev-engine-example/.ralph/events.jsonl`
  - `.e2e-tests/parallel-experimental-dev-engine-example/.ralph/agents.json`
  - `.e2e-tests/parallel-experimental-dev-engine-example/ralph/log/ralph#1/task_plan.md`
  - `.e2e-tests/parallel-experimental-dev-engine-example/.ralph/worktrees/experiment_runner_1/job-1/ralph/log/experiment_runner#1/task_plan.md`
  - `git -C .e2e-tests/parallel-experimental-dev-engine-example/.ralph/worktrees/experiment_runner_1/job-1 log --oneline -n 5`
- 发现 run 只推进到:
  - `experiment.task(exp-001)`
  - `experiment.result(exp-001)`
  - `experiment.reviewed(exp-001, evidence_ok=true)`
- 因为长时间没有新的 durable 事件,且尚未进入 completion 区域,中断了本次 stalled run,避免继续空等 1800 秒超时窗口。

### 总结感悟
- 这轮最重要的收获不是“旧 `job 5` 又出现了”,而是更靠前地确认:
  - 当前真后端 run 根本没走到 completion
  - 所以它不能拿来证明或反驳 completion freeze 的最终效果
- 另一个很值钱的规律是:
  - 验证 completion 尾巴问题时,第一道门不是盯 `LOOP_COMPLETE`
  - 而是先确认 scenario 真的走到了 completion 前夜,否则所有尾巴分析都会失焦

## [2026-03-21 18:35:50] [Session ID: 68546] 任务名称: 真实复跑 `parallel-experimental-dev-engine-example` 并确认“无回流”现象已解除

### 任务内容
- 继续接上一轮对 `parallel-experimental-dev-engine-example` 的排查,用 fresh 真后端录制重新确认“为什么没有回流”。
- 把“原来的无回流主问题”和“上一次 report 里 `LOOP_COMPLETE` 后 `job 5` 尾巴”严格拆开。

### 完成过程
- 先重新回读了 `__memory_axes` 支线记录,确认上一轮已经落地的修复是:
  - 在 `crates/ralph-e2e/src/scenarios/parallel_experimental_dev_engine_example.rs`
  - 把 E2E seed 的 `PROMPT.md` / example `ralph.yml` patch 固化进 snapshot commit
  - 再让 worktree job 基于该 snapshot `HEAD` 启动
- 再核对了整仓验证:
  - `cargo test`
  - 结果全绿
- 随后 fresh 真实复跑:
  - `cargo run -p ralph-e2e -- codex --filter parallel-experimental-dev-engine-example --keep-workspace --verbose`
- 复跑期间持续观察:
  - `.e2e-tests/parallel-experimental-dev-engine-example/.ralph/events.jsonl`
  - `.e2e-tests/parallel-experimental-dev-engine-example/.ralph/agents.json`
  - `.e2e-tests/parallel-experimental-dev-engine-example/ralph/log/...`
- 最终确认 fresh run 已完整通过,并且关键 topic 链再次闭环到:
  - `integration.applied`
  - `experiment.complete`

### 总结感悟
- 这轮最关键的结论是: 用户看到的“没有回流”主问题,本质上是 E2E seed 输入没有进入 `HEAD`,而不是 worker 永远不会回流。
- 上一次 report 里的 `ralph#1 job 5` 尾巴这次没有复现,所以目前不能把它和“无回流”混成同一个根因。它更像一个待继续盯的独立 flaky 方向。
  - `cargo fmt`
  - `cargo test -p ralph-core --lib`
  - `cargo test -p ralph-core smoke_runner`
  - `cargo test -p ralph-cli doctor_`
  - `cargo test`
  - `openspec validate scoped-experience-system --type change`

### 总结感悟
- canonical writer 这件事一旦做成独立治理层,后续 startup bootstrap 和 runtime capability invocation 都能直接复用,而不是再次散落权限判断。
- role handoff 最终落到 sidecar `handoff.md` 而不是 `experience.md` 本体,是这轮最关键的实现修正之一,因为它避免了 handoff 摘要被正常经验写入静默覆盖。

## [2026-03-21 12:41:58] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 任务名称: examples 维度 E2E 验证

### 任务内容
- 按用户要求,直接在 `examples/` 相关场景上做 `ralph-e2e` 验证。
- 重点确认:
  - examples 是否已有正式 E2E 入口
  - 推荐命令是什么
  - 至少拿到一条完整通过证据
  - 批量运行时有没有新的验证陷阱

### 完成过程
- 回读了:
  - `crates/ralph-e2e/README.md`
  - `crates/ralph-e2e/src/main.rs`
  - `crates/ralph-e2e/src/scenarios/parallel_*example.rs`
  - `examples/`
- 确认:
  - `--filter example` 会命中 26 条 Tier 8 examples scenarios
  - 这些 example scenarios 当前统一跑 `Backend::Codex`
- 实际执行:
  - `cargo run -p ralph-e2e -- codex --filter example --report both --skip-analysis`
- 拿到的有效证据:
  - `parallel-trigger-routing-example` 完整通过,耗时 `159.9s`
  - `parallel-experimental-dev-engine-example` 已真实启动,并完成 `exp-001` / `exp-002` 的 `experiment.task` fanout
- 中途判断:
  - 全量 26 条真后端 batch 对交互式会话过重
  - 因此在拿到“1 条完整 pass + 1 条复杂场景动态探针”后,主动中断批处理
- 清理动作:
  - 发现 `Ctrl-C` 后仍残留 `target/release/ralph run ...` 与 `codex app-server`
  - 已手动 `kill` 清干净,避免污染后续验证

### 总结感悟
- examples E2E 这条链路已经是“正式场景”,不是随手拼的 demo 命令,所以验证时应优先复用 `ralph-e2e` 而不是自己手写脚本。
- 真后端 examples 全量批跑非常耗时。交互式回合里更稳的做法是先拿:
  - 1 条最小闭环 example 的 pass
  - 1 条复杂 workflow example 的动态证据
  再决定是否切到无人值守全量跑。

## [2026-03-21 18:39:46] [Session ID: 68546] 任务名称: 收尾复核 `parallel-experimental-dev-engine-example` 真实录制证据

### 任务内容
- 把这轮“为什么没有回流”的最终口径固定下来。
- 再补一遍仓库级 `cargo test`,确保收尾不是只靠 E2E 单场景证据。

### 完成过程
- 重新核对了:
  - `.e2e-tests/report.md`
  - `.e2e-tests/report.json`
  - `.e2e-tests/parallel-experimental-dev-engine-example/.ralph/events.jsonl`
- 确认 fresh 真录制里:
  - `parallel-experimental-dev-engine-example` 已 `PASSED`
  - `Required topic chain observed (example)` 通过
  - `No new jobs after LOOP_COMPLETE (example)` 通过
  - 事件链已完整出现:
    - `experiment.task`
    - `experiment.result`
    - `experiment.reviewed`
    - `integration.task`
    - `integration.applied`
    - `experiment.complete`
- 追加更新了:
  - `task_plan__memory_axes.md`
  - `LATER_PLANS__memory_axes.md`
- 重新执行:
  - `cargo test`
  - 结果全绿

### 总结感悟
- 这轮更重要的不是再改一处代码,而是把“已修复的主问题”和“尚未稳定复现的独立 flaky”分清。
- 只要这个边界不丢,后面再回来看 `parallel-experimental-dev-engine-example`,就不会把 `job 5` 尾巴误诊成“又没有回流了”。

## [2026-03-21 22:00:49] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 任务名称: 单独跟踪 `parallel-experimental-dev-engine-example` 旧 `job 5` 尾巴

### 任务内容
- 把历史上的 `completion_seen=true, new_jobs_after=[("ralph#1", 5)]` 从“无回流主问题”里彻底拆出来单独分析。
- 重点确认:
  - runtime 是否真的允许 completion 后再起一个 `ralph` job
  - 还是只是 scenario 扫 mixed stdout 时的断言假象

### 完成过程
- 回读了 `__memory_axes` 支线记录,确认当前 fresh 真录制已经通过,这轮只盯旧 `job 5` 尾巴。
- 重新静态追踪了:
  - `crates/ralph-core/src/parallel/supervisor.rs`
  - `crates/ralph-core/src/parallel/supervisor/routing.rs`
  - `crates/ralph-core/src/parallel/instance.rs`
- 发现 completion 之后 Supervisor 会停止继续路由新事件,但不会立刻冻结 instance 内已经存在的 `pending`。
- 为了把这条怀疑从“静态猜测”升级成动态证据,新增了一个最小单测:
  - `parallel::supervisor::routing_tests::supervisor_allows_prequeued_ralph_job_to_start_after_completion_promise`
- 实际验证命令:
  - `cargo test -p ralph-core supervisor_allows_prequeued_ralph_job_to_start_after_completion_promise`
- 结果:
  - 测试通过
  - 动态证明:
    - 一条在 completion 前已经进入 `ralph#1.pending` 的 orphan event
    - 确实可以在 `LOOP_COMPLETE` 之后继续起成下一份 `ralph` job
- 同时补了 E2E artifact 保留策略:
  - `crates/ralph-e2e/src/executor.rs`
  - 把 `.e2e/stdout.txt` 从“只保留前段”改成“保留 head + tail”
- 实际验证命令:
  - `cargo test -p ralph-e2e test_truncate_with_notice_preserves_head_and_tail`
  - `cargo test -p ralph-e2e test_truncate_with_notice_returns_original_when_short_enough`
- 结果:
  - 两条单测都通过

### 总结感悟
- 这轮最重要的不是直接宣称“根因已锁定”,而是把一个很强的候选机制拿到了动态证据。
- 目前最稳的口径是:
  - 历史旧 `job 5` 很可能与 `ralph#1.pending` 在 completion drain 窗口内继续起跑有关
  - 但要把“很可能”升级成“已确认”,仍需要下一次复现时直接抓到完整 tail artifact
- 顺手改掉 artifact 只保留前半截这件事,很值。否则这种“问题在尾巴里”的 flaky,下次还会再次失真。

## [2026-03-25 21:09:31] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 任务名称: 修复 `parallel-experimental-dev-engine-example` 的 workspace 根 AGENTS 污染

### 任务内容
- 针对 `parallel-experimental-dev-engine-example` 的 integrator 长尾问题,为隔离 E2E workspace 增加专用根 `AGENTS.md` 覆盖。
- 目标是把 example worker 从仓库级开发流程中隔离出来,只按当前 hat instructions 与 incoming event 完成闭环。

### 完成过程
- 修改了:
  - `crates/ralph-e2e/src/scenarios/parallel_experimental_dev_engine_example.rs`
- 新增了:
  - `E2E_WORKSPACE_AGENTS_OVERRIDE`
  - `write_workspace_root_agents_override(workspace)`
- setup 流程现在会:
  - 在隔离 clone 的 workspace 根目录覆盖极简 `AGENTS.md`
  - 再把该覆盖随 snapshot commit 一起固化进 `HEAD`
  - 保证后续 worktree job 与 `ralph#1` 看到同一套轻量规则
- 已验证:
  - `cargo fmt --all`
  - `cargo test -p ralph-e2e scenarios::parallel_experimental_dev_engine_example::tests::seeded_workspace_snapshot_commit_makes_patched_prompt_visible_to_worktree -- --exact`
  - `cargo test -p ralph-e2e scenarios::parallel_experimental_dev_engine_example::tests::example_config_requires_structured_commit_fields_for_review_and_integration -- --exact`
  - `cargo run -p ralph-e2e -- codex --filter parallel-experimental-dev-engine-example --keep-workspace --verbose`
- 动态结果:
  - `.e2e-tests/report.json` 显示 `passed=true`
  - 事件链完整出现:
    - `integration.applied`
    - `experiment.complete`
    - `LOOP_COMPLETE`

### 总结感悟
- 对 example 类 E2E 来说,“隔离 git workspace”还不够,还要隔离 workspace 根的 agent 规则面,否则 worker 仍会继承仓库开发型流程。
- 这轮修复不是去继续碰 parser,而是把验证场景的输入世界缩回到 example 真正想测试的边界,这一步很关键。

## [2026-03-31 02:34:16] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 任务名称: 收口 example 真后端回归 - all-hat overlay 运行时覆写与 `evidence_ok` 结构化断言

### 任务内容
- 让 `config/all_hat.md` 的 all-hat overlay 保持“编译期内嵌默认值”,同时允许 runtime 显式覆写。
- 让 `parallel-experimental-dev-engine-example` 在 E2E 中使用更轻的 overlay,减少示例场景的无关提示词噪音。
- 修正 `experiment.reviewed` 的 `evidence_ok` 统计方式,避免 YAML / JSON 形态差异造成误杀。

### 完成过程
- 在 `crates/ralph-core/src/config.rs` 新增 `core.all_hat_prompt` 配置层,支持:
  - `compiled`
  - `disabled`
  - `inline`
  - `file`
- 在 `crates/ralph-core/src/prompt_overlay.rs`、`crates/ralph-core/src/event_loop/mod.rs`、`crates/ralph-core/src/parallel/supervisor.rs` 接上新配置:
  - 串行路径读取失败时降级跳过 overlay
  - 并行路径读取失败时直接报错,避免默默带着错误配置继续跑
- 在 `crates/ralph-e2e/src/scenarios/parallel_experimental_dev_engine_example.rs`:
  - 增加 `E2E_LIGHT_ALL_HAT_PROMPT`
  - patch example 配置为 `core.all_hat_prompt.mode: inline`
  - 将 `payload_field_is_true()` 改成结构化解析
- 在 `crates/ralph-e2e/src/scenarios/parallel/mod.rs` 增加顶层 YAML block 替换辅助,避免 patch 配置时靠脆弱字符串拼接。

### 完成过程补充
- 回看了最终证据文件:
  - `.e2e-tests/report-live.md`
  - `.e2e-tests/report.json`
  - `.e2e-tests/parallel-experimental-dev-engine-example/.ralph/events.jsonl`
- 确认第二轮真后端结果已经稳定收口:
  - `Passed: 1 | Failed: 0`
  - `parallel-experimental-dev-engine-example (507.8s)`
  - `integration.applied`
  - `experiment.complete`
  - `No new jobs after LOOP_COMPLETE (example)` 通过

### 总结感悟
- 编译期内嵌 prompt overlay 很适合做默认值,但不适合做不可覆写常量。
- 对 example / E2E 这类场景,最稳的做法不是去改默认资产,而是保留默认,再给场景一个明确的 runtime override 出口。
- 结构化 payload 就应该用结构化解析。只靠字符串形态猜测字段值,迟早会被 YAML / JSON 的细节差异误伤。

## [2026-04-02 11:17:41] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 任务名称: 将 Rerun runtime graph 讨论收口成正式 OpenSpec change

### 任务内容
- 把“Rerun runtime graph”从 explore 讨论,整理成一份正式的 OpenSpec `proposal + design`。
- 明确保留 V1 / V2 两阶段路线,避免以后只剩下 V1 的记忆。
- 把这轮结果正式写回 `__memory_axes` 支线文件,避免下次重新加载上下文时失联。

### 完成过程
- 新建并填写了独立 change:
  - `openspec/changes/rerun-runtime-graphs/proposal.md`
  - `openspec/changes/rerun-runtime-graphs/design.md`
  - `openspec/changes/rerun-runtime-graphs/tasks.md`
  - `openspec/changes/rerun-runtime-graphs/specs/runtime-graph-observability/spec.md`
- 在 proposal / design / spec / tasks 中统一固定了几个关键边界:
  - `ralph hats graph` 继续负责静态 topology
  - Rerun graph 专门负责运行时关系图
  - V1 = `live runtime graph`
  - V2 = `durable replay graph`
- 重新执行了校验:
  - `openspec validate rerun-runtime-graphs --type change`
  - `openspec list --json`
- 确认结果:
  - `Change 'rerun-runtime-graphs' is valid`
  - `rerun-runtime-graphs` 当前 `status = in-progress`
  - `totalTasks = 15`

### 总结感悟
- 这轮最重要的不是“又多写了一份设计文档”,而是把最容易后面失联的两件事固定住了:
  - 静态图和运行时图不是一回事
  - V1 live graph 不等于 V2 replay graph
- 只要这两个边界还在,后续不管是谁回来继续做,都不容易把 Rerun 图误做成“另一份 Mermaid”,也不容易把 live demo 误当成完整 replay 能力。

## [2026-04-03 01:11:04] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 任务名称: apply `rerun-runtime-graphs` - 完成 V1 live runtime graph MVP

### 任务内容
- 为并行运行路径增加 Rerun `.rrd` 录制入口,把 runtime topology / workflow / delivery 三层关系以 V1 live graph 方式落盘。
- 保持 `ralph hats graph` 和 runtime graph 的职责边界清晰,不把这轮实现做成“另一份静态 Mermaid”。
- 把 V1 已完成部分正式写回 OpenSpec,同时保留 V2 durable replay 的剩余缺口。

### 完成过程
- 在 `crates/ralph-cli/src/main.rs` 为 `run` / `resume` 增加:
  - `--runtime-graph-rrd <FILE>`
  - 非并行模式 guard
- 在 `crates/ralph-cli/src/parallel_runner.rs`:
  - 初始化 `RuntimeGraphRecorder`
  - 接上 `instance_state_observer`
  - 接上 `event_observer`
  - 接上新的 `delivery_observer`
  - 在 run 结束时 flush / disconnect `.rrd`
- 新建 `crates/ralph-cli/src/runtime_graph.rs`:
  - 规范化节点 / 边集合
  - 输出三层 graph 视图
  - 增加 3 个单元测试
- 在 `crates/ralph-core/src/parallel/supervisor.rs` / `routing.rs`:
  - 新增 `RuntimeDeliveryMode`
  - 新增 `RuntimeDeliveryObservation`
  - 新增 `with_delivery_observer(...)`
  - 在 direct / queue / fanout / reply 投递时通知 live delivery 观察面
  - 在初始实例与动态实例进入 `Created` 时主动通知 state observer
- 新增 integration test:
  - `crates/ralph-cli/tests/integration_runtime_graph.rs`
  - 覆盖:
    - serial guard
    - parallel run 产生非空 `.rrd`
- OpenSpec 同步:
  - 更新 `proposal.md`
  - 更新 `design.md`
  - 更新 `tasks.md`
  - `openspec validate rerun-runtime-graphs --type change` 通过

### 完成过程补充
- 第一轮真实编译暴露了两个确定问题:
  - `deliver_to_instance_id()` 中 `event` move 后又读取字段
  - `ParallelLoopFlags` 仍保留 `Copy`,但新增字段是 `Option<PathBuf>`
- 已修正并重新验证。
- 还做了一次最小手工 smoke:
  - 临时并行配置 + custom backend 直接输出 `LOOP_COMPLETE`
  - 成功生成:
    - 非空 `runtime.rrd`
    - `record-session` 里 `CompletionPromise` 终止证据

### 总结感悟
- V1 live runtime graph 要想真的有用,关键不是先做 viewer,而是先把 observer 证据面接准。
- 这轮最重要的架构点是:
  - recipient 边不能只靠旧 durable log 猜
  - 必须有最小 live `delivery_observer`
- 当前 `rerun-runtime-graphs` 已经来到很清楚的状态:
  - `11/15` 完成
  - 剩余 `4/15` 全部属于 V2 durable replay

## [2026-04-08 08:22:28] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 任务名称: 核对 `rerun-runtime-graphs` 当前剩余执行项

### 任务内容
- 重新核对 `rerun-runtime-graphs` 的 OpenSpec 状态。
- 确认今天这条 change 还剩哪些执行项,以及这些剩余项属于哪一阶段。

### 完成过程
- 重新执行并核对了:
  - `openspec status --change 'rerun-runtime-graphs' --json`
  - `openspec instructions apply --change 'rerun-runtime-graphs' --json`
  - `openspec/changes/rerun-runtime-graphs/tasks.md`
- 确认当前真实状态:
  - `11/15` 完成
  - `4/15` 未完成
- 确认剩余项全部集中在 V2 durable replay:
  - `3.1`
  - `3.2`
  - `3.3`
  - `3.4`

### 总结感悟
- 这条 change 当前不缺“零散补丁”。
- 它缺的是一整段明确的 V2 durable replay 工作。
- 所以下一轮如果继续,最稳的方式不是横向扩 V1,而是直接把 V2 四项按顺序做完。
