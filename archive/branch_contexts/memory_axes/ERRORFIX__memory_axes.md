## [2026-03-25 21:09:31] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 问题: `parallel-experimental-dev-engine-example` 在 integrator 收尾阶段超时,缺失 `integration.applied` / `experiment.complete`

### 现象
- 真后端运行:
  - `cargo run -p ralph-e2e -- codex --filter parallel-experimental-dev-engine-example --keep-workspace --verbose`
- 旧失败现场表现为:
  - `experiment.result` / `experiment.reviewed` 已完整出现
  - `integration.task` 已派发给 `experiment_integrator#1`
  - integrator 的 `git cherry-pick` 与 `rg` 都成功
  - 但顶层 600s E2E 护栏前没有出现 `integration.applied` / `experiment.complete`
  - report 最终是 `Exit code 130` + `Timed out`

### 原因
- 已验证的高影响原因:
  - scenario 的隔离 workspace 会 clone 整个仓库根目录
  - 若不额外处理,worker 会继承 workspace 根的开发型 `AGENTS.md`
  - 这会把 example worker 带进仓库级工作流,扩大任务范围,增加不必要的上下文和动作
  - 对本场景而言,这会显著拖慢 integrator 的“验证后上报事件”阶段
- 重要边界:
  - 这轮没有把所有历史长尾绝对归因到单一因素
  - 但动态复跑已经证明: 仅修掉这个污染源后,scenario 就恢复 PASS

### 修复
- 在 `crates/ralph-e2e/src/scenarios/parallel_experimental_dev_engine_example.rs` 中:
  - 新增 `E2E_WORKSPACE_AGENTS_OVERRIDE`
  - 新增 `write_workspace_root_agents_override(workspace)`
  - 在 scenario setup 阶段覆盖隔离 workspace 根 `AGENTS.md`
  - 再把覆盖内容随 snapshot commit 固化进 `HEAD`,确保 worktree job 也看到相同规则

### 验证
- `cargo fmt --all`
- `cargo test -p ralph-e2e scenarios::parallel_experimental_dev_engine_example::tests::seeded_workspace_snapshot_commit_makes_patched_prompt_visible_to_worktree -- --exact`
- `cargo test -p ralph-e2e scenarios::parallel_experimental_dev_engine_example::tests::example_config_requires_structured_commit_fields_for_review_and_integration -- --exact`
- `cargo run -p ralph-e2e -- codex --filter parallel-experimental-dev-engine-example --keep-workspace --verbose`
- 关键结果:
  - `.e2e-tests/report.json` -> `passed: true`
  - `.ralph/events.jsonl` 出现:
    - `integration.applied`
    - `experiment.complete`
  - `.e2e/stdout.txt` 出现:
    - `experiment_integrator#1:out:job=1 <event topic="integration.applied"...>`
    - `experiment_integrator#1:out:job=1 <event topic="experiment.complete"...>`
    - `ralph#1:out:job=4 LOOP_COMPLETE`

## [2026-03-31 02:34:16] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 问题: `parallel-experimental-dev-engine-example` 第一轮 live run 实际闭环但被 scenario 误判,且 example / E2E 缺少轻量 all-hat overlay 出口

### 现象
- 同一 session 中,第一轮真后端 live run 曾出现:
  - workflow 事件链实际已经走通
  - 但 scenario 仍把 `evidence_ok` 统计成失败
- 同时,example / E2E worker 会继续继承编译期内嵌的 all-hat overlay:
  - 提示词偏重
  - 对示例场景来说噪音偏多
- 当前最终报告文件显示:
  - `.e2e-tests/report-live.md` -> `Passed: 1 | Failed: 0`
  - `.e2e-tests/report.json` -> `evidence_ok=3`

### 原因
- 已验证的原因1:
  - scenario 里的 `payload_field_is_true()` 过于依赖字符串形态
  - 当 `experiment.reviewed` payload 使用 YAML 形式 `evidence_ok: true` 时,会存在误判空间
- 已验证的原因2:
  - all-hat overlay 原先只靠编译期内嵌默认内容
  - 缺少 runtime 级别的显式覆写出口
  - 这让 example / E2E 只能被动继承开发型重提示词

### 修复
- 在 `crates/ralph-core/src/config.rs` 增加 `core.all_hat_prompt`
- 在 `crates/ralph-core/src/prompt_overlay.rs` 实现四种来源:
  - `compiled`
  - `disabled`
  - `inline`
  - `file`
- 在 `crates/ralph-e2e/src/scenarios/parallel_experimental_dev_engine_example.rs`:
  - 为 E2E 注入 `E2E_LIGHT_ALL_HAT_PROMPT`
  - 把 example patched config 固定为 `core.all_hat_prompt.mode: inline`
  - 将 `payload_field_is_true()` 改为 `serde_yaml::Value` 结构化解析

### 验证
- `cargo fmt`
- `cargo test -p ralph-core --lib`
- `cargo test -p ralph-core smoke_runner`
- `cargo test -p ralph-e2e scenarios::parallel_experimental_dev_engine_example::tests::seeded_workspace_snapshot_commit_makes_patched_prompt_visible_to_worktree -- --exact`
- `cargo test -p ralph-e2e scenarios::parallel_experimental_dev_engine_example::tests::patch_example_config_for_e2e_adds_lightweight_all_hat_overlay -- --exact`
- `cargo test -p ralph-e2e scenarios::parallel_experimental_dev_engine_example::tests::payload_field_is_true_accepts_yaml_and_both_json_spacing_styles -- --exact`
- `cargo run -p ralph-e2e -- codex --filter parallel-experimental-dev-engine-example --keep-workspace --verbose`
- 关键结果:
  - `.e2e-tests/report-live.md`:
    - `Passed: 1 | Failed: 0`
    - `parallel-experimental-dev-engine-example (507.8s)`
  - `.e2e-tests/report.json`:
    - `passed: true`
    - `counts: task=3, result=3, reviewed=3, evidence_ok=3`
    - `new_jobs_after=[]`
