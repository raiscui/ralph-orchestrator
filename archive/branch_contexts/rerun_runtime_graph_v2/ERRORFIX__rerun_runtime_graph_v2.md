## [2026-04-30 10:20:12] [Session ID: 019ddbe6-a5ae-7950-8ba0-27b3b9f53219] 错误修复: V2 durable replay graph 实现过程中的编译与测试漂移

### 问题

- `cargo check` 曾报 `E0502`: 在 `spawn_instances` 遍历 registry 时写 lifecycle durable record, 造成不可变借用和可变借用冲突。
- targeted test 命令参数顺序曾写错: `--exact` 被放在 Cargo 参数区, 而不是 test harness 参数区。
- 扩展 spawn 测试后, `events_path` 被 move 给 `make_supervisor`, 后续读取事件文件时触发 PathBuf move 错误。
- 旧测试 `queue_decision_is_loaded_from_history_and_not_recomputed` 原先断言 events 文件只有一行, 但 V2 会新增 lifecycle durable records, 旧断言不再符合新语义。

### 原因

- lifecycle durable record 写入需要 `&mut self`, 但 registry iterator 仍持有 `&self`。
- Rust test harness 参数必须放在 `--` 之后。
- 原测试过去不需要再次读取 `events_path`, 新增 durable record 断言后需要保留路径。
- V2 的正确行为就是增加 observer-only runtime records, 所以测试不能再把"事件总行数"当成旧行为契约。

### 修复

- `spawn_instances` 中先收集需要落盘的 configured instance ids, 遍历结束后再统一写 lifecycle create records。
- targeted test 命令统一改成 `cargo test --package <pkg> --lib <test_path> -- --exact`。
- 调用 `make_supervisor` 时传入 `events_path.clone()`, 保留后续读取路径。
- 旧测试改为只统计 `dispatch.decision` topic, 继续锁住 queue decision 不被重复计算的行为。

### 验证

- `openspec validate rerun-runtime-graphs --strict`: 通过。
- `cargo test`: 当前 session 完整通过, exit code 0。
- focused tests 在上一轮已通过, 包括 runtime delivery record、lifecycle controls、spawn direct delivery、queue decision replay、CLI runtime graph replay 和 integration runtime graph。

### 后续提醒

- 之后如果继续扩展 runtime graph replay, 不要让普通 workflow event schema 背 runtime graph 私有字段。
- 对 replay fidelity 的判断要保守。缺 durable evidence 就标 approximate, 不要用推断结果冒充 full-fidelity。

## [2026-04-30 21:27:10] [Session ID: 019ddbe6-a5ae-7950-8ba0-27b3b9f53219] 错误修复: archived change 校验命令使用错误

### 问题

- 归档后执行 `openspec validate 2026-04-30-rerun-runtime-graphs --strict` 返回 `Unknown item`。

### 原因

- OpenSpec `validate [item-name]` 解析的是 active change 或 spec item 名, 不按 archive 目录名解析 archived change。

### 修复

- 查询 `openspec validate --help`。
- 改用 `openspec validate --all --strict` 做归档后全量校验。
- 额外用 `openspec validate runtime-graph-observability --strict` 校验已经同步到主目录的 spec。

### 验证

- `openspec validate --all --strict`: 16 passed, 0 failed。
- `openspec validate runtime-graph-observability --strict`: valid。

## [2026-05-01 15:48:09] [Session ID: 019de280-f42a-7171-a1e8-63aed3aef17d] 错误修复: V2 验证命令误用修正

### 问题

- 第一次运行单个 Rust 测试时,把 `--exact` 放在 cargo 参数位置:
  - `cargo test --package ralph-core --lib ... --exact`
- cargo 报错:
  - `error: unexpected argument '--exact' found`
- 第一次复核 dynamic spawn durable lifecycle 测试时,写错测试名:
  - cargo 输出 `running 0 tests`

### 原因

- `--exact` 是 Rust test harness 参数,必须放在 `--` 后面。
- `running 0 tests` 不是通过,只是过滤条件没有匹配到任何测试。

### 修复

- 单个测试统一改为:
  - `cargo test --package ralph-core --lib <test_path> -- --exact`
- 用 `rg` 查真实测试名,把 dynamic spawn 验证改为:
  - `parallel::supervisor::routing_tests::spawn_instance_forces_new_dynamic_instance_and_delivers_direct`

### 验证

- 修正后的 focused tests 全部通过。
- `cargo fmt --all --check` 通过。
- `cargo test` 通过。
- `cargo test -p ralph-core smoke_runner` 通过。

### 后续提醒

- 以后看到 `running 0 tests` 必须当成验证无效,不能记录为 passed。
- 单个测试的 `--exact` 必须放在 `--` 后面。
