# 任务计划: 1 + 2 + 3(落 proposal Appendix C + Group 4 重分类 + 新 change)

## 状态: 三项全部完成

### 落盘

- proposal.md 加 Appendix C(89 行),从 433 → 522
- tasks.md 4.4 改 [x] dropped + 4.15 新增 dropped
- 新建 `openspec/changes/declarative-e2e-mock-parity/`
  - proposal.md 125 行
  - tasks.md 59 行
- (audit-p3-p4.md + audit report 在 change 目录内,207 行,完整)

### 决定

- [决定]: 4.4 + 4.15 dropped,因为目标文件 `mcp.rs` 与整个 ralph-api/ crate 已删
- [决定]: 新的 declarative-e2e-mock-parity change 文件落在独立 change 目录
  [理由]: F1 是独立 concern(同步 mock),不混入 sync-origin-main-features
- [决定]: 新 change 用 「option A: 调用 imperative runner.configure_mock_mode」
  [理由]: imperative 已经做了硬失败 + persist_e2e_artifacts 等改进,dedupe 防止 drift

## 当前 HEAD

仍 `8b27556`(无新代码 commit)

## 工作树状态

- 不需要新 commit(纯文档)
- 等用户决策是否 commit proposal.md / tasks.md 改动

## [2026-08-13 13:05:00] [Session ID: omx-1786600320381-z290x9] 状态更新: e2e-declarative-migration-plan 已 commit

### 完成动作
- 补 `specs/e2e-declarative-coverage-gate/spec.md` delta
  (2 Requirement / 5 Scenario, 首句含 MUST)
- `openspec validate --strict`: 29/29 全绿
- commit `b24131a chore(openspec): open e2e-declarative-migration-plan change with declarative coverage gate delta`

### 当前 HEAD
`b24131a`(ahead of my/main by 9 commits)

### 工作树状态
- 干净(`.scratch/` 是用户 scratch worktree,不动)
- 未 push

### 下一步可选方向
1. Wave 1 启动:任务 1.1 把 `get_all_scenarios()` refactor 到 `ralph_e2e::all_scenarios()`
2. Wave 1 启动:任务 1.2 加 `ScenarioKind` enum
3. Wave 1 启动:任务 1.3 写 `tests/declarative_coverage_gate.rs`(CI gate test)
4. 暂留 Wave 1,先去 push 已有 9 commits 到 my/main
5. 等用户决策下一步

### 决定
- [决定]: 先 commit 当前 staged change,再让用户决策下一步
  [理由]: 用户说"继续刚才的 git跟进 任务",最自然的 git 跟进动作是落盘 staged 内容,而不是直接动手写代码
- [决定]: 补最小 delta(2 Requirement, 5 Scenario),不扩展 22 行 migration 表到 spec
  [理由]: 22 行 migration 不属于 spec contract,属于 plan detail;spec 只 pin 两个 CI gate 的不变式

## [2026-08-13 13:45:00] [Session ID: omx-1786600320381-z290x9] Wave 1 已 commit

### 完成动作
- 加 `pub enum ScenarioKind { Declarative, Imperative, ImperativeExplicitKeep }`
- 加 `pub fn all_scenarios() -> Vec<(ScenarioKind, &'static str, Box<dyn TestScenario>)>`
- 把 61 条 entry 从 main.rs::get_all_scenarios() 搬到 lib.rs(每条带 kind + id)
- main.rs 删旧函数,更新两处 caller,清理 ~22 个 dead imports
- 新文件 `crates/ralph-e2e/tests/declarative_coverage_gate.rs`(114 行,2 test):
  - `declarative_coverage_at_or_above_threshold`:assert ratio >= 0.90,失败时打 drift log
  - `explicit_keep_is_exactly_parallel_experimental_dev_engine_example`:pin 不变量

### 净结果
- 1 commit `50e11cd feat(e2e): add declarative coverage CI gate with ScenarioKind registry`
  - lib.rs +485 行
  - main.rs -244 行(净)
  - tests/declarative_coverage_gate.rs +118 行(新增)
- `cargo test -p ralph-e2e --lib`:526 passed, 0 failed(无回归)
- `cargo test -p ralph-e2e --test declarative_coverage_gate`:1 failed (预期) + 1 ok
- `cargo run -p ralph-e2e -- --list`:scenario list 同序,行为不变
- `openspec validate --all --strict`:29/29 全绿

### 当前 HEAD
`50e11cd`(ahead of my/main by 10 commits)

### 工作树状态
- 干净(`.scratch/` 保留)
- 未 push

### Drift log(故意 fail 的当前状态)
```
Declarative:            39 (connectivity, single-iter, multi-iter, completion, ...)
Imperative:             21 (tool-use, streaming, hat-*, memory-*, timeout, ...)
ImperativeExplicitKeep: 1  (parallel-experimental-dev-engine-example)
Coverage:               65.00 %
Threshold:              90.00 %
Pass / Fail:            FAIL
```

### 决定
- [决定]: 在 lib.rs 内部用 `crate::declarative::from_yaml`,不保留 `ralph_e2e::` 前缀
  [理由]: lib.rs 是 crate 自身,引用自己用 `crate::` 是标准 idiom;`ralph_e2e::` 是给外部消费者看的
- [决定]: `main.rs` 保留 `MockCliError` / `MockConfig` 等非 scenario type imports
  [理由]: 这些仍然在 mock_cli 子命令路径里被使用,删了会破编译
- [决定]: `ImperativeExplicitKeep` 作为第三个 variant,不另起 `Vec<&str>` 黑名单
  [理由]: 单一真相源 + 静态类型保证,future-proof 比 string matching 强
- [决定]: 不把 `ScenarioKind` 挂到 `TestScenario` trait 上(default method)
  [理由]: 那样会强迫修改所有 22 个 imperative impl 的源码,扩散面太大;
          registry 侧打 tag 是单点改动,后续迁移 commit 直接删 tag 即可

### 下一步可选方向
1. **Wave 2 第一个 commit**(任务 2.1.1):迁移 `TimeoutScenario` → `timeout.yaml`(4 个 easy 类里的第一个,风险最低)
2. **Wave 2 第一个 commit**(任务 2.1.2):迁移 `MaxIterationsScenario` → `max-iterations.yaml`(同样 easy)
3. **Wave 2 第一个 commit**(任务 2.3.1):迁移 `MemoryAddScenario` → `memory-add.yaml`(memory 类 8 个里的第一个,难度中等)
4. **暂留 Wave 2**:先 push 现有 10 commits 到 my/main
5. **暂停**:等用户决策下一步

## [2026-08-13 14:10:00] [Session ID: omx-1786600320381-z290x9] Wave 2 任务 2.1.1 已 commit

### 完成动作
- 新文件 `crates/ralph-e2e/scenarios/timeout.yaml`(42 行,id = `timeout-handling`)
- lib.rs registry 把 `TimeoutScenario` 那条改成 `Declarative` + from_yaml(...)
- CLI list 显示:`timeout-handling  Verifies graceful timeout termination (declarative)`
- `TimeoutScenario` struct 留在 errors.rs(测试 + pub use 仍保留,Wave 3 才删)

### 净结果
- 1 commit `c0e1687 feat(e2e): migrate TimeoutScenario → timeout.yaml (Wave 2 task 2.1.1)`
  - timeout.yaml +42 行
  - lib.rs +6 行 / -3 行
- drift log delta:65.00 % → 66.67 %(40 declarative / 20 imperative / 1 keep)
- 526 lib tests 仍全过
- gate test 仍 FAIL(预期,要 19 / 21 migrations 才到 90 %)

### 决定
- [决定]: YAML id 用 `timeout-handling` 而不是 `timeout`,匹配命令式 `TimeoutScenario::id()`
  [理由]: 保持 `scenario.id()` 调用方行为不变,CLI 输出保持一致
- [决定]: 命令式 3 条断言(did_timeout / graceful / duration_near)折成 1 条 declarative
  `termination: TIMEOUT`
  [理由]: 3 条都归结到 `result.timed_out`,executor 把 `termination_reason` 设成 `"TIMEOUT"`
  当且仅当 timed_out;duration_near 由 hard kill 行为保证,无需 schema 字段
- [决定]: 不删除 `TimeoutScenario` struct + pub use
  [理由]: Wave 3 task 3.4 显式推迟 struct 物理删除到一个 release cycle 后,
  本轮只换 registry 调用,不破坏外部 API

### 下一步可选方向
1. **任务 2.1.2**:迁移 `MaxIterationsScenario` → `max-iterations.yaml`
2. **任务 2.1.3**:迁移 `BackendUnavailableScenario` → `backend-unavailable.yaml`
3. **任务 2.1.4**:迁移 `AuthFailureScenario` → `auth-failure.yaml`(完成 §2.1 全部 easy 类)
4. **先 push**:把 12 commits 推到 my/main
5. **暂停**:等用户决策

## [2026-08-13 14:55:00] [Session ID: omx-1786600320381-z290x9] Wave 2 任务 2.1.2 已 commit

### 完成动作
- 新文件 `crates/ralph-e2e/scenarios/max-iterations.yaml`(54 行,id = `max-iterations`)
- lib.rs registry 把 `MaxIterationsScenario` 那条改成 `Declarative` + from_yaml(...)
- CLI list 显示:`max-iterations  Verifies termination at max iterations limit (declarative)`
- `MaxIterationsScenario` struct 留在 errors.rs(测试 + pub use 仍保留,Wave 3 才删)

### 净结果
- 1 commit `d267c97 feat(e2e): migrate MaxIterationsScenario → max-iterations.yaml (Wave 2 task 2.1.2)`
  - max-iterations.yaml +54 行
  - lib.rs +5 行 / -2 行
- drift log delta:66.67 % → 68.33 %(41 declarative / 19 imperative / 1 keep)
- 526 lib tests 仍全过
- gate test 仍 FAIL(预期,要 18 / 21 migrations 才到 90 %)

### 决定
- [决定]: YAML id 用 `max-iterations` 而不是 `max-iter`,匹配命令式 `MaxIterationsScenario::id()`
  [理由]: 同 2.1.1 — 保持 `scenario.id()` 调用方行为不变
- [决定]: 命令式 4 条断言全部 1:1 映射到 declarative schema 字段,无折断言
  [理由]: schema 有 `response_received` / `exact_iterations` / `termination` / `no_timeout`
  正好一一对应,不需要合并;与 2.1.1 的「3 折 1」不同,这次是纯平移
- [决定]: YAML 显式声明 `backends: [claude, kiro, opencode]`
  [理由]: 命令式 `supported_backends()` 也是 `[Claude, Kiro, OpenCode]`(不含 Codex);
  declarative runner 默认全 backend,显式声明能保持 gate test 列表与命令式一致
- [决定]: YAML 省略 `timeout_secs:`,让 runner 落回 `backend.default_timeout()`
  [理由]: 命令式 setup 用 `backend.default_timeout()`,declarative runner
  `None.unwrap_or_else(backend.default_timeout)` 是同样行为,显式写数字会
  把 backend-specific 知识搬到 YAML 里,反而破坏 1:1 等价
- [决定]: 命令式 `termination_reason_is_max` 做 lowercase contains 匹配
  (`max` / `iteration` / `limit`),declarative `termination_matches` 做严格相等;
  YAML 写 `"MAX_ITERATIONS"`(实际 executor 返回值)而不是 `"max"`
  [理由]: executor.detect_termination_reason 在 max iterations 路径固定返回
  `"MAX_ITERATIONS"`,严格相等在该路径下与 contains 在该值集合下语义等价;
  若未来 executor 改成返回 `"MAX_ITERS"` 之类,declarative 会立刻 fail 提示
  schema 与 executor 不一致 — 这是 stricter-check 作为 drift detector 的副效益

### 下一步可选方向
1. **任务 2.1.3**:迁移 `BackendUnavailableScenario` → `backend-unavailable.yaml`(§2.1 倒数第 2 个)
2. **任务 2.1.4**:迁移 `AuthFailureScenario` → `auth-failure.yaml`(完成 §2.1 全部 easy 类)
3. **§2.2 hats**:5 个 hat-collection scenarios(难度中等,需要看 scenario 是否有 inject / snapshot 之类)
4. **§2.3 memory**:8 个 memory scenarios(难度中等,需要 schema 扩展或保留命令式)
5. **先 push**:把 14 commits 推到 my/main
6. **暂停**:等用户决策

## [2026-08-13 15:00:00] [Session ID: omx-1786600320381-z290x9] Wave 2 任务 2.1.3 启动:BackendUnavailableScenario → backend-unavailable.yaml

### 目标
- 读 errors.rs 中 BackendUnavailableScenario 完整定义(setup/run/assertions + supported_backends)
- 查 executor.rs 中 backend unavailable 路径 detect_termination_reason 的返回值
- 写 backend-unavailable.yaml,1:1 映射到 declarative schema 已有字段
- 改 lib.rs registry
- 跑 4 gate: cargo check / cargo test --lib / --list / declarative_coverage_gate
- 预期 drift delta: 41/19/1 → 42/18/1 ≈ 70.00%
- feat + chore 双 commit,沿用 2.1.1 / 2.1.2 模板

### 状态
**目前在阶段1(读命令式)** — 下一步:读 errors.rs:380-? + executor.rs:600-? 后写 YAML

### 状态更新:2.1.3 暂停,改走 2.1.4
**目前在 2.1.3 → 2.1.4 切换点**
- 调研发现:
  - audit-p5-p1.md:73 明确标记 `BackendUnavailableScenario` 需要 schema 扩展
    (`require_backend: <wrong>`),OpenSpec tasks.md §2.1.3 列为 Easy 与 audit 不一致
  - 命令式 setup 设的 `cli.command: nonexistent-cli-...` 在 `backend: claude|kiro|opencode`
    时被 ralph 静默忽略(config.rs:795-803),命令式 test 即便 live run 也不一定真触发
    backend-unavailable 路径
  - schema 缺 3 字段:`exit_code_nonzero` / `stderr_contains` / `failed_within_secs`
- 决议:不强行迁移。强行迁移 = 3 条断言全 fail,但 gate 仍计为 Declarative = 假阳性 = 违反
  "做最正确修复/改动而不是最小修复/改动" 纪律
- 已在 LATER_PLANS.md 记录 schema 扩展工作(待 §2.6 或新 delta spec)
- 本轮直接进 §2.1.4 AuthFailureScenario 迁移(预计类似 2.1.1 难度)

### 下一步
1. **任务 2.1.4** AuthFailureScenario → auth-failure.yaml(沿用 2.1.1/2.1.2 pipeline)
2. (延后) §2.6 schema 扩展 + 重写 2.1.3 setup
3. (延后) push 现有 15 commits
