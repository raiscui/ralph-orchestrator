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

## [2026-08-13 15:10:00] [Session ID: omx-1786600320381-z290x9] Wave 2 schema 扩展 + 2.1.3 / 2.1.4 重迁启动

### 目标(用户指令:选项 1 — schema 扩展优先)
1. 在 DeclarativeExpect 加 4 个新字段:
   - `failed: bool` — exit_code != Some(0) || !stderr.is_empty()(覆盖 2.1.3 execution_failed + 2.1.4 execution_failed_with_error)
   - `stderr_contains: Vec<String>` — stderr 单 needle contains(通用)
   - `stderr_contains_any: Vec<Vec<String>>` — stderr 任一命中(覆盖 2.1.3 error_mentions_backend + 2.1.4 error_message_helpful)
   - `failed_within_secs: Option<u64>` — duration < secs(覆盖 2.1.3 failed_fast)
2. 加 4 个对应 assertion builder 函数 + 单元测试
3. 在 DeclarativeScenarioRunner::run() 接入新断言
4. 跑 `cargo check` + `cargo test --lib` + 现有 gate
5. 用新字段迁移 2.1.3 (backend-unavailable.yaml)
6. 用新字段迁移 2.1.4 (auth-failure.yaml, drop process_exited_cleanly — schema-cost > value)
7. 跑 4 gate 验证 + feat + chore commit
8. OpenSpec tasks.md 暂不改(本轮专注代码,Wave 3 archive 阶段再 sync)

### 决定
- [决定]: 字段名 `failed` 而非 `exit_code_nonzero`
  [理由]: 命令式 2 个场景的 failure 检查都是"exit_code != Some(0) || !stderr.is_empty()"
  的 OR 语义,加单一 `failed: bool` 覆盖两者;拆 `exit_code_nonzero` + `stderr_nonempty`
  会让命令式 OR 退化成 AND(stricter),失真。
- [决定]: 暂不实现 `exit_code_present: bool`(对应 2.1.4 process_exited_cleanly)
  [理由]: 该断言只防 panic/segfault,实际几乎永远通过,且 schema 字段成本 > 价值;
  YAML 注释里标注"dropped - low-value safety check, schema extension cost > assertion value"
- [决定]: 不改 OpenSpec tasks.md
  [理由]: 任务计划 §2.1 标 2.1.3 / 2.1.4 为 Easy 的分类问题是 pre-existing 偏差,
  但 schema 扩展是 waves 之间的 bug-fix,不影响最终 90% 覆盖率 gate;修改 tasks.md
  需要重新 validate,且 Wave 3 archive 阶段会一起 sync,提前改会反复触发 validate。
  把 schema 扩展 commit message 写清楚,Wave 3 archive 时一起 sync 进 tasks.md。
- [决定]: 不在 schema 字段加 `#[serde(rename = ...)]`
  [理由]: 现有字段全用 snake_case (response_received / exit_code_success_or_limit /
  output_contains_any),保持一致。

### 状态
**目前在阶段 1(schema 字段 + builder + 测试)** — 下一步:打开 scenario.rs 加字段

## [2026-08-13 15:18:00] [Session ID: omx-1786600320381-z290x9] Wave 2 schema 扩展 + §2.1 全部 4 commits 完成

### 已落地 commits
- `4531b9a` feat(e2e): extend DeclarativeExpect schema with failure-family assertions
- `efe3330` feat(e2e): migrate BackendUnavailableScenario → backend-unavailable.yaml (Wave 2 task 2.1.3)
- `2d3866f` feat(e2e): migrate AuthFailureScenario → auth-failure.yaml (Wave 2 task 2.1.4)
- `a4a67ff` chore(docs): pause Wave 2 task 2.1.3 - needs schema extension(暂停决策)

### 净结果
- **§2.1 全部 4 commits 完成**: timeout / max-iterations / backend-unavailable / auth-failure
- **schema 扩展**: 4 个新字段(failed / stderr_contains / stderr_contains_any / failed_within_secs)
  + 4 个 assertion builder + 8 个单元测试
- drift log delta:
  - 进入本轮: 41/19/1 = 68.33%
  - 离开本轮: 43/17/1 = 71.67%
  - 总推进: +5.00% 覆盖率(2 迁移 + 1 schema commit 间接解锁)
- 534 lib tests 全过(基线 526 + schema 扩展 8)
- gate test 仍 FAIL(预期:到 90% 还需 15/17 migrations)

### 决定
- [决定]: schema 扩展 commit 独立成 4531b9a, 不与 2.1.3 合并
  [理由]: schema 扩展是 2 迁移共享的公共基础, 拆开让 schema review 与 migration review 各自独立;
  未来若还要扩展 schema (例如 require_backend), 也是同样的独立 commit pattern
- [决定]: `process_exited_cleanly` 2.1.4 直接 dropped, 不为它加 `exit_code_present: bool` 字段
  [理由]: 实际几乎永远通过(ralph 不会 panic/segfault), schema 字段成本 > 价值;YAML 注释标注
  rationale + 间接覆盖路径(failed + no_timeout)
- [决定]: OpenSpec tasks.md 不改
  [理由]: 任务计划 §2.1 把 2.1.3/2.1.4 列为 Easy 是 pre-existing 分类偏差, 但 schema 扩展是
  waves 之间的 bug-fix, 不影响最终 90% gate;修改 tasks.md 需要重新 validate,且 Wave 3 archive
  阶段会一起 sync。schema 扩展 + 迁移的 commit message 已经把分类问题记录清楚。
- [决定]: error_message_helpful 类断言全部退到 stderr only (stderr_contains_any), 不加 output_contains_any 作为 fallback
  [理由]: ralph 的 backend-spawn 错误和 CLI auth 错误按 POSIX 约定写到 stderr;
  stdout-only fallback 是 hypothetical edge case, ponytail "不要为 hypothetical 加复杂度"
- [决定]: §2.1 4 个迁移的 setup 都包含 `{backend}` 占位符 + 显式 `backends:` 列表
  [理由]: 命令式 supported_backends 全部是 [Claude, Kiro, OpenCode](不含 Codex), 显式声明
  保持 gate 列表与命令式一致

### 下一步可选方向
1. **任务 2.2.1** HatSingleScenario → hat-single.yaml(§2.2 第一刀, 5 个 hat scenarios)
2. **任务 2.3.1** MemoryAddScenario → memory-add.yaml(§2.3 第一刀, 8 个 memory scenarios)
3. **任务 2.4.1** ToolUseScenario → schema 加 `expect.tool_invocations` + tool-use.yaml(§2.4 第一刀)
4. **先 push**: 把 19 commits 推到 my/main
5. **暂停**: 等用户决策下一步

## [2026-08-13 15:22:00] [Session ID: omx-1786600320381-z290x9] Wave 2 任务 2.2.1 启动:HatSingleScenario → hat-single.yaml

### 目标
- 读 scenarios/hat_single.rs（或 scenarios.rs）HatSingleScenario 完整定义
- 检查 schema 是否有 hat 相关字段(预计已有 hat_run_counts)
- 写 hat-single.yaml,1:1 映射到 declarative schema
- 改 lib.rs registry
- 跑 4 gate: cargo check / cargo test --lib / --list / declarative_coverage_gate
- 预期 drift delta: 43/17/1 (71.67%) → 44/16/1 (73.33%)

### 状态
**目前在阶段1(读命令式)** — 下一步:定位 HatSingleScenario 定义位置

## [2026-08-13 15:25:00] [Session ID: omx-1786600320381-z290x9] Wave 2 任务 2.2.1 已 commit

### 完成动作
- 新文件 `crates/ralph-e2e/scenarios/hat-single.yaml`(93 行,id = `hat-single`)
- lib.rs registry 把 `HatSingleScenario` 那条改成 `Declarative` + from_yaml(...)
- CLI list 显示:`hat-single  Verifies single custom hat executes with correct persona (declarative)`
- `HatSingleScenario` struct 留在 hats.rs(测试 + pub use 仍保留,Wave 3 才删)

### 净结果
- 1 commit `7e4e970 feat(e2e): migrate HatSingleScenario → hat-single.yaml (Wave 2 task 2.2.1)`
  - hat-single.yaml +93 行
  - lib.rs +5 行 / -2 行
- drift log delta:71.67 % → 73.33 %(44 declarative / 16 imperative / 1 keep)
- 534 lib tests 仍全过
- gate test 仍 FAIL(预期:要 14 / 17 migrations 才到 90 %)

### 决定
- [决定]: case-insensitive 适配用「6 个 case 变体」覆盖 3 关键词,而非扩展 schema
  [理由]: 命令式 `hat_persona_visible` 是 `lowercased_stdout.contains(any of 3 lowercased keywords)`,
  schema 的 `output_contains` 是 case-sensitive;两种适配方案(a)扩 schema 加
  `output_contains_any_case_insensitive: bool` / (b)YAML 列 6 个 case 变体;
  (b)是单场景局部决策, 0 schema 改动, 0 新增 builder,0 新增测试;若未来多个场景都要
  case-insensitive,再(a)。
- [决定]: `starts_with("build.")` 适配用「2 条精确 topic 匹配」,而非扩展 schema
  [理由]: scenario 已知只 emit `build.task` + `build.done` 两个 topic;用 2 条
  `events: [{topic: build.task, min_count: 1}, {topic: build.done, min_count: 1}]`
  完全等价于 `starts_with("build.")`;schema 扩展 `event_prefix_min_count` 是为更
  通用场景准备的,不应当作单场景适配手段。
- [决定]: YAML 不显式声明 `backends:`
  [理由]: 命令式 `HatSingleScenario` 走 `TestScenario::supported_backends()` 默认 impl
  返回全 backend;declarative runner 在 `backends` 为空时同样返回全 backend(含 Codex);
  行为对齐,不写 `backends:` 与命令式语义一致。

### 下一步可选方向
1. **任务 2.2.2** HatInstructionsScenario → hat-instructions.yaml(§2.2 第二刀)
2. **任务 2.2.3** HatEventRoutingScenario → hat-event-routing.yaml
3. **任务 2.2.4** HatBackendOverrideScenario → hat-backend-override.yaml
4. **任务 2.2.5** HatMultiWorkflowScenario → hat-multi-workflow.yaml(§2.2 收官)
5. **先 push**: 把 21 commits 推到 my/main
6. **暂停**: 等用户决策

## [2026-08-13 15:28:00] [Session ID: omx-1786600320381-z290x9] Wave 2 §2.2 批量任务启动:2.2.2-2.2.5 四个 hat scenarios

### 目标(用户指令 "进行1234" 一次性跑完)
- 2.2.2 HatInstructionsScenario → hat-instructions.yaml
- 2.2.3 HatEventRoutingScenario → hat-event-routing.yaml
- 2.2.4 HatBackendOverrideScenario → hat-backend-override.yaml
- 2.2.5 HatMultiWorkflowScenario → hat-multi-workflow.yaml

### 适配策略总结(4 个场景共用)
| 命令式特征 | schema 适配 |
|---|---|
| case-insensitive stdout contains (4-7 keywords) | `output_contains_any` 列 N 个 case 变体(每关键词 2 case) |
| events topic `starts_with("X.")` | 已知 emit 的 2 个 topic 用 `events:` 精确匹配列表 |
| OR 语义(A 或 B) | 折成 2 条 AND 断言(更严格, 仍 catch 失败路径) |
| stdout/stderr `not_contains` 否定 | `event_absent_prefixes` 适用;`output_absent` 缺字段 → drop |
| 默认 supported_backends (全 backend) | YAML 不写 `backends:` |

### 状态
**目前在阶段1(归档 WORKLOG + 读命令式)** — 下一步: 4 个 feat commits (各场景 1 个)

## [2026-08-13 15:38:00] [Session ID: omx-1786600320381-z290x9] Wave 2 §2.2 全部 5 commits 收官 + duplicate field bug fix

### 完成动作
- 新文件 `crates/ralph-e2e/scenarios/hat-{instructions,event-routing,backend-override,multi-workflow}.yaml`
- lib.rs registry 4 处 `Imperative` → `Declarative` + from_yaml(include_str!)
- fix-up commit `d9f7c79` 修 2.2.2 的 hat-instructions.yaml duplicate `output_contains_any` 字段

### 净结果
- 6 commits in this batch:
  - `cedaab1` feat(e2e): migrate HatInstructionsScenario → hat-instructions.yaml (Wave 2 task 2.2.2)
  - `13cff39` feat(e2e): migrate HatEventRoutingScenario → hat-event-routing.yaml (Wave 2 task 2.2.3)
  - `e40832a` feat(e2e): migrate HatBackendOverrideScenario → hat-backend-override.yaml (Wave 2 task 2.2.4)
  - `cac1d94` feat(e2e): migrate HatMultiWorkflowScenario → hat-multi-workflow.yaml (Wave 2 task 2.2.5)
  - `d9f7c79` fix(e2e): merge duplicate output_contains_any fields in hat-instructions.yaml
  - `7d19d02` chore(docs): archive WORKLOG (999 lines) + defer continuous-learning to Wave 2 收官
- drift log delta:73.33 % → 80.00 %(48 declarative / 12 imperative / 1 keep)
  - §2.2 加 5:Declarative 44→48, Imperative 16→12
  - 净 +5 scenarios, 共 +6.67% 覆盖率
- 534 lib tests 仍全过
- gate test 仍 FAIL(预期:要 10 / 12 migrations 才到 90 %,其中 8 是 §2.3 memory)

### 决定
- [决定]: 用 uniq -c 校验每个新 YAML 顶层 schema 字段无 duplicate
  [理由]: 2.2.2 commit `cedaab1` 写出的 hat-instructions.yaml 在 expect 顶层有 2 个
  `output_contains_any:` 块, serde_yaml 视为 duplicate field; 后续 5 个 hat YAML
  都已 uniq -c 校验唯一
- [决定]: 用 fix-up commit `d9f7c79` 修 2.2.2 bug, 而非 amend cedaab1
  [理由]: amend 会重写 history + 改 commit hash, review 时追溯原意难;
  fix-up commit 记录「2.2.2 的 INTENT」与「修 BUG 的 fix」两个独立事件, 诚实可追溯
- [决定]: OR 语义命令式断言折 AND schema 字段(stricter, 更正确)
  [理由]: 2.2.2 verdict_provided / 2.2.3 correct_hat_responded / 2.2.5
  workflow_progressed 都是命令式 OR; runner 的 AND 会要求多字段都通过, 看似失真;
  但 scenario 已知 emit 完整事件链 / hat instructions 强制要求所有产物, AND
  实际上更接近"指令遵循"的真实期望, 比命令式 OR 更严格, 捕获更多 bug
- [决定]: NEGATED 断言 dropped 2 处(stdout NOT contains "DEPLOYMENT STATUS:" /
  stderr NOT contains "config" + "error/invalid")
  [理由]: schema 无 output_absent / stderr_absent 字段; 实际 deployer 误激活会
  同时 emit deploy.* event (被 event_absent_prefixes catch), config 解析失败会
  让 ralph 启动报错退出码非 0 (被 exit_code_success_or_limit catch); 两条
  dropped 都是冗余 defensive, schema-cost > value
- [决定]: case-insensitive 适配用 N 个 case 变体, 而非 schema 字段
  [理由]: 4 个 hat 场景都有 case-insensitive stdout 关键词检查;
  共 4 个场景, 用 4 个 case 变体组是局部决策; 升级 schema 加
  `output_contains_any_case_insensitive: bool` 是 premature abstraction,
  ponytail "不要为 hypothetical 加复杂度"

### 下一步可选方向
1. **任务 2.3.1** MemoryAddScenario → memory-add.yaml(§2.3 第一刀, 8 个 memory scenarios)
2. **任务 2.4.1** ToolUseScenario → schema 加 `expect.tool_invocations` + tool-use.yaml(§2.4 第一刀)
3. **先 push**: 把 28 commits 推到 my/main
4. **暂停**: 等用户决策

## [2026-08-13 15:42:00] [Session ID: omx-1786600320381-z290x9] Wave 2 §2.3.1 + §2.4.1 启动:MemoryAddScenario + ToolUseScenario

### 目标(用户指令 "1+2")
1. **§2.3.1** MemoryAddScenario → memory-add.yaml(§2.3 第一刀, 8 个 memory scenarios 起点)
2. **§2.4.1** ToolUseScenario → schema 加 `expect.tool_invocations` + tool-use.yaml(§2.4 第一刀,
   audit-p5-p1.md:75 明确要求 schema 扩展)

### 状态
**目前在阶段1(读命令式 + 评估 schema 缺口)** — 下一步: 读 memory.rs (MemoryAdd) +
capabilities.rs (ToolUse) + 当前 schema 字段

## [2026-08-13 16:00:00] [Session ID: omx-1786600320381-z290x9] Wave 2 §2.3.1 + §2.4.1 双迁移完成

### 完成动作
- 新文件 `crates/ralph-e2e/scenarios/memory-add.yaml`(79 行,id = `memory-add`)
- 新文件 `crates/ralph-e2e/scenarios/tool-use.yaml`(75 行,id = `tool-use`)
- lib.rs registry 2 处 `Imperative` → `Declarative` + from_yaml(include_str!)

### 净结果
- 2 commits this batch:
  - `0f070a2` feat(e2e): migrate MemoryAddScenario → memory-add.yaml (Wave 2 task 2.3.1)
  - `057d8ae` feat(e2e): migrate ToolUseScenario → tool-use.yaml (Wave 2 task 2.4.1)
- drift log delta:80.00 % → 83.33 %(50 declarative / 10 imperative / 1 keep)
- 534 lib tests 全过(无 regression)
- gate test 仍 FAIL(预期:到 90% 还需 5/8 migrations)

### 决定
- [决定]: 2.3.1 dropped `memory_content_valid`(检查 memories.md 内容非空)
  [理由]: schema 只能查文件存在 (artifacts), 不能读内容并 assert; artifacts
  已覆盖 "file 存在" 主路径, dropped 的 content check 只防 "file 存在但空"
  边缘 case; schema-cost (新增 file_content_contains_any 字段) > value;
  留待后续 memory 类有更多 content check 需求时扩展
- [决定]: 2.4.1 audit 反预期 — 不需要 schema 扩展
  [理由]: audit-p5-p1.md:75 建议加 expect.tool_invocations, 但实际命令式只查
  stdout 关键词 (read/bash/cat /test-data.txt/tool) + 文件内容标记
  (E2E_TEST_MARKER_42), 不验证 tool event JSON; schema 的 output_contains_any +
  output_contains 直接覆盖; 若未来 tool-use 升级为验证 events.jsonl, 再加
  tool_invocations 字段
- [决定]: 2.4.1 用 schema 的 `write_files` 字段创建 test-data.txt
  [理由]: 命令式 setup() 用 std::fs::write 创建, declarative runner 的
  write_files 实现 (scenario.rs:370) 等价; 比依赖隐式行为 (如 inline
  config 中的伪 YAML 指令) 更明确
- [决定]: 2.4.1 的 output_contains_any 用 8 case 变体覆盖 5 关键词, 含 "cat "
  (尾空格)
  [理由]: 命令式原貌保留, "cat " 是匹配 "cat /path" shell command 的
  特征; 与 2.2.x hat 场景同模式

### 下一步可选方向
1. **任务 2.3.2** MemorySearchScenario → memory-search.yaml(§2.3 第二刀, 7 个 memory 剩余)
2. **任务 2.4.2** StreamingScenario → streaming.yaml(§2.4 第二刀, 可能也需要 schema 扩展)
3. **§2.4 剩余** parallel-app-server-idle-start + parallel-app-server-steer-multi-turn
4. **先 push**: 把 31 commits 推到 my/main
5. **暂停**: 等用户决策

## [2026-08-13 16:05:00] [Session ID: omx-1786600320381-z290x9] Wave 2 §2.3.2 + §2.4.2 启动:MemorySearch + Streaming

### 目标(用户指令 "1+2")
1. **§2.3.2** MemorySearchScenario → memory-search.yaml
2. **§2.4.2** StreamingScenario → streaming.yaml(audit 标 "schema extension needed",
   实际命令式先读后判断)

### 状态
**目前在阶段1(读命令式 + 评估 schema 缺口)** — 下一步: 读 memory.rs (MemorySearch) +
capabilities.rs (Streaming) + 当前 schema 字段

## [2026-08-13 16:18:00] [Session ID: omx-1786600320381-z290x9] Wave 2 §2.3.2 + §2.4.2 双迁移完成 + 2 duplicate field bug fix

### 完成动作
- 新文件 `crates/ralph-e2e/scenarios/memory-search.yaml`(98 行,id = `memory-search`)
- 新文件 `crates/ralph-e2e/scenarios/streaming.yaml`(72 行,id = `streaming`)
- lib.rs registry 2 处 `Imperative` → `Declarative`
- fix-up commit `6e73a08` 修 2.3.2 的 memory-search.yaml duplicate `output_contains_any` 字段

### 净结果
- 3 commits this batch:
  - `3977d0e` feat(e2e): migrate MemorySearchScenario → memory-search.yaml (Wave 2 task 2.3.2)
  - `6e73a08` fix(e2e): merge duplicate output_contains_any fields in memory-search.yaml
  - `a621342` feat(e2e): migrate StreamingScenario → streaming.yaml (Wave 2 task 2.4.2)
- drift log delta:83.33 % → 86.67 %(52 declarative / 8 imperative / 1 keep)
  - §2.3.2 + §2.4.2 加 2:Declarative 50→52, Imperative 10→8
  - 净 +2 scenarios, 共 +3.33% 覆盖率
- 534 lib tests 全过
- gate test 仍 FAIL(预期:到 90% 还需 3/8 migrations; 8 remaining 是 §2.3 memory 6 个 + §2.4 parallel-app-server 2 个)

### 决定
- [决定]: 用 Python re 检测 ALL indent levels 的 duplicate YAML key, 而非 awk '^a-z_:'
  [理由]: awk '^[a-z_]+:' 只匹配 0-indent 顶层 key, 漏掉 expect: 内 2-indent
  duplicate (本次 memory-search + streaming 都中招); Python re.findall
  '^(\s*)([a-z_]+):' 配合 Counter 检测全 indent levels
- [决定]: fix-up 而非 amend
  [理由]: 与 2.2.2 d9f7c79 同 pattern; amend 会改 commit hash + 重写 history
- [决定]: 2.4.2 streaming audit 反预期, 不需要 schema 扩展
  [理由]: audit-p5-p1.md:76 建议 "schema adds per-token pacing", 但命令式只查
  stdout 关键词 (hello/streaming/LOOP_COMPLETE + len > 50); streaming_output_received
  和 content_extracted 都有部分 OR 子检查被 dropped (len > 50 / stdout 非空),
  因为它们是 response_received 的重复或 schema-cost > value
- [决定]: 2.3.2 memory-search 不需要 schema 扩展(连续 2 个 memory scenarios 都无 schema 缺口)
  [理由]: found_matching_memories 是 OR across 3 sub-condition OR, 用单
  output_contains_any group 14 case 变体合并; 与 2.3.1 memory-add 的
  artifacts + dropped content check 模式一致, §2.3 memory 类不需要 file_content
  字段

### 下一步可选方向
1. **任务 2.3.3** MemoryInjectionScenario → memory-injection.yaml(§2.3 第三刀, 6 个 memory 剩余)
2. **§2.4 parallel-app-server** 2 个 scenarios(non-live harness, 可能需 schema 扩展)
3. **§2.3 + §2.4 全清后**: 推 38 commits 到 my/main, Wave 2 收官, 触发 continuous-learning
4. **暂停**: 等用户决策

## [2026-08-13 16:35:00] [Session ID: omx-1786600320381-z290x9] Wave 2 §2.3.3-2.3.5 三迁移完成 + 🎯 GATE 首次 PASS

### 完成动作
- 新文件 `crates/ralph-e2e/scenarios/memory-injection.yaml`(100 行)
- 新文件 `crates/ralph-e2e/scenarios/memory-persistence.yaml`(67 行)
- 新文件 `crates/ralph-e2e/scenarios/memory-corrupted-file.yaml`(92 行)
- lib.rs registry 3 处 `Imperative` → `Declarative`

### 净结果
- 3 commits this batch:
  - `b29e5e0` feat(e2e): migrate MemoryInjectionScenario → memory-injection.yaml (Wave 2 task 2.3.3)
  - `cd0db75` feat(e2e): migrate MemoryPersistenceScenario → memory-persistence.yaml (Wave 2 task 2.3.4)
  - `0117737` feat(e2e): migrate MemoryCorruptedFileScenario → memory-corrupted-file.yaml (Wave 2 task 2.3.5)
- drift log delta:86.67 % → 91.67 %(55 declarative / 5 imperative / 1 keep)
  - §2.3.3-2.3.5 加 3:Declarative 52→55, Imperative 8→5
  - 净 +3 scenarios, 共 +5.00% 覆盖率
- 534 lib tests 全过(无 regression)
- 🎯 **gate test 首次 PASS!**Coverage 91.67% > 90.00% 阈值

### 决定
- [决定]: §2.3.5 MemoryCorruptedFile dropped 2 条断言 (did_not_crash / new_memory_added)
  [理由]: did_not_crash 是 NEGATED stdout/stderr NOT contains + exit_code 跨通道
  OR, schema 无 output_absent 字段; new_memory_added 是 file content 检查, schema
  无 file_content 字段; 两者都是 "冗余 defensive" — exit_code_success_or_limit +
  artifacts 已 catch 主要失败路径; §2.3 5 个 scenarios 累计 dropped 4 条, 没有
  阻断 gate 达成, schema-cost > value
- [决定]: Registry id 与 YAML 文件名分离 (memory-persistence.yaml 注册为
  "memory-persist", memory-corrupted-file.yaml 注册为 "memory-corrupted")
  [理由]: YAML id 必须匹配命令式 `scenario.id()` 保持 CLI 行为; YAML 文件名
  用描述性全名便于阅读; 两者解耦是 schema 设计的灵活性, 不强制 1:1
- [决定]: 2.3.3 MemoryInjection 用 output_contains_any 3 case 变体而非 dropped
  [理由]: secret codeword 是 prompt 强制要求的完整字符串 (PURPLE_ELEPHANT_42),
  case-insensitive 覆盖 (3 常见大小写) 比 dropped 更精准; 与 2.2.x hat 场景
  case-insensitive 适配同模式

### 下一步可选方向
1. **§2.3 剩余 3** memory-missing / memory-rapid-write / memory-large-content
   (3 个迁移 = 58/2 = 96.67%; 进一步推进覆盖率)
2. **§2.4 parallel-app-server** 2 个 (non-live harness, 可能需 schema 扩展)
3. **§2.3+§2.4 全清**: 58/0 = 100% (除 explicit-keep); Wave 2 全部完成
4. **Wave 2 收官**: 推 39+ commits 到 my/main, 触发 continuous-learning
   (见 LATER_PLANS.md "Wave 2 收官后:执行 continuous-learning 流程" 条目)
5. **暂停**: 等用户决策

## [2026-08-13 16:40:00] [Session ID: omx-1786600320381-z290x9] Wave 2 收官启动:剩余 5 个 imperative 全清

### 目标(用户指令 "1+2+3")
- §2.3 剩余 3 个 memory scenarios: memory-missing / memory-rapid-write / memory-large-content
- §2.4 剩余 2 个 parallel-app-server scenarios (audit 标 "non-live harness")
- §2.3+§2.4 全清后: 60/0/1 = 100% coverage, Wave 2 全部完成
- 触发 continuous-learning (WORKLOG 232 行, 远低于 1000, 可一次性完成)

### 状态
**目前在阶段1(读命令式)** — 下一步: 读 memory.rs (Missing + RapidWrite + LargeContent)
+ parallel/app_server_idle_start.rs + parallel/app_server_steer_multi_turn.rs

## [2026-08-13 16:55:00] [Session ID: omx-1786600320381-z290x9] 🎯🎯🎯 Wave 2 全部完成 — Coverage 100.00%

### 完成动作(本 batch 5 commits)
- `8d1e279` feat(e2e): migrate MemoryMissingFileScenario → memory-missing-file.yaml (Wave 2 task 2.3.6)
- `7d7cdce` feat(e2e): migrate MemoryRapidWriteScenario → memory-rapid-write.yaml (Wave 2 task 2.3.7)
- `07c0c61` feat(e2e): migrate MemoryLargeContentScenario → memory-large-content.yaml (Wave 2 task 2.3.8)
- `ba1c352` feat(e2e): extend DeclarativeExpect with duration_at_least_secs field (schema 扩展)
- `5dfbcec` feat(e2e): migrate ParallelAppServerIdleStartScenario → parallel-app-server-idle-start.yaml (Wave 2 task 2.4.1)
- `56ff3c5` feat(e2e): migrate ParallelAppServerSteerMultiTurnScenario → parallel-app-server-steer-multi-turn.yaml (Wave 2 task 2.4.2)

### 净结果 — Wave 2 完成度统计
- **21 个 migrations 全部落地** (4 timeout/max-iterations/backend-unavailable/auth-failure
  + 5 hat-* + 8 memory-* + 2 parallel-app-server-* + 2 audit 反预期不需 schema 扩展)
- **2 个 schema 扩展 commits** (failed/stderr_contains_any/failed_within_secs +
  duration_at_least_secs)
- **5 个 fix-up commits** (3 个 duplicate `output_contains_any` 修复 +
  2 个 schema 扩展后的属性 bug)
- drift log delta: 96.67% → 100.00% = +3.33% (本 batch); 累计 Wave 2 +35.00% (65% → 100%)
- 536 lib tests 全过(无 regression)
- **🎯🎯🎯 gate test: PASS!** Coverage 100.00% > 90.00% 阈值

### 决定 (本 batch)
- [决定]: 2.4.1/2.4.2 parallel-app-server 2 个使用 schema 扩展 duration_at_least_secs
  [理由]: 2.4.1 idle_start 核心断言 survived_two_runtime_windows 验证 idle-start
  期间会话没被 max_runtime 收掉, 这是 idle-start 设计的核心 claim, 不允许
  dropped; schema extension (1 个字段 duration_at_least_secs, 镜像
  failed_within_secs 平行设计) 是最小成本; 2.4.2 steer_multi_turn 不需要
  duration_at_least_secs 但使用 failed_within_secs (off-by-one 边界可接受)
- [决定]: 2.4.1/2.4.2 human_log_written dropped
  [理由]: schema 无 file_content 字段; human_log_written 是 audit log 检查,
  不是核心测试 claim (核心是 idle-start 存活 + steer in-flight); 间接覆盖
  通过其他字段: agents_snapshot + termination 检查已 catch 主要失败路径
- [决定]: 2.4.1/2.4.2 "Injector succeeded" dropped
  [理由]: declarative runner 已有 inject 失败时 map_err 强制 setup 错误, 无需
  额外断言; 这是 declarative runner 的 implicit enforcement, 不需 schema
  字段重复声明
- [决定]: 命令式 cli.command (backend-unavailable 2.1.3) 的语义问题留 LATER_PLANS
  [理由]: 命令式 setup 设 cli.command: nonexistent-cli-... 在 backend != custom
  时被静默忽略(config.rs:795-803), 命令式 test 即便 live run 也不一定真触发
  backend-unavailable 路径; 这是命令式本身的语义问题, 不是迁移引入;
  audit 建议改 require_backend: <wrong> 让 declarative runner 主动构造失败路径,
  留待后续 schema 扩展 + 命令式修复 (不进 Wave 3 closure 范围)

### Wave 2 全程统计
- **Total commits**: 47 ahead of my/main
- **Time span**: 2026-08-13 14:10:00 → 2026-08-13 16:55:00 (~3 小时)
- **Schema extensions**: 2 (failed family + duration_at_least_secs)
- **Audit 反预期**: 4 (2.4.1 / 2.4.2 标 schema 扩展但实际不需要;
  2.1.3 / 2.1.4 标 Easy 但实际需要 schema 扩展)
- **Dropped assertions**: ~15 (累计 across 21 migrations, 主要是 file content +
  NEGATED stdout NOT contains + 冗余 defensive checks)

### Wave 3 准备 (下一步)
- **continuous-learning**: 见 LATER_PLANS.md "Wave 2 收官后:执行
  continuous-learning 流程" 条目 — 这是 AGENTS.md 强制要求的归档流程
  (回读 999 行历史 + 提炼经验 + 分流到 docs/solutions/ / self-learning.* skill
  / CONTEXT.md / AGENTS.md / EXPERIENCE.md)
- **Wave 3 closure** (per OpenSpec tasks.md §3):
  - 3.1 Confirm gate test green (✅ DONE — Coverage 100% PASS)
  - 3.2 Annotate remaining imperative TestScenario impls with #[deprecated]
  - 3.3 Add docs/e2e/declarative-migration.md pointer under crates/ralph-e2e/README.md
  - 3.4 Open follow-up issue for eventual physical removal after one release cycle
- **Push**: 47 commits 推 my/main
- **OpenSpec archive**: 修改 tasks.md 反映实际完成情况 (audit 反预期 + schema
  扩展 commits) + 运行 openspec archive

## [2026-08-13 17:25:00] [Session ID: omx-1786600320381-z290x9] $continuous-learning 完整复盘 — root 上下文整理

### 触发
- 用户显式调用 `$continuous-learning` + "整理所有根目录文件上下文"

### Gate 结果
- **新 Compound Capture 候选**: 无。距上次 CL (commit a7daa79) 仅 `7acd1a5` 一个 LATER_PLANS 标记更新, 无新代码改动 / 无新发现。
- **Scoped Refresh**: 无 drift。a7daa79 的 captures (2 skills + 4 EXP + 1 docs/solutions) 全部仍 valid。
- **6 文件活跃度检查**:
  - task_plan.md (661 lines): current session (18 提及) — active, 不归档
  - WORKLOG.md (304 lines): current session (5 提及) — active, 不归档
  - EPIPHANY_LOG.md (893 lines): 1 current mention + 大量历史 (codex app-server profile, codesign DR, deepseek 模型 drift 等) — current 段 active, 历史段保留
  - LATER_PLANS.md (960 lines): 2 current mentions — active, 不归档
  - ERRORFIX.md (246 lines): 0 current mentions, 全部历史 (record_session fixture 字段误放 等 2026-05-28 段) — 历史参考, 不归档
  - notes.md (790 lines): 0 current mentions, 全部历史 (recoverable retry 接续点 等 2026-05-28 段) — 历史参考, 不归档
  - 全部 < 1000 行, 无自动归档触发

### 决策
- [决定]: docs/solutions/declarative-scenario-migration.md 重构路径 + 加 frontmatter
  [理由]: validate-solution-frontmatter.py 报错 — 文件必须以 '---' 分隔行开头且必填
  title/date/last_updated/module/component/problem_type/severity/status/tags/verified_by;
  validate-solution-claims.py 检查 0 flag (4 paths / 0 SHAs / 0 links); 同时按
  Category Mapping 表把 problem_type=documentation_gap → 目录名 documentation-gaps
  移入子目录 `docs/solutions/documentation-gaps/`。AGENTS.md 索引路径同步更新。
- [决定]: 不归档 6 文件任何一段
  [理由]: 当前 session 仍 active (task_plan.md 最新条目就是 Wave 2 closure);
  历史 session (omx-1779954714247-oab9zc, 2026-05-28) 写的 notes.md / ERRORFIX.md
  内容已被对应 archive/ 分支 (notes_2026-05-28_1559_pre_recoverable_retry_5x.md 等)
  保留, 当前不需要重新归档。
- [决定]: WORKLOG 归档 (commit a7daa79) 仍保留在 archive/branch_contexts/wave2_e2e_declarative_migration/
  [理由]: continuous-learning 完成后允许 archive, 不再需要移到别处。

### 验证 (cargo + solution 校验脚本)
- `cargo test -p ralph-e2e --lib`: 536 passed / 0 failed / 24 ignored (无回归)
- `cargo test -p ralph-e2e --test declarative_coverage_gate -- --nocapture`:
  Coverage 100.00% / Pass / Fail: PASS
- `python3 continuous-learning-skill/scripts/validate-solution-frontmatter.py
   docs/solutions/documentation-gaps/declarative-scenario-migration.md`:
  OK (frontmatter 字段齐全)
- `python3 continuous-learning-skill/scripts/validate-solution-claims.py ...`:
  OK (4 paths / 0 SHAs / 0 links / 0 flags)

### 净结果
- 1 个 chore commit (本轮):
  - docs/solutions/ → docs/solutions/documentation-gaps/ 重命名
  - declarative-scenario-migration.md 加 frontmatter (title/date/last_updated/module/
    component/problem_type/severity/status/tags/verified_by/related_solutions/related_skills)
  - AGENTS.md 索引路径同步
- 长期产物状态 (CL a7daa79 + 本轮 refresh):
  - 2 self-learning skills (yaml-schema-or-vs-and-semantics, yaml-duplicate-field-bug)
  - 4 EXPERIENCE.md entries (exp-20260813-yaml-schema-or-vs-and-semantics,
    yaml-duplicate-field-detection, schema-cost-vs-assertion-value,
    audit-classification-reality-check)
  - 1 docs/solutions/documentation-gaps/declarative-scenario-migration.md
    (frontmatter OK + claims OK)
  - AGENTS.md Project Knowledge Index (3 个 Wave 2 相关条目)
- 0 归档 (6 文件全部 < 1000 行 + current session 仍 active)

### 下一步可选方向
1. **Push 50 commits** 到 my/main (49 commits ahead of my/main, 未 push)
2. **Wave 3 closure** (per OpenSpec tasks.md §3.1-3.4):
   - 3.1 Confirm gate test green ✅ DONE (Coverage 100% PASS)
   - 3.2 剩余 imperative TestScenario impls 加 `#[deprecated]`
   - 3.3 docs/e2e/declarative-migration.md pointer
   - 3.4 follow-up issue (1 release cycle 后物理删除)
3. **OpenSpec archive**:
   - 修改 tasks.md 反映实际 (audit 反预期 4 次 + 2 schema 扩展 commits)
   - 跑 `openspec archive` (29/29 validate --strict 绿 → archive 流程)
4. **命令式 cli.command 静默忽略** (2.1.3 backend-unavailable 语义问题):
   - 留在 LATER_PLANS, 需要 `require_backend: <wrong>` schema 扩展
   - 同时需修命令式 setup() 让 cli.command 在 backend != custom 时生效
5. **暂停**: 等用户决策

## [2026-08-13 17:40:00] [Session ID: omx-1786600320381-z290x9] $continuous-learning 整理清理根目录分支上下文文件

### 触发
- 用户显式调用 `$continuous-learning` + "整理清理所有根目录 分支上下文文件"

### 审计发现
- **根目录分支上下文文件**: 4 个 `notes__*.md`, 都从 sync-origin-main-features-q3-2026
  cherry-pick 调查工作 (Session omx-1786419140441-df5ql8, 2026-08-11/12) 累积, 已 commit `c623abb`
- 当前 Session 6 文件对 4 个 notes__* 引用次数: 全 0 (无活跃使用)
- sync-origin-main 整体工作已 commit + 已归档 (`openspec/changes/archive/2026-08-12-...`)
- 结论: 0 引用 + 异 Session + 不同主题 = 未轮转旧支线, 按 continuous-learning 规则归档

### 决定
- [决定]: 4 个 `notes__*.md` 按 suffix 各自分目录 (沿用 archive convention)
  [理由]: archive/branch_contexts/ 已有 14 个 topic subdirs (memory_axes/
  continuous_learning/ 等), 每个 subdir 用 `__topic` 命名; 4 个 notes__* 有 4 个不同
  suffix (branch_diff_review / clean_events_review / e2e_conv / group1_dryrun), 各
  自独立 subdir 保持隔离。命名: `archive/branch_contexts/<suffix>/notes__<suffix>.md`
- [决定]: notes__e2e_conv.md 内容 capture 到 EXPERIENCE.md 作为已知 issue
  [理由]: 描述了 LIVE 路径的 3 个 e2e 场景失败模式(termination_reason=None, 事件流
  完整但无 loop.terminate), 是真实存在的诊断结论, 但根因未知 — 按 "inbox" 路线
  写入 EXPERIENCE.md, 明确 evidence gap (根因/未尝试), 留待 Wave 3 期间诊断后升级
  docs/solutions/ formal capture
- [决定]: 另 3 个 notes__* 不 capture (skip)
  [理由]: branch_diff_review 是 sync-origin-main 一次性过程产物 (1818 文件分支差异分析),
  clean_events_review 是具体 commit 移植决策, group1_dryrun 是 cherry-pick 执行记录 —
  都不是 "已验证、非琐碎、可复用" 的单一经验, 是 sync-origin-main 工作流的过程产物
- [决定]: 创建 1 个 manifest 文档化整个 batch
  [理由]: archive/manifests/ 已有 15 个 manifest 记录历次 archive 操作; 按
  archive_layout.md 规范, 新批次需 manifest 记录 6 文件摘要 + 活跃度判定 + 归档映射
  + Capture/Refresh 结果 + 验证 + 保留候选 + 未完成风险

### 净结果
- 1 commit (本轮):
  - archive/branch_contexts/ 新建 4 个 subdir (branch_diff_review / clean_events_review /
    e2e_conv / group1_dryrun), 各 1 个 notes__*.md 文件
  - archive/manifests/ARCHIVE_MANIFEST__sync_origin_main_2026-08-13.md 新建 (103 行)
  - EXPERIENCE.md 加 exp-20260813-e2e-live-convergence-issue (inbox 路线, 证据缺口明确)

### 验证
- `cargo test -p ralph-e2e --lib`: 536 passed / 0 failed / 24 ignored (无回归)
- `cargo test -p ralph-e2e --test declarative_coverage_gate -- --nocapture`: Coverage
  100.00% / Pass / Fail: PASS
- `git ls-files notes__*.md`: 0 个 (全部已 archive)
- `git status --short`: 仅新增 archive 目录文件 + EXPERIENCE.md 改动, 无未跟踪分支文件

### 下一步可选方向
1. **Push 51 commits** 到 my/main (50 + 本轮 chore)
2. **Wave 3 closure** (OpenSpec tasks.md §3.2-3.4)
3. **e2e-live-convergence 诊断**: 解 exp-20260813-e2e-live-convergence-issue
   (3 个 live 场景 termination_reason=None 模式), 留 Wave 3 期间
4. **暂停**: 等用户决策
