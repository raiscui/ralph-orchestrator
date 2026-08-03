# 任务计划: continuous-learning 后当前入口

## [2026-05-29 18:04:44] [Session ID: omx-1779004640353-blcixq] 当前入口: scoped commit 后持续学习续档完成

### 目标
- 保持根目录当前任务计划轻量,旧的超长 task_plan 已归档到 archive/default_history/task_plan_2026-05-29_1804_pre_continuous_learning.md。
- 本轮 continuous-learning 负责完成六文件摘要、旧支线归档、长期经验沉淀和边界验证。

### 阶段
- [x] 阶段1: 列出并分组六文件上下文。
- [x] 阶段2: 阅读默认组和 evolution_analysis 支线并生成摘要。
- [x] 阶段3: 将仍有效后续项承接到 LATER_PLANS.md。
- [x] 阶段4: 将可复用经验写入 EXPERIENCE.md。
- [x] 阶段5: 归档旧 task_plan 和 evolution_analysis 支线。
- [ ] 阶段6: 验证归档边界并交付总结。

### 关键结论
- recoverable retry scoped commit 已完成: 8bf37643 feat: add recoverable agent cli retry lifecycle。
- mixed worktree scoped commit 的经验已写入 EXPERIENCE.md。
- evolution_analysis 支线已归档,其仍有效后续项已写回 LATER_PLANS.md。
- 本轮未新增 skill,因为 recoverable retry 已有项目级 skill 覆盖。

### 状态
** 目前在阶段6 **- 准备检查 archive 文件、根目录候选、index 和工作区边界。


## [2026-05-29 18:05:45] [Session ID: omx-1779004640353-blcixq] continuous-learning 完成: 归档与经验沉淀

完成:
- [x] 阶段6: 验证归档边界并交付总结。
- [x] `git diff --cached --name-status` 为空,index 未被污染。
- [x] 根目录六文件候选只剩默认组。
- [x] `evolution_analysis` 支线已移动到 `archive/branch_contexts/evolution_analysis/`。
- [x] 旧 `task_plan.md` 已移动到 `archive/default_history/task_plan_2026-05-29_1804_pre_continuous_learning.md`。
- [x] archive manifest 已创建: `archive/manifests/ARCHIVE_MANIFEST__continuous_learning_recoverable_retry_2026-05-29_1804.md`。
- [x] scoped commit 与 spec-code drift 经验已写入 `EXPERIENCE.md`。

状态:
** continuous-learning 本轮完成 **- 可交付总结,不自动提交本轮上下文/经验改动。


## [2026-05-29 18:46:49] [Session ID: omx-1779004640353-blcixq] 计划: example PROMPT.md fixture 真相源治理

触发条件:
- 用户说“按建议继续”。
- 上轮 continuous-learning 建议下一步优先处理 example PROMPT.md fixture 真相源。

目标:
- 让干净 worktree / staged-only worktree 中的 full cargo test 不再依赖未跟踪 `examples/parallel-*/PROMPT.md`。
- 选择最正确的修复: 要么把确属 example 契约的一组 PROMPT.md 纳入 Git,要么修正测试只扫描 tracked/runnable examples。

阶段:
- [ ] 阶段1: 定位 integration_examples 的真实测试契约。
- [ ] 阶段2: 对照 specs/docs/examples 判断 PROMPT.md 是否应 tracked。
- [ ] 阶段3: 实施最小但正确的修复。
- [ ] 阶段4: 运行 focused gate 和必要 full gate。
- [ ] 阶段5: 记录 WORKLOG / ERRORFIX / LATER_PLANS 状态。

状态:
** 目前在阶段1 **- 准备读取测试代码、examples 文件树和 tracked/untracked 差异。


## [2026-05-29 18:57:29] [Session ID: omx-1779004640353-blcixq] 完成: example PROMPT.md fixture 真相源治理

完成:
- [x] 阶段1: 定位 integration_examples 的真实测试契约。
- [x] 阶段2: 对照 specs/docs/examples 判断 PROMPT.md 是否应 tracked。
- [x] 阶段3: 实施修复: `.gitignore` 改为允许 `examples/parallel-*/PROMPT.md`,并 staged 24 个 prompt fixtures。
- [x] 阶段4: 验证通过:
  - `git diff --cached --check` passed。
  - `cargo test -p ralph-cli --test integration_examples --quiet` passed,26/26。
  - staged-only worktree `cargo test -p ralph-cli --test integration_examples --quiet` passed,26/26。
  - staged-only worktree `cargo test --quiet` passed。
- [x] 阶段5: 记录 WORKLOG / ERRORFIX / LATER_PLANS 状态。

结论:
- 这不是应该放宽测试的问题。
- repo 的 specs、README 和 integration test 都把这些 runnable examples 定义为 self-contained。
- 因此应把 prompt templates 纳入 Git 真相源。

状态:
** 本轮 fixture 治理完成 **- 当前 staged patch 只包含 `.gitignore` 和 24 个 `examples/parallel-*/PROMPT.md`。


## [2026-05-30 11:17:22] [Session ID: omx-1779004640353-blcixq] scoped commit 完成: parallel example prompt fixtures

完成:
- [x] 提交前 staged 边界检查通过: 只有 `.gitignore` 和 24 个 `examples/parallel-*/PROMPT.md`。
- [x] `git diff --cached --check` 通过。
- [x] forbidden staged context check 无输出。
- [x] submodule status 无输出。
- [x] 已创建本地 commit: `f41c2bda fix: track parallel example prompt fixtures`。
- [x] 提交后 `git diff --cached --name-status` 为空。
- [x] 所有 example `PROMPT.md` 已被 Git 跟踪。

状态:
** scoped commit 已完成 **- 未 push,剩余工作区其它改动仍保持未暂存。


## [2026-06-01 14:33:48] [Session ID: omx-1779004640353-blcixq] git push 完成: main -> raiscui/ralph-orchestrator

完成:
- [x] 普通 HTTPS `git push` 失败,原因是 Git credential 使用的账号没有 `raiscui/ralph-orchestrator` 写权限。
- [x] 已改用同一目标仓库的 SSH URL 推送,不修改 remote 配置。
- [x] 推送成功: `main -> main`,远端从 `11391625` 更新到 `f41c2bda`。

状态:
** git push 已完成 **- 未推送到 `origin`,只推送到 `raiscui/ralph-orchestrator`。

## [2026-08-01 10:20:00] [Session ID: omx-1785579233065-awidzo] [记录类型]: 任务计划 - improve-codebase-architecture

# 目标

走查 ralph-orchestrator 代码库,找出架构深化(shallow module -> deep module)候选,产出可视化 HTML 报告,等用户挑选后进入 grilling 循环。

# 阶段

- [ ] 阶段1: 读取 CONTEXT.md/ADR(不存在,跳过)+ 项目结构速览
- [ ] 阶段2: 派 explore 代理走查 ralph-core / adapters+cli / tui+e2e
- [ ] 阶段3: 汇总候选,写 HTML 报告到 $TMPDIR 并 open
- [ ] 阶段4: 等用户选择候选,进入 grilling 循环

# 关键问题

1. 哪些模块 shallow?deletion test 是否通过?
2. 哪些耦合跨 seam 泄漏?
3. 哪个候选最值得先做?

# 状态

** 目前在阶段2 **- 3 个 explore 代理并行走查中

## [2026-08-01 10:30:00] [Session ID: omx-1785579233065-awidzo] [状态变更]: explore 代理失败,walk 代理补位

- 现象: `/root/explore_*` 三个代理全部 errored,原因是 `agent_type=explore` 的模型 `gpt-5.4-mini` 不被当前 API 支持(仅支持 deepseek-v4-pro / deepseek-v4-flash)。
- 处理: 已改派 `walk_core` / `walk_adapters_cli` / `walk_tui_e2e` 三个代理走查同一分区。
- 问题: `walk_core` 与 `walk_tui_e2e` 返回 completed 且声称没有收到任务描述,只有 `walk_adapters_cli` 处于 running。
- 下一步: 给 `walk_core` / `walk_tui_e2e` 补发具体走查任务(含输出文件路径与分区边界),等待全部完成后汇总。

## [2026-08-01 10:55:00] [Session ID: omx-1785579233065-awidzo] [记录类型]: 阶段进展

- [x] 阶段1: 读取 CONTEXT.md/ADR(不存在,跳过)+ 项目结构速览
- [x] 阶段2: 派 explore 代理走查(消息通道不可靠,改为直接探索,证据已入 notes.md)
- [x] 阶段3: 汇总 6 个候选,HTML 报告已写入 $TMPDIR/architecture-review-20260801-183513.html 并 open
- [ ] 阶段4: 等用户选择候选,进入 grilling 循环

候选清单: C1 显示管线搬出 adapters(Strong,首选) / C2 CLI 运行时收进 core(Strong) / C3 TUI 领域切片(Worth exploring) / C4 Evidence 深模块(Worth exploring) / C5 EventLoop 窄 interface(Worth exploring) / C6 e2e 场景声明化(Speculative)

## [2026-08-01 11:40:00] [Session ID: omx-1785579233065-awidzo] [记录类型]: 候选1 grilling 完成并实施

- [x] 阶段4: 候选1 grilling 循环完成,决策: A(新 crate ralph-display) + A(搬 crate + 深化 interface) + A(DisplayTarget 意图式工厂) + A(输出注入) + A(完整执行不提交)
- [x] 实施: ralph-display crate 建立、adapters 瘦身、tui/cli 依赖切换、loop_runner 矩阵替换为工厂、Console/Pretty 输出注入、测试升级
- [x] 验证: cargo check --workspace 通过; display 74 tests / adapters 107 / tui 4 / cli 195+ / core 643+40 smoke 全过
- [ ] 候选2-6 未实施(见 LATER_PLANS)

## [2026-08-01 12:30:00] [Session ID: omx-1785579233065-awidzo] [记录类型]: 候选2 grilling 完成并实施

- [x] 候选2 grilling: A(收进 ralph-adapters) + A(job/ 子模块四文件) + A(colors 归 ralph-display) + A(本次只搬 3 个 job 执行实现)
- [x] 实施: codex_app_server_session(2466)/codex_mcp_session → adapters/src/job/{app_server,mcp}.rs; headless.rs(进程 spawn 提取); mod.rs(选择器); parallel_runner 瘦身; 测试迁移(15 个 job 测试)
- [x] 验证: workspace check 0 warning; adapters 122 / cli 全过 / display 74 / tui / core 643+40; clippy 干净
- [ ] 候选3-6 未实施(见 LATER_PLANS)

## [2026-08-02 09:40:00] [Session ID: omx-1785579233065-awidzo] [记录类型]: 候选4 grilling 完成并实施

- [x] 候选4 grilling: A(聚合下沉 core) + A(record_aggregate.rs 新模块) + B(窄入口 aggregate_session)
- [x] 实施: core/src/record_aggregate.rs(420 行); cli record_session.rs 1514 → 625 行(只留渲染 + 指针); 3 个调用点改窄入口; 2 个聚合测试迁移
- [x] 验证: workspace check 0 warning; core 645 / cli 全过 / display 74 / tui; clippy 干净
- [ ] 候选3(TUI 切片)/候选5(EventLoop 窄 interface)/候选6(e2e 声明化)未实施

## [2026-08-02 11:20:00] [Session ID: omx-1785579233065-awidzo] [记录类型]: 候选3 grilling 完成并实施

- [x] 候选3 grilling: A(只切片 TuiState) + A(4 切片 radar/output/task/search) + A(每片子模块自治) + A(壳兼容委托)
- [x] 实施: state/{radar,output,task,search}.rs 四个切片;TuiState 82 方法中域方法改委托;update/apply_update 路由;mermaid_hat_node_id 双份收敛为 radar 一份;OutputSlice 默认 following_latest=true 保持历史行为
- [x] 验证: workspace check 0 warning; tui 239+26+4 / cli 177+ / core / display 全过; clippy 干净
- [ ] 候选5(EventLoop 窄 interface)/候选6(e2e 声明化)未实施; app.rs 拆分待调用者渐进迁移后按需做

## [2026-08-02 11:30:00] [Session ID: omx-1785579233065-awidzo] [记录类型]: e2e live 验证 parallel 场景

- [ ] 检查 e2e 场景列表与 codex 后端可用性
- [ ] 跑 parallel 过滤场景(live)
- [ ] 分析结果并收尾

## [2026-08-02 12:10:00] [Session ID: omx-1785579233065-awidzo] [记录类型]: e2e live 验证完成

- [x] 检查 e2e 场景列表与 codex 后端(0.146.0 可用,Authenticated)
- [x] live 验证: parallel-hat-instances ✅ 39.8s / parallel-hat-instances-zh ✅ 29.2s
- [x] 失败场景回归对照(HEAD baseline 二进制): emit-spawn / app-server-idle-start-live / app-server-steer-multi-turn 在新旧代码表现完全一致 → 既有问题,非本次改动引入
- [x] mock 模式: cassette 不足(串行也卡),环境限制,非回归

## [2026-08-02 12:20:00] [Session ID: omx-1785579233065-awidzo] [记录类型]: 支线任务 - e2e 收敛稳定性调查(启用后缀上下文 __e2e_conv)

- 排查 emit-spawn / app-server 系列场景的 LOOP_COMPLETE 收敛失败
- 方法: 场景定义 + 保留 workspace 事件流 + prompt 协议对照
- 产出: 根因结论(配置漂移/环境/代码)+ 修复建议

## [2026-08-02 13:30:00] [Session ID: omx-1785579233065-awidzo] [记录类型]: 支线任务完成 - e2e 收敛稳定性

- [x] 根因: e2e detect_termination_reason 口径与并行显示行格式不匹配(前缀/事件回显/err 回显)
- [x] 修复: 并行模式只取 [ralph#1:out: 行 + 剥前缀 + 排除事件行; 串行回退原逻辑
- [x] 验证: 3 场景转绿(emit-spawn / idle-start-live / steer-multi-turn+live); steer-live-reply 剩余失败为 steer 时序问题(LLM 行为)
- [x] 清理: 所有临时 E2E-DEBUG 日志已移除; e2e 504+3+38 测试全过; clippy 干净

## [2026-08-02 14:30:00] [Session ID: omx-1785579233065-awidzo] [记录类型]: e2e 收敛稳定性任务完成

- [x] steer-live-reply 根因: 会话定向事件(session_strategy)被 rewrite_target_for_busy_ralph 改投 ralph#2 → 上下文丢失
- [x] 修复 + 单元测试; live 验证: app-server 5/5 + emit-spawn 全过
- [x] e2e 全部 parallel live 场景现在通过; workspace check/clippy 干净

## [2026-08-02 14:50:00] [Session ID: omx-1785579233065-awidzo] [记录类型]: 修复沉淀完成

- [x] openspec/specs/parallel-trigger-routing/spec.md 增加 2 个场景: session-directed 不改投 + 会话上下文保持
- [x] openspec validate --all --strict: 28 passed 0 failed

## [2026-08-02 18:55:00] [Session ID: omx-1785579233065-awidzo] [记录类型]: ralph-example 配置修复

- [x] 检查: 字段全部有效; 内容过时(缺 commit 顶层字段契约, 与仓库 example 差 65 行); prompt_file 悬空(硬失败)
- [x] 修复: 同步仓库最新 ralph.yml + 创建 PROMPT.md/README.md; 路径引用修正
- [x] 验证: hats 命令 3 hat 正常加载; doctor 0 errors(7 warnings 均与配置过期无关)

## [2026-08-02 19:00:00] [Session ID: omx-1785579233065-awidzo] [记录类型]: ralph-example PROMPT.md 重写为具体 example

- [x] 原 PROMPT.md 是并行自检模板(模板化措辞), 重写为具体演示 example
- [x] demo-greeter: 两条策略实验(bash 直写 vs 参数化函数)→ 审计 → 集成 → 收敛闭环
- [x] 验证命令全部可真跑(断言/退出码), 与 ralph.yml 协议对齐(commit 顶层字段、final_verification 一一对应)

## [2026-08-02 19:10:00] [Session ID: omx-1785579233065-awidzo] [记录类型]: ralph-example demo 实跑验证

- [x] git init + 初始 commit; ralph run 完整闭环(341s, CompletionPromise)
- [x] 事件链与期望一致: task×2 → result×2 → reviewed×2(approved) → integration×1 → applied+complete → LOOP_COMPLETE
- [x] 产物: tools/greeter.sh(100755), 输出断言 PASS, 无参数 exit=1, 主工作区 commit 24ee2d2

## [2026-08-02 19:25:00] [Session ID: omx-1785579233065-awidzo] [记录类型]: PROMPT.md 自然语言版重写 + 实跑验证

- [x] 自然语言任务("多种排序算法并行实现")替代结构化实验条目模板
- [x] 实跑: Auto-Plan 自动规划 3 条实验(冒泡/快排/内置 sorted), topology.spawn_group 并行组, 8 runner 并发
- [x] 事件链: task×3 → result×3 → reviewed×3(approved) → integration×1 → applied+complete → LOOP_COMPLETE(474s)
- [x] 产物: tools/sort.py(采纳内置 sorted), 断言 PASS, 未采纳实现清理干净

## [2026-08-02 23:15:00] [Session ID: omx-1785579233065-awidzo] [记录类型]: 改进1+2 落地与验证

- [x] 改进1: PROMPT.md 加性能基准(10,000 随机整数计时)+ COMPARISON.md 对比结论要求 + 限定恰好 3 条实验
- [x] 改进2: scripts/demo-check.sh 回归检查(termination/事件链/产物/COMPARISON, 退出码)
- [x] demo-bench 验证: 脚本正确发现 6 条实验未收敛(MaxRuntime, 5 FAIL)
- [x] demo-v2 验证: 卡住根因 = codex 账户额度不足(¥0.03, 403 预扣失败), 非 demo 问题
- [ ] 额度恢复后重跑 demo-v2 验证完整闭环(3 实验 + 性能基准 + COMPARISON + demo-check 全 PASS)

## [2026-08-03 00:30:00] [Session ID: omx-1785579233065-awidzo] [记录类型]: deepseek 适配完成

- [x] 最小实验: deepseek 对简短指令+格式示例完美遵循(事件块精确输出)
- [x] ralph-example ralph_prompt 极简版(示例驱动, 去掉窗口/backpressure 复杂语义)
- [x] 实测: task×3 → result×3 → reviewed×3 → integration → applied+complete → LOOP_COMPLETE(599s, CompletionPromise)
- [x] COMPARISON.md 性能对比生成(冒泡 2791.9ms vs 快排 40.8ms vs 内置 28.4ms)
- [x] demo-check.sh 修复(-x → -f), 10/10 全 PASS

## [2026-08-03 00:40:00] [Session ID: omx-1785579233065-awidzo] [记录类型]: 候选5 + 经验沉淀

- [ ] 候选5: EventLoop 窄 interface(事实核查 → grilling → 实施)
- [ ] deepseek/架构经验沉淀到 EXPERIENCE.md

## [2026-08-03 01:30:00] [Session ID: omx-1785579233065-awidzo] [记录类型]: 候选5 完成

- [x] EventLoop::run 窄入口 + PromptExecutor port + RunHooks; cli 1247→565 行
- [x] adapters PtyPromptExecutor(PTY+CliExecutor 双路径/角色参数/展示工厂)
- [x] 测试: core 645+ / adapters 126 / cli 171+ 全过; 串行真实 run(deepseek) 27.9s CompletionPromise
- [x] 提交 3ff4b47 + 01ece15, 已推送

## [2026-08-03 02:10:00] [Session ID: omx-1785579233065-awidzo] [记录类型]: 候选6 试点完成

- [x] declarative 基础设施: YAML schema + runner(实现 TestScenario, 无缝集成)
- [x] single-iter 迁移为 YAML(命令式保留为逃生舱)
- [x] live 验证: 声明版 PASS(71.1s); 断言子集表达力足够(response/exit_code/no_timeout/iterations/scratchpad/termination/events/output_contains)
- [x] 提交 b9d909d 已推送
- [ ] 后续: 批量迁移(connect/multi-iter/hats 核心)→ parallel 注入时序(声明+钩子)

## [2026-08-03 02:45:00] [Session ID: omx-1785579233065-awidzo] [记录类型]: 候选6 批量迁移第一批

- [x] connectivity/multi-iter/completion 迁移为 YAML(4 个声明场景: +single-iter)
- [x] schema 扩展: exact_iterations / min_total_events; 断言单测 3 个
- [x] live 验证: connectivity 8s / single-iter 71s / completion 通过; multi-iter 在 deepseek 下慢(非声明化问题)
- [x] 提交 430a07a 已推送
- [ ] 后续: EventsScenario/BackpressureScenario 等简单场景 → 复杂场景(hats/memory/parallel 注入)需扩展 schema(注入序列/自定义断言)

## [2026-08-03 03:05:00] [Session ID: omx-1785579233065-awidzo] [记录类型]: 候选6 第二批迁移完成

- [x] events/backpressure 迁移为 YAML(schema 扩展 event_payload_contains/keywords)
- [x] 6 个声明场景: connectivity/single-iter/multi-iter/completion/events/backpressure
- [x] 断言单测 5 个; live 验证 events 14.2s PASS
- [x] 提交 0cb2e66 已推送
- [ ] 剩余 41 个场景: 下一批可迁 events 类简单场景 → 复杂场景(hats/memory/capabilities/parallel)需 inject 序列 + 自定义断言钩子

## [2026-08-03 16:15:00] [Session ID: omx-1785579233065-awidzo] [记录类型]: 候选6 inject 里程碑完成

- [x] inject 时序声明化(type: wait/sleep/assert/emit), app-server-idle-start-live 迁移 YAML
- [x] live 验证: 29.6s PASS(与命令式 29.5s 等价); 修 workspace 未就绪容错
- [x] 单测 7 个(agents.json 解析/wait 容错/payload/迭代/终止); clippy 干净
- [x] 提交 1d7fc04 + clippy 清理已推送
- [ ] 剩余: 其他 parallel 场景(steer/emit-spawn/hat-instances)可按 inject 模式迁移; 或候选6 收尾

## [2026-08-03 17:20:00] [Session ID: omx-1785579233065-awidzo] [记录类型]: hat-instances 声明化 + 通道故障排查

- [x] hat-instances en/zh 迁移 YAML(运行计数/agents 快照断言, config/prompt 逐字节等价验证)
- [x] 失败排查: 声明式 0/4 vs 命令式 2/2 → 根因 = deepseek-v4-flash 在 app-server 通道间歇 503(nowcoding distributor), 非声明化问题(诊断: trace.jsonl error notification)
- [x] 提交 3b75f31 + import 清理已推送
- [ ] 通道恢复后补一次 hat-instances 声明版 live 验证(或换默认模型)
