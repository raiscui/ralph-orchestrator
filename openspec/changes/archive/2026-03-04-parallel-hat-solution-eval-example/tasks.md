## 1. Example 配置与文档

- [x] 1.1 新增 `examples/parallel-experimental-dev-engine/ralph.yml`（并行 hats“开发永动机”参考配置）
- [x] 1.2 新增 `examples/parallel-experimental-dev-engine/README.md`（说明输入格式、运行方式、你应该看到什么）
- [x] 1.3 在示例工作流中加入独立 `experiment_auditor`（硬门槛审计：产出 `experiment.reviewed`）
- [x] 1.4 在示例工作流中实现“自适应并行度 + 窗口派发”（ralph#1 推断 P_max，运行中 AIMD 调参，禁止洪水式派发）
- [x] 1.5 在示例工作流中加入独立 `experiment_integrator`（主工作区采纳/集成：消费 `integration.task`，产出 `integration.applied`/`integration.rejected`）
- [x] 1.6 收紧 runner 产物要求：`experiment.result` 必须包含 `commit`（避免在 payload 里嵌入超长 `patch`）；auditor 以 commit 作为最低审计载体
- [x] 1.7 视需要更新仓库文档，补充该 example 的入口链接（例如 `README.md` / `examples/` 索引）

## 2. Replay fixture 与 smoke tests

- [x] 2.1 新增 replay fixture：`crates/ralph-core/tests/fixtures/parallel_experimental_dev_engine.jsonl`
- [x] 2.2 在 `crates/ralph-core/tests/smoke_runner.rs` 增加针对该 fixture 的 smoke 测试（验证能收敛且不依赖 live backend）
- [x] 2.3 fixture 必须覆盖：`experiment.task`、`experiment.result`、`experiment.reviewed`、`integration.task`、`integration.applied`、`experiment.complete`（并且能收敛到 `LOOP_COMPLETE`）

## 3. 验证（门禁）

- [x] 3.1 运行 `cargo fmt --check`
- [x] 3.2 运行 `cargo clippy --all-targets --all-features -- -D warnings`
- [x] 3.3 运行 `cargo test`
- [x] 3.4 运行 `cargo test -p ralph-core smoke_runner`

## 4. E2E（真后端，Codex）

- [x] 4.1 新增 E2E scenario：`crates/ralph-e2e/src/scenarios/parallel_experimental_dev_engine_example.rs`（直接跑 example，并在 workspace 预填 EXPERIMENT_PLAN）
- [x] 4.2 注册并暴露该 scenario（`crates/ralph-e2e/src/scenarios/mod.rs` / `crates/ralph-e2e/src/main.rs`）
- [x] 4.3 运行 `cargo test -p ralph-e2e`（至少保证编译通过）
