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

## [2026-08-13 22:05:00] [Session ID: omx-1786600320381-z290x9] e2e-live-convergence 诊断 — 解 exp-20260813-e2e-live-convergence-issue

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
