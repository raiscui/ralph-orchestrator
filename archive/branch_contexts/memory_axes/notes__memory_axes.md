## [2026-03-21 18:10:25] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 笔记: `__memory_axes` 续档后的当前有效摘要与 `parallel-experimental-dev-engine-example` 新主假设

## 来源

### 来源1: 活跃支线六文件与归档快照

- 文件:
  - `task_plan__memory_axes.md`
  - `WORKLOG__memory_axes.md`
  - `LATER_PLANS__memory_axes.md`
  - `EPIPHANY_LOG__memory_axes.md`
  - `archive/branch_contexts/memory_axes/snapshots/2026-03-21_181025/notes__memory_axes_2026-03-21_181025.md`
- 要点:
  - 双轴 memory / scoped experience 的 explore 与 apply 结论都已落盘
  - examples E2E 排查已经证明:
    - 旧的 worker 不回流问题曾真实存在
    - 后续录制里 `experiment.result -> experiment.reviewed` 已 durable 落盘

### 来源2: 当前 E2E workspace 的 git 状态与 HEAD 内容

- 命令:
  - `git -C .e2e-tests/parallel-experimental-dev-engine-example status --short`
  - `git -C .e2e-tests/parallel-experimental-dev-engine-example show HEAD:PROMPT.md`
  - `git -C .e2e-tests/parallel-experimental-dev-engine-example show HEAD:examples/parallel-experimental-dev-engine/PROMPT.md`
- 要点:
  - 当前隔离 workspace 里:
    - `PROMPT.md`
    - `examples/parallel-experimental-dev-engine/PROMPT.md`
    - `examples/parallel-experimental-dev-engine/ralph.yml`
    都是未提交修改
  - `HEAD:PROMPT.md` 仍然是仓库旧内容,不是 E2E 预填实验计划
  - `HEAD:examples/parallel-experimental-dev-engine/PROMPT.md` 仍然是示例默认 smoke prompt

### 来源3: 场景 setup 与 worktree 语义

- 文件:
  - `crates/ralph-e2e/src/scenarios/parallel_experimental_dev_engine_example.rs`
  - `examples/parallel-experimental-dev-engine/ralph.yml`
- 要点:
  - 该场景 setup 的顺序是:
    - 先把仓库 clone 到 E2E workspace
    - 再在 workspace 工作树里写入 E2E 专用的 `PROMPT.md` 和 patched `ralph.yml`
  - 但 example 配置当前仍使用:
    - `parallel.workspace.worktree_backend: worktree`
    - hat `workspace.strategy: worktree`
  - `git worktree add --detach ... HEAD` 只会基于 `HEAD` 提交态创建新 worktree
  - 它不会自动带上源工作树里的未提交修改

## 综合发现

### 六文件摘要（当前有效）

- 这条支线的长期主题仍然是 `scoped-experience-system`
- 但当前活跃子任务已经切到:
  - `parallel-experimental-dev-engine-example` 录制为什么没有闭环回流
- 旧 `notes` 中关于 `<\\/event>` 的结论没有作废
  - 它对应的是一条已经被代码硬化覆盖的真实断点
- 但它不再足以解释当前看到的全部偏差

### 现象

- `exp-001` / `exp-002` 的 `experiment.result -> experiment.reviewed` 已 durable 落盘
- 当前 E2E workspace 里的 example 输入文件是“工作树已改,但 HEAD 未更新”的状态
- 该 example 又要求 runner / integrator 在 `worktree` 中工作

### 当前主假设

- `parallel-experimental-dev-engine-example` 当前更深层的不一致,不是“worker 完全没回流”
- 而是:
  - E2E setup 把实验计划和配置只写进了 workspace 工作树
  - 但并行 job 的 worktree 仍然从 `HEAD` 切出
  - 于是 job 看到的是旧版本仓库内容,不是 E2E 当前预填后的输入

### 最强备选解释

- `parallel/instance.rs` 的 repo root 解析仍可能存在额外问题
- 或者还存在 prompt/source 污染链
- 但在当前证据下,这些都还没有“未提交 patch + worktree 只看 HEAD”来得直接

### 验证

- 动态证据:
  - `git status --short` 明确显示 3 个关键输入文件还处于未提交修改
  - `git show HEAD:PROMPT.md` 明确显示 `HEAD` 里仍是旧 prompt
  - `git show HEAD:examples/parallel-experimental-dev-engine/PROMPT.md` 也仍是默认 smoke prompt
- 静态证据:
  - 场景 setup 确实是在 clone 后直接写文件,没有提交
  - example 配置确实使用 `worktree_backend: worktree`
  - `git worktree add --detach ... HEAD` 的语义天然只看提交态

### 结论

- 已验证结论:
  - “worker 侧完全不回流”已经不是当前主矛盾
  - `<\\/event>` parser hardening 是本轮之前已经真实存在、并且已经处理过的断点
  - 当前更强的新断点是:
    - E2E workspace 的输入 patch 没有进入 `HEAD`
    - 但 worktree job 又只能看到 `HEAD`
- 下一步最合理的修复方向:
  - 在场景 setup 完成 patch 后,把这些 E2E 输入提交成一个隔离 snapshot commit
  - 然后再让并行 worktree 从这个 snapshot `HEAD` 切出
  - 并补一个回归测试,确认 worktree 里看到的是 E2E 预填后的 `PROMPT.md`

## [2026-03-21 18:35:50] [Session ID: 68546] 笔记: `parallel-experimental-dev-engine-example` fresh 真实复跑结论

## 来源

### 来源1: fresh 真实复跑

- 命令:
  - `cargo run -p ralph-e2e -- codex --filter parallel-experimental-dev-engine-example --keep-workspace --verbose`
- 要点:
  - 本次 fresh 复跑最终 `PASSED`
  - 新 report:
    - `.e2e-tests/report.md`
    - `.e2e-tests/report.json`
  - 总耗时:
    - 约 `574.9s`

### 来源2: fresh workspace 事件链

- 文件:
  - `.e2e-tests/parallel-experimental-dev-engine-example/.ralph/events.jsonl`
- 要点:
  - 关键链按顺序再次出现:
    - `experiment.task` x2
    - `experiment.result` x2
    - `experiment.reviewed` x2
    - `integration.task`
    - `integration.applied`
    - `experiment.complete`(integrator)
    - `experiment.complete`(ralph#1 收敛确认)

### 来源3: fresh stdout artifact 与运行中 agent 自述

- 文件:
  - `.e2e-tests/parallel-experimental-dev-engine-example/.e2e/stdout.txt`
  - `.e2e-tests/parallel-experimental-dev-engine-example/ralph/log/experiment_integrator#1/task_plan.md`
  - `.e2e-tests/parallel-experimental-dev-engine-example/ralph/log/experiment_integrator#1/notes.md`
- 要点:
  - integrator 在运行中一度自述:
    - `git show 17ee424...` 是 `bad object`
    - 想改成“手动重建补丁”
  - 但交叉验证显示:
    - `git -C .e2e-tests/parallel-experimental-dev-engine-example show 17ee424...`
    - `git -C .e2e-tests/parallel-experimental-dev-engine-example show 61b367a...`
    都能成功
  - 因此这条只能算 integrator 当时的候选假设,不能当成已确认根因

## 综合发现

### 现象

- 上一轮真实 run 的失败点已经不再是“没有回流”,而是:
  - `No new jobs after LOOP_COMPLETE (example)`
  - `completion_seen=true, new_jobs_after=[("ralph#1", 5)]`
- 这次 fresh 真实复跑里:
  - 场景整体通过
  - 同一个断言没有再失败

### 当前主结论

- 已验证结论:
  - 原用户感知到的“没有回流”主问题,在当前代码下已经修住
  - 关键修复就是:
    - E2E seed 输入必须先提交到 snapshot `HEAD`
    - 再让 worktree job 从这个 `HEAD` 切出
  - fresh run 已经用真后端再次证明:
    - topic 回流链可以完整闭环到 `integration.applied` / `experiment.complete`

### 关于旧的 `job 5` 尾巴

- 当前只能下到“未稳定复现”的结论:
  - 上一轮 report 确实记录过 `new_jobs_after=[("ralph#1", 5)]`
  - 但这次 fresh run 没有重现
- 值得继续留意,但还不能下结论说已经找到它的根因

### 一个值得后续验证的方向

- 静态观察:
  - Supervisor 的 completion 判定使用 `EventParser::contains_promise(...)`
  - example 场景的断言则用“stdout 某行以 `LOOP_COMPLETE` 结尾”来判定 `completion_seen`
- 这两者语义并不完全等价
- 目前这只是候选假设:
  - 有可能旧的 `job 5` 失败属于断言口径偏松造成的假阳性
  - 但本轮没有复现,所以还缺动态证据

## [2026-03-21 22:00:49] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 笔记: `parallel-experimental-dev-engine-example` 旧 `job 5` 尾巴的最新证据收敛

## 来源

### 来源1: static trace - `completion` 后为何还可能再起一个 ralph job

- 文件:
  - `crates/ralph-core/src/parallel/supervisor.rs`
  - `crates/ralph-core/src/parallel/supervisor/routing.rs`
  - `crates/ralph-core/src/parallel/instance.rs`
- 要点:
  - Supervisor 在 `HatInstanceEvent::JobCompleted` 里看到 `ralph` 的 `completion_promise` 后:
    - 会停止继续派生**新的路由事件**
    - 进入 completion drain 窗口
  - 但它不会立刻冻结各 instance 已有的 `pending`
  - `HatInstanceActor::run()` 的 tick 仍会周期性调用 `maybe_start_job()`
  - 只要 `shutdown_requested=false` 且 `pending` 不空,同一实例仍可继续起新的 job

### 来源2: dynamic proof - 最小机制测试

- 新增测试:
  - `parallel::supervisor::routing_tests::supervisor_allows_prequeued_ralph_job_to_start_after_completion_promise`
- 验证命令:
  - `cargo test -p ralph-core supervisor_allows_prequeued_ralph_job_to_start_after_completion_promise`
- 关键结果:
  - 测试通过
  - 断言:
    - `result.termination == Some(TerminationReason::CompletionPromise)`
    - `emitter#1 == 1`
    - `ralph#1 == 3`
- 这条测试构造了:
  - `ralph#1` 第一次先派发 `build.task`
  - `emitter#1` 一次性返回两条 orphan event

## [2026-03-25 20:22:28] [Session ID: 8E08D4FA-9BA2-4C21-BDA5-DBB280CCE00F] 笔记: 旧 stalled run 与干净 seed run 的证据边界重新校准

## 来源

### 来源1: 旧 stalled workspace durable 证据

- 文件:
  - `.e2e-tests/parallel-experimental-dev-engine-example/.ralph/events.jsonl`
  - `.e2e-tests/parallel-experimental-dev-engine-example/.ralph/agents.json`
  - `.e2e-tests/parallel-experimental-dev-engine-example/.ralph/worktrees/`
- 要点:
  - 旧 workspace 的 durable 主流只包含:
    - `experiment.task` x1
    - `experiment.result` x1
    - `experiment.reviewed` x1
  - `events.jsonl` 里唯一的 task/result/reviewed 都对应 `exp-001`
  - `agents.json` 显示:
    - `experiment_runner#1` 收到了 `experiment.task`
    - `experiment_runner#3` 的 `last_input = None`
  - `.ralph/worktrees/` 下只有:
    - `experiment_runner_1`

### 来源2: 当前主仓库与干净 seed 的 git 基线差异

- 命令:
  - `git rev-parse HEAD && git ls-files e2e_marker_exp_001.txt e2e_marker_exp_002.txt`
  - `git -C /tmp/exp002-prehead-seed.9Pffup rev-parse HEAD && git -C /tmp/exp002-prehead-seed.9Pffup ls-files e2e_marker_exp_001.txt e2e_marker_exp_002.txt`
- 要点:
  - 当前主仓库 `HEAD = 97c8211da511e80861ddd720c364e04b27ee4bb1`
  - 主仓库 `HEAD` 已经带有:
    - `e2e_marker_exp_001.txt`
  - 干净 seed `/tmp/exp002-prehead-seed.9Pffup` 的 snapshot `HEAD = 805cdd137ca713f05f0f7d1be102cb3d23ea52d2`
  - 干净 seed 的 `HEAD` 中:
    - `e2e_marker_exp_001.txt`
    - `e2e_marker_exp_002.txt`
    都不存在

### 来源3: 干净 seed 的结构化 durable 证据

## [2026-03-31 02:07:03] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 笔记: `all_hat` 降噪落点的静态确认

## 来源

### 来源1: `prompt_overlay` / `event_loop` / `parallel supervisor`

- 文件:
  - `crates/ralph-core/src/prompt_overlay.rs`
  - `crates/ralph-core/src/event_loop/mod.rs`
  - `crates/ralph-core/src/parallel/supervisor.rs`
  - `crates/ralph-core/src/parallel/instance.rs`
- 要点:
  - 当前 `config/all_hat.md` 通过 `include_str!` 编译期内嵌到 `COMPILED_ALL_HAT_PROMPT`
  - 串行 `EventLoop::new` 与并行 `ParallelSupervisor::new` 都直接调用 `prompt_overlay::load_all_hat_prompt()`
  - 现有 `load_all_hat_prompt()` 不读取运行时配置,只会返回编译期内嵌正文
  - `sanitize_overlay_protocol_examples()` 只做 `<event>` 标签转义,不会过滤开发型长提示

### 来源2: `config.rs`

- 文件:
  - `crates/ralph-core/src/config.rs`
- 要点:
  - 目前 `CoreConfig` 只有:
    - `scratchpad`
    - `specs_dir`
    - `guardrails`
    - `workspace_root`
  - 还没有任何 all-hat overlay 的运行时配置位
  - `EventLoopConfig` 里已有 `ralph_prompt`,但它只作用于 `ralph#1`,不适合承载“所有 hat 共用 overlay”

### 来源3: example 场景 patch 点

- 文件:
  - `crates/ralph-e2e/src/scenarios/parallel_experimental_dev_engine_example.rs`
  - `examples/parallel-experimental-dev-engine/ralph.yml`
- 要点:
  - 该场景已经会在隔离 workspace 里 patch:
    - `cli` block
    - workspace root `AGENTS.md`
    - root/example `PROMPT.md`
  - 因此如果要让 example/E2E 显式选择轻量 overlay,最佳落点就是 patched `ralph.yml`

## 综合发现

### 现象

- 当前闭环已经被真后端 PASS 证明可跑通
- 但 `config/all_hat.md` 仍会无差别注入开发型规则到所有实例 prompt
- 这层注入目前无法被 example/E2E 局部关闭或替换

### 当前主假设

- 如果给 runtime 增加“all-hat overlay 来源配置”,并让 example/E2E 显式使用轻量 overlay,
  就能在不改仓库默认行为的前提下继续压低 worker 噪音与长尾

### 最强备选解释

- 也可能真正影响耗时的主因已经不是 `all_hat` 文本本身,而是:
  - 真实后端响应波动
  - example 协议文本长度
  - 其他 prompt source 的组合效应
- 因此本轮改完后仍需要真后端复跑,不能只凭静态阅读下结论

### 结论

- 已验证结论:
  - 运行时当前没有 all-hat overlay 配置位
  - `core` 是最自然的落点,因为该配置影响所有 hat,且不属于某个单独 workflow 入口
  - example/E2E 最稳的接入方式是: patched `ralph.yml` 显式选择轻量 overlay
- 下一步:
  - 实现一个保持默认兼容的 overlay source 配置
  - 为 example/E2E 添加轻量 overlay
  - 再用测试和真后端证据判断它是否真的带来收益

## [2026-03-25 21:01:40] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 笔记: `experiment_integrator#1` 尾巴断点的现象、主假设与备选解释

## 来源

### 来源1: 旧失败现场的 stdout / durable events / agents snapshot

- 文件:
  - `.e2e-tests/parallel-experimental-dev-engine-example/.e2e/stdout.txt`
  - `.e2e-tests/parallel-experimental-dev-engine-example/.ralph/events.jsonl`
  - `.e2e-tests/parallel-experimental-dev-engine-example/.ralph/agents.json`
- 要点:
  - durable 主流只走到:
    - `integration.task`
  - 缺失:
    - `integration.applied`
    - `experiment.complete`
  - `stdout.txt` 尾部显示:
    - `git cherry-pick 81b8c6e...` 成功
    - `rg -n "exp-001" e2e_marker_exp_001.txt` 成功
    - 随后没有任何 `[experiment_integrator#1:out:job=1] <event ...>` 正文事件
    - 最后整次 run 被 600s 顶层 E2E 超时打断,`experiment_integrator#1` 以 `failed` 收尾
  - `agents.json` 生成时仍显示:
    - `experiment_integrator#1.state = running`

### 来源2: integrator prompt transcript 中的全局上下文污染证据

- 命令:
  - `rg -n "文件上下文工作模式|task_plan|notes|WORKLOG|ERRORFIX|EPIPHANY|LATER_PLANS" .e2e-tests/parallel-experimental-dev-engine-example/.e2e/stdout.txt`
- 要点:
  - integrator 明确继承了仓库根 `AGENTS.md` 的“文件上下文工作模式”
  - 它在处理 `integration.task` 前后,读取了 workspace 根的:
    - `task_plan.md`
    - `EPIPHANY_LOG.md`
    - `LATER_PLANS.md`
    - `WORKLOG.md`
  - 它还在自己的 `ralph/log/experiment_integrator#1/` 下持续写 `task_plan.md` / `notes.md`
  - stderr transcript 里充满了与 example 目标无关的大量 diff / 续档 / 六文件动作

### 来源3: timeout 配置链

- 文件:
  - `examples/parallel-experimental-dev-engine/ralph.yml`
  - `crates/ralph-core/src/config.rs`
  - `crates/ralph-core/src/parallel/supervisor.rs`
  - `crates/ralph-cli/src/parallel_runner.rs`
- 要点:
  - `experiment_integrator.job_timeout_secs = 1800`
  - 默认 `output_stale_timeout_secs = 1800`
  - 本次 E2E 场景总超时只有 600s
  - 所以只要 integrator 在 600s 内没有完成,顶层场景就会先超时,不会等到 job-level stale-timeout 护栏出手

## 综合发现

### 现象

- 当前旧失败现场里,问题已经不是“事件发出来了但 parser 漏吃”。
- 更准确的现象是:
  - integrator 完成了实际 git/rg 操作
  - 但一直没有收敛出最终 assistant 正文事件
  - 顶层 E2E 先超时,再把 run 打断

### 当前主假设

- 主假设:
  - example E2E workspace 克隆了整个仓库根 `AGENTS.md`
  - 这个全局开发型 AGENTS 把 integrator 带进了重型“六文件 / 续档 / 工作日志”流程
  - 导致本来应该几步完成的 `integration.task` 被扩成了长尾任务,最终在 600s 场景护栏前没能产出 `integration.applied`

### 最强备选解释

- 备选解释:
  - integrator 本身的 hat instructions 仍不够强,即使没有仓库级 AGENTS 污染,也可能继续在“验证后如何组织最终事件”这一步漂移
  - 或者 Codex backend 在长 stderr / 多次 diff 回显后存在额外的输出收尾问题

### 能推翻主假设的证据

- 如果给 E2E workspace 覆盖一个极简根 `AGENTS.md` 后,真实复跑仍然卡在相同位置
- 或者在无全局 AGENTS 污染的前提下,integrator 仍然执行完 `git` / `rg` 却长期不发最终事件

### 当前结论

- 已验证结论:
  - 当前旧失败现场的主断点是“integrator 长时间未完成收尾”,不是“`integration.applied` 已输出但未被 parser 接住”
  - job-level timeout 护栏在这条场景里过长,不足以在 E2E 600s 护栏前先止损
- 尚未验证完成的部分:
  - “仓库根 AGENTS 污染”是不是导致 integrator 长尾的根因,还需要通过覆盖 workspace 根 `AGENTS.md` 的真实复跑来证伪

## [2026-03-25 21:09:31] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 笔记: 覆盖 E2E workspace 根 `AGENTS.md` 后,`parallel-experimental-dev-engine-example` 真后端复跑恢复闭环

## 来源

### 来源1: 代码改动与单测

- 文件:
  - `crates/ralph-e2e/src/scenarios/parallel_experimental_dev_engine_example.rs`
- 改动:
  - 在 scenario setup 中新增 `write_workspace_root_agents_override(workspace)`
  - 在隔离 workspace 根目录覆盖极简 `AGENTS.md`
  - 覆盖内容明确要求:
    - 只按当前 hat instructions + incoming event 做最小动作
    - 不读取/维护仓库级六文件
    - verification 完成后直接输出 workflow 事件
- 验证命令:
  - `cargo test -p ralph-e2e scenarios::parallel_experimental_dev_engine_example::tests::seeded_workspace_snapshot_commit_makes_patched_prompt_visible_to_worktree -- --exact`
  - `cargo test -p ralph-e2e scenarios::parallel_experimental_dev_engine_example::tests::example_config_requires_structured_commit_fields_for_review_and_integration -- --exact`
- 结果:
  - 两条单测均通过

### 来源2: 真后端 E2E 复跑

- 命令:
  - `cargo run -p ralph-e2e -- codex --filter parallel-experimental-dev-engine-example --keep-workspace --verbose`
- report:
  - `.e2e-tests/report.json`
- 要点:
  - `passed = true`
  - 总耗时约 `323.4s`
  - 关键断言全部通过:
    - `Exit code = 0`
    - `No timeout`
    - `Required topic chain observed (example)`
    - `No new jobs after LOOP_COMPLETE (example)`

### 来源3: durable 事件链与 integrator 现场

- 文件:
  - `.e2e-tests/parallel-experimental-dev-engine-example/.ralph/events.jsonl`
  - `.e2e-tests/parallel-experimental-dev-engine-example/.e2e/stdout.txt`
  - `.e2e-tests/parallel-experimental-dev-engine-example/ralph/log/experiment_integrator#1/task_plan.md`
  - `.e2e-tests/parallel-experimental-dev-engine-example/ralph/log/experiment_integrator#1/notes.md`
- 要点:
  - durable 主流已出现:
    - `integration.applied`
    - `experiment.complete`
  - stdout 里可见:
    - `experiment_integrator#1:out:job=1` 明确输出两条最终事件
    - `ralph#1:out:job=4` 输出 `LOOP_COMPLETE`
  - integrator 本地日志显示:
    - cherry-pick 成功
    - final verification 成功
    - 阶段4已完成

## 综合发现

### 现象

- 旧失败现场:
  - integrator 做完 `git cherry-pick` + `rg`
  - 但顶层 run 在 600s 前没有看到 `integration.applied` / `experiment.complete`
- 修复后的真复跑:
  - 同一 scenario 在 `323.4s` 内完成
  - 关键链路完整闭环

### 当前主结论

- 已验证结论:
  - 对这个 E2E scenario 来说,在隔离 clone 的 workspace 根目录覆盖极简 `AGENTS.md` 后,真后端复跑恢复通过
  - 这说明“workspace 根 AGENTS 继承”至少是一个被证实的高影响污染源
  - 修复后 integrator 不再停在“阶段4 只整理证据不发事件”的旧失败口径上

### 仍需谨慎的边界

- 目前还不能把所有历史长尾都绝对归因给这一处
  - 因为 `config/all_hat.md` 仍然会把实例级 `ralph/log/<hat>/...` 规则注入 prompt
  - 当前通过说明“剩余提示词负担可接受”,不等于“未来再无同类漂移”
- 但对本轮修复来说:
  - 动态证据已经足够支撑“这次改动把 scenario 拉回 PASS”

### 当前结论

- 该问题当前最稳的口径是:
  - 不是 parser / escaped event 问题
  - 是 E2E example workspace 需要显式隔离仓库级开发型 AGENTS,避免 worker 被拖进错误的工作流

- 文件:
  - `/tmp/exp002-prehead-seed.9Pffup/.ralph/events.jsonl`
  - `/tmp/exp002-prehead-seed.9Pffup/full-run.jsonl`
  - `/tmp/exp002-prehead-seed.9Pffup/.ralph/agents.json`
  - `/tmp/exp002-prehead-seed.9Pffup/.ralph/worktrees/`
- 要点:
  - 只按结构化 `bus.publish` 统计时,当前干净 seed 已 durable 到:
    - `experiment.task` x2
    - `experiment.result` x2
  - 两条 `experiment.result` 的 commit 分别是:
    - `exp-001 -> 332d12ccd459d51ac49b1d4cc8c9ab65aeb5e2be`
    - `exp-002 -> 7c89adda56a6104c4d58fc52c4fb3e7c6452bec4`
  - `agents.json` 显示:
    - `experiment_runner#1` 收到了 `exp-001`
    - `experiment_runner#3` 收到了 `exp-002`
    - `experiment_auditor#1` 与 `experiment_auditor#2` 都处于 `running`
  - `.ralph/worktrees/` 下存在:
    - `experiment_runner_1`
    - `experiment_runner_3`
  - `full-run.jsonl` 末尾存在:
    - `_meta.termination.reason = "Interrupted"`
  - 因此这个干净样本当前只能证明:
    - 当前代码下 `exp-001` 与 `exp-002` 都能 durable 派发并各自产生 `experiment.result`
    - 但这一份录制尚不能证明 `experiment.reviewed -> integration.task -> integration.applied`

## 综合发现

### 现象

- 旧 stalled workspace 真实发生过 durable 断层:
  - 主流只走到了 `exp-001`
- 干净 seed 样本已经推翻了“当前代码仍稳定缺 `exp-002`”这条旧主假设:
  - 现在至少能稳定看到双 `experiment.task` + 双 `experiment.result`
- 但干净 seed 的这份录制被人为中断了:
  - 所以 `reviewed / integration` 还没有拿到完成态证据

### 当前主假设

- 旧 stalled run 里的“只 durable 到 `exp-001`”更像是历史样本现象。
- 它目前还不能直接外推成“当前代码仍然稳定缺 `exp-002`”。
- 当前最需要补的动态证据已经不是 `exp-002` 是否派发,而是:
  - 在干净 seed 上,完整链路能否自然走到
    - `experiment.reviewed`
    - `integration.task`
    - `integration.applied`

### 最强备选解释

- 仍然存在一种可能:
  - 旧 stalled run 里的 durable 断层是某个低频漂移问题
  - 它在当前代码下并未彻底消失,只是这次没有复现
- 但在没有新的干净复现之前,不能把这条备选解释升级成当前根因

### 已验证结论

- 已验证:
  - 当前主仓库 `HEAD` 自己带有 `exp-001` marker,因此不能再拿“直接从当前主仓库 HEAD 起跑的样本”判断 `exp-001` 相关根因
  - 干净 seed `805cdd137ca713f05f0f7d1be102cb3d23ea52d2` 没有这类污染
  - 在这个干净 seed 上,双 experiment 至少都能 durable 到 `experiment.result`
- 尚未验证:
  - 干净 seed 的完整链路是否已经稳定闭环到 `integration.applied`
  - 旧 stalled run 的 durable 断层是否仍可在当前代码下复现

## [2026-03-25 20:38:40] [Session ID: 8E08D4FA-9BA2-4C21-BDA5-DBB280CCE00F] 笔记: `parallel-experimental-dev-engine-example` 本轮 timeout 的真正断点收敛到“HTML 转义事件未被 durable parser 吃进”

## 来源

### 来源1: 本轮真实 E2E 失败结果

- 命令:
  - `cargo run -p ralph-e2e -- codex --filter parallel-experimental-dev-engine-example --keep-workspace --verbose`
- 结果:
  - 场景在 `600.1s` timeout 后失败
  - `.e2e-tests/report.md` / `.e2e-tests/report.json` 明确记录:
    - `task=2`
    - `result=1`
    - `reviewed=1`
    - `integration.task = 0`
    - `integration.applied = 0`

### 来源2: durable 主事件流与实例快照

- 文件:
  - `.e2e-tests/parallel-experimental-dev-engine-example/.ralph/events.jsonl`
  - `.e2e-tests/parallel-experimental-dev-engine-example/.ralph/agents.json`
- 要点:
  - durable 主流只落到了:
    - `experiment.task` x2
    - `experiment.result` x1
    - `experiment.reviewed` x1
    - `human.message` x1
  - `agents.json` 最终显示:
    - `experiment_runner#3` 已 `done`
    - `experiment_auditor#1` 已 `done`
    - `ralph#1` 也已 `done`
  - 说明这不是“worker 还在慢慢跑”,而是 run 已经实质收尾

### 来源3: `stdout.txt` 中 `experiment_runner#3` 的原始输出

- 文件:
  - `.e2e-tests/parallel-experimental-dev-engine-example/.e2e/stdout.txt`
- 关键动态证据:
  - `experiment_runner#3` 明确执行并成功完成了:
    - `rg -n "exp-002" e2e_marker_exp_002.txt`
    - `git add -A`
    - `git -c user.name="ralph" -c user.email="ralph@local" commit -m "exp-002: e2e marker file"`
    - `git show --name-only --oneline HEAD`
    - `git rev-parse HEAD`
  - `stdout.txt` 中还能直接看到它的最终正文输出:
    - `[experiment_runner#3:out:job=1] &lt;event topic="experiment.result" reply="5Y7WNcJwe6Rw"&gt;`
    - 随后有完整 payload:
      - `run_id: e2e`
      - `experiment_id: exp-002`
      - `status: success`
      - `commit: 3451d87b76e7955f6cfedafdb00bcb4c00380282`
  - 但注意:
    - 这里输出的是 **HTML 转义后的** `&lt;event ...&gt;`
    - 不是原始 `<event ...>`

### 来源4: `exp-002` commit 的对象级证据

- 命令:
  - `git -C .e2e-tests/parallel-experimental-dev-engine-example cat-file -t 3451d87b76e7955f6cfedafdb00bcb4c00380282`
  - `git -C .e2e-tests/parallel-experimental-dev-engine-example show --name-only --stat 3451d87b76e7955f6cfedafdb00bcb4c00380282`
  - `git -C .e2e-tests/parallel-experimental-dev-engine-example fsck --no-reflogs --unreachable --no-progress | rg '3451d87|857bbc5'`
- 要点:
  - `3451d87...` 是真实存在的 commit 对象
  - commit 时间是:
    - `2026-03-25 20:29:31 +0800`
  - commit 内容包含:
    - `e2e_marker_exp_002.txt`
    - `ralph/log/experiment_runner#3/task_plan.md`
  - 同时这条 commit 当前处于:
    - `unreachable commit 3451d87...`
  - 这与 durable 主流缺 `experiment.result(exp-002)` 一致:
    - 实验产物存在
    - 但没有被主事件流消费并推进到 auditor

## 综合发现

### 现象

- `exp-002` 并不是“没有执行”
- `exp-002` 也不是“没有产出 commit”
- `exp-002` 甚至不是“没有在 stdout 里写出 result”
- 真正发生的是:
  - `experiment_runner#3` 把 result 以 `&lt;event ...&gt;` 的 HTML 转义形式写到了 stdout
  - durable 事件流没有把它识别成真实事件

### 当前主假设

- 当前最强主假设是:
  - 并行 runtime 的 stdout event parser 只接受原始 `<event ...>`
  - 不接受 HTML 转义后的 `&lt;event ...&gt;`
  - `experiment_runner#3` 恰好在本轮输出了被转义的事件,因此 `experiment.result(exp-002)` 丢失

### 最强备选解释

- 备选解释是:
  - 不是 parser 完全不支持 HTML 转义

## [2026-03-25 20:49:12] [Session ID: 0537D10D-AB29-46A7-B336-BC309E3EC274] 笔记: 新静态证据要求回滚“直接放开通用 parser”的方案

## 来源

### 来源1: `event_parser.rs` 当前静态实现

- 文件:
  - `crates/ralph-core/src/event_parser.rs`
- 要点:
  - `find_event_start()` 只查找原始 `<event`
  - opening tag 结束只查找原始 `>`
  - closing tag 只兼容:
    - `</event>`
    - `<\\/event>`
  - 因此“HTML 转义事件没被 parser 吃进”这条静态判断成立

### 来源2: 仓库内现有协议文本与注释

- 文件:
  - `examples/parallel-experimental-dev-engine/ralph.yml`
  - `examples/*/README.md`
  - `crates/ralph-core/src/prompt_overlay.rs`
- 要点:
  - 多个 example / README 明确写着:
    - `&lt;event ...&gt;` 是展示文本,不是正式发布事件
    - 如果某个 lane 只打印了转义的 `&lt;event ...&gt;`,不要把它当成 ready / result
  - `prompt_overlay.rs` 还专门把 protocol 示例转义,并有测试说明:
    - 这样做是为了避免 accidental event replay

### 来源3: `output_for_parsing` 生成链

- 文件:
  - `crates/ralph-cli/src/parallel_runner.rs`
  - `crates/ralph-cli/src/codex_app_server_session.rs`
- 要点:
  - 并行 runtime 真正喂给 `EventParser` 的不是任意日志,而是 `HatJobResult.output_for_parsing`
  - 这层已经承担“只保留 stdout / 最终 assistant 文本”的语义整理职责
  - 当前 example 使用的是 `codex exec`,对应 `parallel_runner.rs::finalize_output_for_parsing()`

## 综合发现

### 现象

- 动态证据仍然表明:
  - `experiment_runner#3` 的真实最终回复以 `&lt;event topic="experiment.result"&gt;...&lt;/event&gt;` 开头
  - 这导致 durable 主流漏掉了 `exp-002`
- 但新增静态证据同时表明:
  - 仓库当前协议故意把“转义 event 展示文本”排除在正式事件之外

### 旧主假设为何要回滚

- 上一轮更直接的想法是:
  - 在 `EventParser` 里直接兼容 `&lt;event ...&gt;`
- 现在不能直接这么做,因为这会把下面两类文本也纳入正式事件解析:
  - prompt / overlay / README 中故意转义的协议示例
  - future worker 输出里引用的转义示例文本

### 当前主假设

- 更稳的修复点不应该是“放开通用 parser 协议”。
- 更合适的是:
  - 保持 `EventParser` 仍只认原始 `<event ...>`
  - 但在 `output_for_parsing` 形成的最后一层,对“非常窄、非常明确的 Codex 最终回复模式”做归一化
- 这条窄模式至少满足:
  - 输出去掉前导空白后直接以 `&lt;event` 开头
  - 存在匹配的 `&lt;/event&gt;` 或 `&lt;\\/event&gt;`
  - 只解码 tag 边界,不做全量 HTML unescape

### 最强备选解释

- 如果后面证明 `codex app_server` 也会产出同类 escaped final reply,可能还要把同样的窄归一化补到 app_server 路径
- 但当前 example 走的是 `codex exec`,所以第一落点仍应是 `parallel_runner.rs::finalize_output_for_parsing()`

### 当前结论

- 已验证结论:
  - “直接改 parser 接受所有 escaped event”不是当前最佳修复
  - “在 `output_for_parsing` 入口做窄归一化”更符合仓库现有协议与防误判设计
- 下一步:
  - 先为 `finalize_output_for_parsing()` 补失败测试
  - 再实现“仅处理 leading escaped event block”的归一化
  - 而是 `codex app-server / exec` 某条路径把 `runner#3` 的输出二次转义了,导致 parser 看到的根本不是原始 event
- 这条备选解释和主假设并不冲突:
  - 前者解释“为什么只有 runner#3 发生”
  - 后者解释“为什么 durable 主流没吃进去”

### 已验证结论

- 已验证:
  - `runner#3` 完整跑完了 implementation 与 verification
  - `runner#3` 产出了真实 commit `3451d87...`
  - `runner#3` 在 stdout 中输出了 `experiment.result`
  - 但该输出使用的是 `&lt;event ...&gt;` 而不是 `<event ...>`
  - `.ralph/events.jsonl` 没有收录这条 result
- 暂未验证:
  - 应该把修复落在:
    - stdout 事件解析器支持 HTML 转义 event
    - 还是上游输出层禁止/规避转义

## [2026-03-24 12:29:30] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 笔记: completion freeze 方案的当前证据边界与真后端复核计划

## 来源

### 来源1: completion freeze 实现代码

- 文件:
  - `crates/ralph-core/src/parallel/instance.rs`
  - `crates/ralph-core/src/parallel/supervisor.rs`
  - `crates/ralph-core/src/parallel/supervisor/routing_tests.rs`
- 要点:
  - `HatInstanceHandle` 已新增共享 `completion_freeze_requested: Arc<AtomicBool>`
  - Supervisor 在检测到 completion promise 且 `!pause_on_completion_promise` 时,会先对全部实例请求 freeze
  - actor 会在 `tick`、收命令、job 完成、`maybe_start_job()` 等关键入口主动消费这个 freeze 状态
  - 回归测试已从“允许 prequeued job 在 completion 后继续起跑”改成“completion 后冻结 prequeued job”

### 来源2: example 场景断言口径

- 文件:
  - `crates/ralph-e2e/src/scenarios/parallel_experimental_dev_engine_example.rs`
  - `crates/ralph-e2e/src/executor.rs`
- 要点:
  - scenario 里的 `No new jobs after LOOP_COMPLETE (example)` 仍是基于 stdout 行解析 `job_id`
  - 它要回答的问题是:
    - `LOOP_COMPLETE` 被看到之后,stdout 里是否又出现了新的 `job_id`
  - 这和 Supervisor 内部“是否已经进入 completion promise 终止态”不是完全同一层语义

## 综合发现

### 现象

- 已观察到的事实:
  - completion freeze 代码已经在仓库里
  - 定向回归测试和 `cargo test` 已经通过
  - 但目前还没有新鲜的真后端证据,证明 `parallel-experimental-dev-engine-example` 的历史 `job 5` 尾巴也随之消失

### 当前主假设

- completion 后再起尾巴 job 的已知机制之一,就是:
  - Supervisor 已停止继续派生新路由
  - 但某实例内部 `pending` 里还留着 prequeued job
  - actor 的下一次 tick 仍可能把它启动
- 当前实现正是针对这条机制做了“共享 freeze + pending 清空”的收紧

### 最强备选解释

- 即便 runtime 语义已经收紧,scenario 仍可能因为 stdout 观察口径而报出新的尾巴现象
- 例如:
  - completion 识别时点与 stdout 行出现顺序不完全一致
  - 或 artifact 里仍有旧输出残留 / 观察误差

### 验证计划

- 先跑:
  - `cargo run -p ralph-e2e -- codex --filter parallel-experimental-dev-engine-example --keep-workspace --verbose`
- 然后同时检查:
  - `.e2e-tests/report.md`
  - `.e2e-tests/report.json`
  - `.e2e-tests/parallel-experimental-dev-engine-example/.e2e/stdout.txt`
  - `.e2e-tests/parallel-experimental-dev-engine-example/.ralph/events.jsonl`
- 重点回答两个问题:
  - 真后端下,旧 `job 5` 尾巴是否还会出现
  - 如果还出现,它更像 runtime 真实再起 job,还是 scenario 口径问题

### 当前结论

- 已验证结论:
  - 方案 2 已经在代码层落地
  - 它针对的是“completion 后 pending 仍能继续起跑”的真实机制
- 尚未验证的部分:
  - 这条修复是否已经在真后端 example 场景里彻底消灭历史 `job 5` 尾巴

## [2026-03-24 12:39:51] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 笔记: `parallel-experimental-dev-engine-example` 本轮真后端复核停在 completion 之前

## 来源

### 来源1: 真后端执行命令

- 命令:
  - `cargo run -p ralph-e2e -- codex --filter parallel-experimental-dev-engine-example --keep-workspace --verbose`
- 要点:
  - 命令真实启动了 `ralph-e2e`
  - 随后启动了:
    - `target/debug/ralph run -c examples/parallel-experimental-dev-engine/ralph.yml --max-iterations 40 --no-tui`
    - `codex app-server`
    - `experiment_runner#1` 的 `codex exec`
  - 因为 run 长时间无新进展,最终用 `Ctrl-C` 手动中断

### 来源2: 主事件流

- 文件:
  - `.e2e-tests/parallel-experimental-dev-engine-example/.ralph/events.jsonl`
- 要点:
  - 只观察到 3 个关键事件:
    - `experiment.task(exp-001)`
    - `experiment.result(exp-001)`
    - `experiment.reviewed(exp-001, evidence_ok=true)`
  - 未观察到:
    - `experiment.task(exp-002)`
    - `integration.task`
    - `integration.applied`
    - `experiment.complete`
    - `LOOP_COMPLETE`

### 来源3: workspace 内的动态执行证据

- 文件 / 命令:
  - `.e2e-tests/parallel-experimental-dev-engine-example/.ralph/worktrees/experiment_runner_1/job-1/ralph/log/experiment_runner#1/task_plan.md`
  - `git -C .e2e-tests/parallel-experimental-dev-engine-example/.ralph/worktrees/experiment_runner_1/job-1 log --oneline -n 5`
  - `cat .e2e-tests/parallel-experimental-dev-engine-example/.ralph/worktrees/experiment_runner_1/job-1/e2e_marker_exp_001.txt`
- 要点:
  - `experiment_runner#1` 已完成:
    - 创建 `e2e_marker_exp_001.txt`
    - 提交 `a871f0b exp-001: e2e marker file`
    - 跑完 verification
  - 这证明 worker 不是“完全没工作”,而是 `exp-001` 确实跑通了

### 来源4: coordinator 自身记录

- 文件:
  - `.e2e-tests/parallel-experimental-dev-engine-example/ralph/log/ralph#1/task_plan.md`
- 要点:
  - `ralph#1` 记录了:
    - 首批应派发 `exp-001` 与 `exp-002`
    - `exp-001` 审计已完成
    - “等待 exp-002”
  - 但主事件流中没有看到 `exp-002` 真的被发出

## 综合发现

### 现象

- 本轮 run 没有进入 completion 区域
- 因而没有机会触发:
  - `No new jobs after LOOP_COMPLETE (example)` 这条断言真正关心的时刻
- 目前最前面的停滞点是:
  - `exp-001` 已完成并审计通过
  - `exp-002` 没有继续派发或至少没有 durable 落盘

### 当前主假设

- 当前更需要先解释的是:
  - 为什么 coordinator 只 durable 地派发了 `exp-001`
  - 然后在收到 `exp-001 reviewed` 后停住
- 这比“completion 后有没有 job 5 尾巴”更靠前
- 因为 run 还没到 completion,后者当前根本无从验证

### 最强备选解释

- 也存在另一种可能:
  - `exp-002` 实际曾在某个未 durable 的输出里出现
  - 但没有进入主事件日志
- 不过在当前证据下,更稳的表述仍然是:
  - “未观察到 `exp-002` 被 durable 派发”
  - 还不能直接说“绝对没有派发过”

### 结论

- 已验证结论:
  - 这轮真后端 run 不能作为“completion freeze 已经在 example 中消灭旧 `job 5` 尾巴”的证据
  - 原因不是旧 `job 5` 被复现了
  - 而是它根本没有走到 completion
- 当前更前置的新现象:
  - `parallel-experimental-dev-engine-example` 在本轮停在 `exp-001 reviewed` 之后
  - `exp-002` 未进入主事件流
  - `ralph#1` 第二次在延迟后输出 `LOOP_COMPLETE`
  - 第二条 orphan event 已经在这之前进入 `ralph#1.pending`
  - 结果 `ralph#1` 仍然起了第三次 job

### 来源3: artifact 保留策略

- 文件:
  - `crates/ralph-e2e/src/executor.rs`
- 现象:
  - `.e2e/stdout.txt` 之前只保留前 `200_000` 个字符
  - 当前 fresh 成功 run 里,`stdout.txt` 看不到 `LOOP_COMPLETE` 尾巴
  - 这不是因为 run 没 completion,而是因为 artifact 只保留前段
- 已做改动:
  - `truncate_with_notice()` 改为保留 `head + tail`
- 验证命令:
  - `cargo test -p ralph-e2e test_truncate_with_notice_preserves_head_and_tail`
  - `cargo test -p ralph-e2e test_truncate_with_notice_returns_original_when_short_enough`
- 关键结果:
  - 两条单测均通过

## 综合发现

### 现象

- 历史上出现过:
  - `No new jobs after LOOP_COMPLETE (example)` 失败
  - `completion_seen=true, new_jobs_after=[("ralph#1", 5)]`
- 当前 fresh 真录制没有复现这个现象
- 但当前 runtime 内部确实存在一种机制:
  - `completion` 之前已经进入 `ralph#1.pending` 的事件
  - 可以在 `LOOP_COMPLETE` 之后继续起成新的 `ralph` job

### 当前主假设

- 最强候选假设:
  - 旧 `job 5` 尾巴很可能不是“completion 之后又路由了新的外部/下游事件”
  - 而是“某条内部事件在 completion 之前已经被投进 `ralph#1.pending`,随后在 drain 窗口内自然起跑”

### 最强备选解释

- 备选解释1:
  - scenario 对 mixed stdout human log 的扫描口径仍可能制造假象
  - 尤其是当断言只看 `[instance:out|err:job=n] ...` 文本,而不是 runtime 内部状态
- 备选解释2:
  - 历史 run 里也可能同时存在“断言口径偏松 + prequeued pending 机制”两个因素

### 什么证据会推翻当前主假设

- 如果未来拿到那次旧失败的完整 tail 证据,显示:
  - `job 5` 对应的输入并不是 completion 前已投递的内部事件
  - 而是 completion 之后才新产生/新路由的事件
- 那么“prequeued pending 是主要来源”这条主假设就会被削弱

### 结论

- 已验证结论:
  - 当前 runtime 的 completion 语义,只阻止“completion 之后继续路由新事件”
  - 但**不会**阻止“completion 之前已经排队好的 `ralph#1` pending job”在 drain 窗口内起跑
  - 这条机制已经被单测动态证明
- 尚未验证到位的部分:
  - 历史上的旧 `job 5` 是否就是这条机制导致
  - 因为那次 run 的完整 tail artifact 没有保留下来

## [2026-03-31 02:34:16] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 笔记: `parallel-experimental-dev-engine-example` 最终收口 - all-hat overlay 运行时覆写与 `evidence_ok` 误杀修复

## 来源

### 来源1: 最终 live report 与 mtime

- 文件:
  - `.e2e-tests/report-live.md`
  - `.e2e-tests/report.json`
- 要点:
  - 当前最新报告时间戳一致:
    - `Mar 31 02:32:08 2026`
  - 最终 live report 显示:
    - `**Passed:** 1 | **Failed:** 0`
    - `parallel-experimental-dev-engine-example (507.8s)`
  - `report.json` 显示:
    - `"passed": true`
    - `"verdict": "All tests passed"`
    - `No new jobs after LOOP_COMPLETE (example)` = `passed: true`

### 来源2: durable event 证据

- 文件:
  - `.e2e-tests/parallel-experimental-dev-engine-example/.ralph/events.jsonl`
- 要点:
  - durable 主链已完整出现:
    - `experiment.task` x3
    - `experiment.result` x3
    - `experiment.reviewed` x3
    - `integration.task`
    - `integration.applied`
    - `experiment.complete`
  - 当前可直接回看的 `experiment.reviewed` payload 明确包含:
    - YAML 形态的 `evidence_ok: true`
    - `approved` / `rejected` 混合 verdict
  - 最终也已 durable 落盘:
    - `integration.applied`
    - `experiment.complete`

### 来源3: 本轮代码与回归验证

- 文件:
  - `crates/ralph-core/src/config.rs`
  - `crates/ralph-core/src/prompt_overlay.rs`
  - `crates/ralph-core/src/event_loop/mod.rs`
  - `crates/ralph-core/src/parallel/supervisor.rs`
  - `crates/ralph-e2e/src/scenarios/parallel/mod.rs`
  - `crates/ralph-e2e/src/scenarios/parallel_experimental_dev_engine_example.rs`
- 要点:
  - `core.all_hat_prompt` 已支持:
    - `compiled`
    - `disabled`
    - `inline`
    - `file`
  - example / E2E 现在显式注入轻量 `inline` overlay,不再强依赖编译期默认大段提示词
  - `payload_field_is_true()` 已改为结构化解析,不再依赖单一字符串形态

### 来源4: 同 session 第一轮 live run 的观察边界

- 事实:
  - 同一 session 中,第一轮真后端 live run 曾出现“workflow 实际闭环,但 scenario 仍判 fail”的现象
- 证据边界:
  - 那轮失败报告已被第二轮 PASS 报告覆盖
  - 因此当前工作区里可持久回看的直接证据,以:
    - 最终 PASS 报告
    - durable events
    - 新增回归测试
    为主

## 综合发现

### 现象

- 当前最终真后端结果已经明确 PASS。
- 但本 session 的上一轮 live run 曾出现过:
  - workflow durable 事件链已经到达收尾
  - scenario 却没有正确统计 `evidence_ok=true`
- 同时,example / E2E worker 仍会吃到编译期内嵌的 all-hat overlay,提示词噪音偏重。

### 当前主假设

- 主假设1:
  - scenario 对 `experiment.reviewed` 的 `evidence_ok` 判断过于脆弱
  - 它依赖某一种字符串布局,没有把 YAML / JSON / 空格差异都当成同一语义
- 主假设2:
  - all-hat overlay 作为编译期内嵌默认值本身没问题
  - 真正的问题是它缺少运行时显式覆写出口
  - 结果 example / E2E 也被迫继承开发型重提示词

### 最强备选解释

- 备选解释1:
  - 第一轮 live run 的 fail 也可能同时叠加了别的长尾因素
  - 例如真实后端输出节奏差异
- 备选解释2:
  - 仅靠 workspace 根 `AGENTS.md` override 也许已经足够让场景通过
  - all-hat overlay 降噪不一定是“必须项”,而是“更稳更干净的收口项”

### 验证

- 动态证据:
  - `.e2e-tests/report-live.md` 明确显示:
    - `Passed: 1`
    - `parallel-experimental-dev-engine-example (507.8s)`
  - `.e2e-tests/report.json` 明确显示:
    - `Required topic chain observed (example)` 通过
    - `No new jobs after LOOP_COMPLETE (example)` 通过
    - `counts: task=3, result=3, reviewed=3, evidence_ok=3`
  - `.ralph/events.jsonl` 明确显示:
    - `experiment.reviewed` payload 中真实存在 `evidence_ok: true`
    - `integration.applied` / `experiment.complete` 已 durable 落盘
- 静态证据:
  - `core.all_hat_prompt` 新配置把 overlay 来源从“写死 compiled”变成“默认 compiled,但可 runtime override”
  - `payload_field_is_true()` 已改为 `serde_yaml::Value` 结构化读取,不再依赖文本格式细节
- 回归验证:
  - `cargo fmt`
  - `cargo test -p ralph-core --lib`
  - `cargo test -p ralph-core smoke_runner`
  - `cargo test -p ralph-e2e scenarios::parallel_experimental_dev_engine_example::tests::seeded_workspace_snapshot_commit_makes_patched_prompt_visible_to_worktree -- --exact`
  - `cargo test -p ralph-e2e scenarios::parallel_experimental_dev_engine_example::tests::patch_example_config_for_e2e_adds_lightweight_all_hat_overlay -- --exact`
  - `cargo test -p ralph-e2e scenarios::parallel_experimental_dev_engine_example::tests::payload_field_is_true_accepts_yaml_and_both_json_spacing_styles -- --exact`
  - `cargo run -p ralph-e2e -- codex --filter parallel-experimental-dev-engine-example --keep-workspace --verbose`

### 结论

- 已验证结论:
  - `parallel-experimental-dev-engine-example` 当前已经在真后端闭环通过
  - `Required topic chain observed (example)` 和 `No new jobs after LOOP_COMPLETE (example)` 都已变成稳定 PASS
  - `experiment.reviewed` 的 `evidence_ok` 断言此前存在误杀风险,现在已改成结构化解析
  - all-hat overlay 更合理的产品形态是:
    - 编译期内嵌作为默认值
    - runtime 明确允许 `compiled / disabled / inline / file` 覆写
- 仍需谨慎的边界:
  - 第一轮 live run 被误杀的原始失败报告已被覆盖
  - 因此这条结论现在主要依赖:
    - 同 session 观察
    - 当前 durable 事件
    - 新增测试与最终 PASS 报告
  - 不是依赖那份已经不存在的旧 report 文件

## [2026-03-31 02:34:16] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 笔记: 探索 Rerun graph 集成 - Ralph 并行实例关系图的可视化建模

## 来源

### 来源1: Ralph 并行 runtime 关键代码

- 文件:
  - `crates/ralph-core/src/parallel/supervisor.rs`
  - `crates/ralph-core/src/parallel/supervisor/routing.rs`
  - `crates/ralph-core/src/parallel/instance.rs`
  - `crates/ralph-core/src/parallel/command_queue.rs`
  - `crates/ralph-core/src/agents_snapshot.rs`
  - `crates/ralph-core/src/event_logger.rs`
  - `crates/ralph-proto/src/event.rs`
  - `crates/ralph-proto/src/routing.rs`
- 要点:
  - `ParallelSupervisor::spawn_instances()` 负责创建所有静态 hat instances,并始终补上 `ralph#1`
  - `routing::spawn_instance()` / `spawn_dynamic_instance()` 负责动态实例创建
  - `HatInstanceActor` 通过 `Deliver / CancelCurrentJob / Shutdown` 接收控制命令
  - `HatInstanceEvent::StateChanged / JobCompleted / Published` 是 instance -> supervisor 回流主通道
  - `CommandQueue` 不是 agent 节点,更像资源串行化器,尤其是 `workspace.git` lane

### 来源2: 当前 durable 观测数据

- 文件:
  - `.e2e-tests/parallel-experimental-dev-engine-example/.ralph/events.jsonl`
  - `.ralph/agents.json` 的结构定义位于 `crates/ralph-core/src/agents_snapshot.rs`
- 要点:
  - `events.jsonl` 当前稳定记录:
    - `source_instance`
    - `id`
    - `reply`
    - `topic`
    - `triggered`
  - 但 `EventRecord` 没有持久化:
    - `target_instance`
    - fanout recipients
  - `dispatch.decision` 例外,它能记录:
    - `candidates`
    - `chosen_instance`
  - `.agents.json` 只提供实例状态摘要:
    - `instance_id`
    - `hat_id`
    - `state`
    - `is_dynamic`
    - `last_input`
  - 没有 creator / parent / delivered_from 之类关系字段

### 来源3: Rerun 官方 graph 能力

- 官方参考:
  - `https://rerun.io/examples/feature-showcase/graphs`
  - `https://rerun.io/docs/reference/types/archetypes/graph_nodes`
  - Context7 `/rerun-io/rerun`
- 要点:
  - Rerun 的 `GraphView` 可以显示 time-varying graph
  - `GraphNodes` 支持:
    - `node_ids`
    - `positions`
    - `colors`
    - `labels`
    - `radii`
  - `GraphEdges` 支持有向/无向图
  - 官方 graphs example 明确强调:
    - force-based layout
    - 适合节点关系与不同布局形态展示

## 综合发现

### 现象

- Ralph 当前并行 runtime 天然就是一张图:
  - supervisor 创建 instance
  - event 在 instances 间流动
  - queue/fanout/target_instance 决定边的投递语义
  - completion freeze / shutdown 改变节点生命周期
- 但当前 durable 证据并不完整:
  - 可以可靠知道“谁发出了事件”
  - 不能总是可靠知道“事件最终投递给了哪个实例”

### 当前主假设

- 主假设1:
  - Rerun 最适合承载的是“混合图”而不是单一图:
    - 一层是稳定控制拓扑
    - 一层是运行时实例状态
    - 一层是消息边/回复边
- 主假设2:
  - 如果只依赖现有 `.ralph/events.jsonl` 离线重建图,消息边会有盲区
  - 尤其是:
    - `target_instance` 直达
    - fanout recipients
    - 没有 `dispatch.decision` 的普通投递

### 最强备选解释

- 备选解释1:
  - 如果只想画“谁存在、谁当前在跑、最近收到什么”,那 `.agents.json + events.jsonl` 已经够用
  - 不一定非要先补 delivery 级观测点
- 备选解释2:
  - 如果采用 live observer 而不是纯离线 replay,当前 `output_observer / instance_state_observer / event_observer` 已能覆盖大半场景
  - 只是还差一个 dedicated delivery observer 才能完整

### 验证

- 静态证据:
  - `supervisor.rs`:
    - `spawn_instances()` 创建静态实例
    - `shutdown_instances()` 发送 `CancelCurrentJob` 和 `Shutdown`
    - `freeze_pending_on_all_instances()` 在 completion 收敛时冻结 pending
  - `instance.rs`:
    - `HatInstanceActor::run()` 管理 `Idle -> Running -> Idle/Done/Failed`
    - `HatInstanceCommand::Deliver / CancelCurrentJob / Shutdown` 是控制面入口
  - `routing.rs`:
    - `route_event()` 决定 direct / contract / trigger / escalate
    - `deliver_to_instance_id()` / `deliver_fanout()` 是真正的投递动作
    - `dispatch.decision` 是 queue 选择的唯一 durable recipient 证据
  - `event_logger.rs`:
    - `EventRecord` 确实只持久化 `source_instance` / `reply` / `triggered`
    - 没有 `target_instance`
- 动态证据:
  - `.e2e-tests/.../events.jsonl` 的真实样本里:
    - 能看到 `source_instance`
    - 能看到 `reply`
    - 看不到普通事件的 `target_instance`
    - 这证明“仅靠现有 durable log 复原完整投递图”确实有信息缺口
- 外部资料:
  - Rerun 官方 graph 文档确认了:
    - `GraphNodes`
    - `GraphEdges`
    - `GraphView`
    - force-based layout
    - time-varying graph

### 结论

- 已验证结论:
  - Rerun 在产品形态上很适合 Ralph 并行 runtime 关系图
  - 最适合的不是一张“大而全”的图,而是至少两层:
    - 控制/实例层
    - 消息/工作流层
  - 当前若做“纯离线 replay 图”,会缺少一部分最终投递边证据
  - 当前若做“live graph”,可以先基于现有 observer + state/event log 快速起步
- 推荐图模型:
  - 节点:
    - `supervisor`
    - `ralph#1` / `ralph#2`
    - `<hat>#<n>`
    - `workspace.git lane`
    - 可选: `topic::<event_topic>` 或 `workflow::<run_id>`
  - 边:
    - `creates`
    - `delivers`
    - `replies_to`
    - `publishes`
    - `freezes`
    - `shutdowns`
    - `uses_workspace_lane`
  - 时间维:
    - 用节点颜色 / 半径表达 state
    - 用消息边表达当前时刻或窗口内流量

### 一个更稳的分层草案

- View A: Runtime Topology Graph
  - 看谁存在、谁创建谁、谁当前 Running/Idle/Done
- View B: Workflow Event Graph
  - 看 `task.start -> experiment.task -> experiment.result -> ... -> experiment.complete`
- View C: Delivery / Reply Trace
  - 看 `source_instance -> target_instance`
  - 若 target 不完整,至少先看 `source_instance -> topic` 与 `reply -> original_event`

## [2026-04-02 11:17:41] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 笔记: `rerun-runtime-graphs` OpenSpec change 已正式成稿并锁定 V1 / V2

## 来源

### 来源1: 新建的 OpenSpec artifacts

- 文件:
  - `openspec/changes/rerun-runtime-graphs/proposal.md`
  - `openspec/changes/rerun-runtime-graphs/design.md`
  - `openspec/changes/rerun-runtime-graphs/tasks.md`
  - `openspec/changes/rerun-runtime-graphs/specs/runtime-graph-observability/spec.md`
- 要点:
  - `proposal.md` 明确写了为什么现有 `ralph hats graph` 和 `.ralph/events.jsonl` 还不足以表达用户要的 node-like runtime graph
  - `design.md` 明确写了这是“运行时关系图观测模型”,不是“把 Mermaid 换成 Rerun”
  - `tasks.md` 把 V1 / V2 分成 15 个任务,没有把 durable replay 遗忘在聊天里
  - `spec.md` 用 requirement 固定了:
    - 静态图与运行时图是两个产品
    - V1 先用现有 live observability
    - V2 必须依赖 durable delivery / lifecycle evidence

### 来源2: 重新执行的 OpenSpec 校验

- 命令:
  - `openspec validate rerun-runtime-graphs --type change`
  - `openspec list --json`
- 要点:
  - 校验结果:
    - `Change 'rerun-runtime-graphs' is valid`
  - 当前 change 状态:
    - `name = rerun-runtime-graphs`
    - `status = in-progress`
    - `totalTasks = 15`
    - `completedTasks = 0`

### 来源3: 当前 runtime 证据边界

- 文件:
  - `crates/ralph-cli/src/hats.rs`
  - `crates/ralph-core/src/parallel/supervisor.rs`
  - `crates/ralph-core/src/parallel/supervisor/routing.rs`
  - `crates/ralph-core/src/parallel/instance.rs`
  - `crates/ralph-core/src/event_logger.rs`
- 要点:
  - `ralph hats graph` 当前是静态 topology 图,不是运行时关系图
  - 当前 durable log 稳定有:
    - `source_instance`
    - `reply`
    - `topic`
    - `triggered`
  - 当前 durable log 仍缺:
    - `target_instance`
    - fanout recipients
    - dynamic creator lineage
    - freeze / cancel / shutdown 的完整 durable control edges

## 综合发现

### 为什么这次必须单独建 change

- 这次讨论的核心已经不只是“怎么画一张图”。
- 它实际上在定义:
  - Ralph 以后如何区分静态 topology 可视化
  - live 运行时关系图
  - durable replay graph
- 如果把它硬塞进:
  - `startup-resource-bootstrap`
  - `runtime-capability-invocation`
  这些 change 里,后面很容易又回到“顺手提了一嘴 Rerun”,而不是一套可执行设计。

### 为什么一定要同时写 V1 / V2

- 当前主结论:
  - V1 适合先做 `live runtime graph`
  - V2 才是 `durable replay graph`
- 这个拆分不是节奏问题,而是证据边界决定的:
  - live 侧已经有 observer 可接
  - durable 侧还缺完整投递和生命周期证据
- 如果不明确分层,最容易出现的漂移就是:
  - 做了一个 live demo
  - 然后团队误以为“runtime graph 已经完整了”

### 已验证结论

- `rerun-runtime-graphs` 已经是正式 OpenSpec change,不是聊天草稿。
- V1 / V2 已同时记录在:
  - proposal
  - design
  - tasks
  - spec requirements
- runtime graph 和 `ralph hats graph` 的边界已经正式固定:
  - 前者是运行时关系图
  - 后者是静态 topology 图
- V2 replay graph 不能在缺少 durable recipient / lifecycle evidence 的情况下被宣称为 full-fidelity reconstruction。

### 推荐的后续实施顺序

- 第一步:
  - 先实现 V1 live runtime graph
  - 优先接:
    - `output_observer`
    - `instance_state_observer`
    - `event_observer`
- 第二步:
  - 再单独补 V2 需要的 durable 记录
  - 尤其是:
    - `target_instance`
    - fanout recipients
    - creator lineage
    - freeze / cancel / shutdown control edges

## [2026-04-03 01:11:04] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 笔记: apply `rerun-runtime-graphs` V1 live runtime graph MVP

## 来源

### 来源1: OpenSpec 规格与任务状态

- 文件:
  - `openspec/changes/rerun-runtime-graphs/proposal.md`
  - `openspec/changes/rerun-runtime-graphs/design.md`
  - `openspec/changes/rerun-runtime-graphs/specs/runtime-graph-observability/spec.md`
  - `openspec/changes/rerun-runtime-graphs/tasks.md`
- 要点:
  - 这条 change 明确区分:
    - `ralph hats graph` = 静态 topology
    - Rerun runtime graph = 运行时关系图
  - 当前正式进度已更新为:
    - `completedTasks = 11`
    - `remainingTasks = 4`
  - 剩余 4 项全部属于 V2 durable replay graph:
    - `3.1`
    - `3.2`
    - `3.3`
    - `3.4`

### 来源2: Context7 / Rerun SDK

- Library ID:
  - `/rerun-io/rerun`
- 查询主题:
  - `RecordingStreamBuilder::save`
  - `GraphNodes`
  - `GraphEdges`
- 要点:
  - Rust SDK 支持将 recording 直接保存成 `.rrd`
  - `GraphNodes` 支持:
    - labels
    - colors
    - radii
    - show_labels
  - `GraphEdges` 支持 directed graph 表达
  - 这足够承载 V1 的三层视图:
    - runtime topology
    - workflow
    - delivery / reply

### 来源3: 当前实现与编译验证

- 关键代码:
  - `crates/ralph-cli/src/main.rs`
  - `crates/ralph-cli/src/parallel_runner.rs`
  - `crates/ralph-cli/src/runtime_graph.rs`
  - `crates/ralph-core/src/parallel/supervisor.rs`
  - `crates/ralph-core/src/parallel/supervisor/routing.rs`
- 已实现事实:
  - CLI 新入口:
    - `ralph run --runtime-graph-rrd <FILE>`
    - `ralph run --continue --runtime-graph-rrd <FILE>`
  - 并行模式护栏:
    - 非 `parallel.enabled=true` 时,显式报错拒绝该参数
  - V1 `.rrd` 记录器:
    - `Runtime Topology Graph`
    - `Workflow Event Graph`
    - `Delivery / Reply Trace`
  - live 观测面:
    - `instance_state_observer`
    - `event_observer`
    - 新增最小 `delivery_observer`
  - 为了拿到 create 边,`spawn_instances()` / `spawn_instance()` 在 `Created` 时主动通知 observer

### 来源4: 动态验证证据

- 编译 / 测试:
  - `cargo test -p ralph-core supervisor`
  - `cargo test -p ralph-cli runtime_graph`
  - `cargo test -p ralph-cli --test integration_runtime_graph`
  - `cargo build`
  - `cargo test -p ralph-core smoke_runner`
  - `cargo test`
- 结果:
  - 全部通过
- 最小手工 smoke:
  - 使用临时并行配置 + custom backend `sh -c "printf 'LOOP_COMPLETE\n'"`
  - 命令:
    - `target/debug/ralph run --config <tmp>/ralph.yml --no-tui --record-session <tmp>/session.jsonl --runtime-graph-rrd <tmp>/runtime.rrd`
  - 关键结果:
    - `EXIT_CODE=0`
    - `RUNTIME_RRD_BYTES=55951`
    - `record summary` 显示 `Termination reason = CompletionPromise`

## 综合发现

### 这轮 V1 真的已经闭环到哪里

- 不是只写了 design。
- 也不是只把 Rerun 依赖加进来了。
- 当前已经形成可运行的 V1 MVP:
  - CLI 可显式要求录制 `.rrd`
  - 并行 runtime 会把 topology / workflow / delivery 三层关系写入 artifact
  - 有 integration test 锁住:
    - parallel-only guard
    - non-empty `.rrd` artifact

### 为什么 V1 不能只靠旧 durable log 猜 recipient

- 本轮已验证结论:
  - `events.jsonl` 仍然不适合直接重建完整 recipient 图
  - 尤其缺:
    - 最终 `target_instance`
    - fanout recipients
    - create / spawn lineage
    - lifecycle control edges
- 因此 V1 的 live graph 要成立,必须加最小 live delivery 观察面
- 这就是本轮新增 `delivery_observer` 的原因

### 当前 V2 剩余缺口非常清晰

- 不是“还有些细节没做”
- 而是明确还有一条独立路线没开始:
  - delivery-level durable records
  - replay reconstruction order
  - lifecycle control durable evidence
- 所以下一轮如果继续,应当直接从:
  - `3.1`
  - `3.2`
  - `3.3`
  - `3.4`
 继续,而不是再回头讨论 V1 入口
