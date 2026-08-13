# WORKLOG.md

本文件覆盖 Wave 2 任务 2.2.2 ~ 2.2.5 (4 个 hat scenarios 迁移)。

历史归档:
- `WORKLOG__2026-08-13_pre_section_2_2_4_migrations.md` — Wave 1 + §2.1 (4 commits) +
  schema 扩展 + §2.2.1 (1 commit) 共 999 行, 超过 1000 行触发归档。

## [2026-08-13 15:38:00] [Session ID: omx-1786600320381-z290x9] 任务名称: Wave 2 §2.2 全部 5 个 hat scenarios (2.2.1 + 2.2.2-2.2.5 batch)

### 任务内容
- 用户指令 "进行1234" 一次性跑完 §2.2 剩余 4 个 hat scenarios
- 5 个场景共 25 条命令式断言映射到 schema 字段, 包含 2 处 dropped + 5 处 OR 折 AND + 大量 case 变体适配
- WORKLOG 在 999 行临界, 提前归档 + 推迟 continuous-learning 到 Wave 2 收官

### 完成过程
**Phase 0 — 准备(commit 7d19d02)**
- WORKLOG 归档 → WORKLOG__2026-08-13_pre_section_2_2_4_migrations.md (根目录, 未移入 archive/)
- 新 WORKLOG.md 仅 header + 历史归档索引 (7 行)
- EPIPHANY_LOG: 决策 + 推迟理由 + 当前结论
- LATER_PLANS: "Wave 2 收官后执行 continuous-learning" 条目

**Phase 1 — 2.2.1 HatSingleScenario (commit 7e4e970 + 2c20091, 上一轮)**
- 5 命令式断言 → 5 schema 字段 1:1 映射(无 dropped)
- 适配: case-insensitive 6 变体 + starts_with("build.") 用 2 精确 topic

**Phase 2 — 2.2.2 HatInstructionsScenario (commit cedaab1 + fix d9f7c79)**
- 5 命令式断言 → 5 schema 字段, 但 verdict_provided 是 OR 折 AND
- BUG: 写 YAML 时 expect 顶层有 2 个 output_contains_any: 块, serde_yaml 报错
  duplicate field; fix-up commit d9f7c79 合并为 1 个字段 2 nested lists
- 回归教训: 写顶层 schema 字段必须 uniq -c 校验

**Phase 3 — 2.2.3 HatEventRoutingScenario (commit 13cff39)**
- 5 命令式断言 → 4 schema 字段(1 dropped)
- wrong_hat_not_triggered 的 stdout NOT contains "DEPLOYMENT STATUS:" 部分
  schema 无 output_absent 字段, dropped — 实际 deployer 误激活会 emit
  deploy.* event, event_absent_prefixes 已 catch 主要失败路径

**Phase 4 — 2.2.4 HatBackendOverrideScenario (commit e40832a)**
- 5 命令式断言 → 4 schema 字段(1 dropped)
- config_parsed_successfully 是 NEGATED stderr 检查, dropped — exit_code_success_or_limit
  已经 catch config 失败

**Phase 5 — 2.2.5 HatMultiWorkflowScenario (commit cac1d94)**
- 5 命令式断言 → 5 schema 字段, workflow_progressed 是 OR 折 AND(3 events 全出现)
- both_hats_contributed OR 用 8 case 变体单字段完全保留

**Phase 6 — Verification**
- cargo check ok
- cargo test --lib: 534 passed / 0 failed (无 regression)
- cargo run -- --list: 5 hat-* scenarios 全显示 (declarative)
- gate test: drift 73.33% → 80.00%

### 总结感悟
- **duplicate YAML key 是 serde_yaml 静默杀**: 我在 2.2.2 写出 2 个 output_contains_any:
  块, cargo check ok, cargo test --lib ok(单元测试), 但 cargo run -- --list 才会真正
  反序列化 YAML, 然后 panic 退出。教训: 写完每个 YAML 必须跑 cargo run -p ralph-e2e --
  --list 验证, 而不是依赖单元测试通过就以为 OK。后续可加一个集成测试强制启动
  binary 验证所有 YAML 都能 parse(scenario.rs 已经有类似 inline config 测试)。
- **OR 折 AND 在 hat 类场景是正确方向**: 命令式 OR 是 defensive — "任一出现即通过";
  runner 的 AND 强制多字段都通过, 看似失真。但 hat instructions 强制要求所有产物
  出现(text + event + verdict), AND 实际上更接近 "指令遵循" 的真实期望。ponytail
  "stricter check 优于 lenient check" 在 hat 类场景成立, 因为 hat 协议有明确的
  输入输出契约。
- **NEGATED 断言是 dropped 的候选**: schema 无 output_absent / stderr_absent 字段;
  NEGATED 断言本质是 "不要 X", 需要额外的 absent 类字段; 2 处 dropped 都是
  "redundant defensive check" — 主要失败路径已经被正向字段(exit_code /
  event_absent_prefixes) catch。
- **case-insensitive 适配的 ponytail 边界**: 4 个 hat 场景 + 2.1.3/2.1.4 错误场景
  共 6 个场景都有 case-insensitive stdout 关键词检查。若 schema 加
  `output_contains_any_case_insensitive: bool` 是 single field covering all 6,
  节省 6 个 YAML 的 case 变体组。考虑升级时机: 当前每个变体组 ~6-8 个字符串,
  总成本尚可; 升级 schema 是 premature abstraction。等 §2.3 memory 全部完成再评估
  (memory 8 个场景可能还有 case-insensitive, 总数到 14+ 再升 schema)。
- **`# ponytail:` 本轮**: 无独立 lazy 简化空间, 但「OR 折 AND」与「NEGATED dropped」
  是两次 ponytail 应用 — 不为 hypothetical 加 schema 字段, 而用已有的 runner 语义
  处理。
- **WORKLOG 提前归档的代价**: 把 "每个迁移一个 chore commit" 模式打破, 改为
  "§2.2 batch 一个 chore commit"; 代价是 chore 变长(本节 ~80 行), 但避免连续触发
  1000 行规则 + 提前执行 continuous-learning 的不完整流程。trade-off 评估 OK。
