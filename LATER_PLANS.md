# LATER_PLANS

> 说明: 记录本次不落地, 但值得后续跟进的事项. 仅追加到文件末尾.
> 
> 历史归档: `LATER_PLANS__2026-08-13.md` (1058 行, 50+ 段, 2026-08-13 续档前内容, 保留作历史账本).
> 续档触发: LATER_PLANS.md 达到 1058 行 (> 1000 行阈值), 按 AGENTS.md "文件上下文工作模式" 规则重命名 + 新建.
> 续档时间: 2026-08-13 22:05 (Session `omx-1786600320381-z290x9`).

## [2026-08-13 22:05:00] [Session ID: omx-1786600320381-z290x9] Wave 3.4 follow-up: physical removal of deprecated imperative structs (target 2.3.0 release)

### 来源
- OpenSpec tasks.md §3.4 (commit `73cf1fa` + 下一 commit 完成):
  "Open a follow-up issue / change tracker for eventual physical removal
  of the imperative structs after one release cycle."

### 触发条件
- 2.3.0 release day (一个 release cycle after 2.2.x)
- 当前 2.2.2 release 周期内, 21 个 imperative struct 标 `#[deprecated(since = "2.3.0", ...)]`

### 待执行 (Wave 3.4 follow-up)
- 删除 21 个 imperative TestScenario impl structs:
  - crates/ralph-e2e/src/scenarios/errors.rs: TimeoutScenario / MaxIterationsScenario /
    BackendUnavailableScenario / AuthFailureScenario
  - crates/ralph-e2e/src/scenarios/hats.rs: HatSingleScenario / HatMultiWorkflowScenario /
    HatInstructionsScenario / HatEventRoutingScenario / HatBackendOverrideScenario
  - crates/ralph-e2e/src/scenarios/memory.rs: MemoryAddScenario / MemorySearchScenario /
    MemoryInjectionScenario / MemoryPersistenceScenario / MemoryCorruptedFileScenario /
    MemoryMissingFileScenario / MemoryRapidWriteScenario / MemoryLargeContentScenario
  - crates/ralph-e2e/src/scenarios/capabilities.rs: ToolUseScenario / StreamingScenario
  - crates/ralph-e2e/src/scenarios/parallel/app_server_idle_start.rs: ParallelAppServerIdleStartScenario
  - crates/ralph-e2e/src/scenarios/parallel/app_server_steer_multi_turn.rs: ParallelAppServerSteerMultiTurnScenario
- 删除对应 `#[allow(deprecated)]` 修饰 (errors / capabilities / hats / memory / parallel mod.rs pub use)
- 删除对应 `mod tests` 块
- ParallelExperimentalDevEngineExampleScenario (§2.5.0 explicit-keep) 保留

### 验证步骤
- cargo check -p ralph-e2e: 0 error 0 warning (不再有 297 deprecation warnings)
- cargo test -p ralph-e2e --lib: 全过 (从 536 减到 ~470, 减 21 个 impl 的 unit tests)
- cargo test -p ralph-e2e --test declarative_coverage_gate: Coverage 100.00% / PASS
  (registry 不变, 21 declarative scenarios 仍全部跑通)
- cargo run -p ralph-e2e -- --list: 60 declarative + 1 explicit-keep = 61 scenarios 全部可见

### 决策点
- 是否同时删除 `crates/ralph-e2e/docs/e2e/declarative-migration.md` 中 "历史命令式 impl" 表格? — 保留 (作为历史记录, 但加 "已物理删除 (2.3.0 release)" 标注)
- 是否升级 Cargo.toml version 到 2.3.0? — 是 (本 work 是 minor breaking change for crate internal API)

### cli.command 关联
- 命令式 BackendUnavailableScenario 的 `cli.command: nonexistent-cli-...` 在
  `crates/ralph-core/src/config.rs:795-803` 仍会被静默忽略 (仅 `cli.backend == "custom"` 时生效).
- declarative 路径修复见 commit `af3fbf8` (Task 2 batch).
- 命令式本身的语义问题随着本条 2.3.0 物理删除自然解决 (BackendUnavailableScenario
  命令式 impl 被删, 不再有静默忽略路径).

## [2026-08-13 22:05:00] [Session ID: omx-1786600320381-z290x9] ~~e2e-live-convergence 诊断 — 解 exp-20260813-e2e-live-convergence-issue~~ ✅ DONE (root cause: OpenAI Codex balance $0.009910 < $0.103358, see EXP entry + .e2e-tests/parallel-app-server-idle-start-live/)

### 来源
- EXP-20260813-e2e-live-convergence-issue (EXPERIENCE.md): 3 个 live 场景失败模式
  (termination_reason=None, 事件流完整但无 loop.terminate); 根因未知, 留 Wave 3 期间
  诊断.

### 环境阻塞 (2026-08-13 现状)
- ANTHROPIC_API_KEY / KIRO_API_KEY / OPENCODE_API_KEY 全未设置.
- `kiro` binary 不在 PATH (`claude` 在 `/Users/cuiluming/n/bin/claude`,
  `opencode` 在 `/Users/cuiluming/.opencode/bin/opencode`, `kiro` not found).
- ralph binary `target/debug/ralph` 存在.
- 结论: 当前无法跑 live harness, 诊断需用户配合 (提供 API key + 装 kiro) 或同意
  只做理论分析 + 模拟验证.

### 待执行
- 抓 `human-log.md` 看协调者最后输出 (是什么阻止 LOOP_COMPLETE)
- 抓 `agents.json` 看 ralph#1 状态转换 (是否进入 Running 后未回到 Idle)
- 减少 max_iterations 看是否 early termination (排除 max_runtime 提前收掉)
- 对比 declarative 版本 vs live 版本行为差异 (declarative 跑通, live 仍 fail)
- 定位后:
  1. 修命令式或 ralph app-server runtime
  2. 把 exp-20260813-e2e-live-convergence-issue 升级到 docs/solutions/ formal capture
     (problem_type: runtime_error 或新增 live_convergence 类型)
  3. 加 Wave 3 验证 checklist

### 触发条件
- 用户提供 API key + 装 kiro 后
- 或用户显式调用 `$continuous-learning 解 e2e-live-convergence-issue` 时执行

### 2026-08-15: minimax live e2e 重跑 (post-sync verification)

**来源**: sync/origin-v2.10.1 收尾时的 B verification 步骤失败

**现象**:
- `RALPH_E2E_CODEX_PROFILE=minimax cargo run -p ralph-e2e -- codex --filter parallel-emit-spawn-instance`
- 失败断言: LOOP_COMPLETE / spawn.done / agents snapshot / ralph#1 last_input.topic == spawn.done
- 实际 stdout 报 minimax API 高负载: `Reconnecting... 1/5 → 5/5 → internalServerError: high demand`
- ralph#1 第一轮 turn 没产出任何 structured event → supervisor timeout 退出

**分析**:
- **不是 sync 引入的回归**:
  - sync/origin-v2.10.1 只改 `hats.rs` (+363 行) + `event_loop_ralph.rs` (+6 行) + 纯 docs
  - 都没动 `event_loop` 主路径 / supervisor / starting_event 处理
  - minimax profile yaml 配置正确注入 `-p minimax -m gpt-5.5`
- **是 minimax provider 临时基础设施问题**:
  - 2026-08-14 同一 scenario 跑通(commit `abe3c913`)
  - 2026-08-15 minimax API 当前高负载,retry 5 次后放弃

**待执行**:
- minimax API 恢复稳定后重跑:
  ```bash
  RALPH_E2E_CODEX_PROFILE=minimax cargo run -p ralph-e2e -- codex --filter parallel-emit-spawn-instance --keep-workspace
  ```
- 期望: PASS (对照 2026-08-14 的成功结果)
- 如果重跑也失败: 进一步诊断 → 升级到 `docs/solutions/` formal capture

**触发条件**:
- minimax API 高负载缓解后 (任何时间)
- 或用户显式调用 `$verify sync/origin-v2.10.1` 时执行

### 2026-08-15 22:43: 第二次重跑结果 (Re-run #1)

**结果**: ❌ 仍失败,**根因不变**

**证据**:
- stdout 显示 minimax provider 重试 5/5 后 `internalServerError`
- 错误信息:`We're currently experiencing high demand, which may cause temporary errors.`
- 时长:120.1s (与上次一致,卡在 supervisor_shutdown timeout)
- events.jsonl topic summary:
  - 10 runtime.lifecycle
  - 1 runtime.delivery
  - 1 coordinator.no_event_first_turn

**结论**:
- minimax provider 在 2026-08-15 全天持续高负载
- 两次运行(22:25 和 22:43,间隔 18 分钟)都因同一原因失败
- 重跑时机不合适:应当在 minimax provider 容量恢复后重试(无法预测具体时间)

**下一步**:
- 不再重复重跑直到 minimax provider 状态变化
- 下次手动触发时:先用一个轻量 minimax probe (例如 `codex -p minimax exec -m gpt-5.5 'say hi'`)验证 provider 是否可用,再跑完整 e2e


### 2026-08-15 22:48: 第三次重跑结果 (Re-run #2 with MiniMax-M3)

**根因修正**: minimax provider 一直可用,之前失败是因为 yaml 默认 `{model}` 占位符展开为 `gpt-5.5`(代码常量 `DEFAULT_RALPH_E2E_CODEX_MODEL = "gpt-5.5"`),minimax 没这个模型。错误信息被 minimax provider 包装成 "high demand"。

**修复**: `RALPH_E2E_CODEX_MODEL=MiniMax-M3` 环境变量覆盖。

**结果**: ⚠️ 仍 fail,但**根本问题变了**

**事件流**(部分成功):
- 14:47:58 task.start → ralph#1 running
- 14:48:23 ralph#1 spawns **worker#2 (dynamic)** ✓
- 14:48:23 spawn.task published ✓
- 14:48:36 worker#2 done, publishes **spawn.done** ✓
- 14:48:36 ralph#1 receives spawn.done, last_input.topic = "spawn.done" ✓
- 14:49:45 ralph#1 idle (presumably after processing spawn.done)
- 14:49:58 max_runtime_seconds=120 reached → supervisor_shutdown

**Topic summary**:
- 17 runtime.lifecycle
- 3 runtime.delivery
- 1 spawn.task
- 1 spawn.done
- 1 coordinator.no_event_first_turn

**关键进步(对比 Re-run #1)**:
- ✅ `spawn.task` 事件**真正 publish**了(Re-run #1 是 0)
- ✅ **dynamic worker#2 被 spawn 出来了**(Re-run #1 是 0)
- ✅ `spawn.done` 事件**真正 publish**了(Re-run #1 是 0)
- ✅ agents.json 显示 `completed_dynamic_instances: [{instance_id: worker#2, last_input.topic: spawn.task, retirement_reason: dynamic_instance_unregistered_after_done}]`

**剩余失败原因**(只剩这一条):
- ✗ ralph#1 收到 spawn.done 后,**没有 emit LOOP_COMPLETE**,直接 idle
- 测试 yaml 期望:ralph#1 收到 spawn.done → 立即 emit LOOP_COMPLETE → supervisor 终止
- 实际:ralph#1 idle 25 秒 (14:48:36 → 14:49:45),然后被 max_runtime 强杀
- **这是 MiniMax-M3 模型行为问题**(模型"lazy",收到事件后没强制 emit completion promise),不是 sync 回归

**结论**:
- sync/origin-v2.10.1 没破坏 minimax profile path
- 动态 spawn 链路完全工作:ralph#1 → spawn.task → worker#2 → spawn.done → ralph#1
- 唯一缺口:ralph#1 收到 spawn.done 后没有按 prompt 协议 emit LOOP_COMPLETE
- 这是 MiniMax-M3 模型行为漂移(2026-08-14 同一 scenario 46.9s PASS,今天 120s timeout),不是 sync 引起

**✅ 已修复 (2026-08-15 后续)**:
- 分支: fix/completion-via-event
- commits: d275c7e6 + 39c4a0df
- fix/completion-via-event 落地后,parallel-emit-spawn-instance 在 minimax + MiniMax-M3 下
  13.7s PASS, 7/7 assertions。完整方案见
  docs/solutions/lazy-model-completion/README.md。
- 核心改动:complete_publishes topic 升级为 supervisor 硬终止信号
  (新增 TerminationReason::WorkflowCompletionEvent 变体),不再依赖模型
  写 LOOP_COMPLETE 字符串。

**历史记录保留**:本条 Re-run #2 之前的诊断仍然有效,作为
lazy-model hang 现象的初始观察保留。

