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
