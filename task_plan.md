# 任务计划: 当前默认上下文续档后索引

## 目标
保持默认六文件短小可读,并把当前正在推进的 agent guidance contract governance 支线显式索引出来。

## 阶段
- [x] 阶段1: 续档超过 1000 行的旧 task_plan
- [x] 阶段2: 写入持续学习经验
- [ ] 阶段3: 推进 guidance_contract_governance 支线
- [ ] 阶段4: 验证并交付

## 关键问题
1. 为什么新开默认 task_plan? 因为旧 `task_plan.md` 已有 1027 行,超过项目阈值。
2. 旧 task_plan 去哪里了? `archive/default_history/task_plan_2026-05-11_171940.md`。
3. 当前实际工作在哪里记录? `task_plan__guidance_contract_governance.md` 等同后缀支线文件。

## 做出的决定
- 只续档默认 `task_plan.md`,不移动当天活跃支线。
- 将 oh-my-codex 学习落地顺序沉淀到 `EXPERIENCE.md`。

## 遇到错误
- 暂无。

## 状态
**目前在阶段3** - 进入 `guidance_contract_governance` 支线。

## [2026-05-11 17:19:40] [Session ID: omx-1778475786175-ogndry] [支线索引]: 启用 guidance_contract_governance 上下文集

- 启用原因:
  - 用户要求“按建议进行”,即从 `specs/oh-my-codex-learning-analysis.md` 的建议开始落地。
  - 当前目标是先做 agent guidance schema / prompt contract / manifest verifier 的最小治理闭环。
- 支线上下文:
  - `task_plan__guidance_contract_governance.md`
  - `notes__guidance_contract_governance.md`
  - `WORKLOG__guidance_contract_governance.md`
  - `ERRORFIX__guidance_contract_governance.md`
  - `LATER_PLANS__guidance_contract_governance.md`
  - `EPIPHANY_LOG__guidance_contract_governance.md`
- 当前边界:
  - 先建 OpenSpec change 和 docs/tests。
  - 不先照搬完整 OMX team/tmux runtime。
  - 不清理或回滚工作区已有其他改动。

## [2026-05-11 22:45:00] [Session ID: omx-1778510695653-7pd7o2] 任务计划: 比较 ralph-orchestrator、oh-my-codex 与 hermes-agent

### 目标
- 用真实仓库证据说明三个项目的定位、架构边界、运行方式和差异, 避免只按名称猜。

### 阶段
- [x] 阶段1: 读取项目规则和上下文文件概况
- [ ] 阶段2: 查询本地长期记忆和当前项目经验索引
- [ ] 阶段3: 查看 `ralph-orchestrator` 与 `/Users/cuiluming/local_doc/l_dev/my/rust/oh-my-codex` 的 README、入口和关键代码
- [ ] 阶段4: 查看 `https://github.com/NousResearch/hermes-agent` 的当前公开资料
- [ ] 阶段5: 汇总对比结论并记录工作日志

### 当前状态
- 正在进入阶段2。先查本地记忆和项目经验, 再做仓库级证据对比。

## [2026-05-11 22:58:00] [Session ID: omx-1778510695653-7pd7o2] 状态更新: 查询长期记忆和项目经验

- 行动目的: 先确认历史上对 `ralph-orchestrator` 和 `oh-my-codex` 的定位记录, 避免重复踩旧坑或误读两个仓库关系。
- 即将执行: 搜索 `/Users/cuiluming/.codex/memories/MEMORY.md`, 并读取本仓库 `EXPERIENCE.md` 中相关条目。

## [2026-05-11 22:59:00] [Session ID: omx-1778510695653-7pd7o2] 状态更新: 读取两个本地仓库事实源

- 行动目的: 对比必须基于实际仓库入口和文档, 不能靠项目名判断。
- 即将执行: 使用 `omx explore` 和普通只读命令查看 `ralph-orchestrator` 与 `oh-my-codex` 的 README、manifest、目录和关键文档。

## [2026-05-11 23:01:00] [Session ID: omx-1778510695653-7pd7o2] 遇到问题: omx explore 未及时返回

- 现象: `omx explore --prompt ...` 90 秒内没有输出。
- 处理: 不把它当作阻塞项, 改用普通只读命令直接读取 README、Cargo 和关键文档。
- 影响: 本次只是分析任务, 不需要依赖 `omx explore` 才能继续。

## [2026-05-11 23:06:00] [Session ID: omx-1778510695653-7pd7o2] 状态更新: 已完成事实源收集

- [x] 阶段2: 查询本地长期记忆和当前项目经验索引
- [x] 阶段3: 查看 `ralph-orchestrator` 与 `oh-my-codex` 的 README、入口和关键代码
- [x] 阶段4: 查看 `hermes-agent` 的当前公开资料
- [ ] 阶段5: 汇总对比结论并记录工作日志

### 当前状态
- 正在进入阶段5。接下来输出结构化对比, 并把本轮分析写入 WORKLOG。

## [2026-05-11 23:10:00] [Session ID: omx-1778510695653-7pd7o2] 完成记录: 对比分析已完成

- [x] 阶段5: 汇总对比结论并记录工作日志

### 验证证据
- 本地证据: `README.md`, `Cargo.toml`, `crates/ralph-core/src/lib.rs`, `crates/ralph-cli/src/main.rs`。
- 本地证据: `/Users/cuiluming/local_doc/l_dev/my/rust/oh-my-codex/README.md`, `package.json`, `Cargo.toml`, `src/index.ts`, `docs/adapt.md`, `src/adapt/hermes.ts`。
- 外部证据: `https://github.com/NousResearch/hermes-agent` 和 `https://hermes-agent.nousresearch.com/docs/`。

### 状态
- 本次是只读分析任务, 没有修改业务代码。
- 已记录到 `notes.md` 和 `WORKLOG.md`。
- 已检查 `LATER_PLANS.md` 和 `EPIPHANY_LOG.md`, 本轮无新增延期项或重大风险项。

## [2026-05-12 17:21:00] [Session ID: omx-1778510695653-7pd7o2] 任务计划: 重新审视 Ralph 演进方向优先级

### 目标
- 基于当前新增内容和仓库现状,重新判断 Ralph 现在最值得继续做什么,避免只按上一轮口头方向惯性推进。

### 阶段
- [x] 阶段1: 查看工作区状态和上下文文件概况
- [ ] 阶段2: 聚类当前新增内容和活跃 OpenSpec change
- [ ] 阶段3: 对照 runtime evidence / adapter contract / guidance governance / resource bootstrap 等方向做价值排序
- [ ] 阶段4: 输出建议并记录本轮只读分析

### 关键边界
- 这是重新审视和排序任务,默认只读分析,不修改业务代码。
- 不回滚、不整理用户或其他智能体已有改动。
- 结论必须区分已观察事实、候选判断和建议。

### 状态
**目前在阶段2** - 已完成工作区概况检查,接下来聚类新增内容和活跃 change。

## [2026-05-12 17:33:00] [Session ID: omx-1778510695653-7pd7o2] 完成记录: 重新审视已完成

### 阶段状态
- [x] 阶段1: 查看工作区状态和上下文文件概况
- [x] 阶段2: 聚类当前新增内容和活跃 OpenSpec change
- [x] 阶段3: 对照 runtime evidence / adapter contract / guidance governance / resource bootstrap 等方向做价值排序
- [x] 阶段4: 输出建议并记录本轮只读分析

### 验证证据
- `openspec list` 显示 completed / open change 分布。
- `openspec validate --all --strict` 输出 21 passed, 0 failed。
- `.agent/tasks.jsonl` 仍显示 docs gate 任务打开。
- 本轮只读分析已记录到 `notes.md` 和 `WORKLOG.md`。

### 状态
**已完成本轮重新审视** - 接下来可按“先收口 completed changes,再补 adapter contract tests,再做 startup bootstrap v1”的顺序推进。

## [2026-05-12 17:38:00] [Session ID: omx-1778510695653-7pd7o2] 状态更新: 开始第1步 completed OpenSpec changes 收口

- 行动目的: 把已经完成的 active changes 从工作区主战场移入 archive,避免后续实现时重复规划和状态漂移。
- 即将执行:
  1. 读取 bulk archive 技能说明。
  2. 使用 `openspec list` / `openspec status` / tasks checkbox 识别 completed changes。
  3. 检查 delta spec 是否已经同步到主 spec。
  4. 先跑 archive 前验证,再移动目录。
- 边界:
  - 不归档仍有未完成任务的 `startup-resource-bootstrap`、`runtime-capability-invocation`、`tui-mdfried-viewer`。
  - 不回滚或整理无关工作区改动。

## [2026-05-12 17:42:00] [Session ID: omx-1778510695653-7pd7o2] 遇到错误: zsh 字符串未按空格拆分

- 现象: archive 前核验命令把 6 个 change 名称作为一个字符串传给 `openspec status`,报 `Change name cannot contain spaces`。
- 原因: zsh 默认不启用 `SH_WORD_SPLIT`,普通字符串变量不会像 bash 一样自动按空格拆成多个循环项。
- 处理: 改用 zsh 数组逐个遍历 change。

## [2026-05-12 17:45:00] [Session ID: omx-1778510695653-7pd7o2] 状态更新: 执行 completed changes archive

- 行动目的: 归档 6 个 artifacts/tasks 均完成的 active changes,并让 `openspec archive` 同步 delta specs 到主 specs。
- 即将执行:
  - `openspec archive -y state-operation-layer`
  - `openspec archive -y prompt-contract-runtime-alignment`
  - `openspec archive -y agent-guidance-catalog-cli`
  - `openspec archive -y agent-guidance-contracts`
  - `openspec archive -y scoped-experience-system`
  - `openspec archive -y event-id-and-reply`
- 验证:
  - archive 后运行 `openspec validate --all --strict`。

## [2026-05-12 17:48:00] [Session ID: omx-1778510695653-7pd7o2] 遇到问题: event-id-and-reply archive spec sync 重复 requirement

- 现象: `openspec archive -y event-id-and-reply` 同步 `parallel-hat-instances` 时提示 `Every routed event has a stable id` 已存在,然后中止 spec update。
- 当前判断: 这更像 delta spec 已经同步过主 spec,而不是实现缺失。
- 下一步: 对比 delta spec requirement 名称和主 spec 内容;若主 spec 已包含这些要求,使用 `openspec archive -y --skip-specs event-id-and-reply` 完成归档。

## [2026-05-12 17:52:00] [Session ID: omx-1778510695653-7pd7o2] 状态更新: archive 后执行 focused gates

- 已完成:
  - 归档 `state-operation-layer`
  - 归档 `prompt-contract-runtime-alignment`
  - 归档 `agent-guidance-catalog-cli`
  - 归档 `agent-guidance-contracts`
  - 归档 `scoped-experience-system`
  - 归档 `event-id-and-reply`(`parallel-hat-instances` 主 spec 已包含要求,所以使用 `--skip-specs`)
  - `openspec validate --all --strict` 通过,23 passed,0 failed。
- 即将执行:
  - focused Rust tests for guidance/state/experience/event/prompt areas。
  - replay smoke tests。
  - docs build gate,确认打开的 docs task 是否可关闭。

## [2026-05-12 17:55:00] [Session ID: omx-1778510695653-7pd7o2] 错误记录: guidance manifest 指向已归档 change 的旧路径

- 现象: `cargo test --package ralph-core --lib agent_guidance_manifest` 中 `repository_manifest_passes` 失败。
- 关键输出: `agent-guidance-contracts-change` 缺少 `openspec/changes/agent-guidance-contracts/proposal.md`。
- 已验证结论: archive 移动了 change 目录,但 `agent-guidance-manifest.toml` 仍登记旧 active 路径。
- 修复方向: 将对应 openspec_change 资产改为 archived 状态并指向 `openspec/changes/archive/2026-05-12-.../proposal.md`。

## [2026-05-12 17:58:00] [Session ID: omx-1778510695653-7pd7o2] 状态更新: manifest archive path 修复后继续门禁

- 已完成修复: `agent-guidance-manifest.toml` 中 4 个刚归档 change 的路径已改为 archive 路径,status 已改为 `archived`。
- 已验证: `cargo test --package ralph-core --lib agent_guidance_manifest` 通过,13 passed。
- 即将执行: 继续 state/experience/prompt/event focused tests 和 smoke tests。

## [2026-05-12 18:02:00] [Session ID: omx-1778510695653-7pd7o2] 状态更新: 进入 docs gate

- 已验证 focused gates:
  - guidance manifest: 13 passed
  - state operations: 9 passed
  - experience modules: 29 passed
  - event reply/routing: 12 passed
  - prompt contract anchor真实测试: 2 passed
  - smoke runner: 12 passed,相关集成目标无匹配测试但命令成功
- 注意: 两次错误 filter 产生 0 tests,不计入证据;已用真实测试名重跑。
- 即将执行: 使用 `.venv` 跑 docs build gate。

## [2026-05-12 18:07:00] [Session ID: omx-1778510695653-7pd7o2] 状态更新: 处理 docs dependency pin 任务

- 现象: `.agent/tasks.jsonl` 仍有 `Fix: pin docs build dependencies or explicitly document docs gate` 打开。
- 证据: `.venv/bin/mkdocs build --strict` 已通过,但 workflow 当前仍使用未 pin 的 `pip install mkdocs mkdocs-material ...`。
- 即将执行: 新增/复用 docs requirements 文件固定已验证版本,并让 workflow 使用该文件。

## [2026-05-12 20:46:00] [Session ID: omx-1778510695653-7pd7o2] 阶段完成: 第1步 completed changes 收口完成

### 完成内容
- 已归档:
  - `state-operation-layer`
  - `prompt-contract-runtime-alignment`
  - `agent-guidance-catalog-cli`
  - `agent-guidance-contracts`
  - `scoped-experience-system`
  - `event-id-and-reply`
- 已同步主 specs:
  - 由 `openspec archive` 自动创建/更新相关主 specs。
  - `event-id-and-reply` 的 delta 已在 `parallel-hat-instances` 主 spec 中存在,使用 `--skip-specs` 归档。
- 已修复 archive 后 manifest 漂移:
  - `agent-guidance-manifest.toml` 中刚归档的 guidance change 路径已更新到 archive 路径。
- 已修复 docs gate:
  - 新增 `docs/requirements.txt` pin 已验证的 MkDocs 直接依赖版本。
  - `.github/workflows/docs.yml` 改为 `pip install -r docs/requirements.txt`。
  - `.agent/tasks.jsonl` 中 docs gate 任务已关闭。

### 验证证据
- `openspec validate --all --strict`: 23 passed, 0 failed。
- `cargo test --package ralph-core --lib agent_guidance_manifest`: 13 passed。
- `cargo test --package ralph-core --lib state_operations`: 9 passed。
- `cargo test --package ralph-core --lib experience`: 29 passed。
- `cargo test --package ralph-core --lib reply`: 12 passed。
- `cargo test --package ralph-core --lib test_custom_hat_with_rfc2119_patterns`: 1 passed。
- `cargo test --package ralph-core --lib test_custom_hat_instructions_included_in_prompt`: 1 passed。
- `cargo test -p ralph-core smoke_runner`: smoke runner 12 passed。
- `.venv/bin/mkdocs build --strict`: 通过。
- `.agent/tasks.jsonl`: 已无 open tasks。

## [2026-05-12 20:47:00] [Session ID: omx-1778510695653-7pd7o2] 状态更新: 开始第2步 adapter contract tests

- 行动目的: 将 stdout-only event parsing、prompt_mode、event envelope、termination flush 固定为 adapter 层可测契约。
- 即将执行:
  1. 建立 OpenSpec change `adapter-contract-tests`。
  2. 阅读 adapters / recorder / replay / event parser 当前实现。
  3. 添加 focused contract tests。
  4. 跑 focused tests + smoke。
- 边界:
  - 先补测试和必要小修,不引入新 backend runtime。
  - 不热改 topology。

## [2026-05-12 20:58:00] [Session ID: omx-1778510695653-7pd7o2] 状态更新: adapter-contract-tests OpenSpec artifacts 完成

- 已创建:
  - `openspec/changes/adapter-contract-tests/proposal.md`
  - `openspec/changes/adapter-contract-tests/design.md`
  - `openspec/changes/adapter-contract-tests/specs/adapter-contract-tests/spec.md`
  - `openspec/changes/adapter-contract-tests/tasks.md`
- 已验证: `openspec validate adapter-contract-tests --type change` 通过。
- 下一步: 实现 stream / prompt transport / envelope / termination contract tests。

## [2026-05-12 20:55:54] [Session ID: omx-1778510695653-7pd7o2] 状态更新: 继续 adapter-contract-tests 实现

- 当前阶段: 第2步 adapter contract tests。
- 本轮目标: 按 OpenSpec  继续补齐 stdout-only parsing、prompt transport、event envelope、termination flush 的契约测试。
- 即将执行:
  1. 读取 OpenSpec status/instructions 和 change artifacts。
  2. 阅读相关实现与已有测试,避免重复造测试。
  3. 补 focused contract tests,如测试暴露真实问题则做必要小修。
  4. 更新 tasks.md 并运行 focused tests、smoke、OpenSpec validate。
- 停止条件: adapter contract tests 全部完成并有新鲜验证证据,或遇到 OpenSpec/实现边界冲突需要先修正设计。

## [2026-05-12 21:05:00] [Session ID: omx-1778510695653-7pd7o2] 状态更新: 修正 heredoc 流程错误后继续

- 现象: 追加计划时未使用单引号 heredoc,导致正文反引号触发 shell 命令替换。
- 影响: 没有改坏代码,但命令输出出现 `zsh:3: command not found: adapter-contract-tests`,且计划正文中 change 名缺少反引号表现。
- 处理: 已记录到 `ERRORFIX.md`;后续所有包含反引号的上下文追加改用 `cat <<'EOF'`。
- 即将执行: 读取 `adapter-contract-tests` artifacts 和相关实现代码。

## [2026-05-12 21:21:00] [Session ID: omx-1778510695653-7pd7o2] 状态更新: 已补第一轮 adapter contract tests

- 已修改:
  - `crates/ralph-cli/src/parallel_runner.rs`: 加强 stdout-only event parsing 测试,明确 stderr event 文本不能产生额外事件。
  - `crates/ralph-adapters/src/cli_backend.rs`: 新增 custom backend `prompt_mode=stdin` argv/stdin 契约测试。
  - `crates/ralph-e2e/src/mock_cli.rs`: 修复 mock-cli 按 `TerminalWrite.stdout` 分流 stdout/stderr,并新增回归测试。
  - `crates/ralph-e2e/src/runner.rs`: 新增 mock-mode 配置必须写入 `prompt_mode: stdin` 的契约测试。
  - `crates/ralph-core/src/event_logger.rs`: 新增 `EventRecord` 保留 `id` / `reply` 测试。
  - `crates/ralph-core/src/session_recorder.rs`: 新增关键 record sequence strict parse 与 drop 前 flush 测试。
  - `crates/ralph-proto/src/ux_event.rs`: 新增 `TerminalWrite.instance_id` JSON roundtrip 测试。
- 即将执行: 先跑 `cargo fmt --all --check`,如失败再格式化;随后跑 focused tests。

## [2026-05-12 21:23:00] [Session ID: omx-1778510695653-7pd7o2] 状态更新: cargo fmt check 发现格式差异

- 现象: `cargo fmt --all --check` 退出码 1,仅报告 `mock_cli.rs` 与 `runner.rs` 的格式 diff。
- 判断: 这是格式门禁失败,不是业务语义失败。
- 处理: 立即运行 `cargo fmt --all`,然后继续 focused tests。

## [2026-05-12 21:28:00] [Session ID: omx-1778510695653-7pd7o2] 状态更新: 两个 focused filter 产生 0 tests,准备按真实路径重跑

- 现象:
  - `cargo test -p ralph-cli --bin ralph guardrail_tests::parallel_output_for_event_parsing_is_stdout_only -- --exact` 产生 0 tests。
  - `cargo test -p ralph-adapters --lib tests::test_custom_backend_stdin_prompt_mode_keeps_prompt_off_argv -- --exact` 产生 0 tests。
- 处理: 这两条不计入验证证据。先用 `-- --list` 查真实路径,再用完整路径重跑。
- 已有有效通过证据:
  - `mock_cli::tests::test_replay_terminal_write_records_preserves_stdout_stderr_streams`: 1 passed。
  - `runner::tests::test_configure_mock_mode_uses_stdin_prompt_mode_for_mock_cli`: 1 passed。
  - `event_logger::tests::test_event_record_preserves_id_and_reply`: 1 passed。
  - `session_recorder::tests::test_record_session_critical_sequence_strict_parseable_after_flush`: 1 passed。
  - `session_recorder::tests::test_critical_records_flush_to_file_before_recorder_drop`: 1 passed。
  - `ux_event::tests::test_terminal_write_instance_id_roundtrip`: 1 passed。

## [2026-05-12 21:34:00] [Session ID: omx-1778510695653-7pd7o2] 状态更新: adapter contract focused tests 已通过

- 有效 focused tests:
  - `parallel_runner::guardrail_tests::parallel_output_for_event_parsing_is_stdout_only`: 1 passed。
  - `cli_backend::tests::test_custom_backend_stdin_prompt_mode_keeps_prompt_off_argv`: 1 passed。
  - `mock_cli::tests::test_replay_terminal_write_records_preserves_stdout_stderr_streams`: 1 passed。
  - `runner::tests::test_configure_mock_mode_uses_stdin_prompt_mode_for_mock_cli`: 1 passed。
  - `event_logger::tests::test_event_record_preserves_id_and_reply`: 1 passed。
  - `ux_event::tests::test_terminal_write_instance_id_roundtrip`: 1 passed。
  - `event_logger::tests::test_runtime_durable_payloads_are_not_truncated`: 1 passed。
  - `session_recorder::tests::test_record_session_critical_sequence_strict_parseable_after_flush`: 1 passed。
  - `session_recorder::tests::test_critical_records_flush_to_file_before_recorder_drop`: 1 passed。
  - `session_recorder::tests::test_flush_policy_meta_and_bus_publish_flush`: 1 passed。
  - `session_recorder::tests::test_flush_policy_terminal_write_stdout_only`: 1 passed。
- 格式门禁: `cargo fmt --all --check` 通过。
- 即将执行: 将 OpenSpec `tasks.md` 中 2.1-5.2 标记完成,随后跑更宽的 package focused gates。

## [2026-05-12 21:40:00] [Session ID: omx-1778510695653-7pd7o2] 状态更新: adapter-contract-tests 进入 smoke/OpenSpec 门禁

- 已验证:
  - `beautiful-mermaid-rs --ascii` 可渲染 `design.md` 中 Mermaid 图。
  - `openspec validate adapter-contract-tests --type change` 通过。
- 即将执行:
  - `cargo test -p ralph-core smoke_runner`
  - `openspec validate --all --strict`

## [2026-05-12 21:44:00] [Session ID: omx-1778510695653-7pd7o2] 阶段完成: 第2步 adapter contract tests 完成

- 已完成 OpenSpec change: `adapter-contract-tests`。
- 已完成 tasks: 16/16。
- 关键修复: `ralph-e2e mock-cli` 现在按 `TerminalWrite.stdout` 分流 stdout/stderr,不再把 stderr cassette 证据回放到 stdout。
- 验证:
  - focused tests 均为真实 1 passed,两次早先 0 tests filter 不计入证据。
  - `cargo test -p ralph-core smoke_runner`: 12 passed。
  - `openspec validate --all --strict`: 24 passed,0 failed。
- 下一步: 进入第3步 `startup-resource-bootstrap v1`,先读取该 OpenSpec change artifacts 和现有配置/prompt/preset 入口。

## [2026-05-12 21:48:00] [Session ID: omx-1778510695653-7pd7o2] 状态更新: 归档刚完成的 adapter-contract-tests

- 行动目的: 第2步已经完成,为避免 active OpenSpec drift,将 `adapter-contract-tests` 归档并同步主 spec。
- 即将执行:
  - `openspec archive -y adapter-contract-tests`
  - `openspec validate --all --strict`

## [2026-05-12 21:49:00] [Session ID: omx-1778510695653-7pd7o2] 状态更新: adapter-contract-tests 已归档,进入 startup-resource-bootstrap

- 已完成: `adapter-contract-tests` 归档并同步主 spec。
- 当前 active changes:
  - `startup-resource-bootstrap`
  - `runtime-capability-invocation`
  - `tui-mdfried-viewer`
- 即将执行: 使用 `openspec-apply-change` 流程读取 `startup-resource-bootstrap` status/instructions/context files。

## [2026-05-12 22:12:00] [Session ID: omx-1778510695653-7pd7o2] 状态更新: 继续 startup-resource-bootstrap v1 小步接入

- 现象: 上一个大补丁因为 `main.rs` module 声明上下文不匹配而没有应用。
- 处理: 改为小步补丁,先新增 `startup_resources.rs`,再查看真实 `main.rs` module 区域后接入。
- 当前目标不变: startup-only 规则 selector + resolved config artifact + 无 config/prompt 启动闭环。

## [2026-05-12 22:20:00] [Session ID: omx-1778510695653-7pd7o2] 状态更新: startup-resource-bootstrap v1 代码已接入

- 已新增 `crates/ralph-cli/src/startup_resources.rs`:
  - resource catalog schema。
  - 用户资源目录解析。
  - embedded resources 首次同步且不覆盖用户修改。
  - v1 规则 selector。
  - resolved config / selection artifact 写出。
- 已接入 `run_command`:
  - 默认 `ralph.yml` 缺失、无 `-p/-P`、非 continue 时才触发 startup selector。
  - selector 在真实 `EventLoop` / `Supervisor` 初始化前完成。
- 已更新 `.envrc`: 增加 `RALPH_HOME` 默认说明。
- 即将执行: cargo fmt/check focused tests,若出现编译错误则按真实错误修复。

## [2026-05-12 22:21:00] [Session ID: omx-1778510695653-7pd7o2] 状态更新: startup_resources 格式门禁修正

- 现象: `cargo fmt --all --check` 报 `startup_resources.rs` 两处长行格式差异。
- 处理: 运行 `cargo fmt --all`,再进入 focused tests。

## [2026-05-12 22:33:00] [Session ID: omx-1778510695653-7pd7o2] 状态更新: 继续 startup-resource-bootstrap v1 验证与收口

- 当前接手点: 第3步 `startup-resource-bootstrap v1`,前一轮已经新增 `startup_resources.rs` 并接入 `run_command`,但最后的 `cargo fmt --all --check` 输出尚未确认。
- 本轮目标:
  1. 读取当前 OpenSpec change 与真实代码状态,区分已完成实现和未完成任务。
  2. 补足 startup bootstrap 的行为级测试或动态 dry-run 证据。
  3. 更新 `openspec/changes/startup-resource-bootstrap/tasks.md`。
  4. 补必要文档,并跑 focused tests、smoke tests、OpenSpec validate、docs gate。
- 约束:
  - 只做 startup bootstrap v1,不做 runtime capability invocation。
  - 不做 live topology hot switch。
  - 不撤回非本轮创建或明确相关的既有改动。
  - 所有包含反引号的上下文追加使用单引号 heredoc。
- 停止条件: `startup-resource-bootstrap` 已完成可验证任务并通过门禁,或出现与 OpenSpec 边界冲突的真实阻塞。

## [2026-05-12 22:35:30] [Session ID: omx-1778510695653-7pd7o2] 状态更新: omx explore 不可用后切回本地查询

- 现象: `omx explore` 两次尝试均失败,stderr 显示缺少 `JU_CODE` 环境变量。
- 判断: 这是本机 OMX/模型 provider 环境问题,不是 Ralph 业务代码错误。
- 处理: 不把该命令计入仓库证据;改用本地 `rg` / `sed` 查询 `run_command`、`RunArgs`、`dry_run` 和现有测试入口。
- 下一步: 补齐启动行为级证据,优先选择不会污染主仓库的临时目录 dry-run 或隔离测试。

## [2026-05-12 22:40:04] [Session ID: omx-1778510695653-7pd7o2] 状态更新: 修正 resolved artifact 与测试隔离

- 已观察到两个实现风险:
  1. `resolve_default_bootstrap()` 在测试中会解析真实 `HOME`,可能写入用户 `$HOME/.ralph/resources`。
  2. `write_bootstrap_artifacts()` 当前在 CLI override/validate 之前执行,导致 `.ralph/resolved-config.yml` 可能不是最终实际启动配置。
- 当前假设: 通过注入式 resource root resolver 和移动 artifact 写出位置,可以同时满足测试隔离和 resolved config 契约。
- 即将执行:
  - 给 bootstrap resolution 增加基于 `ResourceRoot` 的内部函数。
  - 让单元测试使用 `TempDir` 资源根目录。
  - 将 artifact 写出移动到 CLI override、backend auto-detect、validate 之后,但在 dry-run/真实 loop 之前。
  - 补 CLI dry-run integration test 证明无 `ralph.yml` / 无 `PROMPT.md` 时会产出 artifact。

## [2026-05-12 22:57:56] [Session ID: omx-1778510695653-7pd7o2] 状态更新: startup focused gates 通过,补 3.3 覆盖缺口

- 已通过:
  - `cargo fmt --all --check`
  - `cargo test -p ralph-cli --bin ralph startup_resources -- --nocapture`: 6 passed。
  - `cargo test -p ralph-cli --bin ralph cli_config_explicit_detector -- --nocapture`: 2 passed。
  - `cargo test -p ralph-cli --test integration_startup_resources -- --nocapture`: 2 passed。
  - `openspec validate startup-resource-bootstrap --type change`: 通过。
  - `beautiful-mermaid-rs --ascii` 验证 `docs/runbook/startup-resource-bootstrap.md` Mermaid 图: 通过。
- 重新审视:
  - `tasks.md` 中 3.3 要求覆盖串行 run、并行 run、idle-capable preset、自带 inline prompt preset。
  - 当前已有串行 dry-run 和 prompt resolver 覆盖,但并行/inline preset 覆盖还不够直接。
- 即将执行:
  - 补 startup resource resolver 的并行 workflow 测试。
  - 补 self-contained inline prompt workflow 不应被 idle bootstrap 覆盖的测试。
  - 再跑 focused tests 后继续 smoke/docs/OpenSpec 全量门禁。

## [2026-05-12 23:03:14] [Session ID: omx-1778510695653-7pd7o2] 状态更新: startup 3.3 覆盖补齐后进入全量门禁

- 已修正格式失败: `cargo fmt --all --check` 先前只报告新测试断言换行 diff,已运行 `cargo fmt --all`。
- 已通过:
  - `cargo fmt --all --check`
  - `cargo test -p ralph-cli --bin ralph startup_resources -- --nocapture`: 8 passed。
  - `cargo test -p ralph-cli --test integration_startup_resources -- --nocapture`: 2 passed。
- 新增覆盖:
  - 并行 workflow resolved config 可注入 prompt template 并通过 validate。
  - 自带 inline prompt workflow 不会被 idle/bootstrap prompt 覆盖。
- 即将执行:
  - `cargo test -p ralph-core smoke_runner`
  - `openspec validate --all --strict`
  - `.venv/bin/mkdocs build --strict`

## [2026-05-12 23:04:13] [Session ID: omx-1778510695653-7pd7o2] 状态更新: startup-resource-bootstrap 门禁通过,准备归档

- 已通过全量门禁:
  - `cargo test -p ralph-core smoke_runner`: 12 passed。
  - `openspec validate --all --strict`: 24 passed,0 failed。
  - `.venv/bin/mkdocs build --strict`: 退出码 0。
- docs gate 备注:
  - MkDocs 输出 Material 团队关于 MkDocs 2.0 的上游 warning,不是本仓库文档错误。
  - MkDocs 提示 `agent-guidance-schema.md` / `prompt-contract.md` 未纳入当前 nav,这是现有信息架构状态,本次 startup 页面已加入 nav。
- 即将执行:
  - `openspec archive -y startup-resource-bootstrap`
  - 归档后 `openspec validate --all --strict`

## [2026-05-12 23:05:52] [Session ID: omx-1778510695653-7pd7o2] 阶段完成: startup-resource-bootstrap v1 已归档

- 已完成第3步 `startup-resource-bootstrap v1`。
- 归档结果:
  - `openspec archive -y startup-resource-bootstrap` 成功。
  - 已创建主 spec: `openspec/specs/resource-bootstrap/spec.md`。
  - archive 路径: `openspec/changes/archive/2026-05-12-startup-resource-bootstrap/`。
- 归档后验证:
  - `openspec validate --all --strict`: 24 passed,0 failed。
- 当前 active changes:
  - `runtime-capability-invocation`: 0/11。
  - `tui-mdfried-viewer`: 13/15。
- 下一步: 按用户指定路线进入第4步 `runtime-capability-invocation v1`,边界是隔离 child run / micro-run,不要热改 live topology。

## [2026-05-12 23:08:57] [Session ID: omx-1778510695653-7pd7o2] 状态更新: runtime-capability-invocation v1 落点收窄

- 当前目标: 实现 runtime capability invocation v1。
- 约束边界:
  - 不热改当前 active `EventLoop` / `Supervisor` topology。
  - workflow capability 走 isolated child run artifact。
  - hat capability 走 isolated micro-run artifact。
  - v1 先做规则驱动 chooser,不做 LLM fallback。
- 计划落点:
  1. `ralph-core` 增加 capability metadata / protocol / artifact 数据结构。
  2. `ralph-cli` 增加 capability catalog,从 startup resource catalog 暴露轻量 summary,不加载完整 workflow/hat instructions 到启动上下文。
  3. `ralph tools capability list|summaries|invoke` 作为 `ralph#1` 可用的 agent-facing 调用面。
  4. `invoke` 写 `.ralph/capability-invocations/<id>/invoke.json`、`result.json` 或 `failed.json`,并产出 isolated child/micro-run resolved config。
  5. 用 focused tests 证明 parent topology 不变: invocation 只写 isolated artifacts,不修改当前 `ralph.yml` / active config。
- 下一步: 读取 event logger / tools / config 类型后开始实现 core module 和 CLI capability 工具。

## [2026-05-12 23:27:37] [Session ID: omx-1778510695653-7pd7o2] 状态更新: runtime-capability focused gates 通过

- 已完成第4步 runtime-capability-invocation v1 实现:
  - `crates/ralph-core/src/capability.rs`: metadata/protocol/artifact 类型。
  - `crates/ralph-cli/src/capability.rs`: `ralph tools capability list|summaries|invoke`。
  - `crates/ralph-cli/tests/integration_capability.rs`: 真实 CLI integration。
  - `docs/runbook/runtime-capabilities.md`: v1/v2 路线、artifact、topology 边界。
- 已通过 focused gates:
  - `cargo fmt --all --check`
  - `cargo test -p ralph-core capability -- --nocapture`: 1 passed。
  - `cargo test -p ralph-cli --bin ralph capability -- --nocapture`: 3 passed。
  - `cargo test -p ralph-cli --test integration_capability -- --nocapture`: 2 passed。
  - `openspec validate runtime-capability-invocation --type change`: 通过。
  - `.venv/bin/mkdocs build --strict`: 退出码 0。
- docs gate 备注:
  - Material/MkDocs 2.0 上游 warning 仍存在,不是本次文档错误。
- 即将执行:
  - `cargo test -p ralph-core smoke_runner`
  - `openspec validate --all --strict`
  - 通过后归档 `runtime-capability-invocation`。

## [2026-05-12 23:32:05] [Session ID: omx-1778510695653-7pd7o2] 状态更新: 最终补跑 cargo test 全量门禁

- 原因: 项目规则要求完成前运行 `cargo test`。
- 已有证据覆盖 focused / smoke / docs / OpenSpec,但仍需补齐全量 cargo test。
- 即将执行: `cargo test`。

## [2026-05-12 23:37:57] [Session ID: omx-1778510695653-7pd7o2] 状态更新: 接续上一轮最终门禁

- 当前接续点: 上一轮已完成四条 Ralph 演进主线,但最后启动的 `cargo test` 全量门禁没有在摘要中留下最终退出状态。
- 本次行动: 先检查是否还有正在运行的 cargo/test 进程;如果没有可读的最终结果,直接重新运行 `cargo test`,把最终门禁补成当前会话可验证证据。
- 完成标准:
  - `cargo test` 有明确退出码和关键输出。
  - 若失败,先修复失败并重跑相关 focused test 与全量门禁。
  - 若通过,更新 `WORKLOG.md` 并最终汇报四步完成证据。

## [2026-05-12 23:40:43] [Session ID: omx-1778510695653-7pd7o2] 状态更新: cargo test 已有当前证据,继续补齐收尾门禁

- 已确认: 本轮重新执行的 `cargo test` 以 `CARGO_TEST_EXIT:0` 结束。
- 即将执行:
  - `cargo fmt --all --check`
  - `cargo test -p ralph-core smoke_runner`
  - `openspec validate --all --strict`
  - `openspec list --json`
  - `git status --short`
- 目的: 用当前会话的证据补齐格式、烟测、OpenSpec 与工作区状态,再更新 `WORKLOG.md` 和最终汇报。

## [2026-05-12 23:43:10] [Session ID: omx-1778510695653-7pd7o2] 阶段完成: Ralph 演进四步收口已完成

- [x] 第1步: 收口 completed OpenSpec changes,完成 archive / spec sync / focused tests / smoke tests / docs gate。
- [x] 第2步: 补 adapter contract tests,覆盖 stdout-only event parsing、prompt_mode、event envelope、termination flush。
- [x] 第3步: 推进并归档 `startup-resource-bootstrap v1`,保留 startup-only selector + resolved config artifact 边界。
- [x] 第4步: 推进并归档 `runtime-capability-invocation v1`,采用 isolated child run / micro-run,不热改 live topology。
- [x] 最终门禁: 本轮重新执行 `cargo test`,退出码 0;并补跑 `cargo fmt --all --check`、`cargo test -p ralph-core smoke_runner`、`openspec validate --all --strict`、`.venv/bin/mkdocs build --strict`。

当前状态:
- 用户指定的四步路线已完成。
- `openspec list --json` 当前只剩 `tui-mdfried-viewer` 一个 active change,状态为 13/15 in-progress。
- 本任务不继续扩到 `tui-mdfried-viewer`,除非用户后续明确指定。

## [2026-05-13 12:59:15] [Session ID: omx-1778510695653-7pd7o2] 状态更新: 进入提交前变更分组审查

- 当前目标: 在不提交、不撤回任何内容的前提下,审查当前工作区改动,把 Ralph 演进四步相关变更与其他历史/支线改动区分清楚。
- 即将执行:
  - 查看 `git status --short`、`git diff --stat`、`git diff --name-status`。
  - 按 OpenSpec archive/spec、runtime evidence/adapter contract、startup bootstrap、runtime capability、docs/context、其他支线进行分组。
  - 检查是否存在明显 ephemeral 文件或不应提交的产物。
- 完成标准: 给出可执行的提交分组建议,但不自动提交。

## [2026-05-13 13:01:39] [Session ID: omx-1778510695653-7pd7o2] 阶段完成: 提交前变更分组审查完成

- [x] 已完成 `git status` / `git diff` / untracked 分组审查。
- [x] 已确认 `git diff --check` 无 whitespace error。
- [x] 已确认主要 generated artifact 处于 ignored 状态。
- [x] 已形成提交拆分建议: 不要 `git add .`,应按主题 staged。

当前状态:
- 没有自动提交。
- 没有清理或撤回任何用户/历史支线改动。
- 如果继续到提交阶段,需要先选择一个主题分组做 staged diff 审查。

## [2026-05-13 14:07:53] [Session ID: omx-1778510695653-7pd7o2] 状态更新: 准备 staging 第一组 adapter contract

- 当前目标: 为 `adapter contract tests + evidence stream fixes` 准备一个 staged diff。
- 候选文件:
  - `crates/ralph-adapters/src/cli_backend.rs`
  - `crates/ralph-cli/src/parallel_runner.rs`
  - `crates/ralph-core/src/event_logger.rs`
  - `crates/ralph-core/src/session_recorder.rs`
  - `crates/ralph-e2e/src/mock_cli.rs`
  - `crates/ralph-proto/src/ux_event.rs`
  - `openspec/changes/archive/2026-05-12-adapter-contract-tests/`
  - `openspec/specs/adapter-contract-tests/`
- 风险: 某些候选文件可能包含别的任务线改动,所以先审查 diff,再 stage。

## [2026-05-13 14:15:15] [Session ID: omx-1778510695653-7pd7o2] 阶段完成: 第一组 adapter/evidence contract staged diff 已准备

- [x] 已 reset index,避免残留 staged 状态污染本次分组。
- [x] 已 stage 第一组候选文件。
- [x] 对混线文件 `crates/ralph-cli/src/parallel_runner.rs` 使用 patch staging,只选 stdout-only event parsing guardrail 测试,跳过 runtime graph / Codex escaped-event normalization 等其他线。
- [x] 已修复 `openspec/specs/adapter-contract-tests/spec.md` 末尾多余空行。
- [x] 已重跑 focused tests 和 `openspec validate adapter-contract-tests --type spec`。
- [x] `git diff --cached --check` 通过。

当前状态:
- 第一组 staged diff 已准备好。
- 尚未 commit。
- unstaged worktree 仍保留其他主题线改动。
